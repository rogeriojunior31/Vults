# 0019. Network, secrets and telemetry

**Status:** Accepted, 2026-10-10. Consolidated on 2026-10-12 from 0006 and 0016.

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

## Consequences

- A test runs a day with every feature off and counts zero requests.
- No usage numbers steer the roadmap: we learn from issues and from using it ourselves. Anything
  that would learn from users' data has to learn on the user's machine or not at all.
- The updater lands after 0.2.0 ([0020](0020-0.2.0-at-wave-4-then-0.2.x.md)). `release.yml` will
  sign the packages and publish one manifest per channel; the key is created once and backed up
  offline (losing it means users reinstall by hand). Nightly and beta builds get their own bundle
  id suffix, so they install beside stable without sharing settings or the keyring.

## History

- [0006](0006-keyring-no-telemetry.md), 2026-10-04: secrets only in the keyring; no telemetry;
  leaving the machine is the user's choice.
- [0016](0016-updates-asked-for-and-signed.md), 2026-10-08: the update check, asked for, signed,
  one channel at a time.
