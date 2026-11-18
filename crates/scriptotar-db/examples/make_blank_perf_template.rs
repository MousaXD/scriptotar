//! Produce a disposable *real app-migrated* empty SQLite template for Agent 08.
//! Benchmark-only example; never run against an existing database or user directory.
//!
//! mkdir -p /tmp/scriptotar-perf
//! printf 'SYNTHETIC_ONLY\n' > /tmp/scriptotar-perf/.scriptotar-benchmark-only
//! cargo run --locked --release -p scriptotar-db --example make_blank_perf_template -- \
//!     /tmp/scriptotar-perf/blank-migrated.sqlite3

use std::{env, fs, fs::OpenOptions, path::PathBuf};

use scriptotar_db::{SqliteStore, LATEST_SCHEMA_VERSION};

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let path = args.next().map(PathBuf::from).ok_or_else(|| {
        "usage: make_blank_perf_template ABSOLUTE_OUTPUT_PATH (new file inside marked scratch directory)".to_owned()
    })?;
    if args.next().is_some() || !path.is_absolute() {
        return Err("exactly one absolute new output path is required".to_owned());
    }

    let parent = path
        .parent()
        .ok_or_else(|| "output requires an existing parent directory".to_owned())?;
    let parent = fs::canonicalize(parent)
        .map_err(|e| format!("cannot resolve scratch parent directory: {e}"))?;
    let sentinel = parent.join(".scriptotar-benchmark-only");
    if fs::read_to_string(&sentinel)
        .map(|text| text.trim() != "SYNTHETIC_ONLY")
        .unwrap_or(true)
    {
        return Err(format!(
            "refusing unmarked directory {}; create .scriptotar-benchmark-only containing SYNTHETIC_ONLY",
            parent.display()
        ));
    }

    let name = path
        .file_name()
        .ok_or_else(|| "output must be a file, not a directory".to_owned())?;
    let target = parent.join(name);
    // Exclusive creation prevents accidental overwrite or symlink-following.
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|e| format!("refusing to overwrite {}: {e}", target.display()))?;
    drop(file);

    let result = (|| {
        let store =
            SqliteStore::open(&target).map_err(|e| format!("canonical migration failed: {e}"))?;
        store
            .run_integration_migrations()
            .map_err(|e| format!("integration migrations failed: {e}"))?;
        let version = store
            .schema_version()
            .map_err(|e| format!("schema inspection failed: {e}"))?;
        if version != LATEST_SCHEMA_VERSION {
            return Err(format!(
                "migration mismatch: got v{version}, expected v{LATEST_SCHEMA_VERSION}"
            ));
        }
        Ok(version)
    })();

    match result {
        Ok(version) => {
            println!(
                "SYNTHETIC_ONLY canonical app-migrated blank template: {} (schema v{})",
                target.display(),
                version
            );
            Ok(())
        }
        Err(message) => {
            // This tool owns the newly created file, never an existing user DB.
            for suffix in ["", "-wal", "-shm"] {
                let _ = fs::remove_file(format!("{}{suffix}", target.display()));
            }
            Err(message)
        }
    }
}

fn main() {
    if let Err(message) = run() {
        eprintln!("performance template creation refused: {message}");
        std::process::exit(1);
    }
}
