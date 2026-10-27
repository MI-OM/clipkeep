import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface DependencyToolStatus {
  name: string;
  version: string;
  state: "missing" | "verified" | "invalid";
}

export interface DependencyStatus {
  available: boolean;
  tools: DependencyToolStatus[];
}

export interface DependencyProgress {
  tool: string;
  phase: string;
  percent: number | null;
}

export type DependencyProgressListener = (progress: DependencyProgress) => void;

export interface DependencyService {
  getStatus(): Promise<DependencyStatus>;
  install(listener: DependencyProgressListener): Promise<DependencyStatus>;
}

export class BrowserDependencyService implements DependencyService {
  async getStatus(): Promise<DependencyStatus> {
    return { available: false, tools: [] };
  }

  async install(): Promise<DependencyStatus> {
    throw new Error("Managed media-tool setup is available in the ClipKeep desktop app only.");
  }
}

export class TauriDependencyService implements DependencyService {
  async getStatus(): Promise<DependencyStatus> {
    try {
      return await invoke<DependencyStatus>("get_dependency_status");
    } catch (error) {
      throw new Error(errorMessage(error));
    }
  }

  async install(listener: DependencyProgressListener): Promise<DependencyStatus> {
    const unlisten = await listen<DependencyProgress>(
      "dependency-setup-progress",
      ({ payload }) => listener(payload),
    );
    try {
      return await invoke<DependencyStatus>("install_media_tools");
    } catch (error) {
      throw new Error(errorMessage(error));
    } finally {
      unlisten();
    }
  }
}

export function createDependencyService(): DependencyService {
  return isTauri() ? new TauriDependencyService() : new BrowserDependencyService();
}

function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = error.message;
    if (typeof message === "string") {
      return message;
    }
  }
  return "Media-tool setup failed. Check your connection and available disk space.";
}
