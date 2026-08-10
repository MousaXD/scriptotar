import importlib.util
import pathlib
import sys
import unittest


MODULE_PATH = pathlib.Path(__file__).resolve().parents[1] / "release_ci_gate.py"
spec = importlib.util.spec_from_file_location("release_ci_gate", MODULE_PATH)
release_ci_gate = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = release_ci_gate
assert spec.loader is not None
spec.loader.exec_module(release_ci_gate)

GateError = release_ci_gate.GateError
GATES = release_ci_gate.GATES
prove_release_gates = release_ci_gate.prove_release_gates

SHA = "a" * 40
OTHER_SHA = "b" * 40


def job(name, conclusion="success", status="completed"):
    return {"name": name, "status": status, "conclusion": conclusion}


def run_for(
    gate,
    run_id,
    *,
    sha=SHA,
    created_at="2026-08-10T10:00:00Z",
    status="completed",
    conclusion="success",
    job_overrides=None,
):
    jobs = [job(name) for name in gate.required_jobs]
    if job_overrides:
        by_name = {item["name"]: item for item in jobs}
        by_name.update(job_overrides)
        jobs = list(by_name.values())
    return {
        "databaseId": run_id,
        "headSha": sha,
        "headBranch": "main",
        "event": "push",
        "status": status,
        "conclusion": conclusion,
        "createdAt": created_at,
        "workflowName": gate.workflow_name,
        "jobs": jobs,
        "url": f"https://example.invalid/runs/{run_id}",
    }


class FakeClient:
    def __init__(self, listed, views):
        self.listed = listed
        self.views = views
        self.view_counts = {}

    def list_runs(self, workflow_file, source_sha):
        return list(self.listed.get(workflow_file, []))

    def view_run(self, run_id):
        sequence = self.views[run_id]
        if isinstance(sequence, list):
            index = self.view_counts.get(run_id, 0)
            self.view_counts[run_id] = index + 1
            return sequence[min(index, len(sequence) - 1)]
        return sequence


def healthy_fixture():
    listed = {}
    views = {}
    for index, gate in enumerate(GATES, start=1):
        payload = run_for(gate, index)
        listed[gate.workflow_file] = [payload]
        views[index] = payload
    return listed, views


def prove(client, *, discovery_attempts=1, wait_attempts=1, sleep=lambda _: None):
    return prove_release_gates(
        client,
        SHA,
        discovery_attempts=discovery_attempts,
        discovery_sleep_seconds=0,
        wait_attempts=wait_attempts,
        wait_sleep_seconds=0,
        sleep=sleep,
    )


class ReleaseGateTests(unittest.TestCase):
    def test_all_required_workflows_green_allows_publication(self):
        listed, views = healthy_fixture()
        result = prove(FakeClient(listed, views))
        self.assertEqual(set(result), {gate.key for gate in GATES})

    def test_integration_red_blocks_publication(self):
        listed, views = healthy_fixture()
        gate = next(g for g in GATES if g.key == "integration")
        failed = run_for(gate, 1, conclusion="failure")
        listed[gate.workflow_file] = [failed]
        views[1] = failed
        with self.assertRaisesRegex(
            GateError, "Tauri migration integration.*blocked publication"
        ):
            prove(FakeClient(listed, views))

    def test_security_red_blocks_publication(self):
        listed, views = healthy_fixture()
        gate = next(g for g in GATES if g.key == "security")
        failed = run_for(gate, 2, conclusion="failure")
        listed[gate.workflow_file] = [failed]
        views[2] = failed
        with self.assertRaisesRegex(GateError, "Security hygiene.*blocked publication"):
            prove(FakeClient(listed, views))

    def test_windows_package_red_blocks_publication(self):
        listed, views = healthy_fixture()
        gate = next(g for g in GATES if g.key == "windows")
        failed = run_for(gate, 3, conclusion="failure")
        listed[gate.workflow_file] = [failed]
        views[3] = failed
        with self.assertRaisesRegex(
            GateError, "Windows Tauri Installer.*blocked publication"
        ):
            prove(FakeClient(listed, views))

    def test_linux_package_red_blocks_publication(self):
        listed, views = healthy_fixture()
        gate = next(g for g in GATES if g.key == "linux")
        failed = run_for(gate, 4, conclusion="failure")
        listed[gate.workflow_file] = [failed]
        views[4] = failed
        with self.assertRaisesRegex(GateError, "Linux Tauri Package.*blocked publication"):
            prove(FakeClient(listed, views))

    def test_required_workflow_missing_blocks_publication(self):
        listed, views = healthy_fixture()
        gate = next(g for g in GATES if g.key == "security")
        listed[gate.workflow_file] = []
        with self.assertRaisesRegex(GateError, "Missing mandatory.*Security hygiene"):
            prove(FakeClient(listed, views))

    def test_wrong_sha_run_is_ignored(self):
        listed, views = healthy_fixture()
        gate = next(g for g in GATES if g.key == "integration")
        wrong = run_for(gate, 99, sha=OTHER_SHA)
        listed[gate.workflow_file] = [wrong]
        views[99] = wrong
        with self.assertRaisesRegex(
            GateError, "Missing mandatory.*Tauri migration integration"
        ):
            prove(FakeClient(listed, views))

    def test_stale_historical_green_run_is_ignored_in_favor_of_latest_exact_sha(self):
        listed, views = healthy_fixture()
        gate = next(g for g in GATES if g.key == "integration")
        historical_green = run_for(
            gate, 90, created_at="2026-08-10T09:00:00Z"
        )
        latest_failed = run_for(
            gate,
            91,
            created_at="2026-08-10T10:00:00Z",
            conclusion="failure",
        )
        listed[gate.workflow_file] = [historical_green, latest_failed]
        views[90] = historical_green
        views[91] = latest_failed
        with self.assertRaisesRegex(GateError, "run 91 blocked publication"):
            prove(FakeClient(listed, views))

    def test_skipped_mandatory_job_is_not_success(self):
        listed, views = healthy_fixture()
        gate = next(g for g in GATES if g.key == "integration")
        skipped_name = gate.required_jobs[0]
        skipped = run_for(
            gate,
            1,
            job_overrides={skipped_name: job(skipped_name, conclusion="skipped")},
        )
        listed[gate.workflow_file] = [skipped]
        views[1] = skipped
        with self.assertRaisesRegex(GateError, "mandatory job.*not successful"):
            prove(FakeClient(listed, views))

    def test_in_progress_run_is_waited_for_then_accepted(self):
        listed, views = healthy_fixture()
        gate = next(g for g in GATES if g.key == "linux")
        pending = run_for(gate, 4, status="in_progress", conclusion=None)
        complete = run_for(gate, 4)
        listed[gate.workflow_file] = [pending]
        views[4] = [pending, complete]
        sleeps = []
        result = prove(
            FakeClient(listed, views),
            wait_attempts=2,
            sleep=sleeps.append,
        )
        self.assertEqual(result["linux"]["conclusion"], "success")
        self.assertEqual(sleeps, [0])


if __name__ == "__main__":
    unittest.main()
