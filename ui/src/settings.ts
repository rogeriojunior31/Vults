// Settings: install or remove the hooks per agent, always through a diff the user reviews first.
import { Bridge, type AgentKind, type InstallPreview, type InstallStatus } from "./bridge";
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

function render(): void {
  root.replaceChildren(...AGENTS.map((a) => section(a.kind, a.name)));
}

render();
for (const a of AGENTS) void refresh(a.kind);
