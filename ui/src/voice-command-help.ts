import { el } from "./dom";
import { t } from "./i18n";

/** Shared, read-only phrase guide; examples never execute a command. */
export function voiceCommandHelp(state = { query: "", scope: "all" }): HTMLElement {
  const root = el("div", { class: "voice-command-help" },
    el("p", { text: t("Commands understand English, Portuguese and Spanish. Use a current project or agent name in place of api, web or docs.") }),
    el("dl", {},
      el("dt", { text: t("Move around") }),
      el("dd", { text: t("Say: next session; previous; open api; go to the api terminal; open the web folder.") }),
      el("dt", { text: t("Ask about the flock") }),
      el("dd", { text: t("Say: who is waiting?; what is api doing?; how much of my plan is left?; how are my pull requests?") }),
      el("dt", { text: t("Project notifications and visibility") }),
      el("dd", { text: t("Say: mute notifications for web; unmute notifications for web; pin api; unpin api; hide docs; unhide docs.") }),
      el("dt", { text: t("Media voice commands") }),
      el("dd", { text: t("Enable media voice commands and media controls first. Say: pause media; resume music; next track; previous track; media volume 30; lower media volume. Uses the selected player. Project mute only silences notifications.") }),
      el("dt", { text: t("System audio voice commands") }),
      el("dd", { text: t("Enable system audio voice commands first. Say: mute computer; unmute computer; computer volume 40; raise computer volume; lower computer volume. Changes use the default output, not the microphone. Volume changes do not unmute it.") }),
      el("dt", { text: t("Application voice commands") }),
      el("dd", { text: t("Enable application voice commands first. Say: open Obsidian; open app Firefox; open project api. Use an installed desktop name. Ambiguous names need a choice. Application launches cannot be repeated or undone.") }),
      el("dt", { text: t("Again and undo") }),
      el("dd", { text: t("Say: and docs too; same for web; undo. Only flock commands support repeat and undo, with 90 seconds of context. Opening a terminal or folder and reading a snapshot cannot be undone.") }),
      el("dt", { text: t("Never answers a card") }),
      el("dd", { text: t("Approve, allow, yes and other card answers are refused, even with Chat fallback on. Answer the card with a click or an authorized shortcut.") })),
    el("p", { text: t("Local answers use the latest snapshot Vults already has. They do not refresh a connector or call a model.") }));
  const query = el("input");
  query.type = "search";
  query.dataset.setting = "command-example-search";
  query.placeholder = t("Search command examples");
  query.setAttribute("aria-label", t("Search command examples"));
  query.value = state.query;
  const scope = el("select");
  scope.dataset.setting = "command-example-scope";
  scope.setAttribute("aria-label", t("Command scope"));
  for (const [value, label] of [["all", t("All examples")], ["flock", t("Flock")],
    ["media", t("Media voice commands")], ["computer", t("System audio voice commands")],
    ["apps", t("Application voice commands")], ["safety", t("Never answers a card")]]) {
    const option = el("option", { text: label });
    option.value = value;
    scope.append(option);
  }
  scope.value = state.scope;
  const controls = el("div", { class: "voice-command-search" }, query, scope);
  const list = root.querySelector("dl")!;
  root.replaceChildren(root.children[0]!, controls, list, root.children[root.children.length - 1]!);
  const empty = el("p", { text: t("No command examples match. Try another search or scope.") });
  empty.setAttribute("role", "status");
  root.append(empty);
  const normalize = (value: string) => value.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase();
  const scopes = ["flock", "flock", "flock", "media", "computer", "apps", "flock", "safety"];
  const filter = () => {
    state.query = query.value;
    state.scope = scope.value;
    let visible = 0;
    const terms = normalize(state.query).trim().split(/\s+/u).filter(Boolean);
    const rows = Array.from(list.children);
    for (let i = 0; i < rows.length; i += 2) {
      const title = rows[i] as HTMLElement, examples = rows[i + 1] as HTMLElement;
      const text = normalize(`${title.textContent} ${examples.textContent}`);
      const matches = (state.scope === "all" || scopes[i / 2] === state.scope) && terms.every(term => text.includes(term));
      title.hidden = examples.hidden = !matches;
      if (matches) visible++;
    }
    empty.hidden = visible !== 0;
  };
  query.addEventListener("input", filter);
  scope.addEventListener("change", filter);
  filter();
  return root;
}
