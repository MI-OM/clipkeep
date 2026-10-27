import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import {
  BadgeCheck,
  AudioLines,
  CircleHelp,
  Clapperboard,
  Download,
  Info,
  LayoutDashboard,
  Settings2,
  ShieldCheck,
} from "lucide-react";
import { AboutPage } from "./features/about/AboutPage";
import { DownloadsPage } from "./features/download/DownloadsPage";
import { HomePage } from "./features/download/HomePage";
import { SettingsPage } from "./features/settings/SettingsPage";
import { TranscribePage } from "./features/transcription/TranscribePage";
import { createDownloadService } from "./lib/download-service";
import { HISTORY_LIMIT, loadDownloadHistory, saveDownloadHistory } from "./lib/history";
import { loadSettings, saveSettings } from "./lib/settings";
import { validateVideoUrl } from "./lib/validation";
import { MAX_ACTIVE_DOWNLOADS } from "./types";
import type {
  DownloadJob,
  DownloadRequest,
  DownloadStatus,
  MediaMetadata,
  ProgressEvent,
  Settings,
} from "./types";
import "./App.css";

type Page = "home" | "downloads" | "transcription" | "settings" | "about";

const navItems: Array<{
  id: Page;
  label: string;
  icon: typeof LayoutDashboard;
}> = [
  { id: "home", label: "Save media", icon: LayoutDashboard },
  { id: "downloads", label: "Downloads", icon: Download },
  { id: "transcription", label: "Transcribe", icon: AudioLines },
  { id: "settings", label: "Settings", icon: Settings2 },
  { id: "about", label: "About & licences", icon: Info },
];

function updateJob(jobs: DownloadJob[], id: string, update: Partial<DownloadJob>) {
  return jobs.map((job) => (job.id === id ? { ...job, ...update } : job));
}

function statusAnnouncement(
  status: DownloadStatus,
  progress: number | null,
  desktopRuntime: boolean,
) {
  if (status === "downloading" && progress !== null) {
    return `${desktopRuntime ? "Download" : "Preview"} progress ${progress} percent`;
  }
  if (status === "failed") {
    return desktopRuntime
      ? "The download failed. No completed file was saved."
      : "Preview ended. No media was downloaded.";
  }
  if (status === "cancelled") {
    return desktopRuntime
      ? "The download was cancelled."
      : "Preview cancelled. No media was downloaded.";
  }
  if (status === "completed") {
    return "Download completed.";
  }
  return `${desktopRuntime ? "Download" : "Preview"} status: ${status.replace(/_/g, " ")}`;
}

function App() {
  const [page, setPage] = useState<Page>("home");
  const loadedSettings = useMemo(() => loadSettings(), []);
  const loadedHistory = useMemo(() => loadDownloadHistory(), []);
  const [settings, setSettings] = useState<Settings>(loadedSettings.settings);
  const [settingsNotice, setSettingsNotice] = useState(
    [loadedSettings.error, loadedHistory.error].filter(Boolean).join(" "),
  );
  const [jobs, setJobs] = useState<DownloadJob[]>(loadedHistory.jobs);
  const jobsRef = useRef(loadedHistory.jobs);
  const [announcement, setAnnouncement] = useState("");
  const [isStarting, setIsStarting] = useState(false);
  const [metadata, setMetadata] = useState<MediaMetadata | null>(null);
  const service = useMemo(() => createDownloadService(), []);
  const desktopRuntime = isTauri();
  const activeJobs = jobs.filter(
    (job) => !["completed", "failed", "cancelled"].includes(job.status),
  );
  const activeJob = activeJobs[0];

  useEffect(() => {
    const result = saveSettings(settings);
    setSettingsNotice(result ?? "");
  }, [settings]);

  useEffect(() => () => service.cancelAll(), [service]);

  const onStart = useCallback(
    async (request: DownloadRequest) => {
      setIsStarting(true);
      setSettingsNotice("");
      try {
        let jobId: string | null = null;
        const pendingEvents: ProgressEvent[] = [];
        const handleEvent = (event: ProgressEvent) => {
          if (jobId === null) {
            pendingEvents.push(event);
            return;
          }
          const updatedJobs = updateJob(jobsRef.current, event.jobId, {
            status: event.status,
            progressPercent: event.progressPercent,
            bytesPerSecond: event.bytesPerSecond,
            etaSeconds: event.etaSeconds,
            outputPath: event.outputPath,
            error: event.error,
          });
          jobsRef.current = updatedJobs;
          setJobs(updatedJobs);
          if (["completed", "failed", "cancelled"].includes(event.status)) {
            const historyError = saveDownloadHistory(updatedJobs);
            if (historyError) {
              setSettingsNotice(historyError);
            }
          }
          setAnnouncement(
            statusAnnouncement(event.status, event.progressPercent, desktopRuntime),
          );
        };
        const job = await service.start(request, handleEvent);
        jobId = job.id;
        const initialJob = pendingEvents.reduce(
          (current, event) =>
            event.jobId === job.id
              ? {
                  ...current,
                  status: event.status,
                  progressPercent: event.progressPercent,
                  bytesPerSecond: event.bytesPerSecond,
                  etaSeconds: event.etaSeconds,
                  outputPath: event.outputPath,
                  error: event.error,
                }
              : current,
          job,
        );
        const updatedJobs = [initialJob, ...jobsRef.current].slice(0, HISTORY_LIMIT);
        jobsRef.current = updatedJobs;
        setJobs(updatedJobs);
        const historyError = saveDownloadHistory(updatedJobs);
        if (historyError) {
          setSettingsNotice(historyError);
        }
        const latestEvent = [...pendingEvents]
          .reverse()
          .find((event) => event.jobId === job.id);
        setAnnouncement(latestEvent
          ? statusAnnouncement(latestEvent.status, latestEvent.progressPercent, desktopRuntime)
          : desktopRuntime
            ? "Download started on this device."
            : "Preview started. No media will be downloaded in this build.");
        setPage("downloads");
      } catch (error) {
        setSettingsNotice(
          error instanceof Error ? error.message : "The download could not be started.",
        );
      } finally {
        setIsStarting(false);
      }
    },
    [desktopRuntime, service],
  );

  const onCancel = useCallback(
    async (id: string) => {
      try {
        await service.cancel(id);
      } catch (error) {
        setSettingsNotice(
          error instanceof Error ? error.message : "The download could not be cancelled.",
        );
      }
    },
    [service],
  );

  const onPreview = useCallback(
    async (url: string) => {
      const validation = validateVideoUrl(url);
      if (!validation.valid) {
        throw new Error(validation.message);
      }
      const result = await service.getMetadata(validation.normalizedUrl);
      setMetadata(result);
      return result;
    },
    [service],
  );

  const onRetry = useCallback(
    (job: DownloadJob) => {
      setPage("home");
      setAnnouncement("Paste the original URL to start a new preview.");
      setSettings((current) => ({
        ...current,
        preferredFormat: job.format,
        preferredQuality: job.quality,
      }));
    },
    [],
  );

  const updateSettings = useCallback(
    (update: Partial<Settings>) => {
      setSettings((current) => ({ ...current, ...update }));
    },
    [],
  );

  const pageTitle = navItems.find((item) => item.id === page)?.label ?? "Save media";

  return (
    <div className="app-shell">
      <aside className="sidebar" aria-label="Primary navigation">
        <button className="brand brand-button" type="button" onClick={() => setPage("home")}>
          <span className="brand-mark" aria-hidden="true">
            <Clapperboard size={21} strokeWidth={2.2} />
          </span>
          <span className="brand-copy">
            <strong>clipkeep</strong>
            <small>LOCAL MEDIA TOOL</small>
          </span>
        </button>

        <div className="sidebar-label">WORKSPACE</div>
        <nav className="primary-nav">
          {navItems.map(({ id, label, icon: Icon }) => (
            <button
              className={`nav-link${page === id ? " is-active" : ""}`}
              key={id}
              type="button"
              aria-label={label}
              title={label}
              aria-current={page === id ? "page" : undefined}
              onClick={() => setPage(id)}
            >
              <Icon size={18} aria-hidden="true" />
              <span>{label}</span>
              {id === "downloads" && jobs.length > 0 && (
                <span className="nav-count">{jobs.length}</span>
              )}
            </button>
          ))}
        </nav>

        <div className="sidebar-spacer" />
        <div className="privacy-card">
          <div className="privacy-card-icon">
            <ShieldCheck size={17} aria-hidden="true" />
          </div>
          <div>
            <strong>Private by design</strong>
            <p>Your media stays on this device.</p>
          </div>
        </div>
        <div className="sidebar-footer">
          <span className="status-dot" />
          <span>{desktopRuntime ? "Desktop build" : "UI preview build"}</span>
          <span className="preview-tag">{desktopRuntime ? "LOCAL" : "MOCK"}</span>
        </div>
      </aside>

      <main className="main-panel">
        <header className="topbar">
          <div className="breadcrumbs">
            <span>Workspace</span>
            <span className="breadcrumb-slash">/</span>
            <strong>{pageTitle}</strong>
          </div>
          <div className="topbar-right">
            <span className="local-badge">
              <span className="status-dot" />
              Local only
            </span>
            <button
              className="icon-button help-button"
              type="button"
              aria-label="About ClipKeep"
              onClick={() => setPage("about")}
            >
              <CircleHelp size={18} />
            </button>
          </div>
        </header>

        <div className="page-content">
          <div className="prototype-banner" role="note">
            <Download size={16} aria-hidden="true" />
            <span>
              {desktopRuntime ? (
                <>
                  <strong>Media downloads need local tools.</strong> Install or verify FFmpeg and yt-dlp in Settings.
                </>
              ) : (
                <>
                  <strong>Browser preview.</strong> Downloads are simulated for interface
                  testing; no media is fetched or saved.
                </>
              )}
            </span>
          </div>

          {settingsNotice && (
            <div className="inline-notice notice-warning" role="status">
              <CircleHelp size={17} aria-hidden="true" />
              <span>{settingsNotice}</span>
              <button
                className="notice-dismiss"
                type="button"
                aria-label="Dismiss settings notice"
                onClick={() => setSettingsNotice("")}
              >
                ×
              </button>
            </div>
          )}

          {page === "home" && (
            <HomePage
              settings={settings}
              activeJob={activeJob}
              activeJobCount={activeJobs.length}
              maxActiveJobs={MAX_ACTIVE_DOWNLOADS}
              metadata={metadata}
              isStarting={isStarting}
              onUpdateSettings={updateSettings}
              onStart={onStart}
              onCancel={onCancel}
              onPreview={onPreview}
              onMetadataClear={() => setMetadata(null)}
              onChooseFolderError={setSettingsNotice}
            />
          )}
          {page === "downloads" && (
            <DownloadsPage jobs={jobs} onCancel={onCancel} onRetry={onRetry} />
          )}
          {page === "transcription" && (
            <TranscribePage />
          )}
          {page === "settings" && (
            <SettingsPage
              settings={settings}
              onUpdate={updateSettings}
              onChooseFolderError={setSettingsNotice}
            />
          )}
          {page === "about" && <AboutPage />}
        </div>

        <footer className="main-footer">
          <span>Made for media you’re authorized to save.</span>
          <span className="footer-right">
            <BadgeCheck size={14} aria-hidden="true" />
            No account · No cloud processing
          </span>
        </footer>
        <span className="sr-only" aria-live="polite" aria-atomic="true">
          {announcement}
        </span>
      </main>
    </div>
  );
}

export default App;
