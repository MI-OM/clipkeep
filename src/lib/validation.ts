import type { DownloadRequest, MediaFormat, Quality } from "../types";

const YOUTUBE_HOSTS = new Set([
  "youtube.com",
  "www.youtube.com",
  "m.youtube.com",
  "youtu.be",
  "www.youtu.be",
]);

const ALLOWED_HOSTS = new Set([
  ...YOUTUBE_HOSTS,
  "vimeo.com",
  "www.vimeo.com",
  "player.vimeo.com",
  "facebook.com",
  "www.facebook.com",
  "m.facebook.com",
  "web.facebook.com",
  "fb.watch",
  "www.fb.watch",
  "tiktok.com",
  "www.tiktok.com",
  "m.tiktok.com",
  "vm.tiktok.com",
  "vt.tiktok.com",
  "instagram.com",
  "www.instagram.com",
  "m.instagram.com",
  "instagr.am",
  "www.instagr.am",
]);

const VIDEO_ID_PATTERN = /^[a-zA-Z0-9_-]{11}$/;

export type UrlValidationResult =
  | { valid: true; normalizedUrl: string }
  | { valid: false; message: string };

export function validateVideoUrl(input: string): UrlValidationResult {
  const candidate = input.trim();
  if (!candidate) {
    return { valid: false, message: "Paste a supported video link to continue." };
  }

  let url: URL;
  try {
    url = new URL(candidate);
  } catch {
    return { valid: false, message: "That doesn’t look like a valid link." };
  }

  if (url.protocol !== "https:") {
    return { valid: false, message: "For safety, use a secure https:// link." };
  }
  if (!ALLOWED_HOSTS.has(url.hostname.toLowerCase())) {
    return {
      valid: false,
      message: "Use a supported YouTube, Vimeo, Facebook, TikTok, or Instagram video link.",
    };
  }
  if (url.username || url.password || url.port) {
    return {
      valid: false,
      message: "This link format is not supported. Paste a standard video link.",
    };
  }

  if (YOUTUBE_HOSTS.has(url.hostname.toLowerCase())) {
    const videoId = extractVideoId(url);
    if (!videoId || !VIDEO_ID_PATTERN.test(videoId)) {
      return {
        valid: false,
        message: "We couldn’t find a YouTube video in that link. Check it and try again.",
      };
    }
    return { valid: true, normalizedUrl: `https://www.youtube.com/watch?v=${videoId}` };
  }

  if (!isIndividualPlatformVideo(url)) {
    return { valid: false, message: "Paste a link to an individual video, not a profile or collection." };
  }

  url.hash = "";
  return { valid: true, normalizedUrl: url.toString() };
}

function isIndividualPlatformVideo(url: URL): boolean {
  const host = url.hostname.toLowerCase();
  const path = url.pathname;
  const cleanPath = path.length > 1 ? path.replace(/\/+$/, "") : path;
  if (["vimeo.com", "www.vimeo.com"].includes(host)) {
    return /^\/(?:video\/)?\d+\/?$/.test(path);
  }
  if (host === "player.vimeo.com") {
    return /^\/video\/\d+\/?$/.test(path);
  }
  if (["facebook.com", "www.facebook.com", "m.facebook.com", "web.facebook.com"].includes(host)) {
    return (
      (cleanPath === "/watch" && /^\d+$/.test(url.searchParams.get("v") ?? "")) ||
      /^\/(?:reel|videos)\/[\w.-]+(?:\/[\w.-]+)?$/.test(cleanPath) ||
      /^\/share\/(?:v|r)\/[\w.-]+$/.test(cleanPath)
    );
  }
  if (["fb.watch", "www.fb.watch", "vm.tiktok.com", "vt.tiktok.com"].includes(host)) {
    return /^\/[\w-]+\/?$/.test(path);
  }
  if (["tiktok.com", "www.tiktok.com", "m.tiktok.com"].includes(host)) {
    return /^\/@[\w.]+\/video\/\d+\/?$/.test(path) || /^\/t\/[\w-]+\/?$/.test(path);
  }
  if (["instagram.com", "www.instagram.com", "m.instagram.com", "instagr.am", "www.instagr.am"].includes(host)) {
    return /^\/(?:p|reel|reels|tv)\/[\w-]+\/?$/.test(path);
  }
  return false;
}

function extractVideoId(url: URL): string | null {
  if (url.hostname === "youtu.be" || url.hostname === "www.youtu.be") {
    return url.pathname.split("/").filter(Boolean)[0] ?? null;
  }

  if (url.pathname === "/watch") {
    return url.searchParams.get("v");
  }

  const pathMatch = url.pathname.match(/^\/(?:shorts|embed|live)\/([^/]+)/);
  return pathMatch?.[1] ?? null;
}

export type DownloadRequestResult =
  | { valid: true; request: DownloadRequest }
  | { valid: false; message: string };

export function createDownloadRequest(
  url: string,
  format: MediaFormat,
  quality: Quality,
  destinationDirectory: string,
): DownloadRequestResult {
  const validation = validateVideoUrl(url);
  if (!validation.valid) {
    return { valid: false, message: validation.message };
  }
  return {
    valid: true,
    request: {
      url: validation.normalizedUrl,
      format,
      quality,
      destinationDirectory,
    },
  };
}

export const FORMAT_OPTIONS: Array<{
  id: MediaFormat;
  label: string;
  description: string;
}> = [
  { id: "mp4", label: "MP4 video", description: "Video with audio" },
  { id: "mp3", label: "MP3 audio", description: "Audio only" },
];

export const VIDEO_QUALITY_OPTIONS: Array<{ id: Quality; label: string }> = [
  { id: "best", label: "Best available" },
  { id: "1080p", label: "1080p" },
  { id: "720p", label: "720p" },
  { id: "480p", label: "480p" },
];

export const AUDIO_QUALITY_OPTIONS: Array<{ id: Quality; label: string }> = [
  { id: "best", label: "Best available" },
  { id: "320kbps", label: "320 kbps" },
  { id: "192kbps", label: "192 kbps" },
  { id: "128kbps", label: "128 kbps" },
];
