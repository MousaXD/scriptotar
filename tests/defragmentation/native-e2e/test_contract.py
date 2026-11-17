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

    def test_windows_unavailable_clickable_point_requires_real_native_pointer(self):
        # Source-only regression contract: a UIA GetClickablePoint exception
        # cannot mean success, an unchecked click, or a skipped native event.
        # Fresh installed WebView2 evidence is still mandatory.
        script = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        click = script.split("function Click-InstalledControl(", 1)[1].split(
            "function Type-InstalledEdit(", 1
        )[0]
        self.assertIn("FindAll([System.Windows.Automation.TreeScope]::Subtree", click)
        self.assertIn("$candidate.Current.IsOffscreen", click)
        self.assertIn("$candidate.Current.IsEnabled", click)
        self.assertIn("$ownerRect = $owner.Current.BoundingRectangle", click)
        self.assertIn("$element.GetClickablePoint()", click)
        self.assertIn("} catch {", click)
        self.assertIn("[NativeWebViewWindow]::NativeClick", click)
        self.assertIn('throw "No visible, enabled, bounded UIA control', click)
        self.assertNotIn("Stop-Process", click)
        self.assertNotIn("return # skip", click)

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

    def test_windows_uia_failure_evidence_records_actual_focus_and_patterns(self):
        # UIA diagnostics must distinguish a non-focusable List from a
        # supporting Edit/ComboBox. Pattern enumeration is evidence only.
        source = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("$info.keyboard_focusable = [bool]$element.Current.IsKeyboardFocusable", source)
        self.assertIn("$info.has_keyboard_focus = [bool]$element.Current.HasKeyboardFocus", source)
        self.assertIn("$info.supported_patterns = @($element.GetSupportedPatterns()", source)
        self.assertIn("$info.bounds = @{", source)
        self.assertIn("$probe.child_webview_provider_nodes += $info", source)

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

    def test_windows_arabic_persisted_relaunch_and_english_restore_fail_closed(self):
        # Native UIA evidence, not a simulated DOM locale toggle. Studio
        # main-content heading must follow locale, not just the topbar label.
        source = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("function Find-InstalledDocumentAndStudio([object]$process, [string]$studioName='Studio')", source)
        self.assertIn("function Wait-InstalledStudioPageText(", source)
        self.assertIn("if ($cases.theme_mode_control.status -eq 'PASS')", source)
        self.assertIn("Choose-InstalledOption $themeHwnd 'Interface language' 'ع'", source)
        self.assertIn("Wait-InstalledStudioPageText $themeHwnd 'الاستوديو'", source)
        self.assertIn("Find-InstalledDocumentAndStudio $proc 'الاستوديو'", source)
        self.assertIn("Wait-InstalledStudioPageText $arabicHwnd 'الاستوديو'", source)
        self.assertIn("Choose-InstalledOption $arabicHwnd 'لغة الواجهة' 'EN'", source)
        self.assertIn("Wait-InstalledStudioPageText $arabicHwnd 'Studio'", source)
        self.assertIn("Arabic Studio text still visibly mounted after English restoration", source)
        self.assertIn("$cases.arabic_locale_control = @{", source)
        self.assertIn("windows-arabic-reopened.png", source)
        self.assertIn("windows-english-restored.png", source)

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
        self.assertIn('a.choose(a.global_locale_control(), "EN")', source)
        self.assertIn('a.choose(a.global_locale_control(), "ع")', source)
        self.assertNotIn('a.choose("Interface language", "EN")', source)
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

    def test_webkit_waits_for_canonical_immutable_save_without_writing_db(self):
        # Native screenshot in run #44150090373 showed the actual draft text
        # while Save changes was still "Saving..."; version 1 was still empty.
        # A bounded read-only wait must retry and never downgrade DB proof.
        expected = "Canonical SQLite stores exact GUI draft body and immutable version 2"
        with mock.patch.object(
            native, "verify_canonical_saved_draft",
            side_effect=[native.NativeFailure("revision=1, persisted_len=0"), expected],
        ) as verify, mock.patch.object(native.time, "sleep") as sleeper:
            actual = native.wait_for_canonical_saved_draft(
                Path("/unused"), "QA-A", "QA-Study", "Exact typed body"
            )
        self.assertEqual(actual, expected)
        self.assertEqual(verify.call_count, 2)
        sleeper.assert_called_once()

        with mock.patch.object(
            native, "verify_canonical_saved_draft",
            side_effect=native.NativeFailure("revision=1, persisted_len=0"),
        ), mock.patch.object(native.time, "monotonic",
                             side_effect=[0.0, 0.0, 2.0]), \
             mock.patch.object(native.time, "sleep"):
            with self.assertRaisesRegex(native.NativeFailure, "persisted_len=0"):
                native.wait_for_canonical_saved_draft(
                    Path("/unused"), "QA-A", "QA-Study", "Exact typed body",
                    timeout_seconds=1.0,
                )
        import inspect
        source = inspect.getsource(native.exercise)
        self.assertIn('ax.read_text("Script draft") != body', source)
        self.assertIn("wait_for_canonical_saved_draft(", source)

    def test_linux_sigsegv_forensics_keep_native_failure_and_no_db_writes(self):
        # An exit code -11 must preserve core/journal availability and remain
        # FAIL; coredumpctl is optional on GitHub's actual hosted runner.
        import inspect
        probe = inspect.getsource(native.native_crash_context)
        start = inspect.getsource(native.Session.start)
        main = inspect.getsource(native.main)
        self.assertIn("resource.RLIMIT_CORE", probe)
        module = (Path(__file__).resolve().parent / "linux_webkit.py").read_text()
        self.assertIn("except ImportError:", module)
        self.assertIn("resource = None", module)
        self.assertIn('shutil.which("coredumpctl")', probe)
        self.assertIn('["coredumpctl_tail"]', probe)
        self.assertIn('signal.Signals(-exit_code)', start)
        self.assertIn('native_crash_context(self.proc.pid)', start)
        self.assertIn('raise NativeFailure(', start)
        self.assertIn('root / "process-exit.json"', main)
        self.assertNotIn("UPDATE creative_drafts", probe)
        self.assertNotIn("INSERT INTO", probe)
        with mock.patch.object(native.shutil, "which", return_value=None):
            details = native.native_crash_context(1234)
        self.assertEqual(details["pid"], 1234)
        self.assertFalse(details["coredumpctl_available"])
        self.assertFalse(details["gdb_available"])

    def test_linux_graphical_quit_sigsegv_records_real_process_exit(self):
        import json
        import inspect

        class FakeProc:
            pid = 4321
            returncode = None

            def poll(self):
                return self.returncode

            def wait(self, timeout):
                self.returncode = -11
                return self.returncode

        def fake_xdotool(*arguments, **_kwargs):
            if arguments[1] == "search":
                return SimpleNamespace(stdout="24680\n", returncode=0)
            if arguments[1] == "getwindowname":
                return SimpleNamespace(stdout="Scriptotar\n", returncode=0)
            return SimpleNamespace(stdout="", returncode=0)

        with tempfile.TemporaryDirectory() as tmp:
            session = object.__new__(native.Session)
            session.proc = FakeProc()
            session.root = Path(tmp)
            with mock.patch.object(native, "command", side_effect=fake_xdotool), \
                 mock.patch.object(native, "native_crash_context",
                                   return_value={"pid": 4321}):
                with self.assertRaisesRegex(
                        native.NativeFailure, "graphical shutdown crashed"):
                    session.stop()
            self.assertIsNone(session.proc)
            proof = json.loads((Path(tmp) / "process-exit.json").read_text())
            self.assertEqual(proof["pid"], 4321)
            self.assertEqual(proof["returncode"], -11)
            self.assertEqual(proof["signal"], "SIGSEGV")
            self.assertEqual(
                proof["stage"], "installed native graphical Alt+F4 shutdown"
            )

        source = inspect.getsource(native.exercise)
        self.assertIn("original_error = exc", source)
        self.assertIn('"original_error"', source)
        self.assertIn('"restoration_error"', source)
        self.assertIn('"failed_phase"', source)
        self.assertIn('root / "arabic-transition.json"', source)
        main = inspect.getsource(native.main)
        self.assertIn('"native_arabic_transition_evidence"', main)
        self.assertIn('args.output / "arabic-transition.json"', main)

    def test_webkit_global_locale_selector_is_native_and_label_agnostic(self):
        # Native Arabic accessibility may expose localized project and language
        # names. Read actual EN/ع options in the top bar, close each popup
        # with Escape and never select project/script language by position alone.
        class Rect:
            def __init__(self, y):
                self.y, self.width, self.height = y, 90, 40

        class Node:
            def __init__(self, name, role, y):
                self.name, self.role, self.y = name, role, y

            def getRoleName(self):
                return self.role

            def queryComponent(self):
                return SimpleNamespace(getExtents=lambda _coord: Rect(self.y))

        project = Node("المشروع", "combo box", 77)
        locale = Node("لغة الواجهة", "combo box", 81)
        script_language = Node("لغة النص", "combo box", 620)
        ax = object.__new__(native.AxWebKit)
        ax.atspi = SimpleNamespace(DESKTOP_COORDS=0)
        ax.nodes = lambda: iter([project, locale, script_language])
        active = [None]
        clicked = []
        def native_click(label, role):
            self.assertEqual(role, "combo")
            active[0] = label
            clicked.append(label)
        ax.click = native_click
        ax.available = lambda name, role: (
            role == "list item" and name in ("EN", "ع")
            and active[0] == "لغة الواجهة"
        )
        with mock.patch.object(native, "command") as commands, \
             mock.patch.object(native.time, "sleep"):
            self.assertEqual(ax.global_locale_control(), "لغة الواجهة")
            self.assertEqual(clicked, ["المشروع", "لغة الواجهة"])
            self.assertEqual(commands.call_count, 2)
            ax.nodes = lambda: iter([project, script_language])
            with self.assertRaisesRegex(native.NativeFailure, "both EN/ع"):
                ax.global_locale_control()

        import inspect
        helper = inspect.getsource(native.AxWebKit.global_locale_control)
        source = inspect.getsource(native.exercise)
        self.assertNotIn('self.find("PROJECT"', helper)
        self.assertIn('self.available("EN", "list item")', helper)
        self.assertIn('self.available("ع", "list item")', helper)
        self.assertIn('a.choose(a.global_locale_control(), "ع")', source)
        self.assertIn('a.choose(a.global_locale_control(), "EN")', source)
        self.assertLess(source.index('def arabic():'), source.index('def theme():'))
        self.assertNotIn('a.choose("Interface language"', source)

    def test_windows_project_switch_requires_eventual_exact_native_draft(self):
        # Real installed integration #44176696340 already showed Project A,
        # the correct Study and Scripts 2, but immediate ValuePattern content
        # mismatched. Poll actual UIA text, not a DB-only/screenshot shortcut.
        source = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        waiter = source.split("function Wait-InstalledEditExact(", 1)[1].split(
            "function Verify-InstalledCanonicalDb(", 1
        )[0]
        isolation = source.split(
            "if ($cases.quit_relaunch_persistence.status -eq 'PASS')", 1
        )[1].split(
            "if ($cases.wrong_project_ui_isolation.status -eq 'PASS')", 1
        )[0]
        self.assertIn("Read-InstalledEdit $hwnd $name", waiter)
        self.assertIn("$actual -ceq $expected", waiter)
        self.assertIn("Start-Sleep -Milliseconds 350", waiter)
        self.assertIn("throw \"Installed UIA edit", waiter)
        self.assertNotIn("SetValue(", waiter)
        self.assertIn(
            "Click-InstalledControl $restoredHwnd 'Scripts 2' ([System.Windows.Automation.ControlType]::Hyperlink)",
            isolation,
        )
        self.assertIn(
            "Wait-InstalledEditExact $restoredHwnd 'Script draft' $draftBody",
            isolation,
        )
        self.assertIn(
            "Wait-InstalledCanonicalDb 'candidates' $first $studyTitle $draftBody $copyTitle",
            isolation,
        )

    def test_windows_offscreen_study_candidate_requires_native_scroll_and_recheck(self):
        # Actual restored screenshot at run #44150090373 was scrolled above
        # the script candidate editor. A real UIA ScrollItemPattern is allowed,
        # but no invisible-element click, direct DOM injection or skipped proof.
        source = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        click = source.split("function Click-InstalledControl(", 1)[1].split(
            "function Type-InstalledEdit(", 1)[0]
        self.assertIn("[System.Windows.Automation.ScrollItemPattern]::Pattern", click)
        self.assertIn("$scrollItem.ScrollIntoView()", click)
        # Native WebView2 can expose ScrollItemPattern while refusing to
        # move an offscreen element. Use the visible workflow link and still
        # require actual candidate selection plus canonical immutable proof.
        self.assertIn(
            "Click-InstalledControl $restoredHwnd 'Scripts 2' ([System.Windows.Automation.ControlType]::Hyperlink)",
            source,
        )
        self.assertIn("Wait-InstalledCanonicalDb 'candidates' $first $studyTitle $draftBody $copyTitle", source)
        self.assertIn("reopened-scripts-navigation.png", source)
        self.assertIn("$owner.FindAll([System.Windows.Automation.TreeScope]::Subtree, $condition)", click)
        self.assertIn("-not $candidate.Current.IsOffscreen", click)
        self.assertIn("No visible, enabled, bounded UIA control", click)
        self.assertNotIn("SetValue(", click)
        self.assertNotIn("return # skip", click)

    def test_document_alone_cannot_satisfy_startup(self):
        # Exercise Session.start's actual fail-closed gate, not an obsolete
        # English source-code literal. This is a mocked *contract* test only;
        # installed WebKitGTK acceptance is separately required in CI.
        def startup(document, controls, expected_studio="Studio"):
            observed = []
            ax = SimpleNamespace(
                web_document=lambda: document,
                available=lambda name, role: (
                    observed.append((name, role)) or (name, role) in controls
                ),
                diagnostics=lambda: {"desktop_apps": [], "x11_windows": []},
            )
            proc = SimpleNamespace(pid=34567, poll=lambda: None)
            with tempfile.TemporaryDirectory() as tmp:
                session = native.Session(
                    SimpleNamespace(executable="/synthetic/not-executed"), Path(tmp)
                )
                session.screenshot = lambda _name: None
                with mock.patch.object(native.subprocess, "Popen", return_value=proc), \
                     mock.patch.object(native, "AxWebKit", return_value=ax), \
                     mock.patch.object(native.time, "monotonic",
                                       side_effect=[0.0, 0.0, 36.0]), \
                     mock.patch.object(native.time, "sleep"):
                    try:
                        session.start(expected_studio=expected_studio)
                    except native.NativeUnavailable:
                        return False, observed
                    return True, observed

        # A document without the requested native rail button/link must fail.
        passed, calls = startup(True, set())
        self.assertFalse(passed)
        self.assertEqual(calls, [("Studio", "button"), ("Studio", "link")])

        # A matching control without a real WebKit document must also fail.
        passed, calls = startup(False, {("Studio", "button")})
        self.assertFalse(passed)
        self.assertEqual(calls, [])

        # The persisted Arabic locale must reject an English-only control.
        passed, calls = startup(True, {("Studio", "button")}, "الاستوديو")
        self.assertFalse(passed)
        self.assertEqual(calls, [("الاستوديو", "button"), ("الاستوديو", "link")])

        # Either accessible role is valid, but only with a real document.
        for language, role in (("Studio", "button"), ("الاستوديو", "link")):
            passed, calls = startup(True, {(language, role)}, language)
            self.assertTrue(passed)
            self.assertIn((language, role), calls)



if __name__ == "__main__":
    unittest.main()
