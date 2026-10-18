# Decisions

Architecture decision records: one file per topic that shapes the project, with the rule in force,
the context it was made in and what it costs. Code and guides say *what* the app does; these say
*why*.

| Topic | Record | Status | Folded into it |
|---|---|---|---|
| How we decide | [0001](0001-record-decisions.md) Record decisions as ADRs | Accepted | |
| Stack | [0002](0002-rust-backend-no-sidecars.md) Rust for every backend part; no Python sidecars | Accepted | |
| The hook | [0003](0003-never-block-an-agent.md) The hook never blocks an agent | Accepted | |
| Permissions | [0004](0004-a-human-answers-permissions.md) Who answers a permission | Accepted | 0014, 0017, 0025, 0026 |
| Permissions | [0027](0027-an-open-reply-for-any-tool.md) An open reply for any tool that asks for one | Proposed | |
| Agent configs | [0005](0005-agent-configs-backup-diff-click.md) Agent configs, and the agents we support | Accepted | 0029 |
| Actions | [0011](0011-only-actions-that-work.md) Offer only the actions the agents allow | Accepted | |
| Zeca | [0018](0018-zeca-as-an-agent.md) Zeca: optional, apart from the flock, rules before models | Accepted | 0010, 0012 |
| Network and secrets | [0019](0019-requests-on-the-users-behalf.md) Network, secrets and telemetry | Accepted | 0006, 0016 |
| Releases | [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) Releases: small and often, 0.2.0 at the N2 gate | Accepted | 0007, 0015 |
| Surfaces | [0021](0021-four-sizes-one-nest.md) Surfaces: one core, four sizes, a card answered where you are | Accepted | 0008, 0009, 0022, 0023, 0024, 0030 |
| Voice | [0028](0028-spoken-replies-through-a-realtime-model.md) Voice: local listening, opt-in spoken replies | Accepted | 0013 |
| Tasks removal | [0031](0031-tasks-leave-vults.md) Tasks leave Vults; releases follow N0 to N4 | Accepted | |
| Dictation | [0032](0032-dictation-into-other-apps.md) Dictation into other apps | Accepted | |

A number in the last column was a record of its own until 2026-10-12; an old commit or PR that
cites it means the record it was folded into. Numbers are never reused: the next topic is 0033.

## Writing one

A decision on a topic that already has a record is an edit to that record, in its own PR, with a
dated line in its *History*. Only a new topic gets a new number: copy the shape of an existing
record (**Status**, **Context**, **Decision**, **Consequences**), number it next in line and add it
to the table. Write or edit one when a choice changes a rule in `CLAUDE.md`, the architecture, the
release plan or what the app promises users ([0001](0001-record-decisions.md)).
