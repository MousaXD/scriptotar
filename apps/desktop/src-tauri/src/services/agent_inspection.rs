//! First production agent tool: a consented, locally read-only inspection of
//! one transcript already linked to the selected Content Study. No internet,
//! AI provider or job mutation can be reached through this tool registry.
use super::AppServices;
use scriptotar_agent::{
    Certainty, Engine, Estimate, ExecutionContext, Limits, Plan, PolicyGate, ProbeResult,
    RunSnapshot, RuntimeError, StepInput, StepOutput, StepPlan, ToolExecutor, ToolFailure,
    ToolKind, ToolReceipt, ToolRegistry,
};
#[cfg(test)]
use scriptotar_core::ProjectRepository;
use scriptotar_core::{RepositoryError, RepositoryResult, StudySourceKind};
use scriptotar_db::{EffectClaim, EffectStatus, EffectUpdate, ScopedAgentCheckpointStore};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const MAX_LOCAL_TRANSCRIPT_BYTES: u64 = 250_000;
const LOCAL_AGENT_DEADLINE_MS: u64 = 300_000;

fn current_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn unavailable() -> RepositoryError {
    RepositoryError::Storage("agent checkpoint is unavailable".into())
}

fn run_error(error: RuntimeError) -> RepositoryError {
    match error {
        RuntimeError::Store(scriptotar_agent::StoreError::NotFound) | RuntimeError::NotFound => {
            RepositoryError::NotFound("agent run".into())
        }
        RuntimeError::Store(scriptotar_agent::StoreError::Conflict) => {
            RepositoryError::Conflict("agent run changed during another operation".into())
        }
        RuntimeError::Store(scriptotar_agent::StoreError::Exists) => {
            RepositoryError::Conflict("agent run already exists".into())
        }
        RuntimeError::Store(scriptotar_agent::StoreError::Unavailable) => unavailable(),
        RuntimeError::PolicyDenied => RepositoryError::NotFound("source in active study".into()),
        RuntimeError::WrongState | RuntimeError::StepMismatch => {
            RepositoryError::Conflict("agent run requires a different action".into())
        }
        _ => RepositoryError::Validation("invalid or unsupported agent run".into()),
    }
}

fn transcript_id(input: &StepInput) -> Result<Uuid, RuntimeError> {
    let StepInput::ReadStudyTranscript { transcript_id } = input else {
        return Err(RuntimeError::PolicyDenied);
    };
    Uuid::parse_str(transcript_id).map_err(|_| RuntimeError::PolicyDenied)
}

fn checked_source(
    services: &AppServices,
    project: Uuid,
    study: Uuid,
    id: Uuid,
) -> RepositoryResult<u64> {
    services.require_active_project(project)?;
    services.store.get_study(project, study)?;
    let attached = services
        .store
        .list_study_sources(project, study)?
        .iter()
        .any(|entry| {
            entry.available
                && entry.kind == StudySourceKind::Transcript
                && entry.reference_id == Some(id)
        });
    if !attached {
        return Err(RepositoryError::NotFound(
            "transcript not attached to study".into(),
        ));
    }
    let bundle = services.store.get_transcript_bundle(id)?;
    if bundle.project_id != project {
        return Err(RepositoryError::NotFound(
            "transcript in active project".into(),
        ));
    }
    let bytes = bundle.transcript.text.len() as u64;
    if bytes > MAX_LOCAL_TRANSCRIPT_BYTES {
        return Err(RepositoryError::Validation(
            "local transcript exceeds inspection limit".into(),
        ));
    }
    services.require_active_project(project)?;
    Ok(bytes)
}

#[derive(Clone)]
struct LocalStudyGate {
    services: AppServices,
    project: Uuid,
    study: Uuid,
}
impl PolicyGate for LocalStudyGate {
    fn authorize(&self, plan: &Plan, step: &StepPlan) -> Result<(), RuntimeError> {
        if plan.project_id != self.project.to_string()
            || plan.study_id != self.study.to_string()
            || step.estimate.tokens != 0
            || step.estimate.cost_microusd != 0
            || !step.requires_approval
        {
            return Err(RuntimeError::PolicyDenied);
        }
        let transcript = transcript_id(&step.input)?;
        checked_source(&self.services, self.project, self.study, transcript)
            .map_err(|_| RuntimeError::PolicyDenied)?;
        Ok(())
    }
}

struct LocalStudyDriver {
    gate: LocalStudyGate,
}
impl LocalStudyDriver {
    fn inspect(
        &self,
        ctx: &ExecutionContext,
        input: &StepInput,
    ) -> Result<StepOutput, ToolFailure> {
        let fail = || ToolFailure {
            code: "not_found".into(),
            retryable: false,
            certainty: Certainty::NotApplied,
        };
        if ctx.project_id != self.gate.project.to_string()
            || ctx.study_id != self.gate.study.to_string()
            || ctx.estimate.tokens != 0
            || ctx.estimate.cost_microusd != 0
        {
            return Err(fail());
        }
        let id = transcript_id(input).map_err(|_| fail())?;
        let bytes = checked_source(&self.gate.services, self.gate.project, self.gate.study, id)
            .map_err(|_| fail())?;
        Ok(StepOutput {
            artifact_id: id.to_string(),
            tokens: 0,
            cost_microusd: 0,
            bytes,
        })
    }
}
impl ToolExecutor for LocalStudyDriver {
    fn execute(
        &self,
        context: &ExecutionContext,
        input: &StepInput,
    ) -> Result<ToolReceipt, ToolFailure> {
        let unknown = || ToolFailure {
            code: "effect_claim_unavailable".into(),
            retryable: false,
            certainty: Certainty::UnknownEffect,
        };
        // Claim the idempotency key only after the Engine's Running-intent CAS.
        // A previous claim is *never* permission to execute this tool again.
        match self.gate.services.store.reserve_agent_effect(
            self.gate.project,
            self.gate.study,
            &context.run_id,
            &context.step_id,
            &context.idempotency_key,
        ) {
            Ok(EffectClaim::Claimed(_)) => {}
            Ok(EffectClaim::Existing(_)) | Err(_) => return Err(unknown()),
        }
        let output = match self.inspect(context, input) {
            Ok(output) => output,
            Err(failure) => {
                if self.gate.services.store.record_agent_effect(
                    self.gate.project,
                    self.gate.study,
                    &context.run_id,
                    &context.idempotency_key,
                    EffectUpdate::NotApplied,
                ).is_err() {
                    return Err(unknown());
                }
                return Err(failure);
            }
        };
        // The tool's completion is not reported to the coordinator before
        // the service's durable receipt has committed.
        self.gate.services.store.record_agent_effect(
            self.gate.project,
            self.gate.study,
            &context.run_id,
            &context.idempotency_key,
            EffectUpdate::Applied {
                artifact_id: output.artifact_id.clone(),
            },
        ).map_err(|_| unknown())?;
        Ok(ToolReceipt::Applied(output))
    }

    fn probe(
        &self,
        context: &ExecutionContext,
        input: &StepInput,
        _pending_effect_id: Option<&str>,
    ) -> Result<ProbeResult, ToolFailure> {
        let unknown = || ToolFailure {
            code: "effect_probe_unavailable".into(),
            retryable: false,
            certainty: Certainty::UnknownEffect,
        };
        let effect = self.gate.services.store.get_agent_effect(
            self.gate.project,
            self.gate.study,
            &context.run_id,
            &context.idempotency_key,
        ).map_err(|_| unknown())?;
        let Some(effect) = effect else {
            // No ledger row is not proof that an external effect did not run.
            return Ok(ProbeResult::Unknown);
        };
        match effect.status {
            EffectStatus::NotApplied => Ok(ProbeResult::NotApplied),
            EffectStatus::Accepted => Ok(ProbeResult::Unknown),
            EffectStatus::Applied => {
                let output = self.inspect(context, input).map_err(|_| unknown())?;
                if effect.artifact_id.as_deref() != Some(output.artifact_id.as_str()) {
                    return Ok(ProbeResult::Unknown);
                }
                Ok(ProbeResult::Applied(output))
            }
            EffectStatus::Reserved | EffectStatus::Unknown => {
                // This is exclusively a local, repeatable read. Never apply
                // this path to paid, remote or mutating tools.
                let output = self.inspect(context, input).map_err(|_| unknown())?;
                self.gate.services.store.record_agent_effect(
                    self.gate.project,
                    self.gate.study,
                    &context.run_id,
                    &context.idempotency_key,
                    EffectUpdate::Applied {
                        artifact_id: output.artifact_id.clone(),
                    },
                ).map_err(|_| unknown())?;
                Ok(ProbeResult::Applied(output))
            }
        }
    }
}

type LocalEngine = Engine<ScopedAgentCheckpointStore, LocalStudyDriver, LocalStudyGate>;

impl AppServices {
    fn local_engine(&self, project: Uuid, study: Uuid) -> RepositoryResult<LocalEngine> {
        self.require_active_project(project)?;
        let checkpoints = self.store.scoped_agent_checkpoints(project, study)?;
        let gate = LocalStudyGate {
            services: self.clone(),
            project,
            study,
        };
        let mut registry = ToolRegistry::default();
        registry.allow(ToolKind::ReadStudyTranscript);
        Ok(Engine::new(
            checkpoints,
            LocalStudyDriver { gate: gate.clone() },
            gate,
            registry,
        ))
    }

    fn validate_local_run(
        &self,
        run: &RunSnapshot,
        project: Uuid,
        study: Uuid,
    ) -> RepositoryResult<()> {
        if run.plan.project_id != project.to_string()
            || run.plan.study_id != study.to_string()
            || run.plan.steps.len() != 1
            || !matches!(
                run.plan.steps[0].input,
                StepInput::ReadStudyTranscript { .. }
            )
            || run.plan.steps[0].estimate.cost_microusd != 0
            || run.plan.steps[0].estimate.tokens != 0
        {
            return Err(RepositoryError::Validation(
                "unsupported agent run type".into(),
            ));
        }
        Ok(())
    }

    pub fn start_local_inspection(
        &self,
        project: Uuid,
        study: Uuid,
        transcript: Uuid,
    ) -> RepositoryResult<RunSnapshot> {
        checked_source(self, project, study, transcript)?;
        let engine = self.local_engine(project, study)?;
        let now = current_ms();
        let plan = Plan {
            id: Uuid::new_v4().to_string(),
            project_id: project.to_string(),
            study_id: study.to_string(),
            steps: vec![StepPlan {
                id: "inspect_transcript".into(),
                input: StepInput::ReadStudyTranscript {
                    transcript_id: transcript.to_string(),
                },
                estimate: Estimate {
                    tokens: 0,
                    cost_microusd: 0,
                    max_output_bytes: MAX_LOCAL_TRANSCRIPT_BYTES,
                    cpu_ms: 5_000,
                    peak_memory_mib: 128,
                },
                max_attempts: 1,
                requires_approval: true,
            }],
            limits: Limits {
                max_tool_calls: 1,
                max_tokens: 0,
                max_cost_microusd: 0,
                max_elapsed_ms: LOCAL_AGENT_DEADLINE_MS,
                max_output_bytes: MAX_LOCAL_TRANSCRIPT_BYTES,
                max_cpu_ms: 5_000,
                max_peak_memory_mib: 128,
            },
        };
        let run = engine.create(plan, now).map_err(run_error)?;
        // Only prepare an approval request. No driver executes at this stage.
        engine.tick(&run.plan.id, now).map_err(run_error)
    }

    pub fn list_local_inspections(
        &self,
        project: Uuid,
        study: Uuid,
    ) -> RepositoryResult<Vec<RunSnapshot>> {
        let engine = self.local_engine(project, study)?;
        engine
            .store
            .list_run_ids(25)
            .map_err(|_| unavailable())?
            .into_iter()
            .map(|id| {
                let run = engine.get(&id).map_err(run_error)?;
                self.validate_local_run(&run, project, study)?;
                Ok(run)
            })
            .collect()
    }

    pub fn decide_local_inspection(
        &self,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        action: &str,
    ) -> RepositoryResult<RunSnapshot> {
        if !scriptotar_agent::valid_id(run_id) {
            return Err(RepositoryError::Validation("invalid run id".into()));
        }
        let engine = self.local_engine(project, study)?;
        let before = engine.get(run_id).map_err(run_error)?;
        self.validate_local_run(&before, project, study)?;
        let now = current_ms();
        let next = match action {
            "approve" => {
                let approved = engine
                    .approve(run_id, "inspect_transcript", now)
                    .map_err(run_error)?;
                // The approved action is only a locally bounded, already-attached read.
                engine.tick(&approved.plan.id, now).map_err(run_error)?
            }
            "deny" => engine
                .deny(run_id, "inspect_transcript", now)
                .map_err(run_error)?,
            "cancel" => engine.cancel(run_id, now).map_err(run_error)?,
            "recover" => {
                let result = engine.recover(run_id, now).map_err(run_error)?;
                if result.state == scriptotar_agent::RunState::Blocked
                    && result.steps.first().is_some_and(|step| {
                        matches!(
                            step.state,
                            scriptotar_agent::StepState::Running
                                | scriptotar_agent::StepState::AwaitingResult
                        )
                    })
                {
                    engine.reconcile(run_id, now).map_err(run_error)?
                } else {
                    result
                }
            }
            _ => {
                return Err(RepositoryError::Validation(
                    "unsupported agent action".into(),
                ))
            }
        };
        self.validate_local_run(&next, project, study)?;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};
    use tempfile::TempDir;

    fn attached_transcript(services: &AppServices, project: Uuid, study: Uuid) -> Uuid {
        let source = Uuid::new_v4();
        let media = Uuid::new_v4();
        let transcript = Uuid::new_v4();
        let conn = Connection::open(services.store.path()).unwrap();
        conn.execute(
            "INSERT INTO sources(id,project_id,source_type,locator,created_at)
             VALUES(?1,?2,'local_file','fixture.mp4','2026-11-13T10:00:00Z')",
            params![source.to_string(), project.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO media(id,source_id,local_path,created_at)
             VALUES(?1,?2,'fixture.mp4','2026-11-13T10:00:00Z')",
            params![media.to_string(), source.to_string()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transcripts(id,media_id,text,created_at,updated_at)
             VALUES(?1,?2,'A locally stored transcript','2026-11-13T10:00:00Z','2026-11-13T10:00:00Z')",
            params![transcript.to_string(), media.to_string()],
        ).unwrap();
        services
            .store
            .add_study_source(
                project,
                study,
                &scriptotar_core::StudySourceInput {
                    kind: StudySourceKind::Transcript,
                    reference_id: Some(transcript),
                    locator: None,
                    manual_text: None,
                    source_revision: None,
                    uncertainty: None,
                },
            )
            .unwrap();
        transcript
    }

    #[test]
    fn local_transcript_inspection_requires_approval_and_is_durable() {
        let temp = TempDir::new().unwrap();
        let (project, study, transcript, id);
        {
            let services = AppServices::new(temp.path()).unwrap();
            project = services.active_project_id().unwrap();
            study = services
                .store
                .create_study(project, "Inspect", "")
                .unwrap()
                .id;
            transcript = attached_transcript(&services, project, study);
            let run = services
                .start_local_inspection(project, study, transcript)
                .unwrap();
            assert_eq!(run.state, scriptotar_agent::RunState::AwaitingApproval);
            assert_eq!(run.usage.tool_calls, 0);
            id = run.plan.id;
        }
        let reopened = AppServices::new(temp.path()).unwrap();
        let previous = reopened.list_local_inspections(project, study).unwrap();
        assert_eq!(previous.len(), 1);
        assert_eq!(previous[0].plan.id, id);
        let approved = reopened
            .decide_local_inspection(project, study, &id, "approve")
            .unwrap();
        assert_eq!(approved.state, scriptotar_agent::RunState::Completed);
        assert_eq!(
            approved.steps[0].output.as_ref().unwrap().artifact_id,
            transcript.to_string()
        );
        assert_eq!(approved.usage.reserved_cost_microusd, 0);
        let effect = reopened.store.get_agent_effect(
            project,
            study,
            &id,
            &approved.steps[0].idempotency_key,
        ).unwrap().expect("durable local read receipt");
        assert_eq!(effect.status, EffectStatus::Applied);
        assert_eq!(effect.artifact_id.as_deref(), Some(transcript.to_string().as_str()));
        assert!(reopened
            .decide_local_inspection(project, study, &id, "approve")
            .is_err());
    }

    #[test]
    fn local_inspection_does_not_read_unattached_or_foreign_transcript() {
        let temp = TempDir::new().unwrap();
        let services = AppServices::new(temp.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .store
            .create_study(project, "Inspect", "")
            .unwrap()
            .id;
        let random = Uuid::new_v4();
        assert!(services
            .start_local_inspection(project, study, random)
            .is_err());
        let transcript = attached_transcript(&services, project, study);
        let other_study = services
            .store
            .create_study(project, "Other", "")
            .unwrap()
            .id;
        assert!(services
            .start_local_inspection(project, other_study, transcript)
            .is_err());
        let run = services
            .start_local_inspection(project, study, transcript)
            .unwrap();
        let other_project = scriptotar_core::Project::new("Other project");
        services.store.create_project(&other_project).unwrap();
        assert!(services
            .list_local_inspections(other_project.id, study)
            .is_err());
        assert!(services
            .decide_local_inspection(project, other_study, &run.plan.id, "approve")
            .is_err());
        services.store.archive_study(project, study).unwrap();
        assert!(services
            .decide_local_inspection(project, study, &run.plan.id, "approve")
            .is_err());
    }

    #[test]
    fn local_inspection_denial_does_not_consume_a_tool_call() {
        let temp = TempDir::new().unwrap();
        let services = AppServices::new(temp.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .store
            .create_study(project, "Inspect", "")
            .unwrap()
            .id;
        let transcript = attached_transcript(&services, project, study);
        let run = services
            .start_local_inspection(project, study, transcript)
            .unwrap();
        let denied = services
            .decide_local_inspection(project, study, &run.plan.id, "deny")
            .unwrap();
        assert_eq!(denied.state, scriptotar_agent::RunState::Cancelled);
        assert_eq!(denied.usage.tool_calls, 0);
        assert!(services.store.get_agent_effect(
            project,
            study,
            &denied.plan.id,
            &denied.steps[0].idempotency_key,
        ).unwrap().is_none());
        assert!(services
            .decide_local_inspection(project, study, &run.plan.id, "approve")
            .is_err());
    }
}
