# Decisions

Architecture decision records: one file per topic that shapes the project, with the rule in force,
the context it was made in and what it costs. Code and guides say *what* the app does; these say
*why*. Plans live in the [roadmap](../dev/ROADMAP.md), shipped work in the
[changelog](../../CHANGELOG.md); neither belongs here.

| Topic | Record | Status | Folded into it |
|---|---|---|---|
| Stack | [0002](0002-rust-backend-no-sidecars.md) Rust for every backend part; no Python sidecars | Accepted | |
| The hook | [0003](0003-never-block-an-agent.md) The hook never blocks an agent | Accepted | |
| Permissions | [0004](0004-a-human-answers-permissions.md) Who answers a permission | Accepted | 0014, 0017, 0025, 0026 |
| Agent configs | [0005](0005-agent-configs-backup-diff-click.md) Agent configs, and the agents we support | Accepted | 0029 |
| Actions | [0011](0011-only-actions-that-work.md) Offer only the actions the agents allow | Accepted | |
| Zeca | [0018](0018-zeca-as-an-agent.md) Zeca: optional, apart from the flock, rules before models | Accepted | 0010, 0012, 0031 |
| Network and secrets | [0019](0019-requests-on-the-users-behalf.md) Network, secrets and telemetry | Accepted | 0006, 0016 |
| Surfaces | [0021](0021-four-sizes-one-nest.md) Surfaces: one core, four sizes, a card answered where you are | Accepted | 0008, 0009, 0022, 0023, 0024, 0030, 0031 |
| Voice | [0028](0028-spoken-replies-through-a-realtime-model.md) Voice: local listening, opt-in spoken replies | Accepted | 0013 |
| Dictation | [0032](0032-dictation-into-other-apps.md) Dictation into other apps | Accepted | |

A number in the last column was a record of its own; an old commit or PR that cites it means the
record it was folded into (0031, Tasks removal, went to both 0018 and 0021). Three numbers were retired on 2026-11-15 and live only in git history:
0001 (how to write a record: the rules below), 0020 (releases: now in
[Releasing](../dev/releasing.md)) and 0027 (an open reply for any tool: a proposal, now roadmap
task G1/G2). Numbers are never reused: the next topic is 0033.

## Writing one

A record holds a rule that outlives the work that made it. Write or edit one when a choice changes a
rule in `CLAUDE.md`, the architecture or what the app promises users. A plan, a phase, a release
target or a removal is not a decision: those go in the roadmap or the changelog.

- **One topic, one living record.** A new decision on a topic that already has a record is an edit
  to that record, in its own PR, with a dated line in its *History*. Only a new topic gets a new
  number: copy the shape of an existing record (**Status**, **Context**, **Decision**,
  **Consequences**, **History**), number it next in line and add it to the table.
- **History records decisions, not deliveries.** A dated line says what rule changed and why. Which
  PR or step implemented it belongs in the changelog.
- **Consolidate instead of amending across records.** Records on the same topic are merged into one;
  the merge decides nothing new, drops only what was already superseded, and every link moves to
  the living record. Git keeps the old text.
- **A rule in `CLAUDE.md` changes in its record first**, never in `CLAUDE.md` alone.
