// The settings window: a sidebar and one page per section. Installing hooks always goes through a
// diff the user reviews first.
import { getVersion } from "@tauri-apps/api/app";
import { Bridge, type ApiProvider, type ConnectorStatus, type Flock, type InstallAgent, type InstallPreview, type InstallStatus, type Corner, type Presence, type ProjectPrefs, type Rule, type VoiceStatus } from "../../bridge";
import { SPECIES, speciesSet } from "../../character/flock";
import { LOOK_GROUPS } from "../../character/looks";
import { drawFrame, frameAt } from "../../character/sprites";
import { perchOf } from "../../character/zeca";
import { CONNECTORS } from "../../connectors";
import { el } from "../../dom";
import { Sound } from "../../sound";

type Page = "general" | "agents" | "chat" | "approvals" | "projects" | "connectors" | "flock" | "about";

const PAGES: { id: Page; label: string }[] = [
  { id: "general", label: "General" },
  { id: "agents", label: "Agents" },
  { id: "chat", label: "Chat" },
  { id: "approvals", label: "Approvals" },
  { id: "projects", label: "Projects" },
  { id: "connectors", label: "Connectors" },
  { id: "flock", label: "Flock" },
  { id: "about", label: "About" },
];

const AGENTS: { kind: InstallAgent; name: string }[] = [
  { kind: "claude", name: "Claude Code" },
  { kind: "codex", name: "Codex" },
  { kind: "gemini", name: "Gemini CLI" },
  { kind: "antigravity", name: "Antigravity" },
  { kind: "opencode", name: "OpenCode" },
];

/** What the installer writes for an agent: hooks in its config, or a plugin file of ours. */
const setupWord = (kind: InstallAgent): string => (kind === "opencode" ? "plugin" : "hooks");

interface Panel {
  status: InstallStatus | null;
  message: { text: string; error: boolean } | null;
  pending: { install: boolean; preview: InstallPreview } | null;
}

const root = document.getElementById("settings")!;
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
let voice: VoiceStatus | null = null;
/** Model id → percent downloaded, for every download running. */
const downloading = new Map<string, number>();
let voiceError: string | null = null;
let autostart = false;
let foldAfter = 15;
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
                el("div", { class: "row-about", text: `${AGENTS.find(a => a.kind === r.agent)?.name ?? r.agent} · ${name(r.cwd)} · ${r.cwd}` }),
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
    el("h1", { text: "Projects" }),
    el("p", {
      class: "lede",
      text: "Choices kept per project folder, for every session in it now and later. Right-click a session on the island to mute, pin or hide its project. Muted: no sounds and no notifications when its sessions finish or fail; a card keeps both. Pinned: its sessions come first. Hidden: its sessions stay off the island, but a card from one still shows.",
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
                choice("Muted", !!p.mute, set(cwd, { mute: true })),
                choice("Pinned", !!p.pin, set(cwd, { pin: true })),
                choice("Hidden", !!p.hide, set(cwd, { hide: true })),
                button("Forget", () => {
                  void Bridge.projectSet(cwd, {})
                    .catch(() => {})
                    .finally(() => void refreshProjects());
                }),
              ),
            );
          }),
        )
      : el("section", { class: "card" }, el("p", { class: "note", text: "No project has a choice yet. Every session shows, in the order it arrived, with its sounds." })),
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
  input.setAttribute("aria-label", "Volume");
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
    const word = setupWord(kind);
    const done = `${word[0].toUpperCase()}${word.slice(1)} ${install ? "installed" : "removed"}.`;
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
  if (s.outdated) return badge("Update available", "warn");
  return badge("Installed", "ok");
}

function agentCard(kind: InstallAgent, name: string): HTMLElement {
  const { status: s, message, pending } = panels.get(kind)!;
  const notice = message ? el("p", { class: message.error ? "note error" : "note ok", text: message.text }) : null;
  if (!s) return el("section", { class: "card" }, el("div", { class: "card-title", text: name }), notice);

  const codexHelp =
    s.codex && s.installed && !s.codex.hooksDisabled && s.codex.untrusted > 0
      ? el("p", {
          class: "note warn",
          text: `Codex runs a hook only once you trust it: open Codex, type /hooks and trust the ${s.codex.untrusted} Vults hooks waiting there.`,
        })
      : null;
  // Installing would be refused (`install_blocked` in installer.rs): say why instead of offering it.
  const updateHelp = s.installBlocked
    ? el("p", { class: "note warn", text: s.installBlocked })
    : s.installed && s.outdated
      ? el("p", {
          class: "note warn",
          text: s.otherHookPath
            ? `These hooks run another copy of the hook, at ${s.otherHookPath}. Update them to use this app's own.`
            : kind === "claude"
              ? "These hooks are from an older version. Update them to answer Claude Code's questions from the island."
              : kind === "opencode"
                ? "This plugin is from an older version. Update it, then restart OpenCode."
                : "These hooks are from an older version. Update them to get everything the island can do.",
        })
      : null;
  // The plan's usage reaches the island only through a statusLine of ours.
  const usageHelp =
    s.installed && s.statusLine === "none"
      ? el("p", {
          class: "note",
          text: "Reinstall the hooks to see your plan's usage on the island: it adds a status line that Claude Code fills in and that shows nothing on its screen.",
        })
      : s.statusLine === "theirs"
        ? el("p", {
            class: "note",
            text: s.installed
              ? "You have your own status line. Reinstall the hooks to see your plan's usage on the island too: yours keeps showing in Claude Code, and removing the hooks puts it back as it was."
              : "You have your own status line. Installing the hooks also brings your plan's usage to the island: yours keeps showing in Claude Code, and removing the hooks puts it back as it was.",
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
    updateHelp,
    usageHelp,
    kind === "gemini"
      ? el("p", {
          class: "note",
          text: "Gemini's hooks can't approve a tool, so it asks in its own terminal. The island shows what it is doing, and when it is waiting for you there.",
        })
      : kind === "antigravity"
        ? el("p", {
            class: "note",
            text: "One hooks file for the agy CLI, the app and the IDE. Antigravity asks its permissions itself, and the island shows its sessions as antigravity.",
          })
        : kind === "opencode"
          ? el("p", {
              class: "note",
              text: "OpenCode loads a plugin file instead of hooks: restart it after installing. Its permissions and questions show on the island too: answer there or in OpenCode, whichever comes first.",
            })
          : null,
    notice,
    s.error || pending
      ? null
      : el(
          "div",
          { class: "actions" },
          s.installed ? button(`Remove ${setupWord(kind)}…`, () => void preview(kind, false)) : null,
          s.installBlocked
            ? null
            : // The white button is the step that is due: installing, updating, or a reinstall a note asks
              // for. A reinstall of hooks that are up to date is just there.
              button(
                `${s.outdated ? "Update" : s.installed ? "Reinstall" : "Install"} ${setupWord(kind)}…`,
                () => void preview(kind, true),
                !s.installed || s.outdated || usageHelp !== null,
              ),
        ),
    review,
  );
}

function agentsPage(): HTMLElement[] {
  return [
    el("h1", { text: "Agents" }),
    el("p", {
      class: "lede",
      text: "Vults hears your agents through hooks in their config. Every change shows you the exact diff and takes a dated backup first; hooks from other tools are kept.",
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
    ...(presence === "paused" ? [el("p", { class: "note", text: "Paused: connectors do not check anything until you pick another presence in General." })] : []),
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
          render();
        }),
      ),
      row("Volume", "How loud the sounds play. A cue plays when you let go of the slider.", slider(volume, !sounds)),
      row(
        "Now playing",
        "Shows the song your music player is playing, with play, pause and skip, and Zeca dances to it. Read from your media players on this computer; nothing leaves it.",
        toggle(nowPlaying, async (on) => {
          await Bridge.setNowPlaying(on);
          nowPlaying = on;
        }),
      ),
      row(
        "Fold the island",
        "How long the open island stays once the pointer leaves it. A permission keeps it open until you answer.",
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
        "Presence",
        "Island: the flock at the top. Panel: Zeca in the tray. Quiet: only cards. In all three a card opens the island, with its sound. Paused: agents ask in their terminals, connectors and notifications stop. Also in the tray's menu.",
        segmented(
          [
            { value: "island" as Presence, label: "Island" },
            { value: "panel" as Presence, label: "Panel" },
            { value: "quiet" as Presence, label: "Quiet" },
            { value: "paused" as Presence, label: "Paused" },
          ],
          presence,
          async (p) => {
            await Bridge.setPresence(p);
            presence = p;
          },
        ),
      ),
      row(
        "Corner widget",
        "A small window in a corner of the screen with up to three birds, the sessions that matter most, and how many work or need you. A click opens the island, on the card when one waits; it never answers one.",
        dropdown(
          [
            { value: null as Corner | null, label: "Off" },
            { value: "top-left" as Corner | null, label: "Top left" },
            { value: "top-right" as Corner | null, label: "Top right" },
            { value: "bottom-left" as Corner | null, label: "Bottom left" },
            { value: "bottom-right" as Corner | null, label: "Bottom right" },
          ],
          widget,
          async (c) => {
            await Bridge.setWidget(c);
            widget = c;
          },
        ),
      ),
      row(
        "Do not disturb",
        dndUntil
          ? `On until ${new Date(dndUntil * 1000).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" })}: no sounds and no notifications, and no reminders for a waiting card. A card still opens the island with its sound.`
          : "No sounds and no notifications for a while. A card still opens the island with its sound, so an agent never waits for nobody. It ends by itself.",
        segmented(
          [{ value: 0, label: "Off" }, ...DND_CHOICES.map((m) => ({ value: m, label: m < 60 ? `${m} min` : `${m / 60} h` }))],
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
        "Notifications",
        "Only in Panel, where the island is out of sight: a desktop notification when a session finishes or fails, goes quiet, or a card waits for you. In Island and Quiet the island shows it all, so nothing goes to the desktop. Its only button opens the island; it never answers a card.",
        toggle(notifications, async (on) => {
          await Bridge.setNotifications(on);
          notifications = on;
        }),
      ),
      row(
        "Screen",
        "Where the island sits. Automatic lets the desktop choose; a chosen screen that is unplugged hands the island back until it returns.",
        // A list, not buttons: screen names are long and a narrow window would break them.
        dropdown(
          [
            { value: null as string | null, label: "Automatic" },
            ...monitors.map((m) => ({ value: m.name as string | null, label: m.label, title: m.name })),
            // Keep showing a saved screen that is unplugged right now.
            ...(monitor && !monitors.some((m) => m.name === monitor)
              ? [{ value: monitor as string | null, label: `${monitor} (not connected)`, title: monitor }]
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
        "Start with the desktop",
        "Opens Vults when you log in.",
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
      row("Talk to the chat", "Hold Ctrl+Alt+V to speak to Zeca from anywhere, once a voice model is set up in Chat.", el("span", { class: "kbd", text: "Ctrl+Alt+V" })),
      row("Next / previous session", "Ctrl+Alt+J and Ctrl+Alt+K put the next or the previous session in front, in the flock's order. A card waiting for you stays in front.", el("span", { class: "kbd", text: "Ctrl+Alt+J · Ctrl+Alt+K" })),
      row("Open the island", "Ctrl+Alt+Space unfolds the island from anywhere.", el("span", { class: "kbd", text: "Ctrl+Alt+Space" })),
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
      const o = new Option(x.hasKey ? `${x.label} · key saved` : x.label, x.id, false, x.id === apiSelected);
      g.append(o);
    }
    return g;
  };
  select.append(group("Cloud, with your key", false), group("On this machine", true));
  select.addEventListener("change", () => {
    apiSelected = select.value;
    apiMessage = null;
    void Bridge.apiProviderSet(select.value).then(() => refreshApi());
  });

  const rows: (HTMLElement | null)[] = [
    row("Provider", "Where the API chat sends your messages.", select),
  ];
  if (p) {
    rows.push(row(...keyRow(p, done)));
    rows.push(row(...modelRow(p)));
  }
  rows.push(message ? el("div", { class: "row" }, message) : null);

  return [
    el("h1", { text: "Chat" }),
    ...(zeca ? [] : [el("p", { class: "note", text: "Zeca is off (Flock): no chat or voice until you turn him back on." })]),
    el("p", {
      class: "lede",
      text: "The chat on the island talks through the Claude Code or Codex CLI you are logged into, on your own subscription. It can also use a provider's API with your own key, or a model running on this machine.",
    }),
    el("section", { class: "card rows" }, ...rows),
    el("h2", { text: "Voice" }),
    el("section", { class: "card rows" }, ...voiceRows()),
    el(
      "section",
      { class: "card" },
      el(
        "p",
        { class: "note" },
        document.createTextNode(
          "The API chat only talks: it can't run commands, edit files or open your project, only read the files you drop on the island. Keys are kept in the system keyring, never in a file, and only ever sent to their own provider.",
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
    "Talk to the chat",
    "Hold a conversation by voice: the mic in the chat records you, and the words land in the input for you to check before sending. It is transcribed on this computer by whisper.cpp; the audio never leaves it and is never saved.",
    status.ready
      ? button("Turn off", async () => {
          await Bridge.voiceOff();
          await refreshVoice();
        })
      : badge("Off", "off"),
  );
  const named = (code: string) => LANGUAGES.find((l) => l.value === code)?.label ?? code;
  const language = row(
    "Language you speak",
    "Telling whisper the language makes short phrases far more reliable than detecting it.",
    dropdown<string | null>(
      [
        { value: null, label: status.system ? `System (${named(status.system)})` : "System" },
        { value: "auto", label: "Detect it each time" },
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
    const mb = `${Math.round(m.size / 1_000_000)} MB`;
    let control: HTMLElement;
    const progress = downloading.get(m.id);
    if (progress !== undefined) control = el("span", { class: "muted", text: `Downloading… ${progress}%` });
    else if (m.installed && status.selected === m.id && status.ready) control = badge("In use", "ok");
    else if (m.installed)
      control = button("Use", async () => {
        await Bridge.voiceSelect(m.id);
        await refreshVoice();
      });
    else
      control = button(
        `Download ${mb}`,
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
    return row(m.label, m.installed ? `${mb}, on this computer` : `${mb} from the whisper.cpp models on Hugging Face, checked before use`, control);
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
    return ["Key", "None needed: it runs on this machine and nothing leaves it.", badge("Local", "ok")];
  }
  if (p.hasKey) {
    return [
      `${p.label} API key`,
      "Saved in the system keyring.",
      button("Remove", () => {
        Bridge.apiKeyClear(p.id)
          .then(() => done("Key removed from the keyring."))
          .catch((e) => done(String(e), true));
      }),
    ];
  }
  const input = document.createElement("input");
  input.type = "password";
  input.className = "field";
  input.placeholder = p.keyHint ? `${p.keyHint}…` : "Paste the key";
  input.autocomplete = "off";
  input.spellcheck = false;
  const save = () => {
    Bridge.apiKeySet(p.id, input.value)
      .then(() => done("Saved. Choose the API in the chat to use it."))
      .catch((e) => done(String(e), true));
  };
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") save();
  });
  return [
    `${p.label} API key`,
    "Kept in the system keyring, never in a file. Usage is billed to your account there.",
    el("div", { class: "inline" }, input, button("Save", save, true)),
  ];
}

/** The model, from the provider's live list once it can be asked. */
function modelRow(p: ApiProvider): [string, string, HTMLElement] {
  const list = apiModels.get(p.id);
  if (!p.local && !p.hasKey) return ["Model", "Listed live from the provider once a key is saved.", badge("No key", "off")];
  if (list === undefined) {
    void loadModels(p.id);
    return ["Model", `Asking ${p.label} for its models…`, badge("Loading", "warn")];
  }
  if ("error" in list) {
    return ["Model", list.error, button("Try again", () => void loadModels(p.id))];
  }
  const select = document.createElement("select");
  select.className = "field select";
  const choices = p.model && !list.models.includes(p.model) ? [p.model, ...list.models] : list.models;
  if (!p.model) select.append(new Option("Choose a model", "", true, true));
  for (const m of choices) select.append(new Option(m, m, false, m === p.model));
  select.addEventListener("change", () => {
    void Bridge.apiModelSet(p.id, select.value).then(() => refreshApi());
  });
  const missing = p.model && !list.models.includes(p.model) ? ` ${p.model} is no longer listed.` : "";
  return ["Model", `Listed live from ${p.label}: ${list.models.length} models.${missing}`, select];
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
  if (page === "chat") render();
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
        el("div", { class: "card-title", text: "Vults" }),
        el("p", { class: "row-about", text: "A friendly flock watching your coding agents. MIT licensed; no telemetry." }),
        version ? el("p", { class: "note", text: `Version ${version}` }) : null,
      ),
    ),
    el(
      "section",
      { class: "card rows" },
      row("Settings", "Your settings file.", el("code", { text: settingsPath })),
      row("Data", "The hook relay, the inbox of dropped files, connector state.", el("code", { text: dataPath })),
    ),
  ];
}

// ── Flock ────────────────────────────────────────────────────────────────────

const FLOCKS: { value: Flock; label: string }[] = [
  { value: "brazil", label: "Brazil" },
  { value: "americas", label: "The Americas" },
  { value: "world", label: "The world" },
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
  const paint = () => {
    // A hidden window draws nothing; the next tick catches up.
    if (document.hidden) return;
    const t = performance.now();
    for (const { id, canvas } of canvases) {
      const set = speciesSet(id);
      const ctx = canvas.getContext("2d")!;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      drawFrame(ctx, set, frameAt(set.clips.idle, t), 10, WIRE - perchOf(set), SCALE);
    }
  };
  // One timer at a time: every render of this page replaces it.
  window.clearInterval(previews);
  previews = undefined;
  paint();
  if (!matchMedia("(prefers-reduced-motion: reduce)").matches) previews = window.setInterval(paint, 100);
  return [
    el("h1", { text: "Flock" }),
    el(
      "section",
      { class: "card rows" },
      row(
        "The flock draws from",
        "Where the other sessions' birds come from: Brazil's vultures, the vultures of the Americas (with both condors), or every vulture in the world. A project with three or more sessions gets a king vulture either way. A new flock every time the app starts.",
        segmented(FLOCKS, flock, async (f) => {
          await Bridge.setFlock(f);
          flock = f;
        }),
      ),
      row(
        "Rare visitors",
        "Now and then, while sessions are open, a vulture from outside your flock (a condor, a griffon) crosses the sky once and goes on its way. It never lands.",
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
        "The companion who chats and listens. Off, the flock, cards, notifications and connectors work as ever, with no chat, microphone or talk shortcut; the session in front keeps its own bird and an empty wire stays empty.",
        toggle(zeca, async (on) => {
          await Bridge.setZeca(on);
          zeca = on;
          render();
        }),
      ),
      row(
        "Look",
        "Auto dresses Zeca for the season: a witch hat from October 1 to November 1, a Santa hat from December 1 to 26, a party hat from New Year's Eve to January 2, bunny ears from Good Friday to Easter Monday. Sunglasses and the other outfits only when you pick them. Only Zeca wears it; the flock keeps its feathers.",
        dropdown(LOOKS, zecaLook, async (look) => {
          await Bridge.setZecaLook(look);
          zecaLook = look;
        }),
      ),
    ),
    el("p", {
      class: "note",
      text: zeca
        ? "Zeca is the bird in front: the session that needs you, or the one you picked. Choose his species."
        : "Zeca is off: the session in front keeps its own bird. His species and look wait for him here.",
    }),
    ...group("new-world", "Vultures of the Americas"),
    ...group("old-world", "Vultures of Africa, Europe and Asia"),
  ];
}

// ── Layout ───────────────────────────────────────────────────────────────────

/** The page last drawn: a re-render of it keeps the scroll, a new page starts at the top. */
let drawn: Page | null = null;

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
            : page === "projects"
              ? projectsPage()
            : page === "connectors"
              ? connectorsPage()
              : page === "flock"
                ? flockPage()
                : aboutPage();
  if (page !== "flock") window.clearInterval(previews);
  const nav = el(
    "nav",
    { class: "sidebar" },
    el("div", { class: "brand", text: "Vults" }),
    ...PAGES.map((p) =>
      el("button", {
        class: `nav${p.id === page ? " on" : ""}`,
        text: p.label,
        onclick: () => {
          page = p.id;
          navigated = true;
          location.hash = p.id;
          apiMessage = null;
          if (p.id === "approvals") void refreshRules();
          if (p.id === "projects") void refreshProjects();
          if (p.id === "chat") void refreshApi();
          render();
        },
      }),
    ),
  );
  const scroll = page === drawn ? (root.querySelector("main.page")?.scrollTop ?? 0) : 0;
  const main = el("main", { class: "page" }, ...content);
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
void Bridge.appSettings().then((s) => {
  sounds = s.sounds;
  volume = savedVolume = s.volume;
  autostart = s.autostart;
  // The nearest choice: the file may hold any number in range.
  foldAfter = FOLD_CHOICES.reduce((a, b) => (Math.abs(b - s.foldAfter) < Math.abs(a - s.foldAfter) ? b : a));
  monitor = s.monitor;
  nowPlaying = s.nowPlaying;
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
  render();
});
// The island's speaker button changes the sounds too: keep the toggle and the slider in step.
Bridge.onSettings((s) => {
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
void refreshProjects();
void refreshApi();
void getVersion()
  .then((v) => {
    version = v;
    if (page === "about") render();
  })
  .catch(() => {});
// Statuses age and polls finish in the background.
window.setInterval(() => void refreshConnectors(), 15_000);
// An agent's files change outside the app (a relay built, a hook edited by hand): read them again
// when the window comes back, unless a review is open (it would lose its diff).
window.addEventListener("focus", () => {
  for (const a of AGENTS) if (!panels.get(a.kind)?.pending) void refresh(a.kind);
});
