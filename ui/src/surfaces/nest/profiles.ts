import type { AgentProfile, AgentProfileEdit, ChatAccess } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { SPECIES, speciesSet } from "../../character/flock";
import { drawFrame, frameAt } from "../../character/sprites";
import { dress, LOOK_GROUPS } from "../../character/looks";
import { Scene, FOCUS_SCENE } from "../../island/scene";
import { viewState } from "./view-state";

function preview(species: string, look: string): HTMLCanvasElement {
  const canvas = el("canvas", { class: "agent-profile-preview" });
  const set = dress(speciesSet(species), look);
  const frame = frameAt(set.clips.idle, 0);
  const minX = Math.min(...frame.layers.map(([, x]) => x));
  const minY = Math.min(...frame.layers.map(([, , y]) => y));
  const maxX = Math.max(...frame.layers.map(([part, x]) => x + Math.max(0, ...set.parts[part].map(row => row.length))));
  const maxY = Math.max(...frame.layers.map(([part, , y]) => y + set.parts[part].length));
  // Accessories can extend above the head; center the complete frame without stretching pixels.
  canvas.width = Math.max(80, (maxX - minX + 4) * 2);
  canvas.height = Math.max(96, (maxY - minY + 4) * 2);
  drawFrame(canvas.getContext("2d")!, set, frame,
    Math.floor((canvas.width / 2 - maxX + minX) / 2) - minX - frame.dx,
    Math.floor((canvas.height / 2 - maxY + minY) / 2) - minY - frame.dy, 2);
  canvas.setAttribute("role", "img");
  canvas.setAttribute("aria-label", t("Appearance: {species}. Accessory: {accessory}.", {
    species: t(SPECIES.find(s => s.id === species)?.name ?? species),
    accessory: t(LOOK_GROUPS.flatMap(group => group.looks).find(item => item.value === look)?.label ?? "None"),
  }));
  return canvas;
}

export function createProfiles(load: () => Promise<AgentProfile[]>, back: () => void, save?: (edit: AgentProfileEdit) => Promise<AgentProfile>, setAccess?: (profile: string, access: ChatAccess) => Promise<void>, openChat?: (profile: AgentProfile) => void | Promise<void>, changed?: (profile: AgentProfile) => void) {
  const element = el("section", { class: "agent-profiles" });
  const character = new Scene({ ...FOCUS_SCENE, width: 192, height: 176, wire: null,
    zeca: { x: 48, wire: 148, scale: 4 }, emotes: false });
  character.setActive(false);
  const motion = () => character.setActive(active && !document.hidden && element.isConnected && !!element.querySelector(".agent-profile-character"));
  document.addEventListener("visibilitychange", motion);
  const avatar = (name: string, species: string, look: string) => {
    const button = el("button", { class: "agent-profile-character", onclick: () => { if (!document.body.classList.contains("still") && !window.matchMedia("(prefers-reduced-motion: reduce)").matches) character.react("hello"); } }, character.canvas);
    button.type = "button";
    button.setAttribute("aria-label", t("Greet {name}", { name }));
    character.canvas.setAttribute("aria-hidden", "true");
    button.addEventListener("pointerenter", () => character.lookAt("front"));
    button.addEventListener("pointerleave", () => character.lookAt(null));
    character.update([], null, { clip: "idle", agent: "other", set: dress(speciesSet(species), look) });
    return button;
  };
  let profiles: AgentProfile[] | null = null;
  let selected: string | null = null;
  let failed = false;
  let active = false;
  let asked = 0;
  let pending = false;
  let draft: { profile: AgentProfile | null; title: string; instructions: string; species: string; look: string; skillIds: string[] } | null = null;
  let saving = false;
  let saveError = "";
  let skillOptionsOpen = false;
  let opening: string | null = null;
  const expanded = new Set<string>();
  const openErrors = new Map<string, string>();
  const accessPending = new Set<string>();
  const accessErrors = new Map<string, string>();
  const begin = (profile: AgentProfile | null) => {
    if (pending) return;
    draft = { profile, title: profile ? profile.built_in ? t("{name} copy", { name: t(profile.title) }) : profile.title : "",
      instructions: profile ? profile.built_in ? t(profile.instructions) : profile.instructions : "",
      species: profile?.species ?? "atratus", look: profile?.look ?? "none",
      skillIds: profile?.skills.map(skill => skill.id) ?? [] };
    skillOptionsOpen = !!profile?.skills.length;
    saveError = ""; draw(); element.scrollTop = 0;
    element.querySelector<HTMLInputElement>(".agent-profile-editor input")?.focus();
  };
  const submit = () => {
    if (!draft || !save || saving) return;
    const current = draft;
    const edit: AgentProfileEdit = !current.profile
      ? { action: "create", title: current.title, instructions: current.instructions, species: current.species, look: current.look, skill_ids: current.skillIds }
      : current.profile.built_in
      ? { action: "copy", preset: current.profile.id, title: current.title, instructions: current.instructions, species: current.species, look: current.look, skill_ids: current.skillIds }
      : { action: "update", id: Number(current.profile.id.slice(7)), revision: current.profile.revision,
        title: current.title, instructions: current.instructions, species: current.species, look: current.look, skill_ids: current.skillIds };
    saving = true; saveError = ""; draw();
    void save(edit).then(profile => {
      profiles = [...(profiles ?? []).filter(p => p.id !== profile.id), profile];
      changed?.(profile);
      selected = profile.id; draft = null; saving = false; draw();
      if (active) (element.querySelector<HTMLElement>(".agent-profile-edit") ?? element.querySelector<HTMLElement>(".agent-profile-create"))?.focus({ preventScroll: true });
    }, error => {
      saving = false;
      saveError = error === "conflict" ? t("This profile changed. Cancel and reopen it before saving.")
        : error === "brief" ? t("The profile and pinned skills must fit within 32768 bytes.")
        : error === "appearance" ? t("Choose an available species and accessory.")
        : error === "invalid" ? t("Use a name up to 160 bytes and non-empty instructions up to 32768 bytes.")
        : error === "limit" ? t("You can save up to 100 custom profiles.") : t("Could not save the profile. Your draft is preserved.");
      draw();
    });
  };
  const draw = () => {
    character.setActive(false);
    const title = el("h2", { text: t("Agent profiles") });
    const heading = el("header", { class: "activity-heading" },
      el("div", {}, title, el("p", { text: t("Roles, instructions, and per-profile access.") })),
      el("button", { class: "btn", text: t("Back to Flock"), onclick: back }));
    if (save && !draft) {
      const create = el("button", { class: "btn primary agent-profile-create", text: t("Create profile"), onclick: () => begin(null) });
      create.disabled = pending || profiles === null;
      heading.append(create);
    }
    const content = el("div", { class: `agent-profile-content${draft ? " editing" : ""}` });
    if (draft) {
      const name = el("input"); name.type = "text"; name.value = draft.title; name.required = true; name.disabled = saving;
      const instructions = el("textarea"); instructions.required = true; instructions.disabled = saving;
      instructions.value = draft.instructions;
      instructions.placeholder = t("Describe the agent's role, priorities and limits.");
      name.addEventListener("input", () => { if (draft) { draft.title = name.value; element.querySelector(".agent-profile-character")?.setAttribute("aria-label", t("Greet {name}", { name: draft.title || t("Create profile") })); } });
      instructions.addEventListener("input", () => { if (draft) draft.instructions = instructions.value; });
      const species = el("select"); species.disabled = saving;
      species.dataset.field = "species";
      for (const item of SPECIES) {
        const option = el("option", { text: t(item.name) }); option.value = item.id; species.append(option);
      }
      species.value = draft.species;
      const look = el("select"); look.disabled = saving; look.dataset.field = "look";
      for (const group of LOOK_GROUPS) for (const item of group.looks) {
        if (item.value === "auto") continue;
        const option = el("option", { text: t(item.label) }); option.value = item.value; look.append(option);
      }
      look.value = draft.look;
      const image = el("div", { class: "agent-profile-preview-slot" }, avatar(draft.title || t("Create profile"), draft.species, draft.look));
      const updateAppearance = () => {
        if (!draft || saving) return;
        draft.species = species.value; draft.look = look.value;
        character.update([], null, { clip: "idle", agent: "other", set: dress(speciesSet(draft.species), draft.look) });
      };
      species.addEventListener("change", updateAppearance); look.addEventListener("change", updateAppearance);
      const appearance = el("fieldset", { class: "agent-profile-appearance" }, el("legend", { text: t("Appearance") }),
        el("div", { class: "agent-profile-appearance-grid" }, image, el("div", {},
          el("label", {}, el("span", { text: t("Species") }), species),
          el("label", {}, el("span", { text: t("Accessory") }), look))));
      const skillChoices = el("fieldset", { class: "agent-profile-skills" },
        el("p", { class: "note", text: t("Skill content and revision are saved with this profile. They do not enable tools.") }));
      skillChoices.setAttribute("aria-label", t("Pinned skills"));
      for (const skill of draft.profile?.skill_catalog ?? profiles?.[0]?.skill_catalog ?? []) {
        const checkbox = el("input"); checkbox.type = "checkbox"; checkbox.value = skill.id;
        checkbox.checked = draft.skillIds.includes(skill.id); checkbox.disabled = saving;
        checkbox.addEventListener("change", () => {
          if (!draft) return;
          draft.skillIds = checkbox.checked ? [...new Set([...draft.skillIds, skill.id])] : draft.skillIds.filter(id => id !== skill.id);
        });
        skillChoices.append(el("label", { class: "agent-profile-skill" }, checkbox,
          el("span", {}, el("strong", { text: t(skill.title) }), el("span", { text: `${t("Revision {revision}", { revision: skill.revision })} · ${t(skill.content)}` }))));
      }
      const saveButton = el("button", { class: "btn primary", text: saving ? t("Saving…") : t("Save profile") });
      saveButton.type = "submit"; saveButton.disabled = saving;
      const cancel = el("button", { class: "btn", text: t("Cancel"), onclick: () => {
        draft = null; saveError = ""; draw(); refresh(true);
      } });
      cancel.type = "button"; cancel.disabled = saving;
      const title = !draft.profile ? t("Create profile") : draft.profile.built_in ? t("Create a copy") : t("Edit profile");
      const explanation = !draft.profile ? t("Choose a name, instructions and appearance for your agent.")
        : draft.profile.built_in ? t("Start from {name}. Your changes are saved as a separate profile.", { name: t(draft.profile.title) })
          : t("Changes apply to new conversations. Existing conversations keep their saved revision.");
      const skillOptions = el("details", { class: "agent-profile-skill-options" }, el("summary", { text: t("Pinned skills") }), skillChoices);
      skillOptions.open = skillOptionsOpen;
      skillOptions.addEventListener("toggle", () => { skillOptionsOpen = skillOptions.open; });
      const form = el("form", { class: "agent-profile-editor" },
        el("header", { class: "agent-profile-editor-heading" }, el("h3", { text: title }), el("p", { class: "note", text: explanation })),
        el("div", { class: "agent-profile-editor-grid" },
          el("div", { class: "agent-profile-fields" },
            el("label", {}, el("span", { text: t("Profile name") }), name),
            el("label", {}, el("span", { text: t("Instructions") }), instructions)),
          el("aside", { class: "agent-profile-customization" }, appearance,
            skillOptions)),
        el("p", { class: "note", text: t("Saved locally. This does not start an agent or change permissions.") }),
        el("div", { class: "agent-profile-actions" }, saveButton, cancel));
      form.addEventListener("submit", event => { event.preventDefault(); submit(); });
      if (saveError) { const alert = el("p", { text: saveError }); alert.setAttribute("role", "alert"); form.append(alert); }
      content.append(form);
    } else if (profiles !== null) {
      const list = el("div", { class: "agent-profile-list" });
      list.setAttribute("role", "group");
      list.setAttribute("aria-label", t("Agent profiles"));
      for (const profile of profiles) {
        const canvas = preview(profile.species, profile.look);
        const button = el("button", { class: "agent-profile-choice", onclick: () => {
          selected = profile.id; draw();
          element.querySelector<HTMLElement>(`[data-profile="${CSS.escape(profile.id)}"]`)?.focus({ preventScroll: true });
        } }, canvas, el("span", {}, el("strong", { text: profile.built_in ? t(profile.title) : profile.title }), el("span", { text: profile.about === "Custom agent profile" ? t("Custom agent profile") : t(profile.about) })));
        button.dataset.profile = profile.id;
        button.setAttribute("aria-pressed", String(selected === profile.id));
        list.append(button);
      }
      const profile = profiles.find(profile => profile.id === selected);
      const detail = el("section", { class: "agent-profile-detail" });
      detail.setAttribute("aria-live", "polite");
      if (profile) detail.append(el("h3", { text: profile.built_in ? t(profile.title) : profile.title }),
        el("p", { class: "note", text: profile.built_in ? t("Built-in profile · Revision {revision}", { revision: profile.revision }) : t("Custom profile · Revision {revision}", { revision: profile.revision }) }));
      if (profile) {
        if (openChat) {
          const open = el("button", { class: "btn primary agent-profile-chat-open", text: opening === profile.id ? t("Loading") : t("Open specialist chat"), onclick: () => {
            if (opening || accessPending.has(profile.id) || pending) return;
            opening = profile.id; openErrors.delete(profile.id); draw();
            void Promise.resolve().then(() => openChat(profile)).catch(error => { openErrors.set(profile.id, String(error)); }).finally(() => {
              opening = null; if (active) draw();
            });
          } });
          open.disabled = !!opening || pending || accessPending.has(profile.id);
          detail.append(open);
          const error = el("p", { class: "agent-profile-open-error", text: openErrors.get(profile.id) ?? "" });
          error.hidden = !openErrors.has(profile.id); error.setAttribute("role", "alert"); detail.append(error);
        }
        const access = el("select");
        for (const [value, label] of [["ask", "Asks first"], ["policies", "Your policies"], ["full", "Full access"]] as const) {
          const option = el("option", { text: t(label) }); option.value = value; access.append(option);
        }
        access.dataset.access = profile.id;
        access.value = profile.access;
        access.disabled = !setAccess || pending || accessPending.has(profile.id);
        const about = (level: ChatAccess) => t(level === "ask" ? "A card before every command or edit."
          : level === "policies" ? "Your accepted policies answer what they cover; the rest asks."
            : "Everything runs without a card, but destructive commands and what a policy denies or asks first. Once the conversation reads from outside, cards ask again.");
        const accessAbout = el("p", { class: "note agent-profile-access-note", text: about(profile.access) });
        const accessError = el("p", { class: "note agent-profile-access-error", text: accessErrors.get(profile.id) ?? "" });
        accessError.setAttribute("role", "alert"); accessError.hidden = !accessErrors.has(profile.id);
        access.addEventListener("change", () => {
          if (!setAccess || accessPending.has(profile.id)) return;
          const next = access.value as ChatAccess;
          accessPending.add(profile.id); accessErrors.delete(profile.id); access.disabled = true;
          const open = detail.querySelector<HTMLButtonElement>(".agent-profile-chat-open"); if (open) open.disabled = true;
          void setAccess(profile.id, next).then(() => {
            const current = profiles?.find(item => item.id === profile.id);
            if (current) { current.access = next; changed?.(current); }
          }, () => { accessErrors.set(profile.id, t("Could not save this profile's access level.")); }).finally(() => {
            accessPending.delete(profile.id);
            const current = profiles?.find(item => item.id === profile.id);
            const control = element.querySelector<HTMLSelectElement>(`[data-access="${CSS.escape(profile.id)}"]`);
            if (!control || !current) return;
            const open = control.closest(".agent-profile-detail")?.querySelector<HTMLButtonElement>(".agent-profile-chat-open"); if (open) open.disabled = !!opening || pending;
            control.value = current.access; control.disabled = false;
            const route = control.closest(".agent-profile-route");
            const note = route?.querySelector(".agent-profile-access-note"); if (note) note.textContent = about(current.access);
            const error = route?.querySelector<HTMLElement>(".agent-profile-access-error");
            if (error) { error.textContent = accessErrors.get(profile.id) ?? ""; error.hidden = !accessErrors.has(profile.id); }
          });
        });
        const route = el("section", { class: "agent-profile-route" },

          el("details", { class: "agent-profile-scope" }, el("summary", { text: t("Source") }), el("dl", {},
            el("dt", { text: t("Recipient") }), el("dd", { text: profile.built_in ? t(profile.title) : profile.title }),
            el("dt", { text: t("Executor") }), el("dd", { text: t("Claude Code · isolated") }),
            el("dt", { text: t("Source") }), el("dd", { text: t("This profile and its pinned skills") }),
            el("dt", { text: t("Project and flock context") }), el("dd", { text: t("None") }),
            el("dt", { text: t("Zeca memory") }), el("dd", { text: t("Never shared") }))),
          el("label", {}, el("span", { text: t("Access for this profile") }), access),
          accessAbout, accessError,
          el("details", { class: "agent-profile-boundary" }, el("summary", { text: t("About this chat") }), el("p", { class: "note", text: t("This chat uses the pinned profile in an isolated Claude Code session. It has no project or flock context and never uses Zeca's memory.") })));
        detail.append(route,
          el("details", { class: "agent-profile-instructions" }, el("summary", { text: t("Instructions") }), el("p", { text: profile.built_in ? t(profile.instructions) : profile.instructions })));
        if (profile.skills.length) detail.append(el("details", { class: "agent-profile-skills" }, el("summary", { text: t("Pinned skills") }),
          ...profile.skills.map(skill => el("section", {}, el("h4", { text: `${t(skill.title)} · ${t("Revision {revision}", { revision: skill.revision })}` }), el("p", { text: t(skill.content) })))));

      }
      if (profile && save) {
        const edit = el("button", { class: "btn agent-profile-edit",
          text: profile.built_in ? t("Create a copy") : t("Edit profile"), onclick: () => begin(profile) });
        edit.disabled = pending; detail.append(edit);
      }
      if (profile) {
        const identity = el("div", { class: "agent-profile-identity" });
        const heading = detail.querySelector("h3"); if (heading) identity.append(heading);
        const revision = detail.querySelector(".note"); if (revision) identity.append(revision);
        const actions = el("div", { class: "agent-profile-detail-actions" });
        const open = detail.querySelector(".agent-profile-chat-open"); if (open) actions.append(open);
        const edit = detail.querySelector(".agent-profile-edit"); if (edit) actions.append(edit);
        identity.append(actions);
        const error = detail.querySelector(".agent-profile-open-error"); if (error) identity.append(error);
        detail.prepend(el("div", { class: "agent-profile-hero" }, avatar(profile.built_in ? t(profile.title) : profile.title, profile.species, profile.look), identity));
      }
      if (!profile) detail.append(el("p", { text: t("Choose a profile to read its instructions.") }));
      if (profile) for (const disclosure of detail.querySelectorAll<HTMLDetailsElement>(".agent-profile-scope, .agent-profile-instructions, .agent-profile-skills")) {
        const key = `${profile.id}:${disclosure.className}`;
        disclosure.open = expanded.has(key);
        disclosure.addEventListener("toggle", () => { if (disclosure.open) expanded.add(key); else expanded.delete(key); });
      }
      content.append(list, detail);
    }
    if (failed) content.append(viewState("error", t("Could not load {name}.", { name: t("Agent profiles") }), undefined,
      el("button", { class: "btn", text: t("Try again"), onclick: () => refresh() })));
    else if (profiles === null) content.append(viewState("loading", t("Loading {name}…", { name: t("Agent profiles") })));
    element.replaceChildren(heading, content);
    motion();
  };
  const refresh = (returnFocus = false) => {
    if (!active || pending) return;
    const mine = ++asked;
    pending = true; failed = false; draw();
    void load().then(value => {
      if (mine !== asked) return;
      profiles = value; value.forEach(profile => changed?.(profile)); pending = false; draw();
      if (returnFocus && active && !draft) (element.querySelector<HTMLElement>(".agent-profile-edit") ?? element.querySelector<HTMLElement>(".agent-profile-create"))?.focus({ preventScroll: true });
    }, () => { if (mine === asked) { pending = false; failed = true; draw(); } });
  };
  return { element, relabel: draw,
    updateAccess(id: string, access: ChatAccess) {
      const profile = profiles?.find(item => item.id === id);
      if (profile) { profile.access = access; changed?.(profile); if (active && !draft) draw(); }
    },
    select(id: string) { selected = id; draw(); },
    visible(on: boolean) {
      const entering = on && !active;
      active = on;
      motion();
      if (!on) { asked++; pending = false; }
      else if (entering && !draft && !failed) refresh();
    },
  };
}
