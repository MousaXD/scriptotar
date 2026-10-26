"""Release blocker: every registered desktop IPC command and public UI API is covered.

A new Tauri command or frontend API will fail this test until the binding and
contract invocation test are updated; this prevents untested 'hidden' features.
"""
from __future__ import annotations

import re
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
RUST = (ROOT / 'apps/desktop/src-tauri/src/lib.rs').read_text()
INTERFACE = (ROOT / 'apps/desktop-ui/src/api/client.ts').read_text()
TAURI = (ROOT / 'apps/desktop-ui/src/api/tauriClient.ts').read_text()
MOCK = (ROOT / 'apps/desktop-ui/src/api/mockClient.ts').read_text()
TEST = (ROOT / 'apps/desktop-ui/src/api/tauriClient.test.ts').read_text()


def actual_commands() -> set[str]:
    return set(re.findall(r'#\[tauri::command\]\s*fn\s+(\w+)\s*\(', RUST))


def dispatched_commands() -> set[str]:
    block = re.search(r'\.invoke_handler\(tauri::generate_handler!\[(.*?)\]\)', RUST, re.S)
    if not block:
        raise AssertionError('Missing Tauri command registration')
    return set(re.findall(r'\b[a-z][a-z0-9_]+\b', block.group(1)))


def wired_commands() -> set[str]:
    return set(re.findall(r'\binvoke(?:<[^>]*>)?\(\s*[\'\"]([a-z0-9_]+)[\'\"]', TAURI))


def methods() -> set[str]:
    api = re.search(r'export interface ScriptotarApi \{(.*?)\n\}', INTERFACE, re.S)
    if not api:
        raise AssertionError('Cannot locate ScriptotarApi')
    return set(re.findall(r'^\s+(\w+)\s*\(', api.group(1), re.M))


class ApiSurfaceContractTests(unittest.TestCase):
    def test_every_rust_command_is_dispatched(self):
        self.assertEqual(actual_commands(), dispatched_commands())
        self.assertEqual(len(actual_commands()), 34, "Update coverage inventory when Tauri commands change")

    def test_every_rust_command_is_accessible_via_frontend_client(self):
        self.assertEqual(dispatched_commands(), wired_commands())

    def test_every_frontend_method_has_an_implementation_and_contract_invocation(self):
        names = methods()
        self.assertEqual(len(names), 35, "Update coverage inventory when frontend APIs change")
        # The exact invoked command mapping is covered above.
        for name in names:
            self.assertRegex(TAURI, rf'(?m)^\s+{name}:', f'Missing frontend binding: {name}')
            self.assertRegex(MOCK, rf'(?m)^\s+async\s+{name}\s*\(', f'Missing mock: {name}')
            self.assertRegex(TEST, rf'await\s+api\.{name}\s*\(', f'Missing invocation-contract test: {name}')
        self.assertEqual(len(names), len(set(names)))


if __name__ == '__main__':
    unittest.main()
