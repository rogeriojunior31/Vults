# 0004. Who answers a permission

**Status:** Accepted, 2026-10-04. Consolidated on 2026-10-12 from 0014, 0017, 0025 and 0026.

## Context

Allowing a command is the one thing the app does that can cause harm. It must never come from a
timer, a guess, a model or another program, nor from a message that merely claims to be the user.

## Decision

`core` turns a pending permission into an answer from these inputs only, each a human's choice:

- **A click**: `Intent::Decide` (Allow or Deny, or Ctrl+Alt+Y/N with the card visible), or
  `Intent::DecideAlways` (Always: allows this request and saves a rule).
- **An exact rule** the user saved: the same agent, folder, tool and target. `cargo test` does not
  allow `cargo test && rm -rf build`.
- **A policy** the user wrote, saw as a diff and clicked to accept, like an agent config
  ([0005](0005-agent-configs-backup-diff-click.md)). Policies class actions as read, external read,
  write, destructive, spend or external (`core::policy`), and a deny beats an allow.
- **A paired Android phone's answer**, which counts as a click only when it passes these checks:
  - Pairing is a QR shown on the desktop: the phone's public key, the relay's URL and a one-time
    secret that lives 2 minutes. The user can revoke a phone from Settings.
  - Every answer is signed by the paired device's key. Approve, answer, hand off, accept a task and
    a service action also need a key kept in the Android Keystore with `BIOMETRIC_STRONG`, unlocked
    by the user's biometrics for that one use; deny and dismiss do not. Sending a message or moving
    money does not exist in the protocol.
  - Every answer goes through `core`'s ledger: bound to a hash of the whole request, used once,
    refused after `min(120 s, limits::SERVER_DECISION_TIMEOUT)`, with a counter per device.
  - The desktop stays the source of truth and the only one that acts; the phone only decides. A
    relay sees only encrypted envelopes; turning the phone on is a request on the user's behalf
    ([0019](0019-requests-on-the-users-behalf.md)). On the desktop the island stays the only card
    host ([0021](0021-four-sizes-one-nest.md)).
  - Kotlin is allowed for the Android shell only; its logic is the Rust core through UniFFI.
    Android is the one exception to "Linux first and only"; iOS and macOS come later on the same
    protocol.
- **In Zeca's chat only, what the user chose** in Chat or Settings → Chat & voice → *Who answers
  Zeca's cards*:
  - **Asks first** (the default): a card for every call no *Always* rule covers (*Always in this
    chat* is one exact command, until the conversation ends).
  - **Your policies**: accepted policies answer the chat's calls exactly as they answer Claude
    Code's on the island: a deny denies, an allow allows (Claude Code's calls only, inside the
    conversation's folder), the rest asks.
  - **Full access**: every call runs with no card, except that what a policy denies is denied, what
    a policy asks first still asks, and once the conversation has read content from outside (a web
    page, a fetch, an MCP tool) its cards ask again for the rest of that conversation.
  - The human chooses access directly in the Chat picker or in Settings. The selected scope is
    explicit: Zeca or one specialist profile. A successful save applies the choice to the next call;
    a failed save grants no access. The chat says what ran on whose word, and its CLI still runs with
    `--permission-mode default` so the answers come from Vults. The flock's sessions are untouched.

Above all of these:

- **A destructive call always asks**: no policy, no *Always* rule (one saved before this rule
  included) and no chat setting answers it. Its card offers no **Always allow**; if the core gets
  one anyway, it is a plain Allow for that card, audited as such, and no rule is saved.
- **Every answer** (a human's, a rule's, a policy's with its consent, the phone's, the chat's
  `full-access`) goes to an append-only audit log the user can read.
- A budget's "hard stop" only stops new sessions the app would start; it never blocks or kills an
  agent the app only watches.
- Nothing else answers. A test feeds every other input and checks that none answers; every new
  `Intent` joins it, with a case for each phone decision, valid and forged (no signature, a revoked
  device, an expired or reused offer, a binding that differs).

## Consequences

- No notification, tray item, palette, model, voice or assistant can allow anything; they can
  only bring the card up.
- A lost phone without its biometrics cannot approve anything; with them, it can until revoked.
- Full access trusts the model in the chat's folder: a mistaken edit or a `git push` runs. The
  setting says so, the chip in the chat turns red, and a policy's deny lines keep actions out of
  reach.
- One test pins each half of the destructive rule: Always on a destructive card saves no rule, and
  an old rule does not answer one.

## History

- 0014, 2026-10-10: accepted policies may answer; a deny beats
  an allow, destructive always asks, the audit log, the budget's hard stop.
- 0017, 2026-10-10: a signed answer from a paired
  phone counts as a click; Kotlin and Android as the one exception.
- 0025, 2026-10-12: no *Always* rule answers a destructive
  call.
- 0026, 2026-10-12: who answers Zeca's chat cards
  is the user's choice.

- 2026-10-22: At the maintainer's request, the Chat access picker may increase or decrease access directly. Keep the selected recipient explicit, persist before applying authority, and retain all policy, destructive-action and outside-content guards.
