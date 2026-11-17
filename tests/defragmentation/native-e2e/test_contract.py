"""Contract tests for the evidence collector, NOT native GUI certification."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest import mock
from types import SimpleNamespace

import cargo_lock_fingerprint as lockproof
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

    def test_cargo_fingerprint_ignores_only_dependency_order(self):
        with tempfile.TemporaryDirectory() as tmp:
            lock = Path(tmp) / "Cargo.lock"
            lock.write_text(
                'version = 4\n[[package]]\nname = "scriptotar-desktop"\n'
                'version = "1.4.1"\ndependencies = ["scriptotar-agent", "keyring"]\n'
            )
            first = lockproof.semantic_sha256(lock)
            lock.write_text(
                'version = 4\n[[package]]\nname = "scriptotar-desktop"\n'
                'version = "1.4.1"\ndependencies = ["keyring", "scriptotar-agent"]\n'
            )
            self.assertEqual(first, lockproof.semantic_sha256(lock))
            lock.write_text(
                'version = 4\n[[package]]\nname = "scriptotar-desktop"\n'
                'version = "1.4.1"\ndependencies = ["keyring", "other-package"]\n'
            )
            self.assertNotEqual(first, lockproof.semantic_sha256(lock))

    def test_windows_production_build_preserves_semantic_cargo_lock(self):
        # Workflow contract only: does not establish installed Windows UI PASS.
        workflow = (Path(__file__).resolve().parents[3] /
                    ".github/workflows/installed-webview-acceptance.yml").read_text()
        self.assertIn("tauri build --bundles nsis", workflow)
        self.assertNotIn("tauri build --bundles nsis -- --locked", workflow)
        self.assertIn("cargo_lock_fingerprint.py", workflow)
        self.assertIn("$lockBefore -ne $lockAfter", workflow)
        self.assertIn("cargo metadata --locked", workflow)

    def test_webkit_option_uses_exact_native_component_bounds(self):
        # Contract only: installed GUI result is independently checked.
        calls = []
        bounds = SimpleNamespace(x=100, y=250, width=120, height=36)
        node = SimpleNamespace(
            queryComponent=lambda: SimpleNamespace(getExtents=lambda _kind: bounds)
        )
        ax = object.__new__(native.AxWebKit)
        ax.atspi = SimpleNamespace(DESKTOP_COORDS=0)
        ax.click = lambda name, role=None: calls.append(("control", name, role))
        ax.find = lambda name, role=None, timeout=12: (
            calls.append(("option", name, role)) or node
        )
        with mock.patch.object(native, "command",
                               side_effect=lambda *cmd: calls.append(tuple(cmd))):
            with mock.patch.object(native.time, "sleep"):
                ax.choose("PROJECT", "NativeQA-sample-A")
        self.assertEqual(calls[:2], [
            ("control", "PROJECT", "combo"),
            ("option", "NativeQA-sample-A", "list item")
        ])
        self.assertIn(
            ("xdotool", "mousemove", "160", "268"), calls
        )
        self.assertIn(
            ("xdotool", "click", "--clearmodifiers", "1"), calls
        )

    def test_webkit_popup_mapped_as_comment_requires_real_option_and_escape(self):
        # Run #44114536688 exposed the genuine opened listbox as an AT-SPI
        # comment with two list item children, NOT as role='list'.
        # Require both the popup and its exact named option, then require
        # their disappearance after a native keyboard Escape.
        ax = object.__new__(native.AxWebKit)
        events = []
        ax.click = lambda name, role: events.append(("click", name, role))
        ax.find = lambda name, role, timeout=12: (
            events.append(("find", name, role)) or object()
        )
        ax.available = lambda name, role=None: False
        with mock.patch.object(native, "command", side_effect=lambda *args: events.append(args)):
            with mock.patch.object(native.time, "sleep"):
                ax.escape_custom_select("Script candidates", "QA-copy")
        self.assertIn(("find", "Script candidates", "comment"), events)
        self.assertIn(("find", "QA-copy", "list item"), events)
        self.assertIn(("xdotool", "key", "--clearmodifiers", "Escape"), events)
        ax.available = lambda name, role=None: role == "comment"
        with mock.patch.object(native, "command"):
            with mock.patch.object(native.time, "sleep"):
                with self.assertRaises(native.NativeFailure):
                    ax.escape_custom_select("Script candidates", "QA-copy")

    def test_webkit_pointer_does_not_use_stalling_mousemove_sync(self):
        # A real installed restart left the correct project already active,
        # while xdotool mousemove --sync hung for 15s (run #44107946419).
        # Still require a native pointer action + post-click heading assertion.
        import inspect
        source = inspect.getsource(native.AxWebKit.choose)
        self.assertIn('"xdotool", "mousemove", str(x), str(y)', source)
        self.assertNotIn('"mousemove", "--sync"', source)

    def test_windows_popup_focus_uses_actual_option_owner_not_combobox_name(self):
        # Real installed UIA in #44114536688: ComboBox 'PROJECT',
        # but popup List 'Project'. An exact-name popup lookup incorrectly
        # failed despite existing project B and all three real ListItems.
        script = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        choose = script.split("function Choose-InstalledOption(", 1)[1].split(
            "function Wait-InstalledComboValue(", 1
        )[0]
        self.assertIn("TreeWalker]::ControlViewWalker.GetParent($ancestor)", choose)
        self.assertIn("ControlType]::List", choose)
        self.assertIn("[NativeWebViewWindow]::NativeClick", choose)
        self.assertIn("Wait-InstalledPopupClosed $hwnd $name", choose)
        self.assertNotIn("$list.SetFocus()", choose)
        self.assertNotIn("Find-ExactInstalledControl $hwnd $label ([System.Windows.Automation.ControlType]::List)", choose)

    def test_windows_candidate_switch_requires_rendered_selected_label(self):
        # A candidate copy has identical body text. A pass requires the
        # selected label inside the combobox plus the closed listbox, rather
        # than accidentally counting a visible popup option as selected.
        script = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("function Wait-InstalledComboValue", script)
        self.assertIn("ControlType]::Text", script)
        self.assertIn("$cx -ge $bounds.X", script)
        self.assertIn("$cy -ge $bounds.Y", script)
        self.assertIn("if ($selected -and -not $stillOpen) { return }", script)
        self.assertIn("Wait-InstalledComboValue $providerHwnd 'Script candidates' $copyTitle", script)
        self.assertIn("Wait-InstalledComboValue $restoredHwnd 'Script candidates' $copyTitle", script)
        self.assertIn("Wait-InstalledPopupClosed $hwnd $name", script)
        self.assertIn("$stillOpen = Test-InstalledPopupOptionVisible $hwnd $expected", script)
        self.assertIn("TreeWalker]::ControlViewWalker.GetParent($ancestor)", script)
        self.assertIn("ControlType]::List", script)
        # The actual popup List may be named 'Project' even when its ComboBox
        # is 'PROJECT'; checking $label as a List name would silently accept
        # a still-open candidate popup and give a false selection PASS.
        self.assertNotIn("NameProperty, $label),\n        [System.Windows.Automation.PropertyCondition]::new(\n          [System.Windows.Automation.AutomationElement]::ControlTypeProperty,\n          [System.Windows.Automation.ControlType]::List)", script)
        self.assertNotIn("$list.SetFocus()", script)

    def test_windows_post_restart_negative_ui_and_escape_are_gated(self):
        # These are real installed WebView2 user interactions, not mocks
        # or a bypass for project/draft persistence failure.
        script = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("if ($cases.quit_relaunch_persistence.status -eq 'PASS')", script)
        self.assertIn("if ($cases.wrong_project_ui_isolation.status -eq 'PASS')", script)
        self.assertIn("Choose-InstalledProject $restoredHwnd $second", script)
        self.assertIn("Foreign Study '$studyTitle' visible within project B's native Studio", script)
        self.assertIn("Wait-InstalledCanonicalDb 'candidates' $first $studyTitle $draftBody $copyTitle", script)
        self.assertIn("Test-InstalledPopupOptionVisible $restoredHwnd $studyTitle", script)
        self.assertIn("[System.Windows.Forms.SendKeys]::SendWait('{ESC}')", script)
        self.assertIn("Wait-InstalledPopupClosed $restoredHwnd $studyTitle", script)
        self.assertIn("Wait-InstalledComboValue $restoredHwnd 'Script candidates' $copyTitle", script)
        self.assertNotIn("$list.SetFocus()", script)

    def test_windows_theme_uses_real_restart_and_exact_selected_value(self):
        # Feature-level source contract; only a fresh installed NSIS/UIA run
        # can certify the actual interactions and screenshot.
        script = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("if ($cases.keyboard_custom_controls.status -eq 'PASS')", script)
        self.assertIn("foreach ($choice in @('System', 'Dark'))", script)
        self.assertIn("Choose-InstalledOption $restoredHwnd 'Theme' $choice", script)
        self.assertIn("Wait-InstalledComboValue $restoredHwnd 'Theme' $choice", script)
        self.assertIn("if ($saveTheme.Current.IsEnabled)", script)
        self.assertIn("[System.Windows.Forms.SendKeys]::SendWait('%{F4}')", script)
        self.assertIn("$themeProbe = Find-InstalledDocumentAndStudio $proc", script)
        self.assertIn("Wait-InstalledComboValue $themeHwnd 'Theme' 'Dark'", script)
        self.assertIn("windows-theme-dark-reopened.png", script)
        self.assertIn("$cases.theme_mode_control = @{", script)

    def test_webkit_theme_selected_state_is_required_not_click_only(self):
        # The real Settings selector uses aria-selected; unsupported or
        # unselected AT-SPI state must never be reported as theme persistence.
        ax = object.__new__(native.AxWebKit)
        ax.atspi = SimpleNamespace(STATE_SELECTED=42)
        actions = []
        ax.click = lambda name, role: actions.append(("click", name, role))
        ax.available = lambda *_args: False
        option = SimpleNamespace(getState=lambda: SimpleNamespace(
            contains=lambda state: state == 42
        ))
        ax.find = lambda *_args, **_kwargs: option
        with mock.patch.object(native, "command", side_effect=lambda *args: actions.append(args)), \
             mock.patch.object(native.time, "sleep"):
            ax.assert_selected_option("Theme", "Dark")
        self.assertIn(("click", "Theme", "combo"), actions)
        self.assertIn(("xdotool", "key", "--clearmodifiers", "Escape"), actions)
        ax.find = lambda *_args, **_kwargs: SimpleNamespace(
            getState=lambda: SimpleNamespace(contains=lambda _state: False)
        )
        with mock.patch.object(native, "command", return_value=None), \
             mock.patch.object(native.time, "sleep"):
            with self.assertRaises(native.NativeFailure):
                ax.assert_selected_option("Theme", "Dark")

    def test_webkit_arabic_restart_is_native_and_english_restore_fail_closed(self):
        # The installed Arabic screenshot in #44118342637 shows real RTL,
        # and the later English shell still retained an Arabic Studio heading.
        # Never accept the language switch until both regions restore.
        source = (Path(__file__).resolve().parent / "linux_webkit.py").read_text()
        self.assertIn('session.start(expected_studio="الاستوديو")', source)
        self.assertIn('a.heading("الاستوديو")', source)
        self.assertIn('a.heading("Studio", timeout=12)', source)
        self.assertIn('"Arabic Studio heading remains mounted after restoring English"', source)
        self.assertIn('a.choose("Interface language", "EN")', source)
        self.assertIn('session.screenshot("arabic-reopened")', source)

    def test_webkit_option_rejects_zero_size_bounds(self):
        ax = object.__new__(native.AxWebKit)
        ax.atspi = SimpleNamespace(DESKTOP_COORDS=0)
        ax.click = lambda *args: None
        ax.find = lambda *args, **kwargs: SimpleNamespace(
            queryComponent=lambda: SimpleNamespace(
                getExtents=lambda _kind: SimpleNamespace(
                    x=0, y=0, width=0, height=40
                )
            )
        )
        with self.assertRaises(native.NativeFailure):
            ax.choose("PROJECT", "NativeQA-sample-A")

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

    def test_webkit_labeled_embedded_input_includes_placeholder(self):
        class FakeNode:
            def __init__(self, name, role):
                self.name, self.role = name, role
            def getRoleName(self):
                return self.role
        wanted = FakeNode("Study name e.g. Launch video", "embedded")
        ax = object.__new__(native.AxWebKit)
        ax.nodes = lambda: iter([
            FakeNode("Study name", "label"),
            FakeNode("Study name not-a-field", "push button"),
            wanted,
        ])
        self.assertIs(ax.edit_field("Study name", timeout=0.2), wanted)
        ax.nodes = lambda: iter([FakeNode("Study name", "label")])
        with self.assertRaises(native.NativeFailure):
            ax.edit_field("Study name", timeout=0.01)


    def test_real_canonical_saved_draft_body_and_version_are_required(self):
        import sqlite3
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "scriptotar.sqlite3"
            with sqlite3.connect(db) as con:
                con.executescript("""
                    CREATE TABLE projects(id TEXT PRIMARY KEY, name TEXT, archived_at TEXT);
                    CREATE TABLE content_studies(
                        id TEXT PRIMARY KEY, project_id TEXT, title TEXT, archived_at TEXT);
                    CREATE TABLE creative_drafts(
                        id TEXT PRIMARY KEY, project_id TEXT, study_id TEXT,
                        revision INTEGER, body TEXT, archived_at TEXT);
                    CREATE TABLE draft_versions(
                        id TEXT PRIMARY KEY, draft_id TEXT, version INTEGER, body TEXT);
                """)
                con.execute("INSERT INTO projects VALUES ('project-A','QA-A',NULL)")
                con.execute("INSERT INTO content_studies VALUES ('study-A','project-A','QA-Study',NULL)")
                con.execute(
                    "INSERT INTO creative_drafts VALUES (?,?,?,?,?,NULL)",
                    ("11111111-1111-4111-8111-111111111111", "project-A", "study-A",
                     2, "Original GUI saved body"),
                )
                con.execute(
                    "INSERT INTO draft_versions VALUES (?,?,?,?)",
                    ("v1", "11111111-1111-4111-8111-111111111111", 1, ""),
                )
                con.execute(
                    "INSERT INTO draft_versions VALUES (?,?,?,?)",
                    ("v2", "11111111-1111-4111-8111-111111111111",
                     2, "Original GUI saved body"),
                )
            detail = native.verify_canonical_saved_draft(
                Path(tmp), "QA-A", "QA-Study", "Original GUI saved body"
            )
            self.assertIn("immutable version 2", detail)
            with self.assertRaises(native.NativeFailure):
                native.verify_canonical_saved_draft(
                    Path(tmp), "QA-A", "QA-Study", "Unsaved different body"
                )
            with self.assertRaises(native.NativeFailure):
                native.verify_canonical_saved_draft(
                    Path(tmp), "Foreign", "QA-Study", "Original GUI saved body"
                )
            with sqlite3.connect(db) as con:
                con.execute(
                    "UPDATE draft_versions SET body='Different version' WHERE id='v2'"
                )
            with self.assertRaises(native.NativeFailure):
                native.verify_canonical_saved_draft(
                    Path(tmp), "QA-A", "QA-Study", "Original GUI saved body"
                )


    def test_two_restored_candidates_preserve_distinct_ids_and_immutable_bodies(self):
        import sqlite3
        with tempfile.TemporaryDirectory() as tmp:
            db = Path(tmp) / "scriptotar.sqlite3"
            original = "11111111-1111-4111-8111-111111111111"
            copied = "22222222-2222-4222-8222-222222222222"
            with sqlite3.connect(db) as con:
                con.executescript("""
                    CREATE TABLE projects(id TEXT PRIMARY KEY, name TEXT, archived_at TEXT);
                    CREATE TABLE content_studies(
                        id TEXT PRIMARY KEY, project_id TEXT, title TEXT, archived_at TEXT);
                    CREATE TABLE creative_drafts(
                        id TEXT PRIMARY KEY, project_id TEXT, study_id TEXT,
                        title TEXT, body TEXT, revision INTEGER, archived_at TEXT);
                    CREATE TABLE draft_versions(
                        id TEXT PRIMARY KEY, draft_id TEXT, version INTEGER, body TEXT);
                """)
                con.execute("INSERT INTO projects VALUES ('project-A','QA-A',NULL)")
                con.execute(
                    "INSERT INTO content_studies VALUES ('study-A','project-A','QA-Study',NULL)"
                )
                for draft_id, title, revision in (
                    (original, "QA-Study", 2), (copied, "QA-copy", 1)
                ):
                    con.execute(
                        "INSERT INTO creative_drafts VALUES (?,?,?,?,?,?,NULL)",
                        (draft_id, "project-A", "study-A",
                         title, "Saved body", revision)
                    )
                    con.execute(
                        "INSERT INTO draft_versions VALUES (?,?,?,?)",
                        (title, draft_id, revision, "Saved body"),
                    )
            detail = native.verify_canonical_reopened_candidates(
                Path(tmp), "QA-A", "QA-Study", "Saved body", "QA-copy"
            )
            self.assertIn("Two distinct", detail)
            with self.assertRaises(native.NativeFailure):
                native.verify_canonical_reopened_candidates(
                    Path(tmp), "QA-A", "QA-Study", "Saved body", "Wrong-copy"
                )
            with sqlite3.connect(db) as con:
                con.execute(
                    "UPDATE draft_versions SET body='Corrupted' WHERE draft_id=?",
                    (copied,),
                )
            with self.assertRaises(native.NativeFailure):
                native.verify_canonical_reopened_candidates(
                    Path(tmp), "QA-A", "QA-Study", "Saved body", "QA-copy"
                )

    def test_webkit_reads_editable_draft_not_same_named_heading(self):
        class FakeNode:
            def __init__(self, name, role, content=""):
                self.name, self.role, self.content = name, role, content
            def getRoleName(self):
                return self.role
            def queryText(self):
                return SimpleNamespace(getText=lambda _start, _end: self.content)

        heading = FakeNode("Script draft", "document frame", "heading only")
        editor = FakeNode("Script draft", "embedded", "Persistent script")
        ax = object.__new__(native.AxWebKit)
        ax.nodes = lambda: iter([heading, editor])
        self.assertEqual(ax.read_text("Script draft"), "Persistent script")

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
