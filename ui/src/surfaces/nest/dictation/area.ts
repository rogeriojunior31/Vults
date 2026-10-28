import { createVoiceConversation, type ConversationBackend } from "./conversation";
import { createMeetings, type MeetingBackend } from "./meetings";
import { createDictationNavigation, type DictationWorkPage } from "./navigation";
import { createCommandCheck } from "../../../voice-command-check";
import type { CommandPreview } from "../../../bridge";
import { createCommandReadiness, type CommandReadiness } from "../../../voice-command-readiness";
import { voiceCommandHelp } from "../../../voice-command-help";
import { el } from "../../../dom";
import { t } from "../../../i18n";
import { createDictationHistory, type DictationBackend } from "./history";
import { createYourWords, type WordsBackend } from "./words";
import type { DictationPeriod, DictationStatistics } from "../../../bridge";

export type DictationAreaBackend = DictationBackend & WordsBackend & { statistics(period: DictationPeriod): Promise<DictationStatistics>; commandReadiness?(): Promise<CommandReadiness>; commandCheck?(text: string): Promise<CommandPreview>; conversation?: ConversationBackend; meetings?: MeetingBackend };
export function createDictationArea(backend: DictationAreaBackend, settings: (section?: string) => void, statistics: () => void, characterChanged: () => void = () => {}, placeChanged: () => void = () => {}) {
  const configure = () => settings();
  const history = createDictationHistory(backend, configure);
  const commands = createDictationHistory(backend, () => settings("dictationcommands"), true);
  const readiness = createCommandReadiness(backend.commandReadiness ?? (async () => { throw new Error("Command setup is unavailable."); }), settings);
  const commandCheck = createCommandCheck(backend.commandCheck ?? (async () => { throw new Error("Command check is unavailable."); }));
  const helpState = { query: "", scope: "all" };
  const helpTitle = el("summary");
  const help = el("details", { class: "voice-command-examples" }, helpTitle);
  const names = el("button", { onclick: () => select("dictionary") });
  const guidance = el("div", { class: "voice-command-guidance" }, help, names);
  commands.element.querySelector("header")!.after(readiness.element, commandCheck.element, guidance);
  const words = createYourWords(backend, configure);
  const meetings = createMeetings(backend.meetings, () => settings("dictationvoice"), () => settings("dictationmeetings"));
  const conversation = createVoiceConversation(backend.conversation, () => settings("dictationconversation"), characterChanged);
  let active = false, visible = !document.hidden;
  let selected: DictationWorkPage = "history";
  const navigation = createDictationNavigation(selected, id => select(id), id => settings(`dictation${id}`), statistics);
  const nav = navigation.element;
  const element = el("div", { class: "dictation-area" }, nav, history.element, commands.element, words.dictionary, words.snippets, meetings.element, conversation.element);
  const refresh = () => { conversation.setActive(active && visible && selected === "conversation"); if (selected === "conversation") { meetings.setActive(false); return; } meetings.setActive(active && selected === "meetings"); if (selected === "meetings") { meetings.refresh(); return; } if (selected === "history") history.refresh(); else if (selected === "commands") commands.refresh(); else words.refresh(); };
  const select = (id: DictationWorkPage) => { const changed = selected !== id; selected = id; relabel(); refresh(); if (changed) placeChanged(); };
  const relabel = () => {
    readiness.relabel(); commandCheck.relabel();
    readiness.visible(active && visible && selected === "commands"); commandCheck.visible(active && visible && selected === "commands");
    navigation.select(selected); history.relabel(); commands.relabel(); words.relabel(); meetings.relabel(); conversation.relabel();
    const open = help.open;
    const focused = document.activeElement === helpTitle;
    const activeControl = help.contains(document.activeElement) ? (document.activeElement as HTMLElement)?.dataset.setting : undefined;
    const selection = activeControl === "command-example-search" ? [(document.activeElement as HTMLInputElement).selectionStart, (document.activeElement as HTMLInputElement).selectionEnd] : null;
    helpTitle.textContent = t("What you can say");
    help.replaceChildren(helpTitle, voiceCommandHelp(helpState));
    if (focused) helpTitle.focus({ preventScroll: true });
    if (activeControl) {
      const control = help.querySelector<HTMLInputElement>(`[data-setting="${activeControl}"]`);
      control?.focus({ preventScroll: true });
      if (selection && selection[0] !== null && selection[1] !== null) control?.setSelectionRange(selection[0]!, selection[1]!);
    }
    help.open = open;
    names.textContent = t("Dictionary");
    conversation.element.hidden = selected !== "conversation";
    meetings.element.hidden = selected !== "meetings";
    commands.element.hidden = selected !== "commands";
    history.element.hidden = selected !== "history";
    words.dictionary.hidden = selected !== "dictionary";
    words.snippets.hidden = selected !== "snippets";
  };
  relabel();
  return { element, refresh, relabel, select, current: () => selected, setKeys: (keys: Record<string, string>) => { history.setKeys(keys); commands.setKeys(keys); readiness.setKeys(keys); }, character: conversation.character, perch: conversation.perch,
    visible(on: boolean) { visible = on; readiness.visible(active && visible && selected === "commands"); commandCheck.visible(active && visible && selected === "commands"); conversation.setActive(active && visible && selected === "conversation"); },
    setActive(on: boolean) { active = on; readiness.visible(active && visible && selected === "commands"); commandCheck.visible(active && visible && selected === "commands"); conversation.setActive(on && visible && selected === "conversation"); meetings.setActive(on && selected === "meetings"); },
    commandTargetsChanged: commandCheck.targets,
    commandContextChanged() { readiness.invalidate(); commandCheck.invalidate(); },
    meetingsErased: meetings.invalidate, invalidate: () => { history.invalidate(); commands.invalidate(); } };
}
