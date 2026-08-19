from __future__ import annotations

from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    target = Path(path)
    text = target.read_text()
    if old not in text:
        raise RuntimeError(f"missing expected fragment for {label} in {path}")
    target.write_text(text.replace(old, new, 1))


def append_before(path: str, marker: str, addition: str, label: str) -> None:
    replace_once(path, marker, addition + marker, label)


# ---------------------------------------------------------------------------
# Durable lineage backfill: integration schema v3.
# ---------------------------------------------------------------------------
DB = "crates/scriptotar-db/src/integration.rs"
replace_once(
    DB,
    "pub const LATEST_INTEGRATION_SCHEMA_VERSION: u32 = 2;",
    "pub const LATEST_INTEGRATION_SCHEMA_VERSION: u32 = 3;",
    "integration schema version 3",
)

replace_once(
    DB,
    """            tx.commit().map_err(storage_error)?;\n        }\n        Ok(())\n    }\n\n    pub fn list_job_transcript_links(\n""",
    """            tx.commit().map_err(storage_error)?;\n        }\n        if current < 3 {\n            let tx = connection\n                .transaction_with_behavior(TransactionBehavior::Immediate)\n                .map_err(storage_error)?;\n            let linked_at = now_rfc3339();\n            tx.execute(\n                \"INSERT INTO job_transcript_links(job_id, transcript_id, linked_at)\n                 SELECT j.id, t.id, ?1\n                 FROM jobs j\n                 JOIN sources s\n                   ON s.project_id = j.project_id\n                  AND s.source_type = j.input_kind\n                  AND s.locator = j.input_value\n                 JOIN media m ON m.source_id = s.id\n                 JOIN transcripts t ON t.media_id = m.id\n                 WHERE j.state = 'completed'\n                   AND NOT EXISTS (\n                       SELECT 1 FROM job_transcript_links existing\n                       WHERE existing.job_id = j.id\n                   )\n                   AND NOT EXISTS (\n                       SELECT 1 FROM job_transcript_links existing\n                       WHERE existing.transcript_id = t.id\n                   )\n                   AND (\n                       SELECT COUNT(*) FROM jobs sibling\n                       WHERE sibling.project_id = j.project_id\n                         AND sibling.input_kind = j.input_kind\n                         AND sibling.input_value = j.input_value\n                         AND sibling.state = 'completed'\n                   ) = 1\n                   AND (\n                       SELECT COUNT(*)\n                       FROM transcripts candidate\n                       JOIN media candidate_media ON candidate_media.id = candidate.media_id\n                       JOIN sources candidate_source ON candidate_source.id = candidate_media.source_id\n                       WHERE candidate_source.project_id = j.project_id\n                         AND candidate_source.source_type = j.input_kind\n                         AND candidate_source.locator = j.input_value\n                   ) = 1\",\n                params![linked_at],\n            )\n            .map_err(storage_error)?;\n            tx.execute(\n                \"INSERT INTO integration_schema_migrations(version, name, applied_at)\n                 VALUES(3, 'safe_job_transcript_backfill', ?1)\",\n                params![now_rfc3339()],\n            )\n            .map_err(storage_error)?;\n            tx.commit().map_err(storage_error)?;\n        }\n        Ok(())\n    }\n\n    pub fn list_job_transcript_links(\n""",
    "safe historical lineage backfill",
)

replace_once(
    DB,
    """        assert_eq!(\n            store\n                .list_job_transcript_links(Some(project.id))\n                .unwrap()\n                .get(&job.id),\n            Some(&transcript.id)\n        );\n    }\n\n    #[test]\n    fn legacy_import_is_idempotent_and_creates_backup() {\n""",
    """        assert_eq!(\n            store\n                .list_job_transcript_links(Some(project.id))\n                .unwrap()\n                .get(&job.id),\n            Some(&transcript.id)\n        );\n\n        {\n            let connection = connection(&store).unwrap();\n            connection\n                .execute(\n                    \"DELETE FROM job_transcript_links WHERE job_id = ?1\",\n                    params![job.id.to_string()],\n                )\n                .unwrap();\n            connection\n                .execute(\n                    \"DELETE FROM integration_schema_migrations WHERE version = 3\",\n                    [],\n                )\n                .unwrap();\n        }\n        store.run_integration_migrations().unwrap();\n        assert_eq!(\n            store\n                .list_job_transcript_links(Some(project.id))\n                .unwrap()\n                .get(&job.id),\n            Some(&transcript.id)\n        );\n    }\n\n    #[test]\n    fn historical_lineage_backfill_skips_ambiguous_duplicate_inputs() {\n        let temp = TempDir::new().unwrap();\n        let store = new_store(&temp);\n        let project = Project::new(\"Inbox\");\n        store.create_project(&project).unwrap();\n        let input = \"/tmp/duplicate.mp4\";\n        let first_job = Uuid::new_v4();\n        let second_job = Uuid::new_v4();\n        let source_id = Uuid::new_v4();\n        let media_id = Uuid::new_v4();\n        let transcript_id = Uuid::new_v4();\n        let now = now_rfc3339();\n        let connection = connection(&store).unwrap();\n        for job_id in [first_job, second_job] {\n            connection\n                .execute(\n                    \"INSERT INTO jobs(\n                        id, project_id, input_kind, input_value, state, progress, last_error,\n                        created_at, updated_at\n                    ) VALUES(?1, ?2, 'local_file', ?3, 'completed', 1.0, NULL, ?4, ?4)\",\n                    params![job_id.to_string(), project.id.to_string(), input, now],\n                )\n                .unwrap();\n        }\n        connection\n            .execute(\n                \"INSERT INTO sources(id, project_id, creator_id, source_type, locator, title, created_at)\n                 VALUES(?1, ?2, NULL, 'local_file', ?3, 'duplicate', ?4)\",\n                params![source_id.to_string(), project.id.to_string(), input, now],\n            )\n            .unwrap();\n        connection\n            .execute(\n                \"INSERT INTO media(id, source_id, local_path, duration_seconds, mime_type, created_at)\n                 VALUES(?1, ?2, ?3, 1.0, NULL, ?4)\",\n                params![media_id.to_string(), source_id.to_string(), input, now],\n            )\n            .unwrap();\n        connection\n            .execute(\n                \"INSERT INTO transcripts(\n                    id, media_id, language, text, segments_json, words_json, created_at, updated_at\n                 ) VALUES(?1, ?2, 'en', 'duplicate', NULL, NULL, ?3, ?3)\",\n                params![transcript_id.to_string(), media_id.to_string(), now],\n            )\n            .unwrap();\n        connection\n            .execute(\n                \"DELETE FROM integration_schema_migrations WHERE version = 3\",\n                [],\n            )\n            .unwrap();\n        drop(connection);\n\n        store.run_integration_migrations().unwrap();\n        assert!(store\n            .list_job_transcript_links(Some(project.id))\n            .unwrap()\n            .is_empty());\n    }\n\n    #[test]\n    fn legacy_import_is_idempotent_and_creates_backup() {\n""",
    "lineage backfill regression tests",
)

# ---------------------------------------------------------------------------
# Orchestrator-native job change notifications.
# ---------------------------------------------------------------------------
ORCH = "crates/scriptotar-orchestrator/src/lib.rs"
replace_once(
    ORCH,
    """    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},\n""",
    """    sync::{\n        mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},\n        Arc,\n    },\n""",
    "orchestrator Arc import",
)

replace_once(
    ORCH,
    """#[derive(Debug, Clone)]\npub struct JobOrchestrator<R> {\n""",
    """pub type JobChangeNotifier = Arc<dyn Fn(Uuid) + Send + Sync + 'static>;\n\nfn notify_job(notifier: &JobChangeNotifier, job_id: Uuid) {\n    notifier(job_id);\n}\n\n#[derive(Debug, Clone)]\npub struct JobOrchestrator<R> {\n""",
    "job notifier alias",
)

replace_once(
    ORCH,
    """    pub fn start(repository: R, config: RuntimeConfig) -> Self {\n        let (tx, rx) = mpsc::channel();\n        let worker_repository = repository.clone();\n        thread::spawn(move || worker_loop(worker_repository, config, rx));\n        Self {\n            control: tx,\n            _repository: repository,\n        }\n    }\n""",
    """    pub fn start(repository: R, config: RuntimeConfig) -> Self {\n        Self::start_with_notifier(repository, config, Arc::new(|_| {}))\n    }\n\n    pub fn start_with_notifier(\n        repository: R,\n        config: RuntimeConfig,\n        notifier: JobChangeNotifier,\n    ) -> Self {\n        let (tx, rx) = mpsc::channel();\n        let worker_repository = repository.clone();\n        thread::spawn(move || worker_loop(worker_repository, config, rx, notifier));\n        Self {\n            control: tx,\n            _repository: repository,\n        }\n    }\n""",
    "orchestrator start with notifier",
)

replace_once(
    ORCH,
    """fn worker_loop<R>(repository: R, config: RuntimeConfig, controls: Receiver<Control>)\nwhere\n""",
    """fn worker_loop<R>(\n    repository: R,\n    config: RuntimeConfig,\n    controls: Receiver<Control>,\n    notifier: JobChangeNotifier,\n)\nwhere\n""",
    "worker loop notifier",
)
replace_once(
    ORCH,
    """                &mut host,\n                job_id,\n            ) {\n                record_runtime_error(&repository, job_id, &error);\n""",
    """                &mut host,\n                job_id,\n                &notifier,\n            ) {\n                record_runtime_error(&repository, job_id, &error, &notifier);\n""",
    "run job notifier wiring",
)
replace_once(
    ORCH,
    """            Ok(Control::Cancel(job_id)) => cancel_queued(&repository, &mut queue, job_id),\n""",
    """            Ok(Control::Cancel(job_id)) => {\n                cancel_queued(&repository, &mut queue, job_id, &notifier)\n            }\n""",
    "idle cancellation notifier",
)
replace_once(
    ORCH,
    """    host: &mut Option<SidecarHost>,\n    job_id: Uuid,\n) -> Result<(), OrchestratorError>\n""",
    """    host: &mut Option<SidecarHost>,\n    job_id: Uuid,\n    notifier: &JobChangeNotifier,\n) -> Result<(), OrchestratorError>\n""",
    "run_job notifier parameter",
)
replace_once(
    ORCH,
    """    let job = repository.transition_job(job_id, JobState::Preparing)?;\n    let settings = repository.load_settings()?;\n""",
    """    let job = repository.transition_job(job_id, JobState::Preparing)?;\n    notify_job(notifier, job_id);\n    let settings = repository.load_settings()?;\n""",
    "preparing notification",
)
replace_once(
    ORCH,
    """        drain_controls(repository, controls, queue, sidecar, job_id)?;\n\n        match sidecar.events.recv_timeout(Duration::from_millis(50)) {\n            Ok(ReaderEvent::Protocol(event)) => match handle_event(repository, &job, *event) {\n""",
    """        drain_controls(repository, controls, queue, sidecar, job_id, notifier)?;\n\n        match sidecar.events.recv_timeout(Duration::from_millis(50)) {\n            Ok(ReaderEvent::Protocol(event)) => {\n                match handle_event(repository, &job, *event, notifier) {\n""",
    "active notifier wiring start",
)
replace_once(
    ORCH,
    """                Err(error) => return Err(error),\n            },\n            Ok(ReaderEvent::Violation(error)) => {\n""",
    """                    Err(error) => return Err(error),\n                }\n            }\n            Ok(ReaderEvent::Violation(error)) => {\n""",
    "active notifier wiring end",
)
replace_once(
    ORCH,
    """                    if current.state.is_active() {\n                        repository.transition_job(job_id, JobState::Interrupted)?;\n                    }\n""",
    """                    if current.state.is_active() {\n                        repository.transition_job(job_id, JobState::Interrupted)?;\n                        notify_job(notifier, job_id);\n                    }\n""",
    "interrupted notification",
)
replace_once(
    ORCH,
    """    sidecar: &mut SidecarHost,\n    active_job_id: Uuid,\n) -> Result<(), OrchestratorError>\n""",
    """    sidecar: &mut SidecarHost,\n    active_job_id: Uuid,\n    notifier: &JobChangeNotifier,\n) -> Result<(), OrchestratorError>\n""",
    "drain controls notifier parameter",
)
replace_once(
    ORCH,
    """            Ok(Control::Cancel(job_id)) => cancel_queued(repository, queue, job_id),\n""",
    """            Ok(Control::Cancel(job_id)) => cancel_queued(repository, queue, job_id, notifier),\n""",
    "queued cancel notifier wiring",
)
replace_once(
    ORCH,
    """fn cancel_queued<R>(repository: &R, queue: &mut VecDeque<Uuid>, job_id: Uuid)\nwhere\n""",
    """fn cancel_queued<R>(\n    repository: &R,\n    queue: &mut VecDeque<Uuid>,\n    job_id: Uuid,\n    notifier: &JobChangeNotifier,\n)\nwhere\n""",
    "cancel queued notifier parameter",
)
replace_once(
    ORCH,
    """    {\n        let _ = repository.transition_job(job_id, JobState::Cancelled);\n    }\n}\n\nfn handle_event<R>(\n""",
    """    {\n        if repository\n            .transition_job(job_id, JobState::Cancelled)\n            .is_ok()\n        {\n            notify_job(notifier, job_id);\n        }\n    }\n}\n\nfn handle_event<R>(\n""",
    "cancel queued emission",
)
replace_once(
    ORCH,
    """    original_job: &Job,\n    event: SidecarEvent,\n) -> Result<bool, OrchestratorError>\n""",
    """    original_job: &Job,\n    event: SidecarEvent,\n    notifier: &JobChangeNotifier,\n) -> Result<bool, OrchestratorError>\n""",
    "handle event notifier parameter",
)
replace_once(
    ORCH,
    """            }\n            Ok(false)\n        }\n        SidecarEvent::Result { result, .. } => {\n            ensure_processing(repository, original_job)?;\n            persist_result(repository, original_job, result)?;\n            Ok(true)\n""",
    """            }\n            notify_job(notifier, original_job.id);\n            Ok(false)\n        }\n        SidecarEvent::Result { result, .. } => {\n            ensure_processing(repository, original_job)?;\n            persist_result(repository, original_job, result)?;\n            notify_job(notifier, original_job.id);\n            Ok(true)\n""",
    "progress and result emissions",
)
replace_once(
    ORCH,
    """            if current.state.is_active() {\n                repository.fail_job(original_job.id, &message)?;\n            }\n            Ok(true)\n        }\n        SidecarEvent::Cancelled { .. } => {\n            let current = repository.get_job(original_job.id)?;\n            if current.state.is_active() {\n                repository.transition_job(original_job.id, JobState::Cancelled)?;\n            }\n""",
    """            if current.state.is_active() {\n                repository.fail_job(original_job.id, &message)?;\n                notify_job(notifier, original_job.id);\n            }\n            Ok(true)\n        }\n        SidecarEvent::Cancelled { .. } => {\n            let current = repository.get_job(original_job.id)?;\n            if current.state.is_active() {\n                repository.transition_job(original_job.id, JobState::Cancelled)?;\n                notify_job(notifier, original_job.id);\n            }\n""",
    "failure and cancelled emissions",
)
replace_once(
    ORCH,
    """fn record_runtime_error<R>(repository: &R, job_id: Uuid, error: &OrchestratorError)\nwhere\n""",
    """fn record_runtime_error<R>(\n    repository: &R,\n    job_id: Uuid,\n    error: &OrchestratorError,\n    notifier: &JobChangeNotifier,\n)\nwhere\n""",
    "runtime error notifier parameter",
)
replace_once(
    ORCH,
    """        if job.state.is_active() {\n            let _ = repository.fail_job(job_id, &error.to_string());\n        }\n""",
    """        if job.state.is_active()\n            && repository.fail_job(job_id, &error.to_string()).is_ok()\n        {\n            notify_job(notifier, job_id);\n        }\n""",
    "runtime error emission",
)

append_before(
    ORCH,
    """    #[test]\n    fn local_transcription_persists_result_and_completes_job() {\n""",
    """    #[test]\n    fn job_change_notifier_reports_orchestrator_progress() {\n        let temp = TempDir::new().unwrap();\n        let store = SqliteStore::open(temp.path().join(\"next.sqlite3\")).unwrap();\n        store.run_integration_migrations().unwrap();\n        let project = Project::new(\"Inbox\");\n        store.create_project(&project).unwrap();\n        let mut settings = store.load_settings().unwrap();\n        settings.output_directory = Some(temp.path().join(\"output\").to_string_lossy().into_owned());\n        store.save_settings(&settings).unwrap();\n        let (sidecar, fake_worker) = paths();\n        let config = RuntimeConfig::new(\"python3\", sidecar, temp.path().join(\"output\"))\n            .with_environment(\n                \"SCRIPTOTAR_SIDECAR_ENGINE_WORKER\",\n                fake_worker.to_string_lossy().into_owned(),\n            );\n        let (notify_tx, notify_rx) = mpsc::channel();\n        let notifier: JobChangeNotifier = Arc::new(move |job_id| {\n            let _ = notify_tx.send(job_id);\n        });\n        let orchestrator = JobOrchestrator::start_with_notifier(store.clone(), config, notifier);\n        let job = enqueue_file(&store, &orchestrator, &temp.path().join(\"notify.mp4\"));\n        assert_eq!(wait_for_terminal(&store, job.id).state, JobState::Completed);\n\n        let mut notifications = 0;\n        while let Ok(job_id) = notify_rx.try_recv() {\n            assert_eq!(job_id, job.id);\n            notifications += 1;\n        }\n        assert!(notifications >= 2);\n    }\n\n""",
    "orchestrator notifier regression test",
)

# ---------------------------------------------------------------------------
# AppServices accepts a Tauri notifier without changing test/default callers.
# ---------------------------------------------------------------------------
SERVICES = "apps/desktop/src-tauri/src/services.rs"
replace_once(
    SERVICES,
    "use scriptotar_orchestrator::{JobOrchestrator, RuntimeConfig};",
    "use scriptotar_orchestrator::{JobChangeNotifier, JobOrchestrator, RuntimeConfig};",
    "services notifier import",
)
replace_once(
    SERVICES,
    """impl AppServices {\n    pub fn new(data_dir: impl AsRef<Path>) -> RepositoryResult<Self> {\n        let data_dir = data_dir.as_ref();\n""",
    """impl AppServices {\n    pub fn new(data_dir: impl AsRef<Path>) -> RepositoryResult<Self> {\n        Self::new_with_job_notifier(data_dir, Arc::new(|_| {}))\n    }\n\n    pub fn new_with_job_notifier(\n        data_dir: impl AsRef<Path>,\n        notifier: JobChangeNotifier,\n    ) -> RepositoryResult<Self> {\n        let data_dir = data_dir.as_ref();\n""",
    "services notifier constructor",
)
replace_once(
    SERVICES,
    """        let orchestrator = JobOrchestrator::start(\n            store.clone(),\n            runtime_config(data_dir.join(\"transcription-output\")),\n        );\n""",
    """        let orchestrator = JobOrchestrator::start_with_notifier(\n            store.clone(),\n            runtime_config(data_dir.join(\"transcription-output\")),\n            notifier,\n        );\n""",
    "services notifier orchestration",
)

# ---------------------------------------------------------------------------
# Tauri emits orchestrator-originated job change events.
# ---------------------------------------------------------------------------
TAURI_LIB = "apps/desktop/src-tauri/src/lib.rs"
replace_once(
    TAURI_LIB,
    """    sync::{Mutex, MutexGuard, OnceLock, TryLockError},\n""",
    """    sync::{Arc, Mutex, MutexGuard, OnceLock, TryLockError},\n""",
    "tauri Arc import",
)
replace_once(
    TAURI_LIB,
    "use tauri::Manager;",
    "use tauri::{Emitter, Manager};",
    "tauri Emitter import",
)
replace_once(
    TAURI_LIB,
    """            let services = AppServices::new(&data_dir)?;\n            if matches!(preparation, Some(migration::Preparation::Ready)) {\n""",
    """            let app_handle = app.handle().clone();\n            let job_notifier = Arc::new(move |job_id: Uuid| {\n                let _ = app_handle.emit(\"scriptotar://job-changed\", job_id.to_string());\n            });\n            let services = AppServices::new_with_job_notifier(&data_dir, job_notifier)?;\n            if matches!(preparation, Some(migration::Preparation::Ready)) {\n""",
    "tauri job event bridge",
)

# ---------------------------------------------------------------------------
# Frontend API subscription and event-driven refresh with slow reconciliation.
# ---------------------------------------------------------------------------
CLIENT = "apps/desktop-ui/src/api/client.ts"
replace_once(
    CLIENT,
    """  listJobs(): Promise<Job[]>;\n""",
    """  listJobs(): Promise<Job[]>;\n  subscribeJobChanges(listener: (jobId: string) => void): Promise<() => void>;\n""",
    "frontend job subscription contract",
)

MOCK = "apps/desktop-ui/src/api/mockClient.ts"
replace_once(
    MOCK,
    """    async listJobs() { return structuredClone(snapshot().jobs); },\n""",
    """    async listJobs() { return structuredClone(snapshot().jobs); },\n    async subscribeJobChanges(_listener: (jobId: string) => void) { return () => {}; },\n""",
    "mock job subscription",
)

TAURI_CLIENT = "apps/desktop-ui/src/api/tauriClient.ts"
replace_once(
    TAURI_CLIENT,
    """export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;\ntype CoreBootstrapData = Omit<BootstrapData, 'watchlistStatuses' | 'migrationStatus'>;\n""",
    """export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;\nexport type TauriListen = <T>(\n  event: string,\n  handler: (event: { payload: T }) => void\n) => Promise<() => void>;\ntype CoreBootstrapData = Omit<BootstrapData, 'watchlistStatuses' | 'migrationStatus'>;\n""",
    "tauri listen type",
)
replace_once(
    TAURI_CLIENT,
    "export function createTauriClient(invoke: TauriInvoke): ScriptotarApi {",
    "export function createTauriClient(invoke: TauriInvoke, listen?: TauriListen): ScriptotarApi {",
    "tauri client listen parameter",
)
replace_once(
    TAURI_CLIENT,
    """    listJobs: () => invoke<Job[]>('list_jobs'),\n""",
    """    listJobs: () => invoke<Job[]>('list_jobs'),\n    subscribeJobChanges: (listener) =>\n      listen\n        ? listen<string>('scriptotar://job-changed', (event) => listener(event.payload))\n        : Promise.resolve(() => {}),\n""",
    "tauri job subscription",
)

MAIN = "apps/desktop-ui/src/main.ts"
replace_once(
    MAIN,
    """type GlobalTauri = {\n  core?: {\n    invoke?: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;\n  };\n};\n""",
    """type GlobalTauri = {\n  core?: {\n    invoke?: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;\n  };\n  event?: {\n    listen?: <T>(event: string, handler: (event: { payload: T }) => void) => Promise<() => void>;\n  };\n};\n""",
    "global tauri event typing",
)
replace_once(
    MAIN,
    """if (globalTauri?.core?.invoke) {\n  setApi(createTauriClient(globalTauri.core.invoke));\n""",
    """if (globalTauri?.core?.invoke) {\n  const listen = globalTauri.event?.listen\n    ? <T>(event: string, handler: (event: { payload: T }) => void) =>\n        globalTauri.event!.listen!(event, handler)\n    : undefined;\n  setApi(createTauriClient(globalTauri.core.invoke, listen));\n""",
    "main tauri listen bridge",
)

APP = "apps/desktop-ui/src/App.svelte"
replace_once(
    APP,
    """  let jobRefreshInFlight = false;\n  let operationalRefreshInFlight = false;\n""",
    """  let jobRefreshInFlight = false;\n  let jobRefreshTimer: number | undefined;\n  let operationalRefreshInFlight = false;\n""",
    "job refresh debounce state",
)
replace_once(
    APP,
    """  async function refreshOperationalStatus() {\n""",
    """  function scheduleJobRefresh() {\n    if (jobRefreshTimer !== undefined) return;\n    jobRefreshTimer = window.setTimeout(() => {\n      jobRefreshTimer = undefined;\n      void refreshJobs();\n    }, 120);\n  }\n\n  async function refreshOperationalStatus() {\n""",
    "job refresh debounce function",
)
replace_once(
    APP,
    """  onMount(() => {\n    void load();\n    const jobPoll = window.setInterval(() => {\n      if (data?.jobs.some((job) => activeStates.has(job.state))) void refreshJobs();\n    }, 1000);\n    const operationalPoll = window.setInterval(() => void refreshOperationalStatus(), 15000);\n    return () => {\n      window.clearInterval(jobPoll);\n      window.clearInterval(operationalPoll);\n    };\n  });\n""",
    """  onMount(() => {\n    let disposed = false;\n    let unsubscribeJobs: (() => void) | undefined;\n    void load();\n    void api\n      .subscribeJobChanges(() => scheduleJobRefresh())\n      .then((unsubscribe) => {\n        if (disposed) unsubscribe();\n        else unsubscribeJobs = unsubscribe;\n      })\n      .catch(() => {\n        // Periodic reconciliation below remains the safe fallback.\n      });\n    const jobReconcile = window.setInterval(() => {\n      if (data?.jobs.some((job) => activeStates.has(job.state))) void refreshJobs();\n    }, 15000);\n    const operationalPoll = window.setInterval(() => void refreshOperationalStatus(), 15000);\n    return () => {\n      disposed = true;\n      unsubscribeJobs?.();\n      if (jobRefreshTimer !== undefined) window.clearTimeout(jobRefreshTimer);\n      window.clearInterval(jobReconcile);\n      window.clearInterval(operationalPoll);\n    };\n  });\n""",
    "replace one-second polling with event subscription",
)

TEST = "apps/desktop-ui/src/App.test.ts"
replace_once(
    TEST,
    """  it('refreshes active jobs without repeatedly bootstrapping the full workspace', async () => {\n    const api = createMockClient();\n    const bootstrap = vi.spyOn(api, 'bootstrap');\n    const listJobs = vi.spyOn(api, 'listJobs');\n    await ready(api);\n    await waitFor(() => expect(listJobs).toHaveBeenCalled(), { timeout: 1600 });\n    expect(bootstrap).toHaveBeenCalledTimes(1);\n  });\n""",
    """  it('refreshes active jobs from backend events without repeatedly bootstrapping the full workspace', async () => {\n    let notify: ((jobId: string) => void) | undefined;\n    const api = createMockClient({\n      subscribeJobChanges: async (listener) => {\n        notify = listener;\n        return () => {};\n      }\n    });\n    const bootstrap = vi.spyOn(api, 'bootstrap');\n    const listJobs = vi.spyOn(api, 'listJobs');\n    await ready(api);\n    notify?.('j-1');\n    await waitFor(() => expect(listJobs).toHaveBeenCalled());\n    expect(bootstrap).toHaveBeenCalledTimes(1);\n  });\n""",
    "event-driven frontend regression test",
)

print("technical futures phase two patch applied")
