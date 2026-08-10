use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use tauri::{
    ipc::{CallbackFn, InvokeBody},
    test::{get_ipc_response, mock_builder, mock_context, noop_assets, MockRuntime, INVOKE_KEY},
    webview::InvokeRequest,
    App, WebviewWindow, WebviewWindowBuilder,
};
use tempfile::TempDir;

use crate::{
    __cmd__bootstrap_app, __cmd__create_project, __cmd__enqueue_local_media, __cmd__list_jobs,
    __cmd__select_project, __tauri_command_name_bootstrap_app,
    __tauri_command_name_create_project, __tauri_command_name_enqueue_local_media,
    __tauri_command_name_list_jobs, __tauri_command_name_select_project,
};
use super::AppServices;

const EXPECTED_TRANSCRIPT: &str = "installed fixture transcript survives restart";
const E2E_TIMEOUT: Duration = Duration::from_secs(15);

struct EnvGuard {
    previous: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn set(values: &[(&'static str, &Path)]) -> Self {
        let previous = values
            .iter()
            .map(|(key, _)| (*key, env::var_os(key)))
            .collect::<Vec<_>>();
        for (key, value) in values {
            env::set_var(key, value);
        }
        Self { previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.previous.drain(..).rev() {
            if let Some(value) = value {
                env::set_var(key, value);
            } else {
                env::remove_var(key);
            }
        }
    }
}

fn executable_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_owned()
    }
}

#[cfg(unix)]
fn create_fixture_engine_wrapper(temp: &TempDir) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let wrapper = temp.path().join("scriptotar-installed-e2e-engine");
    fs::write(
        &wrapper,
        "#!/bin/sh\nset -eu\nexec python3 \"$SCRIPTOTAR_E2E_FIXTURE_SCRIPT\"\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&wrapper).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&wrapper, permissions).unwrap();
    wrapper
}

#[cfg(not(unix))]
fn create_fixture_engine_wrapper(_temp: &TempDir) -> PathBuf {
    panic!("the installed-app E2E fixture wrapper currently supports Unix CI only")
}

fn fixture_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../sidecars/transcription/tests/installed_app_fixture_engine.py")
        .canonicalize()
        .expect("installed-app fixture engine script must exist")
}

fn packaged_runtime() -> PathBuf {
    env::var_os("SCRIPTOTAR_E2E_RUNTIME_DIR")
        .map(PathBuf::from)
        .expect("SCRIPTOTAR_E2E_RUNTIME_DIR must point at the installed transcription-runtime")
}

fn build_app(data_dir: &Path) -> App<MockRuntime> {
    let services = AppServices::new(data_dir).expect("create Scriptotar services");
    mock_builder()
        .manage(services)
        .invoke_handler(tauri::generate_handler![
            bootstrap_app,
            list_jobs,
            select_project,
            create_project,
            enqueue_local_media,
        ])
        .build(mock_context(noop_assets()))
        .expect("build mock-runtime Scriptotar app")
}

fn build_webview(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, label, Default::default())
        .build()
        .expect("build mock-runtime webview")
}

fn ipc(webview: &WebviewWindow<MockRuntime>, command: &str, args: Value) -> Value {
    let response = get_ipc_response(
        webview,
        InvokeRequest {
            cmd: command.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_owned(),
        },
    )
    .unwrap_or_else(|error| panic!("IPC command {command} failed: {error:?}"));
    response
        .deserialize::<Value>()
        .unwrap_or_else(|error| panic!("IPC response for {command} was not JSON: {error}"))
}

fn completed_job(bootstrap: &Value, job_id: &str) -> bool {
    bootstrap["jobs"]
        .as_array()
        .expect("jobs array")
        .iter()
        .any(|job| job["id"] == job_id && job["state"] == "completed")
}

fn assert_completed_state(bootstrap: &Value, project_id: &str, job_id: &str) {
    assert_eq!(bootstrap["activeProjectId"], project_id);
    assert!(
        completed_job(bootstrap, job_id),
        "completed job missing: {bootstrap:#}"
    );

    let transcripts = bootstrap["transcripts"]
        .as_array()
        .expect("transcripts array");
    let transcript = transcripts
        .iter()
        .find(|item| item["text"] == EXPECTED_TRANSCRIPT)
        .unwrap_or_else(|| panic!("fixture transcript missing: {bootstrap:#}"));
    assert_eq!(transcript["projectId"], project_id);
    assert_eq!(transcript["title"], "Installed E2E Fixture");
    assert_eq!(transcript["language"], "en");
    assert_eq!(transcript["segments"][0]["text"], EXPECTED_TRANSCRIPT);

    let library = bootstrap["library"].as_array().expect("library array");
    assert!(
        library.iter().any(|item| {
            item["kind"] == "Transcript"
                && item["projectId"] == project_id
                && item["title"] == "Installed E2E Fixture"
        }),
        "transcript library item missing: {bootstrap:#}"
    );

    assert!(
        bootstrap["projects"]
            .as_array()
            .expect("projects array")
            .iter()
            .any(|project| project["id"] == project_id && project["name"] == "Installed E2E"),
        "created project missing: {bootstrap:#}"
    );
}

#[test]
#[ignore = "run from Linux packaging CI with SCRIPTOTAR_E2E_RUNTIME_DIR set to the installed runtime"]
fn installed_app_e2e_round_trip_survives_restart() {
    let runtime = packaged_runtime();
    let supervisor = runtime.join(executable_name("scriptotar-transcription"));
    let sidecar_script = runtime.join("sidecar.py");
    assert!(
        supervisor.is_file(),
        "missing installed supervisor: {}",
        supervisor.display()
    );
    assert!(
        sidecar_script.is_file(),
        "missing installed sidecar: {}",
        sidecar_script.display()
    );

    let temp = TempDir::new().expect("create E2E temp directory");
    let fixture_engine = create_fixture_engine_wrapper(&temp);
    let fixture_script = fixture_script();
    let _env = EnvGuard::set(&[
        ("SCRIPTOTAR_SIDECAR_PYTHON", &supervisor),
        ("SCRIPTOTAR_SIDECAR_SCRIPT", &sidecar_script),
        ("SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE", &fixture_engine),
        ("SCRIPTOTAR_E2E_FIXTURE_SCRIPT", &fixture_script),
    ]);

    let data_dir = temp.path().join("app-data");
    assert!(
        !data_dir.exists(),
        "E2E must begin with a clean app-data directory"
    );
    let media = temp.path().join("fixture.mp4");
    fs::write(&media, b"deterministic local media fixture").expect("write media fixture");

    let app = build_app(&data_dir);
    let webview = build_webview(&app, "installed-e2e-first-run");

    let initial = ipc(&webview, "bootstrap_app", json!({}));
    assert_eq!(initial["transcripts"].as_array().unwrap().len(), 0);
    assert_eq!(initial["jobs"].as_array().unwrap().len(), 0);

    let created = ipc(&webview, "create_project", json!({"name": "Installed E2E"}));
    let project_id = created["activeProjectId"]
        .as_str()
        .expect("created project ID")
        .to_owned();

    let queued = ipc(
        &webview,
        "enqueue_local_media",
        json!({
            "projectId": project_id,
            "path": media.to_string_lossy(),
        }),
    );
    let job_id = queued["id"].as_str().expect("queued job ID").to_owned();

    let deadline = Instant::now() + E2E_TIMEOUT;
    let completed = loop {
        let jobs = ipc(&webview, "list_jobs", json!({}));
        let job = jobs
            .as_array()
            .expect("job list response")
            .iter()
            .find(|job| job["id"] == job_id)
            .cloned()
            .unwrap_or_else(|| panic!("queued job {job_id} disappeared: {jobs:#}"));
        match job["state"].as_str().unwrap_or_default() {
            "completed" => break ipc(&webview, "bootstrap_app", json!({})),
            "failed" | "cancelled" | "interrupted" => {
                panic!("job became terminal without completing: {job:#}")
            }
            _ if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            _ => panic!("job {job_id} did not complete within {E2E_TIMEOUT:?}: {job:#}"),
        }
    };

    assert_completed_state(&completed, &project_id, &job_id);
    let artifact = data_dir
        .join("transcription-output")
        .join(format!("installed-e2e-{job_id}"))
        .join("transcript.txt");
    assert_eq!(
        fs::read_to_string(&artifact).expect("fixture transcript artifact"),
        format!("{EXPECTED_TRANSCRIPT}\n")
    );

    drop(webview);
    drop(app);

    let restarted_app = build_app(&data_dir);
    let restarted_webview = build_webview(&restarted_app, "installed-e2e-restart");
    let restored = ipc(
        &restarted_webview,
        "select_project",
        json!({"projectId": project_id}),
    );
    assert_completed_state(&restored, &project_id, &job_id);
}
