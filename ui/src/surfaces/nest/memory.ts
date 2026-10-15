// The Memory beside the Nest's Chat (plan-zeca M6): what Zeca would remember, waiting for a yes; what he
// remembers now, file by file; and the history, each change undone with a click. Nothing reaches
// his memory without the user: a yes here is a commit, an undo a revert.
import type { MemoryView } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";

export interface MemoryBackend {
  load(): Promise<MemoryView>;
  decide(id: number, accept: boolean): Promise<void>;
  undo(id: string): Promise<void>;
}

const PLACES: Record<string, string> = {
  user: tk("About you"),
  memory: tk("What Zeca learned"),
  persona: tk("How Zeca speaks to you"),
};

/** A place as the user reads it; a roost by its name, from its folder's own claim. */
function place(p: string): string {
  if (p.startsWith("roost:")) return t("Project {name}", { name: p.slice(6) });
  return PLACES[p] ? t(PLACES[p]) : p;
}

/** The errors the memory's commands give back in English, translated here. */
const ERRORS = new Set([
  tk("That proposal is gone."),
  tk("Zeca's memory is off."),
  tk("Only a change Vults made can be undone here."),
]);

const SOURCES: Record<MemoryView["pending"][number]["source"], string> = {
  tool: tk("Zeca asked"),
  extraction: tk("From a finished conversation"),
  sleep: tk("While Zeca slept"),
};

/** The debrief Zeca's last sleep left: a model's reading of the days' summaries, said as such. */
function night(v: MemoryView): HTMLDetailsElement | null {
  const n = v.night;
  if (!n?.debrief) return null;
  return el(
    "details",
    { class: "memory-part memory-night" },
    el("summary", { text: t("While you were away") }),
    el("p", { class: "memory-night-line", text: t("Zeca's reading of the last days' summaries:") }),
    el("p", { class: "memory-debrief", text: n.debrief }),
  );
}

const BY: Record<MemoryView["history"][number]["by"], string> = {
  you: tk("You"),
  zeca: tk("Zeca, with your yes"),
  "zeca-alone": tk("Zeca alone"),
  sleep: tk("Zeca asleep"),
  other: tk("Not written by Vults"),
};

export function createMemory(backend: MemoryBackend): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: "memory" });
  const disclosures = new Map<string, HTMLDetailsElement>();
  let asked = 0;
  let loaded: MemoryView | null = null;
  let note: string | null = null;
  let failed = false;
  let loading = false;
  let acting = false;
  let returning: string | null = null;

  const keep = (key: string, details: HTMLDetailsElement, open = false) => {
    details.open = disclosures.get(key)?.open ?? open;
    disclosures.set(key, details);
    return details;
  };
  const identify = <T extends HTMLElement>(node: T, id: string): T => { node.dataset.fid = id; return node; };
  const action = (id: string, text: string, run: () => void, primary = false) => {
    const button = identify(el("button", { class: `btn${primary ? " primary" : ""}`, text, onclick: run }), id);
    button.disabled = acting || loading;
    return button;
  };
  const count = (n: number) => el("span", { class: "memory-count", text: String(n) });
  const heading = (label: string, n: number) => el("h4", {}, el("span", { text: label }), count(n));

  const act = (work: () => Promise<void>) => {
    if (acting || loading) return;
    acting = true;
    note = null;
    paint();
    void Promise.resolve().then(work).catch((error: unknown) => {
      const text = String(error);
      note = ERRORS.has(text) ? t(text) : text;
    }).then(() => refresh()).finally(() => { acting = false; paint(); });
  };

  const pending = (v: MemoryView): HTMLElement => {
    const rows = v.pending.map(p => {
      const change = (label: string, text: string, className: string) => el("div", { class: `memory-change ${className}` },
        el("span", { class: "memory-change-label", text: label }), el("p", { text }));
      return el("article", { class: "proposal" },
        el("header", { class: "proposal-head" },
          el("span", { class: "proposal-place", text: place(p.place) }),
          el("span", { class: "proposal-source", text: t(SOURCES[p.source]) })),
        p.folder ? el("p", { class: "memory-folder", text: p.folder }) : null,
        el("div", { class: "proposal-diff" },
          p.old ? change(t("Current memory"), p.old, "diff-old") : null,
          p.new ? change(t("Proposed memory"), p.new, "diff-new")
            : change(t("Proposed memory"), t("Remove this memory."), "diff-old")),
        p.reason ? el("p", { class: "proposal-reason", text: p.reason }) : null,
        el("div", { class: "proposal-actions" },
          action(`accept:${p.id}`, p.op === "remove" ? t("Accept removal") : t("Remember"), () => act(() => backend.decide(p.id, true)), true),
          action(`reject:${p.id}`, t("Turn down"), () => act(() => backend.decide(p.id, false)))));
    });
    const decided = v.accepted + v.rejected;
    return el("section", { class: "memory-part memory-proposals" },
      heading(t("Waiting for you"), v.pending.length),
      ...(rows.length ? rows : [el("p", { class: "memory-empty", text: t("Nothing waits.") })]),
      decided ? el("p", { class: "memory-rate", text: t("{a} of {n} accepted in the last 90 days.", { a: v.accepted, n: decided }) }) : null);
  };

  const files = (v: MemoryView): HTMLElement => {
    const nonempty = v.files.filter(f => f.facts.length);
    return el("section", { class: "memory-part" }, heading(t("What Zeca remembers"), nonempty.reduce((n, f) => n + f.facts.length, 0)),
      ...(nonempty.length ? nonempty.map(f => {
        const usage = t("{used} of {limit} characters", { used: f.used, limit: f.limit });
        const progress = f.limit > 0 ? el("progress", { class: `memory-progress${f.used / f.limit >= 0.9 ? " critical" : f.used / f.limit >= 0.7 ? " warning" : ""}` }) : null;
        if (progress) { progress.max = f.limit; progress.value = f.used; progress.setAttribute("aria-label", usage); }
        return keep(`file:${f.path}`, el("details", { class: "memory-file" },
          identify(el("summary", { class: "memory-file-head" }, el("span", { text: place(f.place) }), count(f.facts.length)), `file:${f.path}`),
          el("div", { class: "memory-file-body" }, el("p", { class: "memory-used", text: usage }), progress,
            el("ul", {}, ...f.facts.map(text => el("li", { text }))))), nonempty.length === 1);
      }) : [el("p", { class: "memory-empty", text: t("Nothing yet. Type /remember in the chat, or accept what Zeca proposes.") })]));
  };

  const history = (v: MemoryView): HTMLDetailsElement | null => v.history.length ? keep("history", el("details", { class: "memory-part memory-history" },
    identify(el("summary", {}, el("span", { text: t("History") }), count(v.history.length)), "history"),
    ...v.history.map(c => el("div", { class: "memory-commit" },
      el("div", { class: "memory-commit-text" }, el("span", { class: "memory-commit-msg", text: c.message }), el("span", { class: "memory-commit-by", text: t(BY[c.by]) })),
      c.by === "other" ? null : action(`undo:${c.id}`, t("Undo"), () => act(() => backend.undo(c.id))))))) : null;

  function paint(): void {
    const active = document.activeElement as HTMLElement | null;
    if (active && element.contains(active)) returning = active.dataset.fid ?? null;
    else if (active && active !== document.body) returning = null;
    const scroll = element.parentElement?.scrollTop;
    const title = el("h3", { class: "memory-heading", text: t("Memory") });
    title.tabIndex = -1;
    const state = el("div", { class: "memory-state" });
    state.setAttribute("role", "status");
    if (acting) state.append(el("p", { text: t("Saving…") }));
    else if (loading && !loaded) state.append(el("p", { text: t("Loading") }));
    if (note) state.append(el("p", { class: "memory-note", text: note }));
    if (failed) {
      state.append(el("p", { class: "memory-note", text: loaded ? t("Memory could not update. The last loaded view may be out of date.") : t("Memory could not load. Try again.") }),
        action("retry", t("Try again"), () => { if (!loading && !acting) void refresh(); }));
    }
    const parts: HTMLElement[] = [el("header", { class: "memory-heading-group" }, title,
      el("p", { text: t("Review what Zeca keeps for future conversations.") })), state];
    if (loaded?.on) {
      const overnight = night(loaded);
      const changes = history(loaded);
      parts.push(pending(loaded), files(loaded));
      if (overnight) parts.push(keep("night", overnight));
      if (changes) parts.push(changes);
    } else if (loaded) parts.push(el("p", { class: "memory-empty", text: t("Zeca's memory is off. Turn it on in Settings, Chat & voice, Memory.") }));
    element.setAttribute("aria-busy", String(acting || loading));
    element.replaceChildren(...parts);
    if (scroll !== undefined && element.parentElement) element.parentElement.scrollTop = scroll;
    if (returning) {
      const target = element.querySelector<HTMLElement>(`[data-fid="${CSS.escape(returning)}"]`);
      if (target && !(target as HTMLButtonElement).disabled) { target.focus({ preventScroll: true }); returning = null; }
      else if (!acting && !loading) { title.focus({ preventScroll: true }); returning = null; }
    }
  }

  async function refresh(): Promise<void> {
    const mine = ++asked;
    loading = true;
    paint();
    try {
      const next = await backend.load();
      if (mine !== asked) return;
      loaded = next;
      failed = false;
    } catch {
      if (mine !== asked) return;
      failed = true;
    } finally {
      if (mine === asked) { loading = false; paint(); }
    }
  }
  return { element, refresh: () => { void refresh(); } };
}
