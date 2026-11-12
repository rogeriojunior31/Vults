// The Allow shortcut answers only where Allow is the bright button. A destructive request (or a
// chat card steered by outside content) is allowed by a click: a key pressed by habit, as the queue
// swaps cards, must never run it. Deny still answers from the keyboard.
import { el } from "../dom";
import { t } from "../i18n";

export const allowsByKey = (approval: { why?: string | null }): boolean => approval.why !== "destructive";

/** Shown where the Allow key would be, on a request the key does not allow. */
export function clickOnly(): HTMLElement {
  const hint = el("span", { class: "kbd click-only", text: t("click") });
  hint.title = t("Allow it with a click. The shortcut does not run this request.");
  return hint;
}

/** The Allow key was pressed where only a click allows: the button says so, nothing runs. */
export function refuseAllowKey(scope: ParentNode | null | undefined): void {
  const target = scope?.querySelector<HTMLElement>(".allow-by-click");
  if (!target) return;
  target.classList.remove("key-refused");
  void target.offsetWidth;
  target.classList.add("key-refused");
}
