from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from scripts.prepare_mcp_tools import extract_exact_binary, sha, validate


class TunnelBundleTests(unittest.TestCase):
    def test_offline_binary_manifest_rejects_tampering(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'legal').mkdir()
            (root / 'legal/cloudflared-LICENSE').write_text('fixture license')
            (root / 'tunnel-client').write_bytes(b'fixture-tunnel-client')
            (root / 'cloudflared').write_bytes(b'fixture-cloudflared')
            for binary in ('tunnel-client', 'cloudflared'):
                (root / binary).chmod(0o755)
            manifest = {
                'platform': 'linux',
                'files': {name: sha((root / name).read_bytes()) for name in ('tunnel-client','cloudflared')},
            }
            (root / 'MCP-TOOLS-PROVENANCE.json').write_text(json.dumps(manifest))
            validate(root)
            (root / 'cloudflared').write_bytes(b'tampered')
            with self.assertRaisesRegex(RuntimeError,'SHA-256'):
                validate(root)

    def test_archive_extraction_has_an_exact_filename(self):
        import io, zipfile
        content = io.BytesIO()
        with zipfile.ZipFile(content,'w') as zipped:
            zipped.writestr('bin/tunnel-client',b'expected executable')
            zipped.writestr('../wrong',b'wrong')
        self.assertEqual(extract_exact_binary(content.getvalue(),'tunnel-client'), b'expected executable')
        with self.assertRaises(RuntimeError):
            extract_exact_binary(content.getvalue(),'cloudflared')


if __name__ == '__main__':
    unittest.main()
