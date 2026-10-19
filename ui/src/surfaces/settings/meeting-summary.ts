import type { ApiProvider, SettingsChange } from "../../bridge";
import { el } from "../../dom";
import { t } from "../../i18n";
import { button, row } from "./pieces";

export interface MeetingSettingsBackend {
  load(): Promise<{ meetingSummary: boolean }>;
  providers(): Promise<{ selected: string; providers: ApiProvider[] }>;
  enable(on: boolean): Promise<void>;
}

export function createMeetingSettings(api: MeetingSettingsBackend, configure: () => void) {
  const element = el("section", { class: "card rows meeting-summary-settings" });
  element.dataset.setting = "meeting-summary-settings";
  const input = el("input"); input.type = "checkbox"; input.className = "toggle";
  const destination = el("p", { class: "hint" });
  const notice = el("p", { class: "setting-feedback" });
  const reload = button("", () => void refresh()), provider = button("", configure);
  let data: { meetingSummary: boolean } | null = null;
  let chosen: ApiProvider | null = null;
  let changes: Partial<{ meetingSummary: boolean }> = {};
  let visible = false, loading = false, saving = false, failed = false, providerFailed = false;
  let epoch = 0, revision = 0, pending: boolean | null = null, message: "saved" | "failed" | null = null;
  function draw() {
    const focused = element.contains(document.activeElement) ? document.activeElement as HTMLElement : null;
    const on = data?.meetingSummary ?? false;
    const available = !!chosen?.model?.trim() && (chosen.local || chosen.hasKey);
    input.checked = pending ?? on;
    input.disabled = saving || !data || (!on && (loading || failed || providerFailed || !available));
    input.setAttribute("aria-busy", String(saving));
    destination.textContent = chosen
      ? t("Transcript text goes to {provider} ({model}). No audio is sent.", { provider: chosen.label, model: chosen.model || t("Choose a model") })
      : t("Choose an API provider and model in Chat & voice before enabling meeting summaries.");
    const error = message === "failed" || failed || providerFailed;
    notice.textContent = saving ? t("Saving…") : loading ? t("Reading meeting settings…")
      : message === "failed" ? t("Could not confirm the change. Reload saved settings or try again.")
      : failed || providerFailed ? t("Could not read meeting settings or the provider. Reload to try again.")
      : message === "saved" ? t("Saved.") : "";
    notice.hidden = !notice.textContent;
    notice.classList.toggle("error", error); notice.setAttribute("role", error ? "alert" : "status");
    reload.textContent = t("Reload meeting settings"); (reload as HTMLButtonElement).disabled = loading || saving;
    provider.textContent = t("Chat & voice");
    element.replaceChildren(
      row(t("Enable meeting summaries"), t("Off by default. Explicit generation sends transcript text to the selected API chat model. Up to 20 attempts per rolling 24 hours, including local calls and failures; provider charges may apply."), input),
      destination,
      el("p", { class: "hint", text: t("Generation stops after 120 seconds or when you cancel, leave Meetings, lock the screen or change provider settings. Failure keeps your local transcript. Notes stay only on this page and never execute tasks.") }),
      el("div", { class: "actions" }, provider, reload), notice,
    );
    element.setAttribute("aria-busy", String(loading || saving));
    if (focused?.isConnected) focused.focus({ preventScroll: true });
  }
  async function refresh() {
    if (!visible || saving) return;
    const mine = ++epoch; loading = true; draw();
    const [settings, providers] = await Promise.allSettled([api.load(), api.providers()]);
    if (mine !== epoch || !visible) return;
    failed = settings.status === "rejected";
    if (settings.status === "fulfilled") data = { ...settings.value, ...changes };
    providerFailed = providers.status === "rejected";
    chosen = providers.status === "fulfilled" ? providers.value.providers.find(p => p.id === providers.value.selected) ?? null : null;
    loading = false; draw();
  }
  input.addEventListener("change", () => {
    if (saving || input.disabled) return;
    pending = input.checked; const next = pending, started = revision; saving = true; message = null; draw();
    void api.enable(next).then(() => {
      if (revision === started) { changes.meetingSummary = next; data = { meetingSummary: next }; }
      message = "saved";
    }, () => { message = "failed"; }).finally(() => { pending = null; saving = false; draw(); });
  });
  draw();
  return {
    element, relabel: draw,
    update(change: SettingsChange) {
      if (change.meetingSummary !== undefined) {
        revision++;
        changes.meetingSummary = change.meetingSummary;
        data = { meetingSummary: change.meetingSummary }; draw();
      }
      if (change.api && visible) void refresh();
    },
    visible(on: boolean) { if (visible === on) return; visible = on; ++epoch; loading = false; if (on) void refresh(); },
  };
}
