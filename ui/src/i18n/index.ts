// The app's words in the user's language: `t("Open terminal")` is the English, and the key. The
// catalogs (pt-BR.ts, es.ts, zh.ts) map each English sentence to its translation; a sentence one
// lacks shows in English. `{name}` slots are filled from `vars`, so each language orders its own
// words. `scripts/check-i18n.sh` fails when a `t()` sentence is missing from a catalog.
import es from "./es";
import ptBR from "./pt-BR";
import zh from "./zh";

export type Lang = "en" | "pt-BR" | "es" | "zh";

const CATALOGS: Record<Exclude<Lang, "en">, Record<string, string>> = { "pt-BR": ptBR, es, zh };

/** The languages offered, each named in itself. */
export const LANGUAGES: { code: Lang; name: string }[] = [
  { code: "en", name: "English" },
  { code: "pt-BR", name: "Português (Brasil)" },
  { code: "es", name: "Español" },
  { code: "zh", name: "中文（简体）" },
];

let lang: Lang = "en";

/** The language in use, as the app resolved it (the choice, else the system's). */
export function setLang(code: string): void {
  lang = (LANGUAGES.find((l) => l.code === code)?.code ?? "en") as Lang;
  document.documentElement.lang = lang;
}

export const currentLang = (): Lang => lang;

/** For dates and numbers (`toLocaleDateString`). */
export const locale = (): string => (lang === "zh" ? "zh-CN" : lang);

export function t(english: string, vars?: Record<string, string | number>): string {
  const text = lang === "en" ? english : (CATALOGS[lang][english] ?? english);
  return vars ? text.replace(/\{(\w+)\}/g, (all, name: string) => (name in vars ? String(vars[name]) : all)) : text;
}
