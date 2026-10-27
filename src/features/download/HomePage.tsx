import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import {
  ArrowDownToLine,
  ArrowRight,
  Check,
  ClipboardPaste,
  FileAudio2,
  FileVideo2,
  FolderOpen,
  Link2,
  LockKeyhole,
  ShieldCheck,
  Sparkles,
  X,
} from "lucide-react";
import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AUDIO_QUALITY_OPTIONS,
  createDownloadRequest,
  FORMAT_OPTIONS,
  VIDEO_QUALITY_OPTIONS,
} from "../../lib/validation";
import type {
  DownloadJob,
  DownloadRequest,
  MediaFormat,
  MediaMetadata,
  Settings,
} from "../../types";

interface HomePageProps {
  settings: Settings;
  activeJob: DownloadJob | undefined;
  activeJobCount: number;
  maxActiveJobs: number;
  metadata: MediaMetadata | null;
  isStarting: boolean;
  onUpdateSettings: (update: Partial<Settings>) => void;
  onStart: (request: DownloadRequest) => void;
  onCancel: (id: string) => void;
  onPreview: (url: string) => Promise<MediaMetadata>;
  onMetadataClear: () => void;
  onChooseFolderError: (message: string) => void;
}

export function HomePage({
  settings,
  activeJob,
  activeJobCount,
  maxActiveJobs,
  metadata,
  isStarting,
  onUpdateSettings,
  onStart,
  onCancel,
  onPreview,
  onMetadataClear,
  onChooseFolderError,
}: HomePageProps) {
  const [url, setUrl] = useState("");
  const [format, setFormat] = useState<MediaFormat>(settings.preferredFormat);
  const [validationError, setValidationError] = useState("");
  const [metadataError, setMetadataError] = useState("");
  const [metadataLoading, setMetadataLoading] = useState(false);
  const [isDesktop] = useState(isTauri());
  const qualityOptions = format === "mp3" ? AUDIO_QUALITY_OPTIONS : VIDEO_QUALITY_OPTIONS;
  const availableQualities = format === "mp3"
    ? metadata?.availableAudioQualities
    : metadata?.availableVideoQualities;
  const visibleQualityOptions = metadata
    ? qualityOptions.filter(
        (option) => availableQualities?.includes(option.id),
      )
    : qualityOptions;
  const hasCompatibleQuality = visibleQualityOptions.length > 0;

  useEffect(() => {
    if (
      metadata &&
      visibleQualityOptions.length > 0 &&
      !visibleQualityOptions.some((option) => option.id === settings.preferredQuality)
    ) {
      onUpdateSettings({ preferredQuality: visibleQualityOptions[0]!.id });
    }
  }, [metadata, onUpdateSettings, settings.preferredQuality, visibleQualityOptions]);

  async function chooseFolder() {
    if (!isTauri()) {
      onChooseFolderError("Choose a save folder from the ClipKeep desktop app.");
      return;
    }
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Choose a save location",
      });
      if (typeof selected === "string") {
        onUpdateSettings({ destinationDirectory: selected });
        onChooseFolderError("");
      }
    } catch (error) {
      const detail = error instanceof Error ? error.message : "Unknown dialog error";
      onChooseFolderError(`The folder picker could not be opened. ${detail}`);
    }
  }

  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setValidationError("");
    const result = createDownloadRequest(
      url,
      format,
      settings.preferredQuality,
      settings.destinationDirectory,
    );
    if (!result.valid) {
      setValidationError(result.message);
      return;
    }
    if (!settings.destinationDirectory) {
      setValidationError("Choose a save folder before starting a download.");
      return;
    }
    if (metadata && !hasCompatibleQuality) {
      setValidationError(`No compatible ${format.toUpperCase()} source is available for this link.`);
      return;
    }
    onStart({ ...result.request, title: metadata?.title ?? null });
  }

  async function preview() {
    setMetadataError("");
    setMetadataLoading(true);
    try {
      await onPreview(url);
    } catch (error) {
      const message = error instanceof Error ? error.message : "Video details could not be loaded.";
      setMetadataError(
        message.toLowerCase().includes("too large to process safely")
          ? "Video details are too large to preview safely. You can still download; ClipKeep will use the video ID for the filename."
          : message,
      );
    } finally {
      setMetadataLoading(false);
    }
  }

  return (
    <div className="home-layout">
      <section className="hero-copy">
        <div className="eyebrow">
          <span className="eyebrow-line" />
          YOUR MEDIA, YOUR DEVICE
        </div>
        <h1>Keep what<br />you <span>love.</span></h1>
        <p className="hero-description">
          A calm, private place to save media you’re authorized to keep.
          Paste a link to get started.
        </p>
        <div className="trust-points">
          <div><ShieldCheck size={16} /><span>Private by design</span></div>
          <div><LockKeyhole size={16} /><span>No ClipKeep account or analytics</span></div>
        </div>
      </section>

      <section className="download-card" aria-labelledby="download-card-title">
        <div className="card-heading">
          <div>
            <p className="section-kicker">NEW DOWNLOAD</p>
            <h2 id="download-card-title">Save a video or audio file</h2>
          </div>
          <span className="card-icon"><ArrowDownToLine size={19} /></span>
        </div>

        <form onSubmit={submit} noValidate>
          <label className="field-label" htmlFor="video-url">VIDEO LINK</label>
          <div className={`url-field${validationError ? " has-error" : ""}`}>
            <Link2 size={18} aria-hidden="true" />
            <input
              id="video-url"
              type="url"
              value={url}
              onChange={(event) => {
                setUrl(event.currentTarget.value);
                setValidationError("");
                setMetadataError("");
                onMetadataClear();
              }}
              placeholder="Paste a supported video link"
              autoComplete="url"
              aria-invalid={Boolean(validationError)}
              aria-describedby={validationError ? "url-error" : "url-help"}
            />
            {url && (
              <button
                className="clear-input"
                type="button"
                aria-label="Clear video link"
                onClick={() => {
                  setUrl("");
                  setValidationError("");
                }}
              >
                <X size={15} />
              </button>
            )}
            {!url && <ClipboardPaste size={17} className="paste-hint" aria-hidden="true" />}
          </div>
          {validationError ? (
            <p className="field-error" id="url-error" role="alert">{validationError}</p>
          ) : (
            <p className="field-help" id="url-help">
              {isDesktop
                ? "YouTube, Vimeo, Facebook, TikTok, and Instagram links are recognized. Availability and permission depend on the source."
                : "Supported platform links can be checked in the desktop app; browser preview cannot download media."}
            </p>
          )}
          <div className="metadata-action-row">
            <button
              className="text-button"
              type="button"
              disabled={metadataLoading || !url.trim()}
              onClick={preview}
            >
              {metadataLoading ? "Checking link…" : "Preview video details"}
            </button>
            {metadataError && <span className="field-error" role="alert">{metadataError}</span>}
          </div>
          {metadata && (
            <div className="metadata-preview" aria-live="polite">
              <span className="metadata-preview-icon"><FileVideo2 size={17} /></span>
              <span className="metadata-preview-copy">
                <strong>{metadata.title}</strong>
                <small>
                  {metadata.durationSeconds === null
                    ? "Duration unavailable"
                    : `${Math.floor(metadata.durationSeconds / 60)}:${String(metadata.durationSeconds % 60).padStart(2, "0")}`}
                  {" · "}
                  {metadata.availableVideoQualities.length > 0
                    ? "Compatible MP4 video formats available"
                    : "No compatible MP4 video format"}
                  {" · "}
                  {metadata.availableAudioQualities.length > 0
                    ? "MP3 audio available"
                    : "No compatible audio"}
                </small>
              </span>
              <button
                className="clear-input"
                type="button"
                aria-label="Clear video details"
                onClick={onMetadataClear}
              >
                <X size={14} />
              </button>
            </div>
          )}

          <div className="form-grid">
            <fieldset className="form-fieldset">
              <legend className="field-label">FORMAT</legend>
              <div className="format-options">
                {FORMAT_OPTIONS.map((option) => {
                  const selected = format === option.id;
                  const FormatIcon = option.id === "mp4" ? FileVideo2 : FileAudio2;
                  return (
                    <button
                      className={`format-option${selected ? " is-selected" : ""}`}
                      key={option.id}
                      type="button"
                      aria-pressed={selected}
                      onClick={() => {
                        setFormat(option.id);
                        onUpdateSettings({
                          preferredFormat: option.id,
                          preferredQuality: "best",
                        });
                      }}
                    >
                      <span className="format-icon"><FormatIcon size={18} /></span>
                      <span className="format-copy">
                        <strong>{option.label}</strong>
                        <small>{option.description}</small>
                      </span>
                      <span className="radio-indicator" aria-hidden="true">
                        {selected && <Check size={12} />}
                      </span>
                    </button>
                  );
                })}
              </div>
            </fieldset>

            <div className="quality-field">
              <label className="field-label" htmlFor="quality">QUALITY</label>
              <div className="select-wrap">
                <select
                  id="quality"
                  disabled={Boolean(metadata) && !hasCompatibleQuality}
                  value={settings.preferredQuality}
                  onChange={(event) =>
                    onUpdateSettings({
                      preferredQuality: event.currentTarget.value as Settings["preferredQuality"],
                    })
                  }
                >
                  {visibleQualityOptions.length === 0 ? (
                    <option value={settings.preferredQuality}>No compatible source</option>
                  ) : visibleQualityOptions.map((option) => (
                    <option key={option.id} value={option.id}>{option.label}</option>
                  ))}
                </select>
                <ArrowRight size={14} className="select-chevron" aria-hidden="true" />
              </div>
              <p className="field-help">
                {format === "mp3"
                  ? "MP3 audio is converted locally when verified tools are available."
                  : metadata
                    ? "Only options reported by the source are available."
                    : "Availability depends on the source."}
              </p>
            </div>
          </div>

          <div className="destination-section">
            <div className="destination-heading">
              <label className="field-label">SAVE LOCATION</label>
              <button className="text-button" type="button" onClick={chooseFolder}>
                <FolderOpen size={14} />
                Choose folder
              </button>
            </div>
            <div className="destination-box">
              <span className="destination-icon"><FolderOpen size={17} /></span>
              <span className={settings.destinationDirectory ? "destination-path" : "destination-placeholder"}>
                {settings.destinationDirectory || "Choose a folder on this device"}
              </span>
              {settings.destinationDirectory && (
                <span className="destination-check" aria-label="Folder selected"><Check size={14} /></span>
              )}
            </div>
          </div>

          <button
            className="primary-button download-button"
            type="submit"
            disabled={
              isStarting ||
              activeJobCount >= maxActiveJobs ||
              (Boolean(metadata) && !hasCompatibleQuality)
            }
          >
            <span>
              {isStarting
                ? "Starting…"
                : activeJobCount >= maxActiveJobs
                  ? `Download limit reached (${maxActiveJobs})`
                  : `Start ${format.toUpperCase()} download`}
            </span>
            <ArrowDownToLine size={17} aria-hidden="true" />
          </button>
        </form>

        <div className="card-footnote">
          <LockKeyhole size={13} aria-hidden="true" />
          Only save content you have permission to download.
        </div>
      </section>

      {activeJob && (
        <div className="active-preview" role="status">
          <div className="active-preview-icon"><Sparkles size={16} /></div>
          <div className="active-preview-copy">
            <strong>
              {activeJobCount === 1 ? "1 download in progress" : `${activeJobCount} downloads in progress`}
            </strong>
            <span>
              {isDesktop
                ? `Processing locally. Up to ${maxActiveJobs} downloads can run at once.`
                : "No file is being downloaded in this browser preview."}
            </span>
          </div>
          <button className="cancel-small" type="button" onClick={() => onCancel(activeJob.id)}>
            Cancel
          </button>
        </div>
      )}

      <div className="home-bottom-note">
        <span className="note-icon"><InfoIcon /></span>
        <p>
          ClipKeep is a local utility, not a content source. Respect creators’ rights and the
          terms that apply to media you access.
        </p>
      </div>
    </div>
  );
}

function InfoIcon() {
  return <ShieldCheck size={15} aria-hidden="true" />;
}
