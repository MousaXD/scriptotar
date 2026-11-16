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



def production_rust(source: str) -> str:
    """Drop ordinary unit-test module tails: test-only trait impls cannot certify a host."""
    source = executable_rust(source)
    return re.sub(r"(?s)#\[cfg\(test\)\]\s*mod\s+\w+\s*\{.*\Z", "", source)


def rust_dispatch_calls(source: str) -> list[str]:
    """Read argument lists for .dispatch(...), balancing nested Rust call parentheses."""
    calls: list[str] = []
    for found in re.finditer(r"\.dispatch\s*\(", source):
        opened = source.find("(", found.start())
        depth = 1
        i = opened + 1
        while i < len(source) and depth:
            if source[i] == "(":
                depth += 1
            elif source[i] == ")":
                depth -= 1
            i += 1
        if depth == 0:
            calls.append(source[opened + 1:i - 1])
    return calls

class CombinedHostWiringSecurity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.services = production_rust((HOST / "services.rs").read_text())
        cls.ai_lib = executable_rust(AI_LIB.read_text())
        cls.consent_host = production_rust((HOST / "services/provider_consent.rs").read_text())
        cls.production = "\n".join(
            production_rust(path.read_text())
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


    def test_paid_fence_is_reached_by_a_non_disabled_host_dispatch(self):
        # An unused impl PaidEffectFence for SqlitePaidEffectFence must NOT make
        # combined-host authorization green. No provider or keys are invoked.
        self.require(
            r"\bSqlitePaidEffectFence\s*::\s*from_approved_context\s*\(",
            self.production,
            "durable paid fence exists but is never constructed on a production host path",
        )
        actual_calls = rust_dispatch_calls(self.consent_host)
        self.assertTrue(
            any(
                "DisabledPaidEffectFence" not in args
                and "NoTrustedPricing" not in args
                and "ExistingAiServiceAdapter" not in args
                for args in actual_calls
            ),
            "only fail-closed legacy dispatch is wired; no scope-guarded bounded paid host path",
        )


if __name__ == "__main__":
    unittest.main()
