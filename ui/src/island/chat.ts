// The chat panel: Zeca on the left (his perch comes in from the island), the conversation on the
// right. Its DOM is built once and updated in place, so the input keeps its focus and text while
// session views re-render the island around it, and a streaming reply only touches its own
// message. No Tauri here: the backend comes in.
import { Clock } from "../clock";
import { drawFrame, frameWidth } from "../character/sprites";
import { ZECA } from "../character/zeca";
import type { AgentKind, ApiStatus, ChatAccess, ChatEngines, ChatAnswer, ChatDelta, ChatPick, ChatProvider, ChatSync, ConversationView, ModelChoice, OpenedConversation } from "../bridge";
import { el } from "../dom";
import { Sound } from "../sound";
import { icon } from "./icons";
import { renderLite } from "./markdown";
import { t, tk } from "../i18n";

export interface ChatBackend {
  send(text: string, files: string[], folder: string | null): Promise<void>;
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
  /** The paperclip: the system's file chooser, titled in the user's language; what it picks comes
   *  back as a drop does. */
  pickFiles?: (title: string) => void;
  /** `/remember` and `/forget` (M6): what the memory says back, or null for any other text. */
  memory?: (text: string, folder: string | null) => Promise<MemorySaid | null>;
  /** `/task` (W3): what the app says back once the task is kept, or null for any other text. */
  task?: (text: string, folder: string | null) => Promise<MemorySaid | null>;
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
  // `/task` (app/src/tasks.rs).
  tk("Say what the task is: /task, its goal, then a line for each criterion starting with -."),
  tk("Task added, with no criteria yet:"),
  tk("Task added, with 1 criterion:"),
  tk("Task added, with its criteria:"),
  // `/delegate` (app/src/tasks.rs).
  tk("Delegate from a conversation in a repository: a task works in one."),
  tk("Say which agent: /delegate default claude, or /delegate default codex."),
  tk("From now on, this project's delegated tasks start on:"),
  tk("This project has no rule now: its delegated tasks start on Claude Code."),
  tk("Say what to delegate: /delegate, its goal, then a line for each criterion starting with -."),
  tk("The task is in the Nest, but it didn't start:"),
  tk("Delegated by this project's rule to:"),
  tk("Delegated to the default agent:"),
  tk("Delegated away from a plan above 85% to:"),
  tk("Delegated to the agent that finished most tasks like it here:"),
  // `/continue` (app/src/handoff.rs, runs.rs).
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
  tk("That session is a task's: go on with it in the Nest's Tasks tab."),
  tk("That session couldn't be resumed:"),
  tk("It is still starting: try again in a moment."),
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
};

type Message =
  | { who: "you"; text: string; files: string[] }
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

export class ChatPanel {
  readonly element = el("section", { class: "chat" });
  /** Zeca's place: the island moves his perch in here while the chat shows. */
  readonly perchSlot = el("div", { class: "chat-perch" });
  private open = false;
  private provider: ChatProvider = "claude";
  /** The API chat: offered only while it can be used (a key saved, or a local model). */
  private api: ApiStatus = { ready: false, label: t("API") };
  /** How each CLI engine stands, asked each time the menu opens; none until it answers. */
  private engines: ChatEngines | null = null;
  private enginesAsked = 0;
  private messages: Message[] = [];
  /** The rendered messages, one node per message. */
  private nodes: HTMLElement[] = [];
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
  private readonly jump = el("button", {
    class: "jump-end",
    text: t("↓ new text"),
    onclick: () => this.toEnd(),
  });
  private readonly chips = el("div", { class: "chips" });
  /** The Nest has the room to say how the composer's keys work. */
  private readonly composerHint = el("p", { class: "composer-hint" });
  private readonly input = document.createElement("textarea");
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
    this.setup.setAttribute("role", "img");
    this.input.addEventListener("keydown", (e) => {
      // Escape is the island's: it closes the chat from anywhere.
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void this.submit();
      } else if (e.key === "ArrowUp" && !this.input.value && this.recall()) e.preventDefault();
    });
    this.input.addEventListener("input", () => {
      this.grow();
      this.paintSend();
    });
    this.place.addEventListener("click", () => {
      if (this.started || this.provider === "api") return;
      this.menuOpen = !this.menuOpen;
      this.modelOpen = this.accessOpen = false;
      this.paintHead();
      this.changed();
    });
    this.historyButton.append(icon("list", 14, 2));
    this.historyButton.addEventListener("click", () => {
      if (this.busy) return;
      this.historyOpen = !this.historyOpen;
      this.menuOpen = this.modelOpen = false;
      this.renaming = this.deleting = null;
      if (this.historyOpen) void this.loadSaved();
      this.paintHead();
      this.changed();
    });
    this.accessButton.addEventListener("click", () => {
      if (this.provider === "api") return;
      this.accessOpen = !this.accessOpen;
      this.menuOpen = this.modelOpen = false;
      this.paintHead();
      this.changed();
    });
    this.modelButton.addEventListener("click", () => {
      // It is the engine's pill too, in both windows: the engine can change whatever was said.
      this.modelOpen = !this.modelOpen;
      this.menuOpen = this.accessOpen = false;
      // Started (or the API chat), the island's menu shows only the engines: no list to ask for.
      if (this.modelOpen && !this.started) this.loadModels(this.provider);
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
    this.log.addEventListener("scroll", () => {
      if (this.atEnd()) this.jump.classList.remove("on");
    });
    // On the island the chat takes the prototype's compact form: the engine and the conversations
    // up top, the folder and the model under the composer, a paperclip for a file.
    const compact = this.compact;
    const head = compact
      ? el(
          "div",
          { class: "chat-head" },
          el("div", { class: "model-box engine-box" }, this.modelButton, this.modelMenu),
          el("span", { class: "chat-gap" }),
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
      el("div", { class: "log-box" }, this.log, this.jump),
      this.chips,
      compact
        ? el("div", { class: "composer" }, this.attachButton(), this.mic, this.input, this.wave, this.send)
        : el("div", { class: "composer" }, this.input, this.wave, this.attachButton(), this.mic, this.send),
      compact ? null : this.composerHint,
      compact ? el("div", { class: "chat-foot" }, el("div", { class: "place-box" }, this.place, this.menu), el("div", { class: "access-box" }, this.accessButton, this.accessMenu), this.setup) : null,
    );
    this.drop.prepend(dashes());
    this.element.append(this.perchSlot, main, this.drop);
    this.relabel();
  }

  /** Its words again: the language is known now, or changed. The panel is built before the app
   *  says which one, so the first call comes from there too. */
  relabel(): void {
    this.input.placeholder = t("Ask Zeca…");
    this.composerHint.textContent = t("Enter sends, Shift+Enter starts a new line, and ↑ brings back your last message.");
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
    if (!this.started && !this.picked && front !== this.folder) {
      this.folder = front;
      this.paintHead();
    } else if (changed) this.paintHead();
  }

  /** What Zeca does while the chat is open. */
  clip(now: number): string {
    if (now < this.swallowUntil) return "swallow";
    if (this.dragOver) return "gape";
    const last = this.messages[this.messages.length - 1];
    if (last?.who === "ask" && !last.answer) return "question";
    if (this.voice === "listening") return "listen";
    if (this.voice === "transcribing") return "think";
    if (this.busy && (!last || last.who === "you")) return "think";
    return "idle";
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
  }
  private enabled = true;

  toggle(open = !this.open): void {
    if (open && !this.enabled) return;
    if (open === this.open) return;
    this.open = open;
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
    const last = this.messages[this.messages.length - 1];
    const follow = this.atEnd();
    if (d.kind === "text") {
      if (last?.who === "zeca" && !last.error) {
        // Streaming: only the message being written changes. Tokens come many a second, and each
        // repaint re-reads its markdown and renders the island: at most one every STREAM_MS.
        last.text += d.text;
        if (this.streaming === undefined) {
          const i = this.messages.length - 1;
          this.streaming = window.setTimeout(() => {
            this.streaming = undefined;
            // Reset or rebuilt meanwhile: whatever painted last already shows it.
            if (this.messages[i] !== last) return;
            const follow = this.atEnd();
            this.replace(i);
            this.after(follow);
            this.changed();
          }, STREAM_MS);
        }
        return;
      }
      this.messages.push({ who: "zeca", text: d.text });
    } else if (d.kind === "permission") {
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
      // The API chat's own tools only read: a line says which one it used, no card.
      this.messages.push({ who: "note", text: t("Used {tool}", { tool: d.tool }) });
    } else if (d.kind === "settled") {
      // The user's choice answered with no card: the chat says what ran, and on whose word.
      const text =
        d.by === "full-access"
          ? t("Ran with full access: {target}", { target: d.target })
          : d.allow
            ? t("Allowed by your policy: {target}", { target: d.target })
            : t("Denied by your policy: {target}", { target: d.target });
      this.messages.push({ who: "note", text });
    } else if (d.kind === "allowed") {
      // The user's own rule for this conversation answered: the chat says what ran.
      this.messages.push({
        who: "note",
        text: t("Allowed for this conversation: {target}", { target: d.target }),
      });
    } else {
      // Only a turn's failure can be sent again; a mic that would not open has nothing to resend.
      const turn = this.busy;
      this.busy = false;
      if (d.kind === "error")
        this.messages.push({ who: "zeca", text: d.message, error: true, retry: turn });
      if (d.kind === "stopped")
        this.messages.push({ who: "note", text: t("Stopped.") });
      // Anything still waiting is a no now: it can't be answered any more.
      for (const m of this.messages)
        if (m.who === "ask" && !m.answer && d.kind !== "done")
          m.answer = "deny";
    }
    this.paint();
    this.after(follow);
    this.changed();
  }

  /** What the other window did (`chat-sync`), shown here too. */
  sync(s: ChatSync): void {
    if (s.from === this.opts.window) return;
    if (s.kind === "sent") {
      // A conversation that began in the other window: shown where and how it runs.
      if (!this.started) {
        this.provider = s.provider;
        this.folder = s.folder;
        this.picked = true;
        this.picksFor = s.folder;
        this.picksSeq++;
        this.picks = { [s.provider]: { model: s.model, effort: s.effort } };
      }
      this.messages.push({ who: "you", text: s.text, files: s.files });
      this.busy = true;
      this.turnSince = Clock.now();
      this.started = true;
      this.menuOpen = this.modelOpen = this.historyOpen = false;
      this.paint();
      this.toEnd();
      this.changed();
    } else if (s.kind === "reset") this.cleared(s.provider);
    else if (s.kind === "opened") this.load(s.conversation);
    else {
      const m = this.messages.find((m): m is Ask => m.who === "ask" && m.id === s.id);
      if (!m || m.answer) return;
      m.answer = s.answer;
      this.paint();
      this.changed();
    }
  }

  /** Answers the permission waiting in the chat, if one is; for the global shortcuts. */
  answerWaiting(allow: boolean): boolean {
    if (!this.open) return false;
    const waiting = [...this.messages]
      .reverse()
      .find((m): m is Ask => m.who === "ask" && !m.answer);
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
    this.grow();
    this.paintSend();
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
    this.files = [...new Set([...this.files, ...you.files])];
    void this.submit();
  }

  private async submit(): Promise<void> {
    const text = this.input.value.trim();
    // Typing ahead is fine while a reply streams; sending waits for it.
    if (this.busy || (!text && !this.files.length)) return;
    const files = this.files;
    // The user's own words for the memory or a task, kept by the app: never a turn, never sent to
    // a model. With files attached the command takes only the text, and the files wait for the next
    // message; where nothing keeps commands (the lab), nothing is sent.
    const memory = /^\/(remember|forget)(\s|$)/.test(text);
    const task = /^\/(task|delegate)(\s|$)/.test(text);
    const handoff = /^\/continue(\s|$)/.test(text);
    if (memory || task || handoff) {
      const command = memory ? this.backend.memory : task ? this.backend.task : this.backend.handoff;
      if (!command) return;
      const answer = await command(text, this.folder).catch((e: unknown) => ({ said: String(e), detail: null }));
      if (answer) {
        this.input.value = "";
        this.messages.push({ who: "you", text, files: [] });
        const said = MEMORY_SAID.has(answer.said) ? t(answer.said) : answer.said;
        this.messages.push({ who: "zeca", text: answer.detail ? `${said}\n${answer.detail}` : said });
        this.paint();
        this.toEnd();
        return;
      }
    }
    this.messages.push({ who: "you", text, files });
    this.files = [];
    this.input.value = "";
    this.input.style.height = "";
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
      await this.backend.send(text, files, this.folder);
    } catch (e) {
      this.receive({ kind: "error", message: String(e) });
    }
  }

  /** New, or another provider: the conversation so far stays saved, in the list. */
  private ask(what: ChatProvider | "new"): void {
    if (this.busy) return;
    void this.restart(what === "new" ? null : what);
  }

  private async restart(provider: ChatProvider | null): Promise<void> {
    this.cleared(await this.backend.reset(provider));
  }

  /** A new conversation with `provider`, here or (synced) in the other window. */
  private cleared(provider: ChatProvider): void {
    this.provider = provider;
    this.busy = false;
    this.historyOpen = false;
    // The folder's pick for new conversations, not the one just left.
    this.picksFor = undefined;
    this.messages = [];
    this.started = false;
    this.modelOpen = false;
    this.native = this.nativeSetting;
    this.picked = false;
    this.paint();
    this.changed();
  }

  private answer(m: Ask, answer: ChatAnswer): void {
    if (m.answer) return;
    m.answer = answer;
    Sound.play(answer === "deny" ? "deny" : "allow");
    this.backend.decide(m.id, answer);
    this.paint();
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
    this.log.scrollTop = this.log.scrollHeight;
    this.jump.classList.remove("on");
  }

  /** After the log changed: keep following the end, or say there is more below. */
  private after(follow: boolean): void {
    if (follow) this.toEnd();
    else this.jump.classList.add("on");
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
      b.append((this.opts.render ?? renderLite)(m.text), el("div", { class: "msg-tools" }, this.copyButton(m.text)));
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

  private askNode(m: Ask): HTMLElement {
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
    return el(
      "div",
      { class: `ask${m.answer ? ` ${m.answer === "always" ? "allow" : m.answer}` : ""}` },
      el(
        "div",
        { class: "who" },
        el("span", { class: "label", text: asks(m.tool) }),
        diff,
      ),
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
      el("pre", {
        class: "code",
        text: m.full ? `${m.tool} · ${m.full}` : m.target,
      }),
      m.answer
        ? el(
            "div",
            { class: `settled-label ${m.answer === "always" ? "allow" : m.answer}` },
            icon(m.answer === "deny" ? "close" : "check", 12, 2.6),
            el("span", { text: settled[m.answer] }),
          )
        : el(
            "div",
            { class: "actions" },
            button(t("Deny"), "secondary", "deny", this.keys.deny),
            button(t("Allow"), "primary", "allow", this.keys.allow),
            m.always ? always : null,
          ),
    );
  }

  /** Re-renders one message in place. */
  private replace(i: number): void {
    const next = this.node(this.messages[i]);
    this.nodes[i]?.replaceWith(next);
    this.nodes[i] = next;
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
      c = await this.backend.open(id);
    } catch (e) {
      this.historyOpen = false;
      this.messages.push({ who: "note", text: String(e) });
      this.paint();
      this.changed();
      return;
    }
    this.load(c);
  }

  /** Shows a saved conversation, as it was: opened here, in the other window, or the Nest's start. */
  load(c: OpenedConversation): void {
    this.historyOpen = false;
    this.busy = false;
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
    this.messages = c.lines.map((l): Message => {
      if (l.who === "you") return { who: "you", text: l.text, files: l.files };
      if (l.who === "error") return { who: "zeca", text: l.text, error: true };
      if (l.who === "stopped") return { who: "note", text: t("Stopped.") };
      return { who: "zeca", text: l.text };
    });
    // Opened mid-turn: the reply so far and the cards waiting; the rest streams in.
    if (c.busy) {
      this.busy = true;
      // Its start is not known here: counted from now.
      this.turnSince = Clock.now();
      if (c.partial) this.messages.push({ who: "zeca", text: c.partial });
      for (const w of c.waiting)
        this.messages.push({ who: "ask", id: w.id, tool: w.tool, target: w.target, description: null, full: null, added: 0, removed: 0, always: false, outside: w.outside });
    }
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
        input.value = c.title;
        input.addEventListener("keydown", (e) => {
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
      return el(
        "div",
        { class: `saved-row${c.current ? " on" : ""}` },
        el(
          "button",
          { class: "saved-open", onclick: () => void this.openSaved(c.id) },
          el("span", { class: "saved-title", text: c.title }),
          el("span", { class: "saved-where", text: where }),
        ),
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
    const label = el("span", { text: t("Copy") });
    const b = el("button", { class: "msg-tool" }, icon("copy", 12, 2), label);
    b.addEventListener("click", () => {
      void navigator.clipboard?.writeText(text).then(() => {
        label.textContent = t("Copied");
        window.setTimeout(() => (label.textContent = t("Copy")), 1200);
      }, () => {});
    });
    return b;
  }

  /** The paperclip: the file chooser, the files then swallowed as a drop's. */
  private attachButton(): HTMLElement {
    const b = el("button", { class: "attach", onclick: () => this.backend.pickFiles?.(t("Attach a file")) }, icon("clip", 15, 1.8));
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
        this.models[provider] = undefined;
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
      // The prototype's one pill, on the island and in the Nest: the engine in its color, then its
      // model; the menu holds all three.
      this.modelButton.classList.add("engine");
      this.modelButton.replaceChildren(
        el("span", { class: `engine-dot ${this.agent()}` }),
        el("span", { class: "engine-name", text: this.engineName(this.provider) }),
        ...(api ? [] : [el("span", { class: "engine-model", text: label })]),
        icon("chevronDown", 11, 2.2),
      );
      this.modelButton.setAttribute("aria-label", api ? this.engineName(this.provider) : t("{engine}, {model}", { engine: this.engineName(this.provider), model: label }));
    }
    this.modelButton.title = this.started
      ? t("A conversation keeps its model; New starts another.")
      : t("The model and effort for new conversations in this folder.");
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
      return b;
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
      this.modelMenu.replaceChildren(...engines);
      return;
    }
    // A model that names its efforts offers only those. The CLI's default model could be any of
    // its list: only the efforts every one of them takes (all of them for Claude's aliases' default).
    const shared = choices.length
      ? Object.keys(EFFORTS).filter((e) => choices.every((m) => m.efforts.includes(e)))
      : Object.keys(EFFORTS);
    const efforts = current ? current.efforts : pick.model ? [] : this.provider === "claude" ? Object.keys(EFFORTS) : shared;
    const item = (text: string, on: boolean, next: ChatPick) =>
      el("button", { class: `menu-item${on ? " on" : ""}`, text, onclick: () => this.choose(next) });
    this.modelMenu.replaceChildren(
      ...engines,
      el(
        "div",
        { class: "model-column" },
        el("span", { class: "menu-label", text: t("Model") }),
        item(t("Default model"), !pick.model, { effort: pick.effort }),
        ...(this.modelsFailed[this.provider]
          ? [el("span", { class: "menu-label", text: t("{name} did not list its models.", { name: this.name(this.provider) }) })]
          : []),
        ...choices.map((m) =>
          item(m.label, m.id === pick.model, {
            model: m.id,
            effort: pick.effort && m.efforts.includes(pick.effort) ? pick.effort : null,
          }),
        ),
      ),
      el(
        "div",
        { class: "model-column" },
        el("span", { class: "menu-label", text: t("Effort") }),
        item(t("Default effort"), !pick.effort, { model: pick.model }),
        ...efforts.map((e) => item(t(EFFORTS[e] ?? e), e === pick.effort, { model: pick.model, effort: e })),
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
        return el(
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
      }),
    );
  }

  private paintHead(): void {
    this.loadPicks();
    // The API chat has no tools, so it works nowhere in particular.
    const api = this.provider === "api";
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
      ...choices.map((cwd) =>
        el("button", {
          class: `menu-item${cwd === this.folder ? " on" : ""}`,
          text: cwd ? folderName(cwd) : t("An empty folder"),
          onclick: () => {
            this.folder = cwd;
            this.picked = true;
            this.menuOpen = false;
            this.paintHead();
            this.changed();
          },
        }),
      ),
    );
    this.paintHistory();
  }

  private paintSend(): void {
    const stop = this.busy;
    this.send.classList.toggle("stop", stop);
    this.send.replaceChildren(
      stop ? icon("stop", 12, 2.4) : icon("chevron", 14, 2.4),
    );
    this.send.title = stop ? t("Stop") : t("Send (Enter)");
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
      words.textContent = ms < SHOW_ELAPSED_MS ? t("Thinking…") : t("Thinking… {seconds} s", { seconds: Math.floor(ms / 1000) });
    };
    tick();
    this.thinking = el("div", { class: "typing" }, el("span", { class: "typing-dots" }, el("i", {}), el("i", {}), el("i", {})), words);
    window.clearInterval(this.ticking);
    // Only its own words change: the island is not rendered again for a second going by.
    this.ticking = window.setInterval(tick, 1000);
  }

  private paint(): void {
    this.paintHead();
    this.paintVoice();
    const last = this.messages[this.messages.length - 1];
    // While a turn runs and nothing waits on the user: before the first word, and between a
    // reply's words and its tools, when the CLI can go quiet for a while.
    const waiting = this.busy && !(last?.who === "ask" && !last.answer);
    this.paintThinking(waiting);
    this.nodes = this.messages.map((m) => this.node(m));
    this.log.replaceChildren(
      ...(this.messages.length
        ? this.nodes
        : [
            el("p", {
              class: "hint",
              text:
                this.provider === "api"
                  ? t("Ask anything, or drop a file on the island. This chat only talks: it can't run commands or open your project.")
                  : t("Ask anything, or drop a file on the island."),
            }),
            el(
              "div",
              { class: "suggestions" },
              ...(this.provider === "api" ? [] : SUGGESTIONS).map((q) =>
                el("button", {
                  class: "suggestion",
                  text: t(q),
                  onclick: () => {
                    this.input.value = t(q);
                    void this.submit();
                  },
                }),
              ),
            ),
          ]),
      ...(waiting && this.thinking ? [this.thinking] : []),
    );
    this.chips.replaceChildren(
      ...this.files.map((f) =>
        el(
          "span",
          { class: "chip" },
          icon("file", 11),
          el("span", { text: shortName(f) }),
          el("button", {
            class: "ghost",
            text: "×",
            onclick: () => {
              this.files = this.files.filter((x) => x !== f);
              this.paint();
              this.changed();
            },
          }),
        ),
      ),
    );
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
