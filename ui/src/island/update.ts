// The update notice on the island (FF4, ADR 0019): the one question, then the notice of a newer
// version. Nothing here installs anything; the app opens only the release page it found.
import type { UpdateStatus, ViewModel } from "../bridge";
import { el } from "../dom";
import { t } from "../i18n";
import { icon } from "./icons";

export interface UpdateActions {
  updateChecks?(on: boolean): void;
  updateQuestionShown?(): void;
  updateLater?(): void;
  openUpdatePage?(): void;
}

/** Approvals, questions and a session needing you always come first: the notice waits. */
export function updateShown(update: UpdateStatus | null, v: ViewModel, paused: boolean, chatWaits: boolean): boolean {
  if (!update?.available || (!update.ask && !update.notice)) return false;
  if (v.locked || v.approval || paused || chatWaits) return false;
  return !v.sessions.some(s => s.card || s.waiting || s.attention === "needs-you");
}

/** The notes' first readable line, without Markdown marks; the rest stays on the release page.
 *  A heading ("Highlights") says nothing on its own: the first line under it is preferred. */
export function firstLine(notes: string): string {
  const plain = (l: string) => l.replace(/^\s*(?:#{1,6}|[-*+]|>|\d+\.)\s+/, "").replace(/\*\*|__|`/g, "").trim();
  const lines = notes.split(/\r?\n/).filter(l => l.trim());
  const body = lines.find(l => !/^\s*#{1,6}\s/.test(l));
  return plain(body ?? lines[0] ?? "");
}

/** `shown` runs when the question is built, so a question left unanswered comes back a day later. */
export function updateBox(u: UpdateStatus, actions: UpdateActions, shown: () => void): HTMLElement {
  if (u.notice) {
    const later = el("button", { class: "look-chip", text: t("Later"), onclick: () => actions.updateLater?.() });
    later.title = t("Hide this version for a day");
    const open = el("button", { class: "look-chip", text: t("Open release page"), onclick: () => actions.openUpdatePage?.() });
    const line = firstLine(u.notice.notes);
    const title = t("Vults {version} is available", { version: u.notice.version });
    const text = el("span", { class: "digest-text" },
      el("strong", { text: title }),
      ...(line ? [el("span", { class: "update-line", text: line })] : []));
    const box = el("div", { class: "digest update-notice" }, icon("update", 13), text, open, later);
    box.setAttribute("role", "group");
    box.setAttribute("aria-label", title);
    return box;
  }
  shown();
  const question = t("May Vults look at its releases on GitHub?");
  const yes = el("button", { class: "look-chip primary", text: t("Yes, once a day"), onclick: () => actions.updateChecks?.(true) });
  const no = el("button", { class: "look-chip", text: t("No"), onclick: () => actions.updateChecks?.(false) });
  const text = el("span", { class: "digest-text" },
    el("strong", { text: question }),
    el("span", { class: "update-line", text: t("Once a day, to tell you about a new version. Nothing about you is sent.") }));
  const box = el("div", { class: "digest update-ask" }, icon("update", 13), text, yes, no);
  box.setAttribute("role", "group");
  box.setAttribute("aria-label", question);
  return box;
}
