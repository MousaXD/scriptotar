from __future__ import annotations

import io
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scriptotar_sidecar.errors import SidecarError
from scriptotar_sidecar.protocol import ProtocolWriter
from scriptotar_sidecar.service import EngineSupervisor


class PackagedRuntimeTests(unittest.TestCase):
    def _supervisor(self) -> EngineSupervisor:
        return EngineSupervisor(ProtocolWriter(io.StringIO()), io.StringIO())

    def test_packaged_engine_executable_is_launched_directly(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            executable = Path(raw) / "scriptotar-engine"
            executable.write_bytes(b"fixture")
            with patch.dict(
                os.environ,
                {"SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE": str(executable)},
                clear=False,
            ):
                self.assertEqual(self._supervisor()._engine_command(), [str(executable.resolve())])

    def test_missing_packaged_engine_fails_explicitly(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            missing = Path(raw) / "missing-engine"
            with patch.dict(
                os.environ,
                {"SCRIPTOTAR_SIDECAR_ENGINE_EXECUTABLE": str(missing)},
                clear=False,
            ):
                with self.assertRaisesRegex(SidecarError, "Bundled transcription engine"):
                    self._supervisor()._engine_command()


if __name__ == "__main__":
    unittest.main()