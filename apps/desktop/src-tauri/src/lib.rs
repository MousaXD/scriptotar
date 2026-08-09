mod dto;
mod services;

use std::{io, process::Command};

use dto::{AiPromptInput, BootstrapData, ResearchQuery, UiSettings};
use scriptotar_core::{Job, LegacyImportReport};
use services::AppServices;
use tauri::Manager;
use uuid::Uuid;

#[derive(Debug, serde::Serialize)]
struct BackendHealth {
    schema_version: u32,
}

#[derive(Debug, Clone, Copy)]
enum NativePicker {
    MediaFile,
    OutputDirectory,
}

fn command_error(error: impl ToString) -> String {
    error.to_string()
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
    state.create_project(name).map_err(command_error)
}

#[tauri::command]
fn enqueue_local_media(
    project_id: Uuid,
    path: String,
    state: tauri::State<'_, AppServices>,
) -> Result<Job, String> {
    state
        .enqueue_local_media(project_id, path)
        .map_err(command_error)
}

#[tauri::command]
fn enqueue_url(
    project_id: Uuid,
    url: String,
    state: tauri::State<'_, AppServices>,
) -> Result<Job, String> {
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
fn retry_job(job_id: Uuid, state: tauri::State<'_, AppServices>) -> Result<Job, String> {
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
fn import_legacy_data(state: tauri::State<'_, AppServices>) -> Result<LegacyImportReport, String> {
    state.import_legacy_data().map_err(command_error)
}

#[tauri::command]
fn save_watchlist(
    query: ResearchQuery,
    state: tauri::State<'_, AppServices>,
) -> Result<BootstrapData, String> {
    state.save_watchlist(query)
}

#[tauri::command]
fn scan_creator(query: ResearchQuery, state: tauri::State<'_, AppServices>) -> Result<(), String> {
    state.scan_creator(query)
}

#[tauri::command]
fn queue_research(ids: Vec<String>, state: tauri::State<'_, AppServices>) -> Result<(), String> {
    state.queue_research(ids)
}

#[tauri::command]
fn build_ai_prompt(
    input: AiPromptInput,
    state: tauri::State<'_, AppServices>,
) -> Result<String, String> {
    state.build_ai_prompt(&input)
}

#[tauri::command]
fn run_ai(input: AiPromptInput, state: tauri::State<'_, AppServices>) -> Result<String, String> {
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
            let data_dir = app.path().app_data_dir()?;
            let services = AppServices::new(data_dir)?;
            app.manage(services);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend_health,
            bootstrap_app,
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
