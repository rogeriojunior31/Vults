// Saved conversations share the chat's existing selection and draft owners.
import type { ConversationView } from "../../bridge";
import { el } from "../../dom";
import { locale, t } from "../../i18n";
import { viewState } from "./view-state";

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

function day(ms: number): string {
  const date = new Date(ms);
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return t("Today");
  if (date.toDateString() === yesterday.toDateString()) return t("Yesterday");
  return date.toLocaleDateString(locale(), date.getFullYear() === today.getFullYear()
    ? { month: "long", day: "numeric" } : { year: "numeric", month: "long", day: "numeric" });
}

export function createRecent(on: RecentActions): { element: HTMLElement; refresh: () => void } {
  const back = on.close ? el("button", { class: "nest-recent-back", onclick: on.close }) : null;
  const fresh = el("button", { class: "outline nest-new-chat", onclick: () => { if (!on.busy()) on.fresh(); } });
  const search = el("input", { class: "nest-recent-search" });
  search.type = "search";
  try { search.value = localStorage.getItem(QUERY) ?? ""; } catch { /* Search still works without local storage. */ }
  const rememberQuery = () => { try { localStorage.setItem(QUERY, search.value); } catch { /* Keep the current view usable. */ } };
  const head = el("div", { class: "nest-recent-head" });
  const status = el("div", { class: "nest-recent-status" });
  status.setAttribute("role", "status");
  const list = el("div", { class: "nest-recent-list" });
  const element = el("aside", { class: "nest-recent" }, el("div", { class: "nest-recent-actions" }, back, fresh), search, head, status, list);
  let asked = 0;
  let rows: ConversationView[] | null = null;
  let failed = false;
  let expanded = false;
  let loading = false;

  const paint = () => {
    const active = document.activeElement as HTMLElement | null;
    const focusedId = active && list.contains(active) ? active.dataset.conversation : null;
    const scroll = list.scrollTop;
    list.setAttribute("aria-busy", String(loading));
    const busy = on.busy();
    const query = search.value.trim().toLocaleLowerCase(locale());
    const matches = (rows ?? []).filter(c => !query || [c.title, c.folder, c.label].filter(Boolean).join(" ").toLocaleLowerCase(locale()).includes(query))
      .sort((a, b) => b.updatedMs - a.updatedMs || b.id - a.id);
    const shown = expanded ? matches : matches.slice(0, SHOWN);
    fresh.textContent = t("+ New conversation");
    fresh.disabled = busy;
    fresh.title = busy ? t("Wait for Zeca's reply to end") : "";
    search.placeholder = t("Search conversations");
    search.setAttribute("aria-label", t("Search conversations"));
    element.setAttribute("aria-label", t("Recent"));
    if (back) back.textContent = t("Back to Chat");
    head.textContent = t("Recent");
    status.replaceChildren();
    status.hidden = !failed && (rows !== null || !loading);
    if (failed) {
      const retry = el("button", { class: "outline", text: loading ? t("Loading") : t("Try again"), onclick: () => { if (!loading) refresh(); } });
      retry.disabled = loading;
      status.append(viewState("error", rows ? t("Conversations could not update. The last loaded list may be out of date.") : t("Conversations could not load. Try again."), undefined, retry));
    } else if (loading && rows === null) status.append(viewState("loading", t("Loading")));
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
    if (focusedId) list.querySelector<HTMLElement>(`[data-conversation="${focusedId}"]`)?.focus({ preventScroll: true });
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
  return { element, refresh };
}
