#!/usr/bin/env python3
"""Score offline ASR runs against a local, human-verified reference corpus."""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
import sys
import unicodedata
from pathlib import Path
from typing import Any


NORMALIZATION_POLICY = (
    "Unicode NFC, case-fold, replace punctuation with spaces, preserve digits, "
    "and collapse whitespace. No numeral or word substitutions."
)


def normalize(text: str) -> str:
    normalized = unicodedata.normalize("NFC", text).casefold()
    characters = [
        " " if unicodedata.category(character).startswith("P") else character
        for character in normalized
    ]
    return " ".join("".join(characters).split())


def word_error_count(reference: str, hypothesis: str) -> tuple[int, int]:
    """Return word edit distance and reference word count using linear memory."""
    reference_tokens = normalize(reference).split()
    hypothesis_tokens = normalize(hypothesis).split()
    reference_word_count = len(reference_tokens)
    if len(reference_tokens) < len(hypothesis_tokens):
        reference_tokens, hypothesis_tokens = hypothesis_tokens, reference_tokens

    previous = list(range(len(hypothesis_tokens) + 1))
    for row, reference_token in enumerate(reference_tokens, start=1):
        current = [row]
        for column, hypothesis_token in enumerate(hypothesis_tokens, start=1):
            substitution_cost = 0 if reference_token == hypothesis_token else 1
            current.append(
                min(
                    current[column - 1] + 1,
                    previous[column] + 1,
                    previous[column - 1] + substitution_cost,
                )
            )
        previous = current
    return previous[-1], reference_word_count


def character_error_counts(reference: str, hypothesis: str) -> tuple[int, int]:
    reference_characters = list(normalize(reference))
    hypothesis_characters = list(normalize(hypothesis))
    reference_length = len(reference_characters)
    if len(reference_characters) < len(hypothesis_characters):
        reference_characters, hypothesis_characters = hypothesis_characters, reference_characters
    previous = list(range(len(hypothesis_characters) + 1))
    for row, reference_character in enumerate(reference_characters, start=1):
        current = [row]
        for column, hypothesis_character in enumerate(hypothesis_characters, start=1):
            substitution_cost = 0 if reference_character == hypothesis_character else 1
            current.append(
                min(
                    current[column - 1] + 1,
                    previous[column] + 1,
                    previous[column - 1] + substitution_cost,
                )
            )
        previous = current
    return previous[-1], reference_length


def _required(value: dict[str, Any], key: str, context: str) -> str:
    result = value.get(key)
    if not isinstance(result, str) or not result.strip():
        raise ValueError(f"{context}: '{key}' must be a non-empty string")
    return result


def load_references(path: Path) -> dict[str, dict[str, Any]]:
    references: dict[str, dict[str, Any]] = {}
    with path.open("r", encoding="utf-8-sig", newline="") as source:
        reader = csv.DictReader(source)
        required_columns = {"recording_id", "category", "reference_text", "audio_seconds"}
        if reader.fieldnames is None or not required_columns.issubset(reader.fieldnames):
            raise ValueError(
                "reference CSV requires columns: recording_id, category, "
                "reference_text, audio_seconds"
            )
        for line_number, row in enumerate(reader, start=2):
            context = f"{path}:{line_number}"
            recording_id = _required(row, "recording_id", context)
            category = _required(row, "category", context)
            reference_text = _required(row, "reference_text", context)
            if recording_id in references:
                raise ValueError(f"{context}: duplicate recording_id '{recording_id}'")
            try:
                audio_seconds = float(row["audio_seconds"])
            except (TypeError, ValueError) as error:
                raise ValueError(f"{context}: audio_seconds must be a positive number") from error
            if not math.isfinite(audio_seconds) or audio_seconds <= 0:
                raise ValueError(f"{context}: audio_seconds must be a positive number")
            if not normalize(reference_text):
                raise ValueError(f"{context}: reference_text is empty after normalization")
            references[recording_id] = {
                "category": category,
                "reference_text": reference_text,
                "audio_seconds": audio_seconds,
            }
    if not references:
        raise ValueError("reference CSV contains no recordings")
    return references


def load_hypotheses(path: Path, references: dict[str, dict[str, Any]]) -> list[dict[str, Any]]:
    hypotheses: list[dict[str, Any]] = []
    seen: set[tuple[str, str, str]] = set()
    with path.open("r", encoding="utf-8") as source:
        for line_number, line in enumerate(source, start=1):
            if not line.strip():
                continue
            context = f"{path}:{line_number}"
            try:
                item = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"{context}: invalid JSON: {error.msg}") from error
            if not isinstance(item, dict):
                raise ValueError(f"{context}: each JSONL entry must be an object")
            recording_id = _required(item, "recording_id", context)
            engine = _required(item, "engine", context)
            run = _required(item, "run", context)
            status = item.get("status", "completed")
            if not isinstance(status, str) or status not in {"completed", "failed"}:
                raise ValueError(f"{context}: status must be 'completed' or 'failed'")
            if recording_id not in references:
                raise ValueError(f"{context}: unknown recording_id '{recording_id}'")
            key = (recording_id, engine, run)
            if key in seen:
                raise ValueError(f"{context}: duplicate recording/engine/run entry")
            seen.add(key)

            transcript = item.get("transcript", "")
            if status == "completed" and not isinstance(transcript, str):
                raise ValueError(f"{context}: transcript must be a string")
            if status == "failed" and transcript is None:
                transcript = ""
            if not isinstance(transcript, str):
                raise ValueError(f"{context}: transcript must be a string")

            row: dict[str, Any] = {
                "recording_id": recording_id,
                "engine": engine,
                "run": run,
                "status": status,
                "transcript": transcript,
                "reference": references[recording_id],
            }
            for metric in ("elapsed_seconds", "peak_memory_mib", "model_load_seconds"):
                metric_value = item.get(metric)
                if metric_value is None:
                    row[metric] = None
                    continue
                if not isinstance(metric_value, (int, float)) or not math.isfinite(metric_value) or metric_value < 0:
                    raise ValueError(f"{context}: {metric} must be a non-negative number")
                row[metric] = float(metric_value)
            hypotheses.append(row)
    if not hypotheses:
        raise ValueError("hypothesis JSONL contains no runs")
    return hypotheses


def _ratio(errors: int, total: int) -> float | None:
    return errors / total if total else None


def _median(values: list[float]) -> float | None:
    return statistics.median(values) if values else None


def _score_group(rows: list[dict[str, Any]], expected_count: int) -> dict[str, Any]:
    completed = [row for row in rows if row["status"] == "completed"]
    word_errors = reference_words = 0
    character_errors = reference_characters = 0
    rtfs: list[float] = []

    for row in completed:
        reference = row["reference"]["reference_text"]
        hypothesis = row["transcript"]
        errors, words = word_error_count(reference, hypothesis)
        char_errors, characters = character_error_counts(reference, hypothesis)
        word_errors += errors
        reference_words += words
        character_errors += char_errors
        reference_characters += characters
        if row["elapsed_seconds"] is not None:
            rtfs.append(row["elapsed_seconds"] / row["reference"]["audio_seconds"])

    return {
        "recordings_expected": expected_count,
        "runs_present": len(rows),
        "runs_completed": len(completed),
        "completion_rate": _ratio(len(completed), expected_count),
        "wer": _ratio(word_errors, reference_words),
        "cer": _ratio(character_errors, reference_characters),
        "word_errors": {
            "total": word_errors,
            "reference_words": reference_words,
        },
        "median_rtf": _median(rtfs),
        "median_elapsed_seconds": _median(
            [row["elapsed_seconds"] for row in rows if row["elapsed_seconds"] is not None]
        ),
        "median_model_load_seconds": _median(
            [row["model_load_seconds"] for row in rows if row["model_load_seconds"] is not None]
        ),
        "median_peak_memory_mib": _median(
            [row["peak_memory_mib"] for row in rows if row["peak_memory_mib"] is not None]
        ),
    }


def score(references: dict[str, dict[str, Any]], hypotheses: list[dict[str, Any]]) -> dict[str, Any]:
    engines = sorted({row["engine"] for row in hypotheses})
    all_categories = sorted({row["category"] for row in references.values()})
    report: dict[str, Any] = {
        "schema_version": 1,
        "normalization_policy": NORMALIZATION_POLICY,
        "dataset_recordings": len(references),
        "engines": {},
    }

    for engine in engines:
        rows = [row for row in hypotheses if row["engine"] == engine]
        runs = sorted({row["run"] for row in rows})
        categories: dict[str, Any] = {}
        for category in all_categories:
            category_rows = [
                row
                for row in rows
                if row["reference"]["category"] == category
            ]
            recordings_in_category = sum(
                1 for reference in references.values() if reference["category"] == category
            )
            categories[category] = _score_group(
                category_rows, recordings_in_category * len(runs)
            )
        report["engines"][engine] = {
            **_score_group(rows, len(references) * len(runs)),
            "run_ids": runs,
            "categories": categories,
        }
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--references", required=True, type=Path, help="Human-verified reference CSV")
    parser.add_argument("--hypotheses", required=True, type=Path, help="Engine results in JSON Lines format")
    parser.add_argument("--output", required=True, type=Path, help="JSON report output path")
    args = parser.parse_args(argv)

    try:
        references = load_references(args.references)
        hypotheses = load_hypotheses(args.hypotheses, references)
        report = score(references, hypotheses)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    except (OSError, ValueError) as error:
        print(f"transcription benchmark: {error}", file=sys.stderr)
        return 2
    print(f"Wrote benchmark report to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
