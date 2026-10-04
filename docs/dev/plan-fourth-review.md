# Plan: the fourth reference review

Working plan for the agents that carry out what the fourth review of the reference app found
(upstream `c767db9` → `59f63df`, its 0.1.3 to 0.1.6), plus the KDE panel mode. It is a shared
board: whoever picks a step marks it, refines it, and checks it off. Delete the file once every
step is done or dropped, and move what is left into `road-to-1.0.md`.

Paths written `REF/...` follow `road-to-1.0.md`: `REF/mac/` is the reference's macOS app
(Swift). Read the reference with `git -C <ref clone> show <commit>:<path>`; never check out,
stash or reset in that clone.

## How to work a step

1. **Study first.** Read the reference commits and tests listed for the step and write what you
   learned under *Notes* (the rule, the limits, the edge cases its tests pin down). Do not
   reinvent what they already solved; do not copy what does not fit here (Swift UI, macOS APIs,
   the brand).
2. Mark the step `doing (<branch>)` in the board, in its own worktree off `main`.
3. Implement, with tests and docs in the same PR (CLAUDE.md, *Before every commit*).
4. Gate: `flock-tester` (all checks + the step's *Done when*) → `flock-reviewer` → at most two
   fix rounds, then ask the user → PR → CI green → merge.
5. Mark it `done (#PR)` and add anything learned to *Notes*.

One step, one PR. Steps in the same row group can run in parallel: they touch different files.

## Board

| # | Step | Size | Depends on | Status |
|---|---|---|---|---|
| 0a | PR checks keyed by the head commit | S | | done (#46) |
| 0b | Live diff on the island | L | | done (#47); still to try in the real app with each agent |
| 1 | Finished card: first paragraph only | S | | done (#49) |
| 2 | GitHub alerts: retire stale ones, partial errors | S | 0a | todo |
| 3 | GitHub pacing: faster while running, fresh on open | S | | done (#50) |
| 4 | GitHub card: open PRs, reviews, branch checks | M | 3 | todo |
| 5 | Release: tag must match the version | S | | todo |
| 6 | Seasonal looks for Zeca | M | | todo |
| 7 | Panel mode: Zeca alive in the tray | M | | todo (option B chosen) |
| 8 | Optional: contribution grid, hello bounce | S/M | 4 | later |

---

## 1. Finished card: first paragraph only

**Why.** `Stopped.message` keeps the whole `last_assistant_message`
(`crates/agents/src/claude.rs:104`, `codex.rs:68`); the card clamps it to two lines in CSS
(`ui/src/island/island.css`), so `## Summary **Done.** … --- | a | b |` shows markdown noise.

**Study.** `d5d0e49`: `toOneLine` in `REF/mac/Sources/App/DiffEngine.swift` and its nine cases in
`tests/DiffEngineTests.swift`. A paragraph ends at a blank line, a `---`/`***`/`___` rule or a
`|` table row; it strips `**`, `__`, backticks, leading `#` and bullets, collapses whitespace,
caps at 200 chars.

**Do.** A pure `summary_line(&str) -> Option<String>` next to `filled()` in
`crates/agents/src/lib.rs`, used by Claude, Codex and Gemini's stop. `core` stays agent-neutral.

**Done when.** The nine reference cases pass as Rust tests, plus a `claude.rs` fixture test; the
lab's finished state shows one clean line.

## 2. GitHub alerts: retire stale ones, partial errors

**Why.** After 0a, three gaps remain:
- An older `ci-failed` alert stays next to a newer `ci-passed` (or a failure on a newer commit):
  core only replaces an alert with the same key (`crates/core/src/lib.rs`, the alert insert).
- A review requested again keeps the key `review:…:requested`; the UI remembers every key it
  has shown (`alertsSeen` in `ui/src/island/render.ts`), so the second request is silent.
- Unverified: `gh api graphql` may exit non-zero when the answer has `errors` beside `data`
  (one SAML org, one deleted repo), failing the whole poll into backoff.

**Study.** `28d045c`: `REF/mac/Sources/App/GitHubPulse.swift` (transitions by sha) and
`tests/GitHubPulseTests.swift` (the five sha scenarios); `daa4bec`: partial errors parsed when
`data` is present (`GithubPoller.swift`), alert priority failure > review > pass.

**Do.** An alert carries its item (`pr:X#n`, `branch:X`); a newer `ci-*` alert of the same item
retires the older ones. Forget `alertsSeen` keys no longer in the view. Check the `gh` exit
code on a partial error first; parse stdout when `data` is not null.

**Done when.** Core tests: fail → pass on the same PR leaves one alert; a re-requested review
alerts again. A recorded partial-error answer still yields a snapshot.

## 3. GitHub pacing: faster while running, fresh on open

**Why.** Polls are a fixed 120 s (`crates/connectors/src/github.rs`, `interval`); opening the
island shows data up to two minutes old.

**Study.** `daa4bec`: 60 s while any check is `PENDING`/`EXPECTED`, else 300 s, the first poll
10 s after launch, the first answer silent. `28d045c`: `refreshIfStale(maxAge: 60)` on focus,
expand and detail open (`GithubPoller.swift`).

**Do.** `Connector::interval` gets a hint from the last snapshot (pending or not). A
`Runtime::refresh_if_stale(id, max_age)` wakes the task through a `Notify` raced in the
existing `select!` (`crates/connectors/src/runtime.rs`), using `Status.last_ok`. The island's
expand calls it through the app. Errors keep today's backoff.

**Done when.** Runtime tests with a fake clock: pending → 60 s, idle → 300 s, refresh only when
older than 60 s, never during backoff. `docs/guide/connectors.md` states the cadence.

## 4. GitHub card: open PRs, reviews, branch checks

**Why.** We fetch everything the reference shows but only surface transient alerts (at most
five in core, three on screen).

**Study.** `daa4bec`, `28d045c`: the card and detail views in `IslandViewContent.swift`, what a
row shows, click to open on github.com; `docs/INTEGRATIONS.md` in that range.

**Do.** The runtime exposes the latest snapshot (`Runtime::snapshot(id)`); a command in
`app/src/connectors.rs`; a panel in the island (rows: PR with its check state, reviews waiting
for you, default-branch checks of recent repos). Opening the panel calls step 3's refresh.
Mention in the privacy notes that the snapshot is kept on disk
(`data_dir/connectors/github.json`), unlike the reference.

**Done when.** Lab state and a visual test; a row opens its URL; docs updated.

## 5. Release: tag must match the version

**Why.** `Cargo.toml`, `package.json` and `app/tauri.conf.json` say `0.0.0`; tag `v1.0.0` would
ship packages labelled 0.0.0. The reference added the same guard in `59f63df`
(`scripts/release.sh`).

**Do.** A first job step in `.github/workflows/release.yml` fails unless the tag equals all three
versions. Ties into `road-to-1.0.md` §3.

**Done when.** The step fails on a mismatched test tag in a fork or with `act`, and passes on a
matching one.

## 6. Seasonal looks for Zeca

**Why.** A cheap touch of character: a hat at Halloween, Christmas, New Year, Easter, summer.

**Study.** `b5d2242`: `REF/mac/Sources/App/*Wardrobe.swift` (stable ids, unknown id → auto, the
season table and its overlap rule, Easter by Meeus/Jones/Butcher, cached per day), its 57 tests,
the right-click wardrobe with live preview. Skip the 3D brims and spring physics: they do not
fit 8-bit sprites.

**Do.** Season calendar and Easter in a pure `crates/core` module with tests (the date comes in
as an `Input`, like the flock's season seed). Three to five overlays in
`design/mascots/zeca/zeca.py`, layered in the renderer, only on Zeca (the flock keeps its
species looks). Setting: *Auto* (default), *None*, or one look. Southern summer note: Brazil's
summer is December–March; pick the window with the user.

**Done when.** Calendar tests (boundaries, Easter for several years); every look in the lab and
the visual tests; Settings and docs.

## 7. Panel mode: Zeca alive in the tray

**Why.** The user's idea: besides the island at the top (the default), the app can live in the
KDE panel. A setting picks *Island* or *Panel*.

**Decided (2026-10-04): option B.** The tray icon shows Zeca's state and a click opens the
island as a popup by the tray. Dropped: a Plasma widget (adds QML to the stack) and the island
docked over the panel (panel position and stacking are KWin's).

**Study.** The tray today: `tray()` in `app/src/lib.rs` (Tauri's `TrayIconBuilder`, a
StatusNotifierItem on Plasma). The reference has no panel mode; only its menu-bar item, worth a
look for what it shows. Check on Plasma 6 (nested KWin harness): how often an SNI icon can change
without flicker or CPU cost, how a left click is delivered (Tauri's `on_tray_icon_event` vs.
Plasma's *Activate*), and whether the icon is themed or pixel-perfect at 22 and 24 px.

**Do.**
- Icon frames rendered from Zeca's sprites (`design/mascots/zeca/zeca.py`) per state: idle,
  working, waiting for a click (approval or question, the loudest), done, error. A few frames at
  a low rate, none under reduced motion; the frames ship in the app, not drawn at run time.
- The core's view already says the state; the app maps it to a frame, no new `Effect` unless
  needed.
- *Panel* mode: the island at the top is not drawn; a click on the tray opens it as a layer
  surface anchored to the corner the panel is on (asked once, or read from Plasma if it can be),
  with the same input-region rules. An approval still needs a human click (rule 2): the tray
  only draws attention, it never answers.
- Setting *Island* / *Panel* in Settings, the default *Island*; docs in `docs/guide/`.

**Done when.** On Plasma 6: every state shows its icon, an approval in panel mode makes the icon
call for attention and a click opens the island on the approval card; switching the setting
needs no restart; visual tests for the icon frames.

## 8. Optional, later

- **Contribution grid** (`86fbb79`): one more `gh` query every 30 minutes, levels 0–4, the last
  7 days in the GitHub card, 23 weeks in its detail. After step 4.
- **Hello bounce** (`f789a2a`): a squash-and-bounce landing in Zeca's hello clip; our sounds
  stay synthesized.
- **Not now:** the mascot dragged onto the desktop (`2b02b91`). A second, moving layer surface
  goes against the island's one fixed surface; revisit after 1.0, with step 7's findings.

## Notes

(Add what each step learned from the reference and from building it.)

### 1. Finished card

- **Reference (`d5d0e49`).** `toOneLine(text, maxChars: 200)` splits on `\n`; a line trimmed of
  spaces is a separator when empty, made of 3+ of one of `-`, `*`, `_`, or starting with `|`. Each
  paragraph in turn: drop `**`, `__` and backticks, then per line drop leading `#`s, trim, drop one
  `- `, `* `, `• ` or `^\d+\.\s+`; drop empty lines, join with spaces, collapse whitespace. The
  first non-empty paragraph wins (an empty one falls through), cut with `prefix(200)`, no ellipsis;
  nothing left gives `""`. Its view also went to one line with a tail ellipsis. The commit says
  nine new cases; there are eight, plus five older ones (multi-line, bold, heading, empty, cap).
- **Ported** as `summary_line` in `crates/agents/src/lib.rs`, for Claude (and other tools, which
  read Claude's format), Codex and Gemini. All thirteen cases are Rust tests.
- **Differs on purpose.** `None` instead of `""`. A cut ends in `…` (the card's CSS clamp only
  shows one when the text overflows, so a cut that fits would look whole). Cut by `char`, not
  grapheme: no new dependency, and a split ZWJ emoji at 200 chars is harmless. `str::lines` drops
  a CRLF's `\r` (Swift's `.whitespaces` keeps it, so a `\r` line is not blank there). A paragraph
  made only of headings (`## Summary` above the text) is skipped and only used when nothing else
  has text, otherwise the card would read *Summary*. A heading is 1 to 6 `#` then a space or
  the line's end (the reference took any leading `#`), so `#48 merged`, `#!/bin/sh` and
  `#[derive]` stay text and keep their `#`. It is checked after trimming, so an indented heading
  counts too.
- **UI.** The card keeps its two-line clamp: one line of up to 200 chars still wraps on the
  island, and the second line shows more of it. No visual change; the lab's finished note was
  already one plain sentence.

### 3. GitHub pacing

**From the reference** (`daa4bec`, `28d045c`: `GithubPoller.swift`, `GitHubPulse.swift`):
- `hasPending` is true when any PR's or default branch's rollup is pending (`PENDING` and
  `EXPECTED` both map to pending); the next poll is 60 s then, else 300 s. The delay is picked
  after each answer; an HTTP or parse error schedules the slow 300 s, with no backoff.
- `isStale(fetchedAt, now, maxAge)` is a pure predicate: no fetch yet is stale, and stale means
  strictly older than `maxAge` (60 s). Its three tests: nil, same instant, 61 s ago.
- `refreshIfStale(maxAge: 60)` does nothing while a request is in flight or the data is fresh;
  otherwise it cancels the scheduled poll and polls now. Called when GitHub takes focus, when
  the island expands with GitHub in focus, and when its detail view opens.
- A token generation counter drops an in-flight answer after the token changed. We have no
  token (`gh` owns the login), so nothing to port.
- The first poll waits 10 s after launch. Not ported: ours runs in its own task and `gh` is a
  child process; nothing competes with start-up.

**Built:**
- `Connector::interval(&self, last: &Snapshot)`: GitHub answers 60 s when any item's `ci` is
  `PENDING` or `EXPECTED`, else 300 s. Errors keep today's backoff, from a fixed 120 s base
  whatever the interval (240 s, 480 s, … up to 15 min); rate limits and the 10 min "user must
  act" wait stay too. They replace the interval, so they always win over the fast pace.
- `Runtime::refresh_if_stale(id, max_age)` checks, without blocking: switched on, not rate
  limited, last *attempt* (good or failed) older than `max_age`. It keeps tokio's `Instant`
  rather than `Status.last_ok` (wall seconds): the paused-clock tests drive it, and a wall clock
  change can't fake freshness. Then `Notify::notify_waiters`, raced in the task's `select!`. It
  stores no permit: a request that lands mid-poll or while off is dropped, so it never becomes a
  second poll (`notify_one` would; two tests fail with it).
- **Differs from this step's plan on purpose:** an open does cut an error's wait short (auth,
  missing `gh`, network), at most once a minute since the last try, so the island recovers soon
  after `gh auth login` or a network blip instead of up to 10–15 min later. Only a rate limit is
  always waited out. A failed retry doubles the backoff like any error.
- The island calls it for every connector (`connectors_refresh`) on each open, whatever opened
  it: any open shows the alerts. Step 4's panel can call the same command.
- A poll that ends after the connector was switched off sends no events but still saves its
  snapshot, so switching back on does not replay it.
