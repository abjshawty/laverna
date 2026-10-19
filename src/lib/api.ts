import { invoke } from "@tauri-apps/api/core";
import type { ItemOptions, Queue, Settings, ToolInfo } from "./types";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  detectTools: () => invoke<{ ytdlp: ToolInfo; ffmpeg: ToolInfo }>("detect_tools"),
  readTheme: () => invoke<string>("read_theme"),
  getThemeVars: () => invoke<Record<string, string>>("get_theme_vars"),
  setThemeVars: (vars: Record<string, string>) => invoke<string>("set_theme_vars", { vars }),
  getQueue: () => invoke<Queue>("get_queue"),
  addUrls: (urls: string, options: ItemOptions) => invoke<number[]>("add_urls", { urls, options }),
  removeItem: (id: number) => invoke<void>("remove_item", { id }),
  moveItem: (id: number, index: number) => invoke<void>("move_item", { id, index }),
  setItemOptions: (id: number, options: ItemOptions) => invoke<void>("set_item_options", { id, options }),
  retryItem: (id: number) => invoke<void>("retry_item", { id }),
  cancelCurrent: () => invoke<void>("cancel_current"),
  pauseQueue: () => invoke<void>("pause_queue"),
  resumeQueue: () => invoke<void>("resume_queue"),
  clearFinished: () => invoke<void>("clear_finished"),
  getLog: (id: number) => invoke<string[]>("get_log", { id }),
};
