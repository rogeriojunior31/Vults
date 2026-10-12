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

export function createConnections(actions: ConnectionsActions): {
  element: HTMLElement;
  update: (v: ViewModel) => void;
  refresh: () => void;
  /** Draws again with what it has: the language changed. */
  redraw: () => void;
} {
  const element = el("div", { class: "nest-connections" });
  let view: ViewModel | null = null;
  let status = new Map<string, ConnectorStatus>();
  /** The status answered once: until then nothing is offered, so an enabled connector that has
   * not polled yet is never shown as one to add. */
  let known = false;
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
    // Add a connection: only connectors that exist (ADR 0011) and are off. One that is on has its
    // card; turning one on stays in Settings, the only window that may (ADR 0022).
    // A card that exists means on, whatever an older status said.
    const hasCard = (id: string) => all.some((b) => b.connector === id);
    const isOn = (id: string) => hasCard(id) || status.get(id)?.enabled === true;
    const off = known ? CONNECTORS.filter((c) => !isOn(c.id)) : [];
    // On, but its first poll has not brought a card yet.
    const pending = CONNECTORS.filter((c) => !hasCard(c.id) && status.get(c.id)?.enabled === true);
    const add = off.length
      ? el(
          "section",
          { class: "nest-add" },
          el("h3", { text: t("Add a connection") }),
          ...off.map((c) =>
            el(
              "div",
              { class: "nest-add-row" },
              el("p", {}, el("strong", { text: c.name }), el("span", { text: t(c.about) })),
              el("button", { class: "btn small", text: t("Turn on in Settings…"), onclick: actions.settings }),
            ),
          ),
        )
      : null;
    element.replaceChildren(
      ...(chips ? [chips] : []),
      ...boards.map((b) => {
        const st = status.get(b.connector);
        return boardCard(
          b,
          name(b.connector),
          (item) => actions.open(b.connector, item),
          st ? staleNote(st, Date.now() / 1000) : null,
          filter === "all" ? null : t("Nothing matches this filter."),
        );
      }),
      ...pending.map((c) => el("p", { class: "board-empty", text: t("{name} is on; its card shows after its first check.", { name: c.name }) })),
      ...(add ? [add] : []),
      ...(boards.length || !add
        ? [el("p", { class: "board-empty" }, el("button", { class: "btn small", text: t("Manage connections"), onclick: actions.settings }))]
        : []),
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
      void actions.status().then(
        (all) => {
          status = new Map(all.map((c) => [c.id, c]));
          known = true;
          draw();
        },
        () => {
          known = true;
          draw();
        },
      );
    },
  };
}
