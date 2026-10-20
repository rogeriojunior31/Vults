# Echo cancellation

V6.5.1 qualifies a native Linux path with a read-only inventory and a reproducible synthetic
experiment. It does not activate voice interruption. V6.5.2 must bind both actual streams to the
selected pair before speaker barge-in can be considered; physical speech acceptance remains open.

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

The V6.4 CPAL worker still uses the default output, so the app entry keeps barge-in disabled. The
next task must pin capture and playback, acknowledge stream targets, prevent silent moves and revoke
the route generation on device/module/server changes. Playback drain and heard-text accounting must
remain tied to consumed output. Without qualified AEC, speaker interruption stays off; headphone
interruption requires an explicit user choice, never device-name inference. Settings setup and Q6's
module-loading choice remain open; this task adds no production loader or new microphone entry.

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
switch or physical/live acceptance is marked complete. Next is V6.5.2 routed duplex ownership,
then V6.6 tools and V6.7 capture/UI.
