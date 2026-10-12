// The Nest's Connections tab (ADR 0021): each connector that is on, with the card the island shows.
// Only connectors that exist are offered (ADR 0011); turning one on stays in Settings.
import type { ConnectorStatus } from "../../bridge";
import { CONNECTORS } from "../../connectors";
import { el } from "../../dom";
import { t } from "../../i18n";
import { boardCard, staleNote } from "../../island/board";
import type { RowView, ViewModel } from "../../view.gen";

export interface ConnectionsActions {
  open: (connector: string, item: string) => void;
  status: () => Promise<ConnectorStatus[]>;
  /** Settings → Connectors. */
  settings: () => void;
}

type Filter = "all" | "review" | "failed";
const FILTERS: Record<Filter, (r: RowView) => boolean> = {
  all: () => true,
  review: (r) => r.group === "to-review",
  failed: (r) => r.checks === "failing",
};

export function createConnections(on: ConnectionsActions): {
  element: HTMLElement;
  update: (v: ViewModel) => void;
  refresh: () => void;
  /** Draws again with what it has: the language changed. */
  redraw: () => void;
} {
  const element = el("div", { class: "nest-connections" });
  let view: ViewModel | null = null;
  let status = new Map<string, ConnectorStatus>();
  /** What the cards show: everything, the reviews waiting for you, or the failing checks. */
  let filter: Filter = "all";
  const draw = () => {
    const all = view?.boards ?? [];
    const boards = all.map((b) => ({ ...b, rows: b.rows.filter(FILTERS[filter]) }));
    const count = (f: Filter) => all.reduce((n, b) => n + b.rows.filter(FILTERS[f]).length, 0);
    const chip = (f: Filter, label: (n: number) => string) => {
      const b = el("button", { class: `btn small nest-filter${f === filter ? " on" : ""}`, text: label(count(f)) });
      b.setAttribute("aria-pressed", String(f === filter));
      b.addEventListener("click", () => {
        filter = f;
        draw();
      });
      return b;
    };
    const chips = all.length
      ? el("div", { class: "nest-filters" }, chip("all", (n) => t("All {n}", { n })), chip("review", (n) => t("Waiting for your review {n}", { n })), chip("failed", (n) => t("Checks failed {n}", { n })))
      : null;
    const name = (id: string) =>
      CONNECTORS.find((c) => c.id === id)?.name ?? id;
    element.replaceChildren(
      ...(chips ? [chips] : []),
      ...boards.map((b) => {
        const st = status.get(b.connector);
        return boardCard(
          b,
          name(b.connector),
          (item) => on.open(b.connector, item),
          st ? staleNote(st, Date.now() / 1000) : null,
          filter === "all" ? null : t("Nothing matches this filter."),
        );
      }),
      boards.length
        ? el(
            "p",
            { class: "board-empty" },
            el("button", {
              class: "btn small",
              text: t("Manage connections"),
              onclick: on.settings,
            }),
          )
        : el(
            "p",
            { class: "board-empty" },
            el("span", {
              text:
                t(
                  "No connection is on yet. Turn one on in Settings, Connectors.",
                ) + " ",
            }),
            el("button", {
              class: "btn small",
              text: t("Manage connections"),
              onclick: on.settings,
            }),
          ),
    );
  };
  return {
    element,
    update: (v) => {
      if (
        JSON.stringify(v.boards ?? []) === JSON.stringify(view?.boards ?? [])
      ) {
        view = v;
        return;
      }
      view = v;
      draw();
    },
    redraw: () => draw(),
    refresh: () => {
      draw();
      void on.status().then(
        (all) => {
          status = new Map(all.map((c) => [c.id, c]));
          draw();
        },
        () => {},
      );
    },
  };
}
