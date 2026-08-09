import json
import unittest

from creator import (
    build_prompt,
    estimate_speaking_seconds,
    format_duration,
    normalize_research_item,
    research_scan_command,
)


class CreatorTests(unittest.TestCase):
    def test_prompt_only_mode_produces_self_contained_originality_rules(self):
        prompt = build_prompt(
            "Structure remix",
            "This is the source creator transcript.",
            topic="A new coffee grinder",
            audience="home baristas",
            duration="30 seconds",
            cta="visit the site",
            voice="dry and concise",
        )
        self.assertIn("Structure remix", "Structure remix")
        self.assertIn("abstract structure", prompt)
        self.assertIn("Do not reproduce distinctive wording", prompt)
        self.assertIn("A new coffee grinder", prompt)
        self.assertIn("home baristas", prompt)
        self.assertIn("This is the source creator transcript.", prompt)

    def test_speaking_timer(self):
        text = " ".join(["word"] * 25)
        self.assertEqual(estimate_speaking_seconds(text, 2.5), 10)
        self.assertEqual(format_duration(10), "0:10")
        self.assertEqual(format_duration(65), "1:05")

    def test_research_scan_command_clamps_limit_and_preserves_cookies(self):
        cmd = research_scan_command("/venv/bin/python", "https://www.youtube.com/@creator", 999, "firefox")
        self.assertEqual(cmd[0], "/venv/bin/python")
        self.assertIn("yt_dlp", cmd)
        self.assertIn("200", cmd)
        self.assertIn("--cookies-from-browser", cmd)
        self.assertEqual(cmd[-2:], ["--", "https://www.youtube.com/@creator"])

    def test_normalize_research_item(self):
        raw = {
            "id": "abc",
            "extractor_key": "Youtube",
            "title": "Example",
            "view_count": 1000,
            "like_count": 100,
            "comment_count": 20,
            "upload_date": "20260809",
            "duration": 31,
        }
        item = normalize_research_item(raw, "https://www.youtube.com/@creator")
        self.assertEqual(item["source_url"], "https://www.youtube.com/watch?v=abc")
        self.assertEqual(item["platform"], "YouTube")
        self.assertAlmostEqual(item["engagement_rate"], 12.0)
        self.assertEqual(item["published_at"], "2026-08-09")
        self.assertEqual(json.loads(item["raw_json"])["id"], "abc")


if __name__ == "__main__":
    unittest.main()

class ProviderAdapterTests(unittest.TestCase):
    def test_openai_response_parser(self):
        import creator
        from unittest.mock import patch
        payload = {"output": [{"content": [{"type": "output_text", "text": "hello"}]}]}
        with patch.object(creator, "_post_json", return_value=payload) as post:
            result = creator.request_ai("OpenAI", "gpt-test", "secret", "prompt")
        self.assertEqual(result, "hello")
        args = post.call_args.args
        self.assertEqual(args[0], "https://api.openai.com/v1/responses")
        self.assertEqual(args[1]["Authorization"], "Bearer secret")
        self.assertEqual(args[2]["input"], "prompt")

    def test_anthropic_response_parser(self):
        import creator
        from unittest.mock import patch
        payload = {"content": [{"type": "text", "text": "claude result"}]}
        with patch.object(creator, "_post_json", return_value=payload) as post:
            result = creator.request_ai("Anthropic", "claude-sonnet-5", "secret", "prompt")
        self.assertEqual(result, "claude result")
        self.assertEqual(post.call_args.args[0], "https://api.anthropic.com/v1/messages")

    def test_gemini_response_parser(self):
        import creator
        from unittest.mock import patch
        payload = {"candidates": [{"content": {"parts": [{"text": "gemini result"}]}}]}
        with patch.object(creator, "_post_json", return_value=payload) as post:
            result = creator.request_ai("Gemini", "gemini-3.6-flash", "secret", "prompt")
        self.assertEqual(result, "gemini result")
        self.assertIn("gemini-3.6-flash:generateContent", post.call_args.args[0])

    def test_custom_provider_requires_http_url(self):
        import creator
        with self.assertRaises(ValueError):
            creator.request_ai("OpenAI-compatible", "model", "secret", "prompt", base_url="file:///tmp")
