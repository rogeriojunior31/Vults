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
**Settings → General → Theme** chooses **Dark** (the default), the neutral **Light**, or one of
three SP Night themes, three views of the city at night (**Noite Paulista**; **Garoa**, the flat
grey of drizzle; **Pico do Jaraguá**, the dark of the forest), across every window. <!-- check-english:allow -->
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
full-screen button. The same header holds five tabs in this order: **Flock**, **Chat**, **Dictation**,
**Connections** and **Activity**. They show their icons, the number that needs you on **Flock** and the connectors' rows on **Connections**, and
**Settings** at the right, which shows Settings in this same window, under the same tabs; a tab
brings the Nest back. Over every tab but **Flock**, a tag for each session that needs you (or
failed); a click opens **Flock** with it in front (a card that waits comes first there), where you
answer it. Settings stays a page of its own there: it is the only one that changes an
agent's config, a key, a policy or a phone, and it never shows an agent's text. The switch keeps
the tab you are on: **Room** is the window, the full-screen button the whole screen (**Esc** goes
back to Room), **Quick** closes the Nest and opens the island, **Mini** closes it and leaves the
pill. **Alt+1** to **Alt+5** pick a tab in that order (hidden Chat is skipped). At the bottom, **Zeca on · optional** / **Zeca off**
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
Choose a card to read its instructions and explicit revision. **Create profile** opens a dedicated
blank editor: enter your own name and instructions without choosing or copying a preset.
**Create a copy** remains available on built-in profiles and fills the same editor with that
profile's instructions and appearance. The editor puts name and instructions first, with appearance
and optional pinned skills alongside them in a wide window and below them in a narrow window.
**Save profile** creates a custom profile with Ask access by default; choose another access level
after saving. **Edit profile** saves a new revision. Built-in profiles remain unchanged.
In **Appearance**, choose a **Species** and an **Accessory**; the preview uses the same sprites as
the flock. Accessories are fixed choices, including **None**, without seasonal changes.
The card shows the saved appearance after saving. In **Pinned skills**, choose from the built-in
skills and review their literal content and revision. Each profile save keeps the selected content
with that immutable revision, and each specialist conversation keeps the exact profile and skill
snapshot it began with. Pinned skills do not enable tools.

The profile catalog keeps the birds in a compact list beside the selected profile. Its animated
bird greets you on click and follows your appearance choices in the editor; hidden pages stop
animation, and reduced motion keeps it still. **Open specialist chat** and **Create a copy** sit
beside the selected bird, with its access choice below. Expand
**Instructions**, **Pinned skills** or **Source** for the complete retained content and execution scope.
Opening waits for an access save to finish, and a failed opening stays visible with a retryable action.
Choose **Open specialist chat** to start or resume that profile in the existing **Chat** tab.
The recipient selector switches between Zeca and profiles, using the same composer, transcript,
history sidebar and approval cards while retaining each recipient's draft and conversation. The
animated perch and reply signatures show the species and accessory pinned to that conversation,
including when Zeca is off. Changing the profile affects new conversations; an older conversation
shows its pinned revision and the latest available revision. Start a new conversation to use it. It runs
through an isolated Claude Code session with only the pinned profile and skills: no project or flock
context, files, voice, or Zeca memory. It remains available when Zeca is off. Its saved conversations
and approval cards stay inside the selected profile and never appear in Zeca's Chat history.

Each profile has its own **Access for this profile** setting: **Asks first** is the default, **Your
policies** applies the policies you accepted, and **Full access** answers what those policies leave
open. Policy denies block the call. Destructive commands, policy asks and actions after reading outside
content still show an approval card. Choose any access level directly in the Chat picker or the profile; opening Settings is not required.
Access changes stay synchronized between the profile and Chat and apply to the next tool call only
after saving. A failed save keeps the previous access and offers a retry in Chat. Review the command and any outside-content disclosure before choosing **Allow**;
**Always in this conversation** repeats only that exact call in that conversation. The app records the decision.

Custom profiles live in the local database, independently of conversation history. Each save
retains earlier revisions. A stale editor is rejected instead of overwriting newer changes;
cancel and reopen to read the latest revision. Failed saves preserve your draft. Drafts also
survive navigation within the current window, but are not saved across an app restart.
The catalog and specialist Chat work with Zeca off and do not access his private memory. Saving a
profile does not start an agent, send a provider request, write agent configuration or change permissions.

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

Metrics distinguish current context, cumulative session consumption and the time each agent actually reports. **Context** shows model/provider metadata, native versus catalog cost provenance, and independent reading ages for cost, time and context. Partial reports preserve other channels; an explicit Claude context reset clears its old gauge.

| Agent | Numeric source and limits |
| --- | --- |
| Claude Code | Native status line: reported execution time, API-equivalent cost and latest context. Installing/updating keeps an existing refresh interval, otherwise uses two seconds. The status line's context totals are not cumulative session consumption. |
| Codex | Known session's local index and incremental rollout: native turn durations, latest context, reported cumulative tokens and current standard API-price estimate. Idle time between turns is excluded. |
| OpenCode | Read-only local database: native cost, completed model-response duration, latest context tokens and cumulative completed-response usage. |
| Hermes CLI/Desktop | Observer API hooks for latest prompt tokens and response time observed since plugin load; local database for reported/estimated cost and cumulative usage. Tool/idle time is excluded; CLI sessions need the enabled plugin. |
| Qwen Code | Native monthly usage ledger for API duration, cumulative tokens and per-call API-price estimates. Known chat JSONL is the fallback for context/usage. Subagent consumption never replaces the main context. No context percentage without a native limit. |
| Copilot CLI | Persisted `session.shutdown` ledger: cumulative tokens, API duration, last context and premium-request credits. Shutdown snapshots replace older totals across resumes. Credits are never converted to USD; an API equivalent requires unambiguous prices and no unknown per-call context tier. |
| Kimi Code | Known session's main-agent `usage.record` wire stream. Cache reads/writes remain separate and inherited records before a fork marker are excluded. No invented duration; API pricing requires an exact, unambiguous model. |
| Factory Droid | Known session's cumulative numeric settings summary. No cost, current context or duration is inferred from cumulative tokens. |
| Cursor | Native `preCompact` hook reports context before compaction. The documented hooks do not expose the complete session cost/time metrics. |
| Antigravity | CLI numeric status line, installed alongside the shared hooks with separate backups and previous-command chaining. The native CLI supplies context and estimated USD; desktop/IDE hooks alone do not supply these numbers. |

Local readers run every two seconds for sessions already in the flock, off the UI thread. **Session consumption** keeps cumulative input/output/cache counters separate from current context. Unknown remains **Not reported**, never zero. Source/version coverage differs: installed Codex and numeric databases were inspected locally; additional adapters have isolated contract checks and still require live CLI acceptance where the CLI is not installed.

Catalog estimates use [models.dev](https://models.dev) every 15 minutes when **Automatic model prices** is enabled, or on manual refresh. No session data is sent. After 24 hours without successful verification, cached estimates become unavailable, including unchanged Copilot/Kimi logs. Unknown/ambiguous models, missing cache tariffs and aggregate usage with unknown context tiers are not assigned guessed prices. Native cost reports retain their own provenance.

**Claude usage** separately enables a native account-allowance check every five minutes without
sending a model prompt.
Reading the summary makes no network request and changes no agent configuration. In Full, the
plans' usage remains below, amber from 70% and red from 90%.

The Session summary reports when metrics were last read and distinguishes waiting, failed reads
and delayed checks. Context adds the last agent event, last successful read, last check and the
local source. Reading metrics never refreshes the activity clock. Quiet agents are not marked as
disconnected just because they stop sending events. A failed local read retains the last values
and marks them unavailable. OpenCode's **Model response time** measures completed response
intervals without counting overlaps, gaps or tool execution. Qwen response duration appears only
when the native usage ledger supplies it.

Changes also shows **Repository changes**: a read-only comparison of current local files with
HEAD, including new/deleted files and edits made by commands or scripts. This is the repository's
working state, not attribution to one agent; it can include earlier work. Choose a file to see
numbered added/removed lines. Refresh runs every four seconds only while this tab is visible,
with manual retry and an expanded diff view; approvals always restore their card. Binary/large
files and previews above the size budget are explicitly omitted. Git reads raw objects without
running worktree clean filters, text converters, external diff programs or hooks. No files are
staged or written. Agent-reported edits remain available separately.

Permission cards show the operation's working folder, which may differ from the session's folder.
Codex, Qwen and OpenCode folder fields survive parsing and plugin forwarding. That folder is
included in the signed approval binding and scopes an Always rule; an unresolved relative folder
cannot create one. A disconnected hook expires its card without answering it, and a request that
has already missed the acknowledgement deadline is never offered. The same checks apply to
Vults-owned approval channels. A stale click cannot answer an expired offer.

Context and Settings → Agents also show registry-backed Watch, Answer and Start support.
Execution marked planned is not advertised as enabled; installation remains a separate check.

Context also explains integration limits. Questions from Codex and OpenCode 2 remain in their
terminals, and only Claude/Codex turns started by Vults can be interrupted here. Settings → Agents
checks OpenCode's installed major version before advertising question cards. The read-only
installed-client upgrade canary is `cargo test -p vults-chat installed_acp_clients_initialize_without_a_model_turn -- --ignored --nocapture`;
it initializes OpenCode and Qwen without creating sessions or sending model prompts.

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

**History** uses **Search turns**; **Audit** uses **Search answers and commands**, matching the
records each view contains. **Clear filters** is available when a query is present. Press **Esc**
in a non-empty search to clear it and keep focus there; text composition retains its Escape key.
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

Each list row with check data shows a small segmented summary beside its status. Failures come
first, followed by running and waiting checks. Hover for each check's name and state; assistive
technology reads the same information. Large matrices show up to twelve segments and a remaining
count; the selected detail still lists every check. Rows without check data show no check summary.

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
