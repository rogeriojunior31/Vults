// The Nest's Flock tab (ADR 0021), as the Living Flock prototype draws it: the stage (the flock at
// 3x across the pane, a roost card under each bird), *Session details* with *Needs you* and a
// search, the card of the session in front at the left and its Changes, Activity, Context and Zeca
// at the right; in Full, the plans' usage under it. The card of the session in front answers what
// it waits on, here, as the island would (ADR 0021).
import type { Answer, Diff, Hush, ProjectPref, UsageWindow } from "../../bridge";
import type { AgentKind, SessionView, ViewModel } from "../../view.gen";
import { el } from "../../dom";
import { t } from "../../i18n";
import { assignSpecies } from "../../island/flock";
import { BIRD_W, NEST_HABITAT_SCENE, NEST_HABITAT_WIRE, Scene } from "../../island/scene";
import { tickerSteps } from "../../island/ticker";
import { agentName, badgeOf, lastDiff, needsYou, questionBody, statusText } from "../../island/views";

export interface FlockActions {
  /** Puts the session in front of the flock (core's focus); null gives the choice back. */
  focus: (s: SessionView | null) => void;
  /** Its terminal, where `raise` says it can be brought forward. */
  jump: (s: SessionView) => void;
  /** The Flock, on the card this session waits on. */
  review: (s: SessionView) => void;
  /** Allow or Deny on a permission card: only a click (or its key) calls these (ADR 0004). */
  decide: (request: string, decision: "allow" | "deny") => void;
  /** Allow, and every identical request in this project from now on. */
  decideAlways: (request: string) => void;
  /** The replies to a question card, one per question. */
  answer: (request: string, answers: Answer[]) => void;
  /** "Reply in the terminal": the card goes, the agent's terminal asks. */
  release: (request: string) => void;
  /** A finished step's diff; null once the session no longer keeps it. */
  diff: (s: SessionView, step: number) => Promise<Diff | null>;
  /** The plans' usage windows, for Full's strip. */
  usage: () => Promise<UsageWindow[]>;
  /** The answer to a quiet bird's flag: none of them reaches the agent. */
  hush: (s: SessionView, hush: Hush) => void;
  /** The session's folder, in the editor or the file manager. */
  openFolder: (s: SessionView) => void;
  /** Mute, pin or hide the session's project, or undo it (kept by folder). */
  projectPref: (s: SessionView, pref: ProjectPref, on: boolean) => void;
}

/** What the tab adds for itself: the Chat, with a question about a session (only with Zeca on). */
export interface FlockAsk {
  ask: (s: SessionView) => void;
}

type Detail = "diff" | "activity" | "context" | "zeca";

const key = (s: SessionView) => `${s.agent}:${s.id}`;
const waits = (s: SessionView) => s.status === "approval" || s.status === "question" || s.waiting === true;
/** What *Needs you* keeps: the island's own test. */
export { needsYou };

/** What the search reads: the project, its roost and branch, the agent and what it is doing now. */
const said = (s: SessionView) => [s.project, s.roost, s.branch, agentName(s), statusText(s)].filter(Boolean).join(" ").toLowerCase();

const AGENT_COLOR: Record<AgentKind, string> = {
  claude: "var(--agent-claude)",
  codex: "var(--agent-codex)",
  opencode: "var(--agent-opencode)",
  qwen: "var(--agent-qwen)",
  copilot: "var(--agent-copilot)",
  other: "var(--agent-other)",
};
const colorOf = (agent: AgentKind) => AGENT_COLOR[agent] ?? AGENT_COLOR.other;
/** The plans whose usage windows the app reads. */
const PLAN_NAME: Partial<Record<AgentKind, string>> = { claude: "Claude", codex: "Codex" };

/** What the session has cost at API prices, once it is a cent or more (the island's chip). */
function costChip(s: SessionView): HTMLElement | null {
  if (s.cost_usd == null || s.cost_usd < 0.01) return null;
  const span = el("span", { class: "card-verb cost", text: `≈$${s.cost_usd < 100 ? s.cost_usd.toFixed(2) : Math.round(s.cost_usd)}` });
  span.title = t("What this session's tokens would cost at API prices. A subscription is not charged this.");
  return span;
}

const calmNow = () => matchMedia("(prefers-reduced-motion: reduce)").matches || document.body.classList.contains("still");

/** The card's tone, as the prototype tints it: waiting on you, asking, done, failed. */
function tone(s: SessionView): string {
  if (s.status === "failed") return "tone-fail";
  if (s.status === "question") return "tone-ask";
  if (waits(s) || s.attention === "needs-you") return "tone-warn";
  if (s.status === "finished") return "tone-ok";
  return "";
}

/** Narrowest column a 3x bird and its card fit in. */
const COLUMN_MIN = 96;
const BIRD_PX = BIRD_W * NEST_HABITAT_SCENE.vults.scale;

/** Marks a control so the keyboard focus comes back to it when the tab is drawn again. */
function fid<E extends HTMLElement>(e: E, id: string): E {
  e.dataset.fid = id;
  return e;
}

/** A diff's lines with their numbers in the new file (the old one for a removed line). */
function diffRows(d: Diff): HTMLElement[] {
  const row = (kind: string, n: number | null, text: string) =>
    el("div", { class: `diff-row ${kind}` }, el("span", { class: "ln", text: n === null ? "" : String(n) }), el("span", { text }));
  return d.files.flatMap((f) => [
    ...(d.files.length > 1 ? [row("hunk", null, f.path)] : []),
    ...f.hunks.flatMap((h) => {
      let oldLine = h.old_start;
      let newLine = h.new_start;
      return [
        row("hunk", null, `@@ −${h.old_start ?? "?"} +${h.new_start ?? "?"} @@`),
        ...h.lines.filter((line) => !line.startsWith("\\")).map((line) => {
          const mark = line[0];
          const kind = mark === "+" ? "plus" : mark === "-" ? "minus" : "";
          const n = kind === "minus" ? oldLine : newLine;
          if (kind !== "plus" && oldLine !== null) oldLine++;
          if (kind !== "minus" && newLine !== null) newLine++;
          return row(kind, n, line);
        }),
      ];
    }),
  ]);
}

/** `14:21`: a step's time of day, on the user's clock. */
function clock(unix: number): string {
  const d = new Date(unix * 1000);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

export function createFlock(on: FlockActions & FlockAsk): {
  element: HTMLElement;
  update: (v: ViewModel) => void;
  setZeca: (shown: boolean) => void;
  setActive: (shown: boolean) => void;
  /** Full shows the plans' usage under the details. */
  setFull: (full: boolean) => void;
  /** The plans' usage changed: Full's strip reads it again. */
  usageChanged: () => void;
  /** The keys the desktop bound for Allow and Deny, for the card's buttons. */
  setKeys: (keys: Record<string, string>) => void;
  /** Allow or Deny's key: answers the permission card on screen, if one is. */
  shortcut: (allow: boolean) => boolean;
} {
  const scene = new Scene(NEST_HABITAT_SCENE);
  const badges = el("div", { class: "badges" });
  const sky = el("div", { class: "habitat-sky" }, scene.canvas, badges);
  const heading = el("strong");
  const roosts = el("div", { class: "roost-cards" });
  const selected = el("p", { class: "habitat-selected" });
  // The prototype's count by the heading: who is out on a web sortie, who sits on the wire.
  const summary = el("span", { class: "habitat-summary" });
  const habitat = el(
    "section",
    { class: "flock-stage" },
    el("header", { class: "habitat-header" }, el("div", { class: "habitat-heading" }, el("span", { class: "legend-dot" }), heading), summary),
    sky,
    roosts,
    selected,
  );
  const title = el("h2");
  const search = el("input", { class: "nest-search" });
  search.type = "search";
  const needs = el("button", { class: "attention-filter" });
  const toolbar = el("div", { class: "flock-toolbar" }, title, el("div", { class: "flock-tools" }, needs, el("div", { class: "app-search" }, search)));
  const focusBlock = el("section", { class: "focus-block" });
  const detailTabs = el("nav", { class: "detail-tabs" });
  detailTabs.setAttribute("role", "tablist");
  const detailBody = el("div", { class: "detail-content" });
  detailBody.id = "nest-detail-panel";
  detailBody.setAttribute("role", "tabpanel");
  const details = el("section", { class: "details-pane" }, detailTabs, detailBody);
  const body = el("div", { class: "app-body" }, focusBlock, details);
  const usage = el("div", { class: "usage-card" });
  usage.hidden = true;
  const element = el("div", { class: "nest-flock-tab" }, habitat, toolbar, body, usage);

  // A right click on a bird or its card: the island's quick actions that work from here (ADR
  // 0011). None answers a card: the card itself does.
  const menu = el("div", { class: "nest-menu" });
  menu.setAttribute("role", "menu");
  menu.hidden = true;
  element.append(menu);
  /** Closes the menu; `back` returns the focus to the bird or card it came from (drawn anew since,
   *  so found again by its id). */
  const closeMenu = (back = true) => {
    if (menu.hidden) return;
    const wasIn = menu.contains(document.activeElement);
    menu.hidden = true;
    menu.replaceChildren();
    if (back && wasIn && opener) element.querySelector<HTMLElement>(`[data-fid="${CSS.escape(opener)}"]`)?.focus();
  };
  let opener: string | null = null;
  const openMenu = (s: SessionView, at: HTMLElement, x: number, y: number) => {
    opener = at.dataset.fid ?? null;
    const item = (text: string, go: () => void) => {
      const b = el("button", {
        class: "menu-item",
        text,
        onclick: () => {
          closeMenu();
          go();
        },
      });
      b.setAttribute("role", "menuitem");
      return b;
    };
    const edit = lastDiff(s);
    const inFront = !!view?.focus && view.focus.agent === s.agent && view.focus.id === s.id;
    // A card that waits keeps its session in front: only that one's Activity and diff can show.
    const waiting = view?.approval ?? null;
    const canFront = !waiting || (waiting.agent === s.agent && waiting.session === s.id);
    menu.replaceChildren(
      el("div", { class: "nest-menu-head", text: s.project || agentName(s) }),
      ...[
        s.raise === true ? item(t("Open terminal"), () => on.jump(s)) : null,
        s.cwd ? item(t("Open folder"), () => on.openFolder(s)) : null,
        s.steps.length && canFront
          ? item(t("Activity"), () => {
              on.focus(s);
              detail = "activity";
              draw();
            })
          : null,
        edit && canFront
          ? item(t("View the last diff"), () => {
              on.focus(s);
              showChange(s, edit.step);
            })
          : null,
        inFront ? item(t("Let the flock choose"), () => on.focus(null)) : item(t("Keep in front"), () => on.focus(s)),
        s.cwd ? (s.muted ? item(t("Unmute this project"), () => on.projectPref(s, "mute", false)) : item(t("Mute this project"), () => on.projectPref(s, "mute", true))) : null,
        s.cwd ? (s.pinned ? item(t("Unpin this project"), () => on.projectPref(s, "pin", false)) : item(t("Pin this project"), () => on.projectPref(s, "pin", true))) : null,
        s.cwd ? item(t("Hide this project"), () => on.projectPref(s, "hide", true)) : null,
      ].filter((b): b is HTMLButtonElement => b !== null),
    );
    menu.hidden = false;
    // Inside the tab, where the pointer was.
    const box = element.getBoundingClientRect();
    menu.style.left = `${Math.max(8, Math.min(x - box.left, box.width - menu.offsetWidth - 8))}px`;
    menu.style.top = `${Math.max(8, Math.min(y - box.top, box.height - menu.offsetHeight - 8))}px`;
    menu.querySelector<HTMLElement>(".menu-item")?.focus();
  };
  const menuOn = (b: HTMLElement, s: SessionView) => {
    b.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      openMenu(s, b, e.clientX, e.clientY);
    });
    // The keyboard's way to the same menu.
    b.addEventListener("keydown", (e) => {
      if (e.key !== "ContextMenu" && !(e.shiftKey && e.key === "F10")) return;
      e.preventDefault();
      const r = b.getBoundingClientRect();
      openMenu(s, b, r.left + 8, r.bottom);
    });
  };
  document.addEventListener("pointerdown", (e) => {
    if (!menu.hidden && !menu.contains(e.target as Node)) closeMenu();
  });
  // Tabbing out of it closes it, as a click elsewhere does.
  menu.addEventListener("focusout", (e) => {
    if (!menu.contains(e.relatedTarget as Node | null)) closeMenu(false);
  });
  menu.addEventListener("keydown", (e) => {
    const items = [...menu.querySelectorAll<HTMLElement>(".menu-item")];
    const at = items.indexOf(document.activeElement as HTMLElement);
    if (e.key === "Escape") closeMenu();
    else if (e.key === "ArrowDown") items[(at + 1) % items.length]?.focus();
    else if (e.key === "ArrowUp") items[(at - 1 + items.length) % items.length]?.focus();
    else return;
    e.preventDefault();
    e.stopPropagation();
  });
  let view: ViewModel | null = null;
  let keys: Record<string, string> = {};
  /** The permission whose buttons are on screen: the only one a key may answer. */
  let onScreen: string | null = null;
  /** A card answered once: a second click or key on it sends nothing. */
  let sent: string | null = null;
  let onlyNeeds = false;
  let zeca = true;
  let full = false;
  let detail: Detail = "diff";
  /** The diff shown: whose step, and what came back (undefined while it loads). */
  let shownDiff: { owner: string; step: number; diff: Diff | null | undefined } | null = null;
  let focusDiff = false;
  /** Whose card is shown: Focus diff ends when another session comes in front. */
  let shownFront: string | null = null;
  /** The front session keeps edits to show: without them there is no diff to focus on. */
  let hasEdits = false;
  let columns = 0;
  let width = 0;

  /** Who is in front, as core says it for every surface: a waiting card's session, then the focus. */
  const frontOf = (v: ViewModel): SessionView | null => {
    const at = (r: ViewModel["front"]) => (r ? v.sessions.find((s) => s.agent === r.agent && s.id === r.id) : undefined);
    return at(v.front) ?? at(v.focus) ?? v.sessions[0] ?? null;
  };
  const visible = (v: ViewModel): SessionView[] => {
    const q = search.value.trim().toLowerCase();
    return v.sessions.filter((s) => (!onlyNeeds || needsYou(s)) && (!q || said(s).includes(q)));
  };

  /** Over each vult a button (its name and state on hover and for a screen reader; a click puts it
   *  in front) and its badge when its state asks for one. */
  const drawSky = () => {
    if (!view) return;
    const v = view;
    assignSpecies(v.sessions);
    scene.update(v.sessions, null);
    const front = frontOf(v);
    const byKey = new Map(v.sessions.map((s) => [key(s), s]));
    // Each badge sits by its bird's head, wherever the bird's species puts it.
    const heads = new Map(scene.anchors().map((a) => [a.key, a.y]));
    badges.replaceChildren(
      ...[...scene.slots()].sort((a, b) => a.x - b.x).flatMap((slot) => {
        const s = byKey.get(slot.key);
        if (!s) return [];
        const label = [s.project || agentName(s), agentName(s), statusText(s)].join(" · ");
        const b = fid(el("button", { class: `habitat-bird${front && key(front) === slot.key ? " front" : ""}`, onclick: () => on.focus(s) }), `bird:${slot.key}`);
        menuOn(b, s);
        b.style.left = `${slot.x - 4}px`;
        b.style.width = `${slot.width + 8}px`;
        b.title = label;
        b.setAttribute("aria-label", label);
        const kind = badgeOf(s);
        const badge = kind ? el("span", { class: `badge ${kind}` }) : null;
        if (badge) {
          badge.style.left = `${slot.x + slot.width - 8}px`;
          badge.style.top = `${Math.max(4, (heads.get(slot.key) ?? 40) - 36)}px`;
        }
        return badge ? [b, badge] : [b];
      }),
      ...(columns > 0 && v.sessions.length > columns
        ? [el("span", { class: `habitat-more${v.sessions.slice(columns).some(needsYou) ? " needs" : ""}`, text: `+${v.sessions.length - columns}` })]
        : []),
    );
  };

  /** One column per bird the pane has room for, up to the habitat's eight; a bird over each card. */
  const layout = () => {
    const w = Math.round(sky.clientWidth);
    if (!w || !view) return;
    const cols = Math.max(1, Math.min(view.sessions.length || 1, 8, Math.floor(w / COLUMN_MIN)));
    if (w === width && cols === columns) return;
    width = w;
    columns = cols;
    const col = w / cols;
    scene.resize(w, cols, (slot) => ({ x: Math.max(0, Math.round(col * slot + (col - BIRD_PX) / 2)), wire: NEST_HABITAT_WIRE }));
    roosts.style.gridTemplateColumns = `repeat(${cols}, minmax(0, 1fr))`;
    drawSky();
  };
  new ResizeObserver(() => layout()).observe(sky);

  /** The prototype's "!" by a name: seen, not read twice (the state under it says it). */
  const marker = (needs: boolean) => {
    const m = el("span", { class: "marker", text: needs ? "!" : "" });
    m.setAttribute("aria-hidden", "true");
    return m;
  };
  const roost = (s: SessionView, front: boolean) => {
    const b = fid(el("button", { class: "roost-card", onclick: () => on.focus(s) }), `roost:${key(s)}`);
    menuOn(b, s);
    b.style.setProperty("--bird-color", colorOf(s.agent));
    b.setAttribute("aria-pressed", String(front));
    b.append(
      el("div", { class: "roost-label" }, el("strong", { text: s.project || agentName(s) }), marker(needsYou(s))),
      el("div", { class: "roost-sub", text: statusText(s) }),
      el("div", { class: "roost-agent", text: agentName(s) }),
    );
    return b;
  };

  /** A finished step's +N −M: the Changes tab, on that step. */
  const showChange = (s: SessionView, step: number) => {
    detail = "diff";
    load(s, step);
    draw();
  };

  /** The prototype's ticker: the step just done (a check), the one under way (a chevron), with its
   *  +N −M when it is a finished edit. */
  const tickerOf = (s: SessionView): HTMLElement | null => {
    const steps = tickerSteps(s);
    if (!steps.length || s.status !== "working") return null;
    const row = (step: (typeof steps)[number], now: boolean) => {
      const p = el("p", { class: `card-tick${now ? " now" : ""}` }, el("span", { text: `${now ? "›" : "✓"} ${step.text}` }));
      const d = step.diff;
      if (d) p.append(fid(el("button", { class: "diffbtn", text: `+${d.added} −${d.removed}`, onclick: () => showChange(s, d.step) }), `act:tick-diff:${d.step}`));
      return p;
    };
    const last = steps[steps.length - 1];
    const before = steps.length > 1 ? steps[steps.length - 2] : null;
    return el("div", { class: "card-ticker" }, before ? row(before, false) : null, row(last, true));
  };

  /** A quiet bird: how long, and the user's three answers (as on the island). */
  const silenceOf = (s: SessionView): HTMLElement | null => {
    if (s.status !== "working" || !s.silent) return null;
    const small = (text: string, hush: Hush, title: string) => {
      const b = fid(el("button", { class: "hush", text, onclick: () => on.hush(s, hush) }), `act:${hush}`);
      b.title = title;
      return b;
    };
    return el(
      "div",
      { class: `silence ${s.silent}` },
      el("span", { class: "silence-text", text: s.silent === "loud" ? t("No news for 15 minutes.") : t("No news for 5 minutes.") }),
      small(t("Snooze"), "snooze", t("Hide this flag for 15 minutes")),
      small(t("Keep going"), "keep-going", t("It is fine: no flag for the next 30 minutes")),
      small(t("Dismiss"), "dismiss", t("No flag again until its next prompt")),
    );
  };

  /** Under a card that does not wait: what its kept edits changed (the last steps the app keeps). */
  const changedOf = (s: SessionView): HTMLElement | null => {
    const edits = tickerSteps(s).flatMap((step) => (step.diff ? [step.diff] : []));
    if (!edits.length) return null;
    const added = edits.reduce((n, d) => n + d.added, 0), removed = edits.reduce((n, d) => n + d.removed, 0);
    const last = edits[edits.length - 1];
    return fid(
      el(
        "button",
        { class: "compact-detail", onclick: () => showChange(s, last.step) },
        el("span", {}, el("span", { class: "added", text: `+${added}` }), document.createTextNode(" "), el("span", { class: "removed", text: `−${removed}` })),
        el("span", { text: edits.length === 1 ? t("1 edit") : t("{n} edits", { n: edits.length }) }),
      ),
      "act:changes",
    );
  };

  /** The question card's answers; the Nest has no keyboard to hold and nothing to measure. */
  const questionActions = {
    keys: {},
    decide: () => {},
    decideAlways: () => {},
    answer: (request: string, answers: Answer[]) => on.answer(request, answers),
    release: (request: string) => on.release(request),
    keyboard: () => {},
    relayout: () => {},
    jump: () => {},
    dismiss: () => {},
    hush: () => {},
    openChat: () => {},
  };

  /** The question on screen, kept across views: drawn anew, its *Other…* field would lose what is
   *  being typed and take the keyboard back on every view. */
  let asked: { request: string; node: HTMLElement } | null = null;
  const questionNode = (s: SessionView, question: NonNullable<ViewModel["approval"]>): HTMLElement => {
    if (asked?.request !== question.request)
      asked = { request: question.request, node: el("div", { class: "nest-question" }, ...questionBody(s, question, questionActions)) };
    return asked.node;
  };

  /** Deny, Allow and Always on the permission in front, as the island draws them (ADR 0004:
   *  destructive makes Deny the bright one and offers no Always). */
  const permissionActions = (s: SessionView, ask: NonNullable<ViewModel["approval"]>): HTMLElement => {
    onScreen = ask.request;
    const answer = (d: "allow" | "deny" | "always") => () => {
      if (sent === ask.request) return;
      sent = ask.request;
      if (d === "always") on.decideAlways(ask.request);
      else on.decide(ask.request, d);
    };
    const kbd = (k: string | undefined) => (k ? el("span", { class: "kbd", text: k }) : null);
    const destructive = ask.why === "destructive";
    const always = s.cwd && !ask.cut && !destructive
      ? fid(el("button", { class: "outline", text: t("Always allow"), onclick: answer("always") }), "act:always")
      : null;
    if (always) always.title = t("Allow this exact {tool} call in {project} from now on, without asking", { tool: ask.tool, project: s.project || t("this folder") });
    return el(
      "div",
      { class: "app-card-actions" },
      fid(el("button", { class: destructive ? "primary" : "outline", onclick: answer("deny") }, el("span", { text: t("Deny") }), kbd(keys.deny)), "act:deny"),
      fid(el("button", { class: destructive ? "outline danger" : "primary", onclick: answer("allow") }, el("span", { text: t("Allow") }), kbd(keys.allow)), "act:allow"),
      always,
    );
  };

  /** The session in front: its state, what it is doing or asks, and what can be done from here. */
  const card = (v: ViewModel, s: SessionView) => {
    const ask = v.approval && v.approval.agent === s.agent && v.approval.session === s.id ? v.approval : null;
    // A finished or failed turn says how it ended (its last reply, its error), as the island does.
    const ended = s.status === "finished" || s.status === "failed";
    const ticker = ask ? null : tickerOf(s);
    const words = ask?.description ?? (ended ? s.note : null) ?? (ticker ? null : (s.step ?? statusText(s)));
    const question = ask && ask.questions.length ? ask : null;
    const permission = ask && !question ? ask : null;
    const command = permission ? [permission.tool, permission.full ?? permission.target].filter(Boolean).join(" · ") : null;
    const tag = el("span", { class: "agent-tag" }, el("span", { class: "agent-dot" }));
    tag.style.color = colorOf(s.agent);
    tag.title = agentName(s);
    return el(
      "div",
      { class: `app-card ${tone(s)}` },
      el(
        "div",
        { class: "app-card-head" },
        tag,
        el("strong", { class: "card-name", text: s.project || agentName(s) }),
        s.editor ? el("span", { class: "ed-chip", text: s.editor }) : null,
        // With the ticker under it the head says no step: the ticker does (the prototype's head).
        ticker ? null : el("span", { class: "card-verb", text: statusText(s) }),
        // Working: its subagents, steps so far and cost at API prices, as the prototype's head says.
        s.status === "working" && s.subagents > 0
          ? el("span", { class: "card-verb", text: s.subagents === 1 ? t("+1 subagent") : t("+{n} subagents", { n: s.subagents }) })
          : null,
        s.status === "working" && s.step_count > 0
          ? el("span", { class: "card-verb", text: s.step_count === 1 ? t("1 step") : t("{n} steps", { n: s.step_count }) })
          : null,
        costChip(s),
        ask && ask.queue > 1 ? el("div", { class: "card-side" }, el("span", { class: "q-of", text: t("1 of {n}", { n: ask.queue }) })) : null,
      ),
      question ? null : words ? el("p", { class: "card-words", text: words }) : null,
      ticker,
      silenceOf(s),
      command
        ? el(
            "div",
            { class: "command" },
            el("code", { text: command }),
            fid(el("button", { class: "outline", text: t("Copy"), onclick: () => void navigator.clipboard?.writeText(command).catch(() => {}) }), "act:copy"),
          )
        : null,
      permission?.why
        ? el("p", {
            class: `ask-why ${permission.why}`,
            text: permission.why === "destructive" ? t("Destructive: no policy ever allows it") : t("Your policy asks first for this"),
          })
        : null,
      question ? questionNode(s, question) : null,
      permission ? permissionActions(s, permission) : null,
      el(
        "div",
        { class: "app-card-actions" },
        s.raise === true ? fid(el("button", { class: "outline", text: t("Open terminal"), onclick: () => on.jump(s) }), "act:jump") : null,
        zeca ? fid(el("button", { class: "outline", text: t("Ask Zeca"), onclick: () => on.ask(s) }), "act:ask") : null,
      ),
      // No card for it: it asks in its terminal (its card expired or went there, or it has none).
      waits(s) && !ask ? el("p", { class: "rule-scope", text: t("Needs you: it asks in its terminal.") }) : null,
      waits(s) ? null : changedOf(s),
    );
  };

  /** Loads one step's diff; the tab draws again when it comes, if it is still the one shown. */
  const load = (s: SessionView, step: number) => {
    const asked: { owner: string; step: number; diff: Diff | null | undefined } = { owner: key(s), step, diff: undefined };
    shownDiff = asked;
    const done = (d: Diff | null) => {
      asked.diff = d;
      if (shownDiff === asked) draw();
    };
    void on.diff(s, step).then(done, () => done(null));
  };

  /** Changes: the session's finished edits, one at a time, read from its kept diffs. */
  const changes = (s: SessionView): HTMLElement[] => {
    const edits = tickerSteps(s).flatMap((step) => (step.diff ? [{ text: step.text, diff: step.diff }] : []));
    hasEdits = edits.length > 0;
    if (!edits.length) return [el("p", { class: "detail-footnote", text: t("No changes kept for this session yet.") })];
    if (!shownDiff || shownDiff.owner !== key(s) || !edits.some((e) => e.diff.step === shownDiff!.step)) load(s, edits[edits.length - 1].diff.step);
    const now = shownDiff!;
    const pick = fid(el("select", { class: "form-select" }), "act:diff-pick");
    pick.setAttribute("aria-label", t("The change shown"));
    for (const e of edits) {
      const o = el("option", { text: `${e.text} · +${e.diff.added} −${e.diff.removed}` });
      o.value = String(e.diff.step);
      o.selected = e.diff.step === now.step;
      pick.append(o);
    }
    pick.addEventListener("change", () => {
      load(s, Number(pick.value));
      draw();
    });
    const focusButton = fid(
      el("button", {
        class: "outline",
        text: focusDiff ? t("Back to the card") : t("Focus diff ↗"),
        onclick: () => {
          focusDiff = !focusDiff;
          draw();
        },
      }),
      "act:focus-diff",
    );
    focusButton.setAttribute("aria-pressed", String(focusDiff));
    const sum = edits.find((e) => e.diff.step === now.step)!.diff;
    const d = now.diff;
    const where = d && d.files.length === 1 ? d.files[0].path : t("{n} files", { n: sum.files });
    const out: HTMLElement[] = [
      el("div", { class: "diff-toolbar" }, pick, focusButton),
      el("div", { class: "diff-caption" }, el("span", { text: t("Modified · +{added} −{removed}", { added: sum.added, removed: sum.removed }) }), el("span", { text: where })),
      d === undefined
        ? el("p", { class: "detail-footnote", text: t("Loading the changes…") })
        : d === null
          ? el("p", { class: "detail-footnote", text: t("This change is no longer kept: the session has moved on.") })
          : el("div", { class: "diff-block" }, ...diffRows(d)),
    ];
    if (d?.cut) out.push(el("p", { class: "detail-footnote", text: t("Only the start of this change: the rest was too long to keep.") }));
    return out;
  };

  const activity = (s: SessionView): HTMLElement[] => {
    const steps = tickerSteps(s);
    if (!steps.length) return [el("p", { class: "detail-footnote", text: t("No steps yet.") })];
    const first = s.step_count - s.steps.length + 1;
    const latest = steps.at(-1)?.n;
    return steps
      .slice(-8)
      .reverse()
      .map((step) => {
        // The wall-clock time it started, as the prototype writes it (14:21); its number before
        // the app said the time.
        const at = s.step_at?.[step.n - first] ?? null;
        const time = el("time", { text: at === null ? `#${step.n}` : clock(at) });
        if (at !== null) {
          time.dateTime = new Date(at * 1000).toISOString();
          time.title = `#${step.n}`;
        }
        // Its mark: what it changed, else the latest while the session is still at it (working, or
        // waiting on a card), else done.
        const busy = s.status === "working" || s.status === "approval" || s.status === "question";
        const mark = step.diff ? `+${step.diff.added}` : step.n === latest && busy ? "→" : "✓";
        return el("div", { class: "activity-row" }, time, el("div", {}, el("strong", { text: step.text })), el("span", { text: mark }));
      });
  };

  const context = (s: SessionView): HTMLElement[] => {
    const rows: [string, string | null | undefined][] = [
      [t("Project"), s.project],
      [t("Repository"), s.roost],
      [t("Branch"), s.branch],
      [t("Folder"), s.cwd],
      [t("Agent"), agentName(s)],
      [t("Editor"), s.editor],
      [t("Status"), statusText(s)],
      [t("Cost so far"), s.cost_usd != null && s.cost_usd >= 0.01 ? `≈$${s.cost_usd.toFixed(2)}` : null],
    ];
    return [el("dl", { class: "context-dl" }, ...rows.flatMap(([k, v]) => (v ? [el("dt", { text: k }), el("dd", { text: v })] : [])))];
  };

  const zecaPane = (s: SessionView): HTMLElement[] => [
    el("p", { class: "detail-footnote", text: t("Zeca reads this session's steps and changes, and answers in the Chat. He never answers a card.") }),
    fid(el("button", { class: "primary", text: t("Ask Zeca about this session"), onclick: () => on.ask(s) }), "act:ask-pane"),
  ];

  const draw = () => {
    if (!view) return;
    const v = view;
    // A view redraws the tab: the control that had the keyboard gets it back after.
    const had = (document.activeElement as HTMLElement | null)?.dataset?.fid ?? null;
    onScreen = null;
    layout();
    drawSky();
    const front = frontOf(v);
    const shown = visible(v);
    const clear = fid(el("button", {
      class: "outline",
      text: t("Clear filters"),
      onclick: () => {
        search.value = "";
        onlyNeeds = false;
        relabel();
        draw();
      },
    }), "act:clear");
    roosts.replaceChildren(
      ...(shown.length
        ? shown.map((s) => {
            const card = roost(s, !!front && key(front) === key(s));
            // Under its own bird, filtered or not; a session past the habitat's places comes after.
            const at = v.sessions.indexOf(s);
            if (at < columns) {
              card.style.gridColumn = String(at + 1);
              card.style.gridRow = "1";
            }
            return card;
          })
        : [
            el(
              "div",
              { class: "filtered-note" },
              el("p", { text: v.sessions.length ? t("No session matches. The flock stays in sight above.") : t("No sessions right now. Start an agent and its bird lands here.") }),
              v.sessions.length ? clear : null,
            ),
          ]),
    );
    roosts.classList.toggle("empty", !shown.length);
    selected.textContent = front ? t("Selected: {name}", { name: front.project || agentName(front) }) : "";
    // With less motion nobody flies: they all sit on the wire.
    const out = calmNow() ? 0 : v.sessions.filter((s) => s.status === "working" && s.activity === "web").length;
    summary.textContent = v.sessions.length ? t("{out} in flight · {on} on the wire", { out, on: v.sessions.length - out }) : "";
    body.hidden = !front;
    toolbar.hidden = !v.sessions.length;
    if (front && key(front) !== shownFront) {
      shownFront = key(front);
      focusDiff = false;
    }
    if (front) {
      focusBlock.replaceChildren(card(v, front));
      if (detail === "zeca" && !zeca) detail = "diff";
      const tabs: [Detail, string][] = [
        ["diff", t("Changes")],
        ["activity", t("Activity")],
        ["context", t("Context")],
      ];
      if (zeca) tabs.push(["zeca", t("Zeca")]);
      const sums = tickerSteps(front).reduce((n, step) => (step.diff ? { added: n.added + step.diff.added, removed: n.removed + step.diff.removed } : n), { added: 0, removed: 0 });
      detailTabs.replaceChildren(
        ...tabs.map(([id, word]) => {
          const b = fid(
            el("button", {
              class: "detail-tab",
              text: word,
              onclick: () => {
                detail = id;
                draw();
              },
            }),
            `detail:${id}`,
          );
          if (id === "diff" && (sums.added || sums.removed)) b.append(el("span", { class: "detail-count", text: `+${sums.added} −${sums.removed}` }));
          b.id = `nest-detail-${id}`;
          b.setAttribute("role", "tab");
          b.setAttribute("aria-selected", String(id === detail));
          b.setAttribute("aria-controls", "nest-detail-panel");
          // One stop in the Tab order; Left and Right move between them.
          b.tabIndex = id === detail ? 0 : -1;
          return b;
        }),
      );
      detailBody.replaceChildren(...(detail === "diff" ? changes(front) : detail === "activity" ? activity(front) : detail === "context" ? context(front) : zecaPane(front)));
      detailBody.setAttribute("aria-labelledby", `nest-detail-${detail}`);
    }
    body.classList.toggle("details-focus", focusDiff && detail === "diff" && hasEdits);
    if (had) element.querySelector<HTMLElement>(`[data-fid="${CSS.escape(had)}"]`)?.focus();
  };

  /** Full's strip: each plan's windows, amber from 70% and red from 90%, as on the island. */
  const drawUsage = () => {
    if (!full) return;
    void on.usage().then(
      (windows) => {
        const byAgent = new Map<AgentKind, UsageWindow[]>();
        for (const w of windows) byAgent.set(w.agent, [...(byAgent.get(w.agent) ?? []), w]);
        const span = (m: number) => (m >= 10080 ? t("7d") : t("5h"));
        usage.replaceChildren(
          ...[...byAgent].map(([agent, ws]) => {
            const dot = el("span", { class: "agent-dot" });
            dot.style.color = colorOf(agent);
            return el(
              "div",
              { class: "plan" },
              dot,
              el("span", { text: PLAN_NAME[agent] ?? agent }),
              ...ws.map((w) => el("span", { class: `meter${w.used_percent >= 90 ? " red" : w.used_percent >= 70 ? " amber" : ""}`, text: `${span(w.minutes)} ${Math.round(w.used_percent)}%` })),
            );
          }),
          el("span", { class: "usage-caption", text: t("Amber at 70% · red at 90%") }),
        );
        usage.hidden = !full || !windows.length;
      },
      () => {
        usage.hidden = true;
      },
    );
  };

  const relabel = () => {
    heading.textContent = t("The flock, together");
    title.textContent = t("Session details");
    search.placeholder = t("Find a session…");
    search.setAttribute("aria-label", t("Find a session…"));
    needs.textContent = t("Needs you");
    needs.setAttribute("aria-pressed", String(onlyNeeds));
    detailTabs.setAttribute("aria-label", t("Session details"));
  };
  detailTabs.addEventListener("keydown", (e) => {
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
    const ids = [...detailTabs.querySelectorAll<HTMLElement>(".detail-tab")].map((b) => b.id.replace("nest-detail-", "") as Detail);
    const at = ids.indexOf(detail);
    detail = ids[(at + (e.key === "ArrowRight" ? 1 : -1) + ids.length) % ids.length];
    e.preventDefault();
    draw();
    detailTabs.querySelector<HTMLElement>(`#nest-detail-${detail}`)?.focus();
  });
  search.addEventListener("input", draw);
  needs.addEventListener("click", () => {
    onlyNeeds = !onlyNeeds;
    relabel();
    draw();
  });
  // Up and down walk the sessions shown, from the one in front, as on the island. On the window: a
  // redraw takes the focused control away, and the keys must still arrive.
  let active = true;
  window.addEventListener("keydown", (e) => {
    if (!active || !view || (e.target as Element | null)?.closest?.("input, textarea, select, [contenteditable], .detail-tabs")) return;
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    const rows = visible(view);
    if (!rows.length) return;
    const front = frontOf(view);
    const at = front ? rows.findIndex((s) => key(s) === key(front)) : -1;
    const step = e.key === "ArrowDown" ? 1 : -1;
    const next = rows[at < 0 ? 0 : (at + step + rows.length) % rows.length];
    e.preventDefault();
    on.focus(next);
  });
  relabel();
  return {
    element,
    setKeys: (k) => {
      keys = k;
      draw();
    },
    shortcut: (allow) => {
      const ask = view?.approval;
      if (!element.isConnected || element.offsetParent === null || !ask || ask.request !== onScreen || ask.questions.length) return false;
      if (sent === ask.request) return true;
      sent = ask.request;
      on.decide(ask.request, allow ? "allow" : "deny");
      return true;
    },
    update: (v) => {
      // What the menu offers was read from the view before: a new one closes it.
      closeMenu();
      view = v;
      relabel();
      draw();
    },
    setZeca: (shown) => {
      zeca = shown;
      draw();
    },
    setActive: (shown) => {
      if (!shown) closeMenu(false);
      active = shown;
      scene.setActive(shown);
      if (shown) layout();
    },
    setFull: (f) => {
      if (f === full) return;
      full = f;
      usage.hidden = !f;
      drawUsage();
    },
    usageChanged: drawUsage,
  };
}
