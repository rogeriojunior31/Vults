import { el } from "../dom";
import { currentLang, t } from "../i18n";
import type { AttentionItem, AttentionSource, SessionView, ViewModel } from "../view.gen";
import { CHECKS, tag } from "./board";
import { icon, type IconName } from "./icons";
import { isSeen, markSeen, seenSig } from "./seen";
import { agentName, statusText } from "./views";

export interface NowActions {
  request(request: string, session: SessionView): void;
  session(session: SessionView): void;
  connection(connector: string, item: string): void;
  alert(key: string): void;
  chat(): void;
  memory(): void;
  room(tab: "activity" | "connections"): void;
  relayout(): void;
}

type Row = { key: string; title: string; detail: string; glyph: IconName; level: AttentionItem["level"]; open(): void; seen?: () => void };
const sourceKey = (s: AttentionSource): string => s.kind === "request" ? `request:${s.request}`
  : s.kind === "session" ? `session:${s.session.agent}:${s.session.id}`
  : s.kind === "connection" ? `connection:${s.connector}:${s.item}`
  : s.kind === "alert" ? `alert:${s.key}` : s.kind;

export function nowItems(v: ViewModel, zeca: boolean): AttentionItem[] {
  return (v.inbox ?? []).filter(({ source: s, level }) => {
    if (s.kind === "chat" || s.kind === "memory") return zeca;
    if (s.kind !== "connection" || level === "failed") return true;
    const row = v.boards?.find(b => b.connector === s.connector)?.rows.find(r => r.item === s.item);
    return !!row && !isSeen(s.connector, row);
  });
}

/** This only presents core/app's inbox. Refreshing, seeing and opening never decide a card. */
export function createNow(on: NowActions): { element: HTMLElement; update(v: ViewModel, zeca: boolean): void } {
  const heading = el("h2");
  const allSeen = el("button", { class: "now-seen" });
  const feed = el("div", { class: "now-feed" });
  feed.setAttribute("role", "list");
  const activity = el("button", { class: "now-link", onclick: () => on.room("activity") });
  const connections = el("button", { class: "now-link", onclick: () => on.room("connections") });
  const element = el("section", { class: "now-inbox" }, el("header", { class: "now-head" }, heading, allSeen), feed,
    el("footer", { class: "now-links" }, activity, connections));
  let rows: Row[] = [];
  let signature = "";
  let view: ViewModel;
  let enabled = true;
  const draw = () => {
    const sig = JSON.stringify([view.inbox, view.sessions, view.boards, view.alerts, enabled, currentLang(), seenSig()]);
    if (sig === signature) return;
    signature = sig;
    const had = element.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.now : null;
    const top = feed.scrollTop;
    rows = [];
    for (const item of nowItems(view, enabled)) {
      const s = item.source;
      const key = sourceKey(s);
      const add = (title: string, detail: string, glyph: IconName, open: () => void, seen?: () => void) => rows.push({ key, title, detail, glyph, open, seen, level: item.level });
      switch (s.kind) {
        case "request": {
          const session = view.sessions.find(x => x.agent === s.session.agent && x.id === s.session.id);
          if (!session) break;
          add(s.question ? t("{project} asks a question", { project: session.project || agentName(session) })
            : t("{project} needs your permission", { project: session.project || agentName(session) }),
            [agentName(session), s.description || s.target, s.skipped ? t("Skipped for now") : null].filter(Boolean).join(" · "),
            s.question ? "chat" : "terminal", () => on.request(s.request, session));
          break;
        }
        case "session": {
          const session = view.sessions.find(x => x.agent === s.session.agent && x.id === s.session.id);
          if (!session) break;
          add(session.project || agentName(session), [statusText(session), session.note || session.step].filter(Boolean).join(" · "),
            item.level === "failed" ? "close" : "moon", () => on.session(session));
          break;
        }
        case "connection": {
          const board = view.boards?.find(b => b.connector === s.connector);
          const row = board?.rows.find(r => r.item === s.item);
          if (!board || !row || (item.level !== "failed" && isSeen(s.connector, row))) break;
          const status = row.checks === "failing" ? t(CHECKS.failing) : row.merge ? t("Ready to merge") : tag(row)?.text
            ?? (row.group === "to-review" ? t("Review requested") : t("Ready to merge"));
          add(row.title, `${row.name} · ${status}`, row.group === "branches" ? "branch" : "pull", () => on.connection(s.connector, s.item),
            item.level !== "failed" ? () => markSeen(board, [row]) : undefined);
          break;
        }
        case "alert": {
          const alert = view.alerts.find(a => a.key === s.key);
          if (alert) add(alert.title, alert.detail, "close", () => on.alert(s.key));
          break;
        }
        case "chat":
          if (enabled) add(s.count === 1 ? t("Zeca's chat needs 1 answer") : t("Zeca's chat needs {n} answers", { n: s.count }),
            t("Open Chat to review each request."), "chat", on.chat);
          break;
        case "memory":
          if (enabled) add(s.count === null ? t("Could not read memory proposals.")
            : s.count === 1 ? t("A memory proposal is waiting") : t("{n} memory proposals are waiting", { n: s.count }),
            t("You decide what Zeca keeps."), "list", on.memory);
          break;
      }
    }
    heading.textContent = t("Now");
    feed.setAttribute("aria-label", t("Items needing attention"));
    allSeen.textContent = t("Mark all seen");
    allSeen.dataset.now = "seen-all";
    allSeen.disabled = !rows.some(r => r.seen);
    activity.textContent = t("Activity"); connections.textContent = t("Connections");
    feed.replaceChildren(...rows.map(r => {
      const open = el("button", { class: "now-open", onclick: r.open }, icon(r.glyph, 15),
        el("span", { class: "now-copy" }, el("strong", { text: r.title }), el("small", { text: r.detail })), icon("chevron", 13));
      open.dataset.now = r.key;
      const seen = r.seen ? el("button", { class: "now-seen", text: t("Seen"), onclick: () => { r.seen?.(); draw(); on.relayout(); } }) : null;
      if (seen) { seen.dataset.now = `seen:${r.key}`; seen.setAttribute("aria-label", t("Mark {title} seen", { title: r.title })); }
      const article = el("article", { class: `now-row ${r.level}` }, open, seen);
      article.setAttribute("role", "listitem");
      return article;
    }), ...(rows.length ? [] : [el("div", { class: "now-empty" }, icon("check", 20),
      el("strong", { text: t("Nothing needs your attention") }), el("p", { text: t("Open the Nest when you want the full picture.") }))]));
    feed.scrollTop = top;
    if (had) {
      const same = element.querySelector<HTMLButtonElement>(`[data-now="${CSS.escape(had)}"]`);
      (same && !same.disabled ? same : feed.querySelector<HTMLElement>("button") ?? activity)?.focus({ preventScroll: true });
    }
  };
  allSeen.addEventListener("click", () => { rows.forEach(r => r.seen?.()); draw(); on.relayout(); });
  return { element, update(v, zeca) { view = v; enabled = zeca; draw(); } };
}
