from pathlib import Path

path = Path('crates/scriptotar-orchestrator/src/lib.rs')
text = path.read_text()
old = '''    if repository
        .get_job(job_id)
        .is_ok_and(|job| job.state == JobState::Queued)
    {
        if repository
            .transition_job(job_id, JobState::Cancelled)
            .is_ok()
        {
            notify_job(notifier, job_id);
        }
    }
'''
new = '''    if repository
        .get_job(job_id)
        .is_ok_and(|job| job.state == JobState::Queued)
        && repository
            .transition_job(job_id, JobState::Cancelled)
            .is_ok()
    {
        notify_job(notifier, job_id);
    }
'''
if old not in text:
    raise SystemExit('expected cancel_queued fragment was not found')
path.write_text(text.replace(old, new, 1))
