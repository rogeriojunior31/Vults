// Settings → Agents, what each agent can do (W1), with made-up agents, for the visual tests.
import type { AgentRow } from "../src/bridge";
import { el } from "../src/dom";
import { setLang } from "../src/i18n";
import { createDoctor } from "../src/surfaces/settings/doctor";

setLang(new URLSearchParams(location.search).get("lang") ?? "en");

const AGENTS: AgentRow[] = [
  { id: "claude", name: "Claude Code", command: "claude", permissions: "card", questions: "card", diffs: "numbered", stop: "interrupt", sandbox: "bubblewrap", starts: true, path: "~/.local/bin/claude", version: "2.1.295 (Claude Code)", hooks: "ok" },
  { id: "codex", name: "Codex", command: "codex", permissions: "card", questions: "terminal", diffs: "lines", stop: "interrupt", sandbox: "codex", starts: true, path: "/usr/bin/codex", version: "codex-cli 0.160.1", hooks: "untrusted" },
  { id: "qwen", name: "Qwen Code", command: "qwen", permissions: "card", questions: "terminal", diffs: "lines", stop: "no", sandbox: "container", starts: false, path: null, version: null, hooks: "missing" },
];

const doctor = createDoctor(async () => AGENTS);
document.getElementById("settings")!.append(el("main", { class: "page" }, doctor.element));
doctor.refresh();
