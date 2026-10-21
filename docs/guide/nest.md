# The Nest

The Nest is Vults' own window, for when the island is too small: the whole flock, the
chat, what happened and your connections. Open it from the tray icon (**Nest…**) or with the arrow
at the top of the island's chat. It is built when you open it and gone when you close it, and it
opens with Zeca off too, without its Chat. While it is open the island steps aside, pill and birds
and all: one surface on screen at a time. The Nest has no system title bar: its own header moves it
(a double click fills the screen) and its edges and corners resize it. It first opens large (up to
1180 × 820, within the screen), then at the size you last left it. It comes back when the Nest closes or goes back to
**Quick** or **Mini**. A card that waits is answered where you are: in the Nest while you are in it
(its window has the focus), and on the island, which comes up for it, while you are elsewhere.

Activity, Connections and Settings share neutral controls, page headings, cards and view states.
**Settings → General → Theme** chooses **Black** (the default) or **Light** across every window.
Activity and Connections use underlined view tabs; status colors keep their meaning. The Flock stage,
diffs and Compact controls follow the same theme, including live changes to the birds'
agent colors and wire.

At widths of 900 px or more, drag the separator of Settings, Connections, Dictation, Recent or Memory
to resize that side panel. Double-click resets its width; the fold button keeps a narrow rail.
The separator also accepts arrow keys, Home/End and Enter. Width and folded state are saved
locally, without changing selection or drafts. Narrow windows keep their existing navigation.

Read views use the same loading, empty, filtered and error panels, with a short explanation
and the available next action. A first read does not show empty records or zero summaries
while loading. If refreshing Activity fails, its last loaded records, projects or recap stay
visible with an out-of-date warning and **Try again**; a first-read failure stops the loading
message and offers retry. Raw read errors stay under **Technical details**. Retrying only reads
the view; it never starts a session, answers a permission or changes history settings.

Minimizing the Nest returns the island and its approval shortcuts. Opening the Nest
again restores the window; minimizing does not change its saved Room size.

Reopening from the tray keeps your last tab and Activity subsection until Vults exits.
Opening a specific view takes priority. If Zeca is off, a saved Chat tab opens Flock. Quick's **Now**
opens a connection directly in its selected detail, clearing an old filter that would hide it;
a memory proposal opens Chat's Memory panel. Neither route performs an action. Returning
from Activity or Connections to Quick opens Now.

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

When there is room, the header counts the flock (*4 sessions · 1 needs you*) and holds the song your player is playing
(with Now playing on; previous, play/pause and next beside it), the moon while do not disturb lasts
(a click ends it), the sound, the search (the palette, as **Ctrl+K**), the size switch and the
full-screen button. The same header holds five tabs with their
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
`/continue` or `/remember` in the chat (with Zeca on), or to open a tab. It
never answers a card: a session that waits for you only leads to its card.

Settings uses one page heading and groups related controls consistently. General's
keyboard reference and Agents' capability diagnostics open on demand. Chat & voice
keeps extended setup, access and API explanations beside their controls under **More**.
Open explanations and the scroll position survive refreshes; a focused input keeps
its unsaved text and selection during an unrelated update.

## Agent profiles

Open **Flock > Agent profiles** to inspect the built-in Developer, Reviewer and Researcher profiles.
Choose a card to read its instructions and explicit revision. **Create a copy** opens a local draft
with an editable name and instructions. **Save profile** creates a custom profile; **Edit profile**
saves a new revision of your copy. Built-in profiles remain unchanged. The copy initially inherits the preset's appearance.
In **Appearance**, choose a **Species** and an **Accessory**; the preview uses the same sprites as
the flock. Accessories are fixed choices, including **None**, without seasonal changes.
The card shows the saved appearance after saving. In **Pinned skills**, choose from the built-in
skills and review their literal content and revision. Each profile save keeps the selected content
with that immutable revision, and a future bound conversation can retain the same snapshot.
Pinned skills do not enable tools. Applying a profile to Chat remains gated until recipient,
executor, source and Zeca-off behavior are ready.

Custom profiles live in the local database, independently of conversation history. Each save
retains earlier revisions. A stale editor is rejected instead of overwriting newer changes;
cancel and reopen to read the latest revision. Failed saves preserve your draft. Drafts also
survive navigation within the current window, but are not saved across an app restart.
The catalog works with Zeca off and has no access to his private memory. Saving does not start
an agent, send a provider request, write agent configuration or change permissions.

**Back to Flock** returns to your existing session view. Waiting-session chips remain available
while reading profiles; open one to review its card in the Flock. A failed catalog read offers
**Try again** and does not start an agent or make a provider request.

## Flock

The flock sits on one wire, three times its size, with a project name under each bird. Below it,
a session list shows each project's state, agent, step number, reported running time and context
use. Select a bird or row (or use **↑ ↓**) to put that session in front. Hover for the complete
project, branch, agent and state. A right click, the menu key or **Shift+F10** opens the existing
quick actions: terminal, folder, Activity, last diff, focus, mute, pin and hide. A waiting card
keeps its own session in front.

Above the list, **All sessions**, **Needs you**, **Working**, **Failed** and **Quiet sessions**
filter the rows. Quiet includes resting sessions and birds flagged for silence. Counts cover the
whole flock even while searching. **Find a session…** reads projects, repositories, branches,
agents and current steps. Filtering leaves the selected session and its card in place. The list
and details scroll independently. In a narrow Nest, **Back to sessions** opens the list; choosing
a row returns to its details. In a short window the wire gives way to the selected content.
A new request returns to its card, including while the narrow list is open.

The search, status filter, detail tab and chosen diff survive closing and reopening the Nest or
returning to the island until Vults exits. **Clear filters** clears search and status, keeping
the selected detail. With Zeca off, his saved detail opens **Session**. Restoring navigation
only reads local state. Live updates preserve scroll and keyboard focus; tab changes respect
reduced motion and manual pause. Left/Right and Home/End move between detail tabs.

The selected session's card shows its state and the command or question it asks, with its existing
actions. A finished or failed turn shows its last reply or error. **Snooze**, **Keep going** and
**Dismiss** remain on a session flagged for silence. Below the card:

- **Session** opens by default: running time, cost, context use, four recent steps and up to four
  changed files. **All steps** and **See the changes** open the full kept Activity and Changes.
  Reported zero is displayed as zero; a missing metric says **Not reported** and has no meter.
- **Changes** keeps the last 32 edit diffs, a change picker and **Focus diff**, plus every tracked
  changed file. A file with a kept edit opens that diff. A file detected in the repository during
  a turn without a reported edit says *by a command*, without invented line counts.
- **Activity** shows the kept recent steps and their start times.
- **Context** lists the project, repository, branch, folder, agent, editor, state and reported metrics.
- With Zeca on, **Zeca** opens Chat with a question about that session ready to send.

Running time and context are the CLI's last reported snapshot, currently from Claude Code's
[status line](https://code.claude.com/docs/en/statusline). Running time includes resumed runs;
it is not inferred from step count or Vults uptime. Context is current occupancy and may fall
after compaction. Cost is an API-price estimate, not a subscription charge. Agents that do not
report these values, including Codex hooks, show **Not reported**. Reading the summary makes no
network request and changes no agent configuration. In Full, the plans' usage remains below,
amber from 70% and red from 90%.

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
question. **Skip for now** defers that card without answering it or renewing its deadline;
select its session or **Show the card** to reopen it. Skipped cards keep their birds and
waiting count, while the **Needs you** strip hides their tags. One with no card here
(it asks in its terminal) says so.
**Open terminal** shows where Vults can bring its terminal forward
([The island](island.md#getting-to-a-session)).

## Chat

The chat header identifies Zeca as the recipient separately from the executor. Sending keeps the
conversation and folder selected at the click. A conversation change cancels a queued send; an
obsolete window request is rejected before recall or a model turn, and stale draft saves cannot
replace the new conversation's draft. Local `/remember`, `/forget` and `/continue` commands also
check Zeca as their recipient and retain editable text on rejection. A late command result keeps
newer typing. Changing conversations cancels the chat microphone and discards late transcription;
voice-command handoff cannot inherit a later empty conversation.

**Ask Zeca** actions in Flock, Connections and the command palette check the current recipient
before adding a question. They preserve draft text, attachments and source, including text typed
while preparation waits. A different recipient or a conversation switch rejects the preparation
without resetting a conversation or sending anything. Preparation errors leave a running reply
and its permission cards active.

The island's conversation with Zeca, wide, with replies in full Markdown (a link shows its address
and is never followed); a message, **New** or another engine in either one shows in both. Cards
are answered here too, with **Deny**, **Allow** and **Always in this chat**, as on the island.
The title bar holds the conversation's title, the engine pill (as on the island: the engine in its
color and its model; its menu holds the engines, the models and the effort), the folder, and
the available panel controls. The composer has a paperclip: the files you pick go to this chat, as a drop on the
island does. The left panel button opens **Recent**: a sidebar in a wide window or Full,
or a separate view with **Back to Chat** in a narrow Room. While Recent is visible, its own fold
button (wide) or Back to Chat (narrow) replaces the title-bar button. Conversations are grouped by date,
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

- **Week**: the week's agent figures (time, turns, lines changed, your answers), dictated words, a
  bar a day (a click shows that day), **Save as image…** with or without the project names, and
  the year's grid, with the history's switches ([Activity](activity.md)).
- **Dictation**: measured usage for the last 7 days, 30 days or all time, with daily bars and top apps
  ([Dictation statistics](activity.md#dictation-statistics)). The selected period survives reopening.
- **By project**: this week's time per project in bars (a repository's worktrees add up under it,
  with how many it has), a search
  by project, branch or agent, and a card for each repository a session works in: its folders and
  branches, its sessions (a click puts one in front on **Flock**), your pull requests and checks (from the GitHub card; a pull request or the default
  branch's checks open on GitHub), what Zeca remembers of the project and what its agents cost
  ([Zeca's memory](memory.md#project-memory)). **Preferences ↗** opens **Settings → Projects**.
- **History**: each finished turn of your agents, counts only. Search the loaded
  records by project, agent or status; **Clear filters** restores the list.
- **Cost**: what the agents cost today and the **daily budget** set in **Settings → Activity**,
  your plans' usage windows and what the agents cost over the last two weeks, against that budget,
  and by roost. **Budget settings** opens the budget editor
  ([Cost and a daily budget](activity.md#cost-and-a-daily-budget)).
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

## Dictation

The fifth tab opens [dictation history](dictation.md#dictation-history), including with Zeca off.
The shared navigation groups History, Voice commands, Dictionary and Snippets with separate
configuration pages: Voice, Keyboard shortcuts, Command settings, Vocabulary, Styles, Paste,
Privacy and Diagnostics. The shared transcription model/language editor lives in Voice; Chat
links there. Configuration keeps its privileged webview; transcripts stay in Nest.
Statistics opens Activity > Dictation. Narrow windows use a labeled page picker; the content
scrolls independently. History search, disclosures, drafts and reading position survive size
changes. Erasing transcripts keeps measured usage and Your words preferences. See
[Your words processing](dictation.md#your-words-processing).
