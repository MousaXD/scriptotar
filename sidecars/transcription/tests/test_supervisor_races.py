from __future__ import annotations

import io
import json
import threading
import unittest

from scriptotar_sidecar.protocol import ProtocolWriter
from scriptotar_sidecar.service import EngineSupervisor


class FakeProcess:
    def __init__(self, events: list[dict]):
        self.stdout = io.StringIO("".join(json.dumps(event) + "\n" for event in events))

    def wait(self):
        return 0

    def poll(self):
        return 0


class EngineTerminalRaceTests(unittest.TestCase):
    def supervisor(self) -> tuple[EngineSupervisor, io.StringIO]:
        output = io.StringIO()
        supervisor = EngineSupervisor(ProtocolWriter(output), io.StringIO())
        return supervisor, output

    def assert_abnormal_failure_published_under_lock(self, source) -> None:
        supervisor, output = self.supervisor()
        proc = FakeProcess([])
        supervisor.proc = proc
        supervisor.active_job_id = "job-1"
        original_emit = supervisor.writer.emit
        observed = []

        def check_emit(event_type, **payload):
            # A competing thread must not be able to begin a new job while
            # the previous fatal error is being published. This detects a
            # check-then-emit race, rather than relying on lucky scheduling.
            attempt = []

            def probe():
                acquired = supervisor._lock.acquire(blocking=False)
                attempt.append(acquired)
                if acquired:
                    supervisor._lock.release()

            thread = threading.Thread(target=probe)
            thread.start()
            thread.join(timeout=2)
            self.assertFalse(thread.is_alive())
            self.assertEqual(len(attempt), 1)
            observed.append(attempt[0])
            original_emit(event_type, **payload)

        supervisor.writer.emit = check_emit
        source(supervisor, proc)
        self.assertEqual(observed, [False], "fatal event was emitted outside arbitration lock")
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(len(events), 1)
        self.assertEqual(events[0]["type"], "error")
        self.assertEqual(events[0]["job_id"], "job-1")
        self.assertIsNone(supervisor.active_job_id)

    def test_engine_crash_error_is_serialized_with_next_job_start(self):
        self.assert_abnormal_failure_published_under_lock(
            lambda supervisor, proc: supervisor._read_stdout(proc)
        )

    def test_invalid_engine_protocol_error_is_serialized_with_next_job_start(self):
        self.assert_abnormal_failure_published_under_lock(
            lambda supervisor, proc: supervisor._abort_engine_protocol(proc, "bad JSON")
        )

    def test_cancel_requested_blocks_in_flight_result(self):
        supervisor, output = self.supervisor()
        proc = FakeProcess([
            {"type": "job_started", "job_id": "job-1"},
            {"type": "result", "job_id": "job-1", "result": {"output_dir": "/tmp/finished"}},
        ])
        supervisor.proc = proc
        supervisor.active_job_id = "job-1"
        supervisor._cancel_in_progress = True

        supervisor._read_stdout(proc)

        self.assertEqual(output.getvalue(), "", "cancelled job must not leak a successful result")

    def test_previous_worker_cannot_complete_replacement_workers_job(self):
        supervisor, output = self.supervisor()
        old_proc = FakeProcess([{"type": "result", "job_id": "same-job", "result": {}}])
        replacement = FakeProcess([])
        supervisor.proc = replacement
        supervisor.active_job_id = "same-job"
        supervisor._engine_job_started = True

        supervisor._read_stdout(old_proc)

        self.assertEqual(output.getvalue(), "")
        self.assertEqual(supervisor.active_job_id, "same-job")

    def test_current_worker_still_emits_result_once(self):
        supervisor, output = self.supervisor()
        proc = FakeProcess([
            {"type": "job_started", "job_id": "job-1"},
            {"type": "result", "job_id": "job-1", "result": {"ok": True}},
        ])
        supervisor.proc = proc
        supervisor.active_job_id = "job-1"

        supervisor._read_stdout(proc)

        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual([item["type"] for item in events], ["job_started", "result"])
        self.assertIsNone(supervisor.active_job_id)


if __name__ == "__main__":
    unittest.main()
