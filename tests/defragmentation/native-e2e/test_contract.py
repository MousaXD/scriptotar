"""Contract tests for the evidence collector, NOT native GUI certification."""
import hashlib
import sqlite3
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

    def test_offline_native_three_angle_recovery_is_real_and_denies_fake_success(self):
        # Source contract ONLY. Native PASS needs an actual installed WebView,
        # confirmed rejection, accepted replacement and canonical SQLite proof.
        import inspect
        source = inspect.getsource(native.exercise)
        self.assertIn('a.type_into("Paste three-angle JSON response"', source)
        self.assertIn('a.click("Validate 3 angles", "button")', source)
        self.assertIn('verify_canonical_reopened_candidates(', source)
        self.assertIn('if not checked(report, "error_recovery", error_recovery):', source)
        self.assertNotIn('status(report, "error_recovery", "PASS"', source)
        ps = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("function Paste-InstalledJson(", ps)
        self.assertIn("[System.Windows.Forms.SendKeys]::SendWait('^v')", ps)
        self.assertIn("function Wait-InstalledAngleRejection(", ps)
        self.assertIn("Wait-InstalledCanonicalDb 'candidates'", ps)
        self.assertIn("status='FAIL'\n          reason=\"Installed WebView2 three-angle", ps)
        self.assertNotIn("SetValue($value)", ps)


    def test_long_webkit_native_input_uses_bounded_real_key_chunks(self):
        # This is a transport contract, not an installed-app PASS. Prior run
        # #44343815684 lost 90 of 801 real keyboard characters at 2 ms.
        editor = [""]
        typed = []
        class Node:
            def queryComponent(self):
                return SimpleNamespace(grabFocus=lambda: True)
        ax = object.__new__(native.AxWebKit)
        ax.edit_field = lambda _label: Node()
        ax.read_text = lambda _label: editor[0]
        def keys(*argv, **_kwargs):
            self.assertEqual(argv[0], "xdotool")
            if argv[1] == "key":
                editor[0] = ""
            elif argv[1] == "type":
                self.assertEqual(argv[2], "--clearmodifiers")
                self.assertEqual(argv[-2], "--")
                self.assertLessEqual(len(argv[-1]), 64)
                typed.append(int(argv[argv.index("--delay") + 1]))
                editor[0] += argv[-1]
            else:
                raise AssertionError(f"Unexpected nonnative input method: {argv}")
        value = "Study original JSON: " + "0123456789abcdef" * 52
        with mock.patch.object(native, "command", side_effect=keys), \
             mock.patch.object(native.time, "sleep"):
            ax.type_into("Paste three-angle JSON response", value)
        self.assertEqual(editor[0], value)
        self.assertEqual(len(typed), (len(value) + 63) // 64)
        self.assertTrue(all(delay == 12 for delay in typed))

    def test_long_webkit_input_retries_only_with_exact_semantic_readback(self):
        # First native keyboard attempt loses a character; retry is a real
        # Ctrl+A + slower native key route, never UIA SetValue or direct DB.
        editor = [""]
        delays = []
        class Node:
            def queryComponent(self):
                return SimpleNamespace(grabFocus=lambda: True)
        ax = object.__new__(native.AxWebKit)
        ax.edit_field = lambda _label: Node()
        ax.read_text = lambda _label: editor[0]
        def keys(*argv, **_kwargs):
            if argv[1] == "key":
                editor[0] = ""
            else:
                delay = int(argv[argv.index("--delay") + 1])
                delays.append(delay)
                text = argv[-1]
                editor[0] += (text[:-1] if delay == 12 else text)
        # Fake monotonic advancement is scoped to this unit-level regression,
        # never to any installed runtime or QA evidence collector.
        clock = [0]
        def monotonic():
            clock[0] += 2.5
            return clock[0]
        value = "abcdefghij" * 81
        with mock.patch.object(native, "command", side_effect=keys), \
             mock.patch.object(native.time, "sleep"), \
             mock.patch.object(native.time, "monotonic", side_effect=monotonic):
            ax.type_into("Paste three-angle JSON response", value)
        self.assertEqual(editor[0], value)
        self.assertIn(12, delays)
        self.assertIn(35, delays)

    def test_rendered_native_angle_alert_requires_new_button_adjacent_rectangle(self):
        # Pure pixel contract; real certification still requires installed
        # AT-SPI button bounds, genuine before/after scrot PNGs and SQLite.
        bounds = SimpleNamespace(x=20, y=20, width=120, height=25)
        after = {(x, y): (255, 141, 141)
                 for y in (55, 96) for x in range(30, 130)}
        before_rgb = lambda _x, _y: (21, 28, 35)
        after_rgb = lambda x, y: after.get((x, y), (21, 28, 35))
        proof = native.native_new_alert_border(
            before_rgb, after_rgb, 200, 165, bounds
        )
        self.assertEqual(proof["alert_y_edges"], [55, 96])
        # An unchanged alert present before the click cannot be green.
        with self.assertRaises(native.NativeFailure):
            native.native_new_alert_border(
                after_rgb, after_rgb, 200, 165, bounds
            )
        # Red pixels elsewhere on the native desktop are not a relevant alert.
        with self.assertRaises(native.NativeFailure):
            native.native_new_alert_border(
                before_rgb, lambda x, y: (255, 141, 141) if y == 4
                else (21, 28, 35), 200, 165, bounds
            )
        # Only one red line / no bounded red notice is insufficient.
        with self.assertRaises(native.NativeFailure):
            native.native_new_alert_border(
                before_rgb,
                lambda x, y: (255, 141, 141) if y == 55 and 30 <= x < 130
                else (21, 28, 35), 200, 165, bounds
            )

    def test_native_angle_visual_fallback_never_replaces_immutable_db_and_queue_checks(self):
        import inspect
        source = inspect.getsource(native.exercise)
        self.assertIn('native_angle_alert_image_proof(', source)
        self.assertIn('session.screenshot("pre-negative-angles")', source)
        self.assertIn('session.screenshot("negative-angles-rejected")', source)
        self.assertIn('denied_url_recovery()', source)
        self.assertIn('verify_canonical_reopened_candidates(', source)
        self.assertIn('if not checked(report, "error_recovery", error_recovery):', source)



    def test_webkit_reviewed_three_angles_require_exact_native_action_count(self):
        # This tests only the AT-SPI evidence collector; a native PASS still
        # requires three real buttons after rejected input cleared old state,
        # valid public Rust response and read-only immutable DB/Queue checks.
        class Node:
            def __init__(self, label, role):
                self.name, self.role = label, role
            def getRoleName(self):
                return self.role

        class Ax:
            def __init__(self, nodes):
                self._nodes = nodes
            def nodes(self):
                return iter(self._nodes)

        button = lambda: Node("Prepare script for this angle", "push button")
        self.assertEqual(native.count_native_angle_actions(Ax([])), 0)
        self.assertEqual(native.count_native_angle_actions(Ax([
            Node("Prepare script for this angle", "label"),
            Node("Other action", "push button"),
            Node("Share a practical checklist", "document frame"),
        ])), 0)
        self.assertEqual(native.count_native_angle_actions(Ax([
            button(), button(), button(),
        ])), 3)
        import inspect
        exercise = inspect.getsource(native.exercise)
        self.assertIn('cards_after_rejection = count_native_angle_actions(a)', exercise)
        self.assertIn('if cards_after_rejection != 0:', exercise)
        self.assertIn('if count == 3:', exercise)
        self.assertIn('if count != 3:', exercise)
        self.assertNotIn('a.find("Share a practical checklist"', exercise)
        self.assertIn('verify_canonical_reopened_candidates(', exercise)
        self.assertIn('denied_url_recovery()', exercise)


    def test_webkit_image_map_can_expose_only_exact_production_error_label(self):
        # Actual installed WebKitGTK at run #44313725742 exposed a named
        # native image-map node for Svelte role=alert. Never accept arbitrary
        # similarly named map controls, even with 'angle invalid' tokens.
        class Node:
            def __init__(self, role, name):
                self.role, self.name = role, name
            def getRoleName(self):
                return self.role
        class Ax:
            def nodes(self):
                return iter([
                    Node("image map",
                         "The angle response is invalid or its Study evidence changed."),
                    Node("image map", "Unrelated angle invalid warning in another panel"),
                    Node("button", "The angle response is invalid or its Study evidence changed."),
                ])
        labels = native.native_named_alerts(Ax(), ("angle", "invalid"))
        self.assertEqual(labels, [
            "The angle response is invalid or its Study evidence changed."
        ])
        # This is accessible text evidence, NOT the authenticated Tauri ID.
        report = {"cases": {}}
        def only_webkit_image_map():
            report["angle_rejection_visual_proof"] = {
                "atspi_error_text_exposed": True,
                "request_correlation": {
                    "new_alerts": labels,
                    "prior_alerts": [],
                }
            }
            return "a WebKit image map is still not host denial correlation"
        self.assertFalse(native.checked(report, "error_recovery",
                                        only_webkit_image_map))
        self.assertEqual(report["cases"]["error_recovery"]["status"], "NOT TESTED")

    def test_agent10_unreviewed_screenshot_never_autocertifies_recovery(self):
        # Mirror Agent 10's adversarial callback against the REAL QA checked
        # helper. Neither a correct-looking red border nor downstream SQLite
        # success can upgrade unresolved visual adjudication to PASS.
        report = {"cases": {}}

        def screenshot_only():
            report["angle_rejection_visual_proof"] = {
                "proof": "new bordered error region below Validate",
                "atspi_error_text_exposed": False,
                "human_review_required": True,
            }
            return "three recovery cards; SQLite unchanged"

        self.assertFalse(native.checked(report, "error_recovery", screenshot_only))
        self.assertEqual(report["cases"]["error_recovery"]["status"], "NOT TESTED")
        self.assertIn("screenshot", report["cases"]["error_recovery"]["reason"])

    def test_review_flag_false_does_not_replace_native_semantic_evidence(self):
        # A manually flipped flag is not proof that the installed Tauri IPC
        # surfaced a named native rejection.
        report = {"cases": {}}

        def incomplete():
            report["angle_rejection_visual_proof"] = {
                "atspi_error_text_exposed": False,
                "human_review_required": False,
            }
            return "unreviewed screenshot promoted by mistake"

        self.assertFalse(native.checked(report, "error_recovery", incomplete))
        self.assertEqual(report["cases"]["error_recovery"]["status"], "NOT TESTED")

    def test_missing_native_rejection_evidence_cannot_pass(self):
        report = {"cases": {}}
        self.assertFalse(native.checked(
            report, "error_recovery", lambda: "no rejection proof"
        ))
        self.assertEqual(report["cases"]["error_recovery"]["status"], "NOT TESTED")

    def test_agent10_stale_semantic_flag_is_not_request_correlated_proof(self):
        # Exact independent Agent 10 adversarial shape: a stale/unrelated
        # semantic alert must not be upgraded to a genuine host rejection.
        report = {"cases": {}}

        def stale():
            report["angle_rejection_visual_proof"] = {
                "proof": "named AT-SPI native validation alert",
                "screenshot": "negative-angles-rejected.png",
                "atspi_error_text_exposed": True,
            }
            return "fake historical semantic flag"

        self.assertFalse(native.checked(report, "error_recovery", stale))
        self.assertEqual(report["cases"]["error_recovery"]["status"], "NOT TESTED")

    def test_semantic_rejection_requires_authoritative_backend_identity(self):
        # Test the declared future host contract without claiming a production
        # host response. Agent 00 must implement/expose this on the real Tauri
        # service before a native installed PASS is possible.
        payload = hashlib.sha256(b'{"angles":[{"id":"invalid"}]}').hexdigest()
        correlation = {
            "native_action": "Validate 3 angles",
            "submitted_payload_sha256": payload,
            "native_readback_sha256": payload,
            "prior_alerts": [],
            "new_alerts": [
                "The angle response is invalid or its Study evidence changed."
                + " · request " + "a" * 32
            ],
            "action_monotonic_ns": 1000000000,
            "observed_monotonic_ns": 1200000000,
            "canonical_before_digest": "f" * 64,
            "canonical_after_digest": "f" * 64,
        }
        proof = {
            "atspi_error_text_exposed": True,
            "proof": "fresh named AT-SPI alert after real native action",
            "request_correlation": correlation,
        }
        # Even accurate user-visible semantics and full data immutability
        # CANNOT prove which Tauri rejection happened.
        self.assertFalse(native.valid_native_angle_request_binding(proof))
        fake_unrelated = dict(correlation, new_alerts=[
            "Unrelated angle invalid warning in another workspace panel"
        ])
        self.assertFalse(native.valid_native_angle_request_binding(
            dict(proof, request_correlation=fake_unrelated)
        ))
        # A synthetic receipt tests helper validation only. It MUST NOT be
        # manufactured in native exercise(); source production needs to
        # return this stable opaque identifier.
        host = {
            "submitted_request_id": "a" * 32,
            "tauri_denial_request_id": "a" * 32,
            "native_alert_request_id": "a" * 32,
            "error_code": "INVALID_STUDY_ANGLES",
            "host_source": "installed-process-stderr",
            "accessibility_source": "installed-AT-SPI-alert",
        }
        bound = dict(correlation, authoritative_denial=host)
        self.assertTrue(native.valid_native_angle_request_binding(
            dict(proof, request_correlation=bound)
        ))
        for key, bad in [
            ("prior_alerts", ["The angle response is invalid or its Study evidence changed."]),
            ("native_readback_sha256", "e" * 64),
            ("new_alerts", ["Unrelated angle invalid warning in another workspace panel"]),
            ("observed_monotonic_ns", 500000000),
            ("canonical_after_digest", "e" * 64),
            ("native_action", "unrelated button"),
            ("authoritative_denial", dict(host, tauri_denial_request_id="b" * 32)),
        ]:
            with self.subTest(key=key):
                modified = dict(bound, **{key: bad})
                self.assertFalse(native.valid_native_angle_request_binding(
                    dict(proof, request_correlation=modified)
                ))
        report = {"cases": {}}
        def no_host_binding():
            report["angle_rejection_visual_proof"] = proof
            return "native error looked correct but host correlation unavailable"
        self.assertFalse(native.checked(report, "error_recovery", no_host_binding))
        self.assertEqual(report["cases"]["error_recovery"]["status"], "NOT TESTED")

    def test_host_stderr_and_native_alert_identity_must_independently_agree(self):
        # Deterministic SOURCE unit fixtures only. No native PASS is created.
        angle_text = "The angle response is invalid or its Study evidence changed."
        url_text = "Invalid media URL. Only HTTP and HTTPS URLs are supported."
        ident = "d" * 32
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "installed-stderr.log"
            path.write_text("unrelated host startup\n")
            offset = native.native_host_log_offset(path)
            alert = angle_text + " · request " + ident
            self.assertEqual(native.native_host_denial_ids(
                path, offset, "INVALID_STUDY_ANGLES"), [])
            self.assertIsNone(native.observed_host_denial(
                path, offset, "INVALID_STUDY_ANGLES", [alert], angle_text))
            # Only actual bytes appended after the native action count.
            with path.open("a") as f:
                f.write("[scriptotar-native-denial] code=INVALID_STUDY_ANGLES "
                        "request_id=" + ident + "\n")
            self.assertEqual(native.native_host_denial_ids(
                path, offset, "INVALID_STUDY_ANGLES"), [ident])
            receipt = native.observed_host_denial(
                path, offset, "INVALID_STUDY_ANGLES", [alert], angle_text)
            self.assertIsNotNone(receipt)
            self.assertEqual(receipt["tauri_denial_request_id"], ident)
            self.assertEqual(receipt["native_alert_request_id"], ident)
            self.assertIsNone(native.observed_host_denial(
                path, offset, "INVALID_STUDY_ANGLES",
                [angle_text + " · request " + "e" * 32], angle_text))
            self.assertIsNone(native.observed_host_denial(
                path, offset, "INVALID_STUDY_ANGLES",
                ["Unrelated angle invalid warning · request " + ident], angle_text))
            self.assertIsNone(native.observed_host_denial(
                path, offset, "INVALID_MEDIA_URL", [url_text + " · request " + ident],
                url_text))
            # A new unrelated marker cannot be used as the claimed host ID.
            with path.open("a") as f:
                f.write("[scriptotar-native-denial] code=INVALID_STUDY_ANGLES "
                        "request_id=" + "f" * 32 + "\n")
            self.assertIsNone(native.observed_host_denial(
                path, offset, "INVALID_STUDY_ANGLES", [alert], angle_text))

    def test_native_queue_denial_requires_exact_real_host_and_accessibility_proof(self):
        ident = "b" * 32
        label = "Invalid media URL. Only HTTP and HTTPS URLs are supported."
        host = {
            "submitted_request_id": ident,
            "tauri_denial_request_id": ident,
            "native_alert_request_id": ident,
            "error_code": "INVALID_MEDIA_URL",
            "host_source": "installed-process-stderr",
            "accessibility_source": "installed-AT-SPI-alert",
        }
        example = {
            "authoritative_denial": host,
            "new_alert_text": label + " · request " + ident,
            "canonical_unchanged": True,
            "job_count_unchanged": True,
            "native_input_readback_exact": True,
        }
        self.assertTrue(native.valid_native_url_request_binding(example))
        for bad in [
            dict(example, new_alert_text=label),
            dict(example, new_alert_text="Unrelated invalid URL · request " + ident),
            dict(example, canonical_unchanged=False),
            dict(example, job_count_unchanged=False),
            dict(example, native_input_readback_exact=False),
            dict(example, authoritative_denial=dict(host, tauri_denial_request_id="c" * 32)),
        ]:
            self.assertFalse(native.valid_native_url_request_binding(bad))

    def test_canonical_snapshot_detects_rewrites_and_appended_rows(self):
        with tempfile.TemporaryDirectory() as tmp:
            dbfile = Path(tmp) / "scriptotar.sqlite3"
            with sqlite3.connect(dbfile) as db:
                db.executescript("""
                    CREATE TABLE content_studies (id TEXT PRIMARY KEY, title TEXT);
                    CREATE TABLE jobs (id TEXT PRIMARY KEY, state TEXT);
                    CREATE TABLE draft_versions (id TEXT PRIMARY KEY, body TEXT);
                    INSERT INTO content_studies VALUES ('1', 'Study A');
                    INSERT INTO draft_versions VALUES ('v1', 'original immutable body');
                """)
            original = native.canonical_records_snapshot(Path(tmp))
            self.assertEqual(original["tables"]["content_studies"]["rows"], 1)
            with sqlite3.connect(dbfile) as db:
                db.execute("UPDATE draft_versions SET body='tampered' WHERE id='v1'")
            modified = native.canonical_records_snapshot(Path(tmp))
            self.assertNotEqual(original["digest"], modified["digest"])
            self.assertNotEqual(original["tables"]["draft_versions"],
                                modified["tables"]["draft_versions"])
            with sqlite3.connect(dbfile) as db:
                db.execute("INSERT INTO jobs VALUES ('job-new', 'created')")
            inserted = native.canonical_records_snapshot(Path(tmp))
            self.assertNotEqual(modified["digest"], inserted["digest"])

    def test_queue_no_write_proof_precedes_missing_alert_failure(self):
        # Even on an inaccessible Queue error, retain the read-only canonical
        # job-count observation in exact-head CI artifacts for Agent 00.
        import inspect
        source = inspect.getsource(native.exercise)
        block = source.split("def denied_url_recovery():", 1)[1].split(
            "def error_recovery():", 1
        )[0]
        self.assertIn('report["denied_url_canonical_job_count"]', block)
        self.assertLess(
            block.index('new_count = count_canonical_jobs('),
            block.index('if not error_text:'),
        )
        self.assertIn('session.screenshot("failed-denied-url-no-error")', block)
        self.assertIn("no named", block)

    def test_linux_json_input_is_read_back_from_the_real_native_editor(self):
        # On installed run #44308361975 corrected angles had zero AT-SPI
        # actions. This input-readback guard distinguishes an untyped or
        # truncated native JSON response from a real production validator bug.
        import inspect
        code = inspect.getsource(native.exercise)
        self.assertIn('malformed_angle_json =', code)
        self.assertIn('valid_angle_json =', code)
        self.assertIn(
            'observed_angle_json = a.read_text("Paste three-angle JSON response")',
            code,
        )
        self.assertIn('if observed_angle_json != malformed_angle_json:', code)
        self.assertIn('if observed_angle_json != valid_angle_json:', code)
        self.assertIn('report["angle_recovery_native_diagnostics"]', code)
        self.assertIn('failed-valid-three-angles-scrolled', code)
        self.assertIn('if count != 3:', code)

    def test_windows_native_error_requests_have_fresh_alert_and_full_sqlite_proof(self):
        source = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("function Get-InstalledAngleRejectionAlerts(", source)
        self.assertIn("function Get-InstalledQueueRejectionAlerts(", source)
        self.assertIn("Wait-InstalledAngleRejection $arabicHwnd $priorAngleAlerts", source)
        self.assertIn("if ($priorAngleAlerts.Count -ne 0)", source)
        self.assertIn("$beforeAnglesCanonical = Verify-InstalledCanonicalDb 'snapshot'", source)
        self.assertIn("$afterAnglesCanonical = Verify-InstalledCanonicalDb 'snapshot'", source)
        self.assertIn("if ($beforeAnglesCanonical -cne $afterAnglesCanonical)", source)
        self.assertIn("$beforeQueueCanonical = Verify-InstalledCanonicalDb 'snapshot'", source)
        self.assertIn("$afterQueueCanonical = Verify-InstalledCanonicalDb 'snapshot'", source)
        self.assertIn("$report.denied_url_canonical_snapshot", source)
        self.assertIn("$freshQueueAlerts = @(", source)
        self.assertIn("$_ -cnotin $priorQueueAlerts", source)
        self.assertIn("elif kind == \"snapshot\":", source)
        self.assertIn("canonical_records_snapshot(Path(root))", source)
        self.assertLess(
            source.index("$afterQueueCanonical = Verify-InstalledCanonicalDb 'snapshot'"),
            source.index("if (-not $deniedError)"),
        )

    def test_installed_foreign_reference_is_partial_not_forged_write_ipc_pass(self):
        # Future PR #138 offers genuine installed-host read-only get_study via
        # a normal UI. Never confuse that with forged mutating IPC denial.
        import inspect
        source = inspect.getsource(native.exercise)
        self.assertIn('if ax.available("Open Study reference"):', source)
        self.assertIn('active.type_into("Paste Study reference", reference)', source)
        self.assertIn('active.read_text("Paste Study reference") != reference', source)
        self.assertIn('active.click("Open reference", "button")', source)
        self.assertIn('before = canonical_records_snapshot(root / "data")', source)
        self.assertIn('after = canonical_records_snapshot(root / "data")', source)
        self.assertIn('"status": "PARTIAL" if found_error and before == after else "FAIL"', source)
        self.assertIn('status(report, "wrong_project_ipc_denial", "NOT TESTED"', source)
        self.assertNotIn('status(report, "wrong_project_ipc_denial", "PASS"', source)
        self.assertIn('"This Study reference cannot be opened in the active project."', source)
        self.assertIn('verify_canonical_reopened_candidates(', source)
        # The exact Arabic label must match production PR #138, not a
        # guessed sentence that would never appear as an AT-SPI name.
        self.assertIn("لا يمكن فتح هذا المرجع ضمن المشروع النشط.",
                      native.PRODUCTION_ERROR_LABELS)

    def test_foreign_reference_ids_derive_from_real_canonical_rows_read_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            dbfile = Path(tmp) / "scriptotar.sqlite3"
            p = "aaaabbbb-1111-2222-3333-444444444444"
            st = "aaaa2222-1111-2222-3333-444444444444"
            dr = "aaaa3333-1111-2222-3333-444444444444"
            with sqlite3.connect(dbfile) as db:
                db.executescript("""
                    CREATE TABLE projects (id TEXT, name TEXT, archived_at TEXT);
                    CREATE TABLE content_studies
                      (id TEXT, project_id TEXT, title TEXT, archived_at TEXT);
                    CREATE TABLE creative_drafts
                      (id TEXT, project_id TEXT, study_id TEXT, archived_at TEXT);
                """)
                db.execute("INSERT INTO projects VALUES (?, ?, NULL)", (p, "Project A"))
                db.execute("INSERT INTO content_studies VALUES (?, ?, ?, NULL)",
                           (st, p, "Study A"))
                db.execute("INSERT INTO creative_drafts VALUES (?, ?, ?, NULL)",
                           (dr, p, st))
            original = dbfile.read_bytes()
            self.assertEqual(
                native.canonical_study_reference_ids(Path(tmp), "Project A", "Study A"),
                (p, st, dr),
            )
            self.assertEqual(dbfile.read_bytes(), original)
            with self.assertRaises(native.NativeFailure):
                native.canonical_study_reference_ids(Path(tmp), "Project B", "Study A")

    def test_windows_foreign_study_reference_stays_partial_read_only(self):
        source = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("function Get-InstalledReferenceDenials(", source)
        self.assertIn("canonical_study_reference_ids,", source)
        self.assertIn('elif kind == "study_reference":', source)
        self.assertIn("$report.foreign_study_reference_probe", source)
        self.assertIn("Paste-InstalledJson $restoredHwnd 'Paste Study reference' $reference", source)
        self.assertIn("Verify-InstalledCanonicalDb 'snapshot' $first", source)
        self.assertIn("$priorDenial = @(Get-InstalledReferenceDenials", source)
        self.assertIn("Get-InstalledReferenceDenials $restoredHwnd", source)
        self.assertIn("Click-InstalledControl $restoredHwnd 'Open reference'", source)
        self.assertIn("status='NOT TESTED'", source)
        self.assertIn("forged mutating Tauri IPC", source)
        self.assertNotIn("$cases.wrong_project_ipc_denial = @{ status='PASS'", source)
        self.assertIn("$cases.wrong_project_ipc_denial.status -ne 'FAIL'", source)

    def test_windows_native_negative_case_never_auto_passes_without_host_binding(self):
        # A genuinely visible but unrelated fresh UIA warning and unchanged
        # SQLite are insufficient to identify the rejected Tauri request.
        script = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("$report.error_recovery_observations", script)
        self.assertIn("backend_request_identity = $null", script)
        self.assertIn("status='NOT TESTED'", script)
        self.assertIn("no authoritative Tauri denial ID", script)
        block = script.split("$report.error_recovery_observations = @", 1)[1]
        block = block.split("$cases.error_recovery = @", 1)[1]
        self.assertIn("status='NOT TESTED'", block.split("} catch {", 1)[0])
        self.assertNotIn("status='PASS'", block.split("} catch {", 1)[0])

    def test_windows_queue_url_requires_native_pointer_and_exact_paste(self):
        # A SetFocus/SendKeys-only URL entry left the actual installed
        # WebView2 Media URL field at its placeholder in run #44308361975.
        # Pointer and clipboard are genuine native controls, not UIA SetValue.
        source = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn('function Paste-InstalledQueueUrl(', source)
        helper = source.split('function Paste-InstalledQueueUrl(', 1)[1].split(
            'function Paste-InstalledJson(', 1
        )[0]
        self.assertIn("Click-InstalledControl $hwnd 'Media URL'", helper)
        self.assertIn(') -pointer', helper)
        self.assertIn('Set-Clipboard -Value $url', helper)
        self.assertIn("SendWait('^a')", helper)
        self.assertIn("SendWait('^v')", helper)
        self.assertIn('ValuePattern]::Pattern).Current.Value', helper)
        self.assertIn('if ($actual -cne $url)', helper)
        self.assertNotIn('SetValue(', helper)
        self.assertIn(
            "Paste-InstalledQueueUrl $arabicHwnd $rejectedUrl",
            source,
        )
        self.assertNotIn(
            "Type-InstalledEdit $arabicHwnd 'Media URL'",
            source,
        )

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
        self.assertIn("foreach ($choice in $actualThemeChoices)", script)
        self.assertIn("Choose-InstalledOption $restoredHwnd 'Theme' $choice", script)
        self.assertIn("Wait-InstalledComboValue $restoredHwnd 'Theme' $choice", script)
        self.assertIn("if ($saveTheme.Current.IsEnabled)", script)
        self.assertIn("[System.Windows.Forms.SendKeys]::SendWait('%{F4}')", script)
        self.assertIn("$themeProbe = Find-InstalledDocumentAndStudio $proc", script)
        self.assertIn("Wait-InstalledComboValue $themeHwnd 'Theme' 'Dark'", script)
        self.assertIn("windows-theme-dark-reopened.png", script)
        self.assertIn("$cases.theme_mode_control = @{", script)

    def test_arabic_installed_navigation_captures_do_not_self_certify_rtl(self):
        import inspect
        linux = inspect.getsource(native.exercise)
        self.assertIn('a.click("الطابور", "button")', linux)
        self.assertIn('a.heading("المهام")', linux)
        self.assertIn('session.screenshot("arabic-queue")', linux)
        self.assertIn('a.click("الإعدادات", "button")', linux)
        self.assertIn('session.screenshot("arabic-settings")', linux)
        self.assertIn('a.click("الاستوديو", "button")', linux)
        self.assertIn('status(report, "arabic_rtl", "NOT TESTED"', linux)
        windows = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("'الطابور' ([System.Windows.Automation.ControlType]::Button)", windows)
        self.assertIn("Wait-InstalledStudioPageText $themeHwnd 'المهام'", windows)
        self.assertIn("'الإعدادات' ([System.Windows.Automation.ControlType]::Button)", windows)
        self.assertIn("windows-arabic-queue.png", windows)
        self.assertIn("windows-arabic-settings.png", windows)
        self.assertIn("windows-arabic-studio-return.png", windows)
        self.assertIn("'arabic_rtl', 'light_dark_visual'", windows)

    def test_installed_light_mode_is_conditional_and_never_inferred_from_system(self):
        import inspect
        source = inspect.getsource(native.exercise)
        self.assertIn('light_offered = a.available("Light", "list item")', source)
        self.assertIn('if light_offered:', source)
        self.assertIn('a.choose("Theme", "Light")', source)
        self.assertIn('a.assert_selected_option("Theme", "Light")', source)
        self.assertIn('session.screenshot("light-theme")', source)
        self.assertIn('status(report, "light_dark_visual", "NOT TESTED"', source)
        windows = (Path(__file__).resolve().parent / "windows_webview2.ps1").read_text()
        self.assertIn("$lightOffered = [bool](Test-InstalledPopupOptionVisible", windows)
        self.assertIn("'explicit_light_option'", windows)
        self.assertIn("@('System', 'Light', 'Dark')", windows)
        self.assertIn("@('System', 'Dark')", windows)
        self.assertIn("foreach ($choice in $actualThemeChoices)", windows)
        self.assertIn("windows-theme-", windows)
        self.assertIn("'arabic_rtl', 'light_dark_visual'", windows)

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
            if arguments[1] == "getactivewindow":
                return SimpleNamespace(stdout="24680\n", returncode=0)
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

    def test_linux_graphical_quit_requires_real_focus_without_blocking_sync(self):
        # The hosted Openbox/WebKit X11 activation may deadlock under --sync.
        # A nonblocking request is not enough: actual focus must be observed
        # before dispatching the genuine native Alt+F4.
        session = native.Session(SimpleNamespace(), Path("/unused"))
        proc = mock.Mock(pid=1234)
        proc.poll.return_value = None
        proc.wait.return_value = 0
        session.proc = proc
        seen = []

        def installed_command(*argv, **_kwargs):
            seen.append(argv)
            if argv[1] == "search":
                return SimpleNamespace(returncode=0, stdout="4196049\n")
            if argv[1] == "getwindowname":
                return SimpleNamespace(returncode=0, stdout="Scriptotar")
            if argv[1] == "getactivewindow":
                return SimpleNamespace(returncode=0, stdout="4196049\n")
            return SimpleNamespace(returncode=0, stdout="")

        with mock.patch.object(native, "command", side_effect=installed_command):
            session.stop()
        self.assertIsNone(session.proc)
        self.assertIn(("xdotool", "windowactivate", "4196049"), seen)
        self.assertIn(("xdotool", "getactivewindow"), seen)
        self.assertIn(("xdotool", "key", "--clearmodifiers", "alt+F4"), seen)
        self.assertFalse(any("--sync" in call for call in seen))
        self.assertLess(
            seen.index(("xdotool", "getactivewindow")),
            seen.index(("xdotool", "key", "--clearmodifiers", "alt+F4")),
        )
        proc.terminate.assert_not_called()

        # A SIGSEGV on real Alt+F4 must never be accepted as normal quit.
        # Preserve the signal for independent installed-run diagnostics.
        with tempfile.TemporaryDirectory() as temp_dir:
            session.root = Path(temp_dir)
            session.proc = proc
            proc.wait.return_value = -11
            with mock.patch.object(native, "command", side_effect=installed_command):
                with self.assertRaisesRegex(
                    native.NativeFailure, r"Installed native graphical shutdown crashed: -11 \(SIGSEGV\)"
                ):
                    session.stop()
            import json
            crash = json.loads((Path(temp_dir) / "process-exit.json").read_text())
            self.assertEqual(crash["pid"], 1234)
            self.assertEqual(crash["returncode"], -11)
            self.assertEqual(crash["signal"], "SIGSEGV")
            self.assertEqual(
                crash["stage"], "installed native graphical Alt+F4 shutdown"
            )
        proc.wait.return_value = 0

        session.proc = proc
        seen.clear()
        keyboard_requested = [False]
        def keyboard_focused_command(*argv, **_kwargs):
            if argv[1] in ("getactivewindow", "windowfocus", "getwindowfocus"):
                seen.append(argv)
            if argv[1] == "getactivewindow":
                return SimpleNamespace(returncode=0, stdout="999999\n")
            if argv[1] == "windowfocus":
                keyboard_requested[0] = True
                return SimpleNamespace(returncode=0, stdout="")
            if argv[1] == "getwindowfocus":
                return SimpleNamespace(
                    returncode=0,
                    stdout="4196049\n" if keyboard_requested[0] else "999999\n",
                )
            return installed_command(*argv, **_kwargs)
        with (
            mock.patch.object(native, "command", side_effect=keyboard_focused_command),
            mock.patch.object(native.time, "monotonic", side_effect=[0, 5, 6, 7]),
        ):
            session.stop()
        self.assertIn(("xdotool", "windowfocus", "4196049"), seen)
        self.assertIn(("xdotool", "getwindowfocus"), seen)
        self.assertIn(("xdotool", "key", "--clearmodifiers", "alt+F4"), seen)
        self.assertFalse(any("--sync" in call for call in seen))

        session.proc = proc
        seen.clear()
        def unfocused_command(*argv, **_kwargs):
            if argv[1] == "getactivewindow":
                return SimpleNamespace(returncode=0, stdout="999999\n")
            return installed_command(*argv, **_kwargs)
        with (
            mock.patch.object(native, "command", side_effect=unfocused_command),
            mock.patch.object(native.time, "monotonic", side_effect=[0, 5, 6, 11]),
        ):
            with self.assertRaisesRegex(native.NativeFailure, r"refusing Alt\+F4"):
                session.stop()
        self.assertNotIn(("xdotool", "key", "--clearmodifiers", "alt+F4"), seen)
        proc.terminate.assert_called()

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



    def test_native_error_recovery_requires_real_queue_denial_and_db_readback(self):
        # Static regression contract only; a source check never turns the
        # installed scenario green. The OS runners must perform the actual UI.
        from pathlib import Path
        root = Path(__file__).resolve().parent
        linux = (root / "linux_webkit.py").read_text(encoding="utf-8")
        windows = (root / "windows_webview2.ps1").read_text(encoding="utf-8")
        for source in (linux, windows):
            self.assertIn("file:///scriptotar-native-qa-denied-media.mp4", source)
            self.assertIn("Script draft", source)
            self.assertIn("error_recovery", source)
        self.assertIn('session.screenshot("rejected-url-error")', linux)
        self.assertIn('session.screenshot("recovered-studio-after-denial")', linux)
        self.assertIn("rejected-url-error.png", windows)
        self.assertIn("recovered-studio-after-denial.png", windows)
        self.assertIn('a.click("Queue", "button")', linux)
        self.assertIn('a.heading("Jobs", timeout=15)', linux)
        self.assertIn('a.click("Queue URL", "button")', linux)
        self.assertIn('count_canonical_jobs(root / "data", name_a)', linux)
        self.assertIn('a.choose("Open study", study)', linux)
        self.assertIn('verify_canonical_reopened_candidates(', linux)
        self.assertIn("Click-InstalledControl $arabicHwnd 'Queue URL'", windows)
        self.assertIn("Verify-InstalledCanonicalDb 'jobs'", windows)
        self.assertIn("Wait-InstalledEditExact $arabicHwnd 'Script draft' $draftBody", windows)
        self.assertIn("Verify-InstalledCanonicalDb 'candidates'", windows)
        # Do not accept an ordinary 'Supported media URL' label as an error.
        self.assertIn("$value -ne 'Supported media URL'", windows)
        self.assertIn("status='FAIL'", windows)
        self.assertNotIn('"error_recovery", "PASS"', linux)

    def test_visual_menu_captures_do_not_self_certify(self):
        from pathlib import Path
        root = Path(__file__).resolve().parent
        linux = (root / "linux_webkit.py").read_text(encoding="utf-8")
        windows = (root / "windows_webview2.ps1").read_text(encoding="utf-8")
        self.assertIn('session.screenshot("arabic-locale-menu-open")', linux)
        self.assertIn('session.screenshot("dark-theme-menu-open")', linux)
        self.assertIn('windows-arabic-menu-open.png', windows)
        self.assertIn('windows-theme-menu-open.png', windows)
        self.assertIn('status(report, "arabic_rtl", "NOT TESTED"', linux)
        self.assertIn('status(report, "light_dark_visual", "NOT TESTED"', linux)
        self.assertIn("'arabic_rtl', 'light_dark_visual'", windows)


if __name__ == "__main__":
    unittest.main()
