// The Nest's page, with no Tauri in it: the backend comes in, so the lab can draw it too.
import { el } from "../../dom";
import { t } from "../../i18n";
import { ChatPanel, type ChatBackend } from "../../island/chat";
import { renderFull } from "../../island/markdown";

/** The Nest's header and its Chat tab; `toIsland` brings the island's chat up for a card. */
export function createNest(root: HTMLElement, backend: ChatBackend, toIsland: () => void): ChatPanel {
  const chat = new ChatPanel(backend, () => {}, { window: "nest", render: renderFull, toIsland });
  root.append(
    el(
      "header",
      { class: "nest-head" },
      el("span", { class: "nest-title", text: t("Nest") }),
      el("nav", { class: "tabs" }, el("button", { class: "tab on", text: t("Chat") })),
    ),
    el("div", { class: "nest-chat" }, chat.element),
  );
  chat.toggle(true);
  return chat;
}
