// Independent Agent 10 adversarial host no-write regression.
// This source is injected ONLY into the disposable CI target checkout's
// apps/desktop/src-tauri/src/services.rs existing #[cfg(test)] mod tests.
// It does not modify the production repository, seed fake native data,
// run network requests, or enable provider egress.
//
// A count-only assertion misses unauthorized rewrites. Compare every field
// of every row in every real canonical SQLite table before/after denial.
fn independent_host_canonical_contents(
    db_path: &std::path::Path,
) -> Vec<(String, Vec<Vec<String>>)> {
    use rusqlite::OpenFlags;
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("canonical SQLite must be opened read-only");
    conn.pragma_update(None, "query_only", "ON")
        .expect("canonical SQLite must remain read-only");
    let mut tables_stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' \
             AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .unwrap();
    let table_names: Vec<String> = tables_stmt
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|row| row.unwrap())
        .collect();
    assert!(table_names.iter().any(|name| name == "jobs"));
    assert!(table_names.iter().any(|name| name == "projects"));
    let mut snapshot = Vec::new();
    for name in table_names {
        let quoted = name.replace('"', "\"\"");
        let statement = format!("SELECT * FROM \"{quoted}\"");
        let mut stmt = conn.prepare(&statement).unwrap();
        let column_count = stmt.column_count();
        let mut rows: Vec<Vec<String>> = stmt
            .query_map([], |row| {
                (0..column_count)
                    .map(|col| {
                        let value = row.get_ref(col)?;
                        Ok(format!("{value:?}"))
                    })
                    .collect::<rusqlite::Result<Vec<String>>>()
            })
            .unwrap()
            .map(|row| row.unwrap())
            .collect();
        rows.sort();
        snapshot.push((name, rows));
    }
    snapshot
}

#[test]
fn independent_host_denied_url_and_foreign_project_preserve_all_canonical_contents() {
    let directory = TempDir::new().unwrap();
    let services = AppServices::new(directory.path()).unwrap();
    let current = services.active_project_id().unwrap();
    let db_path = database_path(&directory);
    let before = independent_host_canonical_contents(&db_path);

    // Both schemes reach the real production NetworkPolicy. They MUST be
    // rejected before JobService insertion or orchestration/side effects.
    for invalid in [
        "file:///scriptotar-native-qa-denied-media.mp4",
        "javascript:alert(1)",
    ] {
        assert!(
            matches!(
                services.enqueue_url(current, invalid.to_owned()),
                Err(RepositoryError::Validation(_))
            ),
            "unsupported scheme was not denied at the canonical host"
        );
        assert_eq!(
            independent_host_canonical_contents(&db_path),
            before,
            "denied URL modified canonical SQLite contents, not merely counts"
        );
    }

    // A syntactically valid URL under a forged *inactive* project cannot
    // access another workspace or create any rows in any canonical table.
    let forged = Uuid::new_v4();
    assert_ne!(forged, current);
    assert!(
        matches!(
            services.enqueue_url(forged, "https://example.com/valid-media".to_owned()),
            Err(RepositoryError::NotFound(_))
        ),
        "foreign project ID reached URL admission"
    );
    assert_eq!(
        independent_host_canonical_contents(&db_path),
        before,
        "foreign project URL request mutated canonical SQLite contents"
    );
}

#[test]
fn independent_host_denied_foreign_study_read_preserves_private_draft_and_canonical_contents() {
    let directory = TempDir::new().unwrap();
    let services = AppServices::new(directory.path()).unwrap();
    let project_a = services.active_project_id().unwrap();
    let owner_study = services.create_studio_study(
        project_a,
        crate::dto::UiStudioBrief {
            title: "Private project A Study".into(),
            goal: "Private source".into(),
            audience: "Project A".into(),
            platform: "reels".into(),
            language: "en".into(),
            duration: "45".into(),
            style: "factual".into(),
        },
    ).unwrap();
    let study_a = Uuid::parse_str(&owner_study.id).unwrap();
    let owner_draft = owner_study.draft_id.as_ref()
        .map(|id| Uuid::parse_str(id).unwrap());
    // Positive evidence that the canonical Study and draft actually exist.
    assert_eq!(services.get_studio_study(project_a, study_a).unwrap().id, owner_study.id);

    let project_b_bootstrap = services.create_project("Project B".into()).unwrap();
    let project_b = Uuid::parse_str(&project_b_bootstrap.active_project_id).unwrap();
    assert_ne!(project_a, project_b);
    assert_eq!(services.active_project_id().unwrap(), project_b);
    let db_path = database_path(&directory);
    let before = independent_host_canonical_contents(&db_path);

    // Exactly the IDs the ordinary Tauri get_study command parses. Both a
    // genuine foreign project ID and a forged active project ID with Study A
    // must be denied at the Rust host before any read of private draft text.
    assert!(matches!(
        services.get_studio_study(project_a, study_a),
        Err(RepositoryError::NotFound(_)),
    ), "foreign project Study read bypassed the active-project authority");
    assert!(services.get_studio_study(project_b, study_a).is_err(),
        "forged active project ID opened another project's Study");

    if let Some(draft_a) = owner_draft {
        assert!(matches!(
            services.get_studio_study_draft(project_a, study_a, draft_a),
            Err(RepositoryError::NotFound(_)),
        ), "foreign project draft read bypassed active-project authority");
        assert!(services.get_studio_study_draft(project_b, study_a, draft_a).is_err(),
            "forged active project ID opened another project's draft");
    }
    assert_eq!(independent_host_canonical_contents(&db_path), before,
        "denied foreign Study/draft read changed canonical SQLite contents");

    // Restoring authorized owner access proves the rejection did not corrupt
    // the Study while inspecting a foreign workspace.
    services.select_project(project_a).unwrap();
    let recovered = services.get_studio_study(project_a, study_a).unwrap();
    assert_eq!(recovered.id, owner_study.id);
    assert_eq!(recovered.project_id, project_a.to_string());
}
