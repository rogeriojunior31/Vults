// Activity → Cost (W9, moved by ADR 0021): what the agents cost today and the daily budget. At 80%
// a notification, once spent another, and Vults starts no new session that day; it never stops or
// blocks an agent.
import type { SpendToday } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";

export interface BudgetBackend {
  today: () => Promise<SpendToday>;
  setBudget: (cents: number | null) => Promise<void>;
}

/** A cost as a card says it: $1.25, or under a cent. */
export function money(usd: number): string {
  if (usd === 0) return "$0.00";
  return usd < 0.01 ? "< $0.01" : `$${usd.toFixed(2)}`;
}

export type UsageLevel = "normal" | "warning" | "critical";

/** The same usage warning thresholds everywhere Vults shows a plan. */
export function usageLevel(percent: number): UsageLevel {
  if (percent >= 90) return "critical";
  if (percent >= 70) return "warning";
  return "normal";
}

export function usageLabel(percent: number): string {
  const level = usageLevel(percent);
  return level === "critical"
    ? t("Near limit")
    : level === "warning"
      ? t("Warning")
      : t("Normal");
}

/** Today's spend in one line, as the Tasks tab says it too; `plan` adds what a subscription means
 *  (Cost says it once under its records). */
export function spentLine(today: SpendToday, plan = true): string {
  const spent = money(today.usd);
  let line =
    today.budget !== null
      ? t("Today the agents cost {spent} of your {budget}.", { spent, budget: money(today.budget) })
      : t("Today the agents cost {spent}.", { spent });
  if (plan && today.subscription) line += " " + t("On a subscription this is at API prices: your plan does not charge it.");
  return line;
}

/** `saved` runs after a new budget is kept: the records' budget tick follows it. */
export function createBudget(backend: BudgetBackend, saved: () => void = () => {}): { element: HTMLElement; refresh: () => void } {
  const element = el("section", { class: "task-spend" });
  /** The budget being typed, kept while it is drawn again; null while not edited. */
  let draft: string | null = null;
  let message: string | null = null;
  let asked = 0;
  const draw = (today: SpendToday | null) => {
    if (!today) {
      // Unreadable: only a message about a save that failed is left to say.
      element.replaceChildren(...(message ? [el("p", { class: "board-empty task-message", text: message })] : []));
      message = null;
      return;
    }
    // The user is typing in the field: a redraw now would take the cursor away.
    if (element.contains(document.activeElement) && document.activeElement instanceof HTMLInputElement) return;
    const over = today.budget !== null && today.usd >= today.budget;
    element.classList.toggle("over", over);
    const input = el("input", { class: "task-budget-input" });
    input.type = "number";
    input.min = "0";
    input.step = "0.5";
    input.placeholder = t("No budget");
    input.value = draft ?? (today.budget !== null ? String(today.budget) : "");
    input.addEventListener("input", () => (draft = input.value));
    element.replaceChildren(
      el("p", { class: "task-spend-line", text: spentLine(today, false) }),
      ...(over ? [el("p", { class: "task-spend-line", text: t("Today's budget is spent: Vults starts no new session until tomorrow.") })] : []),
      el(
        "div",
        { class: "proposal-actions" },
        el("label", { class: "task-budget-label", text: t("Daily budget in US dollars") }),
        input,
        el("button", {
          class: "btn small",
          text: t("Save"),
          onclick: () => {
            const typed = input.value.trim();
            const cents = Math.round(Number(typed) * 100);
            // Only an empty field means no budget: anything else unreadable is said, never saved.
            if (typed !== "" && !(Number.isFinite(cents) && cents >= 1 && cents <= 1_000_000)) {
              message = t("Type a budget from $0.01 to $10,000, or leave it empty for none.");
              refresh();
              return;
            }
            draft = null;
            input.blur();
            void backend.setBudget(typed === "" ? null : cents).then(
              () => {
                refresh();
                saved();
              },
              (e) => {
                message = String(e);
                refresh();
              },
            );
          },
        }),
      ),
      ...(message ? [el("p", { class: "board-empty task-message", text: message })] : []),
    );
    message = null;
  };
  const refresh = () => {
    const mine = ++asked;
    void backend.today().then(
      (today) => mine === asked && draw(today),
      () => mine === asked && draw(null),
    );
  };
  return { element, refresh };
}
