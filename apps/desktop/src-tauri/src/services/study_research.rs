//! Agent 03 research adapter. Agent 00 registers this module and the
//! AnalyzeEvidence step; no direct IPC, provider or network execution here.
//! All source IDs below are canonical study_sources.id, NOT platform URLs.

use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use scriptotar_agent::{
    Certainty, Engine, Estimate, ExecutionContext, Limits, Plan, PolicyGate, ProbeResult,
    RunSnapshot, RuntimeError, StepInput, StepOutput, StepPlan, ToolExecutor, ToolFailure,
    ToolKind, ToolReceipt, ToolRegistry,
};
use scriptotar_ai::writing::{
    EvidenceKind, EvidenceLocator, EvidenceRef, EvidenceStatus, FindingKind, ResearchFinding,
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
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::AppServices;

// SQL bounds origin metadata before loading user-controlled text into Rust.
const MAX_RESEARCH_TEXT_BYTES: i64 = 4_096;
const MAX_RESEARCH_PLATFORM_BYTES: i64 = 128;
const MAX_RESEARCH_DATE_BYTES: i64 = 128;
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
    let db = read_db(services)?;
    let active: bool = db
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
        let kind = match attached.kind {
            StudySourceKind::ResearchItem => StudyResearchKind::ResearchItem,
            StudySourceKind::Transcript => StudyResearchKind::Transcript,
            _ => {
                return Err(invalid(
                    "research inspection requires a research item or transcript",
                ));
            }
        };
        let origin = attached.reference_id.ok_or_else(not_found)?;
        if !attached.available
            || origin_content_sha256(&db, project, study, *id, origin, kind)?.is_none()
        {
            return Err(not_found());
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
           AND ss.research_id=?4 AND st.archived_at IS NULL AND p.archived_at IS NULL
            AND length(CAST(r.source_url AS BLOB)) <= ?5
            AND length(CAST(r.platform AS BLOB)) <= ?6
            AND (r.title IS NULL OR length(CAST(r.title AS BLOB)) <= ?5)
            AND length(CAST(r.scanned_at AS BLOB)) <= ?7
            AND (r.published_at IS NULL OR length(CAST(r.published_at AS BLOB)) <= ?7)
            AND (r.creator_id IS NULL OR length(CAST(r.creator_id AS BLOB)) <= 64)",
            params![
                project.to_string(),
                study.to_string(),
                source.to_string(),
                origin.to_string(),
                MAX_RESEARCH_TEXT_BYTES,
                MAX_RESEARCH_PLATFORM_BYTES,
                MAX_RESEARCH_DATE_BYTES,
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

/// A content-bound digest from the same bounded, project/Study-scoped rows
/// used by research analysis. Revision timestamps alone are insufficient.
fn origin_content_sha256(
    db: &Connection,
    project: Uuid,
    study: Uuid,
    source: Uuid,
    origin: Uuid,
    kind: StudyResearchKind,
) -> RepositoryResult<Option<String>> {
    let bytes = match kind {
        StudyResearchKind::ResearchItem => research_row(db, project, study, source, origin)?
            .map(|item| serde_json::to_vec(&item).map_err(|_| storage()))
            .transpose()?,
        StudyResearchKind::Transcript => transcript_row(db, project, study, source, origin)?
            .map(|row| serde_json::to_vec(&row).map_err(|_| storage()))
            .transpose()?,
    };
    Ok(bytes.map(|value| format!("{:x}", Sha256::digest(&value))))
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
    let mut bindings = HashMap::new();
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
        let sha256 = origin_content_sha256(&db, project, study, *id, reference_id, kind)?
            .ok_or_else(not_found)?;
        bindings.insert(
            *id,
            (
                attachment.fingerprint.clone(),
                attachment.source_revision.clone(),
                sha256,
            ),
        );
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
    let mut report = analyze_study_sources(project, study, analyzed_at, sources)
        .map_err(|_| invalid("invalid selected research data"))?;
    for finding in &mut report.sources {
        let (fingerprint, revision, sha256) = bindings
            .remove(&finding.study_source_id)
            .ok_or_else(storage)?;
        finding.attachment_fingerprint = Some(fingerprint);
        finding.attachment_revision = revision;
        finding.origin_sha256 = Some(sha256);
    }
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

/// Reads at most one byte past the hard artifact limit so a malicious or
/// corrupted file cannot allocate its entire contents before admission.
fn read_bounded_report_file(path: &Path) -> RepositoryResult<Vec<u8>> {
    let file = fs::File::open(path).map_err(|_| not_found())?;
    let mut bytes = Vec::new();
    file.take((MAX_REPORT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| storage())?;
    if bytes.len() > MAX_REPORT_BYTES {
        return Err(storage());
    }
    Ok(bytes)
}

fn load_envelope(services: &AppServices, artifact_id: Uuid) -> RepositoryResult<ReportEnvelope> {
    let path = artifact_path(services, artifact_id)?;
    let bytes = read_bounded_report_file(&path)?;
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
        let existing = read_bounded_report_file(&path)?;
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
        drop(handle);
        // An existence check followed by rename can replace another process's
        // completed artifact on Unix. Publish through a same-directory hard
        // link instead: creation is atomic and fails if the ID already exists.
        // On a same-byte racing publication, make the existing link durable
        // before its caller is allowed to record an Applied receipt.
        match fs::hard_link(&temporary, &path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if read_bounded_report_file(&path)? != data {
                    return Err(RepositoryError::Conflict(
                        "research artifact ID already exists".into(),
                    ));
                }
            }
            Err(_) => return Err(storage()),
        }
        fs::remove_file(&temporary).map_err(|_| storage())?;
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
    let saved = load_envelope(services, artifact)?;
    checked_report_envelope(services, project, study, saved)
}

/// Inspect the exact already-read artifact bytes, including for the user-visible
/// path that must authorize the checkpoint and receipt *before* local analysis.
/// Do not reopen the path after authorization: a local file may be replaced.
fn checked_report_envelope(
    services: &AppServices,
    project: Uuid,
    study: Uuid,
    saved: ReportEnvelope,
) -> RepositoryResult<ReportEnvelope> {
    services.require_active_project(project)?;
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
        let correct_kind = matches!(
            (source.kind, finding.kind),
            (
                StudySourceKind::ResearchItem,
                StudyResearchKind::ResearchItem
            ) | (StudySourceKind::Transcript, StudyResearchKind::Transcript)
        );
        if !correct_kind
            || !source.available
            || source.reference_id != Some(finding.reference_id)
            || source.locator != finding.locator
            || source.collected_at != finding.collected_at
            || Some(source.fingerprint.as_str()) != finding.attachment_fingerprint.as_deref()
            || source.source_revision != finding.attachment_revision
            || !matches!(
                finding.access,
                StudyResearchAccess::Available | StudyResearchAccess::Partial
            )
        {
            return Err(not_found());
        }
        let actual_sha256 = origin_content_sha256(
            &db,
            project,
            study,
            finding.study_source_id,
            finding.reference_id,
            finding.kind,
        )?;
        if actual_sha256.as_deref() != finding.origin_sha256.as_deref()
            || finding.origin_sha256.is_none()
        {
            return Err(RepositoryError::Conflict(
                "source content changed since research analysis".into(),
            ));
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
    // An artifact file is mutable local data, not an authority for evidence.
    // Regenerate its deterministic contents from currently authorized sources
    // so edited JSON cannot inject fabricated claims or citations.
    let (_, rebuilt) = inspect_selected(services, project, study, &ids, saved.report.analyzed_at)?;
    if rebuilt != saved.report {
        return Err(RepositoryError::Conflict(
            "research artifact does not match canonical source material".into(),
        ));
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
    services.require_active_project(project)?;
    let saved = load_envelope(services, artifact)?;
    if saved.project_id != project || saved.study_id != study {
        return Err(not_found());
    }
    // An Applied effect by itself does not prove the *selected sources*
    // authorized by the human: verify the persisted run plan, its exact
    // approved step and its output, not just a mutable report artifact.
    let engine = build_study_research_engine(services, project, study)?;
    let run = engine.get(&saved.run_id).map_err(research_run_error)?;
    let report_ids: Vec<Uuid> = saved
        .report
        .sources
        .iter()
        .map(|source| source.study_source_id)
        .collect();
    let approved_ids = run
        .plan
        .steps
        .first()
        .and_then(|step| selected_ids(&step.input).ok());
    // The observation time is factual provenance: it must be consistent
    // with the persisted, user-approved actual dispatch, not merely an
    // arbitrary timestamp in a locally editable report JSON file.
    let dispatched_ms = run
        .events
        .iter()
        .find(|event| {
            event.kind == scriptotar_agent::AuditKind::Dispatched
                && event.step_id.as_deref() == Some(saved.step_id.as_str())
        })
        .map(|event| event.at_ms);
    let analyzed_ms = u64::try_from(saved.report.analyzed_at.unix_seconds)
        .ok()
        .and_then(|seconds| seconds.checked_mul(1_000))
        .and_then(|ms| ms.checked_add(u64::from(saved.report.analyzed_at.nanosecond / 1_000_000)));
    if !valid_research_run(&run, project, study)
        || run.state != scriptotar_agent::RunState::Completed
        || !run.steps[0].approved
        || saved.report.analyzed_at.nanosecond >= 1_000_000_000
        || !matches!(
            (dispatched_ms, analyzed_ms),
            (Some(dispatched), Some(analyzed)) if dispatched.abs_diff(analyzed) <= 10_000
        )
        || approved_ids.as_deref() != Some(report_ids.as_slice())
        || run.steps.first().is_none_or(|step| {
            step.idempotency_key != saved.idempotency_key
                || step
                    .output
                    .as_ref()
                    .is_none_or(|output| output.artifact_id != artifact.to_string())
        })
        || run.plan.steps[0].id != saved.step_id
    {
        return Err(not_found());
    }

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
    // No report/source analysis happened before the completed checkpoint,
    // selected source IDs, step output and receipt were checked. Use the same
    // captured bytes so the authorization cannot be raced by swapping files.
    let validated = checked_report_envelope(services, project, study, saved)?;
    Ok(validated.report)
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

/// Read-only handoff for Agent 05's typed WritingInput. All text originates
/// from a verified, approved and replay-checked canonical research report.
/// A source without transcript evidence does not receive a fabricated citation.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ResearchWritingSource {
    pub study_source_id: Uuid,
    pub origin_id: Uuid,
    pub source_sha256: String,
    pub attachment_fingerprint: String,
    pub attachment_revision: Option<String>,
    pub collected_at: String,
    pub access: StudyResearchAccess,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ResearchWritingContext {
    pub project_id: Uuid,
    pub study_id: Uuid,
    pub sources: Vec<ResearchWritingSource>,
    pub evidence: Vec<EvidenceRef>,
    pub findings: Vec<ResearchFinding>,
    /// Number of distinct actual origin transcripts with attributable text.
    pub attributed_source_count: usize,
    /// Agent 05 must also validate the canonical Study brief and current source
    /// set. This is only the minimum local-evidence admission condition.
    pub meets_three_source_evidence_floor: bool,
    pub limitations: Vec<String>,
    pub source_text_is_untrusted_data: bool,
}

fn research_writing_from_report(
    artifact: Uuid,
    report: &StudyResearchReport,
) -> RepositoryResult<ResearchWritingContext> {
    if !report.source_text_is_untrusted_data {
        return Err(not_found());
    }
    let mut sources = Vec::new();
    let mut evidence = Vec::new();
    let mut findings = Vec::new();
    let mut limitations = report.limitations.clone();
    let mut truncated = false;
    for source in &report.sources {
        let content_hash = source.origin_sha256.clone().ok_or_else(not_found)?;
        let fingerprint = source
            .attachment_fingerprint
            .clone()
            .ok_or_else(not_found)?;
        if !matches!(
            source.access,
            StudyResearchAccess::Available | StudyResearchAccess::Partial
        ) {
            return Err(not_found());
        }
        sources.push(ResearchWritingSource {
            study_source_id: source.study_source_id,
            origin_id: source.reference_id,
            source_sha256: content_hash,
            attachment_fingerprint: fingerprint,
            attachment_revision: source.attachment_revision.clone(),
            collected_at: source.collected_at.clone(),
            access: source.access,
        });
        limitations.extend(
            source
                .limitations
                .iter()
                .map(|reason| format!("Source {}: {reason}", source.study_source_id)),
        );
        for (index, observation) in source.observations.iter().enumerate() {
            if evidence.len() >= 40 || findings.len() >= 30 {
                truncated = true;
                break;
            }
            let anchor = &observation.evidence;
            if anchor.study_source_id != source.study_source_id
                || anchor.reference_id != source.reference_id
                || anchor.transcript_id != source.reference_id
                || anchor.locator != source.locator
                || Some(anchor.transcript_revision.as_str()) != source.origin_revision.as_deref()
                || anchor.excerpt.chars().count() > 160
            {
                return Err(not_found());
            }
            let section = match (anchor.segment_id.as_deref(), anchor.start_ms, anchor.end_ms) {
                (Some(segment), Some(start), Some(end)) if start < end => {
                    format!("Transcript segment {segment} ({start}-{end} ms)")
                }
                (None, None, None) => {
                    "Untimed transcript excerpt; no verified segment/timecode".into()
                }
                _ => return Err(not_found()),
            };
            let id = format!("research:{artifact}:{}:{index}", source.study_source_id);
            let is_literal = observation.beat.is_some();
            evidence.push(EvidenceRef {
                id: id.clone(),
                locator: EvidenceLocator {
                    source_id: source.reference_id.to_string(),
                    canonical_url: None,
                    timestamp_seconds: anchor
                        .start_ms
                        .and_then(|ms| u32::try_from(ms / 1_000).ok()),
                    document_section: Some(section),
                    collected_at: Some(source.collected_at.clone()),
                },
                excerpt: anchor.excerpt.clone(),
                kind: EvidenceKind::SocialTranscript,
                status: if is_literal {
                    EvidenceStatus::Attributed
                } else {
                    EvidenceStatus::Unverified
                },
            });
            findings.push(ResearchFinding {
                id: format!(
                    "research-finding:{artifact}:{}:{index}",
                    source.study_source_id
                ),
                kind: FindingKind::CreativePattern,
                statement: observation.statement.clone(),
                evidence_ids: vec![id],
                limitations: Some(format!(
                    "Transcript-only {}; no independent factual verification. {}",
                    if is_literal {
                        "observation"
                    } else {
                        "hypothesis"
                    },
                    observation.limitations.join(" ")
                )),
            });
        }
    }
    if truncated {
        limitations.push(
            "Research-to-writing evidence was bounded to 40 excerpts and 30 hypotheses.".into(),
        );
    }
    let attributed_source_count = evidence
        .iter()
        .filter(|item| item.status == EvidenceStatus::Attributed)
        .map(|item| item.locator.source_id.as_str())
        .collect::<HashSet<_>>()
        .len();
    let meets_three_source_evidence_floor = attributed_source_count >= 3;
    if !meets_three_source_evidence_floor {
        limitations.push(
            "Fewer than three distinct source transcripts have attributable excerpts; metadata-only source selections do not qualify for three-source writing.".into(),
        );
    }
    limitations.push(
        "Never infer visual frames, engagement outcomes, retention or verified factual claims from this research."
            .into(),
    );
    Ok(ResearchWritingContext {
        project_id: report.project_id,
        study_id: report.study_id,
        sources,
        evidence,
        findings,
        attributed_source_count,
        meets_three_source_evidence_floor,
        limitations,
        source_text_is_untrusted_data: true,
    })
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
        // Persisted checkpoint data is not trusted to have a complete step
        // record; reject malformed runs before reading run.steps[0].
        && run.steps.len() == 1
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

    /// Agent 05 calls this after fresh source/receipt verification, not with
    /// caller-constructed DTO text. It is not a provider request or proof that
    /// the three-source writing evidence quota has been satisfied.
    #[allow(dead_code)]
    pub(super) fn research_writing_context(
        &self,
        project: Uuid,
        study: Uuid,
        artifact: Uuid,
    ) -> RepositoryResult<ResearchWritingContext> {
        let report = read_study_research_artifact(self, project, study, artifact)?;
        research_writing_from_report(artifact, &report)
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
            // Oversized upstream metadata must fail SQL admission, rather
            // than materializing unbounded text during planning or analysis.
            let db = Connection::open(services.store.path()).unwrap();
            db.execute(
                "UPDATE research_items SET title=?1 WHERE id=?2 AND project_id=?3",
                params![
                    "x".repeat(32_768),
                    research_id.to_string(),
                    project.to_string()
                ],
            )
            .unwrap();
            assert!(services
                .start_study_research(project, study, vec![attached.id])
                .is_err());
            db.execute(
                "UPDATE research_items SET title='Recorded metadata' WHERE id=?1 AND project_id=?2",
                params![research_id.to_string(), project.to_string()],
            )
            .unwrap();
            drop(db);
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
            let mut malformed_checkpoint = completed.clone();
            malformed_checkpoint.steps.clear();
            assert!(!valid_research_run(&malformed_checkpoint, project, study));
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
    fn concurrent_report_publication_is_atomic_and_never_replaces_a_winner() {
        use std::sync::{Arc, Barrier};

        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        for _ in 0..8 {
            let artifact = Uuid::new_v4();
            let left_service = services.clone();
            let right_service = services.clone();
            let barrier = Arc::new(Barrier::new(3));
            let left_gate = Arc::clone(&barrier);
            let right_gate = Arc::clone(&barrier);
            let left = std::thread::spawn(move || {
                left_gate.wait();
                write_envelope(&left_service, artifact, b"left report bytes")
            });
            let right = std::thread::spawn(move || {
                right_gate.wait();
                write_envelope(&right_service, artifact, b"right report bytes")
            });
            barrier.wait();
            let left_result = left.join().unwrap();
            let right_result = right.join().unwrap();
            assert_ne!(
                left_result.is_ok(),
                right_result.is_ok(),
                "exactly one different-byte publication may win"
            );
            let (winner, loser) = if left_result.is_ok() {
                (b"left report bytes".as_slice(), b"right report bytes".as_slice())
            } else {
                (b"right report bytes".as_slice(), b"left report bytes".as_slice())
            };
            let path = artifact_path(&services, artifact).unwrap();
            assert_eq!(fs::read(&path).unwrap(), winner);
            assert!(write_envelope(&services, artifact, winner).is_ok());
            assert!(write_envelope(&services, artifact, loser).is_err());
            assert_eq!(fs::read(&path).unwrap(), winner);
            let pending = fs::read_dir(path.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|item| {
                    item.path().extension().is_some_and(|ext| ext == "pending")
                })
                .count();
            assert_eq!(pending, 0, "do not strand pending artifacts");
        }
    }

    #[test]
    fn oversized_cached_artifact_is_rejected_before_full_read() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("oversized.json");
        let file = fs::File::create(&path).unwrap();
        file.set_len((MAX_REPORT_BYTES as u64) + 4_096).unwrap();
        drop(file);
        assert!(read_bounded_report_file(&path).is_err());

        let exact = dir.path().join("exact-limit.json");
        fs::write(&exact, vec![b'x'; MAX_REPORT_BYTES]).unwrap();
        assert_eq!(
            read_bounded_report_file(&exact).unwrap().len(),
            MAX_REPORT_BYTES
        );
        assert!(read_bounded_report_file(&dir.path().join("missing.json")).is_err());
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
        use scriptotar_agent::RunState;
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
        // Genuine approved v8 run with three real selected attachments.
        // Restart from disk is verified below after dropping the live service.
        let awaiting = services
            .start_study_research(project, study, selected.clone())
            .unwrap();
        assert_eq!(awaiting.state, RunState::AwaitingApproval);
        let completed = services
            .decide_study_research(project, study, &awaiting.plan.id, "approve")
            .unwrap();
        assert_eq!(completed.state, RunState::Completed);
        let approved_artifact =
            Uuid::parse_str(&completed.steps[0].output.as_ref().unwrap().artifact_id).unwrap();
        let approved_bridge = services
            .research_writing_context(project, study, approved_artifact)
            .unwrap();
        assert_eq!(approved_bridge.sources.len(), 3);
        assert!(!approved_bridge.evidence.is_empty());
        assert_eq!(approved_bridge.attributed_source_count, 1);
        assert!(!approved_bridge.meets_three_source_evidence_floor);
        assert!(approved_bridge
            .evidence
            .iter()
            .all(|observation| { observation.locator.source_id == transcript.to_string() }));
        assert!(services
            .research_writing_context(Uuid::new_v4(), study, approved_artifact)
            .is_err());

        // Even when all replacement observations are factually derived from
        // canonical sources, an artifact cannot silently narrow/change the
        // three sources for which the user approved this exact run.
        let approved_path = artifact_path(&services, approved_artifact).unwrap();
        let original_approved_bytes = fs::read(&approved_path).unwrap();
        let mut swapped: ReportEnvelope = serde_json::from_slice(&original_approved_bytes).unwrap();
        let (_, unapproved_report) = inspect_selected(
            &services,
            project,
            study,
            &selected[..2],
            ObservationTimestamp::parse_rfc3339("2026-11-16T00:00:00Z").unwrap(),
        )
        .unwrap();
        swapped.report = unapproved_report;
        fs::write(&approved_path, serde_json::to_vec(&swapped).unwrap()).unwrap();
        assert!(services
            .read_study_research_report(project, study, approved_artifact)
            .is_err());
        fs::write(&approved_path, &original_approved_bytes).unwrap();
        assert!(services
            .read_study_research_report(project, study, approved_artifact)
            .is_ok());

        // The report time controls metric freshness. Recreating a perfect
        // canonical report for an invented day must still fail the immutable
        // dispatch-time provenance check, even when source IDs all match.
        let mut shifted: ReportEnvelope =
            serde_json::from_slice(&fs::read(&approved_path).unwrap()).unwrap();
        let earlier = ObservationTimestamp {
            unix_seconds: shifted.report.analyzed_at.unix_seconds - 86_400,
            nanosecond: shifted.report.analyzed_at.nanosecond,
        };
        shifted.report = inspect_selected(&services, project, study, &selected, earlier)
            .unwrap()
            .1;
        fs::write(&approved_path, serde_json::to_vec(&shifted).unwrap()).unwrap();
        assert!(services
            .read_study_research_report(project, study, approved_artifact)
            .is_err());
        fs::write(&approved_path, &original_approved_bytes).unwrap();
        assert!(services
            .read_study_research_report(project, study, approved_artifact)
            .is_ok());

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
        let bridge = research_writing_from_report(artifact, &report).unwrap();
        assert_eq!(bridge.sources.len(), 3);
        assert!(!bridge.evidence.is_empty());
        assert!(bridge
            .evidence
            .iter()
            .all(|item| item.locator.source_id == transcript.to_string()));
        assert!(bridge.findings.iter().all(|item| {
            item.kind == FindingKind::CreativePattern && item.evidence_ids.len() == 1
        }));
        assert!(bridge
            .evidence
            .iter()
            .any(|item| { item.status == EvidenceStatus::Unverified }));
        assert!(bridge.evidence.iter().any(|item| {
            item.locator
                .document_section
                .as_deref()
                .is_some_and(|section| section.contains("segment 0 (0-2000 ms)"))
        }));
        assert!(bridge.source_text_is_untrusted_data);
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
        // A counter or transcript can change without a timestamp bump.
        db.execute(
            "UPDATE research_items SET view_count=99 WHERE id=?1",
            params![research_items[0].id.to_string()],
        )
        .unwrap();
        assert!(checked_report(&services, project, study, artifact).is_err());
        assert!(services
            .read_study_research_report(project, study, approved_artifact)
            .is_err());
        db.execute(
            "UPDATE research_items SET view_count=0 WHERE id=?1",
            params![research_items[0].id.to_string()],
        )
        .unwrap();
        assert!(checked_report(&services, project, study, artifact).is_ok());
        db.execute(
            "UPDATE transcripts SET text='Changed without timestamp bump' WHERE id=?1",
            params![transcript.to_string()],
        )
        .unwrap();
        assert!(checked_report(&services, project, study, artifact).is_err());
        db.execute(
            "UPDATE transcripts SET text='How does this work? But there is another way. Subscribe to follow.' WHERE id=?1",
            params![transcript.to_string()],
        )
        .unwrap();
        assert!(checked_report(&services, project, study, artifact).is_ok());

        // Reopening must never trust fabricated statements inserted in the
        // persisted artifact by another local process.
        let artifact_path = artifact_path(&services, artifact).unwrap();
        let saved_bytes = fs::read(&artifact_path).unwrap();
        let mut tampered: serde_json::Value = serde_json::from_slice(&saved_bytes).unwrap();
        tampered["report"]["limitations"][0] =
            serde_json::Value::String("Invented source citation".into());
        fs::write(&artifact_path, serde_json::to_vec(&tampered).unwrap()).unwrap();
        assert!(checked_report(&services, project, study, artifact).is_err());
        fs::write(&artifact_path, saved_bytes).unwrap();
        assert!(checked_report(&services, project, study, artifact).is_ok());
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

        // The previous source revision was changed only by this test; restore
        // it, close both SQLite handles and reopen from durable storage.
        db.execute(
            "UPDATE transcripts SET updated_at=?2 WHERE id=?1",
            params![transcript.to_string(), timestamp],
        )
        .unwrap();
        drop(db);
        drop(services);
        let restarted = AppServices::new(dir.path()).unwrap();
        let saved = restarted
            .read_study_research_report(project, study, approved_artifact)
            .unwrap();
        assert_eq!(saved.sources.len(), 3);
        let bridge = restarted
            .research_writing_context(project, study, approved_artifact)
            .unwrap();
        assert_eq!(bridge.sources.len(), 3);
        assert!(!bridge.evidence.is_empty());
        assert!(restarted
            .read_study_research_report(Uuid::new_v4(), study, approved_artifact)
            .is_err());
    }

    #[test]
    fn three_real_timed_transcripts_prepare_original_local_writing_request() {
        use scriptotar_agent::RunState;
        use scriptotar_ai::writing::{CreativeBrief, WritingInput, WritingTask};

        let dir = TempDir::new().unwrap();
        let services = AppServices::new(dir.path()).unwrap();
        let project = services.active_project_id().unwrap();
        let study = services
            .store
            .create_study(project, "Three recorded transcripts", "")
            .unwrap()
            .id;
        let timestamp = "2026-11-15T12:30:00Z";
        let mut attached = Vec::new();
        let mut origin_ids = Vec::new();
        {
            let db = Connection::open(services.store.path()).unwrap();
            db.pragma_update(None, "foreign_keys", true).unwrap();
            for index in 0..3 {
                let original_source = Uuid::new_v4();
                let media = Uuid::new_v4();
                let transcript = Uuid::new_v4();
                let locator = format!("/mock/three-source-{index}.mp4");
                db.execute(
                    "INSERT INTO sources(id,project_id,source_type,locator,title,created_at)
                     VALUES(?1,?2,'local_file',?3,'Offline original transcript',?4)",
                    params![
                        original_source.to_string(),
                        project.to_string(),
                        locator,
                        timestamp
                    ],
                )
                .unwrap();
                db.execute(
                    "INSERT INTO media(id,source_id,local_path,duration_seconds,created_at)
                     VALUES(?1,?2,?3,6.0,?4)",
                    params![
                        media.to_string(),
                        original_source.to_string(),
                        format!("/mock/three-source-{index}.mp4"),
                        timestamp
                    ],
                )
                .unwrap();
                let spoken = format!(
                    "How can we make original tutorial {index}? First describe one method. Follow for more."
                );
                let segments = serde_json::json!([
                    {"index":0,"start":0.0,"end":2.0,"text":spoken.clone()},
                    {"index":1,"start":2.0,"end":4.0,"text":"A real timed explanation."},
                    {"index":2,"start":4.0,"end":6.0,"text":"Return to the central question."}
                ]);
                db.execute(
                    "INSERT INTO transcripts(id,media_id,language,text,segments_json,created_at,updated_at)
                     VALUES(?1,?2,'en',?3,?4,?5,?5)",
                    params![
                        transcript.to_string(),
                        media.to_string(),
                        spoken,
                        segments.to_string(),
                        timestamp
                    ],
                )
                .unwrap();
                let source = services
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
                attached.push(source.id);
                origin_ids.push(transcript);
            }
        }
        let proposed = services
            .start_study_research(project, study, attached)
            .unwrap();
        assert_eq!(proposed.state, RunState::AwaitingApproval);
        let completed = services
            .decide_study_research(project, study, &proposed.plan.id, "approve")
            .unwrap();
        assert_eq!(completed.state, RunState::Completed);
        assert_eq!(completed.usage.reserved_cost_microusd, 0);
        assert_eq!(completed.usage.reserved_tokens, 0);
        let artifact =
            Uuid::parse_str(&completed.steps[0].output.as_ref().unwrap().artifact_id).unwrap();

        drop(services);
        let reopened = AppServices::new(dir.path()).unwrap();
        let bridge = reopened
            .research_writing_context(project, study, artifact)
            .unwrap();
        assert_eq!(bridge.sources.len(), 3);
        assert_eq!(bridge.attributed_source_count, 3);
        assert!(bridge.meets_three_source_evidence_floor);
        assert!(bridge.source_text_is_untrusted_data);
        assert!(bridge
            .findings
            .iter()
            .all(|finding| { finding.kind == FindingKind::CreativePattern }));
        assert!(bridge.evidence.iter().all(|item| {
            item.kind == EvidenceKind::SocialTranscript
                && item.status != EvidenceStatus::Corroborated
        }));

        // This merely constructs a zero-cost structured writing request. It
        // does not dispatch AI, verify any factual claim or save a draft.
        let input = WritingInput {
            brief: CreativeBrief {
                goal: "Produce an original tutorial script".into(),
                audience: "New learners".into(),
                platform: "YouTube".into(),
                language: "English".into(),
                region: None,
                dialect: None,
                tone: None,
                duration_seconds: 45,
                objective: "Compare three original angles".into(),
                desired_cta: None,
                constraints: vec!["Do not infer unseen frames or retention".into()],
                style_guide: None,
            },
            evidence: bridge.evidence,
            findings: bridge.findings,
            task: WritingTask::ProposeAngles { count: 3 },
        };
        let prompt = scriptotar_ai::writing::prepare_writing_request(&input)
            .unwrap()
            .prompt;
        for id in origin_ids {
            assert!(prompt.contains(&id.to_string()));
        }
        assert!(!prompt.is_empty());
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
        assert!(reopened
            .start_study_research(project, study, vec![source])
            .is_err());
        assert!(checked_report(&reopened, project, study, artifact).is_err());
        reopened.store.archive_study(project, study).unwrap();
        assert!(checked_report(&reopened, project, study, artifact).is_err());
    }
}
