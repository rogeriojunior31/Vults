// The Nest's Memory tab (plan-zeca M6): what Zeca would remember, waiting for a yes; what he
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

const BY: Record<MemoryView["history"][number]["by"], string> = {
  you: tk("You"),
  zeca: tk("Zeca, with your yes"),
  "zeca-alone": tk("Zeca alone"),
  sleep: tk("Zeca asleep"),
  other: tk("Not written by Vults"),
};

export function createMemory(backend: MemoryBackend): { element: HTMLElement; refresh: () => void } {
  const element = el("div", { class: "memory" });
  let asked = 0;
  let note: string | null = null;

  const act = (work: Promise<void>) => {
    note = null;
    void work.catch((e: unknown) => {
      note = String(e);
    }).finally(refresh);
  };

  const pending = (v: MemoryView): HTMLElement => {
    const rows = v.pending.map((p) => {
      const diff = el("div", { class: "proposal-diff" });
      if (p.old) diff.append(el("div", { class: "diff-old", text: `- ${p.old}` }));
      if (p.new) diff.append(el("div", { class: "diff-new", text: `+ ${p.new}` }));
      return el(
        "article",
        { class: "proposal" },
        el(
          "header",
          { class: "proposal-head" },
          el("span", { class: "proposal-place", text: place(p.place) }),
          el("span", { class: "proposal-source", text: p.source === "extraction" ? t("From a finished conversation") : t("Zeca asked") }),
        ),
        diff,
        p.reason ? el("p", { class: "proposal-reason", text: p.reason }) : null,
        el(
          "div",
          { class: "proposal-actions" },
          el("button", { class: "btn primary", text: t("Remember"), onclick: () => act(backend.decide(p.id, true)) }),
          el("button", { class: "btn", text: t("Turn down"), onclick: () => act(backend.decide(p.id, false)) }),
        ),
      );
    });
    const decided = v.accepted + v.rejected;
    return el(
      "section",
      { class: "memory-part" },
      el("h4", { text: t("Waiting for you") }),
      ...(rows.length ? rows : [el("p", { class: "board-empty", text: t("Nothing waits.") })]),
      decided ? el("p", { class: "memory-rate", text: t("{a} of {n} accepted in the last 90 days.", { a: v.accepted, n: decided }) }) : null,
    );
  };

  const files = (v: MemoryView): HTMLElement =>
    el(
      "section",
      { class: "memory-part" },
      el("h4", { text: t("What Zeca remembers") }),
      ...(v.files.some((f) => f.facts.length)
        ? v.files
            .filter((f) => f.facts.length)
            .map((f) =>
              el(
                "div",
                { class: "memory-file" },
                el(
                  "div",
                  { class: "memory-file-head" },
                  el("span", { text: place(f.place) }),
                  el("span", { class: "memory-used", text: t("{used} of {limit} characters", { used: f.used, limit: f.limit }) }),
                ),
                el("ul", {}, ...f.facts.map((x) => el("li", { text: x }))),
              ),
            )
        : [el("p", { class: "board-empty", text: t("Nothing yet. Type /remember in the chat, or accept what Zeca proposes.") })]),
    );

  const history = (v: MemoryView): HTMLElement | null =>
    v.history.length
      ? el(
          "section",
          { class: "memory-part" },
          el("h4", { text: t("History") }),
          ...v.history.map((c) =>
            el(
              "div",
              { class: "memory-commit" },
              el("span", { class: "memory-commit-msg", text: c.message }),
              el("span", { class: "memory-commit-by", text: t(BY[c.by]) }),
              c.message.startsWith("Undo:") ? null : el("button", { class: "btn small", text: t("Undo"), onclick: () => act(backend.undo(c.id)) }),
            ),
          ),
        )
      : null;

  const refresh = () => {
    const mine = ++asked;
    void backend.load().then(
      (v) => {
        if (mine !== asked) return;
        if (!v.on) {
          element.replaceChildren(el("p", { class: "board-empty", text: t("Zeca's memory is off. Turn it on in Settings, Chat, Memory.") }));
          return;
        }
        element.replaceChildren(
          ...(note ? [el("p", { class: "memory-note", text: note })] : []),
          pending(v),
          files(v),
          ...(history(v) ? [history(v)!] : []),
        );
      },
      () => {},
    );
  };
  return { element, refresh };
}
