// Mirrors the serde shapes in src-tauri/src.

export type Preset = "best" | "2160p" | "1440p" | "1080p" | "720p" | "480p" | "audio";
export const PRESETS: { value: Preset; label: string }[] = [
  { value: "best", label: "Best" },
  { value: "2160p", label: "2160p" },
  { value: "1440p", label: "1440p" },
  { value: "1080p", label: "1080p" },
  { value: "720p", label: "720p" },
  { value: "480p", label: "480p" },
  { value: "audio", label: "Audio only" },
];

export interface Settings {
  output_dir: string | null;
  filename_template: string;
  default_preset: Preset;
  container: "mkv" | "mp4" | "webm";
  audio_format: "original" | "mp3" | "m4a";
  subtitles: { include_by_default: boolean; languages: string; auto_generated: boolean };
  ytdlp_path: string | null;
  ffmpeg_path: string | null;
  cookies_from_browser: string | null;
  rate_limit: string;
  extra_args: string;
  theme_mode: ThemeMode;
}

export type ThemeMode = "system" | "dark" | "light";

export interface ItemOptions {
  preset: Preset;
  subtitles: boolean;
  force: boolean;
}

export type Status =
  | { kind: "pending" }
  | { kind: "downloading" }
  | { kind: "done"; path: string | null }
  | { kind: "skipped" }
  | { kind: "failed"; error: string; hint_update: boolean }
  | { kind: "cancelled" };

export interface Item {
  id: number;
  url: string;
  options: ItemOptions;
  meta: { title: string | null; thumbnail: string | null; duration: number | null; resolved: boolean };
  status: Status;
}

export interface Queue {
  items: Item[];
  next_id: number;
  paused: boolean;
}

export interface ToolInfo {
  path: string | null;
  version: string | null;
}

export type Phase = "video" | "audio" | "download" | "processing";

export interface Progress {
  phase: Phase;
  fraction: number | null;
  downloaded: number | null;
  total: number | null;
  speed: number | null;
  eta: number | null;
}
