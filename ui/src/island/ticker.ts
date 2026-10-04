// The step ticker: the step just done on top (dim, smaller, a check), the current one below
// (bright, a chevron, a shimmer). A new step slides everything up one row. Steps that arrive
// mid-slide wait in a short queue, so a burst scrolls past instead of being dropped. A finished
// edit shows its +N −M, and a click on it opens its diff.
import type { DiffSummary, SessionView } from "../bridge";
import { el } from "../dom";
import { icon } from "./icons";

const SLIDE_MS = 380;
const MAX_QUEUE = 4;

/** One row: its step number (counted like `step_count`), its text, and its diff when it has one. */
export interface TickerStep {
  n: number;
  text: string;
  diff: DiffSummary | null;
}

/** The session's kept steps, numbered. */
export function tickerSteps(s: SessionView): TickerStep[] {
  const first = s.step_count - s.steps.length + 1;
  return s.steps.map((text, i) => ({ n: first + i, text, diff: s.diffs?.[i] ?? null }));
}

export class Ticker {
  readonly element = el("div", { class: "ticker" });
  /** A click on a step's +N −M. */
  onDiff: (step: number) => void = () => {};
  private shown: TickerStep[] = [];
  private queue: TickerStep[] = [];
  private sliding = false;
  /** Identifies the session, so switching focus re-seeds instead of scrolling. */
  private owner = "";

  /** `steps` oldest first; `owner` is the session they belong to. */
  sync(owner: string, steps: TickerStep[]): void {
    const latest = steps.length ? steps : [{ n: 0, text: "…", diff: null }];
    if (owner !== this.owner || this.shown.length === 0) {
      this.owner = owner;
      this.queue = [];
      this.shown = latest.slice(-2);
      this.paint();
      return;
    }
    // A diff lands when its call finishes, after the step showed: the row gains it in place.
    const now = (t: TickerStep) => latest.find((l) => l.n === t.n) ?? t;
    const before = JSON.stringify(this.shown);
    this.shown = this.shown.map(now);
    this.queue = this.queue.map(now);
    if (!this.sliding && JSON.stringify(this.shown) !== before) this.paint();
    const current = (this.queue.at(-1) ?? this.shown.at(-1))?.n ?? 0;
    const fresh = latest.filter((l) => l.n > current);
    this.queue.push(...fresh);
    this.queue = this.queue.slice(-MAX_QUEUE);
    this.next();
  }

  private next(): void {
    if (this.sliding || this.queue.length === 0) return;
    const step = this.queue.shift()!;
    this.sliding = true;
    this.element.append(this.row(step, "incoming"));
    // A beat for the incoming row to lay out before it moves (rAF may be paused, see scene.ts).
    window.setTimeout(() => this.element.classList.add("slide"), 20);
    window.setTimeout(() => {
      // The "…" placeholder (n 0) is not a step: it never becomes the done row.
      this.shown = [...this.shown.filter((t) => t.n > 0), step].slice(-2);
      this.element.classList.remove("slide");
      this.paint();
      this.sliding = false;
      this.next();
    }, SLIDE_MS);
  }

  private paint(): void {
    const [done, current] = this.shown.length > 1 ? this.shown : [null, this.shown[0]];
    this.element.replaceChildren(
      ...(done ? [this.row(done, "done")] : []),
      this.row(current ?? { n: 0, text: "…", diff: null }, "current"),
    );
  }

  private row(step: TickerStep, kind: "done" | "current" | "incoming"): HTMLElement {
    const d = step.diff;
    const diff = d
      ? el(
          "button",
          { class: "diff tick-diff", onclick: () => this.onDiff(step.n) },
          el("span", { class: "add", text: `+${d.added}` }),
          el("span", { class: "del", text: `−${d.removed}` }),
        )
      : null;
    if (diff) diff.title = d!.files > 1 ? `See the changes in ${d!.files} files` : "See the changes";
    return el(
      "div",
      { class: `tick ${kind}` },
      el("span", { class: "tick-icon" }, kind === "done" ? icon("check", 10, 2.4) : icon("chevron", 10, 2.4)),
      el("span", { class: "tick-text", text: step.text }),
      ...(diff ? [diff] : []),
    );
  }
}
