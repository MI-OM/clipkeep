#!/usr/bin/env python3
"""Run a local CPU ASR candidate over a reference CSV and write scorer JSONL."""

from __future__ import annotations

import argparse
import csv
import json
import os
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from typing import Any


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return parsed


def load_recordings(reference_path: Path, audio_dir: Path) -> list[dict[str, Any]]:
    recordings: list[dict[str, Any]] = []
    with reference_path.open("r", encoding="utf-8-sig", newline="") as source:
        reader = csv.DictReader(source)
        required = {"recording_id", "category", "reference_text", "audio_seconds"}
        if reader.fieldnames is None or not required.issubset(reader.fieldnames):
            raise ValueError(
                "reference CSV requires recording_id, category, reference_text, audio_seconds"
            )
        seen: set[str] = set()
        for line_number, row in enumerate(reader, start=2):
            recording_id = (row.get("recording_id") or "").strip()
            if not recording_id or recording_id in seen:
                raise ValueError(f"{reference_path}:{line_number}: invalid or duplicate recording_id")
            seen.add(recording_id)
            audio_name = (row.get("audio_file") or f"{recording_id}.wav").strip()
            audio_path = (audio_dir / audio_name).resolve()
            try:
                audio_path.relative_to(audio_dir.resolve())
            except ValueError as error:
                raise ValueError(f"{reference_path}:{line_number}: audio_file escapes audio directory") from error
            if not audio_path.is_file():
                raise ValueError(f"{reference_path}:{line_number}: audio file not found: {audio_path}")
            recordings.append({"recording_id": recording_id, "audio_path": audio_path})
    if not recordings:
        raise ValueError("reference CSV contains no recordings")
    return recordings


class ProcessTreeMemory:
    """Optional psutil sampler; unavailable memory data is emitted as null."""

    def __init__(self) -> None:
        try:
            import psutil  # type: ignore[import-not-found]
        except ImportError:
            self.psutil = None
            self.peak_mib: float | None = None
            return
        self.psutil = psutil
        self.process = psutil.Process(os.getpid())
        self.peak_mib = 0.0
        self.stopped = threading.Event()
        self.thread = threading.Thread(target=self._sample, name="memory-sampler", daemon=True)
        self.thread.start()

    def _sample(self) -> None:
        while not self.stopped.is_set():
            try:
                processes = [self.process, *self.process.children(recursive=True)]
                rss = sum(process.memory_info().rss for process in processes if process.is_running())
                self.peak_mib = max(self.peak_mib or 0.0, rss / (1024 * 1024))
            except (self.psutil.Error, OSError):
                pass
            self.stopped.wait(0.25)

    def close(self) -> float | None:
        if self.psutil is not None:
            self.stopped.set()
            self.thread.join(timeout=1)
            return self.peak_mib
        return None


def load_engine(args: argparse.Namespace) -> tuple[Any, float]:
    started = time.perf_counter()
    if args.engine == "faster-whisper":
        try:
            from faster_whisper import WhisperModel
        except ImportError as error:
            raise RuntimeError(
                "faster-whisper is not installed in this Python environment; see README.md"
            ) from error
        engine = WhisperModel(
            str(args.model_dir),
            device="cpu",
            compute_type="int8",
            cpu_threads=args.threads,
            num_workers=1,
            local_files_only=True,
        )
    else:
        try:
            import sherpa_onnx
        except ImportError as error:
            raise RuntimeError(
                "sherpa-onnx is not installed in this Python environment; see README.md"
            ) from error
        model = args.model_dir
        required = {
            "encoder": model / "encoder.int8.onnx",
            "decoder": model / "decoder.int8.onnx",
            "joiner": model / "joiner.int8.onnx",
            "tokens": model / "tokens.txt",
        }
        missing = [str(path) for path in required.values() if not path.is_file()]
        if missing:
            raise RuntimeError("Parakeet model directory is missing: " + ", ".join(missing))
        engine = sherpa_onnx.OfflineRecognizer.from_transducer(
            encoder=str(required["encoder"]),
            decoder=str(required["decoder"]),
            joiner=str(required["joiner"]),
            tokens=str(required["tokens"]),
            num_threads=args.threads,
            decoding_method="greedy_search",
            provider="cpu",
            model_type="nemo_transducer",
        )
    return engine, time.perf_counter() - started


def transcribe(engine: Any, engine_name: str, wav_path: Path) -> str:
    if engine_name == "faster-whisper":
        segments, _info = engine.transcribe(
            str(wav_path),
            language="en",
            task="transcribe",
            beam_size=5,
            vad_filter=False,
            condition_on_previous_text=True,
        )
        return " ".join(segment.text.strip() for segment in segments).strip()

    import sherpa_onnx

    samples, sample_rate = sherpa_onnx.read_wave(str(wav_path))
    stream = engine.create_stream()
    stream.accept_waveform(sample_rate, samples)
    engine.decode_stream(stream)
    return stream.result.text.strip()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", choices=("faster-whisper", "parakeet-sherpa"), required=True)
    parser.add_argument("--engine-label", help="Scorer name identifying the exact candidate/model revision")
    parser.add_argument("--references", type=Path, required=True)
    parser.add_argument("--audio-dir", type=Path, required=True)
    parser.add_argument("--model-dir", type=Path, required=True)
    parser.add_argument("--ffmpeg", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--run", default="cold-1")
    parser.add_argument("--threads", type=positive_int, default=max(1, (os.cpu_count() or 2) // 2))
    args = parser.parse_args()

    sampler = ProcessTreeMemory()
    try:
        args.references = args.references.resolve(strict=True)
        args.audio_dir = args.audio_dir.resolve(strict=True)
        args.model_dir = args.model_dir.resolve(strict=True)
        args.ffmpeg = args.ffmpeg.resolve(strict=True)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        recordings = load_recordings(args.references, args.audio_dir)
        engine, model_load_seconds = load_engine(args)
    except (OSError, ValueError, RuntimeError) as error:
        sampler.close()
        print(f"error: {error}", file=sys.stderr)
        return 2

    try:
        with args.output.open("a", encoding="utf-8", newline="\n") as output:
            for index, recording in enumerate(recordings, start=1):
                started = time.perf_counter()
                row: dict[str, Any] = {
                    "recording_id": recording["recording_id"],
                    "engine": args.engine_label or args.engine,
                    "run": args.run,
                    "status": "completed",
                    "transcript": "",
                    "model_load_seconds": model_load_seconds,
                }
                try:
                    with tempfile.TemporaryDirectory(prefix="clipkeep-asr-") as temporary:
                        wav_path = Path(temporary) / "input.wav"
                        conversion = subprocess.run(
                            [
                                str(args.ffmpeg), "-nostdin", "-v", "error", "-y",
                                "-i", str(recording["audio_path"]), "-map", "0:a:0",
                                "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", str(wav_path),
                            ],
                            capture_output=True,
                            text=True,
                            check=False,
                        )
                        if conversion.returncode != 0:
                            raise RuntimeError("FFmpeg could not prepare this audio file")
                        row["transcript"] = transcribe(engine, args.engine, wav_path)
                except Exception as error:  # Keep a failed row so the scorer counts completion accurately.
                    row["status"] = "failed"
                    row["error"] = f"{type(error).__name__}: {error}"[:500]
                row["elapsed_seconds"] = time.perf_counter() - started
                row["peak_memory_mib"] = sampler.peak_mib
                output.write(json.dumps(row, ensure_ascii=False) + "\n")
                output.flush()
                print(
                    f"[{index}/{len(recordings)}] {recording['recording_id']}: {row['status']}",
                    flush=True,
                )
    except KeyboardInterrupt:
        print("Interrupted; completed rows were kept in the JSONL output.", file=sys.stderr)
        return 130
    finally:
        sampler.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
