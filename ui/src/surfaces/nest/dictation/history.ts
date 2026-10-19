import { appliedStyle, appliedLanguage } from "../../../dictation-styles";
import type { DictationHistory, DictationRow } from "../../../bridge";
import { copyTextButton } from "../../../clipboard";
import { el } from "../../../dom";
import { locale, t } from "../../../i18n";
import { viewState } from "../view-state";

export interface DictationBackend {
  load(): Promise<DictationHistory>;
  remove(id: number): Promise<void>;
  clear(): Promise<void>;
  again(id: number): Promise<void>;
}

export function createDictationHistory(backend: DictationBackend, settings: () => void, commandsOnly = false) {
  const element = el("section", { class: "dictation-history" });
  const title = el("h2");
  const about = el("p");
  const configure = el("button", { onclick: settings });
  const figures = el("div", { class: "dictation-figures" });
  const summary = el("summary");
  const metrics = el("details", { class: "dictation-summary" }, summary, figures);
  const wide = window.matchMedia("(min-width: 900px) and (min-height: 561px)");
  let compactMetrics = false;
  metrics.addEventListener("toggle", () => { if (!wide.matches) compactMetrics = metrics.open; });
  wide.addEventListener("change", () => { metrics.open = wide.matches || compactMetrics; });
  const search = el("input", { class: "dictation-search" });
  search.type = "search";
  const refresh = el("button", { onclick: () => { if (!loading && !busy) void read(); } });
  const erase = el("button", { onclick: () => { confirming = "all"; draw(); focusConfirmation(); } });
  const feedback = el("div", { class: "dictation-feedback" });
  const confirmation = el("div", { class: "dictation-confirmation" });
  const list = el("div", { class: "dictation-list" });
  list.tabIndex = -1;
  const limit = el("p", { class: "dictation-note" });
  element.append(el("header", { class: "dictation-heading" }, el("div", {}, title, about), configure),
    el("div", { class: "dictation-toolbar" }, search, refresh, erase), metrics, feedback, confirmation, list, limit);
  let data: DictationHistory | null = null;
  let asked = 0, loading = false, busy = false;
  let failed = false, actionError = false, requested = false;
  let confirming: number | "all" | null = null;
  const expanded = new Set<number>();
  const number = (n: number) => n.toLocaleString(locale(), { maximumFractionDigits: 1 });
  const day = (at: number) => new Date(at * 1000).toLocaleDateString(locale(), { year: "numeric", month: "long", day: "numeric" });
  const retry = el("button", { onclick: () => { if (!loading && !busy) void read(); } });
  const focusConfirmation = () => confirmation.querySelector<HTMLButtonElement>("button")?.focus();
  const paint = () => {
    title.textContent = commandsOnly ? t("Voice commands") : t("Dictation history");
    about.textContent = commandsOnly ? t("Hold the command key to control Vults. Recent commands and their results stay on this computer under Dictation history retention.") : t("Hold the dictation key in another app, speak and release. Only text is kept on this computer.");
    configure.textContent = commandsOnly ? t("Command settings") : t("Dictation settings");
    summary.textContent = t("Today’s dictation");
    metrics.hidden = commandsOnly;
    metrics.open = wide.matches || compactMetrics;
    search.placeholder = commandsOnly ? t("Search voice commands") : t("Search what you dictated");
    search.setAttribute("aria-label", search.placeholder);
    refresh.textContent = loading ? t("Loading") : t("Refresh");
    refresh.disabled = loading || busy;
    erase.hidden = commandsOnly;
    figures.hidden = commandsOnly;
    erase.textContent = t("Erase dictation history…");
    erase.disabled = !data?.rows.length || loading || busy;
    retry.textContent = t("Try again"); retry.disabled = loading || busy;
    list.setAttribute("aria-busy", String(loading || busy));
    list.setAttribute("aria-label", commandsOnly ? t("Recent commands") : t("Dictation history"));
    const stats = data?.today;
    const known = stats && stats.count > 0;
    const tile = (value: string, label: string) => el("div", {}, el("strong", { text: value }), el("span", { text: label }));
    figures.replaceChildren(
      tile(known ? number(stats.words) : t("Unknown"), t("Words today")),
      tile(known ? t("{n} min", { n: number(Math.max(0, stats.words / 45 - stats.secs / 60)) }) : t("Unknown"), t("Estimated time saved today")),
      tile(known && stats.secs >= 1 ? number(stats.words * 60 / stats.secs) : t("Unknown"), t("Words per minute")),
    );
    feedback.replaceChildren(...(failed ? [viewState("error", t("Could not load dictation history."),
      data ? t("The last loaded view is kept and may be out of date.") : t("Try again to read the data saved on this computer."), retry)] : []),
      ...(actionError ? [viewState("error", t("Could not complete this history action. Try again."))] : []),
      ...(requested ? [viewState("empty", t("Copy requested. Follow the island's delivery status."), t("Return to your destination app to paste. Nothing is submitted."))] : []));
    confirmation.replaceChildren();
    if (confirming !== null) {
      const accept = el("button", { class: "dictation-danger", text: t("Erase"), onclick: () => {
        const id = confirming;
        if (id === null || busy) return;
        void act(async () => {
          if (id === "all") await backend.clear(); else await backend.remove(id);
          asked++;
          if (data) data = { ...data, rows: id === "all" ? [] : data.rows.filter(row => row.id !== id) };
          confirming = null;
        });
      } });
      accept.disabled = busy;
      const cancel = el("button", { text: t("Cancel"), onclick: () => { confirming = null; draw(); erase.focus(); } });
      cancel.disabled = busy;
      confirmation.append(el("p", { text: confirming === "all" ? t("Erase every retained dictation? Usage totals stay.") : t("Erase this dictation? Usage totals stay.") }), accept, cancel);
    }
    list.replaceChildren();
    limit.textContent = commandsOnly ? t("Search covers commands among the newest 500 retained dictations.") : t("Search covers the newest 500 retained dictations. Time saved estimates typing at 45 words per minute.");
    if (!data) {
      if (!failed) list.append(viewState("loading", t("Reading dictation history…")));
      return;
    }
    if (data.retention === "0") {
      list.append(viewState("empty", t("Dictation history is not kept."), commandsOnly ? t("Commands still work; no transcript or result is saved.") : t("Usage totals still update. Copy again keeps the last successful text in memory only.")));
      return;
    }
    const query = search.value.trim().toLocaleLowerCase(locale());
    const rows = data.rows.filter(row => (!commandsOnly || row.outcome === "command") && (!query || `${row.heard}\n${row.text}\n${row.app ?? ""}\n${row.session ?? ""}`.toLocaleLowerCase(locale()).includes(query)));
    if (!rows.length) {
      list.append(viewState("empty", query ? (commandsOnly ? t("No commands match your search.") : t("No dictations match your search.")) : (commandsOnly ? t("No voice commands yet.") : t("Nothing dictated yet.")), query ? t("Try another search or clear it.") : commandsOnly ? t("Hold the command key, say a supported phrase and release.") : t("Use the dictation key outside Vults to add your first entry.")));
      return;
    }
    let group: string | undefined;
    for (const row of rows) {
      const groupDay = day(row.at);
      if (group !== groupDay) { group = groupDay; list.append(el("h3", { class: "dictation-day", text: groupDay })); }
      list.append(renderRow(row));
    }
  };
  const draw = () => {
    const scroll = element.scrollTop;
    const active = document.activeElement as HTMLElement | null;
    const key = active && element.contains(active) ? active.dataset.historyAction : null;
    paint();
    element.scrollTop = scroll;
    if (key) (element.querySelector<HTMLElement>(`[data-history-action="${key}"]`) ?? list).focus({ preventScroll: true });
    else if (active === retry) (failed ? retry : list).focus({ preventScroll: true });
  };
  const renderRow = (row: DictationRow) => {
    const transcript = el("p", { class: "dictation-text", text: row.text });
    transcript.setAttribute("dir", "auto");
    const heard = el("details", { class: "dictation-heard" }, el("summary", { text: t("Show what was heard") }), el("p", { text: row.heard }));
    heard.querySelector("summary")!.dataset.historyAction = `heard-${row.id}`;
    heard.open = expanded.has(row.id);
    heard.addEventListener("toggle", () => heard.open ? expanded.add(row.id) : expanded.delete(row.id));
    const time = el("time", { text: new Date(row.at * 1000).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" }) });
    time.dateTime = new Date(row.at * 1000).toISOString();
    const command = row.outcome === "command";
    const outcome = command ? t("Voice command") : row.outcome === "pasted" ? t("Pasted") : row.outcome === "session" ? t("In session") : row.outcome === "held" ? t("Copied; session is waiting") : t("Copied");
    const style = command ? null : appliedStyle(row.style);
    const language = command ? null : appliedLanguage(row.outputLanguage);
    const metadata = el("div", { class: "dictation-meta" }, time,
      el("span", { text: command ? outcome : t("{outcome} · {app}", { outcome, app: row.app || t("Unknown app") }) }),
      style ? el("span", { text: style }) : null,
      language ? el("span", { text: language }) : null,
      row.session ? el("bdi", { text: row.session }) : null,
      el("span", { text: row.words === 1 ? t("1 word") : t("{count} words", { count: number(row.words) }) }),
      el("span", { text: t("{s} s", { s: number(row.secs) }) }),
      row.fixes === null ? null : el("span", { text: row.fixes === 1 ? t("1 correction") : t("{count} corrections", { count: number(row.fixes) }) }));
    const again = el("button", { text: t("Use again"), onclick: () => void act(async () => { await backend.again(row.id); requested = true; }, false) });
    again.setAttribute("aria-label", t("Use dictation from {time} again", { time: day(row.at) + " " + time.textContent }));
    const copy = copyTextButton(row.text, "", t("Copy dictation from {time}", { time: time.textContent ?? "" }));
    const remove = el("button", { text: t("Delete…"), onclick: () => { confirming = row.id; draw(); focusConfirmation(); } });
    remove.setAttribute("aria-label", t("Delete dictation from {time}", { time: day(row.at) + " " + time.textContent }));
    for (const [index, button] of [again, copy, remove].entries()) {
      button.disabled = busy;
      button.dataset.historyAction = `${index}-${row.id}`;
    }
    return el("article", { class: "dictation-entry" }, transcript, heard, metadata, el("div", { class: "dictation-actions" }, command ? null : again, copy, remove));
  };
  const read = async () => {
    const mine = ++asked;
    loading = true; draw();
    try { const next = await backend.load(); if (mine === asked) { data = next; failed = false; } }
    catch { if (mine === asked) failed = true; }
    finally { if (mine === asked) { loading = false; draw(); } }
  };
  const act = async (run: () => Promise<void>, reload = true) => {
    if (busy) return;
    busy = true; actionError = false; requested = false; asked++; loading = false; draw();
    try { await run(); if (reload) await read(); }
    catch { actionError = true; }
    finally { busy = false; draw(); if (!element.closest("[hidden]") && element.isConnected) list.focus({ preventScroll: true }); }
  };
  search.addEventListener("input", draw);
  confirmation.addEventListener("keydown", e => { if (e.key === "Escape" && !busy) { e.preventDefault(); confirming = null; draw(); erase.focus(); } });
  draw();
  return {
    element, refresh: () => void read(), relabel: draw,
    invalidate: () => { asked++; loading = false; data = null; confirming = null; requested = false; expanded.clear(); draw(); },
  };
}
