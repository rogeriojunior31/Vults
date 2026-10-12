# 0026. Who answers the chat's cards is the user's choice

**Status:** Accepted, 2026-10-12. Amends [0004](0004-a-human-answers-permissions.md) and
[0014](0014-consent-before-autonomy.md) for Zeca's chat only.

## Context

Zeca's chat asks before every command or edit its CLI wants to run, and only *Always in this
chat* (one exact command, until the conversation ends) or the user's own Claude Code allow rules
skip a card. Policies answer the flock's cards but not the chat's. A user who works with Zeca all
day answers card after card for calls they would allow anyway, and asked to choose: their policies,
or no card at all.

The chat is not one of the flock's sessions. It runs in a folder the user picked, on a CLI Vults
started itself, and its own cards are shown and answered in Vults only. What it lets run is the
user's call, as long as the choice is theirs, deliberate, visible and on the record.

## Decision

Settings → Chat & voice → **Who answers Zeca's cards** has three choices:

- **Asks first** (the default): a card for every call no *Always* rule covers, as before.
- **Your policies**: the policies the user accepted (ADR 0014) answer the chat's calls too, exactly
  as they answer Claude Code's on the island: a deny denies, an allow allows (Claude Code's calls
  only, inside the conversation's folder), and the rest asks.
- **Full access**: every call runs with no card, except:
  - a destructive call (`rm`, a forced push) always asks, as [0025](0025-destructive-calls-always-ask.md)
    holds for rules and policies;
  - what a policy denies is denied, and what a policy asks first still asks;
  - once the conversation has read content from outside (a web page, a fetch, an MCP tool), its
    cards ask again for the rest of that conversation: words from a page must not run commands
    nobody saw.

More access is given only in Settings, like a policy; the chat's own picker shows the choice and
can only give some back (Full access to Your policies or Asks first). Every answer the choice gives
is in the audit log, as the policy's (with its consent) or as `full-access`, and the chat says what
ran on whose word. The flock's sessions are untouched: their cards keep rule 2 as ADR 0014 wrote it.

## Consequences

- `CLAUDE.md`'s rule 2 names the chat's choice beside the other ways a card is answered.
- Full access trusts the model in the chat's folder: a mistaken edit or a `git push` runs (only
  what can't be undone still asks). The setting says so, the chip in the chat turns red, and a
  policy's deny lines remain the way to keep some actions out of reach.
- The chat's CLI still runs with `--permission-mode default`: the answers come from Vults, so the
  audit log, the outside guard and Stop keep working.
