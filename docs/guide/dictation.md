# Dictation

The N3 development build provides local dictation to the clipboard, with optional paste into
supported native X11 or Plasma Wayland text editors and exact tmux flock panes. It includes
measured island feedback, searchable History, Dictionary/Snippets, Activity statistics and
Settings with Diagnostics. Nothing is submitted automatically. Features shown only in the separate
approved prototype are not enabled here.

## Voice available today

Use [Chat's voice input](chat.md#speaking-instead-of-typing) after choosing a local model in **Settings > Chat & voice**.
Each Chat recording keeps up to one minute of audio in memory. The preview and final transcript
use the same local model; you review the draft before sending it.

The shared audio capture now converts input to mono at 16 kHz, keeps a bounded buffer and reports
a device or capture failure instead of silently returning incomplete audio. This change adds no
automatic model downloads.

Speech detection filters brief noise and keeps short pauses inside a phrase. In tap-to-talk,
Chat still waits about 1.5 seconds of silence after speech before finishing; silence before
you speak does not end the recording.

By default, the preview and final transcript share a local text filter. Runs of six or more repeated
words or phrases are reduced to one occurrence (units up to 60 characters); shorter repetitions
are kept. Known sound annotations are removed while other bracketed text stays intact.
A standalone silence artifact such as "Thank you.", "Obrigado." or "Gracias." is omitted,
even if intentionally spoken; longer sentences containing those words are kept. You can edit
the draft before sending it. This filter uses no model or network request.

## Global dictation (first slice)

For a first dictation, choose a model, focus your destination, hold the talk shortcut, speak and
release. Wait for **Copied**, then paste there. Automatic insertion is a separate choice and
only supports the destinations below; it never sends Enter.

Choose and install a local voice model in **Settings > Chat & voice**. The desktop asks for the
new global shortcut bindings; **Settings > Dictation > Keyboard shortcuts** shows the actual
bindings and updates when you change them in the desktop settings. **Change in KDE…** opens
KDE System Settings > Shortcuts through its installed desktop entry. If that entry is missing
or cannot launch, the page offers retry; other desktops use their own shortcut settings. Vults
does not edit desktop bindings itself. Suggested keys:

- **Ctrl+Alt+V**: outside Vults, hold for at least 300 ms, speak and release. Capture keeps the
  last 600 ms before local transcription. Two short taps, the second within 400 ms of the first
  release, keep recording hands-free; the next tap finishes. A single short tap is discarded.
- **Ctrl+Alt+X**: cancel dictation or Chat voice, including dictation that is already transcribing.
- **Ctrl+Alt+B**: copy the last successful dictation again. It does not interrupt an active turn.

An active dictation keeps its owner if focus moves. A focused Vults Chat uses its existing draft
path; the talk shortcut does not start a recording from Settings. Its separate microphone test
starts only with **Test microphone** in Diagnostics. With Zeca off, independent dictation still works.
In **Settings > Dictation > Keyboard shortcuts**, choose **Hold + double tap** (the default)
or **Hold only**. Hold only discards short taps immediately; a second tap starts a fresh hold.
Space-lock remains an independent choice even in Hold only mode. **Space locks recording** and
**Esc cancels dictation** default on and can be changed on X11; their controls are unavailable
on Wayland. With Space-lock off, Space acts like other typing and can discard a held recording.
With Esc cancellation off, Esc leaves the turn running; the global Cancel voice shortcut always
works. Choices are saved locally and apply to the next recording, including after a restart;
changing them never interrupts an active recording or transcription.

With the default choices on X11 with XI 2.1 available, press **Space** while
holding the talk shortcut to keep recording hands-free. **Esc** cancels during recording, the
capture tail or transcription. Other non-modifier keys discard a hold or a pending single tap;
ordinary typing does not cancel a recording already locked hands-free. The listener does not
consume those keys in your destination app. Talk and Copy again's actual shortcut chords are
excluded from other-key cancellation; auto-repeat is ignored. Space-lock keeps the default
Open-island chord from moving focus while recording. A remapped Open shortcut on another key
still follows its usual action.

On Wayland, use double tap and the separate cancel shortcut; Space-lock and global Esc are not
available. No raw keys are observed through XWayland. If an X11 listener or its actual binding
cannot be recognized, the portal's hold, double-tap and cancel controls still work. Key observation
ends with the turn and keeps no typed text or key history.

**Settings > Dictation > Voice > Dictation microphone** chooses an input for independent
local dictation; Chat keeps using its system default. The page lists inputs without recording,
and **Refresh microphones** updates that list explicitly. Loading, failed reads, stale lists
and no inputs have separate feedback. A disconnected saved microphone stays selected as
unavailable; if it is missing when capture starts, the existing capture fallback chooses the
system default or another input. It does not rewrite the saved preference. An input that is
listed but cannot open reports an error. Choose **System default microphone** to clear the
preference. The choice is saved on this computer and used by the next recording; an active
recording keeps its input. Opening the page or refreshing never starts a recording.

**Settings > Dictation > Voice** also owns **Remove repeated loops** and **Retry empty
dictation with Base**; both start on. Turning loop removal off keeps repetitions in both the
preview and final text. Known sound annotations and standalone silence phrases are still removed.
Turning Base retry off skips the extra speech check and second decode for an empty result.
Choices are saved locally and captured when a new independent recording starts; changing them
never changes an active turn or Chat's processing. Neither choice downloads or contacts a model.

The temporary status stays on the island, or in the Nest when it is open. Mini and Quick reserve
a neutral row above the flock, with an icon and text for listening, hands-free capture,
finishing, transcription, retry and each delivery outcome. Long lines are truncated within that
row. It does not take focus. A waiting card or a locked screen hides the feedback immediately;
expired feedback does not return when the card leaves. The island keeps its fixed native overlay
and accounts for the row before applying Quick's zoom and the top or bottom dock anchor.

During listening and hands-free capture, nine bands follow the microphone's measured audio.
On a supported GPU, the existing local voice preview also shows the words heard so far in the
same row. These tentative words can change before final transcription; CPU-only capture still
shows the bands and state, followed by the final clipboard or insertion result. Preview text
stays in memory, is discarded when capture ends and is never written to the app log or history.
Cancelling or starting another dictation rejects late audio and preview updates. Reduced motion
hides the moving bands while retaining the state and words.

**Settings > Dictation > Voice > Dictation start sound** controls a short start cue;
it starts on and follows the app's sound switch, volume and Do Not Disturb. Only a new recording
plays it. Opening another surface, changing this setting or returning from mute never replays
a cue. Failures and cancellations stay silent.

**Settings > Dictation > Voice > Dictation captions** controls the optional caption;
it starts on. The caption expands a clipped line or shows the final text after a confirmed copy
or delivery, in at most two rows. It uses free space below a top-docked island or above a
bottom-docked island; a full-height Quick view leaves it hidden. In the Nest or Settings it sits
above the temporary feedback line, with space reserved in the page to keep its controls visible.
Cards and screen lock hide it immediately. The caption takes
no clicks or focus and does not enlarge the island's input region. It clears within two seconds
of completion; changing surfaces or toggling captions does not renew that deadline. The text
stays in memory. Dictation has its own Settings category.

**Copied** means the clipboard was read back and matched the text; use your destination
app's Paste command (often Ctrl+V, or Ctrl+Shift+V in terminals). Nothing is submitted automatically.

Confirmed delivery also shows a word total from the final text, with a localized singular or
plural label. It uses Unicode word boundaries: punctuation and emoji do not count, and Chinese
ideographs count individually. The total stays with the delivery line after its caption clears.
Recording, failures and cancellations do not reuse an earlier count. Correction totals await
the metrics phase; no correction count is shown yet.

On Wayland, background copying requires the desktop's `ext-data-control` or `wlr-data-control`
clipboard support. If unavailable, dictation reports an error instead of claiming Copied; it
never takes focus or silently writes only to XWayland. Clipboard access uses in-memory text,
not a transcript file. The app log records dictation phases without recording the text.

At the first talk-key press, Vults requests the focused app's class, title, process id and window
identity without activating a window. X11 checks the foreground window again after reading it.
Plasma Wayland uses a read-only KWin script; this asynchronous observation can happen just after
the press, so it is context rather than proof of the exact destination. Late or failed observations
stay unknown. GNOME and wlroots Wayland also stay unknown, even if XWayland is available.
When known, the copied status names the observed app and suggests Ctrl+Shift+V for a recognized
terminal. A title or process match does not establish a focused terminal pane or permit insertion.
Shared terminal processes with several flock sessions leave the session association unknown.
Window titles and native window identities stay in memory and are not sent to the renderer,
written to history or logged. The recognized application and session may be kept with a successful
transcript under its retention choice. Copy again starts without reusing that app context.

## Supported destinations

Independent dictation works with Zeca on or off. The current delivery paths are deliberately
limited; recognizing an application does not grant insertion.

| Destination | Native X11 | Plasma Wayland |
| --- | --- | --- |
| Firefox text field | Copied; paste manually | Copied; paste manually |
| Kate | Copied; paste manually | Copied; paste manually |
| Konsole, including tmux inside Konsole | Copied; paste manually | Copied; paste manually |
| KWrite, GNOME Text Editor, Mousepad, Leafpad | Opt-in paste after focus and clipboard checks | Opt-in paste with a ready keyboard grant and focus/clipboard checks |
| Tracked flock session in an exact tmux pane hosted by XTerm/UXTerm or Kitty | Opt-in literal single-line delivery after live session/pane checks | Same checks, plus the ready keyboard grant |
| Focused Vults Chat with Zeca on | Fill its unsent draft | Fill its unsent draft |

Firefox, Kate and Konsole keep manual paste even when the insertion switch is on. Direct Kitty
or WezTerm panes also remain copy-only without the supported exact tmux path. A fallback reports
**Copied** rather than **Pasted** or **In session**. On Wayland, copying additionally requires
the clipboard protocol described above; it can fail honestly when that protocol is unavailable.

With Zeca off, the talk key uses independent dictation even over the Nest or island; there is no
Chat draft to fill. Settings does not start a recording. An already active independent dictation
keeps its original owner when focus moves; returning focus never restores an insertion capability.

## Cancel by voice

**Settings > Dictation > Keyboard shortcuts > Cancel by voice** starts on and works on X11
and Wayland. End the recording normally; the final transcript is checked before spoken-code
conversion and before any clipboard write or insertion. These whole phrases cancel when said
alone or at the end of the transcript:

| Language | Cancel phrases |
| --- | --- |
| English | `cancel`, `never mind` |
| Portuguese | `cancela`, `deixa pra lá` (also without the accent) | <!-- check-english:allow -->
| Spanish | `cancela`, `déjalo` (also without the accent) | <!-- check-english:allow -->

Matching ignores case, extra whitespace, the phrases' acute accents and final sentence
punctuation. A trailing phrase must start at a whitespace boundary; `cancel the meeting at noon`,
`path/cancel`, `cancel_by_voice` and quoted `"cancel"` remain ordinary text. This is a literal
suffix rule: even `src slash cancel` cancels the turn. Turn the option off to dictate content
that ends with one of these phrases.

A match reports **Cancelled**, copies/inserts/submits nothing and adds no history or usage totals.
It leaves the previous successful dictation available for Copy again. Replay uses already delivered
text literally, so reusing `cancel` does not trigger this check. This applies only to independent
dictation; Chat's draft voice input keeps its existing behavior. The setting is saved locally and
captured when a recording starts, so a change does not affect the active turn. No continuous
spoken-command listener or model download is added.

## Your words processing

Independent dictation now applies the local Your words entries after final transcription.
Open **Dictation > Dictionary** to add/remove word chips and save replacement rules. Saving
the same replacement source updates its text without changing its order. **Snippets** adds short
triggers and literal bodies, including spaces and line breaks. Deletion asks for confirmation;
failed saves retain the draft and offer another attempt. These pages do not change Chat voice.

**Settings > Dictation > Your words > Help the model with these words** is off by default.
When enabled, only Turbo with fixed English receives the coding seed plus dictionary words,
limited to 224 model tokens with whole entries. English may be chosen
directly or resolved from the system language; automatic detection is excluded. Preview and final
transcription use the same recording snapshot. Replacement and snippet bodies never enter this
context, and it never persists in the cached model or Chat. A Base retry stays unprimed.
Base, Small, Portuguese, other languages and automatic detection receive no dictionary context:
only Turbo/English
both improved WER and kept silence/noise empty in the local synthetic comparison. These fixtures
do not establish human-speech accuracy; real microphone validation remains pending. The local
post-transcription rules below still work with every transcription language, independently of
model vocabulary. You do not need either vocabulary switch to use replacements or snippets.

**Settings > Dictation > Your words > Project vocabulary** is independently off by default.
When enabled, Vults caches names from known Git projects:
Cargo/npm packages and dependencies, shallow workspace members, module names, root file names
and the branch. It reads small manifests and file names locally; it does not read source bodies,
execute package scripts or download anything. Refresh happens on session start and the existing
repository refresh (at most once per minute on subsequent agent events).

The captured session's folder selects the working tree. Otherwise a terminal or recognized
editor title must name exactly one cached project; browsers, unknown and ambiguous titles add
nothing. This is best-effort foreground context, never permission to type. The first preview/final
decode freezes its names, including a missing context, for the whole recording. Project names
follow personal dictionary words under the same 224-token cap and Turbo/English gate. They never
become saved dictionary rules or alter the spelling pipeline. Other models/languages, Base retries
and Chat keep their existing behavior. This reuses V2.3's qualification; it does not establish
accuracy for every project name or replace real microphone validation.

**Try a sentence** in Dictionary runs the real saved local text rules. Changed spans are marked;
**Terminal context** includes spoken code if its Settings choice is on. Spoken cancellation also
uses the current saved choice and runs before your rules. It reads a fresh rules snapshot for
each test, never starts a microphone/model, changes the clipboard or stores a history entry.
Changing the sentence or rules discards an outdated result. This previews text processing,
not speech recognition or destination permission. **How it is applied** explains order and limits.

At recording start, Vults reads a consistent snapshot of the dictionary, replacement rules and
snippets. Edits made during that recording apply to the next one. If the snapshot cannot be read,
the recording fails explicitly before capture rather than silently ignoring the entries.

The order is: protect snippet triggers, apply replacements in their stored order, correct plain
dictionary spellings, convert spoken code for a recognized terminal/session, then restore each
snippet's literal body. Snippet bodies keep spaces, line breaks, dollar signs and punctuation;
no later rule or spoken-code conversion rewrites them. Text that looks like an internal marker
is still ordinary text. Replacements insert literal text, not regular-expression substitutions;
later replacement rules can match the preceding rule's result, followed by dictionary spelling.

Matching ignores case with Unicode case keys and requires whole-word boundaries. Between parts
of a phrase, spaces, underscores, hyphens and other non-word separators are interchangeable;
letters, digits and combining marks remain word content. Control characters, including tabs and
line breaks, are barriers and are never consumed by a match. Dictionary spellings apply only to
entries containing at least two letters/digits and otherwise only letters, digits, spaces,
hyphens or straight apostrophes. Other entries can still be used as optional model vocabulary.

For example, a replacement from `cloud code` to `Claude Code` also matches `CLOUD-CODE`; a
`Vults` dictionary entry fixes `vults` without changing `prevults`. A snippet body is never run
as code or submitted. Newlines or control-bearing output still follow the existing clipboard
fallback and terminal delivery restrictions.

Spoken cancellation is checked against the heard transcript **before** these rules. Expanding
a snippet to `cancel` therefore writes that literal text; saying `cancel` still discards before
expansion. The original heard transcript and final written text stay separate in History.
Copy again and History reuse the final text without applying the rules again. Correction figures
remain unavailable until their metrics phase. Processing stops with an explicit error above
256 KiB of input or intermediate output, with no partial copy or insertion.

## Check Your words locally

1. In **Dictionary**, add `Vults` and a replacement from `cloud code` to `Claude Code`.
   A duplicate word is rejected; saving the same replacement source updates its existing row.
2. In **Snippets**, add `signature` with this exact body:

   ```text
   Regards,
   Ada
   ```

3. Back in **Dictionary**, test `cloud code vults signature`. The result should contain
   `Claude Code Vults Regards,`, followed by `Ada` on the next line. Changed spans are marked;
   the saved snippet's line break stays literal. Test `source slash main` with **Terminal context**
   off, then on: it changes to `source/main` only if **Spoken code** is enabled in Settings.
4. With **Cancel by voice** enabled, test `cancel`: it reports cancellation before rules run.
   These tests do not copy, insert or add History. Delete a test entry, cancel its confirmation
   once, then confirm: only the explicit confirmation removes it.
5. For microphone acceptance, dictate into your usual external app and check the final text,
   the actual Copied/Pasted outcome and, when history is kept, the heard/written History pair. Keep the existing
   [destination restrictions](#supported-destinations); snippets never authorize submitting text.
   A rules change during capture affects the next recording; Copy again reuses the final text.

The first four checks test local processing without downloading or enabling a model feature.
If you also test Whisper vocabulary, use an installed Turbo with English chosen directly or
resolved from the system, and turn on only the context you want. Base/Small, Portuguese and
automatic detection keep their previous prompts even when the switches are on. Project names
require a known Git project cached after session start/repository refresh; unknown or ambiguous
foreground context adds nothing. Compare with the switches off; this is not a promise that a
name or a microphone recording will improve. Restore any preferences you changed for the test.

## Music while you speak

**Settings > Dictation > Voice > Music while you speak** offers **Lower to 25%** (the default),
**Pause it** and **Leave it**. The next independent recording keeps this choice until it stops,
including cancellation. Chat voice and future meeting capture do not use this control.

On Linux, Vults temporarily controls only MPRIS players that are already playing. Lowering caps
the player's own volume at 25%; muted or quieter players stay unchanged. Stopping capture
restores each player's original volume, or resumes players paused by this recording. A player
whose relevant state or volume changed meanwhile is left alone, even if it later returned to
25% or Paused. Crashed players and replacement processes are never resumed or restored.

This uses local media controls without reading track metadata or making network requests.
It works independently of **Now playing**, which controls the song shown in the app.
**Leave it** does not open a media-control connection. Unavailable players or the session bus
never prevent dictation, and restoring one failed player does not prevent restoring the others.
Restoration runs in the background; microphone capture does not wait for a media player.

## Spoken code

**Settings > Dictation > Terminals & sessions > Spoken code** starts on. For a recognized
terminal or session captured for the dictation, it joins identifier chunks around these terms:

| Language | Spoken terms for `.`, `/`, `_`, `-`, `:` |
| --- | --- |
| English | `dot`, `slash`, `underscore` or `underline`, `dash`, `colon` |
| Portuguese | `ponto`, `barra`, `underline` or `underscore`, `traço` or `traco`, `dois pontos` | <!-- check-english:allow -->
| Spanish | `punto`, `barra`, `guion bajo` or `guión bajo`, `guion` or `guión`, `dos puntos` | <!-- check-english:allow -->

For example, `src slash main dot rs` becomes `src/main.rs`. Existing case and punctuation
are preserved: no capitalization or ending period is added. A marker must have identifier-like
chunks on both sides. Whitespace outside a converted chain stays intact; line breaks, tabs and
control characters are never removed to create a chain. This is literal punctuation syntax,
not prose interpretation: turn it off when you want those words verbatim in a terminal.

Prose apps, unknown targets and focused Vults Chat keep their text. Changing the setting applies
to the next recording. A terminal's clipboard fallback keeps its converted text; formatting never
grants insertion, sends Enter or bypasses a waiting session. Copy again and History reuse the
already delivered text without formatting it again. When retained, History's **Show what was
heard** keeps the transcript before spoken-code conversion beside the written result. No audio
or previous clipboard contents are added to history.

## Opt-in X11 insertion

In **Settings > Dictation > Paste**, enable **Insert dictation in supported editors and tmux panes** only
if you want Vults to insert text. It starts off, including after upgrading. This first
implementation supports KWrite, GNOME Text Editor, Mousepad and Leafpad. Other apps and IDEs keep the clipboard path;
terminal delivery requires the exact tmux checks below. Native X11 is described below; Plasma Wayland uses the separate
keyboard grant in the next section. There is no XWayland input bypass.

On native X11, the shortcut callback captures a separate target before queuing recording work.
It checks the active window, exact keyboard-focus child, class, process and process start time,
then observes focus loss until delivery. Changing focus and returning also prevents insertion.
The asynchronous app context above never authorizes a paste. Immediately before Ctrl+V, Vults
rechecks this target, verifies the clipboard owner and requires released keys. It does not raise
an app, release your held modifiers or send Enter. A failed check leaves the transcript copied.
A terminal never gets Ctrl+Shift+V automatically; the tmux path below uses literal bytes.

Insertion temporarily owns the text clipboard. After dispatch, Vults waits about 700 ms and
restores the previous text only while this exact lease still owns it. A newer copy wins even
when its text equals the transcript. Overlapping dictations inherit the original text; an old
lease cannot restore over the new one. A clipboard-only fallback keeps the transcript available.
Previous clipboard contents stay in memory and are never written to history, logs or files.

The lease restores plain text only, up to 64 KiB; it does not restore images or rich formats.
Larger previous text or transcripts use the ordinary copy fallback. The clipboard history
exclusion hint depends on desktop support. Settings explains these limits before enabling
insertion. **Pasted** confirms dispatch to the validated foreground; review the result in your
editor. It does not confirm that the editor consumed, submitted or saved it.

## Plasma Wayland insertion

The same off-by-default insertion switch supports Plasma Wayland with `ext-data-control` and
one clipboard seat. Enabling it requests **keyboard input only** through the RemoteDesktop
portal. The desktop grants the ability to type into other apps; Vults limits its use to paste
in the supported editors above and exact tmux delivery below. It does not request pointer control or screen sharing.

Settings shows whether keyboard access is waiting, ready, temporary or unavailable. Later
dictations reuse the open session. A refusal, missing portal, failed input call or revoked grant
keeps dictation copying. It does not reopen a dialog on the next dictation; **Request keyboard
access** in Settings is the explicit retry. Other Wayland desktops keep the clipboard path.

A reusable restore token lives only in the OS keyring, under the app's own account. It is
removed before use and replaced by the portal's rotated token. Without a usable keyring, an
approved session works for this run without persisting a token. On restart, Vults attempts a
restore only when an existing token can be read and consumed; otherwise it waits for your
Settings request. A desktop can ask again if it no longer accepts a token.

Turning insertion off immediately invalidates pending deliveries, closes the portal session
and removes the saved token. If the keyring cannot remove it, insertion stays off and Settings
shows the failure with **Retry token removal**. Quit closes the session while retaining a usable
token for the next start. No token goes to settings, history, logs, the renderer or a file.

A read-only KWin monitor runs while the grant is active, before the talk key is pressed. The
press captures its current window identity and focus epoch. A compositor checkpoint before
delivery must match that identity, epoch, class, same-user process and process start time;
changing focus and returning also prevents insertion. A Vults window gaining focus invalidates
the captured destination. Missing or late checkpoints leave the text copied. This monitor keeps
no titles or key history and never activates a window. The tmux path uses this same window capability.

The Wayland text lease uses the same 64 KiB limit and in-memory retention as X11. Restoration
changes the bytes served by its own source, without claiming the selection again: a newer copy
wins even if its cancellation event has not arrived yet. The history exclusion hint is desktop
dependent. Multiple seats or missing `ext-data-control` yield ordinary copying. A successful
validated Ctrl+V dispatch shows **Pasted**, with the same limits described above.

Capture is capped at ten minutes and stays in memory. Quiet or very short audio produces no text.
With **Retry empty dictation with Base** enabled (the default), if the selected model returns
no text but Silero heard speech, an already installed Base model can retry once, with visible
feedback; Base is not downloaded for this retry. Successful independent
dictations now update the local History page and usage totals. The last successful text also stays
in memory for Copy again until voice is turned off,
the screen locks or Vults exits. Copied text may remain in the system clipboard or its manager.

## Exact tmux session delivery

The insertion switch also supports a flock session in **tmux inside XTerm/UXTerm or Kitty**.
Kitty needs the `kitten` CLI and must already allow read-only queries through its same-user Unix socket;
Vults does not edit its configuration or enable remote control. Other terminal hosts, missing
socket/pane metadata, shared or ambiguous clients and direct Kitty/WezTerm delivery stay copy-only.
Kitty's [send-text implementation](https://github.com/kovidgoyal/kitty/blob/master/kitty/rc/send_text.py)
does not acknowledge delivery, so its success status is insufficient. WezTerm's
[copy/search overlay](https://github.com/wezterm/wezterm/blob/main/wezterm-gui/src/overlay/copy.rs)
shares the underlying pane id; its CLI cannot establish which input field owns that pane.
Direct delivery therefore remains a clipboard fallback. **Open terminal** can navigate to the
hook's exact WezTerm instance, but that does not authorize typing into it.

The talk press captures the exact active tmux pane, attached client and agent process. A same-user
socket, process ancestry and start times tie them to the captured window. Kitty additionally
reports that exact focused pane. Its child process ancestry, shared PTY and foreground process
group must identify the tmux client. Before delivery, Vults checks them again,
checks the window capability, then asks core for the session's **current** state. Permissions,
questions and any pending card, including an explicitly deferred card, keep the text copied.
The status points you to the card or terminal; answering it does not release queued dictation.
A changed session, process, pane, focus or grant also falls back to copying.

Delivery sends literal UTF-8 bytes with tmux's hex-key path and waits for a server acknowledgement.
The server rechecks the attached client, pane process, active window/pane, copy mode and liveness in its command
queue. No transcript is interpolated into a shell or tmux command, and no Enter is sent. Only a
single line up to 8 KiB with no control characters is supported; multiline or control-bearing
text is copied instead. Copy mode also prevents delivery. **In session** means that this transport
accepted the insertion, not that the agent submitted or executed it. Successful session delivery
does not touch the clipboard; **Copy again** remains an explicit clipboard-only action.

Plasma Wayland requires the existing keyboard grant even for this path; a refusal or revocation
never falls through to terminal IPC. Focused Vults Chat continues to fill its current unsent draft.
The former Lab/Quick task-run composers were removed in N0 and are not recreated.

## Coming in N3

History, dictation gesture/input/processing choices, measured feedback, captions, spoken code,
spoken cancellation and music control are available in this build. Chat keeps its one-minute
recording limit; independent dictation keeps up to ten minutes. Local Your words processing is
connected with Dictionary/Snippets editing, vocabulary controls and sentence preview. Activity's Dictation
statistics show measured usage, and Settings > Dictation includes Diagnostics. Styles,
Meetings and voice commands remain later steps. No placeholder page runs a simulated recording or model request.

Model clean-up, meeting summaries and realtime voice remain separate, off-by-default features.
Ordinary transcription stays local. Dictation never answers an agent's permission; see
[the insertion rules](../adr/0032-dictation-into-other-apps.md) and
[voice and recording rules](../adr/0028-spoken-replies-through-a-realtime-model.md).

## Dictation history

Open **Dictation** in the Nest header, or choose **Open Dictation** in the palette. It remains
available with Zeca off. The page groups the newest 500 retained dictations by local day and
searches their heard/written text, application and session without another database read.
Text and long values wrap within the scrolling history column. **Show what was heard** expands
the original transcript. It can differ from the written text when Your words or spoken-code
conversion applies.

Today's figures use measured audio duration and committed word totals. No recorded day shows
**Unknown**, rather than a measured zero. Speed is unknown below one second of audio. Estimated
time saved compares dictation time against typing at 45 words per minute, floored at zero.
Unknown correction counts stay absent. These figures survive deleting transcript rows.

Retained deliveries also save local stage timings in milliseconds: transcription (including model
loading and any Base retry), text processing and confirmed delivery. The total starts when you
release the held key, press to finish hands-free recording or reach the capture limit; it includes
the microphone tail and time waiting for the worker. Delivery ends when copying or native dispatch
returns; it does not measure the destination app's rendering. The history API provides per-stage p50/p95
using nearest ranks over the newest 50 retained deliveries, ignoring unknown measurements.
Old entries and the unused clean-up stage stay unknown. Cancellation, failure, empty results and
replay add no samples. Erasure, expiry and **Do not keep** remove these timings with their rows;
daily/app usage totals stay independent. [Activity > Dictation](activity.md#dictation-statistics)
shows usage figures; **Settings > Dictation > Diagnostics** shows the stage timings.

**Copy** uses the clipboard with success/failure feedback. **Use again** asks the existing
Copy again worker to copy that retained entry and remember it as the last text. Follow the
island's actual delivery status, then return to your destination to paste. It never types into
the Nest search field, sends Enter, reuses an old window capability or counts another dictation.
An active recording/transcription rejects this action; a deleted or expired entry cannot replay.

**Delete…** and **Erase dictation history…** require their own confirmation. Erasure clears
cached transcripts even if the next read fails. Clearing all history or changing retention
invalidates earlier in-flight history writes; Copy again's existing in-memory lifetime stays
separate from retained history. Audio and the previous clipboard are never written to history.

**Dictation settings** leads to **Settings > Dictation**. Its Keyboard shortcuts, Voice, Paste,
Terminals & sessions, Your words and Privacy groups own the existing controls; General and Chat & voice link here. Model and
language changes remain shared in **Chat & voice**. **Privacy > Keep dictation history** offers
the retention choice. Choose **Do not keep**, **7 days**, **30 days**
(the default), **1 year** or **Always**. Shortening retention immediately erases older text;
opening history and recording also prune it. **Do not keep** writes no transcript or session to
the history, while measured daily/app totals still update. Dictionary/Snippets are working
pages. **Statistics** opens Activity > Dictation; Styles, meetings and voice commands remain later steps.
**Privacy > Erase dictation
history…** uses the same confirmed erasure as the work page, with explicit failure and retry.
Recording mode, microphone, repetition removal, Base retry, spoken code, spoken cancellation
music and vocabulary choices are working controls. Additional delivery options remain
later steps. Active recordings retain their captured choices.

## Diagnostics

Open **Settings > Dictation > Diagnostics** at the end of the page. **Refresh diagnostics** reads
current audio, desktop and local model state without starting capture or requesting a desktop grant.
The selected input uses dictation's normal missing-device fallback; its name and **native input rate**
refer to the device format, before resampling to 16 kHz. An unavailable input remains unknown.
GlobalShortcuts readiness means the talk binding is currently registered. RemoteDesktop reports the
existing keyboard grant; KWin reports whether that grant owns its verified focus script. Neither is
needed for X11 insertion. GPU availability does not establish transcription speed, and VAD readiness
only establishes that its local model is installed.

**Test microphone** opens the selected input for six seconds. Stay quiet for the first two seconds,
then speak for four. Nine speech bands show the live signal. The result estimates the initial quiet
noise floor in **dBFS**, a digital level rather than acoustic room loudness, and flags weak/missing
signal, excessive background level or clipping. Digital silence is floored at -120 dBFS. **Stop test**,
leaving the page/window, locking the screen or beginning Chat/dictation stops the test. Loss of contact
with the Settings page also ends capture. A new test waits for the previous capture to close.
Audio is discarded from memory; this test does not transcribe, copy, insert, save history or fetch models.

The counters count repetitive runs collapsed during final dictation decodes, installed-Base retries
and confirmed **Copied** outcomes since Vults started. Preview and Chat do not add counts; **Copy again**
can add a Copied outcome. These counters are memory-only and reset on exit. The stage table shows the
retained-delivery sample count and p50/p95 in milliseconds; an unmeasured stage is **Unknown**. A failed
refresh preserves the previous readings with a warning. Failed timing reads remain distinct from an
empty timing history, where each stage has zero samples and unknown durations.

![Dictation Diagnostics in the native app](../assets/dictation-diagnostics.png)

Isolated native validation example: unavailable input and shortcuts are reported explicitly.

The timing summary combines the newest retained deliveries, even when model, language or destination
changed. **p50** is the middle observed duration; **p95** is the observed duration at the 95th-percentile
rank, with no interpolation. The total includes the tail and worker queue; the other stages need not
sum to it. A Copied outcome measures clipboard delivery, not insertion into another app. GPU **Ready**
and a few quick samples do not establish a latency guarantee for a particular model or destination.

## Check statistics and Diagnostics locally

1. In **Settings > Dictation > Diagnostics**, use **Refresh diagnostics**. Confirm that it does not
   record or ask for a desktop grant. An unavailable probe should stay unavailable/unknown; X11 does
   not need RemoteDesktop or the KWin focus script.
2. Choose **Test microphone**, stay quiet for two seconds, then speak. Check the bands, noise estimate
   and verdict. Start another test and use **Stop test**, then try leaving the page during a test.
   Capture should end without adding a transcript, changing the clipboard or adding usage/timing samples.
3. Dictate one short sentence into your usual app. Confirm its actual **Copied**, **Pasted** or
   **In session** result and its final text. In **Activity > Dictation**, check the new words, daily
   bar and application attribution. A destination that could not be identified belongs to Unknown.
4. Return to Diagnostics and refresh. A retained successful dictation should add a timing sample;
   unused model clean-up stays Unknown. **Copy again** should leave usage and timing samples unchanged;
   a successful copy may increment the separate Copied counter. Cancelling a new dictation adds no samples.
5. If you retained a disposable test transcript, delete just that entry from History. Its timing sample
   disappears after refresh while usage totals remain. With **Do not keep**, subsequent successful
   dictations add usage but no retained timing samples. Choose retention deliberately; shortening it
   also removes older retained text immediately.
6. Review the pages in Black and Light, at your usual size and in a short, narrow window. Reach Stop,
   Refresh, period selection and the stage table by keyboard. Switch Room/Full and reopen the Nest;
   the statistics period should remain selected. Restart Vults: usage remains, retained timings follow
   their text, and the in-memory counters reset.

When reporting a problem, include the app version, desktop/session and scale, selected microphone,
model/language, actual delivery outcome, the relevant timing sample counts and the steps that failed.
There is no need to share the dictated text. These checks cover the current V4 screens; voice commands,
Styles, Meetings and realtime voice are still separate development steps.
