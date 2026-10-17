// The chat panel: Zeca on the left (his perch comes in from the island), the conversation on the
// right. Its DOM is built once and updated in place, so the input keeps its focus and text while
// session views re-render the island around it, and a streaming reply only touches its own
// message. No Tauri here: the backend comes in.
import { applyActivity, emptyActivity, readActivity } from "./chat-activity";
import type { ChatActivity, ChatActivityEvent } from "../bridge";
import { Clock } from "../clock";
import type { ChatDraft, ChatMention, UsageWindow, SessionView, ChatView } from "../bridge";
import { NAME_BY_KIND } from "../agents";
import { drawFrame, frameWidth } from "../character/sprites";
import { ZECA } from "../character/zeca";
import type { AgentKind, ApiStatus, ChatAccess, ChatEngines, ChatAnswer, ChatDelta, ChatPick, ChatProvider, ChatSync, ConversationView, ModelChoice, OpenedConversation } from "../bridge";
import { el } from "../dom";
import { Sound } from "../sound";
import { icon } from "./icons";
import { renderFull } from "./markdown";
import { copyTextButton } from "../clipboard";
import { locale, t, tk } from "../i18n";
import { usageLabel, usageLevel, usageWindowName } from "../cost";

export interface ChatBackend {
  draft?: () => Promise<ChatDraft>;
  current?: () => Promise<OpenedConversation | null>;
  saveDraft?: (draft: ChatDraft) => Promise<void>;
  send(text: string, files: string[], folder: string | null, mentions?: ChatMention[]): Promise<void>;
  decide(id: string, answer: ChatAnswer): void;
  /** Stops the turn running now. */
  stop(): void;
  reset(provider: ChatProvider | null): Promise<ChatProvider>;
  /** The models the provider's CLI offers. */
  models(provider: ChatProvider): Promise<ModelChoice[]>;
  /** The model and effort a new conversation in `folder` (null: the chat's own) starts with, by engine. */
  pick(folder: string | null): Promise<Record<string, ChatPick>>;
  setPick(folder: string | null, provider: ChatProvider, pick: ChatPick): Promise<void>;
  /** The saved conversations, the latest used first. */
  conversations(): Promise<ConversationView[]>;
  /** Goes on with a saved conversation. */
  open(id: number): Promise<OpenedConversation>;
  rename(id: number, title: string): Promise<void>;
  /** Deletes one; true when the chat was in it (and now starts a new one). */
  remove(id: number): Promise<boolean>;
  keyboard(on: boolean, exclusive?: boolean): void;
  /** Speech to text; absent while no engine is set up, and then there is no mic button. */
  voice?: VoiceBackend;
  /** Opens the conversation in the Nest; absent in the Nest itself. */
  openNest?: () => void;
  /** Opens the existing Flock without answering or starting anything. */
  openFlock?: () => void;
  /** The paperclip: the system's file chooser, titled in the user's language; what it picks comes
   *  back as a drop does. */
  pickFiles?: (title: string) => void;
  /** `/remember` and `/forget` (M6): what the memory says back, or null for any other text. */
  memory?: (text: string, folder: string | null) => Promise<MemorySaid | null>;
  /** `/continue` (W7): words for a stopped session in the folder, or null for any other text. */
  handoff?: (text: string, folder: string | null) => Promise<MemorySaid | null>;
  /** Who answers the chat's cards (ADR 0004); the app takes only less access from here. */
  setAccess?: (access: ChatAccess) => Promise<void>;
  /** Settings' Chat page, where more access is given. */
  openSettings?: () => void;
  /** Whether each CLI engine is installed and signed in, for the engine menu. */
  engines?: () => Promise<ChatEngines>;
}

/** Every sentence a `/remember` or `/forget` may say back (app/src/memory.rs). */
const MEMORY_SAID = new Set([
  tk("Turn on Zeca's memory in Settings, Chat & voice, first."),
  tk("Say what to remember: /remember followed by the fact."),
  tk("This conversation is not in a repository."),
  tk("Remembered."),
  tk("Remembered for the project:"),
  tk("Not remembered:"),
  tk("Say what to forget: /forget followed by words of the fact."),
  tk("Nothing is remembered yet."),
  tk("Not forgotten:"),
  tk("Nothing remembered holds that."),
  tk("Forgotten."),
  tk("More than one fact holds that; say more:"),
  // `/continue` (app/src/handoff.rs, continuation.rs).
  tk("No session in this folder stopped in the last ten minutes."),
  tk("That session is still open in its terminal: tell it there."),
  tk("Vults can't tell whether that session is still open: tell it in its terminal."),
  tk("That session can't be continued from here."),
  tk("Another session is being continued: wait for it, or /continue stop."),
  tk("Today's budget is spent: Vults starts no new session until tomorrow."),
  tk("Continuing this session; it shows in the flock:"),
  tk("Stopped."),
  tk("No session is being continued."),
  tk("Say what to tell it: /continue followed by your words."),
  tk("This conversation has no folder: pick one first."),
  tk("That session couldn't be resumed:"),
  tk("It is still starting: try again in a moment."),
]);

const REFERENCE_ERRORS = new Set([
  tk("Chat references are invalid or too large."),
  tk("A referenced session or project is no longer in the flock. Remove its reference and choose again."),
  tk("Choose flock references from the @ suggestions again before sending."),
  tk("Could not prepare the chat's flock references. Try again."),
]);

/** What a `/remember` or `/forget` says back: an English sentence to translate, and the memory's
 * own detail (why it refused, the facts to choose from). */
export interface MemorySaid {
  said: string;
  detail: string | null;
}

/** Where a chat panel lives: the island or the Nest. Both answer its cards (ADR 0021). */
export interface ChatPanelOptions {
  /** The window's label, to tell its own actions from the other window's. */
  window: "island" | "nest";
  /** How a reply is drawn: the island's light Markdown, or the Nest's full one. */
  render?: (text: string) => DocumentFragment;
}

export interface VoiceBackend {
  /** Starts recording; levels come back through `ChatPanel.voiceLevel`. `tap`: started by a click,
   *  so `ChatPanel.voiceSilence` comes when the user stops talking (if the app can tell). */
  start(tap: boolean): Promise<void>;
  /** Stops recording and resolves with the transcript. */
  stop(): Promise<string>;
  /** Drops the recording. */
  cancel(): void;
}

type Voice = "off" | "listening" | "transcribing";
/** Bars in the waveform: the last levels, newest on the right. */
const WAVE_BARS = 32;
/** Bars beside the words heard so far. */
const WAVE_BARS_HEARD = 10;

/** A folder the chat could work in: the project of a session on the wire. */
export interface Folder {
  cwd: string;
  project: string;
}

/** A file the island would not take, and why. */
export interface Refused {
  name: string;
  reason: "folder" | "too-big" | "unreadable";
}

type Ask = {
  who: "ask";
  id: string;
  tool: string;
  target: string;
  description: string | null;
  full: string | null;
  added: number;
  removed: number;
  always: boolean;
  /** The conversation read content from outside (through this tool) before this card. */
  outside: string | null;
  answer?: ChatAnswer;
  cancelled?: boolean;
};

type ActivityMessage = { who: "activity"; data: ChatActivity; ended: boolean };

type Message =
  | ActivityMessage
  | { who: "you"; text: string; files: string[]; mentions?: ChatMention[] }
  | { who: "zeca"; text: string; error?: boolean; retry?: boolean }
  | { who: "note"; text: string }
  | Ask;

/** Every effort the CLIs share, as the picker names them. */
/** How a CLI engine stands, as its menu row says it. */
const ENGINE_STATE = { "signed-in": tk("Signed in"), installed: tk("Installed"), missing: tk("Not installed") } as const;

const EFFORTS: Record<string, string> = {
  low: tk("Low"),
  medium: tk("Medium"),
  high: tk("High"),
  xhigh: tk("Extra high"),
  max: tk("Max"),
};

/** Who answers the chat's cards, from the least access to the most (ADR 0004). */
const ACCESS: { id: ChatAccess; label: string; about: string }[] = [
  { id: "ask", label: tk("Asks first"), about: tk("A card before every command or edit.") },
  { id: "policies", label: tk("Your policies"), about: tk("Your accepted policies answer what they cover; the rest asks.") },
  { id: "full", label: tk("Full access"), about: tk("Everything runs without a card, but destructive commands and what a policy denies or asks first. Once the conversation reads from outside, cards ask again.") },
];

/** How long Zeca takes to swallow a dropped file. */
const SWALLOW_MS = 1500;
/** How long a vulture takes to carry a dropped file across the drop zone. */
const CARRY_MS = 1300;
/** Ways to start, offered while the conversation is empty. */
const SUGGESTIONS = [
  tk("What is this project?"),
  tk("What changed recently?"),
  tk("Explain the last error"),
];
/** The input grows up to this many lines, then scrolls. */
const MAX_INPUT_LINES = 6;
/** A streamed reply repaints at most this often. */
const STREAM_MS = 80;
/** A turn shows how long it has run once it takes longer than this. */
const SHOW_ELAPSED_MS = 3000;
/** Closer than this to the end of the log counts as reading the end: new text keeps it in view. */
const FOLLOW_PX = 28;
const SCROLL_COOLDOWN_MS = 150;
const REFUSED: Record<Refused["reason"], (name: string) => string> = {
  folder: (n) => t("{name} is a folder. Drop the files inside it instead.", { name: n }),
  "too-big": (n) => t("{name} is over 20 MB, too big to read whole.", { name: n }),
  unreadable: (n) => t("{name} couldn't be read.", { name: n }),
};

/** What a permission asks for, in a few words. */
function asks(tool: string): string {
  if (tool === "Bash") return t("Zeca wants to run");
  if (tool === "Edit" || tool === "Write" || tool === "MultiEdit")
    return t("Zeca wants to change");
  return t("Zeca wants to use {tool}", { tool });
}

const folderName = (cwd: string) =>
  cwd.split(/[\\/]/).filter(Boolean).pop() ?? cwd;

let chatNumber = 0;

export class ChatPanel {
  private readonly menuId = `chat-menu-${++chatNumber}`;
  private draftEdited = false;
  private draftSaved: Promise<void> = Promise.resolve();
  private restored = false;
  private restoring: Promise<void> | null = null;
  private conversationVersion = 0;
  private sourceVersion = 0;
  private liveEnded = false;

  /** Failed reads stay retryable; never write over a draft the surface could not read. */
  restore(): Promise<void> {
    if (this.restored || !this.backend.draft) return Promise.resolve();
    if (!this.restoring) {
      const version = this.conversationVersion;
      const source = this.sourceVersion;
      this.restoring = Promise.all([this.backend.current?.(), this.backend.draft()]).then(([current, draft]) => {
        // Live events or an opened/reset conversation outrank the startup snapshot.
        if (current && source === this.sourceVersion) this.load(current, false, version !== this.conversationVersion);
        this.restoreDraft(draft, true);
        this.restored = true;
      }).catch((error: unknown) => {
        this.restoring = null;
        throw error;
      });
    }
    return this.restoring;
  }

  /** Save before a native window transition; serial writes keep fast typing in order. */
  flushDraft(): Promise<void> {
    if (!this.restored && this.backend.draft) return this.restore().then(() => this.flushDraft());
    this.draftEdited = true;
    this.mentions = this.mentions.filter(m => this.input.value.includes(m.token));
    const draft = { text: this.input.value, files: [...this.files], folder: this.folder, provider: this.provider, ...(this.mentions.length ? { mentions: [...this.mentions] } : {}) };
    this.draftSaved = this.draftSaved.catch(() => {}).then(() => this.backend.saveDraft?.(draft));
    return this.draftSaved;
  }

  private keepDraft(): void {
    this.draftEdited = true;
    void this.flushDraft().catch(() => {});
  }

  restoreDraft(draft: ChatDraft, initial = false): void {
    // A late startup read must not replace typing or a newer sync event.
    if (initial && this.draftEdited) return;
    this.draftEdited = true;
    this.input.value = draft.text;
    this.mentions = (draft.mentions ?? []).filter(m => draft.text.includes(m.token));
    this.closeMentions();
    this.files = [...draft.files];
    if (!this.started && (draft.text || draft.files.length || draft.folder)) {
      this.folder = draft.folder;
      this.picked = true;
    }
    if (!this.started && draft.provider) this.provider = draft.provider;
    this.grow();
    this.paint();
    this.changed();
  }
  readonly element = el("section", { class: "chat" });
  /** The island moves its perch here; the Nest keeps its own scene in the same slot. */
  readonly perchSlot = el("div", { class: "chat-perch" });
  private readonly greeting = el("h2", { class: "chat-greeting" });
  private readonly caption = el("p", { class: "chat-caption" });
  private readonly intro = el("div", { class: "chat-intro" }, this.greeting, this.caption);
  private readonly suggestions = el("div", { class: "suggestions" });
  private readonly replySuggestions = el("div", { class: "reply-chips" });
  private readonly apiNotice = el("p");
  private offers: ChatView | null = null;
  private open = false;
  private provider: ChatProvider = "claude";
  /** The API chat: offered only while it can be used (a key saved, or a local model). */
  private api: ApiStatus = { ready: false, label: t("API") };
  /** How each CLI engine stands, asked each time the menu opens; none until it answers. */
  private engines: ChatEngines | null = null;
  private enginesAsked = 0;
  private messages: Message[] = [];
  private selectedAsk: string | null = null;
  private approvalQueueKey = "";
  private askNodes = new WeakMap<Ask, HTMLElement>();
  private readonly activityNodes = new WeakMap<ActivityMessage, { node: HTMLDetailsElement; summary: HTMLElement; reasoning: HTMLElement; tools: HTMLElement; decisions: HTMLElement; warning: HTMLElement; toolNodes: Map<string, { node: HTMLDetailsElement; status: HTMLElement; detail: HTMLElement }> }>();
  private activityTimer: number | undefined;
  private replyBreak = false;
  private readonly approvalQueue = el("div", { class: "approval-queue" });
  /** The rendered messages, one node per message. */
  private nodes: HTMLElement[] = [];
  private readonly appeared = new WeakSet<Message>();
  private readonly attachmentNodes = new Map<string, HTMLElement>();
  private files: string[] = [];
  private busy = false;
  /** When the turn running now began, for the thinking line's seconds. */
  private turnSince = 0;
  /** The thinking line while a turn runs, and the timer that counts its seconds. */
  private thinking: HTMLElement | null = null;
  private ticking: number | undefined;
  /** Where the conversation works, fixed once it starts. */
  private folder: string | null = null;
  /** The user chose the folder; otherwise it follows the session in front. */
  private picked = false;
  private folders: Folder[] = [];
  private started = false;
  private swallowUntil = 0;
  private searching = false;
  private reaction: { clip: "listen" | "speak"; until: number } | null = null;
  private reactionTimer: number | undefined;
  /** The pending repaint of the reply being streamed. */
  private streaming: number | undefined;
  private dragOver = false;
  /** The drop zone shown from the + tab, until a file comes or it is dismissed. */
  /** Asking before a new conversation clears this one: what was asked for. */
  /** The saved conversations' list: open, what it shows, and the row being renamed or deleted. */
  private historyOpen = false;
  private saved: ConversationView[] = [];
  private renaming: number | null = null;
  private deleting: number | null = null;
  private menuOpen = false;
  private keys: Record<string, string> = {};
  private readonly place = el("button", { class: "place" });
  /** The CLI reads the user's own setup (Settings → Chat): shown for the conversation it began. */
  /** An icon, not words: the head is narrow (its title and label say what it means). */
  private readonly setup = el("span", { class: "setup" }, icon("person", 12, 2));
  /** The switch as it is now; the badge keeps what the conversation started with. */
  private nativeSetting = true;
  private native = true;
  private readonly menu = el("div", { class: "menu" });
  /** The model and effort: picked per folder before the first message, fixed after it. */
  private readonly modelButton = el("button", { class: "place model" });
  private readonly planUsage = el("span", { class: "model-usage" });
  private usage: UsageWindow[] = [];
  private usageTimer: number | undefined;
  private selectionLabel = "";
  private readonly modelMenu = el("div", { class: "menu model-menu" });
  private modelOpen = false;
  /** Who answers the cards, as Settings says; the picker shows it and gives some back. */
  private access: ChatAccess = "ask";
  private readonly accessButton = el("button", { class: "place access" });
  private readonly accessMenu = el("div", { class: "menu access-menu" });
  private accessOpen = false;
  /** The picks for `picksFor`, by engine. */
  private picks: Record<string, ChatPick> = {};
  private picksFor: string | null | undefined = undefined;
  /** Bumped on each read of the picks: an older answer never overwrites a newer one. */
  private picksSeq = 0;
  private models: Partial<Record<ChatProvider, ModelChoice[]>> = {};
  /** The provider's CLI did not list its models the last time it was asked. */
  private modelsFailed: Partial<Record<ChatProvider, boolean>> = {};
  private readonly historyButton = el("button", { class: "ghost history" });
  private readonly newButton = el("button", { class: "ghost", onclick: () => this.ask("new") });
  private readonly historyMenu = el("div", { class: "menu history-menu" });
  private readonly log = el("div", { class: "log" });
  private following = true;
  private lastScrollInput = -Infinity;
  private readonly jump = el("button", {
    class: "jump-end",
    text: t("↓ new text"),
    onclick: () => this.toEnd(),
  });
  private readonly chips = el("div", { class: "chips" });
  /** The Nest has the room to say how the composer's keys work. */
  private readonly composerHint = el("p", { class: "composer-hint" });
  private readonly attachLabel = el("span");
  private readonly input = document.createElement("textarea");
  private mentions: ChatMention[] = [];
  private sessions: SessionView[] = [];
  private readonly mentionMenu = el("div", { class: "menu mention-menu" });
  private mentionRows: { label: string; detail: string; mention: ChatMention }[] = [];
  private mentionIndex = 0;
  private mentionStart = 0;
  private readonly send = el("button", {
    class: "send",
    onclick: () => this.sendOrStop(),
  });
  private voice: Voice = "off";
  /** The mic opening, for a stop that comes before it is open. */
  private starting: Promise<void> | null = null;
  /** A voice model is chosen and downloaded: the mic shows. */
  private voiceReady = false;
  private levels: number[] = Array(WAVE_BARS).fill(0);
  /** What the user has said so far, decoded while they speak (only with a GPU); dimmed, and never
   *  the input's text: only the final transcription goes there. */
  private partial = "";
  private readonly mic = el("button", {
    class: "mic",
    onclick: () => void this.toggleVoice(),
  });
  private readonly wave = el("div", { class: "wave" });
  /** Files just dropped: carried across the drop zone, then on the next message. */
  private carried: { paths: string[] } | null = null;
  private readonly dropBody = el("div", { class: "drop-body" });
  private readonly drop = el("div", { class: "drop-zone" }, this.dropBody);

  constructor(
    private readonly backend: ChatBackend,
    private readonly changed: () => void,
    private readonly opts: ChatPanelOptions = { window: "island" },
  ) {
    this.input.rows = 1;
    this.mentionMenu.id = `${this.menuId}-mentions`;
    this.mentionMenu.setAttribute("role", "listbox");
    this.mentionMenu.setAttribute("aria-label", t("Sessions and projects"));
    this.input.setAttribute("aria-autocomplete", "list");
    this.input.setAttribute("aria-controls", this.mentionMenu.id);
    this.closeMentions();
    this.planUsage.setAttribute("aria-hidden", "true");
    document.addEventListener("visibilitychange", () => {
      if (document.hidden) this.clearReaction();
      this.paintUsage();
    });
    window.addEventListener?.("resize", () => this.sizeMentions());
    this.setup.setAttribute("role", "img");
    this.input.addEventListener("keydown", (e) => {
      if (e.isComposing || e.keyCode === 229) return;
      if (e.shiftKey && (e.key === "Enter" || e.key === "Tab")) { this.closeMentions(); return; }
      if (this.mentionMenu.classList.contains("on") && ["ArrowDown", "ArrowUp", "Enter", "Tab", "Escape"].includes(e.key)) {
        e.preventDefault(); e.stopPropagation();
        if (e.key === "Escape") this.closeMentions();
        else if (e.key === "Enter" || e.key === "Tab") this.pickMention();
        else {
          this.mentionIndex = (this.mentionIndex + (e.key === "ArrowDown" ? 1 : -1) + this.mentionRows.length) % this.mentionRows.length;
          this.paintMentions();
        }
        return;
      }
      // Escape is the island's: it closes the chat from anywhere.
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void this.submit();
      } else if (e.key === "ArrowUp" && !this.input.value && this.recall()) e.preventDefault();
    });
    this.input.addEventListener("input", (event) => {
      this.grow();
      this.paintSend();
      this.keepDraft();
      if (!this.busy) {
        const before = this.clip(Clock.now());
        if (this.input.value.trim()) this.react("listen");
        else this.clearReaction();
        if (before !== this.clip(Clock.now())) this.changed();
      }
      if (!(event as InputEvent | undefined)?.isComposing) this.findMentions();
    });
    this.input.addEventListener("compositionstart", () => this.closeMentions());
    this.input.addEventListener("compositionend", () => this.findMentions());
    this.input.addEventListener("click", () => this.findMentions());
    this.input.addEventListener("keyup", (e) => { if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) this.findMentions(); });
    this.place.addEventListener("click", () => {
      if (this.started || this.provider === "api") return;
      this.menuOpen = !this.menuOpen;
      this.modelOpen = this.accessOpen = this.historyOpen = false;
      this.paintHead();
      this.changed();
    });
    this.historyButton.append(icon("list", 14, 2));
    this.historyButton.addEventListener("click", () => {
      if (this.busy) return;
      this.historyOpen = !this.historyOpen;
      this.menuOpen = this.modelOpen = this.accessOpen = false;
      this.renaming = this.deleting = null;
      if (this.historyOpen) void this.loadSaved();
      this.paintHead();
      this.changed();
    });
    this.accessButton.addEventListener("click", () => {
      if (this.provider === "api") return;
      this.accessOpen = !this.accessOpen;
      this.menuOpen = this.modelOpen = this.historyOpen = false;
      this.paintHead();
      this.changed();
    });
    this.modelButton.addEventListener("click", () => {
      // It is the engine's pill too, in both windows: the engine can change whatever was said.
      this.modelOpen = !this.modelOpen;
      this.menuOpen = this.accessOpen = this.historyOpen = false;
      // Started (or the API chat), the island's menu shows only the engines: no list to ask for.
      if (this.modelOpen && !this.started) {
        if (this.modelsFailed[this.provider]) this.models[this.provider] = undefined;
        this.loadModels(this.provider);
      }
      if (this.modelOpen) {
        // Only the latest answer counts: an older one arriving late is stale.
        const asked = ++this.enginesAsked;
        void this.backend.engines?.().then(
          (e) => {
            if (asked !== this.enginesAsked) return;
            this.engines = e;
            // Each state adds a line on the island: measured again, so the menu stays clickable.
            if (this.modelOpen) {
              this.paintHead();
              this.changed();
            }
          },
          () => {},
        );
      }
      this.paintHead();
      this.changed();
    });
    for (const { id, menu, trigger } of this.menuPanels()) {
      menu.id = `${this.menuId}-${id}`;
      menu.setAttribute("role", "region");
      trigger.setAttribute("aria-controls", menu.id);
      trigger.addEventListener("keydown", event => {
        if (event.isComposing || event.keyCode === 229 || !["ArrowDown", "ArrowUp"].includes(event.key)) return;
        event.preventDefault();
        if (!menu.classList.contains("on")) trigger.click();
        queueMicrotask(() => {
          if (!menu.classList.contains("on")) return;
          const buttons = [...menu.querySelectorAll<HTMLButtonElement>("button")].filter(button => !button.disabled);
          (event.key === "ArrowUp" ? buttons.at(-1) : buttons[0])?.focus();
        });
      });
      menu.addEventListener("keydown", event => {
        if (event.isComposing || event.keyCode === 229 || (event.target as HTMLElement)?.matches("input, textarea")) return;
        if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
        const buttons = [...menu.querySelectorAll<HTMLButtonElement>("button")].filter(button => !button.disabled);
        if (!buttons.length) return;
        event.preventDefault();
        event.stopPropagation();
        const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
        const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1
          : (current + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
        buttons[next].focus();
      });
    }
    this.element.addEventListener("keydown", event => {
      if (event.key !== "Escape" || event.isComposing || event.keyCode === 229 || event.defaultPrevented || this.voice !== "off") return;
      if (this.closeMenus(true)) { event.preventDefault(); event.stopPropagation(); }
    });
    document.addEventListener("pointerdown", event => {
      if (!this.element.isConnected) return;
      const target = event.target as Node | null;
      if (this.mentionMenu.contains(target) || target === this.input) return;
      this.closeMentions();
      if (this.menuPanels().some(({ menu, trigger }) => menu.contains(target) || trigger.contains(target))) return;
      this.closeMenus();
    });
    const pauseFollow = () => {
      this.lastScrollInput = performance.now();
      this.following = false;
    };
    this.log.tabIndex = 0;
    this.log.setAttribute("role", "region");
    this.log.addEventListener("wheel", pauseFollow, { passive: true });
    this.log.addEventListener("touchmove", pauseFollow, { passive: true });
    this.log.addEventListener("pointerdown", pauseFollow);
    this.log.addEventListener("keydown", (e) => {
      if (["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "].includes(e.key)) pauseFollow();
    });
    this.log.addEventListener("scroll", () => {
      this.following = this.atEnd();
      this.log.classList.toggle("scrolled", this.log.scrollTop > 2);
      if (this.following) this.jump.classList.remove("on");
    });
    // Quick keeps conversation actions beside Zeca and the executor beside the draft.
    const compact = this.compact;
    const head = compact
      ? el(
          "div",
          { class: "chat-head" },
          this.intro,
          el("div", { class: "history-box" }, this.historyButton, this.historyMenu),
          this.newButton,
          ...(this.backend.openNest ? [this.nestButton()] : []),
        )
      : el(
      "div",
      { class: "chat-head" },
      // The prototype's engine pill here too (L14), in place of the Claude / Codex switch.
      el("div", { class: "model-box engine-box" }, this.modelButton, this.modelMenu),
      el("div", { class: "place-box" }, this.place, this.menu),
      el("div", { class: "access-box" }, this.accessButton, this.accessMenu),
      this.setup,
      el("div", { class: "history-box" }, this.historyButton, this.historyMenu),
      ...(this.backend.openNest ? [this.nestButton()] : []),
      this.newButton,
    );
    const main = el(
      "div",
      { class: "chat-main" },
      head,
      this.approvalQueue,
      el("div", { class: "log-box" }, this.log, this.jump),
      el("div", { class: "composer" },
        this.mentionMenu,
        this.chips,
        this.input,
        this.wave,
        el("div", { class: "composer-tools" }, this.attachButton(), this.mic, this.send),
      ),
      compact ? null : this.composerHint,
      compact ? el("div", { class: "chat-foot" }, el("div", { class: "model-box engine-box" }, this.modelButton, this.modelMenu), el("div", { class: "place-box" }, this.place, this.menu), el("div", { class: "access-box" }, this.accessButton, this.accessMenu), this.setup) : null,
    );
    this.drop.prepend(dashes());
    this.element.append(this.perchSlot, main, this.drop);
    this.relabel();
    void this.restore().catch(() => {});
  }

  /** Its words again: the language is known now, or changed. The panel is built before the app
   *  says which one, so the first call comes from there too. */
  relabel(): void {
    this.askNodes = new WeakMap();
    this.input.placeholder = t("Ask Zeca…");
    this.input.setAttribute("aria-label", t("Ask Zeca…"));
    this.input.title = t("Type @ to reference a session or project. References limit this turn's flock reads.");
    this.log.setAttribute("aria-label", t("Chat"));
    this.composerHint.textContent = t("Enter sends, Shift+Enter starts a new line, ↑ recalls your last message, and @ references the flock.");
    this.attachLabel.textContent = t("Attach a file");
    this.chips.setAttribute("role", "group");
    this.chips.setAttribute("aria-label", t("Attachments"));
    this.setup.setAttribute("aria-label", t("Your setup"));
    this.setup.title = t("Claude Code and Codex read your own setup here: your CLAUDE.md, skills, MCP servers, hooks and allow rules. A project's settings join only in a folder you already trust in Claude Code. Change it in Settings → Chat & voice.");
    this.historyButton.title = t("Conversations");
    if (this.opts.window === "island") {
      this.newButton.replaceChildren(icon("plus", 14, 2));
      this.newButton.title = t("New");
      this.newButton.setAttribute("aria-label", t("New"));
    } else this.newButton.textContent = t("New");
    this.jump.textContent = t("↓ new text");
    this.paint();
  }

  isOpen(): boolean {
    return this.open;
  }

  /** Which bird talks: the API chat is Claude too. */
  agent(): AgentKind {
    return this.provider === "api" ? "claude" : this.provider;
  }

  /** The app's cached plan windows; this never asks a provider or changes the conversation. */
  setUsage(windows: UsageWindow[]): void {
    this.usage = windows;
    this.paintUsage();
  }

  private paintUsage(): void {
    window.clearTimeout(this.usageTimer);
    this.usageTimer = undefined;
    const now = Date.now() / 1000;
    // API chat borrows Claude's bird, never its subscription.
    const live = this.usage.filter(w => this.provider !== "api" && w.agent === this.provider
      && Number.isFinite(w.used_percent) && w.used_percent >= 0 && w.used_percent <= 100
      && (w.resets_at === null || w.resets_at > now));
    const fullest = live.reduce<UsageWindow | null>((a, w) => !a || w.used_percent > a.used_percent ? w : a, null);
    this.planUsage.hidden = !fullest;
    const hint = this.compact ? "" : this.started
      ? t("A conversation keeps its model; New starts another.")
      : t("The model and effort for new conversations in this folder.");
    this.modelButton.setAttribute("aria-label", this.selectionLabel);
    this.modelButton.title = hint;
    if (!fullest) return;
    const percent = Math.round(fullest.used_percent);
    const summary = t("{engine} plan: {percent}% used · {status}", { engine: this.engineName(this.provider), percent, status: usageLabel(percent) });
    const ring = el("span", { class: "usage-ring" });
    ring.style.setProperty("--used", `${percent}%`);
    this.planUsage.className = `model-usage ${usageLevel(percent)}`;
    this.planUsage.replaceChildren(ring, el("span", { text: `${percent}%` }));
    this.modelButton.setAttribute("aria-label", t("{selection}. {usage}", { selection: this.selectionLabel, usage: summary }));
    const windows = live.map(w => {
      const vars = { window: usageWindowName(w.minutes), percent: w.used_percent };
      return w.resets_at === null
        ? t("{window}: {percent}% used", vars)
        : t("{window}: {percent}% used. Resets {when}.", { ...vars, when: new Date(w.resets_at * 1000).toLocaleString(locale(), { weekday: "short", hour: "2-digit", minute: "2-digit" }) });
    });
    this.modelButton.title = [hint, summary, ...windows].filter(Boolean).join("\n");
    const next = Math.min(...live.flatMap(w => w.resets_at === null ? [] : [w.resets_at]));
    if (Number.isFinite(next) && this.enabled && this.open && !document.hidden) {
      this.usageTimer = window.setTimeout(() => this.paintUsage(), Math.min(2_147_483_647, Math.max(1, (next - now) * 1000)));
    }
  }

  setVoiceReady(on: boolean): void {
    if (on === this.voiceReady) return;
    this.voiceReady = on;
    if (!on) this.cancelVoice();
    this.paintVoice();
    this.changed();
  }

  /** Settings → Chat's choice of who answers the cards: it holds at once, mid-conversation too. */
  setAccess(access: ChatAccess): void {
    if (access === this.access) return;
    this.access = access;
    this.paintHead();
    this.changed();
  }

  /** Settings → Chat's switch; a conversation that started keeps what its CLI read. */
  setNative(on: boolean): void {
    this.nativeSetting = on;
    if (this.started || this.native === on) return;
    this.native = on;
    this.paintHead();
    this.changed();
  }

  setApi(api: ApiStatus): void {
    // A conversation that began keeps its provider (the next one takes Settings' choice).
    const pinned = this.provider === "api" && this.started;
    const next = pinned ? { ...api, label: this.api.label } : api;
    if (next.ready === this.api.ready && next.label === this.api.label) return;
    this.api = next;
    this.paintHead();
    this.changed();
  }

  /** The island's chat: the prototype's compact form (ADR 0021). */
  private get compact(): boolean {
    return this.opts.window === "island";
  }

  /** The engine as the island's pill names it. */
  private engineName(p: ChatProvider): string {
    return p === "api" ? this.api.label : p === "claude" ? "Claude Code" : "Codex";
  }

  private name(p: ChatProvider): string {
    return p === "api" ? this.api.label : p === "claude" ? "Claude" : "Codex";
  }

  setKeys(keys: Record<string, string>): void {
    this.keys = keys;
    this.paint();
  }

  /** The folders of the sessions on the wire, and the one in front (the default). */
  setFolders(folders: Folder[], front: string | null): void {
    const sig = (fs: Folder[]) => fs.map((f) => f.cwd).join("|");
    const changed = sig(folders) !== sig(this.folders);
    this.folders = folders;
    if (!this.started && !this.picked && !this.input.value && !this.files.length && front !== this.folder) {
      this.folder = front;
      this.paintHead();
    } else if (changed) this.paintHead();
  }

  setSessions(sessions: SessionView[]): void {
    this.sessions = sessions;
    if (this.mentionMenu.classList.contains("on")) this.findMentions();
  }

  /** Core offers are presentation only: no model call, notification, sound or turn. */
  setOffers(offers: ChatView | null): void {
    if (JSON.stringify(offers) === JSON.stringify(this.offers)) return;
    this.offers = offers;
    this.paintOffers();
  }

  private sessionMention(s: SessionView): { label: string; mention: ChatMention } {
    const agent = s.agent_name ?? (s.agent === "other" ? t("Agent") : NAME_BY_KIND[s.agent]);
    const duplicates = this.sessions.filter(other => other.agent === s.agent && other.project === s.project).length > 1;
    const label = t("Session: {session}", { session: `${agent} · ${s.project}` });
    const tokenLabel = duplicates ? `${label} · ${s.id}` : label;
    return { label, mention: { token: `@${JSON.stringify(tokenLabel)}`, target: { kind: "session", agent: s.agent, id: s.id } } };
  }

  private chooseSuggestion(id: string): void {
    if (!this.enabled) return;
    const offered = this.offers?.suggestions.find(q => q.id === id);
    if (!offered) {
      if (!this.offers && SUGGESTIONS.includes(id)) this.draft(t(id));
      return;
    }
    let text = offered.text;
    if (offered.session) {
      const session = this.sessions.find(s => s.agent === offered.session!.agent && s.id === offered.session!.id);
      if (!session) return;
      this.mentions = this.mentions.filter(m => this.input.value.includes(m.token));
      let reference = this.mentions.find(m => m.target.kind === "session" && m.target.agent === session.agent && m.target.id === session.id);
      if (!reference) {
        if (this.mentions.length >= 8) return;
        reference = this.sessionMention(session).mention;
        this.mentions.push(reference);
      }
      if (!this.input.value.includes(reference.token)) text += `\n${reference.token}`;
    }
    this.draft(text);
  }

  private chooseFollowup(id: string): void {
    const last = this.messages[this.messages.length - 1];
    if (!this.enabled || this.busy || last?.who !== "zeca" || last.error || !last.text.trim() || !this.offers?.followups.includes(id as ChatView["followups"][number])) return;
    if (id === "flock") {
      this.backend.openFlock?.();
    } else this.draft(id === "explain" ? t("Explain your last reply more simply.") : t("What should I check next?"));
  }

  private paintOffers(): void {
    this.greeting.textContent = this.offers?.greeting ?? t("Ask Zeca");
    const api = t("Ask anything, or drop a file on the island. This chat only talks: it can't run commands or open your project.");
    this.caption.textContent = this.offers?.caption ?? t("Ask anything, or drop a file on the island.");
    this.caption.hidden = !this.offers && (this.compact || this.provider === "api");
    this.apiNotice.textContent = api;
    this.apiNotice.hidden = this.provider !== "api";
    this.intro.hidden = this.compact && (this.messages.length > 0 || this.busy);
    const questions = this.compact ? [] : this.offers?.suggestions ?? (this.provider === "api" ? [] : SUGGESTIONS.map(q => ({ id: q, text: t(q), session: null })));
    this.offerButtons(this.suggestions, questions.map(q => ({ id: q.id, text: q.text })), id => this.chooseSuggestion(id));
    const labels = { explain: t("Explain more simply"), next: t("What should I check next?"), flock: t("Open Flock") };
    const followups = (this.offers?.followups ?? []).filter(id => id !== "flock" || !!this.backend.openFlock);
    this.offerButtons(this.replySuggestions, followups.map(id => ({ id, text: labels[id] })), id => this.chooseFollowup(id));
    this.replySuggestions.setAttribute("role", "group");
    this.replySuggestions.setAttribute("aria-label", t("Continue the conversation"));
    this.suggestions.setAttribute("role", "group");
    this.suggestions.setAttribute("aria-label", t("Suggested questions"));
  }

  private offerButtons(container: HTMLElement, rows: { id: string; text: string }[], choose: (id: string) => void): void {
    const buttons = [...container.querySelectorAll<HTMLButtonElement>("button")];
    const active = buttons.find(b => b === document.activeElement)?.dataset.offer;
    container.replaceChildren(...rows.map(row => {
      const kept = buttons.find(b => b.dataset.offer === row.id);
      const button = kept ?? el("button", { class: container === this.suggestions ? "suggestion" : "reply-chip", onclick: () => choose(row.id) });
      button.dataset.offer = row.id;
      button.textContent = row.text;
      return button;
    }));
    if (active) {
      const kept = [...container.querySelectorAll<HTMLButtonElement>("button")].find(b => b.dataset.offer === active);
      (kept ?? this.input).focus({ preventScroll: true });
    }
  }

  private closeMentions(): void {
    this.mentionMenu.classList.remove("on");
    this.input.setAttribute("aria-expanded", "false");
    this.input.setAttribute("aria-activedescendant", "");
    this.mentionRows = [];
    this.mentionIndex = 0;
  }

  private findMentions(): void {
    if (!this.enabled) { this.closeMentions(); return; }
    const end = this.input.selectionStart ?? this.input.value.length;
    const before = this.input.value.slice(0, end);
    const typed = /(?:^|\s)@([^@"\n]*)$/u.exec(before)?.[1];
    if (typed === undefined || this.mentions.length >= 8 || this.input.selectionEnd !== null && this.input.selectionEnd !== undefined && this.input.selectionEnd !== end) {
      this.closeMentions(); return;
    }
    const query = typed.toLocaleLowerCase();
    if (this.menuOpen || this.modelOpen || this.accessOpen || this.historyOpen) this.closeMenus();
    const previous = this.mentionRows[this.mentionIndex]?.mention.token;
    const rows: typeof this.mentionRows = [];
    const projects = new Map(this.sessions.filter(s => s.cwd).map(s => [s.cwd!, s.project]));
    for (const [cwd, project] of projects) {
      const duplicates = [...projects.values()].filter(p => p === project).length > 1;
      const label = t("Project: {project}", { project });
      const tokenLabel = duplicates ? t("Project: {project}", { project: cwd }) : label;
      rows.push({ label, detail: cwd, mention: { token: `@${JSON.stringify(tokenLabel)}`, target: { kind: "project", cwd } } });
    }
    for (const s of this.sessions) {
      rows.push({ ...this.sessionMention(s), detail: [s.cwd, s.id].filter(Boolean).join(" · ") });
    }
    this.mentionStart = end - (typed.length + 1);
    this.mentionRows = rows.filter(r => `${r.label} ${r.detail}`.toLocaleLowerCase().includes(query)
      && !this.mentions.some(m => m.token === r.mention.token)).slice(0, 8);
    if (!this.mentionRows.length) { this.closeMentions(); return; }
    this.mentionIndex = Math.max(0, this.mentionRows.findIndex(r => r.mention.token === previous));
    this.paintMentions();
  }

  private paintMentions(): void {
    this.mentionMenu.replaceChildren(...this.mentionRows.map((row, i) => {
      const button = el("button", { class: `menu-item${i === this.mentionIndex ? " on" : ""}`, onclick: () => { this.mentionIndex = i; this.pickMention(); } },
        el("strong", { text: row.label }), el("small", { text: row.detail }));
      button.id = `${this.mentionMenu.id}-${i}`;
      button.setAttribute("role", "option");
      button.tabIndex = -1;
      button.setAttribute("aria-selected", String(i === this.mentionIndex));
      button.addEventListener("pointerdown", event => event.preventDefault());
      return button;
    }));
    this.mentionMenu.classList.add("on");
    this.sizeMentions();
    this.input.setAttribute("aria-expanded", "true");
    this.input.setAttribute("aria-activedescendant", `${this.mentionMenu.id}-${this.mentionIndex}`);
    this.mentionMenu.children[this.mentionIndex]?.scrollIntoView?.({ block: "nearest" });
  }

  private sizeMentions(): void {
    if (!this.mentionMenu.classList.contains("on")) return;
    const box = this.element.getBoundingClientRect();
    const composer = this.mentionMenu.parentElement?.getBoundingClientRect();
    const zoom = box.height / this.element.offsetHeight || 1;
    if (composer && Number.isFinite(composer.top - box.top)) {
      this.mentionMenu.style.maxHeight = `${Math.max(40, Math.min(240, (composer.top - box.top) / zoom - 6))}px`;
    }
  }

  private pickMention(): void {
    const row = this.mentionRows[this.mentionIndex];
    if (!row) return;
    const end = this.input.selectionStart ?? this.input.value.length;
    const inserted = `${row.mention.token} `;
    this.input.value = this.input.value.slice(0, this.mentionStart) + inserted + this.input.value.slice(end);
    this.mentions.push(row.mention);
    const caret = this.mentionStart + inserted.length;
    this.input.setSelectionRange(caret, caret);
    this.closeMentions();
    this.grow(); this.paintSend(); this.keepDraft(); this.input.focus();
  }

  /** What Zeca does while the chat is open. */
  clip(now: number): string {
    if (!this.enabled) return "idle";
    if (this.messages.some(m => m.who === "ask" && !m.answer)) return "question";
    if (now < this.swallowUntil) return "swallow";
    if (this.dragOver) return "gape";
    if (this.voice === "listening") return "listen";
    if (this.voice === "transcribing") return "think";
    if (this.busy && this.searching) return "search";
    if (this.reaction && now < this.reaction.until) return this.reaction.clip;
    return this.busy ? "think" : "idle";
  }

  private clearReaction(): void {
    window.clearTimeout(this.reactionTimer);
    this.reactionTimer = undefined;
    this.reaction = null;
  }

  /** Only live events get a brief reaction; saved replies and hidden surfaces never replay it. */
  private react(clip: "listen" | "speak"): void {
    this.clearReaction();
    if (!this.enabled || !this.open || document.hidden) return;
    this.reaction = { clip, until: Clock.now() + (clip === "listen" ? 1500 : 1600) };
    this.reactionTimer = window.setTimeout(() => {
      this.clearReaction();
      this.changed();
    }, clip === "listen" ? 1500 : 1600);
  }

  // ── Voice ────────────────────────────────────────────────────────────────

  /** The microphone's level now, 0..1, while listening. */
  voiceLevel(level: number): void {
    if (this.voice !== "listening") return;
    this.levels = [...this.levels.slice(1), Math.max(0, Math.min(1, level))];
    this.paintVoice();
  }

  /** The text heard so far, while listening. Late ones (the final is on its way) are dropped. */
  voicePartial(text: string): void {
    if (this.voice !== "listening" || text === this.partial) return;
    this.partial = text;
    this.paintVoice();
  }

  /** Lab only: shows a voice state without a backend round trip. */
  showVoice(state: Voice, levels?: number[]): void {
    this.voice = state;
    if (state === "off") this.partial = "";
    if (levels) this.levels = levels.slice(-WAVE_BARS);
    this.paintVoice();
    this.changed();
  }

  private async toggleVoice(): Promise<void> {
    if (this.voice === "off") await this.startVoice(true);
    else if (this.voice === "listening") await this.stopVoice();
  }

  /** Tap-to-talk heard the user finish: stop as the click would. Held recordings never get it. */
  voiceSilence(): void {
    if (this.voice === "listening") void this.stopVoice();
  }

  /** The talk shortcut: held down records, let go transcribes. It opens the chat if needed. */
  holdToTalk(down: boolean): void {
    if (!this.backend.voice || !this.voiceReady || !this.enabled) return;
    if (down && this.voice === "off") {
      this.toggle(true);
      void this.startVoice(false);
    } else if (!down && this.voice === "listening") void this.stopVoice();
  }

  private async startVoice(tap: boolean): Promise<void> {
    const voice = this.backend.voice;
    if (!voice) return;
    this.levels = Array(WAVE_BARS).fill(0);
    this.partial = "";
    this.showVoice("listening");
    this.starting = voice.start(tap).catch((e) => {
      this.showVoice("off");
      this.receive({ kind: "error", message: String(e) });
    });
    await this.starting;
  }

  private async stopVoice(): Promise<void> {
    const voice = this.backend.voice;
    if (!voice) return;
    // A key let go before the mic opened: stop once it has, or the stop finds nothing.
    await this.starting;
    if (this.voice !== "listening") return;
    this.showVoice("transcribing");
    try {
      const text = (await voice.stop()).trim();
      if (text) {
        const before = this.input.value.trimEnd();
        this.input.value = before ? `${before} ${text}` : text;
        this.keepDraft();
      }
    } catch (e) {
      this.receive({ kind: "error", message: String(e) });
    }
    this.showVoice("off");
    // Measured once the input shows again: hidden while transcribing, it would measure 0.
    this.grow();
    this.paintSend();
    this.input.focus();
  }

  /** A turn runs: no other conversation can be opened or started until it ends. */
  isBusy(): boolean {
    return this.busy;
  }

  /** The saved conversations, newest first (the Nest's *Recent*). */
  conversations(): Promise<ConversationView[]> {
    return this.backend.conversations();
  }

  /** Goes on with a saved conversation (the Nest's *Recent*); never while a turn runs. */
  openConversation(id: number): void {
    void this.openSaved(id);
  }

  /** A new conversation, the one so far saved in the list (the Nest's *New conversation*). */
  newConversation(): void {
    this.ask("new");
  }

  /** Puts words in the composer for the user to finish or send (the Nest's *Ask Zeca*); after
   *  what was already typed there, never over it. */
  draft(text: string): void {
    const typed = this.input.value.trim();
    this.input.value = typed ? `${typed}\n\n${text}` : text;
    this.grow();
    this.paintSend();
    this.keepDraft();
    this.input.focus();
  }

  /** Escape while listening drops the recording instead of closing the chat. */
  cancelVoice(): boolean {
    if (this.voice !== "listening") return false;
    this.backend.voice?.cancel();
    this.showVoice("off");
    return true;
  }

  private paintVoice(): void {
    const on = this.voice !== "off";
    this.mic.hidden = !this.backend.voice || !this.voiceReady;
    this.mic.classList.toggle("on", this.voice === "listening");
    // Only when the state changes: this runs on every mic level (each 50 ms), and a button whose
    // icon is replaced between press and release never gets the click.
    const glyph = this.voice === "listening" ? "stop" : "mic";
    if (this.mic.dataset.glyph !== glyph) {
      this.mic.dataset.glyph = glyph;
      this.mic.replaceChildren(glyph === "stop" ? icon("stop", 12, 2.4) : icon("mic", 16, 2));
    }
    this.mic.title = this.voice === "listening" ? t("Stop and transcribe") : t("Speak");
    this.mic.setAttribute("aria-label", this.mic.title);
    this.mic.toggleAttribute("disabled", this.voice === "transcribing");
    this.input.hidden = on;
    this.wave.hidden = !on;
    this.wave.classList.toggle("busy", this.voice === "transcribing");
    this.wave.classList.toggle("heard", !!this.partial);
    // With words to show, a short waveform leaves them the room.
    const levels = this.partial ? this.levels.slice(-WAVE_BARS_HEARD) : this.levels;
    this.wave.replaceChildren(
      ...levels.map((l) => {
        const bar = el("span");
        bar.style.height = `${Math.round(8 + l * 92)}%`;
        return bar;
      }),
      // The partial stays, dimmed, until the final text replaces it; its end shows, where the
      // words are coming in.
      this.partial
        ? el("em", { class: "partial" }, el("span", { text: this.partial }))
        : el("em", {
            text: this.voice === "transcribing" ? t("Transcribing…") : t("Listening…"),
          }),
    );
  }

  /** Zeca on or off: off, the chat closes and nothing opens it (ADR 0018). */
  setEnabled(on: boolean): void {
    this.enabled = on;
    if (!on) {
      this.cancelVoice();
      this.toggle(false);
    }
    this.paintUsage();
  }
  private enabled = true;

  toggle(open = !this.open): void {
    if (open && !this.enabled) return;
    if (!open) {
      this.cancelVoice();
      this.clearReaction();
    }
    if (open === this.open) return;
    this.open = open;
    this.closeMentions();
    this.paintDrop();
    this.menuOpen = false;
    this.modelOpen = false;
    this.accessOpen = false;
    this.backend.keyboard(open);
    if (open) queueMicrotask(() => this.input.focus());
    this.paintHead();
    this.changed();
  }

  /** Something is dragged over the island: the chat opens on the drop zone. */
  setDragOver(on: boolean): void {
    if (on === this.dragOver) return;
    this.dragOver = on;
    if (on) this.toggle(true);
    this.paintDrop();
    this.changed();
  }

  isShowingDrop(): boolean {
    return this.dragOver;
  }

  /** The drop zone in its phase: waiting for a file, carrying it in, then gone (the files are on the next message). */
  private paintDrop(): void {
    const c = this.carried;
    const phase = this.dragOver ? "waiting" : c ? "carrying" : null;
    this.drop.classList.toggle("on", phase !== null);
    // A class, not :has(): the chat repaints a level every 60 ms while it listens.
    this.element.classList.toggle("dropping", phase !== null);
    // Shown from the tab (nothing dragged yet): it explains, and a click puts it away.
    if (this.drop.dataset.phase === `${phase}|${c?.paths.length ?? 0}`) return;
    this.drop.dataset.phase = `${phase}|${c?.paths.length ?? 0}`;
    const label = c ? (c.paths.length === 1 ? shortName(c.paths[0]) : t("{n} files", { n: c.paths.length })) : "";
    if (phase === "carrying") {
      const bird = el("span", { class: "carry-bird" });
      bird.style.backgroundImage = `url(${flightStrip()})`;
      this.dropBody.replaceChildren(
        el("span", { class: "drop-title", text: t("Taking {name}", { name: label }) }),
        el("span", { class: "carry" }, bird, el("span", { class: "carry-track" }, el("span", { class: "carry-fill" }))),
      );
    } else {
      this.dropBody.replaceChildren(
        el("span", { class: "drop-title", text: t("Drop files here") }),
        el("span", { class: "drop-kinds" }, ...["PDF", "Images", "Code", "Text"].map((k) => el("span", { class: "kind", text: k }))),
      );
    }
  }

  /** Carried in: the files stay on the next message, and the input takes the keyboard. */
  private keepDropped(): void {
    this.carried = null;
    this.paintDrop();
    this.changed();
    this.input.focus();
  }

  attach(paths: string[], refused: Refused[] = []): void {
    this.setDragOver(false);
    this.paintDrop();
    for (const r of refused)
      this.messages.push({ who: "note", text: REFUSED[r.reason](r.name) });
    if (paths.length) {
      const fresh = paths.filter((p) => !this.files.includes(p));
      this.files.push(...fresh);
      this.keepDraft();
      this.swallowUntil = Clock.now() + SWALLOW_MS;
      window.setTimeout(() => Sound.play("swallow"), 450);
      window.setTimeout(() => this.changed(), SWALLOW_MS);
      // Carried across the zone, then onto the next message. The zone is the island's: the Nest's
      // paperclip puts them straight on the next message.
      const carried = { paths: fresh };
      this.carried = carried;
      if (!this.compact) {
        this.keepDropped();
        this.toggle(true);
        this.paint();
        this.changed();
        return;
      }
      this.paintDrop();
      // Carried in, it is on the next message (its chip, × to take it off): no question asked.
      window.setTimeout(() => {
        if (this.carried !== carried) return;
        Sound.play("alertOk");
        this.keepDropped();
      }, CARRY_MS);
    }
    this.toggle(true);
    this.paint();
    this.changed();
  }

  receive(d: ChatDelta): void {
    this.conversationVersion++;
    if (d.kind === "done" || d.kind === "stopped" || d.kind === "error") this.liveEnded = true;
    if (d.kind === "activity") {
      const activity = this.turnActivity();
      if (d.event.type === "start") this.replyBreak = true;
      applyActivity(activity.data, d.event);
      if (this.activityTimer === undefined) this.activityTimer = window.setTimeout(() => {
        this.activityTimer = undefined;
        const follow = this.following && this.atEnd();
        this.paint(); this.after(follow); this.changed();
      }, STREAM_MS);
      return;
    }
    if (d.kind === "toolstate") {
      this.clearReaction();
      this.searching = d.active;
      this.changed();
      return;
    }
    const last = this.messages[this.messages.length - 1];
    const follow = this.following && this.atEnd();
    if (d.kind === "text") {
      if (d.text) {
        this.react("speak");
      }
      if (last?.who === "zeca" && !last.error && !this.replyBreak) {
        // Streaming: only the message being written changes. Tokens come many a second, and each
        // repaint re-reads its markdown and renders the island: at most one every STREAM_MS.
        last.text += d.text;
        if (this.streaming === undefined) {
          const i = this.messages.length - 1;
          this.streaming = window.setTimeout(() => {
            this.streaming = undefined;
            // Reset or rebuilt meanwhile: whatever painted last already shows it.
            if (this.messages[i] !== last) return;
            const follow = this.following && this.atEnd();
            this.replace(i);
            this.after(follow);
            this.changed();
          }, STREAM_MS);
        }
        return;
      }
      this.replyBreak = false;
      this.messages.push({ who: "zeca", text: d.text });
    } else if (d.kind === "permission") {
      this.clearReaction();
      const { id, tool, target, description, full, added, removed, always, outside } = d;
      this.messages.push({
        who: "ask",
        id,
        tool,
        target,
        description,
        full,
        added,
        removed,
        always,
        outside,
      });
      Sound.play("approval");
    } else if (d.kind === "tool") {
      this.replyBreak = true;
      this.clearReaction();
      // The API chat's own tools only read: a line says which one it used, no card.
      const activity = this.turnActivity();
      if (!activity.data.tools.some(tool => tool.tool === d.tool)) applyActivity(activity.data, { type: "start", id: `api:${activity.data.tools.length}`, tool: d.tool, detail: "" });
    } else if (d.kind === "settled") {
      this.clearReaction();
      // The user's choice answered with no card: the chat says what ran, and on whose word.
      this.addActivity({ type: "decision", tool: d.tool, target: d.target, answer: d.allow ? "allow" : "deny", by: d.by });
    } else if (d.kind === "allowed") {
      this.clearReaction();
      this.addActivity({ type: "decision", tool: d.tool, target: d.target, answer: "allow", by: "conversation" });
    } else {
      // Only a turn's failure can be sent again; a mic that would not open has nothing to resend.
      const turn = this.busy;
      this.clearReaction();
      this.searching = false;
      if (d.kind === "done" && turn && last?.who === "zeca" && !last.error && last.text.trim()) this.react("speak");
      this.busy = false;
      if (d.kind === "error")
        this.messages.push({ who: "zeca", text: REFERENCE_ERRORS.has(d.message) ? t(d.message) : d.message, error: true, retry: turn });
      if (d.kind === "stopped")
        this.messages.push({ who: "note", text: t("Stopped.") });
      for (const message of this.messages) if (message.who === "activity") message.ended = true;
      // Anything still waiting is a no now: it can't be answered any more.
      for (const m of this.messages)
        if (m.who === "ask" && !m.answer) { m.answer = "deny"; m.cancelled = true; this.askNodes.delete(m); }
    }
    this.paint();
    this.after(follow);
    this.changed();
  }

  /** What the other window did (`chat-sync`), shown here too. */
  sync(s: ChatSync): void {
    if (s.from === this.opts.window) return;
    if (s.kind === "draft") {
      this.restoreDraft(s.draft);
      return;
    }
    this.conversationVersion++;
    if (s.kind !== "answered") this.sourceVersion++;
    if (s.kind === "sent") {
      this.clearReaction();
      this.searching = false;
      this.draftEdited = true;
      this.liveEnded = false;
      // A conversation that began in the other window: shown where and how it runs.
      if (!this.started) {
        this.provider = s.provider;
        this.folder = s.folder;
        this.picked = true;
        this.picksFor = s.folder;
        this.picksSeq++;
        this.picks = { [s.provider]: { model: s.model, effort: s.effort } };
      }
      this.messages.push({ who: "you", text: s.text, files: s.files, mentions: s.mentions });
      this.busy = true;
      this.turnSince = Clock.now();
      this.started = true;
      this.menuOpen = this.modelOpen = this.accessOpen = this.historyOpen = false;
      this.paint();
      this.toEnd();
      this.changed();
    } else if (s.kind === "reset") this.cleared(s.provider);
    else if (s.kind === "opened") this.load(s.conversation, true);
    else {
      const m = this.messages.find((m): m is Ask => m.who === "ask" && m.id === s.id);
      if (!m || m.answer) return;
      m.answer = s.answer;
      this.askNodes.delete(m);
      this.paint();
      this.changed();
    }
  }

  /** Answers the permission waiting in the chat, if one is; for the global shortcuts. */
  answerWaiting(allow: boolean): boolean {
    if (!this.open || !this.enabled || document.hidden || !this.element.isConnected || this.element.offsetParent === null) return false;
    const waiting = this.pendingAsk();
    const node = waiting && this.askNodes.get(waiting);
    if (node) {
      const card = node.getBoundingClientRect(), viewport = this.log.getBoundingClientRect();
      if (Number.isFinite(card.top)) {
        const operation = node.querySelector(".code")?.getBoundingClientRect();
        const actions = node.querySelector(".actions")?.getBoundingClientRect();
        if (card.bottom <= viewport.top || card.top >= viewport.bottom || !operation || !actions
          || operation.bottom <= viewport.top || operation.top >= viewport.bottom
          || actions.top < viewport.top || actions.bottom > viewport.bottom) return false;
      }
    }
    if (!waiting) return false;
    this.answer(waiting, allow ? "allow" : "deny");
    return true;
  }

  // ── Sending ──────────────────────────────────────────────────────────────

  private sendOrStop(): void {
    if (this.busy) {
      this.backend.stop();
      return;
    }
    void this.submit();
  }

  /** The last thing the user sent, back in the composer to change or send again. */
  private recall(): boolean {
    const you = this.lastYou();
    if (!you?.text) return false;
    this.input.value = you.text;
    this.mentions = [...(you.mentions ?? [])];
    this.grow();
    this.paintSend();
    this.keepDraft();
    const end = this.input.value.length;
    this.input.setSelectionRange(end, end);
    return true;
  }

  private lastYou(): Extract<Message, { who: "you" }> | undefined {
    for (let i = this.messages.length - 1; i >= 0; i--) {
      const m = this.messages[i];
      if (m.who === "you") return m;
    }
    return undefined;
  }

  /** The turn that failed, sent again as it was: its words and its files. */
  private retry(): void {
    const you = this.lastYou();
    if (this.busy || !you) return;
    this.input.value = you.text;
    this.mentions = [...(you.mentions ?? [])];
    this.files = [...new Set([...this.files, ...you.files])];
    void this.submit();
  }

  private async submit(): Promise<void> {
    try {
      await this.restore();
    } catch (error) {
      this.receive({ kind: "error", message: String(error) });
      return;
    }
    const text = this.input.value.trim();
    // Typing ahead is fine while a reply streams; sending waits for it.
    if (this.busy || (!text && !this.files.length)) return;
    const files = this.files;
    // The user's own words for memory or continuation, kept by the app: never a turn, never sent to
    // a model. With files attached the command takes only the text, and the files wait for the next
    // message; where nothing keeps commands (the lab), nothing is sent.
    const memory = /^\/(remember|forget)(\s|$)/.test(text);
    const handoff = /^\/continue(\s|$)/.test(text);
    if (memory || handoff) {
      const command = memory ? this.backend.memory : this.backend.handoff;
      if (!command) return;
      const answer = await command(text, this.folder).catch((e: unknown) => ({ said: String(e), detail: null }));
      if (answer) {
        this.input.value = "";
        this.keepDraft();
        this.messages.push({ who: "you", text, files: [] });
        const said = MEMORY_SAID.has(answer.said) ? t(answer.said) : answer.said;
        this.messages.push({ who: "zeca", text: answer.detail ? `${said}\n${answer.detail}` : said });
        this.paint();
        this.toEnd();
        return;
      }
    }
    const mentions = this.mentions.filter(m => text.includes(m.token));
    const wasStarted = this.started;
    this.messages.push({ who: "you", text, files, mentions });
    this.mentions = [];
    this.closeMentions();
    this.files = [];
    this.input.value = "";
    this.input.style.height = "";
    this.keepDraft();
    this.clearReaction();
    this.searching = false;
    this.busy = true;
    this.turnSince = Clock.now();
    this.started = true;
    this.menuOpen = false;
    this.modelOpen = false;
    this.accessOpen = false;
    this.historyOpen = false;
    this.paint();
    this.toEnd();
    this.changed();
    try {
      await this.draftSaved;
      await this.backend.send(text, files, this.folder, mentions);
    } catch (e) {
      if (REFERENCE_ERRORS.has(String(e))) {
        this.started = wasStarted;
        // A rejected reference starts no turn; keep it editable without replacing newer typing.
        if (!this.input.value && !this.files.length) {
          this.input.value = text;
          this.files = files;
          this.mentions = mentions;
          this.grow(); this.keepDraft();
        }
      }
      this.receive({ kind: "error", message: String(e) });
    }
  }

  /** New, or another provider: the conversation so far stays saved, in the list. */
  private ask(what: ChatProvider | "new"): void {
    if (this.busy) return;
    void this.restart(what === "new" ? null : what);
  }

  private async restart(provider: ChatProvider | null): Promise<void> {
    await this.flushDraft();
    this.cleared(await this.backend.reset(provider));
  }

  /** A new conversation with `provider`, here or (synced) in the other window. */
  private cleared(provider: ChatProvider): void {
    this.clearReaction();
    this.searching = false;
    this.mentions = [];
    this.closeMentions();
    this.conversationVersion++;
    this.sourceVersion++;
    this.liveEnded = false;
    this.input.value = "";
    this.files = [];
    this.provider = provider;
    this.busy = false;
    this.menuOpen = this.accessOpen = this.historyOpen = false;
    // The folder's pick for new conversations, not the one just left.
    this.picksFor = undefined;
    this.messages = [];
    this.started = false;
    this.modelOpen = false;
    this.native = this.nativeSetting;
    this.picked = false;
    this.keepDraft();
    this.paint();
    this.changed();
  }

  private answer(m: Ask, answer: ChatAnswer): void {
    if (m.answer || this.pendingAsk() !== m || !this.messages.includes(m) || !this.enabled || !this.open || document.hidden || !this.element.isConnected || this.element.offsetParent === null) return;
    m.answer = answer;
    this.askNodes.delete(m);
    Sound.play(answer === "deny" ? "deny" : "allow");
    this.backend.decide(m.id, answer);
    this.paint();
    this.reviewApproval();
    this.changed();
  }

  // ── Scrolling ────────────────────────────────────────────────────────────

  private atEnd(): boolean {
    return (
      this.log.scrollHeight - this.log.scrollTop - this.log.clientHeight <
      FOLLOW_PX
    );
  }

  private toEnd(): void {
    this.following = true;
    this.lastScrollInput = -Infinity;
    this.log.scrollTop = this.log.scrollHeight;
    this.log.classList.toggle("scrolled", this.log.scrollTop > 2);
    this.jump.classList.remove("on");
  }

  /** After the log changed: keep following the end, or say there is more below. */
  private after(follow: boolean): void {
    // Check again after a queued stream repaint: user input outranks the earlier snapshot.
    if (follow && this.following && performance.now() - this.lastScrollInput >= SCROLL_COOLDOWN_MS) this.toEnd();
    else this.jump.classList.add("on");
  }

  /** Measure content before the island measures its native hit region; menus never size the log. */
  fitContents(): void {
    this.sizeMentions();
    if (!this.compact || !this.element.isConnected) return;
    const top = this.log.scrollTop;
    const height = this.log.style.height;
    this.log.style.height = "auto";
    const content = this.messages.length || this.busy || this.provider === "api" ? this.log.scrollHeight : 0;
    this.log.style.height = height;
    this.element.style.setProperty("--chat-content-height", `${Math.ceil(content)}px`);
    const foot = this.element.querySelector<HTMLElement>(".chat-foot");
    this.element.style.setProperty("--chat-menu-height", `${Math.max(0, this.element.clientHeight - (foot?.offsetHeight ?? 0) - 16)}px`);
    this.log.scrollTop = this.following && performance.now() - this.lastScrollInput >= SCROLL_COOLDOWN_MS
      ? this.log.scrollHeight : top;
  }

  // ── Painting ─────────────────────────────────────────────────────────────

  private grow(): void {
    const before = this.input.style.height;
    this.input.style.height = "auto";
    const line = parseFloat(getComputedStyle(this.input).lineHeight) || 18;
    const border = this.input.offsetHeight - this.input.clientHeight;
    this.input.style.height = `${Math.min(this.input.scrollHeight + border, line * MAX_INPUT_LINES + 16 + border)}px`;
    // Only a new line reshapes the island: a keystroke within one renders nothing else.
    if (this.input.style.height !== before) this.changed();
  }

  private node(m: Message): HTMLElement {
    const node = this.messageNode(m);
    if (!this.appeared.has(m) && (m.who === "you" || (m.who === "zeca" && !m.error))) node.classList.add("message-enter");
    this.appeared.add(m);
    return node;
  }

  private messageNode(m: Message): HTMLElement {
    if (m.who === "activity") return this.activityNode(m);
    if (m.who === "zeca") {
      const last = m === this.messages[this.messages.length - 1];
      if (m.error) {
        // Only the latest failure offers it: an older one was already followed by something else.
        if (!m.retry || !last || this.busy) return el("p", { class: "msg zeca error", text: m.text });
        const again = el("button", { class: "msg-tool", onclick: () => this.retry() }, icon("retry", 12, 2), el("span", { text: t("Try again") }));
        return el("div", { class: "msg zeca error" }, el("span", { text: m.text }), el("div", { class: "msg-tools" }, again));
      }
      // The latest reply, once it is whole, keeps its tools in view; the others show them on hover.
      const b = el("div", { class: `msg zeca${last && !this.busy ? " settled" : ""}` });
      // In the Nest's wide column a turn's reply is signed, so each one reads apart from the next.
      if (this.opts.window === "nest" && this.messages[this.messages.indexOf(m) - 1]?.who === "you") b.append(this.signature());
      b.append((this.opts.render ?? renderFull)(m.text), el("div", { class: "msg-tools" }, this.copyButton(m.text)));
      if (last && !this.busy && m.text.trim()) b.append(this.replySuggestions);
      return b;
    }
    if (m.who === "note") return el("p", { class: "msg note", text: m.text });
    if (m.who === "you") {
      return el(
        "div",
        { class: "msg you" },
        ...(m.text ? [el("span", { text: m.text })] : []),
        ...(m.files.length
          ? [
              el(
                "span",
                { class: "files" },
                ...m.files.map((f) =>
                  el(
                    "span",
                    { class: "file" },
                    icon("file", 11),
                    el("span", { text: shortName(f) }),
                  ),
                ),
              ),
            ]
          : []),
      );
    }
    return this.askNode(m);
  }

  private turnActivity(): ActivityMessage {
    let start = this.messages.length - 1;
    while (start >= 0 && this.messages[start].who !== "you") start--;
    const existing = this.messages.slice(start + 1).find((m): m is ActivityMessage => m.who === "activity");
    if (existing) return existing;
    const activity: ActivityMessage = { who: "activity", data: emptyActivity(), ended: !this.busy };
    this.messages.splice(start + 1, 0, activity);
    return activity;
  }

  private addActivity(event: ChatActivityEvent): void {
    applyActivity(this.turnActivity().data, event);
  }

  private activityNode(m: ActivityMessage): HTMLElement {
    let kept = this.activityNodes.get(m);
    if (!kept) {
      const summary = el("summary");
      const reasoning = el("pre", { class: "activity-reasoning" });
      reasoning.tabIndex = 0;
      reasoning.setAttribute("role", "region");
      reasoning.setAttribute("aria-label", t("Thinking details"));
      const tools = el("div", { class: "activity-tools" });
      const decisions = el("div", { class: "activity-decisions" });
      const warning = el("p", { class: "activity-warning" });
      const node = el("details", { class: "chat-activity" }, summary,
        el("div", { class: "activity-body" }, el("h4", { text: t("Thinking details") }), reasoning, tools, decisions, warning));
      node.addEventListener("toggle", () => this.changed());
      kept = { node, summary, reasoning, tools, decisions, warning, toolNodes: new Map() };
      this.activityNodes.set(m, kept);
    }
    const { data } = m;
    kept.summary.textContent = data.tools.length === 1 ? t("Activity · 1 tool") : data.tools.length ? t("Activity · {n} tools", { n: data.tools.length }) : t("Activity");
    const reasoning = data.reasoning.trim() ? data.reasoning : t("This engine has not provided thinking details.");
    if (kept.reasoning.textContent !== reasoning) kept.reasoning.textContent = reasoning;
    const toolNodes = data.tools.map(tool => {
      const status = tool.ok === true ? t("Completed") : tool.ok === false ? t("Failed") : m.ended ? t("No result reported") : t("Running");
      let entry = kept!.toolNodes.get(tool.id);
      if (!entry) {
        const label = el("span", { class: "tool-status" });
        const detail = el("pre", { text: tool.detail || t("No arguments reported.") });
        detail.tabIndex = 0;
        detail.setAttribute("role", "region");
        detail.setAttribute("aria-label", tool.tool);
        const node = el("details", { class: "activity-tool" }, el("summary", {}, el("span", { text: tool.tool }), label), detail);
        node.addEventListener("toggle", () => this.changed());
        entry = { node, status: label, detail };
        kept!.toolNodes.set(tool.id, entry);
      }
      entry.status.textContent = status;
      entry.status.classList.toggle("failed", tool.ok === false);
      return entry.node;
    });
    if (kept.tools.children.length !== toolNodes.length) kept.tools.replaceChildren(...toolNodes);
    kept.decisions.replaceChildren(...data.decisions.map(d => {
      const text = d.by === "full-access" ? t("Ran with full access: {target}", { target: d.target })
        : d.by === "policy" ? (d.answer === "deny" ? t("Denied by your policy: {target}", { target: d.target }) : t("Allowed by your policy: {target}", { target: d.target }))
        : d.by === "conversation" || d.answer === "always" ? t("Allowed for this conversation: {target}", { target: d.target })
        : d.answer === "deny" ? t("You denied: {target}", { target: d.target }) : t("You allowed: {target}", { target: d.target });
      return el("p", { text });
    }));
    kept.warning.textContent = data.truncated ? t("Activity details were shortened to keep this conversation responsive.") : "";
    return kept.node;
  }

  private pendingAsk(): Ask | undefined {
    const pending = this.messages.filter((m): m is Ask => m.who === "ask" && !m.answer);
    const selected = pending.find(m => m.id === this.selectedAsk) ?? pending[0];
    this.selectedAsk = selected?.id ?? null;
    return selected;
  }

  private reviewApproval(): void {
    const request = this.pendingAsk();
    const node = request && this.askNodes.get(request);
    if (!node) return;
    node.scrollIntoView?.({ block: "nearest" });
    node.querySelector<HTMLElement>(".code")?.scrollIntoView?.({ block: "nearest" });
    node.querySelector<HTMLButtonElement>(".actions button")?.focus({ preventScroll: true });
  }

  private paintApprovalQueue(): void {
    const pending = this.messages.filter((m): m is Ask => m.who === "ask" && !m.answer);
    const key = JSON.stringify([locale(), this.pendingAsk()?.id, pending.map(m => m.id)]);
    if (key === this.approvalQueueKey) return;
    this.approvalQueueKey = key;
    this.approvalQueue.classList.toggle("on", pending.length > 0);
    if (!pending.length) { this.approvalQueue.replaceChildren(); return; }
    const current = this.pendingAsk()!;
    const select = (step: number) => {
      this.selectedAsk = pending[(pending.indexOf(current) + step + pending.length) % pending.length].id;
      this.paint(); this.reviewApproval(); this.changed();
    };
    const review = el("button", { class: "approval-review", text: t("Review approval {index} of {total}", { index: pending.indexOf(current) + 1, total: pending.length }), onclick: () => this.reviewApproval() });
    const previous = el("button", { text: "‹", onclick: () => select(-1) });
    previous.setAttribute("aria-label", t("Previous approval"));
    const next = el("button", { text: "›", onclick: () => select(1) });
    next.setAttribute("aria-label", t("Next approval"));
    const settings = this.backend.openSettings ? el("button", { class: "approval-settings", text: t("Permissions"), onclick: () => this.backend.openSettings?.() }) : null;
    if (settings) settings.title = t("To reduce repeated requests, choose your policies in Settings. Destructive actions still ask.");
    this.approvalQueue.replaceChildren(review, ...(pending.length > 1 ? [previous, next] : []), ...(settings ? [settings] : []));
  }

  private askNode(m: Ask): HTMLElement {
    if (!m.answer && this.pendingAsk() !== m) return el("span", { class: "chat-hidden" });
    const kept = this.askNodes.get(m);
    if (kept) return kept;
    const diff =
      m.added || m.removed
        ? el(
            "span",
            { class: "diff" },
            el("span", { class: "add", text: `+${m.added}` }),
            el("span", { class: "del", text: `−${m.removed}` }),
          )
        : null;
    const kbd = (k: string | undefined) =>
      k ? el("span", { class: "kbd", text: k }) : null;
    const button = (
      text: string,
      kind: string,
      answer: ChatAnswer,
      keys: string | undefined,
    ) =>
      el(
        "button",
        { class: `btn ${kind}`, onclick: () => this.answer(m, answer) },
        el("span", { text }),
        kbd(keys),
      );
    const always = button(t("Always in this chat"), "secondary", "always", undefined);
    always.title = t("Allows this exact command again, without asking, until this conversation ends. A different command still asks.");
    const settled: Record<ChatAnswer, string> = {
      allow: t("Allowed"),
      deny: t("Denied"),
      always: t("Allowed for this conversation"),
    };
    const operation = el("pre", { class: "code", text: m.full ? `${m.tool} · ${m.full}` : m.target });
    operation.tabIndex = 0;
    operation.setAttribute("role", "region");
    operation.setAttribute("aria-label", t("Code"));
    const card = el(
      "div",
      { class: `ask${m.answer ? ` ${m.answer === "always" ? "allow" : m.answer}` : ""}` },
      el(
        "div",
        { class: "who" },
        el("span", { class: "ask-heading" },
          el("span", { class: "ask-state", text: m.answer ? t("Permission answered") : t("Permission required") }),
          el("span", { class: "label", text: asks(m.tool) }),
        ),
        diff,
      ),
      el("div", { class: "ask-context" }, el("span", { text: t("Folder") }),
        el("code", { text: this.folder ?? t("An empty folder") })),
      ...(m.outside
        ? [
            el("div", {
              class: "ask-outside",
              text: t("This conversation read content from outside ({tool}). It may be steering this request: check it carefully.", { tool: m.outside }),
            }),
          ]
        : []),
      ...(m.description
        ? [el("div", { class: "ask-what", text: m.description })]
        : []),
      operation,
      m.answer
        ? el(
            "div",
            { class: `settled-label ${m.answer === "always" ? "allow" : m.answer}` },
            icon(m.answer === "deny" ? "close" : "check", 12, 2.6),
            el("span", { text: m.cancelled ? t("Cancelled") : settled[m.answer] }),
          )
        : el(
            "div",
            { class: "actions" },
            button(t("Deny"), !m.always || m.outside ? "primary" : "secondary", "deny", this.keys.deny),
            button(t("Allow"), !m.always || m.outside ? "secondary" : "primary", "allow", this.keys.allow),
            m.always ? el("details", { class: "ask-more" }, el("summary", { class: "btn secondary", text: t("More choices") }), always) : null,
          ),
    );
    if (m.answer) {
      const receipt = el("details", { class: "approval-receipt" }, el("summary", { text: `${m.cancelled ? t("Cancelled") : settled[m.answer]} · ${m.tool}` }), card);
      receipt.addEventListener("toggle", () => this.changed());
      this.askNodes.set(m, receipt);
      return receipt;
    }
    if (!this.appeared.has(m)) card.classList.add("approval-enter");
    card.addEventListener("animationend", event => { if (event.target === card) card.classList.remove("approval-enter"); });
    this.askNodes.set(m, card);
    return card;
  }

  /** Re-renders one message in place. */
  private replace(i: number): void {
    const next = this.node(this.messages[i]);
    // Keep the bubble itself: streaming must not cancel and restart its entrance.
    const current = this.nodes[i];
    if (current) current.replaceChildren(...Array.from(next.childNodes));
  }

  /** The API choice only while a key is saved (or while its conversation is open). */
  private providers(): ChatProvider[] {
    return this.api.ready || this.provider === "api"
      ? ["claude", "codex", "api"]
      : ["claude", "codex"];
  }

  private async loadSaved(): Promise<void> {
    try {
      this.saved = await this.backend.conversations();
    } catch {
      this.saved = [];
    }
    this.paintHead();
    this.changed();
  }

  /** Goes on with a saved conversation, as it was. */
  private async openSaved(id: number): Promise<void> {
    // Never while a turn runs: opening ends the conversation, and its waiting card with it.
    if (this.busy) return;
    let c: OpenedConversation;
    try {
      await this.flushDraft();
      c = await this.backend.open(id);
    } catch (e) {
      this.historyOpen = false;
      this.messages.push({ who: "note", text: String(e) });
      this.paint();
      this.changed();
      return;
    }
    this.load(c, true);
  }

  /** Shows a saved conversation, as it was: opened here, in the other window, or the Nest's start. */
  load(c: OpenedConversation, clearDraft = false, preserveLive = false): void {
    if (!preserveLive) {
      this.clearReaction();
      this.searching = c.busy && !!c.tools;
    }
    this.conversationVersion++;
    this.sourceVersion++;
    if (clearDraft) {
      this.input.value = "";
      this.mentions = [];
      this.closeMentions();
      this.files = [];
    }
    this.menuOpen = this.modelOpen = this.accessOpen = this.historyOpen = false;
    if (!preserveLive) this.busy = false;
    this.provider = c.provider;
    // Pinned to its API provider, whatever Settings chose since.
    if (c.provider === "api") this.api = { ...this.api, label: c.label };
    this.folder = c.folder;
    this.picked = true;
    this.started = c.started;
    // Its own model and effort, not the folder's pick for new conversations.
    this.picksFor = c.folder;
    this.picksSeq++;
    this.picks = { [c.provider]: { model: c.model, effort: c.effort } };
    const saved = c.lines.map((l): Message => {
      if (l.who === "you") return { who: "you", text: l.text, files: l.files };
      if (l.who === "error") return { who: "zeca", text: l.text, error: true };
      if (l.who === "activity") return { who: "activity", data: readActivity(l.text), ended: true };
      if (l.who === "stopped") return { who: "note", text: t("Stopped.") };
      return { who: "zeca", text: l.text };
    });
    for (const message of saved) this.appeared.add(message);
    if (preserveLive) {
      // The owner is unchanged: bind its source without replacing newer cards or ending events.
      this.messages = [...saved, ...this.messages];
      if (!this.busy && c.busy && !this.liveEnded) {
        this.busy = true;
        this.turnSince = Clock.now();
      }
      this.paint();
      this.changed();
      return;
    }
    this.liveEnded = false;
    this.messages = saved;
    // Opened mid-turn: the reply so far and the cards waiting; the rest streams in.
    if (c.busy) {
      this.busy = true;
      // Its start is not known here: counted from now.
      this.turnSince = Clock.now();
      if (c.activity) this.messages.push({ who: "activity", data: readActivity(JSON.stringify(c.activity)), ended: false });
      if (c.partial) this.messages.push({ who: "zeca", text: c.partial });
      for (const w of c.waiting)
        this.messages.push({ who: "ask", id: w.id, tool: w.tool, target: w.target, description: w.description ?? null, full: w.full ?? null, added: w.added ?? 0, removed: w.removed ?? 0, always: w.always ?? false, outside: w.outside });
    }
    if (clearDraft) this.keepDraft();
    for (const message of this.messages) this.appeared.add(message);
    this.paint();
    this.toEnd();
    this.changed();
  }

  private paintHistory(): void {
    this.historyButton.classList.toggle("on", this.historyOpen);
    this.historyMenu.classList.toggle("on", this.historyOpen);
    if (!this.historyOpen) return;
    const row = (c: ConversationView): HTMLElement => {
      const where = c.folder ? t("{engine} in {folder}", { engine: c.label, folder: folderName(c.folder) }) : c.label;
      if (this.renaming === c.id) {
        const input = document.createElement("input");
        input.className = "rename";
        input.setAttribute("aria-label", t("Rename"));
        input.dataset.choice = `rename:${c.id}`;
        input.value = c.title;
        input.addEventListener("keydown", (e) => {
          if (e.isComposing || e.keyCode === 229) return;
          if (e.key === "Enter") {
            e.preventDefault();
            const title = input.value.trim();
            this.renaming = null;
            if (title) {
              c.title = title;
              void this.backend.rename(c.id, title).then(() => this.loadSaved());
            }
            this.paintHead();
            this.changed();
          } else if (e.key === "Escape") {
            // The island's own Escape closes the chat: this one only ends the renaming.
            e.stopPropagation();
            this.renaming = null;
            this.paintHead();
            this.changed();
          }
        });
        queueMicrotask(() => input.focus());
        return el("div", { class: "saved-row" }, input);
      }
      const edit = el("button", { class: "ghost icon", onclick: () => ((this.renaming = c.id), (this.deleting = null), this.paintHead(), this.changed()) });
      edit.append(icon("pencil", 12, 2));
      edit.title = t("Rename");
      const remove =
        this.deleting === c.id
          ? el("button", {
              class: "ghost danger",
              text: t("Delete"),
              onclick: () =>
                !this.busy &&
                void this.backend.remove(c.id).then((current) => {
                  this.deleting = null;
                  if (current) {
                    this.messages = [];
                    this.started = false;
                    this.picked = false;
                    this.picksFor = undefined;
                    this.paint();
                  }
                  return this.loadSaved();
                }),
            })
          : el("button", { class: "ghost icon", onclick: () => ((this.deleting = c.id), (this.renaming = null), this.paintHead(), this.changed()) });
      if (this.deleting !== c.id) {
        remove.append(icon("close", 12, 2.2));
        remove.title = t("Delete");
      }
      this.choice(edit, `rename-button:${c.id}`);
      this.choice(remove, `delete:${c.id}`);
      return el(
        "div",
        { class: `saved-row${c.current ? " on" : ""}` },
        this.choice(el(
          "button",
          { class: "saved-open", onclick: () => void this.openSaved(c.id) },
          el("span", { class: "saved-title", text: c.title }),
          el("span", { class: "saved-where", text: where }),
        ), `conversation:${c.id}`),
        edit,
        remove,
      );
    };
    this.historyMenu.replaceChildren(
      el("span", { class: "menu-label", text: t("Conversations") }),
      ...(this.saved.length
        ? this.saved.map(row)
        : [el("span", { class: "saved-empty", text: t("No saved conversations yet.") })]),
    );
  }

  /** Zeca's head and name over a reply, with the engine that wrote it. */
  private signature(): HTMLElement {
    const face = el("img", { class: "msg-face" });
    face.alt = "";
    face.src = faceImage();
    return el("div", { class: "msg-who" }, face, el("span", { class: "msg-name", text: "Zeca" }), el("span", { class: "msg-engine", text: this.name(this.provider) }));
  }

  /** Copies a whole reply, as Markdown: what was written, not how it is drawn. */
  private copyButton(text: string): HTMLElement {
    const button = copyTextButton(text, "msg-tool", t("Copy reply"));
    button.prepend(icon("copy", 12, 2));
    return button;
  }

  /** The paperclip: the file chooser, the files then swallowed as a drop's. */
  private attachButton(): HTMLElement {
    const b = el("button", { class: "attach", onclick: () => this.backend.pickFiles?.(t("Attach a file")) }, icon("clip", 16, 1.8), this.attachLabel);
    b.title = t("Attach a file");
    b.setAttribute("aria-label", t("Attach a file"));
    return b;
  }

  private nestButton(): HTMLElement {
    const b = el("button", { class: "ghost history", onclick: () => this.backend.openNest?.() });
    b.append(icon("openOut", 14, 2));
    b.title = t("Open in the Nest");
    return b;
  }

  /** The picks for the folder now chosen, read once per folder. */
  private loadPicks(): void {
    const folder = this.folder;
    if (this.picksFor === folder) return;
    this.picksFor = folder;
    this.picks = {};
    const seq = ++this.picksSeq;
    void this.backend.pick(folder).then(
      (picks) => {
        if (seq !== this.picksSeq) return;
        this.picks = picks;
        this.paintHead();
        this.changed();
      },
      () => {},
    );
  }

  /** The provider's models, asked once (Codex asks the codex the user logged into). */
  private loadModels(provider: ChatProvider): void {
    if (provider === "api" || this.models[provider]) return;
    this.models[provider] = [];
    this.modelsFailed[provider] = false;
    void this.backend.models(provider).then(
      (models) => {
        this.models[provider] = models;
        this.paintHead();
        this.changed();
      },
      () => {
        // Asked again next time the picker opens.
        this.modelsFailed[provider] = true;
        this.paintHead();
        this.changed();
      },
    );
  }

  private choose(pick: ChatPick): void {
    const before = this.picks;
    const provider = this.provider;
    this.picks = { ...this.picks, [provider]: pick };
    // A read still on its way has the old picks.
    this.picksSeq++;
    this.backend.setPick(this.folder, provider, pick).catch(() => {
      // Not saved: the turn would run without it, so the picker says so too.
      if (this.picks[provider] === pick) this.picks = before;
      this.paintHead();
      this.changed();
    });
    this.paintHead();
    this.changed();
  }

  private paintModel(api: boolean): void {
    if (this.provider === "claude" || this.modelOpen) this.loadModels(this.provider);
    const pick = this.picks[this.provider] ?? {};
    const choices = this.models[this.provider] ?? [];
    const current = choices.find((m) => m.id === pick.model);
    const model = pick.model ? (current?.label ?? pick.model) : t("Default model");
    const label = pick.effort ? `${model} · ${t(EFFORTS[pick.effort] ?? pick.effort)}` : model;
    {
      // One control retains the engine, model and effort; Quick stacks its labels to save width.
      this.modelButton.classList.add("engine");
      this.modelButton.replaceChildren(
        el("span", { class: `engine-dot ${this.agent()}` }),
        el("span", { class: "engine-labels" },
          el("span", { class: "engine-name", text: this.engineName(this.provider) }),
          ...(api ? [] : [el("span", { class: "engine-model", text: label })]),
        ),
        this.planUsage,
        icon("chevronDown", 11, 2.2),
      );
      this.selectionLabel = api ? this.engineName(this.provider) : t("{engine}, {model}", { engine: this.engineName(this.provider), model: label });
      this.modelButton.setAttribute("aria-expanded", String(this.modelOpen));
    }
    this.paintUsage();
    this.modelMenu.classList.toggle("on", this.modelOpen);
    if (!this.modelOpen) return;
    // The engines first, as the prototype groups them: the user's own CLIs, then Vults' harness;
    // each says how it stands. Switching starts a new conversation, as the old switch did.
    const row = (p: ChatProvider) => {
      const state = p === "api" ? null : this.engines?.[p];
      // The API row is ready with a key or a local model; once that is gone it says nothing.
      const word = p === "api" ? (this.api.ready ? t("Ready") : null) : state ? t(ENGINE_STATE[state]) : null;
      const b = el(
        "button",
        {
          class: `menu-item engine-row${p === this.provider ? " on" : ""}`,
          onclick: () => {
            this.modelOpen = false;
            // Mid-reply nothing switches (ask waits for the turn): the menu still closes.
            if (p !== this.provider) this.ask(p);
            this.paintHead();
            this.changed();
          },
        },
        el("span", { class: `engine-dot ${p === "api" ? "claude" : p}` }),
        el("span", { text: this.engineName(p) }),
        word ? el("span", { class: `engine-state${state === "missing" ? " off" : ""}`, text: word }) : null,
      );
      if (p === "api")
        b.title = t("{provider} through its API. It only talks: no commands, no edits. Change it in Settings → Chat & voice.", { provider: this.api.label });
      return this.choice(b, `engine:${p}`, p === this.provider);
    };
    const providers = this.providers();
    const harness = providers.filter((p) => p === "api");
    const settings = this.backend.openSettings;
    const engines = [
      el(
        "div",
        { class: "model-column" },
        el("span", { class: "menu-label", text: t("Your setup") }),
        el("span", { class: "menu-hint", text: t("Your CLI, with your CLAUDE.md, skills and MCPs") }),
        ...providers.filter((p) => p !== "api").map(row),
        ...(harness.length
          ? [el("span", { class: "menu-label", text: t("Vults harness") }), el("span", { class: "menu-hint", text: t("Vults runs the loop with your key") }), ...harness.map(row)]
          : []),
        settings
          ? el("button", {
              class: "menu-item engine-settings",
              text: t("Engines in Settings ›"),
              onclick: () => {
                this.modelOpen = false;
                this.paintHead();
                this.changed();
                settings();
              },
            })
          : null,
      ),
    ];
    // A conversation keeps its model, and the API chat has none to pick: only the engines then.
    if (this.started || api) {
      if (this.compact && this.started) engines[0].append(el("p", { class: "model-menu-note", text: t("A conversation keeps its model; New starts another.") }));
      this.modelMenu.replaceChildren(...engines);
      return;
    }
    // A model that names its efforts offers only those. The CLI's default model could be any of
    // its list: only the efforts every one of them takes (all of them for Claude's aliases' default).
    const shared = choices.length
      ? Object.keys(EFFORTS).filter((e) => choices.every((m) => m.efforts.includes(e)))
      : Object.keys(EFFORTS);
    const efforts = current ? current.efforts : pick.model ? [] : this.provider === "claude" ? Object.keys(EFFORTS) : shared;
    const item = (text: string, on: boolean, next: ChatPick, key: string) =>
      this.choice(el("button", { class: `menu-item${on ? " on" : ""}`, text, onclick: () => this.choose(next) }), key, on);
    this.modelMenu.replaceChildren(
      ...engines,
      el(
        "div",
        { class: "model-column" },
        el("span", { class: "menu-label", text: t("Model") }),
        item(t("Default model"), !pick.model, { effort: pick.effort }, "model:default"),
        ...(this.modelsFailed[this.provider]
          ? [el("span", { class: "menu-label", text: t("{name} did not list its models.", { name: this.name(this.provider) }) })]
          : []),
        ...choices.map((m) =>
          item(m.label, m.id === pick.model, {
            model: m.id,
            effort: pick.effort && m.efforts.includes(pick.effort) ? pick.effort : null,
          }, `model:${m.id}`),
        ),
      ),
      el(
        "div",
        { class: "model-column" },
        el("span", { class: "menu-label", text: t("Effort") }),
        item(t("Default effort"), !pick.effort, { model: pick.model }, "effort:default"),
        ...efforts.map((e) => item(t(EFFORTS[e] ?? e), e === pick.effort, { model: pick.model, effort: e }, `effort:${e}`)),
      ),
    );
  }

  private paintAccess(api: boolean): void {
    const now = ACCESS.find((a) => a.id === this.access) ?? ACCESS[0];
    const rank = (id: ChatAccess) => ACCESS.findIndex((a) => a.id === id);
    // The API chat runs nothing: there is nothing to answer.
    this.accessButton.hidden = api;
    this.accessButton.classList.toggle("full", this.access === "full");
    this.accessButton.replaceChildren(el("span", { text: t(now.label) }), icon("chevronDown", 11, 2.2));
    this.accessButton.title = t("Who answers Zeca's cards in this chat.");
    this.accessMenu.classList.toggle("on", this.accessOpen && !api);
    if (!this.accessOpen) return;
    this.accessMenu.replaceChildren(
      el("span", { class: "menu-label", text: t("Who answers the cards") }),
      ...ACCESS.map((a) => {
        // More access is given in Settings, like a policy: from here, the item leads there.
        const more = rank(a.id) > rank(this.access);
        const button = el(
          "button",
          {
            class: `menu-item access-item${a.id === this.access ? " on" : ""}`,
            onclick: () => {
              this.accessOpen = false;
              if (more) this.backend.openSettings?.();
              else if (a.id !== this.access) {
                const before = this.access;
                this.access = a.id;
                void this.backend.setAccess?.(a.id).catch(() => this.setAccess(before));
              }
              this.paintHead();
              this.changed();
            },
          },
          el("span", { class: "access-name", text: t(a.label) }),
          el("span", { class: "access-about", text: t(a.about) }),
          ...(more ? [el("span", { class: "access-more", text: t("Choose it in Settings →") })] : []),
        );
        return this.choice(button, `access:${a.id}`, a.id === this.access);
      }),
    );
  }

  private menuPanels(): { id: string; menu: HTMLElement; trigger: HTMLElement; name: string }[] {
    return [
      { id: "folder", menu: this.menu, trigger: this.place, name: t("Folder") },
      { id: "model", menu: this.modelMenu, trigger: this.modelButton, name: t("Model") },
      { id: "access", menu: this.accessMenu, trigger: this.accessButton, name: t("Who answers the cards") },
      { id: "history", menu: this.historyMenu, trigger: this.historyButton, name: t("Conversations") },
    ];
  }

  private closeMenus(returnFocus = false): boolean {
    if (this.mentionMenu.classList.contains("on")) {
      this.closeMentions();
      if (returnFocus) this.input.focus();
      return true;
    }
    const open = this.menuPanels().find(({ menu }) => menu.classList.contains("on"));
    if (!open) return false;
    this.menuOpen = this.modelOpen = this.accessOpen = this.historyOpen = false;
    this.renaming = this.deleting = null;
    this.paintHead();
    if (returnFocus) open.trigger.focus();
    this.changed();
    return true;
  }

  private choice<T extends HTMLElement>(button: T, key: string, selected?: boolean): T {
    button.dataset.choice = key;
    if (selected !== undefined) button.setAttribute("aria-pressed", String(selected));
    return button;
  }

  private paintHead(): void {
    if (this.menuOpen || this.modelOpen || this.accessOpen || this.historyOpen) this.closeMentions();
    const active = document.activeElement as HTMLInputElement | null;
    const panel = active ? this.menuPanels().find(({ menu }) => menu.contains(active)) : null;
    const choice = panel ? active?.dataset.choice : null;
    const draft = panel && active?.tagName === "INPUT" ? { value: active.value, start: active.selectionStart, end: active.selectionEnd } : null;
    this.loadPicks();
    // The API chat has no tools, so it works nowhere in particular.
    const api = this.provider === "api";
    this.element.classList.toggle("chat-api", api);
    this.place.hidden = api;
    this.setup.hidden = api || !this.native;
    this.paintModel(api);
    this.paintAccess(api);
    this.place.classList.toggle("fixed", this.started);
    this.place.replaceChildren(
      el("span", {
        text: this.folder ? t("in {folder}", { folder: folderName(this.folder) }) : t("in an empty folder"),
      }),
      ...(this.started ? [] : [icon("chevronDown", 11, 2.2)]),
    );
    this.place.title = this.started
      ? t("{folder}. A conversation stays in its folder; New starts another.", { folder: this.folder ?? t("An empty folder") })
      : (this.folder ?? "A folder of its own, empty");
    const choices: (string | null)[] = [
      ...new Set(this.folders.map((f) => f.cwd)),
      null,
    ];
    this.menu.classList.toggle("on", this.menuOpen && !this.started && !api);
    this.menu.replaceChildren(
      ...choices.map((cwd) => {
        const button = el("button", {
          class: `menu-item${cwd === this.folder ? " on" : ""}`,
          text: cwd ? folderName(cwd) : t("An empty folder"),
          onclick: () => {
            this.folder = cwd;
            this.keepDraft();
            this.picked = true;
            this.menuOpen = false;
            this.paintHead();
            this.changed();
          },
        });
        button.title = cwd ?? t("An empty folder");
        return this.choice(button, `folder:${cwd ?? "empty"}`, cwd === this.folder);
      }),
    );
    this.paintHistory();
    for (const { id, menu, trigger, name } of this.menuPanels()) {
      trigger.setAttribute("aria-expanded", String(menu.classList.contains("on")));
      menu.setAttribute("aria-label", name);
      for (const control of menu.querySelectorAll<HTMLElement>("button, input")) control.dataset.choice ??= `${id}:${control.textContent}`;
    }
    if (choice && panel?.menu.classList.contains("on")) {
      const next = [...panel.menu.querySelectorAll<HTMLElement>("button, input")].find(control => control.dataset.choice === choice);
      if (next) {
        if (draft) {
          const input = next as HTMLInputElement;
          input.value = draft.value;
          if (draft.start !== null && draft.end !== null) input.setSelectionRange(draft.start, draft.end);
        }
        next.focus({ preventScroll: true });
      } else panel.trigger.focus();
    } else if (choice && panel && !panel.trigger.hidden) panel.trigger.focus();
  }

  private paintSend(): void {
    const stop = this.busy;
    this.send.classList.toggle("stop", stop);
    this.send.replaceChildren(
      stop ? icon("stop", 12, 2.4) : icon("chevron", 14, 2.4),
    );
    this.send.title = stop ? t("Stop") : t("Send (Enter)");
    this.send.setAttribute("aria-label", this.send.title);
    this.send.toggleAttribute(
      "disabled",
      !stop && !this.input.value.trim() && !this.files.length,
    );
  }

  /** The thinking line: its dots, and how long the turn has run once that is worth saying. */
  private paintThinking(on: boolean): void {
    if (!on) {
      window.clearInterval(this.ticking);
      this.ticking = undefined;
      this.thinking = null;
      return;
    }
    const words = el("span", { class: "typing-words" });
    const tick = () => {
      const ms = Clock.now() - this.turnSince;
      words.textContent = ms < SHOW_ELAPSED_MS ? (this.searching ? t("Using tools…") : t("Working…")) : t("Working… {seconds} s", { seconds: Math.floor(ms / 1000) });
    };
    tick();
    this.thinking = el("div", { class: "typing" }, el("span", { class: "typing-dots" }, el("i", {}), el("i", {}), el("i", {})), words);
    window.clearInterval(this.ticking);
    // Only its own words change: the island is not rendered again for a second going by.
    this.ticking = window.setInterval(tick, 1000);
  }

  private paint(): void {
    this.element.classList.toggle("has-messages", this.messages.length > 0 || this.busy);
    this.paintOffers();
    this.paintHead();
    this.paintVoice();
    // While a turn runs and nothing waits on the user: before the first word, and between a
    // reply's words and its tools, when the CLI can go quiet for a while.
    const waiting = this.busy && !this.pendingAsk();
    this.paintThinking(waiting);
    this.paintApprovalQueue();
    const focused = document.activeElement && this.log.contains(document.activeElement) ? document.activeElement as HTMLElement : null;
    this.nodes = this.messages.map((m) => this.node(m));
    this.log.replaceChildren(
      ...(this.messages.length
        ? this.nodes
        : [
            ...(this.compact ? (this.provider === "api" ? [el("div", { class: "hint" }, this.apiNotice)] : []) : [el("div", { class: "hint" }, this.intro, this.apiNotice)]),
            this.suggestions,
          ]),
      ...(waiting && this.thinking ? [this.thinking] : []),
    );
    if (focused?.isConnected) focused.focus({ preventScroll: true });
    for (const path of this.attachmentNodes.keys()) if (!this.files.includes(path)) this.attachmentNodes.delete(path);
    const attachments = this.files.map((file) => {
      const kept = this.attachmentNodes.get(file);
      if (kept) {
        kept.querySelector("button")?.setAttribute("aria-label", t("Remove {name}", { name: shortName(file) }));
        const kind = kept.querySelector("small");
        if (kind && !kind.dataset.extension) kind.textContent = t("File");
        return kept;
      }
      const remove = el("button", { class: "ghost", onclick: () => {
        this.files = this.files.filter(path => path !== file);
        this.keepDraft(); this.paint(); this.changed();
      } }, icon("close", 14));
      remove.setAttribute("aria-label", t("Remove {name}", { name: shortName(file) }));
      const name = shortName(file);
      const extension = name.match(/^.+\.([a-z\d]{1,10})$/i)?.[1]?.toUpperCase();
      const kind = el("small", { text: extension ?? t("File") });
      if (extension) kind.dataset.extension = extension;
      const chip = el("div", { class: "chip attachment-enter" },
        el("span", { class: "file-icon" }, icon("file", 20)),
        el("span", { class: "file-info" }, el("span", { text: name }), kind),
        remove,
      );
      chip.title = file;
      this.attachmentNodes.set(file, chip);
      return chip;
    });
    if (attachments.length !== this.chips.children.length || attachments.some((node, i) => this.chips.children[i] !== node)) {
      this.chips.replaceChildren(...attachments);
    }
    this.paintSend();
  }
}

/** The drop zone's border: a dashed rounded rectangle that marches. */
function dashes(): SVGSVGElement {
  const ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("class", "drop-dashes");
  svg.setAttribute("aria-hidden", "true");
  const rect = document.createElementNS(ns, "rect");
  // Its size comes from the CSS (calc() is no SVG attribute).
  for (const [k, v] of Object.entries({ x: "1", y: "1", rx: "16" }))
    rect.setAttribute(k, v);
  svg.append(rect);
  return svg;
}

/** Inbox copies are named `<millis>-<original name>`. */
const shortName = (path: string) =>
  (path.split(/[\\/]/).pop() ?? path).replace(/^\d+-/, "");

/** Zeca's flight frames side by side (wings up, gliding, down), drawn once, for the carrying bird. */
let strip: string | null = null;
function flightStrip(): string {
  if (strip) return strip;
  const parts = ["fly_up", "glide", "fly_down"];
  const w = 37;
  const h = 17;
  const canvas = document.createElement("canvas");
  canvas.width = w * parts.length;
  canvas.height = h;
  const ctx = canvas.getContext("2d")!;
  parts.forEach((part, i) => drawFrame(ctx, ZECA, { ms: 0, dx: 0, dy: 0, layers: [[part, 0, 0]] }, i * w, 0, 1));
  strip = canvas.toDataURL();
  return strip;
}

/** Zeca's head, cut from his resting frame, drawn once for the replies' signature. */
let face: string | null = null;
function faceImage(): string {
  if (face) return face;
  const frame = ZECA.clips.idle?.frames[0] ?? Object.values(ZECA.clips)[0].frames[0];
  const w = frameWidth(ZECA, frame);
  const h = Math.max(...frame.layers.map(([p, , y]) => y + ZECA.parts[p].length));
  const canvas = document.createElement("canvas");
  canvas.width = w;
  canvas.height = h;
  drawFrame(canvas.getContext("2d")!, ZECA, { ...frame, dx: 0, dy: 0 }, 0, 0, 1);
  face = canvas.toDataURL();
  return face;
}
