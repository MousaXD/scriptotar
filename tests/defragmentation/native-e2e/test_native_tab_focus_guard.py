"""Synthetic source regressions for native keyboard target checks.

These tests mock only the OS input transport. They NEVER certify an installed
app or produce a PASS for the 16 mandatory native acceptance cases.
"""
import subprocess
import unittest
from types import SimpleNamespace
from unittest.mock import patch

import linux_webkit


class FakeNode:
    name = "الاستوديو"

    def getState(self):
        return self

    def contains(self, value):
        return True

    def getRoleName(self):
        return "push button"

    def queryComponent(self):
        return self

    def getExtents(self, coordinate_system):
        return SimpleNamespace(x=20, y=30, width=100, height=40)


class FakeInstalledAx:
    pid = 311
    atspi = SimpleNamespace(STATE_FOCUSED=1, STATE_FOCUSABLE=2, DESKTOP_COORDS=3)

    def web_document(self):
        return object()

    def nodes(self):
        return [FakeNode()]


class KeyboardTransport:
    def __init__(self, focus_after_tabs=0, active_after_tabs=None,
                 refuse_focus_request=False):
        self.focus_after_tabs = focus_after_tabs
        self.active_after_tabs = (focus_after_tabs if active_after_tabs is None
                                  else active_after_tabs)
        self.refuse_focus_request = refuse_focus_request
        self.tabs = 0
        self.commands = []

    def __call__(self, *args, check=True):
        self.commands.append(args)
        action = args[1] if len(args) > 1 else ""
        if action == "search":
            return subprocess.CompletedProcess(args, 0, "88\n", "")
        if action == "getwindowname":
            return subprocess.CompletedProcess(args, 0, "Scriptotar", "")
        if action == "windowactivate":
            return subprocess.CompletedProcess(args, 0, "", "")
        if action == "windowfocus":
            return subprocess.CompletedProcess(
                args, 1 if self.refuse_focus_request else 0, "", ""
            )
        if action in ("getactivewindow", "getwindowfocus"):
            limit = (self.active_after_tabs if action == "getactivewindow"
                     else self.focus_after_tabs)
            owned = self.tabs < limit
            return subprocess.CompletedProcess(args, 0, "88\n" if owned else "999\n", "")
        if action == "key":
            if args[-1] != "Tab":
                raise AssertionError("Unexpected keyboard input")
            self.tabs += 1
            return subprocess.CompletedProcess(args, 0, "", "")
        raise AssertionError(f"Unexpected native command {args!r}")


class NativeTabInputGuardTests(unittest.TestCase):
    def run_probe(self, transport, count=3):
        report = {}
        with (patch.object(linux_webkit, "command", side_effect=transport),
              patch.object(linux_webkit.time, "sleep", return_value=None),
              patch.object(linux_webkit.time, "monotonic",
                           side_effect=iter([0, 4, 5, 6, 7, 8, 9, 10, 11, 12]))):
            linux_webkit.capture_native_tab_order(
                report, FakeInstalledAx(), "arabic_reopened_native_tab_order", count
            )
        return report

    def test_foreign_keyboard_focus_denies_all_real_tabs(self):
        transport = KeyboardTransport(focus_after_tabs=0)
        report = self.run_probe(transport)
        self.assertEqual(transport.tabs, 0)
        self.assertIn("lacks keyboard focus",
                      report["native_layout_observation_errors"][
                          "arabic_reopened_native_tab_order"])
        self.assertFalse(report["native_layout_observations"][
            "arabic_reopened_native_tab_order"]["certifies_rtl_or_visual"])

    def test_active_window_alone_cannot_authorize_keyboard_input(self):
        transport = KeyboardTransport(focus_after_tabs=0, active_after_tabs=100)
        report = self.run_probe(transport)
        self.assertEqual(transport.tabs, 0)
        self.assertIn("lacks keyboard focus",
                      report["native_layout_observation_errors"][
                          "arabic_reopened_native_tab_order"])
        self.assertFalse(report["native_layout_observations"][
            "arabic_reopened_native_tab_order"]["certifies_rtl_or_visual"])

    def test_keyboard_focus_alone_cannot_authorize_keyboard_input(self):
        transport = KeyboardTransport(focus_after_tabs=100, active_after_tabs=0)
        report = self.run_probe(transport)
        self.assertEqual(transport.tabs, 0)
        self.assertIn("lacks keyboard focus",
                      report["native_layout_observation_errors"][
                          "arabic_reopened_native_tab_order"])

    def test_refuse_failed_windowfocus_request(self):
        transport = KeyboardTransport(focus_after_tabs=100, refuse_focus_request=True)
        report = self.run_probe(transport)
        self.assertEqual(transport.tabs, 0)
        self.assertIn("Cannot request installed X11 keyboard focus",
                      report["native_layout_observation_errors"][
                          "arabic_reopened_native_tab_order"])

    def test_focus_theft_stops_following_tabs(self):
        transport = KeyboardTransport(focus_after_tabs=1)
        report = self.run_probe(transport)
        self.assertEqual(transport.tabs, 1)
        self.assertIn("lost focus", report["native_layout_observation_errors"][
            "arabic_reopened_native_tab_order"])
        self.assertEqual(len(report["native_layout_observations"][
            "arabic_reopened_native_tab_order"]["events"]), 1)

    def test_installed_owned_focus_is_observation_not_certification(self):
        transport = KeyboardTransport(focus_after_tabs=100)
        report = self.run_probe(transport)
        self.assertEqual(transport.tabs, 3)
        self.assertNotIn("native_layout_observation_errors", report)
        evidence = report["native_layout_observations"][
            "arabic_reopened_native_tab_order"]
        self.assertEqual(evidence["installed_x11_window"], "88")
        self.assertEqual(len(evidence["events"]), 3)
        self.assertTrue(all(e["focused_count"] == 1 for e in evidence["events"]))
        self.assertFalse(evidence["certifies_rtl_or_visual"])


if __name__ == "__main__":
    unittest.main()
