mod dto;
mod services;

use std::{
    env,
    path::{Path, PathBuf},
};

use dto::{AiPromptInput, BootstrapData, ResearchQuery, UiJob, UiSettings};
use scriptotar_core::{Job, LegacyImportReport};
use services::AppServices;
use tauri::Manager;
use uuid::Uuid;

#[derive(Debug, serde::Serialize)]
struct BackendHealth {
    schema_version: u32,
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
    state
        .bootstrap()
        .map(|bootstrap| bootstrap.jobs)
        .map_err(command_error)
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
            let services = AppServices::new(data_dir)?;
            app.manage(services);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend_health,
            bootstrap_app,
            list_jobs,
            select_project,
            create_project,
            enqueue_local_media,
            enqueue_url,
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
