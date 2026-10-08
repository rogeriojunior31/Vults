# 0016. Updates: asked for, signed, one channel at a time

**Status:** Accepted, 2026-10-08. Amends [0006](0006-keyring-no-telemetry.md). The updater itself
lands after 0.2.0 ([0015](0015-everything-by-0.2-in-small-releases.md)).

## Context

Today a user learns of a release from GitHub and installs it by hand. The Tauri updater would
download and install new versions itself, from a signed manifest. It would also be the first
request the app makes on its own: [0006](0006-keyring-no-telemetry.md) promises that nothing
leaves the machine unless the user chose it.

## Decision

- The check is off until the user turns it on (in Settings, and offered once on first run). When it
  is on, the app fetches one static manifest from GitHub Releases at most once a day and sends
  nothing beyond the request itself: no id, no usage, no version in a query string.
- An update installs only after a click that shows the version and its notes. Nothing installs on
  its own, and a running agent is never interrupted: the restart waits until the user chooses it.
- Three channels, each with its own manifest URL: **stable** (the default), **beta** (release
  candidates) and **nightly** (selected commits). A build knows its channel at compile time, so a
  stable build can never read the nightly manifest; moving between channels is a reinstall.
- Every update is signed. The private key lives only in the release job's secrets; the public key
  is built into the app. A manifest or package that fails the check is ignored and logged.
- Packages a distribution updates (AUR, a future Flatpak) ship with the updater compiled out.

## Consequences

- 0006 gains one sentence, in the pull request that accepts this record: an update check is the
  only request the app makes on its own, and only when the user turned it on.
- `release.yml` signs the packages and publishes one manifest per channel; the key is created once
  and backed up offline (losing it means users reinstall by hand).
- Nightly and beta builds need their own bundle id suffix, so they install beside stable without
  sharing settings or the keyring.
