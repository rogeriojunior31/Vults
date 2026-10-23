# Connectors

In the Nest, Connections is a triage list beside the selected detail. Repository marks identify
rows, the detail shows the change bar and individual checks, and **Refresh** sits beside the sync
status. Checks start open; your later disclosure choice survives refresh. Narrow windows open
detail as a separate view and return to the selected row. Merging still needs its own confirmation.

Connectors put news from outside services on the island. Each one is off until you switch it on in
**Settings → Connectors**.

## GitHub

Every five minutes it checks, and every minute while CI checks are still running:

- your open pull requests: checks failed or passed, approved, changes requested;
- pull requests where your review is requested;
- the checks on the default branch of the ten repositories you own that were pushed most recently,
  and the latest release of each.

The same single query also brings what Room's **Connections** tab shows of each row: a pull
request's branch, size, reviewers and last update, and each check by name ([The Nest](nest.md#connections)).
Releases are only listed there: never an alert, never on the island.

It uses the GitHub CLI you are already logged into (`gh auth login`), so Vults never sees a token.
The first check only learns how things are; alerts start with the next change. Click an alert to
open it on GitHub, × to dismiss it. Passing checks replace a failure, and a newer review decision
the older one.

After an error it tries again in 15 minutes for a GitHub rate limit, 10 when `gh` is missing or
signed out, and 4 to 15 (doubling) otherwise. Opening the island or the card checks sooner when the
last check is over a minute old, except under a rate limit. GitHub is checked at most 3,000 times a
day; past that, **Settings → Connectors** says **Budget spent**. If GitHub returns partial data
(for example, a repository protected by SAML), Vults keeps the last complete snapshot and shows an
error. It does not treat missing fields as proof that a pull request or repository is absent.

### The GitHub card

Once GitHub has answered, **Now** in Quick gathers the rows that need an action. A row opens
its selected detail in the Nest's **Connections**; **Open in GitHub** there opens its link.
Connections keeps the full list, including quieter rows and releases:

![The GitHub card: your pull requests, a review waiting, default branches, and a failed check below](../assets/island-github-card.png)

- **Your pull requests**, most recently updated first: a dot for the checks (green passing, amber
  running, red failing) and the review (*Approved* or *Changes requested*);
- **Waiting for your review**;
- **Default branches**: the latest commit on each, with how its checks went. Repositories without
  checks are left out.

Select a row for its detail, then choose **Open in GitHub** to follow its link. When a pull request is merged or closed, or a review request
withdrawn, its alerts go with it.

With Zeca on, a row can ask him about it:

- **Review with Zeca**, on a review waiting for you: he reads the diff and tells you the risks
  before you approve;
- **Ask Zeca why**, on failing checks: he reads only the failed step's log and tells you what broke
  and where.

Each opens the chat with the question written in the composer. Nothing is asked until you send
it, so you can change it first.

**Squash and merge** sits on your own pull request once it is approved and its checks pass. The
first click only asks, under the row, what the second will do; **Merge** then squashes it into its
base branch and deletes its branch, through your `gh` login, and the card asks GitHub again, so it
leaves with the next answer. It merges only the commit the card showed green: if someone pushed
since, GitHub refuses. If GitHub refuses (a branch rule, a conflict), the row says why. Each merge writes a line
in the local trace, and nothing else is merged on its own.

**Seen** on quiet Now news hides that row until its checks, review or title changes. **Mark all
seen** does this for all quiet connection news in Now. Requests and failures remain visible;
marking seen never approves, merges or changes GitHub. The Nest's Connections keeps the rows.
The marks are stored on this computer only.

A failed connector poll keeps the last loaded rows and the last-sync/error message in Connections.
A permission card stays in front of Quick until answered or explicitly skipped; Now opens afterward.

What the last check saw (the titles, links and check states the card shows, nothing else) is kept
in `connectors/github.json` in the app's data folder (`~/.local/share/vults/` on Linux), so a
restart does not alert again about old news.


### For Zeca

Connections has **For you** and **For Zeca** views. Switching views keeps the selected pull
request and filters. **For Zeca** controls the built-in GitHub tools. `github_snapshot` reads the
last synced card without a request and is available in API Chat and realtime voice. In supported API Chat providers,
`github_account` identifies the account and host selected by `gh`, `github_search_repositories`
finds repositories visible to that account, and `github_repository` reads metadata for one exact
accessible repository. `github_work_item` reads one issue or pull request by exact repository and
number. Pull requests
include bounded files, latest reviews, inline review threads, checks and merge-readiness fields.
Separate tools read a pull request diff, Actions runs, jobs and redacted logs, or active rules for
one branch. Each operation fetches only its own context. Live GitHub reads share a switch that is
off by default and requires the GitHub connector to be on. Both choices are saved on this computer
and checked on every call, including already-open conversations. A failed save restores the
previous choice. Neither choice silently enables the GitHub connector.

The cached snapshot makes no GitHub request. Each live read goes through the shared network gate
and local trace. Search returns at most ten repositories per cursor page. Discussion returns
five comments per page; changed files, reviews, review threads and checks have separate cursors
and bounded pages. Actions runs and jobs return ten items per page; branch rules return at most
100 per page. Diffs and job logs are separate reads capped at 48 KiB of returned text, with log
secrets redacted. Chat reduces oversized diff/log text further to keep its 16 KiB JSON envelope
valid and marks that coverage as partial; other oversized context fails explicitly. Results
include repository, source URL when GitHub supplies one, retrieval time and explicit page or
byte coverage. A partial GraphQL response or malformed page fails instead of being presented as
complete. Every page is separately budgeted. Job logs verify that the job belongs to the exact
run attempt; the job lookup and log download each pass the gate. The view shows recorded calls
in the last 24 hours and the last recorded use; **Open traces** opens the local Activity traces.
Missing records are shown explicitly. Live reads and connector polling share a 3,000-request
limit per 24 hours. CLI command access is controlled separately in Chat.

The full live-read tools are offered only in Zeca's API Chat for Anthropic, OpenAI, Google
Gemini, OpenRouter, Groq, DeepSeek, Mistral and xAI. Local Ollama and LM Studio, CLI chats and
specialist chats do not receive them. Each result's `source.mode` distinguishes `live` from
`cached_snapshot`; Activity records the same mode with the tool input. A successful live read
marks the conversation as containing outside content, so the existing approval policy can
restore cards.

In supported API Chat providers, Zeca can propose creating or editing an issue, posting or
editing a comment, adding or removing labels, replacing assignees, and closing or reopening an
issue. Every operation is a distinct tool and opens the existing approval card with the
repository, issue, operation and exact requested content. The card cannot be saved as an
**Always** choice. Before the click, Vults reads the active account, host and current target and
checks repository write permission; after the click it repeats those checks and refuses stale
targets or changes to the approved request. The binding includes immutable account, repository,
issue and comment IDs. Pull requests cannot be changed through these issue tools. Writes
currently require repository push access and a repository that is not archived. Duplicate checks
read up to 1,000 issues or comments; if coverage is incomplete, perform the change on GitHub
instead.

After a write, Vults reads the target again and reports only a verified result. If a request
fails or times out, it rereads the target and never blindly repeats the write. A comment that is
already present and other exact duplicate changes are rejected. Every approved attempt records
its request hash and outcome in the local audit log; cancellation records an uncertain outcome.
These operations use the same off-by-default live GitHub switch, connector switch, request
budget and network gate as live reads. The existing confirmed **Merge** action on your pull
request remains available in **For you**.

### Pull request changes

The same live GitHub switch also enables typed proposals in supported API Chat providers:

- `github_pull_request_create` creates a PR or draft from two existing branches in the same
  repository, with an exact title, body and draft choice.
- `github_pull_request_edit` changes the title, body or base branch.
- `github_pull_request_ready` marks a draft ready for review; this may request code owners.
- `github_pull_request_draft` converts an open PR back to a draft.
- `github_pull_request_request_reviewers` requests the exact user logins and team slugs shown
  on the card. Review requests notify their recipients.
- `github_pull_request_review_comment`, `github_pull_request_review_approve` and
  `github_pull_request_review_request_changes` submit distinct review decisions with the exact
  approved body and optional inline comments. Approval of a conversational draft never submits
  a review; each decision requires its own human card.

The card shows the repository, target PR, current base/head refs and commit SHAs, exact content
and recipients. The service rechecks the active account, repository permission, target identity,
refs and review context immediately before execution. Review submissions send the approved head
SHA as `commit_id`. Inline comments require a current changed file, `LEFT` or `RIGHT` side and
an existing line in its current text patch. Stale lines, unavailable patches and right-side
comments on removed files are rejected.

Changes to existing PRs require an open PR with its head in the same repository. Fork heads
are rejected; creating branches or pushing commits is not part of these operations. A request
may contain up to ten single-line inline comments. Each context or duplicate collection follows up to
1,000 items; incomplete coverage refuses the operation. An existing pending review must be
finished or discarded on GitHub first, so its content is never silently reused.

Every change uses the existing human card, per-request network gate and audit log. The remote
state is read again after execution, including after a timeout or request error, without blindly
repeating the write. If refs or remote state change during a request, the result may be uncertain
and must be inspected before retrying. Readback verifies a result; it cannot make branch-based
GitHub endpoints atomic. Merge proposals use the separate controls described below.

### Actions controls

In supported API Chat providers, `github_actions_run` reads the latest workflow run, or an exact
historical attempt when `attempt` is provided. Runs and jobs report queued, running, waiting,
cancelled and completed phases explicitly. Logs are secret-redacted, limited to 48 KiB with an
explicit truncation flag, and further bounded by Chat's 16 KiB tool-result limit. Unavailable logs
are reported as unavailable, never as an empty successful log.

Each control requires its own human card with repository, run ID, latest attempt and exact commit:

- `github_actions_rerun_failed` reruns failed jobs and their dependent jobs.
- `github_actions_rerun_run` reruns the entire workflow, including jobs that already passed.
- `github_actions_rerun_job` reruns one selected failed job and its dependent jobs; `jobId` is required.
- `github_actions_cancel` requests normal cancellation of an active attempt.

Reruns can consume Actions minutes. Debug logging is disabled. Dependency IDs are not enumerated by
the API; the card explicitly includes dependent jobs in its scope. Vults never force-cancels a run
or approves a deployment through these tools. The account, immutable repository identity, attempt,
commit and selected scope are checked again after approval. Repository push access is required;
GitHub can still reject a token without Actions write permission.

An accepted request is not necessarily completed. Results distinguish `verified`,
`accepted_pending_verification`, `rejected`, `uncertain` and `completed_without_cancellation`, and
include the observed run phase. A verified rerun means a new attempt was observed, not that its
jobs passed. A cancellation is verified only when that exact attempt is observed cancelled.
Every request uses the existing connector/live-tool switches, budget and network gate.

Before sending a control, Vults atomically records a durable receipt in the local audit log.
All rerun variants share one receipt per host, immutable repository, run and attempt; cancellation
has a separate receipt. Reservations survive restart. Accepted, reserved and uncertain controls
cannot be sent again for that attempt. A definite rejection permits a fresh proposal and human
approval. Vults rereads after a request without repeating the POST. If verification is pending or
unavailable, inspect the latest run on GitHub rather than resending the control.

### Safe merge and branch deletion

In supported API Chat providers, `github_pull_request_merge` proposes a merge with the exact
repository, PR number, lowercase 40-digit `headSha` and `method` (`merge`, `squash` or `rebase`).
The existing human card shows the account, refs, commits, checks, review decision and applicable
rules. It cannot become an Always rule. After the click, Vults rereads the account, permissions,
head/base commits, readiness, active rulesets and classic branch protection. A changed binding
requires a new card. Failing or pending checks, missing reviews, unresolved discussions, conflicts,
unknown readiness and incomplete coverage refuse the proposal. Fork heads and stacked PRs are
not supported. Check and discussion coverage is limited to 100 entries each; active rules to
1,000. Missing permission to inspect protection fails closed.

Merges use GitHub's asynchronous merge API (version `2026-03-10`) with the approved head SHA and
`bypass_rules: false`. Older servers without this API are unsupported; Vults does not fall back
to an unchecked merge or admin bypass. Rules are enforced remotely even if they change after the
last inspection. A required merge queue uses its configured method; choose that method on the
card. A queue entry is reported as **queued**, never merged. Asynchronous acceptance is **pending**
until observed. A verified merge requires the exact PR/head and its merge commit to be read back.
GitHub's base ref and branch deletion APIs do not provide a transaction with the prior inspection;
readback can report uncertainty when remote state changes concurrently.

`github_pull_request_merge_status` reads current PR state without writing. Supply the optional
`requestId` UUID returned by an asynchronous request to also inspect its pending or failed result.
Pending, queued and uncertain requests are never automatically resubmitted. Durable audit receipts
block duplicate merges for the same repository ID, PR and head SHA, across methods and restarts.
Only a definite rejection permits another proposal; a reserved or uncertain receipt requires
inspection on GitHub rather than blindly retrying. No background status polling is enabled.

`github_pull_request_delete_branch` is a separate human-approved operation after a verified merge.
It requires the exact merged PR head SHA. Vults only deletes its unchanged head branch in the same
repository, excluding default/base branches, protected branches and branches with active rules.
It rechecks before deletion and verifies that the exact ref is absent afterwards. GitHub does not
offer an atomic compare-and-delete for refs; if that branch can receive concurrent pushes, keep it
and manage cleanup on GitHub. The merge operation never includes branch deletion. Repository
settings may independently delete branches automatically after merge.

The existing **Squash and merge** row action shares the checked service, request gate and durable
receipt. Its confirmation keeps the branch and its result distinguishes merged, queued and pending.
All Chat controls use the existing live GitHub and connector switches, per-request budget and trace.
No control changes a branch rule, updates a base branch or bypasses a protection.

Want another service? See [Adding a connector](../contributing/connectors.md).
