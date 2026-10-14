# Tasks

A task is a goal with the criteria that say it is done, kept on this computer, that Vults can
hand to an agent working in a clone of its own. The Nest's **Tasks** tab lists them
([The Nest](nest.md#tasks)).

## Keeping a task

`/task` in the chat keeps a task in one message, sent to no model: the goal on the first line, then
a line for each criterion that says it is done, starting with `-`:

```
/task Fix the flaky login test
- passes 20 runs in a row
- no sleep added
```

On one line, `done when:` starts the criteria, split by `;`. Its roost is the repository of the
conversation's folder. The Nest's **Tasks** tab lists them, the ones still to do first, with each
criterion to tick, who works on it (you, Zeca or an agent), where it stands (*To do*, *Doing*,
*Blocked*, *Done*, *Dropped*) and what it cost once Vults starts sessions for it. **New task** makes
one there too. With the GitHub connector on, a task in a GitHub repository gets **Open an issue**:
on your click, your `gh` opens an issue with its goal and criteria, and the task links to it. Any
task can link one you already have: `#12` (in its GitHub repository) or the issue's address, then
**Link**. Tasks stay in `vults.sqlite`, on this computer.

The issue field keeps its text, focus and selection when the list refreshes.
**Link** clears the draft only after saving succeeds; an error leaves it ready to
retry. Link, Open an issue and Unlink wait for one pending issue request per task,
including list updates, so repeated clicks cannot send competing requests.
An empty issue keeps Link disabled. The backend validates issue numbers and URLs.

**New task** keeps your goal, criteria and project until **Add** succeeds. While
adding, the form waits without accepting a duplicate request. A failure shows its
error and leaves the draft ready to retry. List updates keep the focused field
and text selection; the chosen project stays available even if its session leaves
the flock. An empty goal keeps Add disabled.

If the list cannot update, Tasks keeps the last loaded cards, drafts and open
sections, with a warning that the list may be out of date. **Try again** reloads
it without starting work or submitting the draft. A failed first load shows an
error rather than claiming there are no tasks. Action errors stay visible until
another action succeeds or you open New task.

Each card keeps its goal, status, next criterion and Start, Go on or Stop visible.
**More actions** holds status/owner changes, issue links, fresh starts, handoffs,
reviews and removal. It stays open when the list refreshes. Deleting a task and
removing its folder are separate actions, each requiring a second click.
**Cancel deletion** and **Cancel folder removal** dismiss only their own request.
**Esc** inside More actions cancels the current task's pending confirmations;
focus returns to its action, or More actions when that action is unavailable.
Updates keep focus on the confirmation or cancel
button, and holding Enter cannot act as both confirmation clicks. A running task
keeps Delete disabled until you stop it.

Starting or continuing sends one turn to the agent, which can call tools many
times within that turn. When it returns, review **What it said last**, then **Go on**
if work remains. A finished turn does not mark the task Done or check its criteria.
The card explains that distinction while the task remains open.

If a run or review fails, the task becomes **Blocked** and **What it said last**
opens with the partial response and failure cause. Closing that output stays your
choice on refresh; a new failure opens it again. The bird keeps its **Failed**
state, and all unanswered cards from the ended run return without an approval.

Claude and Codex tasks ask through their driver's permission channel. Their Vults hooks
observe the work without asking for the same permission again. Parallel steps and
subagent events keep unanswered cards visible. When the task CLI exits, its finished
bird stays in the flock for the normal ten-minute retention, rather than leaving
immediately. Terminal sessions keep their usual SessionEnd behavior.

## Delegating

`/delegate` makes a task the same way and starts it at once on the agent the router picks, and
the task's card says why. A project's own rule picks first (`/delegate default codex`, or
`claude`, typed in a conversation in that project; `/delegate default none` clears it); without
one, Claude Code. If that agent's plan is above 85% and the other's is known to be under it, the
task goes to the other. Once a project has 10 finished tasks of one kind (a fix, tests, docs, a
feature), the agent that finished more of them there replaces the rule for that kind, though the
85% still holds. A task counts for the agent that owns it when it is marked done or dropped.

## Starting an agent on a task

**Start with Claude Code** works on a task in a repository. Vults makes the task a clone of its
own, on a branch of its own (`vults/task-12`), under Vults' data folder: a `--shared` clone, which
reads your repository's objects but never writes them. It starts Claude Code there on the task's
goal and criteria, told to commit on that branch and never push. It runs in Claude Code's sandbox
(bubblewrap): it can't start without it, every command still asks, and it writes only in its clone
and the temp folders, never your repository, its branches, hooks or config. Commands you excluded
from the sandbox in your own Claude Code settings still run outside it, each asking first. It reads
your own settings, so it shows in the flock like any session and its permissions are island cards,
answered by you, your rules or your policies. Two tasks run side by side without touching each
other's files. When its turn ends, Vults fetches the task's branch, and only that branch, into your
repository, without running anything of the clone's; **What it said last** under the task keeps the
end of its reply, and **Go on** continues the same session. **Start fresh** hands the task to a new
session instead, when the old one has grown long: the old session writes a context pack (Objective,
Constraints, Done, In progress, Decisions, Files, Failures, Next), Vults puts in what it knows for
certain (the goal, which criteria are ticked, the files changed on the branch), and the new session
starts from that pack alone. The pack stays under the task as **Handoff notes**; if the old session
writes none, nothing is handed off and it keeps its session. **Stop** asks its turn to stop, waits,
then ends it; only a session Vults started can be stopped, and a running task can't be deleted.
What it cost joins the task. **Remove its folder** brings the branch back and removes the clone;
work never committed in it goes with it, so it asks twice.

### Codex

**Start with Codex** does the same with Codex, in its own sandbox (`workspace-write`: its clone and
the temp folders). Codex asks for approvals on its own channel: Vults turns each into the island card
its hook would send (the whole command, or an edit's files and line counts; a call it can't name
whole is a no), so your rules, policies and phone answer it the same way, and a stopped run takes
its waiting card away. On a task with a session, **Hand off to Codex** (or to Claude Code) is
**Start fresh** on the other agent: the session writes its pack, and the other one starts from it.

### Other agents

**Start with another agent…** lists the agents installed here that speak the [Agent Client
Protocol](https://agentclientprotocol.com): OpenCode, Qwen Code, Copilot CLI and Kimi Code.
Choose one, then click **Start with** beside the selector to run the task. Changing
the choice, including with keyboard arrows, sends nothing. It stays with that task
when the list refreshes. If the selected agent is no longer available, the choice
clears and Start waits for another selection; it never substitutes another agent.
The agent works in the task's clone the same way, logged in with its own account. Vults offers it no files
and no terminal of its own: it uses its own tools and asks before each one that writes, runs or
fetches, and each ask is an island card, answered only for that once, never with the agent's own
"always". A change to a file outside the clone, or in its `.git`, a command run elsewhere, a move or
a delete is a no without a card, and so is a file reached through a link that points out of the
clone. OpenCode runs every tool unasked by default, and Qwen Code and Kimi Code may by your own
settings: Vults starts each with a setting, for that process or session alone, that makes it ask
(never written to its config), and a write or command that still runs unasked stops the turn. Their own hooks ask
nothing in these sessions, so a call never shows two cards. Stop, Go on, Start fresh and Review
work as with Codex; the review is Codex's. Checked with OpenCode, and with Kimi Code 2.1.1 against
a stand-in model; Qwen Code and Copilot CLI follow the same protocol but were not tried with a real
account yet.

## Review

**Review with Codex** (or **with Claude Code**) has the other agent review a task's work, in its
clone, as a reviewer that wrote none of it, in a sandbox where it writes nothing: it runs the tests,
reads the diff against where the branch began, checks each criterion, and lists what is wrong.
Codex uses its own reviewer. When the
review asks for changes, its findings go to the task's session, which fixes them and commits, and
the next round reviews again, up to three rounds. A pass ends it; a review with no verdict, or a
third round still asking for changes, is yours to read. The last review stays under the task, and
you still mark the task done yourself.

## Cost and a daily budget

Vults counts what the agents report they cost: each turn of a session it started for a task or a
`/continue` (Claude Code's dollars; Codex reports tokens only, so its tasks show their tokens), and
each Claude Code session it watches, from its status line, counted from the first time Vults sees
it (what it cost before stays out of today). Nothing is guessed from a price list. Zeca's own chat,
summaries and sleep are not counted here, and the budget does not stop them. On a subscription the
dollars are what the tokens would cost at API prices, which your plan does not charge; Vults says so
when it can tell (a plan that reports its usage windows). Each task shows its cost and tokens, each
roost in the Nest's **Activity → By project** its cost today and in all, and the top of the **Tasks** tab today's.

The Nest's **Activity → Cost** sets a **daily budget** in US dollars. At 80% a notification says so; once it is spent
another one does, and Vults starts no new session until the next day: no Start, handoff, review,
`/delegate` or `/continue`. That is all it does. It never stops a session that is running, and never
blocks or slows an agent you started yourself. Costs stay in `vults.sqlite`, on this computer;
**Clear history** leaves them.
