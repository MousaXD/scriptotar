"""Fail-closed evidence-label validator. Validates attestations, not their truth."""
import re

SHA40 = re.compile(r"^[0-9a-f]{40}$")
SHA64 = re.compile(r"^[0-9a-f]{64}$")
SCOPES = {"native-installed", "browser-mock", "rust-ipc", "mcp-e2e",
          "sidecar-fixture", "provider-live", "manual-native"}

def verify(record):
    if not SHA40.fullmatch(str(record.get("source_sha", ""))):
        raise ValueError("exact source_sha required")
    scope = record.get("scope")
    if scope not in SCOPES:
        raise ValueError("unknown evidence scope")
    outcome = record.get("outcome")
    if outcome not in {"PASS", "FAIL", "BLOCKED", "NOT_RUN"}:
        raise ValueError("invalid outcome")
    if outcome != "PASS":
        return outcome
    if not record.get("run_url") or not record.get("observed_at"):
        raise ValueError("PASS requires exact-head run_url and observed_at")
    if scope == "native-installed":
        for key in ("package_sha256", "executable_sha256"):
            if not SHA64.fullmatch(str(record.get(key, ""))):
                raise ValueError(key + " required for installed app")
        hw = record.get("hardware")
        if not isinstance(hw, dict) or not all(hw.get(k) for k in ("os", "cpu", "ram_bytes")):
            raise ValueError("native hardware OS/CPU/RAM required")
        if record.get("native_ui_probe") in (None, "", "browser-mock", "operator-file"):
            raise ValueError("native UI verification required; not a browser mock or operator file")
        if not record.get("method") or not record.get("measurements"):
            raise ValueError("native measurement method/data required")
    if scope == "browser-mock" and not record.get("mock_only"):
        raise ValueError("browser results must be marked mock_only")
    return "PASS"
