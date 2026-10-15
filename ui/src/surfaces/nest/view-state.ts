import { el } from "../../dom";
import { t } from "../../i18n";
import { icon } from "../../island/icons";

/** Read-only view feedback; the caller retains ownership of every action. */
export function viewState(kind: "loading" | "empty" | "error", title: string, about?: string, action?: HTMLButtonElement | null, detail?: string | null): HTMLElement {
  const mark = el("span", { class: "view-state-mark" }, icon(kind === "loading" ? "bars" : kind === "error" ? "retry" : "list", 18, 1.7));
  mark.setAttribute("aria-hidden", "true");
  const extra = detail ? el("details", { class: "view-state-detail" },
    el("summary", { text: t("Technical details") }), el("p", { text: detail })) : null;
  const panel = el("section", { class: `view-state ${kind}` }, mark,
    el("div", { class: "view-state-body" }, el("h3", { text: title }), about ? el("p", { text: about }) : null, action ?? null, extra));
  panel.setAttribute("role", kind === "error" ? "alert" : "status");
  return panel;
}
