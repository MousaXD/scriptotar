"""Fail-closed structural gate for the *real* Tauri host checkout.

This does not certify the locking protocol by itself: native adversarial tests
and a final exact-head installed-app review are separately mandatory.
"""
import pathlib
import re
import unittest

HOST = pathlib.Path(__file__).resolve().parents[3] / "apps/desktop/src-tauri/src"
AI_LIB = pathlib.Path(__file__).resolve().parents[3] / "crates/scriptotar-ai/src/lib.rs"


def executable_rust(source: str) -> str:
    """Ignore ordinary comments so a TODO cannot satisfy a release check."""
    source = re.sub(r"/\*[\s\S]*?\*/", "", source)
    return re.sub(r"(?m)^\s*//[^\n]*$", "", source)


class CombinedHostWiringSecurity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.services = executable_rust((HOST / "services.rs").read_text())
        cls.ai_lib = executable_rust(AI_LIB.read_text())
        cls.host_sources = {
            p: executable_rust(p.read_text())
            for p in HOST.rglob("*.rs")
        }
        cls.production = "\n".join(cls.host_sources.values())

    def test_provider_policy_is_exported_and_included_by_real_service_graph(self):
        self.assertRegex(self.ai_lib, r"\bpub\s+mod\s+provider_consent\s*;")
        self.assertRegex(
            self.services, r"\bprovider_consent\b",
            "provider module is not referenced by central AppServices: an isolated "
            "crate test is NOT a production Tauri integration certificate",
        )

    def test_actual_host_implements_shared_scope_guard_and_authority(self):
        for trait in ("ScopeAuthority", "ScopeDispatchGuard"):
            self.assertRegex(
                self.production,
                r"\bimpl(?:\s*<[^>]+>)?\s+" + trait + r"\s+for\b",
                f"no real host implementation of {trait}; default-deny mock is insufficient",
            )
        self.assertIn("lock_dispatch_scope", self.production)
        self.assertIn("now_ms", self.production)

    def test_actual_host_has_durable_paid_claim_and_pre_send_cas_adapter(self):
        self.assertRegex(
            self.production,
            r"\bimpl(?:\s*<[^>]+>)?\s+PaidEffectFence\s+for\b",
            "v10 effect driver alone does not implement the consent-to-effect seam",
        )
        self.assertIn("mark_dispatched_once", self.production)


if __name__ == "__main__":
    unittest.main()
