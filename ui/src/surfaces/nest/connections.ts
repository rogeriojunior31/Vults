// The Nest's Connections tab (ADR 0021): each connector that is on, with the card the island shows.
// Only connectors that exist are offered (ADR 0011); turning one on stays in Settings.
import type { ConnectorStatus } from "../../bridge";
import { CONNECTORS } from "../../connectors";
import { el } from "../../dom";
import { t } from "../../i18n";
import { boardCard, staleNote } from "../../island/board";
import type { ViewModel } from "../../view.gen";

export interface ConnectionsActions {
  open: (connector: string, item: string) => void;
  status: () => Promise<ConnectorStatus[]>;
  /** Settings → Connectors. */
  settings: () => void;
}

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
  const draw = () => {
    const boards = view?.boards ?? [];
    const name = (id: string) =>
      CONNECTORS.find((c) => c.id === id)?.name ?? id;
    element.replaceChildren(
      ...boards.map((b) => {
        const st = status.get(b.connector);
        return boardCard(
          b,
          name(b.connector),
          (item) => on.open(b.connector, item),
          st ? staleNote(st, Date.now() / 1000) : null,
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
