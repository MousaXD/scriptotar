from __future__ import annotations

import json
import os
import sys
import traceback
from contextlib import contextmanager, redirect_stdout
from functools import partial
from typing import Any, TextIO

from .engine import run_job
from .errors import SidecarError


def emit(output: TextIO, event_type: str, **payload: Any) -> None:
    output.write(json.dumps({"type": event_type, **payload}, ensure_ascii=False, separators=(",", ":")) + "\n")
    output.flush()


@contextmanager
def protocol_output():
    # Preserve a dedicated protocol handle, then send dependency output to
    # stderr at both the Python and OS levels (including native libraries and
    # inherited subprocess stdout). Never relax the supervisor's JSON checks.
    sys.stdout.flush()
    stdout_fd = sys.stdout.fileno()
    with os.fdopen(os.dup(stdout_fd), "w", encoding="utf-8", buffering=1) as output:
        os.dup2(sys.stderr.fileno(), stdout_fd)
        try:
            with redirect_stdout(sys.stderr):
                yield output
        finally:
            sys.stderr.flush()
            os.dup2(output.fileno(), stdout_fd)


def _unexpected_error_payload(exc: Exception) -> dict[str, Any]:
    detail = f"{type(exc).__name__}: {exc}"
    message = (
        "The transcription engine could not complete the job. On first use the selected Whisper model "
        "may need network access to download; retry when online or choose a model that is already cached. "
        f"Technical detail: {detail}"
    )
    return {
        "code": "ENGINE_ERROR",
        "message": message[:2000],
        "retryable": True,
    }


def main() -> int:
    if os.name == "posix":
        os.umask(0o077)
    with protocol_output() as output:
        return run_commands(output)


def run_commands(output: TextIO) -> int:
    send = partial(emit, output)
    send("engine_ready")
    for raw_line in sys.stdin:
        line = raw_line.strip()
        if not line:
            continue
        try:
            command = json.loads(line)
            command_type = command.get("type")
            if command_type == "transcribe":
                job = command["job"]
                job_id = job["id"]
                send("job_started", job_id=job_id)

                def sink(event_type: str, payload: dict[str, Any]) -> None:
                    send(event_type, job_id=job_id, **payload)

                try:
                    result = run_job(job, sink)
                except SidecarError as exc:
                    send("error", job_id=job_id, error=exc.as_payload())
                except Exception as exc:
                    traceback.print_exc(file=sys.stderr)
                    send("error", job_id=job_id, error=_unexpected_error_payload(exc))
                else:
                    send("result", job_id=job_id, result=result)
            elif command_type == "shutdown":
                send("engine_shutdown")
                return 0
            else:
                send(
                    "error",
                    error={"code": "INTERNAL_PROTOCOL", "message": "Unknown internal engine command.", "retryable": False},
                )
        except Exception as exc:
            traceback.print_exc(file=sys.stderr)
            send(
                "error",
                error={"code": "INTERNAL_PROTOCOL", "message": f"{type(exc).__name__}: {exc}", "retryable": False},
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
