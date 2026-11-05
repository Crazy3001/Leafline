// A saved session as something to post: a picture drawn on a canvas and a few lines of text.
// Leafline never posts anything; the player copies or saves it and shares it themselves. The
// picture is drawn from scratch, with no game art, and leaves out the character's name unless
// the player asks for it.

import { formatCompact, formatDuration, splitMapLabel } from "./format";
import { tallyByMonster } from "./drops.svelte";
import { longDay } from "./training";
import type { LoggedDrop, SavedSession, SessionTimeline } from "./types";

export interface ShareOptions {
  name: boolean;
  maps: boolean;
  drops: boolean;
}

export const WIDTH = 1200;
export const HEIGHT = 630;

const C = {
  bg: "#141412",
  panel: "#1a1917",
  line: "rgba(236, 230, 216, 0.1)",
  text: "#ece6d8",
  muted: "#a8a193",
  faint: "#938c7e",
  accent: "#e0a030",
  exp: "#d9c036",
  good: "#6fbf73",
  bad: "#e06a66",
};
const SANS = `"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif`;
const DISPLAY = `Bahnschrift, "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif`;

const percentOnly = (s: SavedSession) => s.expPerHour == null && s.percentPerHour != null;

function levels(s: SavedSession): string | null {
  if (s.level == null) return null;
  const start = s.level - s.levelUps;
  return s.levelUps ? `Lv ${start} → ${s.level}` : `Lv ${s.level}`;
}

function title(s: SavedSession): string {
  if (s.label) return s.label;
  return s.maps.length ? splitMapLabel(s.maps[0].map).name : "Training session";
}

function expRate(s: SavedSession): string {
  return percentOnly(s) ? `${(s.percentPerHour ?? 0).toFixed(1)}%` : formatCompact(s.expPerHour);
}

/** The session's drops as "Item ×2", the most logged first. */
function dropWords(drops: LoggedDrop[], limit: number): string[] {
  const items = new Map<string, number>();
  for (const t of tallyByMonster(drops)) for (const i of t.items) items.set(i.item, (items.get(i.item) ?? 0) + i.count);
  return [...items]
    .sort((a, b) => b[1] - a[1])
    .slice(0, limit)
    .map(([item, n]) => (n > 1 ? `${item} ×${n}` : item));
}

function mapWords(s: SavedSession, limit: number): string[] {
  return s.maps.slice(0, limit).map((m) => `${splitMapLabel(m.map).name} (${formatDuration(m.activeSeconds)})`);
}

/** A few lines for a chat message. */
export function shareText(s: SavedSession, drops: LoggedDrop[], o: ShareOptions): string {
  const who = [o.name ? s.character : null, levels(s), s.job].filter(Boolean).join(" · ");
  const lines = [
    `${title(s)} · ${longDay(s.startedAt ?? s.savedAt)}`,
    [who, formatDuration(s.activeSeconds)].filter(Boolean).join(" · "),
    `${expRate(s)} ${percentOnly(s) ? "per hour" : "EXP/hr"} · ${formatCompact(s.mesosPerHour)} mesos/hr`,
  ];
  if (o.maps && s.maps.length > 1) lines.push(`Maps: ${mapWords(s, 3).join(", ")}`);
  if (o.drops && drops.length) lines.push(`Drops: ${dropWords(drops, 6).join(", ")}`);
  lines.push("Tracked with Leafline");
  return lines.join("\n");
}

function fit(ctx: CanvasRenderingContext2D, text: string, width: number): string {
  if (ctx.measureText(text).width <= width) return text;
  let cut = text;
  while (cut.length > 1 && ctx.measureText(cut + "…").width > width) cut = cut.slice(0, -1);
  return cut.trimEnd() + "…";
}

function leaf(ctx: CanvasRenderingContext2D, x: number, y: number, size: number) {
  const k = size / 24;
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(k, k);
  ctx.fillStyle = C.accent;
  ctx.fill(new Path2D("M4.5 19.5C4 11 9.5 4.5 20 4c.4 9.8-5.6 15.6-15.5 15.5Z"));
  ctx.strokeStyle = "rgba(42, 26, 5, 0.6)";
  ctx.lineWidth = 1.6;
  ctx.stroke(new Path2D("M4.5 19.5 14 10"));
  ctx.restore();
}

function stat(ctx: CanvasRenderingContext2D, x: number, y: number, label: string, value: string, color: string) {
  ctx.fillStyle = C.faint;
  ctx.font = `600 17px ${SANS}`;
  ctx.fillText(label.toUpperCase(), x, y);
  ctx.fillStyle = color;
  ctx.font = `600 56px ${DISPLAY}`;
  ctx.fillText(value, x, y + 58);
}

function chart(ctx: CanvasRenderingContext2D, s: SavedSession, t: SessionTimeline, box: { x: number; y: number; w: number; h: number }) {
  const pts = t.points
    .map((p) => ({ x: p.at, y: percentOnly(s) ? p.percentPerHour : p.expPerHour }))
    .filter((p): p is { x: number; y: number } => p.y != null);
  if (pts.length < 2) return false;
  const xMax = Math.max(s.activeSeconds, pts[pts.length - 1].x) || 1;
  const yMax = Math.max(...pts.map((p) => p.y)) * 1.15 || 1;
  const px = (x: number) => box.x + (x / xMax) * box.w;
  const py = (y: number) => box.y + box.h - (y / yMax) * box.h;

  ctx.strokeStyle = C.line;
  ctx.lineWidth = 1;
  for (let i = 0; i <= 2; i++) {
    const y = Math.round(box.y + (box.h * i) / 2) + 0.5;
    ctx.beginPath();
    ctx.moveTo(box.x, y);
    ctx.lineTo(box.x + box.w, y);
    ctx.stroke();
  }

  ctx.beginPath();
  pts.forEach((p, i) => (i ? ctx.lineTo(px(p.x), py(p.y)) : ctx.moveTo(px(p.x), py(p.y))));
  ctx.lineTo(px(pts[pts.length - 1].x), box.y + box.h);
  ctx.lineTo(px(pts[0].x), box.y + box.h);
  ctx.closePath();
  ctx.fillStyle = "rgba(217, 192, 54, 0.12)";
  ctx.fill();

  ctx.beginPath();
  pts.forEach((p, i) => (i ? ctx.lineTo(px(p.x), py(p.y)) : ctx.moveTo(px(p.x), py(p.y))));
  ctx.strokeStyle = C.exp;
  ctx.lineWidth = 3;
  ctx.lineJoin = "round";
  ctx.stroke();

  for (const e of t.events) {
    ctx.fillStyle = e.kind === "levelUp" ? C.good : C.bad;
    ctx.fillRect(Math.round(px(Math.min(e.at, xMax))) - 4, box.y + box.h - 8, 8, 8);
  }

  ctx.fillStyle = C.faint;
  ctx.font = `15px ${SANS}`;
  ctx.fillText(`${percentOnly(s) ? "%" : "EXP"} per hour over the session`, box.x, box.y - 12);
  ctx.textAlign = "right";
  ctx.fillText(`${formatDuration(xMax)} of play`, box.x + box.w, box.y - 12);
  ctx.textAlign = "left";
  return true;
}

/** Draws the share picture onto a canvas WIDTH × HEIGHT pixels. */
export function drawShareCard(
  canvas: HTMLCanvasElement,
  s: SavedSession,
  t: SessionTimeline | null,
  drops: LoggedDrop[],
  o: ShareOptions,
) {
  canvas.width = WIDTH;
  canvas.height = HEIGHT;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const pad = 56;

  ctx.fillStyle = C.bg;
  ctx.fillRect(0, 0, WIDTH, HEIGHT);
  ctx.fillStyle = C.accent;
  ctx.fillRect(0, 0, WIDTH, 6);
  ctx.textBaseline = "alphabetic";

  // Brand and date.
  leaf(ctx, pad, 36, 30);
  ctx.fillStyle = C.text;
  ctx.font = `600 22px ${DISPLAY}`;
  ctx.fillText("Leafline", pad + 40, 60);
  ctx.textAlign = "right";
  ctx.fillStyle = C.muted;
  ctx.font = `18px ${SANS}`;
  ctx.fillText(longDay(s.startedAt ?? s.savedAt), WIDTH - pad, 60);
  ctx.textAlign = "left";

  // What and who.
  ctx.fillStyle = C.text;
  ctx.font = `600 46px ${DISPLAY}`;
  ctx.fillText(fit(ctx, title(s), WIDTH - pad * 2), pad, 136);
  ctx.fillStyle = C.muted;
  ctx.font = `22px ${SANS}`;
  const who = [o.name ? s.character : null, levels(s), s.job, `${formatDuration(s.activeSeconds)} played`].filter(Boolean).join("  ·  ");
  ctx.fillText(fit(ctx, who, WIDTH - pad * 2), pad, 174);

  // The numbers.
  const top = 236;
  const col = (WIDTH - pad * 2) / 4;
  stat(ctx, pad, top, percentOnly(s) ? "% / hour" : "EXP / hour", expRate(s), C.exp);
  stat(ctx, pad + col, top, "Mesos / hour", formatCompact(s.mesosPerHour), C.accent);
  stat(ctx, pad + col * 2, top, "Levels", s.levelUps ? `+${s.levelUps}` : "0", C.text);
  stat(ctx, pad + col * 3, top, o.drops ? "Drops logged" : "EXP gained", o.drops ? String(drops.length) : formatCompact(s.expGained), C.text);

  // The graph, then what was hunted where.
  const footer = HEIGHT - 40;
  const lines: string[] = [];
  if (o.maps && s.maps.length) lines.push(`Maps  ${mapWords(s, 3).join("   ")}`);
  if (o.drops && drops.length) lines.push(`Drops  ${dropWords(drops, 5).join("   ")}`);
  const chartBottom = footer - 30 - lines.length * 32;
  const drew = t ? chart(ctx, s, t, { x: pad, y: 368, w: WIDTH - pad * 2, h: Math.max(60, chartBottom - 368) }) : false;
  if (!drew) {
    ctx.fillStyle = C.faint;
    ctx.font = `17px ${SANS}`;
    ctx.fillText("No graph for this session.", pad, 400);
  }
  ctx.font = `19px ${SANS}`;
  lines.forEach((line, i) => {
    const y = footer - 18 - (lines.length - 1 - i) * 32;
    const [head, ...rest] = line.split("  ");
    ctx.fillStyle = C.faint;
    ctx.fillText(head, pad, y);
    ctx.fillStyle = C.text;
    ctx.fillText(fit(ctx, rest.join("  ").trim(), WIDTH - pad * 2 - 80), pad + 80, y);
  });

  ctx.fillStyle = C.line;
  ctx.fillRect(0, footer, WIDTH, 1);
  ctx.fillStyle = C.faint;
  ctx.font = `15px ${SANS}`;
  ctx.fillText("Tracked with Leafline, an unofficial fan-made overlay. Not affiliated with Nexon.", pad, footer + 26);
}

/** The canvas as PNG bytes. */
export async function pngBytes(canvas: HTMLCanvasElement): Promise<Uint8Array> {
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (!blob) throw new Error("The picture couldn't be made.");
  return new Uint8Array(await blob.arrayBuffer());
}

/** Puts the picture on the clipboard, ready to paste into Discord or Reddit. */
export async function copyPicture(canvas: HTMLCanvasElement) {
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (!blob) throw new Error("The picture couldn't be made.");
  await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
}
