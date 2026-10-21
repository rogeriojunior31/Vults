import type { VoiceRealtimeSettings } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { button, row } from "./pieces";

export interface RealtimeSettingsBackend {
  load(): Promise<VoiceRealtimeSettings>;
  save(on: boolean, limitMicrousd: string, model: string): Promise<void>;
}
const models = ["gpt-realtime-2.1-mini", "gpt-realtime-2.1"];
const same = (a: VoiceRealtimeSettings | null, b: VoiceRealtimeSettings) =>
  a?.on === b.on && a.model === b.model && a.limitMicrousd === b.limitMicrousd;
function dollars(amount: string): string {
  const value = BigInt(amount);
  return `${value / 1_000_000n}.${String(value % 1_000_000n).padStart(6, "0")}`;
}
function microdollars(text: string): string | null {
  if (!/^\d{1,13}(?:[.,]\d{1,6})?$/.test(text)) return null;
  const [whole, fraction = ""] = text.replace(",", ".").split(".");
  const value = BigInt(whole) * 1_000_000n + BigInt(fraction.padEnd(6, "0"));
  return value <= 9223372036854775807n ? String(value) : null;
}

export function createRealtimeSettings(api: RealtimeSettingsBackend, configureKey: () => void) {
  const element = el("section", { class: "card rows realtime-settings" });
  element.dataset.setting = "realtime-settings";
  const on = el("input"); on.type = "checkbox"; on.className = "toggle";
  const model = el("select", { class: "field select" });
  const budget = el("input"); budget.type = "text"; budget.inputMode = "decimal"; budget.className = "field realtime-budget";
  on.dataset.realtimeField = "on"; model.dataset.realtimeField = "model"; budget.dataset.realtimeField = "budget";
  const notice = el("p", { class: "setting-feedback" });
  const save = button("", () => void submit(), true), reload = button("", () => void refresh(true));
  const key = button("", configureKey);
  let saved: VoiceRealtimeSettings | null = null;
  let visible = false, loading = false, saving = false, dirty = false, conflict = false, epoch = 0;
  let message: "saved" | "load-failed" | "save-failed" | null = null;
  function fill(value: VoiceRealtimeSettings) {
    on.checked = value.on;
    model.replaceChildren(...[...new Set([...models, value.model])].map(id => {
      const option = el("option", { text: id }); option.value = id; return option;
    }));
    model.value = value.model; budget.value = dollars(value.limitMicrousd);
    dirty = false; conflict = false;
  }
  function draw() {
    const focused = element.contains(document.activeElement) ? document.activeElement as HTMLElement : null;
    const limit = microdollars(budget.value);
    const valid = limit !== null && models.includes(model.value) && (!on.checked || limit !== "0");
    for (const control of [on, model, budget]) control.disabled = !saved || loading || saving;
    save.textContent = t("Save"); (save as HTMLButtonElement).disabled = !saved || loading || saving || !dirty || conflict || message === "load-failed" || !valid;
    reload.textContent = dirty ? t("Discard edits and reload") : t("Reload saved settings");
    (reload as HTMLButtonElement).disabled = loading || saving;
    key.textContent = t("Configure OpenAI key");
    const error = conflict || message === "load-failed" || message === "save-failed" || dirty && !valid;
    notice.textContent = saving ? t("Saving…") : loading ? t("Loading…")
      : conflict ? t("Voice settings changed elsewhere. Reload before saving your changes.")
      : message === "load-failed" ? t("Could not read voice settings. Reload to try again.")
      : message === "save-failed" ? t("Could not confirm the change. Reload saved settings or try again.")
      : dirty && !valid ? t("Choose a supported model and a valid USD limit with up to six decimal places. Enabling voice requires a positive limit.")
      : message === "saved" ? t("Saved.") : "";
    notice.hidden = !notice.textContent; notice.classList.toggle("error", error);
    notice.setAttribute("role", error ? "alert" : "status");
    element.setAttribute("aria-busy", String(loading || saving));
    element.replaceChildren(
      row(t("Enable voice conversation"), t("Off by default. When you start a conversation, microphone audio and cached flock context go to OpenAI. API charges apply."), on),
      row(t("Model"), t("This choice is separate from your Chat model and CLI subscription."), model),
      row(t("Budget (USD / 24 hours)"), t("A rolling limit for realtime voice. Unsettled requests retain their reserved cost. Zero disables remote voice."), budget, t("Use a decimal point or comma; do not use thousands separators.")),
      el("p", { class: "hint", text: t("Voice never answers a permission card. Codex consultation is unavailable.") }),
      el("p", { class: "hint", text: t("Saving does not start recording. Open Voice conversation under Work to record a turn.") }),
      el("div", { class: "actions" }, save, reload, key), notice,
    );
    if (focused?.isConnected) focused.focus({ preventScroll: true });
  }
  async function refresh(discard = false) {
    if (!visible || saving) return;
    const mine = ++epoch; loading = true; draw();
    try {
      const value = await api.load();
      if (mine !== epoch || !visible) return;
      if (dirty && !discard) conflict ||= !same(saved, value);
      else fill(value);
      saved = value; message = null;
    } catch {
      if (mine === epoch && visible) message = "load-failed";
    } finally {
      if (mine === epoch) { loading = false; draw(); }
    }
  }
  async function submit() {
    if (!visible || (save as HTMLButtonElement).disabled) return;
    const limit = microdollars(budget.value);
    if (limit === null) return;
    const next = { on: on.checked, model: model.value, limitMicrousd: limit };
    ++epoch; saving = true; message = null; draw();
    try {
      await api.save(next.on, next.limitMicrousd, next.model);
      const actual = await api.load();
      saved = actual;
      if (same(next, actual)) { fill(actual); message = "saved"; }
      else { dirty = true; conflict = true; }
    } catch { message = "save-failed"; }
    finally { saving = false; draw(); }
  }
  for (const control of [on, model, budget]) control.addEventListener(control === budget ? "input" : "change", () => {
    dirty = true; message = null; draw();
  });
  draw();
  return {
    element, relabel: draw,
    changed() { if (visible && !saving) void refresh(); },
    visible(value: boolean) { if (visible === value) return; visible = value; ++epoch; loading = false; if (value) void refresh(); },
  };
}
