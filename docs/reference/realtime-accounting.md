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
renderer has no conversation entry or consultation control yet. Default offerings remain the five
cached flock reads. The backend also supports `consult_codex` with its own default-off
`voice_codex_consultation` choice and original `voice_codex_consultation_budget` identity.
Its Settings-only command invalidates the same voice epoch when a choice changes. Codex is
currently blocked by the shared `SUBSCRIPTION_AUTH_QUALIFIED=false` capability: enabling it is
refused before settings/ledger access, saved consent cannot offer its tool or open the network gate,
and the text helper refuses before CLI or scratch access. Turning a saved choice off remains allowed.
There is no renderer control or capture entry for either provider.

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
remain open; V6.6.3 is not complete.


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
V6.6.3 stays open before V6.7.
