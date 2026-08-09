from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any, TextIO

from .capabilities import capability_report
from .errors import SidecarError
from .protocol import Command, ProtocolWriter, parse_command_line
from .version import SIDECAR_VERSION

MAX_COMMAND_LINE_CHARS = 1024 * 1024
MAX_ENGINE_EVENT_LINE_CHARS = 64 * 1024 * 1024


def _read_bounded_line(stream: TextIO, limit: int) -> tuple[str | None, bool]:
    raw = stream.readline(limit + 1)
    if raw == "":
        return None, False
    if len(raw) <= limit:
        return raw, False

    ended = raw.endswith("\n")
    while not ended:
        remainder = stream.readline(limit + 1)
        if remainder == "":
            break
        ended = remainder.endswith("\n")
    return "", True


class EngineSupervisor:
    def __init__(self, writer: ProtocolWriter, log_stream: TextIO):
        self.writer = writer
        self.log_stream = log_stream
        self.proc: subprocess.Popen[str] | None = None
        self.active_job_id: str | None = None
        self._engine_job_started = False
        self._cancel_in_progress = False
        self._lock = threading.RLock()
        self._write_lock = threading.Lock()
        self._closing = False

    def _log(self, message: str) -> None:
        self.log_stream.write(message.rstrip() + "\n")
        self.log_stream.flush()

    def _engine_command(self) -> list[str]:
        override = os.environ.get("SCRIPTOTAR_SIDECAR_ENGINE_WORKER")
        if override:
            path = Path(override).expanduser().resolve()
            if not path.is_file():
                raise SidecarError("ENGINE_CONFIG", "Configured engine worker path does not exist.")
            return [sys.executable, str(path)]
        return [sys.executable, "-m", "scriptotar_sidecar.engine_worker"]

    def _child_env(self) -> dict[str, str]:
        env = os.environ.copy()
        package_root = str(Path(__file__).resolve().parents[1])
        current = env.get("PYTHONPATH")
        env["PYTHONPATH"] = package_root if not current else package_root + os.pathsep + current
        env["PYTHONUNBUFFERED"] = "1"
        return env

    def ensure_started(self) -> None:
        with self._lock:
            if self.proc is not None and self.proc.poll() is None:
                return
            self.proc = subprocess.Popen(
                self._engine_command(),
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
                start_new_session=(os.name == "posix"),
                env=self._child_env(),
            )
            threading.Thread(target=self._read_stdout, args=(self.proc,), daemon=True).start()
            threading.Thread(target=self._read_stderr, args=(self.proc,), daemon=True).start()

    def _abort_engine_protocol(self, proc: subprocess.Popen[str], message: str) -> None:
        self._log(f"[engine protocol] {message}")
        with self._lock:
            job_id = self.active_job_id
            should_report = (
                proc is self.proc
                and job_id is not None
                and not self._cancel_in_progress
                and not self._closing
            )
            if should_report:
                self.active_job_id = None
                self._engine_job_started = False
        if should_report:
            self.writer.emit(
                "error",
                job_id=job_id,
                error={
                    "code": "ENGINE_PROTOCOL",
                    "message": "Transcription engine produced invalid protocol output.",
                    "retryable": True,
                },
            )
        self._terminate_process_group(proc)

    def _read_stdout(self, proc: subprocess.Popen[str]) -> None:
        assert proc.stdout is not None
        while True:
            raw, too_large = _read_bounded_line(proc.stdout, MAX_ENGINE_EVENT_LINE_CHARS)
            if raw is None:
                break
            if too_large:
                self._abort_engine_protocol(proc, "event exceeded the maximum protocol line size")
                break
            line = raw.strip()
            if not line:
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                self._abort_engine_protocol(proc, f"malformed JSON event: {line[:1000]}")
                break
            if not isinstance(event, dict) or not isinstance(event.get("type"), str):
                self._abort_engine_protocol(proc, f"invalid event shape: {line[:1000]}")
                break
            event_type = event.pop("type")
            if event_type in {"engine_ready", "engine_shutdown"}:
                continue
            if event_type not in {"job_started", "progress", "result", "error"}:
                self._abort_engine_protocol(proc, f"unknown event type: {event_type!r}")
                break
            job_id = event.get("job_id")
            if not isinstance(job_id, str):
                self._abort_engine_protocol(proc, f"event {event_type!r} is missing a job_id")
                break
            with self._lock:
                active_job_id = self.active_job_id
                job_started = self._engine_job_started
            if active_job_id is None:
                self._abort_engine_protocol(proc, f"stale event for inactive job {job_id!r}")
                break
            if job_id != active_job_id:
                self._abort_engine_protocol(
                    proc,
                    f"event job id mismatch: expected {active_job_id!r}, got {job_id!r}",
                )
                break
            if event_type == "job_started":
                if job_started:
                    self._abort_engine_protocol(proc, f"duplicate job_started for {job_id!r}")
                    break
                with self._lock:
                    self._engine_job_started = True
            elif not job_started:
                self._abort_engine_protocol(
                    proc,
                    f"event {event_type!r} arrived before job_started for {job_id!r}",
                )
                break
            if event_type in {"result", "error"}:
                with self._lock:
                    if self.active_job_id == job_id:
                        self.active_job_id = None
                        self._engine_job_started = False
            self.writer.emit(event_type, **event)

        rc = proc.wait()
        with self._lock:
            job_id = self.active_job_id
            should_report = (
                proc is self.proc
                and job_id is not None
                and not self._cancel_in_progress
                and not self._closing
            )
            if proc is self.proc:
                self.proc = None
            if should_report:
                self.active_job_id = None
                self._engine_job_started = False
        if should_report:
            self.writer.emit(
                "error",
                job_id=job_id,
                error={
                    "code": "ENGINE_CRASHED",
                    "message": f"Transcription engine exited unexpectedly with code {rc}.",
                    "retryable": True,
                },
            )

    def _read_stderr(self, proc: subprocess.Popen[str]) -> None:
        assert proc.stderr is not None
        for raw in proc.stderr:
            self._log(f"[engine] {raw.rstrip()}")

    def _send_internal(self, payload: dict[str, Any]) -> None:
        proc = self.proc
        if proc is None or proc.poll() is not None or proc.stdin is None:
            raise SidecarError("ENGINE_UNAVAILABLE", "Transcription engine is not running.", retryable=True)
        encoded = json.dumps(payload, ensure_ascii=False, separators=(",", ":")) + "\n"
        with self._write_lock:
            proc.stdin.write(encoded)
            proc.stdin.flush()

    def start_job(self, job: dict[str, Any]) -> None:
        with self._lock:
            if self.active_job_id is not None:
                raise SidecarError(
                    "BUSY",
                    "The sidecar runs one transcription job at a time.",
                    retryable=True,
                    details={"active_job_id": self.active_job_id},
                )
            self.ensure_started()
            self.active_job_id = job["id"]
            self._engine_job_started = False
            try:
                self._send_internal({"type": "transcribe", "job": job})
            except Exception:
                self.active_job_id = None
                self._engine_job_started = False
                raise

    def _terminate_process_group(self, proc: subprocess.Popen[str], grace_seconds: float = 1.5) -> None:
        if proc.poll() is not None:
            return
        try:
            if os.name == "posix":
                os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
            else:
                proc.terminate()
        except ProcessLookupError:
            return
        except OSError:
            proc.terminate()
        deadline = time.monotonic() + grace_seconds
        while proc.poll() is None and time.monotonic() < deadline:
            time.sleep(0.03)
        if proc.poll() is None:
            try:
                if os.name == "posix":
                    os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
                else:
                    proc.kill()
            except ProcessLookupError:
                pass
            except OSError:
                proc.kill()
        try:
            proc.wait(timeout=1)
        except subprocess.TimeoutExpired:
            pass

    def cancel(self, job_id: str) -> None:
        with self._lock:
            if self.active_job_id != job_id:
                raise SidecarError(
                    "NOT_RUNNING",
                    "The requested job is not currently running.",
                    details={"active_job_id": self.active_job_id},
                )
            proc = self.proc
            self._cancel_in_progress = True
        try:
            if proc is not None:
                self._terminate_process_group(proc)
        finally:
            with self._lock:
                if self.proc is proc:
                    self.proc = None
                self.active_job_id = None
                self._engine_job_started = False
                self._cancel_in_progress = False
        self.writer.emit("cancelled", job_id=job_id, reason="user_requested")

    def shutdown(self) -> None:
        with self._lock:
            self._closing = True
            proc = self.proc
            active = self.active_job_id
        if proc is not None and proc.poll() is None:
            if active is None:
                try:
                    self._send_internal({"type": "shutdown"})
                    proc.wait(timeout=1.5)
                except (SidecarError, subprocess.TimeoutExpired):
                    self._terminate_process_group(proc)
            else:
                self._terminate_process_group(proc)
        with self._lock:
            self.proc = None
            self.active_job_id = None
            self._engine_job_started = False


class SidecarService:
    def __init__(self, stdout: TextIO, stderr: TextIO):
        self.writer = ProtocolWriter(stdout)
        self.stderr = stderr
        self.engine = EngineSupervisor(self.writer, stderr)
        self.should_exit = False

    def emit_error(self, exc: SidecarError, *, job_id: str | None = None, request_id: Any = None) -> None:
        payload: dict[str, Any] = {"error": exc.as_payload()}
        if job_id is not None:
            payload["job_id"] = job_id
        if request_id is not None:
            payload["request_id"] = request_id
        self.writer.emit("error", **payload)

    def handle(self, command: Command) -> None:
        payload = command.payload
        request_id = payload.get("request_id")
        try:
            if command.type == "ping":
                response: dict[str, Any] = {"sidecar_version": SIDECAR_VERSION}
                if request_id is not None:
                    response["request_id"] = request_id
                self.writer.emit("pong", **response)
            elif command.type == "transcribe":
                job = payload["job"]
                self.engine.start_job(job)
                response = {"job_id": job["id"]}
                if request_id is not None:
                    response["request_id"] = request_id
                self.writer.emit("accepted", **response)
            elif command.type == "cancel":
                self.engine.cancel(payload["job_id"])
            elif command.type == "shutdown":
                self.engine.shutdown()
                response = {}
                if request_id is not None:
                    response["request_id"] = request_id
                self.writer.emit("shutdown", **response)
                self.should_exit = True
        except SidecarError as exc:
            self.emit_error(exc, job_id=payload.get("job_id"), request_id=request_id)

    def run(self, stdin: TextIO) -> int:
        self.writer.emit("ready", capabilities=capability_report())
        while True:
            raw_line, too_large = _read_bounded_line(stdin, MAX_COMMAND_LINE_CHARS)
            if raw_line is None:
                break
            if self.should_exit:
                break
            if too_large:
                self.emit_error(
                    SidecarError(
                        "COMMAND_TOO_LARGE",
                        "Command exceeds the maximum protocol line size.",
                    )
                )
                continue
            line = raw_line.strip()
            if not line:
                continue
            try:
                command = parse_command_line(line)
            except SidecarError as exc:
                self.emit_error(exc)
                continue
            self.handle(command)
            if self.should_exit:
                break
        if not self.should_exit:
            self.engine.shutdown()
        return 0
