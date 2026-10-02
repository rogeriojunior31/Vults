// What the open island shows of the flock: the focus card (Zeca on his piece of wire, with a glow
// in the color of the state, and the session's card beside him) and the flock list (one row per
// other session). Each state has its own wash, wording and actions; working sessions show the step
// ticker.
import type { ApprovalView, SessionView } from "../bridge";
import { el } from "../dom";
import { icon } from "./icons";
import type { Ticker } from "./ticker";

export const AGENT_NAME = { claude: "Claude Code", codex: "Codex" } as const;

/** A status in a few words, for the compact pill and the flock list. */
const STATUS_TEXT: Record<SessionView["status"], string> = {
  idle: "Waiting for work",
  thinking: "Thinking…",
  working: "Working",
  approval: "Needs you",
  question: "Has a question",
  finished: "Done",
  failed: "Failed",
  ratelimited: "Rate limited",
};

/** Working shows what it is doing; any other state shows the state itself. */
export function statusText(s: SessionView): string {
  return s.status === "working" && s.step ? s.step : STATUS_TEXT[s.status];
}

/** Statuses that need a look, worn as a badge by a bird that is not in front. */
export const BADGE: Partial<Record<SessionView["status"], string>> = {
  approval: "approval",
  question: "question",
  finished: "finished",
  failed: "failed",
};

export interface CardActions {
  /** Keys bound for "allow" and "deny", shown on the buttons. */
  keys: Record<string, string>;
  decide(request: string, decision: "allow" | "deny"): void;
  /** Allow, and every identical request in this project from now on. */
  decideAlways(request: string): void;
  jump(agent: SessionView["agent"], id: string): void;
  /** OK on a finished or failed card: seen, the badge goes. */
  dismiss(s: SessionView): void;
  openChat(): void;
}

type Wash = "amber" | "green" | "red" | "cyan" | null;

const WASH: Partial<Record<SessionView["status"], Wash>> = {
  approval: "amber",
  ratelimited: "amber",
  question: "cyan",
  finished: "green",
  failed: "red",
};

function who(
  s: SessionView,
  label: string,
  ...extra: (Node | null)[]
): HTMLElement {
  return el(
    "div",
    { class: "who" },
    el("span", { class: `dot ${s.agent}` }),
    el("span", { class: "name", text: s.project || AGENT_NAME[s.agent] }),
    el("span", { class: "label", text: label }),
    ...extra,
  );
}

function button(
  text: string,
  kind: "primary" | "secondary",
  onclick: () => void,
  glyph: Node | null = null,
) {
  return el(
    "button",
    { class: `btn ${kind}`, onclick },
    el("span", { text }),
    glyph,
  );
}

function kbd(keys: string | undefined): Node | null {
  return keys ? el("span", { class: "kbd", text: keys }) : null;
}

function jumpButton(
  s: SessionView,
  actions: CardActions,
  kind: "primary" | "secondary" = "secondary",
) {
  return button(
    "Open terminal",
    kind,
    () => actions.jump(s.agent, s.id),
    icon("openOut", 12),
  );
}

/**
 * The card for the session in front, or for an empty wire. `perch` (Zeca's canvas and glow) moves
 * into it; `jumpFailed` adds a line when the last Open terminal found nothing to bring forward.
 */
export function focusCard(
  s: SessionView | null,
  approval: ApprovalView | null,
  ticker: Ticker,
  perch: HTMLElement,
  actions: CardActions,
  jumpFailed: boolean,
): HTMLElement {
  const state = s ? s.status : "empty";
  perch.className = `perch ${state}${s ? ` ${s.agent}` : ""}`;
  const wash = s ? (WASH[s.status] ?? null) : null;
  const body = el(
    "div",
    { class: "focus-body" },
    ...(s ? sessionBody(s, approval, ticker, actions) : emptyBody(actions)),
  );
  if (s && jumpFailed)
    body.append(
      el("div", {
        class: "hint",
        text: "Couldn't find this session's terminal.",
      }),
    );
  return el(
    "section",
    { class: `card focus${wash ? ` wash ${wash}` : ""}` },
    perch,
    body,
  );
}

function emptyBody(actions: CardActions): HTMLElement[] {
  return [
    el("div", { class: "title", text: "Nothing running right now." }),
    el("div", {
      class: "sub",
      text: "Start Claude Code or Codex in a terminal and it lands on the wire. Or ask Zeca.",
    }),
    el(
      "div",
      { class: "actions start" },
      button("Ask Zeca", "primary", () => actions.openChat(), icon("chat", 12)),
    ),
  ];
}

function sessionBody(
  s: SessionView,
  approval: ApprovalView | null,
  ticker: Ticker,
  actions: CardActions,
): HTMLElement[] {
  const last = s.step ?? "";
  const dismiss = () => actions.dismiss(s);
  switch (s.status) {
    case "approval": {
      if (!approval) break;
      let sent = false;
      const answer = (d: "allow" | "deny" | "always") => () => {
        if (sent) return;
        sent = true;
        if (d === "always") actions.decideAlways(approval.request);
        else actions.decide(approval.request, d);
      };
      const always = el("button", { class: "btn ghost-btn", onclick: answer("always") }, el("span", { text: "Always allow" }));
      always.title = `Allow this exact ${approval.tool} call in ${s.project || "this folder"} from now on, without asking`;
      const diff =
        approval.added || approval.removed
          ? el("span", { class: "diff" }, el("span", { class: "add", text: `+${approval.added}` }), el("span", { class: "del", text: `−${approval.removed}` }))
          : null;
      const queue = approval.queue > 1 ? el("span", { class: "queue", text: `1 of ${approval.queue}` }) : null;
      // The whole command when the target had to cut it; the tool stays in front of it.
      const what = approval.full ? `${approval.tool} · ${approval.full}` : approval.target;
      const parts: (HTMLElement | null)[] = [
        el("div", { class: "card-head" }, who(s, "needs your permission", diff), queue),
        approval.description ? el("div", { class: "ask-what", text: approval.description }) : null,
        el("pre", { class: "code", text: what }),
        el(
          "div",
          { class: "actions" },
          s.cwd ? always : null,
          button("Deny", "secondary", answer("deny"), kbd(actions.keys.deny)),
          button("Allow", "primary", answer("allow"), kbd(actions.keys.allow)),
        ),
        el("div", { class: "expiry" }, el("span", { class: "expiry-text" }), el("span", { class: "expiry-bar" })),
      ];
      return parts.filter((n): n is HTMLElement => n !== null);
    }
    case "question":
      return [
        who(s, "has a question"),
        el("div", {
          class: "title clamp",
          text: s.note ?? "Waiting for your answer.",
        }),
        el("div", { class: "actions" }, jumpButton(s, actions, "primary")),
      ];
    case "finished":
      return [
        who(s, `${AGENT_NAME[s.agent]} finished`),
        el("div", {
          class: "title clamp",
          text: s.note ?? (last || "Turn finished."),
        }),
        el(
          "div",
          { class: "actions" },
          button("OK", "secondary", dismiss),
          jumpButton(s, actions, "primary"),
        ),
      ];
    case "failed":
      return [
        who(s, "stopped on an error"),
        el("div", {
          class: "detail clamp",
          text: s.note ?? (last || "No detail available."),
        }),
        el(
          "div",
          { class: "actions" },
          button("OK", "secondary", dismiss),
          jumpButton(s, actions, "primary"),
        ),
      ];
    case "ratelimited":
      return [
        who(s, "hit a usage limit"),
        el("div", { class: "title", text: "Paused until the limit resets." }),
        el("div", { class: "actions" }, jumpButton(s, actions)),
      ];
    default:
      break;
  }
  const { count, helpers, open } = counted(s, actions);
  const head = (): HTMLElement =>
    el(
      "div",
      { class: "card-head" },
      who(s, AGENT_NAME[s.agent], helpers, count),
      open,
    );
  // Idle: nothing is happening, so no ticker pretending it is; the last step, dimmed.
  if (s.status === "idle") {
    return [
      head(),
      el("div", {
        class: "sub",
        text: s.steps.length
          ? "Waiting for the next prompt."
          : "Waiting for a prompt.",
      }),
      ...(last
        ? [el("div", { class: "hint", text: `Last step: ${last}` })]
        : []),
    ];
  }
  ticker.sync(`${s.agent}:${s.id}`, s.steps);
  return [
    head(),
    s.steps.length
      ? ticker.element
      : el("div", { class: "sub", text: "Thinking…" }),
  ];
}

/** The working card's head: steps so far, subagents, and Open terminal. */
function counted(
  s: SessionView,
  actions: CardActions,
): {
  count: HTMLElement | null;
  helpers: HTMLElement | null;
  open: HTMLElement;
} {
  const count =
    s.step_count > 0
      ? el("span", {
          class: "count",
          text: `${s.step_count} ${s.step_count === 1 ? "step" : "steps"}`,
        })
      : null;
  const helpers =
    s.subagents > 0
      ? el("span", {
          class: "helpers",
          text: `+${s.subagents} ${s.subagents === 1 ? "subagent" : "subagents"}`,
        })
      : null;
  const open = el(
    "button",
    { class: "icon-btn", onclick: () => actions.jump(s.agent, s.id) },
    icon("openOut", 13),
  );
  open.title = "Open terminal";
  return { count, helpers, open };
}

/** One row per session, in wire order; `canvas` draws their birds down the left edge. */
export function flockRows(
  sessions: SessionView[],
  canvas: HTMLCanvasElement,
  pick: (s: SessionView) => void,
): HTMLElement[] {
  return [
    canvas,
    ...sessions.map((s) => {
      const badge = BADGE[s.status];
      const row = el(
        "button",
        { class: `flock-row ${s.status}`, onclick: () => pick(s) },
        el(
          "span",
          { class: "flock-text" },
          el("span", { class: "name", text: s.project || AGENT_NAME[s.agent] }),
          el("span", { class: `status ${s.status}`, text: statusText(s) }),
        ),
        badge ? el("span", { class: `badge ${badge}` }) : null,
      );
      row.title = `${s.project || AGENT_NAME[s.agent]} · ${AGENT_NAME[s.agent]}`;
      return row;
    }),
  ];
}

/** What happened to a permission that was on screen, shown for a moment before what comes next. */
export type Settled = "allow" | "deny" | "terminal" | "expired";

const SETTLED: Record<Settled, { label: string; wash: Wash; perch: string }> = {
  allow: { label: "Allowed", wash: "green", perch: "finished" },
  deny: { label: "Denied", wash: "red", perch: "failed" },
  terminal: { label: "Answered in the terminal", wash: null, perch: "idle" },
  expired: { label: "Nobody answered: the terminal asks now", wash: "amber", perch: "approval" },
};

export function settledCard(s: SessionView, how: Settled, target: string, perch: HTMLElement): HTMLElement {
  const look = SETTLED[how];
  perch.className = `perch ${look.perch}`;
  return el(
    "section",
    { class: `card focus settled${look.wash ? ` wash ${look.wash}` : ""}` },
    perch,
    el(
      "div",
      { class: "focus-body" },
      el(
        "div",
        { class: `settled-label ${how}` },
        how === "allow" ? icon("check", 13, 2.6) : how === "deny" ? icon("close", 13, 2.6) : null,
        el("span", { text: look.label }),
      ),
      el("div", { class: "who" }, el("span", { class: `dot ${s.agent}` }), el("span", { class: "name", text: s.project || AGENT_NAME[s.agent] })),
      el("pre", { class: "code dim", text: target }),
    ),
  );
}
