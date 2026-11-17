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
            "source_revocation", "keyboard_custom_controls", "arabic_locale_control",
            "theme_mode_control", "arabic_rtl",
            "light_dark_visual",
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


    def test_web_document_rejects_shell_only_tree(self):
        class FakeNode:
            def __init__(self, role):
                self.role = role

            def getRoleName(self):
                return self.role

        ax = object.__new__(native.AxWebKit)
        ax.nodes = lambda: iter([FakeNode("application"), FakeNode("frame"),
                                 FakeNode("scroll pane")])
        self.assertFalse(ax.web_document(), "A native shell is not a web document")
        ax.nodes = lambda: iter([FakeNode("scroll pane"), FakeNode("document web")])
        self.assertTrue(ax.web_document())

    def test_windows_production_build_preserves_semantic_cargo_lock(self):
        # Workflow contract only: does not establish installed Windows UI PASS.
        workflow = (Path(__file__).resolve().parents[3] /
                    ".github/workflows/installed-webview-acceptance.yml").read_text()
        self.assertIn("tauri build --bundles nsis", workflow)
        self.assertNotIn("tauri build --bundles nsis -- --locked", workflow)
        self.assertIn("tomllib.loads", workflow)
        self.assertIn("$lockBefore -ne $lockAfter", workflow)
        self.assertIn("cargo metadata --locked", workflow)

    def test_webkit_option_uses_observed_list_item_role(self):
        # Contract only: actual installed GUI interaction is a separate gate.
        calls = []
        ax = object.__new__(native.AxWebKit)
        ax.click = lambda name, role=None: calls.append((name, role))
        ax.choose("PROJECT", "NativeQA-sample-A")
        self.assertEqual(calls, [
            ("PROJECT", "combo"), ("NativeQA-sample-A", "list item")
        ])

    def test_canonical_project_probe_reads_real_sqlite_without_writing(self):
        import sqlite3
        with tempfile.TemporaryDirectory() as tmp:
            data = Path(tmp)
            db = data / "scriptotar.sqlite3"
            with sqlite3.connect(db) as con:
                con.execute("CREATE TABLE projects(id TEXT PRIMARY KEY, name TEXT, archived_at TEXT)")
                con.executemany("INSERT INTO projects VALUES (?, ?, NULL)", [
                    ("11111111-1111-4111-8111-111111111111", "QA-A"),
                    ("22222222-2222-4222-8222-222222222222", "QA-B")
                ])
            self.assertIn("Two distinct", native.verify_canonical_projects(data, ("QA-A", "QA-B")))
            with self.assertRaises(native.NativeFailure):
                native.verify_canonical_projects(data, ("QA-A", "Missing"))

    def test_webkit_heading_requires_semantic_role_not_arbitrary_button(self):
        # Exact installed QA AT-SPI tree maps dashboard h1 to document frame.
        class FakeNode:
            def __init__(self, name, role):
                self.name, self.role = name, role

            def getRoleName(self):
                return self.role

        ax = object.__new__(native.AxWebKit)
        ax.nodes = lambda: iter([
            FakeNode("NativeQA-abc-A", "push button"),
            FakeNode("NativeQA-abc-A", "document frame"),
        ])
        ax.heading("NativeQA-abc-A", timeout=0.3)
        ax.nodes = lambda: iter([FakeNode("NativeQA-abc-A", "push button")])
        with self.assertRaises(native.NativeFailure):
            ax.heading("NativeQA-abc-A", timeout=0.01)

    def test_document_alone_cannot_satisfy_startup(self):
        import inspect
        source = inspect.getsource(native.Session.start)
        self.assertIn('web_document()', source)
        self.assertIn('available("Studio", "button")', source)
        self.assertIn('available("Studio", "link")', source)



if __name__ == "__main__":
    unittest.main()
