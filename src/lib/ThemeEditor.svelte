<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api } from "./api";
  import { setUserCss, theme, THEME_VARS, toHex } from "./theme.svelte";

  /** Variables set by the editor (the managed block in theme.css). */
  let vars = $state<Record<string, string>>({});
  /** Effective values after all CSS applies, for showing what's in use. */
  let computed = $state<Record<string, string>>({});
  let error = $state("");
  let timer: ReturnType<typeof setTimeout> | undefined;

  onMount(async () => {
    vars = await api.getThemeVars();
  });

  // Recompute effective values whenever the user CSS or the mode changes.
  $effect(() => {
    theme.version;
    tick().then(() => {
      const style = getComputedStyle(document.documentElement);
      computed = Object.fromEntries(THEME_VARS.map((v) => [v.name, style.getPropertyValue(v.name).trim()]));
    });
  });

  function save(delay = 200) {
    clearTimeout(timer);
    timer = setTimeout(async () => {
      try {
        setUserCss(await api.setThemeVars($state.snapshot(vars)));
        error = "";
      } catch (e) {
        error = String(e);
      }
    }, delay);
  }

  function set(name: string, value: string) {
    if (value.trim()) vars[name] = value.trim();
    else delete vars[name];
    save();
  }

  function reset(name: string) {
    delete vars[name];
    save(0);
  }

  function resetAll() {
    vars = {};
    save(0);
  }
</script>

<div class="editor">
  {#each THEME_VARS as v (v.name)}
    {@const value = vars[v.name] ?? computed[v.name] ?? ""}
    <div class="var" class:overridden={v.name in vars}>
      <label for={`theme-${v.name}`}>{v.label}</label>
      <div class="inputs">
        {#if v.kind === "color"}
          <input
            type="color"
            class="swatch"
            value={toHex(value)}
            oninput={(e) => set(v.name, e.currentTarget.value)}
            aria-label={`${v.label} color`}
          />
        {/if}
        <input
          id={`theme-${v.name}`}
          class="text"
          {value}
          placeholder={computed[v.name]}
          onchange={(e) => set(v.name, e.currentTarget.value)}
          spellcheck="false"
        />
        <button
          type="button"
          class="reset"
          title="Reset to preset"
          aria-label={`Reset ${v.label}`}
          disabled={!(v.name in vars)}
          onclick={() => reset(v.name)}>↺</button
        >
      </div>
    </div>
  {/each}

  {#if error}<p class="error">{error}</p>{/if}

  <div class="footer">
    <button type="button" onclick={resetAll} disabled={Object.keys(vars).length === 0}>Reset all to preset</button>
  </div>
  <p class="note">
    Changes are saved to the editor's section of <code>~/.config/laverna/theme.css</code>. They apply in both light
    and dark mode. Anything you write by hand outside that section is kept and reloads live.
  </p>
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .var {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 8px;
  }
  .var label {
    color: var(--muted);
  }
  .var.overridden label {
    color: var(--fg);
  }
  .inputs {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .swatch {
    width: 28px;
    height: 28px;
    padding: 2px;
    cursor: pointer;
  }
  .text {
    width: 150px;
    padding: 4px 6px;
    font-size: 0.85em;
    font-family: ui-monospace, monospace;
  }
  .reset {
    width: 28px;
    padding: 4px 0;
  }
  .footer {
    margin-top: 8px;
  }
  .note {
    color: var(--muted);
    font-size: 0.85em;
  }
  code {
    font-size: 0.95em;
  }
  .error {
    color: var(--danger);
    margin: 4px 0;
  }
</style>
