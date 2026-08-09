from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scriptotar_sidecar.recovery import mark_media_ready, prepare_partial, write_private_json


class RecoveryTests(unittest.TestCase):
    def _job(self, root: Path, source: Path) -> dict:
        return {
            "id": "recover-job",
            "input_type": "file",
            "source": str(source),
            "output_root": str(root / "out"),
            "quality": "720p",
            "cookies": "none",
            "copy_source": True,
        }

    def test_interrupted_media_ready_is_reused_but_transcript_is_restarted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "input.mp4"
            source.write_bytes(b"source")
            job = self._job(root, source)
            first = prepare_partial(job)
            media = first.partial_dir / "video.mp4"
            media.write_bytes(b"complete media")
            metadata = {"title": "fixture", "duration_seconds": 2.0}
            mark_media_ready(job, first.partial_dir, media, metadata)
            (first.partial_dir / "transcript.txt").write_text("partial transcript", encoding="utf-8")

            second = prepare_partial(job)
            self.assertEqual(second.reused_media, media)
            self.assertEqual(second.restarted_stage, "transcribing")
            self.assertFalse((first.partial_dir / "transcript.txt").exists())

    def test_unproven_partial_is_discarded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            source = root / "input.mp4"
            source.write_bytes(b"source")
            job = self._job(root, source)
            first = prepare_partial(job)
            junk = first.partial_dir / "video.part"
            junk.write_bytes(b"incomplete")
            second = prepare_partial(job)
            self.assertIsNone(second.reused_media)
            self.assertFalse(junk.exists())
            checkpoint = json.loads((second.partial_dir / "checkpoint.json").read_text(encoding="utf-8"))
            self.assertEqual(checkpoint["stage"], "preparing")

    def test_private_json_is_not_world_readable_on_posix(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "private.json"
            write_private_json(path, {"ok": True})
            self.assertEqual(path.stat().st_mode & 0o077, 0)
