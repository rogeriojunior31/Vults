# The phone's relay

A paired phone gets its cards and sends its answers through a relay: a small server that keeps
sealed messages until the other side reads them. Pairing a phone from Settings is not in the app
yet; this page is for running a relay ahead of it, or checking what one can see.

## What it sees

Each direction between the desktop and one phone has its own box, addressed by 32 random-looking
bytes that only those two can derive from their keys. A message is sealed (XChaCha20-Poly1305,
with a key from X25519) before it leaves either end, so the relay holds only addresses and
ciphertext: no command, no project, no name. A message is handed out once and dropped after 10
minutes even if nobody read it. There are no accounts and nothing is written to disk.

An answer from the phone counts only if it is signed by that phone's key, made for that exact
card, in time, and never seen before. A relay that drops, delays or replays messages can make a
card wait or go to the terminal; it can never answer one.

## Running it

From the repository's root:

```sh
docker build -f packaging/relay/Dockerfile -t vults-relay .
docker run -p 8787:8787 vults-relay
```

or without a container:

```sh
cargo run --release -p vults-relay --bin vults-relay
```

`VULTS_RELAY_ADDR` says where it listens (`0.0.0.0:8787` by default). It speaks plain HTTP: put a
reverse proxy with TLS in front of it, or reach it only over a tailnet or your LAN.

## Limits

| What | Limit |
|---|---|
| A message | 64 KiB |
| Messages waiting in one box | 64 |
| Boxes at once | 10,000 |
| A message left unread | 10 minutes |
| A read waiting for a message | 25 seconds |

## HTTP

| Request | Does |
|---|---|
| `POST /v1/boxes/{box}` | Keeps the body (one sealed message). `202`; `404` for an address that is not 64 lowercase hex characters; `413` too big; `429` the box is full; `503` too many boxes |
| `GET /v1/boxes/{box}?wait=s` | Takes everything waiting, as a JSON array of strings; with nothing there, waits up to `s` seconds (25 at most) for something to come |
| `GET /health` | `ok` |

## Trying it

Two command-line stand-ins play the desktop and the phone:

```sh
cargo run -p vults-relay --bin vults-relay
cargo run -p vults-relay --example desk -- http://127.0.0.1:8787
cargo run -p vults-relay --example phone -- '<the QR text desk printed>'
```

`desk` shows the QR's text, waits for the phone, and sends it one card; `phone` pairs, shows the
card and sends your answer, signed; `desk` prints what Vults would do with it.
