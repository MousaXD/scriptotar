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
import sqlite3
import subprocess
import sys
import tempfile
import time
import uuid
from datetime import datetime, timezone


CASES = (
    "installed_webview", "create_switch_projects", "canonical_project_persistence", "create_study",
    "edit_save_duplicate_candidate", "quit_relaunch_persistence",
    "wrong_project_ui_isolation", "wrong_project_ipc_denial",
    "multi_source_transcript", "source_revocation",
    "error_recovery", "keyboard_custom_controls", "arabic_locale_control",
    "theme_mode_control", "arabic_rtl",
    "light_dark_visual",
)


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def command(*argv: str, check: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(argv, capture_output=True, text=True, check=check, timeout=15)


def verify_canonical_projects(data_dir: Path, expected_names: tuple[str, str]) -> str:
    """Read only the REAL installed app's SQLite after GUI project creation."""
    database = data_dir / "scriptotar.sqlite3"
    if not database.is_file():
        raise NativeFailure(f"Canonical SQLite missing after native project creation: {database}")
    try:
        with sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True, timeout=4) as connection:
            rows = connection.execute(
                "SELECT id, name FROM projects WHERE name IN (?, ?) AND archived_at IS NULL",
                expected_names,
            ).fetchall()
    except sqlite3.Error as exc:
        raise NativeFailure(f"Could not read canonical project rows: {exc}") from exc
    if (len(rows) != 2 or {name for _id, name in rows} != set(expected_names)
            or len({id for id, _name in rows}) != 2):
        raise NativeFailure(
            f"Canonical SQLite does not hold two distinct created projects; "
            f"expected={expected_names}, actual_names={[name for _id, name in rows]}"
        )
    try:
        for project_id, _name in rows:
            uuid.UUID(project_id)
    except (ValueError, TypeError) as exc:
        raise NativeFailure("Canonical SQLite contains invalid created project ID") from exc
    return "Two distinct nonarchived real project rows persisted in installed application SQLite"



def verify_canonical_saved_draft(
    data_dir: Path, project_name: str, study_name: str, expected_body: str
) -> str:
    """Read only canonical Study/draft/version rows after installed GUI save."""
    database = data_dir / "scriptotar.sqlite3"
    if not database.is_file():
        raise NativeFailure(f"Canonical SQLite missing after Study save: {database}")
    try:
        with sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True, timeout=4) as connection:
            rows = connection.execute(
                """
                SELECT d.id, d.revision, d.body, v.version, v.body
                  FROM projects p
                  JOIN content_studies st ON st.project_id=p.id
                  JOIN creative_drafts d ON d.project_id=p.id AND d.study_id=st.id
                  JOIN draft_versions v ON v.draft_id=d.id
                       AND v.version=(SELECT MAX(latest.version) FROM draft_versions latest
                                      WHERE latest.draft_id=d.id)
                 WHERE p.name=? AND p.archived_at IS NULL
                   AND st.title=? AND st.archived_at IS NULL
                   AND d.archived_at IS NULL
                """,
                (project_name, study_name),
            ).fetchall()
    except sqlite3.Error as exc:
        raise NativeFailure(f"Cannot read real persisted Study draft and version: {exc}") from exc
    if len(rows) != 1:
        raise NativeFailure(
            f"Expected one active draft in Study {study_name!r} before duplication; "
            f"found {len(rows)}"
        )
    draft_id, revision, body, saved_version, saved_body = rows[0]
    try:
        uuid.UUID(draft_id)
    except (ValueError, TypeError) as exc:
        raise NativeFailure("Persisted draft has invalid canonical ID") from exc
    if revision < 2 or saved_version < 2 or body != expected_body or saved_body != expected_body:
        raise NativeFailure(
            "Saved GUI draft did not persist the exact expected body in both canonical "
            f"creative_drafts and latest immutable draft_versions (revision={revision}, "
            f"version={saved_version}, expected_len={len(expected_body)}, "
            f"persisted_len={len(body) if body is not None else None})"
        )
    return f"Canonical SQLite stores exact GUI draft body and immutable version {saved_version}"


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
                # Bind to the installed process before matching the accessible
                # name: GTK can publish an empty or branded application name.
                # Newer AT-SPI bindings expose the process PID directly. On
                # older pyatspi, bind the sole app in isolated Xvfb to the
                # X11 window owned by the exact installed executable PID.
                try:
                    owner_pid = app.get_process_id()
                except AttributeError:
                    owner_pid = None
                if owner_pid == self.pid:
                    apps.append(app)
                elif owner_pid in (0, None) and "scriptotar" in name.lower():
                    windows = command("xdotool", "search", "--pid", str(self.pid), check=False)
                    if windows.returncode == 0 and windows.stdout.strip():
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
                # WebKit web-process children can be transiently missing a
                # VISIBLE state even while their parent renders. Do not prune
                # their entire AT-SPI subtree; still require a real document
                # and user-actionable controls before an acceptance PASS.
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
                if role in ("document web", "document frame") or (
                    "document" in role and ("web" in role or "frame" in role)
                ):
                    return True
            except Exception:
                pass
        return False

    def diagnostics(self) -> dict:
        """Bounded native AT-SPI evidence on failure; never substitutes for PASS."""
        result = {"expected_pid": self.pid, "desktop_apps": [], "x11_windows": [],
                  "display": os.environ.get("DISPLAY"),
                  "dbus_session_address_set": bool(os.environ.get("DBUS_SESSION_BUS_ADDRESS")),
                  "gtk_modules": os.environ.get("GTK_MODULES"),
                  "no_at_bridge": os.environ.get("NO_AT_BRIDGE")}
        try:
            windows = command("xdotool", "search", "--pid", str(self.pid), check=False)
            result["x11_windows"] = windows.stdout.split()[:20]
        except Exception as exc:
            result["x11_error"] = f"{type(exc).__name__}: {exc}"
        try:
            desktop = self.atspi.Registry.getDesktop(0)
            for app in desktop:
                entry = {"name": (app.name or "")[:120], "nodes": []}
                try:
                    entry["pid"] = app.get_process_id()
                except Exception as exc:
                    entry["pid_error"] = type(exc).__name__
                pending = [(app, 0)]
                while pending and len(entry["nodes"]) < 320:
                    node, depth = pending.pop(0)
                    try:
                        try:
                            child_count = node.childCount
                        except Exception:
                            child_count = None
                        entry["nodes"].append({
                            "depth": depth, "role": node.getRoleName(),
                            "name": (node.name or "")[:120], "children": child_count
                        })
                        # Previous diagnostics stopped precisely at the WebKit
                        # scroll pane (depth 5), hiding the decisive subtree.
                        if depth < 15:
                            pending.extend((child, depth + 1) for child in node)
                    except Exception as exc:
                        entry["nodes"].append({"depth": depth, "error": type(exc).__name__})
                result["desktop_apps"].append(entry)
                if len(result["desktop_apps"]) >= 12:
                    break
        except Exception as exc:
            result["atspi_error"] = f"{type(exc).__name__}: {exc}"
        return result

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

    def edit_field(self, label: str, timeout: float = 12):
        """Find an actual input/textarea, not a coincidental text label.

        On real WebKitGTK the Svelte label + HTML placeholder are combined
        into one accessible name, e.g. 'Study name e.g. Launch video'.
        The role remains 'embedded'; exact 'Study name' is NOT exposed.
        """
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                for node in self.nodes():
                    try:
                        accessible_name = (node.name or "").strip()
                        role = node.getRoleName().lower()
                        if (role in ("embedded", "entry", "text entry", "text field")
                                and (accessible_name == label
                                     or accessible_name.startswith(label + " "))):
                            return node
                    except Exception:
                        continue
            except NativeUnavailable:
                pass
            time.sleep(0.3)
        raise NativeFailure(f"AT-SPI editable field not found for label {label!r}")

    def type_into(self, name: str, value: str) -> None:
        node = self.edit_field(name)
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
        # The semantic section heading and textarea can share this name.
        # Read the editable AT-SPI node, never the similarly named h2.
        node = self.edit_field(name)
        try:
            return node.queryText().getText(0, -1)
        except Exception as exc:
            raise NativeUnavailable(f"Field {name!r} does not expose AT-SPI Text: {exc}") from exc

    def choose(self, label: str, option: str) -> None:
        """Activate exact-name, PID-owned WebKit option using its AT-SPI bounds.

        Prior AT-SPI list-item action/focus+Return closed the popup without
        switching B to A (run #44095877399). Use a real pointer click on
        the accessible option; caller must still prove the new dashboard.
        """
        self.click(label, "combo")
        node = self.find(option, "list item", timeout=5)
        try:
            bounds = node.queryComponent().getExtents(
                getattr(self.atspi, "DESKTOP_COORDS", 0)
            )
            if bounds.width < 2 or bounds.height < 2:
                raise NativeFailure(f"AT-SPI option has no clickable bounds: {option!r}")
            x = int(bounds.x + bounds.width / 2)
            y = int(bounds.y + bounds.height / 2)
            if x < 0 or y < 0:
                raise NativeFailure(f"Invalid option coordinates for {option!r}: {(x, y)}")
            command("xdotool", "mousemove", str(x), str(y))
            command("xdotool", "click", "--clearmodifiers", "1")
            time.sleep(0.8)
        except NativeFailure:
            raise
        except Exception as exc:
            raise NativeFailure(
                f"Cannot pointer-activate native AT-SPI option {option!r}: {exc}"
            ) from exc

    def heading(self, text: str, timeout: float = 12) -> None:
        """Require an exact semantic heading in the installed WebKit AT-SPI tree.

        WebKitGTK 4.1 maps actual dashboard h1 to 'document frame'. Never
        accept a similarly named button or arbitrary static label.
        """
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                for node in self.nodes():
                    try:
                        if ((node.name or "").strip() == text and
                                node.getRoleName().lower() in ("heading", "document frame")):
                            return
                    except Exception:
                        continue
            except NativeUnavailable:
                pass
            time.sleep(0.4)
        raise NativeFailure(
            f"AT-SPI semantic page heading not found (heading/document frame): {text!r}"
        )

    def heading_available(self, text: str) -> bool:
        try:
            self.heading(text, timeout=1)
            return True
        except (NativeFailure, NativeUnavailable):
            return False

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
        # GTK3's ATK bridge must be available in a minimal Xvfb session.
        # This enables observation; it never bypasses the AT-SPI gate.
        env.setdefault("GTK_MODULES", "atk-bridge")
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
                if self.ax.web_document() and (
                    self.ax.available("Studio", "button") or
                    self.ax.available("Studio", "link")
                ):
                    return
            except NativeUnavailable:
                pass
            time.sleep(0.6)
        diagnostics = self.ax.diagnostics()
        (self.root / "a11y-diagnostics.json").write_text(
            json.dumps(diagnostics, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
        self.screenshot("unverified-window")
        raise NativeUnavailable(
            "Installed app launched but the native WebKitGTK accessibility gate "
            "was not satisfied; see a11y-diagnostics.json "
            f"(apps={[(item.get('name'), item.get('pid')) for item in diagnostics['desktop_apps']]}, "
            f"windows={diagnostics['x11_windows']})"
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
        proc = self.proc
        try:
            if proc.poll() is None:
                # Target only the mapped installed app, not its hidden X11 helpers.
                windows = command("xdotool", "search", "--onlyvisible",
                                  "--pid", str(proc.pid), check=False)
                ids = windows.stdout.split()
                titled = []
                for window_id in ids:
                    title = command("xdotool", "getwindowname", window_id, check=False)
                    if title.returncode == 0 and "scriptotar" in title.stdout.lower():
                        titled.append(window_id)
                if not titled:
                    raise NativeFailure(
                        f"No visible Scriptotar window for graphical quit (XIDs={ids})"
                    )
                target = titled[0]
                focus = command("xdotool", "windowactivate", "--sync", target, check=False)
                if focus.returncode != 0:
                    raise NativeFailure(f"Window manager could not activate Scriptotar {target}")
                key = command("xdotool", "key", "--clearmodifiers", "alt+F4", check=False)
                if key.returncode != 0:
                    raise NativeFailure(f"Alt+F4 could not be delivered to {target}")
                try:
                    proc.wait(timeout=8)
                except subprocess.TimeoutExpired as exc:
                    raise NativeFailure("Installed app did not quit via graphical Alt+F4") from exc
        except Exception:
            # A failed graphical quit stays FAIL, but must not leak a process
            # or interfere with the installed package uninstall.
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=8)
            raise
        finally:
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
    if report["cases"][key]["status"] != "PASS":
        print(f"NATIVE {key}: {report['cases'][key]['status']}: "
              f"{report['cases'][key]['reason']}", flush=True)
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
            # A fresh isolated installation may have no project at all.
            # Startup exposes the Studio rail before the asynchronous workspace
            # bootstrap is finished. Wait for a real *workspace* state instead
            # of assuming the dashboard's New project button already exists.
            deadline = time.monotonic() + 45
            initial_mode = None
            while time.monotonic() < deadline:
                if ax.heading_available("Create your first project"):
                    initial_mode = "first_project"
                    break
                if ax.available("New project", "button"):
                    initial_mode = "dashboard"
                    break
            if not initial_mode:
                raise NativeFailure(
                    "Workspace never exposed either the first-project form or "
                    "the dashboard New project button after native WebKit startup"
                )
            report["project_start_mode"] = initial_mode
            for index, name in enumerate((name_a, name_b)):
                if index != 0 or initial_mode == "dashboard":
                    ax.click("New project", "button")
                ax.type_into("Project name", name)
                ax.click("Create project", "button")
                # A real dashboard h1 is required. Seeing only an option in
                # the project chooser would not prove that creation activated it.
                try:
                    ax.heading(name, timeout=25)
                    ax.find("New project", "button", timeout=4)
                except NativeFailure as exc:
                    screenshot = session.screenshot(f"project-{index + 1}-failure")
                    diagnostics = ax.diagnostics()
                    diagnostics["failure_phase"] = "project_creation_or_activation"
                    diagnostics["expected_project"] = name
                    diagnostics["failure_screenshot"] = screenshot
                    (root / "a11y-diagnostics.json").write_text(
                        json.dumps(diagnostics, indent=2, ensure_ascii=False) + "\n",
                        encoding="utf-8",
                    )
                    raise NativeFailure(
                        f"Create project did not expose an active dashboard heading and action "
                        f"{name!r}; screenshot={screenshot}, see a11y-diagnostics.json"
                    ) from exc
                session.screenshot(f"project-{index + 1}-created")
            if not checked(
                report, "canonical_project_persistence",
                lambda: verify_canonical_projects(root / "data", (name_a, name_b))
            ):
                raise NativeFailure("Canonical project database assertion failed")
            ax.choose("PROJECT", name_a)
            ax.heading(name_a, timeout=25)
            ax.find("New project", "button", timeout=4)
            session.screenshot("project-switch-to-A")
            ax.choose("PROJECT", name_b)
            ax.heading(name_b, timeout=25)
            ax.find("New project", "button", timeout=4)
            session.screenshot("project-switch-to-B")
            session.screenshot("projects")
            return "Created and activated two real projects, then switched both directions via custom selector"
        if not checked(report, "create_switch_projects", projects):
            return

        def new_study():
            ax.choose("PROJECT", name_a)
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
            verify_canonical_saved_draft(root / "data", name_a, study, body)
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

        def persist():
            session.stop()
            session.start()
            a = session.ax
            # The installed application must reopen the same selected
            # project after a true graphical Alt+F4; no artificial reselect.
            a.heading(name_a, timeout=20)
            a.click("Studio", "button")
            a.choose("Open study", study)
            restored = a.read_text("Script draft")
            if restored != body:
                raise NativeFailure(
                    "Script body changed after installed process quit/relaunch"
                )
            a.choose("Script candidates", tag + "-copy")
            if a.read_text("Script draft") != body:
                raise NativeFailure(
                    "Renamed duplicate candidate body changed after relaunch"
                )
            verify_canonical_saved_draft(root / "data", name_a, study, body)
            session.screenshot("reopened")
            return (
                "Graphical Alt+F4/relaunch kept selected project, saved Study "
                "body, immutable version and renamed candidate"
            )
        if not checked(report, "quit_relaunch_persistence", persist):
            # A failed restart can corrupt follow-on UI state: preserve the
            # exact first failing screen and don't create cascading false FAILs.
            return
        ax = session.ax

        def isolation():
            ax.choose("PROJECT", name_b)
            ax.click("Studio", "button")
            ax.heading("Studio")
            if ax.available(study):
                raise NativeFailure("Foreign project's Study name visible in second project's Studio")
            ax.choose("PROJECT", name_a)
            ax.heading("Studio")
            return "Foreign Study absent in second project's native Studio (UI only; not forged IPC)"
        if not checked(report, "wrong_project_ui_isolation", isolation):
            return

        def keyboard():
            # A project switch unmounts its previous Study editor; reopen
            # the real Study before asserting the candidate combobox exists.
            if not ax.available("Script candidates", "combo"):
                ax.choose("Open study", study)
            ax.escape_custom_select("Script candidates")
            command("xdotool", "key", "Escape")
            session.screenshot("controls")
            return "Opened saved Study and closed its native custom selector with Escape"
        if not checked(report, "keyboard_custom_controls", keyboard):
            return

        def arabic():
            a = session.ax
            try:
                a.choose("Interface language", "ع")
                # The translated sidebar button must truly navigate; merely
                # changing a global language label is insufficient.
                a.click("الاستوديو", "button")
                a.heading("الاستوديو")
                session.screenshot("arabic-locale")
            finally:
                # Don't poison subsequent English accessibility assertions.
                if a.available("لغة الواجهة", "combo"):
                    a.choose("لغة الواجهة", "EN")
            a.heading("Studio")
            return "Real native Arabic Studio route and restored English UI"
        if not checked(report, "arabic_locale_control", arabic):
            return

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
        # Preserve the first genuine *interaction* failure while the installed
        # app is still on the exact failing screen. A boot screenshot alone
        # cannot explain project/forms/Studio selector failures.
        first_failed_case = next(
            (name for name, value in report["cases"].items()
             if value["status"] == "FAIL"), None
        )
        if first_failed_case and session.proc is not None:
            report["first_failed_case"] = first_failed_case
            report["failure_screenshot"] = session.screenshot(
                "failed-" + first_failed_case
            )
            if session.ax:
                try:
                    (root / "a11y-diagnostics.json").write_text(
                        json.dumps(session.ax.diagnostics(), indent=2,
                                   ensure_ascii=False) + "\n", encoding="utf-8"
                    )
                    report["failure_accessibility_tree"] = "a11y-diagnostics.json"
                except Exception as exc:
                    report["failure_probe_error"] = f"{type(exc).__name__}: {exc}"
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
        if (root / "a11y-diagnostics.json").exists():
            (args.output / "a11y-diagnostics.json").write_bytes(
                (root / "a11y-diagnostics.json").read_bytes()
            )
            report["native_accessibility_diagnostics"] = "a11y-diagnostics.json"
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
