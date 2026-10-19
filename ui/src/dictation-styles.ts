import type { DictationOutputLanguage, DictationStyle } from "./bridge";
import { el } from "./dom";
import { t, tk } from "./i18n";

export type CleanupSettings = { dictationCleanup: boolean; dictationStyle: DictationStyle; dictationWriteIn: DictationOutputLanguage | null; dictationSessionsEnglish: boolean };
export const STYLES: { id: DictationStyle; name: string; about: string }[] = [
  { id: "verbatim", name: tk("As spoken"), about: tk("No model request. Keep local words, snippets and spoken-code processing.") },
  { id: "natural", name: tk("Natural"), about: tk("Remove hesitations and repetitions while keeping your voice.") },
  { id: "polished", name: tk("Polished"), about: tk("Use full sentences and written punctuation.") },
  { id: "prompt", name: tk("Prompt"), about: tk("Organize an instruction for the agent. Never answer or run it.") },
];
export const OUTPUT_LANGUAGES: { id: DictationOutputLanguage; name: string; applied: string }[] = [
  { id: "en", name: tk("English"), applied: tk("In English") },
  { id: "pt-BR", name: tk("Brazilian Portuguese"), applied: tk("In Brazilian Portuguese") },
  { id: "es", name: tk("Spanish"), applied: tk("In Spanish") },
  { id: "zh", name: tk("Simplified Chinese"), applied: tk("In Simplified Chinese") },
];
export function styleCards(on: boolean, selected?: DictationStyle, choose?: (style: DictationStyle) => void) {
  const cards = el("div", { class: "dictation-style-cards" });
  for (const style of STYLES) {
    const card = choose ? el("button", { class: "dictation-style-card", onclick: () => choose(style.id) }) : el("article", { class: "dictation-style-card" });
    if (choose) { card.setAttribute("aria-pressed", String(style.id === selected)); card.dataset.styleControl = `style-${style.id}`; }
    else if (selected === style.id) card.dataset.selected = "true";
    card.append(el("strong", { text: t(style.name) }), el("span", { text: on ? t(style.about) : t("Clean-up is off. Every line keeps its local text.") }));
    cards.append(card);
  }
  return cards;
}
export function appliedStyle(style: string): string | null {
  const known = STYLES.find(s => s.id === style);
  if (known) return t(known.name);
  return ({ "your-words": t("Your words"), "spoken-code": t("Spoken code"), "your-words+spoken-code": t("Your words and spoken code") } as Record<string, string>)[style] ?? null;
}
export function appliedLanguage(code: string | null | undefined): string | null {
  const language = OUTPUT_LANGUAGES.find(language => language.id === code);
  return language ? t(language.applied) : null;
}
