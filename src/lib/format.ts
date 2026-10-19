import type { Phase, Progress } from "./types";

export function bytes(n: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function clock(seconds: number): string {
  const s = Math.round(seconds);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = String(s % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

const PHASES: Record<Phase, string> = {
  video: "Video",
  audio: "Audio",
  download: "Downloading",
  processing: "Processing",
};

export function progressLine(p: Progress): string {
  if (p.phase === "processing") return "Merging & processing…";
  const parts = [PHASES[p.phase]];
  if (p.fraction != null) parts.push(`${Math.floor(p.fraction * 100)}%`);
  if (p.speed != null) parts.push(`${bytes(p.speed)}/s`);
  if (p.eta != null) parts.push(`ETA ${clock(p.eta)}`);
  if (p.total != null) parts.push(bytes(p.total));
  return parts.join(" · ");
}
