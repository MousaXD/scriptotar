from __future__ import annotations

import json
import os
import queue
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SIDECAR = ROOT / "sidecar.py"
FAKE_ENGINE = Path(__file__).with_name("fake_engine_worker.py")


class SidecarProcess:
    def __init__(self, *, extra_env: dict[str, str] | None = None):
        env = os.environ.copy()
        env["PYTHONPATH"] = str(ROOT) + (os.pathsep + env["PYTHONPATH"] if env.get("PYTHONPATH") else "")
        env["SCRIPTOTAR_SIDECAR_ENGINE_WORKER"] = str(FAKE_ENGINE)
        env["PYTHONUNBUFFERED"] = "1"
        if extra_env:
            env.update(extra_env)
        self.proc = subprocess.Popen(
            [sys.executable, str(SIDECAR)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=env,
        )
        assert self.proc.stdout is not None
        assert self.proc.stderr is not None
        self.events: queue.Queue[dict] = queue.Queue()
        self.stderr_lines: queue.Queue[str] = queue.Queue()
        threading.Thread(target=self._read_stdout, daemon=True).start()
        threading.Thread(target=self._read_stderr, daemon=True).start()
        ready = self.next_event("ready")
        self.assert_event_shape(ready)
        assert ready["capabilities"]["protocol_versions"] == [1]
        assert "supported_models" in ready["capabilities"]

    def _read_stdout(self):
        assert self.proc.stdout is not None
        for line in self.proc.stdout:
            try:
                self.events.put(json.loads(line))
            except json.JSONDecodeError:
                self.events.put({"type": "__invalid_json__", "raw": line})

    def _read_stderr(self):
        assert self.proc.stderr is not None
        for line in self.proc.stderr:
            self.stderr_lines.put(line.rstrip())

    @staticmethod
    def assert_event_shape(event: dict):
        assert event.get("protocol") == 1, event
        assert isinstance(event.get("type"), str), event

    def send(self, payload: dict) -> None:
        assert self.proc.stdin is not None
        self.proc.stdin.write(json.dumps(payload) + "\n")
        self.proc.stdin.flush()

    def send_raw(self, line: str) -> None:
        assert self.proc.stdin is not None
        self.proc.stdin.write(line + "\n")
        self.proc.stdin.flush()

    def next_event(self, event_type: str | None = None, timeout: float = 5.0) -> dict:
        deadline = time.monotonic() + timeout
        skipped: list[dict] = []
        while time.monotonic() < deadline:
            remaining = max(0.01, deadline - time.monotonic())
            try:
                event = self.events.get(timeout=remaining)
            except queue.Empty:
                break
            self.assert_event_shape(event)
            if event_type is None or event.get("type") == event_type:
                for item in skipped:
                    self.events.put(item)
                return event
            skipped.append(event)
        raise AssertionError(f"Timed out waiting for event {event_type!r}; skipped={skipped!r}")

    def stderr_text(self, timeout: float = 0.3) -> str:
        deadline = time.monotonic() + timeout
        lines: list[str] = []
        while time.monotonic() < deadline:
            try:
                lines.append(self.stderr_lines.get(timeout=0.03))
            except queue.Empty:
                pass
        return "\n".join(lines)

    def close(self):
        if self.proc.poll() is None:
            try:
                self.send({"protocol": 1, "type": "shutdown"})
                self.next_event("shutdown", timeout=2.0)
            except Exception:
                self.proc.kill()
            try:
                self.proc.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=2)
        for stream in (self.proc.stdin, self.proc.stdout, self.proc.stderr):
            if stream is not None:
                try:
                    stream.close()
                except OSError:
                    pass


def command_for(job_id: str, source: Path, output: Path) -> dict:
    return {
        "protocol": 1,
        "type": "transcribe",
        "job_id": job_id,
        "input": {"kind": "file", "value": str(source)},
        "output": {"root": str(output)},
        "options": {"model": "small", "max_duration_seconds": 30},
    }


def process_dead(pid: int) -> bool:
    proc_stat = Path(f"/proc/{pid}/stat")
    if proc_stat.exists():
        try:
            fields = proc_stat.read_text(encoding="utf-8").split()
            if len(fields) > 2 and fields[2] == "Z":
                return True
        except OSError:
            pass
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return True
    except PermissionError:
        return False
    return False


class SubprocessProtocolTests(unittest.TestCase):
    def test_ping_ready_malformed_unknown_and_clean_shutdown(self):
        sidecar = SidecarProcess()
        try:
            sidecar.send({"protocol": 1, "type": "ping", "request_id": "ping-1"})
            pong = sidecar.next_event("pong")
            self.assertEqual(pong["request_id"], "ping-1")

            sidecar.send_raw("{broken")
            malformed = sidecar.next_event("error")
            self.assertEqual(malformed["error"]["code"], "MALFORMED_JSON")

            sidecar.send({"protocol": 1, "type": "does-not-exist"})
            unknown = sidecar.next_event("error")
            self.assertEqual(unknown["error"]["code"], "UNKNOWN_COMMAND")

            sidecar.send({"protocol": 999, "type": "ping"})
            unsupported = sidecar.next_event("error")
            self.assertEqual(unsupported["error"]["code"], "UNSUPPORTED_PROTOCOL")

            sidecar.send({"protocol": 1, "type": "shutdown", "request_id": "stop-1"})
            stopped = sidecar.next_event("shutdown")
            self.assertEqual(stopped["request_id"], "stop-1")
            self.assertEqual(sidecar.proc.wait(timeout=2), 0)
        finally:
            sidecar.close()

    def test_sequential_jobs_and_failed_job_then_successful_job(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            ok1 = root / "first.mp4"
            fail = root / "fail.mp4"
            ok2 = root / "second.mp4"
            for path in (ok1, fail, ok2):
                path.write_bytes(b"fixture")
            sidecar = SidecarProcess()
            try:
                sidecar.send(command_for("job-1", ok1, root / "out"))
                self.assertEqual(sidecar.next_event("accepted")["job_id"], "job-1")
                self.assertEqual(sidecar.next_event("result")["job_id"], "job-1")

                sidecar.send(command_for("job-fail", fail, root / "out"))
                sidecar.next_event("accepted")
                failed = sidecar.next_event("error")
                self.assertEqual(failed["job_id"], "job-fail")
                self.assertEqual(failed["error"]["code"], "FAKE_FAILURE")

                sidecar.send(command_for("job-2", ok2, root / "out"))
                sidecar.next_event("accepted")
                result = sidecar.next_event("result")
                self.assertEqual(result["job_id"], "job-2")
                self.assertEqual(result["result"]["transcript"]["clean_text"], "hello world")
            finally:
                sidecar.close()

    def test_library_stderr_never_corrupts_public_stdout(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "stderr.mp4"
            source.write_bytes(b"fixture")
            sidecar = SidecarProcess()
            try:
                sidecar.send(command_for("stderr-job", source, root / "out"))
                sidecar.next_event("accepted")
                sidecar.next_event("result")
                stderr = sidecar.stderr_text()
                self.assertIn("library diagnostic that must never reach public stdout", stderr)
                buffered = []
                while True:
                    try:
                        buffered.append(sidecar.events.get_nowait())
                    except queue.Empty:
                        break
                self.assertFalse(any(item.get("type") == "__invalid_json__" for item in buffered))
            finally:
                sidecar.close()

    @unittest.skipUnless(os.name == "posix", "process-group cleanup assertion is POSIX-specific")
    def test_cancellation_kills_engine_process_group_and_child(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "spawn-child.mp4"
            source.write_bytes(b"fixture")
            pid_file = root / "child.pid"
            sidecar = SidecarProcess(extra_env={"SCRIPTOTAR_TEST_CHILD_PID_FILE": str(pid_file)})
            try:
                sidecar.send(command_for("cancel-me", source, root / "out"))
                sidecar.next_event("accepted")
                sidecar.next_event("progress")
                deadline = time.monotonic() + 3
                while not pid_file.exists() and time.monotonic() < deadline:
                    time.sleep(0.03)
                self.assertTrue(pid_file.exists(), "fake engine did not record child pid")
                child_pid = int(pid_file.read_text(encoding="utf-8"))

                sidecar.send({"protocol": 1, "type": "cancel", "job_id": "cancel-me"})
                cancelled = sidecar.next_event("cancelled", timeout=4)
                self.assertEqual(cancelled["job_id"], "cancel-me")
                deadline = time.monotonic() + 3
                while not process_dead(child_pid) and time.monotonic() < deadline:
                    time.sleep(0.05)
                self.assertTrue(process_dead(child_pid), f"descendant process {child_pid} survived cancellation")

                successor = root / "successor.mp4"
                successor.write_bytes(b"fixture")
                sidecar.send(command_for("after-cancel", successor, root / "out"))
                sidecar.next_event("accepted")
                self.assertEqual(sidecar.next_event("result")["job_id"], "after-cancel")
            finally:
                sidecar.close()

    def test_engine_crash_reports_failure_and_next_job_still_runs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            crashing = root / "crash.mp4"
            successor = root / "successor.mp4"
            crashing.write_bytes(b"fixture")
            successor.write_bytes(b"fixture")
            sidecar = SidecarProcess()
            try:
                sidecar.send(command_for("crash-job", crashing, root / "out"))
                sidecar.next_event("accepted")
                error = sidecar.next_event("error", timeout=4)
                self.assertEqual(error["job_id"], "crash-job")
                self.assertEqual(error["error"]["code"], "ENGINE_CRASHED")

                sidecar.send(command_for("recovery-job", successor, root / "out"))
                sidecar.next_event("accepted")
                self.assertEqual(sidecar.next_event("result")["job_id"], "recovery-job")
            finally:
                sidecar.close()
