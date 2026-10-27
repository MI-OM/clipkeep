import { useEffect, useMemo, useState } from "react";
import {
  FolderOpen,
  HardDrive,
  Info,
  RefreshCw,
  ShieldCheck,
  SlidersHorizontal,
} from "lucide-react";
import { isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  AUDIO_QUALITY_OPTIONS,
  FORMAT_OPTIONS,
  VIDEO_QUALITY_OPTIONS,
} from "../../lib/validation";
import { createDependencyService } from "../../lib/dependency-service";
import type {
  DependencyProgress,
  DependencyStatus,
} from "../../lib/dependency-service";
import type { Settings } from "../../types";

interface SettingsPageProps {
  settings: Settings;
  onUpdate: (update: Partial<Settings>) => void;
  onChooseFolderError: (message: string) => void;
}

export function SettingsPage({ settings, onUpdate, onChooseFolderError }: SettingsPageProps) {
  const desktopRuntime = isTauri();
  const [folderMessage, setFolderMessage] = useState("");
  const [dependencyStatus, setDependencyStatus] = useState<DependencyStatus | null>(null);
  const [dependencyError, setDependencyError] = useState("");
  const [dependencyProgress, setDependencyProgress] = useState<DependencyProgress | null>(null);
  const [isCheckingDependencies, setIsCheckingDependencies] = useState(desktopRuntime);
  const [isInstallingDependencies, setIsInstallingDependencies] = useState(false);
  const dependencyService = useMemo(() => createDependencyService(), []);

  useEffect(() => {
    let mounted = true;
    setIsCheckingDependencies(true);
    dependencyService.getStatus()
      .then((status) => {
        if (mounted) {
          setDependencyStatus(status);
        }
      })
      .catch((error: unknown) => {
        if (mounted) {
          setDependencyError(error instanceof Error
            ? error.message
            : "ClipKeep could not check media-tool status.");
        }
      })
      .finally(() => {
        if (mounted) {
          setIsCheckingDependencies(false);
        }
      });
    return () => {
      mounted = false;
    };
  }, [dependencyService]);

  async function installDependencies() {
    setIsInstallingDependencies(true);
    setDependencyError("");
    setDependencyProgress({ tool: "Preparing", phase: "downloading", percent: 0 });
    try {
      const status = await dependencyService.install(setDependencyProgress);
      setDependencyStatus(status);
      if (!status.available) {
        throw new Error("Media tools were installed, but one or more files did not pass verification.");
      }
      setDependencyProgress({ tool: "Complete", phase: "verified", percent: 100 });
    } catch (error) {
      setDependencyProgress(null);
      setDependencyError(error instanceof Error
        ? error.message
        : "Media-tool setup failed. Check your connection and available disk space.");
    } finally {
      setIsInstallingDependencies(false);
    }
  }

  async function chooseFolder() {
    if (!isTauri()) {
      setFolderMessage("The folder picker is available in the desktop app.");
      return;
    }
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Choose your default save location",
      });
      if (typeof selected === "string") {
        onUpdate({ destinationDirectory: selected });
        setFolderMessage("Default save location updated.");
        onChooseFolderError("");
      }
    } catch (error) {
      const detail = error instanceof Error ? error.message : "Unknown dialog error";
      setFolderMessage(`Could not open the folder picker. ${detail}`);
    }
  }

  return (
    <div className="content-page settings-page">
      <div className="page-intro">
        <div>
          <p className="section-kicker">PREFERENCES</p>
          <h1>Settings</h1>
          <p className="page-subtitle">A few thoughtful defaults for your local workspace.</p>
        </div>
        <span className="page-icon-badge"><SlidersHorizontal size={20} /></span>
      </div>

      <section className="settings-card">
        <div className="settings-card-heading">
          <div className="settings-card-icon"><FolderOpen size={18} /></div>
          <div><h2>Save location</h2><p>Choose where new files should be saved.</p></div>
        </div>
        <div className="settings-row folder-setting">
          <div className="folder-preview">
            <HardDrive size={17} />
            <span>{settings.destinationDirectory || "No default folder selected"}</span>
          </div>
          <button className="secondary-button" type="button" onClick={chooseFolder}>
            <FolderOpen size={15} /> Choose folder
          </button>
        </div>
        <label className="toggle-row">
          <span className="toggle-copy">
            <strong>Remember this folder</strong>
            <small>Keep your save location on this device.</small>
          </span>
          <input
            type="checkbox"
            checked={settings.rememberDestination}
            onChange={(event) => {
              onUpdate({ rememberDestination: event.currentTarget.checked });
              if (!event.currentTarget.checked) {
                onUpdate({ destinationDirectory: "" });
              }
            }}
          />
          <span className="toggle-control" aria-hidden="true" />
        </label>
        {folderMessage && <p className="settings-feedback" role="status">{folderMessage}</p>}
      </section>

      <section className="settings-card">
        <div className="settings-card-heading">
          <div className="settings-card-icon purple"><RefreshCw size={17} /></div>
          <div><h2>Download defaults</h2><p>Starting points for a new download.</p></div>
        </div>
        <div className="settings-row preference-row">
          <label htmlFor="default-format">Preferred format</label>
          <select
            id="default-format"
            value={settings.preferredFormat}
            onChange={(event) => {
              const selectedFormat = FORMAT_OPTIONS.find(
                (option) => option.id === event.currentTarget.value,
              )?.id;
              if (selectedFormat) {
                onUpdate({ preferredFormat: selectedFormat, preferredQuality: "best" });
              }
            }}
          >
            {FORMAT_OPTIONS.map((option) => (
              <option key={option.id} value={option.id}>{option.label}</option>
            ))}
          </select>
        </div>
        <div className="settings-row preference-row">
          <label htmlFor="default-quality">Preferred quality</label>
          <select
            id="default-quality"
            value={settings.preferredQuality}
            onChange={(event) =>
              onUpdate({
                preferredQuality: event.currentTarget.value as Settings["preferredQuality"],
              })
            }
          >
            {(settings.preferredFormat === "mp3"
              ? AUDIO_QUALITY_OPTIONS
              : VIDEO_QUALITY_OPTIONS
            ).map((option) => (
              <option key={option.id} value={option.id}>{option.label}</option>
            ))}
          </select>
        </div>
        <div className="settings-tip">
          <Info size={15} />
          <span>Available quality choices are checked against the source before downloading.</span>
        </div>
      </section>

      <section className="settings-card dependency-card">
        <div className="settings-card-heading">
          <div className="settings-card-icon green"><ShieldCheck size={18} /></div>
          <div><h2>App dependencies</h2><p>Verified tools required to process media.</p></div>
        </div>
        {dependencyStatus?.tools.map((tool) => (
          <div className="dependency-row" key={tool.name}>
            <div className="dependency-name">
              <span className={`dependency-dot ${tool.state === "verified" ? "verified" : "pending"}`} />
              <div><strong>{tool.name}</strong><small>Version {tool.version}</small></div>
            </div>
            <span className={`dependency-status ${tool.state}`}>
              {tool.state === "verified" ? "Verified" : tool.state === "invalid" ? "Needs repair" : "Not installed"}
            </span>
          </div>
        ))}
        <div className="dependency-footnote">
          <Info size={14} />
              {!desktopRuntime
                ? "Managed setup runs in the desktop app. The browser preview cannot install or launch external tools."
                : dependencyStatus
                  ? "Downloads are pinned to release versions and SHA-256 checksums. FFmpeg is a third-party GPLv3 build."
                  : "ClipKeep could not verify dependency status yet."}
        </div>
        {dependencyProgress && (
          <p className="dependency-progress" role="status" aria-live="polite">
            {isInstallingDependencies
              ? `${dependencyProgress.tool}: ${dependencyProgress.phase.replace(/_/g, " ")}${dependencyProgress.percent === null ? "" : ` (${dependencyProgress.percent}%)`}`
              : dependencyStatus?.available
                ? "All media tools are installed and verified."
                : ""}
          </p>
        )}
        {dependencyError && <p className="dependency-error" role="alert">{dependencyError}</p>}
        <button
          className="secondary-button dependency-action"
          type="button"
          disabled={!desktopRuntime || isCheckingDependencies || isInstallingDependencies || dependencyStatus?.available === true}
          onClick={installDependencies}
        >
          {isCheckingDependencies
            ? "Checking tools…"
            : isInstallingDependencies
              ? "Installing verified tools…"
              : !desktopRuntime
                ? "Desktop app required"
                : dependencyStatus?.available
                ? "All tools verified"
                : dependencyStatus?.tools.some((tool) => tool.state === "invalid")
                  ? "Repair media tools"
                  : "Install media tools"}
        </button>
        <p className="dependency-legal">
          The Gyan.dev FFmpeg Essentials build is GPLv3. ClipKeep records its version, checksum, and source;
          review the licensing notes before redistributing this application.
        </p>
      </section>
    </div>
  );
}
