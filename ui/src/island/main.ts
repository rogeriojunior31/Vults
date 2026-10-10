// The island window: Tauri in, DOM out.
import { Bridge, type Place } from "../bridge";
import * as Dock from "./dock";
import { setZecaLook, setZecaSpecies } from "./flock";
import { resting } from "./fsm";
import { createIsland } from "./render";
import { Sound } from "../sound";
import { setLang } from "../i18n";

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
    void Bridge.openSettings("activity");
    void Bridge.recapDismiss();
  },
  stepDiff: (agent, id, step) => Bridge.stepDiff(agent, id, step).catch(() => null),
  openSettings: () => void Bridge.openSettings(),
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
  connectorStatus: (id) => Bridge.connectorsStatus().then((all) => all.find((c) => c.id === id) ?? null),
  chat: {
    send: (text, files, folder) => Bridge.chatSend(text, files, folder),
    decide: (id, allow) => void Bridge.chatDecide(id, allow),
    stop: () => void Bridge.chatStop(),
    reset: (provider) => Bridge.chatReset(provider),
    keyboard: (on, exclusive) => void Bridge.islandKeyboard(on, exclusive),
    voice: {
      start: (tap) => Bridge.voiceStart(tap),
      stop: () => Bridge.voiceStop(),
      cancel: () => void Bridge.voiceCancel(),
    },
  },
});
Dock.routeLayout((x, y, w, h) => void Bridge.layout(x, y, w, h));
island.render({ sessions: [], approval: null, alerts: [] });
// Where it lives first: only the Island preset says hello (by the panel it would open the island
// by the tray at every start; Quiet and Paused show nothing at rest). The hello waits for the
// name, and goes without it if the app cannot say. A new preset redraws at once (Paused empties
// the island).
const place = (p: Place) => {
  Dock.setPlace(p, resting);
  island.render(island.last());
};
const placed = Bridge.islandPlace().then(
  (p) => (place(p), p),
  () => null,
);
Bridge.onPlace(place);
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
Bridge.onFiles((d) => island.chat.attach(d.copied, d.refused));
Bridge.onDrag((over) => island.chat.setDragOver(over));
Bridge.onOpenChat(() => island.chat.toggle(true));
Bridge.onJumpFailed(() => island.jumpFailed());
void Bridge.editorFound().then(island.setEditor, () => {});
Bridge.onShortcut((id) => island.shortcut(id));
Bridge.onPointer((inside) => island.pointer(inside));
Bridge.onAway(() => island.away());
// The corner widget only brings the island up; it never answers a card (ADR 0008).
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
  Sound.setEnabled(s.sounds);
  Sound.setVolume(s.volume);
  island.setFoldAfter(s.foldAfter);
  island.setOpenOnHover(s.openOnHover);
  setZecaSpecies(s.zecaSpecies);
  island.setVisitors(s.visitors);
  island.setZeca(s.zeca);
  island.setLookSetting(s.zecaLook);
  island.render(island.last());
});
void Bridge.apiKeyStatus().then((api) => island.chat.setApi(api));
Bridge.onSettings((s) => {
  if (s.lang !== undefined) setLang(s.lang);
  if (s.sounds !== undefined) Sound.setEnabled(s.sounds);
  if (s.volume !== undefined) Sound.setVolume(s.volume);
  if (s.api !== undefined) island.chat.setApi(s.api);
  if (s.foldAfter !== undefined) island.setFoldAfter(s.foldAfter);
  if (s.openOnHover !== undefined) island.setOpenOnHover(s.openOnHover);
  if (s.voice !== undefined) island.chat.setVoiceReady(s.voice);
  if (s.zecaSpecies !== undefined) setZecaSpecies(s.zecaSpecies);
  if (s.visitors !== undefined) island.setVisitors(s.visitors);
  if (s.zeca !== undefined) island.setZeca(s.zeca);
  if (s.zecaLook !== undefined) island.setLookSetting(s.zecaLook);
  island.render(island.last());
});
