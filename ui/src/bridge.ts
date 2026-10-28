// The only file that talks to Tauri: views in, intents out.
import { invoke } from "@tauri-apps/api/core";
import { emitTo, listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AgentKind, Diff, RowView, SessionView, ViewModel } from "./view.gen";
import type { InstallAgent } from "./agents.gen";
import type { Activity, GithubGrid } from "./surfaces/settings/activity";

// The core's view, generated from crates/core (see `mod ts` in view.rs); the island imports it from here.
/** The four sizes of attention (ADR 0021): the pill, the island, the Nest, the Nest on the whole screen. */
export type Size = "mini" | "quick" | "room" | "full";
export interface VoiceConversation {
  id: string;
  response: string | null;
  capture: string | null;
  routed: boolean;
  semanticFallback: boolean;
  followUp: boolean;
  phase: "preparing" | "starting" | "recording" | "transcribing" | "responding" | "speaking" | "interrupting" | "waiting" | "finished" | "cancelled" | "failed" | "recovery";
  reason: string | null;
  draft: string | null;
  reply: string | null;
}
export interface VoiceRouteSelection { token: string; mode: "headphones" | "echo"; source: string; sink: string; confirmed: boolean }
export interface VoiceRoutes { token: string; sources: { name: string; label: string }[]; sinks: { name: string; label: string }[]; pairs: { source: string; sink: string }[] }
export interface VoiceRealtimeSettings {
  /** Saved opt-in, not a live-session or readiness indicator. */
  on: boolean;
  model: string;
  /** Exact persisted microdollars, including values beyond JavaScript's safe integer range. */
  limitMicrousd: string;
}
export type QuestionAction = "edit" | "send" | "cancel" | "stop-dictation" | "apply-dictation";
export interface TextQuestionView {
  seq: number;
  surface: string;
  phase: string;
  offer: { token: string; question: string; draft: string; clientPid: number; seconds: number; dictation?: { id: number; phase: string; text: string | null } | null } | null;
}
export interface QuestionInstallStatus {
  agent: "claude" | "codex";
  path: string;
  installed: boolean;
  error: string | null;
}
export interface FlockPlace {
  query: string;
  onlyNeeds: boolean;
  detail: "session" | "diff" | "activity" | "context" | "zeca";
  status?: "working" | "failed" | "quiet" | null;
  change?: { owner: string; step: number; focused: boolean } | null;
}
/** A side or corner of the Nest's window, as the compositor takes a resize from it. */
export type ResizeEdge = "North" | "South" | "East" | "West" | "NorthEast" | "NorthWest" | "SouthEast" | "SouthWest";

export type {
  Activity,
  AgentKind,
  AlertLevel,
  AlertView,
  ApprovalView,
  Attention,
  BoardView,
  CardView,
  Checks,
  ChatView,
  ChatSuggestion,
  ChatFollowup,
  Choice,
  Diff,
  DiffSummary,
  DigestView,
  EndedView,
  FileDiff,
  Group,
  Hunk,
  Outcome,
  Outfit,
  Question,
  RowView,
  SessionRef,
  SessionView,
  Silence,
  Status,
  Verdict,
  ViewModel,
} from "./view.gen";

/** The song on screen, from the system's media players. */
export interface NowPlaying {
  title: string;
  artist: string | null;
  playing: boolean;
}
/** Read-only command interpretation; recognition never grants execution. */
export interface CommandPreview {
  status: string;
  actions: { action: string; target: string | null; value: number | null; status: string; candidates: string[] }[];
}
/** Shared local speech model/language inventory; readiness does not verify microphone capture. */
export interface VoiceStatus {
  models: { id: string; label: string; size: number; installed: boolean }[];
  selected: string | null;
  /** As chosen: a code, `auto`, or null to follow the system. */
  language: string | null;
  /** The system's language code. */
  system: string | null;
  downloading: string[];
  ready: boolean;
}
export interface MeetingAssetsStatus {
  id: string;
  phase: "idle" | "checking" | "review" | "downloading" | "cancelling" | "installed" | "cancelled" | "failed" | "unsupported";
  artifact: string;
  downloaded: number;
  total: number;
  error: "verification" | "admission" | "installation" | null;
  bundle: string;
  downloadBytes: number;
}
export type MediaAction = "playpause" | "next" | "previous";

/** One rate-limit window of a subscription, as its CLI reports it. */
export interface UsageWindow {
  agent: AgentKind;
  /** The window's length: 300 is 5 hours, 10080 a week. */
  minutes: number;
  used_percent: number;
  /** Epoch seconds. */
  resets_at: number | null;
}

/** Who the chat talks through: a CLI, or a provider's API with the user's key (or a local model). */
export type ChatProvider = "claude" | "codex" | "api";
/** The paired phone (L2) as Settings shows it. */
export type PhoneDevice = { id: string; name: string; pairedAt: number; revoked: boolean };
export type PhoneSettings = {
  on: boolean;
  transport: "relay" | "direct";
  relay: string | null;
  listen: string | null;
  devices: PhoneDevice[];
};
/** A pairing QR: its text, and its modules row by row ("1" dark). */
export type Pairing = { text: string; size: number; cells: string; seconds: number };
/** How a pairing on screen ended. */
export type PhoneEvent = { paired?: string; expired?: boolean; error?: "closed" | "save" | "serve"; detail?: string };
/** Where Zeca's web search goes (Z7). */
export type WebEngine = "brave" | "tavily" | "searxng";
/** The web tools as Settings shows them: whether a key is saved, never the key. */
export type WebSettings = { on: boolean; engine: WebEngine; searxngUrl: string | null; hasKey: boolean };
/** A model the chat's CLI may run, with the efforts it takes. */
export type ModelChoice = { id: string; label: string; efforts: string[] };
/** A model and an effort for the chat's CLI; none is its own default. */
export type ChatPick = { model?: string | null; effort?: string | null };
/** Each CLI engine of the chat: off the PATH, there, or there and signed in. */
export type EngineState = "missing" | "installed" | "signed-in";
export type ChatEngines = { claude: EngineState; codex: EngineState };

/** The API chat as the island sees it: usable now, and through whom. */
export interface ApiStatus {
  ready: boolean;
  label: string;
}

/** One provider in Settings → Chat. Whether a key is saved, never the key itself. */
export interface ApiProvider {
  id: string;
  label: string;
  local: boolean;
  keyHint: string;
  defaultModel: string;
  hasKey: boolean;
  model: string;
}

/** Where the flock draws its birds from. */
export type Flock = "brazil" | "americas" | "world";

/** A reply to one question: a choice or the user's own words, or several for a multi-select. */
export type Answer = string | string[];

export interface ConnectorZecaTools { snapshot: boolean; calls: number; lastUsed: number | null; workItem: boolean; liveReadCalls: number; liveReadLastUsed: number | null; budget: number }

export interface ConnectorStatus {
  id: string;
  enabled: boolean;
  lastOk: number | null;
  error: string | null;
  watching: number;
  /** The network gate's daily budget for it is spent (ADR 0019): it waits. */
  overBudget: boolean;
  account: string | null;
  host: string | null;
}

/** What the user chose for one project folder; a choice left out is off. */
export interface ProjectPrefs {
  mute?: boolean;
  pin?: boolean;
  hide?: boolean;
  /** Its flock's species, chosen by the user; absent, the pool draws it. */
  species?: string;
}
export type ProjectPref = "mute" | "pin" | "hide";
/** The answer to a quiet bird (crates/core `silence::Hush`). */
export type Hush = "snooze" | "keep-going" | "dismiss";

/** A permission the user chose to always allow: this exact tool and target, in this folder. */
export interface Rule {
  agent: AgentKind;
  cwd: string;
  tool: string;
  target: string;
}

/** CLI execution engines, from the agents' registry. */
export type { Engine } from "./agents.gen";

/** A policy the user accepted (ADR 0004), as it stands. */
export interface PolicyRow {
  consent: number;
  /** Its folder, with `~`; null holds everywhere. */
  project: string | null;
  text: string;
  /** Unix seconds. */
  at: number;
}

export interface PolicyError {
  line: number;
  kind: "unknown-word" | "unknown-what" | "destructive" | "missing" | "metacharacters" | "not-absolute" | "twice" | "unknown-agent" | "empty" | "never-allowed";
}

export interface PolicyPreview {
  /** Against the policy in place for the same folder; empty when it is that one already. */
  diff: string;
  error: PolicyError | null;
  /** The edited policy's folder (or `everywhere`) when accepting revokes it too. */
  revokes: string | null;
}

/** An agent the installer sets up, by the name its hooks run with (the agents' registry): OpenCode gets a plugin file
 *  instead of hooks, and Gemini CLI's hooks, from before its support ended, can only be removed. */
export type { InstallAgent } from "./agents.gen";

export interface InstallStatus {
  agent: InstallAgent;
  configPath: string;
  hookPath: string;
  hookReady: boolean;
  installed: boolean;
  /** Installed, but older than what this version writes: reinstalling brings what is new. */
  outdated: boolean;
  /** Outdated only because the hooks run the hook at this other path (another data folder). */
  otherHookPath: string | null;
  error: string | null;
  /** Why installing or updating would be refused now (the user's status line sits beside another data folder's hook). */
  installBlocked: string | null;
  /** Codex only: whether it will run our hooks. */
  /** `error`: its config.toml could not be read or parsed, and the counts say nothing. */
  codex: { hooksDisabled: boolean; untrusted: number; total: number; error: string | null } | null;
  /** Claude Code only: whose statusLine the config has. Installing over theirs keeps it running behind ours. */
  statusLine: "none" | "ours" | "theirs" | null;
}

export interface InstallPreview {
  diff: string;
  fingerprint: string;
}

/** A repository the flock works in, with its sessions, GitHub rows and memory (M3). */
export type RoostView = {
  key: string;
  name: string;
  github: string | null;
  trees: { path: string; branch: string | null }[];
  /** `kind` and `id` say which session, to put it in front. */
  sessions: { agent: string; status: SessionView["status"]; step: string | null; kind: AgentKind; id: string }[];
  /** `item` and `link` are the GitHub card's row: `boardOpen("github", item)` opens it. */
  prs: { name: string; title: string; checks: RowView["checks"]; review: RowView["review"]; item: string; link: boolean }[];
  checks: RowView["checks"];
  /** The card's row for the default branch's checks, when it opens something. */
  checksItem: string | null;
  memory: string[];
  /** Whether Claude Code reads the AGENTS.md Codex reads (M9): "shared", "offer" or "". */
  agents: "shared" | "offer" | "";
  /** What its agents cost, today and in all (W9); null before anything was spent here. */
  cost: { today: number; total: number; subscription: boolean } | null;
};

/** The Nest's records (W11): finished turns (counts only), the audit log, Zeca's traces, usage. */
export type TurnRow = {
  end: number;
  agent: AgentKind;
  project: string;
  secs: number;
  steps: number;
  commands: number;
  files: number;
  added: number;
  removed: number;
  failed: boolean;
};
export type HistoryView = { on: boolean; turns: TurnRow[] };
export type AuditRow = {
  at: number;
  actor: string;
  act: string;
  agent: string;
  project: string;
  tool: string;
  target: string;
  consent: number | null;
};
export type TraceRow = {
  startMs: number;
  ms: number;
  name: string;
  kind: string;
  status: string;
  child: boolean;
  input: number | null;
  output: number | null;
  costUsd: number | null;
};
export type UsageView = {
  windows: { agent: string; minutes: number; used_percent: number; resets_at: number | null }[];
  days: { day: string; usd: number; tokens: number }[];
  roosts: { name: string; today: number; total: number }[];
  budget: number | null;
  subscription: boolean;
};

/** Today's spend and the daily budget (W9). */
export type SpendToday = { usd: number; tokens: number; budget: number | null; subscription: boolean };

/** The change that would let Claude Code read a project's AGENTS.md, to see before writing. */
export type SharePreview = { path: string; file: string; diff: string; fingerprint: string };

/** Zeca's memory as the Memory beside the Nest's Chat shows it (M6). */
export type MemoryView = {
  on: boolean;
  files: { place: string; path: string; facts: string[]; used: number; limit: number }[];
  pending: { id: number; op: "add" | "replace" | "remove"; place: string; folder: string; old: string; new: string; reason: string; source: "tool" | "extraction" | "sleep" }[];
  history: { id: string; message: string; by: "you" | "zeca" | "zeca-alone" | "sleep" | "other"; seconds: number }[];
  accepted: number;
  rejected: number;
  /** The last night's sleep (M8), if it ran in the last two days. */
  night: { day: string; debrief: string } | null;
};

/** One agent in Settings → Agents (W1): what it can do with Vults, and what this computer has. */
export type AgentRow = {
  id: string;
  name: string;
  command: string;
  permissions: "card" | "terminal";
  questions: "card" | "terminal";
  diffs: "numbered" | "lines" | "none";
  stop: "interrupt" | "no";
  sandbox: "bubblewrap" | "codex" | "container" | "no";
  starts: boolean;
  path: string | null;
  version: string | null;
  hooks: "ok" | "outdated" | "missing" | "untrusted" | "error" | "broken";
};

/** A saved conversation in the chat's list. */
export type ConversationView = { id: number; title: string; provider: ChatProvider; label: string; folder: string | null; updatedMs: number; current: boolean };
export type SpecialistConversationView = ConversationView & { profileTitle: string; profileRevision: number };
export type AgentGithubGrant = { version:number; enabled:boolean; repository:string; host:string; account:string };
export type SpecialistCurrent = { choice?: AgentDefaults | null; generation: number; conversation: OpenedConversation | null; draft: ChatDraft; profileTitle: string; profileRevision: number; profileSpecies: string; profileLook: string };
export type SpecialistChatEvent = { profile: string; delta: ChatDelta };
export type SpecialistChatSync = { profile: string; sync: ChatSync };
/** A saved conversation, opened: what the chat goes on with. */
export type OpenedConversation = {
  generation?: number;
  /** A turn runs: what Zeca said so far, and the cards it waits on. */
  busy: boolean;
  partial: string;
  tools?: boolean;
  activity?: ChatActivity;
  waiting: { id: string; tool: string; target: string; outside: string | null; description?: string | null; full?: string | null; added?: number; removed?: number; always?: boolean }[];
  provider: ChatProvider;
  /** Who it talks to: Claude, Codex, or the API provider it was pinned to. */
  label: string;
  folder: string | null;
  started: boolean;
  model: string | null;
  effort: string | null;
  lines: { who: string; text: string; files: string[] }[];
  profileTitle?: string;
  profileRevision?: number;
  profileSpecies?: string;
  profileLook?: string;
};

/** What one of the chat's windows did, for the other to show too; each skips its own (`from`). */
export type ChatMention = { token: string; target: { kind: "session"; agent: string; id: string } | { kind: "project"; cwd: string } };
export type ChatDraft = { generation?: number; text: string; files: string[]; folder: string | null; provider: ChatProvider | null; mentions?: ChatMention[] };
export type ConnectionsPlace = {
  chosen: string | null;
  picked: string | null;
  repo: string | null;
  tab: "inbox" | "prs" | "runs" | "rels" | null;
  detailOpen: boolean;
  zeca?: boolean;
};
export type ChatSync =
  | { kind: "draft"; from: string; draft: ChatDraft; generation?: number }
  | { kind: "sent"; from: string; text: string; files: string[]; provider: ChatProvider; folder: string | null; model: string | null; effort: string | null; mentions?: ChatMention[] }
  | { kind: "reset"; from: string; provider: ChatProvider; generation?: number; draft?: ChatDraft | null }
  | { kind: "opened"; from: string; conversation: OpenedConversation }
  | { kind: "answered"; from: string; id: string; answer: ChatAnswer };

/** A click on the chat's card; `always` allows the same whole command until the conversation ends. */
export type ChatAnswer = "allow" | "deny" | "always";

/** Who answers the chat's cards (ADR 0004): a card, the user's policies, or full access. */
export type ChatAccess = "ask" | "policies" | "full";

export type ChatActivityEvent =
  | { type: "reasoning"; text: string }
  | { type: "start"; id: string; tool: string; detail: string }
  | { type: "end"; id: string; ok: boolean }
  | { type: "decision"; tool: string; target: string; answer: string; by: string };
export type ChatActivity = {
  reasoning: string;
  tools: { id: string; tool: string; detail: string; ok: boolean | null }[];
  decisions: { tool: string; target: string; answer: string; by: string }[];
  truncated: boolean;
};

export type ChatDelta =
  | { kind: "text"; text: string }
  | { kind: "activity"; event: ChatActivityEvent }
  | {
      kind: "permission";
      id: string;
      tool: string;
      target: string;
      description: string | null;
      full: string | null;
      added: number;
      removed: number;
      /** The card may offer *Always in this chat*: a rule could bind to this very call. */
      always: boolean;
      /** The conversation read content from outside, through this tool: the card warns. */
      outside: string | null;
    }
  | { kind: "allowed"; tool: string; target: string }
  /** The user's own choice for the chat answered with no card (ADR 0004). */
  | { kind: "settled"; tool: string; target: string; allow: boolean; by: "policy" | "full-access" }
  | { kind: "tool"; tool: string; outside: boolean }
  | { kind: "toolstate"; active: boolean }
  | { kind: "done" }
  | { kind: "stopped" }
  | { kind: "error"; message: string };

/** Files dropped on the island or picked with a paperclip: the inbox copies, the ones refused with
 *  why, and the page whose chat takes them. */
export type SpecialistFiles = Dropped & { generation: number };
export interface Dropped {
  copied: string[];
  refused: { name: string; reason: "folder" | "too-big" | "unreadable" }[];
  to: "island" | "nest";
}

/** A setting that changed somewhere (the island, the tray, Settings); only those present changed. */
export type Theme = "sp-night-paulista" | "sp-night-garoa" | "sp-night-jaragua" | "dark" | "light";
export type DictationWords = {
  words: string[];
  replacements: { id: number; from: string; to: string }[];
  snippets: { id: number; trigger: string; text: string }[];
};
export type DictationWordsChange =
  | { action: "addWord" | "deleteWord"; word: string }
  | { action: "setReplacement"; from: string; to: string }
  | { action: "deleteReplacement" | "deleteSnippet"; id: number }
  | { action: "addSnippet"; trigger: string; text: string };

export type DictationTry = { text: string; cancelled: boolean; segments: { text: string; kind: "snippet" | "replacement" | "spelling" | "spokenCode" | null; rule: string | null }[] };

export type DictationStyle = "verbatim" | "natural" | "polished" | "prompt";
export type DictationOutputLanguage = "en" | "pt-BR" | "es" | "zh";

export type DictationMusic = "lower" | "pause" | "leave";

export type SettingsChange = { questionAgents?: boolean; theme?: Theme; sounds?: boolean; volume?: number; api?: ApiStatus; foldAfter?: number; openOnHover?: boolean; voice?: boolean; zecaSpecies?: string; visitors?: boolean; presence?: Presence; zeca?: boolean; nativeChat?: boolean; dictationKeep?: string; dictationSound?: boolean; dictationCommands?: boolean; dictationCommandMedia?: boolean; dictationCommandApps?: boolean; dictationCommandOutput?: boolean; dictationCleanup?: boolean; meetingSummary?: boolean; voiceRealtime?: boolean; dictationStyle?: DictationStyle; dictationWriteIn?: DictationOutputLanguage | null; dictationSessionsEnglish?: boolean; dictationCommandChat?: boolean; dictationMusic?: DictationMusic; dictationCancelByVoice?: boolean; dictationSpokenCode?: boolean; dictationCutRepeats?: boolean; dictationRetryBase?: boolean; dictationVocabulary?: boolean; dictationProjectVocabulary?: boolean; dictationMicrophone?: string | null; dictationDoubleTap?: boolean; dictationSpaceLock?: boolean; dictationEscapeCancel?: boolean; dictationCaption?: boolean; dictationInsert?: boolean; chatAccess?: ChatAccess; memory?: boolean; journalSummary?: boolean; localEmbeddings?: boolean; memorySuggest?: boolean; sleep?: boolean; zecaLook?: string; widget?: Corner | null; projects?: Record<string, ProjectPrefs>; dndUntil?: number | null; language?: string | null; lang?: string; history?: boolean; flock?: Flock };

/** The presence preset (crates/core `Presence`). */
export type Presence = "island" | "panel" | "quiet" | "paused";
export type Place = { presence: Presence; dock: "top" | "bottom" };
/** The screen corner of the corner widget. */
export type Corner = "top-left" | "top-right" | "bottom-left" | "bottom-right";

export type AgentProfileEdit =
  | { action: "create"; title: string; instructions: string; species: string; look: string; skill_ids: string[] }
  | { action: "copy"; preset: string; title: string; instructions: string; species: string; look: string; skill_ids: string[] }
  | { action: "update"; id: number; revision: number; title: string; instructions: string; species: string; look: string; skill_ids: string[] };

export type AgentDefaults =
  | { executor: "claude" | "codex"; model: string | null; effort: string | null }
  | { executor: "api"; provider: string; model: string | null };
export interface AgentProfileState { version: number; defaults: AgentDefaults; archived: boolean }
export type AgentStateChange = { action: "defaults"; defaults: AgentDefaults } | { action: "archive"; archived: boolean };
export interface AgentPreferenceEdit { version: number; defaults: AgentDefaults }
export interface AgentProvider { id: string; label: string; local: boolean }
export interface AgentSkill { id: string; revision: number; title: string; content: string }

export interface AgentProfile {
  github?: AgentGithubGrant;
  built_in: boolean;
  id: string;
  revision: number;
  title: string;
  about: string;
  species: string;
  look: string;
  instructions: string;
  skills: AgentSkill[];
  skill_catalog: AgentSkill[];
  access: ChatAccess;
  state?: AgentProfileState;
  providers?: AgentProvider[];
}

export const Bridge = {
  agentProfiles: () => invoke<AgentProfile[]>("agent_profiles"),
  agentProfileSave: (edit: AgentProfileEdit, preferences?: AgentPreferenceEdit) => invoke<AgentProfile>("agent_profile_save", { edit, preferences: preferences ?? null }),
  agentProfileStateSet: (profile: string, version: number, change: AgentStateChange) => invoke<AgentProfileState>("agent_profile_state_set", { profile, version, change }),
  onAgentProfileState(cb: (event: {profile: string; state: AgentProfileState}) => void): void { void listen<{profile: string; state: AgentProfileState}>("agent-profile-state", e => cb(e.payload)); },
  setSpecialistGithub: (profile:string,version:number,enabled:boolean,repository:string) => invoke<AgentGithubGrant>("set_specialist_github",{profile,version,enabled,repository}),
  setSpecialistChatAccess: (profile: string, access: ChatAccess) => invoke<void>("set_specialist_chat_access", { profile, access }),
  onView(cb: (v: ViewModel) => void): void {
    void listen<ViewModel>("view", (e) => cb(e.payload));
    void invoke<ViewModel | null>("current_view").then((v) => v && cb(v));
  },
  decide: (request: string, decision: "allow" | "deny") => invoke<void>("decide", { request, decision }),
  layout: (x: number, y: number, width: number, height: number) =>
    invoke<void>("layout", { x, y, width, height }),
  installStatus: (agent: InstallAgent) => invoke<InstallStatus>("install_status", { agent }),
  installPreview: (agent: InstallAgent, install: boolean) =>
    invoke<InstallPreview>("install_preview", { agent, install }),
  installApply: (agent: InstallAgent, install: boolean, fingerprint: string) =>
    invoke<string | null>("install_apply", { agent, install, fingerprint }),
  decideAlways: (request: string) => invoke<void>("decide_always", { request }),
  setQuestionAgents: (on: boolean) => invoke<void>("question_agents_set", { on }),
  questionInstallStatus: () => invoke<QuestionInstallStatus[]>("question_install_status"),
  questionInstallPreview: (agent: string, install: boolean) => invoke<InstallPreview>("question_install_preview", { agent, install }),
  questionInstallApply: (agent: string, install: boolean, fingerprint: string) => invoke<string | null>("question_install_apply", { agent, install, fingerprint }),
  textQuestionStatus: (token: string | null) => invoke<TextQuestionView>("text_question_status", { token }),
  textQuestionAction: (token: string, action: QuestionAction, text: string, capture: number | null = null) => invoke<void>("text_question_action", { token, action, text, capture }),
  textQuestionDepart: () => invoke<TextQuestionView>("text_question_depart"),
  onTextQuestion(cb: (view: TextQuestionView) => void): void { void listen<TextQuestionView>("text-question", e => cb(e.payload)); },
  questionAnswer: (request: string, answers: Answer[]) => invoke<void>("question_answer", { request, answers }),
  questionRelease: (request: string) => invoke<void>("question_release", { request }),
  cardSkip: (request: string) => invoke<void>("card_skip", { request }),
  cardReview: (request: string) => invoke<void>("card_review", { request }),
  rulesList: () => invoke<Rule[]>("rules_list"),
  policiesList: () => invoke<PolicyRow[]>("policies_list"),
  /** `replaces`: the policy being edited, revoked too when the text moves it to another folder. */
  policyPreview: (text: string, replaces: number | null) => invoke<PolicyPreview>("policy_preview", { text, replaces }),
  /** The click: `shown` is the diff the user saw, refused if it is no longer the one. */
  policyAccept: (text: string, shown: string, replaces: number | null) => invoke<void>("policy_accept", { text, shown, replaces }),
  policyRevoke: (consent: number) => invoke<void>("policy_revoke", { consent }),
  ruleRemove: (index: number) => invoke<void>("rule_remove", { index }),
  sessionJump: (agent: AgentKind, id: string) => invoke<void>("session_jump", { agent, id }),
  /** Puts a session in front; null gives the choice back to core's rule. */
  sessionFocus: (s: { agent: AgentKind; id: string } | null) => invoke<void>("session_focus", { agent: s?.agent ?? null, id: s?.id ?? null }),
  /** Quick actions: the session's folder, or a file of a step's diff, in VS Code or the file manager. */
  sessionOpenFolder: (agent: AgentKind, id: string) => invoke<void>("session_open_folder", { agent, id }),
  sessionOpenFile: (agent: AgentKind, id: string, step: number, file: number) => invoke<void>("session_open_file", { agent, id, step, file }),
  /** A quick action: mute, pin or hide the session's project (its folder), or undo it. */
  sessionProjectPref: (agent: AgentKind, id: string, pref: ProjectPref, on: boolean) => invoke<void>("session_project_pref", { agent, id, pref, on }),
  sessionProjectBird: (agent: AgentKind, id: string, species: string | null) => invoke<void>("session_project_bird", { agent, id, species }),
  /** The answer to a quiet bird: it changes only its flag, never the agent. */
  sessionHush: (agent: AgentKind, id: string, hush: Hush) => invoke<void>("session_hush", { agent, id, hush }),
  /** Every project with a choice on, by folder. */
  projectsList: () => invoke<Record<string, ProjectPrefs>>("projects_list"),
  /** One project's choices from Settings; all off forgets it. */
  projectSet: (cwd: string, prefs: ProjectPrefs) => invoke<void>("project_set", { cwd, prefs: { mute: !!prefs.mute, pin: !!prefs.pin, hide: !!prefs.hide, species: prefs.species ?? null } }),
  /** Whether `code` is on the PATH, for the quick actions' words. */
  editorFound: () => invoke<boolean>("editor_found"),
  stepDiff: (agent: AgentKind, id: string, step: number) => invoke<Diff | null>("step_diff", { agent, id, step }),
  alertOpen: (key: string) => invoke<void>("alert_open", { key }),
  alertDismiss: (key: string) => invoke<void>("alert_dismiss", { key }),
  boardOpen: (connector: string, item: string) => invoke<void>("board_open", { connector, item }),
  /** *Squash and merge*, confirmed: the core merges only a row it offered it for. */
  boardMerge: (connector: string, item: string) => invoke<void>("board_merge", { connector, item }),
  /** How a confirmed merge went: `error` is GitHub's refusal, none once merged. */
  onRowMerged(cb: (m: { connector: string; item: string; error: string | null; status?: string }) => void): void {
    void listen<{ connector: string; item: string; error: string | null; status?: string }>("row-merged", (e) => cb(e.payload));
  },
  connectorZecaTools: () => invoke<ConnectorZecaTools>("connector_zeca_tools"),
  setConnectorZecaSnapshot: (on: boolean) => invoke<void>("set_connector_zeca_snapshot", { on }),
  setConnectorZecaWorkItem: (on: boolean) => invoke<void>("set_connector_zeca_work_item", { on }),
  connectorsStatus: () => invoke<ConnectorStatus[]>("connectors_status"),
  connectorEnable: (id: string, on: boolean) => invoke<void>("connector_enable", { id, on }),
  connectorsRefresh: () => invoke<void>("connectors_refresh"),
  /** Opens Settings, at one page (`agents`…) when given. */
  openSettings: (section?: string) => invoke<void>("open_settings_window", { section: section ?? null }),
  /** The app asks an open Settings to show one page. */
  onSettingsSection(cb: (section: string) => void): void {
    void listen<string>("settings-section", (e) => cb(e.payload));
  },
  /** Zeca on or off: off, no chat, mic or talk shortcut; the flock keeps working. */
  setZeca: (on: boolean) => invoke<void>("set_zeca", { on }),
  /** Build capability only; this does not inspect model files or microphone readiness. */
  voiceConversationOptions: () => invoke<{ semantic: boolean }>("voice_conversation_options"),
  voiceConversationRoutes: () => invoke<VoiceRoutes>("voice_conversation_routes"),
  voiceConversationStart: (autoFinish = false, route?: VoiceRouteSelection, semantic = false, followUp = false) => invoke<VoiceConversation>("voice_conversation_start", { autoFinish, route, semantic, followUp }),
  voiceConversationStatus: (id: string) => invoke<VoiceConversation>("voice_conversation_status", { id }),
  voiceConversationControl: (id: string, action: "finish" | "cancel" | "next" | "interrupt", response?: string, capture?: string) => invoke<void>("voice_conversation_control", { id, action, response, capture }),
  voiceRealtimeSettings: () => invoke<VoiceRealtimeSettings>("voice_realtime_settings"),
  setVoiceRealtime: (on: boolean, limitMicrousd: string, model: string) =>
    typeof limitMicrousd === "string" && /^(0|[1-9]\d{0,18})$/.test(limitMicrousd) && BigInt(limitMicrousd) <= 9223372036854775807n
      ? invoke<void>("set_voice_realtime", { on, limitMicrousd, model })
      : Promise.reject(new Error("Invalid realtime voice budget.")),
  appSettings: () =>
    invoke<{ questionAgents: boolean; theme: Theme; history: boolean; sounds: boolean; volume: number; autostart: boolean; foldAfter: number; openOnHover: boolean; monitor: string | null; nowPlaying: boolean; codexUsage: boolean; nativeChat: boolean; dictationKeep: string; dictationSound: boolean; dictationCommands: boolean; dictationCommandMedia: boolean; dictationCommandApps: boolean; dictationCommandOutput: boolean; dictationCleanup: boolean; meetingSummary: boolean; dictationStyle: DictationStyle; dictationWriteIn: DictationOutputLanguage | null; dictationSessionsEnglish: boolean; dictationCommandChat: boolean; dictationMusic: DictationMusic; dictationCancelByVoice: boolean; dictationSpokenCode: boolean; dictationCutRepeats: boolean; dictationRetryBase: boolean; dictationVocabulary: boolean; dictationProjectVocabulary: boolean; dictationMicrophone: string | null; dictationDoubleTap: boolean; dictationSpaceLock: boolean; dictationEscapeCancel: boolean; dictationCaption: boolean; dictationInsert: boolean; dictationInsertAvailable: boolean; dictationWayland: boolean; chatAccess: ChatAccess; memory: boolean; journalSummary: boolean; localEmbeddings: boolean; memorySuggest: boolean; sleep: boolean; zecaSpecies: string; zecaLook: string; flock: Flock; visitors: boolean; presence: Presence; notifications: boolean; zeca: boolean; widget: Corner | null; dndUntil: number | null; language: string | null; lang: string; settingsPath: string; dataPath: string }>("app_settings"),
  /** "While you were away" read: it goes. */
  digestDismiss: () => invoke<void>("digest_dismiss"),
  recapDismiss: () => invoke<void>("recap_dismiss"),
  /** Do not disturb for this many minutes, or off with null: no sounds or notifications; cards still show. */
  setDnd: (minutes: number | null) => invoke<void>("set_dnd", { minutes }),
  /** Desktop notifications (finished, failed, a card waiting): on or off. */
  setNotifications: (on: boolean) => invoke<void>("set_notifications", { on }),
  /** The presence preset: Island, Panel (by the tray), Quiet or Paused. Switches at once. */
  setPresence: (presence: Presence) => invoke<void>("set_presence", { presence }),
  /** Where the island is now, for its page on load. */
  islandPlace: () => invoke<Place>("island_place"),
  onPlace(cb: (p: Place) => void): void {
    void listen<Place>("place", (e) => cb(e.payload));
  },
  /** Rare visitors crossing the sky: on or off. */
  setVisitors: (on: boolean) => invoke<void>("set_visitors", { on }),
  /** Zeca's species, by id (ui/src/character/flock/species.ts). */
  setZecaSpecies: (id: string) => invoke<void>("set_zeca_species", { id }),
  /** Zeca's look: "auto" (the calendar), "none", or a look id. The island gets it in the view. */
  setZecaLook: (look: string) => invoke<void>("set_zeca_look", { look }),
  /** Where the other sessions' birds are drawn from; their birds are drawn again at once. */
  setFlock: (flock: Flock) => invoke<void>("set_flock", { flock }),
  setNowPlaying: (on: boolean) => invoke<void>("set_now_playing", { on }),
  setCodexUsage: (on: boolean) => invoke<void>("set_codex_usage", { on }),
  dictationPermission: () => invoke<DictationPermission>("dictation_permission"),
  onDictationPermission: (cb: (state: DictationPermission) => void) => { void listen<DictationPermission>("dictation-permission", e => cb(e.payload)); },
  retryDictationPermission: () => invoke<void>("retry_dictation_permission"),
  dictationWords: () => invoke<DictationWords>("dictation_words"),
  dictationTry: (text: string, terminal: boolean) => invoke<DictationTry>("dictation_try", { text, terminal }),
  changeDictationWords: (edit: DictationWordsChange) => invoke<void>("dictation_words_change", { edit }),
  meetingSummaryStatus: (id: number) => invoke<MeetingSummaryStatus>("meeting_summary_status", { id }),
  meetingSummaryBegin: (id: number) => invoke<MeetingSummaryStatus>("meeting_summary_begin", { id }),
  meetingSummaryCancel: (id: number) => invoke<void>("meeting_summary_cancel", { id }),
  setMeetingSummary: (on: boolean) => invoke<void>("set_meeting_summary", { on }),
  meetingTranscript: (id: number) => invoke<MeetingTranscript | null>("meeting_transcript", { id }),
  meetingSave: (id: number, title: string) => invoke<string>("meeting_save", { id, title }),
  meetingArchiveList: () => invoke<{ epoch: number; rows: MeetingEntry[] }>("meeting_archive_list", { limit: 500 }),
  meetingArchiveRead: (id: string) => invoke<{ epoch: number; meeting: SavedMeeting | null }>("meeting_archive_read", { id }),
  meetingArchiveDelete: (id: string) => invoke<boolean>("meeting_archive_delete", { id }),
  meetingArchiveRename: (id: string, source: MeetingSource, speaker: number, name: string | null) => invoke<boolean>("meeting_archive_rename", { id, source, speaker, name }),
  meetingArchiveExport: (id: string, format: "markdown" | "srt" | "vtt", source: MeetingSource | null, title: string) => invoke<boolean>("meeting_archive_export", { id, format, source, title }),
  onMeetingArchiveErased(cb: () => void): void { void listen("meeting-archive-erased", cb); },
  meetingStatus: (id: number | null) => invoke<MeetingStatus>("meeting_status", { id }),
  meetingAssetsStatus: () => invoke<MeetingAssetsStatus>("meeting_assets_status"),
  meetingAssetsReview: () => invoke<MeetingAssetsStatus>("meeting_assets_review"),
  meetingAssetsBegin: (id: string, consent: boolean) => invoke<MeetingAssetsStatus>("meeting_assets_begin", { id, consent }),
  meetingAssetsCancel: (id: string) => invoke<MeetingAssetsStatus>("meeting_assets_cancel", { id }),
  meetingPrepare: () => invoke<MeetingStatus>("meeting_prepare"),
  meetingBegin: (id: number, consent: boolean, identifySpeakers = false) => invoke<MeetingStatus>("meeting_begin", { id, consent, identifySpeakers }),
  meetingEnd: (id: number, cancel: boolean) => invoke<MeetingStatus>("meeting_end", { id, cancel }),
  meetingRecover: () => invoke<MeetingStatus>("meeting_recover"),
  dictationHistory: () => invoke<DictationHistory>("dictation_history"),
  dictationStatistics: (period: DictationPeriod) => invoke<DictationStatistics>("dictation_statistics", { period }),
  dictationDelete: (id: number) => invoke<void>("dictation_delete", { id }),
  dictationClear: () => invoke<void>("dictation_clear"),
  dictationPasteAgain: (id: number) => invoke<void>("dictation_paste_again", { id }),
  setDictationKeep: (value: string) => invoke<void>("set_dictation_keep", { value }),
  onDictationHistory(cb: (erased: boolean) => void): void { void listen<boolean>("dictation-history", e => cb(e.payload)); },
  dictationDoctor: () => invoke<DictationDiagnostics>("dictation_doctor"),
  dictationMicStart: () => invoke<number>("dictation_mic_start"),
  dictationMicStatus: (id: number) => invoke<DictationMicTest>("dictation_mic_status", { id }),
  dictationMicStop: (id: number) => invoke<void>("dictation_mic_stop", { id }),
  dictationMicrophones: () => invoke<string[]>("dictation_microphones"),
  setDictationMicrophone: (device: string | null) => invoke<void>("set_dictation_microphone", { device }),
  setDictationMusic: (mode: DictationMusic) => invoke<void>("set_dictation_music", { mode }),
  setDictationProcessing: (key: "cutRepeats" | "retryBase" | "spokenCode" | "cancelByVoice" | "vocabulary" | "projectVocabulary", on: boolean) => invoke<void>("set_dictation_processing", { key, on }),
  setDictationGesture: (key: "doubleTap" | "spaceLock" | "escapeCancel", on: boolean) => invoke<void>("set_dictation_gesture", { key, on }),
  dictationStyles: () => invoke<Record<string, string>>("dictation_styles"),
  setDictationStyleOptions: (style: DictationStyle, writeIn: DictationOutputLanguage | null, sessionsEnglish: boolean) => invoke<void>("set_dictation_style_options", { style, writeIn, sessionsEnglish }),
  setDictationAppStyle: (identity: string, style: DictationStyle | null) => invoke<void>("set_dictation_app_style", { identity, style }),
  onDictationStyles: (cb: () => void) => { void listen("dictation-styles", cb); },
  setDictationCleanup: (on: boolean) => invoke<void>("set_dictation_cleanup", { on }),
  setDictationCommands: (key: "enabled" | "chat" | "media" | "applications" | "output", on: boolean) => invoke<void>("set_dictation_commands", { key, on }),
  setDictationSound: (on: boolean) => invoke<void>("set_dictation_sound", { on }),
  setDictationCaption: (on: boolean) => invoke<void>("set_dictation_caption", { on }),
  setDictationInsert: (on: boolean) => invoke<string | null>("set_dictation_insert", { on }),
  setNativeChat: (on: boolean) => invoke<void>("set_native_chat", { on }),
  /** More access only from Settings; the chat's picker only gives some back. */
  setChatAccess: (access: ChatAccess) => invoke<void>("set_chat_access", { access }),
  setMemory: (on: boolean) => invoke<void>("set_memory", { on }),
  setJournalSummary: (on: boolean) => invoke<void>("set_journal_summary", { on }),
  setLocalEmbeddings: (on: boolean) => invoke<void>("set_local_embeddings", { on }),
  setMemorySuggest: (on: boolean) => invoke<void>("set_memory_suggest", { on }),
  setSleep: (on: boolean) => invoke<void>("set_sleep", { on }),
  webSettings: () => invoke<WebSettings>("web_settings"),
  setWeb: (on: boolean) => invoke<void>("set_web", { on }),
  setWebEngine: (engine: WebEngine) => invoke<void>("set_web_engine", { engine }),
  setSearxngUrl: (url: string) => invoke<void>("set_searxng_url", { url }),
  setWebKey: (engine: WebEngine, key: string) => invoke<void>("set_web_key", { engine, key }),
  clearWebKey: (engine: WebEngine) => invoke<void>("clear_web_key", { engine }),
  phoneSettings: () => invoke<PhoneSettings>("phone_settings"),
  setPhone: (on: boolean) => invoke<void>("set_phone", { on }),
  setPhoneTransport: (transport: PhoneSettings["transport"]) => invoke<void>("set_phone_transport", { transport }),
  setPhoneRelay: (url: string) => invoke<void>("set_phone_relay", { url }),
  setPhoneListen: (addr: string) => invoke<void>("set_phone_listen", { addr }),
  phonePair: () => invoke<Pairing>("phone_pair"),
  phonePairCancel: () => invoke<void>("phone_pair_cancel"),
  phoneRevoke: (id: string) => invoke<void>("phone_revoke", { id }),
  onPhone(cb: (e: PhoneEvent) => void): void {
    void listen<PhoneEvent>("phone", (e) => cb(e.payload));
  },
  mediaControl: (action: MediaAction) => invoke<void>("media_control", { action }),
  /** The song on screen now, for an island that loads after it was sent. */
  mediaNow: () => invoke<NowPlaying | null>("media_now"),
  commandCheck: (text: string) => invoke<CommandPreview>("voice_command_check", { text }),
  commandReadiness: async (includeMicrophones = false) => {
    const [voice, settings, microphones] = await Promise.allSettled([
      Bridge.voiceStatus(), Bridge.appSettings(), includeMicrophones ? Bridge.dictationMicrophones() : Promise.resolve(null),
    ]);
    if (voice.status === "rejected" && settings.status === "rejected" && (!includeMicrophones || microphones.status === "rejected")) {
      throw new Error("Could not read command setup.");
    }
    return {
      voice: voice.status === "fulfilled" ? voice.value : null,
      settings: settings.status === "fulfilled" ? settings.value : null,
      microphones: microphones.status === "fulfilled" ? microphones.value : null,
    };
  },
  voiceStatus: () => invoke<VoiceStatus>("voice_status"),
  voiceDownload: (id: string) => invoke<void>("voice_download", { id }),
  voiceSelect: (id: string) => invoke<void>("voice_select", { id }),
  voiceLanguageSet: (language: string | null) => invoke<void>("voice_language_set", { language }),
  voiceOff: () => invoke<void>("voice_off"),
  /** `tap`: a click started it, so `onVoiceSilence` says when the user stopped talking. */
  voiceStart: (tap: boolean, generation?: number) => invoke<void>("voice_start", { tap, generation }),
  voiceStop: () => invoke<string>("voice_stop"),
  voiceCancel: () => invoke<void>("voice_cancel"),
  /** The last usage read, for an island that loads after it. */
  usage: () => invoke<UsageWindow[]>("usage"),
  /** Connected monitors, by maker and model. */
  monitors: () => invoke<{ name: string; label: string }[]>("monitors"),
  /** A screen was plugged in or removed. */
  onMonitors(cb: () => void): void {
    void listen("monitors", () => cb());
  },
  /** `null` lets the desktop choose. */
  setMonitor: (name: string | null) => invoke<void>("set_monitor", { name }),
  setFoldAfter: (seconds: number) => invoke<void>("set_fold_after", { seconds }),
  setOpenOnHover: (on: boolean) => invoke<void>("set_open_on_hover", { on }),
  setTheme: (theme: Theme) => invoke<void>("set_theme", { theme }),
  /** `en`, `pt-BR`, `es`, `zh`, or null to follow the system. */
  setLanguage: (language: string | null) => invoke<void>("set_language", { language }),
  /** Settings → Activity: the week of `monday` (this week's when null), the weeks, the grid. */
  activity: (monday: string | null) => invoke<Activity>("activity", { monday }),
  setHistory: (on: boolean) => invoke<void>("set_history", { on }),
  clearHistory: () => invoke<void>("clear_history"),
  /** The user's GitHub contribution calendar, only with the GitHub connector on (kept an hour). */
  githubCalendar: () => invoke<GithubGrid>("github_calendar"),
  /** Writes the week's PNG where the user picks (the desktop's dialog); where it went, or null if cancelled. */
  saveRecapImage: (name: string, png: Uint8Array) => invoke<string | null>("save_recap_image", { name, png: Array.from(png) }),
  setAutostart: (on: boolean) => invoke<void>("set_autostart", { on }),
  setSounds: (on: boolean) => invoke<void>("set_sounds", { on }),
  /** Percent, 0 to 100. */
  setVolume: (percent: number) => invoke<void>("set_volume", { percent }),
  /** `open_nest` asked the open Nest for a tab. */
  onNestTab: (cb: (tab: string) => void) => listen<string>("nest-tab", (e) => cb(e.payload)),
  nestTabReady: () => invoke<string | null>("nest_tab_ready"),
  /** A setting changed somewhere; only the fields that changed are present. */
  onSettings(cb: (s: SettingsChange) => void): void {
    void listen<SettingsChange>("settings", (e) => cb(e.payload));
  },
  /** Whether the API chat can be used now; keys themselves never come back. */
  apiKeyStatus: () => invoke<ApiStatus>("api_key_status"),
  chatEngines: () => invoke<ChatEngines>("chat_engines"),
  apiKeySet: (provider: string, key: string) => invoke<void>("api_key_set", { provider, key }),
  apiKeyClear: (provider: string) => invoke<void>("api_key_clear", { provider }),
  apiProviders: () => invoke<{ selected: string; providers: ApiProvider[] }>("api_providers"),
  apiProviderSet: (provider: string) => invoke<void>("api_provider_set", { provider }),
  apiModels: (provider: string) => invoke<string[]>("api_models", { provider }),
  apiModelSet: (provider: string, model: string) => invoke<void>("api_model_set", { provider, model }),
  /** The chat's paperclip: the system's file chooser; picked files arrive as `files`, like a drop. */
  chatPickFiles: (title: string) => void invoke<void>("chat_pick_files", { title }),
  chatSend: (text: string, files: string[], folder: string | null, mentions: ChatMention[] = [], generation?: number) => invoke<void>("chat_send", { text, files, folder, mentions, generation }),
  chatDecide: (id: string, answer: ChatAnswer) => invoke<void>("chat_decide", { id, answer }),
  chatStop: () => invoke<void>("chat_stop"),
  chatReset: (provider: ChatProvider | null, generation?: number) => invoke<{ provider: ChatProvider; generation: number }>("chat_reset", { provider, generation }),
  chatCurrent: () => invoke<OpenedConversation | null>("chat_current"),
  chatDraft: () => invoke<ChatDraft>("chat_draft"),
  chatPrepareZeca: (generation?: number) => invoke<void>("chat_prepare_zeca", { generation }),
  chatDraftSet: (draft: ChatDraft, generation?: number) => invoke<void>("chat_draft_set", { draft, generation }),
  connectionsPlace: () => invoke<ConnectionsPlace | null>("nest_connections_place"),
  connectionsPlaceSet: (place: ConnectionsPlace) => invoke<void>("nest_connections_place_set", { place }),
  nestTabSet: (tab: string) => invoke<void>("nest_tab_set", { tab }),
  recordQuery: (tab: "history" | "audit", query?: string) => invoke<string>("nest_record_query", { tab, query: query ?? null }),
  projectQuery: (query?: string) => invoke<string>("nest_record_query", { tab: "projects", query: query ?? null }),
  flockPlace: () => invoke<FlockPlace | null>("nest_flock_place", { place: null }),
  flockPlaceSet: (place: FlockPlace) => invoke<void>("nest_flock_place", { place }),
  closeNest: () => invoke<void>("nest_close"),
  onNestClose: (handler: (event: { preventDefault(): void }) => Promise<void>) => getCurrentWindow().onCloseRequested(handler),
  /** The Nest, at `tab` when given (a tab or Activity sub-tab id of `surfaces/nest/render.ts`). */
  openNest: (tab?: string, full?: boolean) => invoke<void>("open_nest_window", { tab: tab ?? null, full: full ?? null }),
  /** The size switch (ADR 0021); `tab` keeps the place across it. */
  setSize: (size: Size, tab?: string) => invoke<void>("set_size", { size, tab: tab ?? null }),
  nestIsFull: () => invoke<boolean>("nest_is_full"),
  /** Resizes the Nest's window from that edge, the press held: it has no system frame to pull. */
  startResize: (edge: ResizeEdge) => {
    // Outside Tauri (the lab) there is no window to ask.
    try {
      void getCurrentWindow().startResizeDragging(edge).catch(() => {});
    } catch {
      /* no window */
    }
  },
  /** Whether the Nest is open and in sight: the island steps aside meanwhile (ADR 0021). */
  nestIsOpen: () => invoke<boolean>("nest_is_open"),
  onNestOpen(cb: (open: boolean) => void): void {
    void listen<boolean>("nest-open", (e) => cb(e.payload));
  },
  /** Whether the user is in the Nest (its window has the focus and shows the Nest, not Settings):
   *  there the Nest answers the cards, and the island stays aside (ADR 0021). */
  nestHoldsCards: () => invoke<boolean>("nest_holds_cards"),
  onNestHolds(cb: (holds: boolean) => void): void {
    void listen<boolean>("nest-holds", (e) => cb(e.payload));
  },
  /** The Nest went Full or back to Room. */
  onNestFull(cb: (full: boolean) => void): void {
    void listen<boolean>("nest-full", (e) => cb(e.payload));
  },
  onChatSync(cb: (s: ChatSync) => void): void {
    void listen<ChatSync>("chat-sync", (e) => cb(e.payload));
  },
  onSpecialistChat(cb: (event: SpecialistChatEvent) => void): void {
    void listen<SpecialistChatEvent>("specialist-chat", e => cb(e.payload));
  },
  onSpecialistChatSync(cb: (event: SpecialistChatSync) => void): void {
    void listen<SpecialistChatSync>("specialist-chat-sync", e => cb(e.payload));
  },
  chatConversations: () => invoke<ConversationView[]>("chat_conversations"),
  specialistChatConversations: (profile: string) => invoke<SpecialistConversationView[]>("specialist_chat_conversations", { profile }),
  specialistChatCurrent: (profile: string) => invoke<SpecialistCurrent>("specialist_chat_current", { profile }),
  specialistChatSend: (profile: string, text: string, generation: number, folder?: string | null, files: string[] = []) => invoke<void>("specialist_chat_send", { profile, text, generation, folder: folder ?? null, files }),
  specialistChatOpen: (profile: string, id: number, generation: number) => invoke<OpenedConversation>("specialist_chat_open", { profile, id, generation }),
  specialistChatPickFiles: (profile: string, generation: number, title: string) => invoke<SpecialistFiles>("specialist_chat_pick_files", { profile, generation, title }),
  specialistChatChoiceSet: (profile: string, choice: AgentDefaults | null, generation: number) => invoke<SpecialistCurrent>("specialist_chat_choice_set", { profile, choice, generation }),
  specialistChatReset: (profile: string, generation: number) => invoke<SpecialistCurrent & { provider: ChatProvider }>("specialist_chat_reset", { profile, generation }),
  specialistChatDraftSet: (profile: string, draft: ChatDraft, generation: number) => invoke<void>("specialist_chat_draft_set", { profile, draft, generation }),
  specialistChatRename: (profile: string, id: number, title: string) => invoke<void>("specialist_chat_rename", { profile, id, title }),
  specialistChatDelete: (profile: string, id: number) => invoke<{ current: boolean; reset: (SpecialistCurrent & { provider: ChatProvider }) | null }>("specialist_chat_delete", { profile, id }),
  specialistChatStop: (profile: string) => invoke<void>("specialist_chat_stop", { profile }),
  specialistChatDecide: (profile: string, id: string, answer: ChatAnswer) => invoke<void>("specialist_chat_decide", { profile, id, answer }),
  roosts: () => invoke<RoostView[]>("roosts"),
  agentsDoctor: (fresh: boolean) => invoke<AgentRow[]>("agents_doctor", { fresh }),
  roostSharePreview: (key: string) => invoke<SharePreview>("roost_share_preview", { key }),
  roostShareApply: (key: string, file: string, fingerprint: string) =>
    invoke<void>("roost_share_apply", { key, file, fingerprint }),
  continueCommand: (text: string, folder: string | null, generation?: number) =>
    invoke<{ said: string; detail: string | null } | null>("continue_command", { text, folder, generation }),
  nestHistory: () => invoke<HistoryView>("nest_history"),
  nestAudit: () => invoke<AuditRow[]>("nest_audit"),
  nestTraces: () => invoke<TraceRow[]>("nest_traces"),
  nestUsage: () => invoke<UsageView>("nest_usage"),
  spendToday: () => invoke<SpendToday>("spend_today"),
  budgetSet: (cents: number | null) => invoke<void>("budget_set", { cents }),
  onSpend(cb: () => void): void {
    void listen("spend-changed", () => cb());
  },
  memoryCommand: (text: string, folder: string | null, generation?: number) =>
    invoke<{ said: string; detail: string | null } | null>("memory_command", { text, folder, generation }),
  memoryView: () => invoke<MemoryView>("memory_view"),
  memoryDecide: (id: number, accept: boolean) => invoke<void>("memory_decide", { id, accept }),
  memoryUndo: (id: string) => invoke<void>("memory_undo", { id }),
  onMemory(cb: () => void): void {
    void listen("memory", () => cb());
  },
  chatOpen: (id: number, generation?: number) => invoke<OpenedConversation>("chat_open", { id, generation }),
  chatRename: (id: number, title: string) => invoke<void>("chat_rename", { id, title }),
  chatDelete: (id: number) => invoke<boolean>("chat_delete", { id }),
  chatModels: (provider: ChatProvider) => invoke<ModelChoice[]>("chat_models", { provider }),
  chatPick: (folder: string | null) => invoke<Record<string, ChatPick>>("chat_pick", { folder }),
  setChatPick: (folder: string | null, provider: ChatProvider, pick: ChatPick) =>
    invoke<void>("set_chat_pick", { folder, provider, model: pick.model ?? null, effort: pick.effort ?? null }),
  /** `exclusive`: the island was opened from the keyboard, with no click to give it the focus. */
  islandKeyboard: (on: boolean, exclusive = false) => invoke<void>("surface_keyboard", { on, exclusive }),
  firstName: () => invoke<string | null>("first_name"),
  onChat(cb: (d: ChatDelta) => void): void {
    void listen<ChatDelta>("chat", (e) => cb(e.payload));
  },
  /** Files dropped on the island. */
  onFiles(cb: (dropped: Dropped) => void): void {
    void listen<Dropped>("files", (e) => cb(e.payload));
  },
  /** Something is dragged over the island (true), or it left (false). */
  onDrag(cb: (over: boolean) => void): void {
    void listen<boolean>("drag", (e) => cb(e.payload));
  },
  /** A global shortcut was pressed: "allow" or "deny". */
  /** The pointer came onto or left the island's window (GTK's word, in order; see `pointer`). */
  /** The mic's loudness (0..1) while recording, every 50 ms. */
  dictationOverlayStarted: () => invoke<void>("dictation_overlay_started"),
  onDictationOverlayReady(cb: (ready: boolean) => void): void {
    let changed = false;
    void listen<boolean>("dictation-overlay-ready", event => { changed = true; cb(event.payload); })
      .then(() => invoke<boolean>("dictation_overlay_ready"))
      .then(ready => { if (!changed) cb(ready); }, () => { if (!changed) cb(false); });
  },
  dictationStatus: () => invoke<DictationStatus | null>("dictation_status"),
  onDictation(cb: (status: DictationStatus) => void): void {
    void listen<DictationStatus>("dictation", (event) => cb(event.payload));
  },
  onVoiceLevel(cb: (level: number) => void): void {
    void listen<number>("voice-level", (e) => cb(e.payload));
  },
  /** A tap-to-talk recording heard speech and then a pause: time to stop it. */
  onVoiceSilence(cb: () => void): void {
    void listen<null>("voice-silence", () => cb());
  },
  /** While listening on a GPU: the text heard so far, about every 0.8 s. Not the final text. */
  onVoicePartial(cb: (text: string) => void): void {
    void listen<string>("voice-partial", (e) => cb(e.payload));
  },
  onVoiceDownload(cb: (p: { id: string; done: number; total: number }) => void): void {
    void listen<{ id: string; done: number; total: number }>("voice-download", (e) => cb(e.payload));
  },
  /** What is playing, while the setting is on; null when nothing is. */
  onMedia(cb: (now: NowPlaying | null) => void): void {
    void listen<NowPlaying | null>("media", (e) => cb(e.payload));
  },
  /** Subscribe before reading the cache; a late startup read cannot replace a live update. */
  onUsage(cb: (windows: UsageWindow[]) => void): void {
    let updated = false;
    void listen<UsageWindow[]>("usage", (e) => { updated = true; cb(e.payload); }).then(() =>
      invoke<UsageWindow[]>("usage").then((windows) => { if (!updated) cb(windows); }, () => {}),
    ).catch(() => {});
  },
  onPointer(cb: (inside: boolean) => void): void {
    void listen<boolean>("pointer", (e) => cb(e.payload));
  },
  /** The user went to another of the app's windows (Settings): by the panel, the island folds. */
  onAway(cb: () => void): void {
    void listen("away", () => cb());
  },
  /** The corner widget was clicked: the island comes up (on the card, when one waits). */
  openIsland: () => emitTo("island", "open-island"),
  onOpenIsland(cb: () => void): void {
    void listen("open-island", () => cb());
  },
  /** The corner widget in a corner, or none. It comes, moves or goes at once. */
  setWidget: (corner: Corner | null) => invoke<void>("set_widget", { corner }),
  /** Settings was clicked: the island hears `onAway` ("island" is `ISLAND` in app/src/lib.rs). */
  away: () => emitTo("island", "away"),
  onShortcut(cb: (id: string) => void): void {
    void listen<string>("shortcut", (e) => cb(e.payload));
  },
  /** The keys bound so far (the binding may have happened before this window loaded). */
  shortcutKeys: () => invoke<Record<string, string>>("shortcut_keys"),
  openShortcutSettings: () => invoke<void>("open_shortcut_settings"),
  /** The keys the desktop bound, by shortcut id, to show on the buttons. */
  onShortcutKeys(cb: (keys: Record<string, string>) => void): void {
    void listen<Record<string, string>>("shortcut-keys", (e) => cb(e.payload));
  },
  /** Open terminal found nothing to bring forward. */
  onJumpFailed(cb: () => void): void {
    void listen("jump-failed", () => cb());
  },
  onOpenChat(cb: () => void): void {
    void listen("open-chat", () => cb());
  },
  onOpenNow(cb: () => void): void {
    void listen("open-now", cb);
  },
};

export interface DictationStatus {
  command?: boolean;
  result?: string | null;
  expires_at?: number | null;
  words?: number | null;
  text?: string | null;
  caption_until?: number | null;
  bands?: number[];
  partial?: string | null;
  app?: string | null;
  terminal?: boolean;
  session?: { agent: string; session_id: string } | null;
  session_name?: string | null;
  sequence: number;
  generation: number;
  phase: "session" | "held" | "pasted" | "listening" | "locked" | "finishing" | "working" | "retrying" | "copied" | "empty" | "cancelled" | "failed" | "commanded" | "answered" | "asked" | "refused" | "unknown";
  error: string | null;
}

export type DictationPermission = "off" | "requesting" | "ready" | "temporary" | "unavailable";

export interface DictationRow {
  id: number; at: number; app: string | null; session: string | null;
  heard: string; text: string; style: string; language: string | null; outputLanguage: DictationOutputLanguage | null;
  secs: number; words: number; fixes: number | null; outcome: string;
  timings: DictationTimings;
}
export interface DictationTimings {
  total_ms: number | null; stt_ms: number | null; pipeline_ms: number | null;
  cleanup_ms: number | null; insert_ms: number | null;
}
export interface DictationTimingDistribution {
  count: number; p50_ms: number | null; p95_ms: number | null;
}
export interface DictationHistory {
  retention: string;
  rows: DictationRow[];
  today: { words: number; secs: number; count: number } | null;
  timing_summary: Record<"total" | "stt" | "pipeline" | "cleanup" | "insert", DictationTimingDistribution>;
}

export type DictationPeriod = "week" | "month" | "all";
export interface DictationStatistics {
  period: DictationPeriod; from: string | null; through: string;
  usage: { words: number; secs: number; count: number; saved_minutes: number | null; wpm: number | null };
  days: { day: string; words: number; count: number }[];
  chart_limited: boolean;
  apps: { app: string | null; words: number }[];
  apps_complete: boolean;
}

export interface DictationDiagnostics {
  system: { host: string | null; device: string | null; rate: number | null; session: string;
    shortcuts: string; keyboard: string; kwin: string; gpu: boolean; vad: boolean };
  counters: { loops: number; retries: number; copied: number };
  timings: DictationHistory["timing_summary"] | null;
}
export interface DictationMicTest {
  id: number; phase: "running" | "done" | "stopped" | "error"; elapsed_ms: number;
  bands: number[]; noise_db: number | null;
  verdict: "ok" | "no-signal" | "noisy" | "clipping" | null;
}

export interface MeetingStatus {
  id: number;
  phase: "recovering" | "idle" | "preparing" | "ready" | "starting" | "recording" | "stopping" | "transcribing" | "cancelling" | "finished" | "cancelled" | "failed" | "unsupported";
  elapsedMs: number; maxSeconds: number;
  microphone: string | null; monitor: string | null; fallback: boolean;
  error: "recovery" | "sources" | "capture" | "model" | "transcription" | null;
  modelId: string | null; language: string | null; hasTranscript: boolean;
  identificationAvailable?: boolean;
  progress?: { track: "microphone" | "system"; completedMs: number; totalMs: number; stage?: "transcription" | "speakers" } | null;
}

export interface MeetingTrack {
  durationMs: number;
  attribution?: "analyzed" | "unavailable" | null;
  segments: { startMs: number; endMs: number; text: string; speaker?: number | null }[];
}

export interface MeetingTranscript {
  id: number;
  microphone: MeetingTrack;
  system: MeetingTrack;
}

export interface MeetingNotes {
  title: string;
  summary: string;
  key_points: string[];
  todos: { text: string; owner: string | null; due: string | null }[];
}
export type MeetingSource = "microphone" | "system";
export interface MeetingEntry { id: string; recorded_ms: number; title: string }
export interface SavedMeeting extends MeetingEntry {
  microphone: { duration_ms: number; segments: { start_ms: number; end_ms: number; text: string; speaker: number | null }[] };
  system: SavedMeeting["microphone"];
  speakers: { source: MeetingSource; id: number; name: string | null }[];
  notes: MeetingNotes | null;
}
export interface MeetingSummaryStatus {
  id: number;
  phase: "idle" | "generating" | "finished" | "cancelled" | "failed";
  error: "cancelled" | "configuration" | "quota" | "timeout" | "provider" | "invalid" | null;
  notes: MeetingNotes | null;
}
