# Connections prototype scope

These instructions apply only to `design/connections-local/`.

## Read before changing anything

Read `design.md` and `functions.md`, then compare the relevant production renderer.
The existing Vults application is the baseline. This prototype is an incremental
proposal for **Connections > Local environment**, not a replacement design for Vults.

Relevant sources:

- `../../CLAUDE.md` and `../../docs/adr/0021-four-sizes-one-nest.md`.
- `../../ui/src/surfaces/nest/connections.ts`, `render.ts` and `nest.css`.
- `../../ui/src/island/chat.ts` for draft preparation and existing conversations.
- `../../app/src/connectors.rs`, `../../crates/connectors/src/lib.rs` and
  `../../docs/contributing/connectors.md` for runtime ownership and contracts.

## Authorized changes

- Refine this local-environment prototype: Projects, Problems, Servers, Containers,
  Ports and Events, their filters, inspectors and empty states.
- Refine scoped Docker/Podman command previews, existing-container lifecycle
  simulations, Compose/pod previews, HTTP monitoring drafts and diagnostics.
- Prepare an editable Zeca draft from the selected service's bounded context.
- Update the specification and the runnable check to reflect these exact flows.

## Boundaries

- Do not redesign Flock, Tasks, Chat, Activity, Settings, approval cards, the shell,
  Mini/Quick behavior, agent profiles, skills or memory from this prototype.
- Flock is deliberately absent from this preview. This does **not** remove Flock
  from Vults or authorize changing its production navigation or implementation.
- The disabled existing navigation entries are context only. The preview's Chat
  button opens the simulated Zeca draft; it is not a new Chat specification.
- Preserve the production GitHub connector and its full tabs, state and behavior.
  The sample GitHub rows here are reference data, not replacement requirements.
- Reuse current components and owners when proposing implementation. Call out
  concrete mismatches instead of copying this page over the whole application.
- Do not reinterpret CLI availability as a connected or authorized engine. Keep
  target connection, rootless/rootful identity and discovered capabilities explicit.
- A command preview, diagnostic, log or model answer does not authorize execution.
  Production must retain existing user actions, permissions and Settings ownership.
- No new dependencies, production changes, CLI execution against real containers,
  model calls, notifications or external requests are authorized by this artifact.
- Future ideas in `design.md` are research options, not an implementation mandate.
- Keep changes in this directory unless the user explicitly expands the task.
  Preserve other prototypes and concurrent work. Do not commit without a request.

## Files and validation

`index.html` is the interactive preview. `design.md` defines the product scope;
`functions.md` maps every interaction to its proposed CLI/backend and Zeca contract.
The preview reuses the repository's Nest stylesheet and Zeca asset by relative path.
Keep the review frame separate from product requirements.

Run `node design/connections-local/check.mjs` from the repository root. It checks
scoped browser interactions without calling Docker, Podman or a model. It must
remove its temporary captures on success and failure. Do not retain generated
screenshots or build caches here, and do not delete shared dependency caches or
files owned by other work.

Write new source comments and documentation in English, following `CLAUDE.md`.
The existing Portuguese demo labels are review copy, not a separate production
translation system. A production implementation must use the current i18n flow.
