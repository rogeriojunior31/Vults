// Quick's Connections in the open island: what is open at each connector right now, one row each.
// The core says what each row is (checks, review); this file only picks words and colors.
import type { BoardView, ConnectorStatus, RowView } from "../bridge";
import { el } from "../dom";
import { t, tk } from "../i18n";
import { icon } from "./icons";
import { markSeen, unseen } from "./seen";

export const GROUPS: { id: RowView["group"]; label: string }[] = [
  { id: "yours", label: tk("Your pull requests") },
  { id: "to-review", label: tk("Waiting for your review") },
  { id: "branches", label: tk("Default branches") },
];

export const CHECKS: Record<NonNullable<RowView["checks"]>, string> = {
  passing: tk("Checks passing"),
  failing: tk("Checks failing"),
  running: tk("Checks running"),
};

/** What a branch row says at its end. */
const BRANCH: Record<NonNullable<RowView["checks"]>, string> = {
  passing: tk("Passing"),
  failing: tk("Failing"),
  running: tk("Running"),
};

const REVIEW: Record<NonNullable<RowView["review"]>, string> = {
  approved: tk("Approved"),
  changes: tk("Changes requested"),
};

/** A branch row says how its checks went; a pull request says its review, its dot the checks. */
export function tag(r: RowView): { text: string; kind: string } | null {
  if (r.group === "branches" && r.checks) return { text: t(BRANCH[r.checks]), kind: r.checks };
  if (r.review) return { text: t(REVIEW[r.review]), kind: r.review };
  return null;
}

/** How long ago a poll was, said as the card says it; both in seconds since the epoch. */
export function ago(then: number, nowSecs: number): string {
  const s = Math.max(0, Math.round(nowSecs - then));
  if (s < 60) return t("just now");
  if (s < 3600) return t("{n} min ago", { n: Math.round(s / 60) });
  // A release or a quiet branch can be weeks old: "400 h ago" says nothing.
  return s < 172_800 ? t("{n} h ago", { n: Math.round(s / 3600) }) : t("{n} d ago", { n: Math.round(s / 86_400) });
}

/** What the card says by its name when its last poll failed; seconds since the epoch, like `lastOk`. */
export function staleNote(status: Pick<ConnectorStatus, "lastOk" | "error">, nowSecs: number): string | null {
  if (!status.error) return null;
  if (status.lastOk === null) return status.error;
  return t("Last updated {age} · {error}", { age: ago(status.lastOk, nowSecs), error: status.error });
}

export interface ConnectionsActions {
  open(connector: string, item: string): void;
  /** Another connector's rows, from its chip. */
  show(connector: string): void;
  /** The Nest's Connections, where a connector is turned on. */
  add(): void;
  /** Opens the chat with these words in the composer, for the user to send; absent with Zeca off. */
  ask?(text: string): void;
  /** A row or all of them marked seen: the card is drawn again without them. */
  seen?(): void;
  /** Room's Connections, where seen rows still are. */
  room?(): void;
  /** *Squash and merge*, after the user confirmed it on the row. */
  merge?(connector: string, item: string): void;
}

/** Each merge asked for, by connector and row. A merged row stays so until the poll takes it. */
type Merge = { state: "running" } | { state: "merged" | "queued" | "pending" } | { state: "refused"; error: string };
const merges = new Map<string, Merge>();
/** The row whose merge is being confirmed: kept across the card's rebuilds (a poll, a ✓). */
let asking: string | null = null;
const mergeKey = (connector: string, item: string) => `${connector}\n${item}`;

export function mergeStarted(connector: string, item: string): void {
  merges.set(mergeKey(connector, item), { state: "running" });
}

/** A merge ended: a refusal stays on the row; a merged row leaves with the next poll. */
export function mergeEnded(connector: string, item: string, error: string | null, status?: string): void {
  merges.set(mergeKey(connector, item), error ? { state: "refused", error } : { state: status === "queued" ? "queued" : status === "accepted_pending_verification" ? "pending" : "merged" });
}

/** Changes whenever a merge starts or ends, for the card's rebuild check. */
export function mergeSig(): string {
  return JSON.stringify([...merges]);
}

/** What a row asks Zeca, if anything: why its checks fail, or what a review should look at. */
export function question(r: RowView): { label: string; name: string; text: string } | null {
  if (r.group === "to-review")
    return { label: t("Review with Zeca"), name: t("Review {title} with Zeca", { title: r.title }), text: t("Review the pull request {name}, \"{title}\": read its diff and tell me the risks before I approve it.", { name: r.name, title: r.title }) };
  if (r.checks === "failing")
    return { label: t("Ask Zeca why"), name: t("Ask Zeca why the checks fail on {title}", { title: r.title }), text: t("Why are the checks failing on {name}, \"{title}\"? Read only the failed step's log and tell me what broke and where.", { name: r.name, title: r.title }) };
  return null;
}

/** Slides a row out, then marks it; at once when motion is reduced or the page is hidden. */
function dismiss(row: HTMLElement, done: () => void): void {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches || document.hidden) return done();
  let once = false;
  const fin = () => !once && ((once = true), done());
  row.animate([{ opacity: 1, transform: "none" }, { opacity: 0, transform: "translateX(28px)" }], { duration: 200, easing: "ease-in", fill: "forwards" }).finished.then(fin, fin);
  // An animation never finishes while WebKit thinks the page is hidden.
  setTimeout(fin, 300);
}

/**
 * Quick's Connections, as the prototype draws them: a chip per connector with its count, then a
 * card per row (its title, what it is, how it stands). `board` is the one shown; null with none on.
 */
export function connectionsCard(boards: BoardView[], board: BoardView | null, name: (id: string) => string, on: ConnectionsActions, stale: string | null = null): HTMLElement {
  const add = el("button", { class: "qconn-chip add", onclick: () => on.add() }, icon("plus", 12, 2));
  add.title = t("Add a connection");
  add.setAttribute("aria-label", t("Add a connection"));
  const chips = el(
    "div",
    { class: "qconn-chips" },
    ...boards.map((b) =>
      el(
        "button",
        { class: `qconn-chip${b === board ? " on" : ""}`, onclick: () => b !== board && on.show(b.connector) },
        icon("pull", 12, 1.8),
        el("span", { text: name(b.connector) }),
        el("span", { class: "qconn-count", text: String(unseen(b).length) }),
      ),
    ),
    add,
  );
  const note = stale ? el("div", { class: "board-stale", text: stale }) : null;
  if (!board)
    return el("div", { class: "board connections" }, chips, el("div", { class: "board-empty", text: t("No connector has answered yet.") }));
  const label = (g: RowView["group"]) => t(GROUPS.find((x) => x.id === g)?.label ?? "");
  const left = unseen(board);
  // A merged row left the card: its merge is done with. One the core no longer offers (its checks
  // ran again) got no answer: it may be asked again once it is green.
  const offered = (k: string) => board.rows.some((r) => r.merge && mergeKey(board.connector, r.item) === k);
  for (const [k, m] of [...merges]) {
    const gone = !board.rows.some((r) => mergeKey(board.connector, r.item) === k);
    if (k.startsWith(`${board.connector}\n`) && (gone || (m.state === "running" && !offered(k)))) merges.delete(k);
  }
  if (asking?.startsWith(`${board.connector}\n`) && !offered(asking)) asking = null;
  const rows = GROUPS.flatMap((g) => left.filter((r) => r.group === g.id)).map((r) => {
    const glyph = el("span", { class: `qconn-icon ${r.checks ?? "none"}` }, icon(r.group === "branches" ? "branch" : "pull", 14, 1.8));
    if (r.checks) glyph.title = t(CHECKS[r.checks]);
    const ask = on.ask ? question(r) : null;
    // A row's action takes the tag's place: the card is narrow, and the dot still says the checks.
    const end = ask ? null : tag(r);
    const text = el("span", { class: "qconn-text" }, el("span", { class: "qconn-title", text: r.title }), el("span", { class: "qconn-sub", text: `${label(r.group)} · ${r.name}` }));
    const body = el(
      r.link ? "button" : "div",
      { class: `qconn-open${r.link ? " link" : ""}`, onclick: r.link ? () => on.open(board.connector, r.item) : undefined },
      glyph,
      text,
    );
    body.title = r.title;
    const line = el("div", { class: "qconn-row" }, body);
    const merging = on.merge ? merges.get(mergeKey(board.connector, r.item)) : undefined;
    // Run once the row is whole: the confirm goes under everything else on it.
    let reopen: (() => void) | null = null;
    if ((r.merge || merging?.state === "queued" || merging?.state === "pending") && on.merge) {
      const go = on.merge;
      const k = mergeKey(board.connector, r.item);
      // Two clicks: the first only asks, under the row, what the second will do on GitHub.
      const confirm = (focus: boolean) => {
        asking = k;
        act.hidden = true;
        const yes = el("button", {
          class: "qconn-act primary",
          text: t("Merge"),
          onclick: () => {
            asking = null;
            go(board.connector, r.item);
          },
        });
        const no = el("button", {
          class: "qconn-act",
          text: t("Cancel"),
          onclick: () => {
            asking = null;
            ask.remove();
            act.hidden = false;
            act.focus();
          },
        });
        const ask = el("div", { class: "qconn-confirm" }, el("span", { text: t("Squash {name} into its base branch or its required merge queue? This action does not request branch deletion.", { name: r.name }) }), no, yes);
        line.append(ask);
        if (focus) yes.focus();
      };
      const label = merging?.state === "running" ? t("Merging…") : merging?.state === "merged" ? t("Merged") : merging?.state === "queued" ? t("In merge queue") : merging?.state === "pending" ? t("Merge pending") : t("Squash and merge");
      const act = el("button", { class: "qconn-act", text: label, onclick: () => confirm(true) });
      act.disabled = merging?.state === "running" || merging?.state === "merged" || merging?.state === "queued" || merging?.state === "pending";
      line.append(act);
      if (asking === k && !act.disabled) reopen = () => confirm(false);
    } else if (end) line.append(el("span", { class: `qconn-tag ${end.kind}`, text: end.text }));
    if (ask && on.ask) {
      const go = on.ask;
      const act = el("button", { class: "qconn-act", text: ask.label, onclick: () => go(ask.text) });
      act.setAttribute("aria-label", ask.name);
      line.append(act);
    }
    if (on.seen) {
      const after = on.seen;
      const done = el("button", { class: "qconn-done", text: "\u2713", onclick: () => dismiss(line, () => (markSeen(board, [r]), after())) });
      done.title = t("Seen: leaves Quick until it changes, stays in Room");
      done.setAttribute("aria-label", t("Mark {title} as seen", { title: r.title }));
      line.append(done);
    }
    if (merging?.state === "refused") line.append(el("div", { class: "qconn-confirm error", text: t("GitHub did not merge it: {error}", { error: merging.error }) }));
    reopen?.();
    return line;
  });
  const foot =
    on.seen && left.length > 1
      ? el(
          "div",
          { class: "qconn-foot" },
          el("span", { text: t("{n} not seen", { n: left.length }) }),
          el("button", { text: t("Mark all seen"), onclick: () => (markSeen(board, left), on.seen?.()) }),
        )
      : null;
  const empty =
    board.rows.some((r) => r.group !== "releases") && !left.length
      ? el(
          "div",
          { class: "qconn-empty" },
          el("strong", { text: t("All caught up") }),
          el("span", { text: t("Everything you marked seen is still in Room.") }),
          on.room ? el("button", { class: "qconn-act", text: t("Open Connections"), onclick: () => on.room?.() }) : null,
        )
      : el("div", { class: "board-empty", text: t("Nothing open right now.") });
  return el(
    "div",
    { class: "board connections" },
    chips,
    note,
    rows.length ? el("div", { class: "qconn-rows" }, ...rows) : empty,
    foot,
  );
}
