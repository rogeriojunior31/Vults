import { createDictationNavigation, DICTATION_SETTINGS, dictationSetting, type DictationSetting } from "../nest/dictation/navigation";
import { voiceCommandHelp } from "../../voice-command-help";
import { dictationStatus } from "../../island/dictation";
import { sidePanel } from "../nest/panels";
import { applyTheme } from "../../theme";
import { ISLAND_WIDTHS, restoreWidth, saveWidth, watchWidth } from "../../island/geometry";
// Settings, a webview of the Nest's window (ADR 0021): a sidebar and one page per section. Installing hooks always goes through a
// diff the user reviews first.
import { getVersion } from "@tauri-apps/api/app";
import { Bridge, type WebEngine, type WebSettings, type ApiProvider, type ConnectorStatus, type Flock, type InstallAgent, type InstallPreview, type InstallStatus, type Corner, type Presence, type ProjectPrefs, type Rule, type VoiceStatus, type ChatAccess, type DictationPermission, type DictationMusic } from "../../bridge";
import { SPECIES, speciesSet } from "../../character/flock";
import { LOOK_GROUPS, dress } from "../../character/looks";
import { drawFrame, frameAt, type Frame } from "../../character/sprites";
import { perchOf } from "../../character/zeca";
import { AGENTS as REGISTRY } from "../../agents";
import { CONNECTORS } from "../../connectors";
import { el } from "../../dom";
import { button, diff, disclosure, row, saveSetting, toggle } from "./pieces";
import { policiesSection, refreshPolicies } from "./policies";
import { createDoctor } from "./doctor";
import { createMeetingAssets } from "./meeting-assets";
import { createRealtimeSettings } from "./realtime";
import { createMeetingSettings } from "./meeting-summary";
import { createCleanupSettings } from "./dictation-styles";
import { createQuestionAgents } from "./question-agents";
import { createDictationDoctor } from "./dictation-doctor";
import { LANGUAGES as APP_LANGUAGES, locale, setLang, t, tk } from "../../i18n";
import { historyCard, type Activity, type ActivityActions } from "./activity";
import { createBudget } from "./budget";
import { phoneEvent, phonePage, refreshPhone } from "./phone";
import { Sound } from "../../sound";
import { icon, type IconName } from "../../island/icons";
import { createNestFoot, createNestHead, tabByKey } from "../nest/head";
import { needsYou } from "../nest/flock";
import type { SessionView } from "../../view.gen";

type Page = "dictation" | "general" | "agents" | "chat" | "approvals" | "projects" | "activity" | "connectors" | "phone" | "flock" | "about";

/** The sidebar's groups, in the prototype's order: how it feels, the work, the system. */
const GROUPS = [
  { id: "experience", label: tk("Experience") },
  { id: "work", label: tk("Work") },
  { id: "system", label: tk("System") },
] as const;

const PAGES: { id: Page; label: string; group: (typeof GROUPS)[number]["id"]; icon: IconName }[] = [
  { id: "general", label: tk("General"), group: "experience", icon: "gear" },
  { id: "chat", label: tk("Chat & voice"), group: "experience", icon: "chat" },
  { id: "flock", label: tk("Flock & Zeca"), group: "experience", icon: "flock" },
  { id: "agents", label: tk("Agents"), group: "work", icon: "terminal" },
  { id: "approvals", label: tk("Approvals"), group: "work", icon: "list" },
  { id: "projects", label: tk("Projects"), group: "work", icon: "folder" },
  { id: "activity", label: tk("Activity"), group: "system", icon: "bars" },
  { id: "connectors", label: tk("Connectors"), group: "system", icon: "plug" },
  { id: "phone", label: tk("Phone"), group: "system", icon: "phone" },
  { id: "about", label: tk("About"), group: "system", icon: "person" },
];

/** The agents Settings installs, from the registry; a retired one shows only while its hooks are still there, to remove
 *  them. */
const AGENTS: { kind: InstallAgent; name: string; plugin: boolean; retired: boolean }[] = REGISTRY.filter((a) => a.setup !== "none").map(
  (a) => ({ kind: a.id as InstallAgent, name: a.name, plugin: a.setup === "plugin", retired: a.setup === "remove-only" }),
);

/** What the installer writes for an agent: hooks in its config, or a plugin file of ours. */
const setupWord = (kind: InstallAgent): string => (AGENTS.find((a) => a.kind === kind)?.plugin ? "plugin" : "hooks");

interface Panel {
  status: InstallStatus | null;
  message: { text: string; error: boolean } | null;
  pending: { install: boolean; preview: InstallPreview } | null;
}

const dictation = dictationStatus(true);
const meetingAssets = createMeetingAssets({ status: Bridge.meetingAssetsStatus, review: Bridge.meetingAssetsReview,
  begin: Bridge.meetingAssetsBegin, cancel: Bridge.meetingAssetsCancel });
const realtimeSettings = createRealtimeSettings({ load: Bridge.voiceRealtimeSettings, save: Bridge.setVoiceRealtime }, () => choosePage("chat"));
const meetingSettings = createMeetingSettings({ load: Bridge.appSettings, providers: Bridge.apiProviders, enable: Bridge.setMeetingSummary }, () => choosePage("chat"));
const dictationDoctor = createDictationDoctor({ report: Bridge.dictationDoctor, start: Bridge.dictationMicStart,
  status: Bridge.dictationMicStatus, stop: Bridge.dictationMicStop });
const questionAgents = createQuestionAgents({ settings: Bridge.appSettings, set: Bridge.setQuestionAgents,
  status: Bridge.questionInstallStatus, preview: Bridge.questionInstallPreview, apply: Bridge.questionInstallApply });
const cleanupSettings = createCleanupSettings({ load: Bridge.appSettings, providers: Bridge.apiProviders, apps: Bridge.dictationStyles,
  enable: Bridge.setDictationCleanup, options: Bridge.setDictationStyleOptions, app: Bridge.setDictationAppStyle }, () => choosePage("chat"));
Bridge.onDictationStyles(() => { if (page === "dictation") cleanupSettings.refresh(); });
const root = document.getElementById("settings")!;
let boundShortcuts: Record<string, string> = {};
const boundKeys = (...ids: string[]) => ids.map(id => boundShortcuts[id]?.trim() || t("Not bound")).join(" · ");
const keepShortcuts = (keys: Record<string, string>) => {
  boundShortcuts = keys;
  if (page === "general" || page === "dictation") render();
};
Bridge.onShortcutKeys(keepShortcuts);
void Bridge.shortcutKeys().then(keepShortcuts, () => {});

const budget = createBudget({ today: Bridge.spendToday, setBudget: Bridge.budgetSet });
Bridge.onSpend(() => { if (page === "activity") budget.refresh(); });
// Settings is a webview of the Nest's window (ADR 0021), under the Nest's own header: a tab goes
// back to the Nest's webview, the size switch keeps Settings in front.
let nestFull = false;
const head = createNestHead({
  tab: (id) => { if (id !== "dictation" || page !== "dictation") void Bridge.openNest(id); },
  gear: () => choosePage("general"),
  size: (s) => void Bridge.setSize(s, "settings"),
  // The palette lists sessions and their steps: the Nest's, not Settings'.
  search: null,
  sound: (on) => {
    void Bridge.setSounds(on);
    sounds = on;
    render();
  },
});
head.set("settings");
root.before(...head.elements);
/** The session in front, for the footer's terminal: only who it is, never what it says. */
let front: Pick<SessionView, "agent" | "id"> | undefined;
const foot = createNestFoot({
  zeca: (on) => {
    void Bridge.setZeca(on).then(() => {
      zeca = on;
      render();
    });
  },
  look: () => {
    page = "flock";
    navigated = true;
    location.hash = "flock";
    render();
  },
  terminal: () => {
    if (front) void Bridge.sessionJump(front.agent, front.id);
  },
  // Back to the island is Quick: the Nest closes and the island opens (ADR 0021).
  island: () => void Bridge.setSize("quick"),
});
root.after(foot.element);
Bridge.onView((v) => {
  dictation.context(v);
  const f = v.focus;
  const s = f ? v.sessions.find((x) => x.agent === f.agent && x.id === f.id) : undefined;
  front = s ? { agent: s.agent, id: s.id } : undefined;
  foot.setTerminal(!!s && s.raise !== false);
  head.setCounts({
    sessions: v.sessions.length,
    needs: v.sessions.filter(needsYou).length,
    connections: (v.boards ?? []).reduce((n, b) => n + b.rows.filter((r) => r.group !== "releases").length, 0),
  });
});
const setNestFull = (on: boolean) => {
  nestFull = on;
  document.body.classList.toggle("full", on);
  head.setFull(on);
};
void Bridge.nestIsFull().then(setNestFull, () => {});
Bridge.onNestFull(setNestFull);
// Alt+1 to 5 as in the Nest; Esc leaves Full for Room.
window.addEventListener("keydown", (e) => {
  const pick = tabByKey(e, zeca);
  if (pick) {
    e.preventDefault();
    if (pick !== "dictation" || page !== "dictation") void Bridge.openNest(pick);
  } else if (e.key === "Escape" && nestFull && !e.defaultPrevented && !e.isComposing && !(e.target as Element | null)?.closest?.("input, textarea, select, [contenteditable]")) {
    e.preventDefault();
    void Bridge.setSize("room", "settings");
  }
});
const asked = (section: string): Page | null => dictationSetting(section) ? "dictation" : PAGES.find((p) => p.id === section)?.id ?? null;
/** The page the app asked for (the tray's Set up agents…), else General until the agents are read:
 *  Agents when one has no hooks yet or older ones (`chooseStart`). */
let focusDestination = false;
let dictationSection: DictationSetting = dictationSetting(location.hash.slice(1)) ?? "voice";
const linked = asked(location.hash.slice(1));
let page: Page = linked ?? "general";
/** The user went to a page: the start page no longer moves. */
let navigated = linked !== null;
const panels = new Map<InstallAgent, Panel>(AGENTS.map((a) => [a.kind, { status: null, message: null, pending: null }]));
let connectorStatus = new Map<string, ConnectorStatus>();
let sounds = true;
/** Percent, as the slider shows it: a repaint mid-drag keeps the drag. */
let volume = 50;
/** Percent, as last saved: a failed save goes back to it. */
let savedVolume = 50;
let nowPlaying = false;
let codexUsage = false;
let nativeChat = true;
let dictationInsert = false;
let dictationCaption = true;
let dictationCommands = true, dictationCommandChat = false;
let dictationCommandsChanged = false, dictationCommandChatChanged = false;
let dictationSound = true;
let dictationKeep = "30";
let dictationKeepChanged = false;
let dictationMicrophone: string | null = null;
let dictationMicrophoneChanged = false;
let dictationCutRepeats = true;
let dictationRetryBase = true;
let dictationVocabulary = false;
let dictationProjectVocabulary = false;
let dictationVocabularyChanged = false;
let dictationProjectVocabularyChanged = false;
let dictationCutRepeatsChanged = false;
let dictationRetryBaseChanged = false;
let dictationSpokenCode = true;
let dictationSpokenCodeChanged = false;
let dictationMusic: DictationMusic = "lower";
let dictationMusicChanged = false;
let dictationCancelByVoice = true;
let dictationCancelByVoiceChanged = false;
let microphoneSaving = false;
let microphones: string[] | null = null;
let microphonesLoading = false;
let microphonesFailed = false;
let dictationDoubleTap = true;
let dictationSpaceLock = true;
let dictationEscapeCancel = true;
let dictationDoubleTapChanged = false;
let dictationSpaceLockChanged = false;
let dictationEscapeCancelChanged = false;
let dictationInsertChanged = false;
let dictationEraseConfirm = false;
let dictationErasing = false;
let dictationEraseFailed = false;
let dictationErased = false;
let shortcutsOpening = false;
let shortcutsOpenFailed = false;
let dictationInsertAvailable = false;
const permissionErrors = new Set([
  tk("Dictation insertion is changed in Settings."),
  tk("Dictation insertion requires native X11 or Plasma Wayland."),
  tk("The keyboard permission token is unavailable."),
  tk("Insertion is off, but its saved keyboard token could not be removed. Retry turning insertion off."),
  tk("The keyboard permission worker is unavailable."),
  tk("Enable dictation insertion in Settings first."),
  tk("Dictation insertion is unavailable."),
]);
async function permissionWrite<T>(write: () => Promise<T>): Promise<T> {
  try { return await write(); } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(permissionErrors.has(message) ? t(message) : message);
  }
}
let permissionFailure: string | null = null;
let dictationWayland = false;
let permissionEvents = 0;
let dictationPermission: DictationPermission = "off";
Bridge.onDictationPermission(state => { permissionEvents++; dictationPermission = state; if (page === "dictation") render(); });
void Bridge.dictationPermission().then(state => { if (permissionEvents) return; dictationPermission = state; if (page === "dictation") render(); }, () => {});
/** Who answers the chat's cards (ADR 0004). */
let chatAccess: ChatAccess = "ask";
/** Zeca's memory (M4): the journal, and each day summed up by a small model. */
let memory = false;
let journalSummary = false;
let localEmbeddings = false;
let memorySuggest = false;
let sleep = false;
let voice: VoiceStatus | null = null;
/** Model id → percent downloaded, for every download running. */
const downloading = new Map<string, number>();
let voiceError: string | null = null;
let autostart = false;
let foldAfter = 15;
let openOnHover = false;
/** The language chosen, or null to follow the system. */
let language: string | null = null;
let theme: "black" | "light" = "black";
/** Seconds the open island waits before folding, as the settings offer them. */
const FOLD_CHOICES = [5, 10, 15, 30, 60];
let zecaSpecies = "atratus";
let zecaLook = "auto";
/** Zeca's looks, as crates/core/src/looks.rs names them. */
const LOOKS = LOOK_GROUPS.flatMap((g) => g.looks);
let flock: Flock = "brazil";
let visitors = true;
let presence: Presence = "island";
let notifications = true;
/** Do not disturb until then (epoch seconds), or off. */
let dndUntil: number | null = null;
/** The lengths Settings offers, in minutes. */
const DND_CHOICES = [30, 60, 240];
let zeca = true;
let widget: Corner | null = null;
let monitor: string | null = null;
let monitors: { name: string; label: string }[] = [];
let version = "";
let settingsPath = "";
let dataPath = "";
let rules: Rule[] = [];
let projects: Record<string, ProjectPrefs> = {};
let apiProviders: ApiProvider[] = [];
let apiSelected = "";
/** Provider id → its live model list, or why it couldn't be had. */
const apiModels = new Map<string, { models: string[] } | { error: string }>();
const apiLoading = new Set<string>();
let apiMessage: { text: string; error: boolean } | null = null;
/** Zeca's web tools (Z7), and the last thing their rows said. */
let web: WebSettings | null = null;
let webMessage: { text: string; error: boolean } | null = null;

async function refreshRules(): Promise<void> {
  try {
    rules = await Bridge.rulesList();
  } catch {
    rules = [];
  }
  if (page === "approvals") render();
}

function approvalsPage(): HTMLElement[] {
  const name = (cwd: string) => cwd.split(/[\\/]/).filter(Boolean).pop() ?? cwd;
  return [
    el("h1", { text: t("Approvals") }),
    el("p", {
      class: "lede",
      text: t("Permissions you chose to always allow, with Always on a card. Each one covers only that exact command or file, for that agent, in that folder."),
    }),
    rules.length
      ? el(
          "section",
          { class: "card rows" },
          ...rules.map((r, i) =>
            el(
              "div",
              { class: "row" },
              el(
                "div",
                { class: "row-text" },
                el("div", { class: "row-title", text: r.target }),
                el("div", { class: "row-about", text: `${AGENTS.find(a => a.kind === r.agent)?.name ?? r.agent} · ${name(r.cwd)} · ${r.cwd}` }),
              ),
              button(t("Remove"), () => {
                void Bridge.ruleRemove(i).then(refreshRules);
              }),
            ),
          ),
        )
      : el("section", { class: "card" }, el("p", { class: "note", text: t("Nothing is always allowed. Every permission asks.") })),
    ...policiesSection(render),
  ];
}

let activity: Activity | null = null;
let clearing = false;

async function refreshActivity(): Promise<void> {
  try {
    activity = await Bridge.activity(null);
  } catch {
    activity = null;
  }
  if (page === "activity") render();
}

const activityActions: Pick<ActivityActions, "history" | "confirmClear" | "clear"> = {
  history: async (on) => {
    await Bridge.setHistory(on);
    if (activity) activity = { ...activity, history: on };
  },
  confirmClear: (on) => {
    clearing = on;
    render();
  },
  clear: async () => {
    await Bridge.clearHistory();
    clearing = false;
    await refreshActivity();
  },
};

async function refreshProjects(): Promise<void> {
  try {
    projects = await Bridge.projectsList();
  } catch {
    projects = {};
  }
  if (page === "projects") render();
}

/** Mute, pin and hide, per project folder: set from a session's quick actions on the island. */
function projectsPage(): HTMLElement[] {
  const name = (cwd: string) => cwd.split(/[\\/]/).filter(Boolean).pop() ?? cwd;
  const folders = Object.keys(projects).sort((a, b) => name(a).localeCompare(name(b)));
  const set = (cwd: string, change: ProjectPrefs) => async (on: boolean) => {
    const next = { ...projects[cwd], ...Object.fromEntries(Object.keys(change).map((k) => [k, on])) };
    await Bridge.projectSet(cwd, next);
    await refreshProjects();
  };
  const choice = (label: string, on: boolean, change: (on: boolean) => Promise<void>) =>
    el("label", { class: "project-choice" }, toggle(on, change), el("span", { text: label }));
  return [
    el("h1", { text: t("Projects") }),
    el("p", {
      class: "lede",
      text: t("Choices kept per project folder, for every session in it now and later. Right-click a session on the island to mute, pin or hide its project, or to choose its bird. Muted: no sounds and no notifications when its sessions finish or fail; a card keeps both. Pinned: its sessions come first. Hidden: its sessions stay off the island, but a card from one still shows. Bird: every session of the project is that species; Automatic lets the flock draw it."),
    }),
    folders.length
      ? el(
          "section",
          { class: "card rows" },
          ...folders.map((cwd) => {
            const p = projects[cwd];
            return el(
              "div",
              { class: "row" },
              el("div", { class: "row-text" }, el("div", { class: "row-title", text: name(cwd) }), el("div", { class: "row-about", text: cwd })),
              el(
                "div",
                { class: "project-choices" },
                choice(t("Muted"), !!p.mute, set(cwd, { mute: true })),
                choice(t("Pinned"), !!p.pin, set(cwd, { pin: true })),
                choice(t("Hidden"), !!p.hide, set(cwd, { hide: true })),
                dropdown(
                  [
                    { value: null as string | null, label: t("Automatic bird") },
                    // The king vulture is a role, never a project's breed.
                    ...SPECIES.filter((x) => x.id !== "papa").map((x) => ({ value: x.id as string | null, label: x.name })),
                  ],
                  p.species ?? null,
                  async (species) => {
                    await Bridge.projectSet(cwd, { ...projects[cwd], species: species ?? undefined });
                    await refreshProjects();
                  },
                ),
                button(t("Forget"), () => {
                  void Bridge.projectSet(cwd, {})
                    .catch(() => {})
                    .finally(() => void refreshProjects());
                }),
              ),
            );
          }),
        )
      : el("section", { class: "card" }, el("p", { class: "note", text: t("No project has a choice yet. Every session shows, in the order it arrived, with its sounds.") })),
  ];
}

// ── Pieces ───────────────────────────────────────────────────────────────────


/** A row of choices, one on. */
function segmented<T>(choices: { value: T; label: string }[], current: T, change: (v: T) => Promise<void>): HTMLElement {
  const box = el("div", { class: "segmented" });
  const buttons = choices.map(c => {
    const button = el("button", { text: c.label, onclick: () => {
      if (button.getAttribute("aria-pressed") === "true" || box.getAttribute("aria-busy") === "true") return;
      const before = choices[buttons.findIndex(button => button.getAttribute("aria-pressed") === "true")]?.value ?? current;
      paint(c.value);
      saveSetting(box, () => change(c.value), () => paint(before));
    } });
    return button;
  });
  box.append(...buttons);
  const paint = (value: T) => {
    buttons.forEach((button, i) => {
      button.classList.toggle("on", choices[i].value === value);
      button.setAttribute("aria-pressed", String(choices[i].value === value));
    });
  };
  paint(current);
  return box;
}

function dropdown<T>(
  choices: { value: T; label: string; title?: string }[],
  current: T,
  change: (v: T) => Promise<void>,
): HTMLElement {
  const box = el("select", { class: "field dropdown" });
  for (const [i, c] of choices.entries()) {
    const option = el("option", { text: c.label });
    option.value = String(i);
    if (c.title) option.title = c.title;
    option.selected = c.value === current;
    box.append(option);
  }
  box.dataset.savedValue = String(choices.findIndex(c => c.value === current));
  box.addEventListener("change", () => {
    if (box.getAttribute("aria-busy") === "true") return;
    const next = choices[Number(box.value)]!.value;
    const before = box.dataset.savedValue!;
    saveSetting(box, () => change(next), () => { box.value = before; });
  });
  return box;
}

/** The volume, saved and heard once the slider is let go of. */
function slider(current: number, disabled: boolean): HTMLElement {
  const input = el("input", { class: "slider" });
  input.type = "range";
  input.min = "0";
  input.max = "100";
  input.step = "5";
  input.value = String(current);
  input.disabled = disabled;
  input.setAttribute("aria-label", t("Volume"));
  const shown = el("span", { class: "slider-value", text: `${current}%` });
  input.addEventListener("input", () => {
    volume = Number(input.value);
    shown.textContent = `${volume}%`;
  });
  input.addEventListener("change", () => {
    const percent = Number(input.value);
    void Bridge.setVolume(percent).then(
      () => {
        savedVolume = percent;
        // The island may have muted the sounds since: then the preview stays quiet too.
        if (!sounds) return;
        Sound.setVolume(percent);
        Sound.play("approval");
      },
      () => {
        volume = savedVolume;
        input.value = String(volume);
        shown.textContent = `${volume}%`;
      },
    );
  });
  return el("div", { class: "slider-box" }, input, shown);
}


function badge(text: string, kind: "ok" | "warn" | "error" | "off"): HTMLElement {
  return el("span", { class: `badge ${kind}`, text });
}




function ago(secs: number): string {
  const s = Math.max(0, Math.round(Date.now() / 1000 - secs));
  return s < 60 ? t("just now") : s < 3600 ? t("{n} min ago", { n: Math.round(s / 60) }) : t("{n} h ago", { n: Math.round(s / 3600) });
}

// ── Agents ───────────────────────────────────────────────────────────────────

async function refresh(kind: InstallAgent): Promise<void> {
  const panel = panels.get(kind)!;
  try {
    panel.status = await Bridge.installStatus(kind);
  } catch (e) {
    panel.message = { text: String(e), error: true };
    // Unreadable: that is for the Agents page to say.
    if (!navigated) {
      navigated = true;
      page = "agents";
    }
  }
  chooseStart();
  render();
}

/** Opened with no page asked for: Agents when an agent's hooks are older than this version, or
 *  none is installed yet (nothing works without them); General otherwise. An agent the user does
 *  not use is simply not installed, which is fine. Decided once every agent was read. */
function chooseStart(): void {
  if (navigated) return;
  const all = [...panels.values()].map((p) => p.status);
  if (all.some((s) => s === null)) return;
  navigated = true;
  const work = all.some((s) => s?.outdated) || all.every((s) => !s?.installed);
  if (work && page !== "agents") {
    page = "agents";
    location.hash = page;
  }
}

async function preview(kind: InstallAgent, install: boolean): Promise<void> {
  const panel = panels.get(kind)!;
  panel.message = null;
  try {
    panel.pending = { install, preview: await Bridge.installPreview(kind, install) };
  } catch (e) {
    panel.message = { text: String(e), error: true };
  }
  await refresh(kind);
}

async function apply(kind: InstallAgent): Promise<void> {
  const panel = panels.get(kind)!;
  if (!panel.pending) return;
  const { install, preview } = panel.pending;
  panel.pending = null;
  try {
    const backup = await Bridge.installApply(kind, install, preview.fingerprint);
    const plugin = setupWord(kind) === "plugin";
    const done = plugin ? (install ? t("Plugin installed.") : t("Plugin removed.")) : install ? t("Hooks installed.") : t("Hooks removed.");
    panel.message = { text: backup ? t("{done} Backup: {path}", { done, path: backup }) : done, error: false };
  } catch (e) {
    panel.message = { text: String(e), error: true };
  }
  await refresh(kind);
}

function agentStatus(s: InstallStatus): HTMLElement {
  if (s.error) return badge(t("Can't read"), "error");
  if (!s.installed) return badge(t("Not installed"), "off");
  if (s.codex?.error) return badge(t("Codex's config unreadable"), "error");
  if (s.codex?.hooksDisabled) return badge(t("Hooks off in Codex"), "error");
  if (s.codex && s.codex.untrusted > 0) return badge(t("{n} to trust", { n: s.codex.untrusted }), "warn");
  if (s.outdated) return badge(t("Update available"), "warn");
  return badge(t("Installed"), "ok");
}

function agentCard(kind: InstallAgent, name: string): HTMLElement {
  const { status: s, message, pending } = panels.get(kind)!;
  const notice = message ? el("p", { class: message.error ? "note error" : "note ok", text: message.text }) : null;
  if (!s) return el("section", { class: "card" }, el("div", { class: "card-title", text: name }), notice);

  const codexHelp = s.codex?.error
    ? el("p", { class: "note error", text: t("Vults can't read Codex's config.toml, so it can't tell which hooks Codex runs: {error}", { error: s.codex.error }) })
    : s.codex && s.installed && !s.codex.hooksDisabled && s.codex.untrusted > 0
      ? el("p", {
          class: "note warn",
          text: t("Codex runs a hook only once you trust it, and while it is on: open Codex, type /hooks and trust or turn on the {n} Vults hooks waiting there.", { n: s.codex.untrusted }),
        })
      : null;
  // Installing would be refused (`install_blocked` in installer.rs): say why instead of offering it.
  const updateHelp = kind === "gemini"
    ? null
    : s.installBlocked
    ? el("p", { class: "note warn", text: s.installBlocked })
    : s.installed && s.outdated
      ? el("p", {
          class: "note warn",
          text: s.otherHookPath
            ? t("These hooks run another copy of the hook, at {path}. Update them to use this app's own.", { path: s.otherHookPath })
            : kind === "claude"
              ? t("These hooks are from an older version. Update them to answer Claude Code's questions from the island.")
              : kind === "opencode"
                ? t("This plugin is from an older version. Update it, then restart OpenCode.")
                : t("These hooks are from an older version. Update them to get everything the island can do."),
        })
      : null;
  // The plan's usage reaches the island only through a statusLine of ours.
  const usageHelp =
    s.installed && s.statusLine === "none"
      ? el("p", {
          class: "note",
          text: t("Reinstall the hooks to see your plan's usage on the island: it adds a status line that Claude Code fills in and that shows nothing on its screen."),
        })
      : s.statusLine === "theirs"
        ? el("p", {
            class: "note",
            text: s.installed
              ? t("You have your own status line. Reinstall the hooks to see your plan's usage on the island too: yours keeps showing in Claude Code, and removing the hooks puts it back as it was.")
              : t("You have your own status line. Installing the hooks also brings your plan's usage to the island: yours keeps showing in Claude Code, and removing the hooks puts it back as it was."),
          })
        : null;
  const review = pending
    ? el(
        "div",
        { class: "review" },
        el("div", { class: "review-title", text: pending.install ? t("Review the install") : t("Review the removal") }),
        pending.preview.diff ? diff(pending.preview.diff) : el("p", { class: "note", text: t("Nothing to change.") }),
        el(
          "div",
          { class: "actions" },
          button(t("Cancel"), () => {
            panels.get(kind)!.pending = null;
            render();
          }),
          pending.preview.diff ? button(t("Write the file"), () => void apply(kind), true) : null,
        ),
      )
    : null;

  return el(
    "section",
    { class: "card" },
    el("div", { class: "card-head" }, el("div", { class: "card-title", text: name }), agentStatus(s)),
    el("div", { class: "path", text: s.configPath }),
    s.hookReady ? null : el("p", { class: "note error", text: t("The hook relay is missing at {path}.", { path: s.hookPath }) }),
    s.error ? el("p", { class: "note error", text: s.error }) : null,
    codexHelp,
    updateHelp,
    usageHelp,
    kind === "gemini"
      ? el("p", {
          class: "note warn",
          text: t("Vults no longer supports Gemini CLI: Antigravity took its place. Remove the hooks it kept in Gemini's settings."),
        })
      : kind === "antigravity"
        ? el("p", {
            class: "note",
            text: t("One hooks file for the agy CLI, the app and the IDE. Antigravity asks its permissions itself, and the island shows its sessions as antigravity."),
          })
        : kind === "opencode"
          ? el("p", {
              class: "note",
              text: t("OpenCode loads a plugin file instead of hooks: restart it after installing. Its permissions and questions show on the island too: answer there or in OpenCode, whichever comes first."),
            })
          : kind === "qwen"
            ? el("p", {
                class: "note",
                text: t("Restart Qwen Code after installing. Its permissions show on the island; Qwen asks in its own terminal once the card goes away unanswered. Its questions stay in its terminal."),
              })
            : kind === "copilot"
              ? el("p", {
                  class: "note",
                  text: t("Copilot CLI reads every hooks file in ~/.copilot/hooks/: this one is Vults' own. Restart Copilot after installing. Its permissions show on the island; Copilot asks in its own terminal once the card goes away unanswered."),
                })
              : kind === "cursor"
                ? el("p", {
                    class: "note",
                    text: t("One hooks file for the editor and cursor-agent; other tools' hooks in it stay. Cursor asks its permissions itself, and the island shows its sessions as cursor. Restart Cursor after installing."),
                  })
                : kind === "droid"
                  ? el("p", {
                      class: "note",
                      text: t("Droid runs these hooks in the CLI and its sessions show as droid. Droid asks its permissions itself, in its own prompt. Restart Droid after installing."),
                    })
                  : kind === "kimi"
                    ? el("p", {
                        class: "note",
                        text: t("Kimi Code reads these hooks from its config.toml, beside your own settings, and its sessions show as kimi. Kimi asks its permissions itself, in its own prompt. Restart Kimi after installing."),
                      })
                    : null,
    notice,
    s.error || pending
      ? null
      : el(
          "div",
          { class: "actions" },
          s.installed ? button(setupWord(kind) === "plugin" ? t("Remove plugin…") : t("Remove hooks…"), () => void preview(kind, false)) : null,
          s.installBlocked
            ? null
            : // The white button is the step that is due: installing, updating, or a reinstall a note asks
              // for. A reinstall of hooks that are up to date is just there.
              button(
                setupWord(kind) === "plugin"
                  ? s.outdated ? t("Update plugin…") : s.installed ? t("Reinstall plugin…") : t("Install plugin…")
                  : s.outdated ? t("Update hooks…") : s.installed ? t("Reinstall hooks…") : t("Install hooks…"),
                () => void preview(kind, true),
                !s.installed || s.outdated || usageHelp !== null,
              ),
        ),
    review,
  );
}

/** What each agent can do (W1): built once, asked again each time the page is opened. */
const doctor = createDoctor((fresh) => Bridge.agentsDoctor(fresh));
let doctorAsked = false;

function agentsPage(): HTMLElement[] {
  if (!doctorAsked) {
    doctorAsked = true;
    doctor.refresh();
  }
  return [
    el("h1", { text: t("Agents") }),
    el("p", {
      class: "lede",
      text: t("Vults hears your agents through hooks in their config. Every change shows you the exact diff and takes a dated backup first; hooks from other tools are kept."),
    }),
    ...AGENTS.filter((a) => !a.retired || panels.get(a.kind)?.status?.installed || panels.get(a.kind)?.message).map((a) =>
      agentCard(a.kind, a.name),
    ),
    disclosure(t("What each agent can do"),
      el("p", { class: "note", text: t("What this computer has of each agent, and what it can do with Vults: what the router and your policies lean on.") }),
      doctor.element,
    ),
  ];
}

// ── Connectors ───────────────────────────────────────────────────────────────

async function refreshConnectors(): Promise<void> {
  try {
    connectorStatus = new Map((await Bridge.connectorsStatus()).map((c) => [c.id, c]));
  } catch {
    // Shown as "not running" below.
  }
  if (page === "connectors") render();
}

function connectorsPage(): HTMLElement[] {
  return [
    el("h1", { text: t("Connectors") }),
    el("p", { class: "lede", text: t("News from outside services on the island. Each one is off until you switch it on.") }),
    ...(presence === "paused" ? [el("p", { class: "note", text: t("Paused: connectors do not check anything until you pick another presence in General.") })] : []),
    ...CONNECTORS.map((c) => {
      const st = connectorStatus.get(c.id);
      const control = toggle(st?.enabled ?? false, async (on) => {
        await Bridge.connectorEnable(c.id, on);
        await refreshConnectors();
      });
      control.setAttribute("aria-label", c.name);
      const state = !st?.enabled
        ? badge(t("Off"), "off")
        : st.overBudget
          ? badge(t("Budget spent"), "warn")
          : st.error
          ? badge(t("Error"), "error")
          : st.lastOk
            ? badge(t("Watching {what}", { what: st.watching }), "ok")
            : badge(t("Checking…"), "warn");
      return el(
        "section",
        { class: "card" },
        el(
          "div",
          { class: "card-head" },
          el("div", { class: "card-title", text: c.name }),
          el(
            "div",
            { class: "head-right" },
            st?.enabled ? state : null,
            control,
          ),
        ),
        el("p", { class: "row-about", text: t(c.about) }),
        st?.enabled && st.lastOk && !st.error ? el("p", { class: "note", text: t("Last checked {when}.", { when: ago(st.lastOk) }) }) : null,
        st?.enabled && st.error ? el("p", { class: "note error", text: st.error }) : null,
        st?.enabled && st.overBudget
          ? el("p", { class: "note warn", text: t("It made as many checks as it may in a day, so it waits: it checks again once it has room.") })
          : null,
      );
    }),
  ];
}

// ── General ──────────────────────────────────────────────────────────────────

function generalPage(): HTMLElement[] {
  return [
    el("h1", { text: t("General") }),
    el("section", { class: "card rows" },
      el("h2", { class: "settings-group-title", text: t("Appearance") }),
      row(t("Mini and Quick width"), t("Shared width, limited to the available screen space."),
        dropdown(ISLAND_WIDTHS.map(value => ({ value, label: value ? `${value} px` : t("Default") })), restoreWidth(), async value => {
          if (!saveWidth(value)) throw new Error(t("Could not save the width. Choose it again to retry."));
        })),
      row(t("Theme"), t("Black or a neutral light theme, across every window."),
        dropdown([{ value: "black" as const, label: t("Black") }, { value: "light" as const, label: t("Light") }], theme, async value => { await Bridge.setTheme(value); theme = value; })),
      row(
        t("Language"),
        t("The language of the island, these settings, the tray and the notifications. System follows your desktop's."),
        dropdown(
          [{ value: null as string | null, label: t("System") }, ...APP_LANGUAGES.map((l) => ({ value: l.code as string | null, label: l.name }))],
          language,
          async (code) => {
            await Bridge.setLanguage(code);
            language = code;
          },
        ),
      ),
    ),
    el("section", { class: "card rows" },
      el("h2", { class: "settings-group-title", text: t("Sound and notifications") }),
      row(
        t("Sounds"),
        t("Short 8-bit blips when a session needs you, finishes or fails, and for connector news."),
        toggle(sounds, async (on) => {
          await Bridge.setSounds(on);
          sounds = on;
          render();
        }),
      ),
      row(t("Volume"), t("How loud the sounds play. A cue plays when you let go of the slider."), slider(volume, !sounds)),
      row(
        t("Now playing"),
        t("Shows the song your music player is playing, with play, pause and skip, and Zeca dances to it. Read from your media players on this computer; nothing leaves it."),
        toggle(nowPlaying, async (on) => {
          await Bridge.setNowPlaying(on);
          nowPlaying = on;
        }),
      ),
      row(
        t("Do not disturb"),
        dndUntil
          ? t("On until {time}: no sounds and no notifications, and no reminders for a waiting card. A card still opens the island with its sound.", { time: new Date(dndUntil * 1000).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" }) })
          : t("No sounds and no notifications for a while. A card still opens the island with its sound, so an agent never waits for nobody. It ends by itself."),
        segmented(
          [{ value: 0, label: t("Off") }, ...DND_CHOICES.map((m) => ({ value: m, label: m < 60 ? t("{n} min", { n: m }) : t("{n} h", { n: m / 60 }) }))],
          // On, no length is marked: the time left is in the words.
          dndUntil ? -1 : 0,
          async (m) => {
            await Bridge.setDnd(m || null);
            dndUntil = m ? Math.floor(Date.now() / 1000) + m * 60 : null;
            render();
          },
        ),
      ),
      row(
        t("Notifications"),
        t("Only in Panel, where the island is out of sight: a desktop notification when a session finishes or fails, goes quiet, or a card waits for you. In Island and Quiet the island shows it all, so nothing goes to the desktop. Its only button opens the island; it never answers a card."),
        toggle(notifications, async (on) => {
          await Bridge.setNotifications(on);
          notifications = on;
        }),
      ),
    ),
    el("section", { class: "card rows" },
      el("h2", { class: "settings-group-title", text: t("Island") }),
      row(
        t("Presence"),
        t("Choose how Vults stays visible. A card always opens the island with its sound."),
        segmented(
          [
            { value: "island" as Presence, label: t("Island") },
            { value: "panel" as Presence, label: t("Panel") },
            { value: "quiet" as Presence, label: t("Quiet") },
            { value: "paused" as Presence, label: t("Paused") },
          ],
          presence,
          async (p) => {
            await Bridge.setPresence(p);
            presence = p;
          },
        ),
        t("Island: the flock at the top. Panel: Zeca in the tray. Quiet: only cards. Paused: agents ask in their terminals, connectors and notifications stop. Also in the tray's menu."),
      ),
      row(
        t("Fold the island"),
        t("How long the open island stays once the pointer leaves it. A permission keeps it open until you answer."),
        segmented(
          FOLD_CHOICES.map((n) => ({ value: n, label: `${n} s` })),
          foldAfter,
          async (n) => {
            await Bridge.setFoldAfter(n);
            foldAfter = n;
          },
        ),
      ),
      row(
        t("Open on hover"),
        t("Resting the pointer on the island opens it all the way, without a click. Opened that way it folds as soon as the pointer leaves, unless you clicked in it. Not in Panel."),
        toggle(openOnHover, async (on) => {
          await Bridge.setOpenOnHover(on);
          openOnHover = on;
        }),
      ),
      row(
        t("Corner widget"),
        t("A small window in a corner of the screen with up to three birds, the sessions that matter most, and how many work or need you. A click opens the island, on the card when one waits; it never answers one."),
        dropdown(
          [
            { value: null as Corner | null, label: t("Off") },
            { value: "top-left" as Corner | null, label: t("Top left") },
            { value: "top-right" as Corner | null, label: t("Top right") },
            { value: "bottom-left" as Corner | null, label: t("Bottom left") },
            { value: "bottom-right" as Corner | null, label: t("Bottom right") },
          ],
          widget,
          async (c) => {
            await Bridge.setWidget(c);
            widget = c;
          },
        ),
      ),
      row(
        t("Screen"),
        t("Where the island sits. Automatic lets the desktop choose; a chosen screen that is unplugged hands the island back until it returns."),
        // A list, not buttons: screen names are long and a narrow window would break them.
        dropdown(
          [
            { value: null as string | null, label: t("Automatic") },
            ...monitors.map((m) => ({ value: m.name as string | null, label: m.label, title: m.name })),
            // Keep showing a saved screen that is unplugged right now.
            ...(monitor && !monitors.some((m) => m.name === monitor)
              ? [{ value: monitor as string | null, label: t("{name} (not connected)", { name: monitor }), title: monitor }]
              : []),
          ],
          monitor,
          async (m) => {
            await Bridge.setMonitor(m);
            monitor = m;
          },
        ),
      ),
    ),
    el("section", { class: "card rows" },
      el("h2", { class: "settings-group-title", text: t("Startup and usage") }),
      row(
        t("Start with the desktop"),
        t("Opens Vults when you log in."),
        toggle(autostart, async (on) => {
          await Bridge.setAutostart(on);
          autostart = on;
        }),
      ),
      row(
        t("Codex usage"),
        t("Shows your Codex plan's usage on the island. Vults asks the codex you are logged into every 5 minutes, and each ask reaches OpenAI. Off, nothing is asked."),
        toggle(codexUsage, async (on) => {
          await Bridge.setCodexUsage(on);
          codexUsage = on;
        }),
      ),
    ),
    disclosure(t("Keyboard shortcuts"), el(
      "section",
      { class: "card rows" },
      row(t("Allow / Deny from anywhere"), t("Answer the visible permission card. Change global keys in System Settings → Shortcuts."), el("span", { class: "kbd", text: "Ctrl+Alt+Y · Ctrl+Alt+N" })),
      row(t("Dictation"), t("Choose a local model in Dictation > Voice. Hold to dictate into your current app. This shortcut never opens Chat."), el("span", { class: "kbd", text: boundKeys("talk") })),
      row(t("Voice command"), t("Hold to control Vults, then release. Nothing is typed into another app."), el("span", { class: "kbd", text: boundKeys("talk-command") })),
      row(t("Dictation settings"), t("Configure dictation keys, feedback, insertion and history in its own page."), button(t("Dictation settings"), () => choosePage("dictation"))),
      row(t("Next / previous session"), t("Move through the flock. A waiting permission keeps its session in front."), el("span", { class: "kbd", text: "Ctrl+Alt+J · Ctrl+Alt+K" })),
      row(t("Open the island"), t("Open the island with keyboard control: ↑ ↓ sessions, Enter terminal, M actions, Y / N answer, C chat, Esc return."), el("span", { class: "kbd", text: "Ctrl+Alt+Space" })),
      row(t("Change size"), t("Switch to Mini, Quick or Room from anywhere."), el("span", { class: "kbd", text: "Ctrl+Alt+1 · 2 · 3" })),
      row(t("Command palette"), t("Find a session, open a tab or prepare a chat command in Quick or Room."), el("span", { class: "kbd", text: "Ctrl+K" })),
      row(t("The Nest's tabs"), t("Choose Flock, Chat, Activity, Connections or Dictation. Esc leaves Full."), el("span", { class: "kbd", text: "Alt+1–5" })),
    )),
  ];
}

async function refreshMicrophones(): Promise<void> {
  if (microphonesLoading || microphoneSaving) return;
  microphonesLoading = true;
  microphonesFailed = false;
  if (page === "dictation") render();
  try {
    microphones = await Bridge.dictationMicrophones();
  } catch {
    microphonesFailed = true;
  } finally {
    microphonesLoading = false;
    if (page === "dictation") render();
  }
}

function dictationPage(): HTMLElement[] {
  if (dictationSection === "conversation") return [el("h1", { text: t("Talk with Zeca") }), realtimeSettings.element];
  if (dictationSection === "agents") return [el("h1", { text: t("Agents") }), questionAgents.element];
  if (dictationSection === "meetings") return [el("h1", { text: t("Meeting settings") }), meetingSettings.element, meetingAssets.element];
  if (dictationSection === "styles") return [el("h1", { text: t("Styles") }), cleanupSettings.element];
  if (dictationSection === "diagnostics") return [el("h1", { text: t("Diagnostics") }), dictationDoctor.element];
  const microphoneChoices: { value: string | null; label: string }[] = [
    { value: null, label: t("System default microphone") },
    ...(microphones ?? []).map(name => ({ value: name, label: name })),
  ];
  if (dictationMicrophone !== null && !(microphones ?? []).includes(dictationMicrophone)) {
    microphoneChoices.push({ value: dictationMicrophone, label: microphones === null ? dictationMicrophone
      : t("{name} (unavailable)", { name: dictationMicrophone }) });
  }
  const microphone = dropdown(microphoneChoices, dictationMicrophone, async device => {
    microphoneSaving = true;
    render();
    try {
      await Bridge.setDictationMicrophone(device);
      dictationMicrophone = device;
      dictationMicrophoneChanged = true;
    } finally {
      microphoneSaving = false;
      if (page === "dictation") render();
    }
  });
  (microphone as HTMLSelectElement).disabled = microphonesLoading;
  const refresh = button(t("Refresh microphones"), () => void refreshMicrophones());
  (refresh as HTMLButtonElement).disabled = microphonesLoading || microphoneSaving;
  refresh.dataset.setting = "Refresh microphones";
  const microphoneFeedback = el("p", { class: `note${microphonesFailed ? " error" : ""}`, text: microphonesLoading
    ? t("Loading microphones…") : microphonesFailed
      ? microphones === null ? t("Could not list microphones. Refresh to try again.")
        : t("Could not refresh microphones. The last list may be out of date. Refresh to try again.")
      : t("No microphones found. Connect one and refresh.") });
  microphoneFeedback.setAttribute("role", microphonesFailed ? "alert" : "status");
  const shortcutEditor = button(t("Change in KDE…"), () => {
    if (shortcutsOpening) return;
    shortcutsOpening = true;
    shortcutsOpenFailed = false;
    render();
    void Bridge.openShortcutSettings().catch(() => { shortcutsOpenFailed = true; }).finally(() => {
      shortcutsOpening = false;
      if (page === "dictation") render();
    });
  });
  (shortcutEditor as HTMLButtonElement).disabled = shortcutsOpening;
  const shortcutFeedback = el("p", { class: `note${shortcutsOpenFailed ? " error" : ""}`, text: shortcutsOpenFailed
    ? t("Could not open KDE Shortcuts. Open your desktop's shortcut settings manually, or try again.")
    : t("Opening KDE Shortcuts…") });
  shortcutFeedback.setAttribute("role", shortcutsOpenFailed ? "alert" : "status");
  const erase = button(t("Erase"), () => {
    if (dictationErasing) return;
    dictationErasing = true;
    dictationEraseFailed = false;
    dictationErased = false;
    render();
    void Bridge.dictationClear().then(() => {
      dictationEraseConfirm = false;
      dictationErased = true;
    }, () => { dictationEraseFailed = true; }).finally(() => {
      dictationErasing = false;
      if (page === "dictation") render();
    });
  });
  (erase as HTMLButtonElement).disabled = dictationErasing;
  erase.dataset.setting = "Erase dictation confirmation";
  const cancel = button(t("Cancel"), () => { dictationEraseConfirm = false; dictationEraseFailed = false; render(); });
  (cancel as HTMLButtonElement).disabled = dictationErasing;
  cancel.dataset.setting = "Cancel dictation erasure";
  const confirmation = el("div", { class: "note" },
    el("p", { text: t("Erase every retained dictation? Usage totals stay.") }),
    el("div", { class: "actions" }, erase, cancel));
  confirmation.setAttribute("role", "group");
  confirmation.setAttribute("aria-label", t("Erase dictation history"));
  confirmation.addEventListener("keydown", event => {
    if (event.key === "Escape" && !dictationErasing) {
      event.preventDefault(); dictationEraseConfirm = false; dictationEraseFailed = false; render();
    }
  });
  const feedback = el("p", { class: `note${dictationEraseFailed ? " error" : ""}`, text: dictationErasing
    ? t("Erasing dictation history…") : dictationEraseFailed ? t("Could not erase dictation history. Try again.")
    : t("Dictation history erased. Usage totals stay.") });
  feedback.setAttribute("role", dictationEraseFailed ? "alert" : "status");
  return [
    el("h1", { text: t(DICTATION_SETTINGS.find(p => p.id === dictationSection)!.label) }),
    ...(dictationSection === "keys" ? [
      el("section", { class: "card rows" },
        row(t("Dictation"), t("Choose a local model in Dictation > Voice. Hold to dictate into your current app. This shortcut never opens Chat."), el("span", { class: "kbd", text: boundKeys("talk") })),
        row(t("Copy last dictation"), t("Copy the last successful dictation again. Paste it yourself; Vults does not submit anything."), el("span", { class: "kbd", text: boundKeys("talk-again") })),
        row(t("Cancel voice"), t("Discard the current dictation or Chat recording."), el("span", { class: "kbd", text: boundKeys("talk-cancel") })),
        row(t("Desktop shortcut settings"), t("Opens KDE System Settings → Shortcuts. Other desktops manage global keys in their own settings."), shortcutEditor),
        ...(shortcutsOpening || shortcutsOpenFailed ? [shortcutFeedback] : []),
        row(t("Recording mode"), t("Hold to dictate. Optionally double-tap to keep recording hands-free, then press the shortcut again to finish."),
          dropdown([{ value: true, label: t("Hold + double tap") }, { value: false, label: t("Hold only") }], dictationDoubleTap, async on => {
            await Bridge.setDictationGesture("doubleTap", on);
            dictationDoubleTap = on; dictationDoubleTapChanged = true;
          })),
        row(t("Space locks recording"), t("X11 only. Space keeps recording after you release the talk shortcut. When off, Space behaves like other typing and can discard a held recording."), (() => {
          const control = toggle(dictationSpaceLock, async on => {
            await Bridge.setDictationGesture("spaceLock", on);
            dictationSpaceLock = on; dictationSpaceLockChanged = true;
          });
          if (dictationWayland) control.setAttribute("disabled", "");
          return control;
        })()),
        row(t("Esc cancels dictation"), t("X11 only. Esc discards recording or transcription. When off, Esc does not cancel dictation."), (() => {
          const control = toggle(dictationEscapeCancel, async on => {
            await Bridge.setDictationGesture("escapeCancel", on);
            dictationEscapeCancel = on; dictationEscapeCancelChanged = true;
          });
          if (dictationWayland) control.setAttribute("disabled", "");
          return control;
        })()),
        row(t("Cancel by voice"), t("Say a cancel phrase by itself or at the end of the final transcript, such as “cancel” or “never mind”. Discards this dictation without copying, inserting or saving it. English, Portuguese and Spanish; works on X11 and Wayland."), toggle(dictationCancelByVoice, async on => {
          await Bridge.setDictationProcessing("cancelByVoice", on);
          dictationCancelByVoice = on; dictationCancelByVoiceChanged = true;
        })),
        el("p", { class: "hint", text: t("Independent dictation only, checked after recording stops. To dictate a sentence ending in a cancel phrase, turn this off.") }),
        el("p", { class: "hint", text: t("Changes apply to the next dictation. The global Cancel voice shortcut always works. Space and Esc require an available X11 key listener.") }),
      ),
    ] : []),
    ...(dictationSection === "commands" ? [
      el("section", { class: "card rows" },
        row(t("Enable voice commands"), t("Hold the command key, speak and release to control Vults. Transcription and command matching stay on this computer."), toggle(dictationCommands, async on => {
          await Bridge.setDictationCommands("enabled", on);
          dictationCommands = on; dictationCommandsChanged = true;
          if (page === "dictation") render();
        })),
        row(t("Command key"), t("Hold to control Vults, then release. Nothing is typed into another app."), el("span", { class: "kbd", text: boundKeys("talk-command") })),
        row(t("Names it understands"), t("Uses current project and agent names, plus replacements in Dictionary. Small spelling differences can match; when names tie, Vults asks which one."), button(t("Dictionary"), () => void Bridge.openNest("dictation-dictionary"))),
        row(t("When it is not a command"), t("Sending to Zeca uses the current Chat conversation and its model, access rules and budget. Zeca must be on and Chat idle, with no unsent text or attachments. Refused commands and ambiguous names stay local."), (() => {
          const control = dropdown([{ value: false, label: t("Say so on the island") }, { value: true, label: t("Send it to Zeca as a message") }], dictationCommandChat, async on => {
            await Bridge.setDictationCommands("chat", on);
            dictationCommandChat = on; dictationCommandChatChanged = true;
          });
          (control as HTMLSelectElement).disabled = !dictationCommands;
          return control;
        })()),
        disclosure(t("What you can say"), voiceCommandHelp()),
      ),
    ] : []),
    ...(dictationSection === "words" ? [
      el("section", { class: "card rows" },
        row(t("Help the model with these words"), t("Give Whisper your dictionary words as local context. Off by default; currently only Turbo with fixed English is qualified by synthetic tests. Other models and languages keep their existing prompts."), toggle(dictationVocabulary, async on => {
          await Bridge.setDictationProcessing("vocabulary", on);
          dictationVocabulary = on; dictationVocabularyChanged = true;
        })),
        row(t("Project vocabulary"), t("Add cached package, module, file and branch names from the captured session or an unambiguous editor/terminal title. Local only, off by default, with the same Turbo/English restriction."), toggle(dictationProjectVocabulary, async on => {
          await Bridge.setDictationProcessing("projectVocabulary", on);
          dictationProjectVocabulary = on; dictationProjectVocabularyChanged = true;
        })),
        el("p", { class: "hint", text: t("Dictionary words come first under the shared 224-token limit. Changes apply to the next recording. Edit Dictionary and Snippets in Dictation; Chat is unchanged.") }),
      ),
    ] : []),
    ...(dictationSection === "voice" ? [
      el("p", { class: "lede", text: t("Speak in another app. Transcription stays on this computer, with your choice of insertion and history.") }),
      el("section", { class: "card rows" }, ...voiceRows()),
      el("section", { class: "card rows" },
        row(t("Dictation microphone"), t("Used for the next independent dictation. If missing, capture uses the system default or another input. Chat keeps its system default microphone."), microphone),
        el("div", { class: "actions" }, refresh),
        ...(microphonesLoading || microphonesFailed || microphones?.length === 0 ? [microphoneFeedback] : []),
        row(t("Remove repeated loops"), t("Collapse short phrases repeated at least six times in the preview and final text. Sound annotations and standalone silence phrases are still removed when this is off."), toggle(dictationCutRepeats, async on => {
          await Bridge.setDictationProcessing("cutRepeats", on);
          dictationCutRepeats = on; dictationCutRepeatsChanged = true;
        })),
        row(t("Retry empty dictation with Base"), t("Retry once only when the selected model returns no text, speech was detected and a different Base model is already installed. Shows retry feedback and never downloads a model."), toggle(dictationRetryBase, async on => {
          await Bridge.setDictationProcessing("retryBase", on);
          dictationRetryBase = on; dictationRetryBaseChanged = true;
        })),
        el("p", { class: "hint", text: t("Processing choices apply to the next independent dictation. Active recordings keep their choices; Chat keeps its existing processing.") }),
        row(t("Music while you speak"), t("Temporarily lower or pause playing media during independent dictation. Restore each player when recording stops or is cancelled, unless you changed it. Uses local media controls, separately from Now playing; unavailable players are left alone."), dropdown([
          { value: "lower" as const, label: t("Lower to 25%") },
          { value: "pause" as const, label: t("Pause it") },
          { value: "leave" as const, label: t("Leave it") },
        ], dictationMusic, async mode => {
          await Bridge.setDictationMusic(mode);
          dictationMusic = mode; dictationMusicChanged = true;
        })),
        row(t("Dictation start sound"), t("Play a short cue when recording starts. Respects sound volume and Do Not Disturb."), toggle(dictationSound, async (on) => {
          await Bridge.setDictationSound(on);
          dictationSound = dictation.startSound(on);
        })),
        row(t("Dictation captions"), t("Show clipped words and the final transcript near the island. Captions take no clicks and disappear two seconds after completion."), toggle(dictationCaption, async (on) => {
          await Bridge.setDictationCaption(on);
          dictationCaption = dictation.captions(on);
        })),
      ),
    ] : []),
    ...(dictationSection === "paste" ? [
      el("section", { class: "card rows" },
        el("p", { class: "hint", text: t("App recognition works on X11 and Plasma Wayland. GNOME and wlroots Wayland may leave the app unknown. Background copying requires desktop support; if unavailable, dictation shows an error.") }),
        row(t("Insert dictation in supported editors and tmux panes"), t("Off by default. Pastes in KWrite, GNOME Text Editor, Mousepad or Leafpad, or inserts one line into an exact tmux flock pane in XTerm or Kitty. Waiting cards, unknown panes and focus changes keep the text copied. Plasma Wayland requires desktop keyboard permission. Nothing is submitted."), (() => {
          const control = toggle(dictationInsert, async (on) => {
            permissionFailure = await permissionWrite(() => Bridge.setDictationInsert(on));
            dictationInsert = on;
            dictationInsertChanged = true;
            render();
          });
          if (!dictationInsertAvailable && !dictationInsert) control.setAttribute("disabled", "");
          return control;
        })()),
        el("p", { class: "hint", text: t("Insertion temporarily replaces the clipboard and restores only its previous text, up to 64 KiB. Images and rich formats are not restored. A newer copy is kept. Clipboard history exclusion depends on the desktop. Native X11 or Plasma Wayland with ext-data-control is required.") }),
        ...(permissionFailure ? [el("p", { class: "note error", text: permissionErrors.has(permissionFailure) ? t(permissionFailure) : permissionFailure }),
          ...(!dictationInsert ? [(() => {
            const control = button(t("Retry token removal"), () => saveSetting(control, async () => {
              permissionFailure = await permissionWrite(() => Bridge.setDictationInsert(false));
              render();
            }, () => {}));
            return control;
          })()] : [])] : []),
        ...(dictationInsert && dictationWayland ? [row(t("Wayland keyboard access"), t({
          off: tk("Keyboard access is inactive. On Wayland, request access here to enable insertion."),
          requesting: tk("Waiting for the desktop keyboard permission dialog. You can turn insertion off to cancel."),
          ready: tk("Keyboard access is ready. The restore token is kept only in the OS keyring."),
          temporary: tk("Keyboard access is ready for this run. No reusable token was saved; request access again after restarting."),
          unavailable: tk("Keyboard access is unavailable or was revoked. Dictation keeps copying; retry only when you want another desktop prompt."),
        }[dictationPermission]), (() => {
          const control = button(t("Request keyboard access"), () => saveSetting(control, () => permissionWrite(Bridge.retryDictationPermission), () => {}));
          if (["requesting", "ready", "temporary"].includes(dictationPermission)) control.setAttribute("disabled", "");
          return control;
        })())] : []),
      ),
      el("h2", { text: t("Terminals & sessions") }),
      el("section", { class: "card rows" },
        row(t("Spoken code"), t("In recognized terminals or sessions, join identifier chunks around spoken dot, slash, underscore, dash and colon in English, Portuguese or Spanish. Other apps keep their text. Changes apply to the next dictation."), toggle(dictationSpokenCode, async on => {
          await Bridge.setDictationProcessing("spokenCode", on);
          dictationSpokenCode = on; dictationSpokenCodeChanged = true;
        })),
        el("p", { class: "hint", text: t("Preserves case and existing punctuation; never adds an ending period. This changes text only and never submits it.") }),
      ),
    ] : []),
    ...(dictationSection === "privacy" ? [
      el("section", { class: "card rows" },
        row(t("Keep dictation history"), t("Only text is kept on this computer. Shortening retention erases older text immediately. Usage totals stay; audio and previous clipboard contents are never saved."),
          dropdown([{ value: "0", label: t("Do not keep") }, { value: "7", label: t("7 days") }, { value: "30", label: t("30 days") }, { value: "365", label: t("1 year") }, { value: "all", label: t("Always") }],
            dictationKeep, async value => { await Bridge.setDictationKeep(value); dictationKeep = value; dictationKeepChanged = true; })),
        row(t("Erase dictation history"), t("Every retained dictation is erased. Measured daily and app totals stay. Copy again keeps its existing in-memory lifetime."), button(t("Erase dictation history…"), () => {
          if (dictationErasing) return;
          dictationEraseConfirm = true; dictationEraseFailed = false; dictationErased = false; render();
          root.querySelector<HTMLElement>('[data-setting="Erase dictation confirmation"]')?.focus();
        })),
        ...(dictationEraseConfirm ? [confirmation] : []),
        ...(dictationErasing || dictationEraseFailed || dictationErased ? [feedback] : []),
      ),
    ] : []),
  ];
}

// ── Chat ─────────────────────────────────────────────────────────────────────

function chatPage(): HTMLElement[] {
  const p = apiProviders.find((x) => x.id === apiSelected);
  const message = apiMessage ? el("p", { class: `note${apiMessage.error ? " error" : " ok"}`, text: apiMessage.text }) : null;
  const done = (text: string, error = false) => {
    apiMessage = { text, error };
    void refreshApi();
  };

  const select = document.createElement("select");
  select.className = "field select";
  const group = (label: string, local: boolean) => {
    const g = document.createElement("optgroup");
    g.label = label;
    for (const x of apiProviders.filter((x) => x.local === local)) {
      const o = new Option(x.hasKey ? t("{name} · key saved", { name: x.label }) : x.label, x.id, false, x.id === apiSelected);
      g.append(o);
    }
    return g;
  };
  select.append(group(t("Cloud, with your key"), false), group(t("On this machine"), true));
  select.addEventListener("change", () => {
    apiSelected = select.value;
    apiMessage = null;
    void Bridge.apiProviderSet(select.value).then(() => refreshApi());
  });

  const rows: (HTMLElement | null)[] = [
    row(t("Provider"), t("API messages go to this provider. This mode cannot run commands or edit files."), select, t("The API chat can't run commands, edit files or open your project. It reads the files you drop on the island and, when a question needs it, what the island shows (your sessions, the cards waiting, usage, your GitHub card), which then goes to the provider with the conversation. Keys are kept in the system keyring, never in a file, and only ever sent to their own provider.")),
  ];
  if (p) {
    rows.push(row(...keyRow(p, done)));
    rows.push(row(...modelRow(p)));
  }
  rows.push(message ? el("div", { class: "row" }, message) : null);

  return [
    el("h1", { text: t("Chat & voice") }),
    ...(zeca ? [] : [el("p", { class: "note", text: t("Zeca is off (Flock & Zeca): no chat or voice until you turn him back on.") })]),
    el("p", {
      class: "lede",
      text: t("Claude Code and Codex use your CLI login and subscription. API providers use your own key or a local model."),
    }),
    el(
      "section",
      { class: "card rows" },
      row(
        t("Your Claude Code and Codex setup"),
        t("Use your CLI instructions, skills and tools. Changes apply to the next conversation."),
        toggle(nativeChat, async (on) => {
          await Bridge.setNativeChat(on);
          nativeChat = on;
        }),
        t("The chat's CLI reads your own setup, as in your terminal: your CLAUDE.md, skills, MCP servers, hooks and allow rules. A project's settings join only in a folder you already trust in Claude Code. Cards still ask before every command or edit your rules don't allow. Off, it reads none of it. Applies to the next conversation."),
      ),
      row(
        t("Who answers Zeca's cards"),
        t("Choose who approves commands and edits. Destructive commands always ask; every answer is audited."),
        segmented<ChatAccess>(
          [
            { value: "ask", label: t("Asks first") },
            { value: "policies", label: t("Your policies") },
            { value: "full", label: t("Full access") },
          ],
          chatAccess,
          async (v) => {
            await Bridge.setChatAccess(v);
            chatAccess = v;
            render();
          },
        ),
        t("Asks first: a card before every command or edit. Your policies: the policies you accepted in Approvals answer what they cover, and the rest asks. Full access: everything runs without a card, but destructive commands and what a policy denies or asks first; once a conversation reads from outside (a web page, a fetch, an MCP tool), its cards ask again. Every answer is in the audit log. Holds at once."),
      ),
      ...(chatAccess === "full"
        ? [
            el("p", {
              class: "note warn",
              text: t("With full access Zeca runs commands, edits files and reaches the network without asking. Destructive commands still ask; add deny lines to a policy for anything else that must never run."),
            }),
          ]
        : []),
    ),
    el("h2", { text: t("Memory") }),
    el(
      "section",
      { class: "card rows" },
      row(
        t("Zeca's memory"),
        t("Keeps a local journal with secrets redacted. Recalled memories join your chat and reach its model. Off, journaling and recall stop; Clear history in Activity removes the journal."),
        toggle(memory, async (on) => {
          await Bridge.setMemory(on);
          memory = on;
          render();
        }),
        t("Keeps a journal on this computer of what your agents and Zeca's chat did: each turn's time and changes, each card's answer with its command or path, and the chat's lines, secrets redacted. What he remembers joins your conversations with Zeca, and only that reaches their model. Off, none of it reaches a model and nothing more is written; Clear history in Activity removes the journal."),
      ),
      ...(memory
        ? [
            row(
              t("A summary of each day"),
              t("Sends yesterday's journal to Claude Code's Haiku on your subscription for a daily summary. Up to 8 requests a day, recorded in the local trace."),
              toggle(journalSummary, async (on) => {
                await Bridge.setJournalSummary(on);
                journalSummary = on;
              }),
              t("Once a day, sends the day before's journal to Claude Code's smallest model (Haiku, on your subscription, with no tools) to write a short summary for Zeca to recall; only days from when you turn this on. A request on your behalf: at most 8 a day, each kept in the local trace."),
            ),
            row(
              t("Suggest what to remember"),
              t("Sends the conversation you leave to Claude Code's Haiku on your subscription to suggest memories. You review them in Chat, Memory. Up to 20 requests a day; excludes conversations that read outside content."),
              toggle(memorySuggest, async (on) => {
                await Bridge.setMemorySuggest(on);
                memorySuggest = on;
              }),
              t("When you start a new conversation or open another, sends the one you left to Claude Code's smallest model (Haiku, on your subscription, with no tools) to propose up to three facts worth remembering; you accept or turn down each in the Memory beside the Nest's Chat. Never one that read content from outside. A request on your behalf: at most 20 a day, each kept in the local trace."),
            ),
            row(
              t("Let Zeca sleep on it"),
              t("While idle at night, Claude Code's Haiku on your subscription reviews memories and recent summaries. Proposed merges wait for you in the Nest. Up to 4 requests a day, recorded in the local trace."),
              toggle(sleep, async (on) => {
                await Bridge.setSleep(on);
                sleep = on;
              }),
              t("Once a day, from two in the morning and while nothing is working, Zeca proposes removing facts he remembers twice. On, Claude Code's smallest model (Haiku, on your subscription, with no tools) also reads what he remembers about you and across projects and the last week's summaries, to propose merges, which wait for you in the Nest, and write a short debrief. A request on your behalf: at most 4 a day, each kept in the local trace."),
            ),
            row(
              t("Recall by meaning"),
              t("Recall also by meaning, not only by words, through an Ollama running on this computer with nomic-embed-text. Each summary and each message you send is embedded on 127.0.0.1; nothing leaves this computer."),
              toggle(localEmbeddings, async (on) => {
                await Bridge.setLocalEmbeddings(on);
                localEmbeddings = on;
              }),
            ),
          ]
        : []),
    ),
    el("h2", { text: t("API") }),
    el("section", { class: "card rows" }, ...rows),
    ...(web ? [el("h2", { text: t("Web") }), el("section", { class: "card rows" }, ...webRows())] : []),
    el("h2", { text: t("Voice") }),
    el("section", { class: "card rows" },
      row(t("Voice model and language"), t("Dictation and Chat share one local model and spoken language. Manage both in Dictation > Voice."), button(t("Dictation settings"), () => chooseDictation("voice")))),
  ];
}

const VOICE_MODEL_LABELS: Record<string, string> = {
  base: tk("Base: fast, but weak outside English"),
  small: tk("Small: good in Portuguese and other languages"),
  turbo: tk("Large v3 Turbo: the most accurate; best with a GPU"),
};

function voiceRows(): HTMLElement[] {
  if (!voice) return [];
  const status = voice;
  const intro = row(
    t("Voice model and language"),
    t("Dictation and Chat share this local transcription model. Audio stays on this computer and is never saved. Dictation works with Zeca off."),
    status.ready
      ? button(t("Turn off"), async () => {
          await Bridge.voiceOff();
          await refreshVoice();
        })
      : badge(t("Off"), "off"),
  );
  const named = (code: string) => LANGUAGES.find((l) => l.value === code)?.label ?? code;
  const language = row(
    t("Language you speak"),
    t("Telling whisper the language makes short phrases far more reliable than detecting it."),
    dropdown<string | null>(
      [
        { value: null, label: status.system ? t("System ({name})", { name: named(status.system) }) : t("System") },
        { value: "auto", label: t("Detect it each time") },
        ...LANGUAGES,
      ],
      status.language,
      async (code) => {
        await Bridge.voiceLanguageSet(code);
        await refreshVoice();
      },
    ),
  );
  const models = status.models.map((m) => {
    const mb = t("{n} MB", { n: Math.round(m.size / 1_000_000) });
    let control: HTMLElement;
    const progress = downloading.get(m.id);
    if (progress !== undefined) control = el("span", { class: "muted", text: t("Downloading… {pct}%", { pct: progress }) });
    else if (m.installed && status.selected === m.id && status.ready) control = badge(t("In use"), "ok");
    else if (m.installed)
      control = button(t("Use"), async () => {
        await Bridge.voiceSelect(m.id);
        await refreshVoice();
      });
    else
      control = button(
        t("Download {size}", { size: mb }),
        async () => {
          downloading.set(m.id, 0);
          voiceError = null;
          render();
          try {
            await Bridge.voiceDownload(m.id);
          } catch (e) {
            voiceError = String(e);
          }
          downloading.delete(m.id);
          await refreshVoice();
        },
        !status.models.some((x) => x.installed) && m.id === "base",
      );
    return row(t(VOICE_MODEL_LABELS[m.id] ?? m.label), m.installed ? t("{size}, on this computer", { size: mb }) : t("{size} from the whisper.cpp models on Hugging Face, checked before use", { size: mb }), control);
  });
  return [intro, language, ...models, ...(voiceError ? [el("p", { class: "note error", text: voiceError })] : [])];
}

/** Languages whisper knows well, in their own names. */
const LANGUAGES = [
  { value: "pt", label: "Português" }, // check-english:allow (each language in its own name)
  { value: "en", label: "English" },
  { value: "es", label: "Español" }, // check-english:allow
  { value: "fr", label: "Français" }, // check-english:allow
  { value: "de", label: "Deutsch" },
  { value: "it", label: "Italiano" },
];

async function refreshVoice(): Promise<void> {
  voice = await Bridge.voiceStatus();
  // A download this window did not start (reopened Settings) still shows as one.
  for (const id of voice.downloading) if (!downloading.has(id)) downloading.set(id, 0);
  render();
}

/** The key: saved (Remove), missing (a field), or not needed for a local server. */
function keyRow(p: ApiProvider, done: (text: string, error?: boolean) => void): [string, string, HTMLElement] {
  if (p.local) {
    return [t("Key"), t("None needed: it runs on this machine and nothing leaves it."), badge(t("Local"), "ok")];
  }
  if (p.hasKey) {
    return [
      t("{provider} API key", { provider: p.label }),
      t("Saved in the system keyring."),
      button(t("Remove"), () => {
        Bridge.apiKeyClear(p.id)
          .then(() => done(t("Key removed from the keyring.")))
          .catch((e) => done(String(e), true));
      }),
    ];
  }
  const input = document.createElement("input");
  input.type = "password";
  input.className = "field";
  input.placeholder = p.keyHint ? `${p.keyHint}…` : t("Paste the key");
  input.autocomplete = "off";
  input.spellcheck = false;
  const save = () => {
    Bridge.apiKeySet(p.id, input.value)
      .then(() => done(t("Saved. Choose the API in the chat to use it.")))
      .catch((e) => done(String(e), true));
  };
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") save();
  });
  return [
    t("{provider} API key", { provider: p.label }),
    t("Kept in the system keyring, never in a file. Usage is billed to your account there."),
    el("div", { class: "inline" }, input, button(t("Save"), save, true)),
  ];
}

/** The model, from the provider's live list once it can be asked. */
function modelRow(p: ApiProvider): [string, string, HTMLElement] {
  const list = apiModels.get(p.id);
  if (!p.local && !p.hasKey) return [t("Model"), t("Listed live from the provider once a key is saved."), badge(t("No key"), "off")];
  if (list === undefined) {
    void loadModels(p.id);
    return [t("Model"), t("Asking {provider} for its models…", { provider: p.label }), badge(t("Loading"), "warn")];
  }
  if ("error" in list) {
    return [t("Model"), list.error, button(t("Try again"), () => void loadModels(p.id))];
  }
  const select = document.createElement("select");
  select.className = "field select";
  const choices = p.model && !list.models.includes(p.model) ? [p.model, ...list.models] : list.models;
  if (!p.model) select.append(new Option(t("Choose a model"), "", true, true));
  for (const m of choices) select.append(new Option(m, m, false, m === p.model));
  select.addEventListener("change", () => {
    void Bridge.apiModelSet(p.id, select.value).then(() => refreshApi());
  });
  const listed = t("Listed live from {provider}: {n} models.", { provider: p.label, n: list.models.length });
  const missing = p.model && !list.models.includes(p.model) ? ` ${t("{model} is no longer listed.", { model: p.model })}` : "";
  return [t("Model"), `${listed}${missing}`, select];
}

async function loadModels(id: string): Promise<void> {
  if (apiLoading.has(id)) return;
  apiLoading.add(id);
  try {
    apiModels.set(id, { models: await Bridge.apiModels(id) });
  } catch (e) {
    apiModels.set(id, { error: String(e) });
  }
  apiLoading.delete(id);
  if (page === "chat") render();
}

async function refreshApi(): Promise<void> {
  try {
    const r = await Bridge.apiProviders();
    // A key saved or removed changes what the list can say.
    for (const x of r.providers) {
      const before = apiProviders.find((y) => y.id === x.id);
      if (before && before.hasKey !== x.hasKey) apiModels.delete(x.id);
    }
    apiProviders = r.providers;
    apiSelected = r.selected;
  } catch {
    apiProviders = [];
  }
  try {
    web = await Bridge.webSettings();
  } catch {
    web = null;
  }
  if (page === "chat") render();
}

/** Zeca's web: off until turned on, an engine, its key or address. */
function webRows(): HTMLElement[] {
  if (!web) return [];
  const w = web;
  const said = (text: string, error = false) => {
    webMessage = { text, error };
    void refreshApi();
  };
  const rows: HTMLElement[] = [
    row(
      t("Web for Zeca"),
      t("Lets API chat search the web and read pages through your chosen engine and each site's server. Up to 200 requests a day, recorded in the local trace. Outside content never grants permissions."),
      toggle(w.on, async (on) => {
        await Bridge.setWeb(on);
        w.on = on;
      }),
      t("Lets the API chat search the web and read pages when a question needs it. Each search and page read is a request on your behalf, to the engine you choose and to the page's site: at most 200 a day, each kept in the local trace. A page is read by a separate model call with no tools, and only its answer reaches Zeca; search results reach him as short snippets marked untrusted."),
    ),
  ];
  const engine = document.createElement("select");
  engine.className = "field select";
  for (const [id, label] of [["brave", "Brave Search"], ["tavily", "Tavily"], ["searxng", "SearXNG"]] as [WebEngine, string][]) {
    engine.append(new Option(label, id, false, id === w.engine));
  }
  engine.addEventListener("change", () => {
    webMessage = null;
    void Bridge.setWebEngine(engine.value as WebEngine)
      .then(() => refreshApi())
      .catch((e) => {
        webMessage = { text: String(e), error: true };
        void refreshApi();
      });
  });
  rows.push(row(t("Search engine"), t("Where searches go. SearXNG is one you run yourself."), engine));
  if (w.engine === "searxng") {
    const input = document.createElement("input");
    input.className = "field";
    input.placeholder = "https://search.example.com";
    input.value = w.searxngUrl ?? "";
    input.spellcheck = false;
    const save = () =>
      Bridge.setSearxngUrl(input.value)
        .then(() => said(t("Saved.")))
        .catch((e) => said(String(e), true));
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") void save();
    });
    rows.push(row(t("Your SearXNG"), t("Its address; it must answer in JSON (format=json)."), el("div", { class: "inline" }, input, button(t("Save"), () => void save(), true))));
  } else if (w.hasKey) {
    rows.push(
      row(
        t("Search key"),
        t("Saved in the system keyring."),
        button(t("Remove"), () => {
          Bridge.clearWebKey(w.engine)
            .then(() => said(t("Key removed from the keyring.")))
            .catch((e) => said(String(e), true));
        }),
      ),
    );
  } else {
    const input = document.createElement("input");
    input.type = "password";
    input.className = "field";
    input.placeholder = t("Paste the key");
    input.autocomplete = "off";
    input.spellcheck = false;
    const save = () =>
      Bridge.setWebKey(w.engine, input.value)
        .then(() => said(t("Saved.")))
        .catch((e) => said(String(e), true));
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") void save();
    });
    rows.push(row(t("Search key"), t("Kept in the system keyring, never in a file, and only ever sent to that engine."), el("div", { class: "inline" }, input, button(t("Save"), () => void save(), true))));
  }
  if (webMessage) rows.push(el("div", { class: "row" }, el("p", { class: `note${webMessage.error ? " error" : " ok"}`, text: webMessage.text })));
  return rows;
}

// ── About ────────────────────────────────────────────────────────────────────

function aboutPage(): HTMLElement[] {
  return [
    el("h1", { text: t("About") }),
    el(
      "section",
      { class: "card about" },
      el("img", { class: "logo" }),
      el(
        "div",
        {},
        el("div", { class: "card-title", text: "Vults" }),
        el("p", { class: "row-about", text: t("A friendly flock watching your coding agents. MIT licensed; no telemetry.") }),
        version ? el("p", { class: "note", text: t("Version {version}", { version }) }) : null,
      ),
    ),
    el(
      "section",
      { class: "card rows" },
      row(t("Settings"), t("Your settings file."), el("code", { text: settingsPath })),
      row(t("Data"), t("The hook relay, the inbox of dropped files, connector state."), el("code", { text: dataPath })),
    ),
  ];
}

// ── Flock ────────────────────────────────────────────────────────────────────

const FLOCKS: { value: Flock; label: string }[] = [
  { value: "brazil", label: tk("Brazil") },
  { value: "americas", label: tk("The Americas") },
  { value: "world", label: tk("The world") },
];

/** The previews' timer: only while the page is on screen. */
let previews: number | undefined;

function flockPage(): HTMLElement[] {
  const canvases: { id: string; canvas: HTMLCanvasElement }[] = [];
  // Each preview perches on the same wire line, so their sizes compare at a glance.
  const SCALE = 2, W = 44, WIRE = 36;
  const card = (id: string, name: string, latin: string): HTMLElement => {
    const canvas = document.createElement("canvas");
    canvas.width = W * SCALE;
    canvas.height = (WIRE + 4) * SCALE;
    canvases.push({ id, canvas });
    const button = el(
      "button",
      {
        class: `species${id === zecaSpecies ? " on" : ""}`,
        onclick: () => {
          if (id === zecaSpecies) return;
          void Bridge.setZecaSpecies(id).then(
            () => {
              zecaSpecies = id;
              render();
            },
            () => render(),
          );
        },
      },
      canvas,
      el("span", { class: "species-name", text: name }),
      el("span", { class: "species-latin", text: latin }),
    );
    button.setAttribute("aria-pressed", String(id === zecaSpecies));
    return button;
  };
  const group = (family: string, title: string) => [
    el("h2", { text: title }),
    el("div", { class: "species-grid" }, ...SPECIES.filter((s) => s.family === family).map((s) => card(s.id, s.name, s.latin))),
  ];
  // Zeca in his own feathers, or in the look picked (Auto shows his feathers: the season decides).
  const zecaCanvas = document.createElement("canvas");
  zecaCanvas.className = "zeca-preview";
  zecaCanvas.width = W * SCALE;
  zecaCanvas.height = (WIRE + 10) * SCALE;
  zecaCanvas.setAttribute("role", "img");
  const labelPreview = () =>
    zecaCanvas.setAttribute("aria-label", !zecaLook || zecaLook === "auto" || zecaLook === "none" ? t("Zeca, in his own feathers") : t("Zeca, in the look you picked"));
  labelPreview();
  zecaCanvas.hidden = !zeca;
  const shown = new WeakMap<HTMLCanvasElement, Frame>();
  const paint = () => {
    // A hidden window draws nothing; the next tick catches up.
    if (document.hidden) return;
    const t = performance.now();
    for (const { id, canvas } of canvases) {
      const set = speciesSet(id);
      // Idle frames last hundreds of ms: most ticks change nothing, and a repaint is a rect per cell.
      const frame = frameAt(set.clips.idle, t);
      if (shown.get(canvas) === frame) continue;
      shown.set(canvas, frame);
      const ctx = canvas.getContext("2d")!;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      drawFrame(ctx, set, frame, 10, WIRE - perchOf(set), SCALE);
    }
    if (zeca) {
      const set = dress(speciesSet(zecaSpecies), !zecaLook || zecaLook === "auto" ? null : zecaLook);
      const frame = frameAt(set.clips.idle, performance.now());
      if (shown.get(zecaCanvas) !== frame) {
        shown.set(zecaCanvas, frame);
        const ctx = zecaCanvas.getContext("2d")!;
        ctx.clearRect(0, 0, zecaCanvas.width, zecaCanvas.height);
        drawFrame(ctx, set, frame, 10, WIRE + 6 - perchOf(set), SCALE);
      }
    }
  };
  // One timer at a time: every render of this page replaces it.
  window.clearInterval(previews);
  previews = undefined;
  paint();
  if (!matchMedia("(prefers-reduced-motion: reduce)").matches) previews = window.setInterval(paint, 100);
  return [
    el("h1", { text: t("Flock & Zeca") }),
    el(
      "section",
      { class: "card rows" },
      row(
        t("The flock draws from"),
        t("Where the sessions' birds come from: Brazil's vultures, the vultures of the Americas (with both condors), or every vulture in the world. Each project is one species, the same every time the app starts, and the projects on the wire differ while there are species left. A project with three or more sessions gets a king vulture either way."),
        segmented(FLOCKS.map((f) => ({ ...f, label: t(f.label) })), flock, async (f) => {
          await Bridge.setFlock(f);
          flock = f;
        }),
      ),
      row(
        t("Rare visitors"),
        t("Now and then, while sessions are open, a vulture from outside your flock (a condor, a griffon) crosses the sky once and goes on its way. It never lands."),
        toggle(visitors, async (on) => {
          await Bridge.setVisitors(on);
          visitors = on;
        }),
      ),
    ),
    el(
      "section",
      { class: "card rows" },
      row(
        "Zeca",
        t("The companion who chats and listens. Off, the flock, cards, notifications, connectors and independent local dictation keep working; Chat is unavailable."),
        toggle(zeca, async (on) => {
          await Bridge.setZeca(on);
          zeca = on;
          render();
        }),
      ),
      row(
        t("Look"),
        t("Auto dresses Zeca for the season: a witch hat from October 1 to November 1, a Santa hat from December 1 to 26, a party hat from New Year's Eve to January 2, bunny ears from Good Friday to Easter Monday. Sunglasses and the other outfits only when you pick them. Only Zeca wears it; the flock keeps its feathers."),
        el(
          "div",
          { class: "look-control" },
          zecaCanvas,
          dropdown(LOOKS, zecaLook, async (look) => {
            await Bridge.setZecaLook(look);
            zecaLook = look;
            shown.delete(zecaCanvas);
            labelPreview();
            paint();
          }),
        ),
      ),
    ),
    el("p", {
      class: "note",
      text: zeca
        ? t("Zeca is the bird in front: the session that needs you, or the one you picked. Choose his species.")
        : t("Zeca is off: the session in front keeps its own bird. His species and look wait for him here."),
    }),
    ...group("new-world", t("Vultures of the Americas")),
    ...group("old-world", t("Vultures of Africa, Europe and Asia")),
  ];
}

// ── Layout ───────────────────────────────────────────────────────────────────

/** The page last drawn: a re-render of it keeps the scroll, a new page starts at the top. */
let drawn: string | null = null;
const expanded = new Map<string, Set<string>>();

/** Draws the page again when it is the phone's (its rows refresh on their own). */
function renderPhone(): void {
  if (page === "phone") render();
}

function chooseDictation(section: DictationSetting): void {
  dictationSection = section;
  choosePage("dictation");
}

function choosePage(next: Page): void {
  page = next;
  navigated = true;
  location.hash = next === "dictation" ? `dictation${dictationSection}` : next;
  if (next === "dictation") void Bridge.rememberDictationSection(`dictation${dictationSection}`).catch(() => {});
  apiMessage = null;
  if (next === "agents") doctorAsked = false;
  if (next === "approvals") {
    void refreshRules();
    void refreshPolicies(() => page === "approvals" && render());
  }
  if (next === "projects") void refreshProjects();
  if (next === "activity") {
    clearing = false;
    void refreshActivity();
    budget.refresh();
  }
  if (next === "chat") void refreshApi();
  if (next === "phone") void refreshPhone(renderPhone);
  if (next === "dictation" && dictationSection === "voice" && microphones === null && !microphonesFailed) void refreshMicrophones();
  render();
}

const dictationNavigation = createDictationNavigation("dictationvoice", id => void Bridge.openNest(`dictation-${id}`), chooseDictation, () => void Bridge.openNest("dictation-statistics"));

function render(): void {
  const place = page === "dictation" ? `dictation${dictationSection}` : page;
  if (page === "activity") budget.relabel();
  const scroll = place === drawn ? (root.querySelector("main.page")?.scrollTop ?? 0) : 0;
  if (drawn) expanded.set(drawn, new Set([...root.querySelectorAll<HTMLDetailsElement>("details")].filter(node => node.open).map(node => node.dataset.setting ?? "")));
  const active = document.activeElement as HTMLInputElement | HTMLTextAreaElement | null;
  const focus = drawn === place && active && root.contains(active) && active.dataset.setting ? active : null;
  const editable = focus && (focus.tagName === "TEXTAREA" || (focus.tagName === "INPUT" && !["checkbox", "range"].includes(focus.type))) && !focus.readOnly;
  questionAgents.visible(page === "dictation" && dictationSection === "agents" && !document.hidden);
  questionAgents.relabel();
  meetingAssets.visible(page === "dictation" && dictationSection === "meetings" && !document.hidden);
  realtimeSettings.visible(page === "dictation" && dictationSection === "conversation" && !document.hidden);
  meetingSettings.visible(page === "dictation" && dictationSection === "meetings" && !document.hidden);
  meetingSettings.relabel();
  realtimeSettings.relabel();
  cleanupSettings.visible(page === "dictation" && dictationSection === "styles" && !document.hidden);
  cleanupSettings.relabel();
  meetingAssets.relabel();
  dictationDoctor.visible(page === "dictation" && dictationSection === "diagnostics" && !document.hidden);
  dictationDoctor.relabel();
  head.set(page === "dictation" ? "dictation" : "settings");
  head.setZeca(zeca);
  head.setSound(sounds);
  foot.setZeca(zeca);
  foot.setSpecies(zecaSpecies);
  const content =
    page === "general"
      ? generalPage()
      : page === "dictation"
        ? dictationPage()
      : page === "agents"
        ? agentsPage()
        : page === "chat"
          ? chatPage()
          : page === "approvals"
            ? approvalsPage()
            : page === "projects"
              ? projectsPage()
            : page === "activity"
              ? [el("h1", { text: t("Activity") }),
                el("p", { class: "lede", text: t("Choose what stays on this computer. Recaps and records are in Activity.") }),
                el("div", { class: "actions" }, button(t("Open Activity"), () => void Bridge.openNest("week"))),
                budget.element,
                historyCard(activity, { confirming: clearing }, activityActions)]
            : page === "connectors"
              ? connectorsPage()
            : page === "phone"
              ? phonePage()
              : page === "flock"
                ? flockPage()
                : aboutPage();
  if (page !== "flock") window.clearInterval(previews);
  const navButton = (p: (typeof PAGES)[number]) => {
    const b = el(
      "button",
      {
        class: `nav${p.id === page ? " on" : ""}`,
        onclick: () => choosePage(p.id),
      },
      icon(p.icon, 14, 1.6),
      el("span", { text: t(p.label) }),
    );
    if (p.id === page) b.setAttribute("aria-current", "page");
    return b;
  };

  // The Nest's header above already says whose settings these are: groups, then pages.
  const nav = el(
    "nav",
    { class: "sidebar" },
    ...GROUPS.flatMap((g) => [el("p", { class: "nav-group", text: t(g.label) }), ...PAGES.filter((p) => p.group === g.id).map(navButton)]),
  );
  sidePanel(nav, "settings", "Settings", 200, "left", 160, 280);
  const picker = el("select", { class: "settings-page-picker" });
  picker.setAttribute("aria-label", t("Settings page"));
  for (const p of PAGES) {
    const option = el("option", { text: t(p.label) });
    option.value = p.id;
    option.selected = p.id === page;
    picker.append(option);
  }
  picker.addEventListener("change", () => choosePage(picker.value as Page));
  const title = content.find(node => node.tagName === "H1");
  const intro = content.find(node => node.classList.contains("lede"));
  const main = el("main", { class: `page${place !== drawn ? " page-enter" : ""}` },
    el("header", { class: "settings-heading" }, el("div", { class: "settings-title-line" }, icon(page === "dictation" ? DICTATION_SETTINGS.find(p => p.id === dictationSection)!.icon : PAGES.find(p => p.id === page)!.icon, 20, 1.6), title ?? null), intro ?? null),
    ...content.filter(node => node !== title && node !== intro));
  for (const node of main.querySelectorAll<HTMLDetailsElement>("details")) node.open = expanded.get(place)?.has(node.dataset.setting ?? "") ?? false;
  root.classList.toggle("dictation-workspace", page === "dictation");
  if (page === "dictation") {
    dictationNavigation.select(`dictation${dictationSection}`);
    main.id = "nest-pane-dictation";
    main.setAttribute("role", "tabpanel"); main.setAttribute("aria-labelledby", "nest-tab-dictation");
    main.tabIndex = -1;
    root.replaceChildren(dictationNavigation.element, main);
  } else root.replaceChildren(picker, nav, main);
  main.scrollTop = scroll;
  if (focus) {
    const next = [...root.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("[data-setting]")].find(node => node.dataset.setting === focus.dataset.setting && node.tagName === focus.tagName);
    if (next) {
      if (editable) next.value = focus.value;
      next.focus({ preventScroll: true });
      if (editable && focus.selectionStart !== null) next.setSelectionRange(focus.selectionStart, focus.selectionEnd);
    }
  }
  if (focusDestination) { main.focus({ preventScroll: true }); focusDestination = false; }
  drawn = place;
  const logo = root.querySelector<HTMLImageElement>("img.logo");
  if (logo) logo.src = "/icon.png";
}

render();
if (page === "dictation") {
  void Bridge.rememberDictationSection(`dictation${dictationSection}`).catch(() => {});
  if (dictationSection === "voice") void refreshMicrophones();
}
// By the panel the open island may cover this window's corner: a click here folds it, as a click
// outside a panel's popup closes it (the island keeps a waiting card).
window.addEventListener("pointerdown", () => void Bridge.away().catch(() => {}), { capture: true });
if (page === "activity") { void refreshActivity(); budget.refresh(); }
if (page === "phone") void refreshPhone(renderPhone);
Bridge.onPhone(phoneEvent);
watchWidth(() => { if (page === "general") render(); });

void Bridge.appSettings().then((s) => {
  dictationCaption = dictation.captions(s.dictationCaption, true);
  dictationSound = dictation.startSound(s.dictationSound, true);
  if (!dictationCommandsChanged) dictationCommands = s.dictationCommands;
  if (!dictationCommandChatChanged) dictationCommandChat = s.dictationCommandChat;
  if (!dictationKeepChanged) dictationKeep = s.dictationKeep;
  if (!dictationMicrophoneChanged) dictationMicrophone = s.dictationMicrophone;
  if (!dictationCutRepeatsChanged) dictationCutRepeats = s.dictationCutRepeats;
  if (!dictationRetryBaseChanged) dictationRetryBase = s.dictationRetryBase;
  if (!dictationVocabularyChanged) dictationVocabulary = s.dictationVocabulary ?? false;
  if (!dictationProjectVocabularyChanged) dictationProjectVocabulary = s.dictationProjectVocabulary ?? false;
  if (!dictationSpokenCodeChanged) dictationSpokenCode = s.dictationSpokenCode;
  if (!dictationMusicChanged) dictationMusic = s.dictationMusic;
  if (!dictationCancelByVoiceChanged) dictationCancelByVoice = s.dictationCancelByVoice;
  if (!dictationDoubleTapChanged) dictationDoubleTap = s.dictationDoubleTap;
  if (!dictationSpaceLockChanged) dictationSpaceLock = s.dictationSpaceLock;
  if (!dictationEscapeCancelChanged) dictationEscapeCancel = s.dictationEscapeCancel;
  sounds = s.sounds;
  volume = savedVolume = s.volume;
  autostart = s.autostart;
  // The nearest choice: the file may hold any number in range.
  foldAfter = FOLD_CHOICES.reduce((a, b) => (Math.abs(b - s.foldAfter) < Math.abs(a - s.foldAfter) ? b : a));
  openOnHover = s.openOnHover;
  language = s.language;
  theme = applyTheme(s.theme ?? "black", true);
  setLang(s.lang);
  dictation.relabel();
  head.relabel();
  foot.relabel();
  monitor = s.monitor;
  nowPlaying = s.nowPlaying;
  codexUsage = s.codexUsage;
  nativeChat = s.nativeChat;
  if (!dictationInsertChanged) dictationInsert = s.dictationInsert;
  dictationInsertAvailable = s.dictationInsertAvailable;
  dictationWayland = s.dictationWayland;
  chatAccess = s.chatAccess;
  memory = s.memory;
  journalSummary = s.journalSummary;
  localEmbeddings = s.localEmbeddings;
  memorySuggest = s.memorySuggest;
  sleep = s.sleep;
  zecaSpecies = s.zecaSpecies;
  zecaLook = s.zecaLook;
  flock = s.flock;
  visitors = s.visitors;
  presence = s.presence;
  notifications = s.notifications;
  dndUntil = s.dndUntil;
  zeca = s.zeca;
  widget = s.widget;
  settingsPath = s.settingsPath;
  dataPath = s.dataPath;
  render();
});
void refreshVoice();
Bridge.onVoiceDownload((p) => {
  const percent = Math.floor((p.done / p.total) * 100);
  // Thousands of chunks: only a new percent repaints.
  if (!downloading.has(p.id) || downloading.get(p.id) === percent) return;
  downloading.set(p.id, percent);
  render();
});
const refreshMonitors = () =>
  Bridge.monitors()
    .then((m) => {
      monitors = m;
      if (page === "general") render();
    })
    .catch(() => {});
void refreshMonitors();
// The tray asks an open window for one page (Set up agents…).
Bridge.onSettingsSection((section) => {
  const p = asked(section);
  if (p) {
    focusDestination = true;
    const selected = dictationSetting(section);
    if (selected) chooseDictation(selected); else choosePage(p);
  }
});
// The island's speaker button changes the sounds too: keep the toggle and the slider in step.
Bridge.onSettings((s) => {
  if (s.questionAgents !== undefined) questionAgents.changed(s.questionAgents);
  cleanupSettings.update(s);
  meetingSettings.update(s);
  if (s.voiceRealtime !== undefined) realtimeSettings.changed();
  if (s.dictationMusic !== undefined) { dictationMusic = s.dictationMusic; dictationMusicChanged = true; if (page === "dictation") render(); }
  if (s.dictationCancelByVoice !== undefined) { dictationCancelByVoice = s.dictationCancelByVoice; dictationCancelByVoiceChanged = true; if (page === "dictation") render(); }
  if (s.dictationSpokenCode !== undefined) { dictationSpokenCode = s.dictationSpokenCode; dictationSpokenCodeChanged = true; if (page === "dictation") render(); }
  if (s.dictationCutRepeats !== undefined) { dictationCutRepeats = s.dictationCutRepeats; dictationCutRepeatsChanged = true; if (page === "dictation") render(); }
  if (s.dictationVocabulary !== undefined) { dictationVocabulary = s.dictationVocabulary; dictationVocabularyChanged = true; if (page === "dictation") render(); }
  if (s.dictationProjectVocabulary !== undefined) { dictationProjectVocabulary = s.dictationProjectVocabulary; dictationProjectVocabularyChanged = true; if (page === "dictation") render(); }
  if (s.dictationRetryBase !== undefined) { dictationRetryBase = s.dictationRetryBase; dictationRetryBaseChanged = true; if (page === "dictation") render(); }
  if (s.dictationMicrophone !== undefined) { dictationMicrophone = s.dictationMicrophone; dictationMicrophoneChanged = true; if (page === "dictation") render(); }
  if (s.dictationDoubleTap !== undefined) { dictationDoubleTap = s.dictationDoubleTap; dictationDoubleTapChanged = true; if (page === "dictation") render(); }
  if (s.dictationSpaceLock !== undefined) { dictationSpaceLock = s.dictationSpaceLock; dictationSpaceLockChanged = true; if (page === "dictation") render(); }
  if (s.dictationCommands !== undefined) { dictationCommands = s.dictationCommands; dictationCommandsChanged = true; if (page === "dictation") render(); }
  if (s.dictationCommandChat !== undefined) { dictationCommandChat = s.dictationCommandChat; dictationCommandChatChanged = true; if (page === "dictation") render(); }
  if (s.dictationEscapeCancel !== undefined) { dictationEscapeCancel = s.dictationEscapeCancel; dictationEscapeCancelChanged = true; if (page === "dictation") render(); }
  if (s.dictationKeep !== undefined) { dictationKeep = s.dictationKeep; dictationKeepChanged = true; if (page === "dictation") render(); }
  if (s.dictationSound !== undefined) { dictationSound = dictation.startSound(s.dictationSound); if (page === "dictation") render(); }
  if (s.dictationCaption !== undefined) { dictationCaption = dictation.captions(s.dictationCaption); if (page === "dictation") render(); }
  applyTheme(s.theme);
  if (s.dictationInsert !== undefined) { dictationInsert = s.dictationInsert; dictationInsertChanged = true; if (page === "dictation") render(); }
  if (s.theme !== undefined) { theme = s.theme; render(); }
  // Changed from the island's gear: this page shows it too.
  if (s.flock !== undefined && s.flock !== flock) {
    flock = s.flock;
    render();
  }
  if (s.history !== undefined && activity && activity.history !== s.history) {
    activity = { ...activity, history: s.history };
    render();
  }
  if (s.lang !== undefined) {
    setLang(s.lang);
    dictation.relabel();
    head.relabel();
    foot.relabel();
    if (s.language !== undefined) language = s.language;
    render();
  }
  if (s.zeca !== undefined) {
    zeca = s.zeca;
    render();
  }
  if (s.zecaLook !== undefined && s.zecaLook !== zecaLook) {
    zecaLook = s.zecaLook;
    render();
  }
  if (s.presence !== undefined) {
    presence = s.presence;
    render();
  }
  // The chat's own picker gave some access back.
  if (s.chatAccess !== undefined && s.chatAccess !== chatAccess) {
    chatAccess = s.chatAccess;
    if (page === "chat") render();
  }
  if (s.dndUntil !== undefined) {
    dndUntil = s.dndUntil;
    if (page === "general") render();
  }
  // A quick action on the island changed a project.
  if (s.projects !== undefined) {
    projects = s.projects;
    if (page === "projects") render();
  }
  if (s.sounds === undefined && s.volume === undefined) return;
  if (s.sounds !== undefined) sounds = s.sounds;
  if (s.volume !== undefined) volume = savedVolume = s.volume;
  render();
});
Bridge.onMonitors(() => void refreshMonitors());
for (const a of AGENTS) void refresh(a.kind);
void refreshConnectors();
void refreshRules();
void refreshPolicies(() => {
  if (page === "approvals") render();
});
void refreshProjects();
void refreshApi();
void getVersion()
  .then((v) => {
    version = v;
    if (page === "about") render();
  })
  .catch(() => {});
// Statuses age and polls finish in the background; a hidden window reads them when it shows.
window.setInterval(() => {
  if (!document.hidden) void refreshConnectors();
}, 15_000);
window.addEventListener("pagehide", () => { realtimeSettings.visible(false); questionAgents.visible(false); dictationDoctor.visible(false); meetingAssets.visible(false); meetingSettings.visible(false); });
document.addEventListener("visibilitychange", () => {
  questionAgents.visible(page === "dictation" && dictationSection === "agents" && !document.hidden);
  meetingAssets.visible(page === "dictation" && dictationSection === "meetings" && !document.hidden);
  realtimeSettings.visible(page === "dictation" && dictationSection === "conversation" && !document.hidden);
  meetingSettings.visible(page === "dictation" && dictationSection === "meetings" && !document.hidden);
  cleanupSettings.visible(page === "dictation" && dictationSection === "styles" && !document.hidden);
  dictationDoctor.visible(page === "dictation" && dictationSection === "diagnostics" && !document.hidden);
  if (!document.hidden) void refreshConnectors();
});
// An agent's files change outside the app (a relay built, a hook edited by hand): read them again
// when the window comes back, unless a review is open (it would lose its diff).
window.addEventListener("focus", () => {
  for (const a of AGENTS) if (!panels.get(a.kind)?.pending) void refresh(a.kind);
});
