// The settings window: a sidebar and one page per section. Installing hooks always goes through a
// diff the user reviews first.
import { getVersion } from "@tauri-apps/api/app";
import { Bridge, type AgentKind, type ApiProvider, type ConnectorStatus, type InstallPreview, type InstallStatus, type Rule, type VoiceStatus } from "./bridge";
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
  { kind: "gemini", name: "Gemini CLI" },
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
let nowPlaying = false;
let voice: VoiceStatus | null = null;
/** Model id → percent downloaded, for every download running. */
const downloading = new Map<string, number>();
let voiceError: string | null = null;
let autostart = false;
let foldAfter = 15;
/** Seconds the open island waits before folding, as the settings offer them. */
const FOLD_CHOICES = [5, 10, 15, 30, 60];
let monitor: string | null = null;
let monitors: { name: string; label: string }[] = [];
let version = "";
let rules: Rule[] = [];
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
  if (s.outdated) return badge("Update available", "warn");
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
  const updateHelp =
    s.installed && s.outdated
      ? el("p", {
          class: "note warn",
          text:
            kind === "claude"
              ? "These hooks are from an older version. Update them to answer Claude Code's questions from the island."
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
            text: "You have your own status line, so it stays as it is, and the island can't show Claude Code's usage: Claude Code reports it only to the status line.",
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
      : null,
    notice,
    s.error || pending
      ? null
      : el(
          "div",
          { class: "actions" },
          s.installed ? button("Remove hooks…", () => void preview(kind, false)) : null,
          button(s.outdated ? "Update hooks…" : s.installed ? "Reinstall hooks…" : "Install hooks…", () => void preview(kind, true), true),
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
      row("Talk to the chat", "Hold Ctrl+Alt+V to speak to Zeca from anywhere, once a voice model is set up in Chat.", el("span", { class: "kbd", text: "Ctrl+Alt+V" })),
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
          apiMessage = null;
          if (p.id === "approvals") void refreshRules();
          if (p.id === "chat") void refreshApi();
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
  // The nearest choice: the file may hold any number in range.
  foldAfter = FOLD_CHOICES.reduce((a, b) => (Math.abs(b - s.foldAfter) < Math.abs(a - s.foldAfter) ? b : a));
  monitor = s.monitor;
  nowPlaying = s.nowPlaying;
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
Bridge.onMonitors(() => void refreshMonitors());
for (const a of AGENTS) void refresh(a.kind);
void refreshConnectors();
void refreshRules();
void refreshApi();
void getVersion()
  .then((v) => {
    version = v;
    if (page === "about") render();
  })
  .catch(() => {});
// Statuses age and polls finish in the background.
window.setInterval(() => void refreshConnectors(), 15_000);
