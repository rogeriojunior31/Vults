// When the island is hidden, compact or open. No DOM: it takes the pointer, clicks and news, and
// reports transitions.
//
//   hidden ──pointer on the wake strip──▶ compact ──click──▶ open
//      ▲                                    │  ▲                │
//      └──── nobody on the wire, 60 s ──────┘  └── pointer away for `foldAfter`, or Fold ──┘
//
// A permission waiting for an answer opens the island and pins it: it never folds by itself until
// the card is answered. The chat holds it open too (it has the keyboard: folding mid-sentence
// would throw the user's typing away).

export type Mode = "hidden" | "compact" | "open";

export class IslandMachine {
  mode: Mode = "compact";
  onChange: ((from: Mode, to: Mode) => void) | null = null;

  /** Open → compact, this long after the pointer leaves. */
  foldAfterMs = 15_000;
  /** Compact → hidden, this long after the pointer leaves, while nobody is on the wire. */
  hideAfterMs = 60_000;
  /** Something waits for the user: no folding by itself. */
  pinned = false;
  /** The user is busy in the island (the chat): no folding by itself either. */
  private engaged = false;
  /** Sessions on the wire: a compact island with birds on it never hides. Null until first told. */
  private occupied: boolean | null = null;

  private pointerIn = false;
  get pointerInside(): boolean {
    return this.pointerIn;
  }
  private foldTimer: number | undefined;
  private hideTimer: number | undefined;
  /** When the open island will fold, for the countdown; null when it won't. */
  foldAt: number | null = null;

  pointerEntered(): void {
    this.pointerIn = true;
    this.clearTimers();
    if (this.mode === "hidden") this.go("compact");
  }

  pointerLeft(now: number): void {
    this.pointerIn = false;
    this.schedule(now);
  }

  click(): void {
    if (this.mode === "compact") this.go("open");
  }

  /** Fold button, Escape, the OK on a card. */
  fold(now: number): void {
    if (this.pinned) return;
    this.go("compact");
    this.schedule(now);
  }

  /** Opened for the user (the tray's Chat…, a dropped file). */
  open(now: number): void {
    this.go("open");
    this.schedule(now);
  }

  setEngaged(on: boolean, now: number): void {
    if (on === this.engaged) return;
    this.engaged = on;
    this.schedule(now);
  }

  /** A permission: open now, and stay open. */
  openPinned(): void {
    this.pinned = true;
    this.clearTimers();
    this.go("open");
  }

  /** The permission was answered (or withdrawn): folding by itself is allowed again. */
  unpin(now: number): void {
    if (!this.pinned) return;
    this.pinned = false;
    this.schedule(now);
  }

  /** Something happened on the wire: show the compact island if it was hidden. */
  reveal(now: number): void {
    if (this.mode !== "hidden") return;
    this.go("compact");
    this.schedule(now);
  }

  /** Sessions arrived or left: an empty wire may hide again. */
  setOccupied(occupied: boolean, now: number): void {
    if (occupied === this.occupied) return;
    this.occupied = occupied;
    this.schedule(now);
  }

  /** The lab and screenshots: open and never fold. */
  hold(open: boolean): void {
    this.pinned = open;
    this.clearTimers();
    this.go(open ? "open" : "compact");
  }

  private schedule(now: number): void {
    this.clearTimers();
    if (this.pointerIn) return;
    if (this.mode === "open" && !this.pinned && !this.engaged) {
      this.foldAt = now + this.foldAfterMs;
      this.foldTimer = window.setTimeout(() => {
        this.foldAt = null;
        this.go("compact");
        this.schedule(now + this.foldAfterMs);
      }, this.foldAfterMs);
    } else if (this.mode === "compact" && !this.occupied) {
      this.hideTimer = window.setTimeout(
        () => this.go("hidden"),
        this.hideAfterMs,
      );
    }
  }

  private clearTimers(): void {
    window.clearTimeout(this.foldTimer);
    window.clearTimeout(this.hideTimer);
    this.foldTimer = undefined;
    this.hideTimer = undefined;
    this.foldAt = null;
  }

  private go(next: Mode): void {
    if (next === this.mode) return;
    const from = this.mode;
    this.mode = next;
    this.onChange?.(from, next);
  }
}
