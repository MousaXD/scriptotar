from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scriptotar_sidecar.errors import SidecarError
from scriptotar_sidecar.validation import (
    validate_job_payload,
    validate_local_input,
    validate_supported_url,
)
from scriptotar_sidecar.version import PROTOCOL_VERSION


class ValidationTests(unittest.TestCase):
    def test_supported_url(self):
        self.assertEqual(
            validate_supported_url("https://www.youtube.com/watch?v=abc"),
            "https://www.youtube.com/watch?v=abc",
        )

    def test_rejects_lookalike_domain(self):
        with self.assertRaises(SidecarError) as raised:
            validate_supported_url("https://youtube.com.evil.example/video")
        self.assertEqual(raised.exception.code, "UNSUPPORTED_DOMAIN")

    def test_rejects_malformed_port(self):
        with self.assertRaises(SidecarError) as raised:
            validate_supported_url("https://youtube.com:notaport/watch?v=abc")
        self.assertEqual(raised.exception.code, "INVALID_URL")

    def test_rejects_embedded_credentials(self):
        with self.assertRaises(SidecarError):
            validate_supported_url("https://user:pass@youtube.com/watch?v=abc")

    def test_rejects_plaintext_cookie_use(self):
        with self.assertRaises(SidecarError) as raised:
            validate_supported_url("http://youtube.com/watch?v=abc", cookies_browser="firefox")
        self.assertEqual(raised.exception.code, "INSECURE_COOKIE_URL")

    def test_local_input_extension(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "clip.exe"
            path.write_bytes(b"not media")
            with self.assertRaises(SidecarError) as raised:
                validate_local_input(str(path))
            self.assertEqual(raised.exception.code, "UNSUPPORTED_FILE_TYPE")

    def test_transcribe_payload_normalizes_options(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            clip = root / "clip.mp4"
            clip.write_bytes(b"fixture")
            command = {
                "protocol": PROTOCOL_VERSION,
                "type": "transcribe",
                "job_id": "job-1",
                "input": {"kind": "file", "value": str(clip)},
                "output": {"root": str(root / "out")},
                "options": {"model": "small", "language": "ar"},
            }
            job = validate_job_payload(command)
            self.assertEqual(job["model"], "small")
            self.assertEqual(job["language"], "ar")
            self.assertEqual(job["batch_size"], 8)

    def test_rejects_unknown_model(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            clip = root / "clip.mp4"
            clip.write_bytes(b"fixture")
            command = {
                "protocol": PROTOCOL_VERSION,
                "type": "transcribe",
                "job_id": "job-1",
                "input": {"kind": "file", "value": str(clip)},
                "output": {"root": str(root / "out")},
                "options": {"model": "../../custom-model"},
            }
            with self.assertRaises(SidecarError) as raised:
                validate_job_payload(command)
            self.assertEqual(raised.exception.code, "INVALID_MODEL")
