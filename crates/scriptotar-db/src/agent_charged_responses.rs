// Proposed additive v14 charged-response checkpoint.
//
// This is an intentionally staged post-v13 module: Agent 00 must register
// migration_charged_responses as schema v14 AFTER the approved v13 migration.
// Never invent a v13 slot or change historical v8/v10-v12 schema entries.
// The only writer is a trusted adapter which has authenticated a provider
// response AND exact usage evidence before calling this API. It is not IPC.
//
// No new request is sent by this module. A missing seal after a charged remote
// response is UNRESOLVED, not permission to retry or fabricate response data.
use sha2::{Digest, Sha256};

const MAX_CHARGED_RESPONSE_BYTES: usize = 400_000;
const MAX_CHARGED_RESPONSE_CHARS: usize = 100_000;

fn response_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn charged_step(step: &str) -> bool {
    matches!(step, "angles" | "draft-1" | "draft-2" | "draft-3")
}

fn check_original_step_output_limit(
    db: &rusqlite::Connection,
    key: ExternalEffectKey<'_>,
    bytes: usize,
) -> RepositoryResult<()> {
    let run = verified_run_settlement_scope(db, key.project, key.study, key.run_id)?;
    let step = run.plan.steps.iter().find(|step| step.id == key.step_id)
        .ok_or_else(|| conflict("charged writing step missing from original plan"))?;
    if u64::try_from(bytes).unwrap_or(u64::MAX) > step.estimate.max_output_bytes {
        return Err(conflict("original approved writing output byte limit exceeded"));
    }
    Ok(())
}

/// Additive migration for the reviewed v13 -> v14 transition; it has no
/// impact on historical provider effect and writing batch tables.
#[allow(dead_code)] // Agent 00 registers the post-v13 migration on integration.
pub(crate) fn migration_charged_responses(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        r#"
CREATE TABLE agent_charged_responses (
    idempotency_key TEXT PRIMARY KEY
        REFERENCES agent_external_effects(idempotency_key) ON DELETE RESTRICT,
    project_id TEXT NOT NULL, study_id TEXT NOT NULL,
    run_id TEXT NOT NULL, step_id TEXT NOT NULL
        CHECK(step_id IN ('angles','draft-1','draft-2','draft-3')),
    attempt INTEGER NOT NULL CHECK(attempt BETWEEN 1 AND 3),
    provider TEXT NOT NULL COLLATE NOCASE CHECK(length(provider) BETWEEN 1 AND 64),
    request_sha256 TEXT NOT NULL CHECK(length(request_sha256)=64),
    provider_request_id TEXT NOT NULL CHECK(length(provider_request_id) BETWEEN 1 AND 128),
    receipt_id TEXT NOT NULL CHECK(length(receipt_id) BETWEEN 1 AND 128),
    response_bytes BLOB NOT NULL
        CHECK(length(response_bytes) BETWEEN 1 AND 400000),
    response_sha256 TEXT NOT NULL CHECK(length(response_sha256)=64),
    input_tokens INTEGER NOT NULL CHECK(input_tokens>=0),
    output_tokens INTEGER NOT NULL CHECK(output_tokens>=0),
    cost_microusd INTEGER NOT NULL CHECK(cost_microusd>=0),
    sealed_at TEXT NOT NULL,
    UNIQUE(project_id,study_id,run_id,step_id),
    UNIQUE(provider,receipt_id)
);
CREATE INDEX idx_charged_responses_run
    ON agent_charged_responses(project_id,study_id,run_id,step_id);
CREATE TRIGGER charged_response_insert_guard BEFORE INSERT ON agent_charged_responses
WHEN NOT EXISTS (
    SELECT 1 FROM agent_external_effects e
    JOIN agent_effects old ON old.idempotency_key=e.idempotency_key
    WHERE e.idempotency_key=NEW.idempotency_key
      AND e.project_id=NEW.project_id AND e.study_id=NEW.study_id
      AND e.run_id=NEW.run_id AND e.step_id=NEW.step_id
      AND e.attempt=NEW.attempt AND e.provider=NEW.provider
      AND e.fingerprint=NEW.request_sha256
      AND e.status IN ('dispatched','uncertain')
      AND (e.provider_request_id IS NULL OR e.provider_request_id=NEW.provider_request_id)
      AND old.project_id=NEW.project_id AND old.study_id=NEW.study_id
      AND old.run_id=NEW.run_id AND old.step_id=NEW.step_id
      AND old.status IN ('reserved','unknown')
)
BEGIN SELECT RAISE(ABORT,'charged response requires dispatched canonical effect'); END;
CREATE TRIGGER charged_response_immutable_update
    BEFORE UPDATE ON agent_charged_responses
    BEGIN SELECT RAISE(ABORT,'charged response seal immutable'); END;
CREATE TRIGGER charged_response_immutable_delete
    BEFORE DELETE ON agent_charged_responses
    BEGIN SELECT RAISE(ABORT,'charged response seal immutable'); END;
-- A generation step cannot become COMPLETED without the same-transaction
-- response seal. The application must insert seal first, then settle receipt.
-- The four-step boundary is exact; non-writing effect types remain v10-compatible.
CREATE TRIGGER charged_response_required_for_completion
    BEFORE UPDATE OF status ON agent_external_effects
    WHEN NEW.status='completed' AND
      NEW.step_id IN ('angles','draft-1','draft-2','draft-3')
      AND NOT EXISTS(
          SELECT 1 FROM agent_charged_responses s
          WHERE s.idempotency_key=NEW.idempotency_key
            AND s.project_id=NEW.project_id AND s.study_id=NEW.study_id
            AND s.run_id=NEW.run_id AND s.step_id=NEW.step_id
            AND s.attempt=NEW.attempt AND s.provider=NEW.provider
            AND s.request_sha256=NEW.fingerprint
            AND s.provider_request_id=NEW.provider_request_id
            AND s.receipt_id=NEW.receipt_id
            AND s.input_tokens+s.output_tokens=NEW.actual_tokens
            AND s.cost_microusd=NEW.actual_cost_microusd
      )
    BEGIN SELECT RAISE(ABORT,'charged writing completion requires sealed response'); END;
CREATE TRIGGER charged_response_no_precompleted_insert
    BEFORE INSERT ON agent_external_effects
    WHEN NEW.step_id IN ('angles','draft-1','draft-2','draft-3')
      AND NEW.status <> 'reserved'
    BEGIN SELECT RAISE(ABORT,'charged writing effects must begin reserved'); END;
"#,
    )
}

/// Provider proof is verified by the caller's trusted adapter, NOT by taking a
/// provider string or a receipt supplied by Studio/the model. This type must
/// never be accepted from a public IPC or an untrusted provider plug-in.
pub struct AuthenticatedChargedResponse<'a> {
    pub key: ExternalEffectKey<'a>,
    pub attempt: i64,
    pub provider: &'a str,
    pub request_sha256: &'a str,
    pub provider_request_id: &'a str,
    pub receipt_id: &'a str,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
    pub response: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedChargedResponse {
    pub step_id: String,
    pub provider: String,
    pub receipt_id: String,
    pub request_sha256: String,
    pub response_sha256: String,
    pub response: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
}

struct ChargedResponseRow {
    attempt: i64,
    provider: String,
    wire: String,
    request_id: String,
    receipt: String,
    project: String,
    study: String,
    bytes: Vec<u8>,
    digest: String,
    input: i64,
    output: i64,
    cost: i64,
}

fn response_row(
    db: &rusqlite::Connection,
    key: ExternalEffectKey<'_>,
) -> RepositoryResult<Option<ChargedResponseRow>> {
    db.query_row(
        "SELECT attempt,provider,request_sha256,provider_request_id,receipt_id,
                project_id,study_id,response_bytes,response_sha256,
                input_tokens,output_tokens,cost_microusd
         FROM agent_charged_responses
         WHERE idempotency_key=?1 AND project_id=?2 AND study_id=?3
           AND run_id=?4 AND step_id=?5",
        params![key.idempotency_key, key.project.to_string(), key.study.to_string(),
                key.run_id, key.step_id],
        |row| {
            Ok(ChargedResponseRow {
                attempt: row.get(0)?,
                provider: row.get(1)?,
                wire: row.get(2)?,
                request_id: row.get(3)?,
                receipt: row.get(4)?,
                project: row.get(5)?,
                study: row.get(6)?,
                bytes: row.get(7)?,
                digest: row.get(8)?,
                input: row.get(9)?,
                output: row.get(10)?,
                cost: row.get(11)?,
            })
        },
    )
    .optional()
    .map_err(db_error)
}

fn verified_charged_response(
    db: &rusqlite::Connection,
    key: ExternalEffectKey<'_>,
    effect: &ExternalEffect,
) -> RepositoryResult<Option<SealedChargedResponse>> {
    let Some(ChargedResponseRow {
        attempt, provider, wire, request_id, receipt, project, study,
        bytes, digest, input, output, cost,
    }) = response_row(db, key)? else {
        return Ok(None);
    };
    check_original_step_output_limit(db, key, bytes.len())?;
    let original = std::str::from_utf8(&bytes)
        .map_err(|_| RepositoryError::Storage("sealed response is not UTF-8".into()))?;
    let tokens = input.checked_add(output).ok_or_else(|| conflict("sealed usage overflow"))?;
    let legacy: (String, Option<String>) = db
        .query_row(
            "SELECT status,artifact_id FROM agent_effects
             WHERE idempotency_key=?1 AND project_id=?2 AND study_id=?3
               AND run_id=?4 AND step_id=?5",
            params![key.idempotency_key, key.project.to_string(), key.study.to_string(),
                    key.run_id, key.step_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(db_error)?;
    if attempt != effect.attempt
        || provider != effect.provider
        || wire != effect.fingerprint
        || project != key.project.to_string()
        || study != key.study.to_string()
        || request_id != effect.provider_request_id.as_deref().unwrap_or("")
        || receipt != effect.receipt_id.as_deref().unwrap_or("")
        || effect.status != ExternalEffectStatus::Completed
        || effect.actual_tokens != u64::try_from(tokens).unwrap_or(u64::MAX)
        || effect.actual_cost_microusd != u64::try_from(cost).unwrap_or(u64::MAX)
        || digest != response_digest(&bytes)
        || bytes.is_empty()
        || bytes.len() > MAX_CHARGED_RESPONSE_BYTES
        || original.chars().count() > MAX_CHARGED_RESPONSE_CHARS
        || legacy.0 != "applied"
        || legacy.1.as_deref() != Some(receipt.as_str())
    {
        return Err(RepositoryError::Storage(
            "sealed response disagrees with canonical paid receipt".into(),
        ));
    }
    Ok(Some(SealedChargedResponse {
        step_id: key.step_id.to_owned(),
        provider,
        receipt_id: receipt,
        request_sha256: wire,
        response_sha256: digest,
        response: original.to_owned(),
        input_tokens: u64::try_from(input).map_err(|_| conflict("negative input usage"))?,
        output_tokens: u64::try_from(output).map_err(|_| conflict("negative output usage"))?,
        cost_microusd: u64::try_from(cost).map_err(|_| conflict("negative cost"))?,
    }))
}


/// Exact four-step request intent authenticated by Agent 05's research-bound
/// writing plan. Construct these only from a trusted host snapshot and exact
/// sealed wire request, never from a model response or public IPC.
#[derive(Debug, Clone, Copy)]
pub struct ExpectedChargedWritingStep<'a> {
    pub step_id: &'a str,
    pub idempotency_key: &'a str,
    pub request_sha256: &'a str,
}

/// No separate SQLite connection: Agent 05 must call this under the very SAME
/// v11 IMMEDIATE transaction that revalidates the approved research/Study and
/// creates three immutable drafts. Missing/uncertain effects return an error,
/// not permission to send again. This helper never mutates billing or drafts.
///
/// The caller must independently authenticate the original research artifact,
/// source fingerprint, user approval, provider account, price revision and the
/// derived exact prompt/HTTP bytes for each expected request fingerprint.
pub fn load_four_sealed_effect_outcomes(
    tx: &Transaction<'_>,
    project: Uuid,
    study: Uuid,
    run_id: &str,
    expectations: &[ExpectedChargedWritingStep<'_>; 4],
) -> RepositoryResult<[SealedChargedResponse; 4]> {
    const ORDER: [&str; 4] = ["angles", "draft-1", "draft-2", "draft-3"];
    let run = verified_run_settlement_scope(tx, project, study, run_id)?;
    let mut outputs = Vec::with_capacity(4);
    for (expected, step) in expectations.iter().zip(ORDER) {
        if expected.step_id != step
            || !fingerprint_ok(expected.request_sha256)
        {
            return Err(invalid("canonical four-writing-step identity"));
        }
        let original_step = run
            .steps
            .iter()
            .find(|record| record.id == step)
            .ok_or_else(|| conflict("original writing run lacks one required step"))?;
        if original_step.idempotency_key != expected.idempotency_key {
            return Err(conflict("writing step effect key changed"));
        }
        let key = ExternalEffectKey {
            project,
            study,
            run_id,
            step_id: step,
            idempotency_key: expected.idempotency_key,
        };
        let effect = checked(tx, key)?;
        if effect.fingerprint != expected.request_sha256
            || effect.provider != "openai"
            || effect.status != ExternalEffectStatus::Completed
        {
            return Err(conflict("writing result is not sealed against the approved wire"));
        }
        let sealed = verified_charged_response(tx, key, &effect)?
            .ok_or_else(|| conflict("writing step lacks original sealed response"))?;
        if sealed.request_sha256 != expected.request_sha256 {
            return Err(conflict("writing response request fingerprint changed"));
        }
        outputs.push(sealed);
    }
    outputs
        .try_into()
        .map_err(|_| RepositoryError::Storage("four-writing-step result shape changed".into()))
}


impl SqliteStore {
    /// Exact scoped, read-only recovery: no provider call, no new approval and
    /// no writing mutation. A missing checkpoint remains unresolved.
    pub fn read_sealed_charged_response(
        &self,
        key: ExternalEffectKey<'_>,
    ) -> RepositoryResult<Option<SealedChargedResponse>> {
        let mut db = database(self)?;
        // One consistent read snapshot: a concurrent receipt+seal COMMIT must
        // never become a mixed pre-commit effect / post-commit response read.
        let tx = db.transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(db_error)?;
        let effect = checked(&tx, key)?;
        if !charged_step(key.step_id) {
            return Err(invalid("charged writing step"));
        }
        let sealed = verified_charged_response(&tx, key, &effect)?;
        if effect.status == ExternalEffectStatus::Completed && sealed.is_none() {
            return Err(RepositoryError::Storage(
                "completed paid step lacks original response: unresolved".into(),
            ));
        }
        tx.commit().map_err(db_error)?;
        Ok(sealed)
    }

    /// One SQLite IMMEDIATE transaction commits: immutable raw response bytes,
    /// v10/v12 provider receipt and anti-replay identities, and the v8 Applied
    /// fence. A failed statement rolls ALL of them back. Late settlement after
    /// owner archival is allowed; this method never authorizes fresh dispatch.
    pub fn settle_authenticated_charged_response(
        &self,
        evidence: AuthenticatedChargedResponse<'_>,
    ) -> RepositoryResult<SealedChargedResponse> {
        let key = evidence.key;
        let bytes = evidence.response.as_bytes();
        let total_tokens = evidence.input_tokens
            .checked_add(evidence.output_tokens)
            .ok_or_else(|| invalid("charged token overflow"))?;
        if !charged_step(key.step_id)
            || !valid_id(key.run_id)
            || evidence.attempt < 1
            || evidence.attempt > 3
            || evidence.provider != "openai"
            || !fingerprint_ok(evidence.request_sha256)
            || !bounded_id(evidence.provider_request_id)
            || !bounded_id(evidence.receipt_id)
            || bytes.is_empty()
            || bytes.len() > MAX_CHARGED_RESPONSE_BYTES
            || evidence.response.chars().count() > MAX_CHARGED_RESPONSE_CHARS
        {
            return Err(invalid("authenticated charged response"));
        }
        let total = to_i64(total_tokens)?;
        let cost = to_i64(evidence.cost_microusd)?;
        let input = to_i64(evidence.input_tokens)?;
        let output = to_i64(evidence.output_tokens)?;
        let digest = response_digest(bytes);
        let mut db = database(self)?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let effect = checked(&tx, key)?;
        check_original_step_output_limit(&tx, key, bytes.len())?;
        if effect.attempt != evidence.attempt
            || effect.provider != evidence.provider
            || effect.fingerprint != evidence.request_sha256
            || effect.provider_request_id.as_deref().is_some_and(
                |id| id != evidence.provider_request_id)
            || total_tokens > effect.token_limit
            || evidence.cost_microusd > effect.cost_limit_microusd
        {
            return Err(conflict("charged response scope, provider or ceiling mismatch"));
        }

        if effect.status == ExternalEffectStatus::Completed {
            let saved = verified_charged_response(&tx, key, &effect)?
                .ok_or_else(|| conflict("completed paid step has no original response seal"))?;
            if saved.response_sha256 != digest
                || saved.response.as_bytes() != bytes
                || saved.receipt_id != evidence.receipt_id
                || saved.input_tokens != evidence.input_tokens
                || saved.output_tokens != evidence.output_tokens
                || saved.cost_microusd != evidence.cost_microusd
            {
                return Err(conflict("contradictory authenticated charged response"));
            }
            return Ok(saved);
        }
        if !matches!(effect.status,
            ExternalEffectStatus::Dispatched | ExternalEffectStatus::Uncertain)
        {
            return Err(conflict("paid effect was not dispatched"));
        }

        let now = now_rfc3339();
        tx.execute(
            "INSERT INTO agent_charged_responses(
               idempotency_key,project_id,study_id,run_id,step_id,attempt,
               provider,request_sha256,provider_request_id,receipt_id,
               response_bytes,response_sha256,input_tokens,output_tokens,
               cost_microusd,sealed_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                key.idempotency_key, key.project.to_string(), key.study.to_string(),
                key.run_id, key.step_id, evidence.attempt, evidence.provider,
                evidence.request_sha256, evidence.provider_request_id,
                evidence.receipt_id, bytes, digest, input, output, cost, now,
            ],
        )
        .map_err(db_error)?;
        // This UPDATE also invokes v12's provider identity anti-replay triggers.
        let updated = tx.execute(
            "UPDATE agent_external_effects
             SET status='completed',
                 provider_request_id=COALESCE(provider_request_id,?1),
                 receipt_id=?2,actual_tokens=?3,actual_cost_microusd=?4,
                 revision=revision+1,updated_at=?5
             WHERE idempotency_key=?6 AND project_id=?7 AND study_id=?8
               AND run_id=?9 AND step_id=?10 AND attempt=?11 AND revision=?12
               AND status IN ('dispatched','uncertain')",
            params![
                evidence.provider_request_id, evidence.receipt_id, total, cost, now,
                key.idempotency_key, key.project.to_string(), key.study.to_string(),
                key.run_id, key.step_id, evidence.attempt, effect.revision,
            ],
        )
        .map_err(db_error)?;
        if updated != 1 {
            return Err(conflict("charged receipt CAS lost"));
        }
        let changed = tx.execute(
            "UPDATE agent_effects SET status='applied',external_id=?1,
               artifact_id=?2,revision=revision+1,updated_at=?3
             WHERE idempotency_key=?4 AND project_id=?5 AND study_id=?6
               AND run_id=?7 AND step_id=?8 AND status IN ('reserved','unknown')",
            params![
                evidence.provider_request_id, evidence.receipt_id, now,
                key.idempotency_key, key.project.to_string(), key.study.to_string(),
                key.run_id, key.step_id,
            ],
        )
        .map_err(db_error)?;
        if changed != 1 {
            return Err(conflict("legacy charge receipt CAS lost"));
        }
        let completed = checked(&tx, key)?;
        let sealed = verified_charged_response(&tx, key, &completed)?
            .ok_or_else(|| conflict("charged response missing after settlement"))?;
        tx.commit().map_err(db_error)?;
        Ok(sealed)
    }
}


#[cfg(test)]
mod charged_response_regressions {
    use super::*;
    use scriptotar_agent::{
        AuditKind, CheckpointStore, Estimate, Limits, Plan, RunSnapshot,
        RunState, StepInput, StepPlan, StepRecord, StepState, Usage,
    };
    use scriptotar_core::{Project, ProjectRepository};
    use tempfile::TempDir;

    fn fixture(step: &str) -> (TempDir, SqliteStore, Uuid, Uuid, RunSnapshot) {
        fixture_with_limit(step, 400000)
    }

    fn fixture_with_limit(
        step: &str,
        approved_bytes: u64,
    ) -> (TempDir, SqliteStore, Uuid, Uuid, RunSnapshot) {
        let temp = TempDir::new().unwrap();
        let db = SqliteStore::open(temp.path().join("charged.sqlite3")).unwrap();
        assert_eq!(db.schema_version().unwrap(), 12);
        let project = Project::new("Charged responses");
        db.create_project(&project).unwrap();
        let study = db.create_study(project.id, "Receipt test", "").unwrap().id;
        let run_id = Uuid::new_v4().to_string();
        let plan = Plan {
            id: run_id.clone(),
            project_id: project.id.to_string(),
            study_id: study.to_string(),
            steps: vec![StepPlan {
                id: step.to_owned(),
                input: StepInput::DraftScript {
                    brief_id: Uuid::new_v4().to_string(),
                },
                estimate: Estimate {
                    tokens: 500,
                    cost_microusd: 2500,
                    max_output_bytes: approved_bytes,
                    cpu_ms: 1,
                    peak_memory_mib: 1,
                },
                max_attempts: 3,
                requires_approval: true,
            }],
            limits: Limits {
                max_tool_calls: 1,
                max_tokens: 500,
                max_cost_microusd: 2500,
                max_elapsed_ms: 60000,
                max_output_bytes: approved_bytes,
                max_cpu_ms: 100,
                max_peak_memory_mib: 4,
            },
        };
        let mut run = RunSnapshot {
            schema_version: 1,
            plan,
            revision: 0,
            state: RunState::Planned,
            current_step: 0,
            steps: vec![StepRecord {
                id: step.to_owned(),
                state: StepState::Pending,
                attempts: 0,
                approved: false,
                idempotency_key: format!("{run_id}:0:{step}"),
                pending_effect_id: None,
                output: None,
                failure_code: None,
            }],
            usage: Usage::default(),
            started_ms: 100,
            cancel_requested: false,
            pause_requested: false,
            events: vec![],
        };
        run.event(100, AuditKind::Created);
        let checkpoints = db.scoped_agent_checkpoints(project.id, study).unwrap();
        checkpoints.create(run.clone()).unwrap();
        run.revision = 1;
        run.state = RunState::Running;
        run.steps[0].state = StepState::Running;
        run.steps[0].attempts = 1;
        run.steps[0].approved = true;
        run.usage.tool_calls = 1;
        run.event(101, AuditKind::Dispatched);
        checkpoints.compare_and_swap(0, run.clone()).unwrap();

        // Fixture-only post-v12 DDL: production requires Agent 00 to register
        // reviewed v13 followed by v14 in the append-only migration ledger.
        let mut conn = database(&db).unwrap();
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).unwrap();
        migration_charged_responses(&tx).unwrap();
        tx.commit().unwrap();
        (temp, db, project.id, study, run)
    }

    fn key<'a>(
        run: &'a RunSnapshot,
        project: Uuid,
        study: Uuid,
    ) -> ExternalEffectKey<'a> {
        ExternalEffectKey {
            project,
            study,
            run_id: &run.plan.id,
            step_id: &run.plan.steps[0].id,
            idempotency_key: &run.steps[0].idempotency_key,
        }
    }

    fn fp() -> &'static str {
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }

    fn dispatch(db: &SqliteStore, key: ExternalEffectKey<'_>) {
        let fresh = db
            .reserve_external_effect(
                key,
                fp(),
                "openai",
                ExternalEffectLimits {
                    tokens: 500,
                    cost_microusd: 2500,
                },
            )
            .unwrap();
        let ExternalReservation::Fresh(effect) = fresh else {
            panic!("must reserve once");
        };
        db.mark_external_dispatched(key, effect.revision, 1).unwrap();
    }

    fn receipt<'a>(
        key: ExternalEffectKey<'a>,
        text: &'a str,
    ) -> AuthenticatedChargedResponse<'a> {
        AuthenticatedChargedResponse {
            key,
            attempt: 1,
            provider: "openai",
            request_sha256: fp(),
            provider_request_id: "trusted_request_14",
            receipt_id: "trusted_receipt_14",
            input_tokens: 11,
            output_tokens: 18,
            cost_microusd: 110,
            response: text,
        }
    }

    #[test]
    fn charged_result_is_atomic_immutable_and_read_only_after_restart() {
        for step in ["angles", "draft-1", "draft-2", "draft-3"] {
            let (temp, db, project, study, run) = fixture(step);
            let k = key(&run, project, study);
            assert!(db.read_sealed_charged_response(k).unwrap().is_none());
            dispatch(&db, k);
            let outbound = db.get_external_effect(k).unwrap().unwrap();
            assert_eq!(outbound.status, ExternalEffectStatus::Dispatched);

            let response = "{\"text\":\"genuine provider bytes\"}";
            let sealed = db
                .settle_authenticated_charged_response(receipt(k, response))
                .unwrap();
            assert_eq!(sealed.response, response);
            assert_eq!(sealed.input_tokens, 11);
            assert_eq!(sealed.output_tokens, 18);
            assert_eq!(sealed.cost_microusd, 110);
            let completed = db.get_external_effect(k).unwrap().unwrap();
            assert_eq!(completed.status, ExternalEffectStatus::Completed);
            assert_eq!(completed.actual_tokens, 29);
            let duplicate = db
                .settle_authenticated_charged_response(receipt(k, response))
                .unwrap();
            assert_eq!(duplicate, sealed);
            assert_eq!(db.get_external_effect(k).unwrap().unwrap(), completed);
            assert!(db
                .settle_authenticated_charged_response(receipt(k, "different result"))
                .is_err());
            drop(db);

            let db = SqliteStore::open(temp.path().join("charged.sqlite3")).unwrap();
            assert_eq!(db.read_sealed_charged_response(k).unwrap(), Some(sealed));
            assert!(db.mark_external_dispatched(k, completed.revision, 1).is_err());
            let effects = db
                .inspect_external_effects_for_run(project, study, &run.plan.id)
                .unwrap();
            assert_eq!(effects.len(), 1);
        }
    }

    #[test]
    fn uncheckpointed_charged_response_remains_unresolved_without_resend() {
        let (_temp, db, project, study, run) = fixture("angles");
        let k = key(&run, project, study);
        dispatch(&db, k);
        // Simulate response received outside SQLite, then process dies before
        // the authenticated response seal and settlement transaction.
        assert!(db.read_sealed_charged_response(k).unwrap().is_none());
        assert_eq!(db.recover_external_effects(project, study, &run.plan.id).unwrap(), 1);
        let uncertain = db.get_external_effect(k).unwrap().unwrap();
        assert_eq!(uncertain.status, ExternalEffectStatus::Uncertain);
        assert!(db.mark_external_dispatched(k, uncertain.revision, 1).is_err());
        assert!(matches!(
            db.reserve_external_effect(
                k, fp(), "openai",
                ExternalEffectLimits { tokens: 500, cost_microusd: 2500 },
            ).unwrap(),
            ExternalReservation::Existing(_)
        ));
        // A late authenticated response may still settle the original attempt.
        let restored = db
            .settle_authenticated_charged_response(receipt(k, "late authentic result"))
            .unwrap();
        assert_eq!(restored.response, "late authentic result");
    }

    #[test]
    fn saved_step_byte_budget_is_enforced_even_below_global_limit() {
        let (_temp, db, project, study, run) = fixture_with_limit("angles", 9);
        let k = key(&run, project, study);
        dispatch(&db, k);
        assert!(db
            .settle_authenticated_charged_response(receipt(k, "1234567890"))
            .is_err());
        assert!(db.read_sealed_charged_response(k).unwrap().is_none());
        assert_eq!(
            db.get_external_effect(k).unwrap().unwrap().status,
            ExternalEffectStatus::Dispatched
        );
        let sealed = db
            .settle_authenticated_charged_response(receipt(k, "123456789"))
            .unwrap();
        assert_eq!(sealed.response.len(), 9);
    }

    #[test]
    fn old_receipt_only_path_cannot_complete_any_four_step_writing_effect() {
        let (_temp, db, project, study, run) = fixture("draft-2");
        let k = key(&run, project, study);
        dispatch(&db, k);
        assert!(db.reconcile_external_effect(k, 1, ExternalResolution::Completed {
            provider_request_id: Some("trusted_request_14"),
            receipt_id: "trusted_receipt_14",
            tokens: 29,
            cost_microusd: 110,
        }).is_err());
        assert_eq!(
            db.get_external_effect(k).unwrap().unwrap().status,
            ExternalEffectStatus::Dispatched
        );
        assert!(db.read_sealed_charged_response(k).unwrap().is_none());
    }

    #[test]
    fn receipt_or_v8_fault_rolls_back_original_bytes_and_all_accounts() {
        for reject in ["v10", "v8"] {
            let (_temp, db, project, study, run) = fixture("draft-3");
            let k = key(&run, project, study);
            dispatch(&db, k);
            let conn = database(&db).unwrap();
            let sql = if reject == "v10" {
                "CREATE TRIGGER charge_fault BEFORE UPDATE OF status ON agent_external_effects
                 WHEN NEW.status='completed'
                 BEGIN SELECT RAISE(ABORT, 'injected v10 settlement failure'); END;"
            } else {
                "CREATE TRIGGER charge_fault BEFORE UPDATE OF status ON agent_effects
                 WHEN NEW.status='applied'
                 BEGIN SELECT RAISE(ABORT, 'injected v8 settlement failure'); END;"
            };
            conn.execute_batch(sql).unwrap();
            assert!(db
                .settle_authenticated_charged_response(receipt(k, "must not survive"))
                .is_err());
            assert!(db.read_sealed_charged_response(k).unwrap().is_none());
            assert_eq!(
                db.get_external_effect(k).unwrap().unwrap().status,
                ExternalEffectStatus::Dispatched
            );
            let claims: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM agent_external_provider_identities
                     WHERE identity IN ('trusted_request_14','trusted_receipt_14')",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(claims, 0, "v12 identity claims must roll back");
            conn.execute_batch("DROP TRIGGER charge_fault;").unwrap();
            assert!(db
                .settle_authenticated_charged_response(receipt(k, "must not survive"))
                .is_ok());
        }
    }

    #[test]
    fn archived_owner_can_settle_late_but_cannot_redispatch() {
        let (temp, db, project, study, run) = fixture("draft-1");
        let k = key(&run, project, study);
        dispatch(&db, k);
        db.archive_study(project, study).unwrap();
        db.archive_project(project).unwrap();
        db.recover_external_effects(project, study, &run.plan.id).unwrap();
        drop(db);
        let db = SqliteStore::open(temp.path().join("charged.sqlite3")).unwrap();
        let sealed = db
            .settle_authenticated_charged_response(receipt(k, "archive-settled bytes"))
            .unwrap();
        assert_eq!(
            db.read_sealed_charged_response(k).unwrap(),
            Some(sealed)
        );
        assert!(db.reserve_external_effect(
            k, fp(), "openai",
            ExternalEffectLimits { tokens: 500, cost_microusd: 2500 }
        ).is_ok()); // Existing is inspection only; never a second dispatch permit.
        let old = db.get_external_effect(k).unwrap().unwrap();
        assert!(db.mark_external_dispatched(k, old.revision, 1).is_err());
    }


    #[test]
    fn two_connections_settle_same_response_once_without_second_charge() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let (temp, db, project, study, run) = fixture("draft-2");
        let k = key(&run, project, study);
        dispatch(&db, k);
        let gate = Arc::new(Barrier::new(3));
        let path = temp.path().join("charged.sqlite3");
        let threads = (0..2)
            .map(|_| {
                let path = path.clone();
                let run = run.clone();
                let gate = gate.clone();
                thread::spawn(move || {
                    let connection = SqliteStore::open(path).unwrap();
                    gate.wait();
                    connection
                        .settle_authenticated_charged_response(receipt(
                            key(&run, project, study),
                            "same original provider output",
                        ))
                        .unwrap()
                })
            })
            .collect::<Vec<_>>();
        gate.wait();
        let mut results = Vec::new();
        for handle in threads {
            results.push(handle.join().unwrap());
        }
        assert_eq!(results[0], results[1]);
        let effect = db.get_external_effect(k).unwrap().unwrap();
        assert_eq!(effect.status, ExternalEffectStatus::Completed);
        assert_eq!(effect.actual_cost_microusd, 110);
        let conn = database(&db).unwrap();
        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_charged_responses
                 WHERE idempotency_key=?1",
                params![k.idempotency_key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(rows, 1);
        let claimed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM agent_external_provider_identities
                 WHERE idempotency_key=?1",
                params![k.idempotency_key],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(claimed, 2);
    }

    #[test]
    fn forged_identity_usage_and_corrupted_bytes_fail_closed() {
        let (_temp, db, project, study, run) = fixture("angles");
        let k = key(&run, project, study);
        dispatch(&db, k);
        let mut fake = receipt(k, "real bytes");
        fake.provider = "openai:forged-model";
        assert!(db.settle_authenticated_charged_response(fake).is_err());
        let mut fake = receipt(k, "real bytes");
        fake.request_sha256 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert!(db.settle_authenticated_charged_response(fake).is_err());
        let mut fake = receipt(k, "real bytes");
        fake.output_tokens = 500;
        assert!(db.settle_authenticated_charged_response(fake).is_err());
        let real = db
            .settle_authenticated_charged_response(receipt(k, "real bytes"))
            .unwrap();
        assert_eq!(real.response, "real bytes");
        let conn = database(&db).unwrap();
        // Test adversarial SQLite DDL outside the v13 permission threat model:
        // even a modified immutable-seal table is rejected by read digest.
        conn.execute_batch(
            "DROP TRIGGER charged_response_immutable_update;
             UPDATE agent_charged_responses SET response_bytes=X'666f72676564';",
        )
        .unwrap();
        assert!(db.read_sealed_charged_response(k).is_err());
    }
}
