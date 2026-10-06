// Where the island's shape sits in its window. At the top of the screen it hangs from the window's
// top edge. In Panel mode the window sits in the panel's corner: by a bottom (or side) panel the
// shape stands on the window's bottom edge instead, and while it rests nothing takes the mouse.
// The geometry is render.ts's; this only moves it and the input region with it.
import "./dock.css";
import { inPanel, setPanel } from "./fsm";

export type Place = { presence: "island" | "panel"; dock: "top" | "bottom" };

let place: Place = { presence: "island", dock: "top" };
/** The last region render.ts asked for, before the dock moved it. */
let asked: [number, number, number, number] = [0, 0, 0, 0];
let send: (x: number, y: number, w: number, h: number) => void = () => {};

/** Where `send` delivers the region (the bridge). */
export function routeLayout(to: typeof send): void {
  send = to;
}

/** render.ts's region, moved to where the shape is drawn. `hidden` is the resting island. */
export function layout(x: number, y: number, w: number, h: number, hidden: boolean): void {
  asked = [x, y, w, h];
  if (inPanel() && hidden) send(0, 0, 0, 0);
  else if (place.dock === "bottom") send(x, window.innerHeight - h - y, w, h);
  else send(x, y, w, h);
}

/** The app moved the island: the page follows, at once. */
export function setPlace(next: Place, hidden: () => boolean): void {
  place = next;
  document.body.classList.toggle("panel", next.presence === "panel");
  document.body.classList.toggle("dock-bottom", next.dock === "bottom");
  setPanel(next.presence === "panel");
  layout(...asked, hidden());
}
