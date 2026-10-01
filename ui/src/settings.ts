// The settings window: a sidebar and one page per section. Installing hooks always goes through a
// diff the user reviews first.
import { getVersion } from "@tauri-apps/api/app";
import { Bridge, type AgentKind, type ConnectorStatus, type InstallPreview, type InstallStatus, type Rule } from "./bridge";
import { CONNECTORS } from "./connectors";
import { el } from "./dom";

type Page = "general" | "agents" | "chat" | "approvals" | "connectors" | "about";

const PAGES: { id: Page; label: string }[] = [
  { id: "general", label: "General" },
  { id: "agents", label: "Agents" },
  { id: "chat", label: "Chat" },
  { id: "approvals", label: "Approvals" },
  { id: "connectors", label: "Connectors" },
  { id: "about", label: "About" },
];

const AGENTS: { kind: AgentKind; name: string }[] = [
  { kind: "claude", name: "Claude Code" },
  { kind: "codex", name: "Codex" },
];

interface Panel {
  status: InstallStatus | null;
  message: { text: string; error: boolean } | null;
  pending: { install: boolean; preview: InstallPreview } | null;
}

const root = document.getElementById("settings")!;
let page: Page = (location.hash.slice(1) as Page) || "agents";
const panels = new Map<AgentKind, Panel>(AGENTS.map((a) => [a.kind, { status: null, message: null, pending: null }]));
let connectorStatus = new Map<string, ConnectorStatus>();
let sounds = true;
let autostart = false;
let version = "";
let rules: Rule[] = [];
let apiKey = false;
let apiKeyMessage: { text: string; error: boolean } | null = null;

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
    el("h1", { text: "Approvals" }),
    el("p", {
      class: "lede",
      text: "Permissions you chose to always allow, with Always on a card. Each one covers only that exact command or file, for that agent, in that folder.",
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
                el("div", { class: "row-about", text: `${r.agent === "claude" ? "Claude Code" : "Codex"} · ${name(r.cwd)} · ${r.cwd}` }),
              ),
              button("Remove", () => {
                void Bridge.ruleRemove(i).then(refreshRules);
              }),
            ),
          ),
        )
      : el("section", { class: "card" }, el("p", { class: "note", text: "Nothing is always allowed. Every permission asks." })),
  ];
}

// ── Pieces ───────────────────────────────────────────────────────────────────

function toggle(on: boolean, change: (on: boolean) => Promise<void>): HTMLElement {
  const input = document.createElement("input");
  input.type = "checkbox";
  input.className = "toggle";
  input.checked = on;
  input.addEventListener("change", () => {
    void change(input.checked).catch(() => {
      input.checked = !input.checked;
    });
  });
  return input;
}

function row(title: string, about: string, control: HTMLElement): HTMLElement {
  return el(
    "div",
    { class: "row" },
    el("div", { class: "row-text" }, el("div", { class: "row-title", text: title }), el("div", { class: "row-about", text: about })),
    control,
  );
}

function badge(text: string, kind: "ok" | "warn" | "error" | "off"): HTMLElement {
  return el("span", { class: `badge ${kind}`, text });
}

function button(text: string, onclick: () => void, primary = false): HTMLElement {
  return el("button", { class: primary ? "btn primary" : "btn", text, onclick });
}

/** A unified diff with its added and removed lines colored. */
function diff(text: string): HTMLElement {
  const pre = el("pre", { class: "diff" });
  for (const line of text.split("\n")) {
    const kind = line.startsWith("+++") || line.startsWith("---") ? "meta" : line.startsWith("+") ? "add" : line.startsWith("-") ? "del" : line.startsWith("@@") ? "hunk" : "";
    pre.append(el("span", { class: `dl ${kind}`, text: `${line}\n` }));
  }
  return pre;
}

function ago(secs: number): string {
  const s = Math.max(0, Math.round(Date.now() / 1000 - secs));
  return s < 60 ? "just now" : s < 3600 ? `${Math.round(s / 60)} min ago` : `${Math.round(s / 3600)} h ago`;
}

// ── Agents ───────────────────────────────────────────────────────────────────

async function refresh(kind: AgentKind): Promise<void> {
  const panel = panels.get(kind)!;
  try {
    panel.status = await Bridge.installStatus(kind);
  } catch (e) {
    panel.message = { text: String(e), error: true };
  }
  render();
}

async function preview(kind: AgentKind, install: boolean): Promise<void> {
  const panel = panels.get(kind)!;
  panel.message = null;
  try {
    panel.pending = { install, preview: await Bridge.installPreview(kind, install) };
  } catch (e) {
    panel.message = { text: String(e), error: true };
  }
  await refresh(kind);
}

async function apply(kind: AgentKind): Promise<void> {
  const panel = panels.get(kind)!;
  if (!panel.pending) return;
  const { install, preview } = panel.pending;
  panel.pending = null;
  try {
    const backup = await Bridge.installApply(kind, install, preview.fingerprint);
    const done = install ? "Hooks installed." : "Hooks removed.";
    panel.message = { text: backup ? `${done} Backup: ${backup}` : done, error: false };
  } catch (e) {
    panel.message = { text: String(e), error: true };
  }
  await refresh(kind);
}

function agentStatus(s: InstallStatus): HTMLElement {
  if (s.error) return badge("Can't read", "error");
  if (!s.installed) return badge("Not installed", "off");
  if (s.codex?.hooksDisabled) return badge("Hooks off in Codex", "error");
  if (s.codex && s.codex.untrusted > 0) return badge(`${s.codex.untrusted} to trust`, "warn");
  return badge("Installed", "ok");
}

function agentCard(kind: AgentKind, name: string): HTMLElement {
  const { status: s, message, pending } = panels.get(kind)!;
  const notice = message ? el("p", { class: message.error ? "note error" : "note ok", text: message.text }) : null;
  if (!s) return el("section", { class: "card" }, el("div", { class: "card-title", text: name }), notice);

  const codexHelp =
    s.codex && s.installed && !s.codex.hooksDisabled && s.codex.untrusted > 0
      ? el("p", {
          class: "note warn",
          text: `Codex runs a hook only once you trust it: open Codex, type /hooks and trust the ${s.codex.untrusted} Vultures AI hooks waiting there.`,
        })
      : null;
  const review = pending
    ? el(
        "div",
        { class: "review" },
        el("div", { class: "review-title", text: pending.install ? "Review the install" : "Review the removal" }),
        pending.preview.diff ? diff(pending.preview.diff) : el("p", { class: "note", text: "Nothing to change." }),
        el(
          "div",
          { class: "actions" },
          button("Cancel", () => {
            panels.get(kind)!.pending = null;
            render();
          }),
          pending.preview.diff ? button("Write the file", () => void apply(kind), true) : null,
        ),
      )
    : null;

  return el(
    "section",
    { class: "card" },
    el("div", { class: "card-head" }, el("div", { class: "card-title", text: name }), agentStatus(s)),
    el("div", { class: "path", text: s.configPath }),
    s.hookReady ? null : el("p", { class: "note error", text: `The hook relay is missing at ${s.hookPath}.` }),
    s.error ? el("p", { class: "note error", text: s.error }) : null,
    codexHelp,
    notice,
    s.error || pending
      ? null
      : el(
          "div",
          { class: "actions" },
          s.installed ? button("Remove hooks…", () => void preview(kind, false)) : null,
          button(s.installed ? "Reinstall hooks…" : "Install hooks…", () => void preview(kind, true), true),
        ),
    review,
  );
}

function agentsPage(): HTMLElement[] {
  return [
    el("h1", { text: "Agents" }),
    el("p", {
      class: "lede",
      text: "Vultures AI hears your agents through hooks in their config. Every change shows you the exact diff and takes a dated backup first; hooks from other tools are kept.",
    }),
    ...AGENTS.map((a) => agentCard(a.kind, a.name)),
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
    el("h1", { text: "Connectors" }),
    el("p", { class: "lede", text: "News from outside services on the island. Each one is off until you switch it on." }),
    ...CONNECTORS.map((c) => {
      const st = connectorStatus.get(c.id);
      const state = !st?.enabled
        ? badge("Off", "off")
        : st.error
          ? badge("Error", "error")
          : st.lastOk
            ? badge(`Watching ${st.watching}`, "ok")
            : badge("Checking…", "warn");
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
            state,
            toggle(st?.enabled ?? false, async (on) => {
              await Bridge.connectorEnable(c.id, on);
              await refreshConnectors();
            }),
          ),
        ),
        el("p", { class: "row-about", text: c.about }),
        st?.enabled && st.lastOk && !st.error ? el("p", { class: "note", text: `Last checked ${ago(st.lastOk)}.` }) : null,
        st?.enabled && st.error ? el("p", { class: "note error", text: st.error }) : null,
      );
    }),
  ];
}

// ── General ──────────────────────────────────────────────────────────────────

function generalPage(): HTMLElement[] {
  return [
    el("h1", { text: "General" }),
    el(
      "section",
      { class: "card rows" },
      row(
        "Sounds",
        "Short 8-bit blips when a session needs you, finishes or fails, and for connector news.",
        toggle(sounds, async (on) => {
          await Bridge.setSounds(on);
          sounds = on;
        }),
      ),
      row(
        "Start with the desktop",
        "Opens Vultures AI when you log in.",
        toggle(autostart, async (on) => {
          await Bridge.setAutostart(on);
          autostart = on;
        }),
      ),
    ),
    el(
      "section",
      { class: "card rows" },
      row("Allow / Deny from anywhere", "Ctrl+Alt+Y and Ctrl+Alt+N answer the card on the island. Change the keys in System Settings → Shortcuts.", el("span", { class: "kbd", text: "Ctrl+Alt+Y · Ctrl+Alt+N" })),
    ),
  ];
}

// ── Chat ─────────────────────────────────────────────────────────────────────

function chatPage(): HTMLElement[] {
  const message = apiKeyMessage ? el("p", { class: `note${apiKeyMessage.error ? " error" : " ok"}`, text: apiKeyMessage.text }) : null;
  const done = (text: string, error = false) => {
    apiKeyMessage = { text, error };
    render();
  };
  let control: HTMLElement;
  if (apiKey) {
    control = button("Remove", () => {
      Bridge.apiKeyClear()
        .then(() => {
          apiKey = false;
          done("Key removed from the keyring.");
        })
        .catch((e) => done(String(e), true));
    });
  } else {
    const input = document.createElement("input");
    input.type = "password";
    input.className = "field";
    input.placeholder = "sk-ant-…";
    input.autocomplete = "off";
    input.spellcheck = false;
    const save = () => {
      Bridge.apiKeySet(input.value)
        .then(() => {
          apiKey = true;
          done("Saved. Choose API in the chat to use it.");
        })
        .catch((e) => done(String(e), true));
    };
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") save();
    });
    control = el("div", { class: "inline" }, input, button("Save", save, true));
  }
  return [
    el("h1", { text: "Chat" }),
    el("p", {
      class: "lede",
      text: "The chat on the island talks through the Claude Code or Codex CLI you are logged into, on your own subscription. Without one, you can use an Anthropic API key instead.",
    }),
    el(
      "section",
      { class: "card rows" },
      row(
        "Anthropic API key",
        apiKey ? "Saved in the system keyring." : "Kept in the system keyring, never in a file. Usage is billed to your API account.",
        control,
      ),
      message ? el("div", { class: "row" }, message) : null,
    ),
    el(
      "section",
      { class: "card" },
      el(
        "p",
        { class: "note" },
        document.createTextNode(
          "The API chat uses Claude Opus 5.5 and only talks: it can't run commands, edit files or open your project, only read the files you drop on the island. If Claude declines a request on safety grounds, the API retries it on another Claude model in the same call.",
        ),
      ),
    ),
  ];
}

// ── About ────────────────────────────────────────────────────────────────────

function aboutPage(): HTMLElement[] {
  return [
    el("h1", { text: "About" }),
    el(
      "section",
      { class: "card about" },
      el("img", { class: "logo" }),
      el(
        "div",
        {},
        el("div", { class: "card-title", text: "Vultures AI" }),
        el("p", { class: "row-about", text: "A friendly flock watching your coding agents. MIT licensed; no telemetry." }),
        version ? el("p", { class: "note", text: `Version ${version}` }) : null,
      ),
    ),
    el(
      "section",
      { class: "card rows" },
      row("Settings", "Your settings file.", el("code", { text: "~/.config/vultures-ai/settings.json" })),
      row("Data", "The hook relay, the inbox of dropped files, connector state.", el("code", { text: "~/.local/share/vultures-ai/" })),
    ),
  ];
}

// ── Layout ───────────────────────────────────────────────────────────────────

function render(): void {
  const content =
    page === "general"
      ? generalPage()
      : page === "agents"
        ? agentsPage()
        : page === "chat"
          ? chatPage()
          : page === "approvals"
            ? approvalsPage()
            : page === "connectors"
              ? connectorsPage()
              : aboutPage();
  const nav = el(
    "nav",
    { class: "sidebar" },
    el("div", { class: "brand", text: "Vultures AI" }),
    ...PAGES.map((p) =>
      el("button", {
        class: `nav${p.id === page ? " on" : ""}`,
        text: p.label,
        onclick: () => {
          page = p.id;
          location.hash = p.id;
          apiKeyMessage = null;
          if (p.id === "approvals") void refreshRules();
void Bridge.apiKeyStatus()
  .then((on) => {
    apiKey = on;
    if (page === "chat") render();
  })
  .catch(() => {});
          render();
        },
      }),
    ),
  );
  root.replaceChildren(nav, el("main", { class: "page" }, ...content));
  const logo = root.querySelector<HTMLImageElement>("img.logo");
  if (logo) logo.src = "/icon.png";
}

render();
void Bridge.appSettings().then((s) => {
  sounds = s.sounds;
  autostart = s.autostart;
  render();
});
for (const a of AGENTS) void refresh(a.kind);
void refreshConnectors();
void refreshRules();
void getVersion()
  .then((v) => {
    version = v;
    if (page === "about") render();
  })
  .catch(() => {});
// Statuses age and polls finish in the background.
window.setInterval(() => void refreshConnectors(), 15_000);
