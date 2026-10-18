import { el } from "../../../dom";
import { t, tk } from "../../../i18n";
import { icon, type IconName } from "../../../island/icons";
import { sidePanel } from "../panels";
import { createDictationHistory, type DictationBackend } from "./history";
import { createYourWords, type WordsBackend } from "./words";
import type { DictationPeriod, DictationStatistics } from "../../../bridge";

export type DictationAreaBackend = DictationBackend & WordsBackend & { statistics(period: DictationPeriod): Promise<DictationStatistics> };
export function createDictationArea(backend: DictationAreaBackend, settings: () => void, statistics: () => void) {
  const history = createDictationHistory(backend, settings);
  const words = createYourWords(backend, settings);
  let selected: "history" | "dictionary" | "snippets" = "history";
  const nav = el("nav", { class: "dictation-nav" });
  const groups = el("div", { class: "dictation-groups" });
  nav.append(groups);
  const panel = sidePanel(nav, "dictation", "Dictation", 210, "left", 180, 300);
  const element = el("div", { class: "dictation-area" }, nav, history.element, words.dictionary, words.snippets);
  const refresh = () => { if (selected === "history") history.refresh(); else words.refresh(); };
  const select = (id: typeof selected) => { selected = id; relabel(); refresh(); nav.querySelector<HTMLButtonElement>(`[data-dictation-page="${id}"]`)?.focus(); };
  const relabel = () => {
    panel.relabel(); history.relabel(); words.relabel();
    history.element.hidden = selected !== "history";
    words.dictionary.hidden = selected !== "dictionary";
    words.snippets.hidden = selected !== "snippets";
    nav.setAttribute("aria-label", t("Dictation pages"));
    const button = (label: string, glyph: IconName, click?: () => void) => {
      const b = el("button", { onclick: click }, icon(glyph, 14), el("span", { text: label }));
      b.disabled = !click;
      if (!click) b.title = t("Available in a later step.");
      return b;
    };
    const page = (id: typeof selected, text: string, glyph: IconName) => {
      const b = button(text, glyph, () => select(id));
      b.dataset.dictationPage = id;
      if (selected === id) b.setAttribute("aria-current", "page");
      return b;
    };
    const current = page("history", t("History"), "mic");
    groups.replaceChildren(el("h3", { text: t("Voice") }), current,
      ...([[tk("Voice commands"), "flock"], [tk("Talk with Zeca"), "chat"], [tk("Meetings"), "person"]] as [string, IconName][]).map(([label, glyph]) => button(t(label), glyph)),
      el("h3", { text: t("Your words") }),
      page("dictionary", t("Dictionary"), "file"), page("snippets", t("Snippets"), "tag"), button(t("Styles"), "pencil"),
      el("h3", { text: t("System") }), button(t("Dictation settings"), "gear", settings), button(t("Statistics"), "bars", statistics));
  };
  relabel();
  return { element, refresh, relabel, invalidate: history.invalidate };
}
