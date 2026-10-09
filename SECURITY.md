# Security policy

Vults answers your coding agents' permission requests, edits their configuration, keeps API keys
and talks to them over a local socket, so we take security reports seriously.

## Supported versions

Vults is pre-1.0. Fixes land on `main` and in the next release; only the latest release is
supported.

## Reporting a vulnerability

Please **do not open a public issue**. Report it privately through GitHub: on the repository page,
open **Security → Report a vulnerability**.

Include what you can:

- the Vults version, your distribution and desktop (KDE Plasma, GNOME, Hyprland…);
- the steps to reproduce, or a proof of concept;
- what an attacker gains (answer a permission, read a key, write an agent's config…).

You can expect an acknowledgement within a week. Once a fix is released, the advisory is published
with credit to you, unless you prefer otherwise.

## Scope

In scope, for example:

- a permission answered without a human's click, or *Always* matching more than the exact command
  and project it was given for;
- another user, or a process outside your session, reaching the socket;
- an agent's config written without the dated backup, the diff and the click, or another tool's
  hooks lost;
- a secret written anywhere but the OS keyring, or reaching a log, an error or the screen;
- the hook blocking an agent, or Vults making a network request you did not turn on.

Out of scope:

- what an agent does once you allowed it;
- vulnerabilities in the agents themselves (Claude Code, Codex, Gemini CLI, Antigravity) or in
  `gh`, which should go to their maintainers.

How the app protects you is in [docs/safety.md](docs/safety.md).
