"""Fail-closed evidence-label validator. Validates report structure, NEVER its authenticity."""
import math
import re
from datetime import datetime
from urllib.parse import urlparse

SHA40 = re.compile(r"^[0-9a-f]{40}$")
SHA64 = re.compile(r"^[0-9a-f]{64}$")
SCOPES = {"native-installed", "browser-mock", "rust-ipc", "mcp-e2e",
          "sidecar-fixture", "provider-live", "manual-native"}
NATIVE_METRICS = {
    "cold_start_ms", "warm_start_ms", "useful_workspace_ms",
    "ipc_bootstrap_payload_bytes", "peak_process_tree_rss_bytes"
}

def _positive_number(value):
    return (type(value) in (int, float) and math.isfinite(value) and value > 0)

def verify(record):
    if not isinstance(record, dict) or not SHA40.fullmatch(str(record.get("source_sha", ""))):
        raise ValueError("exact source_sha required")
    scope = record.get("scope")
    if scope not in SCOPES:
        raise ValueError("unknown evidence scope")
    outcome = record.get("outcome")
    if outcome not in {"PASS", "FAIL", "BLOCKED", "NOT_RUN"}:
        raise ValueError("invalid outcome")
    if outcome != "PASS":
        return outcome
    uri = urlparse(str(record.get("run_url", "")))
    if (uri.scheme != "https" or uri.hostname != "github.com" or uri.port is not None
            or not re.fullmatch(r"/MousaXD/scriptotar/actions/runs/[1-9][0-9]*", uri.path)
            or uri.query or uri.fragment or uri.username or uri.password):
        raise ValueError("PASS requires a canonical GitHub Actions run_url")
    observed_at = record.get("observed_at")
    try:
        timestamp = datetime.fromisoformat(observed_at.replace("Z", "+00:00"))
    except (ValueError, TypeError, AttributeError):
        raise ValueError("PASS requires ISO-8601 observed_at") from None
    if timestamp.tzinfo is None:
        raise ValueError("observed_at must have timezone")

    if scope == "native-installed":
        for key in ("package_sha256", "executable_sha256"):
            if not SHA64.fullmatch(str(record.get(key, ""))):
                raise ValueError(key + " required for installed app")
        hw = record.get("hardware")
        if (not isinstance(hw, dict) or not all(isinstance(hw.get(k), str) and hw[k].strip()
                                              for k in ("os", "cpu"))
                or not _positive_number(hw.get("ram_bytes"))):
            raise ValueError("native hardware OS/CPU/RAM required")
        if record.get("native_ui_probe") != "installed-webview-automation":
            raise ValueError("native UI verification required; not a browser mock or operator file")
        if not isinstance(record.get("method"), str) or len(record["method"].strip()) < 15:
            raise ValueError("native method must explain observed package/launch protocol")
        values = record.get("measurements")
        if not isinstance(values, dict) or not NATIVE_METRICS.issubset(values):
            raise ValueError("native measurement fields missing")
        for key in NATIVE_METRICS:
            if not _positive_number(values[key]):
                raise ValueError("invalid native measurement: " + key)
    if scope == "browser-mock" and record.get("mock_only") is not True:
        raise ValueError("browser results must be marked mock_only")
    return "PASS"
