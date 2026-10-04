// The island window: Tauri in, DOM out.
import { Bridge } from "./bridge";
import { setZecaLook, setZecaSpecies } from "./island/flock";
import { createIsland } from "./island/render";
import { Sound } from "./sound";

const island = createIsland(document.getElementById("island")!, {
  decide: (request, decision) => void Bridge.decide(request, decision),
  decideAlways: (request) => void Bridge.decideAlways(request),
  answer: (request, answers) => void Bridge.questionAnswer(request, answers),
  release: (request) => void Bridge.questionRelease(request),
  layout: (x, y, w, h) => void Bridge.layout(x, y, w, h),
  openAlert: (key) => void Bridge.alertOpen(key),
  jump: (agent, id) => void Bridge.sessionJump(agent, id),
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
      start: () => Bridge.voiceStart(),
      stop: () => Bridge.voiceStop(),
      cancel: () => void Bridge.voiceCancel(),
    },
  },
});
island.render({ sessions: [], approval: null, alerts: [] });
// The hello waits for the name, and goes without it if the app cannot say.
Bridge.firstName().then(island.greet, () => island.greet(null));
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
void Bridge.voiceStatus().then((v) => island.chat.setVoiceReady(v.ready));
Bridge.onUsage(island.setUsage);
void Bridge.usage().then(island.setUsage);
Bridge.onShortcutKeys((keys) => island.setKeys(keys));
void Bridge.shortcutKeys().then((keys) => island.setKeys(keys));
void Bridge.appSettings().then((s) => {
  Sound.setEnabled(s.sounds);
  island.setFoldAfter(s.foldAfter);
  setZecaSpecies(s.zecaSpecies);
  island.setVisitors(s.visitors);
  island.render(island.last());
});
void Bridge.apiKeyStatus().then((api) => island.chat.setApi(api));
Bridge.onSettings((s) => {
  if (s.sounds !== undefined) Sound.setEnabled(s.sounds);
  if (s.api !== undefined) island.chat.setApi(s.api);
  if (s.foldAfter !== undefined) island.setFoldAfter(s.foldAfter);
  if (s.voice !== undefined) island.chat.setVoiceReady(s.voice);
  if (s.zecaSpecies !== undefined) setZecaSpecies(s.zecaSpecies);
  if (s.visitors !== undefined) island.setVisitors(s.visitors);
  island.render(island.last());
});
