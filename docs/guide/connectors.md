# Connectors

Connectors put news from outside services on the island. Each one is off until you switch it on in
**Settings → Connectors**.

## GitHub

Every five minutes it checks, and every minute while CI checks are still running:

- your open pull requests: checks failed or passed, approved, changes requested. A push whose
  checks already finished when the next check runs still gets its alert;
- pull requests where your review is requested;
- the checks on the default branch of your ten most recently pushed repositories.

It uses the GitHub CLI you are already logged into (`gh auth login`), so Vultures AI never sees a token.
Opening the island checks again right away when the last check is more than a minute old, so it also
retries soon after an error you fixed (say, after `gh auth login`); only a GitHub rate limit is
always waited out. The first check only learns how things are; alerts start with the next change.
Click an alert to open it on GitHub, × to dismiss it.

An alert stays only while it is the latest word on its story: checks that pass replace the failure
on the same pull request or branch, and a newer review decision replaces the older one. A review
requested again after it was withdrawn alerts again. When GitHub cannot answer for one organization
or repository (say, one that needs SAML sign-in), the rest still shows.

Want another service? See [Adding a connector](../contributing/connectors.md).
