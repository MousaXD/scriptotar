"""Agent 10 source-level guard for canonical draft verification.

This guard intentionally SKIPS until PR #117 is integrated. On PR #117 it
fails: metadata-only canonical-draft checks do not bind the supplied claim body.
This is a static guard, not a substitute for native Rust integration tests.
"""

from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
MODULE = ROOT / "apps/desktop/src-tauri/src/services/study_verification.rs"


@unittest.skipUnless(MODULE.exists(), "PR #117 verification adapter not integrated")
class CanonicalDraftSecurityContract(unittest.TestCase):
    def test_claims_must_not_be_extracted_from_unvalidated_caller_draft(self):
        source = MODULE.read_text(encoding="utf-8")
        self.assertIn("pub fn verify_studio_claims_read_only(", source)
        body = source.split("pub fn verify_studio_claims_read_only(", 1)[1]
        body = body.split("#[cfg(test)]", 1)[0]
        self.assertIn("self.store.get_draft(project_id, draft_id)?", body)
        self.assertNotIn(
            "extract_draft_claims(draft)",
            body,
            "A project-scoped ID and matching revision are not proof that the "
            "caller-supplied ScriptDraft text/claims are the canonical saved "
            "draft. Load and verify the saved body/claims or compare a trusted "
            "content digest before deriving any claims from caller input.",
        )


if __name__ == "__main__":
    unittest.main()
