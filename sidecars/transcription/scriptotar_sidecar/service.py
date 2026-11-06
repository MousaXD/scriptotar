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
        # A failed Windows taskkill /T cannot be treated as clean just because
        # the parent later exits; its FFmpeg/yt-dlp descendants may survive.
        self._windows_tree_unverified = False
        self._lock = threading.RLock()
        self._write_lock = threading.Lock()
        self._closing = False

    def _log(self, message: str) -> None:
        self.log_stream.write(message.rstrip() + "\n")
        self.log_stream.flush()

    def _engine_command(self) -> list[str]:
        packaged = os.environ.get("SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE")
        if packaged:
            path = Path(packaged).expanduser().resolve()
            if not path.is_file():
                raise SidecarError(
                    "ENGINE_CONFIG",
                    "Bundled transcription engine executable does not exist.",
                    details={"path": str(path)},
                )
            return [str(path)]

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
                creationflags=(subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0),
                env=self._child_env(),
            )
            threading.Thread(target=self._read_stdout, args=(self.proc,), daemon=True).start()
            threading.Thread(target=self._read_stderr, args=(self.proc,), daemon=True).start()

    def _abort_engine_protocol(self, proc: subprocess.Popen[str], message: str) -> None:
        self._log(f"[engine protocol] {message}")
        with self._lock:
            job_id = self.active_job_id
            tracked_worker = proc is self.proc
            should_report = (
                tracked_worker
                and job_id is not None
                and not self._cancel_in_progress
                and not self._closing
            )
            prior_cancel = self._cancel_in_progress
            if tracked_worker:
                # Never expose the job slot as free until the malformed worker
                # has actually stopped. This also suppresses its late events.
                self._cancel_in_progress = True
            try:
                self._terminate_process_group(proc)
            except Exception as exc:
                # A failed kill cannot be reported as a terminal protocol
                # failure: the worker or its descendants might still be live.
                # Keep the slot reserved, allowing explicit cancellation retry.
                self._log(f"[engine protocol] worker termination unconfirmed: {type(exc).__name__}")
                if should_report:
                    self.writer.emit(
                        "error",
                        job_id=job_id,
                        error={
                            "code": "ENGINE_TERMINATION_FAILED",
                            "message": "Transcription engine emitted invalid protocol data but could not be stopped.",
                            "retryable": True,
                        },
                    )
                return
            if tracked_worker:
                self._cancel_in_progress = prior_cancel
            if should_report:
                self.active_job_id = None
                self._engine_job_started = False
                # Publish the terminal error under the same cancel/start lock,
                # but only after confirmed termination.
                self.writer.emit(
                    "error",
                    job_id=job_id,
                    error={
                        "code": "ENGINE_PROTOCOL",
                        "message": "Transcription engine produced invalid protocol output.",
                        "retryable": True,
                    },
                )

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
            # Validate, publish and finalize each event under the same lock
            # used by cancel(). Otherwise a terminal result can be emitted
            # after cancellation has been acknowledged, or a defunct worker
            # can publish into a newly started job's event stream.
            with self._lock:
                if proc is not self.proc or self._cancel_in_progress or self._closing:
                    continue
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
                    self._engine_job_started = True
                elif not job_started:
                    self._abort_engine_protocol(
                        proc,
                        f"event {event_type!r} arrived before job_started for {job_id!r}",
                    )
                    break
                self.writer.emit(event_type, **event)
                if event_type in {"result", "error"}:
                    self.active_job_id = None
                    self._engine_job_started = False

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
                # Keep unexpected-exit errors ordered with the shared
                # cancel/start lock, just like ordinary terminal events.
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
            if self.active_job_id is not None or self._cancel_in_progress or self._closing:
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
            elif os.name == "nt":
                # terminate() only stops the worker, leaving child FFmpeg/yt-dlp
                # processes running. taskkill /T terminates its descendant tree.
                try:
                    subprocess.run(
                        ["taskkill", "/PID", str(proc.pid), "/T", "/F"],
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                        timeout=5,
                        check=True,
                    )
                except (OSError, subprocess.SubprocessError) as exc:
                    # Parent-only termination would leave FFmpeg/yt-dlp descendants
                    # alive while allowing a false cancellation acknowledgement.
                    # Even if the parent later exits, do not treat the tree as
                    # verified: the descendants might have survived.
                    self._windows_tree_unverified = True
                    raise SidecarError(
                        "ENGINE_TERMINATION_FAILED",
                        "Windows could not confirm transcription process-tree cleanup; the job remains blocked.",
                        retryable=True,
                    ) from exc
                self._windows_tree_unverified = False
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
        except subprocess.TimeoutExpired as exc:
            raise SidecarError(
                "ENGINE_TERMINATION_FAILED",
                "Transcription engine did not stop after cancellation; the job remains blocked.",
                retryable=True,
            ) from exc
        if proc.poll() is None:
            raise SidecarError(
                "ENGINE_TERMINATION_FAILED",
                "Transcription engine termination could not be confirmed; the job remains blocked.",
                retryable=True,
            )

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
                if proc.poll() is None:
                    raise SidecarError(
                        "ENGINE_TERMINATION_FAILED",
                        "Transcription engine is still running; cancellation was not confirmed.",
                        retryable=True,
                    )
            if self._windows_tree_unverified:
                # The reader thread can discard an exited parent handle.
                # Its exit does not prove children were reaped after taskkill
                # failed, so a subsequent cancel must not report success.
                raise SidecarError(
                    "ENGINE_TERMINATION_FAILED",
                    "Windows transcription descendants were not confirmed stopped; the job remains blocked.",
                    retryable=True,
                )
        except Exception:
            # The worker may still be producing FFmpeg/Whisper output. Keep
            # the job reserved and suppress its late events until a retry or
            # shutdown actually confirms that the worker has exited.
            # Never orphan a live process by clearing self.proc here.
            raise
        else:
            with self._lock:
                if self.proc is proc:
                    self.proc = None
                self.active_job_id = None
                self._engine_job_started = False
                self._cancel_in_progress = False
                # Serialize the ACK with a potential successor start_job.
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
            if self._windows_tree_unverified:
                # The worker may have exited after failed taskkill, but its
                # descendants were never proven stopped. Do not send a false
                # shutdown ACK or admit any new transcription work.
                raise SidecarError(
                    "ENGINE_TERMINATION_FAILED",
                    "Windows transcription descendants were not confirmed stopped; shutdown is blocked.",
                    retryable=True,
                )
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
