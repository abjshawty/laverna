<script lang="ts">
  import "$lib/theme.css";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let { children } = $props();
  let userTheme = $state("");

  // Injected after theme.css so the user's file wins. Live reload comes with the file watcher.
  onMount(async () => {
    userTheme = await invoke<string>("read_theme");
  });
</script>

<svelte:head>
  {@html `<style id="user-theme">${userTheme.replaceAll("</", "<\\/")}</style>`}
</svelte:head>

{@render children()}
