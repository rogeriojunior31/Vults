// The chat panel. Its DOM is built once and updated in place, so the input keeps its focus and
// text while session views re-render the island around it. No Tauri here: the backend comes in.
import type { AgentKind, ChatDelta } from "../bridge";
import { el } from "../dom";
import { Sound } from "../sound";
import { icon } from "./icons";
import { renderLite } from "./markdown";

export interface ChatBackend {
  send(text: string, files: string[], folder: string | null): Promise<void>;
  decide(id: string, allow: boolean): void;
  reset(provider: AgentKind | null): Promise<AgentKind>;
  keyboard(on: boolean): void;
}

type Message =
  | { who: "you" | "zeca"; text: string; error?: boolean }
  | { who: "ask"; id: string; tool: string; target: string; answer?: "allow" | "deny" };

/** How long Zeca takes to swallow a dropped file. */
const SWALLOW_MS = 1500;
/** Ways to start, offered while the conversation is empty. */
const SUGGESTIONS = ["What is this project?", "What changed recently?", "Explain the last error"];
/** The input grows up to this many lines, then scrolls. */
const MAX_INPUT_LINES = 6;

export class ChatPanel {
  readonly element = el("section", { class: "chat" });
  private open = false;
  private provider: AgentKind = "claude";
  private messages: Message[] = [];
  private files: string[] = [];
  private busy = false;
  /** Where the conversation works: the focused session's folder, fixed once it starts. */
  private folder: string | null = null;
  private started = false;
  private readonly place = el("span", { class: "place" });
  private swallowUntil = 0;
  private readonly log = el("div", { class: "log" });
  private readonly chips = el("div", { class: "chips" });
  private readonly input = document.createElement("textarea");
  private readonly picker = el("div", { class: "segmented" });
  private readonly send = el("button", { class: "send", onclick: () => void this.submit() }, icon("chevron", 14, 2.4));

  constructor(
    private readonly backend: ChatBackend,
    private readonly changed: () => void,
  ) {
    this.input.rows = 1;
    this.input.placeholder = "Ask Zeca…";
    this.input.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void this.submit();
      } else if (e.key === "Escape") {
        this.toggle(false);
      }
    });
    this.input.addEventListener("input", () => this.grow());
    this.send.title = "Send (Enter)";
    const head = el(
      "div",
      { class: "chat-head" },
      this.picker,
      this.place,
      el("button", { class: "ghost", text: "New", onclick: () => void this.restart(null) }),
      el("button", { class: "ghost", text: "×", onclick: () => this.toggle(false) }),
    );
    this.element.append(head, this.log, this.chips, el("div", { class: "composer" }, this.input, this.send));
    this.paint();
  }

  isOpen(): boolean {
    return this.open;
  }

  agent(): AgentKind {
    return this.provider;
  }

  /** Fits the input to its text, up to a few lines. */
  private grow(): void {
    this.input.style.height = "auto";
    const line = parseFloat(getComputedStyle(this.input).lineHeight) || 18;
    this.input.style.height = `${Math.min(this.input.scrollHeight, line * MAX_INPUT_LINES + 16)}px`;
    this.changed();
  }

  /** The focused session's folder; used when the next conversation starts. */
  setFolder(folder: string | null): void {
    if (this.started || folder === this.folder) return;
    this.folder = folder;
    this.paint();
  }

  /** What Zeca does while the chat is open. */
  clip(now: number): string {
    if (now < this.swallowUntil) return "swallow";
    const last = this.messages[this.messages.length - 1];
    if (last?.who === "ask" && !last.answer) return "question";
    if (this.busy && (!last || last.who === "you")) return "think";
    return "idle";
  }

  toggle(open = !this.open): void {
    if (open === this.open) return;
    this.open = open;
    this.backend.keyboard(open);
    if (open) queueMicrotask(() => this.input.focus());
    this.changed();
  }

  attach(paths: string[]): void {
    this.files.push(...paths.filter((p) => !this.files.includes(p)));
    this.swallowUntil = performance.now() + SWALLOW_MS;
    window.setTimeout(() => Sound.play("swallow"), 450);
    this.toggle(true);
    this.paint();
    this.changed();
    window.setTimeout(() => this.changed(), SWALLOW_MS);
  }

  receive(d: ChatDelta): void {
    const last = this.messages[this.messages.length - 1];
    if (d.kind === "text") {
      if (last?.who === "zeca" && !last.error) last.text += d.text;
      else this.messages.push({ who: "zeca", text: d.text });
    } else if (d.kind === "permission") {
      this.messages.push({ who: "ask", id: d.id, tool: d.tool, target: d.target });
      Sound.play("approval");
    } else {
      this.busy = false;
      if (d.kind === "error") this.messages.push({ who: "zeca", text: d.message, error: true });
    }
    this.paint();
    this.changed();
  }

  private async submit(): Promise<void> {
    const text = this.input.value.trim();
    if (!text || this.busy) return;
    const files = this.files;
    this.messages.push({ who: "you", text: files.length ? `${text}\n📎 ${files.map(shortName).join(", ")}` : text });
    this.files = [];
    this.input.value = "";
    this.input.style.height = "";
    this.busy = true;
    this.paint();
    this.changed();
    this.started = true;
    try {
      await this.backend.send(text, files, this.folder);
    } catch (e) {
      this.receive({ kind: "error", message: String(e) });
    }
  }

  private async restart(provider: AgentKind | null): Promise<void> {
    if (this.busy) return;
    this.provider = await this.backend.reset(provider);
    this.messages = [];
    this.started = false;
    this.paint();
    this.changed();
  }

  /** Answers the permission waiting in the chat, if one is; for the global shortcuts. */
  answerWaiting(allow: boolean): boolean {
    if (!this.open) return false;
    const waiting = [...this.messages].reverse().find((m) => m.who === "ask" && !m.answer);
    if (!waiting || waiting.who !== "ask") return false;
    this.answer(waiting, allow);
    return true;
  }

  private answer(m: Extract<Message, { who: "ask" }>, allow: boolean): void {
    if (m.answer) return;
    m.answer = allow ? "allow" : "deny";
    this.backend.decide(m.id, allow);
    this.paint();
    this.changed();
  }

  private bubble(m: Message): HTMLElement {
    if (m.who === "zeca" && !m.error) {
      const b = el("div", { class: "msg zeca" });
      b.append(renderLite(m.text));
      return b;
    }
    if (m.who !== "ask") return el("p", { class: `msg ${m.who}${m.error ? " error" : ""}`, text: m.text });
    return el(
      "div",
      { class: `ask${m.answer ? ` ${m.answer}` : ""}` },
      el("div", { class: "asks", text: "Zeca wants to use" }),
      el("pre", { class: "target", text: m.target }),
      m.answer
        ? el("div", { class: "asks", text: m.answer === "allow" ? "Allowed" : "Denied" })
        : el(
            "div",
            { class: "actions" },
            el("button", { class: "deny", text: "Deny", onclick: () => this.answer(m, false) }),
            el("button", { class: "allow", text: "Allow", onclick: () => this.answer(m, true) }),
          ),
    );
  }

  private paint(): void {
    this.picker.replaceChildren(
      ...(["claude", "codex"] as const).map((p) =>
        el("button", {
          class: p === this.provider ? "on" : "",
          text: p === "claude" ? "Claude" : "Codex",
          onclick: () => {
            if (p !== this.provider) void this.restart(p);
          },
        }),
      ),
    );
    const name = this.folder?.split(/[\\/]/).filter(Boolean).pop();
    this.place.textContent = name ? `in ${name}` : "";
    this.place.title = this.folder ?? "";
    const last = this.messages[this.messages.length - 1];
    const waiting = this.busy && (!last || last.who === "you");
    this.log.replaceChildren(
      ...(this.messages.length
        ? this.messages.map((m) => this.bubble(m))
        : [
            el("p", { class: "hint", text: "Ask anything, or drop a file on the island." }),
            el(
              "div",
              { class: "suggestions" },
              ...SUGGESTIONS.map((q) =>
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
      ...(waiting ? [el("div", { class: "typing" }, el("i", {}), el("i", {}), el("i", {}))] : []),
    );
    this.log.scrollTop = this.log.scrollHeight;
    this.chips.replaceChildren(
      ...this.files.map((f) =>
        el("span", { class: "chip", text: shortName(f) }, el("button", {
          class: "ghost",
          text: "×",
          onclick: () => {
            this.files = this.files.filter((x) => x !== f);
            this.paint();
            this.changed();
          },
        })),
      ),
    );
    this.input.disabled = this.busy;
    this.send.toggleAttribute("disabled", this.busy);
  }
}

/** Inbox copies are named `<millis>-<original name>`. */
const shortName = (path: string) => (path.split(/[\\/]/).pop() ?? path).replace(/^\d+-/, "");
