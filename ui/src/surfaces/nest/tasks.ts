// The Nest's Tasks tab (plan-zeca W3): what the user wants done, with the criteria that say it is,
// its roost, who works on it, where it stands and what it cost. `/task` in the chat makes one in a
// single message; **New task** here does too. With the GitHub connector on, a task in a GitHub
// repository links to an issue, or opens one on a click.
import type { TaskEdit, TaskRow } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";

export interface TasksBackend {
  load(closed: boolean): Promise<TaskRow[]>;
  create(goal: string, criteria: string[], folder: string | null): Promise<number>;
  edit(id: number, edit: TaskEdit): Promise<void>;
  remove(id: number): Promise<void>;
  openIssue(id: number): Promise<string>;
  /** Opens the task's linked issue in the browser. */
  visit(id: number): void;
  /** The folders the flock works in now, for a new task's roost. */
  folders(): { cwd: string; project: string }[];
}

const STATUS: Record<string, string> = {
  open: tk("To do"),
  doing: tk("Doing"),
  blocked: tk("Blocked"),
  done: tk("Done"),
  dropped: tk("Dropped"),
};

/** Who works on it: the user's label translated, the names as they are. */
const OWNER: Record<string, string> = {
  you: tk("You"),
  zeca: "Zeca",
  claude: "Claude Code",
  codex: "Codex",
  gemini: "Gemini CLI",
  opencode: "OpenCode",
  qwen: "Qwen Code",
  antigravity: "Antigravity",
};

/** The errors the tasks' commands give back in English, translated here. */
const ERRORS = new Set([
  tk("No such task."),
  tk("No such owner."),
  tk("That is not a GitHub issue."),
  tk("Turn on the GitHub connector in Settings first."),
  tk("This task already has an issue."),
  tk("This task is not in a GitHub repository."),
]);

/** `options` maps each value to its label, already in the user's language. */
function select(options: Record<string, string>, value: string, change: (v: string) => void): HTMLSelectElement {
  const s = el("select", { class: "task-select" });
  for (const [v, label] of Object.entries(options)) {
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

function money(usd: number): string {
  return usd < 0.01 ? "< $0.01" : `$${usd.toFixed(2)}`;
}

export function createTasks(backend: TasksBackend): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: "tasks" });
  let closed = false;
  let adding = false;
  let message: string | null = null;
  let asked = 0;

  const fail = (e: unknown) => {
    const text = String(e);
    message = ERRORS.has(text) ? t(text) : text;
    refresh();
  };
  const act = (p: Promise<unknown>) => void p.then(() => refresh(), fail);

  const form = (): HTMLElement => {
    const goal = el("input", { class: "task-goal-input" });
    goal.placeholder = t("What should be done?");
    const criteria = el("textarea", { class: "task-criteria-input" });
    criteria.placeholder = t("Done when… (one per line)");
    criteria.rows = 3;
    const folders = backend.folders();
    const where = el("select", { class: "task-select" });
    const none = el("option", { text: t("No project") });
    none.value = "";
    where.append(none);
    for (const f of folders) {
      const o = el("option", { text: f.project });
      o.value = f.cwd;
      where.append(o);
    }
    if (folders.length) where.value = folders[0].cwd;
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
            if (!goal.value.trim()) return;
            const lines = criteria.value.split("\n").map((l) => l.replace(/^\s*[-*]\s*/, "").trim()).filter(Boolean);
            adding = false;
            act(backend.create(goal.value, lines, where.value || null));
          },
        }),
        el("button", {
          class: "btn small",
          text: t("Cancel"),
          onclick: () => {
            adding = false;
            refresh();
          },
        }),
      ),
    );
  };

  const card = (task: TaskRow): HTMLElement => {
    const met = task.criteria.filter((c) => c.done).length;
    const issue = task.issue
      ? el("button", { class: "btn small", text: t("Issue"), onclick: () => backend.visit(task.id) })
      : task.canOpenIssue
        ? el("button", {
            class: "btn small",
            text: t("Open an issue"),
            onclick: () => act(backend.openIssue(task.id)),
          })
        : null;
    return el(
      "article",
      { class: `task ${task.status}` },
      el(
        "header",
        { class: "roost-head" },
        el("span", { class: "task-goal", text: task.goal }),
        el("span", {
          class: "row-tag",
          text: task.criteria.length ? t("{met} of {all} met", { met, all: task.criteria.length }) : t("No criteria"),
        }),
      ),
      el("p", {
        class: "board-empty",
        text: [task.roost || task.folder || t("No project"), task.costUsd !== null ? t("Cost {cost}", { cost: money(task.costUsd) }) : null]
          .filter(Boolean)
          .join(" · "),
      }),
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
        issue,
        el("button", { class: "btn small", text: t("Delete"), onclick: () => act(backend.remove(task.id)) }),
      ),
    );
  };

  function refresh(): void {
    const mine = ++asked;
    void backend.load(closed).then(
      (rows) => {
        if (mine !== asked) return;
        const open = rows.filter((r) => r.status !== "done" && r.status !== "dropped");
        const rest = rows.filter((r) => r.status === "done" || r.status === "dropped");
        const parts: (HTMLElement | null)[] = [
          el(
            "div",
            { class: "proposal-actions" },
            el("button", {
              class: "btn small",
              text: t("New task"),
              onclick: () => {
                adding = true;
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
          message ? el("p", { class: "board-empty task-message", text: message }) : null,
          adding ? form() : null,
          ...(open.length || adding
            ? open.map(card)
            : [el("p", { class: "board-empty", text: t("No tasks. Type /task in the chat, then the goal and a line per criterion.") })]),
          ...(closed && rest.length ? [el("h4", { class: "tasks-finished", text: t("Finished") }), ...rest.map(card)] : []),
        ];
        element.replaceChildren(...parts.filter((p): p is HTMLElement => p !== null));
        message = null;
      },
      fail,
    );
  }

  return { element, refresh };
}
