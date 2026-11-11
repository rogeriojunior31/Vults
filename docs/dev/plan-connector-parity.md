# GitLab and Gitea parity

Requested 2026-11-10. GitHub is the behavior reference. Deliver equivalent native service features,
with the same confirmation, scope, audit and failure behavior. Do not advertise pending features.
Notion follows this work; it does not replace it. Keep GitHub and Linux regression coverage.

## Delivery sequence

| Slice | GitLab | Gitea | Acceptance |
| --- | --- | --- | --- |
| CP1: complete dashboard | Implemented with documented bounds | Implemented with documented bounds | Diff sizes, reviewers, changes requested, head-bound jobs/checks, default branches, releases, no repeated news |
| CP2: cached chat snapshot | Pending | Pending | Independent Settings/profile permissions, scoped cached reads in API Chat and realtime, no new network request |
| CP3: live reads | Pending | Pending | Account, repository/search, issues/PRs, comments/reviews, diff/files, CI runs/jobs/logs and branch rules; explicit bounded pagination |
| CP4: issue writes | Pending | Pending | Create/edit, comments and comment edits, labels, assignees, close/reopen; human preview, revalidation and audited readback |
| CP5: pull-request writes | Pending | Pending | Create/edit, draft/ready, reviewer requests, reviews and inline comments bound to the reviewed head |
| CP6: CI controls | Pending | Pending | Retry failed jobs/run/job and cancel where supported; exact project/run/head binding and durable duplicate protection |
| CP7: merge controls | Pending | Pending | Confirm exact head, recheck permissions/reviews/rules/discussions/CI, no bypass; status readback and separate branch deletion |
| CP8: acceptance | Pending | Pending | Native Windows UI, Linux CI, API fixtures, real-service account/permissions acceptance when available |

## Invariants

- Every request passes its connector's switch, lock/pause checks, budget and trace. A multi-request
  operation admits each request separately, including validation and readback requests.
- Credentials remain in the CLI or OS keyring. Gitea actions bind the instance, credential generation,
  account, repository identity and object/head; reconnecting invalidates older previews.
- Remote writes need a reviewed human action. Durable receipts distinguish rejection, acceptance,
  verification and uncertainty; timeouts never cause an automatic resubmission.
- No UI merge flag is inferred from green CI alone. Server-side protections are enforced at execution.
  Missing/inaccessible rules or required checks refuse the action; no administrative fallback.
- Service differences remain explicit. GitLab pipelines and Gitea Actions keep their native meanings.
  Unsupported or unknown states cannot appear successful; future releases cannot appear draft.
- Partial responses retain the last complete board. Snapshot format changes rebaseline once.

## Current evidence

The CP1 GitLab query was validated without credentials against the public GitLab.com schema on
2026-11-10. Its measured complexity is below the unauthenticated limit; no personal account or write
was used. This validates query shape, not authenticated feature acceptance.

The running Windows app was checked with an offline CLI fixture: disabled means no CLI, enabling
is Settings-only, one bounded query produces MR/default-branch/release rows, and merge remains off.
Renderer bounds and hit tests cover 100%, 125%, 150% and 200%. In short windows, the detail retains
scrollable space; off-connector invitations no longer compress an active board. The empty state
and Manage connections still expose connector setup.

Gitea loopback fixtures cover complete PR/review/status/Actions/branch/release reads, per-request
admissions, enrichment deduplication, invalid account IDs, mismatched commits/attempts, missing Actions
coverage and incomplete pages. Review decisions use submission time. Background cadence accounts
for each poll's request count. Native WebView renderer fixtures cover Actions/Releases tabs and
failed-job hit tests at 100%, 125%, 150% and 200%, including a 480 by 360 viewport.

Gitea still needs a real test instance. Synthetic fixtures and renderer scale emulation do not replace
real permissions, physical Windows DPI/input or live merge-policy acceptance.
