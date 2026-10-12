// The Nest's Flock tab (ADR 0021): every session, with its roost and branch. Until U7 draws the
// habitat it is a list. Nothing here answers a card: a session that waits leads to the island (D1).
import type { SessionView, ViewModel } from "../../view.gen";
import { el } from "../../dom";
import { t } from "../../i18n";
import { agentName, statusClass, statusText } from "../../island/views";

export interface FlockActions {
  /** Puts the session in front of the flock (core's focus). */
  focus: (s: SessionView) => void;
  /** Its terminal, where `raise` says it can be brought forward. */
  jump: (s: SessionView) => void;
  /** The island, on the card this session waits on. */
  review: (s: SessionView) => void;
}

function row(s: SessionView, on: FlockActions): HTMLElement {
  const place = [s.roost ?? s.project, s.branch].filter(Boolean).join(" · ");
  const waits = s.status === "approval" || s.status === "question" || s.waiting === true;
  const main = el(
    "button",
    { class: "nest-session-main", onclick: () => on.focus(s) },
    el("span", { class: "nest-session-name", text: s.project }),
    el("span", { class: "nest-session-place", text: place }),
    el("span", { class: "nest-session-status", text: `${agentName(s)} · ${statusText(s)}` }),
  );
  main.title = t("Put this session in front");
  return el(
    "article",
    { class: `nest-session ${statusClass(s)}` },
    main,
    el(
      "div",
      { class: "nest-session-actions" },
      waits ? el("button", { class: "btn small primary", text: t("Review on the island"), onclick: () => on.review(s) }) : null,
      s.raise === true ? el("button", { class: "btn small", text: t("Open terminal"), onclick: () => on.jump(s) }) : null,
    ),
  );
}

export function createFlock(on: FlockActions): { element: HTMLElement; update: (v: ViewModel) => void } {
  const element = el("div", { class: "nest-flock" });
  const update = (v: ViewModel) => {
    element.replaceChildren(
      ...(v.sessions.length
        ? v.sessions.map((s) => row(s, on))
        : [el("p", { class: "board-empty", text: t("No sessions right now. Start an agent and its bird lands here.") })]),
    );
  };
  return { element, update };
}
