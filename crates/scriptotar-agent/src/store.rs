//! Persistence boundary. A production adapter must CAS snapshot+audit in one SQLite transaction.
use crate::RunSnapshot;
use std::{collections::HashMap, sync::{Arc, Mutex}};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Exists,
    NotFound,
    Conflict,
    Unavailable,
}

pub trait CheckpointStore: Send + Sync {
    fn create(&self, snapshot: RunSnapshot) -> Result<(), StoreError>;
    fn load(&self, run_id: &str) -> Result<Option<RunSnapshot>, StoreError>;
    /// Exact-revision conditional update. Snapshot and its audit events are one transaction.
    fn compare_and_swap(&self, expected_revision: u64, next: RunSnapshot) -> Result<(), StoreError>;
}

/// Deterministic fixture store only, NOT production persistence.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    runs: Arc<Mutex<HashMap<String, RunSnapshot>>>,
}

impl MemoryStore {
    /// Simulates process restart by rebuilding from serialized, persisted snapshots.
    pub fn from_snapshots(snapshots: impl IntoIterator<Item = RunSnapshot>) -> Self {
        Self {
            runs: Arc::new(Mutex::new(snapshots.into_iter().map(|run| (run.plan.id.clone(), run)).collect())),
        }
    }
}

impl CheckpointStore for MemoryStore {
    fn create(&self, snapshot: RunSnapshot) -> Result<(), StoreError> {
        let mut runs = self.runs.lock().map_err(|_| StoreError::Unavailable)?;
        if runs.contains_key(&snapshot.plan.id) { return Err(StoreError::Exists); }
        runs.insert(snapshot.plan.id.clone(), snapshot);
        Ok(())
    }

    fn load(&self, run_id: &str) -> Result<Option<RunSnapshot>, StoreError> {
        let runs = self.runs.lock().map_err(|_| StoreError::Unavailable)?;
        Ok(runs.get(run_id).cloned())
    }

    fn compare_and_swap(&self, expected_revision: u64, next: RunSnapshot) -> Result<(), StoreError> {
        let mut runs = self.runs.lock().map_err(|_| StoreError::Unavailable)?;
        let current = runs.get(&next.plan.id).ok_or(StoreError::NotFound)?;
        if current.revision != expected_revision || next.revision != expected_revision.checked_add(1).ok_or(StoreError::Conflict)? {
            return Err(StoreError::Conflict);
        }
        // Also protects immutable plan, historical records and checkpoint identity.
        if current.plan != next.plan || current.steps.len() != next.steps.len() || current.schema_version != next.schema_version {
            return Err(StoreError::Conflict);
        }
        runs.insert(next.plan.id.clone(), next);
        Ok(())
    }
}
