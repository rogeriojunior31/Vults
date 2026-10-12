// Quick's short views (ADR 0021), as the prototype draws them: this week, and a few settings, at
// the island's width, each with a way to the Nest for the rest. The week only reads; the settings
// change only what the tray's menu already can (presence) and how long the island stays open.
import type { Flock, Presence } from "../bridge";
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

/** Projects in Quick's week; the Nest's Activity has them all. */
const PROJECTS_SHOWN = 4;

/** A day's short name in the user's language, from the week's Monday and its index. */
function dayName(monday: string, i: number): string {
  const d = new Date(`${monday}T12:00:00`);
  d.setDate(d.getDate() + i);
  return new Intl.DateTimeFormat(currentLang(), { weekday: "short" }).format(d);
}

/** This week, counted on this computer; `null` while it is read. */
/** One of the week buttons: which is shown says aria-pressed. */
function weekButton(text: string, name: string, on: boolean, go: () => void): HTMLElement {
  const b = el("button", { class: `quick-week-btn${on ? " on" : ""}`, text, onclick: go });
  b.setAttribute("aria-label", name);
  b.setAttribute("aria-pressed", String(on));
  return b;
}

export function weekQuick(
  week: WeekView | null,
  history: boolean,
  open: () => void,
  failed = false,
  /** This week or the one before, and the way between them (the prototype's two buttons). */
  nav: { previous: boolean; go: (previous: boolean) => void } | null = null,
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
    el("span", { class: "quick-when", text: nav?.previous ? t("Previous week") : t("This week") }),
  );
  const weeks =
    nav && history
      ? el(
          "div",
          { class: "quick-weeks" },
          weekButton(`‹ ${t("Previous week")}`, t("Previous week"), nav.previous, () => nav.go(true)),
          weekButton(t("This week"), t("This week"), !nav.previous, () => nav.go(false)),
        )
      : null;
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
  const bar = (name: string, secs: number, most: number) => {
    const fill = el("span", { class: "quick-fill" });
    fill.style.width = `${Math.round((secs / Math.max(1, most)) * 100)}%`;
    const row = el(
      "div",
      { class: "quick-bar" },
      el("span", { class: "quick-day", text: name }),
      el("span", { class: "quick-track" }, fill),
      el("span", { class: "quick-secs", text: secs ? duration(secs) : "–" }),
    );
    row.title = name;
    return row;
  };
  // By project, as the prototype draws it (the most first, four at most); a week of turns with no
  // project named falls back to its days.
  const projects = week.projects.slice(0, PROJECTS_SHOWN);
  const days = !week.turns
    ? [el("p", { class: "quick-note", text: week.headline })]
    : projects.length
      ? [el("div", { class: "quick-label", text: t("By project") }), ...projects.map((p) => bar(p.name, p.secs, projects[0].secs))]
      : [el("div", { class: "quick-label", text: t("By day") }), ...week.days.map((secs, i) => bar(dayName(week.monday, i), secs, Math.max(...week.days)))];
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
    weeks,
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
  /** Activity's local history. */
  history: boolean;
  zeca: boolean;
  flock: Flock;
}

/** The gear's pages drawn in place, as the prototype's: the rest open in the Nest. */
export type QuickPage = "general" | "activity" | "flock";
const IN_PLACE: string[] = ["general", "activity", "flock"];

const FLOCKS: { value: Flock; label: string }[] = [
  { value: "brazil", label: tk("Brazil") },
  { value: "americas", label: tk("The Americas") },
  { value: "world", label: tk("The world") },
];

export interface QuickSettingsActions {
  setPresence(p: Presence): void;
  setFoldAfter(seconds: number): void;
  setHistory(on: boolean): void;
  setZeca(on: boolean): void;
  setFlock(flock: Flock): void;
  /** Another of the pages drawn here. */
  show(page: QuickPage): void;
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
    // The island stops growing at its tallest and scrolls: the list must come into sight.
    if (box.classList.contains("open")) list.scrollIntoView({ block: "nearest" });
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
  page: QuickPage = "general",
): HTMLElement {
  const pages = PAGES.map((p) => ({ value: p.id, label: t(p.label) }));
  const head = el(
    "div",
    { class: "quick-settings-head" },
    el("div", { class: "quick-label", text: t("Settings") }),
    pick(pages, page, (id) => (IN_PLACE.includes(id) ? on.show(id as QuickPage) : on.open(id)), relayout, true),
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
  /** A switch, as Settings draws one: its state said by aria-checked. */
  const swap = (onNow: boolean, change: (on: boolean) => void, label: string) => {
    const b = el("button", { class: `quick-switch${onNow ? " on" : ""}`, onclick: () => change(!onNow) }, el("span"));
    b.setAttribute("role", "switch");
    b.setAttribute("aria-checked", String(onNow));
    b.setAttribute("aria-label", label);
    return b;
  };
  if (page === "activity")
    return el(
      "div",
      { class: "card quick quick-settings" },
      head,
      row(t("Local history"), t("Your agents' turns, counted on this computer; nothing leaves it."), swap(s.history, (v) => on.setHistory(v), t("Local history"))),
      foot(t("All settings"), () => on.open("activity")),
    );
  if (page === "flock")
    return el(
      "div",
      { class: "card quick quick-settings" },
      head,
      row("Zeca", t("Your optional companion: off, the flock and the cards work as ever."), swap(s.zeca, (v) => on.setZeca(v), "Zeca")),
      row(
        t("The flock draws from"),
        t("Real species, each with its own movements."),
        pick(
          FLOCKS.map((f) => ({ value: f.value, label: t(f.label) })),
          s.flock,
          (f) => on.setFlock(f),
          relayout,
        ),
      ),
      foot(t("All settings"), () => on.open("flock")),
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
