// The island window: Tauri in, DOM out.
import { Bridge } from "./bridge";
import { createIsland } from "./island/render";
import { Sound } from "./sound";

const island = createIsland(document.getElementById("island")!, {
  decide: (request, decision) => void Bridge.decide(request, decision),
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
    reset: (provider) => Bridge.chatReset(provider),
    keyboard: (on) => void Bridge.islandKeyboard(on),
  },
});
island.render({ sessions: [], approval: null, alerts: [] });
Bridge.onView(island.render);
Bridge.onChat((d) => island.chat.receive(d));
Bridge.onFiles((paths) => island.chat.attach(paths));
Bridge.onOpenChat(() => island.chat.toggle(true));
Bridge.onShortcut((id) => island.shortcut(id));
Bridge.onShortcutKeys((keys) => island.setKeys(keys));
void Bridge.appSettings().then((s) => Sound.setEnabled(s.sounds));
Bridge.onSettings((s) => {
  Sound.setEnabled(s.sounds);
  island.render(island.last());
});
