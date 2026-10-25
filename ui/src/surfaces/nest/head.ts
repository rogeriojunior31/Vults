// The Nest window's frame (ADR 0021), as the Living Flock prototype draws it: the header (the
// mark, the flock line, sound, search, the size switch, full screen), the tabs with their icons and
// counts (Settings at the right), and the footer (Zeca, his look, the terminal, back to the island).
// The Nest's page and Settings' draw the same one: they are two webviews of one window, and a tab
// moves between them. It holds counts and words of its own, never an agent's text, so Settings may
// show it.
import type { MediaAction, NowPlaying, Size } from "../../bridge";
import { speciesSet } from "../../character/flock";
import { drawFrame, frameAt, type SpriteSet } from "../../character/sprites";
import { el } from "../../dom";
import { t } from "../../i18n";
import { resizeEdges } from "./edges";
import { icon as islandIcon, type IconName } from "../../island/icons";

/** The prototype draws the frame's icons with a lighter stroke than the island's. */
const icon = (name: IconName, size: number) => islandIcon(name, size, 1.6);
import { sizeSwitch } from "../../sizes";

export type NestTab = "flock" | "chat" | "activity" | "connections" | "dictation";
export const NEST_TABS: NestTab[] = [
  "flock",
  "chat",
  "dictation",
  "connections",
  "activity",
];

const TAB_ICONS: Record<NestTab | "settings", IconName> = {
  flock: "flock",
  chat: "chat",
  activity: "list",
  connections: "plug",
  dictation: "mic",
  settings: "gear",
};

export interface NestHeadActions {
  tab: (id: NestTab) => void;
  /** The Settings tab. */
  gear: () => void;
  size: (size: Size) => void;
  /** The search button: the palette. None where there is no palette (Settings). */
  search: (() => void) | null;
  /** The sound button: on or off. */
  sound: (on: boolean) => void;
  /** The song's controls; none where there is no song (Settings). */
  media?: (action: MediaAction) => void;
  /** Ends do not disturb (the moon). */
  endDnd?: () => void;
}

/** What the header counts: never a session's name or step. */
export interface NestCounts {
  sessions: number;
  /** Sessions that need you now. */
  needs: number;
  /** The connectors' rows. */
  connections: number;
}

export interface NestHead {
  /** One header with the tab navigation. */
  elements: HTMLElement[];
  /** The tab shown, or Settings. */
  set: (on: NestTab | "settings") => void;
  /** Without Zeca there is no Chat tab. */
  setZeca: (on: boolean) => void;
  setFull: (full: boolean) => void;
  setCounts: (c: NestCounts) => void;
  setSound: (on: boolean) => void;
  /** The song your player is playing (Now playing on), or none. */
  setMedia: (now: NowPlaying | null) => void;
  /** Do not disturb is on: the moon shows, a click ends it. */
  setDnd: (on: boolean) => void;
  /** Draws its words again: the language changed. */
  relabel: () => void;
}

/** The header's flock line, each case a whole sentence. */
function flockLine({ sessions, needs }: NestCounts): string {
  if (sessions === 1) return needs > 0 ? t("1 session · 1 needs you") : t("1 session");
  if (needs === 0) return t("{sessions} sessions", { sessions });
  return needs === 1 ? t("{sessions} sessions · 1 needs you", { sessions }) : t("{sessions} sessions · {needs} need you", { sessions, needs });
}

export function createNestHead(on: NestHeadActions): NestHead {
  // Both pages of the window wear this header, so its edges come with it, once per page.
  if (!document.querySelector(".resize-edges")) document.body.append(resizeEdges());
  let full = false;
  let sound = true;
  let counts: NestCounts = { sessions: 0, needs: 0, connections: 0 };
  const mark = el("img", { class: "app-mark" });
  mark.src = "/icon.png";
  mark.alt = "";
  const line = el("span", { class: "session-count" });
  const soundButton = el("button", {
    class: "header-icon",
    onclick: () => on.sound(!sound),
  });
  const searchButton = on.search
    ? el(
        "button",
        { class: "header-icon", onclick: on.search },
        icon("search", 14),
      )
    : null;
  const sizes = el("span", { class: "nest-sizes" });
  // The prototype's extras by the switch: the song with its controls, and the moon while do not
  // disturb lasts.
  const extras = el("span", { class: "hdr-extras" });
  let media: NowPlaying | null = null;
  let dnd = false;
  const drawExtras = () => {
    // Drawn anew on each song event: the control that had the focus gets it back.
    const had = extras.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.action : null;
    const control = (action: MediaAction, glyph: IconName, label: string) => {
      const b = el("button", { class: "header-icon np-control", onclick: () => on.media?.(action) }, icon(glyph, 12));
      b.dataset.action = action;
      b.title = label;
      b.setAttribute("aria-label", label);
      return b;
    };
    const song = media && on.media ? (media.artist ? `${media.title} · ${media.artist}` : media.title) : null;
    const np = song
      ? el(
          "div",
          { class: `now-playing${media!.playing ? " playing" : ""}` },
          icon("note", 12),
          el("span", { class: "song", text: song }),
          el(
            "span",
            { class: "np-controls" },
            control("previous", "previous", t("Previous")),
            control("playpause", media!.playing ? "pause" : "play", media!.playing ? t("Pause") : t("Play")),
            control("next", "next", t("Next")),
          ),
        )
      : null;
    if (np) {
      np.title = song!;
      np.setAttribute("role", "group");
      np.setAttribute("aria-label", song!);
    }
    const moon = dnd && on.endDnd ? el("button", { class: "header-icon moon", onclick: () => on.endDnd?.() }, icon("moon", 14)) : null;
    if (moon) {
      moon.title = t("Do not disturb is on: no sounds or notifications. Click to end it.");
      moon.setAttribute("aria-label", t("Do not disturb is on: no sounds or notifications. Click to end it."));
    }
    extras.replaceChildren(...([np, moon] as (HTMLElement | null)[]).filter((e): e is HTMLElement => e !== null));
    if (had) extras.querySelector<HTMLElement>(`[data-action="${had}"]`)?.focus();
  };
  const fullButton = el(
    "button",
    {
      class: "header-icon nest-fullscreen",
      onclick: () => on.size(full ? "room" : "full"),
    },
    icon("fullscreen", 14),
  );
  const tab = (id: NestTab | "settings", click: () => void) => {
    const b = el("button", { class: "suite-tab", onclick: click });
    b.dataset.tab = id;
    b.id = `nest-tab-${id}`;
    b.setAttribute("role", "tab");
    if (id !== "settings") b.setAttribute("aria-controls", `nest-pane-${id}`);
    return b;
  };
  const tabs = NEST_TABS.map((id) => tab(id, () => on.tab(id)));
  const gear = tab("settings", on.gear);
  gear.classList.add("nest-gear");
  const nav = el("nav", { class: "suite-nav" }, ...tabs, gear);
  nav.setAttribute("role", "tablist");
  const roving = () => [...tabs, gear].filter((b) => !b.hidden);
  for (const b of [...tabs, gear]) {
    b.addEventListener("keydown", (e) => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) return;
      const available = roving();
      const now = available.indexOf(b);
      if (now < 0) return;
      const next =
        e.key === "Home"
          ? available[0]
          : e.key === "End"
            ? available.at(-1)!
            : available[(now + (e.key === "ArrowRight" ? 1 : -1) + available.length) % available.length];
      e.preventDefault();
      next.focus();
      next.click();
    });
  }

  const drawTab = (
    b: HTMLElement,
    id: NestTab | "settings",
    word: string,
    n: number,
  ) => {
    // The title and label keep the word where a narrow window shows the icon alone.
    b.title = word;
    b.setAttribute("aria-label", n > 0 ? `${word} ${n}` : word);
    b.replaceChildren(
      icon(TAB_ICONS[id], 15),
      el("span", { class: "nav-label", text: word }),
      ...(n > 0 ? [el("span", { class: "nav-count", text: String(n) })] : []),
    );
  };
  /** The counts: the tabs' numbers and the flock line. Nothing with focus is rebuilt. */
  const recount = () => {
    const words: Record<NestTab, string> = {
      flock: t("Flock"),
      chat: t("Chat"),
      activity: t("Activity"),
      connections: t("Connections"),
      dictation: t("Dictation"),
    };
    const n: Partial<Record<NestTab, number>> = { flock: counts.needs, connections: counts.connections };
    for (const b of tabs) {
      const id = b.dataset.tab as NestTab;
      drawTab(b, id, words[id], n[id] ?? 0);
    }
    drawTab(gear, "settings", t("Settings"), 0);
    line.textContent = flockLine(counts);
    line.hidden = counts.sessions === 0;
  };
  /** The toggles' state; their names stay, `aria-pressed` says the state. */
  const restate = () => {
    soundButton.replaceChildren(icon(sound ? "sound" : "mute", 14));
    soundButton.setAttribute("aria-pressed", String(!sound));
    fullButton.setAttribute("aria-pressed", String(full));
  };
  /** The switch, rebuilt only when the language or Full changes. In Full, Room shows pressed, as
   * in the prototype, and still takes the click back to Room. */
  const drawSizes = () => {
    sizes.replaceChildren(sizeSwitch(full ? "full" : "room", on.size, [], ["mini", "quick", "room"]));
    if (full) sizes.querySelector('[data-size="room"]')?.classList.add("on");
  };
  const relabel = () => {
    recount();
    soundButton.title = t("Mute");
    soundButton.setAttribute("aria-label", t("Mute"));
    if (searchButton) {
      searchButton.title = t("Command palette (Ctrl+K)");
      searchButton.setAttribute("aria-label", t("Command palette (Ctrl+K)"));
    }
    fullButton.title = t("Full: the Nest on the whole screen; Esc goes back");
    fullButton.setAttribute("aria-label", t("Full"));
    restate();
    drawSizes();
    nav.setAttribute("aria-label", t("The Nest's tabs"));
  };
  relabel();
  const header = el(
    "header",
    { class: "app-header" },
    mark,
    el("span", { class: "app-logo", text: "vults" }),
    nav,
    el("span", { class: "header-spacer" }),
    line,
    extras,
    soundButton,
    ...(searchButton ? [searchButton] : []),
    sizes,
    fullButton,
  );
  // No system title bar (ADR 0021): the header, where nothing else is, moves the window; a double
  // click fills the screen, as a title bar's would.
  for (const e of [header, header.querySelector(".header-spacer"), line, nav]) e?.setAttribute("data-tauri-drag-region", "");
  return {
    elements: [header],
    set: (id) => {
      for (const b of [...tabs, gear]) {
        const now = b.dataset.tab === id;
        b.classList.toggle("on", now);
        b.setAttribute("aria-selected", String(now));
        b.tabIndex = now ? 0 : -1;
      }
    },
    setZeca: (zeca) => {
      tabs.find((b) => b.dataset.tab === "chat")!.hidden = !zeca;
    },
    setFull: (f) => {
      if (f === full) return;
      full = f;
      restate();
      drawSizes();
    },
    setCounts: (c) => {
      if (
        c.sessions === counts.sessions &&
        c.needs === counts.needs &&
        c.connections === counts.connections
      )
        return;
      counts = c;
      recount();
    },
    setSound: (s) => {
      if (s === sound) return;
      sound = s;
      restate();
    },
    setMedia: (now) => {
      media = now;
      drawExtras();
    },
    setDnd: (onNow) => {
      if (onNow === dnd) return;
      dnd = onNow;
      drawExtras();
    },
    relabel: () => {
      relabel();
      drawExtras();
    },
  };
}

export interface NestFootActions {
  zeca: (on: boolean) => void;
  /** Zeca's look: Settings › Flock & Zeca. */
  look: () => void;
  /** The terminal of the session in front. */
  terminal: () => void;
  island: () => void;
}

export interface NestFoot {
  element: HTMLElement;
  setZeca: (on: boolean) => void;
  setSpecies: (id: string) => void;
  /** Whether a session is in front to jump to. */
  setTerminal: (can: boolean) => void;
  relabel: () => void;
}

/** Zeca's first idle frame, one canvas pixel per sprite pixel; CSS sets its height. */
function drawMark(canvas: HTMLCanvasElement, set: SpriteSet): void {
  const frame = frameAt(set.clips.idle, 0);
  let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
  for (const [part, x, y] of frame.layers) {
    const grid = set.parts[part];
    if (!grid) continue;
    x0 = Math.min(x0, x);
    y0 = Math.min(y0, y);
    x1 = Math.max(x1, x + Math.max(...grid.map((r) => r.length)));
    y1 = Math.max(y1, y + grid.length);
  }
  if (!Number.isFinite(x0)) return;
  canvas.width = x1 - x0;
  canvas.height = y1 - y0;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  drawFrame(ctx, set, frame, -x0, -y0, 1);
}

export function createNestFoot(on: NestFootActions): NestFoot {
  let zeca = true;
  let species = "atratus";
  const canvas = el("canvas", { class: "zeca-mark" });
  canvas.setAttribute("aria-hidden", "true");
  const zecaButton = el("button", {
    class: "nest-zeca",
    onclick: () => on.zeca(!zeca),
  });
  const lookButton = el("button", { class: "zeca-look", onclick: on.look });
  const terminal = el("button", {
    class: "open-terminal",
    onclick: on.terminal,
  });
  // Until a view says who is in front, there is no terminal to open.
  terminal.disabled = true;
  const island = el("button", { class: "foot-island", onclick: on.island });
  const relabel = () => {
    zecaButton.textContent = zeca ? t("Zeca on · optional") : t("Zeca off");
    zecaButton.setAttribute("aria-pressed", String(zeca));
    lookButton.replaceChildren(icon("pencil", 14));
    lookButton.title = t("Zeca's look");
    lookButton.setAttribute("aria-label", t("Zeca's look"));
    lookButton.hidden = !zeca;
    terminal.replaceChildren(
      icon("terminal", 13),
      el("span", { text: t("Open terminal") }),
    );
    island.replaceChildren(
      el("span", { text: t("Back to the island") }),
      icon("fold", 12),
    );
  };
  const paint = () => {
    try {
      drawMark(canvas, speciesSet(species));
    } catch {
      // An unknown species, or no canvas: the words stay.
    }
  };
  relabel();
  paint();
  return {
    element: el(
      "footer",
      { class: "app-footer" },
      el("span", { class: "companion" }, canvas, zecaButton, lookButton),
      el("span", { class: "footer-actions" }, terminal, island),
    ),
    setZeca: (z) => {
      zeca = z;
      canvas.hidden = !z;
      relabel();
    },
    setSpecies: (id) => {
      if (id === species) return;
      species = id;
      paint();
    },
    setTerminal: (can) => {
      terminal.disabled = !can;
    },
    relabel,
  };
}

/** Alt+1 to 5: the tab at that place, whatever the layout (a hidden Chat is skipped). Not while
 * typing or composing. */
export function tabByKey(e: KeyboardEvent, zeca: boolean): NestTab | undefined {
  const typing = (e.target as Element | null)?.closest?.(
    "input, textarea, select, [contenteditable]",
  );
  if (
    e.isComposing ||
    e.defaultPrevented ||
    typing ||
    !e.altKey ||
    e.ctrlKey ||
    e.metaKey
  )
    return undefined;
  const n = /^Digit([1-5])$/.exec(e.code);
  return n
    ? NEST_TABS.filter((id) => id !== "chat" || zeca)[Number(n[1]) - 1]
    : undefined;
}
