<script lang="ts">
  import "$lib/theme.css";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { setUserCss, theme } from "$lib/theme.svelte";

  let { children } = $props();

  // Injected after theme.css so the user's file wins. The backend re-emits it on every file change.
  onMount(() => {
    const unlisten = listen<string>("theme", (e) => setUserCss(e.payload));
    api.readTheme().then(setUserCss);
    return () => unlisten.then((f) => f());
  });
</script>

<svelte:head>
  {@html `<style id="user-theme">${theme.css.replaceAll("</", "<\\/")}</style>`}
</svelte:head>

{@render children()}
