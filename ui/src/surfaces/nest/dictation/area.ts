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
export function createDictationArea(backend: DictationAreaBackend, settings: (section?: string) => void, characterChanged: () => void = () => {}, placeChanged: () => void = () => {}) {
  const openSettings = (section?: string) => { if (selected === "meetings") meetings.beforeLeave(() => settings(section)); else settings(section); };
  const configure = () => openSettings();
  const history = createDictationHistory(backend, configure);
  const commands = createDictationHistory(backend, () => openSettings("dictationcommands"), true, () => showCommands("explore"));
  const readiness = createCommandReadiness(backend.commandReadiness ?? (async () => { throw new Error("Command setup is unavailable."); }), openSettings);
  const commandCheck = createCommandCheck(backend.commandCheck ?? (async () => { throw new Error("Command check is unavailable."); }));
  (commandCheck.element as HTMLDetailsElement).open = true;
  const helpState = { query: "", scope: "move" };
  const helpTitle = el("h3");
  const help = el("section", { class: "voice-command-examples" });
  const names = el("button", { onclick: () => select("dictionary") });
  const helpHeader = el("header", { class: "command-catalog-heading" }, helpTitle, names);
  const workbench = el("div", { class: "voice-command-workbench" }, commandCheck.element, help);
  const explore = el("div", { class: "command-explore" }, workbench, readiness.element);
  const recentButton = el("button"), exploreButton = el("button");
  const commandViews = el("nav", { class: "command-views" }, recentButton, exploreButton);
  let commandView: "recent" | "explore" = "recent";
  const showCommands = (view: typeof commandView) => {
    commandView = view;
    commands.element.classList.toggle("command-view-recent", view === "recent");
    commands.element.classList.toggle("command-view-explore", view === "explore");
    recentButton.setAttribute("aria-pressed", String(view === "recent"));
    exploreButton.setAttribute("aria-pressed", String(view === "explore"));
    explore.hidden = view !== "explore";
    commands.setRecent(view === "recent");
    readiness.visible(active && visible && selected === "commands" && view === "explore");
    commandCheck.visible(active && visible && selected === "commands" && view === "explore");
  };
  recentButton.addEventListener("click", () => showCommands("recent")); exploreButton.addEventListener("click", () => showCommands("explore"));
  commands.element.querySelector(".dictation-heading")!.after(commandViews);
  commands.element.querySelector(".dictation-hero")!.after(explore);
  const words = createYourWords(backend, () => openSettings("dictationwords"));
  const meetings = createMeetings(backend.meetings, () => openSettings("dictationvoice"), () => openSettings("dictationmeetings"));
  const conversation = createVoiceConversation(backend.conversation, () => openSettings("dictationconversation"), characterChanged);
  let active = false, visible = !document.hidden;
  let selected: DictationWorkPage = "history";
  const navigation = createDictationNavigation(selected, id => select(id), id => openSettings(`dictation${id}`));
  const nav = navigation.element;
  const element = el("div", { class: "dictation-area" }, nav, history.element, commands.element, words.dictionary, words.snippets, meetings.element, conversation.element);
  const refresh = () => { conversation.setActive(active && visible && selected === "conversation"); if (selected === "conversation") { meetings.setActive(false); return; } meetings.setActive(active && selected === "meetings"); if (selected === "meetings") { meetings.refresh(); return; } if (selected === "history") history.refresh(); else if (selected === "commands") commands.refresh(); else words.refresh(); };
  const applySelect = (id: DictationWorkPage) => { const changed = selected !== id; selected = id; relabel(); refresh(); if (changed) placeChanged(); };
  const select = (id: DictationWorkPage) => { if (id !== selected && selected === "meetings") meetings.beforeLeave(() => applySelect(id)); else applySelect(id); };
  const relabel = () => {
    readiness.relabel(); commandCheck.relabel();
    readiness.visible(active && visible && selected === "commands" && commandView === "explore"); commandCheck.visible(active && visible && selected === "commands" && commandView === "explore");
    navigation.select(selected); history.relabel(); commands.relabel(); words.relabel(); meetings.relabel(); conversation.relabel();
    const focused = document.activeElement === helpTitle;
    const activeControl = help.contains(document.activeElement) ? (document.activeElement as HTMLElement)?.dataset.setting : undefined;
    const selection = activeControl === "command-example-search" ? [(document.activeElement as HTMLInputElement).selectionStart, (document.activeElement as HTMLInputElement).selectionEnd] : null;
    helpTitle.textContent = t("What you can say");
    help.replaceChildren(helpHeader, voiceCommandHelp(helpState, commandCheck.fill));
    if (focused) helpTitle.focus({ preventScroll: true });
    if (activeControl) {
      const control = help.querySelector<HTMLInputElement>(`[data-setting="${activeControl}"]`);
      control?.focus({ preventScroll: true });
      if (selection && selection[0] !== null && selection[1] !== null) control?.setSelectionRange(selection[0]!, selection[1]!);
    }
    names.textContent = t("Dictionary");
    recentButton.textContent = t("Recent commands"); exploreButton.textContent = t("Explore and test");
    commandViews.setAttribute("aria-label", t("Voice commands")); showCommands(commandView);
    conversation.element.hidden = selected !== "conversation";
    meetings.element.hidden = selected !== "meetings";
    commands.element.hidden = selected !== "commands";
    history.element.hidden = selected !== "history";
    words.dictionary.hidden = selected !== "dictionary";
    words.snippets.hidden = selected !== "snippets";
  };
  relabel();
  return { element, refresh, relabel, select, beforeLeave: (proceed: () => void) => { if (selected === "meetings") meetings.beforeLeave(proceed); else proceed(); }, current: () => selected, setKeys: (keys: Record<string, string>) => { history.setKeys(keys); commands.setKeys(keys); readiness.setKeys(keys); }, character: conversation.character, perch: conversation.perch,
    visible(on: boolean) { visible = on; readiness.visible(active && visible && selected === "commands" && commandView === "explore"); commandCheck.visible(active && visible && selected === "commands" && commandView === "explore"); conversation.setActive(active && visible && selected === "conversation"); },
    setActive(on: boolean) { active = on; readiness.visible(active && visible && selected === "commands" && commandView === "explore"); commandCheck.visible(active && visible && selected === "commands" && commandView === "explore"); conversation.setActive(on && visible && selected === "conversation"); meetings.setActive(on && selected === "meetings"); },
    commandTargetsChanged: commandCheck.targets,
    commandContextChanged() { readiness.invalidate(); commandCheck.invalidate(); },
    meetingsErased: meetings.invalidate, invalidate: () => { history.invalidate(); commands.invalidate(); } };
}
