#!/usr/bin/env python3
"""Installed Scriptotar WebKitGTK / AT-SPI acceptance (never a Vite or mock test).

This runner fails closed when native application accessibility is unavailable.
A fresh isolated SCRIPTOTAR_DATA_DIR is intentional: real media/source scenarios
need independently sourced fixtures and are NOT TESTED on an empty profile.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone


CASES = (
    "installed_webview", "create_switch_projects", "create_study",
    "edit_save_duplicate_candidate", "quit_relaunch_persistence",
    "wrong_project_ui_isolation", "wrong_project_ipc_denial",
    "multi_source_transcript", "source_revocation",
    "error_recovery", "keyboard_custom_controls", "arabic_locale_control",
    "theme_mode_control", "arabic_rtl",
    "light_dark_visual", "windows_webview2",
)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def command(*argv: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(argv, capture_output=True, text=True, check=check, timeout=15)


class NativeUnavailable(RuntimeError):
    """Instrumentation missing: NOT TESTED, never PASS."""


class NativeFailure(AssertionError):
    """Instrumented assertion failed: FAIL."""


class AxWebKit:
    """Only connects to the launched process's AT-SPI tree; no DOM mocks."""

    def __init__(self, pid: int):
        try:
            import pyatspi
        except ImportError as exc:
            raise NativeUnavailable("python3-pyatspi (system Python) unavailable") from exc
        self.atspi = pyatspi
        self.pid = pid

    def nodes(self):
        desktop = self.atspi.Registry.getDesktop(0)
        apps = []
        for app in desktop:
            try:
                name = app.name or ""
                if app.get_process_id() == self.pid and "scriptotar" in name.lower():
                    apps.append(app)
            except Exception:
                continue
        if not apps:
            raise NativeUnavailable(
                f"No AT-SPI application owned by installed process PID {self.pid}; "
                "do not treat a visible process or screenshot as WebView automation"
            )
        pending = [(app, 0) for app in apps]
        seen = 0
        while pending and seen < 3500:
            node, depth = pending.pop()
            seen += 1
            try:
                states = node.getState()
                if depth and not states.contains(self.atspi.STATE_VISIBLE):
                    continue
                yield node
                if depth < 24:
                    pending.extend((child, depth + 1) for child in node)
            except Exception:
                continue

    def find(self, name: str, role: str | None = None, timeout: float = 12):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                for node in self.nodes():
                    try:
                        label = (node.name or "").strip()
                        node_role = node.getRoleName().lower()
                        if label == name and (role is None or role in node_role):
                            return node
                    except Exception:
                        continue
            except NativeUnavailable:
                pass
            time.sleep(0.4)
        raise NativeFailure(f"AT-SPI control not found: role={role!r}, name={name!r}")

    def available(self, name: str, role: str | None = None) -> bool:
        try:
            self.find(name, role=role, timeout=1)
            return True
        except (NativeFailure, NativeUnavailable):
            return False

    def web_document(self) -> bool:
        for node in self.nodes():
            try:
                role = node.getRoleName().lower()
                if ("document" in role and ("web" in role or "frame" in role)) or role == "document web":
                    return True
            except Exception:
                pass
        return False

    def click(self, name: str, role: str | None = None) -> None:
        node = self.find(name, role)
        try:
            actions = node.queryAction()
            for i in range(actions.nActions):
                if actions.getName(i).lower() in ("click", "press", "activate"):
                    if actions.doAction(i):
                        time.sleep(0.6)
                        return
        except Exception:
            pass
        try:
            if node.queryComponent().grabFocus():
                command("xdotool", "key", "--clearmodifiers", "Return")
                time.sleep(0.6)
                return
        except Exception:
            pass
        raise NativeFailure(f"Cannot activate accessible control {name!r}")

    def type_into(self, name: str, value: str) -> None:
        node = self.find(name)
        try:
            focused = node.queryComponent().grabFocus()
            if focused is False:
                raise NativeFailure(f"Cannot focus field {name!r}")
            command("xdotool", "key", "--clearmodifiers", "ctrl+a")
            command("xdotool", "type", "--clearmodifiers", "--delay", "2", "--", value)
            time.sleep(0.65)
        except NativeFailure:
            raise
        except Exception as exc:
            raise NativeFailure(f"Cannot input into field {name!r}: {exc}") from exc

    def read_text(self, name: str) -> str:
        node = self.find(name)
        try:
            return node.queryText().getText(0, -1)
        except Exception as exc:
            raise NativeUnavailable(f"Field {name!r} does not expose AT-SPI Text: {exc}") from exc

    def choose(self, label: str, option: str) -> None:
        self.click(label, "combo")
        self.click(option, "option")

    def heading(self, text: str) -> None:
        self.find(text, "heading")

    def escape_custom_select(self, label: str) -> None:
        self.click(label, "combo")
        self.find(label, "list")
        command("xdotool", "key", "Escape")
        time.sleep(0.3)
        if self.available(label, "list"):
            raise NativeFailure(f"Custom combobox {label!r} ignored Escape")


class Session:
    def __init__(self, args, root: Path):
        self.args = args
        self.root = root
        self.proc = None
        self.ax = None

    def start(self):
        env = dict(os.environ, SCRIPTOTAR_DATA_DIR=str(self.root / "data"))
        env["NO_AT_BRIDGE"] = "0"
        env["GTK_A11Y"] = "1"
        self.proc = subprocess.Popen(
            [str(self.args.executable)], env=env,
            stdout=(self.root / "desktop.log").open("a"),
            stderr=subprocess.STDOUT, start_new_session=True,
        )
        self.ax = AxWebKit(self.proc.pid)
        deadline = time.monotonic() + 35
        while time.monotonic() < deadline:
            if self.proc.poll() is not None:
                raise NativeFailure(f"Installed executable exited: {self.proc.returncode}")
            try:
                if self.ax.web_document() and self.ax.available("Scriptotar"):
                    return
                if self.ax.web_document() and (self.ax.available("Studio", "push button")
                                              or self.ax.available("Studio", "button")):
                    return
            except NativeUnavailable:
                pass
            time.sleep(0.6)
        raise NativeUnavailable(
            "Installed app launched but WebKitGTK document and Scriptotar UI "
            "were not both exposed through AT-SPI"
        )

    def screenshot(self, name: str):
        try:
            path = self.root / f"{name}.png"
            command("scrot", str(path))
            return str(path.name)
        except Exception:
            return None

    def stop(self):
        if self.proc is None:
            return
        if self.proc.poll() is None:
            windows = command("xdotool", "search", "--pid", str(self.proc.pid), check=False)
            ids = windows.stdout.split()
            if ids:
                command("xdotool", "windowactivate", "--sync", ids[0], check=False)
                command("xdotool", "key", "--clearmodifiers", "alt+F4", check=False)
            try:
                self.proc.wait(timeout=8)
            except subprocess.TimeoutExpired:
                self.proc.terminate()
                self.proc.wait(timeout=8)
                raise NativeFailure("Installed app did not quit via graphical Alt+F4")
        self.proc = None


def status(report: dict, scenario: str, state: str, reason: str, image: str | None = None):
    assert scenario in CASES and state in ("PASS", "FAIL", "NOT TESTED")
    record = {"status": state, "reason": reason}
    if image:
        record["screenshot"] = image
    report["cases"][scenario] = record


def checked(report: dict, key: str, fn):
    try:
        detail = fn()
        status(report, key, "PASS", str(detail))
        return True
    except NativeUnavailable as exc:
        status(report, key, "NOT TESTED", str(exc))
    except (NativeFailure, Exception) as exc:
        status(report, key, "FAIL", f"{type(exc).__name__}: {exc}")
    return False


def exercise(args, report: dict, root: Path):
    session = Session(args, root)
    tag = "NativeQA-" + args.source_sha[:8]
    name_a, name_b = tag + "-A", tag + "-B"
    study = tag + "-Study"
    body = "Original native WebKitGTK test candidate " + tag
    try:
        def boot():
            session.start()
            image = session.screenshot("installed-webview")
            report["initial_screenshot"] = image
            return "Installed PID owns an AT-SPI WebKit document and rendered Scriptotar controls"
        if not checked(report, "installed_webview", boot):
            return
        ax = session.ax

        def projects():
            for name in (name_a, name_b):
                ax.click("New project", "button")
                ax.type_into("Project name", name)
                ax.click("Create project", "button")
                ax.heading(name)
            ax.choose("Project", name_a)
            ax.heading(name_a)
            ax.choose("Project", name_b)
            ax.heading(name_b)
            session.screenshot("projects")
            return "Created two real SQLite-backed projects and switched both directions via custom selector"
        if not checked(report, "create_switch_projects", projects):
            return

        def new_study():
            ax.choose("Project", name_a)
            ax.click("Studio", "button")
            ax.heading("Studio")
            ax.type_into("Study name", study)
            ax.click("Create study", "button")
            ax.find("Duplicate current script", "button")
            session.screenshot("study-created")
            return "Study created through installed GUI, with actual persisted candidate controls"
        if not checked(report, "create_study", new_study):
            return

        def drafts():
            ax.type_into("Script draft", body)
            ax.click("Save changes", "button")
            ax.find("Saved in this study")
            ax.click("Duplicate current script", "button")
            ax.find("Rename script", "button")
            ax.click("Rename script", "button")
            ax.type_into("Script candidate name", tag + "-copy")
            ax.click("Save name", "button")
            ax.choose("Script candidates", tag + "-copy")
            ax.find("Script draft")
            session.screenshot("candidate-saved")
            return "Saved draft, duplicated candidate, renamed candidate and opened its custom selector"
        if not checked(report, "edit_save_duplicate_candidate", drafts):
            return

        def isolation():
            ax.choose("Project", name_b)
            ax.click("Studio", "button")
            ax.heading("Studio")
            if ax.available(study):
                raise NativeFailure("Foreign project's Study name visible in second project's Studio")
            ax.choose("Project", name_a)
            ax.heading("Studio")
            return "Foreign Study absent in second project's native Studio (UI only; not forged IPC)"
        checked(report, "wrong_project_ui_isolation", isolation)

        def keyboard():
            ax.escape_custom_select("Script candidates")
            command("xdotool", "key", "Escape")
            session.screenshot("controls")
            return "Custom candidate select opens through native interaction and closes on Escape"
        checked(report, "keyboard_custom_controls", keyboard)

        def persist():
            session.stop()
            session.start()
            a = session.ax
            a.choose("Project", name_a)
            a.click("Studio", "button")
            a.choose("Open study", study)
            text = a.read_text("Script draft")
            if body not in text:
                raise NativeFailure("Script body was not restored after process quit and relaunch")
            a.choose("Script candidates", tag + "-copy")
            session.screenshot("reopened")
            return "Installed process closed and restarted; Study body and renamed candidate restored"
        checked(report, "quit_relaunch_persistence", persist)

        def arabic():
            a = session.ax
            a.choose("Interface language", "ع")
            a.heading("الاستوديو")
            session.screenshot("arabic-locale")
            a.choose("لغة الواجهة", "EN")
            a.heading("Studio")
            return "Native WebKit switched to Arabic UI and back through the real custom language control"
        checked(report, "arabic_locale_control", arabic)

        def theme():
            a = session.ax
            a.click("Settings", "button")
            a.choose("Theme", "System")
            a.click("Save changes", "button")
            session.screenshot("system-theme")
            a.choose("Theme", "Dark")
            a.click("Save changes", "button")
            session.screenshot("dark-theme")
            return "System and dark modes selected and saved by installed GUI (not a visual judgment)"
        checked(report, "theme_mode_control", theme)

        # This run deliberately has no fabricated downloads or media transcripts.
        # The following steps require source fixtures and dedicated adversarial UI inputs.
        status(report, "multi_source_transcript", "NOT TESTED",
               "Disposable profile has no real research/transcript sources; no mock material injected")
        status(report, "source_revocation", "NOT TESTED",
               "Needs a real source with revocable availability, verified after project reload")
        status(report, "wrong_project_ipc_denial", "NOT TESTED",
               "Native UI scoping is not an authorization probe for forged foreign IDs")
        status(report, "error_recovery", "NOT TESTED",
               "No safe native graphical fault-injection mechanism in isolated profile")
        status(report, "arabic_rtl", "NOT TESTED",
               "AT-SPI labels alone cannot certify document direction, mixed-text order or visual RTL")
        status(report, "light_dark_visual", "NOT TESTED",
               "Native visual appearance requires independent screenshot inspection; app offers dark/system")
    finally:
        if session.proc is not None:
            try:
                session.stop()
            except Exception as exc:
                report["cleanup_warning"] = str(exc)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--executable", required=True, type=Path)
    parser.add_argument("--package", required=True, type=Path)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--run-url", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r"[a-f0-9]{40}", args.source_sha):
        parser.error("--source-sha must be an exact 40-character Git commit SHA")
    if not re.fullmatch(r"https://github.com/MousaXD/scriptotar/actions/runs/[1-9][0-9]*",
                        args.run_url):
        parser.error("--run-url must identify an exact GitHub Actions run")
    args.output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="scriptotar-native-", dir=args.output) as tmp:
        root = Path(tmp)
        report = {
            "scope": "native-installed",
            "source_sha": args.source_sha,
            "run_url": args.run_url,
            "utc": datetime.now(timezone.utc).isoformat(),
            "platform": platform.platform(),
            "machine": {"os": platform.platform(), "cpu": platform.processor() or platform.machine(),
                        "ram_kib": next((int(line.split()[1]) for line in
                                         Path("/proc/meminfo").read_text().splitlines()
                                         if line.startswith("MemTotal:")), None)},
            "package": str(args.package.resolve()), "package_sha256": sha256(args.package),
            "executable": str(args.executable.resolve()), "executable_sha256": sha256(args.executable),
            "cases": {case: {"status": "NOT TESTED", "reason": "Not executed"} for case in CASES},
            "native_ui_probe": None,
        }
        try:
            exercise(args, report, root)
            if report["cases"]["installed_webview"]["status"] == "PASS":
                report["native_ui_probe"] = "installed-webview-automation"
        except Exception as exc:
            status(report, "installed_webview", "FAIL", f"Uncaught runner error: {exc}")
        # Preserve evidence outside the short-lived isolated profile; never leak user data.
        for photo in root.glob("*.png"):
            photo.replace(args.output / photo.name)
        if (root / "desktop.log").exists():
            (args.output / "desktop.log").write_bytes((root / "desktop.log").read_bytes()[-100000:])
    states = [item["status"] for item in report["cases"].values()]
    report["verdict"] = "FAIL" if "FAIL" in states else "NOT TESTED" if "NOT TESTED" in states else "PASS"
    (args.output / "result.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    print(json.dumps({"verdict": report["verdict"],
                      "cases": {key: item["status"] for key, item in report["cases"].items()}},
                     indent=2))
    # A partial installed journey must NEVER produce a release-green job.
    return 0 if report["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
