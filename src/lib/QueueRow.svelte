<script lang="ts">
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { api } from "./api";
  import { clock, progressLine } from "./format";
  import { PRESETS, type Item, type Preset, type Progress } from "./types";

  let {
    item,
    progress,
    onShowLog,
    ondragstart,
    ondrop,
  }: {
    item: Item;
    progress: Progress | undefined;
    onShowLog: (item: Item) => void;
    ondragstart: (e: DragEvent) => void;
    ondrop: (e: DragEvent) => void;
  } = $props();

  let editing = $state(false);
  let kind = $derived(item.status.kind);
  let pending = $derived(kind === "pending");

  function setPreset(preset: Preset) {
    api.setItemOptions(item.id, { ...item.options, preset });
  }
  function setSubtitles(subtitles: boolean) {
    api.setItemOptions(item.id, { ...item.options, subtitles });
  }
</script>

<li
  class="row"
  data-status={kind}
  draggable={pending}
  {ondragstart}
  ondragover={(e) => e.preventDefault()}
  {ondrop}
>
  <div class="thumb">
    {#if item.meta.thumbnail}<img src={item.meta.thumbnail} alt="" />{/if}
    {#if item.meta.duration}<span class="duration">{clock(item.meta.duration)}</span>{/if}
  </div>

  <div class="info">
    <div class="title" title={item.url}>{item.meta.title ?? item.url}</div>

    {#if kind === "downloading"}
      <div class="bar" class:indeterminate={progress?.fraction == null}>
        <div class="fill" style:width={progress?.fraction != null ? `${progress.fraction * 100}%` : undefined}></div>
      </div>
      <div class="status">{progress ? progressLine(progress) : "Starting…"}</div>
    {:else if item.status.kind === "failed"}
      <div class="status error" title={item.status.error}>{item.status.error}</div>
      {#if item.status.hint_update}
        <div class="status hint">This often means yt-dlp is out of date. Try updating it.</div>
      {/if}
    {:else}
      <div class="status">
        {#if kind === "pending"}
          {item.meta.resolved ? "Queued" : "Looking up…"}
          · {PRESETS.find((p) => p.value === item.options.preset)?.label}
          {#if item.options.subtitles && item.options.preset !== "audio"}· Subs{/if}
          {#if item.options.force}· Re-download{/if}
        {:else if kind === "done"}Done
        {:else if kind === "skipped"}Already downloaded
        {:else if kind === "cancelled"}Cancelled
        {/if}
      </div>
    {/if}

    {#if editing && pending}
      <div class="edit">
        <select value={item.options.preset} onchange={(e) => setPreset(e.currentTarget.value as Preset)}>
          {#each PRESETS as p}<option value={p.value}>{p.label}</option>{/each}
        </select>
        <label class:disabled={item.options.preset === "audio"}>
          <input
            type="checkbox"
            checked={item.options.subtitles}
            disabled={item.options.preset === "audio"}
            onchange={(e) => setSubtitles(e.currentTarget.checked)}
          />
          Subtitles
        </label>
      </div>
    {/if}
  </div>

  <div class="actions">
    {#if kind === "pending"}
      <button class="sm" onclick={() => (editing = !editing)} aria-pressed={editing}>Options</button>
      <button class="sm" onclick={() => api.removeItem(item.id)}>Remove</button>
    {:else if kind === "downloading"}
      <button class="sm" onclick={() => api.cancelCurrent()}>Cancel</button>
    {:else if item.status.kind === "done"}
      {@const path = item.status.path}
      {#if path}
        <button class="sm" onclick={() => openPath(path)}>Open</button>
        <button class="sm" onclick={() => revealItemInDir(path)}>Show</button>
      {/if}
      <button class="sm" onclick={() => api.removeItem(item.id)}>Remove</button>
    {:else if kind === "skipped"}
      <button class="sm" onclick={() => api.retryItem(item.id)}>Download anyway</button>
      <button class="sm" onclick={() => api.removeItem(item.id)}>Remove</button>
    {:else}
      <button class="sm" onclick={() => api.retryItem(item.id)}>Retry</button>
      <button class="sm" onclick={() => onShowLog(item)}>Log</button>
      <button class="sm" onclick={() => api.removeItem(item.id)}>Remove</button>
    {/if}
  </div>
</li>

<style>
  .row {
    display: flex;
    gap: 12px;
    padding: 8px;
    border-radius: var(--radius);
    align-items: center;
  }
  .row:hover {
    background: var(--surface-hover);
  }
  .row[draggable="true"] {
    cursor: grab;
  }
  .thumb {
    position: relative;
    width: 96px;
    aspect-ratio: 16 / 9;
    flex-shrink: 0;
    border-radius: calc(var(--radius) / 2);
    background: var(--surface);
    overflow: hidden;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .duration {
    position: absolute;
    right: 3px;
    bottom: 3px;
    padding: 0 4px;
    font-size: 0.75em;
    border-radius: 3px;
    background: rgb(0 0 0 / 0.7);
    color: #fff;
  }
  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .title,
  .status {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status {
    color: var(--muted);
    font-size: 0.9em;
  }
  .row[data-status="done"] .status {
    color: var(--success);
  }
  .error {
    color: var(--danger);
  }
  .hint {
    color: var(--warning);
  }
  .bar {
    height: 4px;
    border-radius: 2px;
    background: var(--progress-track);
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.15s linear;
  }
  .indeterminate .fill {
    width: 30%;
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes slide {
    from { transform: translateX(-100%); }
    to { transform: translateX(340%); }
  }
  .edit {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 2px;
  }
  .edit label {
    display: flex;
    gap: 4px;
    align-items: center;
    color: var(--muted);
  }
  .disabled {
    opacity: 0.5;
  }
  .actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }
  .sm {
    padding: 3px 8px;
    font-size: 0.85em;
  }
</style>
