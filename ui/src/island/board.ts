// A connector's card in the open island: what is open there right now, grouped, one row each.
// The core says what each row is (checks, review); this file only picks words and colors.
import type { BoardView, ConnectorStatus, RowView } from "../bridge";
import { el } from "../dom";

const GROUPS: { id: RowView["group"]; label: string }[] = [
  { id: "yours", label: "Your pull requests" },
  { id: "to-review", label: "Waiting for your review" },
  { id: "branches", label: "Default branches" },
];

const CHECKS: Record<NonNullable<RowView["checks"]>, string> = {
  passing: "Checks passing",
  failing: "Checks failing",
  running: "Checks running",
};

/** What a branch row says at its end. */
const BRANCH: Record<NonNullable<RowView["checks"]>, string> = {
  passing: "Passing",
  failing: "Failing",
  running: "Running",
};

const REVIEW: Record<NonNullable<RowView["review"]>, string> = {
  approved: "Approved",
  changes: "Changes requested",
};

/** A branch row says how its checks went; a pull request says its review, its dot the checks. */
function tag(r: RowView): { text: string; kind: string } | null {
  if (r.group === "branches" && r.checks) return { text: BRANCH[r.checks], kind: r.checks };
  if (r.review) return { text: REVIEW[r.review], kind: r.review };
  return null;
}

function row(r: RowView, open: (item: string) => void): HTMLElement {
  const t = tag(r);
  const dot = el("span", { class: `dot${r.checks ? ` ${r.checks}` : " none"}` });
  if (r.checks) dot.title = CHECKS[r.checks];
  const line = el(
    r.link ? "button" : "div",
    { class: `board-row${r.link ? " link" : ""}`, onclick: r.link ? () => open(r.item) : undefined },
    dot,
    el("span", { class: "row-name", text: r.name }),
    el("span", { class: "row-title", text: r.title }),
    t ? el("span", { class: `row-tag ${t.kind}`, text: t.text }) : null,
  );
  line.title = r.title;
  return line;
}

/** What the card says by its name when its last poll failed; seconds since the epoch, like `lastOk`. */
export function staleNote(status: Pick<ConnectorStatus, "lastOk" | "error">, nowSecs: number): string | null {
  if (!status.error) return null;
  if (status.lastOk === null) return status.error;
  const s = Math.max(0, Math.round(nowSecs - status.lastOk));
  const age = s < 60 ? "just now" : s < 3600 ? `${Math.round(s / 60)} min ago` : `${Math.round(s / 3600)} h ago`;
  return `Last updated ${age} · ${status.error}`;
}

/** `stale` comes from [`staleNote`]: the rows stay, they are only said to be old. */
export function boardCard(board: BoardView, name: string, open: (item: string) => void, stale: string | null = null): HTMLElement {
  const note = stale ? el("span", { class: "board-stale", text: stale }) : null;
  if (note) note.title = stale!;
  const groups = GROUPS.flatMap((g) => {
    const rows = board.rows.filter((r) => r.group === g.id);
    if (!rows.length) return [];
    return [el("div", { class: "board-group" }, el("div", { class: "board-label", text: g.label }), ...rows.map((r) => row(r, open)))];
  });
  return el(
    "div",
    { class: "board" },
    el("div", { class: "board-head" }, el("span", { text: name }), note),
    groups.length ? el("div", { class: "board-rows" }, ...groups) : el("div", { class: "board-empty", text: "Nothing open right now." }),
  );
}
