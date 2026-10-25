import { el } from "../../../dom";
import { t, tk } from "../../../i18n";
import { icon, type IconName } from "../../../island/icons";
import { sidePanel } from "../panels";

export const DICTATION_WORK_PAGES = ["history", "commands", "dictionary", "snippets", "meetings", "conversation"] as const;
export type DictationWorkPage = typeof DICTATION_WORK_PAGES[number];
export const DICTATION_SETTINGS = [
  { id: "voice", label: tk("Voice"), icon: "mic" },
  { id: "conversation", label: tk("Talk with Zeca"), icon: "chat" },
  { id: "keys", label: tk("Keyboard shortcuts"), icon: "terminal" },
  { id: "commands", label: tk("Command settings"), icon: "flock" },
  { id: "words", label: tk("Vocabulary"), icon: "file" },
  { id: "styles", label: tk("Styles"), icon: "pencil" },
  { id: "meetings", label: tk("Meeting settings"), icon: "mic" },
  { id: "agents", label: tk("Agents"), icon: "flock" },
  { id: "paste", label: tk("Paste"), icon: "sideRight" },
  { id: "privacy", label: tk("Privacy"), icon: "person" },
  { id: "diagnostics", label: tk("Diagnostics"), icon: "bars" },
] as const;
export type DictationSetting = typeof DICTATION_SETTINGS[number]["id"];
export const dictationSetting = (route: string): DictationSetting | null => route === "dictation" ? "voice"
  : DICTATION_SETTINGS.find(p => `dictation${p.id}` === route)?.id ?? null;

const SETTING_ABOUT: Record<DictationSetting, string> = {
  voice: tk("Choose your microphone, local model and spoken language."),
  conversation: tk("Set up voice conversations, listening and spending limits."),
  keys: tk("Choose how to start, keep and cancel a recording."),
  commands: tk("Control Vults by voice and choose when to use Chat."),
  words: tk("Improve recognition with your vocabulary and spoken code."),
  styles: tk("Choose how dictated text is cleaned up and formatted."),
  meetings: tk("Configure meeting capture, speakers and optional summaries."),
  agents: tk("Set up voice input for an agent's text questions."),
  paste: tk("Choose where your text goes after dictation."),
  privacy: tk("Manage local history, retention and deletion."),
  diagnostics: tk("Test your microphone and inspect capture and delivery problems."),
};
export const dictationSettingAbout = (id: DictationSetting) => t(SETTING_ABOUT[id]);

/** Both webviews draw one navigation; the destination keeps its existing authority. */
export function createDictationNavigation(selected: string, work: (page: DictationWorkPage) => void,
  settings: (page: DictationSetting) => void, statistics: () => void) {
  const groups = el("div", { class: "dictation-groups" });
  const search = el("input", { class: "dictation-page-search" });
  search.type = "search";
  search.dataset.setting = "dictation-page-search";
  const picker = el("select", { class: "dictation-page-picker" });
  picker.dataset.setting = "dictation-page-picker";
  const empty = el("p", { class: "dictation-nav-empty" });
  empty.setAttribute("role", "status");
  const nav = el("nav", { class: "dictation-nav" }, el("div", { class: "dictation-nav-controls" }, search, picker), groups, empty);
  const configurationTitle = el("summary");
  const configuration = el("details", { class: "dictation-nav-config" }, configurationTitle);
  let configurationExpanded = selected.startsWith("dictation") && selected !== "dictationstyles";
  configuration.addEventListener("toggle", () => { if (!search.value.trim()) configurationExpanded = configuration.open; });
  const panel = sidePanel(nav, "dictation", "Dictation", 210, "left", 180, 300);
  const normalize = (text: string) => text.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLocaleLowerCase();
  const relabel = () => {
    const active = document.activeElement as HTMLElement | null;
    const focusedConfiguration = active === configurationTitle;
    const focused = active && nav.contains(active) ? active.dataset.dictationPage : null;
    panel.relabel();
    nav.setAttribute("aria-label", t("Dictation pages")); picker.setAttribute("aria-label", t("Dictation pages"));
    search.placeholder = t("Find a page"); search.setAttribute("aria-label", t("Search Dictation pages"));
    const sections = [
      { label: t("Voice"), config: false, entries: [
        { id: "history", label: t("History"), glyph: "mic", click: () => work("history"), about: "" },
        { id: "commands", label: t("Voice commands"), glyph: "flock", click: () => work("commands"), about: "" },
        { id: "conversation", label: t("Voice conversation"), glyph: "chat", click: () => work("conversation"), about: "" },
        { id: "meetings", label: t("Meetings"), glyph: "mic", click: () => work("meetings"), about: "" },
      ] },
      { label: t("Your words"), config: false, entries: [
        { id: "dictionary", label: t("Dictionary"), glyph: "file", click: () => work("dictionary"), about: "" },
        { id: "snippets", label: t("Snippets"), glyph: "tag", click: () => work("snippets"), about: "" },
        { id: "dictationstyles", label: t("Styles"), glyph: "pencil", click: () => settings("styles"), about: dictationSettingAbout("styles") },
      ] },
      ...[
        { label: t("Voice and shortcuts"), ids: ["voice", "keys", "commands", "conversation", "meetings", "agents"] },
        { label: t("Words and output"), ids: ["words", "paste"] },
        { label: t("Privacy and support"), ids: ["privacy", "diagnostics"] },
      ].map(group => ({ label: group.label, config: true, entries: group.ids.map(id => {
        const page = DICTATION_SETTINGS.find(page => page.id === id)!;
        return { id: `dictation${page.id}`, label: t(page.label), glyph: page.icon,
          about: dictationSettingAbout(page.id), click: () => settings(page.id) };
      }) })),
      { label: t("System"), config: false, entries: [{ id: "statistics", label: t("Statistics"), glyph: "bars", click: statistics, about: "" }] },
    ];
    const query = normalize(search.value.trim());
    groups.replaceChildren(); picker.replaceChildren();
    configurationTitle.textContent = t("Dictation settings");
    configuration.replaceChildren(configurationTitle);
    configuration.open = configurationExpanded || !!query;
    let configCount = 0;
    let selectedVisible = false, count = 0;
    for (const section of sections) {
      const entries = section.entries.filter(entry => normalize(`${entry.label} ${entry.about} ${section.label}`).includes(query));
      if (!entries.length) continue;
      const group = el("section", { class: "dictation-nav-group" }, section.label === t("System") && configCount > 0 ? null : el("h3", { text: section.label }));
      group.setAttribute("aria-label", section.label);
      const options = el("optgroup"); options.label = section.label;
      for (const entry of entries) {
        const b = el("button", { onclick: entry.click }, icon(entry.glyph as IconName,14), el("span", { text: entry.label }));
        b.dataset.dictationPage = entry.id; b.dataset.setting = `dictation-nav-${entry.id}`;
        if (entry.id === selected) { b.setAttribute("aria-current", "page"); selectedVisible = true; }
        group.append(b);
        const option = el("option", { text: entry.label }); option.value = entry.id; option.selected = entry.id === selected; options.append(option);
        count++;
      }
      if (section.config) {
        configuration.append(group);
        if (++configCount === 1) groups.append(el("h3", { text: t("System") }), configuration);
      } else groups.append(group);
      picker.append(options);
    }
    if (!selectedVisible) {
      const placeholder = el("option", { text: t("Choose a page") });
      placeholder.value = ""; placeholder.disabled = true; placeholder.selected = true; picker.prepend(placeholder);
    }
    picker.value = selectedVisible ? selected : "";
    picker.disabled = count === 0;
    empty.textContent = t("No pages found. Try another search."); empty.hidden = count > 0;
    picker.onchange = () => sections.flatMap(section => section.entries).find(entry => entry.id === picker.value)?.click();
    if (focusedConfiguration) configurationTitle.focus({ preventScroll: true });
    if (focused) nav.querySelector<HTMLElement>(`[data-dictation-page="${focused}"]`)?.focus({ preventScroll: true });
  };
  search.addEventListener("input", relabel);
  search.addEventListener("keydown", event => {
    if (event.key === "Escape" && search.value) { event.preventDefault(); event.stopPropagation(); search.value = ""; relabel(); }
  });
  groups.addEventListener("keydown", event => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    const buttons = [...groups.querySelectorAll<HTMLButtonElement>("button")].filter(button => configuration.open || !configuration.contains(button));
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (index < 0) return;
    event.preventDefault();
    const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1
      : (index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
    buttons[next]?.focus();
  });
  relabel();
  return { element: nav, relabel, select(id: string) {
    if (id !== selected && id.startsWith("dictation") && id !== "dictationstyles") configurationExpanded = true;
    selected = id;
    relabel();
  } };
}
