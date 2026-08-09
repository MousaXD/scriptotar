from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(slots=True)
class SidecarError(Exception):
    code: str
    message: str
    retryable: bool = False
    details: dict[str, Any] | None = None

    def as_payload(self) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "code": self.code,
            "message": self.message,
            "retryable": self.retryable,
        }
        if self.details:
            payload["details"] = self.details
        return payload
