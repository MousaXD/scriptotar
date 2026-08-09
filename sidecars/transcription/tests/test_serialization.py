from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scriptotar_sidecar.engine import write_outputs
from scriptotar_sidecar.version import SIDECAR_VERSION


class SerializationTests(unittest.TestCase):
    def test_transcript_outputs_are_structured_and_versioned(self):
        with tempfile.TemporaryDirectory() as tmp:
            partial = Path(tmp)
            job = {"model": "small"}
            metadata = {
                "title": "fixture",
                "uploader": "creator",
                "source_url": "https://youtube.com/watch?v=x",
                "duration_seconds": 1.0,
                "extractor": "Youtube",
            }
            result = {
                "text": "Hello world",
                "clean_text": "Hello world",
                "segments": [
                    {
                        "index": 1,
                        "start": 0.0,
                        "end": 1.0,
                        "text": "Hello world",
                        "words": [
                            {"start": 0.0, "end": 0.4, "word": " Hello", "probability": 0.99},
                            {"start": 0.4, "end": 1.0, "word": " world", "probability": 0.98},
                        ],
                    }
                ],
                "words": [
                    {"start": 0.0, "end": 0.4, "word": " Hello", "probability": 0.99},
                    {"start": 0.4, "end": 1.0, "word": " world", "probability": 0.98},
                ],
                "captions": [{"start": 0.0, "end": 1.0, "text": "Hello world"}],
                "language": "en",
                "language_probability": 0.99,
                "device": "cpu",
                "compute_type": "int8",
                "task": "transcribe",
            }
            artifacts = write_outputs(job, partial, metadata, result)
            self.assertEqual(
                set(artifacts),
                {"text", "clean_text", "timestamp_text", "srt", "vtt", "json"},
            )
            payload = json.loads((partial / "transcript.json").read_text(encoding="utf-8"))
            self.assertEqual(payload["sidecar_version"], SIDECAR_VERSION)
            self.assertEqual(payload["segments"][0]["words"][0]["word"], " Hello")
            self.assertTrue((partial / "transcript.srt").read_text(encoding="utf-8").startswith("1\n00:00:00,000"))
            self.assertTrue((partial / "transcript.vtt").read_text(encoding="utf-8").startswith("WEBVTT"))
