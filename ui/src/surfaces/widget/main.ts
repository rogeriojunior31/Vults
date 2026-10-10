// The corner widget's window: the island's view in, a click out that brings the island up. It has
// no commands of its own (ADR 0008).
import { Bridge } from "../../bridge";
import { setZeca, setZecaLook, setZecaSpecies } from "../../island/flock";
import { createWidget, WIDGET_H, WIDGET_W } from "./render";
import { setLang } from "../../i18n";

const widget = createWidget(document.getElementById("widget")!, {
  open: () => void Bridge.openIsland(),
  focus: (agent, id) => void Bridge.sessionFocus({ agent, id }),
});
// The whole window takes the mouse: it is only the widget.
void Bridge.layout(0, 0, WIDGET_W, WIDGET_H);
// Under a full-screen window nothing is seen: no frames drawn.
document.addEventListener("visibilitychange", () => widget.setActive(!document.hidden));
Bridge.onView((view) => {
  setZecaLook(view.look ?? null);
  widget.render(view);
});
void Bridge.appSettings().then((s) => {
  setLang(s.lang);
  setZecaSpecies(s.zecaSpecies);
  setZeca(s.zeca);
  widget.setPaused(s.presence === "paused");
});
Bridge.onSettings((s) => {
  if (s.lang !== undefined) setLang(s.lang);
  if (s.zecaSpecies !== undefined) setZecaSpecies(s.zecaSpecies);
  if (s.zeca !== undefined) setZeca(s.zeca);
  if (s.presence !== undefined) widget.setPaused(s.presence === "paused");
  else widget.render(widget.last());
});
