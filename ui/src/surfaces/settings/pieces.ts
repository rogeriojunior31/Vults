// Small controls shared by the settings pages.
import { el } from "../../dom";
import { t } from "../../i18n";

let rowNumber = 0;

type SettingField = HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | HTMLButtonElement;
const fields = (control: HTMLElement): SettingField[] => control.matches("input, select, textarea, button")
  ? [control as SettingField] : [...control.querySelectorAll<SettingField>("input, select, textarea, button")];
type SettingView = { control: HTMLElement; feedback: HTMLElement; disabled: boolean[] };
const operations = new Map<string, { pending: boolean; error: boolean; message: string; views: SettingView[]; sync: (control: HTMLElement) => void }>();

function showOperation(view: SettingView, state: NonNullable<ReturnType<typeof operations.get>>): void {
  if (state.pending || state.views.includes(view)) state.sync(view.control);
  view.control.setAttribute("aria-busy", String(state.pending));
  fields(view.control).forEach((field, i) => { field.disabled = state.pending || view.disabled[i]; });
  view.feedback.textContent = state.message;
  view.feedback.className = `setting-feedback${state.error ? " error" : ""}`;
}

/** Native controls share one pending operation even if a settings event repaints their row. */
export function saveSetting(control: HTMLElement, write: () => Promise<void>, rollback: () => void): void {
  const key = control.dataset.setting;
  if (control.getAttribute("aria-busy") === "true" || key && operations.get(key)?.pending) return;
  const focused = fields(control).indexOf(document.activeElement as SettingField);
  const capture = () => {
    const values = fields(control).map(field => ({ value: field.value, checked: (field as HTMLInputElement).checked, pressed: field.getAttribute("aria-pressed") }));
    return (next: HTMLElement) => fields(next).forEach((field, i) => {
      const value = values[i];
      if (!value) return;
      if (field.matches("input")) (field as HTMLInputElement).checked = value.checked;
      if (field.matches("select")) {
        field.value = value.value;
        field.dataset.savedValue = value.value;
      }
      if (value.pressed !== null) {
        field.setAttribute("aria-pressed", value.pressed);
        field.classList.toggle("on", value.pressed === "true");
      }
    });
  };
  const feedback = control.closest(".row")?.querySelector<HTMLElement>(".setting-feedback") ?? el("span", { class: "setting-feedback" });
  if (!feedback.parentElement) control.parentElement?.append(feedback);
  const state = { pending: true, error: false, message: t("Saving…"), views: [{ control, feedback, disabled: fields(control).map(field => field.disabled) }], sync: capture() };
  if (key) operations.set(key, state);
  state.views.forEach(view => showOperation(view, state));
  void Promise.resolve().then(write).then(() => { state.message = t("Saved."); }, () => {
    rollback();
    state.sync = capture();
    state.error = true;
    state.message = t("Could not save. Your previous choice is restored. Try again.");
  }).finally(() => {
    state.pending = false;
    state.views.forEach(view => showOperation(view, state));
    if (focused >= 0 && document.activeElement === document.body) {
      const latest = state.views.filter(view => view.control.isConnected).at(-1);
      if (latest) fields(latest.control)[focused]?.focus({ preventScroll: true });
    }
    state.views = [];
    if (key && operations.get(key) === state) operations.delete(key);
  });
}

export function toggle(on: boolean, change: (on: boolean) => Promise<void>): HTMLElement {
  const input = document.createElement("input");
  input.type = "checkbox";
  input.className = "toggle";
  input.checked = on;
  input.addEventListener("change", () => {
    if (input.getAttribute("aria-busy") === "true") return;
    const next = input.checked;
    saveSetting(input, () => change(next), () => { input.checked = !next; });
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
  control.dataset.setting = title;
  fields(control).forEach((field, i) => {
    if (field !== control) field.dataset.setting = `${title}:${i}`;
    if (!field.matches("button") && !field.hasAttribute("aria-label")) field.setAttribute("aria-labelledby", titleId);
    field.setAttribute("aria-describedby", description.id);
  });
  const feedback = el("span", { class: "setting-feedback" });
  feedback.setAttribute("role", "status");
  const state = operations.get(title);
  if (state) {
    const view = { control, feedback, disabled: fields(control).map(field => field.disabled) };
    if (state.pending) state.views.push(view);
    showOperation(view, state);
  }
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
      feedback,
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
