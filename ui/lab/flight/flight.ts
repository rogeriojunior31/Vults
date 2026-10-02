import { Bird, type Shot } from "../../src/character/director";
import { drawFrame, frameAt, type Frame } from "../../src/character/sprites";
import { ZECA, PERCH_HEIGHT } from "../../src/character/zeca";

const perch = { x: 30, wireY: 116, height: PERCH_HEIGHT, skyRight: 230, skyTop: 10 };
const thermal = { cx: 128, cy: 46, rx: 70, ry: 20, lapMs: 6200, phase: Math.PI };
const home = { x: perch.x + 10, y: perch.wireY - PERCH_HEIGHT + 6 };
const duration = 15000;
const launch = 180;
const join = 1800;
const recall = 11000;
const arrival = 13000;
const canvases = ["current", "proposed"].map(id => document.getElementById(id) as HTMLCanvasElement);
const play = document.getElementById("play") as HTMLButtonElement;
const seek = document.getElementById("seek") as HTMLInputElement;
const speed = document.getElementById("speed") as HTMLSelectElement;
const guide = document.getElementById("path") as HTMLInputElement;
const output = document.getElementById("time") as HTMLOutputElement;
const motion = matchMedia("(prefers-reduced-motion: reduce)");
let paused = motion.matches;
let elapsed = 0;
let last = performance.now();
play.textContent = paused ? "Play" : "Pause";
motion.addEventListener("change", () => {
  if (motion.matches) { paused = true; play.textContent = "Play"; }
});

type Point = { x: number; y: number };
type Pose = Shot & { bank?: number; width?: number };
const frames: Record<string, Frame> = Object.fromEntries(["fly_up", "glide", "fly_down"].map(name =>
  [name, { ms: 0, dx: 0, dy: 0, layers: [[name, 0, 0]] } satisfies Frame]));

function orbit(t: number) {
  const angle = thermal.phase + (t - join) / thermal.lapMs * Math.PI * 2;
  const omega = Math.PI * 2 / thermal.lapMs;
  return {
    x: thermal.cx + Math.cos(angle) * thermal.rx,
    y: thermal.cy + Math.sin(angle) * thermal.ry,
    vx: -Math.sin(angle) * thermal.rx * omega,
    vy: Math.cos(angle) * thermal.ry * omega,
    angle,
  };
}

// Endpoint velocities are in cells per millisecond, so both joins preserve momentum.
function curve(a: Point, b: Point, va: Point, vb: Point, t: number, ms: number): Point {
  const u = t / ms, u2 = u * u, u3 = u2 * u;
  const axis = (key: "x" | "y") => (2 * u3 - 3 * u2 + 1) * a[key]
    + (u3 - 2 * u2 + u) * ms * va[key]
    + (-2 * u3 + 3 * u2) * b[key] + (u3 - u2) * ms * vb[key];
  return { x: axis("x"), y: axis("y") };
}

function position(t: number): Point {
  const zero = { x: 0, y: 0 };
  if (t < join) {
    const end = orbit(join);
    return curve(home, end, zero, { x: end.vx, y: end.vy }, Math.max(0, t - launch), join - launch);
  }
  if (t < recall) return orbit(t);
  const start = orbit(recall);
  return curve(start, home, { x: start.vx, y: start.vy }, zero, Math.min(t - recall, arrival - recall), arrival - recall);
}

function proposal(t: number): Pose {
  if (t < launch || t >= arrival) {
    const settling = t >= arrival && t < arrival + 160;
    return { frame: frameAt(ZECA.clips.idle, t), x: perch.x, y: perch.wireY - PERCH_HEIGHT + (settling || t < launch ? 1 : 0), flip: false };
  }
  const p = position(t);
  const before = position(Math.max(launch, t - 8)), after = position(Math.min(arrival, t + 8));
  const vx = (after.x - before.x) / 16;
  const beat = (t - launch) % 3600;
  const flapping = t < join || (t < recall && beat < 780);
  const wing = ["fly_up", "glide", "fly_down", "glide"][Math.floor(beat / 65) % 4];
  return {
    frame: frames[t > arrival - 300 ? "fly_up" : flapping ? wing : "glide"],
    x: p.x - 18.5, y: p.y - 8, flip: vx < 0,
    bank: Math.max(-0.10, Math.min(0.10, vx * 1.3)),
    width: 0.82 + 0.18 * Math.min(1, Math.abs(vx) / 0.045),
  };
}

// Replaying the public API makes scrubbing deterministic, including the landing transition.
function current(t: number): Shot {
  const bird = new Bird(ZECA, perch);
  bird.soar(thermal, 0);
  if (t >= recall) bird.want("idle", recall);
  const cadence = 1000 / 30;
  return bird.shot(Math.floor(t / cadence) * cadence);
}

function render(canvas: HTMLCanvasElement, pose: Pose): void {
  const ctx = canvas.getContext("2d")!;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.imageSmoothingEnabled = false;
  ctx.strokeStyle = "#657183";
  ctx.lineWidth = 1;
  ctx.beginPath(); ctx.moveTo(30, perch.wireY * 3); ctx.lineTo(690, perch.wireY * 3); ctx.stroke();
  if (guide.checked) {
    ctx.strokeStyle = "#465164"; ctx.setLineDash([4, 8]);
    ctx.beginPath(); ctx.ellipse(thermal.cx * 3, thermal.cy * 3, thermal.rx * 3, thermal.ry * 3, 0, 0, Math.PI * 2); ctx.stroke();
    ctx.setLineDash([]);
  }
  ctx.save();
  ctx.globalAlpha = pose.alpha ?? 1;
  ctx.translate((pose.x + 18.5) * 3, (pose.y + 8) * 3);
  ctx.rotate(pose.bank ?? 0); ctx.scale(pose.width ?? 1, 1);
  drawFrame(ctx, ZECA, pose.frame, -18.5, -8, 3, pose.flip);
  ctx.restore();
}

function draw(): void {
  render(canvases[0], current(elapsed));
  render(canvases[1], proposal(Math.floor(elapsed / (1000 / 30)) * (1000 / 30)));
  seek.value = String(elapsed);
  output.value = `${(elapsed / 1000).toFixed(2)} s`;
}
play.onclick = () => { paused = !paused; play.textContent = paused ? "Play" : "Pause"; };
document.getElementById("restart")!.onclick = () => { elapsed = 0; draw(); };
seek.oninput = () => { paused = true; play.textContent = "Play"; elapsed = Number(seek.value); draw(); };
guide.onchange = draw;
draw();
// Browser preview only; production uses a timer because layer-shell can suspend rAF.
function tick(now: number): void {
  if (!paused && !document.hidden) {
    elapsed = (elapsed + Math.min(now - last, 100) * Number(speed.value)) % duration;
    draw();
  }
  last = now;
  requestAnimationFrame(tick);
}
requestAnimationFrame(tick);
