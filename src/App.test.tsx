import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";

afterEach(() => {
  cleanup();
  localStorage.clear();
  vi.useRealTimers();
});

describe("ClipKeep shell", () => {
  it("validates the URL before starting a preview", () => {
    render(<App />);

    fireEvent.change(screen.getByLabelText("VIDEO LINK"), {
      target: { value: "https://example.com/video" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Start MP4 download" }));

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Use a supported YouTube, Vimeo, Facebook, TikTok, or Instagram video link.",
    );
    expect(screen.queryByText("Preview job")).not.toBeInTheDocument();
  });

  it("keeps the compact navigation controls accessible", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /Remember this folder/ })).toBeInTheDocument();
  });

  it("keeps managed dependency setup unavailable in the browser preview", async () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Settings" }));

    expect(await screen.findByRole("button", { name: "Desktop app required" })).toBeDisabled();
    expect(screen.getByText(/browser preview cannot install or launch external tools/i))
      .toBeInTheDocument();
  });

  it("offers MP3 output and audio bitrate presets in the UI", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /MP3 audio/ }));

    expect(screen.getByRole("button", { name: "Start MP3 download" })).toBeEnabled();
    expect(screen.getByRole("option", { name: "320 kbps" })).toBeInTheDocument();
  });

  it("shows explicit preview progress and never claims a download completed", async () => {
    vi.useFakeTimers();
    localStorage.setItem(
      "clipkeep-settings",
      JSON.stringify({
        preferredFormat: "mp4",
        preferredQuality: "best",
        destinationDirectory: "C:\\Videos",
        rememberDestination: true,
      }),
    );
    render(<App />);

    fireEvent.change(screen.getByLabelText("VIDEO LINK"), {
      target: { value: "https://youtu.be/BaW_jenozKc" },
    });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Start MP4 download" }));
      await Promise.resolve();
    });

    expect(screen.getByText(/no media is fetched or saved/i)).toBeInTheDocument();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2_900);
    });

    expect(screen.getByText("Failed")).toBeInTheDocument();
    expect(screen.getByText("No completed file saved")).toBeInTheDocument();
    expect(screen.queryByText("Completed")).not.toBeInTheDocument();
  });
});
