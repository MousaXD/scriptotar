mod dto;
mod security;
mod services;

#[cfg(test)]
extern crate self as tempfile;

#[cfg(test)]
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(test)]
static NEXT_TEST_DIR_ID: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
pub struct TempDir {
    path: PathBuf,
}

#[cfg(test)]
impl TempDir {
    pub fn new() -> io::Result<Self> {
        let root = std::env::temp_dir();
        for _ in 0..1024 {
            let id = NEXT_TEST_DIR_ID.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(
                "scriptotar-security-test-{}-{id}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not reserve a unique test directory",
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

use dto::{AiPromptInput, BootstrapData, ResearchQuery, UiSettings};
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
    security::validate_project_name(&name)?;
    state.create_project(name).map_err(command_error)
}

#[tauri::command]
fn enqueue_local_media(
    project_id: Uuid,
    path: String,
    state: tauri::State<'_, AppServices>,
) -> Result<Job, String> {
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
) -> Result<Job, String> {
    security::validate_url_argument("media URL", &url)?;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            match security::prepare_legacy_import_bridge(&data_dir) {
                Ok(Some(message)) => eprintln!("[scriptotar-migration] {message}"),
                Ok(None) => {}
                Err(error) => eprintln!("[scriptotar-migration] {error}"),
            }
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
