use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    env, fs,
    hash::{Hash, Hasher},
    io::{self, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process,
    sync::{Mutex, OnceLock},
    time::{Duration, UNIX_EPOCH},
};

use rusqlite::{backup::Backup, Connection, OpenFlags};
use uuid::Uuid;

use crate::dto::{UiMigrationCandidate, UiMigrationStatus};

const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";
const ACTIVE_STAGE_NAME: &str = "history.sqlite3";
const PENDING_STAGE_NAME: &str = ".history.sqlite3.pending-import";
const IDENTITY_SAMPLE_BYTES: usize = 64 * 1024;

static STATUS: OnceLock<Mutex<UiMigrationStatus>> = OnceLock::new();

#[derive(Debug, Clone)]
pub enum Preparation {
    NoLegacyDatabase,
    Ready,
    RequiresChoice(Vec<UiMigrationCandidate>),
    InvalidDatabase(String),
    Failed(String),
}

fn status_cell() -> &'static Mutex<UiMigrationStatus> {
    STATUS.get_or_init(|| Mutex::new(UiMigrationStatus::no_legacy_database()))
}

pub fn current_status() -> UiMigrationStatus {
    status_cell()
        .lock()
        .map(|status| status.clone())
        .unwrap_or_else(|_| {
            UiMigrationStatus::failed(
                "Migration status is temporarily unavailable. Restart Scriptotar and try again.",
            )
        })
}

pub fn set_status(status: UiMigrationStatus) {
    if let Ok(mut current) = status_cell().lock() {
        *current = status;
    }
}

pub fn prepare_startup(data_dir: &Path) -> Preparation {
    let preparation = prepare_from_roots(data_dir, &configured_legacy_data_roots());
    set_status(status_for_preparation(&preparation));
    preparation
}

pub fn retry_discovery(data_dir: &Path) -> Preparation {
    let preparation = prepare_from_roots(data_dir, &configured_legacy_data_roots());
    set_status(status_for_preparation(&preparation));
    preparation
}

pub fn choose_candidate(data_dir: &Path, candidate_id: &str) -> Preparation {
    let preparation =
        choose_candidate_from_roots(data_dir, &configured_legacy_data_roots(), candidate_id);
    set_status(status_for_preparation(&preparation));
    preparation
}

pub fn activate_pending_stage(data_dir: &Path) -> Result<PathBuf, String> {
    let pending = data_dir.join(PENDING_STAGE_NAME);
    validate_regular_sqlite(&pending)?;
    harden_private_file_permissions(&pending)?;
    let active = data_dir.join(ACTIVE_STAGE_NAME);
    match fs::symlink_metadata(&active) {
        Ok(_) => Err(
            "A legacy import staging file already exists. Restart Scriptotar before retrying the migration."
                .to_owned(),
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::rename(&pending, &active)
                .map_err(|_| "Could not activate the prepared legacy import snapshot.".to_owned())?;
            harden_private_file_permissions(&active)?;
            Ok(active)
        }
        Err(_) => Err("Could not inspect the legacy import staging location.".to_owned()),
    }
}

fn status_for_preparation(preparation: &Preparation) -> UiMigrationStatus {
    match preparation {
        Preparation::NoLegacyDatabase => UiMigrationStatus::no_legacy_database(),
        Preparation::Ready => UiMigrationStatus::ready(),
        Preparation::RequiresChoice(candidates) => {
            UiMigrationStatus::requires_choice(candidates.clone())
        }
        Preparation::InvalidDatabase(message) => UiMigrationStatus::invalid_database(message),
        Preparation::Failed(message) => UiMigrationStatus::failed(message),
    }
}

fn prepare_from_roots(data_dir: &Path, roots: &[PathBuf]) -> Preparation {
    if fs::create_dir_all(data_dir).is_err() {
        return Preparation::Failed(
            "Scriptotar could not prepare its migration staging directory.".to_owned(),
        );
    }
    if let Err(error) = harden_private_directory_permissions(data_dir) {
        return Preparation::Failed(error);
    }

    let active = data_dir.join(ACTIVE_STAGE_NAME);
    match fs::symlink_metadata(&active) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                let _ = quarantine_unsafe_stage(&active, data_dir);
                return Preparation::InvalidDatabase(
                    "The existing legacy migration staging entry is not a regular file. It was not followed."
                        .to_owned(),
                );
            }
            if let Err(error) = validate_sqlite_header(&active) {
                let _ = quarantine_unsafe_stage(&active, data_dir);
                return Preparation::InvalidDatabase(safe_invalid_message(&error));
            }
            let pending = data_dir.join(PENDING_STAGE_NAME);
            match fs::symlink_metadata(&pending) {
                Ok(_) => {
                    return Preparation::Failed(
                        "A previous migration snapshot is still pending. Restart Scriptotar and retry migration discovery."
                            .to_owned(),
                    )
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => {
                    return Preparation::Failed(
                        "Scriptotar could not inspect the pending legacy migration snapshot."
                            .to_owned(),
                    )
                }
            }
            if fs::rename(&active, &pending).is_err() {
                return Preparation::Failed(
                    "Scriptotar could not safely prepare the existing migration snapshot for import."
                        .to_owned(),
                );
            }
            if let Err(error) = harden_private_file_permissions(&pending) {
                return Preparation::Failed(error);
            }
            return Preparation::Ready;
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => {
            return Preparation::Failed(
                "Scriptotar could not inspect the legacy migration staging location.".to_owned(),
            )
        }
    }

    let pending = data_dir.join(PENDING_STAGE_NAME);
    match fs::symlink_metadata(&pending) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                let _ = quarantine_unsafe_stage(&pending, data_dir);
                return Preparation::InvalidDatabase(
                    "The pending legacy migration snapshot is not a regular file. It was not followed."
                        .to_owned(),
                );
            }
            return match validate_sqlite_header(&pending)
                .and_then(|_| harden_private_file_permissions(&pending))
            {
                Ok(()) => Preparation::Ready,
                Err(error) if is_invalid_database_error(&error) => {
                    Preparation::InvalidDatabase(safe_invalid_message(&error))
                }
                Err(error) => Preparation::Failed(safe_failure_message(&error)),
            };
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => {
            return Preparation::Failed(
                "Scriptotar could not inspect the pending legacy migration snapshot.".to_owned(),
            )
        }
    }

    let candidates = match discover_legacy_databases(roots) {
        Ok(candidates) => candidates,
        Err(error) => return classify_discovery_error(&error),
    };
    match candidates.as_slice() {
        [] => Preparation::NoLegacyDatabase,
        [candidate] => match validate_sqlite_header(candidate)
            .and_then(|_| stage_legacy_database(candidate, &pending))
        {
            Ok(()) => Preparation::Ready,
            Err(error) if is_invalid_database_error(&error) => {
                Preparation::InvalidDatabase(safe_invalid_message(&error))
            }
            Err(error) => Preparation::Failed(safe_failure_message(&error)),
        },
        many => Preparation::RequiresChoice(candidate_labels(many)),
    }
}

fn choose_candidate_from_roots(
    data_dir: &Path,
    roots: &[PathBuf],
    selected_id: &str,
) -> Preparation {
    for stage in [
        data_dir.join(PENDING_STAGE_NAME),
        data_dir.join(ACTIVE_STAGE_NAME),
    ] {
        match fs::symlink_metadata(&stage) {
            Ok(_) => {
                return Preparation::Failed(
                    "A migration snapshot is already being prepared or imported. Refresh migration status before choosing another candidate."
                        .to_owned(),
                )
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => {
                return Preparation::Failed(
                    "Scriptotar could not inspect the migration staging location safely."
                        .to_owned(),
                )
            }
        }
    }

    let candidates = match discover_legacy_databases(roots) {
        Ok(candidates) => candidates,
        Err(error) => return classify_discovery_error(&error),
    };
    let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate_id(candidate) == selected_id)
    else {
        return Preparation::Failed(
            "The selected migration candidate is stale or no longer available. Refresh migration discovery and choose again."
                .to_owned(),
        );
    };
    let pending = data_dir.join(PENDING_STAGE_NAME);
    match validate_sqlite_header(candidate).and_then(|_| stage_legacy_database(candidate, &pending))
    {
        Ok(()) => Preparation::Ready,
        Err(error) if is_invalid_database_error(&error) => {
            Preparation::InvalidDatabase(safe_invalid_message(&error))
        }
        Err(error) => Preparation::Failed(safe_failure_message(&error)),
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
                Err(_) => {
                    return Err(
                        "A legacy database candidate could not be inspected safely.".to_owned(),
                    )
                }
            };
            if metadata.file_type().is_symlink() {
                return Err(
                    "A legacy database candidate is a symbolic link; Scriptotar refuses to follow it."
                        .to_owned(),
                );
            }
            if !metadata.is_file() {
                continue;
            }
            let canonical = fs::canonicalize(&candidate).map_err(|_| {
                "A legacy database candidate could not be resolved safely.".to_owned()
            })?;
            if seen.insert(canonical.clone()) {
                candidates.push(canonical);
            }
        }
    }
    candidates.sort();
    Ok(candidates)
}

fn candidate_id(path: &Path) -> String {
    let mut identity = path.to_string_lossy().into_owned();
    identity.push('|');
    identity.push_str(&file_identity(path));
    identity.push('|');
    identity.push_str(&file_identity(&path_with_suffix(path, "-wal")));
    let value = Uuid::new_v5(&Uuid::NAMESPACE_URL, identity.as_bytes());
    format!("candidate-{value}")
}

fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn file_identity(path: &Path) -> String {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return "missing".to_owned(),
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    metadata.len().hash(&mut hasher);
    modified.hash(&mut hasher);
    metadata.file_type().is_symlink().hash(&mut hasher);

    if metadata.is_file() {
        if let Ok(mut file) = fs::File::open(path) {
            let mut sample = vec![0_u8; IDENTITY_SAMPLE_BYTES.min(metadata.len() as usize)];
            if file.read_exact(&mut sample).is_ok() {
                sample.hash(&mut hasher);
            }
            if metadata.len() > IDENTITY_SAMPLE_BYTES as u64
                && file
                    .seek(SeekFrom::End(-(IDENTITY_SAMPLE_BYTES as i64)))
                    .is_ok()
            {
                let mut tail = vec![0_u8; IDENTITY_SAMPLE_BYTES];
                if file.read_exact(&mut tail).is_ok() {
                    tail.hash(&mut hasher);
                }
            }
        }
    }
    format!("{}:{modified}:{:016x}", metadata.len(), hasher.finish())
}

fn candidate_labels(candidates: &[PathBuf]) -> Vec<UiMigrationCandidate> {
    candidates
        .iter()
        .enumerate()
        .map(|(index, path)| UiMigrationCandidate {
            id: candidate_id(path),
            label: candidate_label(path, index),
        })
        .collect()
}

fn candidate_label(path: &Path, index: usize) -> String {
    let app = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .unwrap_or("legacy");
    let app = if app.eq_ignore_ascii_case("wesamboss") {
        "WeSamBoss"
    } else if app.eq_ignore_ascii_case("scriptotar") {
        "Scriptotar Classic"
    } else {
        "Legacy Scriptotar"
    };
    format!("{app} database (option {})", index + 1)
}

fn stage_legacy_database(candidate: &Path, staged: &Path) -> Result<(), String> {
    let data_dir = staged
        .parent()
        .ok_or_else(|| "The migration staging path has no parent directory.".to_owned())?;
    fs::create_dir_all(data_dir)
        .map_err(|_| "Scriptotar could not create the migration staging directory.".to_owned())?;
    harden_private_directory_permissions(data_dir)?;
    match fs::symlink_metadata(staged) {
        Ok(_) => {
            return Err(
                "A migration snapshot already exists; Scriptotar refused to overwrite it.".to_owned(),
            )
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err("Scriptotar could not inspect the migration snapshot path.".to_owned())
        }
    }

    let temporary_path = reserve_legacy_staging_path(data_dir)?;
    let snapshot_result = (|| -> Result<(), String> {
        let source = Connection::open_with_flags(
            candidate,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| "Scriptotar could not open the legacy database read-only.".to_owned())?;
        let mut destination = Connection::open(&temporary_path)
            .map_err(|_| "Scriptotar could not create a temporary SQLite snapshot.".to_owned())?;
        let backup = Backup::new(&source, &mut destination)
            .map_err(|_| "Scriptotar could not initialize the SQLite snapshot.".to_owned())?;
        backup
            .run_to_completion(64, Duration::from_millis(10), None)
            .map_err(|_| "Scriptotar could not complete the SQLite snapshot.".to_owned())?;
        drop(backup);
        drop(destination);
        fs::File::open(&temporary_path)
            .and_then(|file| file.sync_all())
            .map_err(|_| "Scriptotar could not flush the migration snapshot to disk.".to_owned())?;
        Ok(())
    })();
    if let Err(error) = snapshot_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    if let Err(error) = harden_private_file_permissions(&temporary_path)
        .and_then(|_| validate_sqlite_header(&temporary_path))
        .and_then(|_| match fs::symlink_metadata(staged) {
            Ok(_) => Err(
                "A migration snapshot appeared while Scriptotar was staging; it was not overwritten."
                    .to_owned(),
            ),
            Err(inspect_error) if inspect_error.kind() == io::ErrorKind::NotFound => {
                fs::rename(&temporary_path, staged)
                    .map_err(|_| "Scriptotar could not finalize the migration snapshot.".to_owned())
            }
            Err(_) => Err("Scriptotar could not inspect the migration snapshot path.".to_owned()),
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
            Err(_) => {
                return Err(
                    "Scriptotar could not reserve a private migration snapshot file.".to_owned(),
                )
            }
        }
    }
    Err("Scriptotar could not reserve a migration snapshot file.".to_owned())
}

fn validate_regular_sqlite(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| "The prepared migration snapshot is missing.".to_owned())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("The prepared migration snapshot is not a regular file.".to_owned());
    }
    validate_sqlite_header(path)
}

fn validate_sqlite_header(path: &Path) -> Result<(), String> {
    let mut file = fs::File::open(path)
        .map_err(|_| "The legacy database could not be opened for validation.".to_owned())?;
    let mut header = [0_u8; 16];
    file.read_exact(&mut header)
        .map_err(|_| "The legacy database is truncated or unreadable.".to_owned())?;
    if &header != SQLITE_HEADER {
        return Err("The legacy database does not have a valid SQLite header.".to_owned());
    }
    Ok(())
}

fn quarantine_unsafe_stage(path: &Path, data_dir: &Path) -> Result<(), String> {
    for attempt in 0..16_u8 {
        let quarantine = data_dir.join(format!(
            ".history.sqlite3.rejected-{}-{attempt}",
            process::id()
        ));
        match fs::rename(path, &quarantine) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => {
                return Err(
                    "Scriptotar could not quarantine an unsafe migration staging entry.".to_owned(),
                )
            }
        }
    }
    Err("Scriptotar could not quarantine an unsafe migration staging entry.".to_owned())
}

fn classify_discovery_error(error: &str) -> Preparation {
    if is_invalid_database_error(error) {
        Preparation::InvalidDatabase(safe_invalid_message(error))
    } else {
        Preparation::Failed(safe_failure_message(error))
    }
}

fn is_invalid_database_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("symbolic link")
        || lower.contains("sqlite header")
        || lower.contains("truncated")
        || lower.contains("not a regular file")
}

fn safe_invalid_message(_error: &str) -> String {
    "A discovered legacy database is not a safe, readable SQLite file. Scriptotar did not follow links or modify the source database."
        .to_owned()
}

fn safe_failure_message(_error: &str) -> String {
    "Legacy migration discovery or snapshotting failed. The source database was left untouched; retry after fixing storage or permission issues."
        .to_owned()
}

#[cfg(unix)]
fn harden_private_directory_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "Scriptotar could not restrict migration directory permissions.".to_owned())
}

#[cfg(not(unix))]
fn harden_private_directory_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn harden_private_file_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|_| "Scriptotar could not restrict migration snapshot permissions.".to_owned())
}

#[cfg(not(unix))]
fn harden_private_file_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    fn real_sqlite(path: &Path, value: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(&format!(
                "CREATE TABLE fixture(value TEXT); INSERT INTO fixture(value) VALUES ('{value}');"
            ))
            .unwrap();
    }

    fn fixture_values(path: &Path) -> Vec<String> {
        Connection::open(path)
            .unwrap()
            .prepare("SELECT value FROM fixture ORDER BY rowid")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn no_legacy_database_is_structured_not_an_error() {
        let temp = TempDir::new().unwrap();
        let data_dir = temp.path().join("next");
        let preparation = prepare_from_roots(&data_dir, &[temp.path().join("legacy-root")]);
        assert!(matches!(preparation, Preparation::NoLegacyDatabase));
    }

    #[test]
    fn invalid_legacy_database_is_reported_without_startup_failure() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("legacy-root");
        let source = root.join("scriptotar/history.sqlite3");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        let mut file = fs::File::create(&source).unwrap();
        file.write_all(b"not sqlite").unwrap();
        let data_dir = temp.path().join("next");

        let preparation = prepare_from_roots(&data_dir, &[root]);
        assert!(matches!(preparation, Preparation::InvalidDatabase(_)));
        assert!(source.exists());
        assert!(!data_dir.join(PENDING_STAGE_NAME).exists());
    }

    #[test]
    fn multiple_candidates_require_explicit_choice_and_selected_source_is_untouched() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("legacy-root");
        let first = root.join("scriptotar/history.sqlite3");
        let second = root.join("wesamboss/history.sqlite3");
        real_sqlite(&first, "first");
        real_sqlite(&second, "second");
        let first_before = fs::read(&first).unwrap();
        let second_before = fs::read(&second).unwrap();
        let data_dir = temp.path().join("next");

        let preparation = prepare_from_roots(&data_dir, std::slice::from_ref(&root));
        let Preparation::RequiresChoice(candidates) = preparation else {
            panic!("expected migration choice");
        };
        assert_eq!(candidates.len(), 2);
        assert!(!data_dir.join(PENDING_STAGE_NAME).exists());

        let selected = candidates
            .iter()
            .find(|candidate| candidate.label.contains("WeSamBoss"))
            .unwrap()
            .id
            .clone();
        let earlier_root = temp.path().join("000-earlier-root");
        real_sqlite(
            &earlier_root.join("scriptotar/history.sqlite3"),
            "newly-discovered",
        );
        let selected = choose_candidate_from_roots(&data_dir, &[earlier_root, root], &selected);
        assert!(matches!(selected, Preparation::Ready));
        let pending = data_dir.join(PENDING_STAGE_NAME);
        assert!(pending.is_file());
        assert_eq!(fixture_values(&pending), vec!["second"]);
        assert_eq!(fs::read(&first).unwrap(), first_before);
        assert_eq!(fs::read(&second).unwrap(), second_before);
    }

    #[test]
    fn stale_candidate_id_fails_when_source_changes() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("legacy-root");
        let first = root.join("scriptotar/history.sqlite3");
        let second = root.join("wesamboss/history.sqlite3");
        real_sqlite(&first, "first");
        real_sqlite(&second, "second");
        let data_dir = temp.path().join("next");
        let Preparation::RequiresChoice(candidates) =
            prepare_from_roots(&data_dir, std::slice::from_ref(&root))
        else {
            panic!("expected migration choice");
        };
        let selected = candidates
            .iter()
            .find(|candidate| candidate.label.contains("WeSamBoss"))
            .unwrap()
            .id
            .clone();

        let connection = Connection::open(&second).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE changed(payload BLOB); INSERT INTO changed(payload) VALUES (zeroblob(1048576));",
            )
            .unwrap();
        drop(connection);

        let result = choose_candidate_from_roots(&data_dir, &[root], &selected);
        assert!(matches!(result, Preparation::Failed(_)));
        assert!(!data_dir.join(PENDING_STAGE_NAME).exists());
    }

    #[test]
    fn disappeared_candidate_fails_without_staging_another_database() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("legacy-root");
        let first = root.join("scriptotar/history.sqlite3");
        let second = root.join("wesamboss/history.sqlite3");
        real_sqlite(&first, "first");
        real_sqlite(&second, "second");
        let data_dir = temp.path().join("next");
        let Preparation::RequiresChoice(candidates) =
            prepare_from_roots(&data_dir, std::slice::from_ref(&root))
        else {
            panic!("expected migration choice");
        };
        let selected = candidates
            .iter()
            .find(|candidate| candidate.label.contains("WeSamBoss"))
            .unwrap()
            .id
            .clone();
        fs::remove_file(second).unwrap();

        let result = choose_candidate_from_roots(&data_dir, &[root], &selected);
        assert!(matches!(result, Preparation::Failed(_)));
        assert!(!data_dir.join(PENDING_STAGE_NAME).exists());
    }

    #[test]
    fn second_selection_cannot_overwrite_pending_snapshot() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("legacy-root");
        real_sqlite(&root.join("scriptotar/history.sqlite3"), "first");
        real_sqlite(&root.join("wesamboss/history.sqlite3"), "second");
        let data_dir = temp.path().join("next");
        let Preparation::RequiresChoice(candidates) =
            prepare_from_roots(&data_dir, std::slice::from_ref(&root))
        else {
            panic!("expected migration choice");
        };
        let first_id = candidates[0].id.clone();
        let second_id = candidates[1].id.clone();
        assert!(matches!(
            choose_candidate_from_roots(&data_dir, std::slice::from_ref(&root), &first_id),
            Preparation::Ready
        ));
        let staged_before = fs::read(data_dir.join(PENDING_STAGE_NAME)).unwrap();
        assert!(matches!(
            choose_candidate_from_roots(&data_dir, &[root], &second_id),
            Preparation::Failed(_)
        ));
        assert_eq!(
            fs::read(data_dir.join(PENDING_STAGE_NAME)).unwrap(),
            staged_before
        );
    }

    #[test]
    fn pending_snapshot_survives_restart_discovery() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("legacy-root");
        real_sqlite(&root.join("scriptotar/history.sqlite3"), "source");
        let data_dir = temp.path().join("next");
        assert!(matches!(
            prepare_from_roots(&data_dir, std::slice::from_ref(&root)),
            Preparation::Ready
        ));
        assert!(matches!(
            prepare_from_roots(&data_dir, &[root]),
            Preparation::Ready
        ));
        assert!(data_dir.join(PENDING_STAGE_NAME).is_file());
    }

    #[test]
    fn one_candidate_uses_wal_aware_snapshot_and_can_be_activated() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().join("legacy-root");
        let source = root.join("scriptotar/history.sqlite3");
        fs::create_dir_all(source.parent().unwrap()).unwrap();

        let writer = Connection::open(&source).unwrap();
        writer.pragma_update(None, "journal_mode", "WAL").unwrap();
        writer.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
        writer
            .execute("CREATE TABLE fixture(value TEXT)", [])
            .unwrap();
        writer
            .execute("INSERT INTO fixture(value) VALUES ('before')", [])
            .unwrap();
        writer
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
        let reader = Connection::open(&source).unwrap();
        reader
            .execute_batch("BEGIN; SELECT count(*) FROM fixture;")
            .unwrap();
        writer
            .execute("INSERT INTO fixture(value) VALUES ('wal-commit')", [])
            .unwrap();
        assert!(path_with_suffix(&source, "-wal").is_file());

        let data_dir = temp.path().join("next");
        let preparation = prepare_from_roots(&data_dir, &[root]);
        assert!(matches!(preparation, Preparation::Ready));
        let pending = data_dir.join(PENDING_STAGE_NAME);
        assert_eq!(fixture_values(&pending), vec!["before", "wal-commit"]);
        let activated = activate_pending_stage(&data_dir).unwrap();
        assert_eq!(activated, data_dir.join(ACTIVE_STAGE_NAME));
        assert!(activated.is_file());
        reader.execute_batch("ROLLBACK;").unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn discovery_rejects_symlinked_legacy_database() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let root = temp.path().join("legacy-root");
        let real = temp.path().join("real.sqlite3");
        real_sqlite(&real, "source");
        let candidate = root.join("scriptotar/history.sqlite3");
        fs::create_dir_all(candidate.parent().unwrap()).unwrap();
        symlink(&real, &candidate).unwrap();

        let preparation = prepare_from_roots(&temp.path().join("next"), &[root]);
        assert!(matches!(preparation, Preparation::InvalidDatabase(_)));
    }

    #[cfg(unix)]
    #[test]
    fn staged_snapshot_permissions_are_private() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let root = temp.path().join("legacy-root");
        real_sqlite(&root.join("scriptotar/history.sqlite3"), "source");
        let data_dir = temp.path().join("next");
        assert!(matches!(
            prepare_from_roots(&data_dir, &[root]),
            Preparation::Ready
        ));
        assert_eq!(fs::metadata(&data_dir).unwrap().permissions().mode() & 0o777, 0o700);
        assert_eq!(
            fs::metadata(data_dir.join(PENDING_STAGE_NAME))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
