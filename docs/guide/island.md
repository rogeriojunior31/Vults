# The island

The island hangs from the top edge of your screen. On KDE Plasma, Hyprland, Sway and other compositors
with layer-shell it sits above every window like a panel; on GNOME it is a regular always-on-top window.
On X11 it shows on every workspace and never takes the keyboard, except while you type in the chat.

With more than one screen, the desktop picks one (usually the focused screen at start). To pin the
island to a screen, choose it in **Settings → General → Screen**. If that screen is unplugged or turned
off, the island moves to another one and goes back when it returns.

## The flock

Each agent session is a vulture: the black vulture, or another of Brazil's vultures (the turkey
vulture, the two yellow-headed vultures; a project with three or more sessions gets a king vulture).
A session keeps its bird while it lives; the flock changes each time the app starts. **Settings →
Flock** widens the flock to the vultures of the Americas or of the whole world, and picks Zeca's
species. **Zeca**, a black vulture unless you pick another, is not a session: he sits on the wire
when nothing runs and is the one you talk to in the chat. The session in front (the one that needs
you, the one you clicked, or else the one that arrived first) takes his spot in its own bird; work
alone never moves it, so the birds keep their species and places and the flock does not shuffle
with every event. Every other session is a **vult**. A small band at the base of each neck shows the agent: orange for Claude Code, teal
for Codex, blue for Gemini CLI.

![The flock of the whole world: a Himalayan griffon in front, then an Andean condor, a cinereous vulture, a lappet-faced vulture and a griffon vulture](../assets/island-flock.png)

The birds do what their sessions do: Zeca lowers his head to read, pecks the wire while editing, tugs
at it while a command runs, takes off and circles when the agent goes to the web, and spreads his
wings when a permission waits for you. New sessions fly in and land; sessions that end fly off. Each
running subagent sends a scout (a turkey or yellow-headed vulture) to circle low beside its session's
bird in the open island. Now and then a rare visitor (a condor, a griffon) rides the thermal once and
goes on its way; **Settings → Flock** turns visitors off. Waiting for work, they take off after two
seconds idle and circle together below the island. A session that finished its turn plays its done
clip first, then its species' signature, and joins them once both are over; its badge stays until you
dismiss it. The flock keeps circling while the island is open, and stays up as long as those sessions
are idle. When a session starts working, only its bird returns to its perch; the others keep
circling. Permissions keep their
priority, and unacknowledged outcomes stay visible. Reduced motion keeps the birds on their perches.
These are the actual sessions, not extra decorative birds. See [Animations](../ANIMATIONS.md) for
every clip and the behavior it comes from.

### Zeca's seasonal looks

Zeca dresses up for the season: a witch hat from October 1 to November 1, a Santa hat from
December 1 to 26, a party hat from New Year's Eve to January 2, and bunny ears from Good Friday
to Easter Monday. The days follow your computer's date and time zone. **Settings → Flock → Look**
keeps him in one look all year (sunglasses are only there, as no summer fits both hemispheres), or
in none. The same list has fourteen outfits with no season: bandanas, caps and hats, shades, a
durag or dreads with a gold grill, a crown, headphones, an eye patch, and chains with a medallion,
a cross, a clock or a chrome plate hanging from the band. Only Zeca wears it; the vults keep their
feathers.

**Right-click Zeca** on the island (on his perch, or on his spot in the folded pill) for the same
choice in place: the looks in three groups (*Seasonal*, *Head*, *With a chain*), with *Auto* and
*None* at the top. Zeca wears whatever the pointer rests on, so you see it on him before you pick;
a click picks it (Settings follows), and Escape or the close button leaves him as he was. A card
that needs you takes the island's place, as always.

![Zeca's looks: the four seasonal ones, sunglasses, and the fourteen outfits with hats, shades, grills and chains](../assets/zeca-looks.png)

### Without Zeca

Zeca is optional. **Settings → Flock → Zeca** turns him off; everything about the flock keeps
working: sessions on the wire, permission and question cards, the Allow / Deny shortcuts,
notifications, connectors and the presets. What goes with him:

- The chat, its tab and the drop tab, the tray's *Chat…*, the microphone and the talk shortcut
  (no chat starts and the mic stays closed; a Codex conversation's server ends). The usage meters
  stay: they read your agents' subscriptions.
- His hello at start-up.
- His place on the wire: the session in front sits there as its own bird, and with nobody running
  the pill reads *Nothing running* over an empty wire.

Turn him back on in the same place; no restart either way.

## Compact, open and hidden

![The compact island: Zeca with the session in front, and four vults with their badges](../assets/island-compact-flock.png)

**Compact**, the island is a small pill of fixed size: Zeca on the left with the session in front,
its project and what it is doing (in color when it needs you: amber for a permission, cyan for a
question, green when done, red when it failed), and up to four vults on the right. A vult whose
session finished, failed, asks something or waits for a permission wears a badge of that color.
Connector news shows as *2 new*.

**Click** the pill to open the island. It stays open while the pointer is on it, and folds back
15 seconds after the pointer leaves; a thin line at the bottom shrinks during the last seconds, and
coming back cancels it. The **Fold** button (the chevron at the top right) folds it at once.
**Settings → General → Fold the island** sets the wait (5, 10, 15, 30 or 60 seconds).

With **Settings → General → Open on hover** on, resting the pointer on the pill (or on the top
edge, where the hidden island wakes) opens it, without a click; just passing over it opens nothing.
Opened that way it folds as soon as the pointer leaves, unless you clicked in it: then it waits like
any open island. Off by default, and not in *Panel*.

Two things keep it open until you are done:

- **a permission card.** It opens the island by itself and stays until you answer; nothing else
  opens it by itself. A session that finishes, fails or asks something plays its sound and gets
  its badge instead.
- **the chat.** It has the keyboard, so the island never folds while you type.

With nobody on the wire for a minute, the island **hides**. Move the pointer to the middle of the top
edge of the screen and it comes back.

## Presence: how much it shows

**Settings → General → Presence**, or the tray's menu, picks one of four presets. The change is
immediate, with no restart.

| Preset | At rest | A card that needs you |
|---|---|---|
| *Island* (the default) | The flock at the top of the screen, as described above | Opens the island, with its sound |
| *Panel* | Nothing at the top; Zeca in the tray (below) | Opens the island by the panel, with its sound |
| *Quiet* | Nothing: no flock, no sound when a session finishes or fails, none for connector news. The strip at the top of the screen still brings the island back | Opens the island at the top, with its sound |
| *Paused* | A pill that says *Paused*, nothing else | None: the agent asks in its terminal at once, as if the app were closed |

In every preset but *Paused*, a card always opens the island and waits for your click there: no
preset hides a card an agent is waiting on. *Paused* is for when you want no interruptions at all:

- A permission or a question goes straight to the agent's terminal; the agent does not wait for
  the app. Cards already waiting when you pause go to their terminals too. Rules you made with
  *Always* do not answer anything while paused.
- Connectors stop checking (they stay on in Settings and start again when you pick another preset).
- No desktop notifications.
- Zeca in the tray still shows the flock, wings open when an agent waits on its terminal.

Pick another preset to come back; the island shows the flock again at once.

### In the panel

- Nothing is drawn at the top of the screen. Zeca sits in the panel's tray and shows what the flock
  is doing: perched (nothing going on), working, wings open with an amber mark (a permission or a
  question waits for you), green sparkles (done), a red mark (failed). He moves slowly, a frame or
  two a second, and holds still when the desktop's animations are turned off.
- **Click** him and the island opens in the panel's corner, beside the tray. It folds away as usual
  once the pointer leaves it, or with the **Fold** button or Escape, and at once when you go to
  Settings, so it never covers that window's corner (a waiting card or an open chat keeps it).
  Switching to *Panel* folds an open island too. **Right-click** him for the menu (*Chat…*, *Set up
  agents…*, the four presets, *Quit*).
- **A card opens the island by itself**, by the panel, with its sound, and Zeca calls for attention
  in the tray until you answer. The tray only shows: Allow and Deny are always clicks on the card.

On KDE Plasma the corner follows the panel that holds the system tray: by a bottom panel (Plasma's
default) the island opens at the bottom right, by a top panel at the top right, by a side panel at
the bottom of that side. On other desktops it opens at the bottom right. The tray needs a panel that
shows StatusNotifierItems (Plasma does; on GNOME, the AppIndicator extension, where a left click may
open the menu instead). Without a tray, start Vults again to open its settings.

Connector news (a failed check, a review) still plays its sound in *Panel* mode, but neither opens
the island nor changes Zeca in the tray: click him to read it.

In the other presets Zeca is in the tray too, with the same states and menu, and a click on him
opens the island at the top.

## The corner widget

**Settings → General → Corner widget** puts a small window in a corner of the screen you pick (top
left, top right, bottom left or bottom right); it is off until you pick one. It shows up to three
birds and how many sessions are working or need you:

- The first bird is the session in front (the one the island shows), Zeca when nothing runs; then
  the sessions that want you most: a card waiting, a failure, finished work.
- The text says how many need you (in amber, with an amber rim around the widget) and how many are
  working; *Nothing running* with an empty wire, *Paused* while the app is paused.
- **Click** it and the island opens, on the card when one waits. Click a bird and its session comes
  to the front first (not while a card waits: the card stays in front, unless that bird's own card
  waits behind it, which then comes first). The widget never answers a
  card: Allow and Deny are clicks on the card in the island.

The widget stays clear of the desktop's panels and sits under full-screen windows, like a panel. It
works with every preset, *Panel* included. Without layer-shell it is a small always-on-top window:
in that corner on X11, wherever the desktop puts it on GNOME. It is another web view: about 40 MiB more memory while
it is on, and none when it is off.

## Desktop notifications

In *Panel*, where the island is out of sight, Vults also tells you through your desktop's
notifications (Linux, any desktop with a notification server: Plasma, GNOME, Mako, Dunst…). At the
top of the screen (*Island*, *Quiet*) the island already shows all of it, and a card opens it with
its sound: nothing goes to the desktop, so you are never told twice. *Paused* shows none either.

- **A session finished**, with the first line of its last reply, or **stopped on an error**, with
  the error.
- **A session needs you**: a permission or a question card waits. It comes at once, with the
  island opening by the panel.
- **A session has gone quiet**: it has been working with no news for 15 minutes
  ([a quiet bird](#a-quiet-bird)).

What finished or failed while paused, or with notifications off, is not raised later, when you
resume or turn them back on.

Each session has at most one notification: a newer one replaces it, and it goes away by itself
once the card is answered (here or in the terminal), the session gets back to work, or the session
leaves (a finished bird leaves the wire after 10 minutes, a silent one after 30, and its
notification with it). Its title is the project's name. Turning notifications back on, or
switching to *Panel*, shows what is going on at that moment, a finished session included; switching
back to the top withdraws them.

A notification has one action, **Open** (or a click on it): the island comes up by the panel, with
that session in front. It never has Allow or Deny: only a click on the card
answers it. Turn them off in **Settings → General → Notifications**, or for a while with **Do
not disturb** just above it: what finishes meanwhile is not raised when it ends.

## The open island

![The open island: the focus card and the flock list](../assets/island-busy-flock.png)

Open, the island has three tabs, as icons (their names show on hover): **Flock**, **Chat**, and
**Drop a file**, which opens the chat on a drop zone saying how to hand Zeca a file. Once GitHub
has answered, a fourth tab shows its card ([Connectors](connectors.md#the-github-card)). On the right: a
sound toggle, the settings button and **Fold**. Settings opens at **Agents** while an agent's
hooks are older than this version or none is installed yet (the tray's *Set up agents…* always
opens there), and at **General** otherwise. Below them, side by side:

- **the focus card**: Zeca, large, on his piece of wire, with a glow in the color of the session's
  state and a mark over his head for it (a thought bubble, an amber `!` for a permission, a cyan `?`,
  green sparkles when done, a red `#` when it failed, sweat at a usage limit), and that session's
  card beside him;
- **the flock list**: one row per other session, with its bird, its project, what it is doing (or
  its state, in color) and its badge. More than four rows scroll. With a single session the card
  takes the whole width.

| Session | Card |
|---|---|
| Working | Its project, the agent, how many steps so far and its running subagents, then the step ticker: the step just done, and the current one |
| Idle | *Waiting for the next prompt*, and the last step |
| Needs permission | The exact command, file or URL, with **Deny** and **Allow** |
| Has a question | The question itself, and **Open terminal** to answer it |
| Finished | The first paragraph of the agent's last reply on one line, with markdown marks and tables left out, **OK** and **Open terminal** |
| Failed | The error, **OK** and **Open terminal** |

A step reads like *Editing main.rs* or *Running cargo build*. A test suite says *Testing cargo test*, and
a tool from an MCP server names the server and the tool: *Calling github · list_prs*.

Once an edit is done, its step shows the lines it added and removed (*Editing main.rs* **+4 −2**).
Click them to see the diff in place of the card, with the file's line numbers when the agent sent
them (Claude Code does; Codex's patches and Gemini CLI's edits have none). One patch over several
files shows each file in turn. **×** or **Esc** goes back. A diff is kept only while its step is among
the session's last eight, and at most 400 lines of it; a longer one says it stops short.

![A finished edit's diff in place of the card: ticker.ts +4 −2, with line numbers](../assets/island-live-diff.png)

**OK** says you have seen it: the badge goes and the card shows the session as idle, until it starts
working again. When the card or the session in front changes, the old card fades out as the new one
fades in.

With nobody on the wire, the card says so and offers **Ask Zeca**.

Zeca notices you: he looks toward the pointer, turns to you when it is on him, preens if you leave it
there, startles at a click, and gets cross at three. When the app starts he lands on his wire and
says hello before the island folds: by your first name when your account has a full name (taken
from your user account, never from the network; a login name like `jdoe42` is not used). The chat
knows it too.

## Now playing

Turn on **Settings → General → Now playing** and the island shows the song your music player is
playing (Spotify, a browser tab, mpv: any player that speaks MPRIS). With nothing running, the
compact island says *♪ Song · Artist*; the open island shows it in the top bar, with previous,
play/pause and next when the pointer is on it. While the music plays, idle birds dance to it.

## Subscription usage

![The top bar with plan usage and the song playing](../assets/island-usage-music.png)

The open island's top bar shows how much of your plans' limits you have used, each next to its
agent's dot: *7d 12%* is 12% of the weekly window. Each window goes by its length (*5h*, *7d*),
since plans differ: some only have a weekly one. It turns amber at 70% and red at 90%, and
pointing at it tells when it resets. No token of yours is ever read: each number comes from the
CLI you logged into.

- **Codex**: asked every 5 minutes, read-only (nothing is spent), never while the screen is locked;
  when a read fails (no Codex, logged out) the next waits twice as long, up to an hour.
- **Claude Code** (Pro or Max): it reports usage only to a status line, so installing the hooks
  also adds one of ours, which shows nothing in Claude Code. The numbers arrive after a session's
  first reply. If you already have a status line of your own, it keeps showing: the install saves
  it and our status line runs it (the diff says so), and removing the hooks puts it back.

## Getting to a session

Click a row in the flock list to put that session in front. It stays there until you pick another
or it leaves; a card waiting for you still comes first. With nothing picked, the session that
arrived first is in front. Click **Open terminal** on its card to
bring its terminal forward: the multiplexer pane first (herdr, tmux, kitty, wezterm), then the
terminal window itself, on KDE Plasma (Wayland or X11) and in any X11 session (Xfce, Cinnamon, MATE,
i3…), on its own workspace and unminimized. When the terminal has several windows, the one titled
after the session's folder comes first. On GNOME, Hyprland or Sway under Wayland, only a terminal
running through XWayland can be raised. kitty is reached through its remote control socket: set
`listen_on unix:/tmp/kitty` and `allow_remote_control socket-only` in `kitty.conf`. When there is
nothing to bring forward, the card says so.

From anywhere, without leaving the window you are in:

- **Ctrl+Alt+J** puts the next session in front and **Ctrl+Alt+K** the previous one, in the
  flock's order, going round at the end. While a card waits it stays in front; the keys choose
  who comes after it.
- **Ctrl+Alt+Space** opens the island. It folds as usual once the pointer is away.

These are global shortcuts through the desktop portal, like Allow and Deny: KDE asks you once to
accept them, and you can change the keys in **System Settings → Shortcuts** (worth it if your
editor uses Ctrl+Alt+J or K: JetBrains IDEs do). **Settings → General** lists them.

A session started in the terminal of VS Code or Cursor carries a small **VS Code** or **Cursor**
tag next to its project, and its button reads **Open in VS Code** or **Open in Cursor**: on KDE and
in X11 sessions it raises that editor's window.

### Quick actions

**Right-click** a vult in the folded pill, a row of the flock list, or the focus card (away from
Zeca, whose right-click keeps his looks) for that session's quick actions, in place of the card:

- **Open terminal**, as on the card, where the app can bring it forward: a herdr, tmux, kitty or
  wezterm pane on any desktop, or the window itself on KDE and in X11 sessions. Elsewhere it is
  greyed out, the menu says the desktop can't, and offers the folder instead.
- **Open folder**: the session's folder in VS Code when `code` is on your `PATH` (the item then
  reads *Open folder in VS Code*), else in your file manager. Only an existing folder, by its full
  path, is opened, and no shell is involved.
- **Activity**: every step the session keeps (its last eight), numbered, with each finished edit's
  **+N −M** to open its diff. **Esc** goes back.
- **View the last diff**, and **Open its file in VS Code** at its first changed line (without VS
  Code, *Show its file in the folder* opens the file manager on it; a file is never run or opened
  with whatever handles its type). A patch over several files opens the first one, and says so.
- **Keep in front**, or *Let the flock choose* for the session you put in front.
- **Mute**, **Pin** or **Hide this project**, for a session with a folder (below).

The menu never has Allow or Deny, and a card that needs you takes the island's place, as always:
the menu does not open over one. To get to a card waiting behind another, use its notification's
**Open** (in *Panel*) or its bird in the corner widget: that card comes to the front of the line.

### While you were away

When the screen locks (any desktop that says so through `org.freedesktop.ScreenSaver`: Plasma,
GNOME and others), the island rests: no animation and no timers, and connectors stop checking
until you are back. Agents and cards go on as ever: a card still opens the island and waits.

When you unlock, or pick another preset after *Paused*, a line over the news says what happened
meanwhile, counted from the sessions themselves (no model): *While you were away: 2 finished, 1
failed, 1 waits for you for 12 min.* In the *Island* preset it opens the island once, then folds
as usual (by the panel or in *Quiet* it waits for you to open it), and it stays in the open island
until you dismiss it with **×**. Only what happened while you were away counts: a session that
finished meanwhile, with notifications off or do not disturb on as well, and the cards that came
meanwhile and still wait. Nothing happened, no line; projects you hid or muted are left out.

### A quiet bird

A session that is *working* (a tool started) and sends nothing for **5 minutes** may be stuck: a
command waiting for input in its terminal, a hung tool, a long build. Its bird gets a grey badge
and its status reads *No news for 5 min*. At **15 minutes** the badge turns amber, the card says
*No news for 15 minutes.* on an amber glow, one alert sound plays, and in *Panel* a desktop
notification says it has gone quiet (unless notifications are off or its project is muted). Thinking (a long
reply) is not flagged, and any news from the session clears the flag at once.

The card has three answers, and none of them touches the agent: the app only tells.

- **Snooze**: the flag goes for 15 minutes, then comes back as it was if the session is still
  quiet, with its sound (and in *Panel* its notification) again.
- **Keep going**: it is fine; the grey flag comes back only 30 minutes later, and the amber one 10
  minutes after that.
- **Dismiss**: no flag again in this run; its next prompt starts watching again.

The corner widget and the tray count a loud quiet bird as worth a glance, like a usage limit. A
silent session leaves the wire after 30 minutes without news, as always; any of the three answers
counts as news for that, and *Keep going* keeps it on the wire until its flag could come back.

### Per project: mute, pin, hide

Sessions leave the wire within 10 to 30 minutes, so these choices belong to the project (its
folder), not to one session: every session in that folder follows them, now and later.

- **Mute**: no sounds and no desktop notifications from its sessions finishing or failing. A card
  from one still opens the island with its sound (and in *Panel* its notification): nothing quiets a card an
  agent is waiting on.
- **Pin**: its sessions come first on the wire, in the pill and in the list (and the next and
  previous session keys walk them first).
- **Hide**: its sessions stay off the island, the tray and the corner widget, and send no
  notifications. A card from one of them still shows, with its session, until it is answered:
  nothing hides a card an agent is waiting on.

**Settings → Projects** lists every project with a choice on, with a toggle for each, and
**Forget** to clear them; it is the way back for a hidden project.

## Sounds

Short 8-bit blips when a session needs you, finishes or fails, and for connector news; softer ones
when the island opens, folds or comes out of hiding, and when Zeca reacts to you. The speaker button
on the island, or **Settings → General → Sounds**, turns them off. **Volume**, just below, sets how
loud they play; a cue plays when you let go of the slider, and the island uses the new volume at once.

A card that keeps waiting sounds again at 45 seconds and every 30 seconds after
([the attention ladder](approvals.md#when-nobody-answers)). **Settings → General → Do not disturb**
silences sounds and notifications for 30 minutes, 1 hour or 4 hours; a moon shows in the open
island's header while it lasts, and a click on it ends it. A card still opens the island with its
sound (and in *Panel* its notification), but does not sound again.
