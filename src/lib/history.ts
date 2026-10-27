import type { DownloadJob, DownloadStatus, Quality } from "../types";

const STORAGE_KEY = "clipkeep-download-history-v1";
export const HISTORY_LIMIT = 100;

const STATUSES: DownloadStatus[] = [
  "queued",
  "validating",
  "fetching_metadata",
  "downloading",
  "merging",
  "converting",
  "finalizing",
  "completed",
  "failed",
  "cancelling",
  "cancelled",
];

const QUALITIES: Quality[] = [
  "best",
  "1080p",
  "720p",
  "480p",
  "320kbps",
  "192kbps",
  "128kbps",
];

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isNullableNumber(value: unknown): value is number | null {
  return value === null || (typeof value === "number" && Number.isFinite(value));
}

function isError(value: unknown): value is DownloadJob["error"] {
  if (value === null) {
    return true;
  }
  if (typeof value !== "object") {
    return false;
  }
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.code === "string" &&
    typeof candidate.message === "string" &&
    typeof candidate.recoverable === "boolean"
  );
}

function isDownloadJob(value: unknown): value is DownloadJob {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.id === "string" &&
    isNullableString(candidate.title) &&
    (candidate.format === "mp4" || candidate.format === "mp3") &&
    QUALITIES.includes(candidate.quality as Quality) &&
    STATUSES.includes(candidate.status as DownloadStatus) &&
    isNullableNumber(candidate.progressPercent) &&
    isNullableNumber(candidate.bytesPerSecond) &&
    isNullableNumber(candidate.etaSeconds) &&
    isNullableString(candidate.outputPath) &&
    isError(candidate.error) &&
    typeof candidate.createdAt === "number" &&
    Number.isFinite(candidate.createdAt)
  );
}

function interrupted(job: DownloadJob): DownloadJob {
  if (["completed", "failed", "cancelled"].includes(job.status)) {
    return job;
  }
  return {
    ...job,
    status: "failed",
    progressPercent: null,
    bytesPerSecond: null,
    etaSeconds: null,
    outputPath: null,
    error: {
      code: "interrupted",
      message: "ClipKeep closed before this download finished. No partial file was marked complete.",
      recoverable: true,
    },
  };
}

export function loadDownloadHistory(): { jobs: DownloadJob[]; error?: string } {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (!stored) {
      return { jobs: [] };
    }
    const parsed: unknown = JSON.parse(stored);
    if (!Array.isArray(parsed)) {
      return { jobs: [], error: "Saved download history was invalid and could not be loaded." };
    }

    const validJobs = parsed.filter(isDownloadJob).slice(0, HISTORY_LIMIT);
    const jobs = validJobs.map(interrupted);
    const hadInvalidEntries = validJobs.length !== parsed.length;
    const hadInterruptedJobs = jobs.some(
      (job, index) => job.status !== validJobs[index]?.status,
    );
    if (hadInterruptedJobs) {
      const saveError = saveDownloadHistory(jobs);
      if (saveError) {
        return {
          jobs,
          error: `${saveError} An unfinished download was marked interrupted.`,
        };
      }
    }
    return {
      jobs,
      ...(hadInvalidEntries
        ? { error: "Some invalid entries in saved download history were skipped." }
        : {}),
    };
  } catch {
    return { jobs: [], error: "Could not read saved download history." };
  }
}

export function saveDownloadHistory(jobs: DownloadJob[]): string | undefined {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(jobs.slice(0, HISTORY_LIMIT)));
    return undefined;
  } catch {
    return "Download history could not be saved on this device.";
  }
}
