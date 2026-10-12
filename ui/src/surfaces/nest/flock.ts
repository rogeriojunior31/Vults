// The Nest's Flock tab (ADR 0021): the flock at 2x in its habitat, a search and a *Needs you*
// filter, every session in a row, and the one in front in detail: its changes, its steps, where it
// works and (with Zeca on) a question about it. Nothing here answers a card: a session that waits
// leads to the island (D1).
import type { SessionView, ViewModel } from "../../view.gen";
import { el } from "../../dom";
import { t } from "../../i18n";
import { assignSpecies } from "../../island/flock";
import { NEST_HABITAT_SCENE, Scene } from "../../island/scene";
import { agentName, badgeOf, statusClass, statusText } from "../../island/views";

export interface FlockActions {
  /** Puts the session in front of the flock (core's focus). */
  focus: (s: SessionView) => void;
  /** Its terminal, where `raise` says it can be brought forward. */
  jump: (s: SessionView) => void;
  /** The island, on the card this session waits on. */
  review: (s: SessionView) => void;
}

/** What the tab adds for itself: the Chat, with a question about a session (only with Zeca on). */
export interface FlockAsk {
  ask: (s: SessionView) => void;
}

const key = (s: SessionView) => `${s.agent}:${s.id}`;
const waits = (s: SessionView) => s.status === "approval" || s.status === "question" || s.waiting === true;

/** What the search reads: the project, its roost and branch, the agent and what it does now. */
const said = (s: SessionView) => [s.project, s.roost, s.branch, agentName(s), statusText(s)].filter(Boolean).join(" ").toLowerCase();

function row(s: SessionView, on: FlockActions, front: boolean): HTMLElement {
  const place = [s.roost ?? s.project, s.branch].filter(Boolean).join(" · ");
  const main = el(
    "button",
    { class: "nest-session-main", onclick: () => on.focus(s) },
    el("span", { class: "nest-session-name", text: s.project || agentName(s) }),
    el("span", { class: "nest-session-place", text: place }),
    el("span", { class: "nest-session-status", text: `${agentName(s)} · ${statusText(s)}` }),
  );
  main.title = t("Put this session in front");
  main.setAttribute("aria-pressed", String(front));
  return el(
    "article",
    { class: `nest-session ${statusClass(s)}${front ? " front" : ""}` },
    main,
    el(
      "div",
      { class: "nest-session-actions" },
      waits(s) ? el("button", { class: "btn small primary", text: t("Review on the island"), onclick: () => on.review(s) }) : null,
      s.raise === true ? el("button", { class: "btn small", text: t("Open terminal"), onclick: () => on.jump(s) }) : null,
    ),
  );
}

/** The session in front, in detail. */
function details(s: SessionView, on: FlockActions & FlockAsk, zeca: boolean): HTMLElement {
  const changed = (s.diffs ?? []).reduce((n, d) => (d ? { added: n.added + d.added, removed: n.removed + d.removed } : n), { added: 0, removed: 0 });
  const fact = (label: string, value: string | null | undefined) =>
    value ? el("div", { class: "nest-fact" }, el("span", { class: "nest-fact-label", text: label }), el("span", { text: value })) : null;
  const steps = s.steps.length
    ? el("ol", { class: "nest-steps" }, ...s.steps.slice(-6).map((step) => el("li", { text: step })))
    : el("p", { class: "board-empty", text: t("No steps yet.") });
  return el(
    "section",
    { class: `nest-details ${statusClass(s)}` },
    el(
      "header",
      { class: "nest-details-head" },
      el("span", { class: "nest-session-name", text: s.project || agentName(s) }),
      el("span", { class: "nest-session-status", text: `${agentName(s)} · ${statusText(s)}` }),
    ),
    waits(s)
      ? el(
          "div",
          { class: "nest-needs" },
          el("span", { text: t("Needs you: the card waits on the island.") }),
          el("button", { class: "btn small primary", text: t("Review on the island"), onclick: () => on.review(s) }),
        )
      : null,
    changed.added || changed.removed ? el("h4", { text: t("Changes +{added} −{removed}", changed) }) : null,
    el("h4", { text: t("Activity") }),
    steps,
    el("h4", { text: t("Context") }),
    fact(t("Folder"), s.cwd),
    fact(t("Repository"), s.roost),
    fact(t("Branch"), s.branch),
    fact(t("Editor"), s.editor),
    fact(t("Cost so far"), s.cost_usd ? `≈$${s.cost_usd.toFixed(2)}` : null),
    el(
      "div",
      { class: "nest-details-actions" },
      s.raise === true ? el("button", { class: "btn small", text: t("Open terminal"), onclick: () => on.jump(s) }) : null,
      zeca ? el("button", { class: "btn small", text: t("Ask Zeca about this session"), onclick: () => on.ask(s) }) : null,
    ),
  );
}

export function createFlock(on: FlockActions & FlockAsk): {
  element: HTMLElement;
  update: (v: ViewModel) => void;
  setZeca: (shown: boolean) => void;
  setActive: (shown: boolean) => void;
} {
  const scene = new Scene(NEST_HABITAT_SCENE);
  const badges = el("div", { class: "badges" });
  const habitat = el("div", { class: "nest-habitat" }, scene.canvas, badges);
  const search = el("input", { class: "nest-search" });
  search.type = "search";
  const needs = el("button", { class: "btn small nest-needs-filter" });
  const list = el("div", { class: "nest-flock" });
  const side = el("div", { class: "nest-side" });
  const tools = el("div", { class: "nest-flock-tools" }, needs, search);
  const element = el("div", { class: "nest-flock-tab" }, habitat, tools, el("div", { class: "nest-flock-body" }, list, side));
  let view: ViewModel | null = null;
  let onlyNeeds = false;
  let zeca = true;

  const draw = () => {
    if (!view) return;
    const v = view;
    assignSpecies(v.sessions);
    scene.update(v.sessions, null);
    const front = v.sessions.find((s) => v.focus && s.agent === v.focus.agent && s.id === v.focus.id) ?? v.sessions.find((s) => v.front && s.agent === v.front.agent && s.id === v.front.id) ?? v.sessions[0] ?? null;
    // Over each vult a button: its name and state on hover and for a screen reader, a click
    // puts it in front; and its badge, when its state asks for one.
    const byKey = new Map(v.sessions.map((s) => [key(s), s]));
    badges.replaceChildren(
      ...[...scene.slots()].sort((a, b) => a.x - b.x).flatMap((slot) => {
        const s = byKey.get(slot.key);
        if (!s) return [];
        const label = [s.project || agentName(s), agentName(s), statusText(s)].join(" · ");
        const b = el("button", { class: `habitat-bird${front && key(front) === slot.key ? " front" : ""}`, onclick: () => on.focus(s) });
        b.style.left = `${slot.x - 4}px`;
        b.style.width = `${slot.width + 8}px`;
        b.title = label;
        b.setAttribute("aria-label", label);
        const kind = badgeOf(s);
        const badge = kind ? el("span", { class: `badge ${kind}` }) : null;
        if (badge) badge.style.left = `${slot.x + slot.width - 6}px`;
        return badge ? [b, badge] : [b];
      }),
    );
    const q = search.value.trim().toLowerCase();
    const shown = v.sessions.filter((s) => (!onlyNeeds || s.attention === "needs-you") && (!q || said(s).includes(q)));
    list.replaceChildren(
      ...(shown.length
        ? shown.map((s) => row(s, on, !!front && key(front) === key(s)))
        : [
            el("p", {
              class: "board-empty",
              text: v.sessions.length ? t("No session matches. The flock stays in sight above.") : t("No sessions right now. Start an agent and its bird lands here."),
            }),
          ]),
    );
    side.replaceChildren(...(front ? [details(front, on, zeca)] : []));
  };
  const relabel = () => {
    search.placeholder = t("Find a session…");
    search.setAttribute("aria-label", t("Find a session…"));
    needs.textContent = t("Needs you");
    needs.setAttribute("aria-pressed", String(onlyNeeds));
    needs.classList.toggle("on", onlyNeeds);
  };
  search.addEventListener("input", draw);
  needs.addEventListener("click", () => {
    onlyNeeds = !onlyNeeds;
    relabel();
    draw();
  });
  // Up and down walk the flock, as on the island; the row's button keeps the focus ring.
  element.addEventListener("keydown", (e) => {
    if (!view || (e.target as Element).closest("input, textarea")) return;
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    const sessions = view.sessions;
    const at = sessions.findIndex((s) => view?.focus && s.agent === view.focus.agent && s.id === view.focus.id);
    const next = sessions[(Math.max(0, at) + (e.key === "ArrowDown" ? 1 : sessions.length - 1)) % Math.max(1, sessions.length)];
    if (next) {
      e.preventDefault();
      on.focus(next);
    }
  });
  relabel();
  return {
    element,
    update: (v) => {
      view = v;
      relabel();
      draw();
    },
    setZeca: (shown) => {
      zeca = shown;
      draw();
    },
    setActive: (shown) => scene.setActive(shown),
  };
}
