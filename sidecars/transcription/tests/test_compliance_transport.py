from __future__ import annotations

import io
import urllib.error
import unittest
from unittest.mock import patch

import compliance_enrichment


class ComplianceTransportTests(unittest.TestCase):
    def test_transient_reset_retries_same_notice_url(self):
        expected = b"verified notice bytes"
        with (
            patch.object(compliance_enrichment.urllib.request, "urlopen", side_effect=[
                urllib.error.URLError(ConnectionResetError(104, "reset")),
                io.BytesIO(expected),
            ]) as urlopen,
            patch.object(compliance_enrichment.time, "sleep") as sleep,
        ):
            self.assertEqual(compliance_enrichment._download(
                "https://example.test/LICENSE",
                expected_sha256=compliance_enrichment._sha256_bytes(expected),
            ), expected)
        self.assertEqual(urlopen.call_count, 2)
        self.assertEqual(urlopen.call_args_list[0].args[0].full_url, urlopen.call_args_list[1].args[0].full_url)
        sleep.assert_called_once_with(1)

    def test_permanent_http_404_does_not_retry(self):
        url = "https://example.test/LICENSE"
        failure = urllib.error.HTTPError(url, 404, "Not Found", {}, None)
        with (
            patch.object(compliance_enrichment.urllib.request, "urlopen", side_effect=failure) as urlopen,
            patch.object(compliance_enrichment.time, "sleep") as sleep,
        ):
            with self.assertRaises(urllib.error.HTTPError) as raised:
                compliance_enrichment._download(url)
        self.assertEqual(raised.exception.code, 404)
        urlopen.assert_called_once()
        sleep.assert_not_called()

    def test_transient_http_503_retries_reviewed_notice(self):
        url = "https://example.test/LICENSE"
        expected = b"reviewed notice"
        failure = urllib.error.HTTPError(url, 503, "Service Unavailable", {}, None)
        with (
            patch.object(compliance_enrichment.urllib.request, "urlopen",
                         side_effect=[failure, io.BytesIO(expected)]) as urlopen,
            patch.object(compliance_enrichment.time, "sleep") as sleep,
        ):
            self.assertEqual(
                compliance_enrichment._download(
                    url, expected_sha256=compliance_enrichment._sha256_bytes(expected)
                ), expected
            )
        self.assertEqual(urlopen.call_count, 2)
        sleep.assert_called_once_with(1)

    def test_transport_failure_exhausts_bounded_attempts(self):
        with (
            patch.object(compliance_enrichment.urllib.request, "urlopen",
                         side_effect=urllib.error.URLError("offline")) as urlopen,
            patch.object(compliance_enrichment.time, "sleep") as sleep,
        ):
            with self.assertRaises(urllib.error.URLError):
                compliance_enrichment._download("https://example.test/LICENSE")
        self.assertEqual(urlopen.call_count, 3)
        self.assertEqual(sleep.call_count, 2)

    def test_checksum_mismatch_fails_closed_without_retry(self):
        with (
            patch.object(compliance_enrichment.urllib.request, "urlopen",
                         return_value=io.BytesIO(b"altered notice")) as urlopen,
            patch.object(compliance_enrichment.time, "sleep") as sleep,
        ):
            with self.assertRaisesRegex(RuntimeError, "checksum drift"):
                compliance_enrichment._download("https://example.test/LICENSE", expected_sha256="0" * 64)
        urlopen.assert_called_once()
        sleep.assert_not_called()

    def test_empty_notice_fails_closed(self):
        with patch.object(compliance_enrichment.urllib.request, "urlopen", return_value=io.BytesIO(b"")):
            with self.assertRaisesRegex(RuntimeError, "empty compliance source"):
                compliance_enrichment._download("https://example.test/LICENSE")


if __name__ == "__main__":
    unittest.main()
