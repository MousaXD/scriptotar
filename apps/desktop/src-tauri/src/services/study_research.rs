//! Agent 03 research adapter. Agent 00 registers this module and the
//! AnalyzeEvidence step; no direct IPC, provider or network execution here.
//! All source IDs below are canonical study_sources.id, NOT platform URLs.

use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use scriptotar_agent::{
    Certainty, ExecutionContext, Plan, PolicyGate, ProbeResult, RuntimeError, StepInput,
    StepOutput, StepPlan, ToolExecutor, ToolFailure, ToolReceipt,
};
use scriptotar_core::{ContentStudy, RepositoryError, RepositoryResult, ResearchItem, StudySourceKind};
use scriptotar_db::{EffectClaim, EffectStatus, EffectUpdate};
use scriptotar_research::{
    intelligence::{ObservationTimestamp, TranscriptSegment},
    study_intelligence::{
        analyze_study_sources, StudyResearchAccess, StudyResearchKind, StudyResearchReport,
        StudySourceMaterial, StudyTranscriptMaterial, MAX_STUDY_RESEARCH_SOURCES,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::AppServices;

const MAX_TRANSCRIPT_BYTES: i64 = 128_000;
const MAX_SEGMENTS_JSON_BYTES: i64 = 128_000;
const MAX_REPORT_BYTES: usize = 96_000;
const MAX_TOOL_RUNTIME: Duration = Duration::from_secs(5);

fn invalid(message: &'static str) -> RepositoryError {
    RepositoryError::Validation(message.into())
}

fn not_found() -> RepositoryError {
    RepositoryError::NotFound("available source in active Content Study".into())
}

fn storage() -> RepositoryError {
    RepositoryError::Storage("Study research artifact storage unavailable".into())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn read_db(services: &AppServices) -> RepositoryResult<Connection> {
    let db = Connection::open_with_flags(services.store.path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| storage())?;
    db.busy_timeout(Duration::from_secs(3))
        .map_err(|_| storage())?;
    Ok(db)
}

fn selected_ids(input: &StepInput) -> RepositoryResult<Vec<Uuid>> {
    let StepInput::AnalyzeEvidence { source_ids } = input else {
        return Err(invalid("research tool input must be AnalyzeEvidence"));
    };
    if source_ids.is_empty() || source_ids.len() > MAX_STUDY_RESEARCH_SOURCES {
        return Err(invalid("select one to eight Study sources"));
    }
    let mut ids = Vec::with_capacity(source_ids.len());
    let mut seen = HashSet::new();
    for id in source_ids {
        let id = Uuid::parse_str(id).map_err(|_| invalid("invalid Study source ID"))?;
        if !seen.insert(id) {
            return Err(invalid("duplicate Study source ID"));
        }
        ids.push(id);
    }
    ids.sort_unstable();
    Ok(ids)
}

fn scoped_study(
    services: &AppServices,
    project: Uuid,
    study: Uuid,
    ids: &[Uuid],
) -> RepositoryResult<ContentStudy> {
    services.require_active_project(project)?;
    let record = services.store.get_study(project, study)?;
    if record.archived_at.is_some() {
        return Err(not_found());
    }
    for id in ids {
        // Existing database repository validates the exact project/Study/source tuple.
        let _ = services.store.get_study_source(project, study, *id)?;
    }
    services.require_active_project(project)?;
    Ok(record)
}

fn research_row(
    db: &Connection,
    project: Uuid,
    study: Uuid,
    source: Uuid,
    origin: Uuid,
) -> RepositoryResult<Option<ResearchItem>> {
    let row = db.query_row(
        "SELECT r.creator_id, r.source_url, r.platform, r.title, r.view_count,
                r.like_count, r.comment_count, r.published_at, r.scanned_at
         FROM study_sources ss
         JOIN content_studies st ON st.id=ss.study_id AND st.project_id=ss.project_id
         JOIN projects p ON p.id=ss.project_id
         JOIN research_items r ON r.id=ss.research_id AND r.project_id=ss.project_id
         WHERE ss.project_id=?1 AND ss.study_id=?2 AND ss.id=?3
           AND ss.kind='research_item' AND ss.reference_id=?4
           AND ss.research_id=?4 AND st.archived_at IS NULL AND p.archived_at IS NULL",
        params![project.to_string(), study.to_string(), source.to_string(), origin.to_string()],
        |row| {
            let creator: Option<String> = row.get(0)?;
            Ok((
                creator,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
            ))
        },
    )
    .optional()
    .map_err(|_| storage())?;
    row.map(
        |(creator, source_url, platform, title, view_count, like_count, comment_count, published_at, scanned_at)| {
            Ok(ResearchItem {
                id: origin,
                project_id: project,
                creator_id: creator.map(|id| Uuid::parse_str(&id).map_err(|_| storage())).transpose()?,
                source_url,
                platform,
                title,
                view_count,
                like_count,
                comment_count,
                published_at,
                duration_seconds: None,
                raw_json: None,
                scanned_at,
            })
        },
    )
    .transpose()
}

fn transcript_row(
    db: &Connection,
    project: Uuid,
    study: Uuid,
    source: Uuid,
    origin: Uuid,
) -> RepositoryResult<Option<(String, Option<String>, String)>> {
    db.query_row(
        "SELECT t.text,t.segments_json,t.updated_at
         FROM study_sources ss
         JOIN content_studies st ON st.id=ss.study_id AND st.project_id=ss.project_id
         JOIN projects p ON p.id=ss.project_id
         JOIN transcripts t ON t.id=ss.transcript_id
         JOIN media m ON m.id=t.media_id
         JOIN sources src ON src.id=m.source_id AND src.project_id=ss.project_id
         WHERE ss.id=?3 AND ss.project_id=?1 AND ss.study_id=?2
           AND ss.kind='transcript' AND ss.reference_id=?4 AND ss.transcript_id=?4
           AND st.archived_at IS NULL AND p.archived_at IS NULL
           AND length(CAST(t.text AS BLOB)) <= ?5
           AND (t.segments_json IS NULL
             OR length(CAST(t.segments_json AS BLOB)) <= ?6)",
        params![
            project.to_string(), study.to_string(), source.to_string(), origin.to_string(),
            MAX_TRANSCRIPT_BYTES, MAX_SEGMENTS_JSON_BYTES,
        ],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
    .map_err(|_| storage())
}

fn parse_transcript(
    origin: Uuid,
    text: &str,
    segments_json: Option<&str>,
    revision: String,
) -> StudyTranscriptMaterial {
    let mut parsed = Vec::new();
    let mut partial = false;
    if let Some(raw) = segments_json {
        if let Ok(Value::Array(values)) = serde_json::from_str::<Value>(raw) {
            // Never map a huge array into an unlimited set of source excerpts.
            // The JSON byte admission happens in SQL before this parse.
            let mut positions: Vec<usize> = (0..values.len().min(48)).collect();
            if values.len() > 48 {
                partial = true;
                positions = (0..16)
                    .chain(values.len() / 2 - 8..values.len() / 2 + 8)
                    .chain(values.len() - 16..values.len())
                    .collect();
            }
            for index in positions {
                let Some(row) = values[index].as_object() else {
                    partial = true;
                    continue;
                };
                let Some(text) = row.get("text").and_then(Value::as_str) else {
                    partial = true;
                    continue;
                };
                let (Some(start), Some(end)) = (
                    row.get("start").and_then(Value::as_f64),
                    row.get("end").and_then(Value::as_f64),
                ) else {
                    partial = true;
                    continue;
                };
                if !start.is_finite()
                    || !end.is_finite()
                    || start < 0.0
                    || end <= start
                    || end > 3_600_000.0
                    || text.chars().count() > 2_000
                {
                    partial = true;
                    continue;
                }
                let id = row
                    .get("index")
                    .and_then(Value::as_u64)
                    .map_or_else(|| index.to_string(), |n| n.to_string());
                parsed.push(TranscriptSegment {
                    segment_id: id,
                    start_ms: (start * 1_000.0).round() as u64,
                    end_ms: (end * 1_000.0).round() as u64,
                    text: text.to_owned(),
                });
            }
            if parsed.len() != values.len() {
                partial = true;
            }
        } else {
            partial = true;
        }
    }
    let untimed_text = if parsed.is_empty() {
        Some(text.chars().take(160).collect()).filter(|value: &String| !value.trim().is_empty())
    } else {
        None
    };
    StudyTranscriptMaterial {
        transcript_id: origin,
        revision,
        segments: parsed,
        untimed_text,
        partial,
    }
}

fn inspect_selected(
    services: &AppServices,
    project: Uuid,
    study: Uuid,
    ids: &[Uuid],
    analyzed_at: ObservationTimestamp,
) -> RepositoryResult<(i64, StudyResearchReport)> {
    let record = scoped_study(services, project, study, ids)?;
    let db = read_db(services)?;
    db.execute_batch("BEGIN DEFERRED TRANSACTION")
        .map_err(|_| storage())?;
    let mut sources = Vec::new();
    for id in ids {
        let attachment = services.store.get_study_source(project, study, *id)?;
        let Some(reference_id) = attachment.reference_id else {
            return Err(invalid("research analysis only supports existing research or transcript IDs"));
        };
        let kind = match attachment.kind {
            StudySourceKind::ResearchItem => StudyResearchKind::ResearchItem,
            StudySourceKind::Transcript => StudyResearchKind::Transcript,
            _ => return Err(invalid("source kind not permitted for research analysis")),
        };
        let mut access = if attachment.available {
            StudyResearchAccess::Available
        } else {
            StudyResearchAccess::Deleted
        };
        let (research, transcript) = if !attachment.available {
            (None, None)
        } else {
            match kind {
                StudyResearchKind::ResearchItem => {
                    let record = research_row(&db, project, study, *id, reference_id)?;
                    if record.is_none() {
                        access = StudyResearchAccess::Unavailable;
                    }
                    (record, None)
                }
                StudyResearchKind::Transcript => {
                    let record = transcript_row(&db, project, study, *id, reference_id)?;
                    match record {
                        Some((text, segments, revision)) => {
                            let transcript = parse_transcript(reference_id, &text, segments.as_deref(), revision);
                            if transcript.partial {
                                access = StudyResearchAccess::Partial;
                            }
                            (None, Some(transcript))
                        }
                        None => {
                            access = StudyResearchAccess::Unsupported;
                            (None, None)
                        }
                    }
                }
            }
        };
        sources.push(StudySourceMaterial {
            project_id: project,
            study_id: study,
            study_source_id: *id,
            kind,
            reference_id,
            locator: attachment.locator,
            collected_at: attachment.collected_at,
            access,
            research,
            transcript,
        });
    }
    db.execute_batch("COMMIT").map_err(|_| storage())?;
    let report = analyze_study_sources(project, study, analyzed_at, sources)
        .map_err(|_| invalid("invalid selected research data"))?;
    services.require_active_project(project)?;
    let current = services.store.get_study(project, study)?;
    if current.revision != record.revision || current.archived_at.is_some() {
        return Err(RepositoryError::Conflict("Study changed during research inspection".into()));
    }
    Ok((record.revision, report))
}

#[derive(Debug, Serialize, Deserialize)]
struct ReportEnvelope {
    project_id: Uuid,
    study_id: Uuid,
    study_revision: i64,
    run_id: String,
    step_id: String,
    idempotency_key: String,
    report: StudyResearchReport,
}

fn report_id(context: &ExecutionContext) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!(
            "scriptotar/study-research/v1/{}/{}/{}/{}/{}",
            context.project_id, context.study_id, context.run_id,
            context.step_id, context.idempotency_key
        )
        .as_bytes(),
    )
}

fn artifact_path(services: &AppServices, artifact_id: Uuid) -> RepositoryResult<PathBuf> {
    let parent = services.store.path().parent().ok_or_else(storage)?;
    Ok(parent
        .join("study-research-reports")
        .join(format!("{artifact_id}.json")))
}

fn load_envelope(services: &AppServices, artifact_id: Uuid) -> RepositoryResult<ReportEnvelope> {
    let path = artifact_path(services, artifact_id)?;
    let bytes = fs::read(path).map_err(|_| not_found())?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err(storage());
    }
    serde_json::from_slice(&bytes).map_err(|_| storage())
}

fn write_envelope(services: &AppServices, artifact_id: Uuid, data: &[u8]) -> RepositoryResult<()> {
    if data.len() > MAX_REPORT_BYTES {
        return Err(invalid("research findings exceed output byte budget"));
    }
    let path = artifact_path(services, artifact_id)?;
    let directory = path.parent().ok_or_else(storage)?;
    fs::create_dir_all(directory).map_err(|_| storage())?;
    if path.exists() {
        let existing = fs::read(&path).map_err(|_| storage())?;
        return if existing == data {
            Ok(())
        } else {
            Err(RepositoryError::Conflict("research artifact ID already exists".into()))
        };
    }
    let temporary = directory.join(format!("{artifact_id}.{}.pending", Uuid::new_v4()));
    let result = (|| {
        let mut handle = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| storage())?;
        handle.write_all(data).map_err(|_| storage())?;
        handle.sync_all().map_err(|_| storage())?;
        fs::rename(&temporary, &path).map_err(|_| storage())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

/// Reads only source revision metadata; never loads full transcript text when
/// checking whether a previously persisted excerpt is still current.
fn current_origin_revision(
    db: &Connection,
    project: Uuid,
    kind: StudyResearchKind,
    origin: Uuid,
) -> RepositoryResult<Option<String>> {
    let sql = match kind {
        StudyResearchKind::ResearchItem => {
            "SELECT scanned_at FROM research_items WHERE id=?1 AND project_id=?2"
        }
        StudyResearchKind::Transcript => {
            "SELECT t.updated_at FROM transcripts t
             JOIN media m ON m.id=t.media_id
             JOIN sources s ON s.id=m.source_id
             WHERE t.id=?1 AND s.project_id=?2"
        }
    };
    db.query_row(
        sql,
        params![origin.to_string(), project.to_string()],
        |row| row.get(0),
    )
    .optional()
    .map_err(|_| storage())
}

fn checked_report(
    services: &AppServices,
    project: Uuid,
    study: Uuid,
    artifact: Uuid,
) -> RepositoryResult<ReportEnvelope> {
    services.require_active_project(project)?;
    let saved = load_envelope(services, artifact)?;
    if saved.project_id != project || saved.study_id != study
        || saved.report.project_id != project || saved.report.study_id != study
        || saved.report.sources.len() > MAX_STUDY_RESEARCH_SOURCES
        || !saved.report.source_text_is_untrusted_data
    {
        return Err(not_found());
    }
    let ids: Vec<Uuid> = saved.report.sources.iter().map(|s| s.study_source_id).collect();
    let record = scoped_study(services, project, study, &ids)?;
    if record.revision != saved.study_revision {
        return Err(RepositoryError::Conflict("Study changed since research snapshot".into()));
    }
    let db = read_db(services)?;
    for finding in &saved.report.sources {
        let source = services.store.get_study_source(project, study, finding.study_source_id)?;
        if source.reference_id != Some(finding.reference_id)
            || source.locator != finding.locator
            || (!source.available && matches!(finding.access, StudyResearchAccess::Available | StudyResearchAccess::Partial))
        {
            return Err(not_found());
        }
        if let Some(expected) = finding.origin_revision.as_deref() {
            if !source.available
                || current_origin_revision(&db, project, finding.kind, finding.reference_id)?
                    .as_deref() != Some(expected)
            {
                return Err(RepositoryError::Conflict(
                    "source changed or was removed after research analysis".into(),
                ));
            }
        } else if matches!(finding.access, StudyResearchAccess::Available | StudyResearchAccess::Partial) {
            // An allegedly inspected source must always be tied to a live
            // origin version. Missing version is not evidence of success.
            return Err(not_found());
        }
    }
    services.require_active_project(project)?;
    Ok(saved)
}

/// Agent 00 may expose this only via a project- and Study-scoped, read-only API.
/// Rechecks source availability; it is NOT a prompt or an approved provider call.
pub(super) fn read_study_research_artifact(
    services: &AppServices,
    project: Uuid,
    study: Uuid,
    artifact: Uuid,
) -> RepositoryResult<StudyResearchReport> {
    let saved = checked_report(services, project, study, artifact)?;
    // A file on disk is not itself an approved, completed agent action.
    // The user-facing read requires the *committed* ledger receipt. The
    // internal probe path can still inspect a reserved file to reconcile a
    // crash between durable file write and effect settlement.
    let effect = services.store.get_agent_effect(
        project,
        study,
        &saved.run_id,
        &saved.idempotency_key,
    )?;
    if !matches!(effect, Some(ref receipt)
        if receipt.status == EffectStatus::Applied
            && receipt.step_id == saved.step_id
            && receipt.tool_kind == "AnalyzeEvidence"
            && receipt.artifact_id.as_deref() == Some(artifact.to_string().as_str()))
    {
        return Err(not_found());
    }
    Ok(saved.report)
}

#[derive(Clone)]
pub(super) struct StudyResearchGate {
    services: AppServices,
    project: Uuid,
    study: Uuid,
}

impl StudyResearchGate {
    pub(super) fn new(services: AppServices, project: Uuid, study: Uuid) -> Self {
        Self { services, project, study }
    }
}

impl PolicyGate for StudyResearchGate {
    fn authorize(&self, plan: &Plan, step: &StepPlan) -> Result<(), RuntimeError> {
        if plan.project_id != self.project.to_string()
            || plan.study_id != self.study.to_string()
            || !step.requires_approval
            || step.estimate.tokens != 0
            || step.estimate.cost_microusd != 0
            || step.estimate.max_output_bytes > MAX_REPORT_BYTES as u64
        {
            return Err(RuntimeError::PolicyDenied);
        }
        let ids = selected_ids(&step.input).map_err(|_| RuntimeError::PolicyDenied)?;
        scoped_study(&self.services, self.project, self.study, &ids)
            .map_err(|_| RuntimeError::PolicyDenied)?;
        Ok(())
    }
}

pub(super) struct StudyResearchDriver {
    pub(super) gate: StudyResearchGate,
}

impl StudyResearchDriver {
    pub(super) fn new(gate: StudyResearchGate) -> Self {
        Self { gate }
    }

    fn guarded_input(&self, ctx: &ExecutionContext, input: &StepInput, check_deadline: bool) -> RepositoryResult<Vec<Uuid>> {
        if ctx.project_id != self.gate.project.to_string()
            || ctx.study_id != self.gate.study.to_string()
            || ctx.estimate.tokens != 0
            || ctx.estimate.cost_microusd != 0
            || ctx.estimate.max_output_bytes > MAX_REPORT_BYTES as u64
            || (check_deadline && ctx.deadline_ms <= now_ms())
        {
            return Err(invalid("invalid or expired research authorization"));
        }
        let ids = selected_ids(input)?;
        scoped_study(&self.gate.services, self.gate.project, self.gate.study, &ids)?;
        Ok(ids)
    }

    fn inspect(&self, ctx: &ExecutionContext, input: &StepInput) -> RepositoryResult<StepOutput> {
        let start = Instant::now();
        let ids = self.guarded_input(ctx, input, true)?;
        let current = scriptotar_core::now_rfc3339();
        let analyzed_at = ObservationTimestamp::parse_rfc3339(&current)
            .ok_or_else(|| invalid("unavailable observation clock"))?;
        let (revision, report) =
            inspect_selected(&self.gate.services, self.gate.project, self.gate.study, &ids, analyzed_at)?;
        let saved = ReportEnvelope {
            project_id: self.gate.project,
            study_id: self.gate.study,
            study_revision: revision,
            run_id: ctx.run_id.clone(),
            step_id: ctx.step_id.clone(),
            idempotency_key: ctx.idempotency_key.clone(),
            report,
        };
        let bytes = serde_json::to_vec(&saved).map_err(|_| storage())?;
        if bytes.len() > MAX_REPORT_BYTES
            || bytes.len() as u64 > ctx.estimate.max_output_bytes
            || start.elapsed() > MAX_TOOL_RUNTIME
            || ctx.deadline_ms <= now_ms()
        {
            return Err(invalid("research operation exceeded authorized resource limits"));
        }
        let id = report_id(ctx);
        write_envelope(&self.gate.services, id, &bytes)?;
        Ok(StepOutput {
            artifact_id: id.to_string(),
            tokens: 0,
            cost_microusd: 0,
            bytes: bytes.len() as u64,
        })
    }

    fn stored(&self, ctx: &ExecutionContext, input: &StepInput) -> RepositoryResult<StepOutput> {
        let ids = self.guarded_input(ctx, input, false)?;
        let id = report_id(ctx);
        let saved = checked_report(&self.gate.services, self.gate.project, self.gate.study, id)?;
        if saved.run_id != ctx.run_id
            || saved.step_id != ctx.step_id
            || saved.idempotency_key != ctx.idempotency_key
            || ids != saved.report.sources.iter().map(|s| s.study_source_id).collect::<Vec<_>>()
        {
            return Err(not_found());
        }
        let bytes = fs::metadata(artifact_path(&self.gate.services, id)?)
            .map_err(|_| not_found())?.len();
        if bytes > ctx.estimate.max_output_bytes {
            return Err(invalid("persisted report exceeds approved budget"));
        }
        Ok(StepOutput {
            artifact_id: id.to_string(), tokens: 0, cost_microusd: 0, bytes,
        })
    }
}

fn failed() -> ToolFailure {
    ToolFailure { code: "study_research_denied".into(), retryable: false, certainty: Certainty::NotApplied }
}
fn uncertain() -> ToolFailure {
    ToolFailure { code: "study_research_effect_uncertain".into(), retryable: false, certainty: Certainty::UnknownEffect }
}

impl ToolExecutor for StudyResearchDriver {
    fn execute(&self, ctx: &ExecutionContext, input: &StepInput) -> Result<ToolReceipt, ToolFailure> {
        self.guarded_input(ctx, input, true).map_err(|_| failed())?;
        match self.gate.services.store.reserve_agent_effect(
            self.gate.project, self.gate.study,
            &ctx.run_id, &ctx.step_id, &ctx.idempotency_key,
        ) {
            Ok(EffectClaim::Claimed(_)) => {}
            Ok(EffectClaim::Existing(_)) | Err(_) => return Err(uncertain()),
        }
        let output = match self.inspect(ctx, input) {
            Ok(result) => result,
            Err(_) => {
                if self.gate.services.store.record_agent_effect(
                    self.gate.project, self.gate.study, &ctx.run_id,
                    &ctx.idempotency_key, EffectUpdate::NotApplied,
                ).is_err() {
                    return Err(uncertain());
                }
                return Err(failed());
            }
        };
        self.gate.services.store.record_agent_effect(
            self.gate.project, self.gate.study, &ctx.run_id,
            &ctx.idempotency_key,
            EffectUpdate::Applied { artifact_id: output.artifact_id.clone() },
        ).map_err(|_| uncertain())?;
        Ok(ToolReceipt::Applied(output))
    }

    fn probe(
        &self,
        ctx: &ExecutionContext,
        input: &StepInput,
        _pending_effect_id: Option<&str>,
    ) -> Result<ProbeResult, ToolFailure> {
        let effect = self.gate.services.store.get_agent_effect(
            self.gate.project, self.gate.study, &ctx.run_id, &ctx.idempotency_key,
        ).map_err(|_| uncertain())?;
        let Some(effect) = effect else {
            return Ok(ProbeResult::Unknown);
        };
        match effect.status {
            EffectStatus::NotApplied => Ok(ProbeResult::NotApplied),
            EffectStatus::Accepted => Ok(ProbeResult::Unknown),
            EffectStatus::Applied | EffectStatus::Reserved | EffectStatus::Unknown => {
                let Ok(output) = self.stored(ctx, input) else {
                    // Never replay a missing/ambiguous persisted effect.
                    return Ok(ProbeResult::Unknown);
                };
                if effect.status == EffectStatus::Applied {
                    if effect.artifact_id.as_deref() != Some(output.artifact_id.as_str()) {
                        return Ok(ProbeResult::Unknown);
                    }
                } else {
                    self.gate.services.store.record_agent_effect(
                        self.gate.project, self.gate.study, &ctx.run_id,
                        &ctx.idempotency_key,
                        EffectUpdate::Applied { artifact_id: output.artifact_id.clone() },
                    ).map_err(|_| uncertain())?;
                }
                Ok(ProbeResult::Applied(output))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriptotar_core::StudySourceInput;
    use tempfile::TempDir;

    #[test]
    fn denies_wrong_project_unattached_and_unsupported_input_before_effect_claim() {
        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services.store.create_study(project, "Study", "").unwrap().id;
        let other = Uuid::new_v4();
        let input = StepInput::AnalyzeEvidence { source_ids: vec![other.to_string()] };
        assert!(selected_ids(&input).is_ok());
        assert!(scoped_study(&services, project, study, &[other]).is_err());
        assert!(scoped_study(&services, Uuid::new_v4(), study, &[other]).is_err());
    }

    #[test]
    fn report_survives_restart_but_revoked_source_is_not_readable() {
        let dir = TempDir::new().unwrap();
        let project;
        let study;
        let source;
        let research_id;
        let artifact = Uuid::new_v4();
        {
            let services = AppServices::new(dir.path()).unwrap();
            project = services.active_project_id().unwrap();
            study = services.store.create_study(project, "Study", "").unwrap().id;
            research_id = Uuid::new_v4();
            // The real source is inserted through the existing research repository.
            let item = ResearchItem {
                id: research_id, project_id: project, creator_id: None,
                source_url: "https://www.youtube.com/watch?v=abc123".into(),
                platform: "YouTube".into(), title: None, view_count: None,
                like_count: None, comment_count: None, published_at: None,
                duration_seconds: None, raw_json: None, scanned_at: "2026-11-15T00:00:00Z".into(),
            };
            use scriptotar_core::ResearchRepository;
            services.store.upsert_research_items(&[item]).unwrap();
            source = services.store.add_study_source(project, study, &StudySourceInput {
                kind: StudySourceKind::ResearchItem, reference_id: Some(research_id),
                locator: None, manual_text: None, source_revision: None, uncertainty: None,
            }).unwrap().id;
            let (_, report) = inspect_selected(&services, project, study, &[source], ObservationTimestamp::parse_rfc3339("2026-11-16T00:00:00Z").unwrap()).unwrap();
            let envelope = ReportEnvelope {
                project_id: project, study_id: study,
                study_revision: services.store.get_study(project, study).unwrap().revision,
                run_id: "test-run".into(), step_id: "research".into(),
                idempotency_key: "key".into(), report,
            };
            write_envelope(&services, artifact, &serde_json::to_vec(&envelope).unwrap()).unwrap();
        }
        let reopened = AppServices::new(dir.path()).unwrap();
        assert!(checked_report(&reopened, project, study, artifact).is_ok());
        // An arbitrary persisted JSON file cannot masquerade as a completed
        // agent action: this fixture has no approved run or effect receipt.
        assert!(read_study_research_artifact(&reopened, project, study, artifact).is_err());
        assert!(checked_report(&reopened, Uuid::new_v4(), study, artifact).is_err());
        let db = Connection::open(reopened.store.path()).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        db.execute(
            "UPDATE research_items SET scanned_at='2026-11-16T08:00:00Z'
             WHERE id=?1 AND project_id=?2",
            params![research_id.to_string(), project.to_string()],
        ).unwrap();
        assert!(checked_report(&reopened, project, study, artifact).is_err());
        db.execute(
            "DELETE FROM research_items WHERE id=?1 AND project_id=?2",
            params![research_id.to_string(), project.to_string()],
        ).unwrap();
        assert!(!reopened.store.get_study_source(project, study, source).unwrap().available);
        assert!(checked_report(&reopened, project, study, artifact).is_err());
        reopened.store.archive_study(project, study).unwrap();
        assert!(checked_report(&reopened, project, study, artifact).is_err());
    }
}
