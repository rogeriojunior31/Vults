# What t3code does well, in Vults

The plan for the areas where t3code (studied at its `61808c68cf`) is ahead, each step with where it
goes and its size (S, M, L). Every step keeps Vults' rules: Linux first, Rust backend, no telemetry,
a request only through `app/src/gate.rs` for a feature the user turned on, secrets only in the
keyring, a permission answered only by a human or by what they chose beforehand, every sentence in
four languages, docs in English and Brazilian Portuguese in the same PR.

The order of work is the roadmap's *t3code first* section ([ROADMAP.md](ROADMAP.md)); section 0 below
is how each step is built.

## 0. How every step here is built

The sections below say *what* each step does. This one says *how*, once for all of them, so a feature
lands whole and is not reopened later.

### Shared pieces, built once

Several sections need the same thing. Each piece is built in the first step that needs it, in the
place named here, and every later step reuses it. A step never draws its own copy.

| Piece | First needed by | Lives in | Reused by |
|---|---|---|---|
| **List and detail pane**: a list (groups, mark, title, one line, a dot) beside the chosen item's detail; arrow keys, Home and End; stacked when narrow | Settings, Agents (done) | `ui/src/surfaces/settings/pieces.ts` today; moves to `ui/src/pane.ts` at C2 | C2, C3, C6 (Connections), U6 (Usage), S10 (Scheduled tasks), T2 (archived conversations) |
| **Prepared-write card**: what will change, the target and its fresh state (head SHA, open or closed), *Send* and *Cancel*, audited; the same card in the Nest and the chat | Connections, C4 | `ui/src/surfaces/nest/prepared.ts` over `app/src/connectors.rs` prepare and execute | C5, C7, C9, C11, W4 (Create PR from the chat), X6 |
| **Markdown as DOM** (never HTML), task boxes, code with highlighting | done | `ui/src/island/markdown.ts` | C2, C6, the Summary of a run (S11) |
| **Diff view**: files, hunks, line numbers, a selection | done in Changes | `ui/src/island/chat-activity.ts`, `nest/workspace.ts` | C8, C9, X5 (ask about lines), W3 (open at a line) |
| **Marks**: a provider, agent, forge or editor by its official mark | done | `ui/src/brands.ts`, `brandMark()` | W3 (editors), C2 (forges), U6 (chart colors), I2 to I4 |
| **Icons**: Lucide only, `icon(name, size, stroke)` | done in Settings | `ui/src/island/icons.ts` | every step; I1 adds the check |
| **Numbers**: tokens as 1.2K, M, B or T; money; *in 2 h*, *51d ago*; pace | U3 | `ui/src/cost.ts` (tokens, money, pace), `ui/src/time.ts` (relative time) | U6, the chat ring, C2 (*opened 51d ago*), S10 (*next run*) |
| **SVG chart**: lines and soft areas per series, a tooltip, no chart library, both themes | U6 | `ui/src/chart.ts` | Records, later usage per roost |
| **Editors table**: the editors found on PATH, each with its go-to-line argument, one absolute path argument, never a shell | W3 | `app/src/open.rs` | W3, C8 (open a file at a line), X5 |
| **Store migrations**: one numbered migration per step that adds a table, with a test that opens an old database | T2 | `crates/store/migrations/` | T2, S3, U4, C10 |
| **A Lab**: a switch in `crates/core/src/labs.rs`, off by default, its docs line | done | `labs.rs`, `labs.gen.ts` | S2, X7, X11, B4, T7 |
| **A gate feature**: switch, daily budget, local trace, docs in the safety table | done | `app/src/gate.rs` | S4 (`ROUTINES`), any new request |

### The checklist for each step

Every step meets all of these in the same PR. A step that cannot is split, never shipped half.

1. **Layers.** Domain in `crates/core` (pure: `reduce`, no IO), IO in its crate, the app glues them,
   the UI renders `State::view()`. `scripts/check-layers.sh` passes.
2. **Rules.**
   - A request is made only for a feature the user turned on, through `app/src/gate.rs`, with its budget and trace.
   - A write is a prepared-write card, a click and an audit line.
   - A permission is answered only as rule 2 says.
   - A secret is only in the keyring.
   - An agent's config is changed only with a backup, a diff and a click.
3. **States.** Each new view has its loading, empty, error with *Try again*, and long-content states.
   Each new action has its pending, done and failed states, and keeps the user's draft on failure.
4. **Words.**
   - Every sentence is short (one per row; *More* holds the rest) and written whole in English.
   - The same PR translates it into pt-BR, es and zh.
   - `scripts/check-i18n.sh` and `scripts/check-english.sh` pass.
5. **Look.** It reuses the shared pieces above, the theme tokens and Lucide icons; marks through
   `brandMark()`. It works in the dark and light themes, at the Nest's smallest size, and with the
   keyboard alone. Every button has its words or an `aria-label`.
6. **Tests.**
   - Rust: unit tests for the domain and parsing.
   - The UI: a `scripts/check-*.mjs` per new surface or flow, covering every state in item 3, added to *Before every commit* in `CLAUDE.md` and to CI.
   - A request or a write: a test with a recorded response, never the network.
7. **Docs.**
   - The guide page in English and pt-BR (with its source mark).
   - The safety table when it sends something.
   - The settings reference when it adds a setting.
   - `CHANGELOG.md`, and this file's step marked done.
8. **In the running app.** Checked in `npm run tauri dev` on Linux before the PR merges, with what was
   seen written in the PR.

### Order and dependencies across sections

- **Nothing outside a section.** Steps with no dependency outside their own section: X1, X3, X4, X8, X9, X10, X12-X15, T1, T6, U1-U3.
- **The pane first.** C2 moves the list and detail pane to `ui/src/pane.ts`; U6, S10 and T2's archive list wait for it.
- **The prepared-write card.** C4 builds it; W4, C5, C7, C9, C11 and X6 use it.
- **The bird on the wire.** S5 (a specialist's run on the island) closes UX8-PROJ and comes before S8.
- **The usage scan.** U4 replaces most of W9b (the OTLP receiver); W9b is reconsidered after U4.
- **Icons last.** I1 to I7 run after the features of phases A to D, so each surface is redrawn once.
- **Decisions before code.** U9 (Claude's reset credits) and B4 (agent access to a browser) need a
  decision recorded as an ADR before any step.

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
| T6 (done) | **Copy conversation** as Markdown | `chat.ts` | S |
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

## 9. The chat's workspace bar: worktrees, PRs, editors and connections

t3code: the thread header shows the project and branch; a worktree per thread (`New thread on
{branch}`), *Open in* with the editors it finds (VS Code, Insiders, VSCodium, Cursor, Zed, Kiro, Trae,
Finder/Files; `apps/web/src/components/Icons.tsx`), git actions from the header (commit, push, create
a PR) and Linear/Jira links on the thread.

What Vults already has:

- roosts group a repository's worktrees (`app/src/roosts.rs`: shared git dir, `owner/repo` from the
  remote, a worktree's own HEAD);
- creating and editing a PR through the GitHub connector, prepared and shown before any write
  (`crates/connectors/src/github/pull_requests.rs`, `PullRequestWrite::Create`), and the same for
  GitLab and Gitea;
- the Nest chat's side panel (`ui/src/surfaces/nest/workspace.ts`: Changes with diffs, Files);
- the official marks of GitHub, GitLab, Gitea and the editors' brands (`ui/src/brands.ts`).

What is missing: the chat does not show the branch or worktree it works in, cannot open the folder in
an editor, and the connections' actions live only in Connections.

### Clean, in one bar

One quiet line above the Nest chat's messages, nothing else added:
`[repo mark] owner/repo  ·  ⎇ branch (worktree)  ·  ↗ Open in ▾  ·  ⋯`

- **Branch chip**: the branch and whether this folder is a worktree; a click lists the repository's
  worktrees (from roosts) with their branch and changes count, and **Chat here** for each. Read-only:
  Vults lists worktrees, the agent creates them.
- **Open in ▾**: the editors found on this computer (by their commands on PATH: `code`, `codium`,
  `cursor`, `zed`, `subl`, and the file manager), each with its mark; the last one used becomes the
  button's default. It opens the conversation's folder, or a file from Changes at its line.
- **⋯ (connections)**: only what the connected forge can do for this branch, each one a prepared
  action you see before it runs: **Create PR** (title and body drafted from the conversation, base and
  head filled, draft by default), **Open PR** when one exists (its checks dot), **Open issue**. A
  forge that is not connected shows one line: *Connect GitHub in Connections*.
- In Quick, nothing of this: Quick stays fast; the Nest is where work is.

Rules: a PR is a write on your behalf, so it is shown first and sent with a click (rule 4, the gate);
opening an editor runs a local program with the folder as its argument, never a shell string.

| # | Step | Where | Size |
|---|---|---|---|
| W1 | The workspace bar: repo mark, branch chip (worktree or not), from the conversation's folder | `app/src/roosts.rs`, `nest/workspace.ts` | S |
| W2 | The worktree list from the chip, with changes count and *Chat here* | `app/src/roosts.rs`, `nest/` | M |
| W3 | **Open in**: editors found on PATH with their marks, last used first; folder, or a file at its line from Changes | `crates/platform`, `app/`, `ui/src/brands.ts` | M |
| W4 | **Create PR** from the chat: drafted from the conversation, shown as the connector's prepared write, sent with a click | `app/src/chat.rs`, `crates/connectors` | M |
| W5 | **Open PR / Open issue** and the PR's checks dot on the bar | `crates/connectors`, `nest/` | S |
| W6 | Docs and a check for the bar's states | `docs/guide/chat.md`, `scripts/` | S |

## 10. Smaller things t3code does well

Checked against Vults by grep; each says what Vults has today.

| # | What t3code does | Vults today | Step for Vults | Size |
|---|---|---|---|---|
| X1 (done) | **Keeps the computer awake while agents work** (`systemd-inhibit`, on by default; `server/src/background/KeepAwake.ts:66`) | nothing | A logind `Inhibit("sleep:idle")` over the existing zbus connection while a session or chat turn works; a switch in General; local only | S |
| X2 | **Open in** any of ~25 editors, a preferred one remembered (`packages/contracts/src/editor.ts`) | only `code`, else the file manager (`app/src/open.rs`) | Merged into W3: a table of editors (code, codium, code-insiders, cursor, zed, JetBrains launchers) with their go-to-line argument; one absolute path argument, never a shell | S-M |
| X3 | **Command palette** over threads, projects, settings, themes and actions; **Settings search** (~200 titles, `settingsSearch.ts`) | Ctrl+K in the Nest: sessions, `/continue`, `/remember`, five tabs (`ui/src/palette.ts`); no Settings search | Add New conversation, every Settings page (opened at its anchor) and the themes to the palette; a filter over Settings row titles with the same strings | S-M |
| X4 | **Fallback model** when the default is out of usage (`settings.ts:1480`) | reads Claude's and Codex's usage windows (`app/src/usage.rs`), no fallback | *When {engine} is at its limit, start new conversations with...*; only at 100 % with a reset in the future; never switches a running conversation | S |
| X5 | **Ask about lines**: a diff range becomes a context chip; *Cite* selected reply text (`composerContext.ts:182`, `AssistantSelectionToolbar.tsx`) | Changes is read-only; nothing quotes a reply | Select lines in Changes, *Ask Zeca*: path, lines and hunk (bounded) in the draft; *Quote* on selected reply text | M |
| X6 | **Threads linked to issues** (`issue/IssueThreadLinks.tsx`) | Connections rows: *Ask Zeca* questions, *Open in*, *Seen* | *Work on this* on an issue or PR row: a chat in its roost's folder with the item as quoted, untrusted context | M |
| X7 | **Steer a running turn** with a queued message; queue or steer by default | messages mid-turn only queue (`chat.ts`) | *Send now* on a queued item for drivers that take input mid-turn (Claude stream-json, ACP), hidden elsewhere; Lab first | M |
| X8 | **Storage** page with cleanup (`StorageSettings.tsx`) | voice model sizes and Download; no Remove | *Remove* on an installed model not in use; one *Space used* line (models, meeting assets, `vults.sqlite`, logs) | S |
| X9 | **Diagnostics** (`DiagnosticsSettings.tsx`) | agent and dictation doctors; no logs or report | *Open logs folder* and *Copy a report* in About (version, desktop, agent versions, doctor results, redacted, to the clipboard only) | S |
| X10 | **Continue threads after restarts**, hold to quit | quitting mid-turn ends it silently | *Zeca is still working. Quit?* while a turn runs; the cut turn marked with *Continue* (a click, never automatic) | S |
| X11 | **Delegate a task** to a child agent (`mcp/toolkits/orchestrator/tools.ts`) | Zeca cannot start a specialist | A `specialist_ask` tool: a card, the user approves, the run uses that specialist's own access, the answer returns to Zeca; Lab first | M |
| X12 | **Skills in the `/` menu** (`ClaudeSkills.ts`) | `/` has remember, forget, continue, new | In *your setup* mode, the names in `~/.claude/skills/*/SKILL.md` and the project's `.claude/skills` as `/` entries; local read only | S |
| X13 | **Pace** on quota windows (ahead, on, under) | % and reset time | The pace word beside the % | S |
| X14 | **Favorite models** in the picker | *hide* planned (section 3) | A star on a model; favorites first | S |
| X15 | **Nest hotkeys** and a cheat sheet (~70 bindable commands) | Ctrl+K, j/k, Esc | New conversation, tab switch, side panel; the list in Keyboard shortcuts | S |

Skipped, with the reason: an integrated terminal (Vults jumps to the agent's own), a file tree (not
an IDE; `@` lists paths), guest share links and tunnels (they expose the machine; the phone and its
relay are the Vults way), 1Password and agent secret requests (secrets only in the keyring), webhooks
(inbound network), *suggest next prompt* (sends every draft to a model), MCP server management (the
CLI owns it), telemetry export (no telemetry), Mermaid and HTML render (heavy for little).

## 11. Usage: plans, resets, cost and tokens

**What t3code does.**

- **Plan limits.** Claude's come from the CLI's `get_usage` (`apps/server/src/provider/claudeUsageLimits.ts:187-233`): the 5-hour *Session* window, *Weekly*, a weekly row per model (`model_scoped`) and *Monthly spend* (`extra_usage`), all updated live from the stream's rate-limit events. Codex's come from app-server `account/rateLimits/read` (`codexUsageLimits.ts:60-132`): primary and secondary windows, a monthly window for Free and Go plans, and the individual credit limit.
- **Resets.** These are banked reset credits, not a window count.
  - Claude: t3code reads the CLI's OAuth token from `~/.claude/.credentials.json` and calls `api.anthropic.com/api/oauth/usage` with a `claude-cli` user agent (`claudeResetCredits.ts:194-230`). That returns the count left for each kind (5-hour, Weekly and others), when they expire, and a *Redeem* (`POST .../reset_rate_limits`).
  - Codex: the same `account/rateLimits/read` returns them as `rateLimitResetCredits`.
- **Tokens and cost**, local only.
  - Sources: Claude's transcripts (`~/.claude/projects/**/*.jsonl`, `Drivers/claudeUsage.ts:135-148`: input, cache read, cache write with its 1-hour split, output, `costUSD`, deduplicated by message and request) and Codex's `token_count` events in `~/.codex/sessions`; also OpenCode, Cursor, Antigravity and others. Scans are cached.
  - Cost per category: `input×in + cacheRead×cr + cacheWrite5m×cw5m + cacheWrite1h×cw1h + output×out`, with speed tiers and context tiers (`usage/usagePricing.ts:357-368`). A cost the agent reports wins, and the split is scaled to it. Every figure says its source: *reported*, *priced* or *unpriced*. Cache savings are shown.
- **UI** (`components/usage/UsagePage.tsx`):
  - toggles: Cost, Tokens or Limits, over 24 hours (hourly) or 7, 30 or 90 days;
  - the total, with *API estimate* and the unpriced share, and each provider's share;
  - a daily chart in hand-written SVG, a line and a soft area per provider with a tooltip (`UsageProviderChart.tsx`; no chart library);
  - tiles for processed tokens, cached input, uncached input, output and cache savings;
  - share bars by token type, cost type and speed, and a table by model or by day;
  - tokens formatted as 1.2K, M, B or T (`packages/shared/src/usageFormat.ts:94-115`);
  - limits as bars with *Resets 18:00 · in 2 h*, the pace, and the reset credits with *Redeem* behind a confirmation.

**What Vults has.**

- **Plan windows.** Codex's through `account/rateLimits/read` and Claude's through the CLI's `get_usage` (`crates/agents/src/usage.rs`), behind the gate features `codex-usage` and `claude-usage`. Only the 5-hour and 7-day windows are read; the per-model weekly rows, `extra_usage` and the reset credits are ignored.
- **Cost.** A `spend` table per day, source and roost, filled only from costs the agents report (`crates/store/src/spend.rs`). Live per-session token metrics (`crates/agents/src/metrics.rs`). Catalog prices from models.dev behind `model-prices` (`crates/agents/src/prices.rs`).
- **UI.** Nest → Records → usage: plan bars, 14 days of CSS bars against the budget, tokens per day, roosts and tiles (`ui/src/surfaces/nest/records.ts:240-300`). The chat's ring shows the fullest window. Settings has *Usage and prices* and the daily budget.
- **Not built.** W9b, the cost of watched sessions through an OTLP receiver.

**The rule for resets.** Vults reads usage only through what the agent's own CLI answers, and never uses the CLI's token itself (`crates/agents/src/usage.rs:1-2`, CLAUDE.md rule 4). Codex's reset credits arrive in a response Vults already asks for, so they can be shown at once. Claude's are different: they need Vults to read the CLI's OAuth token and call Anthropic as if it were the CLI. That goes against the rule, so it is **To decide** below, not a step. Redeeming a credit is never automatic: a click after a confirmation that says what it spends.

| # | Step | Where | Size | Network |
|---|---|---|---|---|
| U1 | **Every window** from the responses already fetched: Claude's weekly per model and monthly spend; Codex's monthly and credit limit | `crates/agents/src/usage.rs`, `crates/core` state | S | none new |
| U2 | **Codex reset credits**: the count left and the next expiry, from `rateLimitResetCredits` in the same read | `crates/agents/src/usage.rs` | S | none new |
| U3 | **Token format and pace**: 1.2K, M, B or T rounding-aware; *ahead*, *on pace* or *under* from the time used against what is used | `ui/src/cost.ts`, chat ring, Records | S | none |
| U4 | **Local usage scan**: Claude's transcripts and Codex's sessions, incremental with an offset cache, deduplicated, into a `usage` table per hour, provider, model and token kind; also sessions Vults never watched. Replaces most of W9b | `crates/agents/src/usage_scan.rs`, `crates/store` migration | L | none |
| U5 | **Cost math**: the 1-hour cache-write rate, speed and context tiers, a split by kind, cache savings; a reported cost wins and the split scales to it; each figure says *reported*, *priced* or *unpriced* | `crates/agents/src/prices.rs` | M | none new (models.dev already gated) |
| U6 | **The Usage page** in the Nest: Cost, Tokens or Limits; 24 h, 7, 30 or 90 days; a hand-written SVG chart per provider in its brand mark's color; tiles; share bars; a table by model or day; limits with *Resets 18:00 · in 2 h*, pace and credits | `ui/src/surfaces/nest/usage.ts`, store queries | L | none |
| U7 | **Settings → History and budget**: how long usage is kept, a price override per model, model aliases | `settings/budget.ts`, `app/src/settings.rs` | M | none |
| U8 | Docs (guide and safety in both languages), checks for the scan, the math and the format | `docs/`, `scripts/` | M | none |

To decide: **U9**, Claude's reset credits and *Redeem*. They need the CLI's OAuth token and a direct call to Anthropic, so an ADR first. One option: ask the CLI for them if a future `get_usage` returns them, and show nothing until then.

## 12. Connections: issues and pull requests you read, review and approve

**The goal.** In Connections, a list beside a detail pane, as t3code's Issues and Pull Requests pages.
You read the real item (description, labels, timeline, code) and you act from there: comment, close,
review, approve, merge. Each act is a preview and a click.

### What t3code does

**Issues** (`apps/web/src/components/issue/`):

- **The list** (`IssuesPage.tsx`):
  - Open, Closed or All; *Everyone*, *Assigned to me*, *Opened by me* or *Mentioning me*.
  - Sorted by *Recently updated* or *Newest*.
  - Search, label and assignee filters, refresh.
  - Grouped by provider and host, with a *not configured* note and *Load more* per group (`:572-655`).
  - Saved views (`issueSavedViews.ts`).
- **The detail** (`IssueDetailPanel.tsx:432`):
  - `owner/repo#n` with a link, an Open/Closed menu and refresh.
  - An editable title, then the author and *opened 51d ago*.
  - Labels as colored chips with a picker, and assignees with a picker.
  - The description in Markdown, task boxes included.
  - Reactions and linked pull requests.
  - An activity timeline mixing events and comments, with *Load older* (`:935-1030`).
  - A comment composer (`IssueCommentComposer.tsx`).

**Pull requests** (`components/pullRequest/`, `PullRequestDetailPanel.tsx:358`) have four tabs:

- **Summary** (`PullRequestSummaryTab.tsx`): facts, description, checks, and comments with their file and line, plus *add to chat*.
- **Timeline** (`PullRequestTimelineTab.tsx`): conversation, review threads, commits, lifecycle events, and reviews with their verdict.
- **Code** (`PullRequestCodeTab.tsx`): the diff per file with *Viewed*, inline threads with *Resolve*, and draft inline comments.
- **Guide**: a review tour written by a model.

Around the tabs:

- a **review form** with Comment, Approve or Request changes (`PullRequestReviewForm.tsx:62`);
- **merge** with a method and auto-merge, ready or draft, update branch, close, delete branch (`PullRequestActionsMenu.tsx`);
- a **checks** popover;
- stacks.

**Data.** GitHub GraphQL plus REST with a token: one from settings, from the environment, or from `gh auth token` (`GitHubCredentials.ts`). There are plug-ins for GitLab (`glab`), Forgejo, Bitbucket, Azure, Jira and Linear.

### What Vults has

**The UI** (`ui/src/surfaces/nest/connections.ts`):

- A chip per connector, a repository filter, search and refresh.
- Tabs Inbox, Pull requests, Actions and Releases.
- A detail pane with the ref, title, branch, author, diff size, reviewers, a checks list and technical facts.
- Buttons: *Open in GitHub*, *Seen*, *Ask Zeca*.
- The only write from the UI is **Squash and merge**, with two clicks.
- Nothing else: no description, comments, timeline, labels, issues list or code view.

**Reads** (`crates/connectors/src/github.rs:47`):

- The board: your open PRs, with reviews, checks and diff size.
- Your repositories.
- `review-requested:@me`.

Richer reads exist only as Zeca's tools (`app/src/flock_tools.rs`):

- `work_item`: body, state, comments, reviews, threads, files, checks and mergeable;
- `pull_request_diff`;
- Actions runs, jobs and logs;
- branch rules.

GitLab and Gitea are read-only snapshots.

**Writes already built, each prepared, shown, clicked and audited:**

- `IssueWrite` (`github.rs:622`): create, edit, comment, labels, assignees, close and reopen.
- `PullRequestWrite` (`github/pull_requests.rs:48`): create, edit, ready, draft, reviewers, and **SubmitReview with Comment, Approve or Request changes**, including inline comments on a line and the approved head SHA as `commit_id`.
- `MergeControl`: merge and delete branch.
- `ActionsControl`: rerun and cancel.

Today they are reachable only through Zeca's GitHub tools. `ui/src/island/markdown.ts` renders Markdown as DOM, task boxes included.

So most of the backend is done. What is missing is reads the UI can call and the panes themselves.

### Rules

- Reads go through `gh api` (the token stays with `gh`) and through the gate, under the GitHub connector's switch, which is the one already on.
- Every write is the existing prepared write: a preview card in the Nest, a click, the fresh check before it runs (head SHA, state), and the audit.
- Approving needs the same click as any write. Nothing approves or merges by itself.
- Text from an issue or PR is untrusted: it is rendered as DOM, never HTML. It reaches Zeca only quoted, through *Ask Zeca* or *Work on this* (X6).

### Steps

| # | Step | Where | Size | Kind |
|---|---|---|---|---|
| C1 | UI reads: `github_item_detail(repo, n)` and a paged `github_item_timeline`, reusing `work_item` and adding labels with colors, assignees, author, reactions, linked PRs and timeline events | `crates/connectors/src/github.rs`, `app/src/connectors.rs` | M | read |
| C2 | **Detail pane v1** for issues and PRs: header, state pill, labels, assignees, body in Markdown, read-only timeline, linked PRs, reaction counts; *Open in GitHub*, *Ask Zeca* | `ui/src/surfaces/nest/connections-detail.ts` | M | UI |
| C3 | **Issues tab**: `is:issue involves:@me`, with Open, Closed or All, involvement, sort, search, label filter and refresh, grouped by connector and repository | `github.rs`, `connections.ts` | M | read + UI |
| C4 | **Comment** on an issue or PR from the pane (the existing `IssueWrite::Comment`): a preview in the Nest, a click, audited; usable with the connector on, not only Zeca's tools | `app/src/connectors.rs`, a Nest preview card | S | write |
| C5 | **State, labels, assignees**: the Open/Closed menu and pickers (a repository-labels read) | same | S | write |
| C6 | **PR tabs, Summary and Timeline**: description, checks, comments with file and line; reviews, threads, commits, events | `connections-detail.ts` | M | UI |
| C7 | **Review**: Comment, Approve or Request changes (summary required for changes), through `SubmitReview`; the preview names the head commit | same | S-M | write |
| C8 | **Code tab**: files and diffs from `pull_request_diff` (`Diff`, `FileDiff` and `Hunk` already in `view.gen.ts`), threads at their lines | same | L | read + UI |
| C9 | Inline comments drafted in Code and sent with the review; **Resolve** a thread (a new GraphQL write) | same, `github/pull_requests.rs` | M | write |
| C10 | **Viewed** files, kept locally in the store (no write to GitHub) | `crates/store` | S | local |
| C11 | Merge method (squash, merge, rebase), ready or draft, rerun failed checks from the checks list: the controls exist, the menu is new | `connections-detail.ts` | S | write |
| C12 | Docs (connectors guide in both languages, safety table), checks for the pane, the previews and stale heads | `docs/`, `scripts/` | M | — |

Later: saved views (local, S), GitLab and Gitea detail reads (M each), a review guide written by Zeca
(needs Zeca's chat on; L), stacks (skip).

Order: C1, C2, C3 (the first useful slice: read everything), then C4, C5, then C6, C7 (approve from
the pane), then C8 to C11, and C12 with each slice.

## 13. Icons across the whole app

**Why.** Settings now uses Lucide, as t3code does, and each provider, agent and connection shows its
official mark (`ui/src/brands.ts`). Next to them the rest of the app looks older: icons drawn by hand,
text glyphs used as icons, sizes and strokes that differ from one surface to the next. Changing
everything at once gives one visual language.

**Done so far.** `ui/src/island/icons.ts` holds Lucide's drawings (ISC, in NOTICE); the flock keeps
its own pixel path. `brandMark()` draws the official marks, in the brand's color unless it is black
or white.

**The rule** (DESIGN.md, *Icons*). One set, Lucide, through `icon(name, size, stroke)`: 14 px in
rows and menus, 16 px in buttons and tabs, 20 px in empty states, stroke 1.75 (2 on small sizes).
A brand appears only through `brandMark()`. No text glyph stands in for an icon (✓ ✕ ▾ ↗ ⎇ ●), and an
icon never carries meaning alone: the button has its words or an `aria-label`. The pixel-art
characters (Zeca, the flock) stay as they are: they are the product, not icons.

| # | Step | Where | Size |
|---|---|---|---|
| I1 | **Inventory**: every icon, hand-drawn SVG and glyph in the UI, by surface, with the Lucide name or brand mark that replaces it; a check that fails on a new inline SVG path or glyph icon outside `icons.ts` and `brands.ts` | `scripts/check-icons.mjs` | S |
| I2 | **The island**: the dock, cards (permission, question, done, failed), the chat's tool steps, attachments, the composer's buttons, the model and engine picker (brand marks) | `ui/src/island/` | M |
| I3 | **The Nest**: tabs, the flock list, roosts, records, connections (each forge's mark, PR and issue states, checks), activity, the workspace bar (editors' marks) | `ui/src/surfaces/nest/` | M |
| I4 | **The rest**: the corner widget, the dictation overlay, Settings leftovers, empty states | `ui/src/surfaces/widget/`, `dictation-overlay/`, `settings/` | S |
| I5 | **The tray**: one monochrome icon per state (idle, working, needs you, paused) in the panel's style, light and dark | `app/icons/tray/`, `crates/platform` | S |
| I6 | **The app icon**: Zeca's head redrawn for 16 to 512 px, sharp at each size; `.desktop`, AUR and installers | `app/icons/`, `packaging/` | M |
| I7 | DESIGN.md's icon rule written out with the sizes; screenshots in the docs refreshed where icons changed | `DESIGN.md`, `docs/` | S |

Order: I1 first (it finds everything and keeps it fixed), then I2 to I5 in one pass per surface, I6
and I7 last. It runs after the features above so every new surface is drawn once, already in the
new style.
