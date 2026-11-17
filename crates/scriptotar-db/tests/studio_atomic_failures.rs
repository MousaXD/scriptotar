//! Fail-closed real-file SQLite regression coverage for P0 Study editor saves.
//! Each failure is injected by a persistent SQLite trigger on the canonical
//! database connection, not a mock or a separate unobserved in-memory store.
use rusqlite::{params, Connection};
use scriptotar_core::{
    CreativeBriefInput, Project, ProjectRepository, StudyEdit, StudySourceKind, StudyStatus,
};
use scriptotar_db::{SqliteStore, StudioBundleUpdate};
use tempfile::TempDir;
use uuid::Uuid;

fn brief(audience: &str) -> CreativeBriefInput {
    CreativeBriefInput {
        audience: audience.into(),
        goal: "Original short-form scripts".into(),
        language: "en".into(),
        region: Some("UAE".into()),
        platform: "reels".into(),
        tone: "educational".into(),
        duration_seconds: Some(45),
        objective: "explain".into(),
        cta: Some("Learn more".into()),
        constraints: vec!["Cite verified sources".into()],
    }
}

fn fixture() -> (TempDir, SqliteStore, Project, Uuid, Uuid, Uuid) {
    let dir = TempDir::new().unwrap();
    let db = SqliteStore::open(dir.path().join("studio.sqlite")).unwrap();
    db.run_integration_migrations().unwrap();
    let project = Project::new("Atomic Studio owner");
    db.create_project(&project).unwrap();
    let study = db
        .create_studio_bundle(
            project.id,
            "Original Study",
            "Original goal",
            &brief("Original audience"),
        )
        .unwrap();
    let drafts = db.list_draft_summaries(project.id, study, 10).unwrap();
    assert_eq!(drafts.len(), 1);
    let research = Uuid::new_v4();
    let conn = Connection::open(db.path()).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    conn.execute(
        "INSERT INTO research_items(id,project_id,source_url,platform,scanned_at)
         VALUES(?1,?2,'https://example.org/atomic-test','sample','2026-11-18T00:00:00Z')",
        params![research.to_string(), project.id.to_string()],
    )
    .unwrap();
    (dir, db, project, study, drafts[0].id, research)
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    title: String,
    goal: String,
    revision: i64,
    audience: String,
    source_count: usize,
    draft_body: String,
    draft_revision: i64,
    versions: usize,
}

fn snapshot(db: &SqliteStore, project: Uuid, study: Uuid, draft: Uuid) -> Snapshot {
    let study_row = db.get_study(project, study).unwrap();
    let brief_row = db.get_creative_brief(project, study).unwrap().unwrap();
    let draft_row = db.get_draft(project, draft).unwrap();
    Snapshot {
        title: study_row.title,
        goal: study_row.goal,
        revision: study_row.revision,
        audience: brief_row.input.audience,
        source_count: db.list_study_sources(project, study).unwrap().len(),
        draft_body: draft_row.body,
        draft_revision: draft_row.revision,
        versions: db.list_draft_versions(project, draft).unwrap().len(),
    }
}

#[test]
fn studio_bundle_rolls_back_every_prior_stage_after_sql_failure_and_reopen() {
    // These failures happen *after* the Study revision CAS in one IMMEDIATE
    // transaction. They cover the brief, deletion, insertion, and final draft
    // history stages without depending on timing or artificially skipping SQL.
    for stage in ["brief", "source_insert", "source_remove", "draft_version"] {
        let (dir, db, project, study, draft, research) = fixture();
        let source = vec![(StudySourceKind::ResearchItem, research)];
        let empty = Vec::new();

        // Only the removal test starts with an attached source. Its attempted
        // deletion is still inside the same canonical editor transaction.
        if stage == "source_remove" {
            let original = brief("Original audience");
            let edit = StudyEdit {
                title: "Original Study".into(),
                goal: "Original goal".into(),
                status: StudyStatus::Active,
            };
            db.save_studio_bundle(
                project.id,
                study,
                StudioBundleUpdate {
                    expected_revision: 1,
                    edit: &edit,
                    brief: &original,
                    brief_changed: false,
                    sources: &source,
                    draft: "",
                    draft_id: Some(draft),
                    expected_draft_revision: Some(1),
                },
            )
            .unwrap();
        }

        let before = snapshot(&db, project.id, study, draft);
        let sql = match stage {
            "brief" => {
                "CREATE TRIGGER fail_studio_brief BEFORE UPDATE ON study_briefs
                        BEGIN SELECT RAISE(ABORT,'injected brief failure'); END;"
            }
            "source_insert" => {
                "CREATE TRIGGER fail_studio_add BEFORE INSERT ON study_sources
                        BEGIN SELECT RAISE(ABORT,'injected source add failure'); END;"
            }
            "source_remove" => {
                "CREATE TRIGGER fail_studio_remove BEFORE DELETE ON study_sources
                        BEGIN SELECT RAISE(ABORT,'injected source remove failure'); END;"
            }
            "draft_version" => {
                "CREATE TRIGGER fail_studio_version BEFORE INSERT ON draft_versions
                        BEGIN SELECT RAISE(ABORT,'injected immutable version failure'); END;"
            }
            _ => unreachable!(),
        };
        Connection::open(db.path())
            .unwrap()
            .execute_batch(sql)
            .unwrap();

        let changed_brief = brief("Changed audience must roll back");
        let edit = StudyEdit {
            title: "Changed Study must roll back".into(),
            goal: "Changed goal must roll back".into(),
            status: StudyStatus::Active,
        };
        let desired = if stage == "source_remove" {
            &empty
        } else {
            &source
        };
        let attempted = db.save_studio_bundle(
            project.id,
            study,
            StudioBundleUpdate {
                expected_revision: before.revision,
                edit: &edit,
                brief: &changed_brief,
                brief_changed: true,
                sources: desired,
                draft: "Changed script body must roll back",
                draft_id: Some(draft),
                expected_draft_revision: Some(before.draft_revision),
            },
        );
        assert!(
            attempted.is_err(),
            "injected {stage} SQL failure unexpectedly saved"
        );
        // SQLite's transaction Drop/rollback must undo even the earlier CAS,
        // brief, source edits and candidate changes. Check this on disk after
        // reopening independently, rather than trusting the original handle.
        drop(db);
        let reopened = SqliteStore::open(dir.path().join("studio.sqlite")).unwrap();
        reopened.run_integration_migrations().unwrap();
        assert_eq!(
            snapshot(&reopened, project.id, study, draft),
            before,
            "{stage} failure persisted a partial Study bundle",
        );
    }
}

#[test]
fn studio_bundle_rejects_stale_and_wrong_project_without_mutation() {
    let (dir, db, project, study, draft, research) = fixture();
    let other = Project::new("Unrelated workspace");
    db.create_project(&other).unwrap();
    let before = snapshot(&db, project.id, study, draft);
    let edit = StudyEdit {
        title: "Foreign or stale modification".into(),
        goal: "Must never commit".into(),
        status: StudyStatus::Active,
    };
    let changed = brief("Unauthorized audience");
    for (owner, expected_revision) in [
        (other.id, before.revision),
        (project.id, before.revision + 1),
    ] {
        let result = db.save_studio_bundle(
            owner,
            study,
            StudioBundleUpdate {
                expected_revision,
                edit: &edit,
                brief: &changed,
                brief_changed: true,
                sources: &[(StudySourceKind::ResearchItem, research)],
                draft: "Unauthorized or stale script",
                draft_id: Some(draft),
                expected_draft_revision: Some(before.draft_revision),
            },
        );
        assert!(
            result.is_err(),
            "foreign or stale editor mutation committed"
        );
    }
    drop(db);
    let reopened = SqliteStore::open(dir.path().join("studio.sqlite")).unwrap();
    assert_eq!(snapshot(&reopened, project.id, study, draft), before);
    assert!(reopened.get_study(other.id, study).is_err());
}
