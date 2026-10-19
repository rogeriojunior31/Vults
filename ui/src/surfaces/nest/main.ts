import { dictationStatus } from "../../island/dictation";
import { applyTheme } from "../../theme";
// The Nest: the Panel and Full sizes (ADR 0021), a window built when opened and gone when closed
// (ADR 0021, D6). It opens with Zeca off too, without its Chat. Its Chat is the island's
// conversation in full Markdown: the same chat, so a message sent in either shows in both. Its
// cards, and the flock's, are answered here while the user is in it (ADR 0021).
import { Bridge } from "../../bridge";
import { setLang, t } from "../../i18n";
import { el } from "../../dom";
import { savedClose, savedTransition } from "./close";
import { createNest } from "./render";
import { shownKeys } from "../../island/views";
import { moltSeason, setMolt } from "../../character/director";
import { followClock, onDayPart } from "../../island/daypart";

const dictation = dictationStatus(true);
Bridge.onDictationOverlayReady(dictation.relocated);
const nest = createNest(
  document.getElementById("nest")!,
  {
    chat: {
      draft: Bridge.chatDraft,
      current: Bridge.chatCurrent,
      saveDraft: Bridge.chatDraftSet,
      send: (text, files, folder, mentions) => Bridge.chatSend(text, files, folder, mentions),
      decide: (id, answer) => void Bridge.chatDecide(id, answer),
      stop: () => void Bridge.chatStop(),
      reset: (provider) => Bridge.chatReset(provider),
      keyboard: () => {},
      models: (provider) => Bridge.chatModels(provider),
      pick: (folder) => Bridge.chatPick(folder),
      setPick: (folder, provider, pick) =>
        Bridge.setChatPick(folder, provider, pick),
      conversations: () => Bridge.chatConversations(),
      open: (id) => Bridge.chatOpen(id),
      rename: (id, title) => Bridge.chatRename(id, title),
      remove: (id) => Bridge.chatDelete(id),
      memory: (text, folder) => Bridge.memoryCommand(text, folder),
      handoff: (text, folder) => Bridge.continueCommand(text, folder),
      setAccess: (access) => Bridge.setChatAccess(access),
      openSettings: () => void Bridge.openSettings("chat"),
      engines: () => Bridge.chatEngines(),
      openFlock: () => void changeSize(async () => nest.open("flock")),
      pickFiles: (title) => Bridge.chatPickFiles(title),
      voice: {
        start: (tap) => Bridge.voiceStart(tap),
        stop: () => Bridge.voiceStop(),
        cancel: () => void Bridge.voiceCancel(),
      },
    },
    roosts: {
      query: Bridge.projectQuery,
      load: () => Bridge.roosts(),
      preview: (key) => Bridge.roostSharePreview(key),
      apply: (key, file, fingerprint) =>
        Bridge.roostShareApply(key, file, fingerprint),
      open: (item) => void Bridge.boardOpen("github", item),
      week: () => Bridge.activity(null).then((a) => a.week),
      focus: (kind, id) => {
        void Bridge.sessionFocus({ agent: kind, id });
        nest.open("flock");
      },
      preferences: () => void Bridge.openSettings("projects"),
    },
    memory: {
      load: () => Bridge.memoryView(),
      decide: (id, accept) => Bridge.memoryDecide(id, accept),
      undo: (id) => Bridge.memoryUndo(id),
    },
    dictation: {
      meetings: {
        status: Bridge.meetingStatus, transcript: Bridge.meetingTranscript, prepare: Bridge.meetingPrepare, begin: Bridge.meetingBegin,
        end: Bridge.meetingEnd, recover: Bridge.meetingRecover,
        summary: { settings: Bridge.appSettings, providers: Bridge.apiProviders, status: Bridge.meetingSummaryStatus,
          begin: Bridge.meetingSummaryBegin, cancel: Bridge.meetingSummaryCancel },
      },
      statistics: Bridge.dictationStatistics,
      load: Bridge.dictationHistory, remove: Bridge.dictationDelete,
      clear: Bridge.dictationClear, again: Bridge.dictationPasteAgain,
      words: Bridge.dictationWords, change: Bridge.changeDictationWords, trySentence: Bridge.dictationTry,
    },
    spend: () => Bridge.spendToday(),
    records: {
      query: Bridge.recordQuery,
      history: () => Bridge.nestHistory(),
      usage: () => Bridge.nestUsage(),
      traces: () => Bridge.nestTraces(),
      audit: () => Bridge.nestAudit(),
    },
    week: {
      activity: (monday) => Bridge.activity(monday),
      github: () => Bridge.githubCalendar(),
      saveImage: (name, png) => Bridge.saveRecapImage(name, png),
      settings: () => void Bridge.openSettings("activity"),
      connectors: () => void Bridge.openSettings("connectors"),
    },
    flock: {
      loadPlace: Bridge.flockPlace,
      savePlace: Bridge.flockPlaceSet,
      focus: (s) => void Bridge.sessionFocus(s && { agent: s.agent, id: s.id }),
      jump: (s) => void Bridge.sessionJump(s.agent, s.id),
      // The card waits on the Flock, its session in front (ADR 0021).
      review: (s) => {
        void Bridge.sessionFocus({ agent: s.agent, id: s.id });
        nest.open("flock");
      },
      decide: (request, decision) => void Bridge.decide(request, decision),
      decideAlways: (request) => void Bridge.decideAlways(request),
      answer: (request, answers) => void Bridge.questionAnswer(request, answers),
      release: (request) => void Bridge.questionRelease(request),
      skip: (request) => void Bridge.cardSkip(request),
      diff: (s, step) => Bridge.stepDiff(s.agent, s.id, step),
      usage: () => Bridge.usage(),
      hush: (s, hush) => void Bridge.sessionHush(s.agent, s.id, hush),
      openFolder: (s) => void Bridge.sessionOpenFolder(s.agent, s.id),
      projectPref: (s, pref, on) => void Bridge.sessionProjectPref(s.agent, s.id, pref, on),
    },
    connections: {
      loadPlace: Bridge.connectionsPlace,
      savePlace: Bridge.connectionsPlaceSet,
      open: (connector, item) => void Bridge.boardOpen(connector, item),
      status: () => Bridge.connectorsStatus(),
      refresh: () => Bridge.connectorsRefresh(),
      settings: () => void Bridge.openSettings("connectors"),
      merge: (connector, item) => Bridge.boardMerge(connector, item),
      onMerged: (cb) => Bridge.onRowMerged(cb),
    },
    settings: (section) => void Bridge.openSettings(section),
    savePlace: Bridge.nestTabSet,
    openDictation: () => void Bridge.openNest("dictation"),
    setZeca: (on) => void Bridge.setZeca(on),
    // Back to the island is Quick: the Nest closes and the island opens (ADR 0021).
    island: () => void changeSize(() => Bridge.setSize("quick", nest.current())),
    sound: (on) => void Bridge.setSounds(on),
    media: (action) => void Bridge.mediaControl(action),
    endDnd: () => void Bridge.setDnd(null).catch(() => {}),
    look: () => void Bridge.openSettings("flock"),
    size: (size) => void changeSize(() => Bridge.setSize(size, nest.current())),
  },
  false,
);
const closeError = el("p", { class: "nest-close-error" });
closeError.hidden = true;
closeError.setAttribute("role", "alert");
document.getElementById("nest")!.append(closeError);
const closingFailed = () => {
  closeError.textContent = t("Could not close the window. Your draft stays here; try again.");
  closeError.hidden = false;
  // The Nest also saves when Settings is visible in their shared window.
  void Bridge.openNest(nest.current()).catch(() => {});
};
const flush = async () => {
  closeError.hidden = true;
  await nest.flushState();
};
const changeSize = savedTransition(flush, () => {
  closeError.textContent = t("Could not change the view. Your draft stays here; try again.");
  closeError.hidden = false;
});
void Bridge.onNestClose(savedClose(
  flush,
  Bridge.closeNest,
  closingFailed,
)).catch(closingFailed);
Bridge.onNestFull((full) => nest.setFull(full));
void Bridge.nestIsFull().then((full) => nest.setFull(full), () => {});
// `open_nest` names the tab in the address when it builds the window, and by an event after.
let asked = location.hash.slice(1);
nest.open(asked);
Bridge.onNestTab((tab) => {
  asked = tab;
  nest.open(tab);
});
Bridge.onMemory(() => nest.memoryChanged());
Bridge.onDictationHistory(erased => nest.dictationChanged(erased));
Bridge.onUsage((windows) => { nest.chat.setUsage(windows); nest.usageChanged(); });
Bridge.onSpend(() => nest.spendChanged());

let lastView: Parameters<typeof nest.view>[0] | null = null;
const dictationContext = () => dictation.context(lastView ?? { sessions: [], approval: null }, nest.chat.hasWaitingApproval());
Bridge.onChat((d) => { nest.chat.receive(d); dictationContext(); });
// Allow and Deny's keys come here while the user is in the Nest (ADR 0021).
Bridge.onShortcut((id) => nest.shortcut(id));
void Bridge.shortcutKeys().then((keys) => nest.setKeys(shownKeys(keys)), () => {});
Bridge.onShortcutKeys((keys) => nest.setKeys(shownKeys(keys)));
// The song in the header, as on the island.
Bridge.onMedia((now) => nest.setMedia(now));
void Bridge.mediaNow().then((now) => nest.setMedia(now), () => {});
// The Nest's paperclip: the files it picked, never the island's.
Bridge.onFiles((d) => d.to === "nest" && nest.chat.attach(d.copied, d.refused));
Bridge.onChatSync((s) => { nest.chat.sync(s); dictationContext(); });
Bridge.onVoiceLevel((level) => nest.chat.voiceLevel(level));
Bridge.onVoiceSilence(() => nest.chat.voiceSilence());
Bridge.onVoicePartial((text) => nest.chat.voicePartial(text));
void Bridge.voiceStatus().then((v) => nest.chat.setVoiceReady(v.ready), () => {});
// The sessions' folders, for a new conversation (the view comes now, then with each change).
// The hour turning changes what the habitat's idle birds do (sunning, sleeping).
onDayPart(() => lastView && nest.view(lastView));
Bridge.onView((v) => {
  lastView = v;
  dictationContext();
  nest.chat.setSessions(v.sessions);
  nest.chat.setOffers(v.chat ?? null);
  const folders = v.sessions.flatMap((s) =>
    s.cwd ? [{ cwd: s.cwd, project: s.project }] : [],
  );
  nest.chat.setFolders(folders, null);
  nest.view(v);
});
void Bridge.appSettings().then((s) => {
  dictation.captions(s.dictationCaption, true);
  applyTheme(s.theme ?? "black", true);
  setLang(s.lang);
  dictation.relabel();
  nest.setZeca(s.zeca);
  nest.setSound(s.sounds);
  nest.setSpecies(s.zecaSpecies);
  nest.chat.setNative(s.nativeChat);
  nest.chat.setAccess(s.chatAccess);
  // Asked for the Chat before Zeca was known to be on.
  // Not if the user went elsewhere meanwhile.
  if (s.zeca && ["chat", "memory"].includes(asked) && nest.current() === "flock") nest.open(asked);
});
void Bridge.apiKeyStatus().then((api) => nest.chat.setApi(api));
Bridge.onSettings((s) => {
  if (s.meetingSummary !== undefined || s.api !== undefined || s.dictationCleanup !== undefined || s.dictationStyle !== undefined || s.dictationWriteIn !== undefined || s.dictationSessionsEnglish !== undefined) nest.dictationChanged();
  if (s.dictationKeep !== undefined) nest.dictationChanged(true);
  if (s.dictationCaption !== undefined) dictation.captions(s.dictationCaption);
  applyTheme(s.theme);
  if (s.lang !== undefined) {
    setLang(s.lang);
    dictation.relabel();
    nest.relabel();
  }
  if (s.zeca !== undefined) nest.setZeca(s.zeca);
  if (s.sounds !== undefined) nest.setSound(s.sounds);
  if (s.zecaSpecies !== undefined) nest.setSpecies(s.zecaSpecies);
  if (s.api !== undefined) nest.chat.setApi(s.api);
  if (s.nativeChat !== undefined) nest.chat.setNative(s.nativeChat);
  if (s.chatAccess !== undefined) nest.chat.setAccess(s.chatAccess);
  if (s.voice !== undefined) nest.chat.setVoiceReady(s.voice);
});
// The flock's flights molt in summer, as on the island.
setMolt(moltSeason(new Date()));
window.setInterval(() => setMolt(moltSeason(new Date())), 3_600_000);

// The idle flock keeps a vulture's day by the local clock (dawn, day, dusk, night).
followClock();
