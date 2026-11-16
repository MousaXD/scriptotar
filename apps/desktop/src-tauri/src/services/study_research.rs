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
    Certainty, Engine, Estimate, ExecutionContext, Limits, Plan, PolicyGate, ProbeResult,
    RunSnapshot, RuntimeError, StepInput, StepOutput, StepPlan, ToolExecutor, ToolFailure,
    ToolKind, ToolReceipt, ToolRegistry,
};
use scriptotar_core::{
    ContentStudy, RepositoryError, RepositoryResult, ResearchItem, StudySourceKind,
};
use scriptotar_db::{EffectClaim, EffectStatus, EffectUpdate, ScopedAgentCheckpointStore};
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
    let active: bool = read_db(services)?
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM content_studies st
                JOIN projects p ON p.id=st.project_id
                WHERE st.id=?1 AND st.project_id=?2
                  AND st.archived_at IS NULL AND p.archived_at IS NULL
            )",
            params![study.to_string(), project.to_string()],
            |row| row.get(0),
        )
        .map_err(|_| storage())?;
    if !active {
        return Err(not_found());
    }
    for id in ids {
        // Validate *both* exact source membership and admissible source kind
        // before requesting user approval, not only after effect reservation.
        let attached = services.store.get_study_source(project, study, *id)?;
        if attached.reference_id.is_none()
            || !matches!(
                attached.kind,
                StudySourceKind::ResearchItem | StudySourceKind::Transcript
            )
        {
            return Err(invalid(
                "research inspection requires a research item or transcript",
            ));
        }
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
    let row = db
        .query_row(
            "SELECT r.creator_id, r.source_url, r.platform, r.title, r.view_count,
                r.like_count, r.comment_count, r.published_at, r.scanned_at
         FROM study_sources ss
         JOIN content_studies st ON st.id=ss.study_id AND st.project_id=ss.project_id
         JOIN projects p ON p.id=ss.project_id
         JOIN research_items r ON r.id=ss.research_id AND r.project_id=ss.project_id
         WHERE ss.project_id=?1 AND ss.study_id=?2 AND ss.id=?3
           AND ss.kind='research_item' AND ss.reference_id=?4
           AND ss.research_id=?4 AND st.archived_at IS NULL AND p.archived_at IS NULL",
            params![
                project.to_string(),
                study.to_string(),
                source.to_string(),
                origin.to_string()
            ],
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
        |(
            creator,
            source_url,
            platform,
            title,
            view_count,
            like_count,
            comment_count,
            published_at,
            scanned_at,
        )| {
            Ok(ResearchItem {
                id: origin,
                project_id: project,
                creator_id: creator
                    .map(|id| Uuid::parse_str(&id).map_err(|_| storage()))
                    .transpose()?,
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
            project.to_string(),
            study.to_string(),
            source.to_string(),
            origin.to_string(),
            MAX_TRANSCRIPT_BYTES,
            MAX_SEGMENTS_JSON_BYTES,
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
            return Err(invalid(
                "research analysis only supports existing research or transcript IDs",
            ));
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
                            let transcript = parse_transcript(
                                reference_id,
                                &text,
                                segments.as_deref(),
                                revision,
                            );
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
        return Err(RepositoryError::Conflict(
            "Study changed during research inspection".into(),
        ));
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
            context.project_id,
            context.study_id,
            context.run_id,
            context.step_id,
            context.idempotency_key
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
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true).mode(0o700);
        builder.create(directory).map_err(|_| storage())?;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).map_err(|_| storage())?;
    }
    #[cfg(not(unix))]
    fs::create_dir_all(directory).map_err(|_| storage())?;
    if path.exists() {
        let existing = fs::read(&path).map_err(|_| storage())?;
        return if existing == data {
            Ok(())
        } else {
            Err(RepositoryError::Conflict(
                "research artifact ID already exists".into(),
            ))
        };
    }
    let temporary = directory.join(format!("{artifact_id}.{}.pending", Uuid::new_v4()));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut handle = options.open(&temporary).map_err(|_| storage())?;
        handle.write_all(data).map_err(|_| storage())?;
        handle.sync_all().map_err(|_| storage())?;
        fs::rename(&temporary, &path).map_err(|_| storage())?;
        #[cfg(unix)]
        {
            // Make the directory entry durable before recording the SQLite
            // Applied receipt. A failure remains uncertain, never replayed.
            fs::File::open(directory)
                .and_then(|dir| dir.sync_all())
                .map_err(|_| storage())?;
        }
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
    if saved.project_id != project
        || saved.study_id != study
        || saved.report.project_id != project
        || saved.report.study_id != study
        || saved.report.sources.len() > MAX_STUDY_RESEARCH_SOURCES
        || !saved.report.source_text_is_untrusted_data
    {
        return Err(not_found());
    }
    let ids: Vec<Uuid> = saved
        .report
        .sources
        .iter()
        .map(|s| s.study_source_id)
        .collect();
    let record = scoped_study(services, project, study, &ids)?;
    if record.revision != saved.study_revision {
        return Err(RepositoryError::Conflict(
            "Study changed since research snapshot".into(),
        ));
    }
    let db = read_db(services)?;
    for finding in &saved.report.sources {
        let source = services
            .store
            .get_study_source(project, study, finding.study_source_id)?;
        if source.reference_id != Some(finding.reference_id)
            || source.locator != finding.locator
            || (!source.available
                && matches!(
                    finding.access,
                    StudyResearchAccess::Available | StudyResearchAccess::Partial
                ))
        {
            return Err(not_found());
        }
        if let Some(expected) = finding.origin_revision.as_deref() {
            if !source.available
                || current_origin_revision(&db, project, finding.kind, finding.reference_id)?
                    .as_deref()
                    != Some(expected)
            {
                return Err(RepositoryError::Conflict(
                    "source changed or was removed after research analysis".into(),
                ));
            }
        } else if matches!(
            finding.access,
            StudyResearchAccess::Available | StudyResearchAccess::Partial
        ) {
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
    let effect =
        services
            .store
            .get_agent_effect(project, study, &saved.run_id, &saved.idempotency_key)?;
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
        Self {
            services,
            project,
            study,
        }
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
            || step.max_attempts != 1
            || step.estimate.cpu_ms == 0
            || step.estimate.cpu_ms > 5_000
            || step.estimate.peak_memory_mib == 0
            || step.estimate.peak_memory_mib > 128
            || plan.limits.max_tokens != 0
            || plan.limits.max_cost_microusd != 0
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

    fn guarded_input(
        &self,
        ctx: &ExecutionContext,
        input: &StepInput,
        check_deadline: bool,
    ) -> RepositoryResult<Vec<Uuid>> {
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
        scoped_study(
            &self.gate.services,
            self.gate.project,
            self.gate.study,
            &ids,
        )?;
        Ok(ids)
    }

    fn inspect(&self, ctx: &ExecutionContext, input: &StepInput) -> RepositoryResult<StepOutput> {
        let start = Instant::now();
        let ids = self.guarded_input(ctx, input, true)?;
        let current = scriptotar_core::now_rfc3339();
        let analyzed_at = ObservationTimestamp::parse_rfc3339(&current)
            .ok_or_else(|| invalid("unavailable observation clock"))?;
        let (revision, report) = inspect_selected(
            &self.gate.services,
            self.gate.project,
            self.gate.study,
            &ids,
            analyzed_at,
        )?;
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
            return Err(invalid(
                "research operation exceeded authorized resource limits",
            ));
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
            || ids
                != saved
                    .report
                    .sources
                    .iter()
                    .map(|s| s.study_source_id)
                    .collect::<Vec<_>>()
        {
            return Err(not_found());
        }
        let bytes = fs::metadata(artifact_path(&self.gate.services, id)?)
            .map_err(|_| not_found())?
            .len();
        if bytes > ctx.estimate.max_output_bytes {
            return Err(invalid("persisted report exceeds approved budget"));
        }
        Ok(StepOutput {
            artifact_id: id.to_string(),
            tokens: 0,
            cost_microusd: 0,
            bytes,
        })
    }
}

/// Typed, deny-by-default service boundary for Agent 00's authorized run/approval
/// endpoints. Creating an engine does not dispatch any tool or grant consent.
pub(super) type StudyResearchEngine =
    Engine<ScopedAgentCheckpointStore, StudyResearchDriver, StudyResearchGate>;

pub(super) fn build_study_research_engine(
    services: &AppServices,
    project: Uuid,
    study: Uuid,
) -> RepositoryResult<StudyResearchEngine> {
    scoped_study(services, project, study, &[])?;
    let checkpoints = services.store.scoped_agent_checkpoints(project, study)?;
    let gate = StudyResearchGate::new(services.clone(), project, study);
    let mut registry = ToolRegistry::default();
    registry.allow(ToolKind::AnalyzeEvidence);
    Ok(Engine::new(
        checkpoints,
        StudyResearchDriver::new(gate.clone()),
        gate,
        registry,
    ))
}

fn failed() -> ToolFailure {
    ToolFailure {
        code: "study_research_denied".into(),
        retryable: false,
        certainty: Certainty::NotApplied,
    }
}
fn uncertain() -> ToolFailure {
    ToolFailure {
        code: "study_research_effect_uncertain".into(),
        retryable: false,
        certainty: Certainty::UnknownEffect,
    }
}

impl ToolExecutor for StudyResearchDriver {
    fn execute(
        &self,
        ctx: &ExecutionContext,
        input: &StepInput,
    ) -> Result<ToolReceipt, ToolFailure> {
        self.guarded_input(ctx, input, true).map_err(|_| failed())?;
        match self.gate.services.store.reserve_agent_effect(
            self.gate.project,
            self.gate.study,
            &ctx.run_id,
            &ctx.step_id,
            &ctx.idempotency_key,
        ) {
            Ok(EffectClaim::Claimed(_)) => {}
            Ok(EffectClaim::Existing(_)) | Err(_) => return Err(uncertain()),
        }
        let output = match self.inspect(ctx, input) {
            Ok(result) => result,
            Err(_) => {
                // After reservation, failure might occur before or after a
                // durable file write. A missing/unreadable path is not proof
                // of non-execution (including on transient filesystem errors).
                // Fail closed and reconcile only an already-persisted artifact.
                if self
                    .gate
                    .services
                    .store
                    .record_agent_effect(
                        self.gate.project,
                        self.gate.study,
                        &ctx.run_id,
                        &ctx.idempotency_key,
                        EffectUpdate::Unknown,
                    )
                    .is_err()
                {
                    return Err(uncertain());
                }
                return Err(uncertain());
            }
        };
        self.gate
            .services
            .store
            .record_agent_effect(
                self.gate.project,
                self.gate.study,
                &ctx.run_id,
                &ctx.idempotency_key,
                EffectUpdate::Applied {
                    artifact_id: output.artifact_id.clone(),
                },
            )
            .map_err(|_| uncertain())?;
        Ok(ToolReceipt::Applied(output))
    }

    fn probe(
        &self,
        ctx: &ExecutionContext,
        input: &StepInput,
        _pending_effect_id: Option<&str>,
    ) -> Result<ProbeResult, ToolFailure> {
        let effect = self
            .gate
            .services
            .store
            .get_agent_effect(
                self.gate.project,
                self.gate.study,
                &ctx.run_id,
                &ctx.idempotency_key,
            )
            .map_err(|_| uncertain())?;
        let Some(effect) = effect else {
            return Ok(ProbeResult::Unknown);
        };
        if effect.step_id != ctx.step_id || effect.tool_kind != "AnalyzeEvidence" {
            return Ok(ProbeResult::Unknown);
        }
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
                    self.gate
                        .services
                        .store
                        .record_agent_effect(
                            self.gate.project,
                            self.gate.study,
                            &ctx.run_id,
                            &ctx.idempotency_key,
                            EffectUpdate::Applied {
                                artifact_id: output.artifact_id.clone(),
                            },
                        )
                        .map_err(|_| uncertain())?;
                }
                Ok(ProbeResult::Applied(output))
            }
        }
    }
}

const RESEARCH_RUN_DEADLINE_MS: u64 = 300_000;

/// IDs are the canonical Study attachments, not original video/research IDs.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyResearchSource {
    pub id: String,
    pub origin_id: String,
    pub kind: String,
    pub locator: String,
    pub available: bool,
    pub collected_at: String,
}

fn research_run_error(error: RuntimeError) -> RepositoryError {
    match error {
        RuntimeError::Store(scriptotar_agent::StoreError::NotFound) | RuntimeError::NotFound => {
            RepositoryError::NotFound("research run".into())
        }
        RuntimeError::Store(scriptotar_agent::StoreError::Unavailable) => storage(),
        RuntimeError::Store(scriptotar_agent::StoreError::Conflict)
        | RuntimeError::Store(scriptotar_agent::StoreError::Exists)
        | RuntimeError::WrongState
        | RuntimeError::StepMismatch => {
            RepositoryError::Conflict("research run changed or is not awaiting that action".into())
        }
        RuntimeError::PolicyDenied => not_found(),
        _ => invalid("research run is invalid or unavailable"),
    }
}

fn valid_research_run(run: &RunSnapshot, project: Uuid, study: Uuid) -> bool {
    run.plan.project_id == project.to_string()
        && run.plan.study_id == study.to_string()
        && run.plan.steps.len() == 1
        && run.plan.steps[0].id == "analyze_evidence"
        && matches!(&run.plan.steps[0].input, StepInput::AnalyzeEvidence { .. })
        && run.plan.steps[0].requires_approval
        && run.plan.steps[0].max_attempts == 1
        && run.plan.steps[0].estimate.tokens == 0
        && run.plan.steps[0].estimate.cost_microusd == 0
        && run.plan.limits.max_tokens == 0
        && run.plan.limits.max_cost_microusd == 0
}

impl AppServices {
    pub fn list_study_research_sources(
        &self,
        project: Uuid,
        study: Uuid,
    ) -> RepositoryResult<Vec<StudyResearchSource>> {
        scoped_study(self, project, study, &[])?;
        let sources = self.store.list_study_sources(project, study)?;
        self.require_active_project(project)?;
        Ok(sources
            .into_iter()
            .filter_map(|source| {
                if !matches!(
                    source.kind,
                    StudySourceKind::ResearchItem | StudySourceKind::Transcript
                ) {
                    return None;
                }
                Some(StudyResearchSource {
                    id: source.id.to_string(),
                    origin_id: source.reference_id?.to_string(),
                    kind: source.kind.as_str().to_owned(),
                    locator: source.locator,
                    available: source.available,
                    collected_at: source.collected_at,
                })
            })
            .collect())
    }

    /// Prepares one zero-cost, local AnalyzeEvidence step; no tool executes
    /// before an explicit approval decision.
    pub fn start_study_research(
        &self,
        project: Uuid,
        study: Uuid,
        source_ids: Vec<Uuid>,
    ) -> RepositoryResult<RunSnapshot> {
        let input = StepInput::AnalyzeEvidence {
            source_ids: source_ids.iter().map(ToString::to_string).collect(),
        };
        let ids = selected_ids(&input)?;
        scoped_study(self, project, study, &ids)?;
        let engine = build_study_research_engine(self, project, study)?;
        let now = now_ms();
        let plan = Plan {
            id: Uuid::new_v4().to_string(),
            project_id: project.to_string(),
            study_id: study.to_string(),
            steps: vec![StepPlan {
                id: "analyze_evidence".into(),
                input,
                estimate: Estimate {
                    tokens: 0,
                    cost_microusd: 0,
                    max_output_bytes: MAX_REPORT_BYTES as u64,
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
                max_elapsed_ms: RESEARCH_RUN_DEADLINE_MS,
                max_output_bytes: MAX_REPORT_BYTES as u64,
                max_cpu_ms: 5_000,
                max_peak_memory_mib: 128,
            },
        };
        let created = engine.create(plan, now).map_err(research_run_error)?;
        engine
            .tick(&created.plan.id, now)
            .map_err(research_run_error)
    }

    pub fn list_study_research_runs(
        &self,
        project: Uuid,
        study: Uuid,
    ) -> RepositoryResult<Vec<RunSnapshot>> {
        let engine = build_study_research_engine(self, project, study)?;
        let mut results = Vec::new();
        for run_id in engine.store.list_run_ids(50).map_err(|_| storage())? {
            let run = engine.get(&run_id).map_err(research_run_error)?;
            if valid_research_run(&run, project, study) {
                results.push(run);
                if results.len() == 25 {
                    break;
                }
            }
        }
        self.require_active_project(project)?;
        Ok(results)
    }

    pub fn decide_study_research(
        &self,
        project: Uuid,
        study: Uuid,
        run_id: &str,
        action: &str,
    ) -> RepositoryResult<RunSnapshot> {
        if !scriptotar_agent::valid_id(run_id) {
            return Err(invalid("invalid research run ID"));
        }
        let engine = build_study_research_engine(self, project, study)?;
        let original = engine.get(run_id).map_err(research_run_error)?;
        if !valid_research_run(&original, project, study) {
            return Err(not_found());
        }
        let now = now_ms();
        let next = match action {
            "approve" => {
                let approved = engine
                    .approve(run_id, "analyze_evidence", now)
                    .map_err(research_run_error)?;
                engine
                    .tick(&approved.plan.id, now)
                    .map_err(research_run_error)?
            }
            "deny" => engine
                .deny(run_id, "analyze_evidence", now)
                .map_err(research_run_error)?,
            "cancel" => engine.cancel(run_id, now).map_err(research_run_error)?,
            "recover" => {
                let recovered = engine.recover(run_id, now).map_err(research_run_error)?;
                if recovered.state == scriptotar_agent::RunState::Blocked
                    && recovered.steps.first().is_some_and(|step| {
                        matches!(
                            step.state,
                            scriptotar_agent::StepState::Running
                                | scriptotar_agent::StepState::AwaitingResult
                        )
                    })
                {
                    engine.reconcile(run_id, now).map_err(research_run_error)?
                } else {
                    recovered
                }
            }
            _ => return Err(invalid("unsupported research action")),
        };
        if !valid_research_run(&next, project, study) {
            return Err(not_found());
        }
        self.require_active_project(project)?;
        Ok(next)
    }

    /// The artifact remains readable only while its applied receipt and
    /// original attached source revisions are still authoritative.
    pub fn read_study_research_report(
        &self,
        project: Uuid,
        study: Uuid,
        artifact: Uuid,
    ) -> RepositoryResult<StudyResearchReport> {
        read_study_research_artifact(self, project, study, artifact)
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
        let study = services
            .store
            .create_study(project, "Study", "")
            .unwrap()
            .id;
        let other = Uuid::new_v4();
        let input = StepInput::AnalyzeEvidence {
            source_ids: vec![other.to_string()],
        };
        assert!(selected_ids(&input).is_ok());
        assert!(scoped_study(&services, project, study, &[other]).is_err());
        assert!(scoped_study(&services, Uuid::new_v4(), study, &[other]).is_err());
        let manual = services
            .store
            .add_study_source(
                project,
                study,
                &StudySourceInput {
                    kind: StudySourceKind::Manual,
                    reference_id: None,
                    locator: Some("pasted note".into()),
                    manual_text: Some("No automatic research on pasted notes".into()),
                    source_revision: None,
                    uncertainty: None,
                },
            )
            .unwrap();
        assert!(scoped_study(&services, project, study, &[manual.id]).is_err());
    }

    #[test]
    fn approved_agent_research_persists_effect_and_can_reopen_result() {
        use scriptotar_agent::{CheckpointStore, RunState};
        use scriptotar_core::ResearchRepository;
        let dir = TempDir::new().unwrap();
        let project;
        let study;
        let artifact;
        let run_id;
        {
            let services = AppServices::new(dir.path()).unwrap();
            project = services.active_project_id().unwrap();
            study = services
                .store
                .create_study(project, "Research", "")
                .unwrap()
                .id;
            let research_id = Uuid::new_v4();
            let recorded = services
                .store
                .upsert_research_items(&[ResearchItem {
                    id: research_id,
                    project_id: project,
                    creator_id: None,
                    source_url: "https://www.youtube.com/watch?v=agent03proof".into(),
                    platform: "YouTube".into(),
                    title: Some("Recorded metadata".into()),
                    view_count: Some(0),
                    like_count: None,
                    comment_count: None,
                    published_at: Some("2026-11-10".into()),
                    duration_seconds: None,
                    raw_json: None,
                    scanned_at: "2026-11-15T00:00:00Z".into(),
                }])
                .unwrap();
            let attached = services
                .store
                .add_study_source(
                    project,
                    study,
                    &StudySourceInput {
                        kind: StudySourceKind::ResearchItem,
                        reference_id: Some(recorded[0].id),
                        locator: None,
                        manual_text: None,
                        source_revision: None,
                        uncertainty: None,
                    },
                )
                .unwrap();
            let sources = services
                .list_study_research_sources(project, study)
                .unwrap();
            assert_eq!(sources.len(), 1);
            assert_eq!(sources[0].origin_id, recorded[0].id.to_string());
            assert_eq!(sources[0].id, attached.id.to_string());
            let pending = services
                .start_study_research(project, study, vec![attached.id])
                .unwrap();
            assert_eq!(pending.state, RunState::AwaitingApproval);
            run_id = pending.plan.id.clone();
            assert!(services
                .store
                .get_agent_effect(project, study, &run_id, &pending.steps[0].idempotency_key)
                .unwrap()
                .is_none());
            assert_eq!(
                services
                    .list_study_research_runs(project, study)
                    .unwrap()
                    .len(),
                1
            );
            assert!(services
                .decide_study_research(Uuid::new_v4(), study, &run_id, "approve")
                .is_err());
            let completed = services
                .decide_study_research(project, study, &run_id, "approve")
                .unwrap();
            assert_eq!(completed.state, RunState::Completed);
            assert!(services
                .decide_study_research(project, study, &run_id, "approve")
                .is_err());
            artifact =
                Uuid::parse_str(&completed.steps[0].output.as_ref().unwrap().artifact_id).unwrap();
            let report = read_study_research_artifact(&services, project, study, artifact).unwrap();
            assert_eq!(report.sources.len(), 1);
            assert_eq!(report.sources[0].study_source_id, attached.id);
            assert_eq!(report.sources[0].metrics.as_ref().unwrap().views, Some(0));
            assert_eq!(report.sources[0].metrics.as_ref().unwrap().likes, None);
        }
        let restarted = AppServices::new(dir.path()).unwrap();
        let report = restarted
            .read_study_research_report(project, study, artifact)
            .unwrap();
        assert_eq!(report.study_id, study);
        assert_eq!(
            restarted
                .list_study_research_runs(project, study)
                .unwrap()
                .len(),
            1
        );
        assert!(restarted
            .read_study_research_report(Uuid::new_v4(), study, artifact)
            .is_err());
        let ledger = restarted
            .store
            .get_agent_effect(
                project,
                study,
                &run_id,
                &restarted
                    .store
                    .scoped_agent_checkpoints(project, study)
                    .unwrap()
                    .load(&run_id)
                    .unwrap()
                    .unwrap()
                    .steps[0]
                    .idempotency_key,
            )
            .unwrap()
            .unwrap();
        assert_eq!(ledger.status, EffectStatus::Applied);
        assert_eq!(
            ledger.artifact_id.as_deref(),
            Some(artifact.to_string().as_str())
        );
        assert!(read_study_research_artifact(&restarted, Uuid::new_v4(), study, artifact).is_err());
    }

    #[test]
    fn malformed_transcript_segments_never_gain_synthetic_timecodes() {
        let transcript_id = Uuid::new_v4();
        let malformed = parse_transcript(
            transcript_id,
            "Untimed transcript only",
            Some("{not valid json"),
            "2026-11-16T00:00:00Z".into(),
        );
        assert!(malformed.partial);
        assert!(malformed.segments.is_empty());
        assert_eq!(
            malformed.untimed_text.as_deref(),
            Some("Untimed transcript only")
        );
        let invalid_times = parse_transcript(
            transcript_id,
            "A plain original transcript",
            Some(r#"[{"index":1,"start":9.0,"end":2.0,"text":"False time"}]"#),
            "2026-11-16T00:00:00Z".into(),
        );
        assert!(invalid_times.partial);
        assert!(invalid_times.segments.is_empty());
    }

    #[test]
    fn three_exact_selected_sources_retain_provenance_and_invalidate_transcript_edits() {
        use scriptotar_core::ResearchRepository;
        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .store
            .create_study(project, "Multiple sources", "")
            .unwrap()
            .id;
        let research_items = (0..2)
            .map(|index| ResearchItem {
                id: Uuid::new_v4(),
                project_id: project,
                creator_id: None,
                source_url: format!("https://www.youtube.com/watch?v=agent03source{index}"),
                platform: "YouTube".into(),
                title: None,
                view_count: if index == 0 { Some(0) } else { None },
                like_count: None,
                comment_count: None,
                published_at: None,
                duration_seconds: None,
                raw_json: None,
                scanned_at: "2026-11-15T00:00:00Z".into(),
            })
            .collect::<Vec<_>>();
        services
            .store
            .upsert_research_items(&research_items)
            .unwrap();
        let attached_research = research_items
            .iter()
            .map(|item| {
                services
                    .store
                    .add_study_source(
                        project,
                        study,
                        &StudySourceInput {
                            kind: StudySourceKind::ResearchItem,
                            reference_id: Some(item.id),
                            locator: None,
                            manual_text: None,
                            source_revision: None,
                            uncertainty: None,
                        },
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();

        let db = Connection::open(services.store.path()).unwrap();
        db.pragma_update(None, "foreign_keys", true).unwrap();
        let raw_source = Uuid::new_v4();
        let media = Uuid::new_v4();
        let transcript = Uuid::new_v4();
        let timestamp = "2026-11-15T00:00:00Z";
        db.execute(
            "INSERT INTO sources(id,project_id,source_type,locator,title,created_at)
             VALUES(?1,?2,'local_file','/mock/research-proof.mp4','Research proof',?3)",
            params![raw_source.to_string(), project.to_string(), timestamp],
        )
        .unwrap();
        db.execute(
            "INSERT INTO media(id,source_id,local_path,duration_seconds,created_at)
             VALUES(?1,?2,'/mock/research-proof.mp4',6.0,?3)",
            params![media.to_string(), raw_source.to_string(), timestamp],
        )
        .unwrap();
        let segments = serde_json::json!([
            {"index":0,"start":0.0,"end":2.0,"text":"How does this work?"},
            {"index":1,"start":2.0,"end":5.0,"text":"But there is another way."},
            {"index":2,"start":5.0,"end":6.0,"text":"Subscribe to follow."}
        ])
        .to_string();
        db.execute(
            "INSERT INTO transcripts(id,media_id,language,text,segments_json,created_at,updated_at)
             VALUES(?1,?2,'en','How does this work? But there is another way. Subscribe to follow.',?3,?4,?4)",
            params![transcript.to_string(), media.to_string(), segments, timestamp],
        )
        .unwrap();
        let attached_transcript = services
            .store
            .add_study_source(
                project,
                study,
                &StudySourceInput {
                    kind: StudySourceKind::Transcript,
                    reference_id: Some(transcript),
                    locator: None,
                    manual_text: None,
                    source_revision: None,
                    uncertainty: None,
                },
            )
            .unwrap();
        let selected = vec![
            attached_research[1].id,
            attached_transcript.id,
            attached_research[0].id,
        ];
        let (revision, report) = inspect_selected(
            &services,
            project,
            study,
            &selected,
            ObservationTimestamp::parse_rfc3339("2026-11-16T00:00:00Z").unwrap(),
        )
        .unwrap();
        assert_eq!(report.sources.len(), 3);
        assert!(report.source_text_is_untrusted_data);
        let transcript_finding = report
            .sources
            .iter()
            .find(|finding| finding.study_source_id == attached_transcript.id)
            .unwrap();
        assert_eq!(
            transcript_finding.origin_revision.as_deref(),
            Some(timestamp)
        );
        assert!(!transcript_finding.observations.is_empty());
        assert!(transcript_finding.observations.iter().all(|observation| {
            observation.evidence.study_source_id == attached_transcript.id
                && observation.evidence.transcript_id == transcript
                && observation.evidence.start_ms.is_some()
                && observation.evidence.end_ms.is_some()
        }));
        let first = report
            .sources
            .iter()
            .find(|finding| finding.study_source_id == attached_research[0].id)
            .unwrap();
        let second = report
            .sources
            .iter()
            .find(|finding| finding.study_source_id == attached_research[1].id)
            .unwrap();
        assert_eq!(first.metrics.as_ref().unwrap().views, Some(0));
        assert_eq!(second.metrics.as_ref().unwrap().views, None);

        let artifact = Uuid::new_v4();
        let envelope = ReportEnvelope {
            project_id: project,
            study_id: study,
            study_revision: revision,
            run_id: "three-source-test".into(),
            step_id: "analyze".into(),
            idempotency_key: "three-source-key".into(),
            report,
        };
        write_envelope(&services, artifact, &serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert!(checked_report(&services, project, study, artifact).is_ok());
        assert!(read_study_research_artifact(&services, project, study, artifact).is_err());
        let other_study = services
            .store
            .create_study(project, "Other study", "")
            .unwrap()
            .id;
        assert!(inspect_selected(
            &services,
            project,
            other_study,
            &selected,
            ObservationTimestamp::parse_rfc3339("2026-11-16T00:00:00Z").unwrap(),
        )
        .is_err());
        db.execute(
            "UPDATE transcripts SET updated_at='2026-11-16T01:00:00Z' WHERE id=?1",
            params![transcript.to_string()],
        )
        .unwrap();
        assert!(checked_report(&services, project, study, artifact).is_err());
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
            study = services
                .store
                .create_study(project, "Study", "")
                .unwrap()
                .id;
            research_id = Uuid::new_v4();
            // The real source is inserted through the existing research repository.
            let item = ResearchItem {
                id: research_id,
                project_id: project,
                creator_id: None,
                source_url: "https://www.youtube.com/watch?v=abc123".into(),
                platform: "YouTube".into(),
                title: None,
                view_count: None,
                like_count: None,
                comment_count: None,
                published_at: None,
                duration_seconds: None,
                raw_json: None,
                scanned_at: "2026-11-15T00:00:00Z".into(),
            };
            use scriptotar_core::ResearchRepository;
            services.store.upsert_research_items(&[item]).unwrap();
            source = services
                .store
                .add_study_source(
                    project,
                    study,
                    &StudySourceInput {
                        kind: StudySourceKind::ResearchItem,
                        reference_id: Some(research_id),
                        locator: None,
                        manual_text: None,
                        source_revision: None,
                        uncertainty: None,
                    },
                )
                .unwrap()
                .id;
            let (_, report) = inspect_selected(
                &services,
                project,
                study,
                &[source],
                ObservationTimestamp::parse_rfc3339("2026-11-16T00:00:00Z").unwrap(),
            )
            .unwrap();
            let envelope = ReportEnvelope {
                project_id: project,
                study_id: study,
                study_revision: services.store.get_study(project, study).unwrap().revision,
                run_id: "test-run".into(),
                step_id: "research".into(),
                idempotency_key: "key".into(),
                report,
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
        )
        .unwrap();
        assert!(checked_report(&reopened, project, study, artifact).is_err());
        db.execute(
            "DELETE FROM research_items WHERE id=?1 AND project_id=?2",
            params![research_id.to_string(), project.to_string()],
        )
        .unwrap();
        assert!(
            !reopened
                .store
                .get_study_source(project, study, source)
                .unwrap()
                .available
        );
        assert!(checked_report(&reopened, project, study, artifact).is_err());
        reopened.store.archive_study(project, study).unwrap();
        assert!(checked_report(&reopened, project, study, artifact).is_err());
    }
}
