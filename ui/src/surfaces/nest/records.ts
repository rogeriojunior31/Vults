// The Nest's records (plan-zeca W11): History (each finished turn, counts only), Usage (the plans'
// windows and what the agents cost), Traces (Zeca's own turns and tools) and Audit (who answered each
// card, and how). Each only reads what this computer keeps; nothing here changes the records.
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
import { money, usageLabel, usageLevel } from "../../cost";
import { viewState } from "./view-state";

export interface RecordsBackend {
  history(): Promise<HistoryView>;
  usage(): Promise<UsageView>;
  traces(): Promise<TraceRow[]>;
  audit(): Promise<AuditRow[]>;
  query?: (tab: "history" | "audit", query?: string) => Promise<string>;
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

function heading(title: string, about: string): HTMLElement {
  return el("header", { class: "activity-heading" }, el("div", {}, el("h2", { text: title }), el("p", { text: about })));
}

function summary(...values: [string, string][]): HTMLElement {
  return el("div", { class: "activity-summary" },
    span(t("Summary of loaded records"), "activity-summary-label"),
    ...values.map(([value, label]) =>
    el("div", { class: "activity-stat" }, el("strong", { text: value }), span(label, "")),
  ));
}

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
    el("div", { class: "record-body" },
      el("div", { class: "record-title" }, span(r.project || t("No project"), "record-what"),
        span(r.failed ? t("Failed") : t("Done"), `record-status ${r.failed ? "failing" : "passing"}`)),
      el("div", { class: "record-context" }, span(agentName(r.agent), "record-who"), span(counts, "record-meta")),
    ),
    el("div", { class: "record-aside" }, span(duration(r.secs), "record-duration"), span(when(r.end * 1000), "record-when")),
  ]);
}

function auditLine(r: AuditRow): HTMLElement {
  const act = r.act in ACT ? t(ACT[r.act]) : r.act;
  const positive =
    r.act === "allow" ||
    r.act === "always-allow" ||
    r.act === "allow-in-conversation";
  return line([
    el("div", { class: "record-body" },
      el("div", { class: "record-title" }, span(r.actor in ACTOR ? t(ACTOR[r.actor]) : r.actor, "record-who"),
        span(act, `record-status ${positive ? "passing" : r.act === "deny" ? "failing" : ""}`)),
      span([agentName(r.agent), r.project].filter(Boolean).join(" · "), "record-meta"),
    ),
    span(when(r.at * 1000), "record-when"),
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
      el("div", { class: "record-body" },
        el("div", { class: "record-title" },
          el("code", { class: "record-what", text: r.name }),
          span(r.status in words ? t(words[r.status]) : r.status,
            `record-status ${r.status === "ok" ? "passing" : r.status === "error" || r.status === "denied" ? "failing" : ""}`),
        ),
        el("div", { class: "record-context" },
          span(r.kind === "execute_tool" ? t("Tool call") : t("Chat turn"), "record-meta"),
          used ? span(used, "record-meta") : null,
          r.costUsd !== null ? span(money(r.costUsd), "record-meta") : null,
        ),
      ),
      el("div", { class: "record-aside" }, span(took(r.ms), "record-duration"), span(when(r.startMs), "record-when")),
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
          bar(w.used_percent, usageLevel(w.used_percent)),
          span(`${w.used_percent}%`, "record-meta"),
          span(usageLabel(w.used_percent), `usage-status ${usageLevel(w.used_percent)}`),
        ]),
      )
    : [
        viewState("empty", t("No plan usage yet"), t(
            "No plan has told its usage yet: Claude Code's comes with its status line, Codex's with its meter in Settings.",
          )),
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
        viewState("empty", t("No costs recorded in the last two weeks.")),
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
        viewState("empty", t("No project costs recorded yet.")),
      ];
  return [
    summary(
      [money(u.days.reduce((n, day) => n + day.usd, 0)), t("Recorded cost · last two weeks")],
      [tokens(u.days.reduce((n, day) => n + day.tokens, 0)), t("Tokens · last two weeks")],
      [String(u.windows.length), t("Plan windows")],
    ),
    el("section", { class: "usage-section" },
      el("h4", { class: "records-head", text: t("Your plans") }),
      ...plans,
    ),
    el("section", { class: "usage-section" },
      el("h4", {
        class: "records-head",
        text:
          u.budget !== null
            ? t("The last two weeks, against your {budget} a day", { budget: money(u.budget) })
            : t("The last two weeks"),
      }),
      ...days,
      u.subscription
        ? el("p", {
            class: "board-empty",
            text: t("On a subscription this is at API prices: your plan does not charge it."),
          })
        : null,
    ),
    el("section", { class: "usage-section" },
      el("h4", { class: "records-head", text: t("By roost") }),
      ...roosts,
    ),
  ];
}

/** One tab: what `load` brings, drawn by `draw`; asked again each time the tab opens. */
function tab<T>(
  cls: string,
  load: () => Promise<T>,
  draw: (v: T, query: string) => HTMLElement[],
  searchable = false,
  query?: RecordsBackend["query"],
): { element: HTMLElement; refresh: () => void; flush: () => Promise<void> } {
  const element = el("div", { class: `records ${cls}` });
  const content = el("div", { class: "records-content" });
  const status = el("div", { class: "records-state" });
  const search = document.createElement("input");
  search.type = "search";
  search.maxLength = 4096;
  search.className = "field";
  search.setAttribute("aria-label", t("Search records"));
  search.placeholder = t("Search records");
  let loaded: T | undefined;
  let loading = false;
  let failed = false;
  let failure: string | null = null;
  const names: Record<string, string> = { history: tk("History"), usage: tk("Cost"), traces: tk("Traces"), audit: tk("Audit") };
  const retry = el("button", { onclick: () => { if (!loading) refresh(); } });
  element.tabIndex = -1;
  const paint = () => {
    element.setAttribute("aria-label", t(names[cls]));
    const returning = document.activeElement === retry;
    if (loaded !== undefined) content.replaceChildren(...draw(loaded, search.value.trim().toLocaleLowerCase(locale())));
    element.setAttribute("aria-busy", String(loading));
    status.replaceChildren();
    if (failed) {
      retry.textContent = loading ? t("Loading") : t("Try again");
      retry.setAttribute("aria-busy", String(loading));
      status.append(viewState("error", loaded !== undefined ? t("Could not update {name}.", { name: t(names[cls]) }) : t("Could not load {name}.", { name: t(names[cls]) }),
        loaded !== undefined ? t("The last loaded view is kept and may be out of date.") : t("Try again to read the data saved on this computer."), retry, failure));
    } else if (loading && loaded === undefined) status.append(viewState("loading", t("Loading {name}…", { name: t(names[cls]) })));
    if (returning) (failed ? retry : searchable ? search : element).focus({ preventScroll: true });
  };
  let edited = false;
  let saved: Promise<void> = Promise.resolve();
  const restored = query ? query(cls as "history" | "audit").then(value => {
    if (!edited) {
      search.value = value;
      paint();
    }
  }).catch(() => {}) : Promise.resolve();
  const change = () => {
    edited = true;
    paint();
    const value = search.value;
    if (query) saved = saved.catch(() => {}).then(() => query(cls as "history" | "audit", value)).then(() => {});
    void saved.catch(() => {});
  };
  search.addEventListener("input", change);
  const clear = el("button", { class: "btn", text: t("Clear filters"), onclick: () => {
    search.value = "";
    change();
    search.focus();
  } });
  if (searchable) element.append(el("div", { class: "records-search" }, search, clear));
  element.append(status, content);
  let asked = 0;
  const refresh = () => {
    search.setAttribute("aria-label", t("Search records"));
    search.placeholder = t("Search records");
    clear.textContent = t("Clear filters");
    const mine = ++asked;
    loading = true;
    paint();
    void load().then(
      (v) => {
        if (mine === asked) {
          loaded = v;
          failed = false;
          loading = false;
          paint();
        }
      },
      (e) => {
        if (mine === asked) {
          failed = true;
          loading = false;
          failure = String(e);
          paint();
        }
      },
    );
  };
  return { element, refresh, flush: async () => {
    await restored;
    // Retry a failed save without requiring the user to change their search first.
    saved = saved.catch(async () => { if (query) await query(cls as "history" | "audit", search.value); });
    await saved;
  } };
}

function groupedRows<T extends { project: string }>(rows: T[], draw: (r: T) => HTMLElement, query: string): HTMLElement[] {
  const groups = new Map<string, HTMLElement[]>();
  for (const row of rows) {
    const node = draw(row);
    if (query && !node.textContent?.toLocaleLowerCase(locale()).includes(query)) continue;
    const project = row.project || t("No project");
    const entries = groups.get(project) ?? [];
    entries.push(node);
    groups.set(project, entries);
  }
  return groups.size
    ? [...groups].map(([project, entries]) => el("section", { class: "record-group" },
        el("h4", { class: "records-head" }, span(project, ""), span(entries.length === 1 ? t("1 record") : t("{n} records", { n: entries.length }), "record-count")), ...entries))
    : [viewState("empty", t("No results"), t("No records match. Clear filters to see all records."))];
}

export function createHistory(load: () => Promise<HistoryView>, query?: RecordsBackend["query"]) {
  return tab("history", load, (h, query) => {
    return [
      heading(t("History"), h.on
          ? t(
              "Each finished turn of your agents, counted on this computer. The week's recap and the year's grid are in Settings, Activity.",
            )
          : t(
              "Settings, Activity keeps no history now: turn it on there to see your agents' turns here.",
            )),
      summary(
        [String(h.turns.length), t("Kept turns")],
        [String(h.turns.reduce((n, turn) => n + turn.commands, 0)), t("Recorded commands")],
        [String(h.turns.filter(turn => turn.failed).length), t("Failed turns")],
      ),
      ...(h.turns.length
        ? groupedRows(h.turns, turnLine, query)
        : [
            viewState("empty", h.on ? t("No finished turn is kept yet.") : t("History is off"), h.on ? t("Finished agent turns will appear here.") : undefined),
          ]),
    ];
  }, true, query);
}

export function createUsage(load: () => Promise<UsageView>) {
  return tab("usage", load, usageBody);
}

export function createTraces(load: () => Promise<TraceRow[]>) {
  return tab("traces", load, (rows) => [
    heading(t("Traces"), t(
        "Zeca's own work, kept 30 days: each chat turn with its model, tokens and cost, and under it each tool card, with how long it waited for an answer.",
      )),
    summary(
      [String(rows.filter(row => row.kind !== "execute_tool").length), t("Chat turns")],
      [String(rows.filter(row => row.kind === "execute_tool").length), t("Tool calls")],
      [String(rows.filter(row => row.status === "error" || row.status === "denied").length), t("Failed or denied")],
    ),
    ...(rows.length
      ? rows.map(traceLine)
      : [viewState("empty", t("No traces yet."), t("Chat turns and tool calls will appear here after Zeca works."))]),
  ]);
}

export function createAudit(load: () => Promise<AuditRow[]>, query?: RecordsBackend["query"]) {
  return tab("audit", load, (rows, query) => {
    return [
      heading(t("Audit"), t(
          "Who answered each card, and how: you, your phone, an Always rule or a policy you accepted. Kept 90 days, never changed.",
        )),
      summary(
        [String(rows.length), t("Audit entries")],
        [String(rows.filter(row => ["allow", "always-allow", "allow-in-conversation"].includes(row.act)).length), t("Allowed")],
        [String(rows.filter(row => row.act === "deny").length), t("Denied")],
      ),
      ...(rows.length
        ? groupedRows(rows, auditLine, query)
        : [
            viewState("empty", t("No card was answered yet."), t("Permission answers will appear here. Opening this view answers nothing.")),
          ]),
    ];
  }, true, query);
}
