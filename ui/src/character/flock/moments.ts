// Four clips composed from a species' own finished frames (its head, its spread-wing pose, its
// sleep), so every size, neck and look comes along: dozing off, waking up, courtship and a full
// crop. They are ready in the sprite; when the island plays them is its own business.
import type { Clip, Frame, Grid, Layer } from "../sprites";
import type { Rig } from "./rig";

const clone = (fr: Frame, ms: number, patch: Partial<Frame> = {}): Frame => ({
  ...fr,
  ...patch,
  ms,
  layers: (patch.layers ?? fr.layers).map((l) => [...l] as Layer),
});
const isHead = (p: string) => p.startsWith("head") || p === "sleep_head";
/** The frame with its head part swapped (or kept, with null) and moved. */
function head(
  fr: Frame,
  ms: number,
  part: string | null,
  dx = 0,
  dy = 0,
): Frame {
  const out = clone(fr, ms);
  out.layers = out.layers.map(([p, x, y]) =>
    isHead(p) ? [part ?? p, x + dx, y + dy] : [p, x, y],
  );
  return out;
}
const puff = (fr: Frame, ms: number, dx = 0): Frame => {
  const out = clone(fr, ms, { dx });
  out.layers = out.layers.map(([p, x, y]) => [
    p === "body" ? "body_puff" : p,
    x,
    y,
  ]);
  return out;
};
const headName = (fr: Frame) => fr.layers.find(([p]) => isHead(p))?.[0];

// ---- the crop: a bulge on the breast below the band, three sizes, in the bird's own plumage ----
function cropGrid(f: number, s: number): Grid {
  const rx = Math.max(1, Math.round(s * f)),
    ry = Math.max(2, Math.round((s + 1.5) * f));
  const rows: string[] = [];
  for (let y = -ry; y <= ry; y++) {
    const w = Math.round(
      (rx + 0.4) * Math.sqrt(Math.max(0, 1 - (y / (ry + 0.6)) ** 2)),
    );
    // Two cells inside the body's edge, to cover its outline there.
    const n = w + 2,
      up = y < 0;
    let r = "";
    for (let x = 0; x < n; x++) {
      const rim = x === n - 1 || ((y === -ry || y === ry) && x >= 2);
      // Lit from above like the body: the rim's upper half catches the light, the lower half is in shade.
      if (rim) r += up ? "i" : y > ry / 2 ? "K" : "B";
      else if (x === n - 2 && up) r += "s";
      else r += "b";
    }
    rows.push(r);
  }
  return rows;
}
/** Where the crop goes on a perched frame: under the band, its left two cells inside the edge. */
function cropAt(set: Rig, fr: Frame, g: Grid): [number, number] | null {
  const body = fr.layers.find(([p]) => p.startsWith("body"));
  const band = fr.layers.find(([p]) => p === "band");
  if (!body || !band) return null;
  const bg = set.parts[body[0]];
  const cy = band[2] + 2 + Math.round(g.length / 2) - 1;
  const row = bg[cy - body[2]] ?? "";
  const edge = body[1] + row.replace(/\.+$/, "").length - 1;
  return [edge - 1, cy - Math.floor(g.length / 2)];
}
/** The frame with the crop at size `s` (1 to 3), on a perched body only. */
function withCrop(set: Rig, fr: Frame, s: number): Frame {
  const name = `crop_${s}`;
  set.parts[name] ??= cropGrid(set.perch / 20, s);
  const at = cropAt(set, fr, set.parts[name]);
  if (!at) return fr;
  const out = clone(fr, fr.ms);
  const i = out.layers.findIndex(([p]) => p === "band");
  out.layers.splice(i, 0, [name, at[0], at[1]]);
  return out;
}

// ---- the clips ----
function anchors(set: Rig) {
  const idle = set.clips.idle.frames;
  const rest = idle[0];
  const back = idle.find((f) => headName(f) === "head_back") ?? rest;
  const sleep = set.clips.sleep.frames[0];
  const stretch = set.clips.done.frames.filter((f) =>
    f.layers.some(([p]) => p === "stretch_wing"),
  );
  return { rest, back, sleep, stretch };
}

/** Dozing off: the head grows heavy twice, a jerk awake between, then it turns onto the back and
 *  the bird settles into its sleep. The feet stay on the wire through the jerk. */
function doze(set: Rig): Clip {
  const { rest, back, sleep } = anchors(set);
  const k = Math.max(1, Math.round(set.perch / 20));
  return {
    loop: false,
    frames: [
      clone(rest, 1200),
      head(rest, 140, "head:blink"),
      clone(rest, 600),
      head(rest, 500, "head:blink", 0, k),
      head(rest, 800, "head:blink", 0, 2 * k),
      head(rest, 90, "head", 1, -2),
      head(rest, 160, "head", 0, -1),
      clone(rest, 300),
      head(rest, 420, "head:blink"),
      clone(rest, 260),
      head(rest, 600, "head:blink", 0, k),
      head(rest, 1000, "head:blink", 0, 2 * k),
      head(back, 300, "head_back:blink"),
      puff(head(back, 0, "head_back:blink"), 240),
      clone(sleep, 900),
    ],
  };
}

/** Waking up: an eye opens with the head still on the back, the head comes round, the done clip's
 *  stretch of wing and leg, a shake of the feathers. */
function wake(set: Rig): Clip {
  const { rest, back, sleep, stretch } = anchors(set);
  return {
    loop: false,
    frames: [
      head(sleep, 300, "head_back"),
      head(sleep, 120, "sleep_head"),
      head(sleep, 200, "head_back"),
      puff(back, 160),
      clone(back, 120),
      clone(rest, 120),
      ...stretch.map((f, i) => clone(f, [160, 400, 100, 120][i] ?? f.ms)),
      clone(rest, 80),
      puff(rest, 60, -1),
      puff(rest, 60, 1),
      puff(rest, 60, -1),
      puff(rest, 60, 1),
      puff(rest, 60, 0),
      head(rest, 100, "head:blink"),
      clone(rest, 300),
    ],
  };
}

// The condors court with the wings fully open, head high and slow; the others hop on the beat.
const SLOW = new Set(["vultur", "gymnogyps"]);
const BEAT = 268;
/** Courtship: wings half open, head low, a hop every other beat and a look to each side (as the
 *  Cathartes circle a mate); both condors open the wings fully and turn slowly, with a bow. The
 *  band is drawn last, so the agent's color never hides behind the lowered head. */
function court(set: Rig, id: string): Clip {
  const approval = set.clips.approval.frames;
  const front =
    approval.find(
      (f) =>
        headName(f) === "head_front" && f.layers.some(([p]) => p === "sunning"),
    ) ?? approval[0];
  const right = approval.find((f) => headName(f) === "head") ?? front;
  const left = approval.find((f) => headName(f) === "head_back") ?? front;
  // The low pose: zeca.py's sun(low=True) drops the wings, band and head a row; the feet stay.
  const lowOf = (fr: Frame) => {
    const out = clone(fr, fr.ms);
    out.layers = out.layers.map(([p, x, y]) =>
      p.startsWith("legs")
        ? [p, x, y]
        : [p === "sunning" ? "sunning_low" : p, x, y + 1],
    );
    return out;
  };
  const hop = (fr: Frame, ms: number, up: number) =>
    clone(fr, ms, { dy: fr.dy - up });
  let frames: Frame[];
  if (SLOW.has(id)) {
    const s = 2 * BEAT;
    frames = [
      clone(front, s),
      head(front, s, null, 0, -1),
      head(right, s, null, 0, -1),
      clone(right, s),
      head(front, s, null, 0, -1),
      head(left, s, null, 0, -1),
      clone(left, s),
      head(lowOf(front), s, null, 0, 2),
      head(lowOf(front), s, null, 0, 1),
    ];
  } else {
    const L = lowOf(front),
      R = lowOf(right),
      Lf = lowOf(left);
    frames = [
      head(L, BEAT, null, 0, 2),
      hop(head(L, 0, null, 0, 1), BEAT, 2),
      head(R, BEAT, null, 0, 2),
      hop(head(R, 0, null, 0, 2), BEAT, 2),
      head(L, BEAT, null, 0, 3),
      hop(L, BEAT, 1),
      head(Lf, BEAT, null, 0, 2),
      hop(head(Lf, 0, null, 0, 2), BEAT, 2),
    ];
  }
  return {
    loop: true,
    frames: frames.map((f) => ({
      ...f,
      layers: [
        ...f.layers.filter(([p]) => p !== "band"),
        ...f.layers.filter(([p]) => p === "band"),
      ],
    })),
  };
}

/** A full crop after eating: the swallow, then the crop full, half, nearly empty, gone. The idle
 *  loop plays at each size (sped up here; the island decides how long each one lasts). */
function fed(set: Rig): Clip {
  const sw = set.clips.swallow.frames;
  const idle = set.clips.idle.frames.filter((f) =>
    f.layers.some(([p]) => p.startsWith("body")),
  );
  const loopWith = (s: number, k: number) =>
    idle.map((f) => withCrop(set, clone(f, Math.round(f.ms * k)), s));
  return {
    loop: true,
    frames: [
      clone(set.clips.idle.frames[0], 900),
      ...sw.slice(0, -1).map((f) => clone(f, f.ms)),
      withCrop(set, clone(sw[sw.length - 1], 500), 3),
      ...loopWith(3, 0.5),
      ...loopWith(2, 0.4),
      ...loopWith(1, 0.4),
      clone(set.clips.idle.frames[0], 900),
    ],
  };
}

/** Adds `doze`, `wake`, `court` and `fed` to a finished species set, and the crop's parts. */
export function moments(set: Rig, id: string): void {
  set.clips.doze = doze(set);
  set.clips.wake = wake(set);
  set.clips.court = court(set, id);
  set.clips.fed = fed(set);
}
