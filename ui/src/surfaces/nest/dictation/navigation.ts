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

/** Both webviews draw one navigation; the destination keeps its existing authority. */
export function createDictationNavigation(selected: string, work: (page: DictationWorkPage) => void,
  settings: (page: DictationSetting) => void, statistics: () => void) {
  const groups = el("div", { class: "dictation-groups" });
  const picker = el("select", { class: "dictation-page-picker" });
  picker.dataset.setting = "dictation-page-picker";
  const nav = el("nav", { class: "dictation-nav" }, picker, groups);
  const panel = sidePanel(nav, "dictation", "Dictation", 210, "left", 180, 300);
  const relabel = () => {
    const active = document.activeElement as HTMLElement | null;
    const focused = active && nav.contains(active) ? active === picker ? "picker" : active.dataset.dictationPage : null;
    panel.relabel();
    nav.setAttribute("aria-label", t("Dictation pages")); picker.setAttribute("aria-label", t("Dictation pages"));
    const entries: { id: string; label: string; glyph: IconName; click: () => void }[] = [
      { id: "history", label: t("History"), glyph: "mic", click: () => work("history") },
      { id: "commands", label: t("Voice commands"), glyph: "flock", click: () => work("commands") },
      { id: "dictionary", label: t("Dictionary"), glyph: "file", click: () => work("dictionary") },
      { id: "snippets", label: t("Snippets"), glyph: "tag", click: () => work("snippets") },
      { id: "meetings", label: t("Meetings"), glyph: "mic", click: () => work("meetings") },
      { id: "conversation", label: t("Voice conversation"), glyph: "chat", click: () => work("conversation") },
      ...DICTATION_SETTINGS.map(p => ({ id: `dictation${p.id}`, label: t(p.label), glyph: p.icon as IconName, click: () => settings(p.id) })),
      { id: "statistics", label: t("Statistics"), glyph: "bars", click: statistics },
    ];
    groups.replaceChildren(); picker.replaceChildren();
    for (const [i,entry] of entries.entries()) {
      if (i === 0 || i === DICTATION_WORK_PAGES.length) groups.append(el("h3", { text: i === 0 ? t("Work") : t("Dictation settings") }));
      const b = el("button", { onclick: entry.click }, icon(entry.glyph,14), el("span", { text: entry.label }));
      b.dataset.dictationPage = entry.id; b.dataset.setting = `dictation-nav-${entry.id}`;
      if (entry.id === selected) b.setAttribute("aria-current", "page");
      groups.append(b);
      const option = el("option", { text: entry.label }); option.value = entry.id; option.selected = entry.id === selected; picker.append(option);
    }
    picker.onchange = () => entries.find(p => p.id === picker.value)?.click();
    if (focused) (focused === "picker" ? picker : nav.querySelector<HTMLElement>(`[data-dictation-page="${focused}"]`))?.focus({ preventScroll: true });
  };
  relabel();
  return { element: nav, relabel, select(id: string) { selected = id; relabel(); } };
}
