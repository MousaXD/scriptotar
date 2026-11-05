from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from scriptotar_sidecar.recovery import mark_media_ready, prepare_partial, write_private_json


class RecoveryIntegrityTests(unittest.TestCase):
    def make_job(self, root: Path, *, copy_source: bool = True) -> dict:
        source = root / "source.mp4"
        source.write_bytes(b"original-media")
        return {
            "id": "integrity-job",
            "input_type": "file",
            "source": str(source),
            "output_root": str(root / "out"),
            "quality": "720p",
            "cookies": "none",
            "copy_source": copy_source,
        }

    def test_same_size_corruption_invalidates_checkpoint(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            job = self.make_job(root)
            first = prepare_partial(job)
            media = first.partial_dir / "video.mp4"
            media.write_bytes(b"complete-media")
            mark_media_ready(job, first.partial_dir, media, {"title": "fixture"})
            media.write_bytes(b"tampered-media")
            self.assertEqual(len(b"complete-media"), len(b"tampered-media"))

            second = prepare_partial(job)
            self.assertIsNone(second.reused_media)
            self.assertFalse(media.exists())
            self.assertEqual(second.restarted_stage, "preparing")

    def test_checkpoint_cannot_reuse_file_outside_its_partial_dir(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            job = self.make_job(root)
            first = prepare_partial(job)
            media = first.partial_dir / "video.mp4"
            media.write_bytes(b"correct-media")
            mark_media_ready(job, first.partial_dir, media, {"title": "fixture"})

            outside = root / "outside.mp4"
            outside.write_bytes(b"outside-media")
            checkpoint_file = first.partial_dir / "checkpoint.json"
            checkpoint = json.loads(checkpoint_file.read_text(encoding="utf-8"))
            checkpoint["media"] = str(outside)
            checkpoint["media_size"] = outside.stat().st_size
            checkpoint["media_sha256"] = hashlib.sha256(outside.read_bytes()).hexdigest()
            write_private_json(checkpoint_file, checkpoint)

            second = prepare_partial(job)
            self.assertIsNone(second.reused_media)
            self.assertTrue(outside.exists())

    def test_legacy_checkpoint_without_integrity_fields_is_not_trusted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            job = self.make_job(root)
            first = prepare_partial(job)
            media = first.partial_dir / "video.mp4"
            media.write_bytes(b"legacy-media")
            mark_media_ready(job, first.partial_dir, media, {"title": "fixture"})
            checkpoint_file = first.partial_dir / "checkpoint.json"
            checkpoint = json.loads(checkpoint_file.read_text(encoding="utf-8"))
            del checkpoint["media_size"]
            del checkpoint["media_sha256"]
            write_private_json(checkpoint_file, checkpoint)

            self.assertIsNone(prepare_partial(job).reused_media)

    def test_copy_free_local_checkpoint_reuses_only_original_source(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            job = self.make_job(root, copy_source=False)
            first = prepare_partial(job)
            source = Path(job["source"])
            mark_media_ready(job, first.partial_dir, source, {"title": "fixture"})

            second = prepare_partial(job)
            self.assertEqual(second.reused_media, source)

            outside = root / "other.mp4"
            outside.write_bytes(source.read_bytes())
            checkpoint_file = first.partial_dir / "checkpoint.json"
            checkpoint = json.loads(checkpoint_file.read_text(encoding="utf-8"))
            checkpoint["media"] = str(outside)
            write_private_json(checkpoint_file, checkpoint)
            self.assertIsNone(prepare_partial(job).reused_media)


if __name__ == "__main__":
    unittest.main()
