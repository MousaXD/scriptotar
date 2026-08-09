import tempfile
import unittest
from pathlib import Path

from core import (
    captions_from_words,
    clean_transcript,
    human_time,
    quality_format,
    quality_sort,
    safe_name,
    srt_time,
    validate_supported_url,
    write_srt,
    write_vtt,
)


class CoreTests(unittest.TestCase):
    def test_safe_name_keeps_arabic(self):
        self.assertIn("مرحبا", safe_name("مرحبا !!! test"))

    def test_times(self):
        self.assertEqual(human_time(65), "01:05")
        self.assertEqual(human_time(3661), "01:01:01")
        self.assertEqual(srt_time(1.234), "00:00:01,234")

    def test_url_allowlist(self):
        self.assertTrue(validate_supported_url("https://www.instagram.com/reel/abc/"))
        self.assertTrue(validate_supported_url("https://youtu.be/abc"))
        with self.assertRaises(ValueError):
            validate_supported_url("https://example.com/video")

    def test_quality(self):
        self.assertEqual(quality_format("720p")[1], "mp4")
        self.assertEqual(quality_sort("720p"), ["res:720", "fps"])
        self.assertEqual(quality_sort("1080p"), ["res:1080", "fps"])
        self.assertIsNone(quality_format("Audio only")[1])

    def test_clean_removes_adjacent_duplicate(self):
        self.assertEqual(clean_transcript([" hello  world ", "hello world", "next"]), "hello world\nnext")

    def test_caption_grouping_and_exports(self):
        words = [
            {"start": 0.0, "end": 0.5, "word": " Hello"},
            {"start": 0.5, "end": 1.0, "word": " world."},
            {"start": 1.2, "end": 1.6, "word": " Next"},
        ]
        captions = captions_from_words(words, max_chars=20)
        self.assertGreaterEqual(len(captions), 1)
        self.assertIn("Hello", write_srt(captions))
        self.assertTrue(write_vtt(captions).startswith("WEBVTT"))


if __name__ == "__main__":
    unittest.main()
