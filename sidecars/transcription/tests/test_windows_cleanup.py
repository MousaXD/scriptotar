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
        # Model the Popen API: idle shutdown tries the graceful stdin path
        # before falling back to verified process-tree termination.
        self.stdin = None

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

    def test_windows_taskkill_unavailable_fails_closed_instead_of_killing_only_parent(self):
        proc = FakeWindowsProcess()
        with (
            patch.object(service.os, "name", "nt"),
            patch.object(service.subprocess, "run", side_effect=FileNotFoundError),
        ):
            with self.assertRaises(service.SidecarError) as raised:
                self.supervisor()._terminate_process_group(proc)
        self.assertEqual(raised.exception.code, "ENGINE_TERMINATION_FAILED")
        self.assertTrue(proc.running)
        self.assertFalse(proc.terminated)

    def test_failed_tree_kill_stays_blocked_after_parent_exits(self):
        proc = FakeWindowsProcess()
        output = io.StringIO()
        supervisor = service.EngineSupervisor(ProtocolWriter(output), io.StringIO())
        supervisor.proc = proc
        supervisor.active_job_id = "windows-job"
        with (
            patch.object(service.os, "name", "nt"),
            patch.object(service.subprocess, "run", side_effect=FileNotFoundError),
        ):
            with self.assertRaises(service.SidecarError) as first:
                supervisor.cancel("windows-job")
        self.assertEqual(first.exception.code, "ENGINE_TERMINATION_FAILED")
        self.assertTrue(supervisor._windows_tree_unverified)
        # The reader can observe the parent exit and discard its handle,
        # but that says nothing about descendants after taskkill failed.
        proc.running = False
        supervisor.proc = None
        with self.assertRaises(service.SidecarError) as retried:
            supervisor.cancel("windows-job")
        self.assertEqual(retried.exception.code, "ENGINE_TERMINATION_FAILED")
        self.assertTrue(supervisor._cancel_in_progress)
        self.assertEqual(supervisor.active_job_id, "windows-job")
        self.assertEqual(output.getvalue(), "")
        with self.assertRaises(service.SidecarError) as busy:
            supervisor.start_job({"id": "after-failed-tree-kill"})
        self.assertEqual(busy.exception.code, "BUSY")

    def test_shutdown_retry_does_not_acknowledge_unverified_descendants(self):
        proc = FakeWindowsProcess()
        output = io.StringIO()
        supervisor = service.EngineSupervisor(ProtocolWriter(output), io.StringIO())
        supervisor.proc = proc
        supervisor.active_job_id = "windows-job"
        with (
            patch.object(service.os, "name", "nt"),
            patch.object(service.subprocess, "run", side_effect=FileNotFoundError),
        ):
            with self.assertRaises(service.SidecarError) as first:
                supervisor.shutdown()
        self.assertEqual(first.exception.code, "ENGINE_TERMINATION_FAILED")
        self.assertTrue(supervisor._windows_tree_unverified)
        self.assertIs(supervisor.proc, proc)
        self.assertEqual(supervisor.active_job_id, "windows-job")
        proc.running = False
        supervisor.proc = None
        with self.assertRaises(service.SidecarError) as retry:
            supervisor.shutdown()
        self.assertEqual(retry.exception.code, "ENGINE_TERMINATION_FAILED")
        self.assertEqual(supervisor.active_job_id, "windows-job")
        self.assertEqual(output.getvalue(), "")
        with self.assertRaises(service.SidecarError) as busy:
            supervisor.start_job({"id": "after-failed-shutdown"})
        self.assertEqual(busy.exception.code, "BUSY")

    def test_failed_idle_shutdown_blocks_future_transcription(self):
        proc = FakeWindowsProcess()
        supervisor = self.supervisor()
        supervisor.proc = proc
        with (
            patch.object(service.os, "name", "nt"),
            patch.object(service.subprocess, "run", side_effect=FileNotFoundError),
        ):
            with self.assertRaises(service.SidecarError):
                supervisor.shutdown()
        self.assertTrue(supervisor._closing)
        self.assertTrue(supervisor._windows_tree_unverified)
        with self.assertRaises(service.SidecarError) as busy:
            supervisor.start_job({"id": "next-job"})
        self.assertEqual(busy.exception.code, "BUSY")

    def test_windows_taskkill_nonzero_exit_preserves_worker_and_blocks_successor(self):
        proc = FakeWindowsProcess()
        output = io.StringIO()
        supervisor = service.EngineSupervisor(ProtocolWriter(output), io.StringIO())
        supervisor.proc = proc
        supervisor.active_job_id = "windows-job"
        with (
            patch.object(service.os, "name", "nt"),
            patch.object(
                service.subprocess,
                "run",
                side_effect=subprocess.CalledProcessError(1, ["taskkill"]),
            ),
        ):
            with self.assertRaises(service.SidecarError) as raised:
                supervisor.cancel("windows-job")
        self.assertEqual(raised.exception.code, "ENGINE_TERMINATION_FAILED")
        self.assertTrue(raised.exception.retryable)
        self.assertIs(supervisor.proc, proc)
        self.assertEqual(supervisor.active_job_id, "windows-job")
        self.assertTrue(supervisor._cancel_in_progress)
        self.assertTrue(proc.running)
        self.assertFalse(proc.terminated)
        self.assertEqual(output.getvalue(), "")
        with self.assertRaises(service.SidecarError) as busy:
            supervisor.start_job({"id": "successor"})
        self.assertEqual(busy.exception.code, "BUSY")


if __name__ == "__main__":
    unittest.main()
