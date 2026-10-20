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
webview can save this choice; there is no renderer control, bridge call or microphone entry yet.
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
backend has no user-facing capture entry. V6.3.5 adds native retention/recovery below; capture,
playback, visible continuation and live-provider acceptance remain required before activation.

## Native turn retention and local text

V6.3.5 changes the native app service to accept complete `realtime::Turn` values, independently of
remote admission. Each value owns 100 ms to 30 seconds of mono 24 kHz PCM16 in memory. One pending
turn is fed into the persistent transport in 100 ms chunks through a two-command queue; a four-event
queue relays output. Raw audio emission yields so buffered socket bursts cannot starve this inline
consumer. Only a successful final receipt releases the pending original; queued receipts are drained
before applying a simultaneous transport failure. A later idle failure cannot recover an older turn.

Off/spent admission, missing keys, transport/protocol/response failures or saturated output retain
the failed turn for local recovery. The transport gets up to 11 seconds to finish its existing
bounded settlement path, then its future/socket is dropped before decoding. The selected installed
Whisper model and language are snapshotted; the existing resampler converts the retained audio to
16 kHz. No download, model clean-up, other provider or automatic Chat send is started. A successful
local decode returns the whole text, preserving whitespace, for the native caller to offer as a
reviewable draft. This is not a claim that Zeca already answered in text or that a fallback UI exists.

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
No microphone, playback, renderer command or permissions path is added. Those owners remain V6.4-V6.7.

Run `cargo test -p vults-chat realtime::transport::` and `cargo test -p vults-app realtime` offline.
Duplex WebSocket tests exercise actual frames with synthetic PCM/keys, two turns, cancellation,
revocation, failed settlement, backpressure and unknown usage. They open no listener or provider
connection. No paid request, real microphone/playback or live speech acceptance is claimed. The
native driver/decoder tests use synthetic PCM and local work doubles; real model/speech quality
and capture/playback acceptance remain separate. Backend failures provide structured reasons and
never silently send to a different provider.


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
discarded; only a matching confirmed probe freezes playback. Notifications of Listen/Probe actions
grant no microphone authority and do not start capture. First consumed output drives the lock;
producer completion requests drain, and confirmed drain drives tail grace. App entry keeps barge-in
disabled until the V6.5 device/AEC policy lands. Scripted headphone cases exercise the available
owner path without asserting that speaker interruption is safe.

The tool-free protocol accepts one assistant audio item/content part at index zero per response.
Attempt, response and item IDs are bound across PCM, transcript, final output and history; response
and item IDs cannot be reused in the bounded session. Changed IDs, another content part, tools,
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
Input events to the native caller. The caller must assemble and commit complete turns; capture is
never uploaded automatically. Explicit headphones/AEC policy stays separate from temporary route
verification; an unverified route clears retained speech/probes before deferred playback events.
The ordinary CPAL entry remains non-interruptible. No renderer command or visible voice conversation
is exposed. Backend V6.4/V6.5 is delivered; see [Echo cancellation](echo-cancellation.md) for routing,
clock and private-graph evidence. Next is V6.6 tools, then V6.7 capture/UI. Physical-device/acoustic
timing, real speech and live-provider acceptance remain open before activation.
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
use the existing runtime executor and the exhaustive voice-only Intent allowlist. External launches
or persistence say Command requested after submission, since submission is not completion proof;
finished core-only changes may say Command completed. A local result emits an identity-bound Local
event, finishes without Generate and retires that input generation. A queued duplicate cannot run
again or upload. A handled input never becomes automatic model/Chat fallback after an error.

Scripted owner tests cover local input before lazy provider construction, local follow-up on an
established connection, stale/duplicate generations and cancellation during admission. Dispatcher
checks cover permission refusal, command disablement, ambiguity, separate context, localized cached
reads, expired admission and once-only mutation. These tests use no microphone, desktop action,
key or provider. Model-side flock tools and consultation remain V6.6.2/V6.6.3; full V6.6, renderer
activation and real speech/provider acceptance are still open.
