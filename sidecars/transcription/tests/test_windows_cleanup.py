from __future__ import annotations

import io
import subprocess
import unittest
from unittest.mock import patch

from scriptotar_sidecar import service
from scriptotar_sidecar.protocol import ProtocolWriter


class FakeWindowsProcess:
    pid = 4242

    def __init__(self):
        self.running = True
        self.terminated = False

    def poll(self):
        return None if self.running else 0

    def wait(self, timeout=None):
        return 0

    def terminate(self):
        self.terminated = True
        self.running = False

    def kill(self):
        self.running = False


class WindowsTreeTerminationTests(unittest.TestCase):
    def supervisor(self):
        return service.EngineSupervisor(ProtocolWriter(io.StringIO()), io.StringIO())

    def test_windows_uses_taskkill_with_descendant_flag(self):
        proc = FakeWindowsProcess()

        def finish_tree(*_args, **_kwargs):
            proc.running = False
            return subprocess.CompletedProcess(args=[], returncode=0)

        with (
            patch.object(service.os, "name", "nt"),
            patch.object(service.subprocess, "run", side_effect=finish_tree) as run,
        ):
            self.supervisor()._terminate_process_group(proc)
        self.assertEqual(run.call_args.args[0], ["taskkill", "/PID", "4242", "/T", "/F"])
        self.assertFalse(proc.terminated)

    def test_windows_falls_back_to_parent_termination_if_taskkill_unavailable(self):
        proc = FakeWindowsProcess()
        with (
            patch.object(service.os, "name", "nt"),
            patch.object(service.subprocess, "run", side_effect=FileNotFoundError),
        ):
            self.supervisor()._terminate_process_group(proc)
        self.assertTrue(proc.terminated)


if __name__ == "__main__":
    unittest.main()
