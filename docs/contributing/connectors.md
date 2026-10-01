# Adding a connector

A connector watches an outside service and puts its news on the island: a failed check, an approved
pull request, a review waiting for you. GitHub is the first one; adding another takes one Rust file,
one line in the registry and one entry in the settings UI.

## What you write

```rust
pub trait Connector: Send + Sync {
    fn id(&self) -> &'static str;                 // "github"
    fn interval(&self) -> Duration;               // how often to poll when all is well
    fn poll(&self) -> Poll<'_>;                   // the service's current state, as a Snapshot
    fn diff(&self, before: &Snapshot, after: &Snapshot) -> Vec<Event>; // news between two states
}
```

A `Snapshot` is a map from a stable key (`pr:owner/repo#12`) to whatever JSON you need to tell what
changed. You never decide *when* something is news against the wall clock: you compare two snapshots.
That is what makes the rest automatic.

## What the runtime does for you

- Polls only while the user has the connector switched on, and stops at once when it is switched off.
- Saves the last snapshot to disk, so a restart does not replay old news.
- On the very first poll it only records a baseline: no flood of alerts about things that were already
  true.
- Backs off on errors (doubling, up to 15 minutes), waits out `Error::RateLimited { retry_after }`, and
  checks only every 10 minutes when the user must act (`Error::Unavailable`, `Error::Auth`).
- Reports status (enabled, last successful poll, error, items watched) to the settings window.

## Steps

1. **`crates/connectors/src/<id>.rs`**: implement `Connector`. Keep `poll` to one request when the
   service allows it, and turn its answer into a `Snapshot` in a pure function you can test.
2. **`crates/connectors/src/lib.rs`**: add `pub mod <id>;` and the connector to `all()`.
3. **`ui/src/connectors.ts`**: add `{ id, name, about }`. `about` says, in one sentence, what it watches
   and how it signs in.
4. **Links**: an event's `url` is shown only if `core::SafeUrl` accepts it (https, allowed host). Add
   the service's host to `HOSTS` in `crates/core/src/safe_url.rs`.
5. **Tests**: feed recorded answers to your snapshot function and check `diff` for each kind of news,
   and for "nothing changed means no news".

## Credentials

Prefer the service's own CLI that the user is already logged into, as GitHub does with `gh`: the app
never handles a token at all. When there is no such CLI, the token goes in the OS keyring (Secret
Service, Credential Manager), never in a file, and never in a log.

## Event keys

`Event::key` identifies the *news*, not just the thing: `pr:owner/repo#12:ci-failed`. The island
replaces an alert with the same key instead of stacking duplicates, and a new failure on another
commit (`branch:owner/repo:ci-failed:<sha>`) is a new alert.
