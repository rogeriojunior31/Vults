# 0018. Zeca: optional, apart from the flock, rules before models

**Status:** Accepted, 2026-10-10. Consolidated on 2026-10-12 from 0010 and 0012.
Amended on 2026-10-16 by [0031](0031-tasks-leave-vults.md): Tasks leave Vults.

## Context

The app does two jobs: it shows what the user's agents are doing (the flock), and it offers a
companion who chats, listens, reads the flock and remembers (Zeca). Some users want only the
first, and the second's power must not leak into it. A small local decision model was also
proposed for the app's own choices, but the events it receives are already typed, and such
models without fine-tuning score below a majority-class guess, with confidence we cannot
calibrate without collecting data ([0019](0019-requests-on-the-users-behalf.md)).

## Decision

- **Optional.** Everything about the flock works with Zeca off: sessions, cards, notifications,
  connectors, the Nest with its records and costs ([0021](0021-four-sizes-one-nest.md)). With him
  off: no Zeca chat, microphone, talk shortcut, *Chat…* in the tray, Chat tab or *Ask Zeca*; the
  island has its own idle look, and `zeca` starts no process, writes no file and makes no request.
- **Specialist Chat uses the existing Chat UI.** A profile opens from Flock into the existing Chat
  tab, selected as the recipient. Switching recipients retains each conversation and draft. A
  specialist remains available with Zeca off without turning him on. The recipient is the immutable
  profile and skill snapshot. Claude Code uses isolated settings in the empty managed workspace or
  a project explicitly selected before the first turn. Configured API/local providers use only the
  pinned instructions and their own messages, without tools or project access. Codex remains blocked
  pending an isolated specialist driver. No flock mention, companion context, MCP, user setup or
  Zeca memory is attached. The user chooses Ask, accepted policies or full access separately
  for each profile; the default is Ask, and changes apply to the next tool call. Destructive actions,
  policy asks and actions after reading outside content require a card; policy denies block the call.
  Zeca's Chat access setting cannot grant specialist access. Voice, Ask Zeca and local companion
  commands are unavailable in this route. Native state, history and approval streams remain scoped
  to the selected profile while the UI reuses the production ChatPanel.
- **Agent preferences are separate from immutable revisions.** Each profile keeps versioned local
  executor/model/effort defaults, independent of Chat access. Content and defaults save atomically;
  conflicts retain the draft. Defaults apply only before the first turn. API history is scoped to its
  profile, provider and model; it never adopts a companion prompt or history. Codex defaults may be
  saved but cannot run yet. A bound conversation keeps its original executor, model and source.
  Custom agents can be archived/restored without deleting revisions or history. Archived agents
  reject new turns and resets; an admitted turn may finish, with Stop and human cards still usable.
  Reading history remains available. Copies start with Ask, regardless of the source's access.
- **Where:** his brain is a `zeca` crate in the Connect layer, beside `chat`; his body (sprites,
  the island, the Nest) stays in Experience. No crate below `zeca` knows it exists; `app` wires it in.
- **What he reads:** the flock through a `FlockReader` built from the view, with no path to an
  `Intent`. Flock content is untrusted input: it may carry an injection.
- **What he does:** typed actions of his own; `zeca` does not depend on `vults-core`, so `app` maps
  each to `core`'s policy classes (read, external read, write, destructive, spend, external) and
  ledger, like any agent's. He never builds `Decide` or `DecideAlways`, by type, and never answers a
  permission, his own or another agent's ([0004](0004-a-human-answers-permissions.md)).
- **What he remembers:** a git repository of his own (through `gix`): every write is a commit,
  undone by a revert.
- **Code is the agents' work:** on the harness (an API key or a local model) he never runs code
  himself. Removing Tasks adds no substitute execution path. CLI-backed Chat and `/continue`
  keep their existing drivers and permission checks; his commands and edits in the chat ask the
  user like any chat.
- **His work is never a session:** his CLI processes carry `VULTS_ZECA=1`; the hook marks their
  events and `core` sends them to the chat's state (taint and activity), never to the flock. Any
  process could set that variable, so a marked event never hides or drops a permission request: it
  is still asked of the user, through the chat's approver.
- **Rules before models** in the app's own decisions:
  - What can be decided from typed state is decided by plain rules in `core`, with tests:
    notifications, attention, Zeca's look, which agent is installed and has quota.
  - Free text from the user (what they ask Zeca) goes to the chat model.
  - A learned classifier may come later only behind the same interface, only for free text, only
    local, and only if an evaluation set on this repository shows the rules fall short.
  - No model ever answers a permission.

## Consequences

- The companion's growth never changes who answers a permission; decisions stay exact, testable
  and explainable.
- `scripts/check-layers.sh` places `zeca` in Connect; a compile-fail test pins that `FlockReader`
  cannot reach an `Intent`.
- The protocol gained a field for the marked event (version 6).

## History

- 0010, 2026-10-04: Zeca is optional and kept apart from the flock;
  his handle cannot build a decision.
- 0012, 2026-10-04: rules before models in the app's own decisions.
- 2026-10-16, [0031](0031-tasks-leave-vults.md): remove Tasks and routing; keep `/continue` and
  costs, and keep the flock independent of Zeca (D7).

- 2026-10-21: UX8 starts with an inert built-in profile catalog in Connect. Profiles describe roles, instructions and appearance, not access. Reading them needs neither Zeca nor his memory; this slice provides no execution or private-context route.
- 2026-10-21: UX8 adds user-owned copies of presets, saved as immutable local database revisions with stale-edit rejection. Name/instruction edits grant no execution, memory access or permission authority; preset provenance and appearance remain pinned.
- 2026-10-21: Profile copies can revise species and a fixed accessory using the existing character catalog. Each revision retains its own appearance; earlier revisions and immutable presets do not change. Seasonal appearance, skills and execution are outside this slice.
- 2026-10-21: Specialist conversation storage now freezes an explicit profile source/revision and complete instructions/appearance at creation. These records cannot carry Zeca's frozen context or be opened/listed by his current Chat route. Specialist execution and availability with Zeca off remain disabled until their own isolation contract is implemented.
- 2026-10-21: Add a Claude Code specialist runtime constructor that keeps only the pinned brief, isolated settings and the existing human approval path. It removes companion MCP, extra directories and API history, rejects companion context/tools and blocks unsupported providers before a turn starts. This primitive does not enable profile Chat, memory recall, journal writes or specialist availability with Zeca off; native routing remains closed.
- 2026-10-21: The native turn now fixes and checks recipient metadata before memory recall. Profile-bound runtimes keep their exact snapshot, source and executor, skip companion recall/context/MCP/journal callbacks, and cannot feed companion memory extraction. Reset/open/stop/delete clear or replace recipient metadata with the runtime. User-facing specialist routing remains closed pending recipient UI and command/voice transitions.
- 2026-10-22: Specialist access is chosen per profile (Ask, accepted policies or full), defaults to Ask and is independent of Zeca's Chat setting. Destructive actions, policy asks and actions after outside content still require a card; policy denies block the call.

- 2026-10-22: Integrate specialists into the existing Chat tab with recipient selection and the production ChatPanel. Keep native runtimes, drafts, histories and approvals scoped by profile; access changes apply to the next tool call.

- 2026-10-24: Persist versioned agent defaults and custom-agent archive/restore independently of immutable revisions and access. Apply Claude defaults only to unbound conversations; reject unsupported specialist executors explicitly. Preserve history, admitted turns, Stop and pending human decisions when archived.

- 2026-10-25: Enable specialist API/local conversations on both supported wires with an exact pinned system prompt and no tools or companion memory. Allow an explicitly selected project for Claude Code, validate and pin its source, and retain executor/model/history through reopening and changed defaults. Codex remains unavailable pending isolated configuration handling.

- 2026-10-25: Allow a conversation-local executor/model/effort override before the first send. Its native source generation invalidates queued older actions; draft text and profile defaults/access remain untouched. New/Open/history clear the unsent override. Reopening a sent conversation retains its stored executor and source. Codex remains blocked until its isolated runtime is qualified.
