//! Migration 6: additive Content Study storage, with origin snapshot tombstones.
//! Existing versions 1–5 remain byte-for-byte unchanged.

use rusqlite::Transaction;

pub(crate) fn migration_6(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        r#"
CREATE TABLE content_studies (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK(length(trim(title)) > 0 AND length(title) <= 200),
    goal TEXT NOT NULL CHECK(length(goal) <= 20000),
    status TEXT NOT NULL CHECK(status IN ('active','paused','completed')),
    revision INTEGER NOT NULL DEFAULT 1 CHECK(revision >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);
CREATE INDEX idx_content_studies_project_updated
    ON content_studies(project_id, archived_at, updated_at DESC, id);

CREATE TABLE study_sources (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    study_id TEXT NOT NULL REFERENCES content_studies(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN
        ('research_item','source','media','transcript','job','ai_run','manual')),
    reference_id TEXT,
    research_id TEXT REFERENCES research_items(id) ON DELETE SET NULL,
    legacy_source_id TEXT REFERENCES sources(id) ON DELETE SET NULL,
    media_id TEXT REFERENCES media(id) ON DELETE SET NULL,
    transcript_id TEXT REFERENCES transcripts(id) ON DELETE SET NULL,
    job_id TEXT REFERENCES jobs(id) ON DELETE SET NULL,
    ai_run_id TEXT REFERENCES ai_runs(id) ON DELETE SET NULL,
    locator TEXT NOT NULL,
    manual_text TEXT,
    fingerprint TEXT NOT NULL,
    source_revision TEXT,
    collected_at TEXT NOT NULL,
    uncertainty TEXT,
    created_at TEXT NOT NULL,
    CHECK(
      (kind = 'manual' AND reference_id IS NULL AND manual_text IS NOT NULL
       AND research_id IS NULL AND legacy_source_id IS NULL AND media_id IS NULL
       AND transcript_id IS NULL AND job_id IS NULL AND ai_run_id IS NULL)
      OR
      (kind = 'research_item' AND reference_id IS NOT NULL AND manual_text IS NULL
       AND legacy_source_id IS NULL AND media_id IS NULL AND transcript_id IS NULL
       AND job_id IS NULL AND ai_run_id IS NULL)
      OR
      (kind = 'source' AND reference_id IS NOT NULL AND manual_text IS NULL
       AND research_id IS NULL AND media_id IS NULL AND transcript_id IS NULL
       AND job_id IS NULL AND ai_run_id IS NULL)
      OR
      (kind = 'media' AND reference_id IS NOT NULL AND manual_text IS NULL
       AND research_id IS NULL AND legacy_source_id IS NULL AND transcript_id IS NULL
       AND job_id IS NULL AND ai_run_id IS NULL)
      OR
      (kind = 'transcript' AND reference_id IS NOT NULL AND manual_text IS NULL
       AND research_id IS NULL AND legacy_source_id IS NULL AND media_id IS NULL
       AND job_id IS NULL AND ai_run_id IS NULL)
      OR
      (kind = 'job' AND reference_id IS NOT NULL AND manual_text IS NULL
       AND research_id IS NULL AND legacy_source_id IS NULL AND media_id IS NULL
       AND transcript_id IS NULL AND ai_run_id IS NULL)
      OR
      (kind = 'ai_run' AND reference_id IS NOT NULL AND manual_text IS NULL
       AND research_id IS NULL AND legacy_source_id IS NULL AND media_id IS NULL
       AND transcript_id IS NULL AND job_id IS NULL)
    ),
    UNIQUE(study_id, kind, fingerprint)
);
CREATE INDEX idx_study_sources_project_study ON study_sources(project_id, study_id, created_at);
CREATE INDEX idx_study_sources_research ON study_sources(research_id);
CREATE INDEX idx_study_sources_legacy_source ON study_sources(legacy_source_id);
CREATE INDEX idx_study_sources_media ON study_sources(media_id);
CREATE INDEX idx_study_sources_transcript ON study_sources(transcript_id);
CREATE INDEX idx_study_sources_job ON study_sources(job_id);
CREATE INDEX idx_study_sources_ai_run ON study_sources(ai_run_id);

CREATE TABLE study_evidence (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    study_id TEXT NOT NULL REFERENCES content_studies(id) ON DELETE CASCADE,
    study_source_id TEXT NOT NULL REFERENCES study_sources(id) ON DELETE CASCADE,
    claim TEXT NOT NULL CHECK(length(trim(claim)) > 0 AND length(claim) <= 20000),
    locator TEXT NOT NULL CHECK(length(trim(locator)) > 0 AND length(locator) <= 4000),
    timestamp_seconds REAL CHECK(timestamp_seconds IS NULL OR
        (timestamp_seconds >= 0 AND timestamp_seconds < 1000000000)),
    certainty TEXT NOT NULL CHECK(certainty IN
        ('unverified','estimated','corroborated','disputed')),
    uncertainty_note TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_study_evidence_source ON study_evidence(study_source_id, created_at);
CREATE INDEX idx_study_evidence_project_study ON study_evidence(project_id, study_id, created_at);

CREATE TABLE study_briefs (
    study_id TEXT PRIMARY KEY REFERENCES content_studies(id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    audience TEXT NOT NULL,
    goal TEXT NOT NULL,
    language TEXT NOT NULL,
    region TEXT,
    platform TEXT NOT NULL,
    tone TEXT NOT NULL,
    duration_seconds INTEGER CHECK(duration_seconds IS NULL OR duration_seconds > 0),
    objective TEXT NOT NULL,
    cta TEXT,
    constraints_json TEXT NOT NULL CHECK(json_valid(constraints_json)),
    revision INTEGER NOT NULL CHECK(revision >= 1),
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_study_briefs_project ON study_briefs(project_id, study_id);

CREATE TABLE creative_drafts (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    study_id TEXT NOT NULL REFERENCES content_studies(id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK(length(trim(title)) > 0 AND length(title) <= 200),
    body TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK(revision >= 1),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);
CREATE INDEX idx_creative_drafts_study_updated
    ON creative_drafts(project_id, study_id, archived_at, updated_at DESC, id);

CREATE TABLE draft_versions (
    id TEXT PRIMARY KEY,
    draft_id TEXT NOT NULL REFERENCES creative_drafts(id) ON DELETE CASCADE,
    version INTEGER NOT NULL CHECK(version >= 1),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    saved_at TEXT NOT NULL,
    UNIQUE(draft_id, version)
);
CREATE INDEX idx_draft_versions_draft ON draft_versions(draft_id, version DESC);

-- A source's original identity/metadata is a snapshot. Only ON DELETE SET NULL
-- can detach a legacy-origin FK. Updating it to another origin is not supported.
CREATE TRIGGER study_source_identity_immutable
BEFORE UPDATE OF project_id, study_id, kind, reference_id, locator,
                 manual_text, fingerprint, source_revision, collected_at, uncertainty
ON study_sources
BEGIN
    SELECT RAISE(ABORT, 'study source identity is immutable');
END;
CREATE TRIGGER study_source_ref_immutable
BEFORE UPDATE OF research_id, legacy_source_id, media_id,
                 transcript_id, job_id, ai_run_id
ON study_sources
WHEN
    (NEW.research_id IS NOT OLD.research_id AND NEW.research_id IS NOT NULL)
 OR (NEW.legacy_source_id IS NOT OLD.legacy_source_id AND NEW.legacy_source_id IS NOT NULL)
 OR (NEW.media_id IS NOT OLD.media_id AND NEW.media_id IS NOT NULL)
 OR (NEW.transcript_id IS NOT OLD.transcript_id AND NEW.transcript_id IS NOT NULL)
 OR (NEW.job_id IS NOT OLD.job_id AND NEW.job_id IS NOT NULL)
 OR (NEW.ai_run_id IS NOT OLD.ai_run_id AND NEW.ai_run_id IS NOT NULL)
BEGIN
    SELECT RAISE(ABORT, 'study source origin cannot be reassigned');
END;

CREATE TRIGGER study_source_insert_guard BEFORE INSERT ON study_sources
BEGIN
    SELECT RAISE(ABORT, 'study does not belong to active project')
    WHERE NOT EXISTS (SELECT 1 FROM content_studies st
                      JOIN projects p ON p.id=st.project_id
                      WHERE st.id=NEW.study_id AND st.project_id=NEW.project_id
                        AND st.archived_at IS NULL AND p.archived_at IS NULL);
    SELECT RAISE(ABORT, 'invalid research source project')
    WHERE NEW.kind='research_item' AND NOT EXISTS
      (SELECT 1 FROM research_items r WHERE r.id=NEW.research_id
       AND r.project_id=NEW.project_id AND r.id=NEW.reference_id);
    SELECT RAISE(ABORT, 'invalid original source project')
    WHERE NEW.kind='source' AND NOT EXISTS
      (SELECT 1 FROM sources s WHERE s.id=NEW.legacy_source_id
       AND s.project_id=NEW.project_id AND s.id=NEW.reference_id);
    SELECT RAISE(ABORT, 'invalid media source project')
    WHERE NEW.kind='media' AND NOT EXISTS
      (SELECT 1 FROM media m JOIN sources s ON s.id=m.source_id
       WHERE m.id=NEW.media_id AND s.project_id=NEW.project_id AND m.id=NEW.reference_id);
    SELECT RAISE(ABORT, 'invalid transcript source project')
    WHERE NEW.kind='transcript' AND NOT EXISTS
      (SELECT 1 FROM transcripts t JOIN media m ON m.id=t.media_id
       JOIN sources s ON s.id=m.source_id
       WHERE t.id=NEW.transcript_id AND s.project_id=NEW.project_id AND t.id=NEW.reference_id);
    SELECT RAISE(ABORT, 'invalid job source project')
    WHERE NEW.kind='job' AND NOT EXISTS
      (SELECT 1 FROM jobs j WHERE j.id=NEW.job_id
       AND j.project_id=NEW.project_id AND j.id=NEW.reference_id);
    SELECT RAISE(ABORT, 'invalid AI run source project')
    WHERE NEW.kind='ai_run' AND NOT EXISTS
      (SELECT 1 FROM ai_runs a WHERE a.id=NEW.ai_run_id
       AND a.project_id=NEW.project_id AND a.id=NEW.reference_id);
END;

CREATE TRIGGER study_evidence_insert_guard BEFORE INSERT ON study_evidence
BEGIN
    SELECT RAISE(ABORT, 'evidence must belong to study/source project')
    WHERE NOT EXISTS (
      SELECT 1 FROM study_sources src
      JOIN content_studies st ON st.id=src.study_id AND st.project_id=src.project_id
      JOIN projects p ON p.id=st.project_id
      WHERE src.id=NEW.study_source_id
        AND src.project_id=NEW.project_id
        AND src.study_id=NEW.study_id
        AND st.archived_at IS NULL AND p.archived_at IS NULL
    );
END;
CREATE TRIGGER study_evidence_identity_immutable
BEFORE UPDATE OF project_id, study_id, study_source_id ON study_evidence
BEGIN SELECT RAISE(ABORT, 'evidence reference cannot change owner'); END;

CREATE TRIGGER study_brief_insert_guard BEFORE INSERT ON study_briefs
WHEN NOT EXISTS (SELECT 1 FROM content_studies st JOIN projects p ON p.id=st.project_id
                 WHERE st.id=NEW.study_id AND st.project_id=NEW.project_id
                   AND st.archived_at IS NULL AND p.archived_at IS NULL)
BEGIN SELECT RAISE(ABORT, 'brief must belong to active study/project'); END;
CREATE TRIGGER study_brief_owner_immutable
BEFORE UPDATE OF study_id, project_id ON study_briefs
BEGIN SELECT RAISE(ABORT, 'brief cannot change study/project'); END;

CREATE TRIGGER creative_draft_insert_guard BEFORE INSERT ON creative_drafts
WHEN NOT EXISTS (SELECT 1 FROM content_studies st JOIN projects p ON p.id=st.project_id
                 WHERE st.id=NEW.study_id AND st.project_id=NEW.project_id
                   AND st.archived_at IS NULL AND p.archived_at IS NULL)
BEGIN SELECT RAISE(ABORT, 'draft must belong to active study/project'); END;
CREATE TRIGGER creative_draft_owner_immutable
BEFORE UPDATE OF study_id, project_id ON creative_drafts
BEGIN SELECT RAISE(ABORT, 'draft cannot change study/project'); END;
CREATE TRIGGER draft_version_insert_guard BEFORE INSERT ON draft_versions
WHEN NOT EXISTS (SELECT 1 FROM creative_drafts d WHERE d.id=NEW.draft_id
                 AND d.revision=NEW.version)
BEGIN SELECT RAISE(ABORT, 'draft version must match saved revision'); END;
CREATE TRIGGER draft_versions_immutable_update BEFORE UPDATE ON draft_versions
BEGIN SELECT RAISE(ABORT, 'draft version is immutable'); END;
-- Ordinary version deletion is disallowed. Cascading deletion of the parent
-- draft/project is permitted because the parent row has already gone away.
CREATE TRIGGER draft_versions_immutable_delete BEFORE DELETE ON draft_versions
WHEN EXISTS (SELECT 1 FROM creative_drafts WHERE id=OLD.draft_id)
BEGIN SELECT RAISE(ABORT, 'draft version is immutable'); END;

-- Origin lineage cannot be silently moved across projects once attached.
CREATE TRIGGER study_research_reassignment_guard BEFORE UPDATE OF project_id ON research_items
WHEN NEW.project_id <> OLD.project_id AND EXISTS (
    SELECT 1 FROM study_sources ss WHERE ss.research_id=OLD.id AND ss.project_id<>NEW.project_id
)
BEGIN SELECT RAISE(ABORT, 'research is linked to a different-project study'); END;
CREATE TRIGGER study_sources_reassignment_guard BEFORE UPDATE OF project_id ON sources
WHEN NEW.project_id <> OLD.project_id AND (
    EXISTS (SELECT 1 FROM study_sources ss WHERE ss.legacy_source_id=OLD.id AND ss.project_id<>NEW.project_id)
 OR EXISTS (SELECT 1 FROM study_sources ss JOIN media m ON m.id=ss.media_id
            WHERE m.source_id=OLD.id AND ss.project_id<>NEW.project_id)
 OR EXISTS (SELECT 1 FROM study_sources ss JOIN transcripts t ON t.id=ss.transcript_id
            JOIN media m ON m.id=t.media_id
            WHERE m.source_id=OLD.id AND ss.project_id<>NEW.project_id)
)
BEGIN SELECT RAISE(ABORT, 'source is linked to a different-project study'); END;
CREATE TRIGGER study_media_reassignment_guard BEFORE UPDATE OF source_id ON media
WHEN NEW.source_id <> OLD.source_id AND EXISTS (
    SELECT 1 FROM study_sources ss JOIN sources s ON s.id=NEW.source_id
    WHERE (ss.media_id=OLD.id OR ss.transcript_id IN
        (SELECT t.id FROM transcripts t WHERE t.media_id=OLD.id))
    AND ss.project_id<>s.project_id
)
BEGIN SELECT RAISE(ABORT, 'media is linked to a different-project study'); END;
CREATE TRIGGER study_transcript_reassignment_guard BEFORE UPDATE OF media_id ON transcripts
WHEN NEW.media_id <> OLD.media_id AND EXISTS (
    SELECT 1 FROM study_sources ss JOIN media m ON m.id=NEW.media_id
    JOIN sources s ON s.id=m.source_id
    WHERE ss.transcript_id=OLD.id AND ss.project_id<>s.project_id
)
BEGIN SELECT RAISE(ABORT, 'transcript is linked to a different-project study'); END;
CREATE TRIGGER study_job_reassignment_guard BEFORE UPDATE OF project_id ON jobs
WHEN NEW.project_id <> OLD.project_id AND EXISTS (
    SELECT 1 FROM study_sources ss WHERE ss.job_id=OLD.id AND ss.project_id<>NEW.project_id
)
BEGIN SELECT RAISE(ABORT, 'job is linked to a different-project study'); END;
CREATE TRIGGER study_ai_run_reassignment_guard BEFORE UPDATE OF project_id ON ai_runs
WHEN NEW.project_id <> OLD.project_id AND EXISTS (
    SELECT 1 FROM study_sources ss WHERE ss.ai_run_id=OLD.id AND ss.project_id<>NEW.project_id
)
BEGIN SELECT RAISE(ABORT, 'AI run is linked to a different-project study'); END;
"#,
    )
}
