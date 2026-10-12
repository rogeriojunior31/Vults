// The Nest's Chat, its *Recent* column (ADR 0021): the saved conversations, the one going on marked,
// and a way to start another. A click goes on with one, as the chat's own list does.
import type { ConversationView } from "../../bridge";
import { el } from "../../dom";
import { locale, t } from "../../i18n";

export interface RecentActions {
  list: () => Promise<ConversationView[]>;
  open: (id: number) => void;
  fresh: () => void;
}

/** Conversations shown at once; the chat's own list keeps every one. */
const SHOWN = 30;

function when(ms: number): string {
  const d = new Date(ms);
  const today = new Date();
  return d.toDateString() === today.toDateString()
    ? d.toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" })
    : d.toLocaleDateString(locale(), { month: "short", day: "numeric" });
}

export function createRecent(on: RecentActions): { element: HTMLElement; refresh: () => void } {
  const fresh = el("button", { class: "btn small nest-new-chat", onclick: on.fresh });
  const head = el("div", { class: "nest-recent-head" });
  const list = el("div", { class: "nest-recent-list" });
  const element = el("aside", { class: "nest-recent" }, fresh, head, list);
  let asked = 0;
  const refresh = () => {
    fresh.textContent = t("New conversation");
    head.textContent = t("Recent");
    const mine = ++asked;
    void on.list().then(
      (rows) => {
        if (mine !== asked) return;
        list.replaceChildren(
          ...(rows.length
            ? rows.slice(0, SHOWN).map((c) => {
                const b = el(
                  "button",
                  { class: `nest-recent-row${c.current ? " current" : ""}`, onclick: () => on.open(c.id) },
                  el("span", { class: "nest-recent-title", text: c.title || t("Untitled") }),
                  el("span", { class: "nest-recent-meta", text: [when(c.updatedMs), c.label].filter(Boolean).join(" · ") }),
                );
                b.setAttribute("aria-current", String(c.current));
                if (c.folder) b.title = c.folder;
                return b;
              })
            : [el("p", { class: "board-empty", text: t("No conversations yet.") })]),
        );
      },
      () => {},
    );
  };
  return { element, refresh };
}
