// Settings, a webview of the Nest's window (ADR 0023): a sidebar and one page per section. Installing hooks always goes through a
// diff the user reviews first.
import { getVersion } from "@tauri-apps/api/app";
import { Bridge, type WebEngine, type WebSettings, type ApiProvider, type ConnectorStatus, type Flock, type InstallAgent, type InstallPreview, type InstallStatus, type Corner, type Presence, type ProjectPrefs, type Rule, type VoiceStatus, type ChatAccess } from "../../bridge";
import { SPECIES, speciesSet } from "../../character/flock";
import { LOOK_GROUPS } from "../../character/looks";
import { drawFrame, frameAt, type Frame } from "../../character/sprites";
import { perchOf } from "../../character/zeca";
import { CONNECTORS } from "../../connectors";
import { el } from "../../dom";
import { button, diff, row, toggle } from "./pieces";
import { policiesSection, refreshPolicies } from "./policies";
import { createDoctor } from "./doctor";
import { LANGUAGES as APP_LANGUAGES, locale, setLang, t, tk } from "../../i18n";
import { activityPage, type Activity, type ActivityActions, type GithubGrid, type GridTab } from "./activity";
import { recapPng } from "./recap-image";
import { phoneEvent, phonePage, refreshPhone } from "./phone";
import { Sound } from "../../sound";
import { icon, type IconName } from "../../island/icons";
import { createNestFoot, createNestHead, tabByKey } from "../nest/head";
import { needsYou } from "../nest/flock";
import type { SessionView } from "../../view.gen";

type Page = "general" | "agents" | "chat" | "approvals" | "projects" | "activity" | "connectors" | "phone" | "flock" | "about";

/** The sidebar's groups, in the prototype's order: how it feels, the work, the system. */
const GROUPS = [
  { id: "experience", label: tk("Experience") },
  { id: "work", label: tk("Work") },
  { id: "system", label: tk("System") },
] as const;

const PAGES: { id: Page; label: string; group: (typeof GROUPS)[number]["id"]; icon: IconName }[] = [
  { id: "general", label: tk("General"), group: "experience", icon: "gear" },
  { id: "chat", label: tk("Chat & voice"), group: "experience", icon: "chat" },
  { id: "flock", label: tk("Flock & Zeca"), group: "experience", icon: "flock" },
  { id: "agents", label: tk("Agents"), group: "work", icon: "terminal" },
  { id: "approvals", label: tk("Approvals"), group: "work", icon: "list" },
  { id: "projects", label: tk("Projects"), group: "work", icon: "folder" },
  { id: "activity", label: tk("Activity"), group: "system", icon: "bars" },
  { id: "connectors", label: tk("Connectors"), group: "system", icon: "plug" },
  { id: "phone", label: tk("Phone"), group: "system", icon: "phone" },
  { id: "about", label: tk("About"), group: "system", icon: "person" },
];

const AGENTS: { kind: InstallAgent; name: string }[] = [
  { kind: "claude", name: "Claude Code" },
  { kind: "codex", name: "Codex" },
  { kind: "gemini", name: "Gemini CLI" },
  { kind: "antigravity", name: "Antigravity" },
  { kind: "opencode", name: "OpenCode" },
  { kind: "qwen", name: "Qwen Code" },
];

/** What the installer writes for an agent: hooks in its config, or a plugin file of ours. */
const setupWord = (kind: InstallAgent): string => (kind === "opencode" ? "plugin" : "hooks");

interface Panel {
  status: InstallStatus | null;
  message: { text: string; error: boolean } | null;
  pending: { install: boolean; preview: InstallPreview } | null;
}

const root = document.getElementById("settings")!;
// Settings is a webview of the Nest's window (ADR 0023), under the Nest's own header: a tab goes
// back to the Nest's webview, the size switch keeps Settings in front.
let nestFull = false;
const head = createNestHead({
  tab: (id) => void Bridge.openNest(id),
  gear: () => {},
  size: (s) => void Bridge.setSize(s, "settings"),
  // The palette lists sessions and their steps: the Nest's, not Settings'.
  search: null,
  sound: (on) => {
    void Bridge.setSounds(on);
    sounds = on;
    render();
  },
});
head.set("settings");
root.before(...head.elements);
/** The session in front, for the footer's terminal: only who it is, never what it says. */
let front: Pick<SessionView, "agent" | "id"> | undefined;
const foot = createNestFoot({
  zeca: (on) => {
    void Bridge.setZeca(on).then(() => {
      zeca = on;
      render();
    });
  },
  look: () => {
    page = "flock";
    navigated = true;
    location.hash = "flock";
    render();
  },
  terminal: () => {
    if (front) void Bridge.sessionJump(front.agent, front.id);
  },
  // Back to the island is Quick: the Nest closes and the island opens (ADR 0024).
  island: () => void Bridge.setSize("quick"),
});
root.after(foot.element);
Bridge.onView((v) => {
  const f = v.focus;
  const s = f ? v.sessions.find((x) => x.agent === f.agent && x.id === f.id) : undefined;
  front = s ? { agent: s.agent, id: s.id } : undefined;
  foot.setTerminal(!!s && s.raise !== false);
  head.setCounts({
    sessions: v.sessions.length,
    needs: v.sessions.filter(needsYou).length,
    connections: (v.boards ?? []).reduce((n, b) => n + b.rows.length, 0),
  });
});
const setNestFull = (on: boolean) => {
  nestFull = on;
  document.body.classList.toggle("full", on);
  head.setFull(on);
};
void Bridge.nestIsFull().then(setNestFull, () => {});
Bridge.onNestFull(setNestFull);
// Alt+1 to 5 as in the Nest; Esc leaves Full for Room.
window.addEventListener("keydown", (e) => {
  const pick = tabByKey(e, zeca);
  if (pick) {
    e.preventDefault();
    void Bridge.openNest(pick);
  } else if (e.key === "Escape" && nestFull && !e.defaultPrevented && !e.isComposing && !(e.target as Element | null)?.closest?.("input, textarea, select, [contenteditable]")) {
    e.preventDefault();
    void Bridge.setSize("room", "settings");
  }
});
const asked = (section: string): Page | null => PAGES.find((p) => p.id === section)?.id ?? null;
/** The page the app asked for (the tray's Set up agents…), else General until the agents are read:
 *  Agents when one has no hooks yet or older ones (`chooseStart`). */
const linked = asked(location.hash.slice(1));
let page: Page = linked ?? "general";
/** The user went to a page: the start page no longer moves. */
let navigated = linked !== null;
const panels = new Map<InstallAgent, Panel>(AGENTS.map((a) => [a.kind, { status: null, message: null, pending: null }]));
let connectorStatus = new Map<string, ConnectorStatus>();
let sounds = true;
/** Percent, as the slider shows it: a repaint mid-drag keeps the drag. */
let volume = 50;
/** Percent, as last saved: a failed save goes back to it. */
let savedVolume = 50;
let nowPlaying = false;
let codexUsage = false;
let nativeChat = true;
/** Who answers the chat's cards (ADR 0025). */
let chatAccess: ChatAccess = "ask";
/** Zeca's memory (M4): the journal, and each day summed up by a small model. */
let memory = false;
let journalSummary = false;
let localEmbeddings = false;
let memorySuggest = false;
let sleep = false;
let voice: VoiceStatus | null = null;
/** Model id → percent downloaded, for every download running. */
const downloading = new Map<string, number>();
let voiceError: string | null = null;
let autostart = false;
let foldAfter = 15;
let openOnHover = false;
/** The language chosen, or null to follow the system. */
let language: string | null = null;
/** Seconds the open island waits before folding, as the settings offer them. */
const FOLD_CHOICES = [5, 10, 15, 30, 60];
let zecaSpecies = "atratus";
let zecaLook = "auto";
/** Zeca's looks, as crates/core/src/looks.rs names them. */
const LOOKS = LOOK_GROUPS.flatMap((g) => g.looks);
let flock: Flock = "brazil";
let visitors = true;
let presence: Presence = "island";
let notifications = true;
/** Do not disturb until then (epoch seconds), or off. */
let dndUntil: number | null = null;
/** The lengths Settings offers, in minutes. */
const DND_CHOICES = [30, 60, 240];
let zeca = true;
let widget: Corner | null = null;
let monitor: string | null = null;
let monitors: { name: string; label: string }[] = [];
let version = "";
let settingsPath = "";
let dataPath = "";
let rules: Rule[] = [];
let projects: Record<string, ProjectPrefs> = {};
let apiProviders: ApiProvider[] = [];
let apiSelected = "";
/** Provider id → its live model list, or why it couldn't be had. */
const apiModels = new Map<string, { models: string[] } | { error: string }>();
const apiLoading = new Set<string>();
let apiMessage: { text: string; error: boolean } | null = null;
/** Zeca's web tools (Z7), and the last thing their rows said. */
let web: WebSettings | null = null;
let webMessage: { text: string; error: boolean } | null = null;

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
    el("h1", { text: t("Approvals") }),
    el("p", {
      class: "lede",
      text: t("Permissions you chose to always allow, with Always on a card. Each one covers only that exact command or file, for that agent, in that folder."),
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
                el("div", { class: "row-about", text: `${AGENTS.find(a => a.kind === r.agent)?.name ?? r.agent} · ${name(r.cwd)} · ${r.cwd}` }),
              ),
              button(t("Remove"), () => {
                void Bridge.ruleRemove(i).then(refreshRules);
              }),
            ),
          ),
        )
      : el("section", { class: "card" }, el("p", { class: "note", text: t("Nothing is always allowed. Every permission asks.") })),
    ...policiesSection(render),
  ];
}

let activity: Activity | null = null;
/** The week shown on Activity: this week's until the user pages. */
let activityWeek: string | null = null;
let clearing = false;
let gridTab: GridTab = "agents";
let hideProjects = false;
let saved: { ok: boolean; text: string } | null = null;
let github: GithubGrid | null = null;

async function refreshGithub(): Promise<void> {
  github = null;
  render();
  try {
    github = await Bridge.githubCalendar();
  } catch (e) {
    github = { state: "error", message: String(e) };
  }
  if (page === "activity") render();
}

async function refreshActivity(): Promise<void> {
  try {
    activity = await Bridge.activity(activityWeek);
  } catch {
    activity = null;
  }
  if (page === "activity") render();
}

const activityActions: ActivityActions = {
  week: (monday) => {
    activityWeek = monday;
    saved = null;
    void refreshActivity();
  },
  tab: (tab) => {
    gridTab = tab;
    if (tab === "github") void refreshGithub();
    else render();
  },
  hideProjects: (on) => {
    hideProjects = on;
  },
  saveImage: () => {
    if (!activity) return;
    const week = activity.week;
    void recapPng(week, { hideProjects })
      .then((png) => Bridge.saveRecapImage(`vults-week-${week.monday}.png`, png))
      .then(
        (path) => {
          saved = path ? { ok: true, text: t("Saved to {path}", { path }) } : null;
          render();
        },
        (e) => {
          saved = { ok: false, text: t("Not saved: {error}", { error: String(e) }) };
          render();
        },
      );
  },
  connectors: () => {
    page = "connectors";
    navigated = true;
    location.hash = "connectors";
    render();
  },
  history: async (on) => {
    await Bridge.setHistory(on);
    if (activity) activity = { ...activity, history: on };
  },
  confirmClear: (on) => {
    clearing = on;
    render();
  },
  clear: async () => {
    await Bridge.clearHistory();
    clearing = false;
    activityWeek = null;
    await refreshActivity();
  },
};

async function refreshProjects(): Promise<void> {
  try {
    projects = await Bridge.projectsList();
  } catch {
    projects = {};
  }
  if (page === "projects") render();
}

/** Mute, pin and hide, per project folder: set from a session's quick actions on the island. */
function projectsPage(): HTMLElement[] {
  const name = (cwd: string) => cwd.split(/[\\/]/).filter(Boolean).pop() ?? cwd;
  const folders = Object.keys(projects).sort((a, b) => name(a).localeCompare(name(b)));
  const set = (cwd: string, change: ProjectPrefs) => async (on: boolean) => {
    const next = { ...projects[cwd], ...Object.fromEntries(Object.keys(change).map((k) => [k, on])) };
    await Bridge.projectSet(cwd, next);
    await refreshProjects();
  };
  const choice = (label: string, on: boolean, change: (on: boolean) => Promise<void>) =>
    el("label", { class: "project-choice" }, toggle(on, change), el("span", { text: label }));
  return [
    el("h1", { text: t("Projects") }),
    el("p", {
      class: "lede",
      text: t("Choices kept per project folder, for every session in it now and later. Right-click a session on the island to mute, pin or hide its project, or to choose its bird. Muted: no sounds and no notifications when its sessions finish or fail; a card keeps both. Pinned: its sessions come first. Hidden: its sessions stay off the island, but a card from one still shows. Bird: every session of the project is that species; Automatic lets the flock draw it."),
    }),
    folders.length
      ? el(
          "section",
          { class: "card rows" },
          ...folders.map((cwd) => {
            const p = projects[cwd];
            return el(
              "div",
              { class: "row" },
              el("div", { class: "row-text" }, el("div", { class: "row-title", text: name(cwd) }), el("div", { class: "row-about", text: cwd })),
              el(
                "div",
                { class: "project-choices" },
                choice(t("Muted"), !!p.mute, set(cwd, { mute: true })),
                choice(t("Pinned"), !!p.pin, set(cwd, { pin: true })),
                choice(t("Hidden"), !!p.hide, set(cwd, { hide: true })),
                dropdown(
                  [
                    { value: null as string | null, label: t("Automatic bird") },
                    // The king vulture is a role, never a project's breed.
                    ...SPECIES.filter((x) => x.id !== "papa").map((x) => ({ value: x.id as string | null, label: x.name })),
                  ],
                  p.species ?? null,
                  async (species) => {
                    await Bridge.projectSet(cwd, { ...projects[cwd], species: species ?? undefined });
                    await refreshProjects();
                  },
                ),
                button(t("Forget"), () => {
                  void Bridge.projectSet(cwd, {})
                    .catch(() => {})
                    .finally(() => void refreshProjects());
                }),
              ),
            );
          }),
        )
      : el("section", { class: "card" }, el("p", { class: "note", text: t("No project has a choice yet. Every session shows, in the order it arrived, with its sounds.") })),
  ];
}

// ── Pieces ───────────────────────────────────────────────────────────────────


/** A row of choices, one on. */
function segmented<T>(choices: { value: T; label: string }[], current: T, change: (v: T) => Promise<void>): HTMLElement {
  const box = el("div", { class: "segmented" });
  const paint = (on: T) =>
    box.replaceChildren(
      ...choices.map((c) =>
        el("button", {
          class: c.value === on ? "on" : "",
          text: c.label,
          onclick: () => {
            if (c.value === on) return;
            paint(c.value);
            void change(c.value).catch(() => paint(on));
          },
        }),
      ),
    );
  paint(current);
  return box;
}

function dropdown<T>(
  choices: { value: T; label: string; title?: string }[],
  current: T,
  change: (v: T) => Promise<void>,
): HTMLElement {
  const box = el("select", { class: "field dropdown" });
  for (const [i, c] of choices.entries()) {
    const option = el("option", { text: c.label });
    option.value = String(i);
    if (c.title) option.title = c.title;
    option.selected = c.value === current;
    box.append(option);
  }
  let on = current;
  box.addEventListener("change", () => {
    const next = choices[Number(box.value)]!.value;
    const before = on;
    on = next;
    void change(next).catch(() => {
      on = before;
      box.value = String(choices.findIndex((c) => c.value === before));
    });
  });
  return box;
}

/** The volume, saved and heard once the slider is let go of. */
function slider(current: number, disabled: boolean): HTMLElement {
  const input = el("input", { class: "slider" });
  input.type = "range";
  input.min = "0";
  input.max = "100";
  input.step = "5";
  input.value = String(current);
  input.disabled = disabled;
  input.setAttribute("aria-label", t("Volume"));
  const shown = el("span", { class: "slider-value", text: `${current}%` });
  input.addEventListener("input", () => {
    volume = Number(input.value);
    shown.textContent = `${volume}%`;
  });
  input.addEventListener("change", () => {
    const percent = Number(input.value);
    void Bridge.setVolume(percent).then(
      () => {
        savedVolume = percent;
        // The island may have muted the sounds since: then the preview stays quiet too.
        if (!sounds) return;
        Sound.setVolume(percent);
        Sound.play("approval");
      },
      () => {
        volume = savedVolume;
        input.value = String(volume);
        shown.textContent = `${volume}%`;
      },
    );
  });
  return el("div", { class: "slider-box" }, input, shown);
}


function badge(text: string, kind: "ok" | "warn" | "error" | "off"): HTMLElement {
  return el("span", { class: `badge ${kind}`, text });
}




function ago(secs: number): string {
  const s = Math.max(0, Math.round(Date.now() / 1000 - secs));
  return s < 60 ? t("just now") : s < 3600 ? t("{n} min ago", { n: Math.round(s / 60) }) : t("{n} h ago", { n: Math.round(s / 3600) });
}

// ── Agents ───────────────────────────────────────────────────────────────────

async function refresh(kind: InstallAgent): Promise<void> {
  const panel = panels.get(kind)!;
  try {
    panel.status = await Bridge.installStatus(kind);
  } catch (e) {
    panel.message = { text: String(e), error: true };
    // Unreadable: that is for the Agents page to say.
    if (!navigated) {
      navigated = true;
      page = "agents";
    }
  }
  chooseStart();
  render();
}

/** Opened with no page asked for: Agents when an agent's hooks are older than this version, or
 *  none is installed yet (nothing works without them); General otherwise. An agent the user does
 *  not use is simply not installed, which is fine. Decided once every agent was read. */
function chooseStart(): void {
  if (navigated) return;
  const all = [...panels.values()].map((p) => p.status);
  if (all.some((s) => s === null)) return;
  navigated = true;
  const work = all.some((s) => s?.outdated) || all.every((s) => !s?.installed);
  if (work && page !== "agents") {
    page = "agents";
    location.hash = page;
  }
}

async function preview(kind: InstallAgent, install: boolean): Promise<void> {
  const panel = panels.get(kind)!;
  panel.message = null;
  try {
    panel.pending = { install, preview: await Bridge.installPreview(kind, install) };
  } catch (e) {
    panel.message = { text: String(e), error: true };
  }
  await refresh(kind);
}

async function apply(kind: InstallAgent): Promise<void> {
  const panel = panels.get(kind)!;
  if (!panel.pending) return;
  const { install, preview } = panel.pending;
  panel.pending = null;
  try {
    const backup = await Bridge.installApply(kind, install, preview.fingerprint);
    const plugin = setupWord(kind) === "plugin";
    const done = plugin ? (install ? t("Plugin installed.") : t("Plugin removed.")) : install ? t("Hooks installed.") : t("Hooks removed.");
    panel.message = { text: backup ? t("{done} Backup: {path}", { done, path: backup }) : done, error: false };
  } catch (e) {
    panel.message = { text: String(e), error: true };
  }
  await refresh(kind);
}

function agentStatus(s: InstallStatus): HTMLElement {
  if (s.error) return badge(t("Can't read"), "error");
  if (!s.installed) return badge(t("Not installed"), "off");
  if (s.codex?.error) return badge(t("Codex's config unreadable"), "error");
  if (s.codex?.hooksDisabled) return badge(t("Hooks off in Codex"), "error");
  if (s.codex && s.codex.untrusted > 0) return badge(t("{n} to trust", { n: s.codex.untrusted }), "warn");
  if (s.outdated) return badge(t("Update available"), "warn");
  return badge(t("Installed"), "ok");
}

function agentCard(kind: InstallAgent, name: string): HTMLElement {
  const { status: s, message, pending } = panels.get(kind)!;
  const notice = message ? el("p", { class: message.error ? "note error" : "note ok", text: message.text }) : null;
  if (!s) return el("section", { class: "card" }, el("div", { class: "card-title", text: name }), notice);

  const codexHelp = s.codex?.error
    ? el("p", { class: "note error", text: t("Vults can't read Codex's config.toml, so it can't tell which hooks Codex runs: {error}", { error: s.codex.error }) })
    : s.codex && s.installed && !s.codex.hooksDisabled && s.codex.untrusted > 0
      ? el("p", {
          class: "note warn",
          text: t("Codex runs a hook only once you trust it, and while it is on: open Codex, type /hooks and trust or turn on the {n} Vults hooks waiting there.", { n: s.codex.untrusted }),
        })
      : null;
  // Installing would be refused (`install_blocked` in installer.rs): say why instead of offering it.
  const updateHelp = s.installBlocked
    ? el("p", { class: "note warn", text: s.installBlocked })
    : s.installed && s.outdated
      ? el("p", {
          class: "note warn",
          text: s.otherHookPath
            ? t("These hooks run another copy of the hook, at {path}. Update them to use this app's own.", { path: s.otherHookPath })
            : kind === "claude"
              ? t("These hooks are from an older version. Update them to answer Claude Code's questions from the island.")
              : kind === "opencode"
                ? t("This plugin is from an older version. Update it, then restart OpenCode.")
                : t("These hooks are from an older version. Update them to get everything the island can do."),
        })
      : null;
  // The plan's usage reaches the island only through a statusLine of ours.
  const usageHelp =
    s.installed && s.statusLine === "none"
      ? el("p", {
          class: "note",
          text: t("Reinstall the hooks to see your plan's usage on the island: it adds a status line that Claude Code fills in and that shows nothing on its screen."),
        })
      : s.statusLine === "theirs"
        ? el("p", {
            class: "note",
            text: s.installed
              ? t("You have your own status line. Reinstall the hooks to see your plan's usage on the island too: yours keeps showing in Claude Code, and removing the hooks puts it back as it was.")
              : t("You have your own status line. Installing the hooks also brings your plan's usage to the island: yours keeps showing in Claude Code, and removing the hooks puts it back as it was."),
          })
        : null;
  const review = pending
    ? el(
        "div",
        { class: "review" },
        el("div", { class: "review-title", text: pending.install ? t("Review the install") : t("Review the removal") }),
        pending.preview.diff ? diff(pending.preview.diff) : el("p", { class: "note", text: t("Nothing to change.") }),
        el(
          "div",
          { class: "actions" },
          button(t("Cancel"), () => {
            panels.get(kind)!.pending = null;
            render();
          }),
          pending.preview.diff ? button(t("Write the file"), () => void apply(kind), true) : null,
        ),
      )
    : null;

  return el(
    "section",
    { class: "card" },
    el("div", { class: "card-head" }, el("div", { class: "card-title", text: name }), agentStatus(s)),
    el("div", { class: "path", text: s.configPath }),
    s.hookReady ? null : el("p", { class: "note error", text: t("The hook relay is missing at {path}.", { path: s.hookPath }) }),
    s.error ? el("p", { class: "note error", text: s.error }) : null,
    codexHelp,
    updateHelp,
    usageHelp,
    kind === "gemini"
      ? el("p", {
          class: "note",
          text: t("Gemini's hooks can't approve a tool, so it asks in its own terminal. The island shows what it is doing, and when it is waiting for you there."),
        })
      : kind === "antigravity"
        ? el("p", {
            class: "note",
            text: t("One hooks file for the agy CLI, the app and the IDE. Antigravity asks its permissions itself, and the island shows its sessions as antigravity."),
          })
        : kind === "opencode"
          ? el("p", {
              class: "note",
              text: t("OpenCode loads a plugin file instead of hooks: restart it after installing. Its permissions and questions show on the island too: answer there or in OpenCode, whichever comes first."),
            })
          : kind === "qwen"
            ? el("p", {
                class: "note",
                text: t("Restart Qwen Code after installing. Its permissions show on the island; Qwen asks in its own terminal once the card goes away unanswered. Its questions stay in its terminal."),
              })
            : null,
    notice,
    s.error || pending
      ? null
      : el(
          "div",
          { class: "actions" },
          s.installed ? button(setupWord(kind) === "plugin" ? t("Remove plugin…") : t("Remove hooks…"), () => void preview(kind, false)) : null,
          s.installBlocked
            ? null
            : // The white button is the step that is due: installing, updating, or a reinstall a note asks
              // for. A reinstall of hooks that are up to date is just there.
              button(
                setupWord(kind) === "plugin"
                  ? s.outdated ? t("Update plugin…") : s.installed ? t("Reinstall plugin…") : t("Install plugin…")
                  : s.outdated ? t("Update hooks…") : s.installed ? t("Reinstall hooks…") : t("Install hooks…"),
                () => void preview(kind, true),
                !s.installed || s.outdated || usageHelp !== null,
              ),
        ),
    review,
  );
}

/** What each agent can do (W1): built once, asked again each time the page is opened. */
const doctor = createDoctor((fresh) => Bridge.agentsDoctor(fresh));
let doctorAsked = false;

function agentsPage(): HTMLElement[] {
  if (!doctorAsked) {
    doctorAsked = true;
    doctor.refresh();
  }
  return [
    el("h1", { text: t("Agents") }),
    el("p", {
      class: "lede",
      text: t("Vults hears your agents through hooks in their config. Every change shows you the exact diff and takes a dated backup first; hooks from other tools are kept."),
    }),
    ...AGENTS.map((a) => agentCard(a.kind, a.name)),
    el("h2", { text: t("What each agent can do") }),
    el("p", { class: "lede", text: t("What this computer has of each agent, and what it can do with Vults: what the router and your policies lean on.") }),
    doctor.element,
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
    el("h1", { text: t("Connectors") }),
    el("p", { class: "lede", text: t("News from outside services on the island. Each one is off until you switch it on.") }),
    ...(presence === "paused" ? [el("p", { class: "note", text: t("Paused: connectors do not check anything until you pick another presence in General.") })] : []),
    ...CONNECTORS.map((c) => {
      const st = connectorStatus.get(c.id);
      const state = !st?.enabled
        ? badge(t("Off"), "off")
        : st.overBudget
          ? badge(t("Budget spent"), "warn")
          : st.error
          ? badge(t("Error"), "error")
          : st.lastOk
            ? badge(t("Watching {what}", { what: st.watching }), "ok")
            : badge(t("Checking…"), "warn");
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
        el("p", { class: "row-about", text: t(c.about) }),
        st?.enabled && st.lastOk && !st.error ? el("p", { class: "note", text: t("Last checked {when}.", { when: ago(st.lastOk) }) }) : null,
        st?.enabled && st.error ? el("p", { class: "note error", text: st.error }) : null,
        st?.enabled && st.overBudget
          ? el("p", { class: "note warn", text: t("It made as many checks as it may in a day, so it waits: it checks again once it has room.") })
          : null,
      );
    }),
  ];
}

// ── General ──────────────────────────────────────────────────────────────────

function generalPage(): HTMLElement[] {
  return [
    el("h1", { text: t("General") }),
    el(
      "section",
      { class: "card rows" },
      row(
        t("Language"),
        t("The language of the island, these settings, the tray and the notifications. System follows your desktop's."),
        dropdown(
          [{ value: null as string | null, label: t("System") }, ...APP_LANGUAGES.map((l) => ({ value: l.code as string | null, label: l.name }))],
          language,
          async (code) => {
            await Bridge.setLanguage(code);
            language = code;
          },
        ),
      ),
      row(
        t("Sounds"),
        t("Short 8-bit blips when a session needs you, finishes or fails, and for connector news."),
        toggle(sounds, async (on) => {
          await Bridge.setSounds(on);
          sounds = on;
          render();
        }),
      ),
      row(t("Volume"), t("How loud the sounds play. A cue plays when you let go of the slider."), slider(volume, !sounds)),
      row(
        t("Now playing"),
        t("Shows the song your music player is playing, with play, pause and skip, and Zeca dances to it. Read from your media players on this computer; nothing leaves it."),
        toggle(nowPlaying, async (on) => {
          await Bridge.setNowPlaying(on);
          nowPlaying = on;
        }),
      ),
      row(
        t("Codex usage"),
        t("Shows your Codex plan's usage on the island. Vults asks the codex you are logged into every 5 minutes, and each ask reaches OpenAI. Off, nothing is asked."),
        toggle(codexUsage, async (on) => {
          await Bridge.setCodexUsage(on);
          codexUsage = on;
        }),
      ),
      row(
        t("Fold the island"),
        t("How long the open island stays once the pointer leaves it. A permission keeps it open until you answer."),
        segmented(
          FOLD_CHOICES.map((n) => ({ value: n, label: `${n} s` })),
          foldAfter,
          async (n) => {
            await Bridge.setFoldAfter(n);
            foldAfter = n;
          },
        ),
      ),
      row(
        t("Open on hover"),
        t("Resting the pointer on the island opens it all the way, without a click. Opened that way it folds as soon as the pointer leaves, unless you clicked in it. Not in Panel."),
        toggle(openOnHover, async (on) => {
          await Bridge.setOpenOnHover(on);
          openOnHover = on;
        }),
      ),
      row(
        t("Presence"),
        t("Island: the flock at the top. Panel: Zeca in the tray. Quiet: only cards. In all three a card opens the island, with its sound. Paused: agents ask in their terminals, connectors and notifications stop. Also in the tray's menu."),
        segmented(
          [
            { value: "island" as Presence, label: t("Island") },
            { value: "panel" as Presence, label: t("Panel") },
            { value: "quiet" as Presence, label: t("Quiet") },
            { value: "paused" as Presence, label: t("Paused") },
          ],
          presence,
          async (p) => {
            await Bridge.setPresence(p);
            presence = p;
          },
        ),
      ),
      row(
        t("Corner widget"),
        t("A small window in a corner of the screen with up to three birds, the sessions that matter most, and how many work or need you. A click opens the island, on the card when one waits; it never answers one."),
        dropdown(
          [
            { value: null as Corner | null, label: t("Off") },
            { value: "top-left" as Corner | null, label: t("Top left") },
            { value: "top-right" as Corner | null, label: t("Top right") },
            { value: "bottom-left" as Corner | null, label: t("Bottom left") },
            { value: "bottom-right" as Corner | null, label: t("Bottom right") },
          ],
          widget,
          async (c) => {
            await Bridge.setWidget(c);
            widget = c;
          },
        ),
      ),
      row(
        t("Do not disturb"),
        dndUntil
          ? t("On until {time}: no sounds and no notifications, and no reminders for a waiting card. A card still opens the island with its sound.", { time: new Date(dndUntil * 1000).toLocaleTimeString(locale(), { hour: "2-digit", minute: "2-digit" }) })
          : t("No sounds and no notifications for a while. A card still opens the island with its sound, so an agent never waits for nobody. It ends by itself."),
        segmented(
          [{ value: 0, label: t("Off") }, ...DND_CHOICES.map((m) => ({ value: m, label: m < 60 ? t("{n} min", { n: m }) : t("{n} h", { n: m / 60 }) }))],
          // On, no length is marked: the time left is in the words.
          dndUntil ? -1 : 0,
          async (m) => {
            await Bridge.setDnd(m || null);
            dndUntil = m ? Math.floor(Date.now() / 1000) + m * 60 : null;
            render();
          },
        ),
      ),
      row(
        t("Notifications"),
        t("Only in Panel, where the island is out of sight: a desktop notification when a session finishes or fails, goes quiet, or a card waits for you. In Island and Quiet the island shows it all, so nothing goes to the desktop. Its only button opens the island; it never answers a card."),
        toggle(notifications, async (on) => {
          await Bridge.setNotifications(on);
          notifications = on;
        }),
      ),
      row(
        t("Screen"),
        t("Where the island sits. Automatic lets the desktop choose; a chosen screen that is unplugged hands the island back until it returns."),
        // A list, not buttons: screen names are long and a narrow window would break them.
        dropdown(
          [
            { value: null as string | null, label: t("Automatic") },
            ...monitors.map((m) => ({ value: m.name as string | null, label: m.label, title: m.name })),
            // Keep showing a saved screen that is unplugged right now.
            ...(monitor && !monitors.some((m) => m.name === monitor)
              ? [{ value: monitor as string | null, label: t("{name} (not connected)", { name: monitor }), title: monitor }]
              : []),
          ],
          monitor,
          async (m) => {
            await Bridge.setMonitor(m);
            monitor = m;
          },
        ),
      ),
      row(
        t("Start with the desktop"),
        t("Opens Vults when you log in."),
        toggle(autostart, async (on) => {
          await Bridge.setAutostart(on);
          autostart = on;
        }),
      ),
    ),
    el(
      "section",
      { class: "card rows" },
      row(t("Allow / Deny from anywhere"), t("Ctrl+Alt+Y and Ctrl+Alt+N answer the card on the island. Change the keys in System Settings → Shortcuts."), el("span", { class: "kbd", text: "Ctrl+Alt+Y · Ctrl+Alt+N" })),
      row(t("Talk to the chat"), t("Hold Ctrl+Alt+V to speak to Zeca from anywhere, once a voice model is set up in Chat & voice."), el("span", { class: "kbd", text: "Ctrl+Alt+V" })),
      row(t("Next / previous session"), t("Ctrl+Alt+J and Ctrl+Alt+K put the next or the previous session in front, in the flock's order. A card waiting for you stays in front."), el("span", { class: "kbd", text: "Ctrl+Alt+J · Ctrl+Alt+K" })),
      row(t("Open the island"), t("Ctrl+Alt+Space unfolds the island from anywhere, and hands it the keys until it folds: ↑ ↓ sessions, Enter the terminal, M actions, Y / N or a number to answer the card on screen, C the chat, Esc back to your window."), el("span", { class: "kbd", text: "Ctrl+Alt+Space" })),
      row(t("Change size"), t("Ctrl+Alt+1, 2 and 3 switch to Mini (the pill), Quick (the island) and Room (the Nest) from anywhere; Full is the Nest's own switch."), el("span", { class: "kbd", text: "Ctrl+Alt+1 · 2 · 3" })),
      row(t("The Nest's tabs"), t("In the Nest, Alt+1 to Alt+5 pick Flock, Tasks, Chat, Activity and Connections; Esc leaves Full."), el("span", { class: "kbd", text: "Alt+1–5" })),
    ),
  ];
}

// ── Chat ─────────────────────────────────────────────────────────────────────

function chatPage(): HTMLElement[] {
  const p = apiProviders.find((x) => x.id === apiSelected);
  const message = apiMessage ? el("p", { class: `note${apiMessage.error ? " error" : " ok"}`, text: apiMessage.text }) : null;
  const done = (text: string, error = false) => {
    apiMessage = { text, error };
    void refreshApi();
  };

  const select = document.createElement("select");
  select.className = "field select";
  const group = (label: string, local: boolean) => {
    const g = document.createElement("optgroup");
    g.label = label;
    for (const x of apiProviders.filter((x) => x.local === local)) {
      const o = new Option(x.hasKey ? t("{name} · key saved", { name: x.label }) : x.label, x.id, false, x.id === apiSelected);
      g.append(o);
    }
    return g;
  };
  select.append(group(t("Cloud, with your key"), false), group(t("On this machine"), true));
  select.addEventListener("change", () => {
    apiSelected = select.value;
    apiMessage = null;
    void Bridge.apiProviderSet(select.value).then(() => refreshApi());
  });

  const rows: (HTMLElement | null)[] = [
    row(t("Provider"), t("Where the API chat sends your messages."), select),
  ];
  if (p) {
    rows.push(row(...keyRow(p, done)));
    rows.push(row(...modelRow(p)));
  }
  rows.push(message ? el("div", { class: "row" }, message) : null);

  return [
    el("h1", { text: t("Chat & voice") }),
    ...(zeca ? [] : [el("p", { class: "note", text: t("Zeca is off (Flock & Zeca): no chat or voice until you turn him back on.") })]),
    el("p", {
      class: "lede",
      text: t("The chat on the island talks through the Claude Code or Codex CLI you are logged into, on your own subscription. It can also use a provider's API with your own key, or a model running on this machine."),
    }),
    el(
      "section",
      { class: "card rows" },
      row(
        t("Your Claude Code and Codex setup"),
        t("The chat's CLI reads your own setup, as in your terminal: your CLAUDE.md, skills, MCP servers, hooks and allow rules. A project's settings join only in a folder you already trust in Claude Code. Cards still ask before every command or edit your rules don't allow. Off, it reads none of it. Applies to the next conversation."),
        toggle(nativeChat, async (on) => {
          await Bridge.setNativeChat(on);
          nativeChat = on;
        }),
      ),
      row(
        t("Who answers Zeca's cards"),
        t("Asks first: a card before every command or edit. Your policies: the policies you accepted in Approvals answer what they cover, and the rest asks. Full access: everything runs without a card, but what a policy denies or asks first; once a conversation reads from outside (a web page, a fetch, an MCP tool), its cards ask again. Every answer is in the audit log. Holds at once."),
        segmented<ChatAccess>(
          [
            { value: "ask", label: t("Asks first") },
            { value: "policies", label: t("Your policies") },
            { value: "full", label: t("Full access") },
          ],
          chatAccess,
          async (v) => {
            await Bridge.setChatAccess(v);
            chatAccess = v;
            render();
          },
        ),
      ),
      ...(chatAccess === "full"
        ? [
            el("p", {
              class: "note warn",
              text: t("With full access Zeca runs commands, edits files and reaches the network without asking, destructive commands included. Add deny lines to a policy for what must never run."),
            }),
          ]
        : []),
    ),
    el("h2", { text: t("Memory") }),
    el(
      "section",
      { class: "card rows" },
      row(
        t("Zeca's memory"),
        t("Keeps a journal on this computer of what your agents and Zeca's chat did: each turn's time and changes, each card's answer with its command or path, and the chat's lines, secrets redacted. What he remembers joins your conversations with Zeca, and only that reaches their model. Off, none of it reaches a model and nothing more is written; Clear history in Activity removes the journal."),
        toggle(memory, async (on) => {
          await Bridge.setMemory(on);
          memory = on;
          render();
        }),
      ),
      ...(memory
        ? [
            row(
              t("A summary of each day"),
              t("Once a day, sends the day before's journal to Claude Code's smallest model (Haiku, on your subscription, with no tools) to write a short summary for Zeca to recall; only days from when you turn this on. A request on your behalf: at most 8 a day, each kept in the local trace."),
              toggle(journalSummary, async (on) => {
                await Bridge.setJournalSummary(on);
                journalSummary = on;
              }),
            ),
            row(
              t("Suggest what to remember"),
              t("When you start a new conversation or open another, sends the one you left to Claude Code's smallest model (Haiku, on your subscription, with no tools) to propose up to three facts worth remembering; you accept or turn down each in the Memory beside the Nest's Chat. Never one that read content from outside. A request on your behalf: at most 20 a day, each kept in the local trace."),
              toggle(memorySuggest, async (on) => {
                await Bridge.setMemorySuggest(on);
                memorySuggest = on;
              }),
            ),
            row(
              t("Let Zeca sleep on it"),
              t("Once a day, from two in the morning and while nothing is working, Zeca proposes removing facts he remembers twice. On, Claude Code's smallest model (Haiku, on your subscription, with no tools) also reads what he remembers about you and across projects and the last week's summaries, to propose merges, which wait for you in the Nest, and write a short debrief. A request on your behalf: at most 4 a day, each kept in the local trace."),
              toggle(sleep, async (on) => {
                await Bridge.setSleep(on);
                sleep = on;
              }),
            ),
            row(
              t("Recall by meaning"),
              t("Recall also by meaning, not only by words, through an Ollama running on this computer with nomic-embed-text. Each summary and each message you send is embedded on 127.0.0.1; nothing leaves this computer."),
              toggle(localEmbeddings, async (on) => {
                await Bridge.setLocalEmbeddings(on);
                localEmbeddings = on;
              }),
            ),
          ]
        : []),
    ),
    el("h2", { text: t("API") }),
    el("section", { class: "card rows" }, ...rows),
    ...(web ? [el("h2", { text: t("Web") }), el("section", { class: "card rows" }, ...webRows())] : []),
    el("h2", { text: t("Voice") }),
    el("section", { class: "card rows" }, ...voiceRows()),
    el(
      "section",
      { class: "card" },
      el(
        "p",
        { class: "note" },
        document.createTextNode(
          t("The API chat can't run commands, edit files or open your project. It reads the files you drop on the island and, when a question needs it, what the island shows (your sessions, the cards waiting, usage, your GitHub card), which then goes to the provider with the conversation. Keys are kept in the system keyring, never in a file, and only ever sent to their own provider."),
        ),
      ),
    ),
  ];
}

/** Speak to Zeca: a model to download once, then a mic in the chat. */
function voiceRows(): HTMLElement[] {
  if (!voice) return [];
  const status = voice;
  const intro = row(
    t("Talk to the chat"),
    t("Hold a conversation by voice: the mic in the chat records you, and the words land in the input for you to check before sending. It is transcribed on this computer by whisper.cpp; the audio never leaves it and is never saved."),
    status.ready
      ? button(t("Turn off"), async () => {
          await Bridge.voiceOff();
          await refreshVoice();
        })
      : badge(t("Off"), "off"),
  );
  const named = (code: string) => LANGUAGES.find((l) => l.value === code)?.label ?? code;
  const language = row(
    t("Language you speak"),
    t("Telling whisper the language makes short phrases far more reliable than detecting it."),
    dropdown<string | null>(
      [
        { value: null, label: status.system ? t("System ({name})", { name: named(status.system) }) : t("System") },
        { value: "auto", label: t("Detect it each time") },
        ...LANGUAGES,
      ],
      status.language,
      async (code) => {
        await Bridge.voiceLanguageSet(code);
        await refreshVoice();
      },
    ),
  );
  const models = status.models.map((m) => {
    const mb = t("{n} MB", { n: Math.round(m.size / 1_000_000) });
    let control: HTMLElement;
    const progress = downloading.get(m.id);
    if (progress !== undefined) control = el("span", { class: "muted", text: t("Downloading… {pct}%", { pct: progress }) });
    else if (m.installed && status.selected === m.id && status.ready) control = badge(t("In use"), "ok");
    else if (m.installed)
      control = button(t("Use"), async () => {
        await Bridge.voiceSelect(m.id);
        await refreshVoice();
      });
    else
      control = button(
        t("Download {size}", { size: mb }),
        async () => {
          downloading.set(m.id, 0);
          voiceError = null;
          render();
          try {
            await Bridge.voiceDownload(m.id);
          } catch (e) {
            voiceError = String(e);
          }
          downloading.delete(m.id);
          await refreshVoice();
        },
        !status.models.some((x) => x.installed) && m.id === "base",
      );
    return row(m.label, m.installed ? t("{size}, on this computer", { size: mb }) : t("{size} from the whisper.cpp models on Hugging Face, checked before use", { size: mb }), control);
  });
  return [intro, language, ...models, ...(voiceError ? [el("p", { class: "note error", text: voiceError })] : [])];
}

/** Languages whisper knows well, in their own names. */
const LANGUAGES = [
  { value: "pt", label: "Português" }, // check-english:allow (each language in its own name)
  { value: "en", label: "English" },
  { value: "es", label: "Español" }, // check-english:allow
  { value: "fr", label: "Français" }, // check-english:allow
  { value: "de", label: "Deutsch" },
  { value: "it", label: "Italiano" },
];

async function refreshVoice(): Promise<void> {
  voice = await Bridge.voiceStatus();
  // A download this window did not start (reopened Settings) still shows as one.
  for (const id of voice.downloading) if (!downloading.has(id)) downloading.set(id, 0);
  render();
}

/** The key: saved (Remove), missing (a field), or not needed for a local server. */
function keyRow(p: ApiProvider, done: (text: string, error?: boolean) => void): [string, string, HTMLElement] {
  if (p.local) {
    return [t("Key"), t("None needed: it runs on this machine and nothing leaves it."), badge(t("Local"), "ok")];
  }
  if (p.hasKey) {
    return [
      t("{provider} API key", { provider: p.label }),
      t("Saved in the system keyring."),
      button(t("Remove"), () => {
        Bridge.apiKeyClear(p.id)
          .then(() => done(t("Key removed from the keyring.")))
          .catch((e) => done(String(e), true));
      }),
    ];
  }
  const input = document.createElement("input");
  input.type = "password";
  input.className = "field";
  input.placeholder = p.keyHint ? `${p.keyHint}…` : t("Paste the key");
  input.autocomplete = "off";
  input.spellcheck = false;
  const save = () => {
    Bridge.apiKeySet(p.id, input.value)
      .then(() => done(t("Saved. Choose the API in the chat to use it.")))
      .catch((e) => done(String(e), true));
  };
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") save();
  });
  return [
    t("{provider} API key", { provider: p.label }),
    t("Kept in the system keyring, never in a file. Usage is billed to your account there."),
    el("div", { class: "inline" }, input, button(t("Save"), save, true)),
  ];
}

/** The model, from the provider's live list once it can be asked. */
function modelRow(p: ApiProvider): [string, string, HTMLElement] {
  const list = apiModels.get(p.id);
  if (!p.local && !p.hasKey) return [t("Model"), t("Listed live from the provider once a key is saved."), badge(t("No key"), "off")];
  if (list === undefined) {
    void loadModels(p.id);
    return [t("Model"), t("Asking {provider} for its models…", { provider: p.label }), badge(t("Loading"), "warn")];
  }
  if ("error" in list) {
    return [t("Model"), list.error, button(t("Try again"), () => void loadModels(p.id))];
  }
  const select = document.createElement("select");
  select.className = "field select";
  const choices = p.model && !list.models.includes(p.model) ? [p.model, ...list.models] : list.models;
  if (!p.model) select.append(new Option(t("Choose a model"), "", true, true));
  for (const m of choices) select.append(new Option(m, m, false, m === p.model));
  select.addEventListener("change", () => {
    void Bridge.apiModelSet(p.id, select.value).then(() => refreshApi());
  });
  const listed = t("Listed live from {provider}: {n} models.", { provider: p.label, n: list.models.length });
  const missing = p.model && !list.models.includes(p.model) ? ` ${t("{model} is no longer listed.", { model: p.model })}` : "";
  return [t("Model"), `${listed}${missing}`, select];
}

async function loadModels(id: string): Promise<void> {
  if (apiLoading.has(id)) return;
  apiLoading.add(id);
  try {
    apiModels.set(id, { models: await Bridge.apiModels(id) });
  } catch (e) {
    apiModels.set(id, { error: String(e) });
  }
  apiLoading.delete(id);
  if (page === "chat") render();
}

async function refreshApi(): Promise<void> {
  try {
    const r = await Bridge.apiProviders();
    // A key saved or removed changes what the list can say.
    for (const x of r.providers) {
      const before = apiProviders.find((y) => y.id === x.id);
      if (before && before.hasKey !== x.hasKey) apiModels.delete(x.id);
    }
    apiProviders = r.providers;
    apiSelected = r.selected;
  } catch {
    apiProviders = [];
  }
  try {
    web = await Bridge.webSettings();
  } catch {
    web = null;
  }
  if (page === "chat") render();
}

/** Zeca's web: off until turned on, an engine, its key or address. */
function webRows(): HTMLElement[] {
  if (!web) return [];
  const w = web;
  const said = (text: string, error = false) => {
    webMessage = { text, error };
    void refreshApi();
  };
  const rows: HTMLElement[] = [
    row(
      t("Web for Zeca"),
      t("Lets the API chat search the web and read pages when a question needs it. Each search and page read is a request on your behalf, to the engine you choose and to the page's site: at most 200 a day, each kept in the local trace. A page is read by a separate model call with no tools, and only its answer reaches Zeca; search results reach him as short snippets marked untrusted."),
      toggle(w.on, async (on) => {
        await Bridge.setWeb(on);
        w.on = on;
      }),
    ),
  ];
  const engine = document.createElement("select");
  engine.className = "field select";
  for (const [id, label] of [["brave", "Brave Search"], ["tavily", "Tavily"], ["searxng", "SearXNG"]] as [WebEngine, string][]) {
    engine.append(new Option(label, id, false, id === w.engine));
  }
  engine.addEventListener("change", () => {
    webMessage = null;
    void Bridge.setWebEngine(engine.value as WebEngine)
      .then(() => refreshApi())
      .catch((e) => {
        webMessage = { text: String(e), error: true };
        void refreshApi();
      });
  });
  rows.push(row(t("Search engine"), t("Where searches go. SearXNG is one you run yourself."), engine));
  if (w.engine === "searxng") {
    const input = document.createElement("input");
    input.className = "field";
    input.placeholder = "https://search.example.com";
    input.value = w.searxngUrl ?? "";
    input.spellcheck = false;
    const save = () =>
      Bridge.setSearxngUrl(input.value)
        .then(() => said(t("Saved.")))
        .catch((e) => said(String(e), true));
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") void save();
    });
    rows.push(row(t("Your SearXNG"), t("Its address; it must answer in JSON (format=json)."), el("div", { class: "inline" }, input, button(t("Save"), () => void save(), true))));
  } else if (w.hasKey) {
    rows.push(
      row(
        t("Search key"),
        t("Saved in the system keyring."),
        button(t("Remove"), () => {
          Bridge.clearWebKey(w.engine)
            .then(() => said(t("Key removed from the keyring.")))
            .catch((e) => said(String(e), true));
        }),
      ),
    );
  } else {
    const input = document.createElement("input");
    input.type = "password";
    input.className = "field";
    input.placeholder = t("Paste the key");
    input.autocomplete = "off";
    input.spellcheck = false;
    const save = () =>
      Bridge.setWebKey(w.engine, input.value)
        .then(() => said(t("Saved.")))
        .catch((e) => said(String(e), true));
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter") void save();
    });
    rows.push(row(t("Search key"), t("Kept in the system keyring, never in a file, and only ever sent to that engine."), el("div", { class: "inline" }, input, button(t("Save"), () => void save(), true))));
  }
  if (webMessage) rows.push(el("div", { class: "row" }, el("p", { class: `note${webMessage.error ? " error" : " ok"}`, text: webMessage.text })));
  return rows;
}

// ── About ────────────────────────────────────────────────────────────────────

function aboutPage(): HTMLElement[] {
  return [
    el("h1", { text: t("About") }),
    el(
      "section",
      { class: "card about" },
      el("img", { class: "logo" }),
      el(
        "div",
        {},
        el("div", { class: "card-title", text: "Vults" }),
        el("p", { class: "row-about", text: t("A friendly flock watching your coding agents. MIT licensed; no telemetry.") }),
        version ? el("p", { class: "note", text: t("Version {version}", { version }) }) : null,
      ),
    ),
    el(
      "section",
      { class: "card rows" },
      row(t("Settings"), t("Your settings file."), el("code", { text: settingsPath })),
      row(t("Data"), t("The hook relay, the inbox of dropped files, connector state."), el("code", { text: dataPath })),
    ),
  ];
}

// ── Flock ────────────────────────────────────────────────────────────────────

const FLOCKS: { value: Flock; label: string }[] = [
  { value: "brazil", label: tk("Brazil") },
  { value: "americas", label: tk("The Americas") },
  { value: "world", label: tk("The world") },
];

/** The previews' timer: only while the page is on screen. */
let previews: number | undefined;

function flockPage(): HTMLElement[] {
  const canvases: { id: string; canvas: HTMLCanvasElement }[] = [];
  // Each preview perches on the same wire line, so their sizes compare at a glance.
  const SCALE = 2, W = 44, WIRE = 36;
  const card = (id: string, name: string, latin: string): HTMLElement => {
    const canvas = document.createElement("canvas");
    canvas.width = W * SCALE;
    canvas.height = (WIRE + 4) * SCALE;
    canvases.push({ id, canvas });
    const button = el(
      "button",
      {
        class: `species${id === zecaSpecies ? " on" : ""}`,
        onclick: () => {
          if (id === zecaSpecies) return;
          void Bridge.setZecaSpecies(id).then(
            () => {
              zecaSpecies = id;
              render();
            },
            () => render(),
          );
        },
      },
      canvas,
      el("span", { class: "species-name", text: name }),
      el("span", { class: "species-latin", text: latin }),
    );
    button.setAttribute("aria-pressed", String(id === zecaSpecies));
    return button;
  };
  const group = (family: string, title: string) => [
    el("h2", { text: title }),
    el("div", { class: "species-grid" }, ...SPECIES.filter((s) => s.family === family).map((s) => card(s.id, s.name, s.latin))),
  ];
  const shown = new WeakMap<HTMLCanvasElement, Frame>();
  const paint = () => {
    // A hidden window draws nothing; the next tick catches up.
    if (document.hidden) return;
    const t = performance.now();
    for (const { id, canvas } of canvases) {
      const set = speciesSet(id);
      // Idle frames last hundreds of ms: most ticks change nothing, and a repaint is a rect per cell.
      const frame = frameAt(set.clips.idle, t);
      if (shown.get(canvas) === frame) continue;
      shown.set(canvas, frame);
      const ctx = canvas.getContext("2d")!;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      drawFrame(ctx, set, frame, 10, WIRE - perchOf(set), SCALE);
    }
  };
  // One timer at a time: every render of this page replaces it.
  window.clearInterval(previews);
  previews = undefined;
  paint();
  if (!matchMedia("(prefers-reduced-motion: reduce)").matches) previews = window.setInterval(paint, 100);
  return [
    el("h1", { text: t("Flock & Zeca") }),
    el(
      "section",
      { class: "card rows" },
      row(
        t("The flock draws from"),
        t("Where the sessions' birds come from: Brazil's vultures, the vultures of the Americas (with both condors), or every vulture in the world. Each project is one species, the same every time the app starts, and the projects on the wire differ while there are species left. A project with three or more sessions gets a king vulture either way."),
        segmented(FLOCKS.map((f) => ({ ...f, label: t(f.label) })), flock, async (f) => {
          await Bridge.setFlock(f);
          flock = f;
        }),
      ),
      row(
        t("Rare visitors"),
        t("Now and then, while sessions are open, a vulture from outside your flock (a condor, a griffon) crosses the sky once and goes on its way. It never lands."),
        toggle(visitors, async (on) => {
          await Bridge.setVisitors(on);
          visitors = on;
        }),
      ),
    ),
    el("h2", { text: "Zeca" }),
    el(
      "section",
      { class: "card rows" },
      row(
        "Zeca",
        t("The companion who chats and listens. Off, the flock, cards, notifications and connectors work as ever, with no chat, microphone or talk shortcut; the session in front keeps its own bird and an empty wire stays empty."),
        toggle(zeca, async (on) => {
          await Bridge.setZeca(on);
          zeca = on;
          render();
        }),
      ),
      row(
        t("Look"),
        t("Auto dresses Zeca for the season: a witch hat from October 1 to November 1, a Santa hat from December 1 to 26, a party hat from New Year's Eve to January 2, bunny ears from Good Friday to Easter Monday. Sunglasses and the other outfits only when you pick them. Only Zeca wears it; the flock keeps its feathers."),
        dropdown(LOOKS, zecaLook, async (look) => {
          await Bridge.setZecaLook(look);
          zecaLook = look;
        }),
      ),
    ),
    el("p", {
      class: "note",
      text: zeca
        ? t("Zeca is the bird in front: the session that needs you, or the one you picked. Choose his species.")
        : t("Zeca is off: the session in front keeps its own bird. His species and look wait for him here."),
    }),
    ...group("new-world", t("Vultures of the Americas")),
    ...group("old-world", t("Vultures of Africa, Europe and Asia")),
  ];
}

// ── Layout ───────────────────────────────────────────────────────────────────

/** The page last drawn: a re-render of it keeps the scroll, a new page starts at the top. */
let drawn: Page | null = null;

/** Draws the page again when it is the phone's (its rows refresh on their own). */
function renderPhone(): void {
  if (page === "phone") render();
}

function render(): void {
  head.setZeca(zeca);
  head.setSound(sounds);
  foot.setZeca(zeca);
  foot.setSpecies(zecaSpecies);
  const content =
    page === "general"
      ? generalPage()
      : page === "agents"
        ? agentsPage()
        : page === "chat"
          ? chatPage()
          : page === "approvals"
            ? approvalsPage()
            : page === "projects"
              ? projectsPage()
            : page === "activity"
              ? activityPage(activity, { tab: gridTab, github, confirming: clearing, hideProjects, saved }, activityActions)
            : page === "connectors"
              ? connectorsPage()
            : page === "phone"
              ? phonePage()
              : page === "flock"
                ? flockPage()
                : aboutPage();
  if (page !== "flock") window.clearInterval(previews);
  const navButton = (p: (typeof PAGES)[number]) => {
    const b = el(
      "button",
      {
        class: `nav${p.id === page ? " on" : ""}`,
        onclick: () => {
          page = p.id;
          navigated = true;
          location.hash = p.id;
          apiMessage = null;
          if (p.id === "agents") doctorAsked = false;
          if (p.id === "approvals") {
            void refreshRules();
            void refreshPolicies(() => {
              if (page === "approvals") render();
            });
          }
          if (p.id === "projects") void refreshProjects();
          if (p.id === "activity") {
            clearing = false;
            void refreshActivity();
          }
          if (p.id === "chat") void refreshApi();
          if (p.id === "phone") void refreshPhone(renderPhone);
          render();
        },
      },
      icon(p.icon, 14, 1.6),
      el("span", { text: t(p.label) }),
    );
    if (p.id === page) b.setAttribute("aria-current", "page");
    return b;
  };
  // The Nest's header above already says whose settings these are: groups, then pages.
  const nav = el(
    "nav",
    { class: "sidebar" },
    ...GROUPS.flatMap((g) => [el("p", { class: "nav-group", text: t(g.label) }), ...PAGES.filter((p) => p.group === g.id).map(navButton)]),
  );
  const scroll = page === drawn ? (root.querySelector("main.page")?.scrollTop ?? 0) : 0;
  const main = el("main", { class: "page" }, el("div", { class: "page-kicker", text: t("Settings") }), ...content);
  root.replaceChildren(nav, main);
  main.scrollTop = scroll;
  drawn = page;
  const logo = root.querySelector<HTMLImageElement>("img.logo");
  if (logo) logo.src = "/icon.png";
}

render();
// By the panel the open island may cover this window's corner: a click here folds it, as a click
// outside a panel's popup closes it (the island keeps a waiting card).
window.addEventListener("pointerdown", () => void Bridge.away().catch(() => {}), { capture: true });
if (page === "activity") void refreshActivity();
if (page === "phone") void refreshPhone(renderPhone);
Bridge.onPhone(phoneEvent);
void Bridge.appSettings().then((s) => {
  sounds = s.sounds;
  volume = savedVolume = s.volume;
  autostart = s.autostart;
  // The nearest choice: the file may hold any number in range.
  foldAfter = FOLD_CHOICES.reduce((a, b) => (Math.abs(b - s.foldAfter) < Math.abs(a - s.foldAfter) ? b : a));
  openOnHover = s.openOnHover;
  language = s.language;
  setLang(s.lang);
  head.relabel();
  foot.relabel();
  monitor = s.monitor;
  nowPlaying = s.nowPlaying;
  codexUsage = s.codexUsage;
  nativeChat = s.nativeChat;
  chatAccess = s.chatAccess;
  memory = s.memory;
  journalSummary = s.journalSummary;
  localEmbeddings = s.localEmbeddings;
  memorySuggest = s.memorySuggest;
  sleep = s.sleep;
  zecaSpecies = s.zecaSpecies;
  zecaLook = s.zecaLook;
  flock = s.flock;
  visitors = s.visitors;
  presence = s.presence;
  notifications = s.notifications;
  dndUntil = s.dndUntil;
  zeca = s.zeca;
  widget = s.widget;
  settingsPath = s.settingsPath;
  dataPath = s.dataPath;
  render();
});
void refreshVoice();
Bridge.onVoiceDownload((p) => {
  const percent = Math.floor((p.done / p.total) * 100);
  // Thousands of chunks: only a new percent repaints.
  if (!downloading.has(p.id) || downloading.get(p.id) === percent) return;
  downloading.set(p.id, percent);
  render();
});
const refreshMonitors = () =>
  Bridge.monitors()
    .then((m) => {
      monitors = m;
      if (page === "general") render();
    })
    .catch(() => {});
void refreshMonitors();
// The tray asks an open window for one page (Set up agents…).
Bridge.onSettingsSection((section) => {
  const p = asked(section);
  if (!p) return;
  navigated = true;
  page = p;
  location.hash = p;
  if (p === "agents") doctorAsked = false;
  render();
});
// The island's speaker button changes the sounds too: keep the toggle and the slider in step.
Bridge.onSettings((s) => {
  if (s.lang !== undefined) {
    setLang(s.lang);
    head.relabel();
    foot.relabel();
    if (s.language !== undefined) language = s.language;
    render();
  }
  if (s.zeca !== undefined) {
    zeca = s.zeca;
    render();
  }
  if (s.zecaLook !== undefined && s.zecaLook !== zecaLook) {
    zecaLook = s.zecaLook;
    render();
  }
  if (s.presence !== undefined) {
    presence = s.presence;
    render();
  }
  // The chat's own picker gave some access back.
  if (s.chatAccess !== undefined && s.chatAccess !== chatAccess) {
    chatAccess = s.chatAccess;
    if (page === "chat") render();
  }
  if (s.dndUntil !== undefined) {
    dndUntil = s.dndUntil;
    if (page === "general") render();
  }
  // A quick action on the island changed a project.
  if (s.projects !== undefined) {
    projects = s.projects;
    if (page === "projects") render();
  }
  if (s.sounds === undefined && s.volume === undefined) return;
  if (s.sounds !== undefined) sounds = s.sounds;
  if (s.volume !== undefined) volume = savedVolume = s.volume;
  render();
});
Bridge.onMonitors(() => void refreshMonitors());
for (const a of AGENTS) void refresh(a.kind);
void refreshConnectors();
void refreshRules();
void refreshPolicies(() => {
  if (page === "approvals") render();
});
void refreshProjects();
void refreshApi();
void getVersion()
  .then((v) => {
    version = v;
    if (page === "about") render();
  })
  .catch(() => {});
// Statuses age and polls finish in the background; a hidden window reads them when it shows.
window.setInterval(() => {
  if (!document.hidden) void refreshConnectors();
}, 15_000);
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) void refreshConnectors();
});
// An agent's files change outside the app (a relay built, a hook edited by hand): read them again
// when the window comes back, unless a review is open (it would lose its diff).
window.addEventListener("focus", () => {
  for (const a of AGENTS) if (!panels.get(a.kind)?.pending) void refresh(a.kind);
});
