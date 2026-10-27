# Transcription benchmark scorer

This directory contains a local benchmark runner and scorer for CPU speech recognition.
The runner compares Parakeet TDT INT8 through sherpa-onnx with Whisper through
faster-whisper. Both receive identical FFmpeg-decoded 16 kHz mono PCM audio; the scorer
compares their outputs against a human-verified reference corpus.

Keep source audio, reference transcripts, and raw model output local. Do not commit them
unless the recording rights and speaker consent explicitly permit redistribution.

## Reference CSV

Use UTF-8 CSV with these columns:

```csv
recording_id,category,reference_text,audio_seconds,audio_file
lecture_001,lecture,"The verified words spoken in this segment.",121.4,lecture_001.wav
accent_001,accent,"Another human-verified reference.",304.0,accent_001.wav
```

`recording_id` must be unique. Categories should follow the corpus plan (clean speech,
lecture/monologue, conversation, accent, noisy, or names/numbers). Reference text must
be human verified; do not use an ASR output as ground truth. `audio_file` is optional;
when omitted, the runner looks for `<recording_id>.wav` under `--audio-dir`. Paths must
remain within that directory. Keep this CSV and the recordings private and local.

## Candidate runners

Create a separate Python environment for each candidate and install the pinned versions
you intend to compare. The runner does not download model weights: `--model-dir` must
point to a previously obtained local model snapshot. Record `python --version`,
`pip freeze`, model revision and model-file hashes alongside the benchmark report.
The Parakeet model also has its own licence and attribution requirements; review those
before redistributing any weights.

Install the runtime matching the candidate in that environment (`faster-whisper` for
Whisper, or `sherpa-onnx` for Parakeet). Install `psutil` to sample peak RSS across the
runner's process tree; without it, memory is recorded as `null`. FFmpeg must be installed
and its executable path supplied explicitly.

Example PowerShell runs (replace paths with your local corpus and model snapshot):

```powershell
python tools/transcription_bench/run_engine.py `
  --engine faster-whisper `
  --engine-label whisper-medium-en-int8 `
  --references tools/transcription_bench/data/references.csv `
  --audio-dir tools/transcription_bench/data/audio `
  --model-dir C:\models\whisper-medium-en-ct2 `
  --ffmpeg C:\ClipKeep\dependencies\ffmpeg.exe `
  --output tools/transcription_bench/runs/whisper.jsonl `
  --run cold-1

python tools/transcription_bench/run_engine.py `
  --engine parakeet-sherpa `
  --engine-label parakeet-tdt-0.6b-v2-int8 `
  --references tools/transcription_bench/data/references.csv `
  --audio-dir tools/transcription_bench/data/audio `
  --model-dir C:\models\sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8 `
  --ffmpeg C:\ClipKeep\dependencies\ffmpeg.exe `
  --output tools/transcription_bench/runs/parakeet.jsonl `
  --run cold-1
```

The runner loads each model once per process, then records every recording's output,
elapsed time, model load time and sampled peak process-tree memory. Start a fresh process
for each repeat. A failed audio conversion or inference creates a `failed` scorer row and
does not stop later recordings. Ctrl+C leaves already written rows intact. Do not combine
repeats in one JSONL output unless their `--run` labels differ.

If a locally sourced package's API changes, pin and document the tested runtime version;
do not silently change candidate settings between runs. The current adapters use CPU
INT8 Whisper with English transcription, beam size 5 and no VAD, and greedy CPU Parakeet
TDT decoding.

## Hypotheses JSONL

Write one JSON object per engine, recording, and repeat:

```json
{"recording_id":"lecture_001","engine":"candidate-name","run":"cold-1","status":"completed","transcript":"The words recognized by the engine.","elapsed_seconds":48.2,"model_load_seconds":7.1,"peak_memory_mib":1830}
{"recording_id":"lecture_001","engine":"candidate-name","run":"cold-2","status":"failed","elapsed_seconds":13.0,"model_load_seconds":6.8,"peak_memory_mib":2020}
```

Only `recording_id`, `engine`, and `run` are required identifiers. `status` defaults to
`completed`; failed runs count against completion rate and do not contribute to WER/CER.
Resource fields are optional non-negative numbers. A runner should record peak memory for
the whole engine process tree where possible, not only the launcher process.

## Run

```powershell
python tools/transcription_bench/score.py `
  --references path\to\references.csv `
  --hypotheses path\to\hypotheses.jsonl `
  --output path\to\report.json
```

WER and CER are aggregated from edit counts, not averaged per clip. The report also
includes category results, completion rate, median real-time factor, median elapsed time,
model load time, and peak memory. RTF is elapsed inference time divided by audio duration.

Normalization is deliberately modest and fixed: Unicode NFC, case folding, punctuation
to spaces, digits preserved, and whitespace collapsed. It does not rewrite numbers or
words. The normalization policy is recorded in every report. Review category-level WER
and critical names/numbers errors before selecting a default model; speed alone is not a
quality decision.
