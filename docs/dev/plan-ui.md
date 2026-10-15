# UI implementation sequence

Reviewed 2026-10-13 against `a03187f6`, the current production surfaces, and the
Compact/Nest proposal. This is the delivery breakdown of the UI rows in
[Plan Zeca](plan-zeca.md#ui-delivery-sequence); Plan Zeca remains the roadmap.
[Every agent](plan-every-agent.md) owns agent protocol and installer work.

The requested direction is to refine the current UI, make Quick useful without
constant scrolling, improve agent workspaces and retain the existing theme.
The proposal implements examples, not the native features. Reproduce each old
finding on the current head before changing production or closing it. The source
roadmaps were rewritten on 2026-10-12; later release/doc commits do not make the
prototype a new production baseline.

## Implementation order

### Current delivery status

Checked implementation items below have renderer or contract evidence; they do
not close a native milestone without its real desktop acceptance.

On 2026-10-14, after merging #351, the user reported reviewing all current
UX1-UX5 areas and accepted them. That closes the existing-feature review block.
UX6's monitor, input-region and window-ownership acceptance remains separate.

After merging #353 on 2026-10-14, the user reported validating its saved system
close, explicit model-read retry and short/wide Chat repairs. This accepts those
slices; the remaining UX6 state and desktop checks below still gate the milestone.

After merging #354 on 2026-10-14, the user accepted Flock filter/detail retention
and the reachable clear action in short windows. The By project slice below is
the next continuity repair; the broader desktop pass still gates UX6.

On 2026-10-14, the user reported reviewing all delivered adjustments through
the merged #357, accepting the Chat, transition and Flock continuity slices.
The final UX6 batch adds integrated regression coverage and fixes delayed Chat
shortcuts and the return announced before saved Nest closure. Implementation
and automated integration are complete; the final desktop matrix below remains
for the user's PR validation before UX7 starts.

| Phase | Delivered | Remaining |
| --- | --- | --- |
| UX1-UX3 | Named controls, keyboard tabs, narrow layouts, Memory return, compact targets and shared usage thresholds | Accepted by the user on 2026-10-14 |
| UX4 | Larger drafts, editable empty-state suggestions, selected-session summary and metrics before explanation | Accepted by the user on 2026-10-14 |
| UX5 | Task status/next step, connection detail/return, searchable grouped History/Audit, concise Presence, memory/web disclosures and phone prerequisites | Accepted by the user on 2026-10-14 |
| UX6 | Shared state and integrated handoff checks; delayed shortcuts and saved native return fixed | Final desktop matrix and user PR acceptance |
| UX7 | Not started | Paired Mini/Quick widths and native geometry contract |
| UX8-UX10 | Not started | Agents workspace, Studio, scoped investigations |
| UX11-UX12 | Feature owners remain authoritative | Companion refinements, agent availability and Android flows |

Deliver the small fixes to existing features in UX1-UX5. UX6-UX7 change surface
behavior and require native checks. UX8-UX10 are new feature work. UX11-UX12 land
with their owning K/M/W/L/G steps, rather than as an unrelated redesign.

| Order | Task | Priority / size | Dependency | Delivery |
| --- | --- | --- | --- | --- |
| 1 | UX1 Names and keyboard | P1 / small | None | Existing controls become accessible without changing authority |
| 2 | UX2 Narrow and short layouts | P1 / small slices | None | Decisions, composer, settings and return actions remain usable |
| 3 | UX3 Cost warning consistency | P2 / small | None | One set of thresholds and explicit cost provenance |
| 4 | UX4 Quick presentation | P2 / small slices | UX1, UX2 | Refined Chat, Flock and Week on the existing owners |
| 5 | UX5 Existing detail and settings polish | P2 / small slices | UX1, UX2 | Better task actions, records, connection detail and setup guidance |
| 6 | UX6 Cross-surface continuity | P1 / medium | UX2 | Selection and drafts survive the real island/Nest handoff |
| 7 | UX7 Paired Mini/Quick extension | P2 / large | UX6; ADR 0021 reconciliation | Shared horizontal extension validated on Linux |
| 9 | UX8 Native Agents workspace | New / large, staged | UX6; ADR 0018/0021 reconciliation | Profiles and specialist context attached to existing task/chat/run owners |
| 10 | UX9 Studio skills and connections | New / large, staged | UX8; configuration and storage contracts | Selective imports and verified connections with retained revisions |
| 11 | UX10 Investigation workflows | New / large, later | UX8; target/evidence contracts; UX9 where tools are needed | A small verified workflow before engine expansion |
| With owner | UX11 Companion refinements | Feature-coupled | K1-K8/K10, M7, W2/W9b as below | Each UI slice ships with its real source of state |
| With owner | UX12 Coverage and Android views | Feature-coupled | Every agent G/R/B/C, L3-L5 | Availability and signed decisions match real capabilities |

Small means renderer-local work with an existing data/action contract. Medium or
large means native state, persistence, surface placement, driver or security
boundary work. These are relative slices, not elapsed-time promises.

The R3 desktop smoke and release tag retain their existing requirements. This
queue does not make future Agents, Android or investigation work a 0.2.0 gate.

## UX1 - accessible names and keyboard, small

- [x] Recheck B03/B04/B09: settings switches, task status/owner and connection
  dismissal have specific labels. Fix the shared helper or actual owner once.
- [x] Recheck B07/B08: the existing island/Nest navigation announces selection;
  tab lists implement arrows, Home/End, roving focus and panel association.
- [x] Keep shortcuts, disabled states and Zeca-off navigation working. Name a
  dismissal by its item, not only its icon.

**Files:** settings controls, `ui/src/surfaces/nest/{head,tasks}.ts`,
`ui/src/island/render.ts`, existing i18n catalogs.

**Done when:** no visible unnamed control; a keyboard-only pass reaches every
existing area and returns from detail. **Reference:** B03, B04, B07, B08, B09; D04.

## UX2 - responsive repairs, small PRs

- [x] Chat (B01/R11): bounded transcript/composer, narrow memory return and saved
  draft; remove composer/footer overlap at 480 x 360.
- [x] Apply short Chat spacing at every width. Empty-conversation suggestions
  scroll when space runs out; centering cannot push the composer over the title
  bar or footer. Renderer checks at 480 x 360 and 1180 x 360 exercise pointer
  clicks on the model picker, failed reads and retry with an unsent draft. In the
  app, shorten a wide Nest with an empty conversation and verify header controls,
  composer, suggestions and footer remain reachable.
- [x] Settings (B02): adapt the page selector and stack wide controls without
  losing advanced options or page selection.
- [x] Approval (R01): shorten the flock scene in low windows so the current card
  and both decision actions are visible before scrolling.
- [x] Targets (R12): enlarge the usable area of small session/size controls.
  Preserve sprite scale and separation between adjacent targets.

**Files:** `ui/src/island/chat.ts`, `ui/src/surfaces/nest/nest.css`, settings CSS
and shared controls. **Done when:** all ten settings pages fit horizontally;
composer/save bars stay inside the frame; pending/destructive cards are usable at
480 x 360 and the existing Quick width. **Reference:** B01, B02, R01, R11, R12.

## UX3 - cost warnings, small

- [x] Recheck B06. Use the same usage thresholds in Cost and Flock: warning at
  70-89%, critical at 90%+, and a readable status in addition to color.
- [x] Retain API estimate versus subscription usage distinctions. Missing usage
  and cost remain unknown; do not replace them with zero or measured totals.

**Files:** `ui/src/surfaces/nest/{records,flock,budget}.ts`.
**Done when:** fixtures at 69/70/89/90/100% agree between views, including labels.
Observed terminal telemetry belongs to W9b, not this fix. **Reference:** B06.

## UX4 - existing Quick content, small slices

- [x] Stop automatic model-query retries after a failure. Keep the failure visible
  in the picker and retry once when the user opens it again; typing and relabeling
  leave the draft and conversation intact. `node scripts/check-chat-models.mjs`
  checks the production Chat panel's failure, explicit retry and recovery without
  a browser server. In the app, reproduce an unavailable CLI model list in Quick
  and Nest, then restore it and reopen the picker; the UI must stay responsive and
  the draft must remain unsent.
- [x] Empty Chat suggestions fill and focus an editable draft without sending;
  the existing Send/Enter action submits it.
- [x] Chat: short recipient/executor/history header; one set of context/memory
  controls beside the companion; empty-state prompts; two comfortable draft
  lines where height permits. Keep attachments, voice, skills, send/stop and the
  current conversation/source owners.
- [x] Flock: use a short selected-session summary and on-demand detail; preserve
  the actual session state and keep a pending decision first. Remove repeated
  labels and tall metadata wrappers.
- [x] Week: period and metrics precede explanation, with concise export/detail
  actions. Use local scroll only for content that actually exceeds the region.
- [x] Reuse the native theme and readable identity labels; do not replace the
  production palette with the older prototype's sage theme.

**Files:** `ui/src/island/{chat,quick,render}.ts`, shared theme, Nest Chat/Week.
**Done when:** defaults fit at the current size and real app window widths; long
content stays editable; history/context return preserves the draft. New Routine
controls remain tied to K4, and new specialist Chat remains tied to UX8.
**Reference:** R02, R05, R13, R19; C03/C09; D01/D02; E01-E03 (prototype revision 05).

## UX5 - existing detail and setup, separate small slices

- [x] Search loaded History and Audit rows by their displayed project, agent,
  status/answer and command. Keep the query when refreshing the tab, hide empty
  groups and expose Clear filters without another backend query.
- [x] Keep memory/web effects, model destinations and daily limits visible;
  put full existing explanations under keyboard-accessible More disclosures.
  Show phone setup prerequisites before the user tries Pair.
- [x] Keep task Start/Go on/Stop before supporting detail; group status/owner,
  issue, handoff/review and removal under More actions. Retain disclosure state
  on refresh and preserve distinct task/folder removal confirmations.
- [x] Explain that a task run is one agent turn, not automatic task completion.
  Fix Claude driver/hook approval races and preserve waiting cards during parallel
  work; controlled task sessions retain their finished bird after CLI exit.
- [x] Preserve an owned run's Failed state and persist its error beside partial
  output. An ended run releases all its waiting cards, including subagent cards,
  without answering them. Blocked task output opens by default and retains the
  user's disclosure choice across refreshes.
- [x] Retain the New task draft until creation succeeds; prevent duplicate
  submission, preserve its project if the live session disappears, and keep field
  focus/text selection across list refreshes. Name each form control explicitly.
- [x] Provide separate cancellation for task deletion and folder removal, scoped
  Escape, retained confirmation focus and a guard against held-key confirmation.
  Disable deletion while the task is running, matching its existing backend rule.
- [x] Distinguish task list loading failures from an empty list. Keep the last
  loaded cards, drafts, focus and disclosures with a stale-data warning and Retry;
  ignore superseded responses and retain action errors until success or New task.
- [x] Retain issue-link drafts until success, including focus/selection on list
  refresh. Give the field a task-specific name and prevent overlapping Link,
  Open an issue and Unlink requests for the same task; failures allow retry.
- [x] Separate selecting another installed task agent from starting its run.
  Keyboard selection sends nothing, the explicit Start names the chosen agent,
  and list refresh retains the task's choice/focus. A missing agent clears the
  choice and cannot silently substitute another executor.
- [x] Keep status, owner and criterion keyboard focus on the same task across
  list updates and saves. Removed controls return to that task's More actions;
  a card leaving the list returns to New task, without taking outside focus.
- [x] Tasks: goal/status/next step first; lifecycle and maintenance under labeled
  disclosure. Preserve issue actions, criteria, budget, run/workspace handling,
  continuation, handoff, independent review and confirmation for removal.
- [x] Connections: below the useful two-column width, selection opens detail in
  its own view; Back restores item, repository filter and focus.
- [x] Activity: short project summaries; searchable/grouped History and Audit;
  tools inside their Trace turn; metrics distinct from retention settings.
- [x] Settings: concise effects, advanced disclosures, per-agent connection
  review, rule scope/revocation and clear phone setup prerequisites.
- [x] Empty and filtered-empty states expose a real first step or clear-filter
  action. Added filters use existing data; heavy new queries get their own slice.

**Files:** `ui/src/surfaces/nest/{tasks,connections,records,roosts}.ts`, settings.
**Done when:** detail return preserves state; grouping hides no working action;
delete task and remove workspace remain distinct. **Reference:** R03, R06-R10,
R14-R19; D04/D05 where they refine already-existing task behavior.

### UX4-UX5 acceptance for these slices

Run `node scripts/check-record-search.mjs` for grouping, displayed status and
command search, empty results, clear/focus and query retention on refresh. It
bundles the production renderer with a small DOM stub; no Lab, browser server or
new dependency is required. CI runs it with the existing geometry check.

Run `node scripts/check-task-actions.mjs` for visible execution actions, retained
disclosure state, continuation on the actual session engine, handoff/review and
distinct removal confirmations. CI runs the same production renderer check.

In the real app, repeat the following at 480 x 360 and a normal Room width:

| Area | Action | Expected |
| --- | --- | --- |
| Quick and Nest Chat | Click an empty-state suggestion, edit it, open Memory and return | No message is sent; the draft stays editable and Send remains available |
| History and Audit | Search a known project/status; in Audit also search a command; switch tabs and return | Only matching groups appear and the query remains |
| History and Audit | Search a missing value, then Clear filters | A distinct filtered-empty explanation appears; clearing restores rows and focuses search |
| Settings, Chat & voice | Open More by keyboard; close it; toggle Memory | Full explanations remain available; dependent options follow the existing Memory switch |
| Settings, Phone | Open with Phone off or no saved address, then complete setup | Pair explains prerequisites first and shows the QR explanation once ready |
| Tasks | Open More actions by keyboard, refresh or switch tabs, then request Delete or Remove its folder | The disclosure remains open; each removal waits for its own second click; Start/Go on/Stop remain outside it |
| Tasks | Reproduce a failed CLI run, then refresh or reopen Tasks | The task is Blocked; partial output and the error are kept and initially expanded; the bird is Failed and no unanswered card survives the ended run |
| New task | Fill goal/criteria/project, refresh while editing, then reproduce a failed save and retry | Focus and selection remain; the error keeps all draft fields; pending creation cannot be submitted twice; only success closes the form |
| Task removal | Arm Delete or Remove its folder, cancel by button or Escape, then repeat using keyboard | Cancellation calls no backend, affects only that task/action and returns focus; a held Enter does not confirm; running tasks cannot be deleted |
| Task list failure | Edit a New task draft, open a card's More actions, reproduce a list-load failure and Retry | Cards and the draft remain with an explicit stale-list warning; focus/selection and disclosure choice survive; Retry only reloads; a failed first load never says No tasks |
| Task issue | Type an invalid issue in More actions, select text, refresh and Link; correct it and retry | The field keeps its draft/focus/selection, errors retain the text, and pending issue controls reject competing requests; success shows Issue/Unlink and only then clears the saved draft |
| Task agent | On a task without a session, choose an installed agent using arrows, refresh, then click its Start; remove that agent from the available list and repeat | Selection alone sends nothing; Start uses that task's chosen agent; focus/choice survive refresh; missing availability clears the choice without switching executors |
| Task keyboard | Focus status/owner or a criterion, refresh and edit it; mark Done with finished tasks hidden | Focus stays on the same task/control; a missing control returns to its More actions; a card leaving the list returns to New task; updates never take focus from another area |

Renderer checks cover these interactions with controlled data. The user's
2026-10-14 review accepted the current UX1-UX5 presentation and interaction block.
The native ownership and geometry checks below remain under UX6.

## UX6 - continuity with native ownership

- [x] Delay Quick/Mini's opening until the Nest's saved close destroys its
  window. A failed close/reopen cancels the return; completion uses the final
  saved route once, honors Zeca off and preserves explicit Settings/global
  routes. A native mock-runtime check verifies the emitted event contract.

- [x] Reject delayed Chat permission shortcuts while its surface is hidden,
  minimized, detached, in another tab or disabled. The shared Chat check covers
  Quick and Nest, and verifies one explicit visible decision exactly once.

- [x] A live update to the same Chat owner still restores that owner's executor,
  folder, saved history and running state, while leaving the newer card/event visible. A
  completed turn stays completed; an opened/reset/sent conversation blocks the
  old source entirely. The Chat check covers a permission arriving before the
  current-conversation read resolves in both Quick and Nest. In the app, open
  Chat during a Codex turn and confirm its executor, source, Stop and card.

- [x] Resolve the Nest's current route after saving a size transition, so a tab
  or Activity subsection selected while the save waits wins over the earlier
  click. A held-save production-entry check reproduced Chat being sent instead
  of Audit; the corrected command uses Audit and retains the Chat draft.
  In the app, click Quick/Full and switch to Activity > Audit while saving;
  returning or reopening must keep Audit, without opening an unrelated Chat.

- [x] A late initial Chat conversation cannot replace live events, an opened
  conversation or a reset. Preserve the current turn, pending card and newer
  draft/source in both Quick and Nest, without sending or answering on restore.
  The Chat check holds the initial read, delivers a sent turn and permission,
  then releases the stale snapshot. In the app, reopen Chat during a turn or
  card arrival; confirm the turn stays active and the card remains visible.

- [x] Share retryable initial Chat restoration in Quick and Nest. Read the
  conversation and draft successfully before writing or sending; failed reads
  keep transitions open and never replace unread text/files with an empty draft.
  A later save retries once and reuses successful restoration. The Chat check
  covers failures of either read, recovery, attachments and bound source.
  In the app, reopen Chat and immediately return/close; confirm the draft remains.
  With an unavailable read, retry the action after recovery without editing.

- [x] Keep the chosen Flock change and expanded view across Nest recreation,
  scoped to the same session and a kept step. Missing steps fall back to the
  latest change and card; another session never inherits expansion. A missing
  diff body returns to the card and disables expansion. Late navigation reads
  leave newer choices alone, and restoring expansion cannot hide a waiting card.
  Native validation bounds the owner and requires a positive step; legacy state
  without a change remains valid. The Flock check exercises recreation, expired
  steps, changed owner, late reads, missing bodies and pending-card restoration.
  In the app, choose an older edit, expand it, return/reopen and verify the same
  edit; repeat after that edit expires and with another session or a pending card.

- [x] Expanded Flock changes return to the card when a permission/question
  arrives, and expansion is unavailable while a request waits. Stale expand
  controls cannot hide a new request. Permission shortcuts require an active
  Flock and visible card; one explicit decision is sent once. The Flock check
  exercises expansion, arrival, stale controls, inactive/hidden guards and an
  explicit decision. In the app, expand a diff, trigger a request for that
  session and confirm the card and its actions return without an automatic answer.

- [x] Wait for initial Chat restoration before size/return actions; serialize
  repeated size requests and show a translated failure with retry. Native system
  close retains a separate guard so a close emitted by Mini/Quick during a size
  command cannot be dropped. `check-nest-close.mjs` checks both paths, competing
  size requests, failed saves/native actions, recovery and nested close. In the
  app, click Mini/Quick immediately on opening Chat, repeat clicks during the
  transition and confirm the draft survives; retry after a failed save.

- [x] Retain By project search independently of History/Audit in native
  app-session memory. Wait for pending saves before Nest close/Quick return;
  restoration leaves newer typing alone. Clear stays cleared on recreation,
  failed saves retry without another edit, and an active query remains editable
  when all projects leave. Search changes do not reload projects or weekly data
  and never apply memory sharing or select a session.

Project validation: `node scripts/check-project-search.mjs` exercises project,
branch and agent filtering, refresh/focus, deferred and ordered saves,
recreation, clear/focus, late reads, failure/retry and empty-project retention.
Native checks keep the three queries separate and bounded and reject writes
from Settings. In the app, search Activity > By project by name, branch or agent,
return to Quick and reopen; repeat with Alt+F4 and confirm the query and matching
cards. Enter a query with no results, Clear filters, close/reopen and confirm it
stays cleared without changing History or Audit searches. This does not close
the broader desktop ownership and geometry acceptance below.

- [x] Retain Flock search, Needs you and detail tab in native app-session memory.
  Flush before closing or returning to the island, restore without overwriting
  newer interaction, and retry failed saves without another edit. Clear resets
  the two filters and focuses search; its toolbar action stays reachable when a
  short window hides the habitat. Zeca off restores Changes instead of Zeca.
  Core remains the selected-session and permission owner: a filter never removes
  the current permission card, and restoration issues no session/action command.

Flock validation: `node scripts/check-flock-place.mjs` exercises production
renderer recreation, ordered saves, clear/focus, late reads, failure/retry, Zeca
off and permission visibility. Rust checks bounds, supported details, calling
webviews and rejected writes retaining the previous state. In the app, select a
session, search its project, enable Needs you and select Context or Activity;
return to Quick, reopen and repeat with Alt+F4. Confirm the three controls and
current card remain. Clear filters, close and reopen: filters remain cleared.
Turn Zeca off after saving its detail tab and verify Changes is restored.

- [x] Keep History and Audit search independently in native app-session memory.
  Flush pending saves before Mini/Quick closes the Nest; reopening restores the
  query without overwriting newer typing. Clear affects only its own records tab.

Record-query validation: `node scripts/check-record-search.mjs` exercises
recreation, ordered saves, clearing one query, late reads and failed-save retention
without additional record loads. Rust checks the app-session state, separate
queries, bounds and the calling webview. In the app, search History and Audit,
switch to Mini/Quick, reopen the Nest and confirm both searches remain; clear
one and repeat. This slice retains the broader native-window acceptance below.

- [x] Wait for Chat restoration and pending draft/navigation saves on system
  close (including Alt+F4). Repeated close requests share the pending attempt;
  failures keep the window open, show a translated alert and allow retry without
  editing the retained search or changing tabs. The close command only accepts
  the Nest webview and targets its fixed shared window, including when Settings
  is visible.

Close validation: `node scripts/check-nest-close.mjs` checks synchronous
cancellation, deferred saves, repeated requests, save/close failures and retry.
The records check exercises a recovered save without another edit. In the app,
type an unsent Chat draft, change History/Audit searches and close immediately
with Alt+F4; reopen from the tray and verify the draft, both queries and the last
subsection. Repeat with Settings visible, Zeca off and a waiting permission;
closing must neither answer the card nor interrupt its run. These desktop checks
remain part of the acceptance below.

- [x] Carry tab/subsection, selected session, repository filter/selected item,
  conversation, draft and source through the integrated renderer state contract.
- [x] Keep one conversation/run/request owner through that contract: transitions
  issue no send/reset/start/stop/focus/answer/merge command and preserve the draft,
  files and source. Pending cards remain reachable after a Mini request.
- [x] Add `node scripts/check-surface-continuity.mjs` to CI and the commit checks.
  It runs production Nest, shared Chat, saved transitions, island state and dock
  measurement together at 480/1180 px in Island, Panel and Quiet. It exercises
  Room/Full and Escape, Quick/Mini return and recreation, failed save/retry,
  minimization/restoration and Zeca off. The backend is controlled; DOM checks
  do not prove compositor behavior. Existing per-component checks retain their
  focused failure, race and consent coverage.

### UX6 final desktop acceptance

The user accepted earlier adjustments through #357. Validate this final PR in
the running app before closing UX6 or starting UX7; automated integration and
mock-runtime event checks are recorded separately from this desktop pass.

| Check | Steps | Expected |
| --- | --- | --- |
| State and B05 | Select a session; set Flock filters; select a Connections repository/item; select Activity > Audit; write Chat text and attach a file. Switch Room/Full, Escape, Quick/Mini and reopen. | The current route, session, filters, item and unsent draft/files/source remain. No conversation or run is created and no card is answered. |
| Pending card and ownership | During a Claude/Codex request, open Nest, minimize, restore and return to Quick. Repeat with Chat open and then a different tab or Settings. | The card remains reachable; only the visible authorized card responds to an explicit decision. Quick's requested return waits for saved Nest closure. |
| Zeca off | Disable Zeca, open Nest, switch sizes and close/reopen while a Flock request waits. | Flock, Tasks and Connections work; Chat stays unavailable and the request is not answered or lost. |
| Geometry and desktop | Use 480 x 360 and a wide/short Nest, move across displays/scales and minimize/restore. Click outside the visible island and type in another app. | No clipped primary action, extra click-catching region or stolen keyboard; one visible surface and matching input region. |

- [ ] Final desktop matrix accepted by the user after reviewing this PR.

This native slice can start after UX2 while renderer-local UX3-UX5 land; it does
not need to wait for the future Agents or Studio features.

Delivered in this slice: an in-memory Chat draft shared by island and Nest,
including text, attachments, folder and provider; Chat return reopens Quick's
Chat. Activity carries its subsection through Room/Full and Escape. The native
draft contract has a Rust regression test; renderer checks cover restoration,
late startup reads and clearing on a new conversation. Connections now keeps its
connector, repository, subsection, selected item and narrow detail across Nest
recreation in the same app session. Missing items return to the list; merge
confirmations are not retained. The island now updates its input coordinates on
viewport changes and replays its first measurement when the bridge connects.
`node scripts/check-island-geometry.mjs` checks production dock calculations without
a browser server. Reopening a minimized Nest explicitly restores its window;
minimization revokes approval-shortcut ownership and does not save its dimensions.
The last Nest tab and Activity subsection are restored after recreation; explicit
routes win, and Zeca off blocks Chat. The Rust regressions cover tab precedence,
focus loss, restore, Settings and destruction. UX6
is implemented and remains awaiting the final desktop window and input-region
acceptance, including monitor changes and click-through under Wayland.

**Files:** `app/src/{panel,lib}.rs`, `ui/src/bridge.ts`, island/Nest entry points.
**Done when:** a desktop pass confirms continuity and authorized card handling;
renderer-only results cannot close this task. **Reference:** B05, R04, C01/C06.

## UX7 - shared extension, native feature work

A bounded Chat-only refinement was implemented on 2026-10-15 at the user's request
([delivery record](#quick-chat-proportions---2026-10-15)). ADR 0021 now describes its
width and zoom within the current fixed overlay. This does not implement the
paired Mini/Quick width selector or close the desktop acceptance below.

- [ ] Reconcile the requested wider Mini/Quick with ADR 0021's fixed overlay and
  four-size contract before landing runtime changes. Keep Room and Full behavior.
- [ ] Implement paired width selection and restoration, with display scaling,
  screen bounds, zoom, anchoring and input region measured together.
- [ ] Use 560/680/760 px with Mini 56 and Quick 600 as initial prototype references;
  validate minimum usable sizes and overflow behavior for more sessions.
- [ ] Verify KWin/Wayland placement, hover/folding, card arrival, click-through
  outside the island and the one-visible-surface behavior on a real desktop.

**Done when:** both sizes retain width, input regions match the visible surface,
cards remain reachable and extra sessions have an accessible overflow path.
**Reference:** A01/A03 (superseded dimensions), A02/A04, C01, R20.

## UX8 - Agents workspace, a new feature in stages

1. **Shell and ownership:** main entry Profiles/Tasks/Studio; reconcile ADRs 0018
   and 0021; retain all native TaskRow actions, storage and task/run relationships.
   Add the Quick summary as a new supported view, with explicit Nest detail.
2. **Profiles:** immutable Vults presets, user copies, role/species/accessory,
   instructions and selected skills. Specify persistence and pinned revision;
   role/appearance does not create a permission profile.
3. **Specialist Chat:** explicit recipient/executor/source, retained conversation
   and task context, and pinned profile/skill revision. Keep Zeca's private memory
   private. Resolve specialist availability with Zeca off as part of the contract.
4. **Flock projection:** working Vults-owned runs use the existing session pipeline
   and their configured identity; watched sessions keep their actual capabilities.
   Prove that one run cannot appear as duplicate birds or conversations.

**Done per stage:** typed native state, persistence where needed, working existing
commands, failure/reset fixtures and a real desktop pass. The standalone browser
prototype's arrays and synthetic run transitions are reference data, not a migration.
**Reference:** C04-C06/C08-C10, D03-D05, R04;
original Studio specification (a local prototype reference, not shipped in this repository).

## UX9 - Studio library and tool connections, later

1. Define native library/source/revision contracts; implement local ZIP discovery,
   validation, selective import and retained supporting files without executing them.
2. Add GitHub source selection and controlled reimport: upstream updates and local
   conflicts remain unchecked; missing bundles are retained; started work stays pinned.
3. Add connection drafts, exact reviewed settings and verified readiness. Only
   Settings handles config writes and credentials, with backup/diff/click and keyring.
4. Attach verified connections and skill revisions to user profiles, then validate
   executor compatibility, permission gates and failure/cleanup states.

**Done per stage:** native persistence and trust-boundary checks, not only browser
import tests. File retention, profile selection and agent installation remain distinct.
**Reference:** C04/C05/C10; Studio Skills/Integrations/Validation. K6 writes proposed
skills with Zeca; it does not replace this library work.

## UX10 - investigations, later and scoped

- [ ] First slice: exact target plus question, destination project and specialist;
  immutable source identity/revision, evidence, findings and explicit unknowns.
- [ ] Start with a bounded static JavaScript/package workflow and a verified
  capability route. Unsupported engines stay unavailable; do not port all 187
  reference operations as one feature.
- [ ] Carry a pinned brief into the existing Developer task and independent review.
  Retain criteria, evidence and unknowns through handoff.
- [ ] Add native/managed/mobile/browser/runtime engines only as separately scoped
  features with readiness, effects, target authority, limits, cancellation and
  owned-resource cleanup. Recorded evidence and live observation remain distinct.

**Done when:** one real end-to-end route produces retained evidence and a linked
task/review; its claims and unknowns survive reopening. Browser X6 and expanded
runtime adapters remain later work with their own gate, not implied by the mock catalog.
**Reference:** Studio Overview/workbench and its staged source specification.

## UX11 - UI delivered with existing companion features

| Owner | UI slice from the prototype | Acceptance to add to that owner |
| --- | --- | --- |
| K1/K2 | Return/day debrief | Short Quick slot, Nest detail, bound Chat source; attention budget/focus never hides a card |
| K3 + W2 | Risk/scope and selected reads | Show actual command/workdir/search folder; per-request batch consent; unselected requests remain pending |
| M7 | Text question | Nonempty answer, exact request/context, expiry/cancel, text escaped; Codex MCP follows installer rules |
| K4 | Routines in Chat and Tasks | Draft starts off; scope/trigger/budget review, edit returns to review, Settings activation, gate and trace |
| K5/K6 | Learning/instruction/skill proposals | Exact path and diff, backup, accept/reject, valid SKILL.md and separate installation |
| K7 | Mood and gaze | Existing core attention drives calm/alert/worried/celebrating; per-mood visual check and reduced motion |
| K8 | Contextual realtime voice | Opt-in model and budget, listen/finish/disabled states, no permission answer by speech |
| K10 | Zeca ACP in editors | Verified editor session, retained context and accurate support status |
| W9b | Watched terminal cost | Off/unknown before opt-in; provenance, reviewed telemetry config, loopback-only receiver; estimates labeled |
| Flock board | Take-off, turn, wire, identity, work, return, visitors | All 12 original IDs remain authoritative; redraw against approved art, per-species/motion checks |

The current animation/export adapter (A05-A07, C07, D13) demonstrates states.
Apply sprite/direction/performance changes with their real director or board item;
retain one clock/lifecycle owner instead of copying the prototype's animation loop.

## UX12 - UI delivered with coverage and mobile features

| Owner | UI slice | Acceptance |
| --- | --- | --- |
| Every agent B4/C1/C5/C6/F3/R2 | Watch/Answer/Drive availability | Registry-backed status, real recorded fixtures, terminal fallback; planned agents never appear installed |
| G1/G2 | Named-tool support | ADR 0027 remains proposed until accepted; protocol VERSION 9 and Settings labels land with their implementation |
| L3 | Android permission/question | Pairing/attestation, signed one-time decision, strong biometrics for allow, no biometrics for deny/dismiss, real expiry/race checks |
| L4 | Task acceptance, handoff, Zeca proposal | Same desktop task/run ID, biometric action, retained context, lock/unlock behavior; no work executed on phone |
| L5 | GitHub service action | Exact repo/run/commit, biometric signature, five-minute limit, no replay |

Existing phone setup copy can ship under UX5. The Android screen, relay, attestation,
push, biometric action and signed-ledger integration stay with L3-L5.
**Reference:** D06-D12; all 13 roadmap examples in the proposal.

## Finding-to-task coverage

Every audit finding has a destination; an implemented prototype is not a closed
production task. A finding already fixed on the current head becomes an app
regression check instead of another implementation ticket.

| Finding | Queue owner |
| --- | --- |
| B01/B02 | UX2 |
| B03/B04 | UX1 |
| B05 | UX6 |
| B06 | UX3 |
| B07/B08/B09 | UX1 |
| R01/R11/R12 | UX2 |
| R02/R05/R13 | UX4 |
| R03/R06/R07/R08/R09/R10/R14/R15/R16/R17/R18 | UX5 |
| R04 | UX8, with UX6 continuity |
| R19 | UX4/UX5 by affected area |
| R20 | UX7; accessible current controls in UX1/UX2 |

| Prototype revision | Queue owner |
| --- | --- |
| A01/A03 | Superseded; use UX7's current reference dimensions |
| A02/A04, C01 | UX7 |
| A05-A07, C07 | UX11 animation/director slices; UX2/UX4 for existing low-risk feedback |
| C02, D01 | Superseded sidebar removed; current direction in UX4 |
| C03/C09, E01-E03 | UX2/UX4 |
| C04/C05/C10 | UX8/UX9/UX10 as scoped above |
| C06/C08 | UX8 |
| D02 | Theme baseline in UX4 and root DESIGN.md, not a new theme feature |
| D03/D04 | UX8; accessibility/disclosure rules reusable in UX1/UX5 |
| D05 | Existing task polish in UX5, specialist identity in UX8 |
| D06-D10/D13 | UX11, with their original roadmap owners |
| D11/D12 | UX12; W9b cost provenance in UX11 |

## Review and completion

### Handoff for the next model — 2026-10-13

Continue on `feat/ui-roadmap`. The verified implementation reference is
`22199c12`, following `138829cb`, `96e0eb4e`, `30b865a0`, `da65ce6e`,
`2a16059b`, `06c5aa33`, `ddea9584` and the UX1/UX3 commits. At that head,
`npm run build`, `scripts/check-i18n.sh` and the former visual suite passed.
This is historical renderer evidence, not validation of the native app. Check
the current Git state before starting; preserve unrelated changes when staging.

Already delivered: named settings/task controls, Nest keyboard navigation,
island selection announcements, consistent usage warnings, short Chat composer,
narrow Settings navigation, short Flock decision priority, narrow Memory return
with draft/focus preservation, and a visible localized Memory label. Session
and size targets were refined from 28 to 24 px after visual review; session
dots remain 9 px visually. Treat 24 px as the current compact-control choice,
not a blanket replacement for the larger desktop-control guidance in DESIGN.md.

The old audit checkboxes above remain reproduction prompts. Do not implement a
duplicate repair merely because its checkbox is unchecked. Do not mount the
generated proposal inside the application or treat it as native completion.

### Next slices and focused acceptance

Deliver one row per implementation/commit. Inspect its current behavior first;
if the problem is already solved, record the evidence and move to the next row.

| Order / owner | Reproduce in the app | What must hold after the change | Validation |
| --- | --- | --- | --- |
| 1 — UX4 Chat | Open Chat and return from context, history and Memory | Long labels wrap; drafts persist; composer actions remain reachable at 480×360. | Real-app smoke |
| 2 — UX4 Flock | Open busy and destructive approval sessions | Selection, owner and next action are clear; pending approval stays first. | Real-app smoke |
| 3 — UX4 Week | Open Activity and Week | Period and metrics precede explanation; export and empty state remain clear. | Real-app smoke |
| 4 — UX5 Tasks | Open a task at 480 px | Goal, status and next action precede maintenance; destructive actions remain distinct. | Real-app smoke |
| 5 — UX5 Connections | Select a connection at 480 px | Detail and return retain selection and filters; state variants remain distinct. | Real-app smoke |
| 6 — UX5 Records/settings | Open Activity and Settings | Grouping retains actions; settings effects and empty states remain understandable. | Real-app smoke |

UX6 onward additionally needs native ownership and window checks; renderer-only
inspection cannot close those tasks.

### Fast validation loop

1. Record the owning UX ID, exact route, affected surface and the observable
   before/after behavior. Read DESIGN.md and the component's real action owner.
2. Reproduce the behavior in the application before editing.
3. After the edit, run build and i18n, then inspect Quick, Nest 760×640, Nest
   480×360 and a wide window when that surface changes. Include English and
   pt-BR, long labels, keyboard focus and reduced motion where relevant.
4. Review the real window for hierarchy, spacing, target separation, sprite
   scale and action visibility.
5. Run the relevant native or core checks when a change crosses renderer
   boundaries.
6. Report exact checks/results, remaining native limitations and the commit.
   Never claim success from partial console output; await the process exit.
   Preserve unrelated working-tree changes when staging.

Example for the next Chat slice:

```sh
npm run build
scripts/check-i18n.sh
git diff --check
```

Inspect the real application at the intended window sizes. Add a regression
assertion only for meaningful behavior not already covered
(draft/focus continuity, consent, reachable actions or actual overlap). Prefer
accessible names over positional button indexes, especially for confirmations.

For each slice, record the owning ID, affected views, reproduction path, source
contract, changes and verification. Use [DESIGN.md](../../DESIGN.md) for UI rules;
keep task priority and completion status here and in Plan Zeca.

Renderer changes need a deliberate review in the running application. Native
state/placement and real provider/mobile changes additionally need their owning
runtime checks. Run the project's required checks before a commit; preserve
English/i18n and translated public docs. Do not broaden a small layout PR into a
driver, model, theme, catalog port or orchestration framework.

Current reference command:

```sh
npm run tauri dev
```


## Six-part UI refinement - 2026-10-15

Requested as one complete batch on `feat/ui-component-polish`. All six renderer
slices below are implemented. This is refinement of UX1-UX6, not UX7's native
geometry change or the future workspaces. Retain production themes, sprites,
permissions and execution owners; add no dependency.

| Order / owner | Implementation | Renderer acceptance |
| --- | --- | --- |
| 1 / UX4-UX5 Tasks | Goal heading, native named criteria progress, separate lifecycle and execution status, next criterion, secondary project/cost context; retain criteria and maintenance | `check-task-actions.mjs`: start/continue/stop, criteria, explicit agent choice, confirmations, drafts and focus |
| 2 / UX5 Connections | Title, review/check summary and primary actions first; technical facts/command in retained native details | `check-surface-continuity.mjs`: selected detail and filters, expanded context through poll, check/action order, explicit merge/cancel and duplicate guard |
| 3 / UX1-UX2 Frame | Shared header icons and footer targets, readable footer, pencil for appearance; short windows reclaim navigation height | Existing Settings and Nest production-renderer checks retain actions, accessible labels and selected navigation; native target/overflow review below |
| 4 / UX4 Chat history | Stable local search across all loaded rows, date groups, project/executor/time, current selection, initial 30 and Show more | Production Recent check: query/focus, empty and filtered states, older result, more rows, busy guard, stale-list retry and obsolete responses |
| 5 / UX1-UX5 Feedback | Row-local saving/saved/error for switches and choices, rollback and pending state across repaint; explicit refresh failure in Connections; history loading/retry | Settings check holds/rejects real control writes across renderer refresh, including segmented and select rollback; Connections tests refresh guard/error/retry |
| 6 / UX2-UX6 Continuity | Same history as a separate narrow Room view, one history entry point, Back/Escape and focus return; stop hidden portrait, preserve query on recreation; long task/connection labels | Integrated check now exercises 480/760/900/1180 widths and 360 px height, mode changes, files/source/draft, filters, hidden/off Zeca and retained scene |

Implementation used the existing native controls, DOM helpers, semantic colors and
motion tokens. Each slice passed its relevant production-renderer check and the
TypeScript/Vite build before the next dependent change. New strings have pt-BR,
Spanish and Chinese translations; Nest and Tasks guides are mirrored in pt-BR.

Desktop acceptance remains the user's next step. Tooling can launch Tauri here
but exposes no native application surface for visual inspection. Do not interpret
DOM mock checks as measured WebKit layout. In the running application, review:

1. Tasks with no criteria, mixed completion, a running agent, blocked output and
   long goals; use Start/Go on/Stop only on intended work and confirm deletions.
2. Connections Inbox and PRs, failing/passing checks, technical disclosure, narrow
   Back, repository filter and refresh failure. Merge still needs explicit consent.
3. Header/footer in Portuguese at 480x360, 760x640 and Full: separate targets,
   keyboard focus, readable counts and no horizontal page overflow.
4. Chat history search by project/title/executor, an old result beyond 30 rows,
   empty search, current conversation, Show more and Back to Chat. Close and reopen
   Nest to check query retention, without sending the draft.
5. Slow or failed settings writes: one pending operation, readable local feedback,
   restored selection on failure and focus return without taking focus from another field.
6. Mini/Quick/Room/Full with many sessions, attachments, long paths, reduced motion
   and Zeca off; check native input regions, sprite scale, retained drafts and at
   least 40 px of transcript in the minimum Nest.

The final repository gates are the build, renderer checks (including Settings,
project search and continuity), fmt/clippy, workspace tests, evals, brand/English/
i18n/layers/network/README checks and diff whitespace. Rust tests use
`TMPDIR=/var/tmp` because an unrelated `/tmp/.git` fixture prevents their temporary
repositories from initializing in the default directory.

Automated result for this batch: the TypeScript/Vite build, all nine renderer
scripts and repository policy checks pass; Rust fmt/clippy pass, with 864 workspace
tests and 8 eval tests passing. Native visual acceptance remains open as described
above; no browser or native screenshot result is implied.


## Quick Chat proportions - 2026-10-15

User requested the fifth suggestion: rebalance the Compact's space for Zeca,
conversation and controls. Delivered on `feat/ui-component-polish`, retaining
Mini, Flock placement, assets, integer sprite scales and existing interaction owners.

- Chat uses 492 CSS px (561 logical px after the existing 1.14 zoom); the other
  Quick tabs keep 440 CSS px (502 logical px). Zeca's existing 112x108 scene gets
  a clear 128x112 slot. The title has its own full line above executor/actions.
- A populated or thinking conversation fills the available height up to 556 CSS
  px. The transcript uses the remainder instead of the old 200/120 px caps.
  An empty chat keeps its natural content height. The draft remains bounded and
  multiple attachments use a horizontal strip rather than additional rows.
- One production geometry module owns zoom and available bounds. The mapped
  native surface stays 720x640; the initial Tauri configuration now agrees with
  that existing runtime size. A smaller viewport lowers the CSS limits before
  synchronous measurement. Top/bottom docking uses the measured rectangle.
- Short viewports prioritize reading and sending and pause the hidden portrait;
  the existing scene resumes when space returns. Overflow keeps a scroll path
  to actions if attention/news and a large draft exhaust the available height.
- ADR 0021, the UI rules and Chat guide describe the bounded change; the public
  changes have reviewed Portuguese mirrors and updated source marks.

Validation: build/typecheck, the production geometry check (normal/wider widths,
smaller viewports, 1/1.25/1.5/2 display-scale bounds, native configuration agreement,
exact top/bottom input rectangles and hidden click-through), Chat checks (empty,
thinking, restored conversation, IME, streaming/manual scroll, drafts, files and
voice), and the integrated surface continuity check. Full repository gates run
before the local commit.

Desktop review remains required in the reopened Tauri: compare Flock and Chat,
open the engine/history/folder/access menus, try long drafts and several files,
check the swallowing/voice clips, fold/reopen and move between Quick and Room.
Confirm click-through outside the wider shape and bottom-panel placement on the
real compositor. Renderer mocks and geometry arithmetic do not measure WebKit
layout or prove the compositor's native input behavior.


## Chat, permissions, Memory and selectors - 2026-10-15

The next four UI suggestions are implemented on `feat/ui-component-polish`.
They reuse the existing theme, motion, DOM and action owners with no dependency.

| Slice | Implementation | Production-renderer acceptance |
| --- | --- | --- |
| Active conversation | One safe full Markdown renderer for Quick and Room; shared readable headings, lists, quotes, tables and code; named keyboard-scroll regions; shared copy feedback with failure/retry | Chat check exercises both surfaces, inert links/HTML, copy guard/recovery and retained draft; existing stream cooldown and IME checks still pass |
| Permission cards | Permission state, operation and full folder; readable command region, separated scope warning, cautious primary action when no exact-call rule is offered or outside content was read | Chat check verifies full command/scope, warning and no invented Always action; existing hidden/disabled/late shortcut and explicit decision checks retain authority |
| Memory | Proposals before saved facts/debrief/history; labeled Current/Proposed text, explicit removal; retained native disclosures, usage bars; pending writes, stale read warning and retry | Continuity check holds/rejects real writes, refreshes during a pending action, checks errors/retry, obsolete reads, retained scroll/disclosures/focus and no writes on opening |
| Selectors | One open panel, common selected treatment and targets; named controlled regions; arrows/Home/End, Escape return and outside dismissal; retain focused choice and unfinished rename/caret across reads | Chat check exercises both surfaces through a delayed model read, four competing panels, text editing, IME, keyboard dismissal, explicit folder selection and no implicit execution/access change |

Public Chat and Memory guides and all three UI translations accompany the change.
The build, all renderer checks and full repository gates run before the local commit.
Native visual acceptance remains for the user in the reopened Tauri: review rich
replies and horizontal table/code scrolling, a real permission's complete operation,
Memory with pending/saved/empty content, and the four selectors in Quick and Room.
No screenshot or measured WebKit layout claim is implied by DOM checks.

Automated acceptance: TypeScript/Vite build, all nine renderer checks, fmt/clippy,
864 workspace tests and 8 eval tests pass. Brand, English, i18n, layer, network,
README translation and whitespace checks pass. Chat/Memory mirror marks match.
Tauri starts on this branch and binds all nine global shortcuts; desktop visual
acceptance remains with the user.
