# The phone

A phone you pair can answer the cards that wait for you while you are away from the desk. It only
decides: this computer still does what you chose, and the island keeps every card too. The Vults
phone app is not out yet; until it is, the command-line test client in
[The phone's relay](../reference/relay.md) pairs and answers like a phone.

## Turning it on

In **Settings → Phone**, turn on **Answer cards from a phone** and choose how the phone reaches
this computer:

- **Through a relay**: a small server that keeps sealed messages until the other side reads them.
  Run your own ([The phone's relay](../reference/relay.md)) and give its address.
- **Straight (LAN or tailnet)**: this computer runs the relay itself on an address you give, such
  as its tailnet address and a port (`100.64.0.2:8787`). The phone must be able to reach it.

Every request to the relay is a request on your behalf: off until you turn it on, at most 10,000 a
day, each kept in the local trace ([Safety](../safety.md)).

## Pairing

Press **Pair**: a QR shows for two minutes. Scan it with the phone. The QR holds this computer's
public key, the relay's address and a one-time secret; only a phone that saw it can pair, and it
pairs one phone. The phone then shows under **Paired phones**.

## Answering

Each card waiting on the island goes to every paired phone, and leaves it when the card leaves the
line. On the phone:

- **Deny** and **Dismiss** (send it to the terminal) need no biometrics.
- **Allow**, **Always** and answering a question need your fingerprint or face, for that one answer.

An answer counts only if it is signed by that phone's key, made for that exact card (the same
command, in the same folder), before the card's time runs out, and never seen before. A relay that
drops, delays or replays messages can make a card wait or go to the terminal; it can never answer
one. **Always** from the phone makes the same rule as on the island, for the folder the card was
offered in.

If this computer cannot save which answers a phone already used, it does not act on that phone's
answer: the card goes to the terminal.

## Revoking

**Revoke** beside a phone and nothing it signs is taken from then on. A revoked phone stays in the
list and cannot pair again under the same keys.
