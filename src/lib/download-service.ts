import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppError,
  DownloadJob,
  DownloadRequest,
  DownloadStatus,
  MediaMetadata,
  ProgressEvent,
} from "../types";

export type ProgressListener = (event: ProgressEvent) => void;

export interface DownloadService {
  getMetadata(url: string): Promise<MediaMetadata>;
  start(request: DownloadRequest, listener: ProgressListener): Promise<DownloadJob>;
  cancel(jobId: string): Promise<void>;
  cancelAll(): void;
}

const MOCK_ERROR: AppError = {
  code: "preview_only",
  message:
    "The browser preview does not connect to a media engine. No media was downloaded or saved.",
  recoverable: false,
};

const MOCK_STEPS: Array<{
  delay: number;
  status: DownloadStatus;
  progressPercent: number | null;
}> = [
  { delay: 500, status: "validating", progressPercent: null },
  { delay: 1_100, status: "downloading", progressPercent: 16 },
  { delay: 1_700, status: "downloading", progressPercent: 43 },
  { delay: 2_300, status: "downloading", progressPercent: 71 },
  { delay: 2_900, status: "failed", progressPercent: null },
];

interface ScheduledJob {
  timers: ReturnType<typeof setTimeout>[];
  listener: ProgressListener;
}

export class MockDownloadService implements DownloadService {
  private readonly jobs = new Map<string, ScheduledJob>();

  async getMetadata(): Promise<MediaMetadata> {
    throw new Error("Video details are available from the ClipKeep desktop app only.");
  }

  async start(
    request: DownloadRequest,
    listener: ProgressListener,
  ): Promise<DownloadJob> {
    const jobId = crypto.randomUUID();
    const timers: ReturnType<typeof setTimeout>[] = [];
    this.jobs.set(jobId, { timers, listener });

    for (const step of MOCK_STEPS) {
      const timer = setTimeout(() => {
        if (!this.jobs.has(jobId)) {
          return;
        }
        listener({
          jobId,
          status: step.status,
          progressPercent: step.progressPercent,
          bytesPerSecond: null,
          etaSeconds: null,
          outputPath: null,
          error: step.status === "failed" ? MOCK_ERROR : null,
        });
        if (step.status === "failed") {
          this.jobs.delete(jobId);
        }
      }, step.delay);
      timers.push(timer);
    }

    return {
      id: jobId,
      title: null,
      format: request.format,
      quality: request.quality,
      status: "queued",
      progressPercent: null,
      bytesPerSecond: null,
      etaSeconds: null,
      outputPath: null,
      error: null,
      createdAt: Date.now(),
    };
  }

  async cancel(jobId: string): Promise<void> {
    const job = this.jobs.get(jobId);
    if (!job) {
      return;
    }
    for (const timer of job.timers) {
      clearTimeout(timer);
    }
    this.jobs.delete(jobId);
    job.listener({
      jobId,
      status: "cancelled",
      progressPercent: null,
      bytesPerSecond: null,
      etaSeconds: null,
      outputPath: null,
      error: null,
    });
  }

  cancelAll(): void {
    for (const jobId of this.jobs.keys()) {
      void this.cancel(jobId);
    }
  }
}

export class TauriDownloadService implements DownloadService {
  async getMetadata(url: string): Promise<MediaMetadata> {
    return invoke<MediaMetadata>("get_media_metadata", { url });
  }

  async start(
    request: DownloadRequest,
    listener: ProgressListener,
  ): Promise<DownloadJob> {
    let jobId: string | null = null;
    const queuedEvents: ProgressEvent[] = [];
    let unlisten = () => {};
    unlisten = await listen<ProgressEvent>("download-progress", ({ payload }) => {
      if (jobId === null) {
        queuedEvents.push(payload);
        return;
      }
      if (payload.jobId !== jobId) {
        return;
      }
      listener(payload);
      if (isTerminal(payload.status)) {
        unlisten();
      }
    });

    try {
      const job = await invoke<DownloadJob>("start_download", { request });
      jobId = job.id;
      for (const event of queuedEvents) {
        if (event.jobId === jobId) {
          listener(event);
          if (isTerminal(event.status)) {
            unlisten();
          }
        }
      }
      return job;
    } catch (error) {
      unlisten();
      throw new Error(errorMessage(error));
    }
  }

  async cancel(jobId: string): Promise<void> {
    try {
      await invoke("cancel_download", { jobId });
    } catch (error) {
      throw new Error(errorMessage(error));
    }
  }

  cancelAll(): void {}
}

export function createDownloadService(): DownloadService {
  return isTauri() ? new TauriDownloadService() : new MockDownloadService();
}

function isTerminal(status: DownloadStatus): boolean {
  return status === "completed" || status === "failed" || status === "cancelled";
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    const structuredMessage = messageFromSerializedPayload(error);
    if (structuredMessage) {
      return structuredMessage;
    }
    if (error.trim()) {
      return error.trim();
    }
  }
  if (error instanceof Error) {
    const structuredMessage = messageFromSerializedPayload(error.message);
    if (structuredMessage) {
      return structuredMessage;
    }
    if (error.message.trim()) {
      return error.message.trim();
    }
  }
  if (typeof error === "object" && error !== null) {
    const structuredMessage = messageFromSerializedPayload(error);
    if (structuredMessage) {
      return structuredMessage;
    }
  }
  return "The desktop operation failed. Please try again.";
}

function messageFromSerializedPayload(payload: unknown): string | undefined {
  if (typeof payload === "string") {
    try {
      return messageFromSerializedPayload(JSON.parse(payload));
    } catch {
      return undefined;
    }
  }
  if (typeof payload !== "object" || payload === null) {
    return undefined;
  }
  const record = payload as Record<string, unknown>;
  if (typeof record.message === "string" && record.message.trim()) {
    return record.message.trim();
  }
  if (typeof record.code === "string" && record.code.trim()) {
    return `ClipKeep could not start the download (${record.code.trim()}).`;
  }
  return undefined;
}
