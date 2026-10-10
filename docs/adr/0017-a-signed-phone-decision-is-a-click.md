# 0017. A decision signed on a paired phone counts as a click

**Status:** Accepted, 2026-10-10. Amends [0008](0008-one-core-many-surfaces.md) (on the desktop the
island stays the only card host; a paired phone may also answer) and the Linux-only priority of
`CLAUDE.md`, for Android alone.

## Context

A card waits for a human for up to 108 s. Away from the desk, the user can only let it go to the
terminal. A phone could answer it, but [0004](0004-a-human-answers-permissions.md) and
[0014](0014-consent-before-autonomy.md) accept only a click on the desktop, a rule or a policy the
user made, and [0008](0008-one-core-many-surfaces.md) keeps every card on the island. A message
that merely claims to come from the user's phone is not a human's click: anything on the network
could send it.

## Decision

- A paired Android phone may answer a card. Its answer counts as a click only when it is signed by
  a key kept in the Android Keystore with `BIOMETRIC_STRONG`, unlocked by the user's biometrics for
  that one use.
- Pairing is a QR shown on the desktop: the phone's public key, the relay's URL and a one-time
  secret that lives 2 minutes. The user can revoke a phone from Settings.
- Every answer goes through `core`'s ledger: bound to a hash of the whole request, used once,
  refused after `min(120 s, limits::SERVER_DECISION_TIMEOUT)`, with a counter per device.
- Every answer from the phone is signed by the paired device's key and passes the ledger. Deny and
  dismiss need no biometric unlock; approve, answer, hand off, accept a task and a service action
  do. Sending a message or moving money does not exist in the protocol.
- The desktop stays the source of truth and the only one that acts; the phone only decides. A
  relay, when used, sees only encrypted envelopes. Turning the phone on follows
  the rule for requests on the user's behalf (0019, `docs/dev/plan-zeca.md` G9).
- Kotlin is allowed for the Android shell only; its logic is the Rust core through UniFFI.
- Android is the one exception to "Linux first and only". iOS and macOS come later on the same
  protocol.

## Consequences

- Design decision D1 (`docs/dev/road-to-0.2.md`) reads "the island and the phone": the island
  stays the only card host on the desktop.
- The rule-2 test grows a case for each phone decision, valid and forged (no signature, a revoked
  device, an expired or reused offer, a binding that differs).
- A lost phone without its biometrics cannot approve anything; with them, it can until revoked.
