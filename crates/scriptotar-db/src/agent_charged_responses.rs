//! Proposed additive v14 charged-response checkpoint.
//!
//! This is an intentionally staged post-v13 module: Agent 00 must register
//! migration_charged_responses as schema v14 AFTER the approved v13 migration.
//! Never invent a v13 slot or change historical v8/v10-v12 schema entries.
//! The only writer is a trusted adapter which has authenticated a provider
//! response AND exact usage evidence before calling this API. It is not IPC.
//!
//! No new request is sent by this module. A missing seal after a charged remote
//! response is UNRESOLVED, not permission to retry or fabricate response data.
use sha2::{Digest, Sha256};

const MAX_CHARGED_RESPONSE_BYTES: usize = 400_000;
const MAX_CHARGED_RESPONSE_CHARS: usize = 100_000;

fn response_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn charged_step(step: &str) -> bool {
    matches!(step, "angles" | "draft-1" | "draft-2" | "draft-3")
}

/// Additive migration for the reviewed v13 -> v14 transition; it has no
/// impact on historical provider effect and writing batch tables.
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

fn response_row(
    db: &rusqlite::Connection,
    key: ExternalEffectKey<'_>,
) -> RepositoryResult<Option<(i64, String, String, String, String, String, String, Vec<u8>, String, i64, i64, i64)>> {
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
            Ok((
                row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?,
                row.get(8)?, row.get(9)?, row.get(10)?, row.get(11)?,
            ))
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
    let Some((attempt, provider, wire, request_id, receipt, project, study,
              bytes, digest, input, output, cost)) = response_row(db, key)? else {
        return Ok(None);
    };
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

impl SqliteStore {
    /// Exact scoped, read-only recovery: no provider call, no new approval and
    /// no writing mutation. A missing checkpoint remains unresolved.
    pub fn read_sealed_charged_response(
        &self,
        key: ExternalEffectKey<'_>,
    ) -> RepositoryResult<Option<SealedChargedResponse>> {
        let db = database(self)?;
        let effect = checked(&db, key)?;
        if !charged_step(key.step_id) {
            return Err(invalid("charged writing step"));
        }
        verified_charged_response(&db, key, &effect)
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
