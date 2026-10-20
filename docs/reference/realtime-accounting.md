# Realtime accounting

V6.3.2 supplies an offline protocol contract in `chat::realtime`. It opens no connection, reads no
key and captures no audio. V6.3.3 adds the gated backend described below; voice remains off by default.
A prepared message grants
no network authority: the app must first reserve its ceiling through the monetary gate.

## Reviewed rates

Qualified on **2026-10-19**, in USD per million tokens:

| Model | Text input / cached / output | Audio input / cached / output |
|---|---|---|
| `gpt-realtime-2.1` | 4 / 0.40 / 24 | 32 / 0.40 / 64 |
| `gpt-realtime-2.1-mini` | 0.60 / 0.06 / 2.40 | 10 / 0.30 / 20 |

Both model pages list a 128,000-token context. These are the only recognized identifiers; no
alias-prefix matching or automatic substitution is allowed. Sources: [Realtime 2.1](https://developers.openai.com/api/docs/models/gpt-realtime-2.1)
and [Realtime 2.1 Mini](https://developers.openai.com/api/docs/models/gpt-realtime-2.1-mini).
The local qualification expires after 30 days, at 2026-11-18 00:00 UTC; dates before qualification
also refuse quotes. This is a local recheck requirement, not a provider price guarantee. Revalidate
rates and model limits before runtime activation and whenever renewing the qualification.

## Reservation bound

The session explicitly sets a finite output cap (1-4,096 tokens) and a positive
`truncation.token_limits.post_instructions` cap no larger than context minus output. The server must
confirm the effective configuration before any response can be prepared. The API's truncation cap
excludes instructions, so it cannot stand alone as a total-input estimate.
[Client-event reference](https://developers.openai.com/api/reference/resources/realtime/client-events).

The conservative reservation is:

```
context * text_input_rate
  + post_instructions_cap * audio_input_rate
  + output_cap * max(text_output_rate, audio_output_rate)
```

This deliberately covers every context token as text, plus the entire bounded conversation as
audio. It includes instructions, token overhead and previous turns without guessing tokens from
milliseconds. It assumes the provider honors the confirmed context/truncation/output limits and
that audio exists only in the bounded conversation. All instructions are text; tools and remote
input transcription are explicitly disabled. Server turn detection is null: response creation is
manual, after admission. A changed effective configuration closes the owner.

For an 8,192-token conversation cap and 1,024 output tokens, the ceilings are US$0.839680 and
US$0.179200 respectively. These are held amounts, not predicted charges. Actual cost uses the
reported text/audio/cache breakdown, checked integer nano-USD arithmetic and one final upward
rounding to micro-USD. Realtime bills conversation context again on later responses; input
transcription has separate billing. [Cost guide](https://developers.openai.com/api/docs/guides/voice-latency-cost).

## Completion and failure

The owner binds one session, one pending attempt and its server response ID. Parallel preparation,
unexpected creation, mismatched audio deltas, stale completion, duplicate completion and malformed
or oversized events cannot settle another attempt. Attempts cannot be reused, with a maximum of
1,024 attempts per owner to bound replay memory. Incoming server events are limited to 256 KiB.

`response.done` produces one receipt. Missing, inconsistent or newly shaped usage leaves cost
unknown and closes the owner. Unknown cost must retain the full hold. Known overruns are returned
in full and also close the owner. Request success is independent of billing availability; a cancelled
response can still cost money. [Server-event reference](https://developers.openai.com/api/reference/resources/realtime/server-events).

The synthetic two-response cassette covers cached context and billable cancellation; it is not a
live provider recording. Run `cargo test -p vults-chat realtime::` offline.

## Runtime transport

V6.3.3 adds an OpenAI implementation of the provider boundary, with a fixed
`wss://api.openai.com/v1/realtime` origin, native TLS, no proxy, redirects or automatic retries.
Monetary admission precedes keyring lookup and connection; every later turn reserves again before
sending PCM. The key stays in Rust and the Authorization header is sensitive; transport failures
return content-free codes. See the [official WebSocket guide](https://developers.openai.com/api/docs/guides/voice-websockets).

Settings version 41 adds independent `voice_realtime` consent, a retained ledger identity, a default
US$1 rolling limit and an explicit reviewed model. Zero disables remote voice. Only the Settings
webview can save this choice. V6.7.1 adds typed bridge calls for that explicit save and Settings-only
readback (`voice_realtime_settings`). V6.7.2 exposes these choices under **Dictation > Talk with
Zeca**. V6.7.3 adds the separate Nest-owned manual controls described below.
The returned `on` is the saved choice, not a live-session or readiness indicator.
Readback returns only `on`, `model` and `limitMicrousd`, with the amount encoded as a decimal string
so large persisted limits survive JavaScript exactly. It omits the private ledger identity and
performs no keyring, device, store or network IO. V6.7.2 also sends write amounts as canonical
decimal strings, preserving the full supported integer range without floating-point conversion.
The native command rejects noncanonical or out-of-range amounts before persistence; authority,
model and budget checks still apply. The form accepts USD with a decimal point or comma and up to
six decimal places, confirms saves by reading back, preserves failed drafts and blocks stale edits
until an explicit reload when another writer changes the saved choice.
The native backend accepts bounded command/output channels and never starts capture by itself.
One desktop voice lease owns the connection. Epoch revocation covers choice changes, lock, release
and permanent shutdown; clearing/rotating the shared OpenAI key also revokes existing connections.
OFF/ON or lock/unlock cannot renew an old owner. Gate time uses one process
monotonic anchor, and a missing/replaced ledger cannot renew consent.

The persistent socket accepts 24 kHz mono PCM16 in 100 ms chunks. Input is at most 30 seconds per
turn, output at most 120 seconds, with a 120-second absolute turn deadline and 10-second IO/setup/
settlement deadlines. Server frames/messages are capped at 256 KiB. Output queue saturation, malformed
PCM, expired rates or exhausted allowance closes the transport. Cancellation/revocation interrupts
pending IO; interruption requests response cancellation and waits for final usage within a bounded
deadline. No more audio is sent after admission fails. Ready/audio publication rechecks ownership
after awaited work. V6.4.2 binds native playback and transcript routing below; capture/UI remain V6.7 work.

Known final receipts are settled once, even after revocation, without a competing unknown-cost
retry. Before any inference could have started, an interrupted connection releases its hold at proven zero
cost. Once inference may have started, missing usage retains the whole ceiling. A detached admission
that was never handed to transport settles at zero and releases its hold through its ownership guard. Task abort/crash after
handoff keeps the hold unresolved. Within the same owner, unresolved work still blocks another
reservation. History clearing never resets this accounting.

## Orphan recovery

V6.3.4 adds Linux-only recovery before a new native owner can reserve. The gate acquires a
nonblocking exclusive kernel lock on the data directory itself, so there is no removable lock file.
It keeps that lock throughout the borrowed transport future and every detached admission, keyring
and settlement worker. A second owner/process receives `Busy` before touching orphan accounting;
task cancellation cannot release a worker's remaining lock reference. The last intentional owner
explicitly unlocks, so an unrelated fork/exec child's inherited descriptor cannot prolong ownership.
The standard library uses Linux `flock`; see [File::try_lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)
and the [Linux lock lifetime](https://man7.org/linux/man-pages/man2/flock.2.html).

Only after acquiring ownership does one ledger-identity-bound transaction close unfinished
`voice-realtime` holds. Unknown actual cost stays null and the full reserved ceiling remains
charged. Its rolling 24-hour window starts at reconciliation, never before the original start time.
Repeated restarts do not move a completed hold's expiry. Existing traces gain an error outcome and
content-free reconciliation marker; erased traces stay erased. Other features and already settled
receipts are untouched. Identity/schema/lock/write failures deny entry; a failed transaction changes
neither holds nor traces. The setup wait is bounded to 10 seconds; any detached worker still retains
ownership until it finishes.

This proves that the former local transport can no longer send; it does not claim the provider
completed at disconnect or refund uncertain work. A crashed process releases its kernel lock;
aborted transports drop their socket. Recovery is not enabled on other platforms. The existing
V6.3.5 backend initially had no user-facing capture entry. Native retention/recovery is described
below; V6.7.3 adds manual controls, while live-provider and physical acceptance remain open.

## Native turn retention and local text

V6.3.5 changes the native app service to accept complete `realtime::Turn` values, independently of
remote admission. Each value owns 100 ms to 30 seconds of mono 24 kHz PCM16 in memory. One pending
turn is fed into the persistent transport in 100 ms chunks through a two-command queue; a four-event
queue relays output. Raw audio emission yields so buffered socket bursts cannot starve this inline
consumer. Only a successful final receipt releases the pending original; queued receipts are drained
before applying a simultaneous transport failure. A later idle failure cannot recover an older turn.

V6.7.3 adds `Turn::from_capture` for a completed native mono 16 kHz float capture and its finalized
local transcript. It reuses the shared stateful resampler to produce 24 kHz PCM16, preserving the
sample clock with the exact integer duration ratio. Input is bounded to 100 ms-30 seconds before
conversion; non-finite audio and invalid transcripts are rejected, and finite filter peaks saturate
to PCM16 without wrapping. The source audio is not modified. Text keeps its original whitespace,
with the existing 4,096-byte native-command bound. This adapter opens no microphone, loads no
model and grants no capture/upload authority. The explicit native capture owner is described below;
conversation controls, the whole milestone and physical/live acceptance remain open.

Off/spent admission, missing keys, transport/protocol/response failures or saturated output retain
the failed turn for local recovery. The transport gets up to 11 seconds to finish its existing
bounded settlement path, then its future/socket is dropped before recovery. If this same turn
already carries a finalized local transcript, recovery returns those exact words without a second
decode or model load, after checking cancellation/revocation and the consumed-turn flag. Otherwise,
the selected installed Whisper model and language are snapshotted; the existing resampler converts
the retained audio to 16 kHz. No download, model clean-up, other provider or automatic Chat send is
started. A successful local decode returns the whole text, preserving whitespace, for the native
caller to offer as a
reviewable draft. V6.7.3 offers those words for editing/copying, without an automatic Chat send.

Missing models, decode failure, empty output, invalid/oversized text or the 30-second local wait
deadline return the original in-memory turn with a typed reason. Very short or trimmed-to-short
turns may yield no local words: Whisper's existing minimum is 500 ms, even though the remote input
minimum is 100 ms. Audio and transcript content are excluded from Debug output and new traces;
this service writes neither audio nor text history.

Cancellation, lock, shutdown, shared-key changes or changed voice/model/language/microphone/Zeca
settings discard work instead of offering stale recovery. The shared settings mutation revokes the
epoch under its lock, so restoring a previous choice cannot revive an old turn. The decoder retains
the native lease through the final publication check; a detached worker also retains its own lease
until native work exits. The wait can expire before an unabortable cold model load finishes; the
stop flag suppresses late output and new voice owners stay Busy until that worker releases its lease.
This retention adapter adds no microphone, playback, renderer command or permission authority.

Run `cargo test -p vults-chat realtime::transport::` and `cargo test -p vults-app realtime` offline.
Duplex WebSocket tests exercise actual frames with synthetic PCM/keys, two turns, cancellation,
revocation, failed settlement, backpressure and unknown usage. They open no listener or provider
connection. No paid request, real microphone/playback or live speech acceptance is claimed. The
native driver/decoder tests use synthetic PCM and local work doubles; real model/speech quality
and capture/playback acceptance remain separate. Backend failures provide structured reasons and
never silently send to a different provider.


## Native first-turn capture

V6.7.3 adds a Linux-only `run_captured` entry with one native `CaptureControl`/`CaptureRequest`
pair. Creating controls opens nothing; the explicit native call acquires the existing desktop
lease before capture. The Nest work page now exposes the explicit controls below; no shortcut
is assigned to this entry.
The configured consent must be enabled and the selected local Whisper model must already be installed.
The shared microphone preference and existing ordinary-capture question fence are reused.

Finish closes and joins the microphone before local transcription and bounded turn assembly.
Recording lasts at most 30 seconds; reaching the cap discards it instead of automatically decoding
or uploading. The total native capture/decode wait is 60 seconds. A cold model load can outlive
that wait, but retains the same desktop lease until actual exit and cannot publish late output.
Cancel or losing the controller revokes the whole session, including after Finish; the controller
must remain alive through the response. Lock, shutdown, settings/key changes and consumer loss also
stop admission. Native phase events contain only Starting, Recording or Transcribing, never samples
or words. Keep the controller until the session ends, then drop it.

The prepared turn enters the existing driver's current generation, local-command gate and guarded
provider path without a renderer audio round trip. Local commands consumed there never open a
provider; recovery reuses the exact local transcript. This entry disables barge-in and adds no
automatic capture, model download, Chat send or permission-answer authority. Conversation controls
and reviewable recovery in the work surface are described below; V6.7.3 remains open. Tests use fake
microphones/decoders and synthetic PCM, without physical capture or paid requests.


## Nest manual controls

The **Dictation > Work > Voice conversation** page is the only renderer entry for the manual owner.
`voice_conversation_start`, `voice_conversation_status` and `voice_conversation_control` require
Nest authority; Settings, island and widget cannot call them. They expose neither PCM, raw errors,
credentials nor settings writes. Start additionally requires the visible, unlocked Dictation work
surface, Zeca and saved remote consent. A native identity binds Finish, Cancel, heartbeat and late
results to this session. A foreign/stale identity cannot control or publish into a newer turn.

The page polls every 500 ms while work is active. Three seconds without contact drops the controller;
leaving the page, changing tabs/webviews, hiding or destroying the Nest also cancels it. Lock,
shutdown and relevant settings/key changes discard the slot and revoke its native work. Stop is
forwarded through the existing owner to retain native/provider teardown and settlement. It cannot
retract a request already sent. No permission-answer path is added.

One Start admits one completed capture. The input channel is closed after that seed, so local
completion or settled playback finishes the owner instead of silently opening another microphone.
The UI shows starting, recording, transcribing, responding and terminal status; responding is not
proof of audible playback. Only heard-history text or the local-command reply is shown, bounded in
memory. Recovery text is editable and copied explicitly; edits survive page navigation locally.
A newly acknowledged Start replaces the previous draft. There is no automatic Chat send, replay,
persistent conversation history, automatic end-of-turn detector or barge-in in this entry.
Native layout validation uses controlled owners without microphones, model loads or paid requests;
physical speech and live-provider acceptance, V6.7.4 island feedback and K8 completion remain open.


## Playback and heard text

V6.4.1 adds an independent `audio::playback::Playback` primitive. It opens only the default output,
with one native worker per process, and accepts mono 24 kHz PCM16 chunks of at most one second,
up to 120 seconds per response. Setup has a three-second deadline; an owner lasts at most 150
seconds. Installed CPAL and the existing stateful resampler support 8/16/24/44.1/48/96 kHz output,
F32/I16/U16 and one to eight channels. Supported devices use a preferred 10 ms callback period.
There is no dependency, microphone, file, network, Settings or renderer addition.

The cursor separates accepted source frames, frames submitted to the device and estimated frames
consumed. The estimate maps CPAL's predicted playback timestamp into the monotonic clock and advances
only submitted audio, excluding inserted underrun silence. It is not acoustic proof: device mute,
hardware buffering and speaker audibility still need desktop acceptance. Invalid clocks, excessive
latency, device errors and bounded-queue failures stop playback. The callback uses a nonblocking
queue lock; contention emits silence without advancing submitted content. Conversion runs on the
producer, outside the callback.

Producer `finish()` flushes conversion and enters Draining. Only an empty PCM queue plus elapsed
scheduled audio enters Finished; a temporary empty queue is not completion. `stop()`, revocation
and Drop freeze consumption and clear pending output. Samples already submitted to the driver may
still be audible after stop; the frozen cursor intentionally excludes that uncertain tail. Device
teardown stays on the worker, with exclusive ownership held until it exits, including a blocked
native setup call. The deadline cannot abort an uncooperative driver call or permit a second worker.

`voice::turn::heard::split` accepts finalized, ordered text segments with 24 kHz frame extents and
the frozen source cursor. Whole played segments are retained; an in-flight segment keeps only the
floor of its proportional word count, on UTF-8 word boundaries. This is an estimate, not exact
word/audio alignment. The API itself does not provide precise transcript alignment after a client
interrupts playback; WebSocket clients own playback and server-item truncation.
[Realtime conversation guide](https://developers.openai.com/api/docs/guides/realtime-conversations).

The pure result preserves the original prefix and unheard complement, computes the model-context
replacement `heard prefix [interrupted]` and exposes the unheard tail once when at least three
whitespace-separated words remain. Explicit interruption still marks a producer stopped after its
received audio drained. Empty/no-played text never becomes a full heard response. The hint makes no
request, repeats no tool and grants no permission. It is not a localized UI label.

V6.4.1 supplies the independent primitive; V6.4.2 adds its native app owner below. The primitive
must remain bound to that consent/revocation owner. V6.5-V6.7 retain echo cancellation, tools,
capture and visible conversation work; the conversation microphone remains unavailable.

Offline tests cover consumption/latency, underrun, drain, stop, invalid clocks, cancellation,
resampling and word/history math. An explicit child-process ALSA null test exercises actual CPAL
setup, conversion, callback, drain and destruction without using desktop speakers. It establishes
native plumbing only; real speaker/headphone timing, interruption, speech quality and live-provider
acceptance remain open. Run the null test only with a private ALSA configuration containing
`pcm.!default { type null }`, `ALSA_CONFIG_PATH` pointing to it and `VULTS_PLAYBACK_NULL_TEST=1`:
`cargo test -p vults-audio native_null_output -- --ignored`.


## Native playback and context repair

V6.4.2 connects the existing native complete-turn service to CPAL. The first authorized complete
turn starts the gated connection; only confirmed remote Ready opens a quiet output worker, and
input is uploaded only after the output reports Playing. Device failure retains the pending input
for the existing local text recovery. A transient teardown Busy waits at most three seconds. Every
output worker retains the native session lease through device destruction, including detached or
blocked setup, so abort/revocation cannot overlap a new session with old native work.

Native commands and speech/probe signals carry the exact turn generation. Stale commands are
discarded; voice interruption needs a matching confirmed probe. Explicit Stop response is bound
to the audible reply. Notifications of Listen/Probe actions
grant no microphone authority and do not start capture. First consumed output drives the lock;
producer completion requests drain, and confirmed drain drives tail grace. The ordinary entry keeps
voice barge-in disabled; only explicit V6.5 headphone/AEC routes permit it. Scripted headphone cases exercise the available
owner path without asserting that speaker interruption is safe.

The spoken response path accepts one assistant audio item/content part at index zero per response.
Attempt, response and item IDs are bound across PCM, transcript, final output and history; response
and item IDs cannot be reused in the bounded session. Changed IDs, another content part, mixed tool/audio output,
text-only output, duplicate audio/transcript endings, malformed text and missing/mismatched final transcripts close
the owner. Text is bounded to 64 KiB and audio to 120 seconds. A finalized transcript covers the
whole received audio extent as one estimated segment; the provider supplies no precise word clocks.
Billing settlement still uses the exact final receipt even if later output validation fails.

On confirmed interruption, the cursor freezes once and is floored to integer source milliseconds.
Further matching PCM can be accounted for but is not played. An active producer receives a targeted
`response.cancel`; after final receipt/settlement the exact item receives
`conversation.item.truncate`. An already completed producer is not cancelled again. Only a matching
item/content index/audio-end acknowledgement readies the connection. Wrong, duplicate or missing
acknowledgement closes it without refunding settled usage or admitting another response.
[Client-event reference](https://developers.openai.com/api/reference/resources/realtime/client-events).

The native owner replaces only its matching in-memory assistant record with the estimated heard
prefix plus `[interrupted]`. It emits an identity-bound History record after drain or acknowledged
interruption; it does not write the Chat database or persist audio/transcripts. A tail of at least
three words becomes a bounded, quoted repair hint, consumed only in the next independently admitted
`response.create`. It never generates a turn alone, retries an action or enables tools. Existing
persona instructions remain present; the hint is conversation data, not authority.

If the socket fails after a successful settled response, valid buffered output can still drain.
Speech/probe signals remain active during that drain. If interrupted then, or if a truncation
acknowledgement fails after settlement, matching heard history is emitted with
`server_synchronized=false`, then the terminal error ends the owner. Such a record cannot authorize
continuation. Cancellation, lock, changed choices and closed output suppress stale publication and
recovery; they stop output before bounded transport cleanup. A completed input is never recovered
or replayed as a new user turn. Callers retain any unaccepted capture until its Generate notification.

V6.5.2 adds a separate Linux `run_routed` entry, guarded before native capture by the existing
Settings consent, cancellation and epoch authority. It pins both native streams to an explicitly
selected AEC pair or devices, retains its lease through teardown and emits bounded memory-only
Input events to the native caller. Complete turns still require explicit native admission; capture is
never uploaded automatically. Explicit headphones/AEC policy stays separate from temporary route
verification; an unverified route clears retained speech/probes before deferred playback events.
The ordinary CPAL entry keeps voice barge-in disabled. No routed renderer command or duplex
voice-conversation UI is exposed. Backend V6.4/V6.5 is delivered; see [Echo cancellation](echo-cancellation.md) for routing,
clock and private-graph evidence. Next is V6.6 tools, then V6.7 capture/UI. Physical-device/acoustic
timing, real speech and live-provider acceptance remain open before activation.
V6.7.3 adds the native routed interruption adapter: checked local model/VAD availability before
stream creation, a bounded eight-second ring, fresh post-playback VAD evidence and generation/probe
bound local scratch transcription. Its words only confirm/reject a probe; they cannot call tools,
answer permissions or create a user turn. A single worker holds the desktop lease until actual
inference/model teardown exits. Cancellation, route loss, stale work and deadlines suppress late
results. On confirmation, a typed memory-only Replay event follows the new Listen action, carrying
the retained onset/pre-roll under the new generation. Generation-bound native Capture operations now
assemble a following turn from that same stream: Begin takes the replay plus subsequent PCM, Finish
alone starts final local decoding and submits the complete turn, and Cancel discards it. The original
24 kHz audio remains bounded to 100 ms through 30 seconds; it is not resampled back from the local
16 kHz decode. One final decode at a time, a 60-second deadline, live route/epoch checks and worker-held
leases suppress stale results without blocking cancellation. The existing local-command router runs
before provider admission. Replay/probe evidence never commits by itself. No routed UI switch or
phone/card authority is added. Existing truncation acknowledgements, heard-prefix history and
next-turn repair still own interruption and gate subsequent uploads. Physical/live validation,
explicit route selection/consent UI and routed control wiring remain open.

Offline socket/native-owner tests cover interruption before/after producer completion, exact
acknowledgements, stale generations, repair admission, failure, revocation, abort and recovery.
The isolated ALSA null setup above also runs the complete native driver with a synthetic provider:
`cargo test -p vults-app native_null_conversation -- --ignored`.
This check exercises actual CPAL into memory history without a microphone, desktop speaker, key or
provider connection. It is not real speech or live-provider acceptance.


## Local commands before realtime upload

V6.6.1 adds Linux command admission to the native owner. `Turn::with_transcript` binds a finalized
local transcript to the same captured PCM, with a 4 KiB limit and no control characters except
whitespace. Its Debug output reports lengths only. The native capture owner is responsible for
that local transcription; V6.7 still owns capture/VAD/turn assembly and visible controls. A missing
transcript fails before provider construction and may return the existing reviewable local-text
recovery. It never sends unclassified audio remotely or executes recovered text automatically.

The runtime's existing state owner receives a cancellable request, checks the same conversation
consent/epoch and command switch under Settings, and reuses V8's shadow-state dispatcher. Each
conversation has its own bounded 90-second repeat/undo/clarification context, separate from independent
dictation. The command switch also revokes existing conversation epochs when changed, including a
later re-enable. Runtime admission checks a fresh absolute three-second deadline after taking locks;
expired, cancelled, consumed or closed-waiter requests cannot change the flock. The native waiter
is polled for cancellation/output closure and drops pending admission on timeout or abort.

Only an unrecognized NotCommand result may proceed to the realtime transport. Recognized commands,
refused card answers, disabled/unavailable actions and clarification questions all stay local.
Reads reuse the existing redacted cached flock formatter; no connector fetch is triggered. Effects
use the existing runtime executor and the exhaustive voice-only Intent allowlist. V6.6.3 adds the
native receipts below: only acknowledged completion may say Command completed; launch submission
remains Command requested. A local result emits an identity-bound Local
event, finishes without Generate and retires that input generation. A queued duplicate cannot run
again or upload. A handled input never becomes automatic model/Chat fallback after an error.

Scripted owner tests cover local input before lazy provider construction, local follow-up on an
established connection, stale/duplicate generations and cancellation during admission. Dispatcher
checks cover permission refusal, command disablement, ambiguity, separate context, localized cached
reads, expired admission and once-only mutation. These tests use no microphone, desktop action,
key or provider. V6.6.2 adds the model-side flock tools below; consultation remains V6.6.3. Full
V6.6, renderer activation and real speech/provider acceptance are still open.


## Native command receipts

The completion slice of V6.6.3 uses the existing executor with one receipt per effect. A core-only
change completes at the state commit; project persistence completes only after the Settings save
succeeds. Editor process creation, file-manager handoff and window-activation submission remain
Requested: these APIs cannot prove the requested window became visible or that its launcher later
succeeded. A known failure reports that some changes may already have happened, rather than claiming
an all-or-nothing rollback. Mixed cached-read/action batches wait for every effect and preserve the
cached answer only after all receipts report completed; submission, failure or timeout replaces it
with the corresponding status. No model response can upgrade a submitted or unknown action to Done.

The native owner supplies one absolute three-second deadline. Admission/receipts leave a 50 ms
publication margin; receipt waiting is additionally capped at two seconds after admission. Unknown
completion reports Requested if the waiter is still active. Cancellation, lock, revoked choices,
closed output or owner timeout suppress late publication. Every queued worker rechecks the exact
Settings/epoch authority, command switch, live waiter, receipt receiver and deadline before starting.
Started external work cannot be undone by cancellation. The native worker retains the exclusive
conversation lease through its launcher API call, including a blocked call. After submission that
worker may exit while the launched editor continues independently. These APIs have no cancellation
acknowledgement; timeout never admits overlapping native workers.

The original capture and all its clones share consumption. The state owner marks it before
releasing admission, even when an acknowledgement is lost. Local results, consumed tool receipts,
finished responses and confirmed interruptions also consume it. Neither a fresh generation label
nor an error may send a consumed capture to local recovery, parser execution or remote upload.
Explicitly capturing a new command remains possible; this guard does not change V8's deliberate
repeat/undo commands. H1's exhaustive voice Intent boundary is unchanged: voice cannot decide cards.

Offline tests cover completed/submitted/failed/missing receipts, mixed batches, timeout publication
through the native owner, queued revocation/expiry/waiter loss, retained leases and clone replay
before/after provider construction and interruption. No desktop action, microphone or live provider
is used. The Claude consultation backend is described below; Codex qualification remains within
V6.6.3. V6.7 still owns capture assembly and visible UI.


## Read-only tools in realtime

V6.6.2 connects the existing five `flock_tools` reads: `flock_list`, `session_detail`, `pending_asks`,
`usage` and `github_snapshot`. They read cached data only; the model cannot execute an Intent,
answer a card, open an agent session, change configuration or fetch a connector. Specs are frozen
once, confirmed exactly by session setup and included in each admitted response. The default Plan
still offers no tools; the native app supplies this read-only registry explicitly.

The supported protocol subset is one function-only response per step, followed eventually by a
separate spoken response. OpenAI documents `response.done` as containing the complete call and
`function_call_output` as keyed by its call ID, followed by another `response.create`.
[Official Realtime tool lifecycle](https://developers.openai.com/api/docs/guides/realtime-mcp).

Vults checks response, attempt, item, call, name, streamed arguments, item completion and final
output together. Calls must be completed successfully; cancelled/failed/incomplete responses never
invoke a tool. Mixed audio/tools, parallel calls, changed identities, missing/extra/duplicate keys,
non-string values or unoffered names fail closed. Arguments are at most 4 KiB, string values at
most 256 bytes and argument deltas at most 128. Call/item IDs remain consumed across turns; a fresh
response cannot replay one. At most eight reads fit in one input's original 120-second deadline.

The exact receipt is durably settled before the read. A separate reservation for the continuation
must succeed before invoking it. Only then does the transport execute the existing Tool and send
one result followed by the new admitted response, in WebSocket order, without re-uploading audio
or retrying a call. Unknown continuation cost keeps its ceiling; work proven not sent is settled
as zero. No tool step creates another key lookup or bypasses the gate.

Results are inert JSON envelopes with `is_error`, `outside`, `text` and `truncated`. Both success
and error envelopes are at most 16 KiB after JSON escaping, before the outer event encodes that
string. UTF-8 prefixes preserve valid JSON when text is shortened. A read error is returned once
as data, never local-parser fallback. Existing outside flags mark GitHub content; tool instructions
keep all results quoted data without permission authority.

A tool receipt is distinct from spoken completion. The native owner publishes its settlement,
releases the original input and stays Thinking with the same quiet output/generation. It emits no
heard history, drain or Listen for a read. Final audio has its own attempt/item identity and uses
the existing transcript, consumption, truncation and history path. A reused tool attempt is rejected
before audio can queue. Failure after a consumed tool response cannot return the original capture
for recovery or replay; it ends the owner without claiming an action completed.

Offline duplex/native-owner checks cover successful/error continuations, separate reservations,
known/unknown receipts, unaffordable reads, invalid/duplicate/cancelled calls, revocation during a
read, escaped result bounds, exact offered specs and final audio/history after input release.
They use synthetic credentials/PCM and no external provider, microphone or desktop action.
The Claude consultation backend below extends this registry only after separate consent. Codex
qualification remains V6.6.3, followed by V6.7 capture/UI. Physical speech, provider compatibility
and acoustic acceptance remain open before activation.


## Text consultation

On Linux, the V6.6.3 native owner adds `consult_claude` only with the separate default-off
`voice_consultation` choice and its original durable budget identity. Only Settings may change it;
changing it revokes the voice epoch even if the old choice is restored before the next poll. The
manual renderer entry has no consultation controls. Default offerings remain the five
cached flock reads. The backend also supports `consult_codex` with its own default-off
`voice_codex_consultation` choice and original `voice_codex_consultation_budget` identity.
Its Settings-only command invalidates the same voice epoch when a choice changes. Codex is
currently blocked by the shared `SUBSCRIPTION_AUTH_QUALIFIED=false` capability: enabling it is
refused before settings/ledger access, saved consent cannot offer its tool or open the network gate,
and the text helper refuses before CLI or scratch access. Turning a saved choice off remains allowed.
Neither provider has a dedicated consultation renderer control; Codex remains unavailable from
the manual voice entry.

Each consultation supplies one nonempty question of at most 256 bytes, without control characters.
The Claude path uses the existing Claude Code text helper with Haiku, a new private empty working directory,
empty built-in tool selection, MCP-tool exclusion, strict MCP configuration, no user/project
settings, disabled skills and no saved session. File-mention expansion, CLAUDE.md discovery, auto
memory and nonessential CLI traffic are explicitly disabled. Clients older than 2.1.299, unknown
versions and other major/minor series are refused before a question starts: older clients may
silently ignore these controls. The shared helper applies this version check to its existing
journal/memory/sleep callers too. [Claude CLI controls](https://code.claude.com/docs/en/cli-reference)
and [environment controls](https://code.claude.com/docs/en/env-vars) define these switches. Vults does
not use bare mode, which bypasses subscription authentication, or weaken managed organizational
policy. This adapter is not an OS sandbox; enforced managed policy may still load hooks or contact
its own services. Managed-client behavior needs separate qualification before activation.

Vults supplies only the question, with no project/session context or project-reading tool. It does
not resume a live agent, delegate work or borrow Chat's history, tools, policies or full access. The separate
`voice-consultation` (Claude) and `voice-codex-consultation` (Codex) gates each durably reserve
at most 20 attempts per rolling 24 hours, including
failures; realtime monetary reservations do not pay for or authorize these CLI attempts. Restart,
consent toggles, history erasure or ledger replacement cannot silently renew the allowance. Traces
contain feature/provider/model, timing and outcome, never question or reply. Vults does not retry a
failed consultation or switch to another provider; the installed CLI retains its own connection
and authentication behavior.

Both providers share one busy slot. One background worker retains that slot and exclusive native voice lease through preparation
and acknowledged child exit. The original call time anchors a 60-second cancellation deadline,
including preparation and CLI version checking. At 1.5 seconds it may emit one translated,
request-identified Consulting notice into the current native output channel. Fast replies skip it;
it is status data, not generated speech or heard history. Cancellation, lost waiter, lock, epoch or
output revocation suppress stale results/notices and request child termination. The worker awaits
exit instead of releasing ownership after a kill request. Uninterruptible local IO or teardown can
keep it busy beyond the deadline; another consultation/session cannot overlap that retained work.

The realtime tool timeout may explicitly reach 60 seconds; cached reads retain their ten-second
limit and the original input's 120-second chain deadline is never extended. Successful/error text
uses the existing bounded inert result envelope with `outside=true`; it has no permission authority.
A consumed receipt prevents input recovery or replay even if consultation or its continuation fails.
The 1.5-second notice has no renderer consumer yet; V6.7 owns its display and opt-in controls.

Offline tests exercise notice timing, original deadlines across slow admission, cancellation and
teardown ownership, child reaping, bounded output, arguments, the client controls/version check,
durable independent admission and consent upgrades. They run synthetic local child processes and
no model, microphone or desktop action. Atomic subscription binding and live/managed-client compatibility
remain open for Codex activation and live acceptance. The guarded backend can be published;
V6.7 engineering proceeds with Codex unavailable.


### Codex qualification

The installed Codex CLI 0.163.0 candidate for `gpt-6.1-sol` does not meet this contract in the
qualified environment. Ignoring user config/rules, selecting ephemeral read-only execution,
excluding project documents and disabling integration/tool features still leaves inline
`additional_tools` and an ambient skills catalog in the first model request. An absent top-level
`tools` field is insufficient: code-mode tool definitions may live in `input` instead. This is
evidence against this candidate, not a claim about every Codex integration. The documented
[CLI controls](https://learn.chatgpt.com/docs/developer-commands?surface=cli) and
[configuration controls](https://learn.chatgpt.com/docs/config-file/config-reference) do not by
themselves establish the supplied-text-only contract. This baseline is not used by the native adapter.

The new staged CLI helper qualifies a narrower contract on exact client 0.163.0. It supplies a
minimal app-owned model catalog instead of inheriting mutable model metadata: shell, patch,
search, code mode and collaboration capabilities are disabled. Fixed caller-owned instructions
replace vendor instructions; project documents, skill catalogs and environment/app/permission
context injection are disabled. These controls were verified against the installed client; some
are internal controls, so unknown/upgraded client versions are refused rather than assumed safe.
The helper does not use another API provider, read credentials or change the user's configuration.

The input is a JSON-quoted question whose dollar and at-sign characters use JSON Unicode escapes.
The model receives instructions to answer the decoded value. This preserves the question while
preventing CLI preprocessing of explicit skill/file references: disabling the catalog alone does
not prevent that expansion. Questions stay on stdin and never enter the catalog or process
arguments. Output uses the existing bounded child runner; cancellation waits for child exit.
Malformed, failed, tool-bearing or incomplete replies cannot become an answer.

The explicit developer probes run in a loopback-only network namespace with synthetic credentials
and questions. Rejected candidates get no model response. The isolated helper must offer zero
tools, exclude private project/file/named-skill canaries and process one fixed text-only stand-in
response through the actual CLI and reply parser. Captured instructions and credentials are not
saved or logged, and no external provider is contacted. Ordinary offline regressions check both
tool registries, input round trips, version restrictions and reply rejection. A filesystem/network-isolated
synthetic-auth test also proves the ChatGPT-only status check refuses API auth without altering it; the original
home/auth/keyring are hidden, and HOME/CODEX_HOME values are never reassigned. The same test
characterizes forced-exec auth deletion only inside that disposable synthetic mount. See the
[roadmap reproduction command](../dev/plan-dictation.md#v6-talk-with-zeca-k8).

The native backend now connects this helper through its own consent, durable quota and content-free
traces, using the shared consultation owner, deadline, cancellation and notice. Claude consent never
authorizes Codex, and exhausting one provider's quota does not consume the other's. No visible
activation or capture caller is present. The helper preserves the installed CLI's authentication,
removes inherited API-key/base-URL/session environment overrides and requires a successful
nonmutating ChatGPT-only `login status` check before starting exec. This is a preflight, not an
atomic binding against a concurrent external login change. The repeatable synthetic-auth regression confirms
that forcing the login method on exec deletes mismatched API authentication; Vults must not use it.
The documented [App Server account read](https://learn.chatgpt.com/docs/app-server) provides no stable
account/workspace identity to bind a separate exec. Subscription-only behavior during an external
login switch remains an activation blocker. Live/managed-client compatibility is also pending;
Codex activation stays open; V6.7 engineering proceeds with that feature unavailable.


### Managed-policy candidate

A separate candidate uses local system `allowed_login_methods = ["chatgpt"]` requirements instead
of forcing the exec login method. [Managed configuration](https://learn.chatgpt.com/docs/enterprise/managed-configuration)
applies these constraints before credential loading. The [security cookbook](https://learn.chatgpt.com/docs/enterprise/security-cookbook)
requires protecting the policy file and its parent from the Codex user; existing sessions do not
automatically inherit a newly deployed policy. Vults does not install or change this system policy.

The installed-client auth regression now isolates both home and `/etc`, using an empty system
mount for controls and a synthetic policy mount for managed cases. Nine scenarios cover ordinary
and restricted status, destructive forced exec, unmanaged API bearer transmission, managed API
refusal without deleting auth, synthetic ChatGPT status, an API replacement between status and
fresh exec, and a replacement after the first ChatGPT-authenticated request receives a synthetic
401. The last case must observe a retry; only the original synthetic ChatGPT bearer may appear,
never the replacement API key. A WebSocket handshake retry or HTTP fallback can satisfy this count;
it does not qualify credential reload. The local server ends requests with a fixed 400; there is no model
response, external provider, real credential or log of headers/auth content. Every child is reaped
and the bounded stand-in is stopped before its private mounts are removed.

These are synthetic file-store observations on CLI 0.163.0, not production eligibility or specific
account binding. Runtime verification of effective policy and protected provenance, keyring
behavior, broader refresh/failure cases and live compatibility remain pending. The activation
capability stays false, including on unmanaged machines; no native or visible feature is enabled.


### Guarded backend publication

Consolidated verification permits publishing the backend with Codex consultation disabled; it
does not qualify or enable that optional adapter. Fresh isolated metadata checks now verify
absent/ChatGPT-only requirements and file/keyring store selection without refreshing tokens.
Required keyring with DBus unavailable refuses both stored API auth and an otherwise-valid
synthetic ChatGPT file. The same ChatGPT file is accepted under file storage, so the keyring cases
check refusal without file fallback; neither synthetic file is altered.
The current host has no protected system requirements file, so no managed route is enabled.

A separate HTTP-only stand-in uses a custom provider requiring the CLI's OpenAI authentication.
It returns a valid POST401 after replacing synthetic ChatGPT auth with a synthetic API key. Both
managed and unmanaged cases terminate after one POST and preserve the replacement file without
sending its key. This covers bounded refusal, not successful refresh or credential reload. The
built-in provider's WebSocket behavior and successful/locked keyring still need qualification.
No real account, keyring, provider or system policy is touched by these tests.

The production microphone setter now invalidates the native voice epoch before publishing a
changed device; restoring it cannot revive the old turn. Unchanged choices and failed saves retain
the current epoch. A setter-level subprocess regression uses a private config root. The question
race/IME/leave checks now run in CI alongside the remaining renderer checks.

V6.7 can now assemble capture and visible controls against the guarded backend. Physical speech,
provider and native UI acceptance belong to that integration; K8 remains open until they pass.
Codex consent, tools, gate and helper execution remain unavailable until their own qualification
is complete. See the [publication checkpoint](../dev/plan-dictation.md#v663-publication-checkpoint-2026-10-20).
