<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { untrack } from "svelte";
  import { api } from "./api";
  import ThemeEditor from "./ThemeEditor.svelte";
  import { PRESETS, type Settings, type ToolInfo } from "./types";

  let {
    settings,
    tools,
    onSaved,
    onClose,
  }: {
    settings: Settings;
    tools: { ytdlp: ToolInfo; ffmpeg: ToolInfo } | null;
    onSaved: (s: Settings) => void;
    onClose: () => void;
  } = $props();

  // Text fields edit plain strings; empty means "not set" and is saved as null.
  // Seeded once per opening of the panel; later prop changes come from our own saves.
  const initial = untrack(() => $state.snapshot(settings));
  let form = $state({
    ...initial,
    output_dir: initial.output_dir ?? "",
    ytdlp_path: initial.ytdlp_path ?? "",
    ffmpeg_path: initial.ffmpeg_path ?? "",
    cookies_from_browser: initial.cookies_from_browser ?? "",
  });
  let error = $state("");
  let saved = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let first = true;

  const BROWSERS = ["firefox", "chrome", "chromium", "brave", "edge", "opera", "vivaldi", "safari", "whale"];
  const nullable = (s: string) => (s.trim() ? s.trim() : null);

  function normalized(): Settings {
    const f = $state.snapshot(form);
    return {
      ...f,
      output_dir: nullable(f.output_dir),
      ytdlp_path: nullable(f.ytdlp_path),
      ffmpeg_path: nullable(f.ffmpeg_path),
      cookies_from_browser: nullable(f.cookies_from_browser),
      filename_template: f.filename_template.trim() || initial.filename_template,
    };
  }

  // Autosave: any change to the form saves after a short pause.
  $effect(() => {
    JSON.stringify(form);
    if (first) {
      first = false;
      return;
    }
    clearTimeout(timer);
    saved = false;
    timer = setTimeout(async () => {
      const next = normalized();
      try {
        await api.saveSettings(next);
        error = "";
        saved = true;
        onSaved(next);
      } catch (e) {
        error = String(e);
      }
    }, 400);
  });

  async function browseDir() {
    const dir = await open({ directory: true, defaultPath: form.output_dir || undefined });
    if (typeof dir === "string") form.output_dir = dir;
  }

  async function browseFile(field: "ytdlp_path" | "ffmpeg_path") {
    const file = await open({ multiple: false, directory: false });
    if (typeof file === "string") form[field] = file;
  }
</script>

<div class="scrim" role="presentation" onclick={onClose}></div>
<aside class="settings" aria-label="Settings">
  <header>
    <h2>Settings</h2>
    <span class="save-state" class:error={!!error}>{error || (saved ? "Saved" : "")}</span>
    <button type="button" class="icon" aria-label="Close" onclick={onClose}>✕</button>
  </header>

  <h3>Downloads</h3>
  <div class="field">
    <label for="s-out">Save to</label>
    <div class="row">
      <input id="s-out" bind:value={form.output_dir} placeholder="~/Videos" />
      <button type="button" onclick={browseDir}>Browse</button>
    </div>
  </div>
  <div class="field">
    <label for="s-tpl">File name</label>
    <input id="s-tpl" bind:value={form.filename_template} spellcheck="false" />
    <small>yt-dlp output template, e.g. <code>%(title)s [%(id)s].%(ext)s</code></small>
  </div>
  <div class="grid">
    <div class="field">
      <label for="s-preset">Default quality</label>
      <select id="s-preset" bind:value={form.default_preset}>
        {#each PRESETS as p}<option value={p.value}>{p.label}</option>{/each}
      </select>
    </div>
    <div class="field">
      <label for="s-container">Video format</label>
      <select id="s-container" bind:value={form.container}>
        <option value="mkv">MKV</option>
        <option value="mp4">MP4</option>
        <option value="webm">WebM</option>
      </select>
    </div>
    <div class="field">
      <label for="s-audio">Audio only format</label>
      <select id="s-audio" bind:value={form.audio_format}>
        <option value="original">Original (no conversion)</option>
        <option value="mp3">MP3</option>
        <option value="m4a">M4A</option>
      </select>
    </div>
  </div>
  {#if form.container === "mp4"}
    <small class="warn">MP4 prefers H.264/AAC streams, which YouTube often caps at 1080p.</small>
  {/if}

  <h3>Subtitles</h3>
  <label class="check">
    <input type="checkbox" bind:checked={form.subtitles.include_by_default} />
    Include subtitles by default
  </label>
  <label class="check">
    <input type="checkbox" bind:checked={form.subtitles.auto_generated} />
    Include auto-generated captions
  </label>
  <div class="field">
    <label for="s-langs">Languages</label>
    <input id="s-langs" bind:value={form.subtitles.languages} placeholder="en" spellcheck="false" />
    <small>Comma-separated, e.g. <code>en,fr</code>. Subtitles are embedded in the video.</small>
  </div>

  <h3>Network</h3>
  <div class="grid">
    <div class="field">
      <label for="s-cookies">Cookies from browser</label>
      <select id="s-cookies" bind:value={form.cookies_from_browser}>
        <option value="">Off</option>
        {#each BROWSERS as b}<option value={b}>{b}</option>{/each}
      </select>
    </div>
    <div class="field">
      <label for="s-rate">Speed limit</label>
      <input id="s-rate" bind:value={form.rate_limit} placeholder="Unlimited, e.g. 2M" spellcheck="false" />
    </div>
  </div>
  <small>Cookies help with age-restricted videos and YouTube's “confirm you're not a bot” check.</small>

  <h3>Tools</h3>
  <div class="field">
    <label for="s-ytdlp">yt-dlp</label>
    <div class="row">
      <input id="s-ytdlp" bind:value={form.ytdlp_path} placeholder="From PATH" spellcheck="false" />
      <button type="button" onclick={() => browseFile("ytdlp_path")}>Browse</button>
    </div>
    <small class:warn={!tools?.ytdlp.path}>
      {tools?.ytdlp.path ? `${tools.ytdlp.version ?? "unknown version"} · ${tools.ytdlp.path}` : "Not found"}
    </small>
  </div>
  <div class="field">
    <label for="s-ffmpeg">ffmpeg</label>
    <div class="row">
      <input id="s-ffmpeg" bind:value={form.ffmpeg_path} placeholder="From PATH" spellcheck="false" />
      <button type="button" onclick={() => browseFile("ffmpeg_path")}>Browse</button>
    </div>
    <small class:warn={!tools?.ffmpeg.path}>
      {tools?.ffmpeg.path ? `${tools.ffmpeg.version ?? "unknown version"} · ${tools.ffmpeg.path}` : "Not found"}
    </small>
  </div>

  <h3>Advanced</h3>
  <div class="field">
    <label for="s-extra">Extra yt-dlp arguments</label>
    <input id="s-extra" bind:value={form.extra_args} placeholder="--sponsorblock-remove sponsor" spellcheck="false" />
    <small>Added after Laverna's own arguments.</small>
  </div>

  <h3>Appearance</h3>
  <div class="field">
    <label for="s-mode">Mode</label>
    <select id="s-mode" bind:value={form.theme_mode}>
      <option value="system">Follow system</option>
      <option value="dark">Dark</option>
      <option value="light">Light</option>
    </select>
  </div>
  <ThemeEditor />
</aside>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.3);
  }
  .settings {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(420px, 92vw);
    padding: 16px;
    background: var(--surface);
    border-left: 1px solid var(--border);
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: 1.2em;
  }
  .save-state {
    flex: 1;
    color: var(--muted);
    font-size: 0.85em;
  }
  .save-state.error {
    color: var(--danger);
  }
  .icon {
    width: 34px;
    padding: 6px 0;
  }
  h3 {
    font-size: 0.85em;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    margin: 16px 0 0;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .field > label {
    font-size: 0.9em;
  }
  .row {
    display: flex;
    gap: 6px;
  }
  .row input {
    flex: 1;
    min-width: 0;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 8px;
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  small {
    color: var(--muted);
    font-size: 0.8em;
    overflow-wrap: anywhere;
  }
  .warn {
    color: var(--warning);
  }
</style>
