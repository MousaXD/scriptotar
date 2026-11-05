from __future__ import annotations

import io
import json
import subprocess
import threading
import unittest

from scriptotar_sidecar.errors import SidecarError
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

    def test_cancelled_acknowledgement_precedes_new_job_acceptance(self):
        supervisor, output = self.supervisor()
        proc = FakeProcess([])
        supervisor.proc = proc
        supervisor.active_job_id = "job-1"
        supervisor._engine_job_started = True
        supervisor._terminate_process_group = lambda _proc: None
        original_emit = supervisor.writer.emit
        observed = []

        def assert_serialized_emit(event_type, **payload):
            # Probe from a different thread: the main thread uses RLock and
            # could otherwise reacquire it even if the event is unprotected.
            attempts = []

            def probe():
                locked = supervisor._lock.acquire(blocking=False)
                attempts.append(locked)
                if locked:
                    supervisor._lock.release()

            other = threading.Thread(target=probe)
            other.start()
            other.join(timeout=2)
            self.assertFalse(other.is_alive())
            self.assertEqual(len(attempts), 1)
            observed.append((event_type, attempts[0]))
            original_emit(event_type, **payload)

        supervisor.writer.emit = assert_serialized_emit
        supervisor.cancel("job-1")

        self.assertEqual(observed, [("cancelled", False)])
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual(len(events), 1)
        self.assertEqual(events[0]["type"], "cancelled")
        self.assertEqual(events[0]["job_id"], "job-1")
        self.assertIsNone(supervisor.active_job_id)
        self.assertFalse(supervisor._cancel_in_progress)

    def test_cancel_does_not_acknowledge_termination_failure(self):
        supervisor, output = self.supervisor()
        proc = FakeProcess([])
        supervisor.proc = proc
        supervisor.active_job_id = "job-1"

        def fail_termination(_proc):
            raise RuntimeError("termination failed")

        supervisor._terminate_process_group = fail_termination
        with self.assertRaisesRegex(RuntimeError, "termination failed"):
            supervisor.cancel("job-1")
        self.assertIs(supervisor.proc, proc, "failed termination must preserve worker tracking")
        self.assertEqual(supervisor.active_job_id, "job-1", "failed termination must hold the job slot")
        self.assertTrue(supervisor._cancel_in_progress, "late events must remain suppressed")
        self.assertEqual(output.getvalue(), "", "failed termination must not be acknowledged as cancelled")
        with self.assertRaises(SidecarError) as busy:
            supervisor.start_job({"id": "successor"})
        self.assertEqual(busy.exception.code, "BUSY")

        # Once the worker is known to be dead, retry can safely finish.
        supervisor._terminate_process_group = lambda _proc: None
        supervisor.cancel("job-1")
        self.assertIsNone(supervisor.proc)
        self.assertIsNone(supervisor.active_job_id)
        self.assertFalse(supervisor._cancel_in_progress)
        events = [json.loads(line) for line in output.getvalue().splitlines()]
        self.assertEqual([event["type"] for event in events], ["cancelled"])

    def test_cancel_refuses_normal_termination_return_while_worker_is_alive(self):
        supervisor, output = self.supervisor()

        class StillRunningProcess:
            def poll(self):
                return None

        proc = StillRunningProcess()
        supervisor.proc = proc
        supervisor.active_job_id = "job-1"
        supervisor._terminate_process_group = lambda _proc: None

        with self.assertRaises(SidecarError) as raised:
            supervisor.cancel("job-1")
        self.assertEqual(raised.exception.code, "ENGINE_TERMINATION_FAILED")
        self.assertIs(supervisor.proc, proc)
        self.assertEqual(supervisor.active_job_id, "job-1")
        self.assertTrue(supervisor._cancel_in_progress)
        self.assertEqual(output.getvalue(), "")

    def test_termination_wait_timeout_is_not_silently_accepted(self):
        supervisor, _ = self.supervisor()

        class StillRunningProcess:
            pid = 4242

            def poll(self):
                return None

            def kill(self):
                pass

            def wait(self, timeout=None):
                raise subprocess.TimeoutExpired("fake-engine", timeout)

        proc = StillRunningProcess()
        # Exercise the real termination helper, not just a patched callback.
        # Use the Windows branch with a mocked taskkill invocation so POSIX
        # process groups on the runner are never signaled.
        from unittest.mock import patch
        from scriptotar_sidecar import service

        with (
            patch.object(service.os, "name", "nt"),
            patch.object(service.subprocess, "run", return_value=subprocess.CompletedProcess([], 0)),
        ):
            with self.assertRaises(SidecarError) as raised:
                supervisor._terminate_process_group(proc, grace_seconds=0)
        self.assertEqual(raised.exception.code, "ENGINE_TERMINATION_FAILED")

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
