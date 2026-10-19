import type { ThemeMode } from "./types";

/** The user's theme.css content, plus a counter bumped after any change that affects computed styles. */
export const theme = $state({ css: "", version: 0 });

export function setUserCss(css: string) {
  theme.css = css;
  theme.version++;
}

export function applyMode(mode: ThemeMode) {
  const root = document.documentElement;
  if (mode === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", mode);
  theme.version++;
}

export interface ThemeVar {
  name: string;
  label: string;
  kind: "color" | "text";
}

/** The theme contract: must match the variables defined in theme.css. */
export const THEME_VARS: ThemeVar[] = [
  { name: "--bg", label: "Background", kind: "color" },
  { name: "--surface", label: "Surface", kind: "color" },
  { name: "--surface-hover", label: "Surface (hover)", kind: "color" },
  { name: "--border", label: "Border", kind: "color" },
  { name: "--fg", label: "Text", kind: "color" },
  { name: "--muted", label: "Muted text", kind: "color" },
  { name: "--accent", label: "Accent", kind: "color" },
  { name: "--accent-fg", label: "Text on accent", kind: "color" },
  { name: "--progress-track", label: "Progress track", kind: "color" },
  { name: "--success", label: "Success", kind: "color" },
  { name: "--warning", label: "Warning", kind: "color" },
  { name: "--danger", label: "Danger", kind: "color" },
  { name: "--radius", label: "Corner radius", kind: "text" },
  { name: "--font", label: "Font", kind: "text" },
  { name: "--font-size", label: "Font size", kind: "text" },
];

let ctx: CanvasRenderingContext2D | null = null;

/** Normalizes any CSS color to #rrggbb for <input type="color"> (alpha is dropped). */
export function toHex(color: string): string {
  ctx ??= document.createElement("canvas").getContext("2d");
  if (!ctx) return "#000000";
  ctx.fillStyle = "#000000";
  ctx.fillStyle = color;
  const v = ctx.fillStyle;
  if (v.startsWith("#")) return v;
  const m = v.match(/\d+(\.\d+)?/g);
  if (!m) return "#000000";
  return "#" + m.slice(0, 3).map((n) => Math.round(Number(n)).toString(16).padStart(2, "0")).join("");
}
