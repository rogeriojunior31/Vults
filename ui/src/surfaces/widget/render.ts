// The corner widget: the sessions that matter most (up to three birds) and how many work or need
// you. It only shows, and a click brings the island up (on the card when one waits): it never
// answers a card (ADR 0008). It draws from the same view as the island.
import type { SessionView, ViewModel } from "../../bridge";
import { idleClip } from "../../island/behavior";
import { assignSpecies, zecaShown } from "../../island/flock";
import { BIRD_W, Scene, type SceneLayout } from "../../island/scene";
import { t } from "../../i18n";

/** The widget's size in CSS pixels; the window (a layer surface) is exactly this. */
export const WIDGET_W = 212;
export const WIDGET_H = 44;
/** At most this many birds. */
export const BIRDS = 3;

const WIRE = 38;
const SCENE_W = 10 + BIRDS * (BIRD_W + 4);
export const WIDGET_SCENE: SceneLayout = {
  width: SCENE_W,
  height: WIDGET_H,
  wire: WIRE,
  // The session in front sits first, as on the island.
  zeca: { x: 8, wire: WIRE, scale: 1 },
  vults: { scale: 1, max: BIRDS - 1, at: (slot) => ({ x: 8 + (slot + 1) * (BIRD_W + 4), wire: WIRE }) },
  skyTop: 0,
  flights: false,
  reflow: true,
  // No room over their heads: the counts say it.
  emotes: false,
};

const RANK: Record<SessionView["attention"], number> = { "needs-you": 4, failed: 3, done: 2, info: 1, quiet: 0 };
const key = (s: { agent: string; id: string }) => `${s.agent}:${s.id}`;

/** The birds shown: the session in front (core's rule), then the others by how much they want
 *  the user, keeping the wire's order among equals. */
export function pick(v: ViewModel): { front: SessionView | null; birds: SessionView[] } {
  const front = (v.front && v.sessions.find((s) => key(s) === key(v.front!))) ?? v.sessions[0] ?? null;
  const rest = v.sessions.filter((s) => s !== front).sort((a, b) => RANK[b.attention] - RANK[a.attention]);
  const birds = front ? [front, ...rest].slice(0, BIRDS) : [];
  return { front, birds };
}

// A rate-limited session waits (the island says so in amber): it rests here.
const WORKING: SessionView["status"][] = ["thinking", "working"];

/** What the counts say, whole sentences for the catalog: one or two lines. */
export function counts(v: ViewModel, paused: boolean): { text: string; kind: "dim" | "work" | "need" }[] {
  if (paused) return [{ text: t("Paused"), kind: "dim" }];
  if (v.sessions.length === 0) return [{ text: t("Nothing running"), kind: "dim" }];
  const need = v.sessions.filter((s) => s.attention === "needs-you").length;
  const working = v.sessions.filter((s) => WORKING.includes(s.status)).length;
  const lines: { text: string; kind: "dim" | "work" | "need" }[] = [];
  if (need > 0) lines.push({ text: need === 1 ? t("1 needs you") : t("{n} need you", { n: need }), kind: "need" });
  if (working > 0) lines.push({ text: t("{n} working", { n: working }), kind: "work" });
  if (lines.length === 0) lines.push({ text: v.sessions.length === 1 ? t("1 resting") : t("{n} resting", { n: v.sessions.length }), kind: "dim" });
  return lines;
}

export interface WidgetActions {
  /** Brings the island up. */
  open(): void;
  /** Puts a session in front first. */
  focus(agent: SessionView["agent"], id: string): void;
}

export interface Widget {
  render(v: ViewModel): void;
  /** The presence preset is Paused: the widget says so and shows no flock. */
  setPaused(on: boolean): void;
  /** The view on screen, to redraw after a setting changes. */
  last(): ViewModel;
  /** Drawing on or off (a full-screen window covers it): the birds keep their places. */
  setActive(on: boolean): void;
}

export function createWidget(root: HTMLElement, actions: WidgetActions): Widget {
  const scene = new Scene(WIDGET_SCENE);
  const text = document.createElement("div");
  text.className = "counts";
  root.replaceChildren(scene.canvas, text);
  root.setAttribute("role", "button");
  root.title = t("Open the island");

  let raw: ViewModel = { sessions: [], approval: null, alerts: [] };
  let paused = false;
  let shown: SessionView[] = [];

  const render = (v: ViewModel) => {
    raw = v;
    const view = paused ? { ...v, sessions: [], approval: null } : v;
    const { front, birds } = pick(view);
    shown = birds;
    assignSpecies(birds);
    // Zeca stays on an empty wire, as on the island; with him off it stays empty (ADR 0010).
    const idle = !front && zecaShown() ? { clip: idleClip(), agent: "claude" as const, alone: true } : null;
    scene.update(birds, front, idle);
    text.replaceChildren(
      ...counts(view, paused).map((line) => {
        const row = document.createElement("span");
        row.className = `count ${line.kind}`;
        row.textContent = line.text;
        return row;
      }),
    );
    root.classList.toggle("needs-you", !paused && view.sessions.some((s) => s.attention === "needs-you"));
  };

  root.addEventListener("click", (e) => {
    // Another bird puts its session in front, unless a card waits: the island opens on the card.
    // A bird whose own card waits in line brings that card first (core). The first bird is
    // already in front; focusing it would pin it there against core's rule.
    if (!paused) {
      const box = scene.canvas.getBoundingClientRect();
      const x = e.clientX - box.left;
      const hit = scene.slots().find((s) => x >= s.x && x < s.x + s.width);
      const session = hit && hit.key !== "zeca" ? shown.find((s) => key(s) === hit.key) : undefined;
      if (session && (!raw.approval || (session.waiting && !session.card))) actions.focus(session.agent, session.id);
    }
    actions.open();
  });

  return {
    render,
    setPaused: (on) => {
      paused = on;
      render(raw);
    },
    last: () => raw,
    setActive: (on) => scene.setActive(on),
  };
}
