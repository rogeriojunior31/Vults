import type { MeetingNotes } from "../../../bridge";
import { el } from "../../../dom";
import { t } from "../../../i18n";

export function meetingNotes(notes: MeetingNotes, mode: "summary" | "todos"): HTMLElement[] {
  if (mode === "summary") {
    const list = el("ul");
    for (const point of notes.key_points) list.append(el("li", { text: point }));
    return [el("h3", { text: notes.title }), el("p", { class: "meeting-note-text", text: notes.summary }),
      el("h4", { text: t("Key points") }), notes.key_points.length ? list : el("p", { text: t("No key points were returned.") })];
  }
  const list = el("ol");
  for (const todo of notes.todos) list.append(el("li", {},
    el("p", { class: "meeting-note-text", text: todo.text }),
    el("p", { class: "meeting-todo-metadata", text: t("Owner: {owner}", { owner: todo.owner ?? t("Not assigned") }) }),
    el("p", { class: "meeting-todo-metadata", text: t("Due: {due}", { due: todo.due ?? t("Not stated") }) })));
  return [notes.todos.length ? list : el("p", { text: t("No explicitly agreed actions were found.") })];
}
