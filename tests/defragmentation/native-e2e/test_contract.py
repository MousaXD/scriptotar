"""Contract tests for the evidence collector, NOT native GUI certification."""
import hashlib
from pathlib import Path
import tempfile
import unittest

import linux_webkit as native


class NativeEvidenceContractTests(unittest.TestCase):
    def test_catalog_covers_release_blockers(self):
        for name in (
            "installed_webview", "create_switch_projects", "create_study",
            "edit_save_duplicate_candidate", "quit_relaunch_persistence",
            "wrong_project_ipc_denial", "multi_source_transcript",
            "source_revocation", "keyboard_custom_controls", "arabic_rtl",
            "light_dark_visual", "windows_webview2",
        ):
            self.assertIn(name, native.CASES)

    def test_pass_requires_known_scenario_and_state(self):
        record = {"cases": {}}
        native.status(record, "installed_webview", "PASS", "real AT-SPI observation")
        self.assertEqual(record["cases"]["installed_webview"]["status"], "PASS")
        with self.assertRaises(AssertionError):
            native.status(record, "invented-scenario", "PASS", "forged")
        with self.assertRaises(AssertionError):
            native.status(record, "installed_webview", "SKIPPED_BUT_PASS", "forged")

    def test_sha256_uses_actual_file_bytes(self):
        with tempfile.TemporaryDirectory() as tmp:
            source = Path(tmp) / "package.deb"
            source.write_bytes(b"package-bytes")
            self.assertEqual(native.sha256(source),
                             hashlib.sha256(b"package-bytes").hexdigest())

    def test_accessibility_probe_is_required_not_optional(self):
        # Prevent a future edit from silently replacing WebView automation with
        # a process-startup or screenshot-only check.
        import inspect
        body = inspect.getsource(native.Session.start)
        self.assertIn("web_document()", body)
        self.assertIn("NativeUnavailable", body)
        runner = inspect.getsource(native.main)
        self.assertIn('return 0 if report["verdict"] == "PASS" else 1', runner)


if __name__ == "__main__":
    unittest.main()
