// The Nest window's edges and corners. It has no system frame (ADR 0021), so the compositor offers
// no border to pull: a press here asks it to resize the window from that side.
import { Bridge, type ResizeEdge } from "../../bridge";
import { el } from "../../dom";

const EDGES: ResizeEdge[] = ["North", "South", "East", "West", "NorthEast", "NorthWest", "SouthEast", "SouthWest"];

/** The eight grips, laid over the page's border; hidden in Full (nothing to resize there). */
export function resizeEdges(): HTMLElement {
  return el(
    "div",
    { class: "resize-edges" },
    ...EDGES.map((edge) => {
      const grip = el("div", { class: `resize-edge ${edge.toLowerCase()}` });
      grip.addEventListener("mousedown", (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        Bridge.startResize(edge);
      });
      return grip;
    }),
  );
}
