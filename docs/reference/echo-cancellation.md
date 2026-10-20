# Echo cancellation

V6.5.1 qualifies a native Linux path with a read-only inventory and a reproducible synthetic
experiment. V6.5.2 adds explicit native duplex ownership for that route and a separate headphones
choice. There is still no renderer entry or visible microphone activation; physical speech and
speaker acceptance remain open before the V6.7 interface.

## Candidate inventory

`vults_audio::echo` uses the existing `meeting-capture` feature and bounded local Pulse connection.
`discover()` opens no audio stream, changes no defaults/routes and loads no modules. It rejects
remote endpoints and unsupported servers, then requires all of the following:

- a PipeWire Pulse server and an explicitly configured `module-echo-cancel` with `aec_method=webrtc`;
- exactly one non-monitor source and one sink owned by that module;
- matching nonempty `node.group`, valid object serials, distinct names and supported sample metadata.

Quoted module arguments use a strict subset of PipeWire's parser: quoting starts immediately after
`=`, a closing quote ends the value, and escaping or ambiguous syntax rejects the candidate. Labels,
a method inside another quoted argument and default engine inference never establish a pair.
Native-only modules without Pulse ownership metadata are outside this inventory's supported subset.

The opaque `Pair` records the server cookie, module, endpoint indexes/serials and group; its Debug
output omits names. `present()` compares a fresh snapshot. Neither discovery nor a successful recheck
is a route lease, acoustic proof or permission to enable interruption. Enumeration can race stream
creation or a route change. Device names/serials are local identifiers, not a trust boundary.

## Both directions are required

The cancellation source supplies cleaned microphone samples. The paired sink receives the exact
assistant output used as the far-end reference, then forwards it to the speaker. Selecting only
the source while CPAL plays to the default output does not establish that reference path.
[PipeWire's four-stream contract](https://docs.pipewire.org/page_module_echo_cancel.html).

The default V6.4 CPAL entry keeps voice barge-in disabled. V6.5.2's separate `realtime::run_routed`
entry requires explicit realtime consent, a selected route and current native authority. Its single
worker pins a 24 kHz mono PCM16 capture stream and output stream to the exact chosen device indexes,
checks their negotiated formats/target acknowledgements, and sets `DONT_MOVE`. No default fallback
is allowed. `Route::echo` validates the selected pair; `Route::headphones` is an explicit assertion
by the user, never a label-based inference. `Route::speakers` keeps interruption disabled.

## Routed native ownership

Subscription precedes the baseline graph snapshot. The route records server/node identities,
module ownership, the AEC module's internal capture/playback connections, their physical endpoints
and active ports. Internal moves, relevant physical changes/removals, module changes or conservative
card events revoke the whole session. Default-device changes do not redirect pinned streams.
Other watched changes suspend verification immediately; only an event-stable matching snapshot
rearms it. Samples captured while verification is suspended are discarded. A graph refresh runs at
least every 100 ms; missing validation/input for 500 ms, invalid timing, overflow or server failure
ends the owner. Setup is limited to three seconds, sessions to ten minutes and responses to 120
seconds of source PCM. Input/output queues and metadata copies are bounded.

One native worker retains the app's lease through teardown and shares output exclusion with CPAL.
It checks revocation before attach/uncork and throughout processing. Callback storage stays owned
until native operations are cancelled; a retired output keeps its native reference until one delete
acknowledgement. Stream/context/mainloop destruction order is explicit. This uses the system
`libpulse` through `libpulse-sys`, without a handwritten Pulse protocol or a production module loader.
Linux builds need libpulse headers; installer dependencies include the runtime library.

Playback maps content spans into native stream positions, including silent gaps. A raw stream clock
alone cannot count heard words: underrun silence is excluded, reported sink latency is subtracted
and corrupt/stale timing fails closed. Producer completion, consumed content and a successful drain
acknowledgement remain separate. Stop freezes the last confirmed content cursor and discards pending
output; the old native stream is retired before a new response can play. The estimate remains at
the reported sink boundary; physical downstream timing still needs acoustic validation.

`run_routed` forwards bounded, memory-only `Input` events to its native caller. For explicitly
chosen headphones/AEC routes, V6.7.3 now also consumes those packets through a local interruption
adapter. Installed transcription/VAD models are checked before native streams open. Detection starts
only after actual playback, keeps an eight-second PCM ring and observes fresh speech windows. The
existing turn machine owns playback locks and exact probe tokens; a separate local transcription
pass may confirm words within the probe budget. Probe text never becomes a command, permission,
Chat draft or provider input. Empty/failed/expired or revoked results cannot confirm an interruption.

One bounded native worker retains the desktop lease through inference and model destruction.
Cancellation, route loss, replaced generations and closed owners suppress late publication. On a
confirmed interruption, `Replay` follows the new `Listen` action and carries the retained onset and
pre-roll under that exact generation. The native caller still owns complete-turn assembly and an
explicit commit; the adapter does not automatically upload/save audio or send it to Chat.

Chosen headphones/AEC policy remains distinct from live route validity. The ordinary Nest voice
page still uses manual capture and the Stop response button; this adds no routed UI activation.
Routed turn assembly, explicit route selection/consent UI and physical/live qualification remain
V6.7 work. Q6's production setup choice is open; Vults loads no system modules silently.

## Reproduce the isolated experiment

Prerequisites: Linux with PipeWire's Pulse and WebRTC echo modules, WirePlumber 0.5, `pactl`, `pw-cat`
and `pw-dump`. The example expects installed stock WirePlumber data under `/usr/share/wireplumber`.
From the repository root:

```sh
cargo run -p vults-audio --features meeting-capture --example echo-cancel-spike
```

The explicit developer command starts a private PipeWire core and policy-only WirePlumber, using
private Unix sockets, a private Pulse cookie and a mode-0700 temporary directory. It pins private
PipeWire/Pulse client configuration, copies the installed stock WirePlumber main configuration to
that directory and applies only its own profile overrides. WirePlumber script lookup is pinned to
the installed stock data. Private state/cache paths and disabled D-Bus, hardware monitors and state
restoration keep the experiment separate from normal desktop routing. It never changes `HOME`.
Exclusive configuration/script lookup matters because private XDG paths alone do not exclude all
user overrides; see [PipeWire configuration lookup](https://github.com/PipeWire/pipewire/blob/1.6.9/src/pipewire/conf.c)
and [WirePlumber locations](https://pipewire.pages.freedesktop.org/wireplumber/daemon/locations.html).

Only private null sinks and their monitors feed the test. A bounded graph dump rejects ALSA or
Bluetooth nodes before streams start. Processes have deadlines; children are killed/reaped and the
temporary directory is removed on normal completion or handled errors. Synthetic float PCM stays
in memory/pipes; no human microphone, desktop speaker, provider, key or network is used. Abrupt
external termination can bypass ordinary process cleanup, as with other local developer commands.

The fixed input is eight seconds of 48 kHz mono seeded noise. A simulated microphone receives that
reference delayed by 20 ms and attenuated by 0.5, plus two local 0.03-amplitude tones at 300/700 Hz
from second six. Gain control, noise suppression and high-pass filtering are disabled explicitly;
`aec_args` must retain its enclosing quotes when `pactl` joins the arguments. This isolates the AEC
measurement from additional suppression. The installed WebRTC build is recorded, without inferring
an AEC implementation/version from a device label.

Before measurement, the synthetic gate was set to at least 15 dB echo reduction after warmup and
each local tone within -3 to +3 dB of its input amplitude. Scoring uses 3-4.5 seconds for echo and
6.3-7.5 seconds for local tones, relative to captured onset. The JSON reports measurements and
`synthetic_gate_passed`; a valid negative measurement still exits successfully. Process, capture,
inventory or metadata errors fail the command. `physical_acceptance` is always false.

## Recorded evidence and limits

The [2026-10-19 fixture](../../crates/spike/fixtures/echo-cancellation-2026-10-19.json) records three
consecutive runs, source fingerprints and installed versions: PipeWire 1.6.9, WirePlumber 0.5.18
and WebRTC audio processing 1.3. All three passed the fixed synthetic gate. The result supports
continuing with the native PipeWire engine; no extra AEC crate is justified by this experiment.

| Metric | Three-run range | Fixed gate |
|---|---|---|
| Echo reduction | 20.233 to 20.299 dB | At least 15 dB |
| Local 300 Hz gain | -1.218 to -0.998 dB | -3 to +3 dB |
| Local 700 Hz gain | -0.523 to -0.317 dB | -3 to +3 dB |

This is one synthetic stationary echo path, two local tones and a shared virtual clock. It does
not qualify human double-talk, microphone/speaker acoustics, moving echo paths, gain changes,
device hotplug, long-session stability, physical latency or the complete app route. No production
switch or physical/live acceptance is marked complete. The V6.5.1 source fingerprints refer to
commit `5099a43d`; the current route evidence is recorded below.

## Routed validation

Run the native route check inside the same private graph:

```sh
cargo run -p vults-audio --features meeting-capture --example echo-cancel-spike -- --duplex
```

It changes only private defaults/routes/modules and tests actual capture/target acknowledgements,
full drain, interrupted cursor freeze, another response after stop, refusal of manual moves,
revocation when an internal AEC connection moves, explicit headphones/speaker policy, module removal,
stale selection refusal and native lease release. The
[2026-10-20 fixture](../../crates/spike/fixtures/duplex-routing-2026-10-20.json) records three final
consecutive passing runs, with source fingerprints from commit `86d6b80d`. Pure and native-owner
regressions additionally cover silence spans,
latency/corrupt clocks, admission revocation, worker-held references and stale interruption evidence.

Backend V6.5 is delivered. Next is V6.6 tools, then V6.7 capture/UI; physical speech, double-talk,
end-to-end acoustic latency, hardware hotplug and long-session acceptance stay open. The isolated
check is synthetic native routing evidence, not physical acceptance.
