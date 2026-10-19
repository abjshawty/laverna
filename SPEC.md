# Laverna — Spec

A minimal desktop wrapper around `yt-dlp` for downloading videos (YouTube-focused, but any
yt-dlp-supported URL works), with a sequential download queue and a user-customizable theme.

## Stack & layout

- Tauri v2. Frontend: Svelte 5 + Vite + TypeScript, run with bun. Rust app in `src-tauri/`.
- Linux first; no platform-specific code (paths via Tauri's path API, no shell-isms).
- Packaging: `tauri build` → AppImage + `.deb`. AUR PKGBUILD later.
- Bundle identifier: `dev.abjshawty.laverna`.
- Testing: pure Rust functions with unit tests — yt-dlp arg building, progress-line parsing,
  queue state transitions. No e2e/UI tests in v1.

## Engine

- `yt-dlp` and `ffmpeg` are resolved from `PATH`, overridable by custom paths in settings.
  Missing binary → clear error state in the UI. Designed so a managed (downloaded, self-updating)
  binary can be added later.
- Rust spawns the yt-dlp CLI (`tokio::process`) with `--newline --progress-template` emitting
  JSON lines, parses them, and emits Tauri events to the frontend.
- One download worker → strictly sequential downloads.
- Separate metadata resolver (`yt-dlp -J --flat-playlist`), concurrency 2, fills in title,
  thumbnail, duration; the raw URL is shown until resolved. Also expands playlists.
- Settings shows detected yt-dlp / ffmpeg versions and paths. Extractor-looking errors include a
  "try updating yt-dlp" hint.

## Input

- One URL field; accepts multiple URLs split on whitespace/newlines. Drag-and-drop of links onto
  the window. No clipboard monitoring.
- Playlist URLs expand into one queue entry per video. Channels: out of scope for v1.

## Quality & format

- Presets: **Best** (default), 2160p, 1440p, 1080p, 720p, 480p, Audio only. Requesting a preset
  above what's available falls back to the best available.
- Container: **MKV** default (lossless merge, any codec); MP4 / WebM selectable. MP4 adds
  `-S vcodec:h264,acodec:aac` and warns this may cap quality.
- Audio only: keep original stream by default; optional conversion to MP3 or M4A.
- Embedded by default: metadata, thumbnail, chapters.
- Subtitles:
  - Global setting **"Include subtitles by default"** (default off), overridable per item.
  - Languages field, default `en` (comma-separated → `--sub-langs`).
  - Separate toggle "Include auto-generated captions" (`--write-auto-subs`), default off.
  - Subtitles are embedded. Ignored for audio-only items.

## Queue

- The add bar has a quality dropdown and a subtitles toggle, prefilled from global defaults;
  they apply to the URLs being added. Pending items are editable via a per-item menu;
  downloading/done items are locked.
- Controls: remove, drag-reorder pending items, cancel current (→ cancelled, retryable),
  retry failed/cancelled, pause/resume whole queue (pause stops the current download; resume
  continues it with `--continue`), clear finished.
- Failure: mark failed with yt-dlp's last error line (full log viewable on the item), continue to
  the next item. No app-level retries (yt-dlp retries fragments itself).
- Duplicates: `--download-archive` file in the app data dir; already-downloaded items are marked
  "already downloaded" and skipped, with a per-item "download anyway".
- Persistence: queue saved as JSON in the app data dir. On restart, pending/interrupted items
  come back paused; partial files resume via `--continue`.
- Per-item progress: percentage bar, speed, ETA, size, and phase label
  (video / audio / merging-processing).
- Finished items: "Open" and "Show in folder" (Tauri opener plugin).
- Output: default `~/Videos` (configurable), template `%(title)s [%(id)s].%(ext)s`.

## UI

- Single window, ~760×540, resizable.
  - Top: add bar (URL field, quality dropdown, subtitles toggle, add button).
  - Middle: queue list — thumbnail, title, progress/status, row actions.
  - Bottom: status line (remaining count, pause/resume).
  - Gear opens Settings as a slide-over panel; the theme editor is a section inside it.
- No tabs, no extra windows.
- Desktop notifications on queue completion and on item failure. No tray in v1.
- Closing while downloading asks for confirmation. The queue is persisted regardless.

## Theming

- The app defines a fixed set of CSS custom properties (`--bg`, `--fg`, `--muted`, `--surface`,
  `--border`, `--accent`, `--accent-fg`, `--danger`, `--success`, `--radius`, `--font`, …).
- Built-in **Dark** and **Light** presets; default follows `prefers-color-scheme`.
- User file `~/.config/laverna/theme.css` overrides the preset and is watched for live reload.
- The in-app theme editor (color pickers, plus text fields for non-colors, and "reset to preset")
  edits those variables and writes them back to the same file, keeping any other hand-written
  CSS intact. It owns one marked block (`/* laverna:editor:start … */ … /* laverna:editor:end */`)
  kept at the end of the file; its overrides apply in both modes.
- Presets use zero-specificity selectors (`:where()`), so any user `:root { … }` rule wins.
- Mode setting: Follow system (default) / Dark / Light.
- Omarchy theme import: later, not v1.

## Settings

Stored in `~/.config/laverna/settings.json`, next to `theme.css`:

- Output directory, filename template
- Default quality preset, container, audio conversion
- Subtitles: include by default, languages, auto-generated captions
- yt-dlp / ffmpeg custom paths (+ detected versions shown)
- Cookies from browser (dropdown, default off). Includes **Zen**, which yt-dlp doesn't know by
  name: it's passed as `firefox:<profile dir>`, with the profile taken from Zen's `profiles.ini`
  (`[Install…] Default=` first, then `Default=1`, then the first profile). Settings shows the
  resolved profile path.
- Rate limit (`-r`, default empty)
- Extra yt-dlp args (free text, appended last)
- Theme mode (system / dark / light)

Settings autosave on change. Invalid extra args are rejected with an inline error.
