#!/usr/bin/env python3
from __future__ import annotations

import os
import sys
import traceback

from scriptotar_sidecar.service import SidecarService


def main() -> int:
    if os.name == "posix":
        os.umask(0o077)
    return SidecarService(sys.stdout, sys.stderr).run(sys.stdin)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except KeyboardInterrupt:
        raise SystemExit(130)
    except SystemExit:
        raise
    except BaseException:
        traceback.print_exc(file=sys.stderr)
        raise SystemExit(70)
