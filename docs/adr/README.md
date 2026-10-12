# Decisions

Architecture decision records: one file per choice that shapes the project, with the context it
was made in and what it costs. Code and guides say *what* the app does; these say *why*.

Records on one topic are consolidated into a living record ([0001](0001-record-decisions.md)): read
that one for the rule in force. The records folded into it stay as stubs, so old links still land.

| Topic | Living record | Status | Merged into it |
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
| Releases | [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) Releases: small and often, 0.2.0 at the Operations gate | Accepted | 0007, 0015 |
| Surfaces | [0021](0021-four-sizes-one-nest.md) Surfaces: one core, four sizes, one card host | Accepted | 0008, 0009, 0022, 0023, 0024 |
| Voice | [0028](0028-spoken-replies-through-a-realtime-model.md) Voice: local listening, opt-in spoken replies | Accepted | 0013 |

## Merged records

| # | Decision | Merged into |
|---|---|---|
| [0006](0006-keyring-no-telemetry.md) | Secrets only in the OS keyring; no telemetry | [0019](0019-requests-on-the-users-behalf.md) |
| [0007](0007-versions-to-0.5.md) | First release 0.1.0; one theme per version up to 0.5 | [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) |
| [0008](0008-one-core-many-surfaces.md) | One core, many desktop surfaces; the island is the only card host | [0021](0021-four-sizes-one-nest.md) |
| [0009](0009-presence-never-hides-a-card.md) | No presence mode leaves an acknowledged card unseen | [0021](0021-four-sizes-one-nest.md) |
| [0010](0010-zeca-is-optional.md) | Zeca is optional, and kept apart from the flock | [0018](0018-zeca-as-an-agent.md) |
| [0012](0012-rules-before-models.md) | Rules before models in the app's own decisions | [0018](0018-zeca-as-an-agent.md) |
| [0013](0013-voice-local-first.md) | Voice is local first; cloud only when chosen | [0028](0028-spoken-replies-through-a-realtime-model.md) |
| [0014](0014-consent-before-autonomy.md) | A new consent rule before any autonomy | [0004](0004-a-human-answers-permissions.md) |
| [0015](0015-everything-by-0.2-in-small-releases.md) | Everything planned lands by 0.2.0, in small 0.1.x releases | [0020](0020-0.2.0-at-wave-4-then-0.2.x.md) |
| [0016](0016-updates-asked-for-and-signed.md) | Updates: asked for, signed, one channel at a time | [0019](0019-requests-on-the-users-behalf.md) |
| [0017](0017-a-signed-phone-decision-is-a-click.md) | A decision signed on a paired phone counts as a click | [0004](0004-a-human-answers-permissions.md) |
| [0022](0022-settings-stays-a-window.md) | Settings stays a window of its own | [0021](0021-four-sizes-one-nest.md) |
| [0023](0023-settings-a-webview-of-the-nest.md) | Settings, a webview of the Nest's window | [0021](0021-four-sizes-one-nest.md) |
| [0024](0024-one-surface-at-a-time.md) | One surface at a time: the island steps aside for the Nest | [0021](0021-four-sizes-one-nest.md) |
| [0025](0025-destructive-calls-always-ask.md) | A destructive call always asks, an Always rule included | [0004](0004-a-human-answers-permissions.md) |
| [0026](0026-the-chats-access-is-the-users-choice.md) | Who answers the chat's cards is the user's choice | [0004](0004-a-human-answers-permissions.md) |
| [0029](0029-the-supported-agents.md) | The supported agents, and retiring one | [0005](0005-agent-configs-backup-diff-click.md) |

## Writing one

Copy the shape of an existing record: **Status**, **Context**, **Decision**, **Consequences**.
Number it next in line and add it to the table. A record is never rewritten to say something
else: a new one supersedes it, and the old one's status says by which. Write one when a choice
changes a rule in `CLAUDE.md`, the architecture, the release plan or what the app promises users.
A consolidation folds the records on one topic into its living record, decides nothing new, and
leaves each folded record as a stub ([0001](0001-record-decisions.md)).
