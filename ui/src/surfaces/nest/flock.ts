// The Nest's Flock tab (ADR 0021), as the Living Flock prototype draws it: the stage (the flock at
// 3x across the pane, a roost card under each bird), *Session details* with *Needs you* and a
// search, the card of the session in front at the left and its Changes, Activity, Context and Zeca
// at the right; in Full, the plans' usage under it. Nothing here answers a card: a session that
// waits shows its request and leads to the island (D1).
import type { Diff, Hush, UsageWindow } from "../../bridge";
import type { AgentKind, SessionView, ViewModel } from "../../view.gen";
import { el } from "../../dom";
import { t } from "../../i18n";
import { assignSpecies } from "../../island/flock";
import { BIRD_W, NEST_HABITAT_SCENE, NEST_HABITAT_WIRE, Scene } from "../../island/scene";
import { tickerSteps } from "../../island/ticker";
import { agentName, badgeOf, needsYou, statusText } from "../../island/views";

export interface FlockActions {
  /** Puts the session in front of the flock (core's focus). */
  focus: (s: SessionView) => void;
  /** Its terminal, where `raise` says it can be brought forward. */
  jump: (s: SessionView) => void;
  /** The island, on the card this session waits on. */
  review: (s: SessionView) => void;
  /** A finished step's diff; null once the session no longer keeps it. */
  diff: (s: SessionView, step: number) => Promise<Diff | null>;
  /** The plans' usage windows, for Full's strip. */
  usage: () => Promise<UsageWindow[]>;
  /** The answer to a quiet bird's flag: none of them reaches the agent. */
  hush: (s: SessionView, hush: Hush) => void;
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

export function createFlock(on: FlockActions & FlockAsk): {
  element: HTMLElement;
  update: (v: ViewModel) => void;
  setZeca: (shown: boolean) => void;
  setActive: (shown: boolean) => void;
  /** Full shows the plans' usage under the details. */
  setFull: (full: boolean) => void;
  /** The plans' usage changed: Full's strip reads it again. */
  usageChanged: () => void;
} {
  const scene = new Scene(NEST_HABITAT_SCENE);
  const badges = el("div", { class: "badges" });
  const sky = el("div", { class: "habitat-sky" }, scene.canvas, badges);
  const heading = el("strong");
  const roosts = el("div", { class: "roost-cards" });
  const selected = el("p", { class: "habitat-selected" });
  const habitat = el(
    "section",
    { class: "flock-stage" },
    el("header", { class: "habitat-header" }, el("div", { class: "habitat-heading" }, el("span", { class: "legend-dot" }), heading)),
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
  let view: ViewModel | null = null;
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

  /** The session in front: its state, what it is doing or asks, and what can be done from here. */
  const card = (v: ViewModel, s: SessionView) => {
    const ask = v.approval && v.approval.agent === s.agent && v.approval.session === s.id ? v.approval : null;
    // A finished or failed turn says how it ended (its last reply, its error), as the island does.
    const ended = s.status === "finished" || s.status === "failed";
    const ticker = ask ? null : tickerOf(s);
    const words = ask?.description ?? (ended ? s.note : null) ?? (ticker ? null : (s.step ?? statusText(s)));
    const command = ask ? [ask.tool, ask.full ?? ask.target].filter(Boolean).join(" · ") : null;
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
      words ? el("p", { class: "card-words", text: words }) : null,
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
      el(
        "div",
        { class: "app-card-actions" },
        waits(s) ? fid(el("button", { class: "primary", text: t("Review on the island →"), onclick: () => on.review(s) }), "act:review") : null,
        s.raise === true ? fid(el("button", { class: "outline", text: t("Open terminal"), onclick: () => on.jump(s) }), "act:jump") : null,
        zeca ? fid(el("button", { class: "outline", text: t("Ask Zeca"), onclick: () => on.ask(s) }), "act:ask") : null,
      ),
      waits(s) ? el("p", { class: "rule-scope", text: t("Needs you: the card waits on the island.") }) : null,
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
    return steps
      .slice(-8)
      .reverse()
      .map((step) =>
        el(
          "div",
          { class: "activity-row" },
          el("time", { text: `#${step.n}` }),
          el("div", {}, el("strong", { text: step.text })),
          el("span", { text: step.diff ? `+${step.diff.added}` : "" }),
        ),
      );
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
    update: (v) => {
      view = v;
      relabel();
      draw();
    },
    setZeca: (shown) => {
      zeca = shown;
      draw();
    },
    setActive: (shown) => {
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
