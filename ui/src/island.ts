// The island window: Tauri in, DOM out.
import { Bridge } from "./bridge";
import { createIsland } from "./island/render";

const render = createIsland(document.getElementById("island")!, {
  decide: (request, decision) => void Bridge.decide(request, decision),
  layout: (x, y, w, h) => void Bridge.layout(x, y, w, h),
});
render({ sessions: [], approval: null });
Bridge.onView(render);
