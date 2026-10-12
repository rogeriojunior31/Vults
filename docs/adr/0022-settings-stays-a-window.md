# 0022. Settings stays a window of its own

**Status:** Accepted, 2026-10-12. Amends [0021](0021-four-sizes-one-nest.md). Superseded by
[0023](0023-settings-a-webview-of-the-nest.md), 2026-10-12: Settings is a webview of the Nest's window.

## Context

[0021](0021-four-sizes-one-nest.md) moved Settings inside the Nest, behind its gear, to save a
webview. But Settings is the one window the app trusts with what changes how Vults and the agents
behave: `settings_page` (`app/src/lib.rs`) and `policies::from_settings` refuse those calls from any
other window. That trust holds because Settings shows no agent's text: a reply, a step or a task an
agent wrote can carry words meant to steer whatever reads them, and the Nest shows all of those.

## Decision

- Settings stays a window of its own, built when opened and gone when closed. The Nest's gear and
  the island's open it, at a page when asked (`open_settings(section)`).
- Only that window may:
  - write an agent's config ([0005](0005-agent-configs-backup-diff-click.md));
  - save or clear a key;
  - make or take a policy ([0014](0014-consent-before-autonomy.md));
  - pair, revoke or reach a phone, whose signed answers count as a click
    ([0017](0017-a-signed-phone-decision-is-a-click.md));
  - turn on a request made on the user's behalf, such as the web, memory and sleep
    ([0019](0019-requests-on-the-users-behalf.md)).
- Its pages take the Living Flock prototype's titles: General, Agents, Chat & voice, Approvals,
  Projects, Activity, Connectors, Phone, Flock & Zeca, About.

## Consequences

- An open Nest with Settings open is two webviews beside the island, not one.
- A window that shows agents' text never gains those powers by being merged with Settings; a new
  surface that needs one of them opens Settings at its page instead.
