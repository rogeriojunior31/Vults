# GitHub implementation plan for Luna

## Goal

This plan is prepared for Luna as the coding model. The runtime assistant remains Zeca.

Let Zeca work with a user's GitHub projects through Vults: find the right repository and
work item, read enough context to reason, propose or perform an explicitly authorized change,
then report checks and review state accurately. The plan covers the connector and model tools;
it does not grant Zeca access merely because a GitHub account is logged into `gh`.

## Existing contract

- GitHub uses the user's `gh` installation. Vults never reads or stores its token.
- Connector polling is off until enabled, passes through `app/src/gate.rs`, has a rolling
  request budget and writes local request traces.
- Model-visible GitHub snapshot is a separate cached read. It makes no request and its switch
  does not enable the connector.
- GitHub content is outside content. Preserve its source through the chat and tool pipeline so
  the established policy can restore approval cards after outside content is read.
- API read tools run directly. Typed connector writes prepare an exact request for the app's
  human approver and audit ledger; their ordinary `Tool::call` rejects execution. The provider
  tool loop cannot execute arbitrary commands.
- Specialist chats are isolated today. Do not quietly add repo context, MCP tools, or tools to
  that isolation boundary. A contract change starts with ADR 0018 and explicit user choice.
- Existing Chat CLI access to `gh` remains governed by the chat access choice. The built-in
  GitHub switch controls only Vults' GitHub tools; it must not claim to disable raw CLI access.

## Delivery sequence

### GH0 — Consolidate the existing Connections work

Finish the in-progress Connections change before expanding the capability. Keep the approved
For you / For Zeca audience switch, retain selection and filters, and show only real capabilities.
The current first step exposes the cached snapshot tool with a live persisted switch, local usage
traces, and an explicit unavailable state for issue creation and model-initiated merge. No
audience change or settings visit triggers a GitHub request.

**Accept when:** the existing Connections UI and tools pass their regression checks; saves are
persisted before authority changes; failed saves restore the old state; the connector and cached
tool switches remain independent; no unsupported write is represented as available.

### GH1 — Account, host, repository context, and access diagnosis

Show which `gh` account and host supplied the latest successful connector response. This label
must not be treated as proof of token scopes or access to a repository. Define a user-triggered,
budgeted way to refresh authentication health if needed; do not add a second background request
to every poll. Represent missing CLI, unauthenticated, rate-limited, inaccessible, and partial
responses distinctly. Keep repository access a per-operation result, since fine-grained tokens,
organization SSO and repository policy can differ.

**Accept when:** tests cover no CLI, no login, active account/host, expired or rejected auth,
rate limit, SSO/forbidden and inaccessible repositories. No credential, token, or token dump can
reach status, traces, model context, or UI. A failed account refresh preserves the last known
identity but marks it stale/error rather than presenting it as current.

### GH2 — Bounded, complete-enough read service

Replace the dashboard's partial snapshot as the only model context with typed reads for account,
repositories, issues, pull requests, comments, changed files/diff, reviews and review threads,
checks, workflow runs/jobs/logs, branch rules and merge readiness. Separate fetches by task so a
simple question does not fetch large diffs or logs. Follow pagination with explicit item, page,
byte and time limits. Return coverage, truncation, freshness and source metadata; never describe a
first page or partial GraphQL result as complete or infer absence from an omitted field.

Reuse existing cached polling data when it answers the question. A live read is an explicit tool
request through the shared connector service and network gate, with the GitHub switch, rolling
budget, local trace, cancellation and per-call timeout. Use typed arguments and bounded results;
do not construct shell command strings from model input.

**Accept when:** fixtures cover pagination, empty versus missing, partial GraphQL errors, null
nodes, truncation, stale cache, rate limits, inaccessible/private repositories, large files,
binary diffs, and malformed provider output. The model can cite repository, ref/commit, URL and
retrieval time for every returned result.

### GH3 — Give supported chats the read tools

Expose narrow, discoverable tools to supported Chat providers for repository search/selection,
issue/PR lookup, discussion/review context, diffs, and Actions status. Tools share the same typed
read service, limits and gate. Preserve provenance and outside-content taint across every result,
including comments and logs. Distinguish cached from live reads in the result and Activity trace.

Do not assume every chat provider supports tools. Do not claim GitHub tools are available in the
CLI or specialist route until that route has an explicit supported contract. A disabled connector
must not be bypassed by a live built-in read.

**Accept when:** tests prove exact tool schemas, per-call revocation, disabled/offline and budget
behavior, provider capability detection, provenance propagation, taint restoration of cards,
and no tool invocation on providers without tool support.

### GH4 — Issues and discussion writes

Support creating/editing issues and comments, labels, assignees, and close/reopen only as distinct
typed operations. Show Zeca's proposed target and exact content in the existing approval flow.
Re-read the target and permissions immediately before execution. Bind approval to the exact repo,
issue, operation and content hash. Record the result in the audit log and verify the remote state.
If a timeout leaves the result uncertain, reconcile by reading before offering a retry; never
blindly repeat a write.

**Accept when:** denied, stale, edited-after-approval, inaccessible, duplicate, timeout/unknown
outcome and successful operations are covered. The user can see exactly what will be written and
what actually happened.

### GH5 — Pull request drafts and reviews

Support draft/create/update PR, ready-for-review, reviewer requests, and submitted reviews as
separate actions. Bind each action to repository, base/head refs and expected head SHA. A review
comment must point to a current changed file and valid diff line; reject stale or invalid anchors.
Submitting an approval or requesting changes always requires a direct human decision on the exact
review body and target. Never infer a review decision from conversational approval of a draft.

**Accept when:** changed head, changed base, stale line, removed file, cross-repository head,
permission failure, cancel, duplicate and successful review cases are tested. The submitted review
matches exactly what the user approved.

### GH6 — Actions inspection and controls

Read runs, attempts, jobs and bounded logs with explicit truncation and secret redaction. Offer
rerun failed jobs, rerun a whole run and cancel as different operations. Bind them to repository,
run ID, attempt and commit SHA; show the exact scope in the approval card. Report queued/running/
cancelled/completed as separate states and verify after every request.

**Accept when:** wrong repo, wrong attempt/SHA, completed run, permission failure, timeout,
redacted logs, unavailable logs, uncertain outcome, duplicate action and each successful control
are covered.

### GH7 — Safe merge and branch rules

Before merge, re-read the exact PR head, required checks, reviews, merge state and applicable
branch/ruleset requirements. Never bypass admin or branch protection. Bind a human confirmation
to repo, PR, head SHA and merge method. Treat a merge queue as queued, not merged. Branch deletion
is a separate explicit choice. Verify the final remote state; for an uncertain response, read
before offering another merge.

**Accept when:** changed head, failing/pending checks, missing review, required queue, denied
permission, conflict, stale confirmation, timeout, already merged, optional branch deletion and
successful merge are all tested without bypassing protection.

### GH8 — Specialist integration by explicit contract

Only after the read and write service is stable, decide whether isolated specialists need GitHub.
Update ADR 0018 first. Define which profile, repository, tools, folder and approval policy the
user selected; grant no ambient access from Zeca, another profile, or a prior conversation. Keep
tool disablement live and auditable.

**Accept when:** no access is inherited across recipient/profile/repository changes, revocation
blocks in-flight and subsequent calls, and the exact grant is visible before the specialist starts.

### GH9 — Recovery, UI, documentation and release readiness

Integrate into existing Connections, Chat, Activity traces and approval cards. Keep UI additions
within approved production/prototype patterns. Document `gh` setup, host/account selection,
available tools, budgets, permissions, recovery, and limitations in English and Brazilian
Portuguese; update source hashes. Add controlled-repository end-to-end coverage and recovery for
app restart, network loss, account switch and GitHub errors. Review rate and byte budgets before
enabling any new background request.

**Accept when:** the controlled test repository completes the flows below; docs and translations
pass CI; a reviewer checks every external write path, and CI passes on Linux and Windows.

## End-to-end flows

1. Ask Zeca to find a repository, inspect an issue and summarize its discussion with source links.
2. Ask for a PR review; Zeca reads the exact diff, checks and existing reviews, then proposes a
   review the user can inspect before submission.
3. Ask Zeca to create or update an issue; the card names the exact repo, target and body, and the
   result is verified remotely after approval.
4. Ask Zeca to diagnose failed Actions; it reads only bounded/redacted logs and proposes a rerun
   of the selected failed job.
5. Ask Zeca to merge a PR; it rechecks head, checks and reviews, preserves branch protection,
   requests human confirmation, and reports merged versus queued accurately.
6. Switch account, disable GitHub, exhaust budget, lose network, or revoke tool access while the
   chat is open; later calls fail clearly and never use stale authority.
7. Restart after an uncertain write; Vults reconciles remote state before offering a retry.

## Current progress

- GH0: the For Zeca cached-snapshot switch, traces and unsupported-action states are implemented
  in the Connections implementation; regression checks pass. Visual acceptance is
  pending in the running app.
- GH1: the successful poll now carries account login and HTTPS host in its own metadata, and
  existing Settings shows them without another request. Tests ensure identity does not become a
  watched item or connector news. Authentication errors are typed; GraphQL partial responses now
  retain the previous complete snapshot and show an error instead of implying omitted resources
  do not exist. Repository-specific capability diagnosis remains open.
- GH2: implemented. `github_search_repositories` finds up to ten repositories visible to the
  selected `gh` account and returns account/host provenance. `github_work_item` reads one exact
  issue or pull request with bounded body, comments, changed files, latest reviews, inline review
  threads, checks and merge-readiness fields. Discussion, files, reviews, review threads and checks
  have independent cursors. Separate reads return a pull request diff, paged Actions runs/jobs,
  capped and redacted job logs, and active branch rules (including organization rules that apply to
  the branch). Text and result pages are capped; invalid or partial GraphQL/REST coverage is an
  error, not an empty result. Every live read shares the persisted off-by-default switch, requires
  the GitHub connector, passes the rolling request budget and local trace, rechecks revocation, and
  has a 30-second kill-on-drop timeout. Validation covers cursor pages, malformed/partial output,
  bounded text, outside-content tool registration and the realtime tool limit. Visual acceptance
  of the existing Connections controls remains pending in the running app.
- GH3: implemented. The API chat offers the typed GitHub read tools only to explicitly supported
  cloud providers; local model providers, CLI chats, specialist chats and realtime voice do not
  receive live reads. Every live result carries `source.mode: live` plus account/repository URL
  and retrieval metadata when available; the cached snapshot identifies `cached_snapshot`. Activity
  records the same mode with the call input. Successful GitHub reads use the existing outside-content
  taint, which persists with the conversation and restores its approval cards. Existing gate checks
  still enforce the independent GitHub and live-read switches, budget, per-call revocation and
  cancellation. Tests cover provider capability, tool schemas, taint, Activity mode, disabled reads,
  and realtime exclusion.
- GH4: implemented in the current branch. API Chat exposes nine typed issue-write
  tools. Each prepares a card with exact content and a request hash; the card cannot use Always.
  The service binds the account ID, host, repository ID, issue ID and current content; it rechecks
  that binding and permission before execution. Comment editing also binds its exact comment ID.
  Duplicate scans follow up to 10 pages of 100 items and refuse incomplete coverage. Mutation
  errors reconcile against remote state without retrying; cancellation keeps an uncertain audit
  outcome. The 2026-10-23 review corrected label DELETE, assignee replacement PATCH, the last check
  page, JSON transport limits, request accounting and settings refresh races. Fixture tests exercise
  actual request sequences, denial, stale identity/target, changed hashes, partial writes and
  unknown outcomes. Live account writes and native visual acceptance remain pending.
- GH5: implemented in the current branch. Eight distinct Chat tools propose PR
  creation/editing, draft/ready transitions, reviewer requests, and COMMENT/APPROVE/REQUEST_CHANGES
  reviews. The service shares GH4's account/repository permission check, gate and human-card flow.
  It binds the exact PR ID, base/head refs and SHAs, current content and review context; reviews
  send the approved `commit_id`. Inline comments validate the current changed file, side and
  line against its text patch. Fork heads and pre-existing pending reviews are refused. Remote
  verification handles mutation errors without retrying; cancellation retains an uncertain audit
  outcome with the approved account, host, refs and commit SHAs. Fixture tests cover exact HTTP requests, each successful operation, stale head/base,
  changed content/anchors, missing or removed files, denied/revoked access, duplicates, timeout
  and cancellation. Live repository writes and visual card acceptance remain pending.
- GH6: implemented in the current branch. Four distinct Actions controls propose
  rerunning failed jobs and dependents, the whole workflow, one failed job and dependents, or
  normal cancellation. The existing human cards bind account, immutable repository, run, attempt,
  SHA and scope. Each HTTP call passes the gate; execution rereads before and after the POST.
  Results distinguish accepted/pending, verified, rejected, uncertain and natural completion
  during cancellation. A durable atomic audit reservation blocks duplicate controls across
  concurrent calls and restarts; all rerun variants share a receipt per attempt. Definite
  rejection alone permits a fresh proposal. Exact latest/historical run reads and explicit
  phases complement bounded, redacted logs. Fixture tests cover endpoints, scope, stale targets,
  permissions, timeouts, cancellation, duplicate admission, pagination and readback uncertainty.
  Shared Chat renderer tests cover localized Actions cards in Island and Nest. Live repository
  writes and native visual acceptance remain pending; automated tests never mutate GitHub.
- GH7: implemented in the current branch. Two typed human-card controls merge an
  exact PR head with the selected method, or separately delete its unchanged merged branch.
  Readiness checks include account/repository identity, base/head refs and SHAs, check rollup,
  review decision, unresolved discussions, active rulesets and classic branch protection.
  Mutation uses the asynchronous GitHub API version 2026-03-10 with `bypass_rules: false` and
  exact SHA. Required queues retain the configured method and are reported as queued, never
  merged. A read-only merge-status tool inspects current PR and optional async request UUID.
  Durable receipts prevent duplicate writes across methods/restarts. The existing row merge
  shares the service and keeps branches; Island/Nest retain distinct disabled queue/pending
  states after polling. Fixture and renderer checks cover success, stale refs/rules/request,
  permissions, checks/reviews, conflicts, queue, timeout, partial coverage, duplicate admission
  and separate deletion. Fork/stacked PRs, more than 100 checks/discussions, more than 1,000
  active rules, and unavailable protection metadata fail closed. Ref deletion has no atomic
  compare-and-delete; concurrent pushes require keeping the branch and managing it on GitHub.
  Live repository mutations and visual acceptance of native cards remain pending.
- GH8–GH9: not started.
