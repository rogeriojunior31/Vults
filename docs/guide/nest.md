# The Nest

The Nest is Vults' own window, for when the island is too small: the whole flock, your tasks, the
chat, what happened and your connections. Open it from the tray icon (**Nest…**) or with the arrow
at the top of the island's chat. It is built when you open it and gone when you close it, and it
opens with Zeca off too, without its Chat. While it is open the island steps aside, pill and birds
and all: one surface on screen at a time. The Nest has no system title bar: its own header moves it
(a double click fills the screen) and its edges resize it. It comes back when the Nest closes or goes back to
**Quick** or **Mini**, and a card that waits brings it up anyway, since only the island answers
one.

The header counts the flock (*4 sessions · 1 needs you*) and holds the sound, the search (the
palette, as **Ctrl+K**), the size switch and the full-screen button. Under it, five tabs with their
icons, the number that needs you on **Flock** and the connectors' rows on **Connections**, and
**Settings** at the right, which shows Settings in this same window, under the same tabs; a tab
brings the Nest back. Settings stays a page of its own there: it is the only one that changes an
agent's config, a key, a policy or a phone, and it never shows an agent's text. The switch keeps
the tab you are on: **Room** is the window, the full-screen button the whole screen (**Esc** goes
back to Room), **Quick** closes the Nest and opens the island, **Mini** closes it and leaves the
pill. **Alt+1** to **Alt+5** pick a tab. At the bottom, **Zeca on · optional** / **Zeca off**
switches him (as **Settings → Flock & Zeca → Zeca** does), **Zeca's look** opens his page in
Settings, **Open terminal** brings up the terminal of the session in front, and **Back to the
island** brings the island up.

**Ctrl+K**, in the Nest or the open island (when it has the keyboard: opened with Ctrl+Alt+Space, or
in its chat), opens the palette: type to jump to a session, to start
`/task`, `/delegate`, `/continue` or `/remember` in the chat (with Zeca on), or to open a tab. It
never answers a card: a session that waits for you only leads to the island.

## Flock

The flock across the window, three times its size, each bird over a card with its session's
project, state and agent. A click on a bird or a card (or **↑ ↓**) puts that session in front.
Under them, **Session details**: **Needs you** keeps only the sessions waiting for you, and **Find a
session…** reads projects, repositories, branches, agents and what each is doing now.

The session in front has its card at the left: its state, what it is doing, and for one that waits,
what it asks (the command, with **Copy**, and *1 of 2* when more wait). At the right, its
**Changes** (each finished edit's diff, chosen in the list; **Focus diff** gives the diff the whole
width), **Activity** (its last steps), **Context** (worktree, repository, branch, folder, agent,
editor, state and cost) and, with Zeca on, **Zeca**: **Ask Zeca** opens the Chat with the question
ready. In Full, your plans' usage sits under it, amber from 70% and red from 90%.

A session that waits for you shows **Review on the island →**: only the island answers a card
([Approving from the island](approvals.md)), and the button brings it up with that session in
front. **Open terminal** shows where Vults can bring its terminal forward
([The island](island.md#getting-to-a-session)).

## Tasks

Your [tasks](chat.md#tasks), under the roost each belongs to (the ones with none last): each with
its goal, criteria, owner, status and cost, why the router chose its agent, its review rounds as
marks (up to three, the last in its verdict's color), the buttons that start, review or hand it
off, and today's spend against your daily budget (set in Activity → Cost). **New task** keeps one without the chat, which
is how you make one with Zeca off.

## Chat

The island's conversation with Zeca, wide, with replies in full Markdown (a link shows its address
and is never followed); a message, **New** or another engine in either one shows in both. Cards
show here too, but only the island answers them: **Answer on the island** brings its chat up.
The title bar holds the conversation's title, the chat's engine, folder and model, and two panel
buttons. In a wide window or Full, the left one shows **Recent**: your newest 30 conversations, the
one going on marked, and **+ New conversation** (both wait while Zeca replies); a click goes on with
one. The right one opens **Memory**, what Zeca remembers, beside the conversation: what waits for
your yes, the files and their history ([Zeca's memory](memory.md)). Before anything is said, the
composer waits in the middle with a few questions to start. Without Zeca there is no Chat tab.

## Activity

What this computer keeps, read only here:

- **Week**: the week's four figures (time with your agents, turns, lines changed, your answers), a
  bar a day (a click shows that day), **Save as image…** with or without the project names, and
  the year's grid, with the history's switches ([Activity](activity.md)).
- **By project**: a card for each repository a session works in: its folders and branches, its
  sessions, your pull requests and checks (from the GitHub card; a pull request or the default
  branch's checks open on GitHub), what Zeca remembers of the project and what its tasks cost
  ([Zeca's memory](memory.md#project-memory)).
- **History**: each finished turn of your agents, counts only.
- **Cost**: what the agents cost today and the **daily budget** you set here, your plans' usage
  windows and what the agents cost over the last two weeks, against that budget, and by roost ([Cost and a daily budget](chat.md#cost-and-a-daily-budget)).
- **Traces**: Zeca's own work, each chat turn with its model, tokens and cost, and under it each
  tool card with how long it waited for an answer; kept 30 days.
- **Audit**: who answered each card and how (you, your phone, an Always rule or a policy you
  accepted); kept 90 days and never changed.

## Connections

A chip for each connector you turned on, with how many rows it has; **+** opens **Settings →
Connectors**. Under it, the chosen connector: when it last synced (or why its last poll failed),
**↻ Refresh** to ask again (at most once a minute), and **All**, **Waiting for your review** and **Checks failed**, each with
its count. For GitHub its rows come by group (your pull requests, the reviews waiting for you, your
default branches), each with its checks or review; a row picked shows beside them with
**Open in GitHub ↗**. **Add a connection** lists each connector Vults has that is off, with what it
watches; **Turn on in Settings…** and **Manage connections** open **Settings → Connectors**, the
only place that turns one on ([Connectors](connectors.md)).
