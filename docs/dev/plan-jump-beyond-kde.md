# Open terminal beyond KDE

Internal board (2026-10-09), checked against the code at `52cbc0d`. Deleted when the PR merges.

*Open terminal* raises the session's window only on KDE (a KWin script). Everywhere else it can
only focus a multiplexer pane, and the quick actions say "This desktop can't bring a terminal
forward" (ADR 0011). This step makes it work on any X11 session (and XWayland windows), fixes the
kitty path, and hardens the KWin one. The mechanics come from the reference app's
`windows/src-tauri/src/platform/linux_focus.rs` (MIT, attributed in `NOTICE`); the code is
rewritten to our style, not copied.

## 1. What the code says today

| Fact | Where | What it means |
|---|---|---|
| A window is raised only when `XDG_CURRENT_DESKTOP` contains `KDE` | `crates/platform/src/jump.rs` (`jump`) | Xfce, i3, Cinnamon, MATE, GNOME on X11: pane only, never the window. |
| `raise` in the view mirrors that rule by hand ("keep in step") | `crates/core/src/view.rs` (`raises`) | Both sides change together, with tests on each. |
| The hook passes `XDG_CURRENT_DESKTOP`, but not `XDG_SESSION_TYPE`, `DISPLAY` or `WAYLAND_DISPLAY` | `crates/hook/src/main.rs` (`TERMINAL_VARS`) | Core cannot tell an X11 session from a Wayland one. |
| `kitty @ focus-window` runs with no `--to` | `jump.rs` (`multiplexer_commands`) | Outside kitty, `kitty @` has no socket to talk to: it fails unless the app itself was started from kitty. `KITTY_LISTEN_ON` is not captured. |
| The KWin script file is `$TMPDIR/vults-jump-<pid>.js`, written with `std::fs::write` | `jump.rs` (`kwin_activate`) | A predictable name in a shared folder (a symlink planted there gets written through). Use `$XDG_RUNTIME_DIR`, `create_new`, mode 0600. |
| The KWin script reports nothing; `jump` counts a loaded script as success | `jump.rs` | When no window matches, the island never says *jump-failed*. |
| A minimized window stays minimized | KWin script | Raise and unminimize. |
| `ancestors` walks into processes of other users (sshd, the display manager) | `jump.rs` (`ancestors`) | Stop at a process that is not ours, like the reference does. |
| No X11 client crate in the tree (`x11`/`x11-dl` come through gtk/tao, FFI only) | `Cargo.lock` | Add `x11rb` (pure Rust, MIT/Apache-2.0, `default-features = false`): `cargo deny check`. |

## 2. Design

- **D1. Which way, per session.** A pure function `methods(session_type, wayland_display,
  display, desktop) -> Vec<Method>`, best first:
  - Plasma on Wayland: `KWin`, then `X11` (an XWayland terminal);
  - any X11 session: `X11`, then `KWin` on Plasma X11;
  - Wayland outside Plasma with `DISPLAY` set: `X11` only, for XWayland windows;
  - nothing else.
- **D2. What the view promises (`raise`).** Only what works for native windows too: a pane, or
  `pid` known **and** (Plasma, or an X11 session). On GNOME/Hyprland/Sway Wayland the button stays
  greyed with today's hint, even though `jump` still *tries* X11 for an XWayland terminal: a
  bonus, never a promise (ADR 0011). Core reads the session's env, as today; the hook adds
  `XDG_SESSION_TYPE`, `DISPLAY` and `WAYLAND_DISPLAY` to `TERMINAL_VARS`.
- **D3. X11 through EWMH.** One connection: `_NET_CLIENT_LIST`, every window's `_NET_WM_PID` in
  one round trip, keep those owned by an ancestor (nearest first). With several windows, prefer
  the one whose title contains the session folder's name (case-insensitive), else the first.
  Switch to its `_NET_WM_DESKTOP` (`_NET_CURRENT_DESKTOP` message), then `_NET_ACTIVE_WINDOW`
  with source 2 (pager), timestamp 0, so window managers honour it over focus-stealing prevention.
- **D4. kitty.** The hook captures `KITTY_LISTEN_ON`; `jump` passes `--to <it>` and matches the
  window by `id:` as today. Only a `unix:` socket of plain path characters is accepted (no `tcp:`,
  no spaces, at most 107 bytes). Without it the kitty command is not run at all.
- **D5. KWin, hardened.** Script in `$XDG_RUNTIME_DIR` (created 0700 by systemd; refuse if not
  ours), `create_new` + 0600, unique name per call. Unminimize, prefer the window titled after the
  folder (the folder travels as `String.fromCharCode(...)` codes: nothing of it can be read as
  code). The script answers over D-Bus (`callDBus` to our unique name, checked as `:N.M`) with
  `Activated` or `NotFound`, read through zbus (already in the tree) with a 2 s limit, so
  `jump` returns the truth. This replaces `gdbus` and the fixed 300 ms sleep.
- **D6. Never a shell, never a payload value as code.** As today: arguments one by one; only
  numbers from /proc and checked ids reach a command or a script.

## 3. Steps (one PR)

| # | What | Files | Done when |
|---|---|---|---|
| 1 | Hook: add `XDG_SESSION_TYPE`, `DISPLAY`, `WAYLAND_DISPLAY`, `KITTY_LISTEN_ON` to `TERMINAL_VARS` | `crates/hook/src/main.rs` | Hook tests pass (the protocol doc names no variable: unchanged) |
| 2 | `methods()` pure + table test (the reference's cases: Plasma Wayland/X11, Xfce, GNOME Wayland with and without `DISPLAY`, tty) | `crates/platform/src/jump.rs` | Test green |
| 3 | `raises()` follows D2; tests for X11 Xfce (true), GNOME Wayland (false), Plasma (true), no pid (false) | `crates/core/src/view.rs`, `crates/core/src/tests.rs` | Core tests green, the "keep in step" comment points at `methods` |
| 4 | `ancestors` stops at a process of another user | `jump.rs` | Test: the walk from our own pid never holds pid 1 |
| 5 | X11 module (D3) with `x11rb`; window choice is a pure fn with tests (nearest ancestor wins, folder title wins among its windows) | `crates/platform/src/jump/x11.rs`, `crates/platform/Cargo.toml` | Unit tests; `cargo deny check` clean |
| 6 | kitty `--to` (D4) and socket check with tests (valid, `tcp:`, spaces, `$(…)`, empty) | `jump.rs` | Tests green |
| 7 | KWin hardened (D5); tests: only a unique bus name and numbers are in the script, the folder as codes | `crates/platform/src/jump/kwin.rs` | Tests green; manual on Plasma 6 Wayland |
| 8 | `jump(t)` runs pane first, then each method until one says it raised | `jump.rs` | `jump-failed` shows when no window was found |
| 9 | Docs: island.md *Quick actions* and *Open terminal* (where it works), architecture if needed, pt-BR translations with new source marks; `NOTICE` line | `docs/guide/island.md`, `docs/pt-br/guide/island.md`, `NOTICE` | `docs.yml` and `check-brand.sh` pass (the reference is named only in `NOTICE`) |

## 4. Manual checks

- Plasma 6 Wayland (this machine): Konsole, kitty, VS Code window raised; minimized one comes back;
  two Konsole windows, the one titled after the folder wins; no window → *jump-failed*.
- Plasma X11 session: X11 path raises (log says which method).
- Xfce or i3 on X11 (a VM or `Xephyr` + the WM): the window and its workspace.
- GNOME Wayland (VM): button greyed, hint unchanged; an XWayland terminal still raised on click.
- kitty with `listen_on unix:/tmp/kitty` + `allow_remote_control socket-only`: the right tab; without
  it, nothing run and no error.
- No file left in `$XDG_RUNTIME_DIR` after a jump.

## 5. Out of scope

- GNOME Wayland native windows (no API; a GNOME Shell extension would be its own step).
- wlroots `foreign-toplevel` activation (Hyprland, Sway): possible later through `hyprctl` /
  `swaymsg`, a separate step.
- Terminals by name (Ghostty, Alacritty tabs): no remote API worth it yet.
