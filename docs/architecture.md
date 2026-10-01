# Architecture

Everything flows one way:

```
agent ──hook JSON──▶ vultures-ai-hook ──protocol line──▶ ipc server ──▶ core::reduce ──▶ effects
                                                                              │
                                                                     State::view ──▶ UI renders
```

| Crate | Role | Must not use |
|---|---|---|
| `brand` | The app's name, slug and bundle id; generates `ui/src/brand.ts` | anything |
| `protocol` | Versioned wire messages, limits, socket and pipe names | tokio, Tauri |
| `peer` | Same-user checks on both ends of the connection | tokio, Tauri |
| `hook` | Reads the agent's hook JSON, forwards it, prints the agent's decision format | tokio, HTTP |
| `ipc` | Async server: connection limits, ack-then-decide, routing to the app | Tauri |
| `core` | Pure domain: sessions, activities, alerts, island state; clock injected | IO, async, Tauri |

## Why the hook waits for an acknowledgement

A permission request keeps its connection open. The server only waits for a human once the UI says
the card is on screen. If the UI is paused or not listening, the agent gets its answer (silence, so it
asks in the terminal) within 800 ms instead of two minutes.

## Why the decision is only produced by `core`

`core::reduce` turns a `PermissionRequested` event into pending state, and only an `Intent::Decide`,
which the UI sends on a click, turns pending state into a `RespondPermission` effect. Nothing else can
approve a tool call.
