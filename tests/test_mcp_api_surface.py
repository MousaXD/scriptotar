"""Ensure every MCP tool has a handler, catalog schema and executable E2E coverage."""
from __future__ import annotations

import re
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
MCP = (ROOT / 'crates/scriptotar-mcp/src/server.rs').read_text()
STDIO = (ROOT / 'tests/mcp_stdio_e2e.py').read_text()
DESKTOP = (ROOT / 'tests/mcp_desktop_cli_e2e.py').read_text()
DOCS = (ROOT / 'docs/mcp/README.md').read_text()


class McpApiCoverageTests(unittest.TestCase):
    def test_tool_catalog_matches_handlers_and_tests_for_all_fourteen(self):
        catalog = set(re.findall(r'\"name\":\"([a-z_]+)\"', MCP[MCP.index('fn tool_catalog()'):]))
        handled = set(re.findall(r'^\s*\"([a-z_]+)\"\s*=>\s*\{', MCP, flags=re.M))
        invoked = set(re.findall(r"\bcall\(['\"]([a-z_]+)['\"]", STDIO + DESKTOP))
        self.assertEqual(len(catalog), 14, 'Tool count changed; update the certification inventory')
        self.assertEqual(catalog, handled, 'Some published MCP tools have no handlers or vice versa')
        self.assertEqual(invoked - catalog, {'change_permissions'}, 'Only the intentionally rejected permission-mutation tool may be unknown')
        self.assertEqual(catalog, invoked - {'change_permissions'}, 'Every MCP tool must be invoked by a compiled-process E2E')
        for tool in sorted(catalog):
            self.assertIn(f'`{tool}`', DOCS, f'Missing MCP permissions documentation: {tool}')

    def test_transcription_protocol_types_are_exercised(self):
        protocol = (ROOT / 'sidecars/transcription/scriptotar_sidecar/protocol.py').read_text()
        source = (ROOT / 'sidecars/transcription/tests/test_subprocess.py').read_text()
        operation_set = set(re.findall(r'\"(ping|transcribe|cancel|shutdown)\"',
                                       protocol.split('KNOWN_TYPES = ', 1)[1].split('\n', 1)[0]))
        self.assertEqual(operation_set, {'ping', 'transcribe', 'cancel', 'shutdown'})
        for operation in operation_set:
            self.assertIn(f'"{operation}"', source, f'Sidecar protocol needs an integration test for {operation}')


if __name__ == '__main__':
    unittest.main()
