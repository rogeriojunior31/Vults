// The Nest's side pane beside the Chat: Zeca's Memory, the files this conversation changed, and
// the conversation folder's files to reference. It reads only; nothing here runs or answers.
import { el } from "../../dom";
import { t } from "../../i18n";
import { icon } from "../../island/icons";
import type { ChatPanel } from "../../island/chat";

type Tab = "memory" | "changes" | "files";

const TABS: { id: Tab; label: () => string }[] = [
  { id: "memory", label: () => t("Memory") },
  { id: "changes", label: () => t("Changes") },
  { id: "files", label: () => t("Files") },
];

/** A diff's lines, colored, as the chat's activity draws them. */
function diffLines(diff: string): HTMLElement {
  return el("div", { class: "diff-lines" }, ...diff.split("\n").map(line => {
    const kind = line.startsWith("@@") ? "hunk" : line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : "ctx";
    return el("span", { class: `d-line d-${kind}`, text: kind === "hunk" ? line.replace(/^@@\s?/, "") : line || " " });
  }));
}

export function createWorkspace(memory: HTMLElement, chat: () => ChatPanel): {
  element: HTMLElement;
  show: (tab: Tab) => void;
  /** The conversation changed: the Changes tab reads it again while it shows. */
  changed: () => void;
  relabel: () => void;
} {
  let tab: Tab = "memory";
  const tabs = el("div", { class: "workspace-tabs" });
  tabs.setAttribute("role", "tablist");
  const changes = el("div", { class: "workspace-changes" });
  const search = el("input", { class: "workspace-search" }) as HTMLInputElement;
  const found = el("div", { class: "workspace-found" });
  const files = el("div", { class: "workspace-files" }, search, found);
  const views: Record<Tab, HTMLElement> = { memory, changes, files };
  const body = el("div", { class: "workspace-body" }, memory, changes, files);
  const element = el("div", { class: "workspace" }, tabs, body);
  const open = new Set<string>();
  let shown = "";

  const paintTabs = () => {
    tabs.replaceChildren(...TABS.map(({ id, label }) => {
      const b = el("button", { class: `workspace-tab${tab === id ? " on" : ""}`, text: label(), onclick: () => show(id) });
      b.setAttribute("role", "tab");
      b.setAttribute("aria-selected", String(tab === id));
      return b;
    }));
    for (const id of Object.keys(views) as Tab[]) views[id].hidden = id !== tab;
  };

  const paintChanges = () => {
    const list = chat().changes();
    const key = JSON.stringify([list, [...open]]);
    if (key === shown) return;
    shown = key;
    if (!list.length) {
      changes.replaceChildren(el("p", { class: "workspace-empty", text: t("Files the agent changes in this conversation show here, with their diffs.") }));
      return;
    }
    changes.replaceChildren(...list.map(change => {
      const row = el("details", { class: "workspace-change" },
        el("summary", {},
          el("span", { class: "workspace-path", text: change.path }),
          el("span", { class: "tool-counts" }, el("span", { class: "d-add", text: `+${change.added}` }), el("span", { class: "d-del", text: `−${change.removed}` }))),
        el("div", { class: "activity-diff" }, ...change.diffs.map(diffLines)),
        el("div", { class: "workspace-actions" },
          el("button", { class: "btn secondary", text: t("Reference in the draft"), onclick: () => chat().reference(change.path) })));
      row.open = open.has(change.path);
      row.addEventListener("toggle", () => { if (row.open) open.add(change.path); else open.delete(change.path); });
      return row;
    }));
  };

  let asked = 0;
  let timer: number | undefined;
  const findFiles = () => {
    const query = search.value.trim();
    const n = ++asked;
    if (!query) {
      found.replaceChildren(el("p", { class: "workspace-empty", text: chat().workFolder() ? t("Type to find a file or folder in the conversation's folder.") : t("Choose the conversation's folder first.") }));
      return;
    }
    void chat().findPaths(query).then(hits => {
      if (n !== asked) return;
      found.replaceChildren(...(hits.length ? hits.map(hit => {
        const b = el("button", { class: "workspace-file", onclick: () => chat().reference(hit.path) },
          icon(hit.dir ? "folder" : "file", 13), el("span", { text: hit.path }));
        b.title = t("Reference in the draft");
        return b;
      }) : [el("p", { class: "workspace-empty", text: t("No matches") })]));
    }, () => { if (n === asked) found.replaceChildren(el("p", { class: "workspace-empty", text: t("The folder could not be read.") })); });
  };
  search.addEventListener("input", () => { window.clearTimeout(timer); timer = window.setTimeout(findFiles, 150); });

  function show(next: Tab): void {
    tab = next;
    paintTabs();
    if (tab === "changes") { shown = ""; paintChanges(); }
    if (tab === "files") { findFiles(); search.focus(); }
  }

  const relabel = () => {
    search.placeholder = t("Find a file or folder");
    search.setAttribute("aria-label", search.placeholder);
    tabs.setAttribute("aria-label", t("Side panel"));
    shown = "";
    paintTabs();
    if (tab === "changes") paintChanges();
    if (tab === "files") findFiles();
  };
  relabel();
  return { element, show, changed: () => { if (tab === "changes") paintChanges(); }, relabel };
}
