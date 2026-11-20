#!/usr/bin/env python3
"""Agent10 independent exact-source preview vs final-release trust policy audit.

This script NEVER publishes, changes approval variables, calls GitHub APIs,
executes source-controlled shell code, or writes production source. It loads
the immutable main Python release-gate module only after its Git object is
authenticated separately by the calling workflow.

A historical Scriptotar Next 1.4.1 *preview* intentionally discloses incomplete
native/performance certification. This audit does not redefine that contract.
It proves the legacy preview gate CANNOT be reused to claim the stronger
Scriptotar defragmentation final-release native/security certification.
"""
from __future__ import annotations

import argparse
import importlib.util
import sys
from pathlib import Path

LEGACY_MAIN_SHA = "69e60f91c0b00560332dac06b732fc2259c1d632"
PUBLISHER_PR155_SHA = "1f9e661f3c72c28a5695d1c7dfa124280f5b1494"
EXPECTED_PREVIEW_GATES = frozenset({
    "integration.yml", "security-hygiene.yml", "windows-tauri.yml",
    "linux-tauri.yml", "mcp-e2e.yml",
})
FINAL_REQUIRED_NATIVE = {
    "installed-webview-acceptance.yml": (
        "Installed Linux WebKitGTK / AT-SPI (fail closed)",
        "Installed Windows WebView2 / UI Automation (fail closed)",
    )
}
FINAL_REQUIRED_SECURITY = ("independent-security-regressions.yml",)

OWNER_VARS = (
    "vars.SCRIPTOTAR_NEXT_DISTRIBUTION_APPROVED == 'true'",
    "vars.SCRIPTOTAR_NEXT_1_4_1_DISTRIBUTION_APPROVED == 'true'",
)
OWNER_SHORTCUT = "owner-approve-scriptotar-next-1.4.1-hotfix-publish-20261112"
SOURCE_SHA = "a" * 40


def load_pinned_gate(path: Path):
    spec = importlib.util.spec_from_file_location("agent10_preview_gate_source", path)
    assert spec and spec.loader
    mod = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


def condition_from_workflow(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    found = [s.strip().removeprefix("if: ").strip()
             for s in text.splitlines()
             if s.strip().startswith("if: github.event_name == 'workflow_run'")]
    assert len(found) == 1, f"Unexpected privileged publisher job count: {len(found)}"
    return found[0]


def synthetic_exact_sha_gate_test(module) -> None:
    class OfflineFakeClient:
        def __init__(self):
            self.queried = []

        def list_runs(self, workflow_file, source_sha):
            self.queried.append(workflow_file)
            # Deliberately DENY installed native results. If queried, it
            # would be absent; this never represents certified native.
            if workflow_file == "installed-webview-acceptance.yml":
                return []
            gate = next(g for g in module.GATES if g.workflow_file == workflow_file)
            idx = list(module.GATES).index(gate) + 1
            return [self._payload(gate, idx)]

        def view_run(self, run_id):
            gate = module.GATES[run_id - 1]
            return self._payload(gate, run_id)

        @staticmethod
        def _payload(gate, run_id):
            return {
                "databaseId": run_id,
                "headSha": SOURCE_SHA,
                "headBranch": "main",
                "event": "push",
                "status": "completed",
                "conclusion": "success",
                "createdAt": "2026-11-20T12:00:00Z",
                "workflowName": gate.workflow_name,
                "jobs": [
                    {"name": name, "status": "completed", "conclusion": "success"}
                    for name in gate.required_jobs
                ],
            }

    client = OfflineFakeClient()
    selected = module.prove_release_gates(
        client, SOURCE_SHA,
        discovery_attempts=1, discovery_sleep_seconds=0,
        wait_attempts=1, wait_sleep_seconds=0,
        sleep=lambda _: None,
    )
    assert len(selected) == len(module.GATES)
    assert "installed-webview-acceptance.yml" not in client.queried
    print("REPRODUCED: main preview gate accepts five synthetic exact-SHA green runs",
          "while installed Linux and Windows native acceptance is absent")
    print("SCOPED: this describes the *legacy preview* contract, not a claim",
          "of unauthorized publication or completed distribution")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--main", type=Path, default=Path(".agent10-pinned-main"))
    parser.add_argument("--proposed", type=Path, default=Path(".agent10-pinned-publisher-fix"))
    parser.add_argument("--strict-final-release", action="store_true")
    args = parser.parse_args()

    gate = load_pinned_gate(args.main / "scripts/release_ci_gate.py")
    workflows = frozenset(g.workflow_file for g in gate.GATES)
    assert workflows == EXPECTED_PREVIEW_GATES, ("Unexpected pinned preview gates", workflows)
    assert "installed-webview-acceptance.yml" not in workflows
    synthetic_exact_sha_gate_test(gate)

    original = condition_from_workflow(
        args.main / ".github/workflows/tauri-next-release.yml"
    )
    proposed = condition_from_workflow(
        args.proposed / ".github/workflows/tauri-next-release.yml"
    )
    assert all(v in original and v in proposed for v in OWNER_VARS)
    assert OWNER_SHORTCUT in original and "||" in original
    assert OWNER_SHORTCUT not in proposed and "||" not in proposed
    assert " && ".join(OWNER_VARS) in proposed
    assert "github.event.workflow_run.head_branch == 'main'" in proposed
    print("REPRODUCED: pinned main preview condition can authorize special",
          "commit text without both repository-owner approval variables")
    print("VERIFIED SOURCE: pinned PR155 removes special-message OR and",
          "requires both owner variables; it is not merged to main")

    missing = (set(FINAL_REQUIRED_NATIVE) | set(FINAL_REQUIRED_SECURITY)).difference(workflows)
    if args.strict_final_release:
        if missing or OWNER_SHORTCUT in original:
            print("FINAL RELEASE VETO: legacy main preview gate is NOT a",
                  "substitute for required installed 16/16 both-platform native",
                  "certification, independent security gate, and owner-only publisher authorization.",
                  "Missing native workflow gate:", ", ".join(sorted(missing)),
                  file=sys.stderr)
            return 1
        print("FINAL RELEASE POLICY: PASS (only if no missing gates)")
    else:
        print("PASS: both independent unsafe-baseline negative controls reproduced;",
              "stronger final-release policy remains deliberately uncertified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
