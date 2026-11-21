//! Agent10 no-network independent OS-process negative / positive controls for v13.
 //! This is intentionally test-only. A green run proves exactly the tested
 //! SQLite trigger boundary, not provider transport, pricing or receipt safety.
use rusqlite::params;
use scriptotar_db::SqliteStore;
use std::{
    path::Path,
    process::{Command, Stdio},
    thread,
    time::Duration,
};
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn agent10_os_process_writer_fixture() {
    let Ok(path) = std::env::var("SCRIPTOTAR_AGENT10_V13_DB") else {
        return; // Normal cargo test invocation never modifies arbitrary DBs.
    };
    let study = std::env::var("SCRIPTOTAR_AGENT10_V13_STUDY").unwrap();
    let expected_block = std::env::var("SCRIPTOTAR_AGENT10_V13_BLOCK")
        .unwrap() == "1";
    let connection = rusqlite::Connection::open(path).unwrap();
    let result = connection.execute(
        "UPDATE content_studies SET goal='written by independent OS process' WHERE id=?1",
        params![study],
    );
    if expected_block {
        let error = result.expect_err("independent OS process modified a frozen Study");
        assert!(
            error.to_string().contains("provider dispatch scope is frozen"),
            "unexpected failure, not a verified trigger rejection: {error}"
        );
        println!("AGENT10_V13_TRIGGER_REJECTED");
    } else {
        assert_eq!(result.unwrap(), 1);
        println!("AGENT10_V13_WRITE_AFTER_RELEASE");
    }
}

fn actual_os_process_probe(db_path: &Path, study: Uuid, blocked: bool) -> String {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg("agent10_os_process_writer_fixture")
        .arg("--nocapture")
        .env("SCRIPTOTAR_AGENT10_V13_DB", db_path)
        .env("SCRIPTOTAR_AGENT10_V13_STUDY", study.to_string())
        .env("SCRIPTOTAR_AGENT10_V13_BLOCK", if blocked { "1" } else { "0" })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut exited = false;
    for _ in 0..800 {
        if child.try_wait().unwrap().is_some() {
            exited = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    if !exited {
        let _ = child.kill();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        exited && output.status.success(),
        "child process did not complete cleanly before guard release. stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn agent10_v13_actual_second_os_process_is_denied_then_allowed_after_guard() {
    let directory = TempDir::new().unwrap();
    let db = SqliteStore::open(directory.path().join("v13.sqlite3")).unwrap();
    assert_eq!(db.schema_version().unwrap(), 13);
    let project = Uuid::new_v4();
    let study = Uuid::new_v4();
    let connection = rusqlite::Connection::open(db.path()).unwrap();
    connection
        .execute(
            "INSERT INTO projects(id,name,created_at) VALUES(?1,'Real process fixture','2026-11-21')",
            params![project.to_string()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO content_studies(
                id,project_id,title,goal,status,revision,created_at,updated_at
            ) VALUES(?1,?2,'Live Study','Approved original','active',1,'2026-11-21','2026-11-21')",
            params![study.to_string(), project.to_string()],
        )
        .unwrap();
    drop(connection);

    let guard = db
        .acquire_provider_dispatch_barrier(project, study, &"a".repeat(64))
        .unwrap();
    guard.verify().unwrap();
    let rejected = actual_os_process_probe(db.path(), study, true);
    assert!(rejected.contains("AGENT10_V13_TRIGGER_REJECTED"));
    guard.verify().unwrap();
    let unchanged: String = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row(
            "SELECT goal FROM content_studies WHERE id=?1",
            params![study.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(unchanged, "Approved original");
    drop(guard);

    let allowed = actual_os_process_probe(db.path(), study, false);
    assert!(allowed.contains("AGENT10_V13_WRITE_AFTER_RELEASE"));
    let after: String = rusqlite::Connection::open(db.path())
        .unwrap()
        .query_row(
            "SELECT goal FROM content_studies WHERE id=?1",
            params![study.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(after, "written by independent OS process");
}
