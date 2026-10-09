// Generated from crates/core by `VULTS_REGEN=1 cargo test -p vults-core view_ts`. Do not edit.

export type AgentKind = "claude" | "codex" | "gemini" | "opencode" | "other";

export type Status = "idle" | "thinking" | "working" | "approval" | "question" | "finished" | "failed" | "ratelimited";

export type Attention = "quiet" | "info" | "done" | "failed" | "needs-you";

export type Activity = "read" | "search" | "edit" | "run" | "web" | "plan" | "subagent" | "think" | "work";

export type AlertLevel = "info" | "ok" | "warn" | "error";

export type Outfit = "auto" | "none" | "witch-hat" | "santa-hat" | "party-hat" | "bunny-ears" | "sunglasses" | "west-coast" | "fitted-cap" | "mountain-hat" | "headband" | "dreads" | "front-knot" | "durag" | "crown" | "bucket-hat" | "clock-chain" | "headphones" | "shutter-shades" | "chrome-chain" | "eye-patch";

export type ViewModel = { sessions: Array<SessionView>, approval: ApprovalView | null, alerts: Array<AlertView>,
/**
 * The most any session wants the user.
 */
attention?: Attention,
/**
 * The cards that left the line most recently and how, newest first: a surface tells the user
 * what became of the card it showed.
 */
ended?: Array<EndedView>,
/**
 * Each switched-on connector's card, once it has polled.
 */
boards?: Array<BoardView>,
/**
 * What Zeca wears today (`crate::looks`), if anything.
 */
look?: Outfit | null,
/**
 * The session the user put in front, if any.
 */
focus?: SessionRef | null,
/**
 * The session in front by [`State::front`]'s rule: the card's, the user's, the first at work.
 */
front?: SessionRef | null,
/**
 * Do not disturb: surfaces make no sound. Cards still show.
 */
dnd?: boolean,
/**
 * The screen is locked: surfaces rest (no animation, no timers).
 */
locked?: boolean,
/**
 * "While you were away", until dismissed.
 */
digest?: DigestView | null, };

export type SessionView = { id: string, agent: AgentKind,
/**
 * Another tool's name (`AgentKind::Other`), taken from its session id: `<name>/<id>`.
 */
agent_name?: string | null, project: string,
/**
 * The project folder, where the chat works when this session is in front.
 */
cwd: string | null, status: Status,
/**
 * What the status asks of the user.
 */
attention: Attention,
/**
 * The card first in line is this session's, and its status still waits on it: the island
 * shows that card, with this session in front.
 */
card: boolean,
/**
 * A card of this session waits, first in line or behind another.
 */
waiting?: boolean,
/**
 * *Open terminal* can bring it forward (`platform::jump`): its multiplexer's pane, or on KDE
 * its window. Elsewhere a quick action offers its folder instead (ADR 0011).
 */
raise?: boolean,
/**
 * Its project is muted: no sound for it here, no desktop notification.
 */
muted?: boolean,
/**
 * Its project is pinned: it comes first on the wire.
 */
pinned?: boolean,
/**
 * Working with no news for a while: its bird is flagged (`crate::silence`).
 */
silent?: Silence | null, activity: Activity | null, step: string | null,
/**
 * The latest steps, oldest first, for the island's step ticker.
 */
steps: Array<string>,
/**
 * What each of `steps` changed, when it is a finished edit; the full diff comes from
 * [`State::diff`] by its step number.
 */
diffs?: Array<DiffSummary | null>,
/**
 * How many steps the session has taken so far.
 */
step_count: number, subagents: number,
/**
 * The question, the last reply or the error that goes with the status.
 */
note: string | null,
/**
 * The editor whose terminal the session runs in ("Cursor", "VS Code").
 */
editor: string | null,
/**
 * Its bird's species, by the renderer's id (`crate::flock`, `ui/src/character/flock/species.ts`).
 * Zeca keeps his own.
 */
species: string, };

export type DiffSummary = { step: number, added: number, removed: number, files: number, };

export type ApprovalView = { request: string, agent: AgentKind,
/**
 * The session that asked, to put it in front.
 */
session: string, project: string, tool: string, target: string,
/**
 * The agent's own words for the action ("Run the test suite").
 */
description: string | null,
/**
 * The whole command, when `target` had to cut it.
 */
full: string | null,
/**
 * `target` is only the start of the call: no Always for it.
 */
cut: boolean,
/**
 * Lines an edit adds and removes; both 0 when it is not an edit.
 */
added: number, removed: number,
/**
 * A question card's questions, in order; empty for a permission.
 */
questions: Array<Question>,
/**
 * How many permissions and questions wait, this one included.
 */
queue: number,
/**
 * The reminders it has earned by waiting (the attention ladder): one more sound each.
 */
reminders?: number, };

export type EndedView = { request: string, agent: AgentKind, session: string, outcome: Outcome, };

export type SessionRef = { agent: AgentKind, id: string, };

export type Outcome = "allowed" | "denied" | "answered" | "released" | "terminal" | "expired" | "rule";

export type Silence = "quiet" | "loud";

export type DigestView = { seq: number,
/**
 * The whole sentence ("While you were away: 2 finished, 1 failed.").
 */
text: string, finished: number, failed: number, waiting: number, };

export type Question = { question: string,
/**
 * A short tag for it ("Color").
 */
header: string, options: Array<Choice>,
/**
 * Several choices may be picked.
 */
multi: boolean, };

export type Choice = { label: string, description: string | null, };

export type AlertView = { key: string,
/**
 * New each time the news arrives: the island sounds an alert once per `seq`.
 */
seq: number, connector: string, level: AlertLevel, title: string, detail: string,
/**
 * The alert opens something when clicked.
 */
link: boolean, };

export type BoardView = { connector: string, rows: Array<RowView>, };

export type RowView = { item: string, group: Group, name: string, title: string, checks: Checks | null, review: Verdict | null,
/**
 * The row opens something when clicked.
 */
link: boolean, };

export type Group = "yours" | "to-review" | "branches";

export type Checks = "passing" | "failing" | "running";

export type Verdict = "approved" | "changes";

export type Diff = { files: Array<FileDiff>,
/**
 * The patch was longer than what reached us: the card says it stops short.
 */
cut: boolean, };

export type FileDiff = { path: string, added: number, removed: number, hunks: Array<Hunk>, };

export type Hunk = {
/**
 * The hunk's first line in the old and the new file, when the agent said.
 */
old_start: number | null, new_start: number | null,
/**
 * Each line behind its mark: `+`, `-` or a space.
 */
lines: Array<string>, };
