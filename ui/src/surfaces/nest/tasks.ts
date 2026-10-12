// The Nest's Tasks tab (plan-zeca W3): what the user wants done, with the criteria that say it is,
// its roost, who works on it, where it stands and what it cost. `/task` in the chat makes one in a
// single message; **New task** here does too. With the GitHub connector on, a task in a GitHub
// repository links to an issue, or opens one on a click.
import type { Engine, OtherAgent, SpendToday, TaskEdit, TaskRow } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";
import { money, spentLine } from "./budget";

export interface TasksBackend {
  /** Zeca is on: `/task` in the chat is a way in too. */
  zeca: () => boolean;
  load(closed: boolean): Promise<TaskRow[]>;
  create(goal: string, criteria: string[], folder: string | null): Promise<number>;
  edit(id: number, edit: TaskEdit): Promise<void>;
  remove(id: number): Promise<void>;
  openIssue(id: number): Promise<string>;
  /** Opens the task's linked issue in the browser. */
  visit(id: number): Promise<void>;
  /** The folders the flock works in now, for a new task's roost. */
  folders(): { cwd: string; project: string }[];
  /** The chosen agent in the task's clone (W4); stopping it; removing its folder. */
  start(id: number, engine: Engine): Promise<void>;
  /** A handoff to a new session through a context pack (W5), on the same agent or the other. */
  startFresh(id: number, engine: Engine): Promise<void>;
  stop(id: number): Promise<void>;
  /** The other agent reviews the work, and the session fixes what it finds (W6). */
  review(id: number): Promise<void>;
  removeWorktree(id: number): Promise<void>;
  /** The agents that speak ACP, installed here (W10): **Start with** offers them too. */
  agents(): OtherAgent[];
  /** Today's spend and the daily budget (W9), for its line at the top. */
  today(): Promise<SpendToday>;
}

/** Why the router chose its agent (W8), as the card says it. */
function routed(route: string): string | null {
  const [why, agent, a, b] = route.split(":");
  const name = OWNER[agent] ?? agent;
  switch (why) {
    case "rule":
      return t("Delegated to {agent}: this project's rule.", { agent: name });
    case "default":
      return t("Delegated to {agent}: no rule for this project yet.", { agent: name });
    case "quota":
      return t("Delegated to {agent}: {busy} is at {used}% of its plan.", { agent: name, busy: OWNER[a] ?? a, used: b });
    case "score":
      return t("Delegated to {agent}: it finished {done} of {of} tasks like this here.", { agent: name, done: a, of: b });
    default:
      return null;
  }
}

/** A review's verdict, as its summary line says it. */
const VERDICT: Record<string, string> = {
  pass: tk("Review passed (round {round})"),
  changes: tk("Review asked for changes (round {round})"),
  none: tk("Review gave no verdict (round {round}): read it"),
};

const STATUS: Record<string, string> = {
  open: tk("To do"),
  doing: tk("Doing"),
  blocked: tk("Blocked"),
  done: tk("Done"),
  dropped: tk("Dropped"),
};

/** Who works on it: the names as they are; only "You" is translated. */
const OWNER: Record<string, string> = {
  you: tk("You"),
  zeca: "Zeca",
  claude: "Claude Code",
  codex: "Codex",
  gemini: "Gemini CLI",
  opencode: "OpenCode",
  qwen: "Qwen Code",
  antigravity: "Antigravity",
  copilot: "Copilot CLI",
};

/** The errors the tasks' commands give back in English, translated here. */
const ERRORS = new Set([
  tk("No such task."),
  tk("No such owner."),
  tk("That is not a GitHub issue."),
  tk("Turn on the GitHub connector in Settings first."),
  tk("This task already has an issue."),
  tk("This task is not in a GitHub repository."),
  tk("This task's issue is being opened."),
  tk("This task's issue can't be opened."),
  tk("This task is already running."),
  tk("This task is finished."),
  tk("This task is not in a git repository."),
  tk("That agent can't run here."),
  tk("This task isn't running."),
  tk("Stop the task first."),
  tk("This task's folder is in the way: remove it first."),
  tk("This task's folder was changed in a way Vults won't trust: remove its folder."),
  tk("The task's branch didn't come back into the repository."),
  tk("This task has no work to review yet."),
  tk("This task is still starting."),
  tk("This task has no session to hand off."),
  tk("The session wrote no handoff notes: it keeps its session."),
  tk("Today's budget is spent: Vults starts no new session until tomorrow."),
  tk("That budget is too high."),
]);

/** `options` maps each value to its label, already in the user's language. */
function select(options: Record<string, string>, value: string, change: (v: string) => void): HTMLSelectElement {
  const s = el("select", { class: "task-select" });
  // A value the list doesn't know is still shown as it is, never as another choice.
  const all = value in options ? options : { ...options, [value]: value };
  for (const [v, label] of Object.entries(all)) {
    const o = el("option", { text: label });
    o.value = v;
    o.selected = v === value;
    s.append(o);
  }
  s.addEventListener("change", () => change(s.value));
  return s;
}

function translated(labels: Record<string, string>): Record<string, string> {
  return Object.fromEntries(Object.entries(labels).map(([k, v]) => [k, t(v)]));
}

/** Tokens as a card says them: 1,234 or 1.2M. */
function tokens(n: number): string {
  return n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n.toLocaleString("en-US");
}

/** The review loop's most rounds (W6). */
const MAX_ROUNDS = 3;

/** Open tasks under their roost (by its key: two repositories may share a name), the roosts in
 *  the order of their names so a group stays put while its tasks change, the ones with none last. */
function byRoost(open: TaskRow[]): { roost: string; tasks: TaskRow[] }[] {
  const groups = new Map<string, { roost: string; tasks: TaskRow[] }>();
  for (const task of open) {
    const g = groups.get(task.roostKey) ?? { roost: task.roost, tasks: [] };
    g.tasks.push(task);
    groups.set(task.roostKey, g);
  }
  return [...groups.values()].sort((a, b) => Number(!a.roost) - Number(!b.roost) || a.roost.localeCompare(b.roost));
}

export function createTasks(backend: TasksBackend): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: "tasks" });
  let closed = false;
  let message: string | null = null;
  let asked = 0;
  /** The New task form's text, kept while the list is drawn again; null while it is closed. */
  let draft: { goal: string; criteria: string; where: string | null } | null = null;
  /** The tasks whose issue is being opened, and the one whose Delete waits for a second click. */
  const opening = new Set<number>();
  let deleting: number | null = null;
  let removing: number | null = null;
  /** What the user typed to link an issue, by task. */
  const linking = new Map<number, string>();

  const said = (e: unknown): string => {
    const text = String(e);
    return ERRORS.has(text) ? t(text) : text;
  };
  const act = (p: Promise<unknown>) =>
    void p.then(
      () => refresh(),
      (e) => {
        message = said(e);
        refresh();
      },
    );

  const form = (d: { goal: string; criteria: string; where: string | null }): HTMLElement => {
    const goal = el("input", { class: "task-goal-input" });
    goal.placeholder = t("What should be done?");
    goal.value = d.goal;
    goal.addEventListener("input", () => (d.goal = goal.value));
    const criteria = el("textarea", { class: "task-criteria-input" });
    criteria.placeholder = t("Done when… (one per line)");
    criteria.rows = 3;
    criteria.value = d.criteria;
    criteria.addEventListener("input", () => (d.criteria = criteria.value));
    const where = el("select", { class: "task-select" });
    const none = el("option", { text: t("No project") });
    none.value = "";
    where.append(none);
    const seen = new Set<string>();
    for (const f of backend.folders()) {
      if (seen.has(f.cwd)) continue;
      seen.add(f.cwd);
      const o = el("option", { text: f.project });
      o.value = f.cwd;
      where.append(o);
    }
    where.value = d.where ?? "";
    where.addEventListener("change", () => (d.where = where.value || null));
    return el(
      "section",
      { class: "task-new" },
      goal,
      criteria,
      el(
        "div",
        { class: "proposal-actions" },
        where,
        el("button", {
          class: "btn small primary",
          text: t("Add"),
          onclick: () => {
            if (!d.goal.trim()) return;
            const lines = d.criteria
              .split("\n")
              .map((l) => l.replace(/^\s*[-*]\s+/, "").trim())
              .filter(Boolean);
            draft = null;
            act(backend.create(d.goal, lines, d.where));
          },
        }),
        el("button", {
          class: "btn small",
          text: t("Cancel"),
          onclick: () => {
            draft = null;
            refresh();
          },
        }),
      ),
    );
  };

  /** The task's issue: open it, unlink it, or link one (`#12`, an address), or open one. */
  const issue = (task: TaskRow): HTMLElement[] => {
    if (task.issue) {
      return [
        el("button", { class: "btn small", text: t("Issue"), onclick: () => act(backend.visit(task.id)) }),
        el("button", {
          class: "btn small",
          text: t("Unlink"),
          onclick: () => act(backend.edit(task.id, { kind: "issue", issue: "" })),
        }),
      ];
    }
    const typed = el("input", { class: "task-issue-input" });
    typed.placeholder = t("#12 or an issue's address");
    typed.value = linking.get(task.id) ?? "";
    typed.addEventListener("input", () => linking.set(task.id, typed.value));
    const link = el("button", {
      class: "btn small",
      text: t("Link"),
      onclick: () => {
        if (!typed.value.trim()) return;
        linking.delete(task.id);
        act(backend.edit(task.id, { kind: "issue", issue: typed.value }));
      },
    });
    const parts: HTMLElement[] = [typed, link];
    if (task.canOpenIssue) {
      const busy = opening.has(task.id);
      const open = el("button", {
        class: "btn small",
        text: busy ? t("Opening…") : t("Open an issue"),
        onclick: () => {
          if (opening.has(task.id)) return;
          opening.add(task.id);
          refresh();
          act(backend.openIssue(task.id).finally(() => opening.delete(task.id)));
        },
      });
      open.disabled = busy;
      parts.push(open);
    }
    return parts;
  };

  /** Start, Stop and the worktree: only sessions Vults started can be stopped here. A session
   *  belongs to the agent that owns the task; a handoff may go to the other one. */
  const run = (task: TaskRow): HTMLElement[] => {
    const parts: HTMLElement[] = [];
    const button = (text: string, act_: () => Promise<void>, primary = false) =>
      el("button", { class: `btn small${primary ? " primary" : ""}`, text, onclick: () => act(act_()) });
    if (task.running) {
      parts.push(button(t("Stop"), () => backend.stop(task.id)));
    } else if (task.canStart && task.hasSession) {
      // A session goes on with the agent it ran on, whatever the owner select says.
      const mine: Engine = task.sessionEngine ?? "claude";
      const other: Engine = mine === "codex" ? "claude" : "codex";
      parts.push(button(t("Go on"), () => backend.start(task.id, mine), true));
      // A handoff: the session writes its pack, and a new one starts from it alone.
      parts.push(button(t("Start fresh"), () => backend.startFresh(task.id, mine)));
      parts.push(button(other === "codex" ? t("Hand off to Codex") : t("Hand off to Claude Code"), () => backend.startFresh(task.id, other)));
      // The other agent reviews it: it wrote none of it.
      parts.push(button(other === "codex" ? t("Review with Codex") : t("Review with Claude Code"), () => backend.review(task.id)));
    } else if (task.canStart) {
      parts.push(button(t("Start with Claude Code"), () => backend.start(task.id, "claude"), true));
      parts.push(button(t("Start with Codex"), () => backend.start(task.id, "codex")));
      const others = backend.agents();
      if (others.length) {
        const pick = el("select", { class: "task-select" });
        const head = el("option", { text: t("Start with another agent…") });
        head.value = "";
        pick.append(head);
        for (const a of others) {
          const o = el("option", { text: a.title });
          o.value = a.name;
          pick.append(o);
        }
        pick.addEventListener("change", () => {
          if (pick.value) act(backend.start(task.id, pick.value as Engine));
        });
        parts.push(pick);
      }
    }
    if (task.hasWorktree && !task.running) {
      // Its branch comes back first; uncommitted work in the folder goes with it, so twice.
      const sure = removing === task.id;
      parts.push(
        el("button", {
          class: "btn small",
          text: sure ? t("Uncommitted work in it is lost. Remove?") : t("Remove its folder"),
          onclick: () => {
            if (!sure) {
              removing = task.id;
              refresh();
              return;
            }
            removing = null;
            act(backend.removeWorktree(task.id));
          },
        }),
      );
    }
    return parts;
  };

  /** The review loop's rounds (W6, at most three), one mark each: the last in its verdict's color. */
  const rounds = (round: number, verdict: TaskRow["reviewVerdict"]): HTMLElement => {
    const marks = el("span", { class: "task-rounds" });
    // A pass ends the loop: no marks for rounds that will not come.
    const last = verdict === "pass" ? round : MAX_ROUNDS;
    for (let i = 1; i <= last; i++) {
      const state = i < round ? "done" : i === round ? (verdict ?? "none") : "later";
      marks.append(el("span", { class: `task-round ${state}` }));
    }
    const label = t("Review round {round} of {max}", { round, max: MAX_ROUNDS });
    // The words beside them say it: the marks are only a picture.
    marks.setAttribute("aria-hidden", "true");
    return el("div", { class: "task-rounds-line" }, marks, el("span", { class: "board-empty", text: label }));
  };

  const card = (task: TaskRow): HTMLElement => {
    const met = task.criteria.filter((c) => c.done).length;
    const sure = deleting === task.id;
    return el(
      "article",
      { class: `task ${task.status}` },
      el(
        "header",
        { class: "roost-head" },
        el("span", { class: "task-goal", text: task.goal }),
        task.running ? el("span", { class: "row-tag running", text: t("Running") }) : null,
        el("span", {
          class: "row-tag",
          text: task.criteria.length ? t("{met} of {all} met", { met, all: task.criteria.length }) : t("No criteria"),
        }),
      ),
      task.route && routed(task.route) ? el("p", { class: "board-empty task-route", text: routed(task.route)! }) : null,
      task.reviewRound ? rounds(task.reviewRound, task.reviewVerdict) : null,
      el("p", {
        class: "board-empty",
        text: [
          task.roost || task.folder || t("No project"),
          task.costUsd !== null ? t("Cost {cost}", { cost: money(task.costUsd) }) : null,
          task.tokens !== null ? t("{tokens} tokens", { tokens: tokens(task.tokens) }) : null,
        ]
          .filter(Boolean)
          .join(" · "),
      }),
      task.reply
        ? el("details", { class: "task-reply" }, el("summary", { text: t("What it said last") }), el("p", { text: task.reply }))
        : null,
      task.pack
        ? el("details", { class: "task-reply" }, el("summary", { text: t("Handoff notes") }), el("p", { text: task.pack }))
        : null,
      task.review
        ? el(
            "details",
            { class: "task-reply" },
            el("summary", { text: t(VERDICT[task.reviewVerdict ?? "none"], { round: task.reviewRound ?? 1 }) }),
            el("p", { text: task.review }),
          )
        : null,
      ...task.criteria.map((c, index) => {
        const box = el("input", { class: "task-check" });
        box.type = "checkbox";
        box.checked = c.done;
        box.addEventListener("change", () => act(backend.edit(task.id, { kind: "criterion", index, done: box.checked })));
        return el("label", { class: `task-criterion${c.done ? " met" : ""}` }, box, el("span", { text: c.text }));
      }),
      el(
        "div",
        { class: "proposal-actions" },
        select(translated(STATUS), task.status, (status) => act(backend.edit(task.id, { kind: "status", status }))),
        select({ ...OWNER, you: t(OWNER.you) }, task.owner, (owner) => act(backend.edit(task.id, { kind: "owner", owner }))),
        ...run(task),
        ...issue(task),
        el("button", {
          class: "btn small",
          text: sure ? t("Delete for good?") : t("Delete"),
          onclick: () => {
            if (!sure) {
              deleting = task.id;
              refresh();
              return;
            }
            deleting = null;
            act(backend.remove(task.id));
          },
        }),
      ),
    );
  };

  /** Today's spend, in one line; the budget is set in Activity → Cost. */
  const spend = (today: SpendToday | null): HTMLElement | null => {
    if (!today) return null;
    const over = today.budget !== null && today.usd >= today.budget;
    return el(
      "section",
      { class: `task-spend${over ? " over" : ""}` },
      el("p", { class: "task-spend-line", text: spentLine(today) }),
      over ? el("p", { class: "task-spend-line", text: t("Today's budget is spent: Vults starts no new session until tomorrow.") }) : null,
    );
  };

  function refresh(): void {
    const mine = ++asked;
    const today = backend.today().catch(() => null);
    void backend.load(closed).then(
      async (rows) => {
        const spent = await today;
        if (mine !== asked) return;
        const open = rows.filter((r) => r.status !== "done" && r.status !== "dropped");
        const rest = rows.filter((r) => r.status === "done" || r.status === "dropped");
        const parts: (HTMLElement | null)[] = [
          el(
            "div",
            { class: "proposal-actions" },
            el("button", {
              class: "btn small primary",
              text: t("New task"),
              onclick: () => {
                const first = backend.folders()[0];
                draft ??= { goal: "", criteria: "", where: first ? first.cwd : null };
                message = null;
                refresh();
              },
            }),
            el("button", {
              class: "btn small",
              text: closed ? t("Hide finished") : t("Show finished"),
              onclick: () => {
                closed = !closed;
                refresh();
              },
            }),
          ),
          spend(spent),
          message ? el("p", { class: "board-empty task-message", text: message }) : null,
          draft ? form(draft) : null,
          ...(open.length || draft
            ? byRoost(open).flatMap((g) => [el("h4", { class: "tasks-roost", text: g.roost || t("No project") }), ...g.tasks.map(card)])
            : [
                el("p", {
                  class: "board-empty",
                  text: backend.zeca()
                    ? t("No tasks. Type /task in the chat, then the goal and a line per criterion.")
                    : t("No tasks. New task keeps one: its goal and what done means."),
                }),
              ]),
          ...(closed && rest.length ? [el("h4", { class: "tasks-finished", text: t("Finished") }), ...rest.map(card)] : []),
        ];
        element.replaceChildren(...parts.filter((p): p is HTMLElement => p !== null));
        message = null;
      },
      // Unreadable: say so and stop; the next change or a click on the tab asks again.
      (e) => {
        if (mine !== asked) return;
        element.replaceChildren(el("p", { class: "board-empty task-message", text: said(e) }));
      },
    );
  }

  return { element, refresh };
}
