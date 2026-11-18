#!/usr/bin/env python3
"""Independent Agent 10 regression: unreviewed screenshot-only denial is not PASS.

This does NOT operate the installed app and is not proof that malformed JSON is
accepted by Rust. It tests the narrower certification/reporting boundary that
must distinguish machine-proven native denial from image-only evidence requiring
human adjudication. The target QA runner comes from Agent 07's exact PR head.
"""
from pathlib import Path
import runpy
import sys
import unittest

TARGET = Path(sys.argv[1] if len(sys.argv) > 1 else
              "target/tests/defragmentation/native-e2e/linux_webkit.py").resolve()


class IndependentNativeVisualEvidence(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = TARGET.read_text(encoding="utf-8")
        cls.native = runpy.run_path(str(TARGET), run_name="_agent10_review")

    def test_image_only_angle_rejection_cannot_be_autocertified(self):
        # A runner without image fallback is not exposed to this particular gap.
        if ("native_angle_alert_image_proof(" not in self.source
                or '"human_review_required": True' not in self.source):
            return
        report = {"cases": {}}

        def no_semantic_error_text():
            # Model precisely the image-only evidence shape written by Agent 07.
            # A changed red rectangle does not establish which error appeared.
            report["angle_rejection_visual_proof"] = {
                "proof": "new bordered error region immediately below native Validate control",
                "atspi_error_text_exposed": False,
                "human_review_required": True,
            }
            return "three angles reviewed; original SQLite drafts unchanged"

        passed = self.native["checked"](report, "error_recovery", no_semantic_error_text)
        self.assertFalse(
            passed,
            "FAIL OPEN: a successful callback marked screenshot-only angle "
            "rejection as certified before independent human visual review",
        )
        self.assertNotEqual(
            report["cases"]["error_recovery"]["status"], "PASS",
            "FAIL OPEN: unadjudicated red rectangle became native error_recovery PASS",
        )

    def test_unbound_semantic_flag_is_not_request_correlated_proof(self):
        """A caller-owned semantic=True bit cannot authenticate the denied IPC.

        The actual QA runner currently sets this shape after finding *any*
        visible AT-SPI node whose name contains both 'angle' and 'invalid'.
        Nothing in that shape proves the node was newly produced by this exact
        malformed payload or tied to the public Tauri response. Simulate that
        indistinguishable forged/stale evidence; a callback succeeding must
        not certify this negative path without bound proof.
        """
        report = {"cases": {}}

        def stale_unrelated_semantic_alert():
            report["angle_rejection_visual_proof"] = {
                "proof": "named AT-SPI native validation alert",
                "screenshot": "negative-angles-rejected.png",
                "atspi_error_text_exposed": True,
                # No submitted request identity, observed denial, or before/
                # after semantic transition links this to the actual action.
            }
            return "three angles reviewed; original SQLite drafts unchanged"

        passed = self.native["checked"](
            report, "error_recovery", stale_unrelated_semantic_alert
        )
        self.assertFalse(
            passed,
            "FAIL OPEN: stale/unrelated 'angle invalid' text plus a caller-owned "
            "semantic flag certified rejection without request-correlated "
            "native IPC denial evidence",
        )
        self.assertNotEqual(
            report["cases"]["error_recovery"]["status"], "PASS",
            "FAIL OPEN: semantic-looking metadata alone became host certification",
        )

    def test_visual_fallback_does_not_hide_human_review_obligation(self):
        if "native_angle_alert_image_proof(" not in self.source:
            return
        self.assertIn('"human_review_required": True', self.source)
        self.assertIn("atspi_error_text_exposed", self.source)


if __name__ == "__main__":
    # unittest arguments must not treat the QA source file as a test selector.
    unittest.main(argv=[sys.argv[0]], verbosity=2)
