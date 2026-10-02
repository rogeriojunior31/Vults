// The chat panel: Zeca on the left (his perch comes in from the island), the conversation on the
// right. Its DOM is built once and updated in place, so the input keeps its focus and text while
// session views re-render the island around it, and a streaming reply only touches its own
// message. No Tauri here: the backend comes in.
import { Clock } from "../clock";
import type { AgentKind, ChatDelta, ChatProvider } from "../bridge";
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
}

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
const PROVIDER_NAMES: Record<ChatProvider, string> = {
  claude: "Claude",
  codex: "Codex",
  api: "API",
};
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
  /** Whether an API key is saved: the API choice is only offered then. */
  private apiKey = false;
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
  private readonly drop = el(
    "div",
    { class: "drop-zone" },
    el("span", { class: "drop-title", text: "Drop files here" }),
    el("span", { class: "drop-kinds", text: "PDF · images · code · text" }),
  );

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
      el("div", { class: "composer" }, this.input, this.send),
    );
    this.drop.prepend(dashes());
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

  setApiKey(on: boolean): void {
    if (on === this.apiKey) return;
    this.apiKey = on;
    this.paintHead();
    this.changed();
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
    if (this.busy && (!last || last.who === "you")) return "think";
    return "idle";
  }

  toggle(open = !this.open): void {
    if (open === this.open) return;
    this.open = open;
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
    this.drop.classList.toggle("on", on);
    this.changed();
  }

  attach(paths: string[], refused: Refused[] = []): void {
    this.setDragOver(false);
    for (const r of refused)
      this.messages.push({ who: "note", text: REFUSED[r.reason](r.name) });
    if (paths.length) {
      this.files.push(...paths.filter((p) => !this.files.includes(p)));
      this.swallowUntil = Clock.now() + SWALLOW_MS;
      window.setTimeout(() => Sound.play("swallow"), 450);
      window.setTimeout(() => this.changed(), SWALLOW_MS);
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
    this.input.style.height = `${Math.min(this.input.scrollHeight, line * MAX_INPUT_LINES + 16)}px`;
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
    return this.apiKey || this.provider === "api"
      ? ["claude", "codex", "api"]
      : ["claude", "codex"];
  }

  private paintHead(): void {
    this.picker.replaceChildren(
      ...this.providers().map((p) => {
        const b = el("button", {
          class: p === this.provider ? "on" : "",
          text: PROVIDER_NAMES[p],
          onclick: () => {
            if (p !== this.provider) this.ask(p);
          },
        });
        if (p === "api")
          b.title =
            "Claude with your API key. It only talks: no commands, no edits.";
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
                  : `Switch to ${PROVIDER_NAMES[c]}? This chat is cleared.`,
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
