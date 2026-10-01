// Settings: install or remove the hooks per agent, always through a diff the user reviews first.
import { Bridge, type AgentKind, type ConnectorStatus, type InstallPreview, type InstallStatus } from "./bridge";
import { CONNECTORS } from "./connectors";
import { el } from "./dom";

const root = document.getElementById("settings")!;
const AGENTS: { kind: AgentKind; name: string }[] = [
  { kind: "claude", name: "Claude Code" },
  { kind: "codex", name: "Codex" },
];

interface Panel {
  status: InstallStatus | null;
  message: { text: string; error: boolean } | null;
  pending: { install: boolean; preview: InstallPreview } | null;
}

const panels = new Map<AgentKind, Panel>(AGENTS.map((a) => [a.kind, { status: null, message: null, pending: null }]));

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

function codexNotes(s: InstallStatus): HTMLElement | null {
  if (!s.codex || !s.installed) return null;
  if (s.codex.hooksDisabled) {
    return el("p", { class: "error", text: "Codex has hooks turned off ([features] hooks = false in config.toml)." });
  }
  if (s.codex.untrusted > 0) {
    return el("p", {
      class: "warn",
      text: `Codex runs a hook only once you trust it. Open Codex, type /hooks and trust the ${s.codex.untrusted} Vultures AI hooks waiting there.`,
    });
  }
  return el("p", { class: "ok", text: "Codex trusts every Vultures AI hook." });
}

function section(kind: AgentKind, name: string): HTMLElement {
  const { status: s, message, pending } = panels.get(kind)!;
  const notice = message ? el("p", { class: message.error ? "error" : "ok", text: message.text }) : null;
  if (!s) return el("section", { class: "agent" }, el("h1", { text: name }), notice);

  const review = pending
    ? el(
        "div",
        { class: "review" },
        el("h2", { text: pending.install ? "Review the install" : "Review the removal" }),
        pending.preview.diff
          ? el("pre", { class: "diff", text: pending.preview.diff })
          : el("p", { text: "Nothing to change." }),
        el(
          "div",
          { class: "actions" },
          el("button", {
            text: "Cancel",
            onclick: () => {
              panels.get(kind)!.pending = null;
              render();
            },
          }),
          pending.preview.diff
            ? el("button", { class: "primary", text: "Write the file", onclick: () => void apply(kind) })
            : null,
        ),
      )
    : null;

  return el(
    "section",
    { class: "agent" },
    el("h1", { text: name }),
    el("p", { class: "path", text: s.configPath }),
    el("p", {
      text: s.error
        ? "This file can't be read, so it will not be touched."
        : s.installed
          ? "Hooks are installed."
          : "Hooks are not installed.",
    }),
    s.hookReady ? null : el("p", { class: "error", text: `The hook relay is missing at ${s.hookPath}.` }),
    s.error ? el("p", { class: "error", text: s.error }) : null,
    codexNotes(s),
    notice,
    s.error || pending
      ? null
      : el(
          "div",
          { class: "actions" },
          s.installed ? el("button", { text: "Remove hooks…", onclick: () => void preview(kind, false) }) : null,
          el("button", {
            class: "primary",
            text: s.installed ? "Reinstall hooks…" : "Install hooks…",
            onclick: () => void preview(kind, true),
          }),
        ),
    review,
  );
}

let connectorStatus = new Map<string, ConnectorStatus>();

async function refreshConnectors(): Promise<void> {
  try {
    connectorStatus = new Map((await Bridge.connectorsStatus()).map((c) => [c.id, c]));
  } catch {
    // Shown as "not running" below.
  }
  render();
}

function ago(secs: number): string {
  const s = Math.max(0, Math.round(Date.now() / 1000 - secs));
  return s < 60 ? "just now" : s < 3600 ? `${Math.round(s / 60)} min ago` : `${Math.round(s / 3600)} h ago`;
}

function connectorsSection(): HTMLElement {
  return el(
    "section",
    { class: "agent" },
    el("h1", { text: "Connectors" }),
    ...CONNECTORS.map((c) => {
      const st = connectorStatus.get(c.id);
      const toggle = document.createElement("input");
      toggle.type = "checkbox";
      toggle.checked = st?.enabled ?? false;
      toggle.addEventListener("change", async () => {
        await Bridge.connectorEnable(c.id, toggle.checked).catch(() => {});
        await refreshConnectors();
      });
      const line = !st?.enabled
        ? "Off."
        : st.error
          ? null
          : st.lastOk
            ? `Watching ${st.watching} items · checked ${ago(st.lastOk)}.`
            : "Checking…";
      return el(
        "div",
        { class: "connector" },
        el("label", { class: "switch" }, toggle, el("span", { class: "name", text: c.name })),
        el("p", { class: "path", text: c.about }),
        line ? el("p", { text: line }) : null,
        st?.enabled && st.error ? el("p", { class: "error", text: st.error }) : null,
      );
    }),
  );
}

let sounds = true;

function generalSection(): HTMLElement {
  const toggle = document.createElement("input");
  toggle.type = "checkbox";
  toggle.checked = sounds;
  toggle.addEventListener("change", async () => {
    sounds = toggle.checked;
    await Bridge.setSounds(sounds).catch(() => {});
  });
  return el(
    "section",
    { class: "agent" },
    el("h1", { text: "General" }),
    el("label", { class: "switch" }, toggle, el("span", { class: "name", text: "Sounds" })),
    el("p", { class: "path", text: "Short 8-bit blips when a session needs you, finishes or fails, and for connector news." }),
  );
}

function render(): void {
  root.replaceChildren(...AGENTS.map((a) => section(a.kind, a.name)), connectorsSection(), generalSection());
}

render();
void Bridge.appSettings().then((s) => {
  sounds = s.sounds;
  render();
});
for (const a of AGENTS) void refresh(a.kind);
void refreshConnectors();
// The status line ages and polls finish in the background.
window.setInterval(() => void refreshConnectors(), 15_000);
