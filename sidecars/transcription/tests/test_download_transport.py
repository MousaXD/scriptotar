from __future__ import annotations

import contextlib
import functools
import importlib.util
import io
import tempfile
import threading
import unittest
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from unittest.mock import patch

from scriptotar_sidecar import engine


@unittest.skipUnless(importlib.util.find_spec("yt_dlp"), "requires pinned engine dependencies")
class DownloadTransportTests(unittest.TestCase):
    def test_real_ytdlp_download_emits_json_hook_progress_without_console_output(self):
        # Exercise the real yt-dlp HTTP downloader without depending on social
        # platform availability. These bytes test transport, not media decoding.
        class Handler(SimpleHTTPRequestHandler):
            def log_message(self, *_args):
                pass

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            payload = b"transport fixture" * 16384
            (root / "source.mp4").write_bytes(payload)
            output = root / "output"
            output.mkdir()
            server = ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=tmp))
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                job = {
                    "source": f"http://127.0.0.1:{server.server_port}/source.mp4",
                    "quality": "720p", "cookies": "none", "max_duration_seconds": 30,
                }
                console = io.StringIO()
                events = []
                with (
                    patch.object(engine, "validate_supported_url", side_effect=lambda url, **_kw: url),
                    patch.object(engine, "probe_duration", return_value=1.0),
                    contextlib.redirect_stdout(console),
                ):
                    media, _metadata = engine.download_url(job, output, lambda kind, data: events.append((kind, data)))
                self.assertEqual(media.read_bytes(), payload)
                self.assertEqual(console.getvalue(), "")
                self.assertTrue(any(data.get("percent", 0) > 0 for kind, data in events if kind == "progress"))
            finally:
                server.shutdown()
                server.server_close()
                thread.join(timeout=2)
