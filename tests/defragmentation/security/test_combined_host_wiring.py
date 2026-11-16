"""Fail closed for an unregistered Tauri consent/effect host.

Structural checks are necessary but are not independent proof that all project,
Study and source mutations acquire the same lock during network egress.
"""
import pathlib
import re
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
HOST = ROOT / "apps/desktop/src-tauri/src"
AI_LIB = ROOT / "crates/scriptotar-ai/src/lib.rs"


def executable_rust(source: str) -> str:
    # Ordinary comments must not satisfy a production wiring assertion.
    source = re.sub(r"/\*[\s\S]*?\*/", "", source)
    return re.sub(r"(?m)^\s*//[^\n]*$", "", source)


class CombinedHostWiringSecurity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.services = executable_rust((HOST / "services.rs").read_text())
        cls.ai_lib = executable_rust(AI_LIB.read_text())
        cls.production = "\n".join(
            executable_rust(path.read_text())
            for path in HOST.rglob("*.rs")
        )

    @staticmethod
    def require(pattern: str, content: str, message: str) -> None:
        # Never include full application source in a failing assertion's logs.
        if re.search(pattern, content) is None:
            raise AssertionError(message)

    def test_provider_policy_is_exported_and_included_by_real_service_graph(self):
        self.require(
            r"\bpub\s+mod\s+provider_consent\s*;",
            self.ai_lib,
            "provider policy is not exported by the compiled AI crate",
        )
        self.require(
            r"\bprovider_consent\b",
            self.services,
            "provider policy missing from central AppServices compilation graph",
        )

    def test_actual_host_implements_shared_scope_guard_and_authority(self):
        for trait in ("ScopeAuthority", "ScopeDispatchGuard"):
            self.require(
                r"\bimpl(?:\s*<[^>]+>)?\s+" + trait + r"\s+for\b",
                self.production,
                f"no real host {trait} implementation (mock/default denial is not integration)",
            )
        for method in ("lock_dispatch_scope", "now_ms"):
            self.assertTrue(
                method in self.production,
                f"host has no authoritative dispatch-scope method {method}",
            )

    def test_actual_host_has_durable_paid_claim_and_pre_send_cas_adapter(self):
        self.require(
            r"\bimpl(?:\s*<[^>]+>)?\s+PaidEffectFence\s+for\b",
            self.production,
            "host has no PaidEffectFence implementation bridging consent to v10 ledger",
        )
        self.assertTrue(
            "mark_dispatched_once" in self.production,
            "host has no pre-egress DISPATCHED compare-and-swap adapter",
        )


if __name__ == "__main__":
    unittest.main()
