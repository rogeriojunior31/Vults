// The Nest's Connections tab (ADR 0021), as the Living Flock prototype draws it: a chip for each
// connector that is on, then the chosen one's head (when it last synced, Refresh), its filters as
// tabs, its rows by group, and the row picked in detail beside them. Only connectors that exist are
// offered (ADR 0011), and only what each row holds is shown; turning one on stays in Settings.
import type { BoardView, ConnectorStatus } from "../../bridge";
import { CONNECTORS } from "../../connectors";
import { el } from "../../dom";
import { t } from "../../i18n";
import { ago, CHECKS, GROUPS, staleNote, tag } from "../../island/board";
import { icon } from "../../island/icons";
import type { RowView, ViewModel } from "../../view.gen";

export interface ConnectionsActions {
  open: (connector: string, item: string) => void;
  status: () => Promise<ConnectorStatus[]>;
  /** Polls every connector now. */
  refresh: () => Promise<void>;
  /** Settings → Connectors. */
  settings: () => void;
}

type Filter = "all" | "review" | "failed";
const FILTERS: Record<Filter, (r: RowView) => boolean> = {
  all: () => true,
  review: (r) => r.group === "to-review",
  failed: (r) => r.checks === "failing",
};

/** A row's state as the prototype colors its icon: what needs doing first. */
const stateOf = (r: RowView): string => (r.checks === "failing" || r.review === "changes" ? "bad" : r.review === "approved" || r.checks === "passing" ? "good" : r.checks === "running" ? "running" : "plain");

/** Every row carries a word at its end, as in the prototype: the card's own, else its checks, else
 *  that a review waits. */
const label = (r: RowView): { text: string; kind: string } | null =>
  tag(r) ?? (r.checks ? { text: t(CHECKS[r.checks]), kind: r.checks } : r.group === "to-review" ? { text: t("Review requested"), kind: "ask" } : null);

/** Marks a control so the keyboard focus comes back to it when the tab is drawn again. */
function fid<E extends HTMLElement>(e: E, id: string): E {
  e.dataset.fid = id;
  return e;
}

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
  /** What the rows show: everything, the reviews waiting for you, or the failing checks. */
  let filter: Filter = "all";
  /** The connector shown, and the row picked in it. */
  let chosen: string | null = null;
  let picked: string | null = null;
  let refreshing = false;
  const name = (id: string) => CONNECTORS.find((c) => c.id === id)?.name ?? id;

  const row = (r: RowView) => {
    const end = label(r);
    const line = fid(
      el(
        "button",
        {
          class: `conn-row ${stateOf(r)}${r.item === picked ? " on" : ""}`,
          onclick: () => {
            picked = r.item;
            draw();
          },
        },
        icon("pull", 15, 1.6),
        el("span", { class: "conn-row-words" }, el("strong", { text: r.title }), el("small", { text: r.name })),
        end ? el("span", { class: `conn-tag ${end.kind}`, text: end.text }) : null,
      ),
      `row:${r.item}`,
    );
    line.setAttribute("aria-pressed", String(r.item === picked));
    line.title = r.title;
    return line;
  };

  /** The row picked, with what the connector said of it and the way to it. */
  const detail = (b: BoardView, r: RowView) => {
    const end = tag(r);
    const group = GROUPS.find((g) => g.id === r.group);
    const facts: [string, string | null][] = [
      [t("Kind"), group ? t(group.label) : null],
      [t("Checks"), r.checks ? t(CHECKS[r.checks]) : null],
      [t("Review"), end && r.review ? end.text : null],
    ];
    return el(
      "section",
      { class: `conn-detail ${stateOf(r)}` },
      el("div", { class: "conn-detail-head" }, icon("pull", 14, 1.6), el("span", { text: r.name }), end ? el("span", { class: `conn-tag ${end.kind}`, text: end.text }) : null),
      el("h3", { text: r.title }),
      el("dl", { class: "context-dl" }, ...facts.flatMap(([k, v]) => (v ? [el("dt", { text: k }), el("dd", { text: v })] : []))),
      r.link
        ? fid(el("button", { class: "primary", text: t("Open in {name} ↗", { name: name(b.connector) }), onclick: () => actions.open(b.connector, r.item) }), "act:open")
        : null,
    );
  };

  const draw = () => {
    const had = (document.activeElement as HTMLElement | null)?.dataset?.fid ?? null;
    const all = view?.boards ?? [];
    if (!chosen || !all.some((b) => b.connector === chosen)) chosen = all[0]?.connector ?? null;
    const board = all.find((b) => b.connector === chosen) ?? null;
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
              el("button", { class: "outline", text: t("Turn on in Settings…"), onclick: actions.settings }),
            ),
          ),
        )
      : null;
    const chips = all.length
      ? el(
          "div",
          { class: "conn-chips" },
          ...all.map((b) => {
            const c = fid(
              el(
                "button",
                {
                  class: `conn-chip${b.connector === chosen ? " on" : ""}`,
                  onclick: () => {
                    chosen = b.connector;
                    picked = null;
                    draw();
                  },
                },
                icon("pull", 13, 1.6),
                el("span", { text: name(b.connector) }),
                el("span", { class: "conn-count", text: String(b.rows.length) }),
              ),
              `chip:${b.connector}`,
            );
            c.setAttribute("aria-pressed", String(b.connector === chosen));
            return c;
          }),
          fid(el("button", { class: "conn-chip add", text: "+", onclick: actions.settings }), "chip:add"),
        )
      : null;
    if (chips) {
      const plus = chips.lastElementChild as HTMLElement;
      plus.title = t("Manage connections");
      plus.setAttribute("aria-label", t("Manage connections"));
    }
    let body: HTMLElement[] = [];
    if (board) {
      const st = status.get(board.connector);
      const now = Date.now() / 1000;
      const stale = st ? staleNote(st, now) : null;
      const line = stale ?? (st?.lastOk ? t("Synced {age}", { age: ago(st.lastOk, now) }) : null);
      const count = (f: Filter) => board.rows.filter(FILTERS[f]).length;
      const tab = (f: Filter, word: string) => {
        const b = fid(
          el("button", {
            class: `detail-tab nest-filter${f === filter ? " on" : ""}`,
            text: word,
            onclick: () => {
              filter = f;
              picked = null;
              draw();
            },
          }),
          `filter:${f}`,
        );
        b.setAttribute("aria-pressed", String(f === filter));
        return b;
      };
      const rows = board.rows.filter(FILTERS[filter]);
      if (!picked || !rows.some((r) => r.item === picked)) picked = rows[0]?.item ?? null;
      const groups = GROUPS.flatMap((g) => {
        const mine = rows.filter((r) => r.group === g.id);
        return mine.length ? [el("div", { class: "records-head", text: t(g.label) }), ...mine.map((r) => row(r))] : [];
      });
      const pick = rows.find((r) => r.item === picked);
      const refresh = fid(
        el("button", {
          class: "outline",
          text: t("↻ Refresh"),
          onclick: () => {
            if (refreshing) return;
            refreshing = true;
            draw();
            void actions.refresh().finally(() => {
              refreshing = false;
              draw();
            });
          },
        }),
        "act:refresh",
      );
      (refresh as HTMLButtonElement).disabled = refreshing;
      body = [
        el(
          "header",
          { class: "conn-head" },
          el("div", {}, el("h2", { text: name(board.connector) }), line ? el("p", { class: `conn-status${stale ? " stale" : ""}`, text: line }) : null),
          el("div", { class: "conn-actions" }, refresh, el("button", { class: "outline", text: t("Manage connections"), onclick: actions.settings })),
        ),
        el("nav", { class: "detail-tabs conn-tabs" }, tab("all", t("All {n}", { n: count("all") })), tab("review", t("Waiting for your review {n}", { n: count("review") })), tab("failed", t("Checks failed {n}", { n: count("failed") }))),
        el(
          "div",
          { class: "conn-body" },
          el("div", { class: "conn-list" }, ...(groups.length ? groups : [el("p", { class: "board-empty", text: filter === "all" ? t("Nothing open right now.") : t("Nothing matches this filter.") })])),
          pick ? detail(board, pick) : null,
        ),
      ];
    }
    element.replaceChildren(
      ...(chips ? [chips] : []),
      ...body,
      ...pending.map((c) => el("p", { class: "board-empty", text: t("{name} is on; its card shows after its first check.", { name: c.name }) })),
      ...(add ? [add] : []),
      ...(!board && !add ? [el("p", { class: "board-empty" }, el("button", { class: "outline", text: t("Manage connections"), onclick: actions.settings }))] : []),
    );
    if (had) element.querySelector<HTMLElement>(`[data-fid="${CSS.escape(had)}"]`)?.focus();
  };
  return {
    element,
    update: (v) => {
      if (JSON.stringify(v.boards ?? []) === JSON.stringify(view?.boards ?? [])) {
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
