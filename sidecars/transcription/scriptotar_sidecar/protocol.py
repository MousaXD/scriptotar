from __future__ import annotations

import json
import threading
from dataclasses import dataclass
from typing import Any, TextIO

from .errors import SidecarError
from .validation import validate_job_id, validate_job_payload
from .version import PROTOCOL_VERSION

KNOWN_TYPES = {"ping", "transcribe", "cancel", "shutdown"}


@dataclass(frozen=True, slots=True)
class Command:
    type: str
    protocol: int
    payload: dict[str, Any]


def parse_command_line(line: str) -> Command:
    try:
        raw = json.loads(line)
    except json.JSONDecodeError as exc:
        raise SidecarError(
            "MALFORMED_JSON",
            "Command must be one JSON object per line.",
            details={"line": exc.lineno, "column": exc.colno},
        ) from exc
    if not isinstance(raw, dict):
        raise SidecarError("INVALID_COMMAND", "Command must be a JSON object.")
    protocol = raw.get("protocol")
    if protocol != PROTOCOL_VERSION:
        raise SidecarError(
            "UNSUPPORTED_PROTOCOL",
            f"Unsupported protocol version {protocol!r}; expected {PROTOCOL_VERSION}.",
            details={"supported": [PROTOCOL_VERSION]},
        )
    command_type = raw.get("type")
    if not isinstance(command_type, str) or command_type not in KNOWN_TYPES:
        raise SidecarError(
            "UNKNOWN_COMMAND",
            "Unknown command type.",
            details={"type": command_type},
        )

    allowed: dict[str, set[str]] = {
        "ping": {"protocol", "type", "request_id"},
        "shutdown": {"protocol", "type", "request_id"},
        "cancel": {"protocol", "type", "request_id", "job_id"},
        "transcribe": {"protocol", "type", "request_id", "job_id", "input", "output", "options"},
    }
    unknown = sorted(set(raw) - allowed[command_type])
    if unknown:
        raise SidecarError(
            "INVALID_COMMAND",
            "Command contains unknown fields.",
            details={"fields": unknown},
        )

    payload = dict(raw)
    if command_type == "cancel":
        payload["job_id"] = validate_job_id(raw.get("job_id"))
    elif command_type == "transcribe":
        payload["job"] = validate_job_payload(raw)
    return Command(command_type, protocol, payload)


class ProtocolWriter:
    def __init__(self, stream: TextIO):
        self._stream = stream
        self._lock = threading.Lock()

    def emit(self, event_type: str, **payload: Any) -> None:
        event = {"protocol": PROTOCOL_VERSION, "type": event_type, **payload}
        encoded = json.dumps(event, ensure_ascii=False, separators=(",", ":")) + "\n"
        with self._lock:
            self._stream.write(encoded)
            self._stream.flush()
