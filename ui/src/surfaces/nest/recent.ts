// Saved conversations share the chat's existing selection and draft owners.
import type { ConversationView } from "../../bridge";
import { el } from "../../dom";
import { locale, t } from "../../i18n";
import { viewState } from "./view-state";
import { day } from "../../day";

export interface RecentActions {
  list: () => Promise<ConversationView[]>;
  open: (id: number) => void;
  fresh: () => void;
  /** A turn runs: opening or starting one waits for it to end. */
  busy: () => boolean;
  close?: () => void;
}

/** Keep the initial list short; filtering searches every loaded conversation. */
const SHOWN = 30;
const QUERY = "vults.recent-query";

export function createRecent(on: RecentActions): { element: HTMLElement; refresh: () => void; clear: () => void } {
  const back = on.close ? el("button", { class: "nest-recent-back", onclick: on.close }) : null;
  const fresh = el("button", { class: "outline nest-new-chat", onclick: () => { if (!on.busy()) on.fresh(); } });
  const busyNotice = el("p", { class: "nest-recent-busy" });
  busyNotice.setAttribute("role", "status");
  busyNotice.hidden = true;
  const search = el("input", { class: "nest-recent-search" });
  search.type = "search";
  try { search.value = localStorage.getItem(QUERY) ?? ""; } catch { /* Search still works without local storage. */ }
  const rememberQuery = () => { try { localStorage.setItem(QUERY, search.value); } catch { /* Keep the current view usable. */ } };
  const head = el("div", { class: "nest-recent-head" });
  const status = el("div", { class: "nest-recent-status" });
  status.setAttribute("role", "status");
  const list = el("div", { class: "nest-recent-list" });
  const element = el("aside", { class: "nest-recent" }, el("div", { class: "nest-recent-actions" }, back, fresh), busyNotice, search, head, status, list);
  let asked = 0;
  let rows: ConversationView[] | null = null;
  let failed = false;
  let expanded = false;
  let loading = false;
  let statusKey = "";
  let returning: string | null = null;

  const paint = () => {
    const active = document.activeElement as HTMLElement | null;
    if (active && (list.contains(active) || status.contains(active))) returning = active.dataset.recentFocus ?? null;
    else if (active && active !== document.body) returning = null;
    const scroll = list.scrollTop;
    list.setAttribute("aria-busy", String(loading));
    const busy = on.busy();
    const busyMessage = busy ? t("Wait for the current action to finish before opening or starting another conversation.") : "";
    // Keep one live node and avoid announcing the same explanation on every refresh.
    if (busyNotice.textContent !== busyMessage) busyNotice.textContent = busyMessage;
    busyNotice.hidden = !busy;
    const query = search.value.trim().toLocaleLowerCase(locale());
    const matches = (rows ?? []).filter(c => !query || [c.title, c.folder, c.label].filter(Boolean).join(" ").toLocaleLowerCase(locale()).includes(query))
      .sort((a, b) => b.updatedMs - a.updatedMs || b.id - a.id);
    const shown = expanded ? matches : matches.slice(0, SHOWN);
    fresh.textContent = t("+ New conversation");
    fresh.disabled = busy;
    fresh.title = busyMessage;
    search.placeholder = t("Search conversations");
    search.setAttribute("aria-label", t("Search conversations"));
    element.setAttribute("aria-label", t("Recent"));
    if (back) back.textContent = t("Back to Chat");
    head.textContent = t("Recent");
    const nextStatusKey = JSON.stringify([failed, loading, rows === null, locale()]);
    if (nextStatusKey !== statusKey) {
      statusKey = nextStatusKey;
      status.replaceChildren();
      status.hidden = !failed && (rows !== null || !loading);
      if (failed) {
        const retry = el("button", { class: "outline", text: loading ? t("Loading") : t("Try again"), onclick: () => { if (!loading) refresh(); } });
        retry.disabled = loading;
        retry.dataset.recentFocus = "retry";
        status.append(viewState("error", rows ? t("Conversations could not update. The last loaded list may be out of date.") : t("Conversations could not load. Try again."), undefined, retry));
      } else if (loading && rows === null) status.append(viewState("loading", t("Loading")));
    }
    const parts: HTMLElement[] = [];
    let previousDay = "";
    for (const c of shown) {
      const group = day(c.updatedMs);
      if (group !== previousDay) parts.push(el("h3", { class: "nest-recent-day", text: group }));
      previousDay = group;
      const project = c.folder?.split(/[\\/]/).filter(Boolean).at(-1);
      const button = el("button", { class: `nest-recent-row${c.current ? " current" : ""}`, onclick: () => { if (!on.busy() || c.current) on.open(c.id); } },
        el("span", { class: "nest-recent-title", text: c.title || t("Untitled") }),
        el("span", { class: "nest-recent-meta", text: [project, c.label].filter(Boolean).join(" · ") }),
        el("time", { class: "nest-recent-time", text: new Date(c.updatedMs).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" }) }),
      );
      button.dataset.conversation = String(c.id);
      button.dataset.recentFocus = `conversation:${c.id}`;
      if (c.current) button.setAttribute("aria-current", "true");
      button.disabled = busy && !c.current;
      button.title = [c.title || t("Untitled"), c.folder, c.label].filter(Boolean).join("\n");
      parts.push(button);
    }
    if (rows !== null && !matches.length) {
      parts.push(viewState("empty", query ? t("No conversations match your search.") : t("No conversations yet.")));
      if (query) parts.push(el("button", { class: "outline", text: t("Clear filters"), onclick: () => { search.value = ""; rememberQuery(); expanded = false; paint(); search.focus(); } }));
    }
    if (!expanded && matches.length > SHOWN) parts.push(el("button", { class: "outline", text: t("Show more"), onclick: () => { expanded = true; paint(); list.querySelectorAll<HTMLElement>(".nest-recent-row")[SHOWN]?.focus(); } }));
    list.replaceChildren(...parts);
    list.scrollTop = scroll;
    if (returning) {
      const next = element.querySelector<HTMLButtonElement>(`[data-recent-focus="${CSS.escape(returning)}"]`);
      if (next && !next.disabled) {
        next.focus({ preventScroll: true });
        returning = null;
      } else if (!loading) {
        search.focus({ preventScroll: true });
        returning = null;
      }
    }
  };
  search.addEventListener("input", () => { rememberQuery(); expanded = false; paint(); list.scrollTop = 0; });
  search.addEventListener("keydown", e => {
    if (e.key !== "Escape" || e.isComposing || e.keyCode === 229 || !search.value) return;
    e.preventDefault();
    e.stopPropagation();
    search.value = "";
    rememberQuery();
    expanded = false;
    paint();
  });
  const refresh = () => {
    const mine = ++asked;
    loading = true;
    paint();
    void on.list().then(
      next => { if (mine === asked) { rows = next; failed = false; } },
      () => { if (mine === asked) failed = true; },
    ).finally(() => { if (mine === asked) { loading = false; paint(); } });
  };
  return { element, refresh, clear: () => { asked++; rows = null; failed = false; loading = false; expanded = false; paint(); } };
}
