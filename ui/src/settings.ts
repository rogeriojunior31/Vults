// Settings: install or remove the hooks, always through a diff the user reviews first.
import { Bridge, type AgentKind, type InstallPreview, type InstallStatus } from "./bridge";
import { el } from "./dom";

const root = document.getElementById("settings")!;
const AGENT: AgentKind = "claude";

let message: { text: string; error: boolean } | null = null;
let pending: { install: boolean; preview: InstallPreview } | null = null;

async function refresh(): Promise<void> {
  try {
    render(await Bridge.installStatus(AGENT));
  } catch (e) {
    message = { text: String(e), error: true };
    root.replaceChildren(el("div", {}, notice()));
  }
}

function notice(): HTMLElement | null {
  return message ? el("p", { class: message.error ? "error" : "ok", text: message.text }) : null;
}

async function preview(install: boolean): Promise<void> {
  message = null;
  try {
    pending = { install, preview: await Bridge.installPreview(AGENT, install) };
  } catch (e) {
    message = { text: String(e), error: true };
  }
  await refresh();
}

async function apply(): Promise<void> {
  if (!pending) return;
  const { install, preview } = pending;
  pending = null;
  try {
    const backup = await Bridge.installApply(AGENT, install, preview.fingerprint);
    const done = install ? "Hooks installed." : "Hooks removed.";
    message = { text: backup ? `${done} Backup: ${backup}` : done, error: false };
  } catch (e) {
    message = { text: String(e), error: true };
  }
  await refresh();
}

function render(s: InstallStatus): void {
  const review = pending
    ? el(
        "section",
        { class: "review" },
        el("h2", { text: pending.install ? "Review the install" : "Review the removal" }),
        pending.preview.diff
          ? el("pre", { class: "diff", text: pending.preview.diff })
          : el("p", { text: "Nothing to change." }),
        el(
          "div",
          { class: "actions" },
          el("button", { text: "Cancel", onclick: () => ((pending = null), void refresh()) }),
          pending.preview.diff
            ? el("button", { class: "primary", text: "Write the file", onclick: () => void apply() })
            : null,
        ),
      )
    : null;

  root.replaceChildren(
    el(
      "div",
      {},
      el("h1", { text: "Claude Code" }),
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
    notice(),
    s.error || pending
      ? null
      : el(
          "div",
          { class: "actions" },
          s.installed
            ? el("button", { text: "Remove hooks…", onclick: () => void preview(false) })
            : null,
          el("button", {
            class: "primary",
            text: s.installed ? "Reinstall hooks…" : "Install hooks…",
            onclick: () => void preview(true),
          }),
        ),
      review,
    ),
  );
}

void refresh();
