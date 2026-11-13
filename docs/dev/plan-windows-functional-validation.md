# Windows functional validation

Priority changed by the user on 2026-11-11: stabilize existing Windows behavior before adding
connectors or other product features. Notion and further connector expansion are deferred.
GitLab live-read work is preserved on its work branch/stash and is not part of these fixes.
Keep Linux behavior and CI coverage intact. Deliver isolated, tested PRs and clean their build caches.

## Acceptance matrix

| Area | Required evidence | Current status |
| --- | --- | --- |
| Startup and lifecycle | Current build; single instance; owned child processes stop on timeout/exit; closed agent sessions retire | Doctor cleanup fixed with native fixtures; broader app/session acceptance pending |
| Room and Settings | Gear/navigation, back/close/reopen, retained drafts, reachable controls and no unexpected click interception | Broad native WebView pass pending |
| Scale and placement | 100%, 125%, 150%, 175%, 200%; short windows and monitor changes; physical input distinguished from renderer emulation | Connector renderer checks exist; broad/native DPI acceptance pending |
| Agents and hooks | CLI discovery, launchers with spaces, install/status, trusted same-user pipe events, start/end and stale-session behavior | Existing fixtures/CI retained; current Windows pass pending |
| Shortcuts | Existing Windows chords respected, conflicts visible, customized/disabled chords persist, press/release and cancel | Existing checks retained; current Windows pass pending |
| Dictation and commands | Device/model readiness, startup latency, transcription/delivery/cancel, focus loss and clipboard restoration | Inventory and synthetic tests first; live speech acceptance separate |
| Audio and platform capabilities | Playback/session controls, app launchers, file/URL opening, notifications; unsupported paths identified explicitly | Audit in progress |
| Distribution and persistence | Fresh installer with source/build identity, update/restart, settings/models preserved, spaces/Unicode paths | Installed executable has a 2026-11-08 timestamp; source-labelled candidate pending |

## Evidence rules

Use isolated profiles and owned test processes. Do not overwrite the user's agent configuration,
credentials, shortcut preferences or voice models for a smoke test. Synthetic API/CLI/audio fixtures
and renderer scale emulation do not establish real-account, microphone, physical DPI/input or
desktop acceptance. Report concrete failures and remaining acceptance gaps; do not label Windows
100% functional merely because compilation or synthetic checks pass.

## First investigation

The agent doctor bounds version probes to three seconds. Its Unix timeout kills a process group;
Windows previously killed only the initial process. A private `.cmd` regression fixture reproduced
a relaunched worker continuing after timeout: its heartbeat advanced from 146 to 156 after return.
Diagnostics now start suspended, enter an anonymous non-inheritable kill-on-close Windows Job,
then resume only their verified process thread. Closing that job stops its owned descendants on
success, failure and timeout; setup failures stop the still-suspended process. Unix behavior is unchanged.

Native Windows tests cover inherited stdout, workers without stdout, failed and successful probes,
normal npm launchers and paths with spaces. All 1,537 workspace/eval tests, 24 UI checks, fmt,
workspace/all-target clippy with warnings denied, static checks and cargo-deny passed locally.
These diagnostic process tests do not establish physical desktop, real-speech or complete Windows acceptance.

## Hook commands per agent shell (2026-11-13)

Antigravity, Droid, Kimi, Qwen, Cursor and Copilot were given the Git Bash command
(`'C:/…/vults-hook.exe' --agent …`), and their Windows shells read those single quotes literally.
A probe workspace (`.agents/hooks.json`) showed agy 1.5.8 running `cmd /s /c "<command>"`, so its
sessions never arrived; `|| exit 0` hid the failure. The other shells come from their code or docs:
Droid `%ComSpec% /d /s /c`, Kimi `%ComSpec% /c`, Qwen cmd (Git Bash under MSYS), Cursor and Copilot
PowerShell (Copilot's `powershell` key). A plain path is now written unquoted with forward slashes,
the same for cmd, PowerShell and Git Bash; other paths are quoted per shell. The ignored native test
`windows_hooks_run_in_each_agents_shell` passes in cmd, Git Bash, PowerShell 5.1 and pwsh. Only
Antigravity ran live; Cursor, Copilot, Droid, Kimi and Qwen were not installed. Qwen in cmd mode
still cannot run a hook path with a space.

## Console windows (2026-11-13)

The release app is a GUI process, so each console child started without `CREATE_NO_WINDOW` got a
window: with Windows Terminal as the default terminal, a Terminal window that stayed while the child
ran. Launching the installed 0.1.8 showed one 0.58 s in, owned by `gh api graphql` (the GitHub
connector's first poll), and again on each poll; the turn's `git` looks and the OpenCode plugin's
detached relay did the same. With `gh`, `folder.rs`'s git and the plugin's `windowsHide` fixed, the
branch's release build started `gh` at 0.6 s and opened no console window in 20 s.

## `/continue` (2026-11-13)

`/continue` refused every Windows session ("Vults can't tell whether that session is still open"):
its process check read `/proc` and was unknown elsewhere, while Windows hooks always send a PID. It
now uses the presence watcher's probe (same-user process, exited or not), on Linux too.
