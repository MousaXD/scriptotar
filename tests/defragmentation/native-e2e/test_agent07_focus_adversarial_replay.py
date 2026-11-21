"""Supplemental OFFLINE adverse OS-recipient simulations for exact PR196 source.

These mocks run neither an installed app nor real desktop input. A green result
is only a source-regression observation. It NEVER certifies RTL/visual cases.
"""
import subprocess
import unittest
from unittest.mock import patch

import linux_webkit
from test_native_tab_focus_guard import FakeInstalledAx, KeyboardTransport


class NoDocument(FakeInstalledAx):
    def web_document(self):
        return None


class InterferenceTransport(KeyboardTransport):
    def __init__(self, *, behavior):
        super().__init__(focus_after_tabs=100)
        self.behavior = behavior

    def __call__(self, *args, check=True):
        action = args[1] if len(args) > 1 else ""
        if self.behavior == "ambiguous" and action == "search":
            return subprocess.CompletedProcess(args, 0, "88\n99\n", "")
        if self.behavior == "activate" and action == "windowactivate":
            self.commands.append(args)
            return subprocess.CompletedProcess(args, 1, "", "")
        if self.behavior == "key" and action == "key":
            self.commands.append(args)
            self.tabs += 1
            return subprocess.CompletedProcess(args, 1, "", "")
        if self.behavior == "active_error" and action == "getactivewindow":
            self.commands.append(args)
            return subprocess.CompletedProcess(args, 1, "", "")
        if self.behavior == "focus_error" and action == "getwindowfocus":
            self.commands.append(args)
            return subprocess.CompletedProcess(args, 1, "", "")
        return super().__call__(*args, check=check)


class SupplementalNativeFocusRefusal(unittest.TestCase):
    def probe(self, transport, ax=None):
        report = {}
        with (patch.object(linux_webkit, "command", side_effect=transport),
              patch.object(linux_webkit.time, "sleep", return_value=None),
              patch.object(linux_webkit.time, "monotonic", side_effect=iter(range(0, 80, 4)))):
            linux_webkit.capture_native_tab_order(
                report, ax if ax is not None else FakeInstalledAx(), "adversarial", 3
            )
        return report

    def assert_not_certifying(self, report):
        item = report["native_layout_observations"]["adversarial"]
        self.assertIs(item["certifies_rtl_or_visual"], False)
        self.assertNotIn("cases", report)
        self.assertIn("adversarial", report["native_layout_observation_errors"])

    def test_absent_real_atspi_document_blocks_before_desktop_input(self):
        target = KeyboardTransport(focus_after_tabs=100)
        result = self.probe(target, NoDocument())
        self.assertEqual(target.tabs, 0)
        self.assert_not_certifying(result)
        self.assertIn("no AT-SPI web document", result["native_layout_observation_errors"]["adversarial"])

    def test_ambiguous_pid_owned_titled_windows_block_before_desktop_input(self):
        target = InterferenceTransport(behavior="ambiguous")
        result = self.probe(target)
        self.assertEqual(target.tabs, 0)
        self.assert_not_certifying(result)
        self.assertIn("Expected one visible", result["native_layout_observation_errors"]["adversarial"])

    def test_refused_activation_blocks_before_desktop_input(self):
        target = InterferenceTransport(behavior="activate")
        result = self.probe(target)
        self.assertEqual(target.tabs, 0)
        self.assert_not_certifying(result)
        self.assertIn("Cannot activate installed", result["native_layout_observation_errors"]["adversarial"])

    def test_unavailable_active_window_query_blocks_before_desktop_input(self):
        target = InterferenceTransport(behavior="active_error")
        result = self.probe(target)
        self.assertEqual(target.tabs, 0)
        self.assert_not_certifying(result)
        self.assertIn("lacks keyboard focus", result["native_layout_observation_errors"]["adversarial"])

    def test_unavailable_keyboard_focus_query_blocks_before_desktop_input(self):
        target = InterferenceTransport(behavior="focus_error")
        result = self.probe(target)
        self.assertEqual(target.tabs, 0)
        self.assert_not_certifying(result)
        self.assertIn("lacks keyboard focus", result["native_layout_observation_errors"]["adversarial"])

    def test_failed_real_key_delivery_is_observation_error_not_acceptance(self):
        target = InterferenceTransport(behavior="key")
        result = self.probe(target)
        self.assertEqual(target.tabs, 1)
        self.assert_not_certifying(result)
        self.assertIn("delivery failed", result["native_layout_observation_errors"]["adversarial"])
        self.assertEqual(result["native_layout_observations"]["adversarial"]["events"], [])


if __name__ == "__main__":
    unittest.main()
