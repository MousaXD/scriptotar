//! Lightweight in-memory projection for the existing full-workspace bootstrap.
//!
//! Keep this aggregation separate from persistence. The DB crate owns future
//! query-level summary/pagination APIs; this only removes repeated CPU scans.

use std::collections::HashMap;

use scriptotar_core::Job;
use uuid::Uuid;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct ProjectJobSummary<'a> {
    pub(super) item_count: usize,
    pub(super) latest_updated_at: Option<&'a str>,
}

/// Summarize all jobs in O(jobs) time and O(projects-with-jobs) extra memory.
/// Timestamps retain the existing lexicographic comparison behavior.
pub(super) fn summarize_project_jobs(jobs: &[Job]) -> HashMap<Uuid, ProjectJobSummary<'_>> {
    let mut summaries = HashMap::new();
    for job in jobs {
        let summary: &mut ProjectJobSummary<'_> = summaries.entry(job.project_id).or_default();
        summary.item_count += 1;
        summary.latest_updated_at = Some(match summary.latest_updated_at {
            Some(previous) => previous.max(job.updated_at.as_str()),
            None => job.updated_at.as_str(),
        });
    }
    summaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use scriptotar_core::JobInput;

    fn job(project_id: Uuid, timestamp: &str) -> Job {
        let mut job = Job::new(
            project_id,
            JobInput::Url("https://example.com/video".to_owned()),
        );
        job.updated_at = timestamp.to_owned();
        job
    }

    #[test]
    fn empty_workspace_has_no_summaries() {
        assert!(summarize_project_jobs(&[]).is_empty());
    }

    #[test]
    fn aggregates_multiple_projects_and_uses_latest_update_not_iteration_order() {
        let first_project = Uuid::new_v4();
        let second_project = Uuid::new_v4();
        let untouched_project = Uuid::new_v4();
        let jobs = [
            job(first_project, "2026-10-12T10:00:00+00:00"),
            job(second_project, "2026-10-20T09:00:00+00:00"),
            job(first_project, "2026-10-02T11:00:00+00:00"),
            job(first_project, "2026-10-18T13:00:00+00:00"),
        ];

        let summaries = summarize_project_jobs(&jobs);
        assert_eq!(summaries.len(), 2);
        assert_eq!(
            summaries.get(&first_project),
            Some(&ProjectJobSummary {
                item_count: 3,
                latest_updated_at: Some("2026-10-18T13:00:00+00:00"),
            })
        );
        assert_eq!(
            summaries.get(&second_project),
            Some(&ProjectJobSummary {
                item_count: 1,
                latest_updated_at: Some("2026-10-20T09:00:00+00:00"),
            })
        );
        assert_eq!(summaries.get(&untouched_project), None);
    }

    #[test]
    fn one_job_per_project_does_not_mix_counts_or_timestamps() {
        let first_project = Uuid::new_v4();
        let second_project = Uuid::new_v4();
        let jobs = [
            job(first_project, "2026-10-01T00:00:00+00:00"),
            job(second_project, "2026-11-01T00:00:00+00:00"),
        ];
        let summaries = summarize_project_jobs(&jobs);
        assert_eq!(summaries.get(&first_project).unwrap().item_count, 1);
        assert_eq!(summaries.get(&second_project).unwrap().item_count, 1);
        assert_eq!(
            summaries.get(&first_project).unwrap().latest_updated_at,
            Some("2026-10-01T00:00:00+00:00")
        );
    }
}
