# Connectors

Connectors put news from outside services on the island. Each one is off until you switch it on in
**Settings → Connectors**.

## GitHub

Every five minutes it checks, and every minute while any of those checks is still running:

- your open pull requests: checks failed or passed, approved, changes requested. A push whose
  checks already finished when the next check runs still gets its alert;
- pull requests where your review is requested;
- the checks on the default branch of your ten most recently pushed repositories.

It uses the GitHub CLI you are already logged into (`gh auth login`), so Vultures AI never sees a token.
Opening the island checks again right away when the last check is more than a minute old (not while
it is waiting after an error). The first check only learns how things are; alerts start with the next
change. Click an alert to open
it on GitHub, × to dismiss it.

Want another service? See [Adding a connector](../contributing/connectors.md).
