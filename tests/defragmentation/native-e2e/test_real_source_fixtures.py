"""Deterministic SOURCE-ONLY tests; never evidence of installed transcription."""
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import real_source_fixtures as fixtures


class RealMediaFixtureContractTests(unittest.TestCase):
    def test_public_sources_are_distinct_and_use_exact_commons_binaries(self):
        self.assertEqual(len(fixtures.SOURCES), 2)
        self.assertEqual(len({s["original_url"] for s in fixtures.SOURCES}), 2)
        self.assertEqual(len({s["narrator"] for s in fixtures.SOURCES}), 2)
        self.assertTrue(all(s["page"].startswith(
            "https://commons.wikimedia.org/wiki/File:"
        ) for s in fixtures.SOURCES))
        self.assertTrue(all(s["original_url"].startswith(
            "https://upload.wikimedia.org/wikipedia/commons/"
        ) for s in fixtures.SOURCES))

    def test_duplicate_audio_hashes_are_never_two_sources(self):
        original = [
            {"source_id": "first", "original_url": "one",
             "original_sha256": "a" * 64, "webm_sha256": "b" * 64},
            {"source_id": "second", "original_url": "two",
             "original_sha256": "c" * 64, "webm_sha256": "d" * 64},
        ]
        fixtures.validate_distinct(original)
        for key in ("source_id", "original_url", "original_sha256", "webm_sha256"):
            copied = [dict(record) for record in original]
            copied[1][key] = copied[0][key]
            with self.subTest(key=key), self.assertRaises(ValueError):
                fixtures.validate_distinct(copied)
        with self.assertRaises(ValueError):
            fixtures.validate_distinct([original[0]])

    def test_media_is_not_downloaded_implicitly_or_via_redirect(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "source.ogg"
            with mock.patch.object(fixtures.urllib.request, "build_opener") as opener:
                with self.assertRaises(ValueError):
                    fixtures.download_original("http://127.0.0.1/audio.ogg", target)
                opener.assert_not_called()
            self.assertFalse(target.exists())
        self.assertIsNone(fixtures.RejectRedirects().redirect_request(
            None, None, 302, "redirect", {}, "https://untrusted.example/"
        ))

    def test_manifest_never_certifies_native_transcripts_or_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            fake_duration = 141.0
            def fake_download(url, path):
                path.write_bytes(b"OggS" + url.encode("utf-8"))
            def fake_run(cmd, **_kwargs):
                Path(cmd[-1]).write_bytes(b"webm" + str(cmd[cmd.index("-i") + 1]).encode())
                return None
            with mock.patch.object(fixtures, "download_original", side_effect=fake_download), \
                 mock.patch.object(fixtures, "probe_audio", return_value=fake_duration), \
                 mock.patch.object(fixtures.subprocess, "run", side_effect=fake_run):
                # Source-only synthetic bytes check provenance bookkeeping,
                # NOT real decoding, native transcription or full media.
                with self.assertRaises(ValueError):
                    fixtures.prepare(output)
                self.assertFalse((output / "origin-manifest.json").exists())

    def test_existing_output_is_rejected_before_any_network(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "old.txt").write_text("unrelated previous run")
            with mock.patch.object(fixtures, "download_original") as download:
                with self.assertRaisesRegex(ValueError, "NEW empty"):
                    fixtures.prepare(root)
                download.assert_not_called()


if __name__ == "__main__":
    unittest.main()
