"""Agent 10 regression: unbound caller claims must never be Verified.

Skipped until Agent 04 verification adapter is integrated. It permits a
read-only uncertainty report for an unsaved caller draft only when its output
explicitly disables verification and requires publication review.
"""

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
HOST = ROOT / "apps/desktop/src-tauri/src/services/study_verification.rs"
CORE = ROOT / "crates/scriptotar-ai/src/writing/verification/mod.rs"


@unittest.skipUnless(HOST.exists() and CORE.exists(),
                     "Agent 04 verification implementation not integrated")
class CanonicalDraftSecurityContract(unittest.TestCase):
    def test_unbound_draft_body_never_receives_verified_status(self):
        source = HOST.read_text(encoding="utf-8")
        core = CORE.read_text(encoding="utf-8")
        self.assertIn("pub fn verify_studio_claims_read_only(", source)
        body = source.split("pub fn verify_studio_claims_read_only(", 1)[1]
        body = body.split("#[cfg(test)]", 1)[0]
        self.assertIn("self.store.get_draft(project_id, draft_id)?", body)
        if "extract_draft_claims(draft)" in body:
            self.assertIn(
                "draft_content_bound: false",
                body,
                "Host analyzes untrusted draft content, yet does not revoke "
                "Verified eligibility for that noncanonical body",
            )
            self.assertIn(
                "&& ctx.draft_content_bound",
                core,
                "Verifier does not require trusted canonical draft content "
                "binding before returning Verified",
            )
            self.assertIn(
                "publication_review_required = true",
                core,
                "An unbound draft must never be publication-approved",
            )


if __name__ == "__main__":
    unittest.main()
