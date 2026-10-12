// Quick's Connections in the open island: what is open at each connector right now, one row each.
// The core says what each row is (checks, review); this file only picks words and colors.
import type { BoardView, ConnectorStatus, RowView } from "../bridge";
import { el } from "../dom";
import { t, tk } from "../i18n";
import { icon } from "./icons";

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
  return s < 60 ? t("just now") : s < 3600 ? t("{n} min ago", { n: Math.round(s / 60) }) : t("{n} h ago", { n: Math.round(s / 3600) });
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
}

/**
 * Quick's Connections, as the prototype draws them: a chip per connector with its count, then a
 * card per row (its title, what it is, how it stands). `board` is the one shown; null with none on.
 */
export function connectionsCard(boards: BoardView[], board: BoardView | null, name: (id: string) => string, on: ConnectionsActions, stale: string | null = null): HTMLElement {
  const add = el("button", { class: "conn-chip add", onclick: () => on.add() }, icon("plus", 12, 2));
  add.title = t("Add a connection");
  add.setAttribute("aria-label", t("Add a connection"));
  const chips = el(
    "div",
    { class: "conn-chips" },
    ...boards.map((b) =>
      el(
        "button",
        { class: `conn-chip${b === board ? " on" : ""}`, onclick: () => b !== board && on.show(b.connector) },
        icon("pull", 12, 1.8),
        el("span", { text: name(b.connector) }),
        el("span", { class: "conn-count", text: String(b.rows.length) }),
      ),
    ),
    add,
  );
  const note = stale ? el("div", { class: "board-stale", text: stale }) : null;
  if (!board)
    return el("div", { class: "board connections" }, chips, el("div", { class: "board-empty", text: t("No connector is on yet.") }));
  const label = (g: RowView["group"]) => t(GROUPS.find((x) => x.id === g)?.label ?? "");
  const rows = GROUPS.flatMap((g) => board.rows.filter((r) => r.group === g.id)).map((r) => {
    const end = tag(r);
    const glyph = el("span", { class: `conn-icon ${r.checks ?? "none"}` }, icon(r.group === "branches" ? "branch" : "pull", 14, 1.8));
    if (r.checks) glyph.title = t(CHECKS[r.checks]);
    const line = el(
      r.link ? "button" : "div",
      { class: `conn-row${r.link ? " link" : ""}`, onclick: r.link ? () => on.open(board.connector, r.item) : undefined },
      glyph,
      el("span", { class: "conn-text" }, el("span", { class: "conn-title", text: r.title }), el("span", { class: "conn-sub", text: `${label(r.group)} · ${r.name}` })),
      end ? el("span", { class: `conn-tag ${end.kind}`, text: end.text }) : null,
    );
    line.title = r.title;
    return line;
  });
  return el(
    "div",
    { class: "board connections" },
    chips,
    note,
    rows.length ? el("div", { class: "conn-rows" }, ...rows) : el("div", { class: "board-empty", text: t("Nothing open right now.") }),
  );
}
