// Quick's short views (ADR 0021), as the prototype draws them: this week, and a few settings, at
// the island's width, each with a way to the Nest for the rest. The week only reads; the settings
// change only what the tray's menu already can (presence) and how long the island stays open.
import type { Presence } from "../bridge";
import { el } from "../dom";
import { duration } from "../duration";
import { currentLang, t, tk } from "../i18n";
import type { WeekView } from "../view.gen";
import { icon } from "./icons";

function foot(label: string, open: () => void): HTMLElement {
  return el(
    "div",
    { class: "quick-foot" },
    el("button", { class: "btn secondary small", text: label, onclick: open }),
  );
}

/** A day's short name in the user's language, from the week's Monday and its index. */
function dayName(monday: string, i: number): string {
  const d = new Date(`${monday}T12:00:00`);
  d.setDate(d.getDate() + i);
  return new Intl.DateTimeFormat(currentLang(), { weekday: "short" }).format(d);
}

/** This week, counted on this computer; `null` while it is read. */
export function weekQuick(
  week: WeekView | null,
  history: boolean,
  open: () => void,
  failed = false,
): HTMLElement {
  const stat = (value: string, label: string) =>
    el(
      "div",
      { class: "quick-stat" },
      el("strong", { text: value }),
      el("span", { text: label }),
    );
  const off = history
    ? null
    : el("p", {
        class: "quick-note",
        text: t("The local history is off: Settings, Activity."),
      });
  const head = el(
    "div",
    { class: "quick-head" },
    el("span", { text: t("Activity") }),
    el("span", { class: "quick-when", text: t("This week") }),
  );
  if (week === null)
    return el(
      "div",
      { class: "card quick quick-week" },
      head,
      el("p", {
        class: "quick-note",
        text: failed ? t("Could not read it.") : t("Reading…"),
      }),
      foot(t("Open in the Nest"), open),
    );
  const most = Math.max(1, ...week.days);
  const days = week.turns
    ? [
        el("div", { class: "quick-label", text: t("By day") }),
        ...week.days.map((secs, i) => {
          const fill = el("span", { class: "quick-fill" });
          fill.style.width = `${Math.round((secs / most) * 100)}%`;
          return el(
            "div",
            { class: "quick-bar" },
            el("span", { class: "quick-day", text: dayName(week.monday, i) }),
            el("span", { class: "quick-track" }, fill),
            el("span", {
              class: "quick-secs",
              text: secs ? duration(secs) : "–",
            }),
          );
        }),
      ]
    : [el("p", { class: "quick-note", text: week.headline })];
  return el(
    "div",
    { class: "card quick quick-week" },
    head,
    off,
    el(
      "div",
      { class: "quick-stats" },
      stat(duration(week.active_secs), t("with your agents")),
      stat(String(week.turns), week.turns === 1 ? t("turn") : t("turns")),
      stat(`+${week.added} −${week.removed}`, t("lines")),
      stat(String(week.allowed + week.denied + week.answered), t("answers")),
    ),
    ...days,
    foot(t("Open in the Nest"), open),
  );
}

/** Settings' pages, as its sidebar names them; General is drawn here, the others open in the Nest. */
const PAGES: { id: string; label: string }[] = [
  { id: "general", label: tk("General") },
  { id: "chat", label: tk("Chat & voice") },
  { id: "flock", label: tk("Flock & Zeca") },
  { id: "agents", label: tk("Agents") },
  { id: "approvals", label: tk("Approvals") },
  { id: "projects", label: tk("Projects") },
  { id: "activity", label: tk("Activity") },
  { id: "connectors", label: tk("Connectors") },
  { id: "phone", label: tk("Phone") },
  { id: "about", label: tk("About") },
];

const PRESENCES: { value: Presence; label: string }[] = [
  { value: "island", label: tk("Island") },
  { value: "panel", label: tk("Panel") },
  { value: "quiet", label: tk("Quiet") },
  { value: "paused", label: tk("Paused") },
];

const FOLD_CHOICES = [5, 10, 15, 30, 60];

export interface QuickSettings {
  presence: Presence;
  foldAfter: number;
}

export interface QuickSettingsActions {
  setPresence(p: Presence): void;
  setFoldAfter(seconds: number): void;
  /** Settings in the Nest, at that page. */
  open(page?: string): void;
}

/**
 * A pick that unfolds in place: the island takes the mouse only over its own rectangle, so a list
 * hanging outside it could not be clicked.
 */
function pick<T extends string | number>(
  choices: { value: T; label: string }[],
  value: T,
  onPick: (v: T) => void,
  relayout: () => void,
  wide = false,
): HTMLElement {
  const box = el("div", { class: `quick-pick${wide ? " wide" : ""}` });
  const current = choices.find((c) => c.value === value) ?? choices[0];
  const button = el(
    "button",
    { class: "quick-pick-btn" },
    el("span", { text: current.label }),
    icon("chevron", 11, 2),
  );
  const list = el(
    "div",
    { class: "quick-pick-list" },
    ...choices.map((c) =>
      el("button", {
        class: `quick-pick-item${c.value === value ? " on" : ""}`,
        text: c.label,
        onclick: () => {
          box.classList.remove("open");
          relayout();
          if (c.value !== value) onPick(c.value);
        },
      }),
    ),
  );
  button.addEventListener("click", () => {
    box.classList.toggle("open");
    relayout();
  });
  box.append(button, list);
  return box;
}

/** Quick's gear: General's island settings here, any other page in the Nest. `null` while read. */
export function settingsQuick(
  s: QuickSettings | null,
  on: QuickSettingsActions,
  relayout: () => void,
  failed = false,
): HTMLElement {
  const pages = PAGES.map((p) => ({ value: p.id, label: t(p.label) }));
  const head = el(
    "div",
    { class: "quick-settings-head" },
    el("div", { class: "quick-label", text: t("Settings") }),
    pick(pages, "general", (id) => on.open(id), relayout, true),
  );
  if (s === null)
    return el(
      "div",
      { class: "card quick quick-settings" },
      head,
      el("p", {
        class: "quick-note",
        text: failed ? t("Could not read them.") : t("Reading…"),
      }),
    );
  const row = (title: string, sub: string, control: HTMLElement) =>
    el(
      "div",
      { class: "quick-setting" },
      el(
        "div",
        { class: "quick-setting-text" },
        el("strong", { text: title }),
        el("span", { text: sub }),
      ),
      control,
    );
  const fold = FOLD_CHOICES.reduce((a, b) =>
    Math.abs(b - s.foldAfter) < Math.abs(a - s.foldAfter) ? b : a,
  );
  return el(
    "div",
    { class: "card quick quick-settings" },
    head,
    row(
      t("Presence"),
      t("Where the island appears."),
      pick(
        PRESENCES.map((p) => ({ value: p.value, label: t(p.label) })),
        s.presence,
        (p) => on.setPresence(p),
        relayout,
      ),
    ),
    row(
      t("Fold the island"),
      t("A permission keeps it open until you answer."),
      pick(
        FOLD_CHOICES.map((n) => ({ value: n, label: `${n} s` })),
        fold,
        (n) => on.setFoldAfter(n),
        relayout,
      ),
    ),
    foot(t("All settings"), () => on.open()),
  );
}
