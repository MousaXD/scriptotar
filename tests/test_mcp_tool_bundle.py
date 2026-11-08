from __future__ import annotations

import json
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import call, patch
from urllib.error import HTTPError, URLError

from scripts.prepare_mcp_tools import extract_exact_binary, retrieve, retrieve_release_asset, sha, validate


class TunnelBundleTests(unittest.TestCase):
    def test_gateway_timeout_retries_same_source_and_verifies_bytes(self):
        url = 'https://example.test/pinned.zip'
        payload = b'reviewed archive'
        with (
            patch('scripts.prepare_mcp_tools.urlopen', side_effect=[
                HTTPError(url, 504, 'Gateway Time-out', {}, None), io.BytesIO(payload),
            ]) as request,
            patch('scripts.prepare_mcp_tools.time.sleep') as sleep,
        ):
            self.assertEqual(retrieve(url, sha(payload)), payload)
            self.assertEqual([item.args[0].full_url for item in request.call_args_list], [url, url])
            sleep.assert_called_once_with(1)

    def test_transport_retry_exhaustion_remains_failure(self):
        with (
            patch('scripts.prepare_mcp_tools.urlopen', side_effect=URLError('unavailable')) as request,
            patch('scripts.prepare_mcp_tools.time.sleep') as sleep,
        ):
            with self.assertRaises(URLError):
                retrieve('https://example.test/pinned.zip', sha(b'reviewed'))
            self.assertEqual(request.call_count, 3)
            self.assertEqual(sleep.call_args_list, [call(1), call(2)])

    def test_release_api_fallback_preserves_asset_identity_and_checksum(self):
        url = 'https://github.com/owner/project/releases/download/v1/pinned.zip'
        api = 'https://api.github.com/repos/owner/project/releases/assets/123'
        payload = b'reviewed archive'
        metadata = json.dumps({'assets': [{
            'id': 123, 'name': 'pinned.zip', 'browser_download_url': url,
        }]}).encode()
        with (
            patch('scripts.prepare_mcp_tools.urlopen', side_effect=[
                HTTPError(url, 504, 'Gateway Time-out', {}, None) for _ in range(3)
            ] + [io.BytesIO(metadata), io.BytesIO(payload)]) as request,
            patch('scripts.prepare_mcp_tools.time.sleep'),
        ):
            self.assertEqual(retrieve_release_asset('owner/project', 'v1', 'pinned.zip', sha(payload)), payload)
            self.assertEqual(request.call_args.args[0].full_url, api)
            self.assertEqual(request.call_args.args[0].get_header('Accept'), 'application/octet-stream')

    def test_api_fallback_rejects_wrong_asset_and_tampered_bytes(self):
        url = 'https://github.com/owner/project/releases/download/v1/pinned.zip'
        for download_url, payload in ((url + '-wrong', b'reviewed'), (url, b'tampered')):
            metadata = json.dumps({'assets': [{
                'id': 123, 'name': 'pinned.zip', 'browser_download_url': download_url,
            }]}).encode()
            with (
                self.subTest(download_url=download_url),
                patch('scripts.prepare_mcp_tools.urlopen', side_effect=[
                    HTTPError(url, 504, 'Gateway Time-out', {}, None) for _ in range(3)
                ] + [io.BytesIO(metadata), io.BytesIO(payload)]),
                patch('scripts.prepare_mcp_tools.time.sleep'),
            ):
                with self.assertRaises(RuntimeError):
                    retrieve_release_asset('owner/project', 'v1', 'pinned.zip', sha(b'reviewed'))

    def test_permanent_errors_and_invalid_bytes_do_not_retry(self):
        url = 'https://example.test/pinned.zip'
        for response, expected, limit, error in (
            (HTTPError(url, 404, 'Not Found', {}, None), None, 20, HTTPError),
            (io.BytesIO(b'tampered'), sha(b'reviewed'), 20, RuntimeError),
            (io.BytesIO(b'oversized'), None, 2, RuntimeError),
        ):
            with (
                self.subTest(error=error, expected=expected, limit=limit),
                patch('scripts.prepare_mcp_tools.urlopen', side_effect=[response]) as request,
                patch('scripts.prepare_mcp_tools.time.sleep') as sleep,
            ):
                with self.assertRaises(error):
                    retrieve(url, expected, limit)
                request.assert_called_once()
                sleep.assert_not_called()

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
