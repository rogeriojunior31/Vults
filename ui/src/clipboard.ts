// Shared copy feedback for code blocks and complete chat replies.
import { el } from "./dom";
import { t } from "./i18n";

export function copyTextButton(text: string, className: string, name: string): HTMLButtonElement {
  const label = el("span", { text: t("Copy") });
  label.setAttribute("role", "status");
  const button = el("button", { class: className }, label);
  button.setAttribute("aria-label", name);
  let copying = false;
  let timer: number | undefined;
  button.addEventListener("click", () => {
    if (copying) return;
    copying = true;
    window.clearTimeout(timer);
    button.setAttribute("aria-busy", "true");
    void Promise.resolve().then(() => {
      if (!navigator.clipboard) throw new Error("Clipboard unavailable");
      return navigator.clipboard.writeText(text);
    }).then(() => {
      label.textContent = t("Copied");
      button.title = t("Copied");
    }, () => {
      label.textContent = t("Copy failed. Try again.");
      button.title = t("Copy failed. Try again.");
    }).finally(() => {
      copying = false;
      button.setAttribute("aria-busy", "false");
      timer = window.setTimeout(() => { label.textContent = t("Copy"); button.title = name; }, 2500);
    });
  });
  return button;
}
