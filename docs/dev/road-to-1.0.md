# Road to 1.0: gaps and ports from the reference app

Internal checklist (2026-10-02). Two sources:

- **Gaps**: what the original plan (phases F0 to F6) promised and the code does not do yet.
- **Ports**: what the reference app (the MIT project named in `NOTICE`) does better and is worth
  bringing here. It was reviewed at upstream `main` `e98c182` (its 0.1.1), plus the
  `n22-gemini` branch. Paths written `REF/...` are relative to its repository root; `REF/mac/` is
  its macOS Swift folder and `REF/windows/` its Tauri app (which also builds for Linux).

Each item says what is wrong or missing, where it lives here, where the reference does it, and
how we know it is done. Check items off as they land; delete the file once it is empty.

---

## 1. Robustness fixes (small, do first)

### 1.1 Writing an agent config replaces a symlink with a plain file

**Status: done** (2026-10-02).

- **Here**: `crates/agent-config/src/lib.rs:136-145`. We write `.<name>.tmp-<pid>` beside `path`
  and `rename` it over `path`.
- **Problem**: when `~/.claude/settings.json` (or `~/.codex/hooks.json`) is a symlink into a
  dotfiles repo (stow, chezmoi, home-manager), the rename replaces the link. The dotfiles copy
  stops receiving updates and the user's setup silently forks.
- **Reference**: `REF/windows/src-tauri/src/hooks.rs:306` resolves the path with
  `std::fs::canonicalize` before writing, so the temp file and the rename happen beside the link's
  target and the link survives (commit `bef6558`).
- **Fix**: resolve `path` with `canonicalize` when it exists (keep `path` when it does not), and
  write the temp file and the backup beside the resolved target. The diff and fingerprint still
  read the same bytes.
- **Done when**: a test that installs into a symlinked config file passes. After the write the
  link is still a link, the target holds the new content, and the backup sits beside the target.

### 1.2 Writing an agent config widens its permissions

**Status: done** (2026-10-02).

- **Here**: same place. `std::fs::write(&temp, …)` creates the file with the umask (usually 0644),
  and the rename carries that mode over.
- **Problem**: a config the user locked to 0600 becomes world-readable after an install.
- **Reference**: `REF/windows/src-tauri/src/hooks.rs:327` `write_like` creates the temp file
  0600 and then copies the original's mode onto it before the rename.
- **Fix**: on unix, open the temp file with `OpenOptions::mode(0o600)` and then
  `set_permissions` it to the original's mode when an original exists. A new file stays 0600.
- **Done when**: tests cover an original at 0600 (it stays 0600) and an original at 0644 (it
  stays 0644).

### 1.3 The socket's fallback directory is trusted without a check

**Status: done** (2026-10-02).

- **Here**: `crates/ipc/src/lib.rs:108-113` creates the parent directory with mode 0700. If that
  directory already exists, nothing checks it. The path comes from `crates/protocol/src/lib.rs:143`
  `socket_path`, which falls back to the temp directory when `$XDG_RUNTIME_DIR` is unset.
- **Problem**: in `/tmp`, another user can create the directory first, or plant a symlink, and
  then observe or squat the socket. Peer credentials still protect each connection
  (`crates/peer`), but the directory itself should be ours.
- **Reference**: `REF/windows/src-tauri/src/platform/linux.rs:96-105` (app) and
  `REF/windows/hook/src/unix.rs:29-35` (relay) both require `symlink_metadata` to show a real
  directory, owned by our uid, with `mode & 0o077 == 0`. Otherwise they refuse.
- **Fix**: run the same check in `serve_unix` after creating the directory, and in the hook
  before it connects. On failure the hook exits 0 with empty output (rule 1); the app logs the
  failure and does not serve.
- **Done when**: a unit test proves a symlinked directory and a 0755 directory are both refused.

### 1.4 Crash on quit (carried over from plan F1)

**Status: done** (2026-10-02), nothing to fix. The release build, quit from the tray menu (the same
`app.exit(0)`) with the island open: 10/10 clean on X11 (openbox) and 10/10 on KWin Wayland with
layer-shell, plus 5/5 with Settings open and 5/5 mid chat turn. Clean means exit 0, no
`free()`/abort line on stderr, and no core dump. Each run was isolated (nested display, private
bus) and clicked "Quit" through the tray's `com.canonical.dbusmenu`.

- The original port printed `free(): corrupted unsorted chunks` on exit. Nobody has shown it is
  gone here.
- **Done when**: launching and quitting from the tray 10 times in a row, under KWin, leaves no
  abort in `journalctl --user` and no core dump.

---

## 2. Features

### 2.1 Choose the island's monitor, and follow display changes

**Status: done** (2026-10-02, tested with two screens on KWin). Also fixed on the way: when its
output left, the compositor closed the surface, gtk-layer-shell called `gtk_window_close`, and Tauri
destroyed the island for good. The island now refuses to close and maps itself again.

- **Here**: `crates/platform/src/linux.rs:30` `init_island` never calls `set_monitor`, so the
  compositor picks the output (usually the focused one at map time). Nothing reacts when monitors
  are plugged in or out, or when the scale changes.
- **Reference**:
  - `REF/windows/src-tauri/src/island.rs:118` `target_monitor` lets the user choose the primary
    monitor or the one under the cursor.
  - `island.rs:195` `spawn_cursor_poll` compares monitor position, size and scale and emits
    `screen-changed`, and `apply_geometry` (`island.rs:152`) re-applies the layout.
  - On Linux it polls only twice a second (commit `e98c182`: idle CPU fell from 3.1% to 0.4%).
- **Fix, our way**: use GDK instead of polling. Watch `gdk::Display`'s `monitor-added` and
  `monitor-removed` signals, and pass the chosen output to `gtk_layer_shell::set_monitor`. Add a
  setting with the values "Primary", "Focused at start" and a list of connector names
  (`gdk::Monitor::model` / connector). It goes in `app/src/settings.rs` and must be documented in
  `docs/reference/settings.md`.
- **Gotcha**: the surface must not be resized or re-mapped (see `CLAUDE.md`). Moving it to another
  output may need one unmap/map cycle; test that on KWin before relying on it.
- **Done when**: with two monitors, the setting moves the island, and unplugging its monitor moves
  it to the remaining one without restarting the app.

### 2.2 Generic agents (`--agent <name>`)

**Status: done** (2026-10-02, tested through the real hook). Protocol version 2.

- **Here**: `crates/protocol/src/lib.rs:39` `AgentKind` is a closed enum. The hook maps an unknown
  `--agent` to Claude (`crates/hook/src/main.rs`, test `parses_agent_and_event`), so a third-party
  tool shows up as a Claude session.
- **Reference**: commits `91654c7` and `46dfa67`. The relay adds the agent name to the payload.
  The name must match `^[a-z0-9-]{1,24}$`, and `claude` is reserved so nothing can impersonate it.
  The app creates a pill with an id-hash color. Permission requests from external agents are not
  answered: the terminal asks the user instead. `Stop` / `SessionEnd` remove the pill.
- **Fix**: add `AgentKind::Other(name)` (validated, built-in names reserved) to the protocol,
  which needs a protocol version note in `docs/reference/protocol.md`. Activities are generic.
  Never answer a `PermissionRequest` for `Other` (empty output, so the agent asks the user
  itself). The bird's color is derived from the name.
- **Done when**: `vultures-ai-hook --agent my-tool Stop` with a minimal JSON payload makes a
  session appear on the wire, labeled `my-tool`, and an approval from it never shows a card.

### 2.3 Gemini CLI and Antigravity

- **Here**: not supported.
- **Reference**: branch `n22-gemini` only, not upstream main yet. Commits `0fdbda2` (Windows/Linux)
  and `b202eca` (macOS).
  - `REF/windows/hook/src/main.rs:118` `normalize_event`: `BeforeTool` / `BeforeToolSelection` →
    `PreToolUse`, `BeforeAgent` → `SubagentStart`, `PreInvocation` → `UserPromptSubmit`, …
  - `:140` `normalize_tool_fields`: `toolCall{name,args}` → `tool_name` / `tool_input`, with
    PascalCase keys flattened (`CommandLine` → `command`). The session id comes from
    `conversationId` / `sessionId` / `$GEMINI_SESSION_ID` (`:176`, `:268`).
  - Install targets: `~/.gemini/settings.json` (timeouts in ms) and `~/.gemini/config/hooks.json`
    (timeouts in seconds).
- **Fix**: add a `crates/agents/src/gemini.rs` agent next to `claude.rs` and `codex.rs`. Put the
  normalization there, not in the hook (the hook stays dumb and fast). Its install entries go
  through `crates/agent-config` with the same backup, diff and click.
- **Validate before building**: get the exact hook schema and the approval reply format from a
  real Gemini CLI session, and record a fixture under `crates/agents/tests/fixtures/` the way
  `codex-session.jsonl` was. The reference never got approvals working for these agents.
- **Done when**: a Gemini session shows on the wire with readable steps, and Allow/Deny from the
  island reaches it (or, if Gemini has no approval hook, that is written down in `docs/guide/`).

### 2.4 Subscription usage on the island (carried over from plan F6)

- **Here**: only `Status::RateLimited` (`crates/core/src/lib.rs:479`), set from `StopFailure` with
  `error: "rate_limit"` (`crates/agents/src/claude.rs:84`). There is no quota or percentage.
- **Reference**: none. It also detects rate limits only from notification text.
- **Sources, validated 2026-10-02** (real reads, fixtures in `crates/agents/tests/fixtures/`):
  - **Codex** (0.159.3): `codex app-server`, after `initialize`, answers
    `account/rateLimits/read` with `{"excludeResetCreditDetails": true}` (the background-poll
    form). Fixture: `codex-rate-limits.json`. Each window is
    `{usedPercent, windowDurationMins, resetsAt}` (epoch seconds) under `rateLimits.primary` /
    `.secondary`, plus `planType`. **Name a window by `windowDurationMins`, never by its slot**:
    on a `prolite` plan `primary` is the weekly window (10080) and `secondary` is null. The
    server also pushes `account/rateLimits/updated`. Reading costs nothing. Never send
    `supportsLunaReserve` (it records an experiment exposure on the account), and never call the
    other `account/*` methods: `rateLimitResetCredit/consume` spends a reset,
    `sendAddCreditsNudgeEmail` sends mail.
  - **Claude Code** (2.1.286): only the statusLine command's stdin carries it, as
    `rate_limits.five_hour` / `.seven_day` = `{used_percentage, resets_at}` (epoch seconds).
    It shows up only after the session's first API response (`null` before), only for Pro/Max
    logins, and only in the interactive TUI (`-p` is undocumented). Fixture:
    `claude-statusline.jsonl` (before and after the first reply). No other documented source
    exists: `/usage` is local history, and there is no usage endpoint.
- **What that means for the build**: Codex can be polled by the app (spawn `codex app-server`,
  read, close; or listen on the chat's server when it runs). Claude needs a statusLine entry in
  `~/.claude/settings.json` that runs `vultures-ai-hook statusline`: forward `rate_limits`, then
  print the line. That is an agent-config write: dated backup, diff, click (rule 3). A statusLine
  the user already has must keep working: chain to it and print its output, never replace it.
- **Done when**: the island shows the 5-hour and weekly usage for each logged-in CLI. The data
  must come only from the CLIs themselves, never from a token we read.

### 2.5 Model picker for the API chat (optional)

- **Here**: `crates/chat/src/api.rs:19` hard-codes `MODEL`.
- **Reference**: `REF/mac/Sources/App/ClaudeService.swift` fetches `GET /v1/models?limit=100`,
  filters the list, falls back to a static list, and accepts a custom id. Readable errors when a
  model is not found (commit `4c61595`).
- **Fix**: a setting, filled from `/v1/models` with the keyring key, plus a "custom id" field.
  Keep `fallbacks: "default"`.

### 2.6 Server-side web search in the API chat (optional)

- **Reference**: `REF/windows/src-tauri/src/claude.rs:112` sends
  `{"type":"web_search_20260209","name":"web_search","max_uses":5}`.
- **Fix**: add the tool to the request in `crates/chat/src/api.rs`, and show search steps in the
  chat the way CLI tool steps are shown.

### 2.7 `.rpm` package (optional)

**Status: done** (2026-10-02). `deb` and `rpm` are built on Ubuntu 22.04 and the AppImage on 24.04;
both pin gtk-layer-shell 0.6 or newer (the keyboard's on-demand mode); a separate publish job writes
`SHA256SUMS` after merging the packages.

- **Here**: `app/tauri.bundle.linux.json` builds `deb` and `appimage`.
- **Reference**: commit `06c57ea` adds `rpm`. It also pins `libgtk-layer-shell0 (>= 0.6)` in the
  deb depends and publishes `SHA256SUMS` from a separate publish job (`bef6558`). Both are worth
  copying into `.github/workflows/release.yml`.

---

## 3. Release 1.0 (Linux)

- Version is `0.0.0` in `package.json` and `app/tauri.conf.json`, and there is no tag. The
  release workflow (`.github/workflows/release.yml`, fixed in `27fab92`) has never run on a real
  tag.
- **Done when**: tag `v1.0.0` → draft release with AppImage, `.deb` (and `.rpm` if 2.7 landed),
  `SHA256SUMS`, and an updated AUR `PKGBUILD`; README stops saying "build from source".

---

## 4. Decided against (do not port)

| Reference behavior | Why not |
|---|---|
| "Always" answered with Claude Code's `permission_suggestions` as `updatedPermissions` (`REF/mac/Sources/App/HookServer.swift:1200`) | Claude Code would then write its own settings without our backup and diff (rule 3). Our rules (`22b8068`) stay in the app. Revisit only if users ask for rules that work with the app closed. |
| WAV sounds through GStreamer, and the `GST_REGISTRY` AppImage fix (`4c55ab8`) | Our sounds are synthesized in WebAudio (`ui/src/sound.ts`); no media framework is bundled. |
| Polling the cursor for the island's monitor | Wayland gives no global cursor; GDK monitor signals are cheaper (2.1). |
| Only VS Code sessions shown (mac `HookServer.processEvent`) | A bug in the reference; every terminal counts here. |
| Full JSON shown as the "diff" (mac `SettingsView.swift`) | We show a real diff. |
| Placeholder question/error views, no Y/N shortcuts, no streaming | Already done properly here (AskUserQuestion, Ctrl+Alt+Y/N, streaming). |
| Pollers that never pause | Ours sleep while disabled, with no timers (`crates/connectors/src/runtime.rs:101`), and back off on errors and rate limits. A global "pause all" from the tray (the reference has one) is a possible small addition. |

## 5. Small ideas (no commitment)

- Click Zeca three times quickly and he gets dizzy for a few seconds (the reference does this
  with its mascot). It would be a new clip in `design/mascots/zeca/zeca.py`, plus a visual test.
- Badges on birds that are not focused (done / error) instead of switching the view. Check
  first whether the flock rows already cover this.
