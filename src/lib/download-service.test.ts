import { afterEach, describe, expect, it, vi } from "vitest";
import { MockDownloadService } from "./download-service";

const request = {
  url: "https://youtu.be/BaW_jenozKc",
  format: "mp4" as const,
  quality: "best" as const,
  destinationDirectory: "C:\\Videos",
};

afterEach(() => {
  vi.useRealTimers();
});

describe("MockDownloadService", () => {
  it("emits preview progress and ends in failure without claiming completion", async () => {
    vi.useFakeTimers();
    const service = new MockDownloadService();
    const listener = vi.fn();

    await service.start(request, listener);
    vi.advanceTimersByTime(2_900);

    const events = listener.mock.calls.map(([event]) => event);
    expect(events.map((event) => event.status)).toEqual([
      "validating",
      "downloading",
      "downloading",
      "downloading",
      "failed",
    ]);
    expect(events[events.length - 1]?.error?.code).toBe("preview_only");
    expect(events.some((event) => event.status === "completed")).toBe(false);
  });

  it("cancels a preview and clears its scheduled progress", async () => {
    vi.useFakeTimers();
    const service = new MockDownloadService();
    const listener = vi.fn();
    const { id } = await service.start(request, listener);

    service.cancel(id);
    vi.advanceTimersByTime(3_000);

    expect(listener).toHaveBeenCalledTimes(1);
    expect(listener.mock.calls[0][0].status).toBe("cancelled");
    expect(listener.mock.calls[0][0].error).toBeNull();
  });
});
