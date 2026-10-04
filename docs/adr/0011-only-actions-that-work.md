# 0011. Offer only the actions the agents allow

**Status:** Accepted, 2026-10-04.

## Context

Quick actions on a bird were proposed: open terminal, view diff, approve, stop, rename. But the app
only reaches an agent while a permission or question hook waits for an answer, and it does not
own the agent's process. Jumping to a terminal raises the window on KDE and focuses tmux, kitty,
wezterm and herdr panes; elsewhere it cannot raise a window.

## Decision

A menu shows only what works today, and says when it cannot (*Open terminal* on a desktop where
we cannot raise windows). *Stop* is not offered until an agent gives a supported way to stop it
from outside; until then it is research. Preferences such as mute, pin and hide are kept per
project, because sessions leave after 10 to 30 minutes and are not saved.

## Consequences

- Fewer buttons than the mock-ups, but none that lies.
- Sessions the app starts itself (0.2.0) can be stopped, because there we own the process.
