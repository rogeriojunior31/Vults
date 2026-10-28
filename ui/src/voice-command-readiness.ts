import type { VoiceStatus } from "./bridge";
import { el } from "./dom";
import { t, tk } from "./i18n";

export const VOICE_MODEL_LABELS: Record<string, string> = {
  base: tk("Base: fast, but weak outside English"),
  small: tk("Small: good in Portuguese and other languages"),
  turbo: tk("Large v3 Turbo: the most accurate; best with a GPU"),
};

export type CommandSettings = {
  dictationCommands: boolean;
  dictationCommandMedia: boolean;
  dictationCommandApps: boolean;
  dictationCommandOutput: boolean;
  nowPlaying: boolean;
  dictationMicrophone: string | null;
};
export type CommandReadiness = {
  voice: VoiceStatus | null;
  settings: CommandSettings | null;
  microphones: string[] | null;
};

/** Inventory only: discovering an input does not verify capture or native action availability. */
export function createCommandReadiness(
  read: () => Promise<CommandReadiness>,
  settings: (section: string) => void,
) {
  const element = el("details", { class: "voice-command-readiness" });
  element.dataset.setting = "command-readiness";
  const title = el("summary");
  title.dataset.setting = "command-readiness-summary";
  const body = el("div");
  const refresh = el("button", { onclick: () => void load() });
  refresh.dataset.setting = "command-readiness-refresh";
  const cancel = el("button", { onclick: () => {
    if (!active || !loading) return;
    epoch++; loading = false; failed = false; changed = false; cancelled = true; snapshot = null; paint();
  } });
  cancel.dataset.setting = "command-readiness-cancel";
  const feedback = el("p");
  feedback.setAttribute("role", "status");
  const keys = el("dl");
  const inventory = el("dl");
  const links = el("div", { class: "voice-command-search" });
  const about = el("p");
  let bindings: Record<string, string> = {};
  let snapshot: CommandReadiness | null = null;
  let active = false, loading = false, failed = false, changed = false, cancelled = false, epoch = 0;
  const link = (label: string, section: string) => {
    const button = el("button", { text: label, onclick: () => settings(section) });
    button.dataset.setting = `command-readiness-${section}`;
    return button;
  };
  const voiceLink = link("", "dictationvoice"), keyLink = link("", "dictationkeys"), diagnosticLink = link("", "dictationdiagnostics");
  links.append(voiceLink, keyLink, diagnosticLink);
  const entry = (label: string, value: string) => [el("dt", { text: label }), el("dd", { text: value })];
  const paint = () => {
    const cancelFocused = document.activeElement === cancel;
    title.textContent = t("Command setup");
    about.textContent = t("Speech is transcribed first, then matched to local commands. Dictation uses its own shortcut.");
    keys.replaceChildren(...entry(t("Voice command"), bindings["talk-command"]?.trim() || t("Not bound")),
      ...entry(t("Dictation"), bindings.talk?.trim() || t("Not bound")));
    const voice = snapshot?.voice, prefs = snapshot?.settings;
    const model = voice?.models.find(model => model.id === voice.selected);
    const language = voice?.language === "auto" ? t("Detect it each time") : voice?.language ?? voice?.system ?? t("Unknown");
    const inputs = snapshot?.microphones;
    const microphone = inputs === null || inputs === undefined ? t("Microphone availability is unknown.")
      : inputs.length === 0 ? t("No microphones found. Connect one and refresh.")
      : prefs?.dictationMicrophone && !inputs.includes(prefs.dictationMicrophone)
        ? t("The preferred microphone is missing. Capture may use another input.")
        : t("An input device was found. Test capture in Diagnostics.");
    const enabled = (value: boolean | undefined) => value === undefined ? t("Unknown") : value ? t("On") : t("Off");
    inventory.replaceChildren(
      ...entry(t("Voice model and language"), !voice ? t("Unknown") : voice.ready && model?.installed ? t(VOICE_MODEL_LABELS[model.id] ?? model.id) : t("Choose an installed speech model in Voice.")),
      ...entry(t("Language you speak"), language),
      ...entry(t("Microphone"), microphone),
      ...entry(t("Enable voice commands"), enabled(prefs?.dictationCommands)),
      ...entry(t("Media voice commands"), !prefs ? t("Unknown") : !prefs.dictationCommandMedia ? t("Off") : prefs.nowPlaying ? t("On") : t("Media controls must also be on.")),
      ...entry(t("Application voice commands"), enabled(prefs?.dictationCommandApps)),
      ...entry(t("System audio voice commands"), enabled(prefs?.dictationCommandOutput)),
    );
    inventory.hidden = !snapshot;
    feedback.textContent = cancelled ? t("Check cancelled. You can check again.") : changed ? t("Command setup changed. Check again.") : loading ? t("Checking command setup…") : failed ? t("Could not read command setup. Check again.")
      : !snapshot ? t("Check the speech model, input devices and local-action switches without recording.")
        : t("Setup snapshot only. Capture and desktop actions still need a real test.");
    refresh.textContent = t("Check command setup");
    refresh.disabled = loading;
    cancel.textContent = t("Cancel setup check");
    cancel.hidden = !loading;
    inventory.setAttribute("aria-busy", String(loading));
    if (cancelFocused && !loading && active) refresh.focus({ preventScroll: true });
    voiceLink.textContent = t("Voice"); keyLink.textContent = t("Keyboard shortcuts"); diagnosticLink.textContent = t("Diagnostics");
  };
  async function load() {
    if (loading || !active) return;
    const generation = ++epoch;
    loading = true; failed = false; changed = false; cancelled = false; paint();
    try {
      const result = await read();
      if (generation !== epoch || !active) return;
      snapshot = result;
    } catch {
      if (generation !== epoch || !active) return;
      failed = true;
      snapshot = null;
    } finally {
      if (generation === epoch && active) { loading = false; paint(); }
    }
  }
  element.append(title, body);
  body.append(about, keys, feedback, inventory, el("div", { class: "voice-command-actions" }, refresh, cancel), links);
  paint();
  return {
    element, relabel: paint,
    invalidate() {
      const hadReading = loading || snapshot !== null;
      epoch++; loading = false; snapshot = null; failed = false; cancelled = false;
      changed = changed || hadReading; paint();
    },
    setKeys(value: Record<string, string>) { bindings = value; paint(); },
    visible(on: boolean) {
      if (on === active) return;
      active = on;
      if (!on) { epoch++; loading = false; snapshot = null; failed = false; changed = false; cancelled = false; paint(); }
    },
  };
}
