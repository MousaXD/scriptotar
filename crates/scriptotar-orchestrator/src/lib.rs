use std::{
    collections::{HashMap, VecDeque},
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    thread,
    time::Duration,
};

use scriptotar_core::{
    now_rfc3339, ApplicationSettings, ContentRepository, Job, JobInput, JobRepository,
    JobRuntimeRepository, JobState, Media, RepositoryError, SettingsRepository, Source, SourceType,
    Transcript,
};
use scriptotar_media::{
    SidecarCommand, SidecarEvent, SidecarInput, SidecarInputKind, SidecarOptions, SidecarOutput,
    SidecarResult, SIDECAR_PROTOCOL_VERSION,
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("sidecar I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sidecar protocol error: {0}")]
    Protocol(String),
    #[error("sidecar is unavailable: {0}")]
    Unavailable(String),
    #[error("orchestrator control channel is closed")]
    Closed,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub python: PathBuf,
    pub sidecar_script: PathBuf,
    pub fallback_output_root: PathBuf,
    pub environment: HashMap<String, String>,
}

impl RuntimeConfig {
    pub fn new(
        python: impl Into<PathBuf>,
        sidecar_script: impl Into<PathBuf>,
        fallback_output_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            python: python.into(),
            sidecar_script: sidecar_script.into(),
            fallback_output_root: fallback_output_root.into(),
            environment: HashMap::new(),
        }
    }

    pub fn with_environment(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug)]
enum Control {
    Enqueue(Uuid),
    Cancel(Uuid),
}

#[derive(Debug)]
enum ReaderEvent {
    Protocol(SidecarEvent),
    Violation(String),
    Closed,
}

#[derive(Debug)]
struct SidecarHost {
    child: Child,
    stdin: ChildStdin,
    events: Receiver<ReaderEvent>,
}

impl SidecarHost {
    fn start(config: &RuntimeConfig) -> Result<Self, OrchestratorError> {
        if !config.sidecar_script.is_file() {
            return Err(OrchestratorError::Unavailable(format!(
                "sidecar script does not exist: {}",
                config.sidecar_script.display()
            )));
        }

        let mut command = Command::new(&config.python);
        command
            .arg(&config.sidecar_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&config.environment);
        if let Some(parent) = config.sidecar_script.parent() {
            command.current_dir(parent);
        }

        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| {
            OrchestratorError::Unavailable("sidecar stdin was not piped".to_owned())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            OrchestratorError::Unavailable("sidecar stdout was not piped".to_owned())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            OrchestratorError::Unavailable("sidecar stderr was not piped".to_owned())
        })?;

        let (tx, events) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for raw in reader.lines() {
                match raw {
                    Ok(line) if line.trim().is_empty() => {}
                    Ok(line) => match serde_json::from_str::<SidecarEvent>(&line) {
                        Ok(event) => {
                            if tx.send(ReaderEvent::Protocol(event)).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = tx.send(ReaderEvent::Violation(format!(
                                "{error}: {}",
                                line.chars().take(300).collect::<String>()
                            )));
                            return;
                        }
                    },
                    Err(error) => {
                        let _ = tx.send(ReaderEvent::Violation(error.to_string()));
                        return;
                    }
                }
            }
            let _ = tx.send(ReaderEvent::Closed);
        });

        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                eprintln!("[scriptotar-sidecar] {line}");
            }
        });

        let mut host = Self {
            child,
            stdin,
            events,
        };
        host.wait_ready()?;
        Ok(host)
    }

    fn wait_ready(&mut self) -> Result<(), OrchestratorError> {
        match self.events.recv_timeout(Duration::from_secs(10)) {
            Ok(ReaderEvent::Protocol(SidecarEvent::Ready {
                protocol,
                capabilities,
            })) => {
                if protocol != SIDECAR_PROTOCOL_VERSION
                    || !capabilities
                        .protocol_versions
                        .contains(&SIDECAR_PROTOCOL_VERSION)
                {
                    return Err(OrchestratorError::Protocol(format!(
                        "sidecar does not support protocol {}",
                        SIDECAR_PROTOCOL_VERSION
                    )));
                }
                Ok(())
            }
            Ok(ReaderEvent::Protocol(other)) => Err(OrchestratorError::Protocol(format!(
                "expected ready event, got {other:?}"
            ))),
            Ok(ReaderEvent::Violation(error)) => Err(OrchestratorError::Protocol(error)),
            Ok(ReaderEvent::Closed) => Err(OrchestratorError::Unavailable(
                "sidecar exited before ready".to_owned(),
            )),
            Err(RecvTimeoutError::Timeout) => Err(OrchestratorError::Unavailable(
                "sidecar did not become ready".to_owned(),
            )),
            Err(RecvTimeoutError::Disconnected) => Err(OrchestratorError::Unavailable(
                "sidecar event reader stopped".to_owned(),
            )),
        }
    }

    fn send(&mut self, command: &SidecarCommand) -> Result<(), OrchestratorError> {
        let encoded = serde_json::to_string(command)
            .map_err(|error| OrchestratorError::Protocol(error.to_string()))?;
        self.stdin.write_all(encoded.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()?;
        Ok(())
    }

    fn is_alive(&mut self) -> Result<bool, OrchestratorError> {
        Ok(self.child.try_wait()?.is_none())
    }

    fn shutdown(&mut self) {
        let _ = self.send(&SidecarCommand::Shutdown {
            protocol: SIDECAR_PROTOCOL_VERSION,
            request_id: None,
        });
        for _ in 0..20 {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => thread::sleep(Duration::from_millis(25)),
                Err(_) => break,
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for SidecarHost {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            self.shutdown();
        }
    }
}

#[derive(Debug, Clone)]
pub struct JobOrchestrator<R> {
    control: Sender<Control>,
    _repository: R,
}

impl<R> JobOrchestrator<R>
where
    R: JobRepository
        + JobRuntimeRepository
        + SettingsRepository
        + ContentRepository
        + Clone
        + Send
        + Sync
        + 'static,
{
    pub fn start(repository: R, config: RuntimeConfig) -> Self {
        let (tx, rx) = mpsc::channel();
        let worker_repository = repository.clone();
        thread::spawn(move || worker_loop(worker_repository, config, rx));
        Self {
            control: tx,
            _repository: repository,
        }
    }

    pub fn enqueue(&self, job_id: Uuid) -> Result<(), OrchestratorError> {
        self.control
            .send(Control::Enqueue(job_id))
            .map_err(|_| OrchestratorError::Closed)
    }

    pub fn cancel(&self, job_id: Uuid) -> Result<(), OrchestratorError> {
        self.control
            .send(Control::Cancel(job_id))
            .map_err(|_| OrchestratorError::Closed)
    }
}

fn worker_loop<R>(repository: R, config: RuntimeConfig, controls: Receiver<Control>)
where
    R: JobRepository + JobRuntimeRepository + SettingsRepository + ContentRepository + Clone,
{
    let mut queue = VecDeque::new();
    let mut host: Option<SidecarHost> = None;

    loop {
        if let Some(job_id) = queue.pop_front() {
            if let Err(error) = run_job(
                &repository,
                &config,
                &controls,
                &mut queue,
                &mut host,
                job_id,
            ) {
                record_runtime_error(&repository, job_id, &error);
            }
            continue;
        }

        match controls.recv() {
            Ok(Control::Enqueue(job_id)) => queue.push_back(job_id),
            Ok(Control::Cancel(job_id)) => cancel_queued(&repository, &mut queue, job_id),
            Err(_) => break,
        }
    }

    if let Some(mut host) = host {
        host.shutdown();
    }
}

fn run_job<R>(
    repository: &R,
    config: &RuntimeConfig,
    controls: &Receiver<Control>,
    queue: &mut VecDeque<Uuid>,
    host: &mut Option<SidecarHost>,
    job_id: Uuid,
) -> Result<(), OrchestratorError>
where
    R: JobRepository + JobRuntimeRepository + SettingsRepository + ContentRepository + Clone,
{
    let job = repository.get_job(job_id)?;
    if job.state != JobState::Queued {
        return Ok(());
    }
    let job = repository.transition_job(job_id, JobState::Preparing)?;
    let settings = repository.load_settings()?;
    let output_root = settings
        .output_directory
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| config.fallback_output_root.clone());
    fs::create_dir_all(&output_root)?;

    let should_restart = match host.as_mut() {
        Some(existing) => !existing.is_alive()?,
        None => true,
    };
    if should_restart {
        *host = Some(SidecarHost::start(config)?);
    }
    let sidecar = host
        .as_mut()
        .ok_or_else(|| OrchestratorError::Unavailable("sidecar did not start".to_owned()))?;

    sidecar.send(&transcribe_command(&job, &settings, &output_root))?;

    loop {
        drain_controls(repository, controls, queue, sidecar, job_id)?;

        match sidecar.events.recv_timeout(Duration::from_millis(50)) {
            Ok(ReaderEvent::Protocol(event)) => {
                if handle_event(repository, &job, event)? {
                    return Ok(());
                }
            }
            Ok(ReaderEvent::Violation(error)) => {
                sidecar.shutdown();
                return Err(OrchestratorError::Protocol(error));
            }
            Ok(ReaderEvent::Closed) => {
                let current = repository.get_job(job_id)?;
                if current.state.is_active() {
                    repository.transition_job(job_id, JobState::Interrupted)?;
                }
                return Ok(());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                sidecar.shutdown();
                return Err(OrchestratorError::Unavailable(
                    "sidecar event stream disconnected".to_owned(),
                ));
            }
        }
    }
}

fn drain_controls<R>(
    repository: &R,
    controls: &Receiver<Control>,
    queue: &mut VecDeque<Uuid>,
    sidecar: &mut SidecarHost,
    active_job_id: Uuid,
) -> Result<(), OrchestratorError>
where
    R: JobRepository + JobRuntimeRepository + SettingsRepository + ContentRepository + Clone,
{
    loop {
        match controls.try_recv() {
            Ok(Control::Enqueue(job_id)) => {
                if job_id != active_job_id && !queue.contains(&job_id) {
                    queue.push_back(job_id);
                }
            }
            Ok(Control::Cancel(job_id)) if job_id == active_job_id => {
                sidecar.send(&SidecarCommand::Cancel {
                    protocol: SIDECAR_PROTOCOL_VERSION,
                    request_id: None,
                    job_id: job_id.to_string(),
                })?;
            }
            Ok(Control::Cancel(job_id)) => cancel_queued(repository, queue, job_id),
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return Ok(()),
        }
    }
}

fn cancel_queued<R>(repository: &R, queue: &mut VecDeque<Uuid>, job_id: Uuid)
where
    R: JobRepository,
{
    queue.retain(|queued| *queued != job_id);
    if repository
        .get_job(job_id)
        .is_ok_and(|job| job.state == JobState::Queued)
    {
        let _ = repository.transition_job(job_id, JobState::Cancelled);
    }
}

fn handle_event<R>(
    repository: &R,
    original_job: &Job,
    event: SidecarEvent,
) -> Result<bool, OrchestratorError>
where
    R: JobRepository + JobRuntimeRepository + ContentRepository,
{
    match event {
        SidecarEvent::Ready { .. }
        | SidecarEvent::Pong { .. }
        | SidecarEvent::Accepted { .. }
        | SidecarEvent::JobStarted { .. }
        | SidecarEvent::Shutdown { .. } => Ok(false),
        SidecarEvent::Progress {
            job_id,
            stage,
            percent,
            ..
        } if event_job_matches(original_job.id, &job_id) => {
            enter_stage(repository, original_job, &stage)?;
            if let Some(percent) = percent {
                repository.update_job_progress(
                    original_job.id,
                    Some((percent / 100.0).clamp(0.0, 1.0)),
                )?;
            }
            Ok(false)
        }
        SidecarEvent::Result { job_id, result } if event_job_matches(original_job.id, &job_id) => {
            ensure_processing(repository, original_job)?;
            persist_result(repository, original_job, result)?;
            Ok(true)
        }
        SidecarEvent::Error { job_id, error, .. }
            if job_id
                .as_deref()
                .is_none_or(|job_id| event_job_matches(original_job.id, job_id)) =>
        {
            let message = format!("{}: {}", error.code, error.message);
            let current = repository.get_job(original_job.id)?;
            if current.state.is_active() {
                repository.fail_job(original_job.id, &message)?;
            }
            Ok(true)
        }
        SidecarEvent::Cancelled { job_id, .. } if event_job_matches(original_job.id, &job_id) => {
            let current = repository.get_job(original_job.id)?;
            if current.state.is_active() {
                repository.transition_job(original_job.id, JobState::Cancelled)?;
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn event_job_matches(job_id: Uuid, sidecar_job_id: &str) -> bool {
    sidecar_job_id == job_id.to_string()
}

fn enter_stage<R>(repository: &R, job: &Job, stage: &str) -> Result<(), OrchestratorError>
where
    R: JobRepository,
{
    match stage {
        "downloading" => transition_if_needed(repository, job, JobState::Downloading)?,
        "transcribing" => transition_if_needed(repository, job, JobState::Transcribing)?,
        "processing" => transition_if_needed(repository, job, JobState::Processing)?,
        _ => {}
    }
    Ok(())
}

fn transition_if_needed<R>(
    repository: &R,
    job: &Job,
    target: JobState,
) -> Result<(), OrchestratorError>
where
    R: JobRepository,
{
    loop {
        let current = repository.get_job(job.id)?;
        if current.state == target {
            return Ok(());
        }
        let next = match (current.state, target) {
            (JobState::Preparing, JobState::Downloading) => JobState::Downloading,
            (JobState::Preparing, JobState::Transcribing) => JobState::Transcribing,
            (JobState::Preparing, JobState::Processing) => match job.input {
                JobInput::Url(_) => JobState::Downloading,
                JobInput::LocalFile(_) => JobState::Transcribing,
            },
            (JobState::Downloading, JobState::Transcribing | JobState::Processing) => {
                JobState::Transcribing
            }
            (JobState::Transcribing, JobState::Processing) => JobState::Processing,
            _ => return Ok(()),
        };
        repository.transition_job(job.id, next)?;
    }
}

fn ensure_processing<R>(repository: &R, job: &Job) -> Result<(), OrchestratorError>
where
    R: JobRepository,
{
    transition_if_needed(repository, job, JobState::Processing)
}

fn persist_result<R>(
    repository: &R,
    job: &Job,
    result: SidecarResult,
) -> Result<(), OrchestratorError>
where
    R: JobRepository + ContentRepository,
{
    let now = now_rfc3339();
    let source_type = match job.input {
        JobInput::Url(_) => SourceType::Url,
        JobInput::LocalFile(_) => SourceType::LocalFile,
    };
    let locator = match &job.input {
        JobInput::Url(value) | JobInput::LocalFile(value) => value.clone(),
    };
    let source = Source {
        id: Uuid::new_v4(),
        project_id: job.project_id,
        creator_id: None,
        source_type,
        locator: result.source.source_url.unwrap_or(locator.clone()),
        title: result.source.title,
        created_at: now.clone(),
    };
    let media_path = result.artifacts.media.unwrap_or_else(|| match &job.input {
        JobInput::LocalFile(value) => value.clone(),
        JobInput::Url(_) => result.output_dir.clone(),
    });
    let media = Media {
        id: Uuid::new_v4(),
        source_id: source.id,
        local_path: media_path,
        duration_seconds: result
            .transcript
            .duration_seconds
            .or(result.source.duration_seconds),
        mime_type: None,
        created_at: now.clone(),
    };
    let transcript = Transcript {
        id: Uuid::new_v4(),
        media_id: media.id,
        language: result.transcript.language,
        text: if result.transcript.clean_text.trim().is_empty() {
            result.transcript.text
        } else {
            result.transcript.clean_text
        },
        segments_json: Some(
            serde_json::to_string(&result.transcript.segments)
                .map_err(|error| OrchestratorError::Protocol(error.to_string()))?,
        ),
        words_json: Some(
            serde_json::to_string(&result.transcript.words)
                .map_err(|error| OrchestratorError::Protocol(error.to_string()))?,
        ),
        created_at: now.clone(),
        updated_at: now,
    };
    repository.persist_transcription(job.id, &source, &media, &transcript)?;
    Ok(())
}

fn transcribe_command(
    job: &Job,
    settings: &ApplicationSettings,
    output_root: &Path,
) -> SidecarCommand {
    let (kind, value) = match &job.input {
        JobInput::Url(value) => (SidecarInputKind::Url, value.clone()),
        JobInput::LocalFile(value) => (SidecarInputKind::File, value.clone()),
    };
    SidecarCommand::Transcribe {
        protocol: SIDECAR_PROTOCOL_VERSION,
        request_id: None,
        job_id: job.id.to_string(),
        input: SidecarInput { kind, value },
        output: SidecarOutput {
            root: output_root.to_string_lossy().into_owned(),
        },
        options: SidecarOptions {
            model: settings.transcription_model.clone(),
            device: settings.transcription_device.clone(),
            language: settings.language.clone(),
            quality: settings.download_quality.clone(),
            cookies_browser: settings
                .cookie_browser
                .clone()
                .unwrap_or_else(|| "none".to_owned()),
            max_duration_seconds: settings.max_duration_seconds,
            copy_source: settings.copy_source,
            translate: settings.translate,
            batched: settings.batched,
            batch_size: 8,
            keep_failed: settings.keep_failed_artifacts,
        },
    }
}

fn record_runtime_error<R>(repository: &R, job_id: Uuid, error: &OrchestratorError)
where
    R: JobRepository + JobRuntimeRepository,
{
    if let Ok(job) = repository.get_job(job_id) {
        if job.state.is_active() {
            let _ = repository.fail_job(job_id, &error.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriptotar_core::{Project, ProjectRepository};
    use scriptotar_db::SqliteStore;
    use tempfile::TempDir;

    fn paths() -> (PathBuf, PathBuf) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        (
            root.join("sidecars/transcription/sidecar.py"),
            root.join("sidecars/transcription/tests/fake_engine_worker.py"),
        )
    }

    fn setup() -> (TempDir, SqliteStore, JobOrchestrator<SqliteStore>) {
        let temp = TempDir::new().unwrap();
        let store = SqliteStore::open(temp.path().join("next.sqlite3")).unwrap();
        store.run_integration_migrations().unwrap();
        let project = Project::new("Inbox");
        store.create_project(&project).unwrap();
        let mut settings = store.load_settings().unwrap();
        settings.output_directory = Some(temp.path().join("output").to_string_lossy().into_owned());
        store.save_settings(&settings).unwrap();
        let (sidecar, fake_worker) = paths();
        let config = RuntimeConfig::new("python3", sidecar, temp.path().join("output"))
            .with_environment(
                "SCRIPTOTAR_SIDECAR_ENGINE_WORKER",
                fake_worker.to_string_lossy().into_owned(),
            );
        let orchestrator = JobOrchestrator::start(store.clone(), config);
        (temp, store, orchestrator)
    }

    fn enqueue_file(
        store: &SqliteStore,
        orchestrator: &JobOrchestrator<SqliteStore>,
        path: &Path,
    ) -> Job {
        fs::write(path, b"fixture").unwrap();
        let project_id = store.list_projects().unwrap()[0].id;
        let job = Job::new(
            project_id,
            JobInput::LocalFile(path.to_string_lossy().into_owned()),
        );
        store.insert_job(&job).unwrap();
        orchestrator.enqueue(job.id).unwrap();
        job
    }

    fn wait_for_terminal(store: &SqliteStore, job_id: Uuid) -> Job {
        for _ in 0..300 {
            let job = store.get_job(job_id).unwrap();
            if matches!(
                job.state,
                JobState::Completed
                    | JobState::Failed
                    | JobState::Cancelled
                    | JobState::Interrupted
            ) {
                return job;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("job did not become terminal");
    }

    #[test]
    fn local_transcription_persists_result_and_completes_job() {
        let (temp, store, orchestrator) = setup();
        let job = enqueue_file(&store, &orchestrator, &temp.path().join("normal.mp4"));
        assert_eq!(wait_for_terminal(&store, job.id).state, JobState::Completed);
        let transcripts = store.list_transcripts(Some(job.project_id)).unwrap();
        assert_eq!(transcripts.len(), 1);
        assert_eq!(transcripts[0].transcript.text, "hello world");
    }

    #[test]
    fn failed_job_does_not_stall_the_next_job() {
        let (temp, store, orchestrator) = setup();
        let failed = enqueue_file(&store, &orchestrator, &temp.path().join("fail.mp4"));
        let next = enqueue_file(&store, &orchestrator, &temp.path().join("normal.mp4"));
        assert_eq!(wait_for_terminal(&store, failed.id).state, JobState::Failed);
        assert_eq!(
            wait_for_terminal(&store, next.id).state,
            JobState::Completed
        );
    }

    #[test]
    fn engine_crash_does_not_stall_the_next_job() {
        let (temp, store, orchestrator) = setup();
        let crashed = enqueue_file(&store, &orchestrator, &temp.path().join("crash.mp4"));
        let next = enqueue_file(&store, &orchestrator, &temp.path().join("normal.mp4"));
        assert_eq!(
            wait_for_terminal(&store, crashed.id).state,
            JobState::Failed
        );
        assert_eq!(
            wait_for_terminal(&store, next.id).state,
            JobState::Completed
        );
    }

    #[test]
    fn cancellation_keeps_queue_healthy() {
        let (temp, store, orchestrator) = setup();
        let blocked = enqueue_file(&store, &orchestrator, &temp.path().join("spawn-child.mp4"));
        for _ in 0..200 {
            let job = store.get_job(blocked.id).unwrap();
            if job.state == JobState::Transcribing {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        orchestrator.cancel(blocked.id).unwrap();
        let next = enqueue_file(&store, &orchestrator, &temp.path().join("normal.mp4"));
        assert_eq!(
            wait_for_terminal(&store, blocked.id).state,
            JobState::Cancelled
        );
        assert_eq!(
            wait_for_terminal(&store, next.id).state,
            JobState::Completed
        );
    }
}
