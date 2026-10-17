// The island window: Tauri in, DOM out.
import { Bridge, type Place } from "../bridge";
import * as Dock from "./dock";
import { setZecaLook, setZecaSpecies } from "./flock";
import { resting, setNestHolds, setNestOpen } from "./fsm";
import { createIsland } from "./render";
import { moltSeason, setMolt } from "../character/director";
import { Sound } from "../sound";
import { setLang, t } from "../i18n";
import { followClock } from "./daypart";
import { savedTransition } from "../surfaces/nest/close";

Dock.restorePlace();
const island = createIsland(document.getElementById("island")!, {
  decide: (request, decision) => void Bridge.decide(request, decision),
  decideAlways: (request) => void Bridge.decideAlways(request),
  answer: (request, answers) => void Bridge.questionAnswer(request, answers),
  release: (request) => void Bridge.questionRelease(request),
  layout: (x, y, w, h) => Dock.layout(x, y, w, h, resting()),
  openAlert: (key) => void Bridge.alertOpen(key),
  jump: (agent, id) => void Bridge.sessionJump(agent, id),
  focus: (agent, id) => void Bridge.sessionFocus({ agent, id }),
  unfocus: () => void Bridge.sessionFocus(null),
  openFolder: (agent, id) => void Bridge.sessionOpenFolder(agent, id),
  openFile: (agent, id, step, file) => void Bridge.sessionOpenFile(agent, id, step, file),
  editorFound: () => Bridge.editorFound(),
  projectPref: (agent, id, pref, on) => void Bridge.sessionProjectPref(agent, id, pref, on),
  projectBird: (agent, id, species) => void Bridge.sessionProjectBird(agent, id, species),
  hush: (agent, id, hush) => void Bridge.sessionHush(agent, id, hush),
  endDnd: () => void Bridge.setDnd(null),
  dismissDigest: () => void Bridge.digestDismiss(),
  dismissRecap: () => void Bridge.recapDismiss(),
  openActivity: () => {
    void changeView(() => Bridge.openNest("week"));
    void Bridge.recapDismiss();
  },
  stepDiff: (agent, id, step) => Bridge.stepDiff(agent, id, step).catch(() => null),
  openSettings: (page) => void Bridge.openSettings(page),
  setSize: (size, tab) => void changeView(() => Bridge.setSize(size, tab)),
  quickSettings: () =>
    Bridge.appSettings().then((s) => ({ presence: s.presence, foldAfter: s.foldAfter, history: s.history, zeca: s.zeca, flock: s.flock })),
  setHistory: (on) => Bridge.setHistory(on),
  setFlock: (f) => Bridge.setFlock(f),
  setZecaSetting: (on) => Bridge.setZeca(on),
  setPresence: (p) => void Bridge.setPresence(p),
  setFoldAfter: (n) => void Bridge.setFoldAfter(n),
  // The week before is seven days back, whatever was done in it (the history snaps a day to its Monday).
  week: (previous) => Bridge.activity(previous ? weekAgo() : null).then((a) => ({ week: a.week, history: a.history })),
  // Refused (the app busy): the picker marks what is saved again.
  setLook: (look) =>
    void Bridge.setZecaLook(look).catch(() => Bridge.appSettings().then((s) => island.setLookSetting(s.zecaLook), () => {})),
  opened: () => void Bridge.connectorsRefresh(),
  media: (action) => void Bridge.mediaControl(action),
  setSounds: (on) => {
    Sound.setEnabled(on);
    void Bridge.setSounds(on);
  },
  dismissAlert: (key) => void Bridge.alertDismiss(key),
  openRow: (connector, item) => void Bridge.boardOpen(connector, item),
  mergeRow: (connector, item) => void Bridge.boardMerge(connector, item).catch(() => island.merged(connector, item, t("The app did not take it."))),
  connectorStatus: (id) => Bridge.connectorsStatus().then((all) => all.find((c) => c.id === id) ?? null),
  chat: {
    draft: Bridge.chatDraft,
    current: Bridge.chatCurrent,
    saveDraft: Bridge.chatDraftSet,
    send: (text, files, folder) => Bridge.chatSend(text, files, folder),
    decide: (id, answer) => void Bridge.chatDecide(id, answer),
    stop: () => void Bridge.chatStop(),
    reset: (provider) => Bridge.chatReset(provider),
    models: (provider) => Bridge.chatModels(provider),
    conversations: () => Bridge.chatConversations(),
    open: (id) => Bridge.chatOpen(id),
    rename: (id, title) => Bridge.chatRename(id, title),
    remove: (id) => Bridge.chatDelete(id),
    openNest: () => void changeView(() => Bridge.openNest("chat")),
    pickFiles: (title) => Bridge.chatPickFiles(title),
    memory: (text, folder) => Bridge.memoryCommand(text, folder),
    handoff: (text, folder) => Bridge.continueCommand(text, folder),
    pick: (folder) => Bridge.chatPick(folder),
    setPick: (folder, provider, pick) => Bridge.setChatPick(folder, provider, pick),
    keyboard: (on, exclusive) => void Bridge.islandKeyboard(on, exclusive),
    setAccess: (access) => Bridge.setChatAccess(access),
    openSettings: () => void Bridge.openSettings("chat"),
    engines: () => Bridge.chatEngines(),
    voice: {
      start: (tap) => Bridge.voiceStart(tap),
      stop: () => Bridge.voiceStop(),
      cancel: () => void Bridge.voiceCancel(),
    },
  },
});
const changeView = savedTransition(() => island.chat.flushDraft(), () => {
  island.chat.toggle(true);
  island.chat.receive({ kind: "error", message: t("Could not change the view. Your draft stays here; try again.") });
});
Dock.routeLayout((x, y, w, h) => void Bridge.layout(x, y, w, h));
island.render({ sessions: [], approval: null, alerts: [] });
// Where it lives first: only the Island preset says hello (by the panel it would open the island
// by the tray at every start; Quiet and Paused show nothing at rest). The hello waits for the
// name, and goes without it if the app cannot say. A new preset redraws at once (Paused empties
// the island).
const place = (p: Place) => {
  Dock.setPlace(p, resting);
  island.setPresence(p.presence);
  island.render(island.last());
};
const placed = Bridge.islandPlace().then(
  (p) => (place(p), p),
  () => null,
);
Bridge.onPlace(place);
// The Nest open is the same surface grown larger (ADR 0021): the island steps aside, and its
// region with it, until the Nest closes, goes back to Quick or is minimized.
const nestOpen = (open: boolean) => {
  // Its chat went to the Nest with Room or Full: the island lets it go, or it would stay open
  // over the Nest. A waiting card keeps it (D1).
  if (open && island.chat.isOpen()) island.chat.toggle(false);
  setNestOpen(open);
  nestShown = open;
  island.render(island.last());
};
Bridge.onNestOpen(nestOpen);
void Bridge.nestIsOpen().then(nestOpen, () => {});
// In the Nest the user answers a card there; leaving it, the island takes the card (ADR 0021).
const nestHolds = (holds: boolean) => {
  setNestHolds(holds);
  island.render(island.last());
};
Bridge.onNestHolds(nestHolds);
void Bridge.nestHoldsCards().then(nestHolds, () => {});
// After the settings too: with Zeca off there is no hello at all.
const settings = Bridge.appSettings();
void Promise.all([placed, settings.catch(() => null)]).then(([p, s]) => {
  if ((p && p.presence !== "island") || s?.zeca === false) return;
  Bridge.firstName().then(island.greet, () => island.greet(null));
});
Bridge.onView((view) => {
  setZecaLook(view.look ?? null);
  island.render(view);
});
Bridge.onChat((d) => island.chat.receive(d));
Bridge.onChatSync((s) => island.chat.sync(s));
// Every page hears it; the island takes its own drop and paperclip, and the Nest's files when the
// Nest closed while its chooser was open (they would wait for nobody).
let nestShown = false;
Bridge.onFiles((d) => (d.to === "island" || !nestShown) && island.chat.attach(d.copied, d.refused));
Bridge.onDrag((over) => island.chat.setDragOver(over));
Bridge.onOpenChat(() => island.chat.toggle(true));
Bridge.onJumpFailed(() => island.jumpFailed());
Bridge.onRowMerged((m) => island.merged(m.connector, m.item, m.error));
void Bridge.editorFound().then(island.setEditor, () => {});
Bridge.onShortcut((id) => island.shortcut(id));
Bridge.onPointer((inside) => island.pointer(inside));
Bridge.onAway(() => island.away());
// The corner widget only brings the island up; it never answers a card (ADR 0021).
Bridge.onOpenIsland(() => island.shortcut("open"));
Bridge.onMedia(island.setMedia);
void Bridge.mediaNow().then(island.setMedia);
Bridge.onVoiceLevel((level) => island.chat.voiceLevel(level));
Bridge.onVoiceSilence(() => island.chat.voiceSilence());
Bridge.onVoicePartial((text) => island.chat.voicePartial(text));
void Bridge.voiceStatus().then((v) => island.chat.setVoiceReady(v.ready));
Bridge.onUsage(island.setUsage);
void Bridge.usage().then(island.setUsage);
Bridge.onShortcutKeys((keys) => island.setKeys(keys));
void Bridge.shortcutKeys().then((keys) => island.setKeys(keys));
void settings.then((s) => {
  setLang(s.lang);
  island.chat.relabel();
  Sound.setEnabled(s.sounds);
  Sound.setVolume(s.volume);
  island.setFoldAfter(s.foldAfter);
  island.setOpenOnHover(s.openOnHover);
  setZecaSpecies(s.zecaSpecies);
  island.setVisitors(s.visitors);
  island.setZeca(s.zeca);
  island.chat.setNative(s.nativeChat);
  island.chat.setAccess(s.chatAccess);
  island.setLookSetting(s.zecaLook);
  island.render(island.last());
});
void Bridge.apiKeyStatus().then((api) => island.chat.setApi(api));
Bridge.onSettings((s) => {
  if (s.lang !== undefined) {
    setLang(s.lang);
    island.chat.relabel();
  }
  if (s.sounds !== undefined) Sound.setEnabled(s.sounds);
  if (s.volume !== undefined) Sound.setVolume(s.volume);
  if (s.api !== undefined) island.chat.setApi(s.api);
  if (s.foldAfter !== undefined) island.setFoldAfter(s.foldAfter);
  if (s.openOnHover !== undefined) island.setOpenOnHover(s.openOnHover);
  if (s.voice !== undefined) island.chat.setVoiceReady(s.voice);
  if (s.zecaSpecies !== undefined) setZecaSpecies(s.zecaSpecies);
  if (s.visitors !== undefined) island.setVisitors(s.visitors);
  // Changed elsewhere (Settings): the gear shows it too.
  if (s.history !== undefined) island.settingsChanged({ history: s.history });
  if (s.flock !== undefined) island.settingsChanged({ flock: s.flock });
  if (s.zeca !== undefined) island.setZeca(s.zeca);
  if (s.nativeChat !== undefined) island.chat.setNative(s.nativeChat);
  if (s.chatAccess !== undefined) island.chat.setAccess(s.chatAccess);
  if (s.zecaLook !== undefined) island.setLookSetting(s.zecaLook);
  island.render(island.last());
});

// The summer molt follows the calendar; checked again every hour, for an island left running.
setMolt(moltSeason(new Date()));
window.setInterval(() => setMolt(moltSeason(new Date())), 3_600_000);

// The idle flock keeps a vulture's day by the local clock (dawn, day, dusk, night).
followClock();

/** The local date seven days ago, as the history reads a day (`2026-10-05`). */
function weekAgo(): string {
  const d = new Date();
  d.setDate(d.getDate() - 7);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
