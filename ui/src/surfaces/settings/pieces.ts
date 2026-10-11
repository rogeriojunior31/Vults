// Small controls shared by the settings pages (and their lab).
import { el } from "../../dom";

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

export function row(title: string, about: string, control: HTMLElement): HTMLElement {
  return el(
    "div",
    { class: "row" },
    el("div", { class: "row-text" }, el("div", { class: "row-title", text: title }), el("div", { class: "row-about", text: about })),
    control,
  );
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
