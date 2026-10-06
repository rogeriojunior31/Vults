// The chat panel: Zeca on the left (his perch comes in from the island), the conversation on the
// right. Its DOM is built once and updated in place, so the input keeps its focus and text while
// session views re-render the island around it, and a streaming reply only touches its own
// message. No Tauri here: the backend comes in.
import { Clock } from "../clock";
import { drawFrame } from "../character/sprites";
import { ZECA } from "../character/zeca";
import type { AgentKind, ApiStatus, ChatDelta, ChatProvider } from "../bridge";
import { el } from "../dom";
import { Sound } from "../sound";
import { icon } from "./icons";
import { renderLite } from "./markdown";

export interface ChatBackend {
  send(text: string, files: string[], folder: string | null): Promise<void>;
  decide(id: string, allow: boolean): void;
  /** Stops the turn running now. */
  stop(): void;
  reset(provider: ChatProvider | null): Promise<ChatProvider>;
  keyboard(on: boolean): void;
  /** Speech to text; absent while no engine is set up, and then there is no mic button. */
  voice?: VoiceBackend;
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
  answer?: "allow" | "deny";
};

type Message =
  | { who: "you"; text: string; files: string[] }
  | { who: "zeca"; text: string; error?: boolean }
  | { who: "note"; text: string }
  | Ask;

/** How long Zeca takes to swallow a dropped file. */
const SWALLOW_MS = 1500;
/** How long a vulture takes to carry a dropped file across the drop zone. */
const CARRY_MS = 1300;
/** Ways to start, offered while the conversation is empty. */
const SUGGESTIONS = [
  "What is this project?",
  "What changed recently?",
  "Explain the last error",
];
/** The input grows up to this many lines, then scrolls. */
const MAX_INPUT_LINES = 6;
/** Closer than this to the end of the log counts as reading the end: new text keeps it in view. */
const FOLLOW_PX = 28;
const REFUSED: Record<Refused["reason"], (name: string) => string> = {
  folder: (n) => `${n} is a folder. Drop the files inside it instead.`,
  "too-big": (n) => `${n} is over 20 MB, too big to read whole.`,
  unreadable: (n) => `${n} couldn't be read.`,
};

/** What a permission asks for, in a few words. */
function asks(tool: string): string {
  if (tool === "Bash") return "Zeca wants to run";
  if (tool === "Edit" || tool === "Write" || tool === "MultiEdit")
    return "Zeca wants to change";
  return `Zeca wants to use ${tool}`;
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
  private api: ApiStatus = { ready: false, label: "API" };
  private messages: Message[] = [];
  /** The rendered messages, one node per message. */
  private nodes: HTMLElement[] = [];
  private files: string[] = [];
  private busy = false;
  /** Where the conversation works, fixed once it starts. */
  private folder: string | null = null;
  /** The user chose the folder; otherwise it follows the session in front. */
  private picked = false;
  private folders: Folder[] = [];
  private started = false;
  private swallowUntil = 0;
  private dragOver = false;
  /** The drop zone shown from the + tab, until a file comes or it is dismissed. */
  private dropHint = false;
  /** Asking before a new conversation clears this one: what was asked for. */
  private confirming: ChatProvider | "new" | null = null;
  private menuOpen = false;
  private keys: Record<string, string> = {};
  private readonly picker = el("div", { class: "segmented" });
  private readonly place = el("button", { class: "place" });
  private readonly menu = el("div", { class: "menu" });
  private readonly confirm = el("div", { class: "confirm" });
  private readonly log = el("div", { class: "log" });
  private readonly jump = el("button", {
    class: "jump-end",
    text: "↓ new text",
    onclick: () => this.toEnd(),
  });
  private readonly chips = el("div", { class: "chips" });
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
  private readonly mic = el("button", {
    class: "mic",
    onclick: () => void this.toggleVoice(),
  });
  private readonly wave = el("div", { class: "wave" });
  /** Files just dropped: carried across the drop zone, then ready to be asked about. */
  private carried: { paths: string[]; ready: boolean } | null = null;
  private readonly dropBody = el("div", { class: "drop-body" });
  private readonly drop = el("div", { class: "drop-zone" }, this.dropBody);

  constructor(
    private readonly backend: ChatBackend,
    private readonly changed: () => void,
  ) {
    this.input.rows = 1;
    this.input.placeholder = "Ask Zeca…";
    this.input.addEventListener("keydown", (e) => {
      // Escape is the island's: it closes the chat from anywhere.
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void this.submit();
      }
    });
    this.input.addEventListener("input", () => {
      this.grow();
      this.paintSend();
    });
    this.place.addEventListener("click", () => {
      if (this.started || this.provider === "api") return;
      this.menuOpen = !this.menuOpen;
      this.paintHead();
      this.changed();
    });
    this.log.addEventListener("scroll", () => {
      if (this.atEnd()) this.jump.classList.remove("on");
    });
    const head = el(
      "div",
      { class: "chat-head" },
      this.picker,
      el("div", { class: "place-box" }, this.place, this.menu),
      el("button", {
        class: "ghost",
        text: "New",
        onclick: () => this.ask("new"),
      }),
      el("button", {
        class: "ghost",
        text: "×",
        onclick: () => this.toggle(false),
      }),
    );
    const main = el(
      "div",
      { class: "chat-main" },
      head,
      this.confirm,
      el("div", { class: "log-box" }, this.log, this.jump),
      this.chips,
      el("div", { class: "composer" }, this.input, this.wave, this.mic, this.send),
    );
    this.drop.prepend(dashes());
    this.drop.addEventListener("click", () => this.hideDrop());
    this.element.append(this.perchSlot, main, this.drop);
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

  setApi(api: ApiStatus): void {
    if (api.ready === this.api.ready && api.label === this.api.label) return;
    const other = api.label !== this.api.label;
    this.api = api;
    // Another provider has none of this conversation: start over rather than pretend.
    if (other && this.provider === "api" && this.messages.length) {
      void this.restart("api");
      return;
    }
    this.paintHead();
    this.changed();
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
    if (this.dragOver || this.dropHint) return "gape";
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

  /** Lab only: shows a voice state without a backend round trip. */
  showVoice(state: Voice, levels?: number[]): void {
    this.voice = state;
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
        this.grow();
      }
    } catch (e) {
      this.receive({ kind: "error", message: String(e) });
    }
    this.showVoice("off");
    this.paintSend();
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
    this.mic.title = this.voice === "listening" ? "Stop and transcribe" : "Speak";
    this.mic.toggleAttribute("disabled", this.voice === "transcribing");
    this.input.hidden = on;
    this.wave.hidden = !on;
    this.wave.classList.toggle("busy", this.voice === "transcribing");
    this.wave.replaceChildren(
      ...this.levels.map((l) => {
        const bar = el("span");
        bar.style.height = `${Math.round(8 + l * 92)}%`;
        return bar;
      }),
      el("em", {
        text: this.voice === "transcribing" ? "Transcribing…" : "Listening…",
      }),
    );
  }

  /** Zeca on or off: off, the chat closes and nothing opens it (ADR 0010). */
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
    if (!open) this.dropHint = false;
    this.paintDrop();
    this.menuOpen = false;
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

  /** The + tab: the drop zone, saying how to give Zeca a file. */
  showDrop(): void {
    this.dropHint = true;
    this.toggle(true);
    this.paintDrop();
    this.changed();
  }

  hideDrop(): void {
    if (!this.dropHint) return;
    this.dropHint = false;
    this.paintDrop();
    this.changed();
  }

  isShowingDrop(): boolean {
    return this.dragOver || this.dropHint;
  }

  /** The drop zone in its phase: waiting for a file, carrying it in, or asking what it is for. */
  private paintDrop(): void {
    const c = this.carried;
    const phase = this.dragOver ? "waiting" : c ? (c.ready ? "ready" : "carrying") : this.dropHint ? "waiting" : null;
    this.drop.classList.toggle("on", phase !== null);
    // Shown from the tab (nothing dragged yet): it explains, and a click puts it away.
    this.drop.classList.toggle("hint", phase === "waiting" && this.dropHint && !this.dragOver);
    this.drop.classList.toggle("ready", phase === "ready");
    if (this.drop.dataset.phase === `${phase}|${c?.paths.length ?? 0}`) return;
    this.drop.dataset.phase = `${phase}|${c?.paths.length ?? 0}`;
    const label = c ? (c.paths.length === 1 ? shortName(c.paths[0]) : `${c.paths.length} files`) : "";
    if (phase === "carrying") {
      const bird = el("span", { class: "carry-bird" });
      bird.style.backgroundImage = `url(${flightStrip()})`;
      this.dropBody.replaceChildren(
        el("span", { class: "drop-title", text: `Taking ${label}` }),
        el("span", { class: "carry" }, bird, el("span", { class: "carry-track" }, el("span", { class: "carry-fill" }))),
      );
    } else if (phase === "ready") {
      const one = c!.paths.length === 1;
      this.dropBody.replaceChildren(
        el("span", { class: "drop-title", text: one ? `${label} is ready.` : `${label} are ready.` }),
        el("span", { class: "drop-sub", text: one ? "What do you want to do with it?" : "What do you want to do with them?" }),
        el(
          "span",
          { class: "actions" },
          el("button", { class: "btn primary", text: "Ask about it", onclick: () => this.keepDropped() }),
          el("button", { class: "btn secondary", text: "Cancel", onclick: () => this.dropDropped() }),
        ),
      );
    } else {
      this.dropBody.replaceChildren(
        el("span", { class: "drop-title", text: "Drop files here" }),
        el("span", { class: "drop-kinds" }, ...["PDF", "Images", "Code", "Text"].map((k) => el("span", { class: "kind", text: k }))),
        el("span", { class: "drop-how", text: "Drag one from your file manager onto the island" }),
      );
    }
  }

  /** Ask about it: the files stay on the next message, and the input takes the keyboard. */
  private keepDropped(): void {
    this.carried = null;
    this.paintDrop();
    this.changed();
    this.input.focus();
  }

  /** Cancel: the files just dropped come off the next message. */
  private dropDropped(): void {
    const gone = new Set(this.carried?.paths ?? []);
    this.files = this.files.filter((f) => !gone.has(f));
    this.carried = null;
    this.paint();
    this.paintDrop();
    this.changed();
  }

  attach(paths: string[], refused: Refused[] = []): void {
    this.dropHint = false;
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
      // Carried across the zone, then the question of what it is for.
      const carried = { paths: fresh, ready: false };
      this.carried = carried;
      this.paintDrop();
      window.setTimeout(() => {
        if (this.carried !== carried) return;
        carried.ready = true;
        Sound.play("alertOk");
        this.paintDrop();
        this.changed();
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
        // Streaming: only the message being written changes.
        last.text += d.text;
        this.replace(this.messages.length - 1);
        this.after(follow);
        this.changed();
        return;
      }
      this.messages.push({ who: "zeca", text: d.text });
    } else if (d.kind === "permission") {
      const { id, tool, target, description, full, added, removed } = d;
      this.messages.push({
        who: "ask",
        id,
        tool,
        target,
        description,
        full,
        added,
        removed,
      });
      Sound.play("approval");
    } else {
      this.busy = false;
      if (d.kind === "error")
        this.messages.push({ who: "zeca", text: d.message, error: true });
      if (d.kind === "stopped")
        this.messages.push({ who: "note", text: "Stopped." });
      // Anything still waiting is a no now: it can't be answered any more.
      for (const m of this.messages)
        if (m.who === "ask" && !m.answer && d.kind !== "done")
          m.answer = "deny";
    }
    this.paint();
    this.after(follow);
    this.changed();
  }

  /** Answers the permission waiting in the chat, if one is; for the global shortcuts. */
  answerWaiting(allow: boolean): boolean {
    if (!this.open) return false;
    const waiting = [...this.messages]
      .reverse()
      .find((m): m is Ask => m.who === "ask" && !m.answer);
    if (!waiting) return false;
    this.answer(waiting, allow);
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

  private async submit(): Promise<void> {
    const text = this.input.value.trim();
    // Typing ahead is fine while a reply streams; sending waits for it.
    if (this.busy || (!text && !this.files.length)) return;
    const files = this.files;
    this.messages.push({ who: "you", text, files });
    this.files = [];
    this.input.value = "";
    this.input.style.height = "";
    this.busy = true;
    this.started = true;
    this.menuOpen = false;
    this.paint();
    this.toEnd();
    this.changed();
    try {
      await this.backend.send(text, files, this.folder);
    } catch (e) {
      this.receive({ kind: "error", message: String(e) });
    }
  }

  /** New, or another provider: asks first when there is a conversation to lose. */
  private ask(what: ChatProvider | "new"): void {
    if (this.busy) return;
    if (this.messages.length === 0) {
      void this.restart(what === "new" ? null : what);
      return;
    }
    this.confirming = what;
    this.paintHead();
    this.changed();
  }

  private async restart(provider: ChatProvider | null): Promise<void> {
    this.confirming = null;
    this.provider = await this.backend.reset(provider);
    this.messages = [];
    this.started = false;
    this.picked = false;
    this.paint();
    this.changed();
  }

  private answer(m: Ask, allow: boolean): void {
    if (m.answer) return;
    m.answer = allow ? "allow" : "deny";
    Sound.play(allow ? "allow" : "deny");
    this.backend.decide(m.id, allow);
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
    this.input.style.height = "auto";
    const line = parseFloat(getComputedStyle(this.input).lineHeight) || 18;
    const border = this.input.offsetHeight - this.input.clientHeight;
    this.input.style.height = `${Math.min(this.input.scrollHeight + border, line * MAX_INPUT_LINES + 16 + border)}px`;
    this.changed();
  }

  private node(m: Message): HTMLElement {
    if (m.who === "zeca") {
      if (m.error) return el("p", { class: "msg zeca error", text: m.text });
      const b = el("div", { class: "msg zeca" });
      b.append(renderLite(m.text));
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
      allow: boolean,
      keys: string | undefined,
    ) =>
      el(
        "button",
        { class: `btn ${kind}`, onclick: () => this.answer(m, allow) },
        el("span", { text }),
        kbd(keys),
      );
    return el(
      "div",
      { class: `ask${m.answer ? ` ${m.answer}` : ""}` },
      el(
        "div",
        { class: "who" },
        el("span", { class: "label", text: asks(m.tool) }),
        diff,
      ),
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
            { class: `settled-label ${m.answer}` },
            icon(m.answer === "allow" ? "check" : "close", 12, 2.6),
            el("span", { text: m.answer === "allow" ? "Allowed" : "Denied" }),
          )
        : el(
            "div",
            { class: "actions" },
            button("Deny", "secondary", false, this.keys.deny),
            button("Allow", "primary", true, this.keys.allow),
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

  private paintHead(): void {
    this.picker.replaceChildren(
      ...this.providers().map((p) => {
        const b = el("button", {
          class: p === this.provider ? "on" : "",
          text: this.name(p),
          onclick: () => {
            if (p !== this.provider) this.ask(p);
          },
        });
        if (p === "api")
          b.title = `${this.api.label} through its API. It only talks: no commands, no edits. Change it in Settings → Chat.`;
        return b;
      }),
    );
    // The API chat has no tools, so it works nowhere in particular.
    const api = this.provider === "api";
    this.place.hidden = api;
    this.place.classList.toggle("fixed", this.started);
    this.place.replaceChildren(
      el("span", {
        text: `in ${this.folder ? folderName(this.folder) : "an empty folder"}`,
      }),
      ...(this.started ? [] : [icon("chevronDown", 11, 2.2)]),
    );
    this.place.title = this.started
      ? `${this.folder ?? "An empty folder"}. A conversation stays in its folder; New starts another.`
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
          text: cwd ? folderName(cwd) : "An empty folder",
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
    const c = this.confirming;
    this.confirm.classList.toggle("on", c !== null);
    this.confirm.replaceChildren(
      ...(c === null
        ? []
        : [
            el("span", {
              text:
                c === "new"
                  ? "Start a new chat? This one is cleared."
                  : `Switch to ${this.name(c)}? This chat is cleared.`,
            }),
            el("button", {
              class: "ghost",
              text: "Keep it",
              onclick: () => (
                (this.confirming = null),
                this.paintHead(),
                this.changed()
              ),
            }),
            el("button", {
              class: "btn primary small",
              text: c === "new" ? "New chat" : "Switch",
              onclick: () => void this.restart(c === "new" ? null : c),
            }),
          ]),
    );
  }

  private paintSend(): void {
    const stop = this.busy;
    this.send.classList.toggle("stop", stop);
    this.send.replaceChildren(
      stop ? icon("stop", 12, 2.4) : icon("chevron", 14, 2.4),
    );
    this.send.title = stop ? "Stop" : "Send (Enter)";
    this.send.toggleAttribute(
      "disabled",
      !stop && !this.input.value.trim() && !this.files.length,
    );
  }

  private paint(): void {
    this.paintHead();
    this.paintVoice();
    const last = this.messages[this.messages.length - 1];
    const waiting = this.busy && (!last || last.who === "you");
    this.nodes = this.messages.map((m) => this.node(m));
    this.log.replaceChildren(
      ...(this.messages.length
        ? this.nodes
        : [
            el("p", {
              class: "hint",
              text:
                this.provider === "api"
                  ? "Ask anything, or drop a file on the island. This chat only talks: it can't run commands or open your project."
                  : "Ask anything, or drop a file on the island.",
            }),
            el(
              "div",
              { class: "suggestions" },
              ...(this.provider === "api" ? [] : SUGGESTIONS).map((q) =>
                el("button", {
                  class: "suggestion",
                  text: q,
                  onclick: () => {
                    this.input.value = q;
                    void this.submit();
                  },
                }),
              ),
            ),
          ]),
      ...(waiting
        ? [
            el(
              "div",
              { class: "typing" },
              el("i", {}),
              el("i", {}),
              el("i", {}),
            ),
          ]
        : []),
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
