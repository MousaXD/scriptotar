"""Agent 10 negative proof: self-described ZIP metadata is not native certification.

No installed software, screenshot capture, or provider access occurs here.
The genuine installed WebView acceptance suite is the only native UI gate.
"""
import json
import tempfile
import unittest
import zipfile
from pathlib import Path

from test_visual_observations import SHA, fixture_zip
from visual_observations import analyze


class Agent10UntrustedArchiveBoundaryTests(unittest.TestCase):
    def test_forged_installed_verdict_is_only_unauthenticated_input(self):
        for platform in ("linux", "windows"):
            with self.subTest(platform=platform), tempfile.TemporaryDirectory() as temp:
                archive_path = Path(temp) / "synthetic-not-installed.zip"
                fixture_zip(archive_path, platform)
                with zipfile.ZipFile(archive_path) as z:
                    original_members = [
                        (info.filename, z.read(info))
                        for info in z.infolist()
                    ]
                report = json.loads(next(
                    value for name, value in original_members
                    if name == "result.json"
                ))
                # Every value below is an untrusted self-assertion, including
                # the GitHub run URL and package/executable SHA-looking data.
                report["run_url"] = (
                    "https://github.com/MousaXD/scriptotar/actions/runs/999999999999"
                )
                report["package_sha256"] = "d" * 64
                report["executable_sha256"] = "e" * 64
                report["verdict"] = "PASS"
                report["cases"]["light_dark_visual"] = {"status": "PASS"}
                report["cases"]["arabic_rtl"] = {"status": "PASS"}
                with zipfile.ZipFile(archive_path, "w") as z:
                    for name, payload in original_members:
                        z.writestr(
                            name,
                            json.dumps(report).encode("utf-8")
                            if name == "result.json" else payload,
                        )

                observation = analyze(archive_path, SHA)
                self.assertEqual(observation["source_sha"], SHA)
                # These two booleans are forgeable observations, not proof
                # of a native installed app, even for a valid exact SHA.
                self.assertTrue(observation["native_theme_ui_gate_passed"])
                self.assertTrue(observation[
                    "light_dark_and_persisted_dark_observed"
                ])
                self.assertEqual(
                    observation["native_16_case_verdict_unchanged"],
                    "PASS",
                    "This output merely echoes the forged, unsigned JSON verdict",
                )
                # The reader correctly keeps these certifications NOT TESTED.
                self.assertEqual(
                    observation["light_dark_visual_certification"], "NOT TESTED"
                )
                self.assertEqual(
                    observation["arabic_rtl_certification"], "NOT TESTED"
                )
                self.assertIn(
                    "not a signature", observation["limits"].lower()
                )

    def test_invalid_forged_sha_or_result_json_still_fails_closed(self):
        with tempfile.TemporaryDirectory() as temp:
            archive_path = Path(temp) / "untrusted.zip"
            fixture_zip(archive_path, "linux", source="f" * 40)
            with self.assertRaisesRegex(ValueError, "identity"):
                analyze(archive_path, SHA)
            fixture_zip(archive_path, "linux", source=SHA)
            with self.assertRaisesRegex(ValueError, "Invalid 40-character"):
                analyze(archive_path, "not-a-git-sha")


if __name__ == "__main__":
    unittest.main()
