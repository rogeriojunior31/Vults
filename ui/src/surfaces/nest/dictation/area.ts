import { createMeetings, type MeetingBackend } from "./meetings";
import { createDictationNavigation, type DictationWorkPage } from "./navigation";
import { voiceCommandHelp } from "../../../voice-command-help";
import { el } from "../../../dom";
import { t } from "../../../i18n";
import { createDictationHistory, type DictationBackend } from "./history";
import { createYourWords, type WordsBackend } from "./words";
import type { DictationPeriod, DictationStatistics } from "../../../bridge";

export type DictationAreaBackend = DictationBackend & WordsBackend & { statistics(period: DictationPeriod): Promise<DictationStatistics>; meetings?: MeetingBackend };
export function createDictationArea(backend: DictationAreaBackend, settings: (section?: string) => void, statistics: () => void) {
  const configure = () => settings();
  const history = createDictationHistory(backend, configure);
  const commands = createDictationHistory(backend, () => settings("dictationcommands"), true);
  const help = el("details");
  const names = el("button", { onclick: () => select("dictionary") });
  const guidance = el("div", {}, help, names);
  commands.element.querySelector("header")!.after(guidance);
  const words = createYourWords(backend, configure);
  const meetings = createMeetings(backend.meetings, () => settings("dictationvoice"), () => settings("dictationmeetings"));
  let active = false;
  let selected: DictationWorkPage = "history";
  const navigation = createDictationNavigation(selected, id => select(id), id => settings(`dictation${id}`), statistics);
  const nav = navigation.element;
  const element = el("div", { class: "dictation-area" }, nav, history.element, commands.element, words.dictionary, words.snippets, meetings.element);
  const refresh = () => { meetings.setActive(active && selected === "meetings"); if (selected === "meetings") { meetings.refresh(); return; } if (selected === "history") history.refresh(); else if (selected === "commands") commands.refresh(); else words.refresh(); };
  const select = (id: DictationWorkPage) => { selected = id; relabel(); refresh(); };
  const relabel = () => {
    navigation.select(selected); history.relabel(); commands.relabel(); words.relabel(); meetings.relabel();
    const open = help.open;
    help.replaceChildren(el("summary", { text: t("What you can say") }), voiceCommandHelp());
    help.open = open;
    names.textContent = t("Dictionary");
    meetings.element.hidden = selected !== "meetings";
    commands.element.hidden = selected !== "commands";
    history.element.hidden = selected !== "history";
    words.dictionary.hidden = selected !== "dictionary";
    words.snippets.hidden = selected !== "snippets";
  };
  relabel();
  return { element, refresh, relabel, select, setActive(on: boolean) { active = on; meetings.setActive(on && selected === "meetings"); },
    meetingsErased: meetings.invalidate, invalidate: () => { history.invalidate(); commands.invalidate(); } };
}
