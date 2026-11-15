# Decisions

Six records, one per rule the whole project stands on. Records 0001 to 0004 hold the reasons for
the four rules in `CLAUDE.md`, with the same numbers; 0005 and 0006 hold the two boundaries every
feature crosses. Code and guides say *what* the app does; these say *why*. Plans live in the
[roadmap](../dev/ROADMAP.md), shipped work in the [changelog](../../CHANGELOG.md).

| # | Record | Rule in `CLAUDE.md` | Before 2026-11-15 |
|---|---|---|---|
| [0001](0001-hook-never-blocks.md) | The hook never blocks an agent | 1 | 0003 |
| [0002](0002-permissions.md) | Who answers a permission, voice included | 2 | 0004, with the voice rules of 0028 and 0032 |
| [0003](0003-agent-configs.md) | Agent configs, and the agents we support | 3 | 0005 |
| [0004](0004-network-and-secrets.md) | Network, secrets, telemetry, voice and audio | 4 | 0019, with the privacy rules of 0028 and 0032 |
| [0005](0005-zeca.md) | Zeca: optional, apart from the flock, rules before models | | 0018 |
| [0006](0006-surfaces.md) | Surfaces: one core, four sizes, a card answered where you are; only actions that work | | 0021, with 0011 |

## Old numbers

The records were renumbered on 2026-11-15. A commit, PR or changelog entry from before that date
uses the old numbers: read them through the last column above. Inside the records, *former NNNN*
always means an old number. Retired without a successor, their text in git history: 0001 (how to
write a record, below), 0002 (Rust, no sidecars, no GPL: now *Stack* in `CLAUDE.md`), 0020
(releases: [Releasing](../dev/releasing.md)), 0027 (an open reply for any tool: roadmap G1/G2) and
0031 (Tasks removal: the changelog). Older numbers folded before (0006 to 0030) are listed in each
record's *History*.

## Writing one

A record exists only for a rule that the whole project stands on: a rule in `CLAUDE.md`, or a
boundary every feature must respect. A choice that belongs to one feature is not a record; it goes
in that feature's guide or reference page. A plan, a phase, a release target or a removal is not a
decision either: those go in the roadmap or the changelog.

- **A new decision is an edit to the record of its rule**, in its own PR, with a dated line in its
  *History*. A new record needs a new rule in `CLAUDE.md` first; it takes the next number and a
  row here.
- **History records decisions, not deliveries.** A dated line says what rule changed and why. Which
  PR or step implemented it belongs in the changelog.
- **A rule in `CLAUDE.md` changes in its record first**, never in `CLAUDE.md` alone.
