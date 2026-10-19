<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import QueueRow from "$lib/QueueRow.svelte";
  import SettingsPanel from "$lib/SettingsPanel.svelte";
  import { applyMode } from "$lib/theme.svelte";
  import { PRESETS, type Item, type Preset, type Progress, type Queue, type Settings, type ToolInfo } from "$lib/types";

  const ITEM_MIME = "application/x-laverna-item";

  let settings = $state<Settings | null>(null);
  let queue = $state<Queue>({ items: [], next_id: 0, paused: false });
  let progress = $state<Record<number, Progress>>({});
  let tools = $state<{ ytdlp: ToolInfo; ffmpeg: ToolInfo } | null>(null);

  let urlInput = $state("");
  let preset = $state<Preset>("best");
  let subtitles = $state(false);
  let settingsOpen = $state(false);
  let logFor = $state<{ item: Item; lines: string[] } | null>(null);
  let dropping = $state(false);

  let remaining = $derived(
    queue.items.filter((i) => i.status.kind === "pending" || i.status.kind === "downloading").length,
  );
  let hasFinished = $derived(queue.items.some((i) => i.status.kind === "done" || i.status.kind === "skipped"));
  let missingTool = $derived(tools && (!tools.ytdlp.path || !tools.ffmpeg.path));

  onMount(() => {
    const unlisten = [
      listen<Queue>("queue", (e) => (queue = e.payload)),
      listen<{ id: number; progress: Progress }>("progress", (e) => {
        progress[e.payload.id] = e.payload.progress;
      }),
    ];
    (async () => {
      settings = await api.getSettings();
      applyMode(settings.theme_mode);
      preset = settings.default_preset;
      subtitles = settings.subtitles.include_by_default;
      queue = await api.getQueue();
      tools = await api.detectTools();
    })();
    return () => unlisten.forEach((u) => u.then((f) => f()));
  });

  async function addUrls(urls: string) {
    if (!urls.trim()) return;
    await api.addUrls(urls, { preset, subtitles, force: false });
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    await addUrls(urlInput);
    urlInput = "";
  }

  function droppedUrls(e: DragEvent): string | null {
    const dt = e.dataTransfer;
    if (!dt || dt.types.includes(ITEM_MIME)) return null;
    // uri-list may contain comment lines starting with '#'.
    const list = dt.getData("text/uri-list") || dt.getData("text/plain");
    const urls = list.split(/\s+/).filter((u) => /^https?:\/\//.test(u));
    return urls.length ? urls.join("\n") : null;
  }

  function dropOnRow(e: DragEvent, target: Item) {
    e.preventDefault();
    e.stopPropagation();
    dropping = false;
    const dragged = e.dataTransfer?.getData(ITEM_MIME);
    if (dragged) {
      const index = queue.items.findIndex((i) => i.id === target.id);
      api.moveItem(Number(dragged), index).catch(() => {});
      return;
    }
    const urls = droppedUrls(e);
    if (urls) addUrls(urls);
  }

  function dropOnWindow(e: DragEvent) {
    e.preventDefault();
    dropping = false;
    const urls = droppedUrls(e);
    if (urls) addUrls(urls);
  }

  async function showLog(item: Item) {
    logFor = { item, lines: await api.getLog(item.id) };
  }

  async function settingsSaved(next: Settings) {
    const prev = settings;
    settings = next;
    applyMode(next.theme_mode);
    // Keep the add bar in step with new defaults.
    if (prev?.default_preset !== next.default_preset) preset = next.default_preset;
    if (prev?.subtitles.include_by_default !== next.subtitles.include_by_default) {
      subtitles = next.subtitles.include_by_default;
    }
    if (prev?.ytdlp_path !== next.ytdlp_path || prev?.ffmpeg_path !== next.ffmpeg_path) {
      tools = await api.detectTools();
    }
  }

  function togglePause() {
    queue.paused ? api.resumeQueue() : api.pauseQueue();
  }
</script>

<main
  ondragover={(e) => {
    e.preventDefault();
    dropping = !e.dataTransfer?.types.includes(ITEM_MIME);
  }}
  ondragleave={(e) => {
    if (!e.relatedTarget) dropping = false;
  }}
  ondrop={dropOnWindow}
  class:dropping
>
  <form class="add-bar" onsubmit={submit}>
    <input class="url" placeholder="Paste YouTube links…" bind:value={urlInput} aria-label="Video URLs" />
    <select bind:value={preset} aria-label="Quality">
      {#each PRESETS as p}
        <option value={p.value}>{p.label}</option>
      {/each}
    </select>
    <label class="toggle" class:disabled={preset === "audio"}>
      <input type="checkbox" bind:checked={subtitles} disabled={preset === "audio"} />
      Subs
    </label>
    <button class="primary" type="submit" disabled={!urlInput.trim()}>Add</button>
    <button type="button" class="icon" aria-label="Settings" onclick={() => (settingsOpen = true)}>⚙</button>
  </form>

  {#if missingTool}
    <div class="banner">
      {!tools?.ytdlp.path ? "yt-dlp" : "ffmpeg"} was not found on PATH. Install it or set its path in Settings.
    </div>
  {/if}

  <section class="queue">
    {#if queue.items.length === 0}
      <p class="empty">Nothing queued. Paste or drop a link.</p>
    {:else}
      <ul>
        {#each queue.items as item (item.id)}
          <QueueRow
            {item}
            progress={progress[item.id]}
            onShowLog={showLog}
            ondragstart={(e) => e.dataTransfer?.setData(ITEM_MIME, String(item.id))}
            ondrop={(e) => dropOnRow(e, item)}
          />
        {/each}
      </ul>
    {/if}
  </section>

  <footer class="status-line">
    <span>
      {remaining} remaining{#if queue.paused && remaining > 0}&nbsp;· paused{/if}
    </span>
    <span class="footer-actions">
      {#if hasFinished}
        <button type="button" onclick={() => api.clearFinished()}>Clear finished</button>
      {/if}
      <button type="button" onclick={togglePause} disabled={remaining === 0 && !queue.paused}>
        {queue.paused ? "Resume" : "Pause"}
      </button>
    </span>
  </footer>
</main>

{#if logFor}
  <div class="scrim" role="presentation" onclick={() => (logFor = null)}></div>
  <div class="log" role="dialog" aria-label="Download log">
    <header>
      <h2>{logFor.item.meta.title ?? logFor.item.url}</h2>
      <button type="button" class="icon" aria-label="Close" onclick={() => (logFor = null)}>✕</button>
    </header>
    <pre>{logFor.lines.length ? logFor.lines.join("\n") : "No output from the last attempt (logs aren't kept across restarts)."}</pre>
  </div>
{/if}

{#if settingsOpen && settings}
  <SettingsPanel {settings} {tools} onSaved={settingsSaved} onClose={() => (settingsOpen = false)} />
{/if}

<style>
  /* Flex, not grid: the banner is conditional, so fixed grid rows would shift. */
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  main.dropping .queue {
    outline: 2px dashed var(--accent);
    outline-offset: -8px;
  }

  .add-bar {
    display: flex;
    gap: 8px;
    padding: 12px;
    border-bottom: 1px solid var(--border);
  }
  .url {
    flex: 1;
    min-width: 0;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
  }
  .toggle.disabled {
    opacity: 0.5;
  }
  .icon {
    width: 34px;
    padding: 6px 0;
  }

  .banner {
    padding: 8px 12px;
    background: color-mix(in srgb, var(--warning) 18%, transparent);
    border-bottom: 1px solid var(--border);
  }

  .queue {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .empty {
    color: var(--muted);
    text-align: center;
    margin-top: 80px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 8px;
  }

  .status-line {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border-top: 1px solid var(--border);
    color: var(--muted);
  }
  .footer-actions {
    display: flex;
    gap: 8px;
  }

  .scrim {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.3);
  }
  .log {
    position: fixed;
    inset: 40px;
    display: flex;
    flex-direction: column;
    padding: 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .log pre {
    flex: 1;
    overflow: auto;
    margin: 12px 0 0;
    font-size: 0.8em;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: 1.2em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
 </style>
