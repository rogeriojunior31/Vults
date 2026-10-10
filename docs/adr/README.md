# Decisions

Architecture decision records: one file per choice that shapes the project, with the context it
was made in and what it costs. Code and guides say *what* the app does; these say *why*.

| # | Decision | Status |
|---|---|---|
| [0001](0001-record-decisions.md) | Record decisions as ADRs | Accepted |
| [0002](0002-rust-backend-no-sidecars.md) | Rust for every backend part; no Python sidecars | Accepted |
| [0003](0003-never-block-an-agent.md) | The hook never blocks an agent | Accepted |
| [0004](0004-a-human-answers-permissions.md) | A permission is answered by a human, or by an exact rule a human made | Accepted, amended by 0014 |
| [0005](0005-agent-configs-backup-diff-click.md) | Agent configs change only after a backup, a diff and a click | Accepted |
| [0006](0006-keyring-no-telemetry.md) | Secrets only in the OS keyring; no telemetry | Accepted |
| [0007](0007-versions-to-0.5.md) | First release 0.1.0; one theme per version up to 0.5 | Superseded by 0015 |
| [0008](0008-one-core-many-surfaces.md) | One core, many desktop surfaces; the island is the only card host | Accepted, amended by 0017 |
| [0009](0009-presence-never-hides-a-card.md) | No presence mode leaves an acknowledged card unseen | Accepted |
| [0010](0010-zeca-is-optional.md) | Zeca is optional, and kept apart from the flock | Accepted |
| [0011](0011-only-actions-that-work.md) | Offer only the actions the agents allow | Accepted |
| [0012](0012-rules-before-models.md) | Rules before models in the app's own decisions | Accepted |
| [0013](0013-voice-local-first.md) | Voice is local first; cloud only when chosen | Accepted |
| [0014](0014-consent-before-autonomy.md) | A new consent rule before any autonomy | Accepted |
| [0015](0015-everything-by-0.2-in-small-releases.md) | Everything planned lands by 0.2.0, in small 0.1.x releases | Accepted |
| [0016](0016-updates-asked-for-and-signed.md) | Updates: asked for, signed, one channel at a time | Accepted |
| [0017](0017-a-signed-phone-decision-is-a-click.md) | A decision signed on a paired phone counts as a click | Accepted |
| [0018](0018-zeca-as-an-agent.md) | Zeca as an agent: a brain in Connect that never answers a permission | Accepted |

## Writing one

Copy the shape of an existing record: **Status**, **Context**, **Decision**, **Consequences**.
Number it next in line and add it to the table. A record is never rewritten to say something
else: a new one supersedes it, and the old one's status says by which. Write one when a choice
changes a rule in `CLAUDE.md`, the architecture, the release plan or what the app promises users.
