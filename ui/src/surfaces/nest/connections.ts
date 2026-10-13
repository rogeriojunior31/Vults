// The Nest's Connections tab (ADR 0021), as the Living Flock prototype draws it: a chip for each
// connector that is on, then the chosen one's head (when it last synced, a repository filter,
// Refresh), its tabs (Inbox, Pull requests, Actions, Releases), its rows, and the row picked in
// detail beside them: its facts, each check, a command to copy. Only connectors that exist are
// offered (ADR 0011), and only what each row holds is shown; turning one on stays in Settings.
import type { BoardView, ConnectorStatus } from "../../bridge";
import { CONNECTORS } from "../../connectors";
import { el } from "../../dom";
import { t, tk } from "../../i18n";
import { ago, CHECKS, GROUPS, question, staleNote, tag } from "../../island/board";
import { isSeen, markSeen, onSeen } from "../../island/seen";
import { icon } from "../../island/icons";
import type { CheckState, RowView, Said, ViewModel } from "../../view.gen";

export interface ConnectionsActions {
  open: (connector: string, item: string) => void;
  status: () => Promise<ConnectorStatus[]>;
  /** Polls every connector now. */
  refresh: () => Promise<void>;
  /** Settings → Connectors. */
  settings: () => void;
  /** *Squash and merge*, confirmed: the core merges only a row it offered it for. Rejects when the
   *  app did not take it. */
  merge: (connector: string, item: string) => Promise<void>;
  /** How each merge went, from the app. */
  onMerged: (cb: (m: { connector: string; item: string; error: string | null }) => void) => void;
}

/** What the tab adds for itself: the Chat with a question in its composer, only with Zeca on. */
export interface ConnectionsAsk {
  ask: ((text: string) => void) | null;
}

type Tab = "inbox" | "prs" | "runs" | "rels";
/** What waits on the user: a review asked of them, their pull request failing, sent back or ready
 *  to merge, a default branch failing. */
const needs = (r: RowView) =>
  r.group === "to-review" || (r.group === "yours" && (r.checks === "failing" || r.review === "changes" || r.merge)) || (r.group === "branches" && r.checks === "failing");
const TABS: Record<Tab, (r: RowView) => boolean> = {
  inbox: needs,
  prs: (r) => r.group === "yours" || r.group === "to-review",
  runs: (r) => r.group === "branches",
  rels: (r) => r.group === "releases",
};
/** The Inbox's sections, in the prototype's order. */
const INBOX: { label: string; has: (r: RowView) => boolean }[] = [
  { label: tk("Review requests"), has: (r) => r.group === "to-review" },
  { label: tk("Your pull requests"), has: (r) => r.group === "yours" },
  { label: tk("Failing"), has: (r) => r.group === "branches" },
];
const RUN: Record<CheckState, string> = { passed: tk("passed"), failed: tk("failed"), running: tk("running"), waiting: tk("waiting"), skipped: tk("skipped") };
const RELEASE = { latest: tk("Latest release"), prerelease: tk("Pre-release"), draft: tk("Draft") } as const;
const iconOf = (r: RowView) => (r.group === "branches" ? "branch" : r.group === "releases" ? "tag" : "pull");

/** Where its reviewers stand, one sentence each, an answer before a wait. */
function reviewLine(r: RowView): string | null {
  const who = (s: Said) => r.facts.reviewers.filter((x) => x.said === s).map((x) => `@${x.login}`).join(", ");
  const parts = [
    who("changes") && t("{who} asked for changes", { who: who("changes") }),
    who("approved") && t("Approved by {who}", { who: who("approved") }),
    who("commented") && t("{who} commented", { who: who("commented") }),
    who("waiting") && t("Waiting on {who}", { who: who("waiting") }),
  ].filter(Boolean);
  return parts.length ? parts.join(" · ") : null;
}

/** `+120 −40 · 6 files`. */
function diffLine(r: RowView): string | null {
  const d = r.facts.diff;
  if (!d) return null;
  return d.files === 1 ? t("+{added} −{removed} · 1 file", { added: d.added, removed: d.removed }) : t("+{added} −{removed} · {files} files", { added: d.added, removed: d.removed, files: d.files });
}

/** A row's state as the prototype colors its icon: what needs doing first. */
const stateOf = (r: RowView): string => (r.checks === "failing" || r.review === "changes" ? "bad" : r.review === "approved" || r.checks === "passing" ? "good" : r.checks === "running" ? "running" : "plain");

/** Every row carries a word at its end, as in the prototype: the card's own, else its checks, else
 *  that a review waits. */
const label = (r: RowView): { text: string; kind: string } | null =>
  r.facts.release ? { text: t(RELEASE[r.facts.release]), kind: r.facts.release } : tag(r) ?? (r.checks ? { text: t(CHECKS[r.checks]), kind: r.checks } : r.group === "to-review" ? { text: t("Review requested"), kind: "requested" } : null);

/** Marks a control so the keyboard focus comes back to it when the tab is drawn again. */
function fid<E extends HTMLElement>(e: E, id: string): E {
  e.dataset.fid = id;
  return e;
}

export function createConnections(actions: ConnectionsActions & { zeca: () => ConnectionsAsk["ask"] }): {
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
  /** The tab shown, none picked yet: the Inbox when something waits, else the pull requests. */
  let tab: Tab | null = null;
  /** One repository's rows only (`owner/repo`), or all of them. */
  let repo: string | null = null;
  /** The row whose command was just copied, for the button to say so a moment. */
  let copied: string | null = null;
  /** The connector shown, and the row picked in it. */
  let chosen: string | null = null;
  let picked: string | null = null;
  /** At narrow widths a selected row becomes its own view. */
  let detailOpen = false;
  let refreshing = false;
  /** The row whose merge is being confirmed, and how each merge asked for here went. */
  let asking: string | null = null;
  const merges = new Map<string, { state: "running" | "merged" } | { state: "refused"; error: string }>();
  actions.onMerged((m) => {
    merges.set(`${m.connector}\n${m.item}`, m.error ? { state: "refused", error: m.error } : { state: "merged" });
    draw();
  });
  const busy = (m: ReturnType<typeof merges.get>) => m?.state === "running" || m?.state === "merged";
  // A row marked seen on the island dims here too.
  onSeen(() => draw());
  const name = (id: string) => CONNECTORS.find((c) => c.id === id)?.name ?? id;

  const bar = (part: number) => {
    const b = el("b");
    b.style.width = `${Math.round(part * 100)}%`;
    const line = el("div", { class: "conn-prog" }, b);
    line.setAttribute("role", "progressbar");
    line.setAttribute("aria-valuenow", String(Math.round(part * 100)));
    line.setAttribute("aria-label", t("Checks done"));
    return line;
  };

  /** A row's second line: its name, then its branch, or when it last changed. */
  const sub = (r: RowView) => {
    const f = r.facts;
    const more = r.group !== "branches" && f.branch ? f.branch : f.updated !== null ? ago(f.updated, Date.now() / 1000) : null;
    return more ? `${r.name} · ${more}` : r.name;
  };

  const row = (b: BoardView, r: RowView) => {
    const end = label(r);
    const line = fid(
      el(
        "button",
        {
          // Seen rows stay here, dimmed: they left Quick, not the panel.
          class: `conn-row ${stateOf(r)}${r.item === picked ? " on" : ""}${isSeen(b.connector, r) ? " seen" : ""}`,
          onclick: () => {
            picked = r.item;
            if (window.innerWidth < 900) detailOpen = true;
            draw();
          },
        },
        icon(iconOf(r), 15, 1.6),
        el("span", { class: "conn-row-words" }, el("strong", { text: r.title }), el("small", { text: sub(r) })),
        end ? el("span", { class: `conn-tag ${end.kind}`, text: end.text }) : null,
      ),
      `row:${r.item}`,
    );
    line.setAttribute("aria-pressed", String(r.item === picked));
    line.title = r.title;
    return line;
  };

  /** The row picked, with what the connector said of it and the way to it. */
  const detail = (b: BoardView, r: RowView, narrow: boolean) => {
    const end = label(r);
    const f = r.facts;
    const when = f.updated !== null ? ago(f.updated, Date.now() / 1000) : null;
    const passed = f.runs.filter((c) => c.state === "passed").length;
    // Each check is listed below; without them the word for all of them stands here.
    const checks = f.runs.length ? t("{passed} of {n} passed", { passed, n: f.runs.length }) : r.checks ? t(CHECKS[r.checks]) : null;
    const facts: [string, string | null][] =
      r.group === "releases"
        ? [
            [t("State"), f.release ? t(RELEASE[f.release]) : null],
            [f.release === "draft" ? t("Created") : t("Published"), when],
          ]
        : r.group === "branches"
          ? [
              [t("Branch"), f.branch],
              [t("Checks"), checks],
              [t("Updated"), when],
            ]
          : [
              [t("Author"), f.author ? `@${f.author}` : null],
              [t("Branch"), f.branch],
              [t("Diff"), diffLine(r)],
              [t("Reviews"), reviewLine(r) ?? (r.review ? tag(r)?.text ?? null : null)],
              [t("Checks"), f.runs.length ? null : checks],
              [t("Updated"), when],
            ];
    // While checks run, how many are done.
    const done = f.runs.filter((c) => c.state !== "running" && c.state !== "waiting").length;
    const progress = r.checks === "running" && f.runs.length ? done / f.runs.length : null;
    const command = f.command;
    const ref = f.repo ? r.name.replace(/^[^# ]+/, f.repo) : r.name;
    const k = `${b.connector}\n${r.item}`;
    const merging = merges.get(k);
    const ask = actions.zeca();
    const q = ask ? question(r) : null;
    // The prototype's actions: the row's own (merge, a question for Zeca), the way to it, and ✓.
    const acts: HTMLElement[] = [];
    if (r.merge) {
      const label = merging?.state === "running" ? t("Merging…") : merging?.state === "merged" ? t("Merged") : t("Squash and merge");
      const go = fid(el("button", { class: "primary", text: label, onclick: () => ((asking = k), draw()) }), "act:merge");
      go.disabled = busy(merging);
      acts.push(go);
    }
    if (q && ask) acts.push(fid(el("button", { class: r.merge ? "outline" : "primary", text: q.label, onclick: () => ask(q.text) }), "act:zeca"));
    if (r.link) acts.push(fid(el("button", { class: acts.length ? "outline" : "primary", text: t("Open in {name} ↗", { name: name(b.connector) }), onclick: () => actions.open(b.connector, r.item) }), "act:open"));
    // A release never reaches Quick: nothing for ✓ to clear.
    if (r.group !== "releases" && !isSeen(b.connector, r)) {
      const done = fid(el("button", { class: "outline", text: t("✓ Seen"), onclick: () => (markSeen(b, [r]), draw()) }), "act:seen");
      done.title = t("Seen: leaves Quick until it changes, stays in Room");
      acts.push(done);
    }
    // Two clicks: the first only asks what the second will do on GitHub.
    const confirm =
      r.merge && asking === k && !busy(merging)
        ? el(
            "div",
            { class: "conn-confirm" },
            el("span", { text: t("Squash {name} into its base branch and delete its branch on GitHub?", { name: r.name }) }),
            fid(el("button", { class: "outline", text: t("Cancel"), onclick: () => ((asking = null), draw()) }), "act:merge-cancel"),
            fid(
              el("button", {
                class: "primary",
                text: t("Merge"),
                onclick: () => {
                  asking = null;
                  merges.set(k, { state: "running" });
                  actions.merge(b.connector, r.item).catch(() => {
                    merges.set(k, { state: "refused", error: t("The app did not take it.") });
                    draw();
                  });
                  draw();
                },
              }),
              "act:merge-yes",
            ),
          )
        : null;
    return el(
      "section",
      { class: `conn-detail ${stateOf(r)}` },
      narrow
        ? fid(
            el("button", {
              class: "outline conn-back",
              text: `← ${t("Connections")}`,
              onclick: () => {
                detailOpen = false;
                draw();
                element.querySelector<HTMLElement>(`[data-fid="row:${CSS.escape(r.item)}"]`)?.focus();
              },
            }),
            "act:back-detail",
          )
        : null,
      el("div", { class: "conn-detail-head" }, icon(iconOf(r), 14, 1.6), el("span", { text: ref }), end ? el("span", { class: `conn-tag ${end.kind}`, text: end.text }) : null),
      el("h3", { text: r.title }),
      el("dl", { class: "context-dl" }, ...facts.flatMap(([k, v]) => (v ? [el("dt", { text: k }), el("dd", { text: v })] : []))),
      progress !== null ? bar(progress) : null,
      f.runs.length
        ? el(
            "div",
            { class: "conn-checks" },
            ...f.runs.map((c) => el("div", { class: `conn-check ${c.state}` }, el("i"), el("span", { text: c.name }), el("span", { text: t(RUN[c.state]) }))),
          )
        : null,
      acts.length ? el("div", { class: "conn-acts" }, ...acts) : null,
      confirm,
      merging?.state === "refused" ? el("p", { class: "conn-refused", text: t("GitHub did not merge it: {error}", { error: merging.error }) }) : null,
      command
        ? el(
            "div",
            { class: "conn-cmd" },
            el("code", { text: command }),
            fid(
              el("button", {
                text: copied === k ? t("Copied") : t("Copy"),
                onclick: () =>
                  void navigator.clipboard
                    ?.writeText(command)
                    .then(() => {
                      copied = k;
                      draw();
                      window.setTimeout(() => {
                        if (copied === k) (copied = null), draw();
                      }, 1500);
                    })
                    .catch(() => {}),
              }),
              "act:copy",
            ),
          )
        : null,
    );
  };

  const draw = () => {
    const had = (document.activeElement as HTMLElement | null)?.dataset?.fid ?? null;
    // A merge whose row left the card is done with; one the core no longer offers (its checks ran
    // again, a push moved its head) got no answer, so it may be asked again once it is green.
    for (const [k, m] of [...merges]) {
      const [connector, item] = k.split("\n");
      const r = view?.boards?.find((b) => b.connector === connector)?.rows.find((x) => x.item === item);
      if (!r || (m.state === "running" && !r.merge)) merges.delete(k);
    }
    const all = view?.boards ?? [];
    if (!chosen || !all.some((b) => b.connector === chosen)) chosen = all[0]?.connector ?? null;
    const board = all.find((b) => b.connector === chosen) ?? null;
    // Add a connection: only connectors that exist (ADR 0011) and are off. One that is on has its
    // card; turning one on stays in Settings, the only window that may (ADR 0021).
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
              fid(el("button", { class: "outline", text: t("Turn on in Settings…"), onclick: actions.settings }), `add:${c.id}`),
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
                    detailOpen = false;
                    draw();
                  },
                },
                icon("pull", 13, 1.6),
                el("span", { text: name(b.connector) }),
                // What is open there; a release is not.
                el("span", { class: "conn-count", text: String(b.rows.filter((r) => r.group !== "releases").length) }),
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
      // The repositories it has rows for, in the order GitHub gave them.
      const repos = [...new Set(board.rows.flatMap((r) => (r.facts.repo ? [r.facts.repo] : [])))];
      if (repo && !repos.includes(repo)) repo = null;
      const mine = board.rows.filter((r) => !repo || r.facts.repo === repo);
      const count = (id: Tab) => mine.filter(TABS[id]).length;
      const shown: Tab = tab ?? (count("inbox") ? "inbox" : "prs");
      const fresh = mine.filter((r) => needs(r) && !isSeen(board.connector, r)).length;
      const failing = mine.filter((r) => r.group === "branches" && r.checks === "failing").length;
      const tabButton = (id: Tab, word: string, badge: string | null, hot = false) => {
        const b = fid(
          el(
            "button",
            {
              class: `detail-tab nest-filter${id === shown ? " on" : ""}`,
              onclick: () => {
                tab = id;
                picked = null;
                detailOpen = false;
                draw();
              },
            },
            el("span", { text: word }),
            badge ? el("b", { class: hot ? "hot" : "", text: badge }) : null,
          ),
          `tab:${id}`,
        );
        b.setAttribute("aria-pressed", String(id === shown));
        return b;
      };
      const rows = mine.filter(TABS[shown]);
      const heads =
        shown === "inbox"
          ? INBOX.map((g) => ({ label: t(g.label), has: g.has }))
          : shown === "prs"
            ? GROUPS.filter((g) => g.id !== "branches").map((g) => ({ label: t(g.label), has: (r: RowView) => r.group === g.id }))
            : [{ label: "", has: () => true }];
      // The first row shown is picked, in the sections' order.
      if (!picked || !rows.some((r) => r.item === picked)) picked = heads.flatMap((g) => rows.filter(g.has))[0]?.item ?? null;
      const groups = heads.flatMap((g) => {
        const these = rows.filter(g.has);
        return these.length ? [...(g.label ? [el("div", { class: "records-head", text: g.label })] : []), ...these.map((r) => row(board, r))] : [];
      });
      const empty = shown === "inbox" ? t("Nothing needs you right now.") : repo ? t("Nothing here for this repository.") : t("Nothing open right now.");
      const option = (value: string, text: string) => {
        const o = el("option", { text });
        o.value = value;
        return o;
      };
      const pickRepo = repos.length > 1 ? fid(el("select", { class: "conn-repo" }, option("", t("All repos")), ...repos.map((x) => option(x, x))), "act:repo") : null;
      if (pickRepo) {
        pickRepo.value = repo ?? "";
        pickRepo.setAttribute("aria-label", t("Repository"));
        pickRepo.addEventListener("change", () => {
          repo = pickRepo.value || null;
          picked = null;
          detailOpen = false;
          draw();
        });
      }
      const pick = rows.find((r) => r.item === picked);
      // Asks the connectors again; the answer comes as new rows and a new status, read after. The
      // button stays enabled (and keeps the keyboard): a second click while asking is ignored.
      const refresh = fid(
        el("button", {
          class: "outline",
          text: t("↻ Refresh"),
          onclick: () => {
            if (refreshing) return;
            refreshing = true;
            void actions.refresh().finally(() => {
              refreshing = false;
              readStatus();
            });
          },
        }),
        "act:refresh",
      );
      body = [
        el(
          "header",
          { class: "conn-head" },
          el("div", {}, el("h2", { text: name(board.connector) }), line ? el("p", { class: `conn-status${stale ? " stale" : ""}`, text: line }) : null),
          el("div", { class: "conn-actions" }, pickRepo, refresh, fid(el("button", { class: "outline", text: t("Manage connections"), onclick: actions.settings }), "act:manage")),
        ),
        el(
          "nav",
          { class: "detail-tabs conn-tabs" },
          tabButton("inbox", t("Inbox"), fresh ? t("{n} new", { n: fresh }) : null),
          tabButton("prs", t("Pull requests"), String(count("prs"))),
          tabButton("runs", t("Actions"), failing ? t("{n} failing", { n: failing }) : String(count("runs")), failing > 0),
          tabButton("rels", t("Releases"), String(count("rels"))),
        ),
        (() => {
          const narrow = window.innerWidth < 900;
          const showDetail = !!pick && (!narrow || detailOpen);
          return el(
            "div",
            { class: `conn-body${showDetail && narrow ? " detail-open" : ""}` },
            narrow && showDetail ? null : el("div", { class: "conn-list" }, ...(groups.length ? groups : [el("p", { class: "board-empty", text: empty })])),
            showDetail ? detail(board, pick!, narrow) : null,
          );
        })(),
      ];
    }
    element.replaceChildren(
      ...(chips ? [chips] : []),
      ...body,
      ...pending.map((c) => el("p", { class: "board-empty", text: t("{name} is on; its card shows after its first check.", { name: c.name }) })),
      ...(add ? [add] : []),
      ...(!board && !add ? [el("p", { class: "board-empty" }, el("button", { class: "outline", text: t("Manage connections"), onclick: actions.settings }))] : []),
    );
    if (had) {
      // A button gone or disabled by its own click (Cancel, Merge, ✓) hands the focus to the
      // detail's next action, not to the page.
      const at = (id: string) => element.querySelector<HTMLButtonElement>(`[data-fid="${CSS.escape(id)}"]:not(:disabled)`);
      (at(had) ?? (had.startsWith("act:") ? (at("act:merge") ?? at("act:zeca") ?? at("act:open")) : null))?.focus();
    }
  };
  /** Reads when each connector last synced (and why a poll failed): the head says it as a fact. */
  const readStatus = () => {
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
  };
  // "Synced 2 min ago" ages while the tab is open; a failed poll changes no row. Asked again each
  // minute, only while the tab is in sight.
  window.setInterval(() => {
    if (element.isConnected && element.offsetParent !== null && !document.hidden) readStatus();
  }, 60_000);
  return {
    element,
    update: (v) => {
      if (JSON.stringify(v.boards ?? []) === JSON.stringify(view?.boards ?? [])) {
        view = v;
        return;
      }
      view = v;
      // New rows came from a poll: its status is new too.
      readStatus();
    },
    redraw: () => draw(),
    refresh: () => {
      draw();
      readStatus();
    },
  };
}
