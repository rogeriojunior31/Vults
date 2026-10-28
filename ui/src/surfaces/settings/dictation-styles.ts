import type { ApiProvider, DictationOutputLanguage, DictationStyle, SettingsChange } from "../../bridge";
import { OUTPUT_LANGUAGES, STYLES, styleCards, type CleanupSettings } from "../../dictation-styles";
import { el } from "../../dom";
import { t } from "../../i18n";
import { button, row } from "./pieces";

export interface CleanupBackend {
  load(): Promise<CleanupSettings>;
  providers(): Promise<{ selected: string; providers: ApiProvider[] }>;
  apps(): Promise<Record<string, string>>;
  enable(on: boolean): Promise<void>;
  options(style: DictationStyle, writeIn: DictationOutputLanguage | null, english: boolean): Promise<void>;
  app(identity: string, style: DictationStyle | null): Promise<void>;
}
const PRESETS: Record<string, string> = { konsole: "Konsole", code: "VS Code", thunderbird: "Thunderbird", firefox: "Firefox", slack: "Slack", obsidian: "Obsidian" };
export function createCleanupSettings(api: CleanupBackend, providerSettings: () => void) {
  const element = el("section", { class: "dictation-cleanup-settings" });
  element.tabIndex = -1;
  element.dataset.setting = "cleanup-section";
  let data: CleanupSettings | null = null, provider: ApiProvider | null = null, apps: Record<string, string> | null = null;
  let visible = false, loading = false, saving = false, failed = false, providerFailed = false, appsFailed = false, error = false;
  const fieldRevision: Partial<Record<keyof CleanupSettings, number>> = {};
  let epoch = 0, changes: Partial<CleanupSettings> = {}, revision = 0;
  let message: "saved" | "failed" | "budget" | "refresh" | null = null;
  let messageScope = "settings";
  let returnFocus: string | null = null;
  let processingExpanded = false;
  let expanded = true, draft = "", draftStyle: DictationStyle = "natural";
  const select = <T extends string | null>(choices: { value: T; label: string }[], value: T, key: string, change: (value: T) => void) => {
    const control = el("select", { class: "field" }); control.dataset.styleControl = key;
    choices.forEach((choice, i) => { const option = el("option", { text: choice.label }); option.value = String(i); option.selected = choice.value === value; control.append(option); });
    control.addEventListener("change", () => { const choice = choices[Number(control.value)]; if (choice) change(choice.value); });
    return control;
  };
  const check = (value: boolean, key: string, change: (value: boolean) => void) => {
    const input = el("input", { class: "toggle" }); input.type = "checkbox"; input.checked = value; input.dataset.styleControl = key;
    input.addEventListener("change", () => change(input.checked)); return input;
  };
  const write = async (operation: () => Promise<void>, update?: () => Partial<CleanupSettings>, scope = "settings") => {
    if (saving || !data) return;
    messageScope = scope;
    returnFocus = element.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.styleControl ?? null : null;
    epoch++; loading = false; saving = true; message = null; error = false;
    const before = revision; draw();
    try {
      await operation();
      const patch = update?.();
      if (patch && data) for (const field of Object.keys(patch) as (keyof CleanupSettings)[]) {
        if ((fieldRevision[field] ?? 0) <= before) data = { ...data, [field]: patch[field] };
      }
      if (message === null) message = "saved";
    }
    catch (cause) { message = String(cause).includes("budget") ? "budget" : "failed"; error = true; }
    finally {
      saving = false; draw();
      if (returnFocus && visible && element.isConnected && (document.activeElement === document.body || element.contains(document.activeElement))) {
        const controls = [...element.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLButtonElement>("[data-style-control]")];
        const requested = controls.find(node => node.dataset.styleControl === returnFocus && !node.disabled);
        const next = requested ?? controls.find(node => node.dataset.styleControl === (returnFocus === "app-save" ? "app-name" : "cleanup"));
        next?.focus({ preventScroll: returnFocus !== "app-save" });
      }
      returnFocus = null;
    }
  };
  const options = (change: Partial<CleanupSettings>) => {
    if (!data) return;
    const next = { ...data, ...change };
    void write(() => api.options(next.dictationStyle, next.dictationWriteIn, next.dictationSessionsEnglish), () => change);
  };
  const app = (identity: string, style: DictationStyle | null, scope = `app-${identity}`) => void write(async () => {
    await api.app(identity, style);
    try { apps = await api.apps(); appsFailed = false; draft = ""; }
    catch { appsFailed = true; message = "refresh"; }
  }, undefined, scope);
  const refresh = async () => {
    if (saving) return;
    const mine = ++epoch;
    loading = true; changes = {}; message = null; error = false; draw();
    const results = await Promise.allSettled([api.load(), api.providers(), api.apps()]);
    if (mine !== epoch) return;
    const [settings, providers, entries] = results;
    failed = settings.status === "rejected";
    if (settings.status === "fulfilled") data = { ...settings.value, ...changes };
    providerFailed = providers.status === "rejected";
    provider = providers.status === "fulfilled" ? providers.value.providers.find(p => p.id === providers.value.selected) ?? null : null;
    appsFailed = entries.status === "rejected";
    if (entries.status === "fulfilled") apps = entries.value;
    loading = false; draw();
  };
  function draw() {
    const parent = element.parentElement, scroll = parent?.scrollTop;
    const active = document.activeElement as HTMLElement | null;
    const key = active && element.contains(active) ? active.dataset.styleControl : null;
    const ready = !!data && !failed;
    const on = data?.dictationCleanup ?? false;
    const available = !!provider && (provider.local || provider.hasKey);
    const toggle = check(on, "cleanup", value => void write(() => api.enable(value), () => ({ dictationCleanup: value })));
    toggle.disabled = saving || !data || (!on && (!ready || !available || loading));
    const notice = el("p", { class: `note${error || failed ? " error" : ""}`, text: saving ? t("Saving…") : message === "saved" ? t("Saved.") : message === "budget"
      ? t("Clean-up cannot verify its saved request budget. Local dictation still works.") : message === "failed" ? t("Could not confirm the change. Reload saved settings or try again.")
      : message === "refresh" ? t("The style was saved, but the app list could not refresh. Reload to check.") : failed ? t("Could not read clean-up settings. Reload to try again.") : loading ? t("Reading clean-up settings…") : "" });
    notice.hidden = !notice.textContent || (on && messageScope !== "settings"); notice.setAttribute("role", error || failed ? "alert" : "status");
    const reload = button(t("Reload clean-up settings"), () => void refresh()); reload.dataset.styleControl = "reload"; (reload as HTMLButtonElement).disabled = loading || saving;
    const destination = provider ? t("Text and spelling/style context go to {provider} ({model}). No audio is sent.", { provider: provider.label, model: provider.model }) : t("Choose an API provider in Chat & voice before enabling clean-up.");
    const providerLink = button(t("Chat & voice"), providerSettings); providerLink.dataset.styleControl = "provider";
    const processing = el("details", { class: "cleanup-processing" }, el("summary", { text: t("Processing and costs") }),
      el("p", { class: "hint", text: t("Off by default. Up to 2,000 attempts in any rolling 24 hours, including local calls. Provider charges may apply. Failure keeps your local text.") }),
      el("p", { class: "hint", text: provider?.local ? t("The selected provider uses a local endpoint on this computer.") : t("With a cloud provider, dictated text leaves this computer when clean-up is enabled.") }));
    processing.dataset.setting = "cleanup-processing"; processing.open = processingExpanded;
    processing.addEventListener("toggle", () => { processingExpanded = processing.open; });
    element.replaceChildren(el("section", { class: "card rows cleanup-activation" },
      row(t("Clean up with a model"), t("Apply your chosen writing style after local transcription."), toggle),
      el("div", { class: "cleanup-provider" }, el("p", { class: "note", text: destination }), providerLink),
      ...(!available || providerFailed ? [el("p", { class: "note error", text: providerFailed ? t("Could not read the selected provider. Reload to try again.") : t("Save an API key or choose a local provider in Chat & voice.") })] : []),
      processing, notice, ...(!loading && (failed || providerFailed || appsFailed || message === "failed" || message === "budget" || message === "refresh") ? [reload] : [])),
    );
    if (!on) {
      const preview = el("section", { class: "dictation-style-preview" }, el("h3", { text: t("Styles") }),
        el("p", { class: "hint", text: t("Clean-up is off. Every line keeps its local text.") }), styleCards(true));
      preview.setAttribute("aria-label", t("Styles"));
      element.append(preview);
    }
    if (on && data) {
      const choices = el("section", { class: "card rows" },
        row(t("Write in"), t("Keep the spoken language or translate the final text. As spoken never calls a model or translates."), select([{ value: null, label: t("The spoken language") }, ...OUTPUT_LANGUAGES.map(l => ({ value: l.id as DictationOutputLanguage | null, label: t(l.name) }))], data.dictationWriteIn, "language", value => options({ dictationWriteIn: value }))),
        row(t("Agent sessions in English"), t("Use English and Prompt only for recognized flock sessions. Other apps keep your Write in choice."), check(data.dictationSessionsEnglish, "english", value => options({ dictationSessionsEnglish: value }))),
        el("p", { class: "hint", text: t("Choices apply to the next recording. Questions stay questions; the model never answers or runs your words.") }),
        el("p", { class: "hint", text: t("Timeouts keep local text: four seconds normally, eight for translation, twelve for Prompt. Turning clean-up off cancels filtering.") }));
      const defaults = el("section", { class: "dictation-style-defaults" }, el("h3", { text: t("Default style") }), el("p", { class: "hint style-default-scope", text: t("Used only for apps without an app rule or built-in default. Recognized agent sessions always use Prompt.") }), styleCards(true, data.dictationStyle, style => options({ dictationStyle: style })), el("p", { class: "hint", text: t("The examples illustrate each style; they are not generated previews.") }));
      defaults.setAttribute("aria-label", t("Default style"));
      const details = el("details", { class: "settings-disclosure dictation-app-styles" }, el("summary", { text: t("Style per app") }));
      details.dataset.setting = "Style per app"; details.dataset.styleControl = "apps"; details.open = expanded;
      details.addEventListener("toggle", () => { expanded = details.open; });
      const body = el("div", { class: "disclosure-body" }, el("p", { class: "hint", text: t("Terminals and editors default to As spoken, mail to Polished and chats to Natural. App choices override these defaults; recognized flock sessions keep Prompt. Missing focus keeps local text.") }));
      if (appsFailed) body.append(el("p", { class: "note error", text: t("Could not load app styles. The last list may be out of date. Reload to try again.") }));
      const appChoices = [{ value: null, label: t("Use app default") }, ...STYLES.map(s => ({ value: s.id as DictationStyle | null, label: t(s.name) }))];
      const appIds = [...new Set([...Object.keys(PRESETS), ...Object.keys(apps ?? {})])];
      for (const id of appIds) {
        const selected = STYLES.find(s => s.id === apps?.[id])?.id ?? null;
        const control = select(appChoices, selected, `app-${id}`, value => app(id, value));
        control.disabled = saving || appsFailed || !apps;
        const effective = selected ?? (id === "konsole" || id === "code" || id === "obsidian" ? "verbatim" : id === "thunderbird" ? "polished" : id === "slack" ? "natural" : data.dictationStyle);
        const appRow = row(PRESETS[id] ?? id, t("Effective style: {style}", { style: t(STYLES.find(style => style.id === effective)!.name) }), control);
        if (messageScope === `app-${id}` && notice.textContent) {
          const feedback = el("p", { class: notice.className, text: notice.textContent }); feedback.setAttribute("role", error ? "alert" : "status");
          appRow.querySelector(".row-text")!.append(feedback);
        }
        body.append(appRow);
      }
      const name = el("input", { class: "field" }); name.value = draft; name.maxLength = 1024; name.dataset.styleControl = "app-name";
      name.setAttribute("aria-label", t("Application name or desktop ID")); name.placeholder = t("Application name or desktop ID");
      name.addEventListener("input", () => { draft = name.value; add.disabled = saving || appsFailed || !draft.trim(); });
      const style = select(STYLES.map(s => ({ value: s.id, label: t(s.name) })), draftStyle, "app-style", value => { draftStyle = value; });
      style.setAttribute("aria-label", t("Style for the new app"));
      const add = button(t("Save app style"), () => { if (draft.trim()) app(draft.trim(), draftStyle, "new-app"); }) as HTMLButtonElement;
      add.dataset.styleControl = "app-save"; add.disabled = saving || !draft.trim() || appsFailed;
      body.append(el("div", { class: "cleanup-app-form" }, name, style, add));
      if ((messageScope === "new-app" || messageScope.startsWith("app-") && !appIds.includes(messageScope.slice(4))) && notice.textContent) {
        const feedback = el("p", { class: notice.className, text: notice.textContent }); feedback.setAttribute("role", error ? "alert" : "status"); body.append(feedback);
      }
      if (appsFailed || messageScope !== "settings" && error) body.append(button(t("Reload clean-up settings"), () => void refresh()));
      details.append(body);
      element.append(defaults, choices, details);
      if (saving || failed) element.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLButtonElement>("input, select, button").forEach(control => { if (control !== toggle || saving) control.disabled = true; });
    }

    element.querySelectorAll<HTMLElement>("[data-style-control]").forEach(control => { control.dataset.setting = `cleanup-${encodeURIComponent(control.dataset.styleControl!)}`; });
    if (parent && scroll !== undefined) parent.scrollTop = scroll;
    if (key && element.isConnected) [...element.querySelectorAll<HTMLElement>("[data-style-control]")].find(n => n.dataset.styleControl === key)?.focus({ preventScroll: true });
  }
  draw();
  return { element, relabel: draw, refresh: () => void refresh(), visible(on: boolean) { if (on && !visible) void refresh(); visible = on; },
    update(change: SettingsChange) {
      const patch: Partial<CleanupSettings> = {};
      if (change.dictationCleanup !== undefined) patch.dictationCleanup = change.dictationCleanup;
      if (change.dictationStyle !== undefined) patch.dictationStyle = change.dictationStyle;
      if (change.dictationWriteIn !== undefined) patch.dictationWriteIn = change.dictationWriteIn;
      if (change.dictationSessionsEnglish !== undefined) patch.dictationSessionsEnglish = change.dictationSessionsEnglish;
      if (Object.keys(patch).length) { revision++; for (const field of Object.keys(patch) as (keyof CleanupSettings)[]) fieldRevision[field] = revision; Object.assign(changes, patch); if (data) data = { ...data, ...patch }; draw(); }
      if (change.api && visible && !saving) void refresh();
    },
  };
}
