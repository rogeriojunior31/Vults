import { el } from "./dom";
import { t } from "./i18n";

/** Shared, read-only phrase guide; examples never execute a command. */
export function voiceCommandHelp(): HTMLElement {
  return el("div", { class: "voice-command-help" },
    el("p", { text: t("Commands understand English, Portuguese and Spanish. Use a current project or agent name in place of api, web or docs.") }),
    el("dl", {},
      el("dt", { text: t("Move around") }),
      el("dd", { text: t("Say: next session; previous; open api; go to the api terminal; open the web folder.") }),
      el("dt", { text: t("Ask about the flock") }),
      el("dd", { text: t("Say: who is waiting?; what is api doing?; how much of my plan is left?; how are my pull requests?") }),
      el("dt", { text: t("Quiet a project") }),
      el("dd", { text: t("Say: mute web; unmute web; pin api; unpin api; hide docs; unhide docs.") }),
      el("dt", { text: t("Media voice commands") }),
      el("dd", { text: t("Enable media voice commands and media controls first. Say: pause media; resume music; next track; previous track; media volume 30; lower media volume. Uses the selected player. Project mute only silences notifications.") }),
      el("dt", { text: t("Application voice commands") }),
      el("dd", { text: t("Enable application voice commands first. Say: open Obsidian; open app Firefox; open project api. Use an installed desktop name. Ambiguous names need a choice. Application launches cannot be repeated or undone.") }),
      el("dt", { text: t("Again and undo") }),
      el("dd", { text: t("Say: and docs too; same for web; undo. Only flock commands support repeat and undo, with 90 seconds of context. Opening a terminal or folder and reading a snapshot cannot be undone.") }),
      el("dt", { text: t("Never answers a card") }),
      el("dd", { text: t("Approve, allow, yes and other card answers are refused, even with Chat fallback on. Answer the card with a click or an authorized shortcut.") })),
    el("p", { text: t("Local answers use the latest snapshot Vults already has. They do not refresh a connector or call a model.") }));
}
