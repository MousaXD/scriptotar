//! Application-controlled, fail-closed permissions shared by Tauri and the MCP process.

mod http;
pub mod server;
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PERMISSIONS_FILE: &str = "mcp-permissions.json";
const MAX_POLICY_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct McpPermissions {
    pub enabled: bool,
    pub http_token: String,
    pub read_data: bool,
    pub write_data: bool,
    pub read_files: bool,
    pub write_files: bool,
    pub overwrite_files: bool,
    pub execute_commands: bool,
    pub search_creators: bool,
    pub download_videos: bool,
    pub allowed_paths: Vec<String>,
    pub allowed_programs: Vec<String>,
}

impl McpPermissions {
    pub fn require(&self, allowed: bool, capability: &str) -> Result<(), String> {
        if !self.enabled {
            return Err("MCP is disabled in Scriptotar Settings".into());
        }
        if !allowed {
            return Err(format!("{capability} permission is disabled in Scriptotar Settings"));
        }
        Ok(())
    }

    /// Only the desktop app calls save_permissions. The MCP protocol never exposes it.
    pub fn validate(&mut self) -> Result<(), String> {
        if !self.http_token.is_empty()
            && (self.http_token.len() != 64 || !self.http_token.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            return Err("invalid MCP HTTP bearer token".into());
        }
        if self.allowed_paths.len() > 16 || self.allowed_programs.len() > 16 {
            return Err("at most 16 approved directories and 16 programs are supported".into());
        }
        for path in &mut self.allowed_paths {
            *path = canonical_directory(path)?.to_string_lossy().into_owned();
        }
        for program in &mut self.allowed_programs {
            let path = strict_absolute(program)?;
            let canonical = fs::canonicalize(&path).map_err(|_| "approved executable does not exist".to_owned())?;
            if !canonical.is_file() {
                return Err("approved executable must be a file".into());
            }
            *program = canonical.to_string_lossy().into_owned();
        }
        self.allowed_paths.sort();
        self.allowed_paths.dedup();
        self.allowed_programs.sort();
        self.allowed_programs.dedup();
        if self.enabled {
            if (self.read_files || self.write_files) && self.allowed_paths.is_empty() {
                return Err("file access requires a directory explicitly approved in the app".into());
            }
            if self.execute_commands && self.allowed_programs.is_empty() {
                return Err("command execution requires an executable explicitly approved in the app".into());
            }
            if self.overwrite_files && !self.write_files {
                return Err("overwrite requires write-files permission".into());
            }
        }
        Ok(())
    }
}

fn strict_absolute(raw: &str) -> Result<PathBuf, String> {
    if raw.len() > 4096 || raw.contains('\0') || raw.chars().any(|c| c.is_control()) {
        return Err("invalid path".into());
    }
    let path = PathBuf::from(raw);
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) {
        return Err("an absolute normalized path is required".into());
    }
    Ok(path)
}

fn canonical_directory(raw: &str) -> Result<PathBuf, String> {
    let canonical =
        fs::canonicalize(strict_absolute(raw)?).map_err(|_| "approved directory does not exist".to_owned())?;
    if !canonical.is_dir() {
        return Err("approved path must be a directory".into());
    }
    Ok(canonical)
}

pub fn permissions_path(data_dir: &Path) -> PathBuf {
    data_dir.join(PERMISSIONS_FILE)
}

pub fn load_permissions(data_dir: &Path) -> Result<McpPermissions, String> {
    let path = permissions_path(data_dir);
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(McpPermissions::default()),
        Err(_) => return Err("MCP permissions cannot be inspected".into()),
    };
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > MAX_POLICY_BYTES {
        return Err("MCP permissions file is unsafe; access denied".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err("MCP permissions file has unsafe access rights; access denied".into());
        }
    }
    let content = fs::read(&path).map_err(|_| "MCP permissions cannot be read".to_owned())?;
    let mut policy: McpPermissions =
        serde_json::from_slice(&content).map_err(|_| "MCP permissions are invalid; access denied".to_owned())?;
    policy.validate()?;
    Ok(policy)
}

pub fn save_permissions(data_dir: &Path, mut policy: McpPermissions) -> Result<McpPermissions, String> {
    policy.validate()?;
    if policy.http_token.is_empty() {
        policy.http_token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    }
    fs::create_dir_all(data_dir).map_err(|_| "could not prepare Scriptotar data directory".to_owned())?;
    let dest = permissions_path(data_dir);
    if let Ok(meta) = fs::symlink_metadata(&dest) {
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("refusing to replace an unsafe MCP permissions file".into());
        }
    }
    let tmp = data_dir.join(format!(".mcp-permissions-{}.tmp", Uuid::new_v4()));
    let result = (|| -> Result<(), String> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp).map_err(|_| "could not create private permissions file".to_owned())?;
        file.write_all(&serde_json::to_vec_pretty(&policy).map_err(|e| e.to_string())?)
            .and_then(|_| file.sync_all())
            .map_err(|_| "could not persist MCP permissions".to_owned())?;
        drop(file);
        fs::rename(&tmp, &dest).map_err(|_| "could not activate MCP permissions".to_owned())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.map(|()| policy)
}

pub fn authorized_read(policy: &McpPermissions, data_dir: &Path, raw: &str) -> Result<PathBuf, String> {
    policy.require(policy.read_files, "read-files")?;
    let path = strict_absolute(raw)?;
    let resolved = fs::canonicalize(&path).map_err(|_| "file not found".to_owned())?;
    if !resolved.is_file() {
        return Err("not a regular file".into());
    }
    authorize_path(policy, &resolved)?;
    block_policy_file(data_dir, &resolved)?;
    Ok(resolved)
}

pub fn authorized_write(
    policy: &McpPermissions,
    data_dir: &Path,
    raw: &str,
    overwrite: bool,
) -> Result<PathBuf, String> {
    policy.require(policy.write_files, "write-files")?;
    if overwrite && !policy.overwrite_files {
        return Err("overwrite-files permission is disabled".into());
    }
    let path = strict_absolute(raw)?;
    let name = path.file_name().ok_or("file name required")?;
    let parent = fs::canonicalize(path.parent().ok_or("parent directory required")?)
        .map_err(|_| "parent directory does not exist".to_owned())?;
    let resolved = parent.join(name);
    authorize_path(policy, &resolved)?;
    block_policy_file(data_dir, &resolved)?;
    if let Ok(meta) = fs::symlink_metadata(&resolved) {
        if !overwrite {
            return Err("file exists; overwrite permission and overwrite=true required".into());
        }
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err("refusing to overwrite non-regular file or symlink".into());
        }
    }
    Ok(resolved)
}

pub fn authorized_program(policy: &McpPermissions, raw: &str) -> Result<PathBuf, String> {
    policy.require(policy.execute_commands, "execute-commands")?;
    let actual = fs::canonicalize(strict_absolute(raw)?).map_err(|_| "executable not found".to_owned())?;
    if !policy.allowed_programs.iter().any(|program| Path::new(program) == actual) {
        return Err("executable is not approved in Scriptotar Settings".into());
    }
    if !actual.is_file() {
        return Err("executable is not a file".into());
    }
    Ok(actual)
}

pub fn authorized_directory(policy: &McpPermissions, raw: &str) -> Result<PathBuf, String> {
    let path = canonical_directory(raw)?;
    authorize_path(policy, &path)?;
    Ok(path)
}

fn authorize_path(policy: &McpPermissions, path: &Path) -> Result<(), String> {
    if policy.allowed_paths.iter().any(|root| path.starts_with(root)) {
        Ok(())
    } else {
        Err("path is outside the directories approved in Scriptotar Settings".into())
    }
}

fn block_policy_file(data_dir: &Path, path: &Path) -> Result<(), String> {
    // Distinct database permissions must not be bypassed through file tools.
    // SQLite WAL/SHM files can contain database contents and transactional state.
    let protected = ["mcp-permissions.json", "scriptotar.sqlite3", "scriptotar.sqlite3-wal", "scriptotar.sqlite3-shm"];
    if path.parent() == Some(data_dir)
        && path.file_name().and_then(|name| name.to_str()).is_some_and(|name| protected.contains(&name))
    {
        Err("MCP cannot access its permissions or Scriptotar SQLite database through file tools".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_policy_denies_everything() {
        let dir = tempfile::tempdir().unwrap();
        let p = load_permissions(dir.path()).unwrap();
        assert!(p.require(p.read_data, "data").is_err());
    }
    #[test]
    fn grants_are_scoped_and_runtime_revocable() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("allowed");
        fs::create_dir_all(&sub).unwrap();
        let secret = dir.path().join("secret.txt");
        fs::write(&secret, "no").unwrap();
        let mut p = McpPermissions {
            enabled: true,
            read_files: true,
            write_files: true,
            allowed_paths: vec![sub.to_string_lossy().into()],
            ..Default::default()
        };
        p = save_permissions(dir.path(), p).unwrap();
        assert!(authorized_read(&p, dir.path(), secret.to_str().unwrap()).is_err());
        assert!(authorized_write(&p, dir.path(), sub.join("new.txt").to_str().unwrap(), false).is_ok());
        assert!(authorized_read(&p, dir.path(), permissions_path(dir.path()).to_str().unwrap()).is_err());
        p.enabled = false;
        save_permissions(dir.path(), p).unwrap();
        let revoked = load_permissions(dir.path()).unwrap();
        assert!(authorized_write(&revoked, dir.path(), sub.join("new.txt").to_str().unwrap(), false).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn symlink_and_public_policy_are_rejected() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("file");
        fs::write(&f, "{}").unwrap();
        symlink(&f, permissions_path(dir.path())).unwrap();
        assert!(load_permissions(dir.path()).is_err());
        fs::remove_file(permissions_path(dir.path())).unwrap();
        fs::copy(&f, permissions_path(dir.path())).unwrap();
        fs::set_permissions(permissions_path(dir.path()), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load_permissions(dir.path()).is_err());
    }
}
