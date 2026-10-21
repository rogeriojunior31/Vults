import type { AgentProfile, AgentProfileEdit } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { SPECIES, speciesSet } from "../../character/flock";
import { drawFrame, frameAt } from "../../character/sprites";
import { dress, LOOK_GROUPS } from "../../character/looks";
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

export function createProfiles(load: () => Promise<AgentProfile[]>, back: () => void, save?: (edit: AgentProfileEdit) => Promise<AgentProfile>) {
  const element = el("section", { class: "agent-profiles" });
  let profiles: AgentProfile[] | null = null;
  let selected: string | null = null;
  let failed = false;
  let active = false;
  let asked = 0;
  let pending = false;
  let draft: { profile: AgentProfile; title: string; instructions: string; species: string; look: string } | null = null;
  let saving = false;
  let saveError = "";
  const begin = (profile: AgentProfile) => {
    if (pending) return;
    draft = { profile, title: profile.built_in ? t("{name} copy", { name: t(profile.title) }) : profile.title,
      instructions: profile.built_in ? t(profile.instructions) : profile.instructions, species: profile.species, look: profile.look };
    saveError = ""; draw();
    element.querySelector<HTMLInputElement>(".agent-profile-editor input")?.focus();
  };
  const submit = () => {
    if (!draft || !save || saving) return;
    const current = draft;
    const edit: AgentProfileEdit = current.profile.built_in
      ? { action: "copy", preset: current.profile.id, title: current.title, instructions: current.instructions, species: current.species, look: current.look }
      : { action: "update", id: Number(current.profile.id.slice(7)), revision: current.profile.revision,
        title: current.title, instructions: current.instructions, species: current.species, look: current.look };
    saving = true; saveError = ""; draw();
    void save(edit).then(profile => {
      profiles = [...(profiles ?? []).filter(p => p.id !== profile.id), profile];
      selected = profile.id; draft = null; saving = false; draw();
      if (active) element.querySelector<HTMLElement>(".agent-profile-edit")?.focus({ preventScroll: true });
    }, error => {
      saving = false;
      saveError = error === "conflict" ? t("This profile changed. Cancel and reopen it before saving.")
        : error === "appearance" ? t("Choose an available species and accessory.")
        : error === "invalid" ? t("Use a name up to 160 bytes and non-empty instructions up to 32768 bytes.")
        : error === "limit" ? t("You can save up to 100 custom profiles.") : t("Could not save the profile. Your draft is preserved.");
      draw();
    });
  };
  const draw = () => {
    const title = el("h2", { text: t("Agent profiles") });
    const heading = el("header", { class: "activity-heading" },
      el("div", {}, title, el("p", { text: t("Roles and instructions. Access is controlled separately.") })),
      el("button", { class: "btn", text: t("Back to Flock"), onclick: back }));
    const content = el("div", { class: "agent-profile-content" });
    if (draft) {
      const name = el("input"); name.type = "text"; name.value = draft.title; name.required = true; name.disabled = saving;
      const instructions = el("textarea"); instructions.required = true; instructions.disabled = saving;
      instructions.value = draft.instructions;
      name.addEventListener("input", () => { if (draft) draft.title = name.value; });
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
      const image = el("div", { class: "agent-profile-preview-slot" }, preview(draft.species, draft.look));
      const updateAppearance = () => {
        if (!draft || saving) return;
        draft.species = species.value; draft.look = look.value;
        image.replaceChildren(preview(draft.species, draft.look));
      };
      species.addEventListener("change", updateAppearance); look.addEventListener("change", updateAppearance);
      const appearance = el("fieldset", { class: "agent-profile-appearance" }, el("legend", { text: t("Appearance") }),
        el("div", { class: "agent-profile-appearance-grid" }, image, el("div", {},
          el("label", {}, el("span", { text: t("Species") }), species),
          el("label", {}, el("span", { text: t("Accessory") }), look))));
      const saveButton = el("button", { class: "btn primary", text: saving ? t("Saving…") : t("Save profile") });
      saveButton.type = "submit"; saveButton.disabled = saving;
      const cancel = el("button", { class: "btn", text: t("Cancel"), onclick: () => {
        draft = null; saveError = ""; draw(); refresh(true);
      } });
      cancel.type = "button"; cancel.disabled = saving;
      const form = el("form", { class: "agent-profile-editor" },
        el("h3", { text: draft.profile.built_in ? t("Create a custom profile") : t("Edit profile") }),
        el("label", {}, el("span", { text: t("Profile name") }), name),
        appearance,
        el("label", {}, el("span", { text: t("Instructions") }), instructions),
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
        } }, canvas, el("span", {}, el("strong", { text: profile.built_in ? t(profile.title) : profile.title }), el("span", { text: t(profile.about) })));
        button.dataset.profile = profile.id;
        button.setAttribute("aria-pressed", String(selected === profile.id));
        list.append(button);
      }
      const profile = profiles.find(profile => profile.id === selected);
      const detail = el("section", { class: "agent-profile-detail" });
      detail.setAttribute("aria-live", "polite");
      if (profile) detail.append(el("h3", { text: profile.built_in ? t(profile.title) : profile.title }),
        el("p", { class: "note", text: profile.built_in ? t("Built-in profile · Revision {revision}", { revision: profile.revision }) : t("Custom profile · Revision {revision}", { revision: profile.revision }) }),
        el("h4", { text: t("Instructions") }), el("p", { text: profile.built_in ? t(profile.instructions) : profile.instructions }));
      if (profile && save) {
        const edit = el("button", { class: "btn agent-profile-edit",
          text: profile.built_in ? t("Create a copy") : t("Edit profile"), onclick: () => begin(profile) });
        edit.disabled = pending; detail.append(edit);
      }
      if (!profile) detail.append(el("p", { text: t("Choose a profile to read its instructions.") }));
      content.append(list, detail);
    }
    if (failed) content.append(viewState("error", t("Could not load {name}.", { name: t("Agent profiles") }), undefined,
      el("button", { class: "btn", text: t("Try again"), onclick: () => refresh() })));
    else if (profiles === null) content.append(viewState("loading", t("Loading {name}…", { name: t("Agent profiles") })));
    element.replaceChildren(heading, content);
  };
  const refresh = (returnFocus = false) => {
    if (!active || pending) return;
    const mine = ++asked;
    pending = true; failed = false; draw();
    void load().then(value => {
      if (mine !== asked) return;
      profiles = value; pending = false; draw();
      if (returnFocus && active && !draft) element.querySelector<HTMLElement>(".agent-profile-edit")?.focus({ preventScroll: true });
    }, () => { if (mine === asked) { pending = false; failed = true; draw(); } });
  };
  return { element, relabel: draw, visible(on: boolean) {
    active = on;
    if (!on) { asked++; pending = false; }
    else if (profiles === null && !failed) refresh();
  } };
}
