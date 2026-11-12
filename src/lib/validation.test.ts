import { describe, expect, it } from "vitest";
import { createDownloadRequest, validateVideoUrl } from "./validation";

describe("validateVideoUrl", () => {
  it.each([
    "https://www.youtube.com/watch?v=BaW_jenozKc",
    "https://youtu.be/BaW_jenozKc",
    "https://m.youtube.com/shorts/BaW_jenozKc",
    "https://www.youtube.com/embed/BaW_jenozKc",
  ])("accepts and normalizes supported YouTube links: %s", (url) => {
    expect(validateVideoUrl(url)).toEqual({
      valid: true,
      normalizedUrl: "https://www.youtube.com/watch?v=BaW_jenozKc",
    });
  });

  it.each([
    "https://vimeo.com/76979871",
    "https://www.tiktok.com/@creator/video/7106594312292453675",
    "https://www.instagram.com/reel/Cabc123_-/",
  ])("accepts individual videos on other supported platforms: %s", (url) => {
    expect(validateVideoUrl(url)).toEqual({
      valid: true,
      normalizedUrl: new URL(url).toString(),
    });
  });

  it("drops the fragment from non-YouTube links", () => {
    expect(validateVideoUrl("https://vimeo.com/76979871#t=10")).toEqual({
      valid: true,
      normalizedUrl: "https://vimeo.com/76979871",
    });
  });

  it.each([
    ["", "Paste a supported video link to continue."],
    ["not a url", "That doesn’t look like a valid link."],
    ["http://youtube.com/watch?v=BaW_jenozKc", "For safety, use a secure https:// link."],
    ["https://youtube.com.evil.example/watch?v=BaW_jenozKc", "Use a supported YouTube, Vimeo, Facebook, TikTok, or Instagram video link."],
    ["https://example.com/watch?v=BaW_jenozKc", "Use a supported YouTube, Vimeo, Facebook, TikTok, or Instagram video link."],
    ["https://youtube.com/watch?v=bad", "We couldn’t find a YouTube video in that link. Check it and try again."],
    ["https://www.tiktok.com/@creator", "Paste a link to an individual video, not a profile or collection."],
    ["https://user:pass@youtube.com/watch?v=BaW_jenozKc", "This link format is not supported. Paste a standard video link."],
    ["file:///C:/video.mp4", "For safety, use a secure https:// link."],
  ])("rejects invalid URL %s", (url, message) => {
    expect(validateVideoUrl(url)).toEqual({ valid: false, message });
  });

  it("returns a typed request only for valid URLs", () => {
    expect(
      createDownloadRequest(
        "https://youtu.be/BaW_jenozKc",
        "mp3",
        "720p",
        "C:\\Users\\test\\Videos",
      ),
    ).toEqual({
      valid: true,
      request: {
        url: "https://www.youtube.com/watch?v=BaW_jenozKc",
        format: "mp3",
        quality: "720p",
        destinationDirectory: "C:\\Users\\test\\Videos",
      },
    });
  });
});
