from __future__ import annotations

import io
import json
import unittest

from scriptotar_sidecar.protocol import ProtocolWriter
from scriptotar_sidecar.service import EngineSupervisor


class FakeProcess:
    def __init__(self, events: list[dict]):
        self.stdout = io.StringIO("".join(json.dumps(event) + "\n" for event in events))

    def wait(self):
        return 0


class EngineTerminalRaceTests(unittest.TestCase):
    def supervisor(self) -> tuple[EngineSupervisor, io.StringIO]:
        output = io.StringIO()
        supervisor = EngineSupervisor(ProtocolWriter(output), io.StringIO())
        return supervisor, output

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
