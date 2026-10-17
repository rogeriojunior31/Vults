// Settings alone owns the daily budget; Activity reads its figures (ADR 0031).
import type { SpendToday } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { row } from "./pieces";

export interface BudgetBackend {
  today: () => Promise<SpendToday>;
  setBudget: (cents: number | null) => Promise<void>;
}

export function createBudget(backend: BudgetBackend) {
  const element = el("section", { class: "card rows" });
  const input = el("input", { class: "field budget-input" });
  input.type = "number";
  input.min = "0.01";
  input.max = "10000";
  input.step = "0.01";
  let loaded: SpendToday | null = null;
  let draft: string | null = null;
  let loading = false;
  let saving = false;
  let message = "";
  let failed = false;
  let asked = 0;
  input.addEventListener("input", () => { draft = input.value; });
  const save = el("button", { class: "btn", onclick: () => {
    if (saving || !loaded) return;
    const typed = input.value.trim();
    const cents = Math.round(Number(typed) * 100);
    draft = typed;
    if (typed !== "" && !(Number.isFinite(cents) && cents >= 1 && cents <= 1_000_000)) {
      failed = true;
      message = t("Type a budget from $0.01 to $10,000, or leave it empty for none.");
      draw();
      return;
    }
    // A read started before this write must not restore the old budget afterwards.
    asked++;
    loading = false;
    saving = true;
    failed = false;
    message = "";
    const focused = document.activeElement === save;
    draw();
    void backend.setBudget(typed === "" ? null : cents).then(() => {
      loaded = { ...loaded!, budget: typed === "" ? null : cents / 100 };
      draft = null;
      message = t("Saved.");
    }, error => { failed = true; message = String(error); }).finally(() => {
      saving = false;
      draw();
      if (focused && document.activeElement === document.body && save.isConnected) save.focus({ preventScroll: true });
    });
  } });
  const setting = row(t("Daily budget in US dollars"), "", el("div", { class: "budget-controls" }, input, save));
  const feedback = el("p", { class: "note" });
  const retry = el("button", { class: "btn", onclick: () => refresh() });
  element.append(setting, feedback, retry);
  const draw = () => {
    setting.querySelector<HTMLElement>(".row-title")!.textContent = t("Daily budget in US dollars");
    setting.querySelector<HTMLElement>(".row-about")!.textContent = t("Leave it empty for no limit. Vults never stops an agent you are watching.");
    input.placeholder = t("No budget");
    input.value = draft ?? (loaded?.budget != null ? String(loaded.budget) : "");
    input.disabled = !loaded || saving;
    save.disabled = !loaded || saving;
    save.textContent = saving ? t("Saving…") : t("Save");
    feedback.textContent = loading && !loaded ? t("Loading") : message;
    feedback.className = `note${failed ? " error" : ""}`;
    feedback.setAttribute("role", failed ? "alert" : "status");
    feedback.hidden = !feedback.textContent;
    retry.textContent = t("Try again");
    retry.hidden = !!loaded || loading;
  };
  const refresh = () => {
    if (saving) return;
    const mine = ++asked;
    loading = true;
    draw();
    void backend.today().then(value => {
      if (mine !== asked) return;
      loaded = value;
      loading = false;
      failed = false;
      message = "";
      draw();
    }, error => {
      if (mine !== asked) return;
      loading = false;
      failed = true;
      message = String(error);
      draw();
    });
  };
  return { element, refresh, relabel: draw };
}
