// One agent workspace renders production data or an explicitly marked, local-only design preview.
import type { AgentDefaults, AgentPreferenceEdit, AgentProfile, AgentProfileEdit, AgentProfileState, AgentProvider, AgentStateChange, ChatAccess } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { SPECIES, speciesSet } from "../../character/flock";
import { dress, LOOK_GROUPS } from "../../character/looks";
import { drawFrame, frameAt } from "../../character/sprites";
import { Scene, FOCUS_SCENE } from "../../island/scene";
import { ChatPanel } from "../../island/chat";
import { icon } from "../../island/icons";

type Executor = "claude" | "codex" | "api" | "local";
type Example = AgentProfile & { archived: boolean; executor: Executor; model: string; provider: string; effort: string };
export interface AgentWorkspaceBackend {
  save(edit: AgentProfileEdit, preferences: AgentPreferenceEdit): Promise<AgentProfile>;
  state(profile: string, version: number, change: AgentStateChange): Promise<AgentProfileState>;
  access(profile: string, access: ChatAccess): Promise<void>;
  open(profile: AgentProfile): Promise<void>;
  changed(profile: AgentProfile): void;
  reload(): void;
}
const initialState = (): AgentProfileState => ({
  version: 0, defaults: { executor: "claude", model: null, effort: null }, archived: false,
});
const sampleProviders: AgentProvider[] = [
  { id: "anthropic", label: "Anthropic", local: false }, { id: "openai", label: "OpenAI", local: false },
  { id: "google", label: "Google Gemini", local: false }, { id: "openrouter", label: "OpenRouter", local: false },
  { id: "groq", label: "Groq", local: false }, { id: "deepseek", label: "DeepSeek", local: false },
  { id: "mistral", label: "Mistral", local: false }, { id: "xai", label: "xAI", local: false },
  { id: "ollama", label: "Ollama", local: true }, { id: "lmstudio", label: "LM Studio", local: true },
];
const view = (profile: AgentProfile): Example => {
  const state = profile.state ?? initialState();
  const defaults = state.defaults;
  const local = defaults.executor === "api" && (profile.providers ?? sampleProviders).some(p => p.id === defaults.provider && p.local);
  return { ...structuredClone(profile), state: structuredClone(state), archived: state.archived,
    executor: local ? "local" : defaults.executor, model: defaults.model ?? "",
    provider: defaults.executor === "api" ? defaults.provider : "anthropic",
    effort: defaults.executor === "api" ? "" : defaults.effort ?? "" };
};
const preferences = (agent: Example): AgentDefaults => agent.executor === "api" || agent.executor === "local"
  ? { executor: "api", provider: agent.provider, model: agent.model.trim() || null }
  : { executor: agent.executor, model: agent.model.trim() || null, effort: agent.effort || null };
const executors: [Executor, string][] = [["claude", "Claude Code"], ["codex", "Codex"], ["api", "API"], ["local", "Local model"]];
const nameOf = (agent: Example) => agent.built_in ? t(agent.title) : agent.title;
const executorName = (agent: Example) => agent.executor === "api" || agent.executor === "local" ? (agent.providers??sampleProviders).find(p=>p.id===agent.provider)?.label ?? agent.provider : executors.find(([id]) => id === agent.executor)![1];
const accessName = (access: ChatAccess) => t(access === "ask" ? "Asks first" : access === "policies" ? "Your policies" : "Full access");
const button = (text: string, action: () => void, primary = false) => {
  const node = el("button", { class: `btn${primary ? " primary" : ""}`, text, onclick: action });
  node.type = "button"; return node;
};
function select(label: string, options: [string, string][], value: string, change: (value: string) => void): HTMLLabelElement {
  const input = el("select"); input.setAttribute("aria-label", label);
  for (const [value, text] of options) { const option = el("option", { text }); option.value = value; input.append(option); }
  input.value = value; input.addEventListener("change", () => change(input.value));
  return el("label", { class: "agent-preview-select" }, el("span", { text: label }), input);
}
function face(agent: Example): HTMLCanvasElement {
  const canvas = el("canvas", { class: "agent-preview-face" }); canvas.width = 112; canvas.height = 112;
  const set = dress(speciesSet(agent.species), agent.look);
  const frame = frameAt(set.clips.idle, 0);
  const left = Math.min(...frame.layers.map(([, x]) => x));
  const top = Math.min(...frame.layers.map(([, , y]) => y));
  const right = Math.max(...frame.layers.map(([part, x]) => x + Math.max(...set.parts[part].map(row => row.length))));
  const bottom = Math.max(...frame.layers.map(([part, , y]) => y + set.parts[part].length));
  const scale = Math.max(1, Math.min(2, Math.floor(104 / Math.max(right - left, bottom - top))));
  drawFrame(canvas.getContext("2d")!, set, frame, Math.floor((112 / scale - right + left) / 2) - left - frame.dx, Math.floor((112 / scale - bottom + top) / 2) - top - frame.dy, scale);
  canvas.setAttribute("aria-hidden", "true"); return canvas;
}

export function createAgentWorkspace(presets: AgentProfile[], back: () => void, backend?: AgentWorkspaceBackend, start: "catalogue" | "editor" | "chat" = "catalogue") {
  const root = el("section", { class: "agent-preview" });
  let examples: Example[] = presets.filter(p=>backend||p.built_in).map(view);
  if (!backend && examples[0]) examples.unshift({ ...structuredClone(examples[0]), id: "preview-custom-1", built_in: false, title: t("My developer"), executor: "codex", skills: examples[0].skill_catalog.slice(0, 2) });
  let selected = examples[0]?.id;
  let serial = 1;
  let filter = "all";
  let query = "";
  let draft: Example | null = null;
  let original: Example | null = null;
  let defaultsOnly = false;
  let pending = false;
  let saveError = "";
  const feedback = el("p", { class: "agent-preview-error" }); feedback.setAttribute("role", "alert");
  const errorText = (error: unknown) => error === "conflict" ? t("This profile changed. Cancel and reopen it before saving.")
    : error === "invalid" ? t("Use a name up to 160 bytes and non-empty instructions up to 32768 bytes.")
    : error === "brief" ? t("The profile and pinned skills must fit within 32768 bytes.")
    : error === "limit" ? t("You can save up to 100 custom profiles.")
    : error === "appearance" ? t("Choose an available species and accessory.")
    : error === "archived" ? t("This agent is archived. Restore it before sending a new message.")
    : t("Could not save the profile. Your draft is preserved.");
  const busy = (value: boolean) => {
    pending = value;
    for (const control of root.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLSelectElement | HTMLTextAreaElement>("button,input,select,textarea")) control.disabled = value;
  };
  const replace = (profile: AgentProfile) => {
    const agent = view(profile);
    const index = examples.findIndex(a => a.id === agent.id);
    if (index < 0) examples.unshift(agent); else examples[index] = agent;
    backend?.changed(profile); return agent;
  };
  let chatFor: string | null = null;
  let active = false;
  let skillQuery = "";
  const panels = new Map<string, ChatPanel>();
  const chatChoices = new Map<string, { agent: Example; project: string }>();
  const scene = new Scene({ ...FOCUS_SCENE, width: 192, height: 176, wire: null, zeca: { x: 48, wire: 148, scale: 4 }, emotes: false });
  scene.setActive(false);
  const motion = () => scene.setActive(active && !document.hidden && root.isConnected && !!root.querySelector(".agent-preview-hero"));
  document.addEventListener("visibilitychange", motion);
  const portrait = (agent: Example) => {
    scene.update([], null, { clip: "idle", agent: "other", set: dress(speciesSet(agent.species), agent.look) });
    scene.canvas.setAttribute("aria-hidden", "true");
    const node = button("", () => {
      if (!document.body.classList.contains("still") && !window.matchMedia("(prefers-reduced-motion: reduce)").matches) scene.react("hello");
    });
    node.className = "agent-profile-character agent-preview-hero";
    node.setAttribute("aria-label", t("Greet {name}", { name: nameOf(agent) || t("Create agent") }));
    node.append(scene.canvas);
    node.addEventListener("pointerenter", () => scene.lookAt("front"));
    node.addEventListener("pointerleave", () => scene.lookAt(null)); return node;
  };
  const field = (label: string, value: string, update: (value: string) => void, multiline = false) => {
    const input = multiline ? el("textarea") : el("input"); input.value = value;
    input.setAttribute("aria-label", label); input.addEventListener("input", () => update(input.value));
    return el("label", {}, el("span", { text: label }), input);
  };
  const begin = (agent: Example | null, onlyDefaults=false) => {
    if(pending||agent?.archived)return; original=agent; defaultsOnly=onlyDefaults; feedback.textContent=""; saveError="";
    const base = agent ?? examples.find(a => a.built_in);
    if (!base) return;
    draft = { ...structuredClone(base), id: !agent || (agent.built_in && !onlyDefaults) ? `preview-custom-${++serial}` : agent.id, built_in: false, archived: false,
      title: !agent ? "" : agent.built_in && !onlyDefaults ? t("{name} copy", { name: nameOf(agent) }) : agent.title,
      access: !agent || (agent.built_in && !onlyDefaults) ? "ask" : agent.access,
      instructions: !agent ? "" : agent.built_in ? t(agent.instructions) : agent.instructions, skills: !agent ? [] : structuredClone(agent.skills) };
    if (!agent) { draft.executor="claude"; draft.model=""; draft.effort=""; draft.provider="anthropic"; }
    skillQuery = ""; chatFor = null; draw();
    root.querySelector<HTMLInputElement>(".agent-preview-editor input")?.focus();
  };
  const defaults = (agent: Example, modelField: HTMLLabelElement) => {
    const provider = el("div");
    const effort = select(t("Default effort"), [["",t("Use the executor default")],["low",t("Low")],["medium",t("Medium")],["high",t("High")],["xhigh",t("Extra high")],["max",t("Maximum")]], agent.effort, value=>{agent.effort=value;});
    const paintProvider = () => {
      provider.replaceChildren(); effort.hidden=agent.executor==="api"||agent.executor==="local";
      if (agent.executor === "api" || agent.executor === "local") provider.append(select(t("Provider"),
        (agent.providers??sampleProviders).filter(p=>p.local===(agent.executor==="local")).map(p=>[p.id,p.label]), agent.provider, value => { agent.provider = value; }));
    };
    paintProvider();
    return el("fieldset", { class: "agent-preview-defaults" }, el("legend", { text: t("Conversation defaults") }),
      select(t("Executor"), executors.map(([id, name]) => [id, id === "local" ? t("Local model") : name]), agent.executor, value => {
        agent.executor = value as Executor; agent.provider = value === "local" ? "ollama" : "anthropic"; agent.model = ""; paintProvider();
        const model = modelField.querySelector("input"); if (model) model.value = "";
      }), provider,
      modelField,
      effort,
      el("p", { class: "note", text: backend?t("Defaults apply to new conversations. API agents use messages only; Codex is not available yet."):t("Use these defaults for new conversations. You can choose another executor or model in Chat.") }));
  };
  // Keep each model field with its defaults form rather than a shared mutable selection.
  const defaultFields = (agent: Example) => {
    const modelField = field(t("Default model"), agent.model, value => { agent.model = value; });
    modelField.querySelector("input")!.placeholder = t("Use the executor default");
    return defaults(agent, modelField);
  };
  const skills = (agent: Example) => {
    const content = el("div", { class: "agent-preview-skill-list" });
    const count = el("span", { class: "agent-preview-count" });
    const picked = el("div", { class: "agent-preview-selected-skills" });
    const paint = () => {
      count.textContent = t("{n} selected", { n: agent.skills.length });
      picked.replaceChildren(...agent.skills.map(skill => {
        const remove = button(t("Remove {name}", { name: t(skill.title) }), () => { agent.skills = agent.skills.filter(s => s.id !== skill.id); paint(); });
        remove.setAttribute("aria-label", t("Remove {name}", { name: t(skill.title) }));
        remove.replaceChildren(el("span", { text: t(skill.title) }), icon("close", 12, 2)); return remove;
      }));
      const found = agent.skill_catalog.filter(skill => `${t(skill.title)} ${t(skill.content)}`.toLocaleLowerCase().includes(skillQuery.toLocaleLowerCase()));
      content.replaceChildren(...found.map(skill => {
        const check = el("input"); check.type = "checkbox"; check.checked = agent.skills.some(s => s.id === skill.id);
        check.setAttribute("aria-label", t(skill.title));
        check.addEventListener("change", () => { agent.skills = check.checked ? [...agent.skills.filter(s => s.id !== skill.id), structuredClone(skill)] : agent.skills.filter(s => s.id !== skill.id); paint(); });
        return el("article", {}, el("label", {}, check, el("strong", { text: t(skill.title) })),
          el("details", {}, el("summary", { text: t("Details") }), el("p", { text: t(skill.content) })));
      }));
      if (!found.length) content.append(el("p", { class: "note", text: t("No matching skills.") }));
    };
    const search = el("input"); search.type = "search"; search.value = skillQuery; search.placeholder = t("Search skills"); search.setAttribute("aria-label", t("Search skills"));
    search.addEventListener("input", () => { skillQuery = search.value; paint(); }); paint();
    return el("section", { class: "agent-preview-skills" }, el("div", { class: "agent-preview-section-heading" }, el("h3", { text: t("Pinned skills") }), count),
      el("p", { class: "note", text: t("Skills guide the agent. They do not enable tools or increase access.") }), picked, search, content);
  };
  const editor = (agent: Example) => {
    const error = el("p", { class: "agent-preview-error", text: saveError }); error.setAttribute("role", "alert");
    const form = el("form", { class: "agent-preview-editor" });
    const name = field(t("Agent name"), agent.title, value => { agent.title = value; }); name.querySelector("input")!.required = true;
    const instructions = field(t("Instructions"), agent.instructions, value => { agent.instructions = value; }, true); instructions.querySelector("textarea")!.required = true;
    const appearance = el("section", { class: "agent-preview-appearance" }, portrait(agent),
      select(t("Species"), SPECIES.map(s => [s.id, t(s.name)]), agent.species, value => { agent.species = value; scene.update([], null, { clip: "idle", agent: "other", set: dress(speciesSet(value), agent.look) }); }),
      select(t("Accessory"), LOOK_GROUPS.flatMap(g => g.looks).filter(l => l.value !== "auto").map(l => [l.value, t(l.label)]), agent.look, value => { agent.look = value; scene.update([], null, { clip: "idle", agent: "other", set: dress(speciesSet(agent.species), value) }); }));
    const access = select(t("Access for this profile"), (["ask", "policies", "full"] as ChatAccess[]).map(level => [level, accessName(level)]), agent.access, value => { agent.access = value as ChatAccess; });
    if(backend)access.querySelector("select")!.disabled=true;
    const save = el("button", { class: "btn primary", text: backend?t("Save profile"):t("Save preview") }); save.type = "submit";
    form.addEventListener("submit", event => {
      event.preventDefault(); if(pending)return;
      if (!agent.title.trim() || !agent.instructions.trim() || new TextEncoder().encode(agent.title).length > 160 || new TextEncoder().encode([agent.instructions, ...agent.skills.map(s => s.content)].join("\n\n")).length > 32768) {
        error.textContent = t("Use a name up to 160 bytes and non-empty instructions up to 32768 bytes."); return;
      }
      agent.title=agent.title.trim();
      if(!backend){const i=examples.findIndex(a=>a.id===agent.id);if(i<0)examples.unshift(agent);else examples[i]=agent;selected=agent.id;draft=null;filter="all";query="";draw();return;}
      const values={title:agent.title,instructions:agent.instructions,species:agent.species,look:agent.look,skill_ids:agent.skills.map(s=>s.id)};
      const edit:AgentProfileEdit=!original?{action:"create",...values}:original.built_in?{action:"copy",preset:original.id,...values}:{action:"update",id:Number(original.id.slice(7)),revision:original.revision,...values};
      const expected=(!original||original.built_in)&&!defaultsOnly?0:original?.state?.version??0;
      busy(true);saveError="";error.textContent="";
      const saved=defaultsOnly&&original
        ?backend.state(original.id,expected,{action:"defaults",defaults:preferences(agent)}).then(state=>({...original!,state}))
        :backend.save(edit,{version:expected,defaults:preferences(agent)});
      void saved.then(profile => {
        replace(profile); selected = profile.id; draft = null; pending = false; filter = "all"; query = ""; draw();
        root.querySelector<HTMLButtonElement>("[data-preview-edit]")?.focus();
      }, failure => { pending = false; saveError = errorText(failure); draw(); });
    });
    form.append(el("div", { class: "agent-preview-editor-grid" },
      el("div", { class: "agent-preview-fields" }, el("h2", { text: defaultsOnly?t("Conversation defaults"):t("Make this agent yours") }), ...(defaultsOnly?[portrait(agent),el("h3",{text:agent.built_in?t(agent.title):agent.title})]:[name,instructions,skills(agent)])),
      el("aside", {}, ...(defaultsOnly?[]:[appearance]), defaultFields(agent), ...(backend?[el("p",{class:"note",text:t("Permissions are managed separately in Chat.")})]:[access]))),
      el("footer", { class: "agent-preview-save" }, error, save, button(t("Cancel"), () => { if(pending)return; draft = null; saveError=""; draw(); backend?.reload(); })));
    return form;
  };
  const chat = (agent: Example) => {
    let panel = panels.get(agent.id);
    if (!panel) {
      panel = new ChatPanel({ keyboard() {}, models: async () => [], pick: async () => ({}), setPick: async () => {},
        conversations: async () => [], rename: async () => {}, remove: async () => false,
        open: async () => { throw new Error("Preview conversations are kept in this panel."); },
        reset: async () => "claude", stop: async () => {}, decide() {},
        send: async () => { panel!.receive({ kind: "text", text: t("Preview only. Your message was not sent to a model.") }); panel!.receive({ kind: "done" }); },
      }, () => {
        if (!panel) return;
        const started = panel.element.classList.contains("has-messages");
        for (const control of panel.element.querySelectorAll<HTMLInputElement | HTMLSelectElement>(".agent-preview-chat-controls input, .agent-preview-chat-controls select")) {
          if (control.getAttribute("aria-label") !== t("Access for this profile")) control.disabled = started;
        }
      }, { window: "nest", specialist: { name: nameOf(agent), revision: agent.revision, species: agent.species, look: agent.look } });
      panels.set(agent.id, panel);
    }
    const retained = chatChoices.get(agent.id) ?? { agent: { ...agent }, project: "" };
    chatChoices.set(agent.id, retained);
    const choices = retained.agent;
    const provider = el("div", { class: "agent-preview-chat-provider" });
    const paintProvider = () => {
      provider.replaceChildren();
      if (choices.executor !== "api" && choices.executor !== "local") { provider.hidden = true; return; }
      provider.hidden = false;
      const available = (agent.providers ?? sampleProviders).filter(p => p.local === (choices.executor === "local"));
      if (!available.some(p => p.id === choices.provider)) choices.provider = available[0].id;
      provider.append(select(t("Provider"), available.map(p => [p.id, p.label]), choices.provider, value => { choices.provider = value; }));
      provider.querySelector("span")?.classList.add("agent-preview-sr-only");
    };
    paintProvider();
    const heading = el("div", { class: "agent-preview-chat-controls" }, button(nameOf(agent), () => { chatFor = null; draw(); }),
      select(t("Executor"), executors.map(([id, name]) => [id, id === "local" ? t("Local model") : name]), choices.executor, value => { choices.executor = value as Executor; paintProvider(); }), provider,
      field(t("Model"), choices.model, value => { choices.model = value; }),
      select(t("Project"), [["", t("No project")], ["example", t("Example project")]], retained.project, value => { retained.project = value; }),
      select(t("Access for this profile"), (["ask", "policies", "full"] as ChatAccess[]).map(level => [level, accessName(level)]), choices.access, value => { choices.access = value as ChatAccess; agent.access = choices.access; }),
      button(t("New conversation"), () => { panels.get(agent.id)?.toggle(false); panels.delete(agent.id); chatChoices.delete(agent.id); draw(); }));
    for (const label of heading.querySelectorAll("label")) label.querySelector("span")?.classList.add("agent-preview-sr-only");
    const modelInput = heading.querySelector<HTMLInputElement>('input[aria-label="'+t("Model")+'"]');
    if (modelInput) modelInput.placeholder = t("Default model");
    panel.element.querySelector(".chat-head")!.replaceChildren(heading);
    const caption = panel.element.querySelector<HTMLElement>(".chat-caption");
    if (caption) caption.textContent = t("Choose an executor and an optional project for this conversation.");
    panel.element.querySelector(".log-box")?.prepend(panel.perchSlot);
    panel.perchSlot.replaceChildren(portrait(agent));
    panel.setEnabled(!agent.archived); panel.toggle(true);
    const started = panel.element.classList.contains("has-messages");
    for (const control of heading.querySelectorAll<HTMLInputElement | HTMLSelectElement>("input, select")) if (control.getAttribute("aria-label") !== t("Access for this profile")) control.disabled = started;
    const attach = button(t("Attach an example file"), () => panel!.attach(["preview-notes.txt"])); attach.className = "attach"; attach.setAttribute("aria-label", t("Attach an example file")); attach.replaceChildren(icon("clip", 16, 2));
    const row = panel.element.querySelector(".composer-row")!; if (!row.querySelector(".attach")) row.prepend(attach);
    return el("div", { class: "nest-chat agent-preview-chat" }, panel.element);
  };
  const catalogue = () => {
    const results = el("div", { class: "agent-preview-results" });
    const detail = el("aside", { class: "agent-preview-detail" });
    const changeState = (agent:Example,change:AgentStateChange) => {
      if(pending)return;
      if(!backend){agent.archived=change.action==="archive"?change.archived:agent.archived;filter=agent.archived?"archived":"mine";paintResults();return;}
      busy(true); feedback.textContent="";
      void backend.state(agent.id,agent.state?.version??0,change).then(state=>{replace({...agent,state});pending=false;selected=agent.id;filter=state.archived?"archived":"mine";draw();},failure=>{busy(false);feedback.textContent=errorText(failure);});
    };
    const paintDetail = () => {
      const agent = examples.find(a => a.id === selected);
      detail.replaceChildren(); if (!agent) return;
      const edit = button(agent.built_in ? t("Create a copy") : t("Edit agent"), () => begin(agent)); edit.dataset.previewEdit = "true";
      detail.append(portrait(agent), el("h2", { text: nameOf(agent) }), el("p", { class: "note", text: agent.about === "Custom agent profile" ? t("Custom agent profile") : t(agent.about) }),
        el("p", { class: "note", text: agent.built_in?t("Built-in profile · Revision {revision}",{revision:agent.revision}):t("Custom profile · Revision {revision}",{revision:agent.revision}) }),
        el("div", { class: "agent-preview-tags" }, el("span", { text: executorName(agent) }), el("span", { text: agent.model || t("Default model") }), el("span", { text: accessName(agent.access) })),
        button(agent.archived ? t("Restore agent") : backend?t("Open specialist chat"):t("Open preview chat"), () => { if (agent.archived) {changeState(agent,{action:"archive",archived:false});return;}
          if(backend){if(pending)return;busy(true);feedback.textContent="";void backend.open(agent).then(()=>busy(false),error=>{busy(false);feedback.textContent=String(error);});return;}
          chatFor = agent.id; draw(); }, true), ...(agent.archived ? [] : [edit]));
      detail.querySelectorAll<HTMLButtonElement>("button").forEach(control=>{if(control.textContent===t("Open specialist chat"))control.classList.add("agent-profile-chat-open");});
      if (!agent.built_in && !agent.archived) detail.append(button(t("Archive agent"), () => changeState(agent,{action:"archive",archived:true})));
      if(backend && agent.archived) detail.append(button(t("View conversations"),()=>{if(!pending)void backend.open(agent).catch(error=>{feedback.textContent=String(error);});}));
      if(backend && agent.built_in)detail.append(button(t("Conversation defaults"),()=>begin(agent,true)));
      if(backend && !agent.archived){
        const access=select(t("Access for this profile"),(["ask","policies","full"] as ChatAccess[]).map(a=>[a,accessName(a)]),agent.access,value=>{
          if(pending)return;busy(true);feedback.textContent="";void backend.access(agent.id,value as ChatAccess).then(()=>{agent.access=value as ChatAccess;backend.changed(agent);busy(false);},()=>{busy(false);access.querySelector("select")!.value=agent.access;feedback.textContent=t("Could not save this profile's access level.");});
        }); detail.append(access);
      }
      if(backend && agent.executor!=="claude")detail.append(el("p",{class:"note",text:t("API agents use messages only. Codex specialist execution is not available yet.")}));
      detail.append(el("details", {}, el("summary", { text: t("Instructions") }), el("p", { text: agent.built_in ? t(agent.instructions) : agent.instructions })),
        el("details", {}, el("summary", { text: t("Pinned skills") }), ...agent.skills.map(s => el("p", { text: `${t(s.title)} · ${t("Revision {revision}",{revision:s.revision})}` }))));
      detail.append(feedback);
      motion();
    };
    const paintResults = () => {
      results.replaceChildren();
      const matches = examples.filter(a => (filter === "archived" ? a.archived : !a.archived && (filter === "all" || (filter === "templates" ? a.built_in : !a.built_in))) && `${nameOf(a)} ${t(a.about)} ${executorName(a)}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()));
      if (!matches.some(a => a.id === selected)) selected = matches[0]?.id;
      for (const builtIn of [false, true]) {
        const group = matches.filter(a => a.built_in === builtIn); if (!group.length) continue;
        const grid = el("div", { class: "agent-preview-grid" });
        for (const agent of group) {
          const choice = button("", () => { selected = agent.id; paintResults(); root.querySelector<HTMLButtonElement>(`[data-example="${agent.id}"]`)?.focus({ preventScroll: true }); });
          choice.className = "agent-preview-card"; choice.dataset.example = agent.id; choice.setAttribute("aria-pressed", String(selected === agent.id));
          choice.append(face(agent), el("strong", { text: nameOf(agent) }), el("span", { text: executorName(agent) }), el("small", { text: agent.archived ? t("Archived") : agent.built_in ? t("Template") : t("Skills: {n}", { n: agent.skills.length }) })); grid.append(choice);
        }
        results.append(el("h2", { text: builtIn ? t("Templates") : t("Your agents") }), grid);
      }
      if (!matches.length) results.append(el("p", { class: "note", text: t("No matching agents. Try another search or create an agent.") }));
      paintDetail();
    };
    const search = el("input"); search.type = "search"; search.value = query; search.placeholder = t("Search agents"); search.setAttribute("aria-label", t("Search agents"));
    search.addEventListener("input", () => { query = search.value; paintResults(); });
    const tabs = el("div", { class: "agent-preview-filters" });
    for (const [id, label] of [["all", "All"], ["mine", "Your agents"], ["templates", "Templates"], ["archived", "Archived"]] as const) {
      const choice = button(id === "all" ? t("All agents") : t(label), () => { filter = id; for (const child of tabs.querySelectorAll("button")) child.setAttribute("aria-pressed", String(child === choice)); paintResults(); });
      choice.setAttribute("aria-pressed", String(filter === id)); tabs.append(choice);
    }
    paintResults();
    return el("div", {}, el("div", { class: "agent-preview-toolbar" }, search, tabs), el("div", { class: "agent-preview-catalogue" }, results, detail));
  };
  function draw() {
    scene.setActive(false);
    const agent = examples.find(a => a.id === chatFor);
    root.replaceChildren(...(backend?[]:[el("div", { class: "agent-preview-banner" }, el("span", { text: t("Design preview. Examples stay here; your agents and settings are unchanged.") }))]),
      el("header", { class: "agent-preview-heading" }, el("div", {}, el("h1", { text: draft ? t("Configure agent") : chatFor ? t("Chat") : t("Agents") }), el("p", { class: "note", text: t("Give each agent a role, a look and its own way of working.") })),
        draft || chatFor ? null : button(t("Create agent"), () => begin(null), true),
        button(chatFor ? t("Back to agents") : backend?t("Back to Flock"):t("Close preview"), () => { if (chatFor) { panels.get(chatFor)?.toggle(false); chatFor = null; draw(); } else { visible(false); back(); } })),
      draft ? editor(draft) : agent ? chat(agent) : catalogue());
    if(pending)busy(true);
    motion();
  }
  function visible(on: boolean) { active = on; for (const panel of panels.values()) panel.toggle(on && panel === panels.get(chatFor ?? "")); motion(); }
  if (start === "editor") begin(examples[0] ?? null);
  else { if (start === "chat") chatFor = selected ?? null; draw(); }
  return { element: root, visible, relabel: draw,
    update(rows: AgentProfile[]) {
      for (const row of rows) {
        const current = examples.find(a => a.id === row.id);
        if (current && row.state && row.state.version > (current.state?.version ?? 0))
          examples[examples.indexOf(current)] = view({ ...current, state: row.state });
      }
      if (pending || draft) return;
      examples = rows.map(row => {
        const current = examples.find(a => a.id === row.id);
        return view(current && (current.state?.version ?? 0) > (row.state?.version ?? 0) ? { ...row, state: current.state } : row);
      });
      draw();
    },
    select(id: string) {
      selected = id;
      if (!draft) { filter = examples.find(a => a.id === id)?.archived ? "archived" : "all"; query = ""; draw(); }
    },
    updateState(id: string, state: AgentProfileState) {
      const current = examples.find(a => a.id === id);
      if (!current || state.version <= (current.state?.version ?? 0)) return;
      examples[examples.indexOf(current)] = view({ ...current, state });
      if (!pending && !draft) draw();
    },
    updateAccess(id: string, access: ChatAccess) {
      const current = examples.find(a => a.id === id);
      if (current) { current.access = access; if (!draft && !pending) draw(); }
    },
  };
}
