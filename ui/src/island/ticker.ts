// The step ticker: the step just done on top (dim, smaller, a check), the current one below
// (bright, a chevron, a shimmer). A new step slides everything up one row. Steps that arrive
// mid-slide wait in a short queue, so a burst scrolls past instead of being dropped.
import { el } from "../dom";
import { icon } from "./icons";

const SLIDE_MS = 380;
const MAX_QUEUE = 4;

export class Ticker {
  readonly element = el("div", { class: "ticker" });
  private shown: string[] = [];
  private queue: string[] = [];
  private sliding = false;
  /** Identifies the session, so switching focus re-seeds instead of scrolling. */
  private owner = "";

  /** `steps` oldest first; `owner` is the session they belong to. */
  sync(owner: string, steps: string[]): void {
    const latest = steps.length ? steps : ["…"];
    if (owner !== this.owner || this.shown.length === 0) {
      this.owner = owner;
      this.queue = [];
      this.shown = latest.slice(-2);
      this.paint(false);
      return;
    }
    const current = this.queue.at(-1) ?? this.shown.at(-1);
    const from = latest.lastIndexOf(current ?? "");
    const fresh = from >= 0 ? latest.slice(from + 1) : latest.slice(-1);
    this.queue.push(...fresh);
    this.queue = this.queue.slice(-MAX_QUEUE);
    this.next();
  }

  private next(): void {
    if (this.sliding || this.queue.length === 0) return;
    const step = this.queue.shift()!;
    this.sliding = true;
    this.element.append(row(step, "incoming"));
    // A beat for the incoming row to lay out before it moves (rAF may be paused, see scene.ts).
    window.setTimeout(() => this.element.classList.add("slide"), 20);
    window.setTimeout(() => {
      this.shown = [...this.shown, step].slice(-2);
      this.element.classList.remove("slide");
      this.paint(false);
      this.sliding = false;
      this.next();
    }, SLIDE_MS);
  }

  private paint(_animate: boolean): void {
    const [done, current] = this.shown.length > 1 ? this.shown : [null, this.shown[0]];
    this.element.replaceChildren(
      ...(done ? [row(done, "done")] : []),
      row(current ?? "…", "current"),
    );
  }
}

function row(text: string, kind: "done" | "current" | "incoming"): HTMLElement {
  return el(
    "div",
    { class: `tick ${kind}` },
    el("span", { class: "tick-icon" }, kind === "done" ? icon("check", 10, 2.4) : icon("chevron", 10, 2.4)),
    el("span", { class: "tick-text", text }),
  );
}
