// The island window: Tauri in, DOM out.
import { Bridge } from "./bridge";
import { createIsland } from "./island/render";
import { Sound } from "./sound";

const island = createIsland(document.getElementById("island")!, {
  decide: (request, decision) => void Bridge.decide(request, decision),
  decideAlways: (request) => void Bridge.decideAlways(request),
  layout: (x, y, w, h) => void Bridge.layout(x, y, w, h),
  openAlert: (key) => void Bridge.alertOpen(key),
  jump: (agent, id) => void Bridge.sessionJump(agent, id),
  openSettings: () => void Bridge.openSettings(),
  setSounds: (on) => {
    Sound.setEnabled(on);
    void Bridge.setSounds(on);
  },
  dismissAlert: (key) => void Bridge.alertDismiss(key),
  chat: {
    send: (text, files, folder) => Bridge.chatSend(text, files, folder),
    decide: (id, allow) => void Bridge.chatDecide(id, allow),
    stop: () => void Bridge.chatStop(),
    reset: (provider) => Bridge.chatReset(provider),
    keyboard: (on) => void Bridge.islandKeyboard(on),
  },
});
island.render({ sessions: [], approval: null, alerts: [] });
Bridge.onView(island.render);
Bridge.onChat((d) => island.chat.receive(d));
Bridge.onFiles((d) => island.chat.attach(d.copied, d.refused));
Bridge.onDrag((over) => island.chat.setDragOver(over));
Bridge.onOpenChat(() => island.chat.toggle(true));
Bridge.onJumpFailed(() => island.jumpFailed());
Bridge.onShortcut((id) => island.shortcut(id));
Bridge.onShortcutKeys((keys) => island.setKeys(keys));
void Bridge.shortcutKeys().then((keys) => island.setKeys(keys));
void Bridge.appSettings().then((s) => {
  Sound.setEnabled(s.sounds);
  island.setFoldAfter(s.foldAfter);
});
void Bridge.apiKeyStatus().then((on) => island.chat.setApiKey(on));
Bridge.onSettings((s) => {
  if (s.sounds !== undefined) Sound.setEnabled(s.sounds);
  if (s.apiKey !== undefined) island.chat.setApiKey(s.apiKey);
  if (s.foldAfter !== undefined) island.setFoldAfter(s.foldAfter);
  island.render(island.last());
});
