// The island window: Tauri in, DOM out.
import { Bridge } from "./bridge";
import { createIsland } from "./island/render";

const island = createIsland(document.getElementById("island")!, {
  decide: (request, decision) => void Bridge.decide(request, decision),
  layout: (x, y, w, h) => void Bridge.layout(x, y, w, h),
  chat: {
    send: (text, files) => Bridge.chatSend(text, files),
    reset: (provider) => Bridge.chatReset(provider),
    keyboard: (on) => void Bridge.islandKeyboard(on),
  },
});
island.render({ sessions: [], approval: null });
Bridge.onView(island.render);
Bridge.onChat((d) => island.chat.receive(d));
Bridge.onFiles((paths) => island.chat.attach(paths));
Bridge.onOpenChat(() => island.chat.toggle(true));
