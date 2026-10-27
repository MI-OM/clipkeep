import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import { AudioLines, Download, FileAudio, LoaderCircle, Sparkles } from "lucide-react";

interface TranscriptionModel { id: string; name: string; description: string; downloadMb: number; minimumRamGb: number; recommendedRamGb: number; minimumAvailableRamMb: number; installed: boolean; recommended: boolean; canRunNow: boolean; license: string; }
interface TranscriptionStatus { models: TranscriptionModel[]; totalRamMb: number; availableRamMb: number; cpuThreads: number; recommendedModelId: string; }
interface TranscriptResult { sourcePath: string; title: string; text: string; durationSeconds: number; elapsedSeconds: number; modelName: string; }
interface ModelInstallProgress { file: string; phase: string; percent: number | null; }

export function TranscribePage() {
  const desktopRuntime = isTauri();
  const [status, setStatus] = useState<TranscriptionStatus | null>(null);
  const [selectedModelId, setSelectedModelId] = useState("");
  const [sourcePath, setSourcePath] = useState("");
  const [result, setResult] = useState<TranscriptResult | null>(null);
  const [transcript, setTranscript] = useState("");
  const [busy, setBusy] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState<ModelInstallProgress | null>(null);
  const [notice, setNotice] = useState("");

  useEffect(() => {
    if (!desktopRuntime) return;
    invoke<TranscriptionStatus>("get_transcription_status").then((next) => { setStatus(next); setSelectedModelId((current) => current || next.recommendedModelId); }).catch(() => setNotice("Could not check transcription model status."));
  }, [desktopRuntime]);

  async function refreshStatus() {
    try { const next = await invoke<TranscriptionStatus>("get_transcription_status"); setStatus(next); setNotice("System memory checked."); }
    catch { setNotice("Could not refresh system memory."); }
  }

  async function chooseMedia() {
    try {
      const selected = await open({ multiple: false, title: "Choose audio or video to transcribe", filters: [{ name: "Audio and video", extensions: ["mp3", "wav", "m4a", "aac", "flac", "ogg", "opus", "mp4", "mkv", "mov", "webm"] }] });
      if (typeof selected === "string") { setSourcePath(selected); setResult(null); setTranscript(""); setNotice(""); }
    } catch { setNotice("Could not open the file picker."); }
  }

  async function startTranscription() {
    if (!sourcePath) return;
    setBusy(true); setNotice("ClipKeep is preparing audio and loading the selected local model. Longer files may take a while.");
    try {
      const completed = await invoke<TranscriptResult>("transcribe_file", { sourcePath, modelId: selectedModelId });
      setResult(completed); setTranscript(completed.text);
      setNotice(completed.text ? "Transcription complete. Review the text before using important names, numbers, or quotations." : "No speech was recognized in this file.");
    } catch (error) { setNotice(getInvokeErrorMessage(error, "Transcription failed.")); }
    finally { setBusy(false); }
  }

  async function installModel() {
    const selectedModel = status?.models.find((model) => model.id === selectedModelId);
    setInstalling(true); setNotice(`Downloading ${selectedModel?.name ?? "the transcription model"}. It stays on this device.`);
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listen<ModelInstallProgress>("transcription-model-progress", ({ payload }) => { setInstallProgress(payload); setNotice(payload.phase === "verified" ? `${payload.file} verified.` : `Downloading ${payload.file} · ${payload.percent ?? 0}%`); });
      const installed = await invoke<TranscriptionStatus>("install_transcription_model", { modelId: selectedModelId });
      setStatus(installed); setInstallProgress(null); setNotice("Transcription model installed. Choose a file to begin.");
    } catch (error) { setNotice(getInvokeErrorMessage(error, "Could not install the transcription model.")); }
    finally { unlisten?.(); setInstalling(false); }
  }

  async function exportText() {
    try {
      const path = await save({ title: "Save transcript", defaultPath: `${result?.title ?? "transcript"}.txt`, filters: [{ name: "Text transcript", extensions: ["txt"] }] });
      if (!path) return;
      await invoke("save_transcript", { request: { path, text: transcript } }); setNotice("Transcript saved.");
    } catch (error) { setNotice(getInvokeErrorMessage(error, "Could not save the transcript.")); }
  }

  const selectedModel = status?.models.find((model) => model.id === selectedModelId);
  return <div className="content-page transcription-page">
    <div className="page-intro"><div><p className="section-kicker">PRIVATE, LOCAL SPEECH TO TEXT</p><h1>Transcribe a file</h1><p className="page-subtitle">Turn audio or video you already have into editable text on this device.</p></div><span className="page-icon-badge"><AudioLines size={20} /></span></div>
    <section className="transcription-card">
      <div className="transcription-engine-row"><span className="transcription-engine-icon"><Sparkles size={17} /></span><div><h2>Choose a transcription model</h2><p>{status ? `${(status.totalRamMb / 1024).toFixed(0)} GB RAM · ${status.cpuThreads} logical CPU threads · ${status.availableRamMb} MB free` : "Checking this device..."}</p></div>{status && <button className="secondary-button" type="button" onClick={refreshStatus} disabled={busy || installing}>Refresh</button>}</div>
      <div className="transcription-model-list" role="radiogroup" aria-label="Transcription model">{status?.models.map((model) => <label className={`transcription-model-option${selectedModelId === model.id ? " selected" : ""}`} key={model.id}><input type="radio" name="transcription-model" value={model.id} checked={selectedModelId === model.id} onChange={() => setSelectedModelId(model.id)} disabled={busy || installing} /><span className="transcription-model-copy"><strong>{model.name} {model.recommended && <em>Recommended</em>}</strong><small>{model.description}</small><small>{model.downloadMb} MB download - {model.minimumRamGb} GB minimum RAM - {model.recommendedRamGb} GB recommended - {model.license} - {model.installed ? "Installed" : "Not installed"}</small>{!model.canRunNow && <small className="transcription-memory-warning">{model.id === "parakeet-tdt-ctc-110m-int8" ? "Starting is disabled on this device while this model crash is investigated." : `Needs ${model.minimumAvailableRamMb} MB free; currently ${status.availableRamMb} MB available. Close apps and refresh.`}</small>}</span></label>)}</div>
      {selectedModel && desktopRuntime && !selectedModel.installed && <button className="secondary-button" type="button" onClick={installModel} disabled={installing}>{installing ? "Setting up…" : `Install model · ${selectedModel.downloadMb} MB`}</button>}
      <p className="transcription-license">Speech recognition runs on this device. Model files download only after you choose Install.</p>
      <button className="transcription-file-picker" type="button" onClick={chooseMedia} disabled={!desktopRuntime || busy}><FileAudio size={20} /><span><strong>{sourcePath ? sourcePath.split(/[\\/]/).pop() : "Choose an audio or video file"}</strong><small>{sourcePath || "MP3, WAV, M4A, FLAC, MP4, MKV, MOV, WebM and more"}</small></span><span className="secondary-button transcription-browse-label">Browse files</span></button>
      {!desktopRuntime && <p className="transcription-note">Open the rebuilt desktop app to transcribe files locally.</p>}
      <button className="primary-button transcription-start" type="button" onClick={startTranscription} disabled={!desktopRuntime || !selectedModel?.installed || !selectedModel.canRunNow || !sourcePath || busy}>{busy ? <><LoaderCircle className="transcription-spinner" size={16} /> Transcribing locally…</> : "Start transcription"}</button>
      {installing && installProgress?.percent !== null && installProgress?.percent !== undefined && <div className="transcription-progress" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={installProgress.percent}><span style={{ width: `${installProgress.percent}%` }} /></div>}
      {notice && <p className="transcription-notice" role="status">{notice}</p>}
    </section>
    {result && <section className="transcript-result-card"><div className="transcript-result-heading"><div><p className="section-kicker">TRANSCRIPT</p><h2>{result.title}</h2><p>{formatDuration(result.durationSeconds)} audio · finished in {result.elapsedSeconds.toFixed(1)} seconds · {result.modelName}</p></div><button className="secondary-button" type="button" onClick={exportText}><Download size={15} /> Save TXT</button></div><label className="transcript-editor-label" htmlFor="transcript-editor">Edit transcript</label><textarea id="transcript-editor" className="transcript-editor" value={transcript} onChange={(event) => setTranscript(event.currentTarget.value)} placeholder="Recognized speech will appear here." /><p className="transcription-note">Machine transcription can make mistakes. Check quotations, names, and numbers.</p></section>}
  </div>;
}

function formatDuration(seconds: number) { const minutes = Math.floor(seconds / 60); const remainingSeconds = Math.floor(seconds % 60); return minutes ? `${minutes} min ${remainingSeconds} sec` : `${remainingSeconds} sec`; }

function getInvokeErrorMessage(error: unknown, fallback: string) {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object") {
    const detail = error as { message?: unknown; code?: unknown };
    if (typeof detail.message === "string") return detail.message;
    if (typeof detail.code === "string") return `Transcription error (${detail.code}).`;
  }
  return fallback;
}
