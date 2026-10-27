import type { MediaFormat, Quality, Settings } from "../types";

const STORAGE_KEY = "clipkeep-settings";

export const DEFAULT_SETTINGS: Settings = {
  preferredFormat: "mp4",
  preferredQuality: "best",
  destinationDirectory: "",
  rememberDestination: true,
};

function isFormat(value: unknown): value is MediaFormat {
  return value === "mp4" || value === "mp3";
}

function isQuality(value: unknown): value is Quality {
  return (
    value === "best" ||
    value === "1080p" ||
    value === "720p" ||
    value === "480p" ||
    value === "320kbps" ||
    value === "192kbps" ||
    value === "128kbps"
  );
}

function isSettings(value: unknown): value is Settings {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as Record<string, unknown>;
  return (
    isFormat(candidate.preferredFormat) &&
    isQuality(candidate.preferredQuality) &&
    typeof candidate.destinationDirectory === "string" &&
    typeof candidate.rememberDestination === "boolean"
  );
}

export function loadSettings(): { settings: Settings; error?: string } {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) {
      return { settings: DEFAULT_SETTINGS };
    }
    const parsed: unknown = JSON.parse(stored);
    if (!isSettings(parsed)) {
      return {
        settings: DEFAULT_SETTINGS,
        error: "Saved preferences were invalid, so ClipKeep loaded safe defaults.",
      };
    }
    return {
      settings: {
        ...parsed,
        destinationDirectory: parsed.rememberDestination
          ? parsed.destinationDirectory
          : "",
      },
    };
  } catch {
    return {
      settings: DEFAULT_SETTINGS,
      error: "Could not read saved preferences. The default settings are in use.",
    };
  }
}

export function saveSettings(settings: Settings): string | undefined {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
    return undefined;
  } catch {
    return "Preferences could not be saved on this device.";
  }
}
