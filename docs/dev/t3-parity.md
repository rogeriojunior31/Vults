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

## 5. Scheduled tasks

t3code: a task (title, prompt, schedule `interval` / `fixed_time` / `once`, project, model,
permission mode, thread mode new or continue) in SQLite; a scheduler in the server ticking every 5 s,
one run at a time; a fixed-time run over 10 minutes late is skipped as missed; a run is a thread
whose message says a scheduled task sent it, with a badge in the timeline.

Vults: Zeca's sleep is the scheduler pattern (`app/src/sleep.rs`: local time, waits while busy,
records a day as done only after it succeeds); specialists have an access level each; the island
already flies a bird in and out when a key appears in or leaves the flock (`ui/src/island/scene.ts`,
`ui/src/character/director.ts`).

The rule: a run unattended may use only what the user chose for that specialist beforehand: its
access level (*asks first*, *your policies*, *full access*), never more; destructive actions and an
agent's questions still wait for the user, the bird holding on the island and a notification; a
card nobody answers by its deadline stops the run. Rule 2 says so in CLAUDE.md before the code
lands.

1. Design written here and in CLAUDE.md rule 2 and 4: access, missed runs, the gate feature. S
2. Store: `scheduled_tasks` (specialist, title, prompt, schedule, folder, new or continue, access,
   enabled, next and last run, status, error, count) and a runs table. M
3. Gate feature `ROUTINES`: switch, budget, trace. S
4. A run with no window: `Origin::Scheduled(task)` in `app/src/chat.rs`, the specialist's runtime and
   `settle_in_route` with its access capped; the prompt marked as sent by a scheduled task; no run
   over another on the same specialist. L
5. Scheduler `app/src/routines.rs` (from `sleep.rs`): about every 30 s, next run in local time, the
   missed window, rows left running recovered at start, one run at a time. M
6. On the island: a running task is a flock entry (working, approval, done) with the specialist's
   species and look, so it flies in, works, notifies and flies out after a moment; looks for every
   bird. M
7. Notifications: started, needs you, finished or failed, *Open* to its conversation. S
8. In Specialists: *Scheduled tasks* beside each specialist (list, create, edit, on/off, run now,
   last status); a *Scheduled* mark on the run's message and conversation. M-L
9. Docs. S

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
