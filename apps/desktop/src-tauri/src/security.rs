use std::{
    collections::HashSet,
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process,
    time::Duration,
};

use rusqlite::{backup::Backup, Connection, OpenFlags};

use crate::dto::{AiPromptInput, ResearchQuery};

const MAX_PROJECT_NAME_BYTES: usize = 256;
const MAX_PATH_BYTES: usize = 4096;
const MAX_URL_BYTES: usize = 4096;
const MAX_MODEL_BYTES: usize = 256;
const MAX_TASK_BYTES: usize = 512;
const MAX_AI_CONTEXT_BYTES: usize = 2 * 1024 * 1024;
const MAX_AI_FIELD_BYTES: usize = 16 * 1024;
const MAX_API_KEY_BYTES: usize = 16 * 1024;
const MAX_RESEARCH_IDS: usize = 200;
const MAX_RESEARCH_ID_BYTES: usize = 256;
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

pub fn validate_project_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("project name cannot be empty".to_owned());
    }
    validate_text_size("project name", trimmed, MAX_PROJECT_NAME_BYTES)
}

pub fn validated_local_media_path(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    validate_text_size("local media path", raw, MAX_PATH_BYTES)?;
    if raw.contains('\0') {
        return Err("local media path contains an invalid NUL byte".to_owned());
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err("local media path must be absolute".to_owned());
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| "local media path could not be resolved".to_owned())?;
    let metadata = fs::metadata(&canonical)
        .map_err(|_| "local media path could not be inspected".to_owned())?;
    if !metadata.is_file() {
        return Err("local media path must point to a regular file".to_owned());
    }
    Ok(canonical.to_string_lossy().into_owned())
}

pub fn validate_url_argument(label: &str, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }
    validate_text_size(label, trimmed, MAX_URL_BYTES)?;
    if trimmed.chars().any(char::is_control) {
        return Err(format!("{label} contains control characters"));
    }
    Ok(())
}

pub fn validate_research_query(query: &ResearchQuery) -> Result<(), String> {
    validate_url_argument("research profile URL", &query.profile_url)?;
    if !(1..=200).contains(&query.limit) {
        return Err("research limit must be between 1 and 200".to_owned());
    }
    Ok(())
}

pub fn validate_research_ids(ids: &[String]) -> Result<(), String> {
    if ids.is_empty() {
        return Err("select at least one research item".to_owned());
    }
    if ids.len() > MAX_RESEARCH_IDS {
        return Err(format!(
            "at most {MAX_RESEARCH_IDS} research items can be queued at once"
        ));
    }
    for id in ids {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err("research item IDs cannot be empty".to_owned());
        }
        validate_text_size("research item ID", trimmed, MAX_RESEARCH_ID_BYTES)?;
        if trimmed.chars().any(char::is_control) {
            return Err("research item ID contains control characters".to_owned());
        }
    }
    Ok(())
}

pub fn validate_ai_input(input: &AiPromptInput) -> Result<(), String> {
    validate_text_size("AI mode", input.mode.trim(), 64)?;
    validate_text_size("AI provider", input.provider.trim(), 128)?;
    validate_text_size("AI model", input.model.trim(), MAX_MODEL_BYTES)?;
    validate_text_size("AI task", input.task.trim(), MAX_TASK_BYTES)?;
    validate_text_size(
        "AI source context",
        &input.source_text,
        MAX_AI_CONTEXT_BYTES,
    )?;
    for (label, value) in [
        ("AI topic", input.topic.as_str()),
        ("AI audience", input.audience.as_str()),
        ("AI duration", input.duration.as_str()),
        ("AI CTA", input.cta.as_str()),
        ("AI voice instructions", input.voice.as_str()),
    ] {
        validate_text_size(label, value, MAX_AI_FIELD_BYTES)?;
    }
    if let Some(base_url) = input.base_url.as_deref() {
        if !base_url.trim().is_empty() {
            validate_url_argument("AI provider endpoint", base_url)?;
        }
    }
    if let Some(api_key) = input.api_key.as_deref() {
        if api_key.len() > MAX_API_KEY_BYTES {
            return Err("API key exceeds the supported size".to_owned());
        }
        if api_key
            .chars()
            .any(|character| matches!(character, '\r' | '\n' | '\0'))
        {
            return Err("API key contains invalid control characters".to_owned());
        }
    }
    Ok(())
}

fn validate_text_size(label: &str, value: &str, max_bytes: usize) -> Result<(), String> {
    if value.len() > max_bytes {
        return Err(format!("{label} exceeds the supported size"));
    }
    Ok(())
}

pub fn prepare_legacy_import_bridge(data_dir: &Path) -> Result<Option<String>, String> {
    let roots = configured_legacy_data_roots();
    prepare_legacy_import_bridge_from_roots(data_dir, &roots)
}

fn prepare_legacy_import_bridge_from_roots(
    data_dir: &Path,
    roots: &[PathBuf],
) -> Result<Option<String>, String> {
    let staged = data_dir.join("history.sqlite3");
    match fs::symlink_metadata(&staged) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(
                    "legacy import staging path exists but is not a regular file; refusing to follow it"
                        .to_owned(),
                );
            }
            validate_sqlite_header(&staged)?;
            harden_private_file_permissions(&staged)?;
            return Ok(Some(
                "legacy import staging database already exists; it will be handled idempotently"
                    .to_owned(),
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "could not inspect legacy import staging path: {error}"
            ))
        }
    }

    let candidates = discover_legacy_databases(roots)?;
    match candidates.as_slice() {
        [] => Ok(None),
        [candidate] => {
            validate_sqlite_header(candidate)?;
            stage_legacy_database(candidate, &staged)?;
            Ok(Some(format!(
                "staged one legacy database for safe import from {}",
                candidate.display()
            )))
        }
        many => {
            let listed = many
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "multiple legacy databases were found; refusing to choose automatically: {listed}"
            ))
        }
    }
}

fn configured_legacy_data_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(value) = env::var_os("XDG_DATA_HOME") {
        if !value.is_empty() {
            roots.push(PathBuf::from(value));
        }
    }
    if let Some(home) = env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join(".local/share"));
        roots.push(home.join(".var/app/io.github.mousaxd.scriptotar/data"));
    }
    roots
}

fn discover_legacy_databases(roots: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for root in roots {
        for app_dir in ["scriptotar", "wesamboss"] {
            let candidate = root.join(app_dir).join("history.sqlite3");
            let metadata = match fs::symlink_metadata(&candidate) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(format!(
                        "could not inspect legacy database {}: {error}",
                        candidate.display()
                    ))
                }
            };
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "legacy database {} is a symbolic link; refusing to follow it",
                    candidate.display()
                ));
            }
            if !metadata.is_file() {
                continue;
            }
            let canonical = fs::canonicalize(&candidate).map_err(|error| {
                format!(
                    "could not resolve legacy database {}: {error}",
                    candidate.display()
                )
            })?;
            if seen.insert(canonical.clone()) {
                candidates.push(canonical);
            }
        }
    }
    candidates.sort();
    Ok(candidates)
}

fn stage_legacy_database(candidate: &Path, staged: &Path) -> Result<(), String> {
    let data_dir = staged
        .parent()
        .ok_or_else(|| "legacy staging path has no parent directory".to_owned())?;
    fs::create_dir_all(data_dir)
        .map_err(|error| format!("could not create migration staging directory: {error}"))?;
    harden_private_directory_permissions(data_dir)?;

    let temporary_path = reserve_legacy_staging_path(data_dir)?;
    let snapshot_result = (|| -> Result<(), String> {
        let source = Connection::open_with_flags(
            candidate,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| format!("could not open legacy database for snapshot: {error}"))?;
        let mut destination = Connection::open(&temporary_path)
            .map_err(|error| format!("could not create temporary SQLite snapshot: {error}"))?;
        let backup = Backup::new(&source, &mut destination)
            .map_err(|error| format!("could not initialize legacy SQLite backup: {error}"))?;
        backup
            .run_to_completion(64, Duration::from_millis(10), None)
            .map_err(|error| format!("could not snapshot legacy SQLite database: {error}"))?;
        drop(backup);
        drop(destination);
        fs::File::open(&temporary_path)
            .and_then(|file| file.sync_all())
            .map_err(|error| format!("could not flush staged legacy database: {error}"))?;
        Ok(())
    })();
    if let Err(error) = snapshot_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    if let Err(error) = harden_private_file_permissions(&temporary_path)
        .and_then(|_| validate_sqlite_header(&temporary_path))
        .and_then(|_| {
            fs::rename(&temporary_path, staged).map_err(|rename_error| {
                format!("could not finalize staged legacy database: {rename_error}")
            })
        })
    {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    Ok(())
}

fn reserve_legacy_staging_path(data_dir: &Path) -> Result<PathBuf, String> {
    for attempt in 0..16_u8 {
        let path = data_dir.join(format!(
            ".history.sqlite3.importing-{}-{attempt}",
            process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => {
                drop(file);
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "could not create temporary legacy import file: {error}"
                ))
            }
        }
    }
    Err("could not reserve a temporary legacy import file".to_owned())
}

fn validate_sqlite_header(path: &Path) -> Result<(), String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("could not inspect legacy database: {error}"))?;
    let mut header = [0_u8; 16];
    file.read_exact(&mut header)
        .map_err(|error| format!("legacy database is truncated or unreadable: {error}"))?;
    if &header != SQLITE_HEADER {
        return Err("legacy database does not have a valid SQLite header".to_owned());
    }
    Ok(())
}

#[cfg(unix)]
fn harden_private_directory_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("could not restrict application data permissions: {error}"))
}

#[cfg(not(unix))]
fn harden_private_directory_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn harden_private_file_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("could not restrict staged database permissions: {error}"))
}

#[cfg(not(unix))]
fn harden_private_file_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn fake_sqlite(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(SQLITE_HEADER).unwrap();
        file.write_all(b"fixture").unwrap();
    }

    fn real_sqlite(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE fixture(value TEXT); INSERT INTO fixture(value) VALUES ('source');",
            )
            .unwrap();
    }

    #[test]
    fn discovery_finds_scriptotar_and_wesamboss_locations_without_duplicates() {
        let temp = TempDir::new().unwrap();
        let scriptotar = temp.path().join("scriptotar/history.sqlite3");
        let wesamboss = temp.path().join("wesamboss/history.sqlite3");
        fake_sqlite(&scriptotar);
        fake_sqlite(&wesamboss);

        let found = discover_legacy_databases(&[temp.path().to_path_buf()]).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.contains(&fs::canonicalize(scriptotar).unwrap()));
        assert!(found.contains(&fs::canonicalize(wesamboss).unwrap()));
    }

    #[test]
    fn bridge_stages_one_legacy_database_without_modifying_source() {
        let temp = TempDir::new().unwrap();
        let source_root = temp.path().join("source");
        let source = source_root.join("scriptotar/history.sqlite3");
        real_sqlite(&source);
        let before = fs::read(&source).unwrap();
        let data_dir = temp.path().join("next");

        let message =
            prepare_legacy_import_bridge_from_roots(&data_dir, std::slice::from_ref(&source_root))
                .unwrap()
                .unwrap();
        let staged = data_dir.join("history.sqlite3");

        assert!(message.contains("staged one legacy database"));
        assert_eq!(fs::read(&source).unwrap(), before);
        let staged_connection = Connection::open(&staged).unwrap();
        let value: String = staged_connection
            .query_row("SELECT value FROM fixture", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "source");
        assert!(fs::read_dir(&data_dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".history.sqlite3.importing-")
        }));
    }

    #[test]
    fn bridge_snapshot_includes_committed_wal_frames() {
        let temp = TempDir::new().unwrap();
        let source_root = temp.path().join("source");
        let source = source_root.join("scriptotar/history.sqlite3");
        fs::create_dir_all(source.parent().unwrap()).unwrap();

        let writer = Connection::open(&source).unwrap();
        writer.pragma_update(None, "journal_mode", "WAL").unwrap();
        writer.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
        writer.execute("CREATE TABLE fixture(value TEXT)", []).unwrap();
        writer
            .execute("INSERT INTO fixture(value) VALUES ('before')", [])
            .unwrap();
        writer.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();

        let reader = Connection::open(&source).unwrap();
        reader
            .execute_batch("BEGIN; SELECT count(*) FROM fixture;")
            .unwrap();
        writer
            .execute("INSERT INTO fixture(value) VALUES ('wal-commit')", [])
            .unwrap();
        let wal = PathBuf::from(format!("{}-wal", source.display()));
        assert!(wal.is_file());
        assert!(fs::metadata(&wal).unwrap().len() > 0);

        let data_dir = temp.path().join("next");
        prepare_legacy_import_bridge_from_roots(&data_dir, std::slice::from_ref(&source_root))
            .unwrap()
            .unwrap();

        let staged = Connection::open(data_dir.join("history.sqlite3")).unwrap();
        let values = staged
            .prepare("SELECT value FROM fixture ORDER BY rowid")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(values, vec!["before", "wal-commit"]);
        reader.execute_batch("ROLLBACK;").unwrap();
    }

    #[test]
    fn bridge_refuses_ambiguous_legacy_databases() {
        let temp = TempDir::new().unwrap();
        let source_root = temp.path().join("source");
        fake_sqlite(&source_root.join("scriptotar/history.sqlite3"));
        fake_sqlite(&source_root.join("wesamboss/history.sqlite3"));

        let error =
            prepare_legacy_import_bridge_from_roots(&temp.path().join("next"), &[source_root])
                .unwrap_err();
        assert!(error.contains("multiple legacy databases"));
        assert!(!temp.path().join("next/history.sqlite3").exists());
    }

    #[test]
    fn bridge_rejects_non_sqlite_input() {
        let temp = TempDir::new().unwrap();
        let source_root = temp.path().join("source");
        let source = source_root.join("scriptotar/history.sqlite3");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, b"not a sqlite database").unwrap();

        let error =
            prepare_legacy_import_bridge_from_roots(&temp.path().join("next"), &[source_root])
                .unwrap_err();
        assert!(error.contains("valid SQLite header"));
    }

    #[cfg(unix)]
    #[test]
    fn discovery_rejects_symlinked_legacy_database() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let source_root = temp.path().join("source");
        let real = temp.path().join("real.sqlite3");
        fake_sqlite(&real);
        let candidate = source_root.join("scriptotar/history.sqlite3");
        fs::create_dir_all(candidate.parent().unwrap()).unwrap();
        symlink(&real, &candidate).unwrap();

        let error = discover_legacy_databases(&[source_root]).unwrap_err();
        assert!(error.contains("symbolic link"));
    }

    #[test]
    fn local_media_paths_must_be_absolute_regular_files() {
        assert!(validated_local_media_path("relative.mp4").is_err());
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("video.mp4");
        fs::write(&path, b"fixture").unwrap();
        let validated = validated_local_media_path(path.to_str().unwrap()).unwrap();
        assert_eq!(PathBuf::from(validated), fs::canonicalize(path).unwrap());
    }

    #[test]
    fn rejects_oversized_ipc_payloads() {
        assert!(validate_project_name(&"x".repeat(MAX_PROJECT_NAME_BYTES + 1)).is_err());
        let ids = (0..=MAX_RESEARCH_IDS)
            .map(|index| index.to_string())
            .collect::<Vec<_>>();
        assert!(validate_research_ids(&ids).is_err());
    }
}
