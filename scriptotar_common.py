#!/usr/bin/env python3
from __future__ import annotations

import csv
import json
import os
import queue
import re
import shutil
import signal
import sqlite3
import subprocess
import sys
import threading
import time
import tkinter as tk
import uuid
from datetime import datetime
from pathlib import Path
from tkinter import filedialog, messagebox, simpledialog, ttk

from creator import (
    DEFAULT_MODELS, PROVIDERS, TASKS, build_prompt, clear_secret, estimate_speaking_seconds,
    format_duration, lookup_secret, normalize_research_item, request_ai, research_scan_command,
    secret_tool_available, store_secret,
)

APP = "Scriptotar"
VERSION = "1.2.0"
INSTALL = Path("/opt/scriptotar")
DATA = Path.home() / ".local" / "share" / "scriptotar"
LEGACY_DATA = Path.home() / ".local" / "share" / "wesamboss"
DEFAULT_OUTPUT = Path.home() / "Videos" / "Scriptotar"
LEGACY_DEFAULT_OUTPUT = Path.home() / "Videos" / "WesamBoss"
VENV = DATA / "venv"
VPY = VENV / "bin" / "python"
WORKER = INSTALL / "worker.py"
REQS = INSTALL / "requirements-engine.txt"
MARKER = VENV / ".scriptotar-engine-version"
SETTINGS_FILE = DATA / "settings.json"
DB_FILE = DATA / "history.sqlite3"
ENGINE_VERSION = "1.1.0-engine1"

BG = "#101217"
PANEL = "#191c22"
PANEL2 = "#22262e"
ENTRY = "#0c0e12"
TEXT = "#f3f5f7"
MUTED = "#a7afb9"
BORDER = "#343a44"
ACCENT = "#eceef1"
GOOD = "#8ad49f"
WARN = "#f2c97d"
BAD = "#ff9d9d"

DEFAULTS = {
    "output": str(DEFAULT_OUTPUT),
    "model": "medium",
    "device": "auto",
    "language": "auto",
    "quality": "720p",
    "cookies": "none",
    "max_duration": "60 min",
    "copy_source": True,
    "translate": False,
    "batched": False,
    "keep_failed": False,
    "project": "Inbox",
    "ai_mode": "Copy prompt only",
    "ai_provider": "OpenAI",
    "ai_model": "gpt-5.2",
    "ai_base_url": "",
    "auto_watch": False,
    "watch_interval": "60 min",
}


def migrate_legacy_branding() -> None:
    """Migrate WesamBoss 1.1 user data into Scriptotar on first launch."""
    try:
        DATA.parent.mkdir(parents=True, exist_ok=True)
        if not DATA.exists() and LEGACY_DATA.is_dir():
            try:
                LEGACY_DATA.rename(DATA)
            except OSError:
                shutil.copytree(LEGACY_DATA, DATA)

        legacy_marker = VENV / ".wesamboss-engine-version"
        if legacy_marker.is_file() and not MARKER.exists():
            MARKER.write_text(legacy_marker.read_text(encoding="utf-8"), encoding="utf-8")

        if LEGACY_DEFAULT_OUTPUT.is_dir() and not DEFAULT_OUTPUT.exists():
            try:
                LEGACY_DEFAULT_OUTPUT.rename(DEFAULT_OUTPUT)
            except OSError:
                # Moving across filesystems can fail. Leave the existing output in place
                # rather than copying potentially many gigabytes during app startup.
                pass
    except Exception:
        # Branding migration must never prevent the application from starting.
        pass


def load_settings() -> dict:
    try:
        data = json.loads(SETTINGS_FILE.read_text(encoding="utf-8"))
        legacy_output = str(LEGACY_DEFAULT_OUTPUT)
        if data.get("output") == legacy_output and DEFAULT_OUTPUT.exists():
            data["output"] = str(DEFAULT_OUTPUT)
        return {**DEFAULTS, **data}
    except Exception:
        return dict(DEFAULTS)


def max_duration_seconds(label: str) -> int:
    return {
        "30 min": 1800,
        "60 min": 3600,
        "2 hours": 7200,
        "Unlimited": 0,
    }.get(label, 3600)
