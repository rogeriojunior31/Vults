// The card for the session in front, by state. Each state has its own wash, wording and
// actions; working sessions show the step ticker.
import type { ApprovalView, SessionView } from "../bridge";
import { el } from "../dom";
import { icon } from "./icons";
import type { Ticker } from "./ticker";

export const AGENT_NAME = { claude: "Claude Code", codex: "Codex" } as const;

export interface CardActions {
  /** Keys bound for "allow" and "deny", shown on the buttons. */
  keys: Record<string, string>;
  decide(request: string, decision: "allow" | "deny"): void;
  jump(agent: SessionView["agent"], id: string): void;
  dismiss(): void;
}

type Wash = "amber" | "green" | "red" | "cyan" | null;

function card(wash: Wash, ...children: (Node | null)[]): HTMLElement {
  return el("section", { class: wash ? `card wash ${wash}` : "card" }, ...children);
}

function who(s: SessionView, label: string, extra: Node | null = null): HTMLElement {
  return el(
    "div",
    { class: "who" },
    el("span", { class: `dot ${s.agent}` }),
    el("span", { class: "name", text: s.project || AGENT_NAME[s.agent] }),
    el("span", { class: "label", text: label }),
    extra,
  );
}

function button(text: string, kind: "primary" | "secondary", onclick: () => void, glyph: Node | null = null) {
  return el("button", { class: `btn ${kind}`, onclick }, el("span", { text }), glyph);
}

function kbd(keys: string | undefined): Node | null {
  return keys ? el("span", { class: "kbd", text: keys }) : null;
}

function jumpButton(s: SessionView, actions: CardActions, kind: "primary" | "secondary" = "secondary") {
  return button("Open terminal", kind, () => actions.jump(s.agent, s.id), icon("openOut", 12));
}

export function sessionCard(
  s: SessionView,
  approval: ApprovalView | null,
  ticker: Ticker,
  actions: CardActions,
): HTMLElement {
  const last = s.step ?? "";
  switch (s.status) {
    case "approval": {
      if (!approval) break;
      let sent = false;
      const answer = (d: "allow" | "deny") => () => {
        if (sent) return;
        sent = true;
        actions.decide(approval.request, d);
      };
      return card(
        "amber",
        who(s, "needs your permission"),
        el("pre", { class: "code", text: approval.target }),
        el(
          "div",
          { class: "actions" },
          button("Deny", "secondary", answer("deny"), kbd(actions.keys.deny)),
          button("Allow", "primary", answer("allow"), kbd(actions.keys.allow)),
        ),
      );
    }
    case "question":
      return card(
        "cyan",
        who(s, "has a question"),
        el("div", { class: "title", text: "Waiting for your answer in the terminal." }),
        el("div", { class: "actions" }, jumpButton(s, actions, "primary")),
      );
    case "finished":
      return card(
        "green",
        who(s, `${AGENT_NAME[s.agent]} finished`),
        el("div", { class: "title", text: last || "Session finished." }),
        el("div", { class: "actions" }, button("OK", "secondary", () => actions.dismiss()), jumpButton(s, actions, "primary")),
      );
    case "failed":
      return card(
        "red",
        who(s, "stopped on an error"),
        el("div", { class: "title", text: last || "No detail available." }),
        el("div", { class: "actions" }, jumpButton(s, actions, "primary")),
      );
    case "ratelimited":
      return card(
        "amber",
        who(s, "hit a usage limit"),
        el("div", { class: "title", text: "Paused until the limit resets." }),
        el("div", { class: "actions" }, jumpButton(s, actions)),
      );
    default:
      break;
  }
  ticker.sync(`${s.agent}:${s.id}`, s.steps);
  const count = s.step_count > 0 ? el("span", { class: "count", text: `${s.step_count} steps` }) : null;
  const open = el("button", { class: "icon-btn", onclick: () => actions.jump(s.agent, s.id) }, icon("openOut", 13));
  open.title = "Open terminal";
  return card(
    null,
    el("div", { class: "card-head" }, who(s, AGENT_NAME[s.agent], count), open),
    s.steps.length ? ticker.element : el("div", { class: "title dim", text: s.status === "thinking" ? "Thinking…" : "Waiting for a prompt." }),
  );
}
