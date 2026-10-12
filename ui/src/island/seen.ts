// Connections' rows the user marked seen with ✓: they leave Quick until they change, and stay in
// Room. Only this island's memory: nothing leaves the machine, nothing is written to GitHub.
import type { BoardView, RowView } from "../bridge";

const KEY = "vults.seen";

/** What a row says; a seen row comes back once this changes (its checks, its review, or a
 *  branch's latest commit, which is in its title). */
const said = (r: RowView) => `${r.checks ?? ""}|${r.review ?? ""}|${r.title}`;

/** Each connector's marks, by row. */
let marks: Record<string, Record<string, string>> = load();

function load(): Record<string, Record<string, string>> {
  try {
    const kept = JSON.parse(localStorage.getItem(KEY) ?? "{}") as unknown;
    return kept && typeof kept === "object" && !Array.isArray(kept) ? (kept as Record<string, Record<string, string>>) : {};
  } catch {
    return {};
  }
}

function save(): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(marks));
  } catch {
    // Without storage the marks last until the island restarts.
  }
}

export function isSeen(connector: string, r: RowView): boolean {
  return marks[connector]?.[r.item] === said(r);
}

/** The rows Quick still shows. */
export function unseen(board: BoardView): RowView[] {
  return board.rows.filter((r) => !isSeen(board.connector, r));
}

/** Marks rows seen, and forgets the marks of this connector's rows that are gone: one coming
 *  back later is news again. */
export function markSeen(board: BoardView, rows: RowView[]): void {
  const open = new Set(board.rows.map((r) => r.item));
  const kept = Object.fromEntries(Object.entries(marks[board.connector] ?? {}).filter(([item]) => open.has(item)));
  for (const r of rows) kept[r.item] = said(r);
  marks = { ...marks, [board.connector]: kept };
  save();
}

/** Changes whenever a mark does, for the card's rebuild check. */
export function seenSig(): string {
  return JSON.stringify(marks);
}
