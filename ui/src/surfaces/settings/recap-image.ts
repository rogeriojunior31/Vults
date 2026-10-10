// The week as an image to share (docs/guide/activity.md): drawn here, in a canvas, from the
// week's counts; the app only writes the PNG where the user picks. Nothing is uploaded. With
// `hideProjects` no project name is drawn.
import type { WeekView } from "../../view.gen";
import { speciesSet } from "../../character/flock";
import { drawFrame, frameAt } from "../../character/sprites";
import { perchOf } from "../../character/zeca";
import { duration, weekLabel } from "./activity";

export const IMAGE_W = 1080;
export const IMAGE_H = 1350;

// The app's own tokens (themes.css), as a canvas needs them.
const BG = "#000000";
const PANEL = "#141518";
const LINE = "rgba(255, 255, 255, 0.06)";
const FG = "#f5f6f8";
const DIM = "#9398a1";
const MUTED = "#7a7f88";
const OK = "#22c55e";
const FONT = "system-ui, sans-serif";

const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/** The week's sentence, in the sharer's words, without the project when it is hidden. */
export function imageHeadline(w: WeekView, hideProjects: boolean): string {
  const time = duration(w.active_secs);
  const turns = `${w.turns} ${w.turns === 1 ? "turn" : "turns"}`;
  const project = !hideProjects && w.top_project ? `, most on ${w.top_project}` : "";
  return `${turns}, ${time} with my agents${project}.`;
}

function rounded(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number): void {
  ctx.beginPath();
  ctx.roundRect(x, y, w, h, r);
  ctx.fill();
}

/** Words wrapped to `width`, drawn from `y` down; the y after the last line. */
function wrap(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, width: number, line: number): number {
  let row = "";
  for (const word of text.split(" ")) {
    const next = row ? `${row} ${word}` : word;
    if (ctx.measureText(next).width > width && row) {
      ctx.fillText(row, x, y);
      y += line;
      row = word;
    } else row = next;
  }
  if (row) ctx.fillText(row, x, y);
  return y + line;
}

export function recapImage(w: WeekView, opts: { hideProjects: boolean }): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = IMAGE_W;
  canvas.height = IMAGE_H;
  const ctx = canvas.getContext("2d")!;
  ctx.fillStyle = BG;
  ctx.fillRect(0, 0, IMAGE_W, IMAGE_H);
  const pad = 72;
  ctx.fillStyle = PANEL;
  rounded(ctx, pad / 2, pad / 2, IMAGE_W - pad, IMAGE_H - pad, 48);

  // Zeca on his wire in the top corner, clear of any title or time: the black vulture, as drawn
  // everywhere in the app.
  const set = speciesSet("atratus");
  ctx.imageSmoothingEnabled = false;
  const scale = 6;
  const wireY = 210;
  ctx.fillStyle = "#3a3a40";
  ctx.fillRect(IMAGE_W - pad - 200, wireY, 180, 4);
  drawFrame(ctx, set, frameAt(set.clips.idle, 0), Math.round((IMAGE_W - pad - 180) / scale), Math.round(wireY / scale) - perchOf(set), scale);

  ctx.textBaseline = "alphabetic";
  ctx.fillStyle = DIM;
  ctx.font = `600 34px ${FONT}`;
  ctx.fillText("Vults", pad + 20, pad + 70);
  ctx.fillStyle = FG;
  ctx.font = `700 58px ${FONT}`;
  ctx.fillText("My week with the agents", pad + 20, pad + 150);
  ctx.fillStyle = DIM;
  ctx.font = `500 36px ${FONT}`;
  ctx.fillText(weekLabel(w.monday, w.sunday), pad + 20, pad + 205);

  // The time, big, then the sentence.
  ctx.fillStyle = OK;
  ctx.font = `800 132px ${FONT}`;
  ctx.fillText(duration(w.active_secs), pad + 20, 470);
  ctx.fillStyle = FG;
  ctx.font = `500 40px ${FONT}`;
  wrap(ctx, imageHeadline(w, opts.hideProjects), pad + 20, 545, IMAGE_W - 2 * pad - 40, 54);
  // Two lines kept for the sentence, so the rest never moves.
  let y = 545 + 2 * 54;

  // Six numbers, two rows of three.
  const stats: [string, string][] = [
    [String(w.turns), w.turns === 1 ? "turn" : "turns"],
    [`+${w.added}`, "lines added"],
    [`−${w.removed}`, "lines removed"],
    [String(w.commands), w.commands === 1 ? "command" : "commands"],
    [String(w.allowed + w.denied + w.answered), "answers"],
    [String(w.files), w.files === 1 ? "file" : "files"],
  ];
  y += 10;
  const cellW = (IMAGE_W - 2 * pad - 40 - 2 * 24) / 3;
  stats.forEach(([value, label], i) => {
    const x = pad + 20 + (i % 3) * (cellW + 24);
    const top = y + Math.floor(i / 3) * 140;
    ctx.fillStyle = "rgba(255, 255, 255, 0.04)";
    rounded(ctx, x, top, cellW, 120, 24);
    ctx.fillStyle = FG;
    ctx.font = `700 52px ${FONT}`;
    ctx.fillText(value, x + 28, top + 64);
    ctx.fillStyle = DIM;
    ctx.font = `500 28px ${FONT}`;
    ctx.fillText(label, x + 28, top + 100);
  });
  y += 2 * 140 + 10;

  // Time per day.
  const barsH = 150;
  const barW = (IMAGE_W - 2 * pad - 40 - 6 * 20) / 7;
  const top = Math.max(1, ...w.days);
  w.days.forEach((secs, d) => {
    const x = pad + 20 + d * (barW + 20);
    ctx.fillStyle = "rgba(255, 255, 255, 0.04)";
    rounded(ctx, x, y, barW, barsH, 16);
    const h = Math.round((secs / top) * barsH);
    if (h > 0) {
      ctx.fillStyle = OK;
      rounded(ctx, x, y + barsH - h, barW, h, 16);
    }
    ctx.fillStyle = MUTED;
    ctx.font = `500 26px ${FONT}`;
    ctx.textAlign = "center";
    ctx.fillText(WEEKDAYS[d], x + barW / 2, y + barsH + 40);
    ctx.textAlign = "left";
  });

  ctx.fillStyle = LINE;
  ctx.fillRect(pad + 20, IMAGE_H - pad - 90, IMAGE_W - 2 * pad - 40, 2);
  ctx.fillStyle = MUTED;
  ctx.font = `500 26px ${FONT}`;
  ctx.fillText("Counted on my computer by Vults · no prompt or code leaves it", pad + 20, IMAGE_H - pad - 40);
  return canvas;
}

/** The image's PNG bytes. */
export async function recapPng(w: WeekView, opts: { hideProjects: boolean }): Promise<Uint8Array> {
  const blob = await new Promise<Blob | null>((done) => recapImage(w, opts).toBlob(done, "image/png"));
  if (!blob) throw new Error("the image could not be drawn");
  return new Uint8Array(await blob.arrayBuffer());
}
