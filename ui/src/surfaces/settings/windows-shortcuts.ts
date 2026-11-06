import type { WindowsShortcuts } from "../../bridge";
import { el } from "../../dom";
import { t, tk } from "../../i18n";
import { button, row } from "./pieces";

const ACTIONS = [
  ["open", tk("Open the island")], ["next", tk("Next session")], ["previous", tk("Previous session")],
  ["mini", tk("Mini")], ["quick", tk("Quick")], ["room", tk("Room")],
  ["allow", tk("Allow")], ["deny", tk("Deny")], ["talk-cancel", tk("Cancel voice")],
  ["talk", tk("Dictation")], ["talk-command", tk("Voice command")], ["talk-again", tk("Copy last dictation")],
] as const;
const MODIFIERS = ["Ctrl+Alt", "Ctrl+Shift", "Alt+Shift", "Ctrl+Alt+Shift"];
const KEYS = ["Space", "Left", "Right", "Up", "Down", "Home", "End", "PageUp", "PageDown", "Insert",
  ...Array.from({ length: 24 }, (_, i) => `F${i + 1}`).filter(key => key !== "F12")];
type Port = { read(): Promise<WindowsShortcuts | null>; save(bindings: Record<string, string>): Promise<WindowsShortcuts> };

export function createWindowsShortcuts(port: Port, onReady: () => void = () => {}) {
  const root = el("div", { class: "windows-shortcuts" });
  let state: WindowsShortcuts | null | undefined;
  let draft: Record<string, string> = {};
  let busy = false, loading = false, readFailed = false, message = "", failed = false;
  const dirty = () => !!state && (state.conflicts.length > 0 || ACTIONS.some(([id]) => draft[id] !== state?.requested[id]));

  async function refresh() {
    if (loading || busy) return;
    loading = true; readFailed = false; render();
    try { state = await port.read(); draft = { ...state?.requested }; }
    catch { readFailed = true; }
    finally { loading = false; render(); onReady(); }
  }
  async function save() {
    if (busy || !state || !dirty()) return;
    busy = true; message = ""; failed = false; render();
    try { state = await port.save({ ...draft }); draft = { ...state.requested }; message = t("Shortcuts applied."); }
    catch (error) {
      failed = true;
      const reason = String(error);
      message = reason.includes("shortcut-conflict")
        ? t("A shortcut is already used or reserved. Your previous shortcuts are unchanged.")
        : reason.includes("shortcut-duplicate") ? t("Choose a different combination for each action.")
        : t("Could not apply shortcuts. Your previous shortcuts are unchanged. Try again.");
    } finally { busy = false; render(); }
  }
  function render(): HTMLElement {
    root.hidden = state === null;
    if (state === null) return root;
    root.replaceChildren();
    if (!state) {
      root.append(el("p", { class: readFailed ? "note error" : "note", text: readFailed
        ? t("Could not load shortcut settings. Try again.") : t("Loading shortcuts…") }));
      if (readFailed) root.append(button(t("Try again"), () => { void refresh(); }));
      return root;
    }
    root.append(el("p", { class: "hint", text: t("Windows checks conflicts with other registered global shortcuts. Local shortcuts inside other apps cannot be detected. Windows keys and typing keys are excluded to preserve system shortcuts and AltGr.") }));
    const updateButtons = () => {
      const apply = root.querySelector<HTMLButtonElement>("[data-shortcuts-apply]");
      if (apply) apply.disabled = busy || !dirty();
    };
    for (const [id, label] of ACTIONS) {
      const value = draft[id] || "", parts = value.split("+");
      const key = parts.pop() ?? "";
      const modifiers = el("select");
      modifiers.setAttribute("aria-label", t("Modifiers for {action}", { action: t(label) }));
      for (const value of MODIFIERS) { const option = el("option", { text: value }); option.value = value; modifiers.append(option); }
      modifiers.value = parts.join("+") || "Ctrl+Alt";
      modifiers.disabled = busy || !value;
      const keys = el("select");
      keys.setAttribute("aria-label", t("Key for {action}", { action: t(label) }));
      const disabled = el("option", { text: t("Disabled") }); disabled.value = ""; keys.append(disabled);
      for (const key of KEYS) { const option = el("option", { text: key }); option.value = key; keys.append(option); }
      keys.value = key; keys.disabled = busy;
      const changed = () => { draft[id] = keys.value ? `${modifiers.value}+${keys.value}` : ""; modifiers.disabled = busy || !keys.value; message = ""; root.querySelector(".shortcut-feedback")?.remove(); updateButtons(); };
      modifiers.addEventListener("change", changed); keys.addEventListener("change", changed);
      const active = state.active[id];
      const status = active ? t("Active: {key}", { key: active }) : state.requested[id]
        ? t("Already used or unavailable") : t("Disabled");
      root.append(row(t(label), status, el("div", { class: "shortcut-controls" }, modifiers, keys)));
    }
    const apply = button(busy ? t("Applying shortcuts…") : t("Apply shortcuts"), () => { void save(); }, true);
    apply.dataset.shortcutsApply = ""; apply.disabled = busy || !dirty();
    const restore = button(t("Restore defaults"), () => { if (!busy && state) { draft = { ...state.defaults }; message = ""; render(); } });
    restore.disabled = busy;
    root.append(el("div", { class: "shortcut-actions" }, apply, restore));
    if (message) { const note = el("p", { class: failed ? "note error shortcut-feedback" : "note shortcut-feedback", text: message }); note.setAttribute("role", failed ? "alert" : "status"); root.append(note); }
    return root;
  }
  return { render, refresh, supported: () => !!state, desktopEditor: () => state === null, initialize: () => { void refresh(); } };
}
