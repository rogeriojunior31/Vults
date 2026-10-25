import { appliedStyle, appliedLanguage } from "../../../dictation-styles";
import type { DictationHistory, DictationRow } from "../../../bridge";
import { copyTextButton } from "../../../clipboard";
import { el } from "../../../dom";
import { locale, t } from "../../../i18n";
import { icon } from "../../../island/icons";
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
  const shortcutKeys = el("div", { class: "dictation-keycaps" });
  const shortcutAbout = el("p");
  const wave = el("span", { class: "dictation-shortcut-wave" });
  wave.setAttribute("aria-hidden", "true");
  for (let i = 0; i < 30; i++) { const bar = el("i"); bar.style.height = `${18 + Math.round(Math.abs(Math.sin(i * 1.7) * Math.cos(i * .45)) * 70)}%`; wave.append(bar); }
  const hero = el("section", { class: "dictation-hero" }, el("div", { class: "dictation-shortcut" }, shortcutKeys, shortcutAbout, wave), metrics);
  let boundKeys: Record<string, string> = {};
  let metricsExpanded = false, metricsChanged = false;
  metrics.addEventListener("toggle", () => { metricsExpanded = metrics.open; metricsChanged = true; });
  const search = el("input", { class: "dictation-search" });
  search.type = "search";
  const refresh = el("button", { onclick: () => { if (!loading && !busy) void read(); } });
  const erase = el("button", { onclick: () => { returnTo = null; confirming = "all"; draw(); focusConfirmation(); } });
  const maintenanceTitle = el("summary");
  const maintenance = el("details", { class: "dictation-maintenance" }, maintenanceTitle, erase);
  const results = el("p", { class: "dictation-results" });
  results.setAttribute("role", "status");
  const feedback = el("div", { class: "dictation-feedback" });
  const confirmation = el("div", { class: "dictation-confirmation" });
  const list = el("div", { class: "dictation-list" });
  list.tabIndex = -1;
  const limit = el("p", { class: "dictation-note" });
  element.append(el("header", { class: "dictation-heading" }, el("div", {}, title, about), configure),
    hero, el("div", { class: "dictation-toolbar" }, search, refresh, maintenance), results, feedback, confirmation, list, limit);
  let data: DictationHistory | null = null;
  let asked = 0, loading = false, busy = false;
  let failed = false, actionError = false, requested = false;
  let confirming: number | "all" | null = null;
  let returnTo: string | null = null;
  const expanded = new Set<number>();
  const number = (n: number) => n.toLocaleString(locale(), { maximumFractionDigits: 1 });
  const day = (at: number) => new Date(at * 1000).toLocaleDateString(locale(), { year: "numeric", month: "long", day: "numeric" });
  const retry = el("button", { onclick: () => { if (!loading && !busy) void read(); } });
  const focusConfirmation = () => confirmation.querySelector<HTMLButtonElement>("button")?.focus();
  const cancelConfirmation = () => {
    confirming = null; draw();
    (returnTo ? element.querySelector<HTMLElement>(`[data-history-action="${returnTo}"]`) ?? list : erase).focus({ preventScroll: true });
  };
  const clearSearch = el("button", { onclick: () => { search.value = ""; draw(); search.focus(); } });
  const paint = () => {
    title.textContent = commandsOnly ? t("Voice commands") : t("Dictation history");
    about.textContent = commandsOnly ? t("Hold the command key to control Vults. Recent commands and their results stay on this computer under Dictation history retention.") : t("Your recent dictations, kept on this computer.");
    hero.hidden = commandsOnly;
    shortcutAbout.textContent = t("Hold the shortcut in your destination app, speak and release.");
    const binding = boundKeys["talk"]?.trim();
    shortcutKeys.replaceChildren(...(binding ? binding.split("+").map(key => el("kbd", { text: key })) : [el("span", { text: t("Not bound") })]));
    shortcutKeys.setAttribute("aria-label", binding || t("Not bound"));
    configure.textContent = commandsOnly ? t("Command settings") : t("Dictation settings");
    summary.textContent = t("Today’s dictation");
    maintenanceTitle.textContent = t("History options");
    maintenance.hidden = commandsOnly;
    clearSearch.textContent = t("Clear search");
    results.hidden = true;
    list.setAttribute("role", "region");
    confirmation.setAttribute("role", "region");
    confirmation.setAttribute("aria-label", t("Confirm history deletion"));
    metrics.hidden = commandsOnly;
    metrics.open = metricsExpanded;
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
          if (id === "all") expanded.clear(); else expanded.delete(id);
          confirming = null;
        });
      } });
      accept.disabled = busy; accept.dataset.historyAction = "erase-confirm";
      const cancel = el("button", { text: t("Cancel"), onclick: cancelConfirmation });
      cancel.disabled = busy; cancel.dataset.historyAction = "cancel-confirm";
      confirmation.append(el("p", { text: confirming === "all" ? t("Erase every retained dictation? Usage totals stay.") : t("Erase this dictation? Usage totals stay.") }), cancel, accept);
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
    results.hidden = false;
    const resultText = rows.length === 1 ? t("1 entry") : t("{count} entries", { count: number(rows.length) });
    if (results.textContent !== resultText) results.textContent = resultText;
    if (!rows.length) {
      list.append(viewState("empty", query ? (commandsOnly ? t("No commands match your search.") : t("No dictations match your search.")) : (commandsOnly ? t("No voice commands yet.") : t("Nothing dictated yet.")), query ? t("Try another search or clear it.") : commandsOnly ? t("Hold the command key, say a supported phrase and release.") : t("Use the dictation key outside Vults to add your first entry."), query ? clearSearch : null));
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
    const heard = el("details", { class: "dictation-heard" }, el("summary", { text: t("Details") }), el("p", { class: "dictation-detail-label", text: t("Original transcript") }), el("p", { text: row.heard }));
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
      row.session ? el("bdi", { text: row.session }) : null);
    heard.append(el("div", { class: "dictation-meta" },
      style ? el("span", { text: style }) : null,
      language ? el("span", { text: language }) : null,
      el("span", { text: row.words === 1 ? t("1 word") : t("{count} words", { count: number(row.words) }) }),
      el("span", { text: t("{s} s", { s: number(row.secs) }) }),
      row.fixes === null ? null : el("span", { text: row.fixes === 1 ? t("1 correction") : t("{count} corrections", { count: number(row.fixes) }) })));
    const again = el("button", { text: t("Use again"), onclick: () => void act(async () => { await backend.again(row.id); requested = true; }, false) });
    again.setAttribute("aria-label", t("Use dictation from {time} again", { time: day(row.at) + " " + time.textContent }));
    const copy = copyTextButton(row.text, "", t("Copy dictation from {time}", { time: day(row.at) + " " + time.textContent }));
    again.title = t("Use again"); again.replaceChildren(icon("retry", 15));
    const copyLabel = copy.querySelector("span");
    if (copyLabel) copyLabel.className = "dictation-action-label";
    copy.append(icon("copy", 15));
    const remove = el("button", { text: t("Delete…"), onclick: () => { returnTo = `2-${row.id}`; confirming = row.id; draw(); focusConfirmation(); } });
    remove.setAttribute("aria-label", t("Delete dictation from {time}", { time: day(row.at) + " " + time.textContent }));
    remove.title = t("Delete…"); remove.replaceChildren(icon("close", 15));
    for (const [index, button] of [again, copy, remove].entries()) {
      button.disabled = busy;
      button.dataset.historyAction = `${index}-${row.id}`;
    }
    const mark = el("span", { class: "dictation-destination" }); mark.setAttribute("aria-hidden", "true");
    if (command || row.session) mark.append(icon(command ? "flock" : "terminal", 15));
    else mark.textContent = row.app?.slice(0, 1).toLocaleUpperCase(locale()) || "?";
    return el("article", { class: "dictation-entry" }, transcript, metadata, heard, el("div", { class: "dictation-actions" }, command ? null : again, copy, remove), mark);
  };
  const read = async () => {
    const mine = ++asked;
    loading = true; draw();
    try { const next = await backend.load(); if (mine === asked) { if (!data && !metricsChanged && next.today?.count && window.matchMedia?.("(min-width: 900px) and (min-height: 561px)").matches) metricsExpanded = true; data = next; failed = false; } }
    catch { if (mine === asked) failed = true; }
    finally { if (mine === asked) { loading = false; draw(); } }
  };
  const act = async (run: () => Promise<void>, reload = true) => {
    if (busy) return;
    const origin = (document.activeElement as HTMLElement | null)?.dataset.historyAction;
    busy = true; actionError = false; requested = false; asked++; loading = false; draw();
    try { await run(); if (reload) await read(); }
    catch { actionError = true; }
    finally {
      busy = false; draw();
      const focused = document.activeElement as HTMLElement | null;
      const ownsFocus = !focused || focused === document.body || focused === list || confirmation.contains(focused)
        || (!!origin && element.contains(focused) && focused.dataset.historyAction === origin);
      if (ownsFocus && !document.hidden && !element.closest("[hidden]") && element.isConnected) {
        if (confirming !== null) focusConfirmation();
        else (origin ? element.querySelector<HTMLElement>(`[data-history-action="${origin}"]`) ?? list : list).focus({ preventScroll: true });
      }
    }
  };
  search.addEventListener("input", draw);
  search.addEventListener("keydown", event => {
    if (event.key === "Escape" && search.value) {
      event.preventDefault(); event.stopPropagation(); search.value = ""; draw();
    }
  });
  confirmation.addEventListener("keydown", e => { if (e.key === "Escape" && !busy) { e.preventDefault(); e.stopPropagation(); cancelConfirmation(); } });
  draw();
  return {
    element, setKeys(keys: Record<string, string>) { boundKeys = keys; draw(); }, refresh: () => void read(), relabel: draw,
    invalidate: () => { asked++; loading = false; data = null; confirming = null; requested = false; expanded.clear(); draw(); },
  };
}
