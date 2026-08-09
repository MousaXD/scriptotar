mod services;

use scriptotar_ai::ProviderConfig;
use scriptotar_core::{ApplicationSettings, Job, JobInput, Project};
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
fn create_project(
    name: String,
    state: tauri::State<'_, AppServices>,
) -> Result<Project, String> {
    state.create_project(name).map_err(command_error)
}

#[tauri::command]
fn list_projects(state: tauri::State<'_, AppServices>) -> Result<Vec<Project>, String> {
    state.list_projects().map_err(command_error)
}

#[tauri::command]
fn enqueue_job(
    project_id: Uuid,
    input: JobInput,
    state: tauri::State<'_, AppServices>,
) -> Result<Job, String> {
    state.enqueue_job(project_id, input).map_err(command_error)
}

#[tauri::command]
fn list_jobs(
    project_id: Option<Uuid>,
    state: tauri::State<'_, AppServices>,
) -> Result<Vec<Job>, String> {
    state.list_jobs(project_id).map_err(command_error)
}

#[tauri::command]
fn cancel_job(job_id: Uuid, state: tauri::State<'_, AppServices>) -> Result<Job, String> {
    state.cancel_job(job_id).map_err(command_error)
}

#[tauri::command]
fn retry_job(job_id: Uuid, state: tauri::State<'_, AppServices>) -> Result<Job, String> {
    state.retry_job(job_id).map_err(command_error)
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppServices>) -> Result<ApplicationSettings, String> {
    state.load_settings().map_err(command_error)
}

#[tauri::command]
fn save_settings(
    settings: ApplicationSettings,
    state: tauri::State<'_, AppServices>,
) -> Result<(), String> {
    state.save_settings(&settings).map_err(command_error)
}

#[tauri::command]
fn validate_research_url(
    url: String,
    state: tauri::State<'_, AppServices>,
) -> Result<String, String> {
    state.validate_research_url(&url)
}

#[tauri::command]
fn validate_ai_endpoint(
    config: ProviderConfig,
    state: tauri::State<'_, AppServices>,
) -> Result<String, String> {
    state.validate_ai_endpoint(&config)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let services = AppServices::new(data_dir.join("scriptotar.sqlite3"))?;
            app.manage(services);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend_health,
            create_project,
            list_projects,
            enqueue_job,
            list_jobs,
            cancel_job,
            retry_job,
            get_settings,
            save_settings,
            validate_research_url,
            validate_ai_endpoint,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Scriptotar desktop shell");
}
