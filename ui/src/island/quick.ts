// Quick's short views (ADR 0021): the open tasks and this week, at the island's width, each with a
// way to the Nest for the rest. They only read; nothing here starts, answers or changes anything.
import type { TaskRow } from "../bridge";
import { el } from "../dom";
import { duration } from "../duration";
import { t, tk } from "../i18n";
import type { WeekView } from "../view.gen";

const STATUS: Record<TaskRow["status"], string> = {
  open: tk("To do"),
  doing: tk("Doing"),
  blocked: tk("Blocked"),
  done: tk("Done"),
  dropped: tk("Dropped"),
};

/** Tasks shown at once; the Nest has them all. */
const TASKS_SHOWN = 5;

function foot(label: string, open: () => void): HTMLElement {
  return el("div", { class: "quick-foot" }, el("button", { class: "btn secondary small", text: label, onclick: open }));
}

/** The open tasks, newest first; `null` while they are read. */
export function tasksQuick(rows: TaskRow[] | null, open: () => void, zeca: boolean): HTMLElement {
  const live = (rows ?? []).filter((r) => r.status !== "done" && r.status !== "dropped");
  const body = rows === null
    ? [el("p", { class: "quick-note", text: t("Reading…") })]
    : live.length
      ? live.slice(0, TASKS_SHOWN).map((r) =>
          el(
            "div",
            { class: `quick-row ${r.status}` },
            el("span", { class: "quick-goal", text: r.goal }),
            el("span", { class: "quick-meta", text: [r.roost, t(STATUS[r.status]), r.costUsd ? `$${r.costUsd.toFixed(2)}` : ""].filter(Boolean).join(" · ") }),
          ),
        )
      : [el("p", { class: "quick-note", text: zeca ? t("No open tasks. Type /task in the chat to keep one.") : t("No open tasks.") })];
  const more = live.length > TASKS_SHOWN ? el("p", { class: "quick-note", text: t("{n} more in the Nest", { n: live.length - TASKS_SHOWN }) }) : null;
  return el(
    "div",
    { class: "card quick" },
    el("div", { class: "quick-head", text: t("Tasks") }),
    ...body,
    more,
    foot(t("Open in the Nest"), open),
  );
}

/** This week, counted on this computer; `null` while it is read. */
export function weekQuick(week: WeekView | null, history: boolean, open: () => void): HTMLElement {
  const stat = (value: string, label: string) => el("div", { class: "quick-stat" }, el("strong", { text: value }), el("span", { text: label }));
  const body = week === null
    ? [el("p", { class: "quick-note", text: history ? t("Reading…") : t("The local history is off: Settings, Activity.") })]
    : [
        el("p", { class: "quick-headline", text: week.headline }),
        ...(week.turns
          ? [
              el(
                "div",
                { class: "quick-stats" },
                stat(duration(week.active_secs), t("with your agents")),
                stat(String(week.turns), week.turns === 1 ? t("turn") : t("turns")),
                stat(`+${week.added} −${week.removed}`, t("lines")),
                stat(String(week.allowed + week.denied + week.answered), t("answers")),
              ),
            ]
          : []),
      ];
  return el("div", { class: "card quick" }, el("div", { class: "quick-head", text: t("This week") }), ...body, foot(t("Open in the Nest"), open));
}
