import { el } from "./dom";
import { t, tk } from "./i18n";
import { icon } from "./island/icons";
import { searchText } from "./surfaces/nest/dictation/search";

// These phrases are examples from the bounded native catalog, never executable suggestions.
const CATEGORIES = [
  { id: "move", label: tk("Navigation"), title: tk("Move around"), examples: [tk("next session"), tk("open api"), tk("go to the api terminal"), tk("open the web folder")],
    detail: tk("Say: next session; previous; open api; go to the api terminal; open the web folder.") },
  { id: "ask", label: tk("Flock questions"), title: tk("Ask about the flock"), examples: [tk("who is waiting?"), tk("what is api doing?"), tk("how much of my plan is left?"), tk("how are my pull requests?")],
    detail: tk("Say: who is waiting?; what is api doing?; how much of my plan is left?; how are my pull requests?") },
  { id: "projects", label: tk("Projects"), title: tk("Project notifications and visibility"), examples: [tk("mute notifications for web"), tk("pin api"), tk("hide docs"), tk("unhide docs")],
    detail: tk("Say: mute notifications for web; unmute notifications for web; pin api; unpin api; hide docs; unhide docs.") },
  { id: "media", label: tk("Media"), title: tk("Media voice commands"), examples: [tk("pause media"), tk("resume music"), tk("next track"), tk("media volume 30")],
    detail: tk("Enable media voice commands and media controls first. Say: pause media; resume music; next track; previous track; media volume 30; lower media volume. Uses the selected player. Project mute only silences notifications.") },
  { id: "computer", label: tk("Computer audio"), title: tk("System audio voice commands"), examples: [tk("mute computer"), tk("unmute computer"), tk("computer volume 40"), tk("raise computer volume")],
    detail: tk("Enable system audio voice commands first. Say: mute computer; unmute computer; computer volume 40; raise computer volume; lower computer volume. Changes use the default output, not the microphone. Volume changes do not unmute it.") },
  { id: "apps", label: tk("Applications"), title: tk("Application voice commands"), examples: [tk("open Obsidian"), tk("open app Firefox"), tk("open project api")],
    detail: tk("Enable application voice commands first. Say: open Obsidian; open app Firefox; open project api. Use an installed desktop name. Ambiguous names need a choice. Application launches cannot be repeated or undone.") },
  { id: "repeat", label: tk("Again and undo"), title: tk("Again and undo"), examples: [tk("and docs too"), tk("same for web"), tk("undo")],
    detail: tk("Say: and docs too; same for web; undo. Only flock commands support repeat and undo, with 90 seconds of context. Opening a terminal or folder and reading a snapshot cannot be undone.") },
  { id: "safety", label: tk("Limits"), title: tk("Never answers a card"), examples: [],
    detail: tk("Approve, allow, yes and other card answers are refused, even with Chat fallback on. Answer the card with a click or an authorized shortcut.") },
];

export type CommandHelpState = { query: string; scope: string; notes?: string[] };

/** A searchable catalog. Choosing an example fills the local checker; it never runs it. */
export function voiceCommandHelp(state: CommandHelpState = { query: "", scope: "move" }, choose?: (phrase: string) => void): HTMLElement {
  const query = el("input"); query.type = "search"; query.value = state.query;
  query.dataset.setting = "command-example-search";
  query.placeholder = t("Search command examples"); query.setAttribute("aria-label", t("Search command examples"));
  const picker = el("select", { class: "command-category-picker" });
  picker.dataset.setting = "command-example-scope"; picker.setAttribute("aria-label", t("Command category"));
  const categories = el("div", { class: "command-categories" }); categories.setAttribute("role", "group"); categories.setAttribute("aria-label", t("Command category"));
  const detail = el("div", { class: "command-category-detail" });
  detail.setAttribute("role", "region");
  const empty = el("div", { class: "command-catalog-empty" },
    el("p", { text: t("No command examples match. Try another search.") }),
    el("button", { text: t("Clear search"), onclick: () => { query.value = state.query = ""; draw(); query.focus(); } }));
  empty.setAttribute("role", "status");
  const root = el("div", { class: "voice-command-help" },
    el("p", { class: "command-catalog-language", text: t("Commands understand English, Portuguese and Spanish. Use a current project or agent name in place of api, web or docs.") }),
    el("div", { class: "voice-command-search" }, query, picker),
    el("div", { class: "command-catalog" }, categories, detail), empty);
  const draw = () => {
    const focused = (document.activeElement as HTMLElement | null)?.dataset.setting;
    const terms = searchText(state.query).trim().split(/\s+/u).filter(Boolean);
    const matches = CATEGORIES.filter(category => {
      const text = searchText([t(category.label), t(category.title), t(category.detail), ...category.examples.map(example => t(example))].join(" "));
      return terms.every(term => text.includes(term));
    });
    const selected = matches.find(category => category.id === state.scope) ?? matches[0];
    if (selected) state.scope = selected.id;
    categories.replaceChildren(); picker.replaceChildren(); detail.replaceChildren();
    empty.hidden = matches.length > 0; categories.hidden = detail.hidden = matches.length === 0; picker.disabled = matches.length === 0;
    for (const category of matches) {
      const button = el("button", { class: "command-category", onclick: () => { state.scope = category.id; draw(); } },
        el("span", { text: t(category.label) }), icon("chevron", 12));
      button.dataset.setting = `command-category-${category.id}`;
      button.setAttribute("aria-pressed", String(category === selected));
      categories.append(button);
      const option = el("option", { text: t(category.label) }); option.value = category.id; option.selected = category === selected; picker.append(option);
    }
    picker.value = selected?.id ?? "";
    if (selected) {
      detail.setAttribute("aria-label", t(selected.title));
      detail.append(el("h4", { text: t(selected.title) }));
      if (selected.examples.length) {
        detail.append(el("p", { class: "command-example-hint", text: t("Choose an example to edit in the checker. Nothing runs.") }));
        const phrases = el("div", { class: "command-phrases" });
        for (const example of selected.examples) {
          const phrase = t(example);
          const item = choose ? el("button", { class: "command-phrase", onclick: () => choose(phrase) }, el("span", { text: phrase }), icon("plus", 14))
            : el("div", { class: "command-phrase" }, el("span", { text: phrase }));
          item.dataset.setting = `command-example-${selected.id}-${selected.examples.indexOf(example)}`;
          if (choose) item.setAttribute("aria-label", t("Use example: {phrase}", { phrase }));
          phrases.append(item);
        }
        detail.append(phrases);
        const notes = el("details", { class: "command-category-notes" }, el("summary", { text: t("Requirements and more examples") }), el("p", { text: t(selected.detail) }));
        notes.dataset.setting = `command-notes-${selected.id}`;
        notes.querySelector("summary")!.dataset.setting = `command-notes-summary-${selected.id}`;
        notes.open = state.notes?.includes(selected.id) ?? false;
        notes.addEventListener("toggle", () => { state.notes = notes.open ? [...new Set([...(state.notes ?? []), selected.id])] : (state.notes ?? []).filter(id => id !== selected.id); });
        detail.append(notes);
      } else detail.append(el("p", { class: "command-category-limit", text: t(selected.detail) }));
      if (["move", "ask", "projects", "repeat"].includes(selected.id)) detail.querySelector("details")?.append(el("p", { class: "command-catalog-note", text: t("Local answers use the latest snapshot Vults already has. They do not refresh a connector or call a model.") }));
    }
    if (focused && focused !== "command-example-search" && focused !== "command-example-scope") root.querySelector<HTMLElement>(`[data-setting="${focused}"]`)?.focus({ preventScroll: true });
  };
  query.addEventListener("input", () => { state.query = query.value; draw(); });
  query.addEventListener("keydown", event => {
    if (event.key === "Escape" && query.value && !event.isComposing && event.keyCode !== 229) {
      event.preventDefault(); event.stopPropagation(); query.value = state.query = ""; draw();
    }
  });
  picker.addEventListener("change", () => { state.scope = picker.value; draw(); });
  categories.addEventListener("keydown", event => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    const buttons = [...categories.querySelectorAll<HTMLButtonElement>("button")];
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement); if (index < 0) return;
    event.preventDefault();
    const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
    buttons[next]?.focus();
  });
  draw();
  return root;
}
