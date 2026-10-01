// The chat panel. Its DOM is built once and updated in place, so the input keeps its focus and
// text while session views re-render the island around it. No Tauri here: the backend comes in.
import type { AgentKind, ChatDelta } from "../bridge";
import { el } from "../dom";
import { Sound } from "../sound";

export interface ChatBackend {
  send(text: string, files: string[]): Promise<void>;
  reset(provider: AgentKind | null): Promise<AgentKind>;
  keyboard(on: boolean): void;
}

interface Message {
  who: "you" | "zeca";
  text: string;
  error?: boolean;
}

/** How long Zeca takes to swallow a dropped file. */
const SWALLOW_MS = 1500;

export class ChatPanel {
  readonly element = el("section", { class: "chat" });
  private open = false;
  private provider: AgentKind = "claude";
  private messages: Message[] = [];
  private files: string[] = [];
  private busy = false;
  private swallowUntil = 0;
  private readonly log = el("div", { class: "log" });
  private readonly chips = el("div", { class: "chips" });
  private readonly input = document.createElement("textarea");
  private readonly picker = document.createElement("select");

  constructor(
    private readonly backend: ChatBackend,
    private readonly changed: () => void,
  ) {
    this.input.rows = 2;
    this.input.placeholder = "Ask Zeca…";
    this.input.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void this.submit();
      } else if (e.key === "Escape") {
        this.toggle(false);
      }
    });
    for (const [value, label] of [
      ["claude", "Claude"],
      ["codex", "Codex"],
    ]) {
      const o = document.createElement("option");
      o.value = value;
      o.textContent = label;
      this.picker.append(o);
    }
    this.picker.addEventListener("change", () => void this.restart(this.picker.value as AgentKind));
    const head = el(
      "div",
      { class: "chat-head" },
      this.picker,
      el("button", { class: "ghost", text: "New", onclick: () => void this.restart(null) }),
      el("button", { class: "ghost", text: "×", onclick: () => this.toggle(false) }),
    );
    this.element.append(head, this.log, this.chips, this.input);
    this.paint();
  }

  isOpen(): boolean {
    return this.open;
  }

  agent(): AgentKind {
    return this.provider;
  }

  /** What Zeca does while the chat is open. */
  clip(now: number): string {
    if (now < this.swallowUntil) return "swallow";
    const last = this.messages[this.messages.length - 1];
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
    this.busy = true;
    this.paint();
    this.changed();
    try {
      await this.backend.send(text, files);
    } catch (e) {
      this.receive({ kind: "error", message: String(e) });
    }
  }

  private async restart(provider: AgentKind | null): Promise<void> {
    if (this.busy) return;
    this.provider = await this.backend.reset(provider);
    this.messages = [];
    this.paint();
    this.changed();
  }

  private paint(): void {
    this.picker.value = this.provider;
    this.log.replaceChildren(
      ...(this.messages.length
        ? this.messages.map((m) => el("p", { class: `msg ${m.who}${m.error ? " error" : ""}`, text: m.text }))
        : [el("p", { class: "hint", text: "Ask anything, or drop a file on the island." })]),
      ...(this.busy ? [el("p", { class: "typing", text: "…" })] : []),
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
  }
}

/** Inbox copies are named `<millis>-<original name>`. */
const shortName = (path: string) => (path.split(/[\\/]/).pop() ?? path).replace(/^\d+-/, "");
