from __future__ import annotations

import json
import shutil
import subprocess
import sys
from pathlib import Path

from scriptotar_sidecar.engine_worker import main as engine_main


def _tool_version(name: str) -> dict[str, str]:
    path = shutil.which(name)
    if not path:
        raise RuntimeError(f"{name} is not available on PATH")
    completed = subprocess.run(
        [path, "-version"],
        check=True,
        capture_output=True,
        text=True,
        timeout=15,
    )
    first_line = (completed.stdout or completed.stderr).splitlines()[0]
    return {"path": str(Path(path).resolve()), "version": first_line[:300]}


def self_test() -> int:
    import faster_whisper
    import yt_dlp

    report = {
        "ok": True,
        "python": sys.version.split()[0],
        "faster_whisper": getattr(faster_whisper, "__version__", "unknown"),
        "yt_dlp": yt_dlp.version.__version__,
        "ffmpeg": _tool_version("ffmpeg"),
        "ffprobe": _tool_version("ffprobe"),
    }
    sys.stdout.write(json.dumps(report, ensure_ascii=False, separators=(",", ":")) + "\n")
    sys.stdout.flush()
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        raise SystemExit(self_test())
    raise SystemExit(engine_main())
