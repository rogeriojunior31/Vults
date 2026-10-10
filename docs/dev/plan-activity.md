# Activity: the weekly recap and the contribution grid

Internal board (2026-10-09), checked against the code at `46b5254`. Deleted when its last step
merges. Items 5 and 6 of the list taken from the reference app (MIT, attributed in `NOTICE`; we
copy mechanics, not code): its `RecapStore` keeps each agent turn in a local file for 12 weeks and
shows the last week on Monday morning; its contribution grid is the user's GitHub calendar.

## 1. Where we stand

- No history: the core holds the sessions on the wire and forgets them when they leave. The digest
  (`crates/core/src/away.rs`, *While you were away*) is the closest thing: counts kept in memory
  while the user is away, told once, then dropped.
- The core is pure (no IO, no clock but the `Instant`s it is given): it decides, the app writes.
  So the core says *a turn ended* (an effect), and the app keeps the file.
- Local dates: the app gets today's date from the desktop (`vults_platform::linux::today`, glib);
  on Windows `today()` is `None` for now.
- The GitHub connector already asks `gh api graphql` (`crates/connectors/src/github.rs`): the
  contribution calendar is one more query, on the user's own `gh` login.
- Settings is a window with pages (`ui/src/surfaces/settings/main.ts`); the island shows the digest
  in its alerts slot with a ×.

## 2. Decisions (with the user, 2026-10-09)

- **D1. Two grids, in tabs:** *Agents* (local, from our history: how much the agents worked each
  day) and *GitHub* (the user's contribution calendar, only with the GitHub connector on).
- **D2. Where:** on Monday morning the island shows last week's recap once, as a card in its alerts
  slot; a new **Settings → Activity** page has the recap of any kept week and the grids.
- **D3. History on by default**, local only: a toggle to stop it and **Clear history** in Settings →
  Activity. Kept: 12 weeks of turns, and a year of per-day totals for the grid.
- **D4. Share as image later**, as the last step.
- **D5. What is kept, and what never is.** A turn keeps: its end (Unix seconds) and local day, its
  length, the agent, the project's folder name (not its path), and counts: steps, commands, files
  edited, lines added and removed, permissions allowed, denied and answered, questions, whether it
  failed. Never: prompts, replies, commands, file names, paths, notes. Nothing leaves the machine
  (no telemetry, `CLAUDE.md`).
- **D6. Deterministic, no model** (ADR 0012): every number is a count or a sum; the recap's words
  are built in `i18n`.

## 3. The model

**A turn** runs from a prompt (`UserPromptSubmit`; for agents without it, the first event of a
session at rest) to its end: `Stop`, `StopFailure`, `SessionEnd`, or 2 hours without an event (a
crashed agent: it ends at its last event, as the reference does). A session's draft lives on the
`Session` in core; the counters grow with its events (`ToolStarted` with `Activity::Run` is a
command; a finished edit's diff adds files and lines; an `Ended` outcome of its request adds a
decision). When it ends the core emits `Effect::Turn(Turn)`.

**Files** in the data folder (`~/.local/share/vults/`, `%LOCALAPPDATA%\Vults\` on Windows):

- `history.jsonl`: one turn per line, appended (`{"v":1,"end":1791600000,"day":"2026-10-09",
  "secs":840,"agent":"claude","project":"site","steps":31,"commands":6,"files":4,"added":120,
  "removed":30,"allowed":2,"denied":0,"answered":1,"failed":false}`). Older than 12 weeks: dropped
  at start-up (rewritten atomically). A line that does not parse is skipped, not fatal.
- `days.json`: `{ "v": 1, "days": { "2026-10-09": { "turns": 5, "secs": 4200 } } }`, a year kept,
  updated with each turn. The grid reads only this.

**The week** (ISO, Monday to Sunday, local days) is summed in core, pure (`crates/core/src/recap.rs`):
active time (overlapping turns merged, so parallel sessions don't count twice), turns, sessions,
lines added and removed, files, commands, decisions, failures, the top agent and project, the
busiest day, the longest turn. **Grid levels** follow GitHub's: 0 for no turn, then quartiles of
the year's non-zero days, by active time.

## 4. Steps

One step, one PR, in this order.

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| A1 | **Done.** **Turns, in core.** The draft on `Session`, its counters, the end rules (Stop, StopFailure, SessionEnd, 2 h silent via the existing tick), `Effect::Turn`. No IO | M | | Core tests: a turn's counts from a recorded Claude Code session; a failed turn; two turns in one session; a silent session ends at its last event; a hidden or muted project still counts |
| A2 | **Done.** **History, in the app.** `history.rs`: append, prune at start, `days.json`, atomic writes; local day and offset from the desktop (Windows: from the system clock, in `vults_platform`). Settings `history` (default on) and **Clear history**, schema version bump | M | A1 | Tests: append and read back, a bad line skipped, prune at 12 weeks, the day totals; history off writes nothing; clear removes both files |
| A3 | **The recap, in core.** `recap.rs`: the week from turns (merged time, tops, busiest day, longest), grid levels from days; the recap's sentences in `i18n` | S | A1 | Tests: overlapping turns counted once; ISO week edges (Sunday 23:59, Monday 00:00); an empty week is none; quartile levels |
| A4 | **Settings → Activity.** The week's recap with ‹ › over the kept weeks; the grid, *Agents* tab (53 × 7, tooltips with the day's turns and time); the history toggle and Clear history (with a confirm). Commands `activity_week(offset)`, `activity_days()` | M | A2, A3 | Visual tests of the page (a full week, an empty one, the grid); commands tested |
| A5 | **The Monday card.** On start, unlock and each day tick: Monday from 08:00 local, last week has turns, not shown this week (`recap_shown_week` in settings) → `Input::Recap(view)`; the island shows it in the alerts slot like the digest, with *Open Activity* and ×. A card waiting comes first; *Island* opens once, *Panel* and *Quiet* wait to be opened, *Paused* never | M | A3, A4 | Core tests (shown once a week, never over a card, presets); visual test of the card |
| A6 | **The GitHub tab.** `github.rs`: the contribution calendar (`viewer.contributionsCollection.contributionCalendar`, one query when the tab opens, kept an hour in memory), only with the GitHub connector on; off, the tab says how to turn it on. GitHub's own levels | S | A4 | Parse test from a recorded answer; tab visual test; no call with the connector off |
| A7 | **Share as image.** *Save as image…* on the recap: a PNG drawn in the page's canvas (1080 × 1350), *Hide project names* option, a save dialog. Nothing is uploaded | S | A4 | Visual test of the image; the file is written only where the user chose |

Docs in the same PR as each step: a new `docs/guide/activity.md` (+ pt-BR), the settings reference
(`history`, `recap_shown_week`), `docs/safety.md` (what is kept, D5), the README feature list,
CHANGELOG.

## 5. Risks

- **Privacy.** A history is new for us. D5 keeps it to counts and folder names; the safety page
  says so, and Clear history really removes the files. A test checks no prompt, command or path
  reaches `history.jsonl`.
- **Clock jumps** (suspend, timezone change): the end and day are taken when the turn ends, from
  the desktop's local time; a turn longer than its wall time is capped at it.
- **Windows local dates**: `today()` is `None` there. A2 adds the local date and UTC offset from
  the system in `vults_platform`; until it does, Windows uses UTC days and says so in the log.
- **File growth**: about 200 bytes a turn; 12 weeks of a heavy user is under 1 MB. `days.json`
  stays under 30 KB.
- **The grid's GitHub call** uses the user's `gh` and its rate budget (one query per tab open,
  cached): no polling.

## 6. Out of scope

- Any upload, sync or sharing service: the image is a local file.
- A recap written by a model.
- Per-session timelines (Activity on the island already lists a session's steps).
