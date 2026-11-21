"""Agent 10 regression: a caller's unsaved/foreign/stale draft cannot be Verified.

Two fail-closed policies are legitimate:
* issue an explicitly unbound/nonpublishable uncertainty report; or
* refuse to generate any claim report unless the caller exactly matches an
  authenticated current immutable DraftVersion and stored structured manifest.

Never treat a returned report's positive content binding as acceptable merely
because a caller supplied a real draft UUID. The host must check the whole
manifest against the exact saved version before extracting user-proposed claims.

This is a source-contract preflight, NOT independent execution proof. The Rust
SQLite/adversarial integration tests and exact-head CI are still mandatory.
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

        # A rejected unbound request is strictly safer than an uncertainty
        # report. A literal 'draft_content_bound: false' check alone would
        # reject the real canonical saved-version-only policy.
        if "extract_draft_claims(draft)" in body:
            if "draft_content_bound: false" in body:
                self.assertIn(
                    "publication_review_required = true",
                    core,
                    "An unbound uncertainty report cannot authorize publishing",
                )
            else:
                # The successful path is allowed to claim content binding
                # ONLY after byte-identical saved-body and full typed-claim
                # equality. Check the refusal boundary occurs first.
                self.assertIn("scoped_current_saved_version(", body)
                self.assertIn("let structured = structured.ok_or_else(", body)
                self.assertIn(
                    "if !typed_revision_matches(&structured, &saved_version, draft, &linked_sources)",
                    body,
                )
                self.assertIn(
                    "submitted claims differ from the exact saved DraftVersion",
                    body,
                )
                self.assertLess(
                    body.index("if !typed_revision_matches("),
                    body.index("extract_draft_claims(draft)"),
                )
                self.assertIn("export_candidate(&candidate) != version.body", source)
                self.assertIn("canonical == *supplied", source)
                self.assertIn("draft_content_bound: true", body)
                self.assertIn("human_reviews: Vec::new()", body)
                self.assertIn("saved version has no trusted structured claims", body)

            self.assertIn(
                "&& ctx.draft_content_bound",
                core,
                "The verifier must require trusted saved draft binding",
            )
            self.assertIn(
                "publication_review_required = true",
                core,
                "No current claim review authorizes publication",
            )
            self.assertIn(
                "inventory_complete: false",
                core,
                "Typed claims cannot certify full-script assertion coverage",
            )


if __name__ == "__main__":
    unittest.main()
