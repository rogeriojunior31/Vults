import { createDictationNavigation, DICTATION_SETTINGS, dictationSetting, dictationSettingAbout, type DictationSetting } from "../nest/dictation/navigation";
import { VOICE_MODEL_LABELS } from "../../voice-command-readiness";
import { failure } from "../../errors";
import { dictationStatus } from "../../island/dictation";
import { sidePanel } from "../nest/panels";
import { createAudit } from "../nest/records";
import { applyTheme, DEFAULT_THEME } from "../../theme";
import { ISLAND_WIDTHS, restoreWidth, saveWidth, watchWidth } from "../../island/geometry";
// Settings, a webview of the Nest's window (ADR 0006): a sidebar and one page per section. Installing hooks always goes through a
// diff the user reviews first.
import { getVersion } from "@tauri-apps/api/app";
import { Bridge, type WebEngine, type WebSettings, type ApiProvider, type ConnectorStatus, type Flock, type InstallAgent, type InstallPreview, type InstallStatus, type Corner, type Presence, type ProjectPrefs, type Rule, type VoiceStatus, type ChatAccess, type UpdateStatus, type DictationPermission, type DictationMusic, type Theme } from "../../bridge";
import { SPECIES, speciesSet } from "../../character/flock";
import { LOOK_GROUPS, dress } from "../../character/looks";
import { drawFrame, frameAt, type Frame } from "../../character/sprites";
import { perchOf } from "../../character/zeca";
import { AGENTS as REGISTRY } from "../../agents";
import { acpSummary, apiSummary, dot, engineSummary } from "./provider-status";
import { brandMark } from "../../brands";
import { LABS, labOn } from "../../labs";
import { CONNECTORS, connectorError } from "../../connectors";
import { el } from "../../dom";
import { button, confirmed, diff, disclosure, row, saveSetting, toggle } from "./pieces";
import { policiesSection, policyCount, policyEditing, refreshPolicies } from "./policies";
import { createDoctor } from "./doctor";
import { createMeetingAssets } from "./meeting-assets";
import { createRealtimeSettings } from "./realtime";
import { createMeetingSettings } from "./meeting-summary";
import { createCleanupSettings } from "./dictation-styles";
import { createQuestionAgents } from "./question-agents";
import { createDictationDoctor } from "./dictation-doctor";
import { createWindowsPlayerAudio } from "./windows-player-audio";
import { createGitea } from "./gitea";
import { createGithubCheck } from "./github-check";
import { createWindowsShortcuts } from "./windows-shortcuts";
import { LANGUAGES as APP_LANGUAGES, locale, setLang, t, tk } from "../../i18n";
import { historyCard, type Activity, type ActivityActions } from "./activity";
import { createBudget } from "./budget";
import { phoneEvent, phonePage, refreshPhone } from "./phone";
import { Sound } from "../../sound";
import { icon, type IconName } from "../../island/icons";
import { createNestFoot, createNestHead, tabByKey } from "../nest/head";
import { needsYou } from "../nest/flock";
import type { SessionView } from "../../view.gen";

type Page = "zeca" | "dictation" | "general" | "appearance" | "island" | "sound" | "shortcuts" | "agents" | "chat" | "approvals" | "projects" | "activity" | "connectors" | "phone" | "flock" | "labs" | "about";

/** The sidebar's groups, in the prototype's order: how it feels, the work, the system. */
const GROUPS = [
  { id: "companion", label: tk("Zeca, chat and agents") },
  { id: "experience", label: tk("Experience") },
  { id: "work", label: tk("Work") },
  { id: "system", label: tk("System") },
] as const;

const PAGES: { id: Page; label: string; group: (typeof GROUPS)[number]["id"]; icon: IconName }[] = [
  { id: "zeca", label: "Zeca", group: "companion", icon: "bird" },
  { id: "chat", label: tk("Chat & voice"), group: "companion", icon: "chat" },
  { id: "agents", label: tk("Agents"), group: "companion", icon: "terminal" },
  { id: "general", label: tk("General"), group: "experience", icon: "gear" },
  { id: "appearance", label: tk("Appearance"), group: "experience", icon: "palette" },
  { id: "island", label: tk("Island"), group: "experience", icon: "island" },
  { id: "sound", label: tk("Sound and notifications"), group: "experience", icon: "sound" },
  { id: "shortcuts", label: tk("Keyboard shortcuts"), group: "experience", icon: "keyboard" },
  { id: "dictation", label: tk("Dictation"), group: "experience", icon: "mic" },
  { id: "flock", label: tk("Flock"), group: "experience", icon: "flock" },

  { id: "approvals", label: tk("Approvals"), group: "work", icon: "check" },
  { id: "projects", label: tk("Projects"), group: "work", icon: "folder" },
  { id: "activity", label: tk("History and budget"), group: "system", icon: "bars" },
  { id: "connectors", label: tk("Connection setup"), group: "system", icon: "plug" },
  { id: "phone", label: tk("Phone"), group: "system", icon: "phone" },
  { id: "labs", label: tk("Labs"), group: "system", icon: "flask" },
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
const gitea = createGitea({ read: Bridge.giteaConnection, save: Bridge.setGiteaConnection });
const githubCheck = createGithubCheck({ check: Bridge.checkGithubAccess }, () => {
  const st = connectorStatus.get("github");
  return st?.account && st.host ? `${st.account} (${st.host})` : null;
});
window.addEventListener("pagehide", () => gitea.hide());
document.addEventListener("visibilitychange", () => gitea.context(page === "connectors" && !document.hidden, connectorStatus.get("gitea")?.enabled ?? false));
let boundShortcuts: Record<string, string> = {};
const boundKeys = (...ids: string[]) => ids.map(id => boundShortcuts[id]?.trim() || t("Not bound")).join(" · ");
const keepShortcuts = (keys: Record<string, string>) => {
  boundShortcuts = keys;
  if (page === "general" || page === "appearance" || page === "island" || page === "sound" || page === "shortcuts" || page === "dictation") render();
};
Bridge.onShortcutKeys(keepShortcuts);
void Bridge.shortcutKeys().then(keepShortcuts, () => {});
const windowsShortcuts = createWindowsShortcuts({ read: Bridge.windowsShortcuts, save: Bridge.setWindowsShortcuts }, () => {
  if (page === "general" || page === "appearance" || page === "island" || page === "sound" || page === "shortcuts" || page === "dictation") render();
});
windowsShortcuts.initialize();
const playerAudio = createWindowsPlayerAudio({ status: Bridge.mediaAudioStatus, list: Bridge.mediaAudioChoices, select: Bridge.mediaAudioSelect }, () => { if (page === "dictation") render(); });


Bridge.onMediaAudioCleared(() => playerAudio.changed());

const budget = createBudget({ today: Bridge.spendToday, setBudget: Bridge.budgetSet });
Bridge.onSpend(() => { if (page === "activity") budget.refresh(); });
// Settings is a webview of the Nest's window (ADR 0006), under the Nest's own header: a tab goes
// back to the Nest's webview, the size switch keeps Settings in front.
let nestFull = false;
const head = createNestHead({
  tab: (id) => { void Bridge.openNest(id); },
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
  // Back to the island is Quick: the Nest closes and the island opens (ADR 0006).
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
    void Bridge.openNest(pick);
  } else if (e.key === "Escape" && nestFull && !e.defaultPrevented && !e.isComposing && !(e.target as Element | null)?.closest?.("input, textarea, select, [contenteditable]")) {
    e.preventDefault();
    void Bridge.setSize("room", "settings");
  }
});
const asked = (section: string): Page | null => section === "chatmemory" ? "zeca" : dictationSetting(section) ? "dictation" : PAGES.find((p) => p.id === section)?.id ?? null;
/** The page the app asked for (the tray's Set up agents…), else General until the agents are read:
 *  Agents when one has no hooks yet or older ones (`chooseStart`). */
let focusDestination = dictationSetting(location.hash.slice(1)) !== null || location.hash.slice(1) === "chatmemory";
let chatSection: "memory" | null = location.hash.slice(1) === "chatmemory" ? "memory" : null;
let dictationSection: DictationSetting = dictationSetting(location.hash.slice(1)) ?? "voice";
const linked = asked(location.hash.slice(1));
let page: Page = linked ?? "general";
/** The user went to a page: the start page no longer moves. */
let navigated = linked !== null;
const panels = new Map<InstallAgent, Panel>(AGENTS.map((a) => [a.kind, { status: null, message: null, pending: null }]));
let connectorStatus = new Map<string, ConnectorStatus>();
/** Read once at least, and whether the last read failed: until then a switch is unknown, not off. */
let connectorsRead = false;
let connectorsFailed = false;
let sounds = true;
/** Percent, as the slider shows it: a repaint mid-drag keeps the drag. */
let volume = 50;
/** Percent, as last saved: a failed save goes back to it. */
let savedVolume = 50;
let nowPlaying = false;
let codexUsage = false;
let claudeUsage = false;
let priceUpdates = false;
/** The update notice; null until read. Distro packages say it is not available. */
let update: UpdateStatus | null = null;
let priceStatus: import("../../bridge").ModelPriceStatus | null = null;
let nativeChat = true;
let dictationInsert = false;
let dictationCaption = true;
let dictationCommands = true, dictationCommandChat = false;
let dictationCommandMedia = false, dictationCommandMediaChanged = false;
let dictationCommandApps = false, dictationCommandAppsChanged = false;
let dictationCommandOutput = false, dictationCommandOutputChanged = false;
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
  tk("Insertion is off, but its saved keyboard token could not be removed. Turn insertion off again."),
  tk("The keyboard permission worker is unavailable."),
  tk("Enable dictation insertion in Settings first."),
  tk("Dictation insertion is unavailable."),
  tk("Windows focus tracking is unavailable."),
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
/** Who answers the chat's cards (ADR 0002). */
let chatAccess: ChatAccess = "ask";
/** Zeca's memory (M4): the journal, and each day summed up by a small model. */
let memory = false;
let journalSummary = false;
let localEmbeddings = false;
let memorySuggest = false;
let sleep = false;
let antigravityApprovals = false;
/** Settings → Labs: the beta features turned on, by key. */
let labs: Record<string, boolean> = {};
let voice: VoiceStatus | null = null;
/** Model id → percent downloaded, for every download running. */
const downloading = new Map<string, number>();
let voiceError: string | null = null;
let autostart = false;
let keepAwake = true;
let foldAfter = 15;
let openOnHover = false;
/** The language chosen, or null to follow the system. */
let language: string | null = null;
let theme: Theme = DEFAULT_THEME;
/** Why the last theme change did not hold, until the next one. */
let themeNote: string | null = null;
/** A theme being saved: the tiles wait, so two clicks never race. */
let themeSaving = false;
/** SP Night's flavors keep their names in every language. */
const THEMES: [Theme, string][] = [
  ["dark", tk("Dark")],
  ["light", tk("Light")],
  ["sp-night-paulista", "SP Night: Noite Paulista"],
  ["sp-night-garoa", "SP Night: Garoa"],
  ["sp-night-jaragua", "SP Night: Pico do Jaraguá"], // check-english:allow
];
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
let quietWhenWatching = true;
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
/** Null until read: an unread list is unknown, never "nothing". A failed read keeps the last one. */
let rules: Rule[] | null = null;
let rulesFailed = false;
let rulesNote: string | null = null;
let projects: Record<string, ProjectPrefs> | null = null;
let projectsFailed = false;
let projectsNote: string | null = null;
/** Full access waits for a second, explicit click before it is saved. */
let askingFull = false;
let accessNote: string | null = null;

/** A list that is being read, or could not be: the page says so instead of showing it empty. */
function readState(failed: boolean, reading: string, failure: string, retry: () => void): HTMLElement {
  const card = el(
    "section",
    { class: "card read-state" },
    el("p", { class: failed ? "note error" : "note", text: failed ? failure : reading }),
    failed ? button(t("Try again"), retry) : null,
  );
  card.setAttribute("role", failed ? "alert" : "status");
  return card;
}
let apiProviders: ApiProvider[] = [];
/** The chat's engines as last checked, and when; read when Chat & voice opens, again on request. */
let engines: import("../../bridge").ChatEngines | null = null;
let enginesLoading = false;
let enginesNote: string | null = null;
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
    rulesFailed = false;
  } catch {
    rulesFailed = true;
  }
  if (page === "approvals") render();
}

function approvalsPage(): HTMLElement[] {
  const name = (cwd: string) => cwd.split(/[\\/]/).filter(Boolean).pop() ?? cwd;
  // What answers without a card, each behind one line: the log is what people come here to read.
  const group = (id: string, title: string, open: boolean, ...body: (HTMLElement | null)[]) => {
    const d = el("details", { class: "approvals-group" }, el("summary", { text: title }), ...body);
    d.dataset.setting = id;
    d.open = open;
    return d;
  };
  const ruleCount = rules?.length;
  return [
    el("h1", { text: t("Approvals") }),
    el("p", { class: "lede", text: t("Every answer a card got, and what answers without one.") }),
    // What answers without a card is not only these rules: Zeca's chat access says so here too.
    ...(chatAccess === "ask" ? [] : [el("div", { class: "approvals-banner" },
      icon("warning", 14, 2),
      el("span", { text: chatAccess === "full"
        ? t("Full access: Zeca's chat runs commands and edits without a card. Only a policy's deny still refuses.")
        : t("Zeca's chat follows your policies: what they allow runs without a card.") }),
      button(t("Chat & voice"), () => choosePage("chat")))]),
    el("h2", { class: "settings-group-title", text: t("What was answered") }),
    el("p", { class: "lede", text: t("Every card's answer and who gave it. Kept 90 days, never changed.") }),
    answersLog.element,
    el("h2", { class: "settings-group-title", text: t("What answers without a card") }),
    group("approvals-rules", ruleCount === undefined ? t("Always allowed") : t("Always allowed ({n})", { n: ruleCount }), !!rulesNote || rulesFailed,
    el("p", {
      class: "lede",
      text: t("What you allowed with Always on a card. Each covers only that exact command or file, agent and folder."),
    }),
    ...(rulesFailed
      ? [readState(true, "", t("Could not read your always-allowed permissions, so what is allowed is unknown."), () => void refreshRules())]
      : []),
    ...(rulesNote ? [el("p", { class: "note error", text: rulesNote })] : []),
    ...(!rules
      ? rulesFailed
        ? []
        : [readState(false, t("Reading your always-allowed permissions…"), "", () => {})]
      : [rules.length
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
              confirmed(t("Remove"), t("This permission asks again on its next request."), t("Remove permission"), () =>
                Bridge.ruleRemove(i).then(
                  () => {
                    rulesNote = null;
                    return refreshRules();
                  },
                  () => {
                    rulesNote = t("Could not remove the permission. It still applies.");
                    render();
                  },
                ),
              ),
            ),
          ),
        )
      : el("section", { class: "card" }, el("p", { class: "note", text: t("No Always rule. Every agent permission asks.") }))])),
    group("approvals-policies", t("Policies ({n})", { n: policyCount() }), policyEditing(), ...policiesSection(render, false)),
  ];
}

/** The audit log, read when Approvals opens; it keeps its filter and search while the page repaints. */
const answersLog = createAudit(() => Bridge.nestAudit(), undefined, false);

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
    projectsFailed = false;
  } catch {
    projectsFailed = true;
  }
  if (page === "projects") render();
}

/** Mute, pin and hide, per project folder: set from a session's quick actions on the island. */
function projectsPage(): HTMLElement[] {
  const name = (cwd: string) => cwd.split(/[\\/]/).filter(Boolean).pop() ?? cwd;
  const known = projects ?? {};
  const folders = Object.keys(known).sort((a, b) => name(a).localeCompare(name(b)));
  const set = (cwd: string, change: ProjectPrefs) => async (on: boolean) => {
    const next = { ...known[cwd], ...Object.fromEntries(Object.keys(change).map((k) => [k, on])) };
    await Bridge.projectSet(cwd, next);
    await refreshProjects();
  };
  const choice = (label: string, on: boolean, change: (on: boolean) => Promise<void>) =>
    el("label", { class: "project-choice" }, toggle(on, change), el("span", { text: label }));
  return [
    el("h1", { text: t("Projects") }),
    el("p", {
      class: "lede",
      text: t("Choices per project folder, for all its sessions. Right-click a session on the island to set them."),
    }),
    el("details", { class: "row-more" }, el("summary", { text: t("What each choice does") }), el("p", { text: t("Muted: no sounds or notifications, except on cards. Pinned: shown first. Hidden: off the island, except its cards. Bird: one species for all its sessions, or Automatic.") })),
    ...(projectsFailed
      ? [readState(true, "", t("Could not show your project choices, but they still apply."), () => void refreshProjects())]
      : []),
    ...(projectsNote ? [el("p", { class: "note error", text: projectsNote })] : []),
    ...(!projects
      ? projectsFailed
        ? []
        : [readState(false, t("Reading your project choices…"), "", () => {})]
      : [folders.length
      ? el(
          "section",
          { class: "card rows" },
          ...folders.map((cwd) => {
            const p = known[cwd];
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
                    await Bridge.projectSet(cwd, { ...known[cwd], species: species ?? undefined });
                    await refreshProjects();
                  },
                ),
                confirmed(t("Forget"), t("This project's choices are cleared."), t("Forget project"), () =>
                  Bridge.projectSet(cwd, {}).then(
                    () => {
                      projectsNote = null;
                      return refreshProjects();
                    },
                    () => {
                      projectsNote = t("Could not forget the project. Its choices still apply.");
                      render();
                    },
                  ),
                ),
              ),
            );
          }),
        )
      : el("section", { class: "card" }, el("p", { class: "note", text: t("No project choices yet. Every session shows in arrival order, with sounds.") }))]),
  ];
}

// ── Pieces ───────────────────────────────────────────────────────────────────


/** A row of choices, one on. */
/** Full access, asked: what it stops asking, Cancel (focused) and the grant. */
function fullAccessConfirm(warning: string): HTMLElement {
  const about = el("p", { class: "note warn with-icon" }, icon("warning", 14, 2), el("span", { text: warning }));
  about.id = "full-access-effect";
  const stop = () => {
    askingFull = false;
    render();
  };
  const grant = el("button", { class: "btn danger", text: t("Turn on full access") });
  grant.setAttribute("aria-describedby", about.id);
  grant.onclick = () => {
    grant.disabled = true;
    Bridge.setChatAccess("full").then(
      () => {
        chatAccess = "full";
        askingFull = false;
        render();
      },
      () => {
        askingFull = false;
        accessNote = t("Could not turn on full access. Zeca's cards still ask.");
        render();
      },
    );
  };
  const box = el("div", { class: "row access-confirm" }, el("div", { class: "row-text" }, about), el("div", { class: "actions" }, button(t("Cancel"), stop), grant));
  box.addEventListener("keydown", (e) => {
    if (e.key !== "Escape") return;
    e.stopPropagation();
    stop();
  });
  return box;
}

function segmented<T>(choices: { value: T; label: string }[], current: T, change: (v: T) => Promise<void>, gate?: (v: T) => boolean): HTMLElement {
  const box = el("div", { class: "segmented" });
  const buttons = choices.map(c => {
    const button = el("button", { text: c.label, onclick: () => {
      if (button.getAttribute("aria-pressed") === "true" || box.getAttribute("aria-busy") === "true") return;
      if (gate && !gate(c.value)) return;
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
    panel.message = { text: failure(t("Could not read this agent's setup."), e), error: true };
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
    panel.message = { text: failure(t("Could not prepare the review."), e), error: true };
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
    panel.message = { text: failure(t("Could not write the file."), e), error: true };
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

/** How an agent works with Vults: read on request, so its card shows its state first. */
function howItWorks(text: string): HTMLElement {
  return disclosure(t("How it works"), el("p", { class: "note", text }));
}

/** A step that undoes (Remove): present, never the eye's first stop. */
function quiet(b: HTMLButtonElement): HTMLButtonElement {
  b.classList.add("quiet");
  return b;
}

function agentCard(kind: InstallAgent, name: string, found = true): HTMLElement {
  const { status: s, message, pending } = panels.get(kind)!;
  const notice = message ? el("p", { class: message.error ? "note error" : "note ok", text: message.text }) : null;
  if (!s) return el("section", { class: "card" }, el("div", { class: "card-title", text: name }), notice);

  const codexHelp = s.codex?.error
    ? el("p", { class: "note error", text: t("Vults can't read Codex's config.toml, so it can't see Codex's hooks: {error}", { error: s.codex.error }) })
    : s.codex && s.installed && !s.codex.hooksDisabled && s.codex.untrusted > 0
      ? el("p", {
          class: "note warn",
          text: t("Codex runs only hooks you trust and turn on: open Codex, type /hooks and trust or turn on the {n} Vults hooks.", { n: s.codex.untrusted }),
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
            ? t("These hooks report to another copy of Vults, at {path}. Update them to use this app.", { path: s.otherHookPath })
            : kind === "claude"
              ? t("These hooks are outdated. Update them to answer Claude Code from the island.")
              : kind === "opencode"
                ? t("This plugin is outdated. Update it, then restart OpenCode.")
                : t("These hooks are outdated. Update them to get every island feature."),
        })
      : null;
  // The plan's usage reaches the island only through a statusLine of ours.
  const usageHelp =
    s.installed && s.statusLine === "none"
      ? howItWorks(t("Reinstall the hooks to see your plan's usage on the island. It adds a hidden status line to Claude Code."))
      : s.statusLine === "theirs"
        ? el("p", {
            class: "note",
            text: s.installed
              ? t("Reinstall the hooks to see your plan's usage on the island too. Your status line stays; removing the hooks restores it.")
              : t("Installing the hooks also shows your plan's usage on the island. Your status line stays; removing the hooks restores it."),
          })
        : null;
  const review = pending
    ? el(
        "div",
        { class: "review" },
        el("div", { class: "review-title", text: pending.install ? t("Review the install") : t("Review the removal") }),
        pending.preview.diff ? diff(pending.preview.diff) : howItWorks(t("Nothing to change.")),
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
    el("div", { class: "card-head" }, el("div", { class: "card-title brand-title" }, brandMark(kind, name, 20), el("span", { text: name })), agentStatus(s)),
    el("div", { class: "path", text: s.configPath }),
    s.hookReady ? null : el("p", { class: "note error", text: t("The hook relay is missing at {path}.", { path: s.hookPath }) }),
    s.error ? el("p", { class: "note error", text: s.error }) : null,
    codexHelp,
    updateHelp,
    usageHelp,
    kind === "gemini"
      ? el("p", {
          class: "note warn",
          text: t("Gemini CLI is no longer supported; Antigravity replaced it. Remove its hooks from Gemini's settings."),
        })
      : kind === "antigravity"
        ? el(
            "div",
            {},
            howItWorks(t("Hooks watch Antigravity's CLI, desktop and IDE sessions. Cost and context come only from the CLI's status line, which keeps your old command.")),
            s.installed
              ? row(t("Approve commands from the island"), t("Shell commands no Antigravity rule covers wait on a card: Allow runs, Deny stops, no answer falls back to Antigravity's prompt. Older hooks need Update hooks… first."), toggle(antigravityApprovals, async (on) => {
                  await Bridge.setAntigravityApprovals(on);
                  antigravityApprovals = on;
                }))
              : null,
          )
        : kind === "opencode"
          ? howItWorks(t("OpenCode loads a plugin, not hooks: restart it after installing. Answer permissions on the island or in OpenCode; in OpenCode 2, questions only in its terminal."))
          : kind === "hermes"
            ? howItWorks(t("After installing, run hermes plugins enable vults and open a new Hermes session. Sessions, tools and changes show in Flock; permissions and questions stay in Hermes."))
          : kind === "qwen"
            ? howItWorks(t("Restart Qwen Code after installing. Permissions show on the island, then in Qwen's terminal if unanswered. Questions stay in its terminal."))
            : kind === "copilot"
              ? howItWorks(t("Vults adds its own file to ~/.copilot/hooks/. Restart Copilot after installing. Permissions show on the island, then in Copilot's terminal if unanswered."))
              : kind === "cursor"
                ? howItWorks(t("One hooks file for the editor and cursor-agent; other hooks in it stay. Cursor asks permissions itself. Restart Cursor after installing."))
                : kind === "droid"
                  ? howItWorks(t("Hooks run in Droid's CLI; Droid asks permissions in its own prompt. Restart Droid after installing."))
                  : kind === "kimi"
                    ? howItWorks(t("Hooks go in Kimi's config.toml, beside your settings; Kimi asks permissions itself. Restart Kimi after installing."))
                    : null,
    notice,
    s.error || pending
      ? null
      : el(
          "div",
          { class: "actions" },
          s.installBlocked
            ? null
            : // The white button is the step that is due: installing, updating, or a reinstall a note asks
              // for. A reinstall of hooks that are up to date is just there.
              button(
                setupWord(kind) === "plugin"
                  ? s.outdated ? t("Update plugin…") : s.installed ? t("Reinstall plugin…") : t("Install plugin…")
                  : s.outdated ? t("Update hooks…") : s.installed ? t("Reinstall hooks…") : t("Install hooks…"),
                () => void preview(kind, true),
                // An agent that is not on this computer has nothing due: its install stays a plain button.
                (found && !s.installed) || s.outdated || usageHelp !== null,
              ),
          s.installed ? quiet(button(setupWord(kind) === "plugin" ? t("Remove plugin…") : t("Remove hooks…"), () => void preview(kind, false))) : null,
        ),
    review,
  );
}

/** What each agent can do (W1): built once, asked again each time the page is opened. */
const doctor = createDoctor((fresh) => Bridge.agentsDoctor(fresh), (rows) => {
  const found = rows.filter((r) => r.path !== null).map((r) => r.id).sort().join(" ");
  if (present !== null && [...present].sort().join(" ") === found) return;
  present = new Set(found ? found.split(" ") : []);
  // A review in progress keeps its place: the grouping follows on the next paint.
  if (page === "agents" && ![...panels.values()].some((p) => p.pending)) render();
});
/** The agents the doctor found on this computer; null until it answered. */
let present: Set<string> | null = null;
let doctorAsked = false;

/** The agents on this computer first, each with the step that is due; the others fold away. Before
 *  the doctor answers, or if its check fails, every agent shows as it always did. */
/** The agent whose detail shows, kept while Settings is open. */
let agentSelected: string | null = null;

/** One agent in Zeca's chat: how its engine stands and its own sign-in; none for one that is not an
 *  engine. One still in Labs says so. */
function agentEngine(kind: string): HTMLElement | null {
  const id = kind === "claude" || kind === "codex" ? kind : `acp:${kind}`;
  const lab = LABS.find(l => l.id === `acp-${kind}`);
  if (!engines && !enginesLoading && !enginesNote) checkEngines();
  const listed = id === "claude" || id === "codex" || (engines?.acp ?? []).some(a => a.id === id);
  const head = el("h3", { class: "shortcut-subtitle", text: t("In Zeca's chat") });
  if (lab && !labOn(labs, lab.id))
    return el("div", { class: "agent-engine" }, head,
      el("div", { class: "row" }, el("div", { class: "row-text" }, el("div", { class: "row-about", text: t("It joins the chat once you turn it on in Labs.") })),
        button(t("Labs"), () => choosePage("labs"))));
  if (!listed || !engines) return engines || lab ? null : el("div", { class: "agent-engine" }, head, el("p", { class: "note", text: t("Checking the engines…") }));
  const summary = id === "claude" || id === "codex" ? engineSummary(engines[id]) : acpSummary();
  const SIGN_IN = ["claude", "codex", "acp:antigravity", "acp:opencode"];
  const canSignIn = SIGN_IN.includes(id) && (id.startsWith("acp:") || engines[id as "claude" | "codex"] === "installed");
  const signIn = canSignIn
    ? button(t("Sign in"), () => {
        Bridge.chatSignIn(id as import("../../bridge").ChatProvider).then(
          () => { enginesNote = t("Finish signing in in the terminal that opened, then check again."); render(); },
          e => { enginesNote = failure(t("Could not open a terminal."), e); render(); },
        );
      })
    : null;
  return el("div", { class: "agent-engine" }, head,
    el("div", { class: "row engine-row" },
      el("div", { class: "row-about engine-state" }, dot(summary.dot), el("span", { text: summary.headline })),
      el("div", { class: "engine-actions" }, ...(signIn ? [signIn] : []), button(enginesLoading ? t("Checking…") : t("Check again"), () => checkEngines()))),
    enginesNote ? el("p", { class: "note", text: enginesNote }) : null);
}

function agentCards(): HTMLElement[] {
  const listed = AGENTS.filter((a) => !a.retired || panels.get(a.kind)?.status?.installed || panels.get(a.kind)?.message);
  const known = present;
  const here = (a: (typeof AGENTS)[number]) => {
    const p = panels.get(a.kind);
    return !known || known.has(a.kind) || !!p?.status?.installed || !!p?.status?.error || !!p?.message || !!p?.pending;
  };
  const found = listed.filter(here), others = listed.filter((a) => !here(a));
  const due = listed.filter((a) => panels.get(a.kind)?.status?.outdated).length;
  // As t3code lists its providers: what is connected first, then what is here to set up, then the rest.
  const connected = found.filter((a) => !a.retired && panels.get(a.kind)?.status?.installed);
  const toSetUp = found.filter((a) => !a.retired && !panels.get(a.kind)?.status?.installed);
  const retired = found.filter((a) => a.retired);
  const groups: [string, typeof found][] = [
    [t("Connected to Vults"), connected], [t("On this computer, not connected yet"), toSetUp],
    [t("No longer supported"), retired], [t("Not found on this computer"), others],
  ];
  const all = groups.flatMap(([, list]) => list);
  if (!all.some(a => a.kind === agentSelected)) agentSelected = all[0]?.kind ?? null;
  const state = (a: (typeof AGENTS)[number]): [string, "ok" | "warn" | "off"] => {
    const s = panels.get(a.kind)?.status;
    if (a.retired) return [t("No longer supported"), "warn"];
    if (s?.error) return [t("Can't read"), "warn"];
    if (s?.installed) return s.outdated ? [t("Update available"), "warn"] : [t("Connected"), "ok"];
    return others.includes(a) ? [t("Not on this computer"), "off"] : [t("Not connected yet"), "off"];
  };
  const list = el("div", { class: "provider-list agent-list" });
  list.setAttribute("role", "radiogroup");
  list.setAttribute("aria-label", t("Agents"));
  for (const [title, agents] of groups) {
    if (!agents.length) continue;
    list.append(el("span", { class: "provider-group", text: title }));
    for (const a of agents) {
      const [line, kind] = state(a);
      const on = a.kind === agentSelected;
      const item = el("button", { class: `provider-item${on ? " on" : ""}`, onclick: () => { agentSelected = a.kind; render(); } },
        brandMark(a.kind, a.name, 18),
        el("span", { class: "provider-text" }, el("span", { class: "provider-name", text: a.name }), el("span", { class: "provider-state", text: line })),
        dot(kind));
      item.setAttribute("role", "radio");
      item.setAttribute("aria-checked", String(on));
      list.append(item);
    }
  }
  list.addEventListener("keydown", event => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    const items = [...list.querySelectorAll<HTMLButtonElement>(".provider-item")];
    const at = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : (at + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
    event.preventDefault();
    items[next]?.focus();
  });
  const chosen = all.find(a => a.kind === agentSelected);
  const detail = chosen
    ? el("div", { class: "provider-detail agent-detail" }, agentCard(chosen.kind, chosen.name, !others.includes(chosen)), agentEngine(chosen.kind))
    : el("div", { class: "provider-detail" }, el("p", { class: "note", text: t("No supported agent found. Install one, then reopen this page.") }));
  return [
    ...(due ? [el("p", { class: "note warn with-icon" }, icon("warning", 14, 2), el("span", { text: due === 1 ? t("1 agent needs an update.") : t("{n} agents need an update.", { n: due }) }))]
      : connected.length ? [el("p", { class: "note ok with-icon" }, icon("check", 14, 2), el("span", { text: t("All connected agents are up to date.") }))] : []),
    el("section", { class: "card provider-card" }, el("div", { class: "provider-pane" }, list, detail)),
  ];
}

/** The API providers: the list beside the chosen one's key and model (t3code's providers). */
function apiSection(): HTMLElement[] {
  const p = apiProviders.find((x) => x.id === apiSelected);
  const message = apiMessage ? el("p", { class: `note${apiMessage.error ? " error" : " ok"}`, text: apiMessage.text }) : null;
  const done = (text: string, error = false) => {
    apiMessage = { text, error };
    void refreshApi();
  };

  // As t3code shows its providers: the list beside the chosen one's detail (stacked when narrow).
  const summary = p ? apiSummary(p) : null;
  const detail = p
    ? el("div", { class: "provider-detail" },
        el("div", { class: "provider-detail-head" },
          el("div", { class: "provider-detail-name" }, brandMark(p.id, p.label, 22), el("span", { text: p.label }), dot(summary!.dot))),
        el("div", { class: "rows" }, row(...keyRow(p, done)), row(...modelRow(p))),
        message)
    : el("div", { class: "provider-detail" }, el("p", { class: "note", text: t("Choose a provider on the left.") }));
  const rows: (HTMLElement | null)[] = [
    el("div", { class: "provider-pane" }, providerList(), detail),
  ];
  const apiNote = t("The API chat can't run commands or edit files. It reads files you drop and, when needed, what the island shows, and sends it to the provider. Keys stay in the system keyring and go only to their provider.");

  return [
    el("h2", { class: "settings-group-title", text: t("API providers") }),
    el("section", { class: "card provider-card" }, ...rows),
    el("p", { class: "note", text: apiNote }),
  ];
}

function agentsPage(): HTMLElement[] {
  if (!doctorAsked) {
    doctorAsked = true;
    doctor.refresh();
  }
  return [
    el("h1", { text: t("Agents") }),
    el("p", {
      class: "lede",
      text: t("Every agent and provider in one place: what Vults watches in your flock, and what runs Zeca's chat."),
    }),
    el("h2", { class: "settings-group-title", text: t("Agents") }),
    ...agentCards(),
    ...apiSection(),
    disclosure(t("What each agent can do"),
      el("p", { class: "note", text: t("What each agent on this computer can do with Vults, used by the router and your policies.") }),
      doctor.element,
    ),
  ];
}

// ── Connectors ───────────────────────────────────────────────────────────────

async function refreshConnectors(): Promise<void> {
  try {
    connectorStatus = new Map((await Bridge.connectorsStatus()).map((c) => [c.id, c]));
    connectorsRead = true;
    connectorsFailed = false;
  } catch {
    connectorsFailed = true;
  }
  if (page === "connectors") render();
}

function connectorsPage(): HTMLElement[] {
  return [
    el("h1", { text: t("Connection setup") }),
    el("p", { class: "lede", text: t("News from outside services on the island. Each one is off until you switch it on.") }),
    ...(presence === "paused" ? [el("p", { class: "note", text: t("Paused: connectors do not check anything until you pick another presence in General.") })] : []),
    ...(connectorsFailed
      ? [readState(true, "", t("Could not read which connections are on. Their switches stay as they are."), () => void refreshConnectors())]
      : !connectorsRead ? [readState(false, t("Reading your connections…"), "", () => {})] : []),
    ...CONNECTORS.map((c) => {
      const st = connectorStatus.get(c.id);
      const control = toggle(st?.enabled ?? false, async (on) => {
        await Bridge.connectorEnable(c.id, on);
        await refreshConnectors();
      });
      if (!connectorsRead) control.disabled = true;
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
          el("div", { class: "card-title brand-title" }, brandMark(c.id, c.name, 20), el("span", { text: c.name })),
          el(
            "div",
            { class: "head-right" },
            st?.enabled ? state : null,
            control,
          ),
        ),
        el("p", { class: "row-about", text: t(c.about) }),
        c.id === "gitea" ? gitea.element : null,
        c.id === "github" ? githubCheck.element : null,
        c.id === "gitea" && st?.account && st.host ? el("p", { class: "note", text: t("Last successful Gitea account: {account} on {host}.", { account: st.account, host: st.host }) }) : null,
        c.id === "github" && st?.account && st.host ? el("p", { class: "note", text: t("Last successful GitHub CLI account: {account} on {host}.", { account: st.account, host: st.host }) }) : null,
        c.id === "gitlab" && st?.account && st.host ? el("p", { class: "note", text: t("Last successful GitLab CLI account: {account} on {host}.", { account: st.account, host: st.host }) }) : null,
        st?.enabled && st.lastOk && !st.error ? el("p", { class: "note", text: t("Last checked {when}.", { when: ago(st.lastOk) }) }) : null,
        st?.enabled && st.error ? el("p", { class: "note error", text: connectorError(st.error) }) : null,
        st?.enabled && st.overBudget
          ? el("p", { class: "note warn", text: t("Today's check limit is reached. It checks again once it has room.") })
          : null,
      );
    }),
  ];
}

// ── General ──────────────────────────────────────────────────────────────────

function priceCatalogRow(): HTMLElement {
  const checked = priceStatus?.checkedAt;
  const text = checked == null ? t("No price catalog yet.") : `models.dev · ${t("Prices checked: {time}", { time: new Date(checked * 1000).toLocaleString(locale()) })}`;
  const state = priceStatus?.failed ? t("Price refresh failed.") : priceStatus?.stale && checked != null ? t("Cached prices have expired.") : "";
  const refresh = button(priceStatus?.updating ? t("Updating prices…") : t("Refresh prices"), () => {
    refresh.disabled = true;
    void Bridge.refreshModelPrices().then(status => { priceStatus = status; render(); }, () => {
      priceStatus = { enabled: priceUpdates, checkedAt: checked ?? null, stale: priceStatus?.stale ?? true, updating: false, failed: true }; render();
    });
  });
  refresh.disabled = !priceUpdates || !!priceStatus?.updating;
  return row(t("Price catalog"), [text, state].filter(Boolean).join(" · "), refresh);
}

Bridge.onModelPrices(status => { priceStatus = status; priceUpdates = status.enabled; if (page === "general") render(); });
Bridge.onUpdateNotice(status => { update = status; if (page === "general") render(); });

/** What the last look found, in one sentence. */
function updateState(u: UpdateStatus): string {
  if (u.checking) return t("Looking for a new version…");
  if (u.notice) return t("Vults {version} is available. You have {current}.", { version: u.notice.version, current: u.current ?? "?" });
  if (u.failed) return t("The last look failed. Vults tries again tomorrow, or now with Check now.");
  if (u.checkedAtMs != null) return t("You have the latest version, {current}. Checked {time}.", { current: u.current ?? "?", time: new Date(u.checkedAtMs).toLocaleString(locale()) });
  return t("Not checked yet.");
}

function updatesSection(): HTMLElement[] {
  const u = update;
  if (!u?.available) return [];
  const check = button(u.checking ? t("Checking…") : t("Check now"), () => {
    check.disabled = true;
    void Bridge.checkUpdateNow().then(status => { update = status; render(); }, () => { if (update) update = { ...update, checking: false, failed: true }; render(); });
  });
  check.disabled = u.checking;
  const controls = el("div", { class: "actions" }, check);
  if (u.notice) controls.append(button(t("Open release page"), () => void Bridge.openUpdatePage().catch(() => {})));
  return [
    el("h2", { class: "settings-group-title", text: t("Updates") }),
    el("section", { class: "card rows" },
      row(
        t("Look for new versions"),
        t("Once a day, Vults checks GitHub's public release list for a newer version. Nothing about you is sent; nothing is downloaded or installed."),
        toggle(u.consent.state === "yes", async on => { update = await Bridge.setUpdateChecks(on); render(); }),
      ),
      row(t("Version"), updateState(u), controls),
    ),
  ];
}

/** A small picture of Vults in `value`'s own colors: the island with its flock on the wire, the
 *  Nest's list with its chosen item, a card. Its colors are the theme's tokens, scoped to it. */
function themePreview(value: Theme): HTMLElement {
  const bird = (agent: string) => el("span", { class: `tp-bird tp-${agent}` });
  const preview = el("div", { class: "tp" },
    el("div", { class: "tp-island" }, el("span", { class: "tp-wire" }), bird("claude"), bird("codex"), bird("zeca")),
    el("div", { class: "tp-nest" },
      el("div", { class: "tp-side" }, el("span", { class: "tp-item on" }), el("span", { class: "tp-item" }), el("span", { class: "tp-item" })),
      el("div", { class: "tp-card" }, el("span", { class: "tp-title" }), el("span", { class: "tp-line" }), el("span", { class: "tp-line short" }),
        el("span", { class: "tp-chip" }))));
  preview.dataset.theme = value;
  preview.setAttribute("aria-hidden", "true");
  return preview;
}

/** The themes as tiles, each a picture of Vults in its colors; one radio group, arrows move. */
function themeTiles(): HTMLElement {
  const group = el("div", { class: "theme-tiles" });
  group.setAttribute("role", "radiogroup");
  group.setAttribute("aria-label", t("Theme"));
  group.setAttribute("aria-busy", String(themeSaving));
  const tiles = THEMES.map(([value, label]) => {
    // SP Night's three keep the family as a tag, so each flavor's own name fits its tile.
    const night = value.startsWith("sp-night") ? label.replace(/^SP Night: /, "") : null;
    const name = night ?? t(label);
    const on = value === theme;
    const tile = el("button", { class: `theme-tile${on ? " on" : ""}`, onclick: () => choose(value) },
      themePreview(value),
      el("span", { class: "theme-name" },
        el("span", { text: name }),
        value === DEFAULT_THEME ? el("span", { class: "theme-tag", text: t("Default") }) : null));
    tile.setAttribute("aria-label", night ? label : name);
    tile.setAttribute("role", "radio");
    tile.setAttribute("aria-checked", String(on));
    tile.setAttribute("aria-disabled", String(themeSaving));
    tile.tabIndex = on ? 0 : -1;
    tile.dataset.setting = `theme:${value}`;
    return tile;
  });
  async function choose(value: Theme): Promise<void> {
    if (value === theme || themeSaving) return;
    const before = theme;
    theme = value;
    themeSaving = true;
    themeNote = null;
    render();
    document.querySelector?.<HTMLElement>(`[data-setting="theme:${value}"]`)?.focus();
    try {
      await Bridge.setTheme(value);
    } catch (e) {
      theme = before;
      themeNote = failure(t("Could not change the theme."), e);
    } finally {
      themeSaving = false;
      render();
    }
  }
  group.addEventListener("keydown", event => {
    const keys = ["ArrowRight", "ArrowDown", "ArrowLeft", "ArrowUp", "Home", "End"];
    if (!keys.includes(event.key)) return;
    event.preventDefault();
    const at = THEMES.findIndex(([value]) => value === theme);
    const n = THEMES.length;
    const next = event.key === "Home" ? 0 : event.key === "End" ? n - 1
      : (at + (event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : -1) + n) % n;
    void choose(THEMES[next][0]);
  });
  // Dark and Light first; SP Night's three views under their family's name.
  const family = el("span", { class: "theme-family", text: "SP Night" });
  family.setAttribute("aria-hidden", "true");
  const first = THEMES.findIndex(([value]) => value.startsWith("sp-night"));
  group.append(...tiles.slice(0, first), family, ...tiles.slice(first));
  return group;
}

function generalPage(): HTMLElement[] {
  return [
    el("h1", { text: t("General") }),
    el("p", { class: "lede", text: t("Startup, plan usage and the price list Vults uses for cost estimates.") }),
    el("h2", { class: "settings-group-title", text: t("Startup") }),
    el("section", { class: "card rows" },
      row(
        t("Start with the desktop"),
        t("Opens Vults when you log in."),
        toggle(autostart, async (on) => {
          await Bridge.setAutostart(on);
          autostart = on;
        }),
      ),
      row(
        t("Keep the computer awake while agents work"),
        t("It won't suspend while a session or a chat turn runs. The screen still locks."),
        toggle(keepAwake, async (on) => {
          await Bridge.setKeepAwake(on);
          keepAwake = on;
        }),
      ),
    ),
    el("h2", { class: "settings-group-title", text: t("Usage and prices") }),
    el("section", { class: "card rows" },
      row(
        t("Codex usage"),
        t("Shows your Codex plan's usage. Asks codex every 5 minutes, which reaches OpenAI. Off, nothing is asked."),
        toggle(codexUsage, async (on) => {
          await Bridge.setCodexUsage(on);
          codexUsage = on;
        }),
      ),
      row(
        t("Claude usage"),
        t("Reads your Claude plan's usage via Claude Code every 5 minutes; may contact Anthropic, no prompt sent. Session reports work when off."),
        toggle(claudeUsage, async (on) => {
          await Bridge.setClaudeUsage(on);
          claudeUsage = on;
        }),
      ),
      row(
        t("Automatic model prices"),
        t("Checks models.dev every 15 minutes; no session data is sent. Estimates use standard API prices, cached up to 24 hours."),
        toggle(priceUpdates, async on => { await Bridge.setPriceUpdates(on); priceUpdates = on; render(); }),
      ),
      priceCatalogRow(),
    ),
    ...updatesSection(),
  ];
}

/** Settings → Appearance: how every window looks (t3code's Colors & themes, then Interface). */
function appearancePage(): HTMLElement[] {
  return [
    el("h1", { text: t("Appearance") }),
    el("p", { class: "lede", text: t("Colors, width and language for every window.") }),
    el("h2", { class: "settings-group-title", text: t("Colors and themes") }),
    el("section", { class: "card rows" },
      el("div", { class: "row theme-row" },
        el("div", { class: "row-title", text: t("Theme") }),
        el("div", { class: "row-about", text: t("Dark, light or SP Night. Every window follows it.") }),
        themeTiles(),
        themeNote ? el("p", { class: "note error", text: themeNote }) : null),
    ),
    el("h2", { class: "settings-group-title", text: t("Window and language") }),
    el("section", { class: "card rows" },
      row(t("Mini and Quick width"), t("Shared width, limited to the available screen space."),
        dropdown(ISLAND_WIDTHS.map(value => ({ value, label: value ? `${value} px` : t("Default") })), restoreWidth(), async value => {
          if (!saveWidth(value)) throw new Error(t("Could not save the width. Choose it again to retry."));
        })),
      row(
        t("Language"),
        t("For the island, settings, tray and notifications. System follows your desktop."),
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
  ];
}

/** Settings → Island. */
function islandPage(): HTMLElement[] {
  return [
    el("h1", { text: t("Island") }),
    el("p", { class: "lede", text: t("Where the island sits, how it opens and folds, and how present it is.") }),
    el("h2", { class: "settings-group-title", text: t("Behavior") }),
    el("section", { class: "card rows" },
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
        t("Island: the flock at the top. Panel: Zeca in the tray. Quiet: cards only. Paused: agents ask in their terminals; connectors and notifications stop."),
      ),
      row(
        t("Fold the island"),
        t("How long the island stays open after the pointer leaves. A permission keeps it open."),
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
        t("Hover to open the island fully, no click needed. It folds when the pointer leaves, unless you clicked in it. Not in Panel."),
        toggle(openOnHover, async (on) => {
          await Bridge.setOpenOnHover(on);
          openOnHover = on;
        }),
      ),
    ),
    el("h2", { class: "settings-group-title", text: t("Placement") }),
    el("section", { class: "card rows" },
      row(
        t("Corner widget"),
        t("A corner window with up to three birds and how many sessions work or need you."),
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
        t("A click opens the island, on the card when one waits; it never answers one."),
      ),
      row(
        t("Screen"),
        t("Automatic lets the desktop choose. If the chosen screen is unplugged, the island moves until it returns."),
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
  ];
}

/** Settings → Sound and notifications. */
function soundPage(): HTMLElement[] {
  return [
    el("h1", { text: t("Sound and notifications") }),
    el("p", { class: "lede", text: t("What Vults plays and shows when an agent finishes, fails or needs you.") }),
    el("h2", { class: "settings-group-title", text: t("Sounds") }),
    el("section", { class: "card rows" },
      row(
        t("Sounds"),
        t("8-bit blips when a session needs you, finishes or fails, and for connector news."),
        toggle(sounds, async (on) => {
          await Bridge.setSounds(on);
          sounds = on;
          render();
        }),
      ),
      row(t("Volume"), t("Sound volume. A cue plays when you release the slider."), slider(volume, !sounds)),
      row(
        t("Now playing"),
        t("Shows what your music player is playing, with controls, and Zeca dances. Nothing leaves this computer."),
        toggle(nowPlaying, async (on) => {
          await Bridge.setNowPlaying(on);
          nowPlaying = on;
        }),
      ),
    ),
    el("h2", { class: "settings-group-title", text: t("Notifications") }),
    el("section", { class: "card rows" },
      row(
        t("Do not disturb"),
        dndUntil
          ? t("On until {time}: no sounds, notifications or reminders. A card still opens the island with its sound.", { time: new Date(dndUntil * 1000).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" }) })
          : t("No sounds or notifications for a while; it ends by itself. A card still opens the island with its sound."),
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
        t("Panel only: a desktop notification when a session finishes, fails, goes quiet or a card waits."),
        toggle(notifications, async (on) => {
          await Bridge.setNotifications(on);
          notifications = on;
        }),
        t("In Island and Quiet, the island shows it all. Its only button opens the island, never answers a card."),
      ),
      row(
        t("Stay quiet when I am looking"),
        t("No sound and no island when a session ends while its terminal is in front. Cards always sound and open."),
        toggle(quietWhenWatching, async (on) => {
          await Bridge.setQuietWhenWatching(on);
          quietWhenWatching = on;
        }),
        t("Vults checks the active window once, when a turn ends. On GNOME Wayland and Windows terminals, every finish still sounds."),
      ),
    ),
  ];
}

/** Dictation's keys, on the Keyboard shortcuts page once dictation is set up: the global
 *  shortcuts, how the talk key records, and X11's recording keys. */
function dictationShortcuts(): HTMLElement[] {
  const shortcutEditor = button(t("Change in KDE…"), () => {
    if (shortcutsOpening) return;
    shortcutsOpening = true;
    shortcutsOpenFailed = false;
    render();
    void Bridge.openShortcutSettings().catch(() => { shortcutsOpenFailed = true; }).finally(() => {
      shortcutsOpening = false;
      if (page === "shortcuts") render();
    });
  });
  (shortcutEditor as HTMLButtonElement).disabled = shortcutsOpening;
  const shortcutFeedback = el("p", { class: `note${shortcutsOpenFailed ? " error" : ""}`, text: shortcutsOpenFailed
    ? t("Could not open KDE Shortcuts. Open your desktop's shortcut settings, or try again.")
    : t("Opening KDE Shortcuts…") });
  shortcutFeedback.setAttribute("role", shortcutsOpenFailed ? "alert" : "status");
  return [
      el("section", { class: "card rows" },
        row(t("Dictation"), t("Hold to dictate into the current app. Needs a local model in Settings > Dictation > Voice input."), keycaps(boundKeys("talk"))),
        row(t("Voice command"), t("Hold to control Vults, then release. Nothing is typed into another app."), keycaps(boundKeys("talk-command"))),
        row(t("Copy last dictation"), t("Copy the last dictation again, to paste yourself."), keycaps(boundKeys("talk-again"))),
        row(t("Cancel voice"), t("Discard the current dictation or Chat recording."), keycaps(boundKeys("talk-cancel"))),
        windowsShortcuts.render(),
        ...(windowsShortcuts.desktopEditor() ? [row(t("Desktop shortcut settings"), t("Opens KDE System Settings → Shortcuts. Other desktops use their own settings."), shortcutEditor)] : []),
        ...(shortcutsOpening || shortcutsOpenFailed ? [shortcutFeedback] : []),
      ),
      el("h3", { class: "shortcut-subtitle", text: t("Recording") }),
      el("section", { class: "card rows" },
        row(t("Recording mode"), t("Hold to dictate. Double-tap to go hands-free, then press again to finish."),
          dropdown([{ value: true, label: t("Hold + double tap") }, { value: false, label: t("Hold only") }], dictationDoubleTap, async on => {
            await Bridge.setDictationGesture("doubleTap", on);
            dictationDoubleTap = on; dictationDoubleTapChanged = true;
          })),
      ),
      ...(windowsShortcuts.supported() ? [] : [(() => {
        const keys = disclosure(t("X11 recording keys"),
          row(t("Space locks recording"), t("X11 only. Space keeps recording after you release the shortcut; when off, Space can discard it."), (() => {
            const control = toggle(dictationSpaceLock, async on => {
              await Bridge.setDictationGesture("spaceLock", on);
              dictationSpaceLock = on; dictationSpaceLockChanged = true;
            });
            if (dictationWayland) control.setAttribute("disabled", "");
            return control;
          })()),
          row(t("Esc cancels dictation"), t("X11 only. Esc discards the recording or transcription."), (() => {
            const control = toggle(dictationEscapeCancel, async on => {
              await Bridge.setDictationGesture("escapeCancel", on);
              dictationEscapeCancel = on; dictationEscapeCancelChanged = true;
            });
            if (dictationWayland) control.setAttribute("disabled", "");
            return control;
          })()),
        );
        keys.dataset.setting = "dictation-x11-keys";
        return keys;
      })()]),
  ];
}

/** A shortcut as keycaps: each key a cap, alternatives apart; one not bound reads as words. */
function keycaps(text: string): HTMLElement {
  const box = el("span", { class: "keycaps" });
  box.setAttribute("aria-label", text);
  text.split(" · ").forEach((combo, i) => {
    if (i) box.append(el("span", { class: "keycaps-or", text: "/" }));
    const trimmed = combo.trim();
    if (!trimmed || trimmed === t("Not bound")) { box.append(el("span", { class: "keycaps-none", text: trimmed || t("Not bound") })); return; }
    for (const key of trimmed.split(/\+(?!$)/)) box.append(el("kbd", { text: key }));
  });
  return box;
}

/** Settings → Keyboard shortcuts. */
function shortcutsPage(): HTMLElement[] {
  // Dictation's keys join once it has a voice model: before, they would do nothing.
  const dictation = voice?.ready
    ? dictationShortcuts()
    : [el("section", { class: "card rows" },
        row(t("Dictation is not set up"), t("Its keys show here once dictation has a voice model."), button(t("Set up dictation"), () => chooseDictation("voice"))))];
  return [
    el("h1", { text: t("Keyboard shortcuts") }),
    el("p", { class: "lede", text: t("Keys that reach Vults from any app, and inside its windows.") }),
    el("h2", { class: "settings-group-title", text: t("Island and flock") }),
    el(
      "section",
      { class: "card rows" },
      windowsShortcuts.render(),
      row(t("Allow / Deny from anywhere"), windowsShortcuts.supported() ? t("Answer the waiting permission card from any app.") : t("Answer the visible permission card. Change global keys in System Settings → Shortcuts."), keycaps(boundKeys("allow", "deny"))),
      row(t("Next / previous session"), t("Move through the flock. A waiting permission keeps its session in front."), keycaps(boundKeys("next", "previous"))),
      row(t("Open the island"), t("Open the island with keyboard control: ↑ ↓ sessions, Enter terminal, M actions, Y / N answer, C chat, Esc return."), keycaps(boundKeys("open"))),
      row(t("Change size"), t("Switch to Mini, Quick or Room from anywhere."), keycaps(boundKeys("mini", "quick", "room"))),
      row(t("Command palette"), t("Find a session, open a tab or prepare a chat command in Quick or Room."), keycaps("Ctrl+K")),
      row(t("The Nest's tabs"), t("Choose Flock, Chat, Dictation, Connections or Activity. Esc leaves Full."), keycaps("Alt+1–5")),
    ),
    el("h2", { class: "settings-group-title", text: t("Dictation") }),
    ...dictation,
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

function dictationSectionPage(dictationSection: DictationSetting): HTMLElement[] {
  if (dictationSection === "conversation") return [el("h1", { text: t("Talk with Zeca") }), windowsShortcuts.supported()
    ? el("p", { class: "hint", text: t("Voice conversation is available on Linux only.") })
    : realtimeSettings.element];
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
        : t("Could not refresh microphones; the list may be outdated. Refresh to try again.")
      : t("No microphones found. Connect one and refresh.") });
  microphoneFeedback.setAttribute("role", microphonesFailed ? "alert" : "status");
  const requestErasure = button(t("Erase dictation history…"), () => {
    if (dictationErasing) return;
    dictationEraseConfirm = true; dictationEraseFailed = false; dictationErased = false; render();
    root.querySelector<HTMLElement>('[data-setting="Cancel dictation erasure"]')?.focus();
  });
  requestErasure.dataset.dictationEraseRequest = "";
  (requestErasure as HTMLButtonElement).disabled = dictationErasing;
  const cancelErasure = () => {
    if (dictationErasing) return;
    dictationEraseConfirm = false; dictationEraseFailed = false; render();
    root.querySelector<HTMLElement>("[data-dictation-erase-request]")?.focus({ preventScroll: true });
  };
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
      const active = document.activeElement as HTMLElement | null;
      const restore = dictationSection === "privacy" && (active === document.body || active?.dataset.setting === "Erase dictation confirmation" || active?.dataset.setting === "Cancel dictation erasure");
      if (page === "dictation") {
        render();
        if (restore) root.querySelector<HTMLElement>(dictationEraseFailed ? '[data-setting="Cancel dictation erasure"]' : '[data-dictation-erase-request]')?.focus({ preventScroll: true });
      }
    });
  });
  erase.classList.add("dictation-danger");
  (erase as HTMLButtonElement).disabled = dictationErasing;
  erase.dataset.setting = "Erase dictation confirmation";
  const cancel = button(t("Cancel"), cancelErasure);
  (cancel as HTMLButtonElement).disabled = dictationErasing;
  cancel.dataset.setting = "Cancel dictation erasure";
  const confirmation = el("div", { class: "note dictation-erasure-confirmation" },
    el("p", { text: t("Erase every retained dictation? Usage totals stay.") }),
    el("div", { class: "actions" }, cancel, erase));
  confirmation.setAttribute("role", "group");
  confirmation.setAttribute("aria-label", t("Erase dictation history"));
  confirmation.addEventListener("keydown", event => {
    if (event.key === "Escape" && !dictationErasing) {
      event.preventDefault(); event.stopPropagation(); cancelErasure();
    }
  });
  const feedback = el("p", { class: `note${dictationEraseFailed ? " error" : ""}`, text: dictationErasing
    ? t("Erasing dictation history…") : dictationEraseFailed ? t("Could not erase dictation history. Try again.")
    : t("Dictation history erased. Usage totals stay.") });
  feedback.setAttribute("role", dictationEraseFailed ? "alert" : "status");
  return [
    el("h1", { text: t(DICTATION_SETTINGS.find(p => p.id === dictationSection)!.label) }),
    ...(windowsShortcuts.supported() ? [
      el("p",{class:"hint windows-intro",text:t("Hold the shortcut to record with the local model, release to transcribe. Text is copied; optional insertion works in desktop text fields, not browsers or terminals.")}),
      el("p",{class:"hint windows-intro",text:t("Windows supports navigation, player and app volume, optional system audio commands and installed apps. Dictation can pause media.")}),
    ] : []),
    ...(dictationSection === "keys" ? [
      el("section", { class: "card rows" },
        row(t("Cancel by voice"), t("Say “cancel” or “never mind” alone or at the end. The dictation is discarded, never copied, inserted or saved."), toggle(dictationCancelByVoice, async on => {
          await Bridge.setDictationProcessing("cancelByVoice", on);
          dictationCancelByVoice = on; dictationCancelByVoiceChanged = true;
        }), windowsShortcuts.supported() ? t("English, Portuguese and Spanish.") : t("English, Portuguese and Spanish; works on X11 and Wayland.")),
        el("p", { class: "hint", text: t("Standalone dictation only, checked when recording stops. Turn off to dictate a sentence ending in a cancel phrase.") }),
        // Windows holds and releases through its own hotkeys: no listener sees Space or Esc.
        el("p", { class: "hint", text: windowsShortcuts.supported()
          ? t("Applies to the next dictation. Cancel voice always works.")
          : t("Applies to the next dictation. Cancel voice always works; Space and Esc need X11.") }),
      ),
    ] : []),
    ...(dictationSection === "commands" ? [
      el("section", { class: "card rows" },
        row(t("Enable voice commands"), t("Hold the command key, speak and release to control Vults. It all stays on this computer."), toggle(dictationCommands, async on => {
          await Bridge.setDictationCommands("enabled", on);
          dictationCommands = on; dictationCommandsChanged = true;
          if (page === "dictation") render();
        })),
        row(t("Media voice commands"), windowsShortcuts.supported() ? t("Pause, resume and skip on the selected player. Needs Now playing; for volume, pick its audio app below.") : t("Pause, resume, skip and volume for the selected player. Needs Media controls; project mute only silences its notifications."), toggle(dictationCommandMedia, async on => {
          await Bridge.setDictationCommands("media", on);
          dictationCommandMedia = on; dictationCommandMediaChanged = true;
          if (page === "dictation") render();
        })),
        row(t("Application voice commands"), windowsShortcuts.supported() ? t("Open Start Menu and Microsoft Store apps by name. Names stay local; if several match, you choose. No command lines, URLs or files.") : t("Open installed apps by name. Names stay local; if a project and an app both match, you choose. No command lines, URLs or files are run."), toggle(dictationCommandApps, async on => {
          await Bridge.setDictationCommands("applications", on);
          dictationCommandApps = on; dictationCommandAppsChanged = true;
        })),
        row(t("System audio voice commands"), t("Mute or change the volume of the default output, up to 100%. Player controls stay separate."), toggle(dictationCommandOutput, async on => {
          await Bridge.setDictationCommands("output", on);
          dictationCommandOutput = on; dictationCommandOutputChanged = true;
        })),
        ...(windowsShortcuts.supported() ? [playerAudio.render()] : []),
        row(t("Command key"), t("Hold to control Vults, then release. Nothing is typed into another app."), keycaps(boundKeys("talk-command"))),
        row(t("Names it understands"), t("Matches project and agent names and your Dictionary. Close spellings count; on a tie, Vults asks."), button(t("Dictionary"), () => void Bridge.openNest("dictation-dictionary"))),
      ),
      el("section", { class: "card rows" },
        el("p", { class: "dictation-command-routing", text: !dictationCommands ? t("Voice commands are off. Dictation keeps its own shortcut.")
          : dictationCommandChat ? t("Unrecognized phrases may be sent to the current Chat.") : t("Unrecognized phrases stay local.") }),
        row(t("When it is not a command"), t("Sending to Zeca uses the current Chat conversation and its model, access rules and budget."), (() => {
          const control = dropdown([{ value: false, label: t("Say so on the island") }, { value: true, label: t("Send it to Zeca as a message") }], dictationCommandChat, async on => {
            await Bridge.setDictationCommands("chat", on);
            dictationCommandChat = on; dictationCommandChatChanged = true;
          });
          (control as HTMLSelectElement).disabled = !dictationCommands;
          return control;
        })(), t("Needs Zeca on and Chat idle, with nothing unsent. Refused or ambiguous commands stay local.")),
      ),
    ] : []),
    ...(dictationSection === "words" ? [
      el("section", { class: "card rows" },
        row(t("Help the model with these words"), t("Give Whisper your dictionary words as local context. Off by default; tested only with Turbo in English."), toggle(dictationVocabulary, async on => {
          await Bridge.setDictationProcessing("vocabulary", on);
          dictationVocabulary = on; dictationVocabularyChanged = true;
        })),
        row(t("Project vocabulary"), t("Also add package, file and branch names from the current session or window title. Local only, off by default, Turbo in English only."), toggle(dictationProjectVocabulary, async on => {
          await Bridge.setDictationProcessing("projectVocabulary", on);
          dictationProjectVocabulary = on; dictationProjectVocabularyChanged = true;
        })),
        el("p", { class: "hint", text: t("Dictionary words go first, up to 224 tokens. Applies to the next recording; Chat is unchanged.") }),
      ),
    ] : []),
    ...(dictationSection === "voice" ? [
      el("p", { class: "lede", text: t("Speak in another app. Transcription stays on this computer.") }),
      el("h2", { text: t("Model and language") }),
      el("section", { class: "card rows" }, ...voiceRows()),
      el("h2", { text: t("Microphone") }),
      el("section", { class: "card rows" },
        row(t("Dictation microphone"), t("Used for the next dictation; if it is missing, another input is used. Chat keeps the default microphone."), microphone),
        el("div", { class: "actions" }, refresh),
        ...(microphonesLoading || microphonesFailed || microphones?.length === 0 ? [microphoneFeedback] : []),
      ),
      el("h2", { text: t("Sound and feedback") }),
      el("section", { class: "card rows" },
        row(t("Music while you speak"), windowsShortcuts.supported() ? t("Pause playing media while you dictate, and resume it if you did not change it. Leave it is the Windows default.") : t("Lower or pause playing media while you dictate, and restore it afterwards unless you changed it."), dropdown([
          ...(!windowsShortcuts.supported() ? [{ value: "lower" as const, label: t("Lower to 25%") }] : []),
          { value: "pause" as const, label: t("Pause it") },
          { value: "leave" as const, label: t("Leave it") },
        ], windowsShortcuts.supported() && dictationMusic === "lower" ? "leave" : dictationMusic, async mode => {
          await Bridge.setDictationMusic(mode);
          dictationMusic = mode; dictationMusicChanged = true;
        })),
        row(t("Dictation start sound"), t("Play a short cue when recording starts. Respects sound volume and Do Not Disturb."), toggle(dictationSound, async (on) => {
          await Bridge.setDictationSound(on);
          dictationSound = dictation.startSound(on);
        })),
        row(t("Dictation captions"), t("Show captions of your words near the island. They take no clicks and vanish two seconds after you finish."), toggle(dictationCaption, async (on) => {
          await Bridge.setDictationCaption(on);
          dictationCaption = dictation.captions(on);
        })),
      ),
      disclosure(t("Advanced processing"), el("section", { class: "card rows" },
        row(t("Remove repeated loops"), t("Collapse short phrases repeated six or more times."), toggle(dictationCutRepeats, async on => {
          await Bridge.setDictationProcessing("cutRepeats", on);
          dictationCutRepeats = on; dictationCutRepeatsChanged = true;
        }), t("Voice commands keep repeats in names. When off, sound tags and silence phrases are still removed.")),
        row(t("Retry empty dictation with Base"), t("If the model returns no text, retry once with an installed Base model. Never downloads a model."), toggle(dictationRetryBase, async on => {
          await Bridge.setDictationProcessing("retryBase", on);
          dictationRetryBase = on; dictationRetryBaseChanged = true;
        })),
        el("p", { class: "hint", text: t("Applies to the next dictation. Recordings in progress and Chat are unchanged.") }),
      )),
    ] : []),
    ...(dictationSection === "paste" ? [
      (() => {
        const keyboardReady = !dictationWayland || ["ready", "temporary"].includes(dictationPermission);
        const mode = !dictationInsert ? "clipboard" : !dictationInsertAvailable ? "unavailable" : !keyboardReady ? "waiting" : "insertion";
        const title = mode === "clipboard" ? t("Clipboard only.") : mode === "unavailable" ? t("Insertion is unavailable on this desktop.")
          : mode === "waiting" ? t("Clipboard fallback until keyboard access is ready.") : t("Insertion enabled for supported destinations.");
        const overview = el("section", { class: "card dictation-delivery-state" }, icon(mode === "insertion" ? "sideRight" : "copy", 20),
          el("div", {}, el("h2", { text: title }), el("p", { class: "hint", text: t("Insertion never submits text. Check the final delivery status before continuing.") })));
        overview.setAttribute("role", "status"); overview.dataset.deliveryMode = mode;
        return overview;
      })(),
      el("section", { class: "card rows" },
        ...(!windowsShortcuts.supported() ? [el("p", { class: "hint", text: t("App detection works on X11 and Plasma Wayland; GNOME and wlroots may miss it. If background copying is unsupported, dictation shows an error.") })] : []),
        row(windowsShortcuts.supported() ? t("Insert dictation in supported Windows text fields") : t("Insert dictation in supported editors and tmux panes"), windowsShortcuts.supported() ? t("Off by default. Types only into a recognized Windows text field; password fields, terminals and focus changes keep the text copied. Nothing is submitted; direct insertion leaves your clipboard untouched.") : t("Off by default. Pastes into KWrite, GNOME Text Editor, Mousepad, Leafpad or a flock tmux pane in XTerm or Kitty; otherwise the text stays copied. Plasma Wayland needs keyboard permission. Nothing is submitted."), (() => {
          const control = toggle(dictationInsert, async (on) => {
            permissionFailure = await permissionWrite(() => Bridge.setDictationInsert(on));
            dictationInsert = on;
            dictationInsertChanged = true;
            render();
          });
          if (!dictationInsertAvailable && !dictationInsert) control.setAttribute("disabled", "");
          return control;
        })()),
        ...(!windowsShortcuts.supported() ? [el("p", { class: "hint", text: t("Insertion borrows the clipboard and restores only its previous text, up to 64 KiB; images and rich formats are lost. Clipboard history may keep it, depending on the desktop. Needs X11 or Plasma Wayland with ext-data-control.") })] : []),
        ...(permissionFailure ? [el("p", { class: "note error", text: permissionErrors.has(permissionFailure) ? t(permissionFailure) : permissionFailure }),
          ...(!dictationInsert ? [(() => {
            const control = button(t("Retry token removal"), () => saveSetting(control, async () => {
              permissionFailure = await permissionWrite(() => Bridge.setDictationInsert(false));
              render();
            }, () => {}));
            return control;
          })()] : [])] : []),
        ...(dictationInsert && dictationWayland ? [row(t("Wayland keyboard access"), t({
          off: tk("No keyboard access. On Wayland, request it here to enable insertion."),
          requesting: tk("Waiting for the desktop's keyboard permission. Turn insertion off to cancel."),
          ready: tk("Keyboard access is ready. The restore token is kept only in the OS keyring."),
          temporary: tk("Keyboard access is ready until you restart; then request it again."),
          unavailable: tk("Keyboard access is unavailable or was revoked. Dictation keeps copying; retry to ask the desktop again."),
        }[dictationPermission]), (() => {
          const control = button(t("Request keyboard access"), () => saveSetting(control, () => permissionWrite(Bridge.retryDictationPermission), () => {}));
          if (["requesting", "ready", "temporary"].includes(dictationPermission)) control.setAttribute("disabled", "");
          return control;
        })())] : []),
      ),
      el("h2", { text: t("Terminals & sessions") }),
      el("section", { class: "card rows" },
        row(t("Spoken code"), t("In terminals, join words around spoken dot, slash, underscore, dash and colon (English, Portuguese, Spanish)."), toggle(dictationSpokenCode, async on => {
          await Bridge.setDictationProcessing("spokenCode", on);
          dictationSpokenCode = on; dictationSpokenCodeChanged = true;
        }), t("Other apps keep their text. Changes apply to the next dictation.")),
        el("p", { class: "hint", text: t("Keeps case and punctuation; never adds a final period or submits.") }),
      ),
    ] : []),
    ...(dictationSection === "privacy" ? [
      el("section", { class: "card rows" },
        row(t("Keep dictation history"), t("Only text is kept on this computer, never audio or clipboard contents. A shorter retention erases older text at once."),
          dropdown([{ value: "0", label: t("Do not keep") }, { value: "7", label: t("7 days") }, { value: "30", label: t("30 days") }, { value: "365", label: t("1 year") }, { value: "all", label: t("Always") }],
            dictationKeep, async value => { await Bridge.setDictationKeep(value); dictationKeep = value; dictationKeepChanged = true; })),
      ),
      el("section", { class: "card rows dictation-privacy-erasure" },
        row(t("Erase dictation history"), t("Erases every saved dictation. Daily and per-app totals stay."), requestErasure),
        ...(dictationEraseConfirm ? [confirmation] : []),
        ...(dictationErasing || dictationEraseFailed || dictationErased ? [feedback] : []),
      ),
    ] : []),
  ];
}

const dictationTool = () => dictationSection === "styles" || dictationSection === "diagnostics";
function dictationPage(): HTMLElement[] {
  if (dictationTool()) return dictationSectionPage(dictationSection);
  // The Windows notes are the page's, said once under its lede: kept from the first group only.
  const intro: HTMLElement[] = [];
  const body = (id: DictationSetting) => dictationSectionPage(id).filter(node => {
    if (!node.classList.contains("windows-intro")) return true;
    if (id === "voice") intro.push(node);
    return false;
  }).filter(node => node.tagName !== "H1" && !node.classList.contains("lede")).map(node => node.tagName === "H2" ? el("h3", { text: node.textContent ?? "" }) : node);
  const group = (id: string, title: string, children: HTMLElement[], folded = false) => {
    const node = folded ? disclosure(title, ...children) : el("section", { class: "dictation-preference-group" }, el("h2", { text: title }), ...children);
    node.dataset.dictationSection = id;
    node.dataset.setting = `dictation-group-${id}`;
    return node;
  };
  const voice = body("voice");
  const feedback = voice.findIndex(node => node.tagName === "H3" && node.textContent === t("Sound and feedback"));
  const sound = feedback >= 0 ? voice.splice(feedback, 2) : [];
  const processing = voice.pop()!;
  return [el("h1", { text: t("Dictation") }),
    el("p", { class: "lede", text: t("Set up your voice, recording and text delivery.") }),
    ...intro,
    group("voice", t("Voice input"), voice),
    group("keys", t("Recording"), [...body("keys"), ...sound]),
    group("paste", t("Text delivery"), body("paste")),
    group("privacy", t("History and privacy"), body("privacy")),
    el("h2", { text: t("Additional features") }),
    group("words", t("Recognition and vocabulary"), [processing, ...body("words")], true),
    group("commands", t("Voice commands"), body("commands"), true),
    group("conversation", t("Talk with Zeca"), body("conversation"), true),
    group("meetings", t("Meetings"), body("meetings"), true),
    group("agents", t("Spoken answers for agents"), body("agents"), true),
  ];
}

// ── Chat ─────────────────────────────────────────────────────────────────────

/** Asks the engines how they stand (each CLI's own word on its login). */
function checkEngines(): void {
  if (enginesLoading) return;
  enginesLoading = true;
  Bridge.chatEngines().then(
    e => { engines = e; enginesNote = null; },
    e => { enginesNote = failure(t("Could not check the engines."), e); },
  ).finally(() => { enginesLoading = false; if (page === "chat" || page === "agents") render(); });
}

/** The API providers as one list: each with its dot and how it stands, the chosen one marked. */
function providerList(): HTMLElement {
  const list = el("div", { class: "provider-list" });
  list.setAttribute("role", "radiogroup");
  list.setAttribute("aria-label", t("Provider"));
  const item = (x: ApiProvider) => {
    const s = apiSummary(x);
    const b = el("button", { class: `provider-item${x.id === apiSelected ? " on" : ""}`, onclick: () => {
      if (x.id === apiSelected) return;
      apiSelected = x.id;
      apiMessage = null;
      render();
      void Bridge.apiProviderSet(x.id).then(() => refreshApi());
    } }, brandMark(x.id, x.label, 18), el("span", { class: "provider-text" }, el("span", { class: "provider-name", text: x.label }), el("span", { class: "provider-state", text: s.headline })), dot(s.dot));
    b.setAttribute("role", "radio");
    b.setAttribute("aria-checked", String(x.id === apiSelected));
    return b;
  };
  const group = (label: string, local: boolean) => {
    const items = apiProviders.filter(x => x.local === local).map(item);
    return items.length ? [el("span", { class: "provider-group", text: label }), ...items] : [];
  };
  list.append(...group(t("Cloud, with your key"), false), ...group(t("On this machine"), true));
  // Arrows move through the list, as a radio group does.
  list.addEventListener("keydown", event => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    const items = [...list.querySelectorAll<HTMLButtonElement>(".provider-item")];
    const at = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : (at + (event.key === "ArrowDown" ? 1 : -1) + items.length) % items.length;
    event.preventDefault();
    items[next]?.focus();
  });
  return list;
}

function chatPage(): HTMLElement[] {
  return [
    el("h1", { text: t("Chat & voice") }),
    ...(zeca ? [] : [el("p", { class: "note", text: t("Chat and voice need Zeca. Turn him on in Zeca.") })]),
    el("p", {
      class: "lede",
      text: t("How Zeca's chat asks, searches the web and listens. Its agents and providers are in Agents."),
    }),
    el("h2", { class: "settings-group-title", text: t("Permissions and setup") }),
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
        t("The chat reads your CLI setup: CLAUDE.md, skills, MCP servers, hooks and allow rules, plus project settings in folders you trust in Claude Code. Cards still ask for what your rules don't allow."),
      ),
      row(
        t("Who answers Zeca's cards"),
        t("Choose who approves commands and edits. Every answer is audited."),
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
            askingFull = false;
            accessNote = null;
            render();
          },
          // Full access is asked first, below, with what it stops asking; nothing is saved by this click.
          (v) => {
            if (v !== "full") return true;
            askingFull = true;
            accessNote = null;
            render();
            document.querySelector<HTMLElement>(".access-confirm .btn")?.focus();
            return false;
          },
        ),
        t("Asks first: a card for every command or edit. Your policies: your accepted policies answer, the rest asks. Full access: everything runs; only what a policy denies is refused."),
      ),
      ...(askingFull && chatAccess !== "full"
        ? [fullAccessConfirm(t("Full access runs commands, edits and network requests without asking, destructive ones too. Add deny lines to a policy for what must never run."))]
        : chatAccess === "full"
          ? [el("p", { class: "note warn with-icon" }, icon("warning", 14, 2), el("span", { text: t("Full access runs commands, edits and network requests without asking, destructive ones too. Add deny lines to a policy for what must never run.") }))]
          : []),
      ...(accessNote ? [el("p", { class: "note error", text: accessNote })] : []),
    ),
    ...(web ? [el("h2", { class: "settings-group-title", text: t("Web") }), el("section", { class: "card rows" }, ...webRows())] : []),
    el("h2", { class: "settings-group-title", text: t("Voice") }),
    el("section", { class: "card rows" },
      row(t("Voice model and language"), t("Dictation and Chat share one local model and spoken language. Manage both in Settings > Dictation > Voice input."), button(t("Dictation settings"), () => chooseDictation("voice")))),
  ];
}


function voiceRows(): HTMLElement[] {
  if (!voice) return [];
  const status = voice;
  const intro = row(
    t("Voice model and language"),
    t("Shared by Dictation and Chat, even with Zeca off. Audio stays on this computer and is never saved."),
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
    t("Setting your language makes short phrases far more reliable than auto-detect."),
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
    // At 100% the file is whole and being checked against its hash before use.
    if (progress !== undefined) control = el("span", { class: "muted", text: progress >= 100 ? t("Verifying…") : t("Downloading… {pct}%", { pct: progress }) });
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
            voiceError = failure(t("Could not download the model."), e);
          }
          downloading.delete(m.id);
          await refreshVoice();
        },
        // The model that suits the spoken language: Base is weak outside English.
        !status.models.some((x) => x.installed) && m.id === (/^(en|auto)/.test(status.language ?? status.system ?? "en") ? "base" : "small"),
      );
    // What sets each model apart, at a glance (t3code's model cards): speed or accuracy, the GPU,
    // and the one that suits the language spoken.
    const recommended = m.id === (/^(en|auto)/.test(status.language ?? status.system ?? "en") ? "base" : "small");
    const tags = [
      recommended ? [t("Recommended"), "ok"] : null,
      m.id === "base" ? [t("Fastest"), "off"] : m.id === "small" ? [t("Balanced"), "off"] : m.id === "turbo" ? [t("Most accurate"), "off"] : null,
      m.id === "turbo" ? [t("Best with a GPU"), "off"] : null,
    ].filter((x): x is string[] => x !== null);
    const modelRow = row(t(VOICE_MODEL_LABELS[m.id] ?? m.label), m.installed ? t("{size}, on this computer", { size: mb }) : t("{size} whisper.cpp model from Hugging Face, checked before use", { size: mb }), control);
    modelRow.classList.add("model-row");
    modelRow.querySelector(".row-title")?.after(el("div", { class: "model-tags" }, ...tags.map(([text, kind]) => badge(text, kind as "ok" | "off"))));
    return modelRow;
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
      confirmed(t("Remove"), t("Removes the key from the keyring. This provider stops until you save a new one."), t("Remove key"), () =>
        Bridge.apiKeyClear(p.id)
          .then(() => done(t("Key removed from the keyring.")))
          .catch((e) => done(failure(t("Could not remove the key."), e), true)),
      ),
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
      .catch((e) => done(failure(t("Could not save the key."), e), true));
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
  if (page === "chat" || page === "agents") render();
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
  if (page === "chat" || page === "agents") render();
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
      t("API chat can search through your engine and open pages. Up to 200 requests a day, logged locally."),
      toggle(w.on, async (on) => {
        await Bridge.setWeb(on);
        w.on = on;
      }),
      t("Each search or page read is a request to your engine or the page's site: at most 200 a day, all logged locally. A separate model with no tools reads each page; Zeca gets only its answer, and search snippets marked untrusted."),
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
        webMessage = { text: failure(t("Could not change the search engine."), e), error: true };
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
        .catch((e) => said(failure(t("Could not save the address."), e), true));
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") void save();
    });
    rows.push(row(t("Your SearXNG"), t("Its address; it must answer in JSON (format=json)."), el("div", { class: "inline" }, input, button(t("Save"), () => void save(), true))));
  } else if (w.hasKey) {
    rows.push(
      row(
        t("Search key"),
        t("Saved in the system keyring."),
        confirmed(t("Remove"), t("Removes the key from the keyring. Web search stops until you save a new one."), t("Remove key"), () =>
          Bridge.clearWebKey(w.engine)
            .then(() => said(t("Key removed from the keyring.")))
            .catch((e) => said(failure(t("Could not remove the key."), e), true)),
        ),
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
        .catch((e) => said(failure(t("Could not save the key."), e), true));
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") void save();
    });
    rows.push(row(t("Search key"), t("Kept in the system keyring and sent only to that engine."), el("div", { class: "inline" }, input, button(t("Save"), () => void save(), true))));
  }
  if (webMessage) rows.push(el("div", { class: "row" }, el("p", { class: `note${webMessage.error ? " error" : " ok"}`, text: webMessage.text })));
  return rows;
}

// ── About ────────────────────────────────────────────────────────────────────

/** What opening the documentation said, when it failed. */
let aboutNote: string | null = null;

/** Settings → Labs: each beta feature with its switch, all off until turned on here. */
function labsPage(): HTMLElement[] {
  const rows = LABS.map(lab => {
    const blocked = lab.requires !== null && !labOn(labs, lab.requires);
    const control = toggle(labOn(labs, lab.id), async on => {
      await Bridge.setLab(lab.id, on);
      labs = { ...labs, [lab.id]: on };
    });
    control.disabled = blocked;
    const about = lab.restart ? t("{about} It takes effect after Vults restarts.", { about: t(lab.about) }) : t(lab.about);
    return row(t(lab.name), about, control);
  });
  return [
    el("h1", { text: t("Labs") }),
    el("p", { class: "lede", text: t("Beta features. Each stays off until you turn it on, and may change or break.") }),
    el("h2", { class: "settings-group-title", text: t("Chat engines") }),
    el("section", { class: "card rows" }, ...rows),
  ];
}

function aboutPage(): HTMLElement[] {
  return [
    el("h1", { text: t("About") }),
    el(
      "section",
      { class: "card about" },
      (() => { const logo = el("img", { class: "logo" }); logo.alt = "Vults"; logo.addEventListener("error", () => { logo.hidden = true; }); return logo; })(),
      el(
        "div",
        {},
        el("div", { class: "card-title", text: version ? `Vults ${version}` : "Vults" }),
        el("p", { class: "row-about", text: t("A friendly flock watching your coding agents. MIT licensed; no telemetry.") }),
        el("div", { class: "actions about-actions" }, button(t("Documentation"), () => {
          aboutNote = null;
          Bridge.openDocs().catch((e) => { aboutNote = failure(t("Could not open the documentation."), e); render(); });
        })),
        aboutNote ? el("p", { class: "note error", text: aboutNote }) : null,
      ),
    ),
    el("h2", { class: "settings-group-title", text: t("Files on this computer") }),
    el(
      "section",
      { class: "card rows" },
      row(t("Settings file"), t("Every choice on these pages."), el("code", { text: settingsPath })),
      row(t("Data folder"), t("The hook relay, the inbox of dropped files, connector state."), el("code", { text: dataPath })),
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

/** What Zeca remembers, and what may work on it: the Zeca page's last section (deep link
 *  `chatmemory`, from the Nest's Memory). */
function memorySection(): HTMLElement[] {
  const memoryTitle = el("h2", { class: "settings-group-title", text: t("Memory") });
  memoryTitle.dataset.chatSection = "memory";
  memoryTitle.dataset.setting = "chat-memory-heading";
  memoryTitle.tabIndex = -1;
  return [
    memoryTitle,
    ...(chatSection === "memory" ? [button(t("Back to Chat"), () => void Bridge.openNest("memory"))] : []),
    el(
      "section",
      { class: "card rows" },
      row(
        t("Zeca's memory"),
        t("Keeps a local journal, secrets redacted. What Zeca recalls reaches the chat's model. Off, both stop."),
        toggle(memory, async (on) => {
          await Bridge.setMemory(on);
          memory = on;
          render();
        }),
        t("Logs each turn's time and changes, each card's answer and the chat on this computer, secrets redacted. Only what Zeca recalls reaches a model; Clear history in Activity deletes the journal."),
      ),
      ...(memory
        ? [
            row(
              t("A summary of each day"),
              t("Sends yesterday's journal to Claude Code's Haiku, on your subscription, for a summary. Up to 8 requests a day, logged locally."),
              toggle(journalSummary, async (on) => {
                await Bridge.setJournalSummary(on);
                journalSummary = on;
              }),
              t("Once a day, Haiku (no tools) writes a short summary of the previous day for Zeca to recall. Only days since you turned this on; at most 8 requests a day, logged locally."),
            ),
            row(
              t("Suggest what to remember"),
              t("Sends the conversation you leave to Claude Code's Haiku, on your subscription, to suggest memories. Up to 20 requests a day; skips ones with outside content."),
              toggle(memorySuggest, async (on) => {
                await Bridge.setMemorySuggest(on);
                memorySuggest = on;
              }),
              t("Haiku (no tools) proposes up to three facts from it; accept or decline each in the Memory beside the Nest's Chat. At most 20 requests a day, logged locally."),
            ),
            row(
              t("Let Zeca sleep on it"),
              t("At night, while idle, Claude Code's Haiku, on your subscription, proposes memory merges in the Nest. Up to 4 requests a day, logged locally."),
              toggle(sleep, async (on) => {
                await Bridge.setSleep(on);
                sleep = on;
              }),
              t("Each night from 2 a.m., while idle, Zeca proposes removing duplicate facts. With this on, Haiku (no tools) also reads his memories and last week's summaries to propose merges, which wait in the Nest, and writes a short debrief."),
            ),
            row(
              t("Recall by meaning"),
              t("Recall by meaning, not only words, with a local Ollama (nomic-embed-text). Nothing leaves this computer."),
              toggle(localEmbeddings, async (on) => {
                await Bridge.setLocalEmbeddings(on);
                localEmbeddings = on;
              }),
            ),
          ]
        : []),
    ),
  ];
}

function zecaPage(): HTMLElement[] {
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
    el("h3", { class: "species-family", text: title }),
    el("div", { class: "species-grid" }, ...SPECIES.filter((s) => s.family === family).map((s) => card(s.id, s.name, s.latin))),
  ];
  // Zeca in his own feathers, or in the look picked (Auto shows his feathers: the season decides).
  const zecaCanvas = document.createElement("canvas");
  zecaCanvas.className = "zeca-preview";
  // His portrait leads the page: twice the species' scale, still whole pixels.
  const HERO = SCALE * 2;
  zecaCanvas.width = W * HERO;
  zecaCanvas.height = (WIRE + 10) * HERO;
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
        drawFrame(ctx, set, frame, 10, WIRE + 6 - perchOf(set), HERO);
      }
    }
  };
  // One timer at a time: every render of this page replaces it.
  window.clearInterval(previews);
  previews = undefined;
  paint();
  if (!matchMedia("(prefers-reduced-motion: reduce)").matches) previews = window.setInterval(paint, 100);
  const species = SPECIES.find((x) => x.id === zecaSpecies);
  return [
    el("h1", { text: "Zeca" }),
    el("p", { class: "lede", text: t("The bird who chats, listens and remembers. Choose who he is and how he looks.") }),
    el("section", { class: "card zeca-hero" },
      zecaCanvas,
      el("div", { class: "zeca-hero-body" },
        el("div", { class: "zeca-hero-name" }, el("span", { text: "Zeca" }), species ? el("span", { class: "zeca-hero-species", text: species.name }) : null),
        el("p", { class: "row-about", text: zeca
          ? t("Zeca is the bird in front: the session that needs you, or the one you picked.")
          : t("Zeca is off: the session in front keeps its own bird.") }),
        el("div", { class: "rows" },
          row(
            t("Zeca on"),
            t("Chats and listens. Off, Chat stops; the flock, cards and dictation keep working."),
            toggle(zeca, async (on) => {
              await Bridge.setZeca(on);
              zeca = on;
              render();
            }),
          )))),
    el("h2", { class: "settings-group-title", text: t("Look") }),
    el("section", { class: "card rows" },
      row(
        t("What he wears"),
        t("Only Zeca wears it; the flock keeps its feathers."),
        dropdown(LOOKS, zecaLook, async (look) => {
          await Bridge.setZecaLook(look);
          zecaLook = look;
          shown.delete(zecaCanvas);
          labelPreview();
          paint();
        }),
        t("Auto picks a seasonal hat: witch (Oct 1 to Nov 1), Santa (Dec 1 to 26), party (Dec 31 to Jan 2), bunny ears (Good Friday to Easter Monday). Other outfits only when you pick them."),
      ),
    ),
    ...memorySection(),
    el("h2", { class: "settings-group-title", text: t("Species") }),
    el("p", { class: "note", text: t("The one you pick is Zeca on the island and in Chat.") }),
    ...group("new-world", t("Vultures of the Americas")),
    disclosure(t("Vultures of Africa, Europe and Asia"), ...group("old-world", t("Vultures of Africa, Europe and Asia")).slice(1)),
  ];
}

function flockPage(): HTMLElement[] {
  window.clearInterval(previews);
  previews = undefined;
  return [
    el("h1", { text: t("Flock") }),
    el("p", { class: "lede", text: t("Your sessions' birds: where they come from, and who drops by.") }),
    el("h2", { class: "settings-group-title", text: t("Birds") }),
    el("section", { class: "card rows" },
      row(
        t("The flock draws from"),
        t("Where session birds come from: Brazil, the Americas (with both condors), or the whole world."),
        segmented(FLOCKS.map((f) => ({ ...f, label: t(f.label) })), flock, async (f) => {
          await Bridge.setFlock(f);
          flock = f;
        }),
        t("Each project keeps the same species, and projects on the wire differ while species last. Three or more sessions bring a king vulture."),
      ),
      row(
        t("Rare visitors"),
        t("Now and then a vulture from outside your flock crosses the sky once. It never lands."),
        toggle(visitors, async (on) => {
          await Bridge.setVisitors(on);
          visitors = on;
        }),
      ),
    ),
  ];
}

// ── Layout ───────────────────────────────────────────────────────────────────

/** Dictation keeps each configuration page's reading position during this window's lifetime. */
let drawn: string | null = null;
const expanded = new Map<string, Set<string>>();
const dictationScroll = new Map<string, number>();

/** Draws the page again when it is the phone's (its rows refresh on their own). */
function renderPhone(): void {
  if (page === "phone") render();
}

function chooseDictation(section: DictationSetting): void {
  focusDestination = true;
  dictationSection = section;
  choosePage("dictation");
}

function choosePage(next: Page, section?: "memory"): void {
  page = next;
  // Dictation's keys show once it is set up: asked again each time the page opens.
  if (next === "shortcuts") void refreshVoice().catch(() => {});
  chatSection = next === "zeca" ? section ?? null : null;
  navigated = true;
  location.hash = next === "dictation" ? `dictation${dictationSection}` : chatSection === "memory" ? "chatmemory" : next;
  apiMessage = null;
  if (next === "agents") doctorAsked = false;
  if (next === "approvals") {
    void refreshRules();
    void refreshPolicies(() => page === "approvals" && render());
    answersLog.refresh();
  }
  if (next === "projects") void refreshProjects();
  if (next === "activity") {
    clearing = false;
    void refreshActivity();
    budget.refresh();
  }
  if (next === "chat" || next === "agents") void refreshApi();
  if (next === "phone") void refreshPhone(renderPhone);
  // Read again on the way in, like Projects and Phone: a stale or failed read is never shown as fresh.
  if (next === "connectors") void refreshConnectors();
  if (next === "dictation" && !dictationTool() && microphones === null && !microphonesFailed) void refreshMicrophones();
  render();
}

const dictationNavigation = createDictationNavigation("dictationstyles", id => void Bridge.openNest(`dictation-${id}`), chooseDictation, () => chooseDictation("voice"));

function render(): void {
  gitea.context(page === "connectors" && !document.hidden, connectorStatus.get("gitea")?.enabled ?? false);
  githubCheck.relabel();
  playerAudio.context(page === "dictation" && dictationSection === "commands" && windowsShortcuts.supported() && !document.hidden, dictationCommands && dictationCommandMedia && nowPlaying);
  const tool = page === "dictation" && dictationTool();
  const preferences = page === "dictation" && !tool;
  const place = tool ? `dictation${dictationSection}` : page === "zeca" && chatSection === "memory" ? "chatmemory" : page;
  if (page === "activity") budget.relabel();
  const currentScroll = root.querySelector("main.page")?.scrollTop ?? 0;
  const sidebarScroll = root.querySelector(".sidebar")?.scrollTop ?? 0;
  if (drawn?.startsWith("dictation")) dictationScroll.set(drawn, currentScroll);
  const scroll = place === drawn ? currentScroll : dictationScroll.get(place) ?? 0;
  if (drawn) expanded.set(drawn, new Set([...root.querySelectorAll<HTMLDetailsElement>("details")].filter(node => node.open).map(node => node.dataset.setting ?? "")));
  const active = document.activeElement as HTMLInputElement | HTMLTextAreaElement | null;
  const focus = drawn === place && active && root.contains(active) && active.dataset.setting ? active : null;
  const editable = focus && (focus.tagName === "TEXTAREA" || (focus.tagName === "INPUT" && !["checkbox", "range"].includes(focus.type))) && !focus.readOnly;
  questionAgents.visible(page === "dictation" && !dictationTool() && !document.hidden);
  questionAgents.relabel();
  meetingAssets.visible(page === "dictation" && !dictationTool() && !document.hidden);
  realtimeSettings.visible(page === "dictation" && !dictationTool() && !document.hidden);
  meetingSettings.visible(page === "dictation" && !dictationTool() && !document.hidden);
  meetingSettings.relabel();
  realtimeSettings.relabel();
  cleanupSettings.visible(page === "dictation" && dictationSection === "styles" && !document.hidden);
  cleanupSettings.relabel();
  meetingAssets.relabel();
  dictationDoctor.visible(page === "dictation" && dictationSection === "diagnostics" && !document.hidden);
  dictationDoctor.relabel();
  head.set(tool ? "dictation" : "settings");
  head.setZeca(zeca);
  head.setSound(sounds);
  foot.setZeca(zeca);
  foot.setSpecies(zecaSpecies);
  const content =
    page === "general"
      ? generalPage()
      : page === "appearance"
        ? appearancePage()
      : page === "island"
        ? islandPage()
      : page === "sound"
        ? soundPage()
      : page === "shortcuts"
        ? shortcutsPage()
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
              ? [el("h1", { text: t("History and budget") }),
                el("p", { class: "lede", text: t("What Vults keeps on this computer, and how much it may spend a day.") }),
                el("h2", { class: "settings-group-title", text: t("Budget") }),
                budget.element,
                el("h2", { class: "settings-group-title", text: t("History") }),
                historyCard(activity, { confirming: clearing }, activityActions),
                el("div", { class: "actions" }, button(t("Open Activity"), () => void Bridge.openNest("week")))]
            : page === "connectors"
              ? connectorsPage()
            : page === "phone"
              ? phonePage()
              : page === "flock"
                ? flockPage()
              : page === "zeca"
                ? zecaPage()
                : page === "labs"
                  ? labsPage()
                  : aboutPage();
  if (page !== "zeca") window.clearInterval(previews);
  const navButton = (p: (typeof PAGES)[number]) => {
    const b = el(
      "button",
      {
        class: `nav${p.id === page ? " on" : ""}`,
        onclick: () => p.id === "dictation" ? chooseDictation("voice") : choosePage(p.id),
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
  picker.addEventListener("change", () => picker.value === "dictation" ? chooseDictation("voice") : choosePage(picker.value as Page));
  const title = content.find(node => node.tagName === "H1");
  const intro = content.find(node => node.classList.contains("lede"));
  const main = el("main", { class: `page${place !== drawn ? " page-enter" : ""}` },
    el("header", { class: "settings-heading" }, el("div", { class: "settings-title-line" }, icon(tool ? DICTATION_SETTINGS.find(p => p.id === dictationSection)!.icon : PAGES.find(p => p.id === page)!.icon, 20, 1.6), title ?? null), intro ?? (tool ? el("p", { class: "lede", text: dictationSettingAbout(dictationSection) }) : null)),
    ...content.filter(node => node !== title && node !== intro));
  main.tabIndex = -1;
  for (const node of main.querySelectorAll<HTMLDetailsElement>("details")) node.open = expanded.get(place)?.has(node.dataset.setting ?? "") ?? node.open;
  root.classList.toggle("dictation-workspace", tool);
  root.classList.toggle("dictation-preferences", preferences);
  if (tool) {
    main.id = "nest-pane-dictation";
    main.setAttribute("role", "tabpanel"); main.setAttribute("aria-labelledby", "nest-tab-dictation");
    dictationNavigation.select(`dictation${dictationSection}`);
    root.replaceChildren(dictationNavigation.element, main);
    dictationNavigation.relabel();
  } else root.replaceChildren(picker, nav, main);
  if (preferences) {
    const target = main.querySelector<HTMLElement>(`[data-dictation-section="${dictationSection}"]`);
    if (target?.tagName === "DETAILS" && focusDestination) (target as HTMLDetailsElement).open = true;
  }
  nav.scrollTop = sidebarScroll;
  if (place !== drawn) nav.querySelector<HTMLElement>(page === "dictation" ? `[data-dictation-page="dictation${dictationSection}"]` : "[aria-current=page]")?.scrollIntoView?.({ block: "nearest" });
  main.scrollTop = scroll;
  if (preferences && focusDestination) main.querySelector<HTMLElement>(`[data-dictation-section="${dictationSection}"]`)?.scrollIntoView?.({ block: "start" });
  if (focus) {
    const next = [...root.querySelectorAll<HTMLInputElement | HTMLTextAreaElement>("[data-setting]")].find(node => node.dataset.setting === focus.dataset.setting && node.tagName === focus.tagName);
    if (next) {
      if (editable) next.value = focus.value;
      next.focus({ preventScroll: true });
      if (editable && focus.selectionStart !== null) next.setSelectionRange(focus.selectionStart, focus.selectionEnd);
    }
  }
  if (focusDestination) {
    const destination = page === "zeca" && chatSection === "memory" ? main.querySelector<HTMLElement>('[data-chat-section="memory"]') : null;
    destination?.scrollIntoView?.({ block: "start" });
    (destination ?? main).focus({ preventScroll: true });
    focusDestination = false;
  }
  drawn = place;
  const logo = root.querySelector<HTMLImageElement>("img.logo");
  if (logo) logo.src = "/icon.png";
}

render();
if (page === "dictation" && !dictationTool()) void refreshMicrophones();
// By the panel the open island may cover this window's corner: a click here folds it, as a click
// outside a panel's popup closes it (the island keeps a waiting card).
window.addEventListener("pointerdown", () => void Bridge.away().catch(() => {}), { capture: true });
if (page === "activity") { void refreshActivity(); budget.refresh(); }
if (page === "phone") void refreshPhone(renderPhone);
Bridge.onPhone(phoneEvent);
watchWidth(() => { if (page === "appearance") render(); });

void Bridge.appSettings().then((s) => {
  dictationCaption = dictation.captions(s.dictationCaption, true);
  dictationSound = dictation.startSound(s.dictationSound, true);
  if (!dictationCommandOutputChanged) dictationCommandOutput = s.dictationCommandOutput ?? false;
  if (!dictationCommandAppsChanged) dictationCommandApps = s.dictationCommandApps ?? false;
  if (!dictationCommandMediaChanged) dictationCommandMedia = s.dictationCommandMedia ?? false;
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
  keepAwake = s.keepAwake ?? true;
  // The nearest choice: the file may hold any number in range.
  foldAfter = FOLD_CHOICES.reduce((a, b) => (Math.abs(b - s.foldAfter) < Math.abs(a - s.foldAfter) ? b : a));
  openOnHover = s.openOnHover;
  language = s.language;
  theme = applyTheme(s.theme ?? DEFAULT_THEME, true);
  setLang(s.lang);
  dictation.relabel();
  head.relabel();
  foot.relabel();
  monitor = s.monitor;
  nowPlaying = s.nowPlaying;
  codexUsage = s.codexUsage;
  claudeUsage = s.claudeUsage ?? false;
  priceUpdates = s.priceUpdates ?? false;
  void Bridge.modelPriceStatus().then(status => { priceStatus = status; if (page === "general") render(); }, () => {});
  void Bridge.updateStatus().then(status => { update = status; if (page === "general") render(); }, () => {});
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
  antigravityApprovals = s.antigravityApprovals;
  labs = s.labs ?? {};
  zecaSpecies = s.zecaSpecies;
  zecaLook = s.zecaLook;
  flock = s.flock;
  visitors = s.visitors;
  presence = s.presence;
  notifications = s.notifications;
  quietWhenWatching = s.quietWhenWatching ?? true;
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
      if (page === "island") render();
    })
    .catch(() => {});
void refreshMonitors();
// The tray asks an open window for one page (Set up agents…).
Bridge.onSettingsSection((section) => {
  const p = asked(section);
  if (p) {
    focusDestination = true;
    const selected = dictationSetting(section);
    if (selected) chooseDictation(selected); else choosePage(p, section === "chatmemory" ? "memory" : undefined);
  }
});
// The island's speaker button changes the sounds too: keep the toggle and the slider in step.
Bridge.onSettings((s) => {
  if (s.labs !== undefined) { labs = s.labs; if (page === "labs") render(); }
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
  if (s.dictationCommandOutput !== undefined) { dictationCommandOutput = s.dictationCommandOutput; dictationCommandOutputChanged = true; if (page === "dictation") render(); }
  if (s.dictationCommandApps !== undefined) { dictationCommandApps = s.dictationCommandApps; dictationCommandAppsChanged = true; if (page === "dictation") render(); }
  if (s.dictationCommandMedia !== undefined) { dictationCommandMedia = s.dictationCommandMedia; dictationCommandMediaChanged = true; if (page === "dictation") render(); }
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
    if (page === "chat" || page === "approvals") render();
  }
  if (s.dndUntil !== undefined) {
    dndUntil = s.dndUntil;
    if (page === "sound") render();
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
if (page === "approvals") answersLog.refresh();
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
window.addEventListener("pagehide", () => { playerAudio.context(false, dictationCommands && dictationCommandMedia && nowPlaying); realtimeSettings.visible(false); questionAgents.visible(false); dictationDoctor.visible(false); meetingAssets.visible(false); meetingSettings.visible(false); });
document.addEventListener("visibilitychange", () => {
  playerAudio.context(page === "dictation" && dictationSection === "commands" && windowsShortcuts.supported() && !document.hidden, dictationCommands && dictationCommandMedia && nowPlaying);
  questionAgents.visible(page === "dictation" && !dictationTool() && !document.hidden);
  meetingAssets.visible(page === "dictation" && !dictationTool() && !document.hidden);
  realtimeSettings.visible(page === "dictation" && !dictationTool() && !document.hidden);
  meetingSettings.visible(page === "dictation" && !dictationTool() && !document.hidden);
  cleanupSettings.visible(page === "dictation" && dictationSection === "styles" && !document.hidden);
  dictationDoctor.visible(page === "dictation" && dictationSection === "diagnostics" && !document.hidden);
  if (!document.hidden) void refreshConnectors();
});
// An agent's files change outside the app (a relay built, a hook edited by hand): read them again
// when the window comes back, unless a review is open (it would lose its diff).
window.addEventListener("focus", () => {
  playerAudio.refreshStatus();
  for (const a of AGENTS) if (!panels.get(a.kind)?.pending) void refresh(a.kind);
});

// Enter the development proposal through the existing Nest route, without changing saved settings.
if (import.meta.env.DEV && import.meta.env.VITE_AGENTS_PREVIEW === "1") void Bridge.openNest("flock-profiles");
