from __future__ import annotations

import sys
import types
import unittest
from pathlib import Path
from unittest.mock import patch

from scriptotar_sidecar import engine
from scriptotar_sidecar.errors import SidecarError


def fake_whisper(*, lazy_cuda_failure: str | None = None, cpu_failure: str | None = None, init_cuda_failure: bool = False):
    """Exercise real transcribe_media control flow without a model download."""
    module = types.ModuleType("faster_whisper")
    created: list[str] = []

    class WhisperModel:
        def __init__(self, name: str, *, device: str, compute_type: str):
            created.append(device)
            self.device = device
            if device == "cuda" and init_cuda_failure:
                raise RuntimeError("libcublas.so.12 cannot be loaded")

        def transcribe(self, _media: str, **_kwargs):
            def segments():
                if self.device == "cuda":
                    # The GPU may fail *after* it has returned a partial result.
                    yield types.SimpleNamespace(start=0.0, end=1.0, text="GPU partial result", words=[])
                    if lazy_cuda_failure:
                        raise RuntimeError(lazy_cuda_failure)
                else:
                    if cpu_failure:
                        raise RuntimeError(cpu_failure)
                    yield types.SimpleNamespace(start=0.0, end=1.0, text="CPU final result", words=[])

            return segments(), types.SimpleNamespace(language="en", language_probability=0.99)

    class BatchedInferencePipeline:
        def __init__(self, *, model):
            self.model = model

        def transcribe(self, media: str, **kwargs):
            return self.model.transcribe(media, **kwargs)

    module.WhisperModel = WhisperModel
    module.BatchedInferencePipeline = BatchedInferencePipeline
    return module, created


class CudaFallbackTests(unittest.TestCase):
    def setUp(self) -> None:
        self.events: list[tuple[str, dict]] = []
        self.media = Path("/tmp/fixture.mp4")  # Fake model never opens this path.
        self.metadata = {"duration_seconds": 1.0}

    def emit(self, event: str, payload: dict) -> None:
        self.events.append((event, payload))

    def test_auto_uses_cpu_if_nvidia_driver_exists_but_cuda_libraries_are_missing(self):
        cache = engine.WhisperCache()
        with (
            patch.object(engine.shutil, "which", return_value="/usr/bin/nvidia-smi"),
            patch.object(engine.subprocess, "run"),
            patch.object(engine.ctypes, "CDLL", side_effect=OSError("libcublas.so.12 not found")) as loader,
        ):
            self.assertEqual(cache.choose_device("auto"), ("cpu", "int8"))
            if engine.os.name == "nt" or engine.sys.platform.startswith("linux"):
                loader.assert_called()
            self.assertEqual(cache.choose_device("cpu"), ("cpu", "int8"))
            self.assertEqual(cache.choose_device("cuda"), ("cuda", "int8_float16"))

    def test_auto_chooses_gpu_only_if_driver_and_cuda_runtime_are_ready(self):
        cache = engine.WhisperCache()
        with (
            patch.object(engine.shutil, "which", return_value="/usr/bin/nvidia-smi"),
            patch.object(engine.subprocess, "run"),
            patch.object(engine.ctypes, "CDLL") as loader,
        ):
            self.assertEqual(cache.choose_device("auto"), ("cuda", "int8_float16"))
            self.assertEqual(loader.call_count, 3)

    def test_auto_recovers_from_lazy_cuda_failure_without_duplicate_segments(self):
        for batched in (False, True):
            with self.subTest(batched=batched):
                module, created = fake_whisper(lazy_cuda_failure="RuntimeError: Library libcublas.so.12 not found")
                cache = engine.WhisperCache()
                with (
                    patch.dict(sys.modules, {"faster_whisper": module}),
                    patch.object(engine, "MODEL_CACHE", cache),
                    patch.object(engine.shutil, "which", return_value="/usr/bin/nvidia-smi"),
                    patch.object(engine.subprocess, "run"),
                    patch.object(cache, "cuda_runtime_available", return_value=True),
                ):
                    result = engine.transcribe_media(
                        {"device": "auto", "model": "small", "batched": batched},
                        self.media,
                        self.metadata,
                        self.emit,
                    )
                    self.assertEqual(result["device"], "cpu")
                    self.assertEqual(result["compute_type"], "int8")
                    self.assertEqual(result["text"], "CPU final result")
                    self.assertEqual(len(result["segments"]), 1)
                    self.assertTrue(cache.auto_cuda_disabled)
                    self.assertEqual(created, ["cuda", "cpu"])

                    # An additional job in this engine process must reuse CPU.
                    second = engine.transcribe_media(
                        {"device": "auto", "model": "small"}, self.media, self.metadata, self.emit
                    )
                    self.assertEqual(second["device"], "cpu")
                    self.assertEqual(created, ["cuda", "cpu"])
                    self.assertTrue(any("Retrying on CPU" in data.get("message", "") for _, data in self.events))

    def test_auto_recovers_when_cuda_model_initialization_fails(self):
        module, created = fake_whisper(init_cuda_failure=True)
        cache = engine.WhisperCache()
        with (
            patch.dict(sys.modules, {"faster_whisper": module}),
            patch.object(engine, "MODEL_CACHE", cache),
            patch.object(engine.shutil, "which", return_value="/usr/bin/nvidia-smi"),
            patch.object(engine.subprocess, "run"),
            patch.object(cache, "cuda_runtime_available", return_value=True),
        ):
            result = engine.transcribe_media({"device": "auto"}, self.media, self.metadata, self.emit)
        self.assertEqual(result["device"], "cpu")
        self.assertTrue(cache.auto_cuda_disabled)
        self.assertEqual(created, ["cuda", "cpu"])

    def test_explicit_cuda_does_not_silently_fall_back(self):
        module, created = fake_whisper(lazy_cuda_failure="Library libcublas.so.12 is not found")
        cache = engine.WhisperCache()
        with patch.dict(sys.modules, {"faster_whisper": module}), patch.object(engine, "MODEL_CACHE", cache):
            with self.assertRaises(SidecarError) as raised:
                engine.transcribe_media({"device": "cuda"}, self.media, self.metadata, self.emit)
        self.assertEqual(raised.exception.code, "CUDA_RUNTIME_UNAVAILABLE")
        self.assertIn("CPU or Auto", raised.exception.message)
        self.assertEqual(created, ["cuda"])
        self.assertFalse(cache.auto_cuda_disabled)

    def test_non_cuda_errors_are_not_hidden_by_cpu_fallback(self):
        module, created = fake_whisper(lazy_cuda_failure="unrelated storage corruption")
        cache = engine.WhisperCache()
        with (
            patch.dict(sys.modules, {"faster_whisper": module}),
            patch.object(engine, "MODEL_CACHE", cache),
            patch.object(engine.shutil, "which", return_value="/usr/bin/nvidia-smi"),
            patch.object(engine.subprocess, "run"),
            patch.object(cache, "cuda_runtime_available", return_value=True),
        ):
            with self.assertRaisesRegex(RuntimeError, "storage corruption"):
                engine.transcribe_media({"device": "auto"}, self.media, self.metadata, self.emit)
        self.assertEqual(created, ["cuda"])
        self.assertFalse(cache.auto_cuda_disabled)

    def test_cpu_errors_are_not_caught_by_cuda_fallback(self):
        module, created = fake_whisper(cpu_failure="CPU decoder failed")
        cache = engine.WhisperCache()
        with patch.dict(sys.modules, {"faster_whisper": module}), patch.object(engine, "MODEL_CACHE", cache):
            with self.assertRaisesRegex(RuntimeError, "CPU decoder failed"):
                engine.transcribe_media({"device": "cpu"}, self.media, self.metadata, self.emit)
        self.assertEqual(created, ["cpu"])


if __name__ == "__main__":
    unittest.main()
