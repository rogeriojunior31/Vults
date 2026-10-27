# 0019. Network, secrets and telemetry

**Status:** Accepted, 2026-10-10. Consolidated on 2026-10-12 from 0006 and 0016.
Amended on 2026-10-17 for N3 V0.4: dictation model requests, loopback limits and realtime spend.
These are implementation requirements for V3, V5 and V6, not enabled or shipped requests.

## Context

The app sees commands, file names, diffs and API keys, next to private work. Some features still
need the network while the user is not asking at that moment: connectors, web search and fetch
during a chat turn, Zeca's sleep, routines, the phone's relay and push, spoken replies, the update
check. Each sends something the user cares about off the machine, costs money or quota, and could
become telemetry by accident.

## Decision

- **Secrets only in the OS keyring**, never in a file or a log.
- **Never telemetry.** The app sends nothing about its use; no request carries usage, an id or
  anything about the user beyond what the feature needs to do its job.
- **A request the user starts** with a click or a message (a chat turn, an API chat) is their
  explicit choice, said where it is turned on.
- **A request on the user's behalf** (one that leaves the machine without a click at that moment,
  including a web search or fetch the model makes during a turn the user started) is a feature with:
  - its own **switch** in Settings, off by default; turning it on says what it sends, where, and
    what it costs;
  - a **budget** (requests, tokens or money per day), and it stops when it is spent;
  - a **local trace** the user can read (what, where, when, cost), sent nowhere else;
  - **one network gate owned by `app`** (`app/src/gate.rs`), which checks the switch and budget.
    No crate opens a connection for such a feature around it.
- Connectors (GitHub through `gh` today) are such features: each is off until the user connects
  it, and its polling goes through the gate.
- **Updates are asked for and signed**:
  - The check is off until the user turns it on (in Settings, and offered once on first run). When
    on, it fetches one static manifest from GitHub Releases at most once a day and sends nothing
    beyond the request: no id, no usage, no version in a query string.
  - An update installs only after a click that shows the version and its notes. A running agent is
    never interrupted: the restart waits until the user chooses it.
  - Three channels, each with its own manifest URL: **stable** (the default), **beta** (release
    candidates) and **nightly** (selected commits). A build knows its channel at compile time, so a
    stable build never reads the nightly manifest; moving between channels is a reinstall.
  - Every update is signed. The private key lives only in the release job's secrets; the public
    key is built into the app. A manifest or package that fails the check is ignored and logged.
  - Packages a distribution updates (AUR, a future Flatpak) ship with the updater compiled out.

## Dictation model requests

Local speech-to-text, dictionary passes and deterministic voice commands do not call a remote model.
The optional N3 model features below are separate requests on the user's behalf, not permission
to send a transcript merely because recording or Chat is enabled. Each switch is off by default
and changed in the Settings webview, with the provider, data and limit explained before enabling
it. Reusing Chat's provider does not reuse consent to a different feature.

| Gate feature | Phase | Data sent to the selected model | Limit in any rolling 24 hours | When off, spent or unavailable |
|---|---|---|---|---|
| `dictation-cleanup` | V3 | The dictated text and the vocabulary/style context needed to filter or translate it; no audio | 2,000 attempted model requests, including retries | Keep the raw local transcript; never answer or execute it |
| `meeting-summary` | V5 | The meeting transcript needed for summary and to-dos; no recorded audio | 20 attempted model requests, including retries | Keep the local transcript; leave model summary unavailable |
| `voice-realtime` | V6 / K8 | Live conversation audio and the bounded conversation context disclosed in Settings | A positive monetary limit, default US$1; zero disables remote voice | Stop remote audio and continue in text with an explanation |

- **One gate, separate budgets.** These identifiers go through `app`'s gate before a request or
  realtime connection starts. Attempts, including failed calls and retries, count toward the
  relevant quota; no model fallback gets an uncounted request. Budgets survive restart and do
  not reset at midnight. Concurrent work shares the same allowance. A missing or unreadable
  budget cannot grant a fresh allowance. Turning a switch off cancels that feature's in-flight
  model work; local transcription remains usable.
- **Realtime reserves spend before sending.** V6 must extend the count-only gate with monetary
  accounting and a bounded streaming allowance before `voice-realtime` can run. Include
  in-flight reservations, reconcile reported usage and stop streaming before the allowance is
  exhausted. Missing rates or unknown usage must not mean zero cost or unlimited audio; refuse
  further remote audio when a safe remaining allowance cannot be established. The amount in
  the prototype is a chosen limit, not a provider price or evidence that accounting exists.
- **Loopback is not a consent or quota bypass (Q3).** N3's local Ollama/LM Studio calls still
  use the feature switch, request quota and local trace through the gate. They consume no
  remote monetary allowance. This supersedes the draft proposal to skip the entire budget:
  local model work still needs a finite request limit under this record. To qualify as
  loopback, use an explicitly validated loopback IP endpoint with no proxy or redirects;
  a provider label or a hostname such as `localhost` is insufficient. Do not silently retry
  against a remote provider. This contract governs Vults' connection, not a guarantee that an
  independently configured local service cannot forward data elsewhere. The existing recall
  embedding path is unchanged; it does not authorize a new N3 request outside the gate.
- **Trace operations, not dictation contents.** Keep feature, provider/model, local/remote
  classification, timing, outcome and available usage/cost locally. Never include raw audio,
  transcript text, clipboard contents, keys or restore tokens in a request trace. Unknown
  cost stays unknown. Transcript history follows its separate retention choice, including
  *Do not keep*. No trace is sent to a provider or used as telemetry.
- **Failure preserves the local work.** Clean-up is a text filter without tools: refusal,
  timeout or invalid output uses the raw transcript and does not change the delivery guard in
  [0032](0032-dictation-into-other-apps.md). Meeting transcription does not depend on summary
  consent. Remote voice consent does not enable follow-up listening or model clean-up;
  [0028](0028-spoken-replies-through-a-realtime-model.md) still governs listening and audio.

## Local voice commands (VC1)

Dictation and commands reuse the existing speech transcription model. The app interprets command
text locally using supported phrases and current targets. This needs no additional text-model
provider, Claude integration, intent request budget or network feature. The separate default-off
local action switches authorize only the typed actions in
[0032](0032-dictation-into-other-apps.md#natural-voice-commands-vc1).

The earlier `voice-command-intent` proposal is withdrawn before activation following the owner's
scope correction. Existing optional transcript clean-up and unknown-speech Chat handoff retain
their own consent and network gates; neither is required to run local commands. This correction
creates no exemption for other network requests and changes none of the general consent rules.

## Consequences

- A test runs a day with every feature off and counts zero requests.
- Before N3's model features ship, extend this check to each feature, local endpoints included.
  Cover exact quota boundaries, retries, concurrent reservations, restart, unreadable budget
  state, mid-stream disablement and unknown realtime cost, plus raw-transcript fallback and
  traces without transcript contents. Register the actual call sites in the network inventory
  and its safety guide when implemented; declaring identifiers here adds no network access.
- No usage numbers steer the roadmap: we learn from issues and from using it ourselves. Anything
  that would learn from users' data has to learn on the user's machine or not at all.
- The updater lands after 0.2.0 ([0020](0020-0.2.0-at-wave-4-then-0.2.x.md)). `release.yml` will
  sign the packages and publish one manifest per channel; the key is created once and backed up
  offline (losing it means users reinstall by hand). Nightly and beta builds get their own bundle
  id suffix, so they install beside stable without sharing settings or the keyring.

## History

- 2026-10-26: owner correction withdraws the unactivated command-intent network proposal;
  existing speech transcription plus local command matching needs no separate text provider.

- 2026-10-26: VC1 initially proposed independent command-model consent; superseded by the owner
  correction above before activation.

- 2026-10-17: N3 V0.4 names the three independently opted-in model features and their rolling
  limits. Q3 keeps loopback calls behind the switch, quota and trace; only remote monetary
  accounting is exempt. Realtime requires spend accounting before implementation can ship.
- 0006, 2026-10-04: secrets only in the keyring; no telemetry;
  leaving the machine is the user's choice.
- 0016, 2026-10-08: the update check, asked for, signed,
  one channel at a time.
