// The Nest's records (plan-zeca W11): History (each finished turn, counts only), Usage (the plans'
// windows and what the agents cost), Traces (Zeca's own turns and tools) and Audit (who answered each
// card, and how). Each only reads what this computer keeps; nothing here changes anything.
import type {
  AuditRow,
  HistoryView,
  TraceRow,
  TurnRow,
  UsageView,
} from "../../bridge";
import { NAME_BY_ID } from "../../agents";
import { el } from "../../dom";
import { locale, t, tk } from "../../i18n";
import { duration } from "../settings/activity";
import { money } from "./budget";

export interface RecordsBackend {
  history(): Promise<HistoryView>;
  usage(): Promise<UsageView>;
  traces(): Promise<TraceRow[]>;
  audit(): Promise<AuditRow[]>;
}

const AGENT: Record<string, string> = { ...NAME_BY_ID, other: tk("Another tool") };

const agentName = (a: string) => (a in AGENT ? t(AGENT[a]) : a);

/** Who answered a card. */
const ACTOR: Record<string, string> = {
  human: tk("You"),
  phone: tk("Your phone"),
  rule: tk("An Always rule"),
  policy: tk("A policy"),
  "full-access": tk("Full access"),
  system: tk("Vults"),
};

/** What the answer did. */
const ACT: Record<string, string> = {
  allow: tk("Allowed"),
  deny: tk("Denied"),
  "always-allow": tk("Allowed, and saved as a rule"),
  "allow-in-conversation": tk("Allowed for the conversation"),
  answer: tk("Answered"),
  release: tk("Sent to the terminal"),
  expire: tk("Nobody answered in time"),
  "config-install": tk("Hooks installed"),
  "config-remove": tk("Hooks removed"),
  "audit-cleared": tk("The log was cleared"),
};

/** How a chat turn of Zeca's traces ended. */
const STATUS: Record<string, string> = {
  ok: tk("Done"),
  error: tk("Failed"),
  stopped: tk("Stopped"),
  denied: tk("Denied"),
};

/** How a tool card inside a turn was answered. */
const CARD: Record<string, string> = {
  ok: tk("Allowed"),
  denied: tk("Denied"),
  error: tk("Failed"),
  stopped: tk("Stopped"),
};

/** A span's length: milliseconds, seconds, then minutes. */
function took(ms: number): string {
  if (ms < 1000) return t("{ms} ms", { ms });
  if (ms < 60_000) return t("{s} s", { s: Math.round(ms / 1000) });
  return duration(Math.round(ms / 1000));
}

const when = (ms: number) =>
  new Date(ms).toLocaleString(locale(), {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });

const tokens = (n: number) =>
  n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n.toLocaleString("en-US");

/** A plan window's length, in words. */
function windowName(minutes: number): string {
  if (minutes === 300) return t("5 hours");
  if (minutes === 10080) return t("A week");
  if (minutes === 1440) return t("1 day");
  if (minutes === 60) return t("1 hour");
  if (minutes % 1440 === 0) return t("{n} days", { n: minutes / 1440 });
  if (minutes % 60 === 0) return t("{n} hours", { n: minutes / 60 });
  return t("{n} minutes", { n: minutes });
}

function line(parts: (HTMLElement | null)[], cls = ""): HTMLElement {
  return el(
    "div",
    { class: `record ${cls}`.trim() },
    ...parts.filter((p): p is HTMLElement => p !== null),
  );
}

const span = (text: string, cls: string) => el("span", { class: cls, text });

const clamp = (percent: number) => `${Math.max(0, Math.min(100, Math.round(percent)))}%`;

/** A bar filled to `percent`, with `cls` on its fill, and a tick at `mark` (the budget) if any. */
function bar(percent: number, cls = "", mark: number | null = null): HTMLElement {
  const fill = el("span", { class: `usage-fill ${cls}`.trim() });
  fill.style.width = clamp(percent);
  const tick = mark === null ? null : el("span", { class: "usage-mark" });
  if (tick && mark !== null) tick.style.left = clamp(mark);
  return el("span", { class: "usage-bar" }, fill, tick);
}

function turnLine(r: TurnRow): HTMLElement {
  const counts = [
    r.steps === 1 ? t("1 step") : t("{n} steps", { n: r.steps }),
    r.commands
      ? r.commands === 1
        ? t("1 command")
        : t("{n} commands", { n: r.commands })
      : null,
    r.files ? `+${r.added} −${r.removed}` : null,
  ]
    .filter(Boolean)
    .join(" · ");
  return line([
    span(when(r.end * 1000), "record-when"),
    span(agentName(r.agent), "record-who"),
    span(r.project || t("No project"), "record-what"),
    span(duration(r.secs), "record-meta"),
    span(counts, "record-meta"),
    r.failed ? span(t("Failed"), "row-tag failing") : null,
  ]);
}

function auditLine(r: AuditRow): HTMLElement {
  const act = r.act in ACT ? t(ACT[r.act]) : r.act;
  const positive =
    r.act === "allow" ||
    r.act === "always-allow" ||
    r.act === "allow-in-conversation";
  return line([
    span(when(r.at * 1000), "record-when"),
    span(r.actor in ACTOR ? t(ACTOR[r.actor]) : r.actor, "record-who"),
    span(
      act,
      `row-tag ${positive ? "passing" : r.act === "deny" ? "failing" : ""}`,
    ),
    span(
      [agentName(r.agent), r.project].filter(Boolean).join(" · "),
      "record-meta",
    ),
    el("code", {
      class: "record-target",
      text: [r.tool, r.target].filter(Boolean).join(" · "),
    }),
    r.consent !== null
      ? span(t("Policy accepted as #{id}", { id: r.consent }), "record-meta")
      : null,
  ]);
}

function traceLine(r: TraceRow): HTMLElement {
  // A tool card was allowed or denied; a chat turn was done, failed or stopped.
  const words = r.kind === "execute_tool" ? CARD : STATUS;
  const used =
    r.input !== null || r.output !== null
      ? t("{input} in, {output} out", {
          input: tokens(r.input ?? 0),
          output: tokens(r.output ?? 0),
        })
      : null;
  return line(
    [
      span(when(r.startMs), "record-when"),
      el("code", { class: "record-what", text: r.name }),
      span(
        r.status in words ? t(words[r.status]) : r.status,
        `row-tag ${r.status === "ok" ? "passing" : r.status === "error" || r.status === "denied" ? "failing" : ""}`,
      ),
      span(took(r.ms), "record-meta"),
      used ? span(used, "record-meta") : null,
      r.costUsd !== null ? span(money(r.costUsd), "record-meta") : null,
    ],
    r.child ? "child" : "",
  );
}

function usageBody(u: UsageView): HTMLElement[] {
  const plans = u.windows.length
    ? u.windows.map((w) =>
        line([
          span(agentName(w.agent), "record-who"),
          span(windowName(w.minutes), "record-what"),
          bar(w.used_percent),
          span(`${w.used_percent}%`, "record-meta"),
        ]),
      )
    : [
        el("p", {
          class: "board-empty",
          text: t(
            "No plan has told its usage yet: Claude Code's comes with its status line, Codex's with its meter in Settings.",
          ),
        }),
      ];
  const top = Math.max(...u.days.map((d) => d.usd), u.budget ?? 0, 0.01);
  const days = u.days.length
    ? u.days.map((d) =>
        line([
          span(
            new Date(`${d.day}T12:00:00`).toLocaleDateString(locale(), {
              weekday: "short",
              month: "short",
              day: "numeric",
            }),
            "record-when",
          ),
          bar(
            (d.usd / top) * 100,
            u.budget !== null && d.usd >= u.budget ? "over" : "",
            u.budget !== null ? (u.budget / top) * 100 : null,
          ),
          span(money(d.usd), "record-meta"),
          d.tokens
            ? span(
                t("{tokens} tokens", { tokens: tokens(d.tokens) }),
                "record-meta",
              )
            : null,
        ]),
      )
    : [
        el("p", {
          class: "board-empty",
          text: t("Nothing spent in the last two weeks."),
        }),
      ];
  const roosts = u.roosts.length
    ? u.roosts.map((r) =>
        line([
          span(r.name, "record-who"),
          span(
            t("Cost today {today}, {total} in all", {
              today: money(r.today),
              total: money(r.total),
            }),
            "record-meta",
          ),
        ]),
      )
    : [
        el("p", {
          class: "board-empty",
          text: t("No roost has spent anything yet."),
        }),
      ];
  return [
    el("h4", { class: "records-head", text: t("Your plans") }),
    ...plans,
    el("h4", {
      class: "records-head",
      text:
        u.budget !== null
          ? t("The last two weeks, against your {budget} a day", {
              budget: money(u.budget),
            })
          : t("The last two weeks"),
    }),
    ...days,
    u.subscription
      ? el("p", {
          class: "board-empty",
          text: t(
            "On a subscription this is at API prices: your plan does not charge it.",
          ),
        })
      : null,
    el("h4", { class: "records-head", text: t("By roost") }),
    ...roosts,
  ].filter((p): p is HTMLElement => p !== null);
}

/** One tab: what `load` brings, drawn by `draw`; asked again each time the tab opens. */
function tab<T>(
  cls: string,
  load: () => Promise<T>,
  draw: (v: T) => HTMLElement[],
): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: `records ${cls}` });
  let asked = 0;
  const refresh = () => {
    const mine = ++asked;
    void load().then(
      (v) => {
        if (mine === asked) element.replaceChildren(...draw(v));
      },
      (e) => {
        if (mine === asked)
          element.replaceChildren(
            el("p", { class: "board-empty", text: String(e) }),
          );
      },
    );
  };
  return { element, refresh };
}

export function createHistory(load: () => Promise<HistoryView>) {
  return tab("history", load, (h) => [
    el("p", {
      class: "board-empty",
      text: h.on
        ? t(
            "Each finished turn of your agents, counted on this computer. The week's recap and the year's grid are in Settings, Activity.",
          )
        : t(
            "Settings, Activity keeps no history now: turn it on there to see your agents' turns here.",
          ),
    }),
    ...(h.turns.length
      ? h.turns.map(turnLine)
      : [
          el("p", {
            class: "board-empty",
            text: t("No finished turn is kept yet."),
          }),
        ]),
  ]);
}

export function createUsage(load: () => Promise<UsageView>) {
  return tab("usage", load, usageBody);
}

export function createTraces(load: () => Promise<TraceRow[]>) {
  return tab("traces", load, (rows) => [
    el("p", {
      class: "board-empty",
      text: t(
        "Zeca's own work, kept 30 days: each chat turn with its model, tokens and cost, and under it each tool card, with how long it waited for an answer.",
      ),
    }),
    ...(rows.length
      ? rows.map(traceLine)
      : [el("p", { class: "board-empty", text: t("No traces yet.") })]),
  ]);
}

export function createAudit(load: () => Promise<AuditRow[]>) {
  return tab("audit", load, (rows) => [
    el("p", {
      class: "board-empty",
      text: t(
        "Who answered each card, and how: you, your phone, an Always rule or a policy you accepted. Kept 90 days, never changed.",
      ),
    }),
    ...(rows.length
      ? rows.map(auditLine)
      : [
          el("p", {
            class: "board-empty",
            text: t("No card was answered yet."),
          }),
        ]),
  ]);
}
