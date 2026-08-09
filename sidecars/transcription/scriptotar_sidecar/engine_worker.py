from __future__ import annotations

import json
import os
import sys
import traceback
from typing import Any

from .engine import run_job
from .errors import SidecarError


def emit(event_type: str, **payload: Any) -> None:
    sys.stdout.write(json.dumps({"type": event_type, **payload}, ensure_ascii=False, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def main() -> int:
    if os.name == "posix":
        os.umask(0o077)
    emit("engine_ready")
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
                emit("job_started", job_id=job_id)

                def sink(event_type: str, payload: dict[str, Any]) -> None:
                    emit(event_type, job_id=job_id, **payload)

                try:
                    result = run_job(job, sink)
                except SidecarError as exc:
                    emit("error", job_id=job_id, error=exc.as_payload())
                except Exception as exc:
                    traceback.print_exc(file=sys.stderr)
                    emit(
                        "error",
                        job_id=job_id,
                        error={
                            "code": "ENGINE_ERROR",
                            "message": f"{type(exc).__name__}: {exc}",
                            "retryable": False,
                        },
                    )
                else:
                    emit("result", job_id=job_id, result=result)
            elif command_type == "shutdown":
                emit("engine_shutdown")
                return 0
            else:
                emit(
                    "error",
                    error={"code": "INTERNAL_PROTOCOL", "message": "Unknown internal engine command.", "retryable": False},
                )
        except Exception as exc:
            traceback.print_exc(file=sys.stderr)
            emit(
                "error",
                error={"code": "INTERNAL_PROTOCOL", "message": f"{type(exc).__name__}: {exc}", "retryable": False},
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
