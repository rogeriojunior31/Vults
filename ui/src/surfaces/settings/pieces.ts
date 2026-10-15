// Small controls shared by the settings pages.
import { el } from "../../dom";
import { t } from "../../i18n";

let rowNumber = 0;

export function toggle(on: boolean, change: (on: boolean) => Promise<void>): HTMLElement {
  const input = document.createElement("input");
  input.type = "checkbox";
  input.className = "toggle";
  input.checked = on;
  input.addEventListener("change", () => {
    void change(input.checked).catch(() => {
      input.checked = !input.checked;
    });
  });
  return input;
}

export function row(title: string, about: string, control: HTMLElement, more?: string): HTMLElement {
  const titleId = `settings-row-title-${++rowNumber}`;
  const titleElement = el("div", { class: "row-title", text: title });
  titleElement.id = titleId;
  const description = el("div", { class: "row-about", text: about });
  description.id = `${titleId}-about`;
  if (!control.matches("input, select, textarea, button")) control.setAttribute("role", "group");
  control.setAttribute("aria-labelledby", titleId);
  control.setAttribute("aria-describedby", description.id);
  const fields = control.matches("input, select, textarea, button") ? [control] : [...control.querySelectorAll<HTMLElement>("input, select, textarea, button")];
  fields.forEach((field, i) => { field.dataset.setting = `${title}:${i}`; });
  const details = more ? el("details", { class: "row-more" }, el("summary", { text: t("More") }), el("p", { text: more })) : null;
  if (details) details.dataset.setting = title;
  return el(
    "div",
    { class: "row" },
    el(
      "div",
      { class: "row-text" },
      titleElement,
      description,
      details,
    ),
    control,
  );
}

export function disclosure(title: string, ...content: HTMLElement[]): HTMLDetailsElement {
  const details = el("details", { class: "settings-disclosure" }, el("summary", { text: title }), el("div", { class: "disclosure-body" }, ...content));
  details.dataset.setting = title;
  return details;
}

export function button(text: string, onclick: () => void, primary = false): HTMLElement {
  return el("button", { class: primary ? "btn primary" : "btn", text, onclick });
}

/** A unified diff with its added and removed lines colored. */
export function diff(text: string): HTMLElement {
  const pre = el("pre", { class: "diff" });
  for (const line of text.split("\n")) {
    const kind = line.startsWith("+++") || line.startsWith("---") ? "meta" : line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : line.startsWith("@@") ? "hunk" : "";
    pre.append(el("span", { class: `dl ${kind}`, text: `${line}\n` }));
  }
  return pre;
}
