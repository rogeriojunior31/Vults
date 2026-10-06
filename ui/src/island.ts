// The island window: Tauri in, DOM out.
import { Bridge, type Place } from "./bridge";
import * as Dock from "./island/dock";
import { setZecaLook, setZecaSpecies } from "./island/flock";
import { resting } from "./island/fsm";
import { createIsland } from "./island/render";
import { Sound } from "./sound";

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
  stepDiff: (agent, id, step) => Bridge.stepDiff(agent, id, step).catch(() => null),
  openSettings: () => void Bridge.openSettings(),
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
    keyboard: (on) => void Bridge.islandKeyboard(on),
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
void placed.then((p) => {
  if (p && p.presence !== "island") return;
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
Bridge.onShortcut((id) => island.shortcut(id));
Bridge.onPointer((inside) => island.pointer(inside));
Bridge.onMedia(island.setMedia);
void Bridge.mediaNow().then(island.setMedia);
Bridge.onVoiceLevel((level) => island.chat.voiceLevel(level));
Bridge.onVoiceSilence(() => island.chat.voiceSilence());
void Bridge.voiceStatus().then((v) => island.chat.setVoiceReady(v.ready));
Bridge.onUsage(island.setUsage);
void Bridge.usage().then(island.setUsage);
Bridge.onShortcutKeys((keys) => island.setKeys(keys));
void Bridge.shortcutKeys().then((keys) => island.setKeys(keys));
void Bridge.appSettings().then((s) => {
  Sound.setEnabled(s.sounds);
  Sound.setVolume(s.volume);
  island.setFoldAfter(s.foldAfter);
  setZecaSpecies(s.zecaSpecies);
  island.setVisitors(s.visitors);
  island.setZeca(s.zeca);
  island.render(island.last());
});
void Bridge.apiKeyStatus().then((api) => island.chat.setApi(api));
Bridge.onSettings((s) => {
  if (s.sounds !== undefined) Sound.setEnabled(s.sounds);
  if (s.volume !== undefined) Sound.setVolume(s.volume);
  if (s.api !== undefined) island.chat.setApi(s.api);
  if (s.foldAfter !== undefined) island.setFoldAfter(s.foldAfter);
  if (s.voice !== undefined) island.chat.setVoiceReady(s.voice);
  if (s.zecaSpecies !== undefined) setZecaSpecies(s.zecaSpecies);
  if (s.visitors !== undefined) island.setVisitors(s.visitors);
  if (s.zeca !== undefined) island.setZeca(s.zeca);
  island.render(island.last());
});
