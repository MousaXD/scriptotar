#!/usr/bin/env python3
"""Independent Agent10 adversarial source-only test for PR179 JSON evidence.

The original PR179 code is checked out and authenticated as DATA before this
test imports the minimal stdlib-only JSON parser. All fixtures are fabricated
and do not authenticate PID, application, media, CPU, or native performance.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
from pathlib import Path
import sys
import tempfile


PR179_SHA = "c5a5e74819718a87e7d40ac2d59480c9165502e0"
GATE_BLOB = "94aaa7a7ca7e6360a8a568966de6d7cc23ce72ec"


def load_candidate(directory: Path):
    module_path = directory / "attached_memory_gate.py"
    assert module_path.is_file(), "Original pinned attached-memory parser unavailable"
    sys.path.insert(0, str(directory))
    spec = importlib.util.spec_from_file_location("agent10_reviewed_original_memory_gate", module_path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", type=Path, default=Path(".agent10-pr179-data"))
    args = parser.parse_args()
    directory = (args.target.resolve(strict=True) /
                 "tests/defragmentation/benchmarks")
    module = load_candidate(directory)
    with tempfile.TemporaryDirectory(prefix="agent10-memory-json-") as temp:
        path = Path(temp) / "synthetic-untrusted-attached.json"
        fixture = {
            "scope": "SYNTHETIC_NOT_NATIVE_EVIDENCE",
            "certification": "NOT_CERTIFIED",
            "source_sha_operator_supplied": "a" * 40,
            "raw_samples": [
                {"provider": "synthetic-only", "elapsed_ms": i * 10.0}
                for i in range(3)
            ],
        }
        original = json.dumps(fixture, separators=(",", ":"))
        path.write_text(original, encoding="utf-8")
        parsed = module.load_attached_report(path)
        assert parsed == fixture, "Normal valid bounded JSON must parse without becoming certified"
        print("PASS: benign three-sample synthetic JSON parsed; no native installation or certification", flush=True)

        attacks = {}
        for first, second in (("f" * 40, "a" * 40), ("a" * 40, "f" * 40)):
            label = ("first-hostile" if first.startswith("f") else "last-hostile")
            attacks[f"duplicate-source-{label}"] = (
                original[:-1] + ',"source_sha_operator_supplied":' +
                json.dumps(first) + ',"source_sha_operator_supplied":' +
                json.dumps(second) + "}"
            ), "duplicate JSON member"
        attacks["duplicate-identical-source"] = (
            original[:-1] + ',"source_sha_operator_supplied":' +
            json.dumps("a" * 40) + "}"
        ), "duplicate JSON member"
        escaped_name = '"source_\\u0073ha_operator_supplied"'
        attacks["unicode-escaped-equivalent-member"] = (
            original[:-1] + "," + escaped_name + ":" + json.dumps("a" * 40) + "}"
        ), "duplicate JSON member"
        attacks["nested-provider-duplicate"] = (
            '{"raw_samples":[{"provider":"original","provider":"spoofed"}]}'
        ), "duplicate JSON member"
        attacks["nested-identical-duplicate"] = (
            '{"raw_samples":[{"provider":"same","provider":"same"}]}'
        ), "duplicate JSON member"
        for forbidden in ("NaN", "Infinity", "-Infinity"):
            attacks[f"nonfinite-{forbidden}"] = (
                original[:-1] + ',"unreviewed_extra":' + forbidden + "}"
            ), "nonfinite JSON constant"
        for forbidden in ("1e9999", "-1e9999"):
            attacks[f"overflow-{forbidden}"] = (
                original[:-1] + ',"unreviewed_extra":' + forbidden + "}"
            ), "nonfinite JSON number"
        attacks["too-many-samples"] = (
            '{"raw_samples":[' + ",".join("{}" for _ in range(2001)) + ']}'
        ), "bounded sample count"
        attacks["not-an-object"] = ('[{},{}]', "must be a JSON object")
        # A depth of 1,400 is legal on current 3.12/3.13 accelerated decoders;
        # use 12,000 to exceed the actual built-in JSON decoder recursion guard.
        attacks["deep-recursion"] = (
            '{"raw_samples":' + "[" * 12000 + "0" + "]" * 12000 + "}"
        ), "invalid or deeply nested"
        attacks["malformed-json"] = ('{"raw_samples":[}', "invalid or deeply nested")

        passed = 0
        for label, (payload, required_error) in attacks.items():
            if "duplicate-source" in label:
                # The Python baseline parser silently keeps only the last
                # conflicting raw value; validate that negative fixture.
                assert json.loads(payload)["source_sha_operator_supplied"] == (
                    "a" * 40 if label.endswith("first-hostile") else "f" * 40
                )
            path.write_text(payload, encoding="utf-8")
            try:
                module.load_attached_report(path)
            except ValueError as exc:
                if required_error not in str(exc):
                    raise AssertionError(f"{label}: rejected for wrong reason: {exc}") from exc
                print(f"PASS: {label} refused for {required_error}", flush=True)
                passed += 1
            else:
                raise AssertionError(f"RELEASE VETO: {label} accepted into native memory evidence parser")

        path.write_bytes(b"\xff")
        try:
            module.load_attached_report(path)
        except ValueError as exc:
            assert "must be UTF-8" in str(exc), exc
            passed += 1
            print("PASS: invalid UTF-8 refused", flush=True)
        else:
            raise AssertionError("RELEASE VETO: invalid UTF-8 accepted")

        path.write_bytes(b" " * (module.MAX_ATTACHED_REPORT_BYTES + 1))
        try:
            module.load_attached_report(path)
        except ValueError as exc:
            assert "bounded byte limit" in str(exc), exc
            passed += 1
            print("PASS: >16MiB input refused before decoding", flush=True)
        else:
            raise AssertionError("RELEASE VETO: oversized input accepted")

        # A complete, bounded 2000-element sample list is a *format* positive,
        # never sufficient for preflight or installed performance acceptance.
        path.write_text(
            '{"raw_samples":[' + ",".join("{}" for _ in range(module.MAX_ATTACHED_SAMPLES)) + ']}',
            encoding="utf-8",
        )
        bounded = module.load_attached_report(path)
        assert len(bounded["raw_samples"]) == module.MAX_ATTACHED_SAMPLES
        print("PASS: exact 2000 row cap accepted as unverified JSON FORMAT only", flush=True)
        print(f"PASS: {passed}/{passed} hostile JSON source-format controls failed closed", flush=True)


if __name__ == "__main__":
    main()
