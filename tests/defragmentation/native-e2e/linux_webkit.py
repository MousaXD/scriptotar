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
import shutil
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
import uuid
from datetime import datetime, timezone

try:
    import resource  # Unix-only; Windows probe imports shared read-only SQLite helpers.
except ImportError:
    resource = None


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


def native_crash_context(pid: int) -> dict:
    """Read bounded Linux crash metadata after a real installed-process exit.

    Never retries the app, injects a signal, changes SQLite or turns a
    crashed installed WebView into PASS. A missing coredump is evidence too.
    """
    details = {"pid": pid}
    try:
        details["core_pattern"] = Path("/proc/sys/kernel/core_pattern").read_text(
            encoding="utf-8"
        ).strip()
    except (OSError, UnicodeError) as exc:
        details["core_pattern_error"] = type(exc).__name__
    if resource is not None:
        try:
            soft, hard = resource.getrlimit(resource.RLIMIT_CORE)
            details["core_limit_bytes"] = {"soft": soft, "hard": hard}
        except (OSError, ValueError) as exc:
            details["core_limit_error"] = type(exc).__name__
    else:
        details["core_limit_error"] = "UnavailableOnThisPlatform"
    coredumpctl = shutil.which("coredumpctl")
    details["coredumpctl_available"] = bool(coredumpctl)
    details["gdb_available"] = bool(shutil.which("gdb"))
    if coredumpctl:
        try:
            probe = subprocess.run(
                [coredumpctl, "--no-pager", "info", str(pid)],
                capture_output=True, text=True, timeout=12, check=False,
            )
            details["coredumpctl_returncode"] = probe.returncode
            details["coredumpctl_tail"] = (probe.stdout + probe.stderr)[-12000:]
        except (OSError, subprocess.TimeoutExpired) as exc:
            details["coredumpctl_error"] = type(exc).__name__
    return details


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



def count_canonical_jobs(data_dir: Path, project_name: str) -> int:
    """Read-only count of jobs for the actual installed project's SQLite."""
    database = data_dir / "scriptotar.sqlite3"
    if not database.is_file():
        raise NativeFailure("Installed canonical SQLite missing during denied URL test")
    try:
        with sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True, timeout=4) as db:
            row = db.execute(
                "SELECT COUNT(*) FROM jobs j JOIN projects p ON p.id=j.project_id "
                "WHERE p.name=? AND p.archived_at IS NULL", (project_name,)
            ).fetchone()
            if row is None:
                raise NativeFailure("Could not count installed canonical jobs")
            return int(row[0])
    except sqlite3.Error as exc:
        raise NativeFailure(f"Installed canonical jobs read-only probe failed: {exc}") from exc


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


def wait_for_canonical_saved_draft(
    data_dir: Path, project_name: str, study_name: str, expected_body: str,
    timeout_seconds: float = 20.0,
) -> str:
    """Wait for the real installed GUI save; never mutate canonical SQLite.

    WebKitGTK's Save changes action may still show "Saving..." after the
    accessibility click returns. An empty version-1 row is not evidence of a
    failed save until the in-flight action has been allowed to finish.
    """
    deadline = time.monotonic() + timeout_seconds
    last_error = "No canonical draft checked yet"
    while time.monotonic() < deadline:
        try:
            return verify_canonical_saved_draft(
                data_dir, project_name, study_name, expected_body
            )
        except NativeFailure as exc:
            last_error = str(exc)
        time.sleep(0.35)
    raise NativeFailure(
        f"Installed GUI draft did not finish its canonical immutable save "
        f"within {timeout_seconds:g}s; last SQLite evidence: {last_error}"
    )



def verify_canonical_reopened_candidates(
    data_dir: Path, project_name: str, study_name: str,
    expected_body: str, expected_copy_title: str,
) -> str:
    """Independently prove BOTH renamed and original drafts survived restart.

    The pre-duplication save check expects one draft. Reusing that assertion
    after duplication would incorrectly fail correctly persisted data.
    """
    database = data_dir / "scriptotar.sqlite3"
    if not database.is_file():
        raise NativeFailure(f"Canonical SQLite missing on Study relaunch: {database}")
    try:
        with sqlite3.connect(database.resolve().as_uri() + "?mode=ro", uri=True, timeout=4) as con:
            rows = con.execute(
                """
                SELECT d.id, d.title, d.body, d.revision, v.version, v.body
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
        raise NativeFailure(f"Cannot read restored canonical candidates: {exc}") from exc
    if len(rows) != 2 or {row[1] for row in rows} != {study_name, expected_copy_title}:
        raise NativeFailure(
            "Installed Study relaunch must retain exactly the original and "
            f"renamed duplicate candidates, actual titles={[row[1] for row in rows]}"
        )
    try:
        ids = {uuid.UUID(row[0]) for row in rows}
    except (ValueError, TypeError) as exc:
        raise NativeFailure("Restored candidate has invalid canonical UUID") from exc
    if len(ids) != 2:
        raise NativeFailure("Restored original and duplicate share an invalid draft identity")
    for _id, title, body, revision, version, saved_body in rows:
        min_version = 2 if title == study_name else 1
        if (revision < min_version or version < min_version
                or body != expected_body or saved_body != expected_body):
            raise NativeFailure(
                f"Restored candidate {title!r} does not preserve exact immutable "
                "GUI-authored body and version"
            )
    return "Two distinct original/renamed candidates and immutable versions restored from canonical SQLite"


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

    def global_locale_control(self) -> str:
        """Find the real toolbar language selector by its native options.

        Neither PROJECT nor Interface language is guaranteed to retain an
        English accessible name after switching locale. Inspect only bounded
        top-bar comboboxes and require both UI options through real AT-SPI.
        Opening + Escape must not change the selected language or project.
        """
        candidates = []
        for node in self.nodes():
            try:
                if "combo" not in node.getRoleName().lower():
                    continue
                label = (node.name or "").strip()
                rect = node.queryComponent().getExtents(
                    getattr(self.atspi, "DESKTOP_COORDS", 0)
                )
                # Installed Xvfb/Openbox top bar: content controls occur
                # substantially below y=190. Never infer from text/DOM.
                if (label and 30 <= rect.y <= 190
                        and rect.width >= 20 and rect.height >= 15):
                    candidates.append(label)
            except Exception:
                continue
        observed = []
        matches = []
        for label in dict.fromkeys(candidates):
            try:
                self.click(label, "combo")
                has_en = self.available("EN", "list item")
                has_ar = self.available("ع", "list item")
                observed.append((label, has_en, has_ar))
                if has_en and has_ar:
                    matches.append(label)
            except (NativeFailure, NativeUnavailable) as exc:
                observed.append((label, type(exc).__name__))
            finally:
                # ESC only closes a native popup; it never selects a value.
                command("xdotool", "key", "--clearmodifiers", "Escape")
                time.sleep(0.2)
        if len(matches) != 1:
            raise NativeFailure(
                "Expected exactly one native top-bar language combo with both "
                f"EN/ع options, observed {observed!r}"
            )
        return matches[0]

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

    def escape_custom_select(self, label: str, expected_option: str) -> None:
        """Certify native popup opens and Escape closes it (WebKit AT-SPI).

        The actual installed WebKitGTK tree in run #44114536688 exposes
        the rendered candidate popup as a *comment* named Script candidates,
        with list item children. Searching for role='list' was a QA harness
        mistake, not a failed WebView interaction. Both the wrapper and a
        concrete expected option must exist before the Escape keystroke.
        """
        self.click(label, "combo")
        self.find(label, "comment", timeout=5)
        self.find(expected_option, "list item", timeout=5)
        command("xdotool", "key", "--clearmodifiers", "Escape")
        time.sleep(0.3)
        if (self.available(label, "comment") or
                self.available(expected_option, "list item")):
            raise NativeFailure(
                f"Installed WebKit custom combobox {label!r} stayed open after Escape"
            )


    def assert_selected_option(self, label: str, option: str) -> None:
        """Verify an installed custom Select's actual native selected state.

        A screenshot or a click alone cannot prove persisted theme settings.
        The Svelte option has aria-selected; require AT-SPI STATE_SELECTED after
        reopening the real popup, then send Escape as a real user would.
        """
        self.click(label, "combo")
        try:
            node = self.find(option, "list item", timeout=5)
            try:
                selected = node.getState().contains(self.atspi.STATE_SELECTED)
            except Exception as exc:
                raise NativeUnavailable(
                    f"AT-SPI cannot expose selected state for {label!r}/{option!r}: {exc}"
                ) from exc
            if not selected:
                raise NativeFailure(
                    f"Installed native selector {label!r} does not select {option!r}"
                )
        finally:
            command("xdotool", "key", "--clearmodifiers", "Escape")
            time.sleep(0.3)
        if self.available(label, "comment"):
            raise NativeFailure(
                f"Installed native selector {label!r} remained expanded after Escape"
            )


class Session:
    def __init__(self, args, root: Path):
        self.args = args
        self.root = root
        self.proc = None
        self.ax = None

    def start(self, expected_studio: str = "Studio"):
        # Locale is persisted by the installed WebView. The Arabic restart
        # must use the translated accessible rail name, not English Studio.
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
                exit_code = self.proc.returncode
                signal_name = None
                if exit_code < 0:
                    try:
                        signal_name = signal.Signals(-exit_code).name
                    except ValueError:
                        signal_name = f"signal {-exit_code}"
                crash = {
                    "pid": self.proc.pid,
                    "returncode": exit_code,
                    "signal": signal_name,
                    "startup_expected_studio": expected_studio,
                    "stage": "installed native application startup after launch/relaunch",
                    "native_crash_context": native_crash_context(self.proc.pid)
                    if exit_code < 0 else None,
                }
                (self.root / "process-exit.json").write_text(
                    json.dumps(crash, indent=2) + "\n", encoding="utf-8"
                )
                raise NativeFailure(
                    f"Installed executable exited: {exit_code}"
                    + (f" ({signal_name})" if signal_name else "")
                )
            try:
                if self.ax.web_document() and (
                    self.ax.available(expected_studio, "button") or
                    self.ax.available(expected_studio, "link")
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
                # Nonblocking X11 activation: --sync can hang on a real
                # WebKitGTK top-level after the Arabic toolbar changes.
                # Confirm the active native window before dispatching Alt+F4.
                focus = command("xdotool", "windowactivate", target, check=False)
                if focus.returncode != 0:
                    raise NativeFailure(
                        f"Window manager could not request Scriptotar activation {target}"
                    )
                focused = False
                deadline = time.monotonic() + 4
                while time.monotonic() < deadline:
                    active = command("xdotool", "getactivewindow", check=False)
                    if active.returncode == 0 and active.stdout.strip() == target:
                        focused = True
                        break
                    time.sleep(0.2)
                if not focused:
                    # Openbox/WebKitGTK can leave _NET_ACTIVE_WINDOW unchanged
                    # after an Arabic toolbar transition. Request real X11
                    # *keyboard* focus for this exact installed application
                    # window, then independently observe that X focus moved.
                    # A focus request alone never licenses sending Alt+F4.
                    requested = command("xdotool", "windowfocus", target, check=False)
                    if requested.returncode == 0:
                        keyboard_deadline = time.monotonic() + 4
                        while time.monotonic() < keyboard_deadline:
                            keyboard = command("xdotool", "getwindowfocus", check=False)
                            if (keyboard.returncode == 0
                                    and keyboard.stdout.strip() == target):
                                focused = True
                                break
                            time.sleep(0.2)
                if not focused:
                    raise NativeFailure(
                        f"Installed Scriptotar window {target} never acquired "
                        "verified native X11 active-window or keyboard focus; "
                        "refusing Alt+F4"
                    )
                key = command("xdotool", "key", "--clearmodifiers", "alt+F4", check=False)
                if key.returncode != 0:
                    raise NativeFailure(f"Alt+F4 could not be delivered to {target}")
                try:
                    exit_code = proc.wait(timeout=8)
                except subprocess.TimeoutExpired as exc:
                    raise NativeFailure("Installed app did not quit via graphical Alt+F4") from exc
                if exit_code < 0:
                    try:
                        signal_name = signal.Signals(-exit_code).name
                    except ValueError:
                        signal_name = f"signal {-exit_code}"
                    crash = {
                        "pid": proc.pid, "returncode": exit_code,
                        "signal": signal_name,
                        "stage": "installed native graphical Alt+F4 shutdown",
                        "native_crash_context": native_crash_context(proc.pid),
                    }
                    (self.root / "process-exit.json").write_text(
                        json.dumps(crash, indent=2) + "\n", encoding="utf-8"
                    )
                    raise NativeFailure(
                        f"Installed native graphical shutdown crashed: "
                        f"{exit_code} ({signal_name})"
                    )
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
            if ax.read_text("Script draft") != body:
                raise NativeFailure(
                    "Installed WebKitGTK editor did not contain exact native typed body"
                )
            ax.click("Save changes", "button")
            wait_for_canonical_saved_draft(root / "data", name_a, study, body)
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
            verify_canonical_reopened_candidates(
                root / "data", name_a, study, body, tag + "-copy"
            )
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
            ax.escape_custom_select("Script candidates", tag + "-copy")
            session.screenshot("controls")
            return (
                "Reopened persisted Study, observed WebKitGTK native popup with "
                "actual renamed candidate list item, and verified Escape closed it"
            )
        if not checked(report, "keyboard_custom_controls", keyboard):
            return

        def arabic():
            # Separate a genuine Arabic/relaunch failure from an attempted
            # English restoration. The latter must never mask the former.
            a = session.ax
            phase = "select_arabic"
            original_error = None
            restore_error = None
            trace = {
                "steps": [], "source_sha": args.source_sha,
                "run_url": args.run_url,
            }

            def reached(step):
                nonlocal phase
                phase = step
                trace["steps"].append(step)

            try:
                a.choose(a.global_locale_control(), "ع")
                reached("arabic_selected")
                # Observe actual native translated navigation and page heading.
                a.click("الاستوديو", "button")
                a.heading("الاستوديو")
                session.screenshot("arabic-locale")
                # Independently review a *genuinely open* native Arabic
                # toolbar popup; switching language alone is not RTL review.
                # This capture is not a PASS assertion for the visual gate.
                try:
                    locale_combo = a.global_locale_control()
                    a.click(locale_combo, "combo")
                    try:
                        a.find("EN", "list item", timeout=5)
                        a.find("ع", "list item", timeout=5)
                        report.setdefault("visual_capture", {})["arabic_menu"] = (
                            session.screenshot("arabic-locale-menu-open")
                        )
                    finally:
                        command("xdotool", "key", "--clearmodifiers", "Escape")
                        time.sleep(0.3)
                except Exception as capture_error:
                    report.setdefault("visual_capture_errors", {})["arabic_menu"] = (
                        f"{type(capture_error).__name__}: {capture_error}"
                    )
                reached("arabic_studio_observed")

                # Real Alt+F4, then a new PID; do not write settings or
                # replace graphical actions with a process restart shortcut.
                old_pid = session.proc.pid
                reached("graphical_arabic_quit")
                session.stop()
                reached("arabic_process_exited_graphically")
                session.start(expected_studio="الاستوديو")
                trace["old_pid"] = old_pid
                trace["new_pid"] = session.proc.pid
                if old_pid == session.proc.pid:
                    raise NativeFailure(
                        "Arabic restart reused the previous installed PID"
                    )
                reached("arabic_relaunch_native_document")
                a = session.ax
                a.click("الاستوديو", "button")
                a.heading("الاستوديو")
                if a.available("Studio", "button"):
                    raise NativeFailure(
                        "Arabic persisted-relaunch shell still exposes English Studio navigation"
                    )
                session.screenshot("arabic-reopened")
                reached("arabic_heading_after_relaunch")
            except Exception as exc:
                original_error = exc
                trace["original_error"] = f"{type(exc).__name__}: {exc}"
                trace["failed_phase"] = phase
                raise
            finally:
                proc = session.proc
                trace["restore_attempted_with_live_process"] = (
                    proc is not None and proc.poll() is None
                )
                trace["process_pid_at_restore"] = proc.pid if proc else None
                trace["process_returncode_at_restore"] = (
                    proc.returncode if proc else None
                )
                if proc is None or proc.poll() is not None:
                    restore_error = NativeFailure(
                        "Installed process unavailable during graphical English restoration"
                    )
                else:
                    try:
                        a = session.ax
                        a.choose(a.global_locale_control(), "EN")
                        trace["english_gui_selector_restored"] = True
                    except Exception as exc:
                        restore_error = exc
                if restore_error:
                    trace["restoration_error"] = (
                        f"{type(restore_error).__name__}: {restore_error}"
                    )
                trace["last_completed_phase"] = phase
                (root / "arabic-transition.json").write_text(
                    json.dumps(trace, indent=2, ensure_ascii=False) + "\n",
                    encoding="utf-8",
                )
                # Preserve the original (often a stopped process or SIGSEGV)
                # rather than replacing it with a less useful cleanup error.
                if original_error is None and restore_error is not None:
                    raise NativeFailure(
                        "Arabic GUI restoration failed: " + str(restore_error)
                    ) from restore_error

            # Both native shell and page heading must return to English.
            reached("verify_english_studio")
            a = session.ax
            a.find("Studio", "button", timeout=8)
            a.heading("Studio", timeout=12)
            if a.heading_available("الاستوديو"):
                raise NativeFailure(
                    "Arabic Studio heading remains mounted after restoring English"
                )
            session.screenshot("english-restored")
            trace["english_heading_restored"] = True
            trace["last_completed_phase"] = "english_heading_verified"
            (root / "arabic-transition.json").write_text(
                json.dumps(trace, indent=2, ensure_ascii=False) + "\n",
                encoding="utf-8",
            )
            return (
                "Real Arabic nav and semantic heading, native graphical relaunch "
                "with retained Arabic locale, and complete English restoration"
            )
        if not checked(report, "arabic_locale_control", arabic):
            return

        # Run Arabic first: an independently observed Linux SIGSEGV during
        # dark-theme relaunch otherwise leaves the native UI inaccessible and
        # masks the real locale result with an unrelated missing-combo error.
        def theme():
            a = session.ax
            a.click("Settings", "button")
            # This build exposes System and Dark; there is no explicit Light
            # option. System cannot be called Light without observed OS state.
            a.choose("Theme", "System")
            a.click("Save changes", "button")
            a.assert_selected_option("Theme", "System")
            session.screenshot("system-theme")
            a.choose("Theme", "Dark")
            a.click("Save changes", "button")
            a.assert_selected_option("Theme", "Dark")
            session.screenshot("dark-theme")
            # Capture actual installed theme popup and native options for
            # independent contrast/stacking inspection. The System/Dark
            # persistence PASS and visual-review verdict stay separate.
            try:
                a.click("Theme", "combo")
                try:
                    a.find("System", "list item", timeout=5)
                    a.find("Dark", "list item", timeout=5)
                    report.setdefault("visual_capture", {})["dark_theme_menu"] = (
                        session.screenshot("dark-theme-menu-open")
                    )
                finally:
                    command("xdotool", "key", "--clearmodifiers", "Escape")
                    time.sleep(0.3)
            except Exception as capture_error:
                report.setdefault("visual_capture_errors", {})["dark_theme_menu"] = (
                    f"{type(capture_error).__name__}: {capture_error}"
                )
            session.stop()
            session.start()
            a = session.ax
            a.click("Settings", "button")
            a.assert_selected_option("Theme", "Dark")
            session.screenshot("dark-theme-reopened")
            return (
                "Real Settings System/Dark selection, saved changes, installed "
                "graphical restart and restored Theme control; screenshot evidence "
                "requires independent visual inspection"
            )
        # An actual process crash remains FAIL and must not be retried away.
        if not checked(report, "theme_mode_control", theme):
            return

        # This run deliberately has no fabricated downloads or media transcripts.
        # The following steps require source fixtures and dedicated adversarial UI inputs.
        status(report, "multi_source_transcript", "NOT TESTED",
               "Disposable profile has no real research/transcript sources; no mock material injected")
        status(report, "source_revocation", "NOT TESTED",
               "Needs a real source with revocable availability, verified after project reload")
        status(report, "wrong_project_ipc_denial", "NOT TESTED",
               "Native UI scoping is not an authorization probe for forged foreign IDs")
        def denied_url_recovery():
            # A genuine GUI request reaches production NetworkPolicy. An
            # unsupported file: URL is rejected before JobService enqueue.
            # No database seeding, debug bypass, or synthetic success.
            a = session.ax
            old_count = count_canonical_jobs(root / "data", name_a)
            a.click("Queue", "button")
            a.heading("Jobs", timeout=15)
            a.type_into("Media URL", "file:///scriptotar-native-qa-denied-media.mp4")
            a.click("Queue URL", "button")
            error_text = ""
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                for node in a.nodes():
                    try:
                        if "alert" not in node.getRoleName().lower():
                            continue
                        label = (node.name or "").strip()
                        if not label:
                            try:
                                label = node.queryText().getText(0, -1).strip()
                            except Exception:
                                pass
                        lower = label.lower()
                        if (len(label) > 8 and any(word in lower for word in
                                ("url", "scheme", "protocol", "unsupported",
                                 "invalid", "blocked", "not allowed"))):
                            error_text = label
                            break
                    except Exception:
                        continue
                if error_text:
                    break
                time.sleep(0.35)
            if not error_text:
                session.screenshot("failed-denied-url-no-error")
                raise NativeFailure(
                    "Unsupported URL was submitted through installed Queue, "
                    "but no native error alert was exposed"
                )
            session.screenshot("rejected-url-error")
            new_count = count_canonical_jobs(root / "data", name_a)
            if new_count != old_count:
                raise NativeFailure(
                    "Unsupported URL created a canonical job despite GUI rejection "
                    f"(before={old_count}, after={new_count})"
                )
            # Recover through ordinary user navigation and readback. Previously
            # edited immutable candidate rows must still hold the exact body.
            a.click("Studio", "button")
            a.heading("Studio")
            a.choose("Open study", study)
            if a.read_text("Script draft") != body:
                raise NativeFailure("Original native draft changed after denied URL")
            verify_canonical_reopened_candidates(
                root / "data", name_a, study, body, tag + "-copy"
            )
            session.screenshot("recovered-studio-after-denial")
            report["denied_url_error"] = error_text[:300]
            return (
                "Production Queue rejected unsupported URL with visible native alert, "
                "job count unchanged, and original Study/candidate text recovered"
            )
        def error_recovery():
            a = session.ax
            a.click("Studio", "button")
            if not a.available("Script candidates", "combo"):
                a.choose("Open study", study)
            a.type_into("Creative goal", "Describe practical original editing tips")
            a.type_into("Audience", "People learning video editing")
            a.click("Prepare 3 creative angles", "button")
            a.edit_field("Paste three-angle JSON response", timeout=12)
            a.type_into("Paste three-angle JSON response", '{"angles":[{"id":"only-one-angle","approach":"Invalid","hook":"Incomplete","promise":"Missing two angles","outline":[{"role":"hook","intent":"Should be rejected","estimated_seconds":7}],"evidence_ids":[]}]}')
            a.click("Validate 3 angles", "button")
            # Observe a visible native accessibility error, never infer a
            # backend rejection from a disabled button or malformed input.
            deadline = time.monotonic() + 12
            saw_error = False
            while time.monotonic() < deadline:
                for node in a.nodes():
                    try:
                        label = (node.name or "").lower()
                        if ("angle" in label and "invalid" in label
                                and node.getRoleName().lower() in
                                ("alert", "text", "static", "paragraph", "section")):
                            saw_error = True
                            break
                    except Exception:
                        continue
                if saw_error:
                    break
                time.sleep(0.3)
            if not saw_error:
                raise NativeFailure(
                    "Malformed three-angle JSON produced no visible native rejection"
                )
            session.screenshot("negative-angles-rejected")
            a.type_into("Paste three-angle JSON response", '{"angles":[{"id":"angle-recovery-problem","approach":"Explain an everyday problem","hook":"Start with the overlooked problem","promise":"Make the context clear","outline":[{"role":"hook","intent":"Introduce the everyday problem","estimated_seconds":7}],"evidence_ids":[]},{"id":"angle-recovery-contrast","approach":"Contrast two useful approaches","hook":"Compare two possible choices","promise":"Show a practical distinction","outline":[{"role":"hook","intent":"Open with an original contrast","estimated_seconds":7}],"evidence_ids":[]},{"id":"angle-recovery-checklist","approach":"Share a practical checklist","hook":"Three steps to get started","promise":"Give practical next actions","outline":[{"role":"hook","intent":"Introduce a practical checklist","estimated_seconds":7}],"evidence_ids":[]}]}')
            a.click("Validate 3 angles", "button")
            a.find("Share a practical checklist", timeout=12)
            # The original immutable script must remain intact. This local
            # response review cannot create a draft or send an AI request.
            if a.read_text("Script draft") != body:
                raise NativeFailure(
                    "Local angle recovery changed the original saved script"
                )
            verify_canonical_reopened_candidates(
                root / "data", name_a, study, body, tag + "-copy"
            )
            session.screenshot("three-angles-recovered")
            denied_url_recovery()
            return (
                "Real WebKitGTK malformed JSON rejected through public Rust IPC, "
                "three deterministic angles subsequently validated in installed "
                "Studio, unchanged canonical draft and immutable candidate versions; "
                "no provider execution"
            )
        if not checked(report, "error_recovery", error_recovery):
            return
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
        if (root / "process-exit.json").exists():
            (args.output / "process-exit.json").write_bytes(
                (root / "process-exit.json").read_bytes()
            )
            report["native_process_exit_evidence"] = "process-exit.json"
        if (root / "arabic-transition.json").exists():
            (args.output / "arabic-transition.json").write_bytes(
                (root / "arabic-transition.json").read_bytes()
            )
            report["native_arabic_transition_evidence"] = "arabic-transition.json"
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
