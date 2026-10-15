# The Nest

The Nest is Vults' own window, for when the island is too small: the whole flock, your tasks, the
chat, what happened and your connections. Open it from the tray icon (**Nest…**) or with the arrow
at the top of the island's chat. It is built when you open it and gone when you close it, and it
opens with Zeca off too, without its Chat. While it is open the island steps aside, pill and birds
and all: one surface on screen at a time. The Nest has no system title bar: its own header moves it
(a double click fills the screen) and its edges and corners resize it. It first opens large (up to
1180 × 820, within the screen), then at the size you last left it. It comes back when the Nest closes or goes back to
**Quick** or **Mini**. A card that waits is answered where you are: in the Nest while you are in it
(its window has the focus), and on the island, which comes up for it, while you are elsewhere.

Tasks, Activity and Connections share the same black page background, raised cards,
heading scale and neutral controls. Connections uses Activity's segmented navigation;
Tasks and Connections use the same button and field treatment. Status colors keep
their existing meaning, including readable finished-task controls.

Read views use the same loading, empty, filtered and error panels, with a short explanation
and the available next action. A first read does not show empty records or zero summaries
while loading. If refreshing Activity fails, its last loaded records, projects or recap stay
visible with an out-of-date warning and **Try again**; a first-read failure stops the loading
message and offers retry. Raw read errors stay under **Technical details**. Retrying only reads
the view; it never runs a task, answers a permission or changes history settings.

Minimizing the Nest returns the island and its approval shortcuts. Opening the Nest
again restores the window; minimizing does not change its saved Room size.

Reopening from the tray keeps your last tab and Activity subsection until Vults exits.
Opening a specific view takes priority. If Zeca is off, a saved Chat tab opens Flock.

Closing with the desktop's close command (such as **Alt+F4**) also waits for the
Chat draft, navigation and record searches to save. If saving or closing fails,
the window stays open with an alert; try closing again. This also saves the Nest's
state when Settings is visible in the shared window. Closing does not answer a
waiting permission or stop a run.

The size switch and **Back to the island** wait for the Chat's initial draft to
restore and for pending saves. Repeated clicks while changing views share one
attempt. If saving or changing the view fails, the Nest keeps the draft and shows
an alert; try the same control again.

In a short Chat window, the composer uses less space and the empty conversation's
suggestions scroll if needed. This applies to wide windows too, keeping the title
bar's controls and the footer reachable.

The header counts the flock (*4 sessions · 1 needs you*) and holds the song your player is playing
(with Now playing on; previous, play/pause and next beside it), the moon while do not disturb lasts
(a click ends it), the sound, the search (the palette, as **Ctrl+K**), the size switch and the
full-screen button. Under it, five tabs with their
icons, the number that needs you on **Flock** and the connectors' rows on **Connections**, and
**Settings** at the right, which shows Settings in this same window, under the same tabs; a tab
brings the Nest back. Over every tab but **Flock**, a tag for each session that needs you (or
failed); a click opens **Flock** with it in front (a card that waits comes first there), where you
answer it. Settings stays a page of its own there: it is the only one that changes an
agent's config, a key, a policy or a phone, and it never shows an agent's text. The switch keeps
the tab you are on: **Room** is the window, the full-screen button the whole screen (**Esc** goes
back to Room), **Quick** closes the Nest and opens the island, **Mini** closes it and leaves the
pill. **Alt+1** to **Alt+5** pick a tab. At the bottom, **Zeca on · optional** / **Zeca off**
switches him (as **Settings → Flock & Zeca → Zeca** does), the pencil (**Zeca's look**) opens his page in
Settings, **Open terminal** brings up the terminal of the session in front, and **Back to the
island** brings the island up.

Settings switches and choice controls show saving, saved or a local failure message.
While a row is saving, repeated changes wait even if Settings refreshes. A failed save
restores the previous choice; it never reports success for an unsuccessful write.

**Ctrl+K**, in the Nest or the open island (when it has the keyboard: opened with Ctrl+Alt+Space, or
in its chat), opens the palette: type to jump to a session, to start
`/task`, `/delegate`, `/continue` or `/remember` in the chat (with Zeca on), or to open a tab. It
never answers a card: a session that waits for you only leads to its card.

Settings uses one page heading and groups related controls consistently. General's
keyboard reference and Agents' capability diagnostics open on demand. Chat & voice
keeps extended setup, access and API explanations beside their controls under **More**.
Open explanations and the scroll position survive refreshes; a focused input keeps
its unsaved text and selection during an unrelated update.

## Flock

The flock across the window, three times its size, each bird over a card with its session's
project, state and agent, and by the heading how many are out on a web sortie and how many sit on
the wire. A click on a bird or a card (or **↑ ↓**) puts that session in front. A right click (or
the menu key, or **Shift+F10**) opens its quick actions, as on the island: **Open terminal**, **Open folder**,
**Activity**, **View the last diff**,
**Keep in front** (or **Let the flock choose**), and mute, pin or hide its project. While a card
waits, only that session's **Activity** and diff can show.
Under them, **Session details**: **Needs you** keeps only the sessions waiting for you, and **Find a
session…** reads projects, repositories, branches, agents and what each is doing now.

The selected bird and its card share a light selection mark. Project names have
up to two lines; hovering over a card shows its full name, branch, agent and state.
**Needs you** includes the total waiting count across the flock, even while searching.
The detail card separates the project heading from its agent and state, and detail
tabs use a brief entrance only when you switch them; live updates do not replay it.
Reduced motion and manual pause skip that entrance.

The search, **Needs you** filter and detail tab stay where you left them when you
close and reopen the Nest or return to the island, until Vults exits. **Clear
filters** in the search toolbar clears the search and Needs you, keeping the detail tab. If Zeca is off,
his saved detail tab opens **Changes**. Filtering the session list leaves the
current permission card visible; restoring these controls never answers it.

The session in front has its card at the left: its state, what it is doing, and for one that waits,
what it asks (the command, with **Copy**, and *1 of 2* when more wait). A working one shows its
steps so far, its subagents and its cost in the head, the step just done and the one under way
(a finished edit's **+N −M** opens it in **Changes**), and, when it has gone quiet, **Snooze**,
**Keep going** and **Dismiss**, as on the island. A finished or failed one shows its last reply or
its error. Under the card, what its kept edits changed (from its last steps). At the right, its
**Changes** (each finished edit's diff, chosen in the list; **Focus diff** gives the diff the whole
width), **Activity** (its last steps, each at the time it started), **Context** (worktree, repository, branch, folder, agent,
editor, state and cost) and, with Zeca on, **Zeca**: **Ask Zeca** opens the Chat with the question
ready. In Full, your plans' usage sits under it, amber from 70% and red from 90%.

If a permission or question arrives while **Focus diff** is expanded, the card
comes back. Expansion stays unavailable while the request waits. Permission
shortcuts only answer an active Flock card that is visible.

The chosen change and expanded view survive reopening the Nest for the same
session while that step is still kept. A different session starts with its own
latest change; a step that expires returns to the latest kept change and card.
If the diff body is unavailable, the card returns and expansion is unavailable.

A session that waits for you is answered on its card, as on the island
([Approving from the island](approvals.md)): **Deny**, **Allow** and **Always allow** for a
permission (**Ctrl+Alt+Y** and **Ctrl+Alt+N** too, while the Nest has the focus), the choices for a
question. One with no card here (it asks in its terminal) says so.
**Open terminal** shows where Vults can bring its terminal forward
([The island](island.md#getting-to-a-session)).

## Tasks

Your [tasks](tasks.md), under the roost each belongs to (the ones with none last): each with
its goal, criteria, owner, status and cost, why the router chose its agent, its review rounds as
marks (up to three, the last in its verdict's color), the buttons that start, review or hand it
off, and today's spend against your daily budget (set in Activity → Cost). **New task** keeps one without the chat, which
is how you make one with Zeca off.

## Chat

The island's conversation with Zeca, wide, with replies in full Markdown (a link shows its address
and is never followed); a message, **New** or another engine in either one shows in both. Cards
are answered here too, with **Deny**, **Allow** and **Always in this chat**, as on the island.
The title bar holds the conversation's title, the engine pill (as on the island: the engine in its
color and its model; its menu holds the engines, the models and the effort), the folder, and two
panel buttons. The composer has a paperclip: the files you pick go to this chat, as a drop on the
island does. The left panel button opens **Recent**: a sidebar in a wide window or Full,
or a separate view with **Back to Chat** in a narrow Room. Conversations are grouped by date,
with their project, executor and time below the title; the current conversation is marked.
**Search conversations** searches all loaded titles, folders and executors locally, including
conversations past the initial 30. **Show more** reveals the rest. The search stays on this
computer when the Nest closes and reopens. **+ New conversation** and switching to another
conversation wait while Zeca replies. A failed update keeps the last list and offers **Try again**.
The right button opens **Memory**, what Zeca remembers, beside the conversation: what waits for
your yes, the files and their history ([Zeca's memory](memory.md)). Before anything is said, the
composer waits in the middle with a few questions to start. Without Zeca there is no Chat tab.

## Activity

History and Audit keep their searches independently until Vults exits. Returning
to Mini or Quick waits for pending search saves; reopening the Nest restores the
filters. **Clear filters** clears only that subsection's search. A delayed
restoration leaves any newer typing alone.

What this computer keeps, read only here:

- **Week**: the week's four figures (time with your agents, turns, lines changed, your answers), a
  bar a day (a click shows that day), **Save as image…** with or without the project names, and
  the year's grid, with the history's switches ([Activity](activity.md)).
- **By project**: this week's time per project in bars (a repository's worktrees add up under it,
  with how many it has), a search
  by project, branch or agent, and a card for each repository a session works in: its folders and
  branches, its sessions (a click puts one in front on **Flock**), your pull requests and checks (from the GitHub card; a pull request or the default
  branch's checks open on GitHub), what Zeca remembers of the project and what its tasks cost
  ([Zeca's memory](memory.md#project-memory)). **Preferences ↗** opens **Settings → Projects**.
- **History**: each finished turn of your agents, counts only. Search the loaded
  records by project, agent or status; **Clear filters** restores the list.
- **Cost**: what the agents cost today and the **daily budget** you set here, your plans' usage
  windows and what the agents cost over the last two weeks, against that budget, and by roost ([Cost and a daily budget](tasks.md#cost-and-a-daily-budget)).
- **Traces**: Zeca's own work, each chat turn with its model, tokens and cost, and under it each
  tool card with how long it waited for an answer; kept 30 days.
- **Audit**: search the loaded records by project, agent, answer, tool or command.
  **Clear filters** restores the list. Who answered each card and how (you, your phone, an Always rule or a policy you
  accepted); kept 90 days and never changed.

Activity > **By project** keeps its search until Vults exits, including when you
return to the island or close and reopen the Nest. **Clear filters** clears only
this search; History and Audit keep theirs. A project list that becomes empty
leaves an active search editable. Typing filters the loaded projects without
reloading them or the weekly activity.

## Connections

Closing and reopening the Nest keeps the repository filter, subsection and selected
item until Vults exits. In a narrow window, the selected detail opens again; Back
returns to the filtered list. If the item disappears, the list opens instead.
An unfinished merge confirmation is discarded when the Nest closes.

The chosen connector has one header: its last sync (or why it failed), **↻ Refresh** to ask again
(at most once a minute), and **Manage connections** for **Settings → Connectors**. A selector
appears only when several connectors have cards. An enabled connector awaiting its first card
shows its waiting/error state with Refresh and Settings; an unreadable status offers **Try again**
without treating the connector as off. A spent daily request budget says why checks are waiting.

For GitHub, four tabs keep their counts: **Inbox** (what waits on you: reviews asked of you, your
pull requests failing, sent back or ready to merge, and a default branch failing; how many are
new), **Pull requests**, **Actions** (your default branches' checks; red with how many fail) and
**Releases** (each repository's latest). Search filters the loaded tab by title, reference,
repository, branch or author, alongside the **All repos** filter. It makes no network request.
The search stays on this computer across reopening; **Clear filters** restores the list.

The list and selected detail scroll independently; ordinary polling keeps their reading
positions, search focus and expanded details. At narrow widths, **Back** or **Esc** returns from
the detail to its selected row. With focus in Search, **Esc** clears its query; an unfinished
merge confirmation is dismissed first. Selecting, searching or expanding never performs an action.

A selected row puts its title, branch, author and size (`+120 −40 · 6 files`) before its actions,
including **Open in GitHub ↗**, as on the island ([The GitHub card](connectors.md#the-github-card)).
Actions and the explicit merge confirmation come before the check list. Successful checks start
collapsed; running or newly failing checks open, failed checks first, with a count and a progress
bar while running. Manual disclosure choices survive unchanged polling. Failing checks take
priority over an approval badge. **Technical details**, when present, keeps update information
and a `gh` command with **Copy** and retry feedback (Vults never runs it).

A failed refresh keeps the current rows and allows another attempt. **✓ Seen** marks a quieter
title without fading its status or actions. **Add a connection** lists only supported connectors
known to be off; **Turn on in Settings…** and **Manage connections** open the only place that
turns one on ([Connectors](connectors.md)).
