# What t3code does well, in Vults

The plan for six areas where t3code (studied at its `61808c68cf`) is ahead, each step with where it
goes and its size (S, M, L). Every step keeps Vults' rules: Linux first, Rust backend, no telemetry,
a request only through `app/src/gate.rs` for a feature the user turned on, secrets only in the
keyring, a permission answered only by a human or by what they chose beforehand, every sentence in
four languages, docs in English and Brazilian Portuguese in the same PR.

Order: **Labs** first (it lets the others ship off by default), then **Chat folders**, **Settings**,
**Code**, **Scheduled tasks**, **Environments**.

## 1. Labs

t3code: a Labs flag is an ordinary boolean setting, default off, with a row on a Labs page
(`apps/web/src/components/settings/LabsSettingsPanel.tsx`); code reads the setting; a flag that
graduates moves its row, a retired key stays readable.

Vults has no experimental mechanism. The registry pattern to copy is `crates/agents/src/registry.rs`,
which generates `ui/src/agents.gen.ts` with a freshness test.

1. `crates/core/src/labs.rs`: `enum Lab` and a table (id, English name and description, restart,
   requires, since); generates `ui/src/labs.gen.ts`, fresh by test. S
2. `Settings.labs: BTreeMap<String, bool>` (missing is off, unknown kept), `VERSION` bump,
   `Public.labs`, `settings::lab(app, Lab)`, one `set_lab` command. S
3. `ui/src/labs.ts` (`labOn`), Settings → **Labs** page in the system group: one warning line, a
   switch, description and *Restart needed* per row; a row whose `requires` is off is disabled. M
4. First flags: the ACP engines not yet validated live (Qwen Code, Copilot CLI, Kimi Code), then
   meeting speaker labels; later SSH sessions, scheduled tasks, environments. M
5. Docs: `docs/reference/settings.md` and its translation; CHANGELOG. S

## 2. Chat: where it works

t3code adds a project by palette (typed or pasted path with listings), native picker, new project
(`git init`), clone from a URL or a provider's `owner/repo`, drop on the sidebar, `t3 <dir>`; it
dedupes by path and opens a thread there.

Vults: the place menu lists only live sessions' folders and *An empty folder*; a dropped folder is an
`@path`; nothing checks the folder is a directory; roosts are read-only.

1. `send` canonicalizes the folder and requires a directory; `chat_check_folder(path)`. S
2. *Choose a folder…* in the place menu: `chat_pick_folder` (`tauri-plugin-dialog`, like the
   paperclip). S
3. Recent folders: `chat_recent_folders` (saved conversations' folders that still exist, roost
   roots); the menu groups *Live / Recent / An empty folder* with a filter that takes a pasted
   path. S
4. A folder dropped on the place pill or an empty chat sets the folder; elsewhere it stays an
   `@path`. The Nest gets the drop too. S-M
5. *Chat here* on each roost and tree row in `ui/src/surfaces/nest/roosts.ts`. S
6. New project: a name under a projects folder (setting, default `~/Projects`), `mkdir`, optional
   local `git init`. M
7. New worktree from a roost (local `git worktree add`), chat there. M
8. Clone from a URL or `owner/repo` through a gate feature (budget, trace), hooks off, destination
   confirmed, the clone untrusted. L
9. `vults <dir>` and *Open with Vults* (single-instance argv, `MimeType=inode/directory`). M

## 3. Settings

### Providers

t3code: a list beside a detail pane; each row an icon, name, version, status dot and one headline
(*Authenticated*, *Not found: CLI not detected on PATH*, *Needs attention*…); *Checked 5m ago* and
Refresh; models with counts, filter, favorites, hidden.

1. `ui/src/surfaces/settings/provider-status.ts`: one `providerSummary()` (headline, detail, dot)
   for API providers and agents; each headline a whole sentence. S
2. API providers as a list (dot, label, *Key saved* / *Local* / *No key*) with a detail pane (key in
   the keyring, model, Test connection) instead of the `<select>`; its own view below 900 px. M
3. Agents page: version and CLI path in each header, *Checked X ago* with *Check again* (from
   `doctor.ts`). M
4. Models detail: count, filter, default; *hide from the picker* (a settings field). S-M
5. A version advisory with a copyable update command, from the local `--version` only. S

No periodic health check over the network: a request only on a click or when the page opens.

### Voice

t3code: model cards (badges, languages, size, speed, *Verifying download…*), *Test microphone* with
a level meter under the picker, *Test model* that shows the transcript, hardware and unload in
Advanced, a reason when a control can't be used.

1. `crates/voice/src/models.rs`: languages, multilingual, speed tier, recommendation per language;
   through `voice_status`. S
2. `voiceRows()` (`ui/src/surfaces/settings/main.ts`) as model cards with badges, facts and the
   downloading → verifying → in use states; an unmeasured figure says *Unknown*. M
3. The microphone test (from `dictation-doctor.ts`) under the microphone picker, with a `<meter>`. M
4. *Test model*: a few seconds recorded and transcribed in memory, the text shown. M
5. Hardware (GPU/CPU) and unload after idle under advanced processing. S

No cloud transcription: audio stays on the computer.

### Appearance

t3code: theme tiles with a small preview of the app in each theme, a reset-to-default on each row,
*(default)* in menus, live previews.

1. `pieces.ts`: a reset-to-default slot on `row()`; *(default)* on default menu items. S
2. Theme tiles: a radio group of the five themes, each a small wireframe in its own tokens, keyboard
   as DESIGN.md's tab lists. M
3. Appearance as its own page, width with a small Mini/Quick preview; Zeca's look beside it. S-M

Not copied: theme import and editing, downloaded fonts (DESIGN.md).

## 4. Code: projects that keep their context

t3code: a project (root, default model, scripts, pinned, archived) owns its threads; settings resolve
project → environment → repo file → default; a provider switch hands off with a budget and records
what it covered, so switching back sends only the rest, and the agent can read what was left out;
threads in local or worktree mode; a diff pane of the turn, uncommitted and branch changes; commit,
push and PR.

Vults: a conversation has a folder string; per-folder model picks; `chat_switch_engine` hands off the
first message and the latest lines (48,000 characters); roost memory in the frozen context; the
Changes tab reads only the agent's reported diffs.

1. Project record: `crates/store` migration, `projects(root UNIQUE, title, default engine, model,
   effort, instructions, actions, pinned, archived)`, seeded from conversations and roosts; the
   per-folder picks fold in. M
2. A new conversation takes its project's engine and model; project instructions join the frozen
   context as the user's words. S
3. Recent grouped by project (pinned first, *New conversation in this project*, pin and archive). M
4. Handoff v2: what each engine already saw (`handoff_covered`), so switching back sends only the
   rest; short tool and command outcomes; the budget from the target model's context; a read-only
   `conversation_read` tool for the lines left out. M
5. The handoff on reopening with another engine, and *Continue in a new conversation*. S
6. Git-backed Changes: uncommitted and branch-against-base diffs from `app/src/folder.rs` (no hooks,
   no filters), a source picker in the side panel. M
7. Project actions (name, command, icon): run on a click only, in a terminal; one from a repo file is
   shown as a diff and accepted with a click. M
8. A worktree per conversation (`git worktree add`, a `vults/` branch), a guard against two busy
   conversations in one checkout, cleanup offered on delete. L
9. Commit, push and PR from Changes through `gh`, each a click after a preview, audited. M
10. Later: the same prompt to several engines, each in its worktree, compared in the Nest. M

## 5. Scheduled tasks: vultures that come, work and leave

**The idea.** You create a scheduled task and dress the vulture that does it. At its time the bird
flies onto the island, works as any session does (working, a card if it needs you, done), sends a
notification, and flies away. The run is a conversation in Specialists, marked so you know a schedule
sent it, not you.

**What t3code does** (studied at `61808c68cf`): a task has a title, a prompt, a schedule (`interval`,
`fixed_time` or `once`), a project, a model, a permission mode and a thread mode (a new thread each
run, or continue one). Tasks live in SQLite; a scheduler in the server ticks every 5 s and runs one at
a time; a fixed-time run more than 10 minutes late is skipped as missed; a run is a thread whose first
message says a scheduled task sent it, with a badge in the timeline. The editor
(`apps/mobile/src/features/settings/scheduled-task-editor.tsx`, the web settings page) has presets
for the time, a project picker, run now, pause and the last runs.

**What Vults already has for it:**

- the scheduler pattern: Zeca's sleep (`app/src/sleep.rs`), in local time, waiting while busy, a day
  done only after it succeeds;
- Specialists (`ui/src/surfaces/nest/profiles.ts`, `specialist.ts`): profiles with their own chat,
  conversations, access level, **species and look** (`profileSpecies`, `profileLook`);
- the flight: a bird arrives from the left and leaves by the right edge (`director.ts`, `arrive()` and
  `leave()`), used when a session joins or leaves the flock (`ui/src/island/scene.ts`);
- looks for every species (`crates/core/src/looks.rs`), notifications (`crates/core/src/notify.rs`),
  the audit log and the network gate.

What is missing: a specialist's run is not on the wire yet (UX8-PROJ), so it has no bird on the
island; nothing runs without a window; there is no task, schedule or run in the store.

### How it works for the person

1. **Specialists → Scheduled tasks** (a button beside the Specialists title, with a clock icon and the
   count) opens the list: each task with its bird, title, next run in words (*Today at 18:00*,
   *Every Monday at 09:00*), last result (dot and one line) and a switch.
2. **New scheduled task** asks, in this order:
   - **What**: title and the prompt (the composer the chat uses: @ for files, attachments);
   - **When**: *Once*, *Every day*, *Weekdays*, *Every week on…*, *Every N hours*, at a time, local;
   - **Who does it**: a specialist (its engine, model, folder and access come with it) or *Zeca*;
   - **The vulture**: species and look, a name; the preview is the real sprite flying in place.
     It starts as the specialist's own bird; changing it here changes only this task's;
   - **Each run**: *a new conversation* (default) or *continue the last one*;
   - **If it needs you**: the specialist's access, shown and capped (never more than it has); how long
     a card waits before the run stops (15 min by default).
3. **At its time** the bird flies in from the left with the task's name over it, takes a perch like a
   session, and its state follows the session pattern: working (the tool it runs), a card if it asks,
   done or failed. A notification says it started only when it needs you; at the end one says
   *Daily review finished* (or *failed: reason*) with **Open**.
4. **After it ends** the bird stays a few seconds as *done*, then flies off to the right. A failed run
   stays until seen, like a session that needs you.
5. **In the chat** the run is a conversation of that specialist with a **Scheduled** mark: a clock
   badge on the conversation in the list and on its first message (*Sent by "Daily review", every day
   at 18:00*), and **Edit task** from there. You can answer it like any conversation.

### Rules

- A run unattended uses only what was chosen for that specialist beforehand: its access (*asks
  first*, *your policies*, *full access*), never more. Destructive actions and an agent's questions
  still wait for a person; a card nobody answers by its deadline stops the run. CLAUDE.md rule 2 says
  so before the code lands.
- One run at a time per specialist; a run that would start over a running one waits for it.
- A run more than 10 minutes late (computer asleep or off) is **missed**: recorded, notified once,
  not run late. *Run now* runs it at once.
- A run that reaches the network is a gate feature (`ROUTINES`): its own switch, budget and trace.
  Off by default; the whole feature starts in Labs.
- Every run is audited; the task, its prompt and its runs stay local (the store).

### Steps

| # | Step | Where | Size |
|---|---|---|---|
| S1 | Design here and in CLAUDE.md rule 2 and 4: access cap, card deadline, missed runs, gate feature | `CLAUDE.md`, this file | S |
| S2 | A Lab `scheduled-tasks`, off | `crates/core/src/labs.rs` | S |
| S3 | Store: `scheduled_tasks` (title, prompt, schedule, specialist, species, look, name, run mode, card deadline, enabled, next, last, status) and `scheduled_runs` (task, started, ended, outcome, conversation) | `crates/store` migration | M |
| S4 | Gate feature `ROUTINES`: switch, budget, local trace | `app/src/gate.rs` | S |
| S5 | A specialist's run on the wire with its identity (UX8-PROJ): the session it runs is a flock entry with the profile's species and look | `crates/core`, `app/src/chat.rs` | M |
| S6 | A run with no window: `Origin::Scheduled(task)`, the specialist's runtime, access capped, the first message marked; card deadline stops it | `app/src/chat.rs` | L |
| S7 | The scheduler `app/src/routines.rs` (from `sleep.rs`): every 30 s, next run in local time, missed window, rows left running recovered at start | `app/src/routines.rs` | M |
| S8 | On the island: the run's entry arrives flying with its name, perches, follows the session states, stays done a few seconds, leaves flying; failed stays until seen | `ui/src/island/scene.ts`, `director.ts` | M |
| S9 | Notifications: needs you, finished, failed, missed; **Open** goes to its conversation | `crates/core/src/notify.rs` | S |
| S10 | Specialists → **Scheduled tasks**: the list, the editor (what, when, who, the vulture with a live preview, each run, if it needs you), run now, pause, last runs | `ui/src/surfaces/nest/` | L |
| S11 | The **Scheduled** mark: clock badge on the conversation and its first message, *Edit task* | `ui/src/island/chat.ts`, `nest/` | S |
| S12 | Docs (guide in English and Portuguese), checks for the scheduler and the flight | `docs/guide/`, `scripts/` | M |

Order: S1-S4, then S5 (it also closes UX8-PROJ), S6-S7, S8-S9, S10-S11, S12.

## 6. Environments

t3code: each machine runs its own server; threads belong to one environment; SSH hosts read from
`~/.ssh/config`, the server installed and started over SSH, reached through `ssh -L`, paired with a
one-time token for a session bound to a key; Tailscale discovery; a supervisor that backs off,
re-probes after waking and names the hop that fails.

Vults keeps the desktop as the one hub: nothing is installed on the other machine but `vults-hook`.
The stash *wip: ssh remote sessions (batch B1)* already sends a hook's events over a socket
forwarded with `ssh -R`, stamps the host, and keeps local rules and *Always* away from remote paths.

1. Revive the stash on this branch: the listener per host assigns the host (never trusting the
   hook's), `host` on `SessionView`. S
2. `crates/remote`: hosts and states, a backoff supervisor model, `ssh -G` and `known_hosts`
   parsing; the user's own keys and agent, no SSH secret stored. M
3. Gate feature `REMOTE`, off by default, per host. S
4. Tunnel runner `app/src/remote.rs`: a supervised `ssh -N -R` per host, strict host keys, backoff,
   a re-probe after waking. M
5. The hook installed on the other machine over SSH, with the same diff, dated backup and click. M
6. Sessions of other machines in the flock with their host, grouped by it. S
7. The chat on another machine: `ssh -T host -- <cli>` for the drivers (stream-json, ACP and the
   Codex app server all run over stdio), a folder picker per host, a remote `@` list. M
8. A hub view: each host's state and failing hop, its agents, sessions and last event. M
9. Read-only remote diffs and listing, bounded. S-M
10. Tailscale discovery from the local CLI. S
11. Later: WSL as a host, Windows and macOS. L

## 7. Threads: conversations that look after themselves

t3code: Settings → Threads (`SettingsPanels.tsx:1935`), the thread menu (`threadActionMenu.logic.ts`),
fork from a reply (`TimelineAssistantRows.tsx:163`), the command palette searching every thread.

What Vults already has: a list of conversations in Quick and the Nest, **Rename** and **Delete**
(`chat_rename`, `chat_delete`), **Ctrl+F** inside one, kept nodes, ↑/↓ history, the queue mid-turn,
a lost session resumed by handoff. Settings has nothing for them yet.

| Step | What | Where | Size |
|---|---|---|---|
| T1 | A **Conversations** section in Settings → Chat & voice: the engine and model a new one starts with, and who answers its cards (moves there) | `settings/main.ts`, `app/src/settings.rs` | S |
| T2 | **Pin** and **Archive** in each conversation's menu; archived ones in a list with **Unarchive**; Delete asks first (setting, on) | `crates/store` migration, `app/src/chat.rs`, `chat.ts` | M |
| T3 | **Titles by a model**: the first turn names the conversation, **Regenerate title** in its menu; off without an engine; Claude/Codex CLI or the API provider, through the gate | `crates/chat`, `app/src/chat.rs` | M |
| T4 | **Fork from this reply**: a new conversation with the turns up to it (handoff text, as `lost_session`) | `crates/chat/src/lib.rs`, `chat.ts` | M |
| T5 | **Search every conversation** from the Nest (titles and text, from the store) | `crates/store`, `nest/` | M |
| T6 | **Copy conversation** as Markdown, and its id | `chat.ts` | S |
| T7 | **After a usage limit**: wait and resume when it resets (the agent's reset time, never guessed); **settle** idle ones after N days | `app/src/chat.rs`, Labs first | L |

## 8. Browser: what agents open

t3code: Settings → Browser (`IntegrationsSettings.tsx:1695`) and a built-in preview browser the agent
drives (`apps/server/src/mcp/toolkits/preview/tools.ts`: navigate, snapshot, act, record), with
profiles, a mini-player and local servers found by port.

What Vults already has: links open in the system browser (`app/src/runtime.rs:1383`); Zeca's web
(search engine choice: Brave, Tavily or your SearXNG, and page reads, through the gate, 200 a day).

| Step | What | Where | Size |
|---|---|---|---|
| B1 | **Open links in**: the system browser (now) or a Vults window; Ctrl+click always the system one | `app/src/runtime.rs`, Settings → General | S |
| B2 | **Local servers**: a session's `localhost:port` shown on its card, **Open** in a preview window | `crates/core` (from tool output), `ui/` | M |
| B3 | **Preview window**: a Tauri webview with address bar, back/forward, reload, open in system browser, screenshot into the chat | `app/`, `ui/src/surfaces/preview/` | L |
| B4 | **Agent access to the preview** (off by default, per project): navigate, snapshot, click, type, through the `vults-zeca` MCP; each act a card unless a policy covers it | `crates/zeca`, MCP, Labs first | L |
| B5 | Profiles, recording, device sizes, cookie import | after B4, only if asked | — |

Rules for both: nothing leaves the computer for a title or a search unless its feature is on; a
browser the agent drives is a Lab first, never on by default, and never answers a permission.
