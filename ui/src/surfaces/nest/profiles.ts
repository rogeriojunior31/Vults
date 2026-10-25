import type { AgentProfile, AgentProfileState, ChatAccess } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { createAgentWorkspace, type AgentWorkspaceBackend } from "./agent-workspace";
import { viewState } from "./view-state";

export function createProfiles(
  load: () => Promise<AgentProfile[]>, back: () => void,
  save?: AgentWorkspaceBackend["save"], setAccess?: AgentWorkspaceBackend["access"],
  openChat?: (profile: AgentProfile) => void | Promise<void>, changed?: (profile: AgentProfile) => void,
  setState?: AgentWorkspaceBackend["state"],
  setGithub?: AgentWorkspaceBackend["github"],
) {
  const element = el("section", { class: "agent-profiles" });
  let workspace: ReturnType<typeof createAgentWorkspace> | null = null;
  let active = false;
  let failed = false;
  let pending = false;
  let request = 0;
  let selected: string | null = null;
  const unavailable = () => Promise.reject("unavailable");
  const backend: AgentWorkspaceBackend = {
    save: save ?? unavailable, state: setState ?? unavailable, access: setAccess ?? unavailable, github:setGithub,
    open: async profile => { await openChat?.(profile); }, reload: refresh, changed: profile => changed?.(profile),
  };
  const draw = () => {
    if (workspace) {
      element.replaceChildren(workspace.element); workspace.visible(active);
      if(failed)element.append(viewState("error",t("Could not load {name}.",{name:t("Agent profiles")}),undefined,
        el("button",{class:"btn",text:t("Try again"),onclick:refresh})));
    }
    else element.replaceChildren(viewState(failed ? "error" : "loading",
      failed ? t("Could not load {name}.", { name: t("Agent profiles") }) : t("Loading {name}…", { name: t("Agent profiles") }),
      undefined, ...(failed ? [el("button", { class: "btn", text: t("Try again"), onclick: refresh })] : [])));
  };
  function refresh() {
    if (!active || pending) return;
    pending = true; failed = false; const mine = ++request; draw();
    void load().then(rows => {
      if (mine !== request) return;
      pending = false;
      const first = workspace === null;
      if (workspace) workspace.update(rows);
      else workspace = createAgentWorkspace(rows, back, backend);
      if (selected) { workspace.select(selected); selected = null; }
      rows.forEach(profile => changed?.(profile)); draw();
      if (first && import.meta.env.DEV && import.meta.env.VITE_AGENTS_PREVIEW === "1" && import.meta.env.VITE_AGENTS_PREVIEW_START === "github")
        queueMicrotask(() => element.querySelector<HTMLInputElement>('[aria-label="'+t("GitHub repository")+'"]')?.scrollIntoView({block:"center"}));
      if (first && import.meta.env.DEV && import.meta.env.VITE_AGENTS_PREVIEW === "1" && ["chat","setup"].includes(import.meta.env.VITE_AGENTS_PREVIEW_START) && rows[0])
        void backend.open(rows[0]).then(()=>{if(import.meta.env.VITE_AGENTS_PREVIEW_START==="setup")document.querySelector<HTMLButtonElement>(".chat-controls .model")?.click();}).catch(error => element.append(viewState("error", t("Could not load {name}.", {name:t("Chat")}), String(error))));
    }, () => { if (mine === request) { pending = false; failed = true; draw(); } });
  }
  draw();
  return { element,
    relabel() { workspace?.relabel(); draw(); },
    select(id: string) { if(workspace)workspace.select(id);else selected = id; },
    updateState(id: string, state: AgentProfileState) { workspace?.updateState(id, state); },
    updateAccess(id: string, access: ChatAccess) { workspace?.updateAccess(id, access); },
    visible(on: boolean) {
      const entering = on && !active; active = on; workspace?.visible(on);
      if (!on) { request++; pending = false; }
      else if (entering && !failed) refresh();
    },
  };
}
