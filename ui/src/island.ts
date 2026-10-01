// The island window: Tauri in, DOM out.
import { Bridge } from "./bridge";
import { createIsland } from "./island/render";
import { Sound } from "./sound";

const island = createIsland(document.getElementById("island")!, {
  decide: (request, decision) => void Bridge.decide(request, decision),
  layout: (x, y, w, h) => void Bridge.layout(x, y, w, h),
  openAlert: (key) => void Bridge.alertOpen(key),
  dismissAlert: (key) => void Bridge.alertDismiss(key),
  chat: {
    send: (text, files) => Bridge.chatSend(text, files),
    reset: (provider) => Bridge.chatReset(provider),
    keyboard: (on) => void Bridge.islandKeyboard(on),
  },
});
island.render({ sessions: [], approval: null, alerts: [] });
Bridge.onView(island.render);
Bridge.onChat((d) => island.chat.receive(d));
Bridge.onFiles((paths) => island.chat.attach(paths));
Bridge.onOpenChat(() => island.chat.toggle(true));
void Bridge.appSettings().then((s) => Sound.setEnabled(s.sounds));
Bridge.onSettings((s) => Sound.setEnabled(s.sounds));
