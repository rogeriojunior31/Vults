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
      el("dt", { text: t("Again and undo") }),
      el("dd", { text: t("Say: and docs too; same for web; undo. The last successful command is remembered for 90 seconds. Opening a terminal or folder and reading a snapshot cannot be undone.") }),
      el("dt", { text: t("Never answers a card") }),
      el("dd", { text: t("Approve, allow, yes and other card answers are refused, even with Chat fallback on. Answer the card with a click or an authorized shortcut.") })),
    el("p", { text: t("Local answers use the latest snapshot Vults already has. They do not refresh a connector or call a model.") }));
}
