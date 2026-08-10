mod dto;
mod migration;
mod security;
mod services;

use std::{
    collections::HashMap,
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, MutexGuard, OnceLock, TryLockError},
};

use dto::{
    AiPromptInput, BootstrapData, ResearchQuery, UiJob, UiMigrationStatus, UiSettings,
    UiWatchlistStatus,
};
use scriptotar_core::{LegacyImportReport, SettingsRepository, WatchlistRepository};
use scriptotar_db::{SqliteStore, WatchlistRefreshState};
use services::AppServices;
use tauri::Manager;
use uuid::Uuid;

const MIGRATION_COMPLETED_MARKER: &str = ".legacy-migration-completed";
static MIGRATION_OPERATION: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, serde::Serialize)]
struct BackendHealth {
    schema_version: u32,
}

#[derive(Clone)]
struct OperationalState {
    data_dir: PathBuf,
    store: SqliteStore,
}

#[derive(Debug, Clone, Copy)]
enum NativePicker {
    MediaFile,
    OutputDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrationLockError {
    Busy,
    Poisoned,
}

fn command_error(error: impl ToString) -> String {
    error.to_string()
}

fn packaged_executable(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    }
}

fn set_path_env_if_missing(key: &str, value: &Path) {
    if env::var_os(key).is_none() {
        env::set_var(key, value.as_os_str());
    }
}

fn configure_packaged_runtime(resource_dir: &Path, data_dir: &Path) {
    let runtime_dir = resource_dir.join("transcription-runtime");
    set_path_env_if_missing(
        "SCRIPTOTAR_SIDECAR_PYTHON",
        &runtime_dir.join(packaged_executable("scriptotar-transcription")),
    );
    set_path_env_if_missing("SCRIPTOTAR_SIDECAR_SCRIPT", &runtime_dir.join("sidecar.py"));
    set_path_env_if_missing(
        "SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE",
        &runtime_dir
            .join("engine")
            .join(packaged_executable("scriptotar-engine")),
    );
    set_path_env_if_missing(
        "SCRIPTOTAR_YTDLP_EXECUTABLE",
        &runtime_dir.join(packaged_executable("scriptotar-ytdlp")),
    );
    set_path_env_if_missing("HF_HOME", &data_dir.join("models"));

    if env::var_os("PYTHONUNBUFFERED").is_none() {
        env::set_var("PYTHONUNBUFFERED", "1");
    }

    let ffmpeg_dir = runtime_dir.join("ffmpeg");
    let mut search_path = vec![ffmpeg_dir];
    if let Some(existing) = env::var_os("PATH") {
        search_path.extend(env::split_paths(&existing));
    }
    if let Ok(joined) = env::join_paths(search_path) {
        env::set_var("PATH", joined);
    }
}

fn try_migration_operation() -> Result<MutexGuard<'static, ()>, MigrationLockError> {
    match MIGRATION_OPERATION.get_or_init(|| Mutex::new(())).try_lock() {
        Ok(guard) => Ok(guard),
        Err(TryLockError::WouldBlock) => Err(MigrationLockError::Busy),
        Err(TryLockError::Poisoned(_)) => Err(MigrationLockError::Poisoned),
    }
}

fn migration_completed(data_dir: &Path) -> Result<bool, String> {
    let marker = data_dir.join(MIGRATION_COMPLETED_MARKER);
    match fs::symlink_metadata(&marker) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(
            "The legacy migration completion marker is unsafe. Scriptotar refused to follow it."
                .to_owned(),
        ),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("Scriptotar could not inspect the migration completion marker.".to_owned()),
    }
}

fn mark_migration_completed(data_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(data_dir)
        .map_err(|_| "Scriptotar could not prepare migration state storage.".to_owned())?;
    let marker = data_dir.join(MIGRATION_COMPLETED_MARKER);
    let mut file = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&marker)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return if migration_completed(data_dir)? {
                harden_migration_marker_permissions(&marker)
            } else {
                Err("Scriptotar could not validate the migration completion marker.".to_owned())
            };
        }
        Err(_) => {
            return Err("Scriptotar could not create the migration completion marker.".to_owned())
        }
    };
    let result = file
        .write_all(b"scriptotar-legacy-migration-completed-v1\n")
        .and_then(|_| file.sync_all())
        .map_err(|_| "Scriptotar could not persist the migration completion marker.".to_owned())
        .and_then(|_| harden_migration_marker_permissions(&marker));
    if result.is_err() {
        drop(file);
        let _ = fs::remove_file(&marker);
    }
    result
}

#[cfg(unix)]
fn harden_migration_marker_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|_| "Scriptotar could not restrict migration marker permissions.".to_owned())
}

#[cfg(not(unix))]
fn harden_migration_marker_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn finalize_migration_success(report: LegacyImportReport, data_dir: &Path) -> UiMigrationStatus {
    if let Err(error) = mark_migration_completed(data_dir) {
        eprintln!("[scriptotar-migration] could not finalize completion marker: {error}");
        let status = UiMigrationStatus::failed(
            "Legacy data was imported, but Scriptotar could not persist the one-time migration completion marker. The source database was not modified. Restart and retry migration finalization.",
        );
        migration::set_status(status.clone());
        return status;
    }

    let active_stage = data_dir.join("history.sqlite3");
    if let Err(error) = fs::remove_file(&active_stage) {
        if error.kind() != io::ErrorKind::NotFound {
            eprintln!("[scriptotar-migration] could not remove completed staging snapshot");
        }
    }
    let status = UiMigrationStatus::completed(report);
    migration::set_status(status.clone());
    status
}

fn complete_prepared_migration(services: &AppServices, data_dir: &Path) -> UiMigrationStatus {
    let in_progress = UiMigrationStatus::in_progress();
    migration::set_status(in_progress);
    if let Err(error) = migration::activate_pending_stage(data_dir) {
        let status = UiMigrationStatus::failed(error);
        migration::set_status(status.clone());
        return status;
    }
    match services.import_legacy_data() {
        Ok(report) => finalize_migration_success(report, data_dir),
        Err(error) => {
            eprintln!("[scriptotar-migration] import failed: {error}");
            let status = UiMigrationStatus::failed(
                "The legacy snapshot was prepared safely, but importing it failed. The source database was not modified. Retry after resolving the local database error.",
            );
            migration::set_status(status.clone());
            status
        }
    }
}

fn completed_status_if_marked(data_dir: &Path) -> Option<UiMigrationStatus> {
    match migration_completed(data_dir) {
        Ok(true) => {
            let status = UiMigrationStatus::previously_completed();
            migration::set_status(status.clone());
            Some(status)
        }
        Ok(false) => None,
        Err(error) => {
            let status = UiMigrationStatus::failed(error);
            migration::set_status(status.clone());
            Some(status)
        }
    }
}

fn migration_lock_status(error: MigrationLockError) -> UiMigrationStatus {
    match error {
        MigrationLockError::Busy => UiMigrationStatus::in_progress(),
        MigrationLockError::Poisoned => UiMigrationStatus::failed(
            "Migration coordination is unavailable. Restart Scriptotar before retrying migration.",
        ),
    }
}

#[tauri::command]
fn backend_health(state: tauri::State<'_, AppServices>) -> Result<BackendHealth, String> {
    Ok(BackendHealth {
        schema_version: state.schema_version().map_err(command_error)?,
    })
}

#[tauri::command]
fn bootstrap_app(state: tauri::State<'_, AppServices>) -> Result<BootstrapData, String> {
    state.bootstrap().map_err(command_error)
}

#[tauri::command]
fn list_jobs(state: tauri::State<'_, AppServices>) -> Result<Vec<UiJob>, String> {
    state.list_jobs().map_err(command_error)
}

#[tauri::command]
fn get_watchlist_statuses(
    state: tauri::State<'_, OperationalState>,
) -> Result<Vec<UiWatchlistStatus>, String> {
    let watchlists = state.store.list_watchlists(None).map_err(command_error)?;
    let persisted = state
        .store
        .list_watchlist_refresh_status(None)
        .map_err(command_error)?
        .into_iter()
        .map(|status| (status.watchlist_id, status))
        .collect::<HashMap<_, _>>();
    let auto_watch = state
        .store
        .load_settings()
        .map_err(command_error)?
        .auto_watch;

    Ok(watchlists
        .into_iter()
        .map(|watchlist| {
            let status = persisted.get(&watchlist.id);
            let mut state_name = status
                .map(|status| status.state.as_str())
                .unwrap_or_else(|| {
                    if watchlist.last_scan_at.is_some() {
                        "healthy"
                    } else {
                        "never_scanned"
                    }
                })
                .to_owned();
            if !auto_watch && state_name == WatchlistRefreshState::RetryScheduled.as_str() {
                state_name = "failed".to_owned();
            }
            UiWatchlistStatus {
                watchlist_id: watchlist.id.to_string(),
                project_id: watchlist.project_id.to_string(),
                label: watchlist.label,
                state: state_name,
                last_attempt_at: status.and_then(|status| status.last_attempt_at.clone()),
                last_successful_scan_at: status
                    .and_then(|status| status.last_success_at.clone())
                    .or(watchlist.last_scan_at),
                last_error: status.and_then(|status| status.last_error.clone()),
                next_retry_at: if auto_watch {
                    status.and_then(|status| status.next_retry_at.clone())
                } else {
                    None
                },
            }
        })
        .collect())
}

#[tauri::command]
fn get_migration_status() -> UiMigrationStatus {
    migration::current_status()
}

#[tauri::command]
fn retry_legacy_migration(
    services: tauri::State<'_, AppServices>,
    operational: tauri::State<'_, OperationalState>,
) -> UiMigrationStatus {
    let _guard = match try_migration_operation() {
        Ok(guard) => guard,
        Err(error) => return migration_lock_status(error),
    };
    if let Some(status) = completed_status_if_marked(&operational.data_dir) {
        return status;
    }
    match migration::retry_discovery(&operational.data_dir) {
        migration::Preparation::Ready => {
            complete_prepared_migration(&services, &operational.data_dir)
        }
        _ => migration::current_status(),
    }
}

#[tauri::command]
fn select_legacy_migration_candidate(
    candidate_id: String,
    services: tauri::State<'_, AppServices>,
    operational: tauri::State<'_, OperationalState>,
) -> UiMigrationStatus {
    let _guard = match try_migration_operation() {
        Ok(guard) => guard,
        Err(error) => return migration_lock_status(error),
    };
    if let Some(status) = completed_status_if_marked(&operational.data_dir) {
        return status;
    }
    match migration::choose_candidate(&operational.data_dir, &candidate_id) {
        migration::Preparation::Ready => {
            complete_prepared_migration(&services, &operational.data_dir)
        }
        _ => migration::current_status(),
    }
}

#[tauri::command]
fn select_project(
    project_id: Uuid,
    state: tauri::State<'_, AppServices>,
) -> Result<BootstrapData, String> {
    state.select_project(project_id).map_err(command_error)
}

#[tauri::command]
fn create_project(
    name: String,
    state: tauri::State<'_, AppServices>,
) -> Result<BootstrapData, String> {
    security::validate_project_name(&name)?;
    state.create_project(name).map_err(command_error)
}

#[tauri::command]
fn enqueue_local_media(
    project_id: Uuid,
    path: String,
    state: tauri::State<'_, AppServices>,
) -> Result<scriptotar_core::Job, String> {
    let path = security::validated_local_media_path(&path)?;
    state
        .enqueue_local_media(project_id, path)
        .map_err(command_error)
}

#[tauri::command]
fn enqueue_url(
    project_id: Uuid,
    url: String,
    state: tauri::State<'_, AppServices>,
) -> Result<scriptotar_core::Job, String> {
    security::validate_url_argument("media URL", &url)?;
    state.enqueue_url(project_id, url).map_err(command_error)
}

#[tauri::command]
fn choose_local_media() -> Result<Option<String>, String> {
    native_picker(NativePicker::MediaFile)
}

#[tauri::command]
fn choose_output_directory() -> Result<Option<String>, String> {
    native_picker(NativePicker::OutputDirectory)
}

#[tauri::command]
fn cancel_job(job_id: Uuid, state: tauri::State<'_, AppServices>) -> Result<(), String> {
    state.cancel_job(job_id).map_err(command_error)
}

#[tauri::command]
fn retry_job(
    job_id: Uuid,
    state: tauri::State<'_, AppServices>,
) -> Result<scriptotar_core::Job, String> {
    state.retry_job(job_id).map_err(command_error)
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppServices>) -> Result<UiSettings, String> {
    state.load_settings().map_err(command_error)
}

#[tauri::command]
fn save_settings(settings: UiSettings, state: tauri::State<'_, AppServices>) -> Result<(), String> {
    state.save_settings(settings).map_err(command_error)
}

#[tauri::command]
fn import_legacy_data(
    state: tauri::State<'_, AppServices>,
    operational: tauri::State<'_, OperationalState>,
) -> Result<LegacyImportReport, String> {
    let _guard = try_migration_operation().map_err(|error| match error {
        MigrationLockError::Busy => "Legacy migration is already in progress.".to_owned(),
        MigrationLockError::Poisoned => {
            "Migration coordination is unavailable. Restart Scriptotar before retrying migration."
                .to_owned()
        }
    })?;
    if migration_completed(&operational.data_dir)? {
        migration::set_status(UiMigrationStatus::previously_completed());
        return Err("Legacy migration is already complete on this installation.".to_owned());
    }
    migration::set_status(UiMigrationStatus::in_progress());
    match state.import_legacy_data() {
        Ok(report) => {
            let status = finalize_migration_success(report.clone(), &operational.data_dir);
            if status.state == "completed" {
                Ok(report)
            } else {
                Err("Legacy data was imported, but migration finalization needs recovery.".to_owned())
            }
        }
        Err(error) => {
            eprintln!("[scriptotar-migration] manual import failed: {error}");
            migration::set_status(UiMigrationStatus::failed(
                "Legacy import failed. The source database was left untouched; retry after resolving the local database error.",
            ));
            Err("Legacy import failed. Check migration status for recovery options.".to_owned())
        }
    }
}

#[tauri::command]
fn save_watchlist(
    query: ResearchQuery,
    state: tauri::State<'_, AppServices>,
) -> Result<BootstrapData, String> {
    security::validate_research_query(&query)?;
    state.save_watchlist(query)
}

#[tauri::command]
fn scan_creator(query: ResearchQuery, state: tauri::State<'_, AppServices>) -> Result<(), String> {
    security::validate_research_query(&query)?;
    state.scan_creator(query)
}

#[tauri::command]
fn queue_research(ids: Vec<String>, state: tauri::State<'_, AppServices>) -> Result<(), String> {
    security::validate_research_ids(&ids)?;
    state.queue_research(ids)
}

#[tauri::command]
fn build_ai_prompt(
    input: AiPromptInput,
    state: tauri::State<'_, AppServices>,
) -> Result<String, String> {
    security::validate_ai_input(&input)?;
    state.build_ai_prompt(&input)
}

#[tauri::command]
fn run_ai(input: AiPromptInput, state: tauri::State<'_, AppServices>) -> Result<String, String> {
    security::validate_ai_input(&input)?;
    state.run_ai(&input)
}

fn command_output(command: &mut Command) -> Result<Option<String>, io::Error> {
    let output = command.output()?;
    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        return Ok((!value.is_empty()).then_some(value));
    }
    if output.status.code() == Some(1) {
        return Ok(None);
    }
    Err(io::Error::other(format!(
        "native picker exited with {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    )))
}

#[cfg(target_os = "linux")]
fn native_picker(kind: NativePicker) -> Result<Option<String>, String> {
    let mut zenity = Command::new("zenity");
    zenity.arg("--file-selection");
    match kind {
        NativePicker::MediaFile => {
            zenity
                .arg("--title=Choose video")
                .arg("--file-filter=Supported media | *.mp4 *.mkv *.mov *.webm *.m4v *.avi *.mp3 *.wav *.m4a *.flac *.ogg *.opus")
                .arg("--file-filter=All files | *");
        }
        NativePicker::OutputDirectory => {
            zenity
                .arg("--directory")
                .arg("--title=Choose Scriptotar output folder");
        }
    }
    match command_output(&mut zenity) {
        Ok(value) => return Ok(value),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("native file picker failed: {error}")),
    }

    let mut kdialog = Command::new("kdialog");
    match kind {
        NativePicker::MediaFile => {
            kdialog
                .arg("--getopenfilename")
                .arg(".")
                .arg("*.mp4 *.mkv *.mov *.webm *.m4v *.avi *.mp3 *.wav *.m4a *.flac *.ogg *.opus|Supported media");
        }
        NativePicker::OutputDirectory => {
            kdialog.arg("--getexistingdirectory").arg(".");
        }
    }
    match command_output(&mut kdialog) {
        Ok(value) => Ok(value),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Err(
            "No supported native file picker is installed. Install zenity or kdialog, or use the advanced manual path field."
                .to_owned(),
        ),
        Err(error) => Err(format!("native file picker failed: {error}")),
    }
}

#[cfg(target_os = "windows")]
fn native_picker(kind: NativePicker) -> Result<Option<String>, String> {
    let script = match kind {
        NativePicker::MediaFile => {
            r#"Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Title = 'Choose video'; $d.Filter = 'Supported media|*.mp4;*.mkv;*.mov;*.webm;*.m4v;*.avi;*.mp3;*.wav;*.m4a;*.flac;*.ogg;*.opus|All files|*.*'; if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { [Console]::Write($d.FileName) }"#
        }
        NativePicker::OutputDirectory => {
            r#"Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.FolderBrowserDialog; $d.Description = 'Choose Scriptotar output folder'; if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { [Console]::Write($d.SelectedPath) }"#
        }
    };
    let mut command = Command::new("powershell.exe");
    command
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-STA")
        .arg("-Command")
        .arg(script);
    command_output(&mut command).map_err(|error| format!("native file picker failed: {error}"))
}

#[cfg(target_os = "macos")]
fn native_picker(kind: NativePicker) -> Result<Option<String>, String> {
    let script = match kind {
        NativePicker::MediaFile => "POSIX path of (choose file with prompt \"Choose video\")",
        NativePicker::OutputDirectory => {
            "POSIX path of (choose folder with prompt \"Choose Scriptotar output folder\")"
        }
    };
    let mut command = Command::new("osascript");
    command.arg("-e").arg(script);
    command_output(&mut command).map_err(|error| format!("native file picker failed: {error}"))
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
fn native_picker(_kind: NativePicker) -> Result<Option<String>, String> {
    Err("native file picking is not available on this platform".to_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = env::var_os("SCRIPTOTAR_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or(app.path().app_data_dir()?);
            if !cfg!(debug_assertions) {
                let resource_dir = app.path().resource_dir()?;
                configure_packaged_runtime(&resource_dir, &data_dir);
            }

            let preparation = if let Some(status) = completed_status_if_marked(&data_dir) {
                if status.state == "completed" {
                    None
                } else {
                    None
                }
            } else {
                Some(migration::prepare_startup(&data_dir))
            };
            let operational_store = SqliteStore::open(data_dir.join("scriptotar.sqlite3"))?;
            operational_store.run_integration_migrations()?;
            operational_store.recover_interrupted_watchlist_refreshes()?;

            let services = AppServices::new(&data_dir)?;
            if matches!(preparation, Some(migration::Preparation::Ready)) {
                complete_prepared_migration(&services, &data_dir);
            }

            app.manage(OperationalState {
                data_dir: data_dir.clone(),
                store: operational_store,
            });
            app.manage(services);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend_health,
            bootstrap_app,
            list_jobs,
            get_watchlist_statuses,
            get_migration_status,
            retry_legacy_migration,
            select_legacy_migration_candidate,
            select_project,
            create_project,
            enqueue_local_media,
            enqueue_url,
            choose_local_media,
            choose_output_directory,
            cancel_job,
            retry_job,
            get_settings,
            save_settings,
            import_legacy_data,
            save_watchlist,
            scan_creator,
            queue_research,
            build_ai_prompt,
            run_ai,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Scriptotar desktop shell");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn migration_completion_marker_survives_restart_and_is_private() {
        let temp = TempDir::new().unwrap();
        assert!(!migration_completed(temp.path()).unwrap());
        mark_migration_completed(temp.path()).unwrap();
        assert!(migration_completed(temp.path()).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(temp.path().join(MIGRATION_COMPLETED_MARKER))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn migration_operation_rejects_a_second_concurrent_request() {
        let first = try_migration_operation().unwrap();
        assert!(matches!(
            try_migration_operation(),
            Err(MigrationLockError::Busy)
        ));
        drop(first);
        assert!(try_migration_operation().is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn migration_completion_marker_refuses_symlink() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        fs::write(&target, b"marker").unwrap();
        symlink(&target, temp.path().join(MIGRATION_COMPLETED_MARKER)).unwrap();
        assert!(migration_completed(temp.path()).is_err());
        assert!(mark_migration_completed(temp.path()).is_err());
    }
}
