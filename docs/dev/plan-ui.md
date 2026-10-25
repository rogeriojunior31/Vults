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

As of 2026-10-16, [N0](plan-zeca.md#n0-remove-tasks) removes Tasks
([ADR 0031](../adr/0031-tasks-leave-vults.md)). References to its delivered UI below
are historical; its acceptance rows and removed renderer check no longer apply.
The current delivery order is N0-N4 in Plan Zeca.

On 2026-10-17, N1.1-N1.4 deliver the shared frame, Black/neutral Light themes,
Activity dashboard, Connections triage and grouped Settings. Native checks cover the
three pages at 480, 760 and 1180 px in both themes, panel folding/reset, retained Chat
drafts and Connections return focus. The Flock/Compact follow-up corrects fixed dark
surfaces, unreadable diff/control colors and stale canvas palettes on a live theme change.
The user validated the corrected Flock/Compact themes in the running app on 2026-10-17
and accepted N1 for PR review. N1.1-N1.5 are complete. The broader
[UX6 desktop matrix](#ux6-final-desktop-acceptance) and release smoke remain tracked
separately, including multiple displays/scales and window ownership.

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
| UX7 | Paired widths/proportions, bounded session overflow and keyboard continuity implemented | Native compositor/desktop acceptance deferred |
| UX8 | Native catalog/editor/appearance, pinned skills and revisions, isolated Claude specialist Chat with per-profile access and history | Quick summary and execution projection |
| UX9-UX10 | Not started | Studio and scoped investigations |
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
| 9 | UX8 Native Agents workspace | New / large, staged | UX6; ADR 0018/0021 reconciliation | Profiles and specialist context attached to existing session/conversation owners |
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

The former Tasks renderer check was retired in N0 along with its surface.
The Tasks rows below record the earlier acceptance only.

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
| Zeca off | Disable Zeca, open Nest, switch sizes and close/reopen while a Flock request waits. | Flock and Connections work; Chat stays unavailable and the request is not answered or lost. |
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

**2026-10-21 UX7.1 shared width implementation:** at the maintainer's request, proceed with
implementation while manual voice/desktop acceptance stays deferred. Quick's General page now
saves one presentation-only width (Default/560/680/760) for Mini and all Quick tabs. It restores
before first rendering, preserves the saved choice on smaller screens and reports storage failure
without applying an unsaved choice. The mapped overlay reserves 880 x 640 at startup; no width
change resizes/hides it. Existing synchronous geometry, dock anchoring and Scene resettling remain
the owners. Room/Full, default dimensions, Mini height, cards and capture authority stay unchanged.
The 56 px Mini/600 px Quick heights remain prototype references for a later refinement, not a
claim of this slice. Native compositor and overflow acceptance remain open; no user sign-off is
inferred. See ADR 0021's dated amendment.

**2026-10-21 UX7.2 paired proportions:** selected widths now use Mini's 56 logical-pixel
height and Quick's 600 logical-pixel total-height cap, including inline dictation feedback.
The original dimensions remain the Default. Both sizes account for available screen height;
Mini's existing 38 px canvas and its badges move together without changing sprite scale, owners
or clip phase. Quick keeps content-driven height and existing scrolling/action priorities.
Build/typecheck and focused geometry/continuity checks are engineering checks only; the maintainer
requested no new manual validation in this step. Real desktop acceptance stays deferred.

**2026-10-21 UX7.3 session overflow:** Mini now reserves a separate 32 px list action when
sessions exceed its available perches. Capacity depends on width and reserves the action before
placing birds; narrow views reduce the visible count instead of overlapping targets. Keep the
existing eight-other-bird maximum. The icon's localized accessible name/tooltip states the hidden
count and its warning color reflects hidden attention. Mouse and native button keyboard activation
use the same saved transition to Room/Flock as Quick's existing +N. The stable button remains a
sibling of Mini's role=button, not an interactive descendant. Remove it for Quick, hidden/locked
views and absent navigation; a disappearing overflow returns keyboard focus to Mini when it stays
visible. No decision, microphone or separate interaction owner is introduced. Native desktop
acceptance remains deferred at the maintainer's request.

**2026-10-21 UX7.4 keyboard continuity:** repair the shared inline Quick picker instead of
adding a second width control. Stable groups and expanded/control relationships describe the
selector; arrows/Home/End move focus without changing a value, explicit activation selects, Escape
closes the list before the global fold handler, Tab/blur leave without trapping focus and IME
composition is ignored. Selection returns focus before calling its owner; Quick redraw restores
the same picker trigger and scroll position. Failed width saves announce the retained choice.
The production-renderer regression exercises these key paths with no desktop, provider or
settings writes. This closes the planned UX7 implementation slices; the compositor/desktop matrix
below stays deferred, without an acceptance or release-completion claim.

**2026-10-21 maintainer feedback:** the 56 px resting bar was too tall over application tabs.
Restore Mini's original 38 px height for every chosen width, with no extra canvas/badge inset.
At the maintainer's explicit request, General > Appearance exposes the same width control as
Quick > General. Both use the same presentation-storage key and shared change/refocus refresh;
failed writes retain the saved width. This supersedes the Mini height in UX7.2; Quick's selected
600 px cap remains. The app stays open with the personal profile for ongoing validation.

**2026-10-21 native wire correction before UX8:** the maintainer noticed Quick's wire stayed
narrow after increasing width. The live personal-profile app reproduced a 471 px habitat with
an approximately 420 px canvas. Measure the mounted overview instead of reusing HABITAT_W;
resize/reposition the existing Scene, labels, badges and overflow column together. A guarded
ResizeObserver also catches content-width changes caused by scrollbars without using rAF. Account for
the original integer sprite size before applying zoom to perch positions, and partition hit
columns using shared rounded boundaries. A native WebKit matrix at Default/560/680/760 passed
canvas-to-host alignment, bounded label targets and 38 px Mini height; screenshots were inspected.
Mini's visible 560 px wire also matched its canvas and 38 px frame. Restore the maintainer's
560 px preference after the matrix. This does not claim the complete deferred compositor matrix
or start UX8 implementation.

A bounded Chat-only refinement was implemented on 2026-10-15 at the user's request
([delivery record](#quick-chat-proportions---2026-10-15)). ADR 0021 now describes its
width and zoom within the current fixed overlay. This does not implement the
paired Mini/Quick width selector or close the desktop acceptance below.

- [x] Reconcile the requested wider Mini/Quick with ADR 0021's fixed overlay and
  four-size contract before landing runtime changes. Keep Room and Full behavior.
- [x] Implement paired width selection and restoration, with display scaling,
  screen bounds, zoom, anchoring and input region measured together.
- [x] Implement 560/680/760 px with Mini 38 and Quick 600, viewport bounds and a keyboard-accessible
  session overflow path. Physical minimum-size usability remains in the deferred desktop pass.
- [ ] Verify KWin/Wayland placement, hover/folding, card arrival, click-through
  outside the island and the one-visible-surface behavior on a real desktop.

**Done when:** both sizes retain width, input regions match the visible surface,
cards remain reachable and extra sessions have an accessible overflow path.
**Reference:** A01/A03 (superseded dimensions), A02/A04, C01, R20.

## UX8 - Agents workspace, a new feature in stages

**Resume checkpoint after the UX7/UX8 groundwork PR:** profiles can be read, copied and edited
locally, and their selected built-in skill content is pinned in immutable revisions and conversation
snapshots. Profile detail states its recipient/executor/source and its availability with Zeca off.
Specialist Chat opens inside the selected profile, uses its own event stream and saved history, and
keeps the pinned revision for each conversation. Users choose Ask, accepted policies, or full access
per profile; the default is Ask, independent of Zeca's Chat setting. The selected level is read for each tool call; policy denies block the call, while destructive
actions, policy asks and actions after outside content still require a card. Quick summaries and execution projection follow. Native evidence is retained locally;
automated checks do not replace unperformed maintainer or external-provider acceptance. Keep N3/K8
and the N4 release gates open.

**2026-10-21 UX8 Ask Zeca routes:** Flock session questions, Connections questions, palette prompts
and Quick's empty-flock Ask Zeca use one ChatPanel preparation path. It restores and flushes the
existing draft, checks the native recipient/generation and appends to the latest text only while
the source is unchanged. Native preparation is read-only, requires a Chat surface and Zeca on,
and rejects specialist ownership. It does not reset a recipient, send a turn or answer a card.
Preparation remains available beside an active companion reply. Failure is a local note rather
than a terminal stream error, preserving the live reply and its approvals. Rejected or obsolete
questions never replace current text/files/source; native source-bound draft saves remain the
last guard. Profile Chat stays gated for the remaining skills and availability contract.
Seventeen native Chat tests, production Chat/continuity renderer checks, app Clippy and UI/native
builds pass. The integrated Nest check clicks the real selected-session action at each supported
fixture width; native personal checks exercised Flock Ask Zeca and Quick palette preparation,
rejected an obsolete generation and restored the original draft without a model turn.

**2026-10-21 UX8 companion commands and voice ownership:** local memory and continuation commands
carry the composer's source generation and retain a native recipient lock through their action.
Specialist metadata/runtime, obsolete sources, busy Chat and changed started folders reject before
memory or continuation work. Rejected commands preserve editable text; late results/errors do not
enter another conversation, and successful commands keep newer typing. Chat microphone start, preview/level/silence events and
transcription completion check the companion recipient/generation. Reset/open/current-delete cancel
the native recorder, both renderers cancel their recording run, and an obsolete transcript cannot
enter the new composer. Voice-command handoff captures generation before asynchronous dispatch;
an identical empty draft after reset is still a different source. Profile Chat remains gated for
Ask Zeca transitions, skills and Zeca-off availability; this slice makes no provider request.
Nineteen targeted native Chat/Voice tests, renderer continuity/Chat checks, Clippy, builds and
repository checks pass. Personal native checks reject stale commands and microphone admission
before side effects; empty command usage renders in Portuguese in Nest and Quick, with the draft
restored afterward. No audio capture, memory fact write or model request was used for this pass.

**2026-10-21 UX8 conversation-bound composer:** native source generations distinguish resets,
opens and separate empty conversations. First-turn persistence keeps the same generation. Chat
draft snapshots and reset replies carry the source identity; mismatched startup snapshots retry.
Both renderers capture the identity and folder before awaiting draft saves. Native turn preflight refuses obsolete sends before recall, while
draft writes check identity under the draft lock. A local switch cancels a queued send and obsolete
failures cannot overwrite the new composer or transcript. The existing header names Zeca separately
from the executor without adding a row. Switching Zeca off resets the source and preserves the
unsent draft; obsolete cross-surface draft events are ignored. Profile Chat remains gated pending
slash-command, voice, Ask Zeca, skills and availability ownership. Targeted native and renderer fixtures cover first save,
empty reset, open, late save and late failure without provider requests. Native personal-profile
checks rejected obsolete send/save requests without changing the draft. Nest at 1180 x 820 and
480 x 360 and Quick kept the recipient visible without an added header row or horizontal overflow.
Sixteen native Chat tests, both renderer checks, UI/native builds, Clippy and repository checks passed.

1. **UX8.1 catalog and ownership:** read-only Agent profiles inside Flock, with an inert native
   built-in catalog, explicit revisions and the bounded saved route `flock-profiles`. Keep five
   main tabs, session-state retention, Zeca-off availability and visible card attention. ADRs
   0018/0021 record the boundary. Tasks and task/run relationships remain removed under ADR 0031.
   The Quick summary remains a later slice.
2. **Profiles:** immutable Vults presets, user copies, role/species/accessory,
   instructions and selected skills. Specify persistence and pinned revision;
   role/appearance does not create a permission profile.
3. **Specialist Chat:** explicit recipient/executor/source, retained conversation and pinned
   profile/skill revision. The first route uses its isolated chat folder without project/flock
   context; any future project context needs its own reviewed scope. Keep Zeca's private memory
   private and the specialist route available with Zeca off.
4. **Flock projection:** working Vults-owned runs use the existing session pipeline
   and their configured identity; watched sessions keep their actual capabilities.
   Prove that one run cannot appear as duplicate birds or conversations.

**Done per stage:** typed native state, persistence where needed, working existing
commands, failure/reset fixtures and a real desktop pass. The standalone browser
prototype's arrays and synthetic run transitions are reference data, not a migration.
**Reference:** C04-C06/C08-C10, D03-D05, R04;
original Studio specification (a local prototype reference, not shipped in this repository).

**2026-10-21 UX8.1 implementation:** the Connect catalog provides Developer, Reviewer and
Researcher presets with stable ids, revision 1, species and English instructions. A Nest-only read
command returns those immutable descriptions. UI translations include the native marked text in
the existing catalog audit. Flock opens the read-only profile list and instruction detail; selected
profile survives local redraw, stale reads are ignored and failure has an explicit retry. Reading
starts no process, network request, config write or memory read. Custom profile persistence,
copy/edit, Chat binding, Quick summaries and execution projection remain later UX8 slices.
Native checks at 1180 x 820 and 480 x 360 verified exclusive screen ownership, translated cards,
selection and the return focus. Island and Settings cannot call the Nest-only catalog endpoint.

**2026-10-21 UX8 profile copies:** Nest-owned copy/edit commands save literal user names and
instructions in migration 29. Presets stay immutable; each copy retains its preset revision,
appearance and description, while each edit appends a revision under an expected-revision check.
History erasure preserves these preferences. Errors retain the in-window draft; saving starts no
agent and does not alter access. Native copy/edit/reload, rejected stale writes and Island write
rejection passed in an isolated profile. Wide and 480 x 360 editor checks kept Save/Cancel reachable,
including scrolled instructions. The personal profile was reopened without test copies.
Appearance/skills editing and Chat binding remain subsequent slices.

**2026-10-21 UX8 profile appearance:** custom profiles select from the existing 23 species and
fixed accessory catalog. The editor changes only its preview while drafting; the saved card uses
the same dressed sprite after saving. Migration 30 adds `none` to earlier revisions and retains
species, text and provenance. Native validation rejects unknown ids and calendar-dependent `auto`.
New revisions keep their own species/accessory; no earlier revision, Zeca setting or watched session
changes. The renderer check covers every species/accessory combination without clipping. Native
copy/edit/reload at 1180 x 820 and 480 x 360, scrolled controls, invalid appearance, stale saves and
Island write rejection passed in an isolated profile. The personal profile was restored without
test copies. Skills and Chat binding remain separate subsequent slices.

**2026-10-21 UX8 conversation binding storage:** migration 31 adds an optional typed profile
snapshot to each conversation. It records preset/custom identity and revision plus the literal
name, instructions, description, species and fixed accessory. Custom snapshots must match the
retained revision exactly; missing or altered references fail before creating a record. Later
profile edits do not change existing conversations, and the snapshot cannot be rewritten even
before a first reply. Conversation rename/resume preserve it; deletion/history erasure remove it
with the conversation while preserving user profiles. Legacy conversations retain their original
context and have no profile snapshot.

This storage slice deliberately enables no recipient selector or Chat action. Profile-bound
records reject Zeca's frozen context. The current Zeca list/open/render paths exclude or refuse
them rather than resuming them under his persona, MCP tools or private memory. A corrupt snapshot
fails reading rather than falling back to Zeca. Runtime persona/tool isolation, explicit executor
and project context, pinned skills and the Zeca-off capability decision remain required before
specialist Chat becomes available. Store tests cover exact retained revisions, forged/missing
references, direct snapshot rewrite rejection, history deletion and migration from schema 30.
A native isolated fixture confirmed that only the regular conversation is listed/opened, a bound
conversation is rejected without replacing it, and no model request is made.

**2026-10-21 UX8 specialist runtime primitive:** the Chat crate adds a distinct specialist role
and a Claude Code constructor. Its immutable brief replaces the companion persona; resume forces
isolated settings and strips MCP, extra directories and API history. Companion frozen context,
API tools and user/project settings cannot be attached later. Driver factories reject unsupported
providers, ACP and unsanitized specialist specs. Turn preflight rejects companion context before
starting a driver or reaching an API key/network path. The existing CLI stdio approval flow and
conversation rules stay authoritative.

This is not the native specialist Chat feature yet. The app's profile-record guards remain closed.
Native routing still needs an explicit recipient/executor/source, exact snapshot/session pairing,
and separation of recall prefetch, volatile context, journaling, memory proposals and MCP scope.
Draft/reset/open/voice/Ask Zeca transitions must preserve the intended recipient before a profile
Chat action is exposed. The Zeca-off capability decision, skills and flock projection remain open.
Local checks cover exact brief/CLI flags, rejected configuration and tool injection, sanitized
resume and preflight rejection without driver creation. The Chat suite includes fake CLI
interrupt/resume fixtures; real external-provider qualification remains unperformed.

**2026-10-21 UX8 native recipient and memory separation:** ChatState retains the exact recipient
snapshot alongside its locked runtime. Native restoration selects the isolated Claude constructor
for bound records, preserves their saved pick/session and validates full identity, source and brief.
Turn preflight reads the persisted record before recall and refuses missing/mismatched metadata;
specialist voice commands, flock mentions and source overrides are not yet supported. The first-turn
record builder carries the pinned snapshot instead of dropping it. Specialist turns skip companion
recall, volatile context, MCP scope, native-setting initialization, journal writes and Memory refresh.
Reset/open/stop/delete update recipient metadata with the runtime. The memory-extraction reader
rejects specialist records before reading their messages.

The current list/open/render guards still block user-facing specialist Chat. Exposing the profile
CTA requires recipient/executor/source UI, slash-command ownership, voice/Ask Zeca transitions and
cross-surface draft continuity. UI-origin sends must carry an expected conversation/recipient identity
to reject a stale draft after a switch. No new model request, profile action or Zeca-off capability is enabled
by this native separation slice. Targeted checks passed for native restore/turn identity, source,
revision, appearance, model/effort/session mismatches, missing history and memory-extraction
exclusion. Existing companion Chat and runtime isolation checks remain passing. No live provider
request or renderer change was made in this step.

**2026-10-21 UX8 pinned skills:** the profile editor can select from the small built-in skill catalog.
Each choice carries a stable id, revision, title and literal content into a new immutable profile
revision. Migration 32 defaults existing revisions to no skills. Conversation snapshots retain the
selected content, and the isolated specialist brief uses that exact snapshot when restored. Stale,
duplicate, oversized or malformed selections fail closed. Skill selection does not activate tools
or expose the gated Chat action. Renderer build, translation audit and source checks pass; manual
desktop acceptance remains open.

**2026-10-22 UX8 specialist route contract:** profile detail now states the recipient, isolated
Claude Code executor, pinned profile/skills source, absence of project/flock context and private
Zeca memory. Specialist text Chat is defined as profile-owned and available with Zeca off, without
adding a global Chat tab. Each profile owns an Ask/Policies/Full choice, defaulting to Ask; it is
independent of Zeca's access setting and retains destructive/outside-content confirmation. The entry
remains gated until native routing and cross-surface continuity enforce the contract.

**2026-10-22 UX8 specialist Zeca-off preflight:** native Chat send now checks the Zeca switch only
for companion turns. Profile-bound turns no longer inherit that switch; their selected profile
access still governs tool cards. Window sends resolve their recipient before the switch check, while
voice commands remain Zeca-only; companion sends stop before memory recall and recheck after local
async work. The UI route remains gated until it owns separate conversation state and cross-surface
transitions.

**2026-10-22 UX8 specialist history boundary:** a Nest-only command now lists retained conversations
for one preset or current/retained custom profile at a time. It returns only conversation metadata,
including the pinned profile revision, excludes companion and other-profile records, and never
changes the active Zeca conversation. The native selector can consume this inventory later;
specialist Chat stays gated until its independent runtime and event stream are ready.

**2026-10-22 UX8 specialist Chat delivery:** each profile now opens in the existing Nest Chat tab
with a recipient selector and the production ChatPanel, retaining the isolated Claude Code executor, pinned instructions/skills, profile-specific Ask/Policies/Full
access, and a retained history filtered by profile identity. Start, reset and resume preserve the exact
conversation snapshot. Specialist messages, approvals and state updates use a Nest-only event stream;
they never enter Zeca's transcript, draft, memory, tools, voice or project/flock context. Permission
cards remain explicit, show outside-content disclosures and support deny, allow and conversation-only
Always. The profile editor and access setting remain available independently of the Chat view.
The shared Chat renderer covers recipient switching, scoped sending, approval routing, Zeca-off
access and the companion transcript/draft boundary. The roadmap's Quick summary and execution
projection remain separate work; a live-provider turn and desktop visual acceptance remain pending.

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
| 1 / UX4-UX5 Tasks | Goal heading, native named criteria progress, separate lifecycle and execution status, next criterion, secondary project/cost context; retain criteria and maintenance | Historical Tasks check, retired in N0: start/continue/stop, criteria, explicit agent choice, confirmations, drafts and focus |
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


## Connections workspace refinement - 2026-10-15

User requested a focused Connections UI pass on `feat/ui-component-polish`.
The production connector, action and merge owners remain authoritative; no
connector, network polling behavior or permission is invented by the renderer.

- One service header with sync/wait/error/budget state, Refresh and Settings;
  show connector selection only for multiple cards and preserve enabled/waiting
  versus known-off versus failed local status reads. Ignore obsolete status reads.
- A retained local search filters title, reference, repository, branch and author
  within the current subsection alongside the repository filter. Show result and
  section counts, explicit empty results and Clear filters. No network search.
- The list and detail have bounded independent scroll regions with keyboard focus.
  Polls retain reading positions and disclosure state; narrow Back/Escape returns
  to the selected row. A short frame keeps a scroll fallback to its controls.
- Put branch/author/diff and explicit actions before checks. Passing checks start
  collapsed; a newly running/failing state opens the list with failed checks first.
  Retain a manual disclosure choice while state is unchanged. A failing check's
  label takes precedence over an approval; seen rows retain readable status/actions.
- Technical detail has update metadata and a copy-only command with the shared
  copy failure/retry feedback; omit empty technical sections. The existing merge
  confirmation, refusal and duplicate guards remain intact. Escape only cancels.

The production continuity check exercises search by branch and author, clear/repo
filters, empty results, stable input/focus, two scroll positions, status-change
check priority, manual disclosures, explicit merge availability, narrow Escape/IME,
local status failures/retry and obsolete read rejection without external actions.
The Nest guide has a reviewed Portuguese mirror and UI strings in all catalogs.

Native acceptance remains with the user in Tauri: compare Room and Full with a
long list, many checks and long names; try search, clear, independent scrolling,
Back/Escape, reduced motion, empty/waiting/stale states and explicit actions.
Renderer checks do not measure WebKit layout or prove native compositor behavior.

Automated acceptance: build/typecheck, all nine renderer checks, fmt/clippy,
864 workspace tests and 8 eval tests pass, along with brand, English, i18n,
layer, network, README translation and whitespace checks. Tauri remains running
on this branch and received the updated renderer through Vite. Native visual
acceptance is still pending the user's review.


## Workspace theme alignment - 2026-10-15

The user identified Tasks and Connections as visually different from the rest of
Vults. The Nest frame still used the card surface as its page background, while
Activity and Chat had explicit black canvases; Tasks also retained a local palette
and button overrides. This pass keeps the current branch and interaction owners.

- Use the existing page-background role for the Nest. Tasks/Connections cards and
  the shared spend card use Activity's panel, border and radius roles.
- Reuse the Activity heading and segmented navigation. Give Tasks its page heading;
  remove Connections' separate service-icon heading and underline-tab styles.
- Share neutral secondary/primary button rules and token-based input/focus roles
  between Tasks and Connections. Remove their old palettes and late overrides.
  Keep warning borders and execution/approval meanings; finished cards dim their
  title without fading controls or statuses.
- Align the page margins and compact card padding with Activity. No new theme,
  dependency, action, persistence or permission behavior is introduced.

Validation uses the existing production-renderer checks, including task creation,
criteria, confirmations, failed writes, Connections filters, polling, focus and
narrow continuity, plus the build and complete repository gates. This presentation
pass adds no implementation-mirroring tests. Tauri receives the changes through
Vite; user review of the real Room/Full theme and short/narrow layout remains open.

Automated result: TypeScript/Vite build, nine renderer checks, fmt/clippy,
864 workspace tests and 8 eval tests pass. Brand, English, i18n, layers,
network, README translation and whitespace checks pass; the Nest mirror mark
matches. The running Tauri process received the updated UI through Vite.


## Empty, loading and read-error states - 2026-10-15

The user selected the first follow-up: consistent empty/loading/error feedback.
A small production DOM renderer and stylesheet provide one neutral panel with a
state icon, title, optional explanation, explicit action and closed technical
error disclosure. It uses the existing theme and icons, also inside Week's shadow
root. Tasks, Connections, records, By project, Week, Memory and Recent reuse it.
No third-party assets, dependency or background action is added.

- Tasks shows its initial read instead of an empty list, retains its existing
  draft/failure recovery, and names the empty state and available creation path.
- Records distinguishes loading, history off, actual emptiness and no filter
  match. History, Audit, Traces and Usage keep the last successful snapshot and
  totals on a read failure, with an explicit guarded retry and stale warning.
- By project now reports failed reads rather than silently dropping them. A
  known list and query remain editable, with the existing serialized/queued reads.
- Week reports errors instead of staying in its null/loading view, retains its
  last recap on refresh failure, and rejects obsolete reads. Successful clearing
  drops the old in-memory recap before reading back. Retry never clears or exports.
- Memory, Recent and Connections share the same panel treatment while retaining
  their existing permission, selection, loading, refresh and disclosure owners.
  Missing cost data says that no cost was recorded rather than implying no spend.

Existing production checks cover Tasks' initial loading and failure/draft/retry.
Records checks now hold/reject reads for all four views, preserve snapshots and
summaries, guard retry and reject obsolete responses. By project checks initial
error/retry and stale-list retention without project actions. The integrated Week
check covers cold failure, explicit retry, a retained recap and read ordering,
with zero history-setting/clear/export actions. The public Nest guide and all UI
catalogs accompany the change.

Tauri remains available through the existing dev process. Native acceptance is
still the user's review: compare empty/filtered/waiting/error states in Room and
Full, open technical details, retry, and check short/narrow views and keyboard
focus. DOM checks do not measure WebKit layout or the native compositor.

Automated result: TypeScript/Vite build, nine renderer checks, fmt/clippy,
864 workspace tests and 8 eval tests pass. Brand, English, i18n, layer,
network, README translation and whitespace checks pass; the Nest mirror source
mark matches. The running Tauri process received the updated renderer via Vite.

### UX8 profile implementation review - 2026-10-22

The specialist route now requires a native generation even for an empty conversation. Failed reads
keep send disabled and retryable; obsolete reads, sends and failures cannot overwrite another route.
The separate specialist screen has been removed: the existing Chat tab owns recipient selection,
messages, drafts, history, reasoning/tools and approval cards through the production ChatPanel.
Switching recipients clears old history rows before loading the selected history. Approval writes
reject duplicate clicks and shortcuts outside the visible Chat. Empty runtimes retain their pinned identity after profile edits.

The native turn stays reserved through transcript/session persistence and emits completion afterward.
Preflight Stop is cancellable, busy open/reset requests fail promptly, outside-content state resumes
with its conversation, and specialist permission decisions retain their profile audit identity.
Profile saves retain the selected access and reject combined instructions/skills beyond the executor
limit. History erasure also stops and resets specialist runtimes. Automated regression coverage
exercises these failures; live-provider and desktop visual acceptance remain separate.

### UX8 profile flow refinement - 2026-10-22

The selected profile presents Chat first, keeps its access choice visible and moves the full
instructions, pinned skill content and execution scope into retained disclosures. Opening is
single-flight, waits for access persistence and reports errors in the requesting profile. Catalog
reentry refreshes saved access; Chat and profile changes update each other's existing controls.
Pinned conversations disclose their retained revision when a newer profile is available. A new
specialist conversation returns generation and identity together, without a second fallible read.
The retained species/accessory drives both the production animated perch and reply signatures.
Production-renderer checks cover failure/retry, save/open races, revision transitions, retained
appearance/drafts and approval isolation. Native checks cover the reset wire shape. Desktop visual
and live-provider acceptance remain pending; Quick summaries and run projection remain later slices.

### UX8 direct profile creation - 2026-10-22

Agent profiles now offers Create profile as a direct blank editor, alongside the existing preset
copy route. The native create action carries no preset; storage retains empty preset provenance
with revision zero and the profile's own immutable revisions. Copies keep their real preset source.
The shared create/copy/edit editor separates identity/instructions from appearance and optional
skills, adapts to narrow windows and preserves drafts through navigation and failed saves. Saving
starts no turn and retains default Ask access for a new profile. Renderer checks cover direct
creation, distinct wire actions, duplicate-save guards, retry and cancellation; store tests cover
original provenance, retained revisions and rejection of invalid preset/revision combinations.
Desktop visual acceptance remains pending in the running app.

### UX8 character and Chat control refinement - 2026-10-22

Desktop feedback rejected the form-heavy profile layout and oversized native recipient select.
Profiles now uses a compact bird catalog beside the selected animated character, with Chat/copy
actions alongside its identity and secondary explanations behind disclosures. Creation uses the
same production Scene for a larger live appearance preview; inactive pages stop it and reduced
motion stays static. Species labels now use the language catalogs. The native recipient select
retains keyboard behavior within the existing 30 px Chat control sizing.
The maintainer also requested direct access selection for both Zeca and specialists. ADR 0004
records this change. Native Zeca access accepts only Chat/Settings surfaces and publishes authority
after persistence; the shared ChatPanel applies either direction without Settings redirection,
blocks duplicate saves and keeps a failed save visible with the previous access active. Regression
checks exercise both recipients and the visible/hidden character owner. Desktop screenshots were
inspected for the profile catalog; editor and Chat acceptance remain with the maintainer.

The maintainer clarified that no profile-specific prototype exists yet. This is a new UI to refine
together, not an approved prototype implementation. Keep the current layout provisional, review it
with the maintainer before further visual expansion, and do not equate passing checks with design
acceptance.

### Existing Connections check overview - 2026-10-22

At the maintainer's request, pause further agent-profile/skill design work and return to existing
approved prototype areas. This slice implements the segmented check overview already present in
`design/ui-motion-proposal/connections.js` (`checkBar`) and its template styles. Production rows
retain their explicit status text and show failed/running/waiting checks before successful ones.
The overview is bounded to twelve segments plus a remainder count, with complete named states in
the accessible label and tooltip. The existing detail remains the owner of the full matrix; no
check data produces no invented success summary. Selection, filtering, merging and polling retain
their existing contracts. Renderer checks cover mixed/absent/large matrices, failure visibility and
unchanged source ordering. Native visual acceptance remains for the running app.

### Existing History and Audit search refinement - 2026-10-22

Following the accumulated-work commit, continue within the approved existing-feature prototype.
`design/ui-motion-proposal/records.js` names the History search Search turns and the Audit search
Search answers and commands; production now uses those same scoped labels and accessible names.
Clear filters is disabled for an empty query. Escape clears a non-empty query in place, preserves
IME composition and lets an empty search's Escape reach the owning surface. Existing ordered
query persistence, independent History/Audit filters and loaded-record search remain unchanged.
Production-renderer checks cover both labels, clear state, focus, composition, ordered saves and
zero extra record reads. Profiles and skills receive no new work in this slice. Native visual
acceptance remains for the running application.

### Existing Week image export feedback - 2026-10-22

Continue within Activity's existing Save as image action from the approved records prototype.
The production Week renderer now shows Saving image and disables repeated export/privacy edits
through PNG generation and the native save dialog. Export captures the displayed week's snapshot
and privacy choice, preserves pending state across refreshes and scopes completion feedback to
that period. Changing weeks cannot show the old export's result on the new week. Encoding and save
failures remain retryable; native cancellation reports no false success. Result messages expose
status/alert roles. Regression checks run the real image renderer with a held encoder and mocked
save dialog, covering duplicates, privacy, navigation, refresh, errors, cancellation and success.
No actual image is written by the checks; native dialog/visual acceptance remains for the running
app. Agent profiles and skills remain paused.

### Week privacy toggle feedback repair - 2026-10-22

The image privacy choice now updates only the current export draft, without a
settings-save status. Standalone persisted settings reuse one status node per
control, including failure and retry, instead of appending a node on every write.
Renderer regression checks cover twenty consecutive changes, rollback and
independent sibling controls. Build, Settings checks, English and i18n checks
pass. The running development app received the update; native visual acceptance
of the recap card remains with the user.


### Connections For Zeca - 2026-10-23

Deliver the approved `conn-face-zeca` prototype against actual capabilities. For you keeps the
existing list/detail, filters and confirmed merge. For Zeca exposes the built-in GitHub snapshot
read used by API Chat and realtime voice, with a native persisted switch checked per call and
forwarded through scoped sources. Writes publish only after persistence. Existing installs retain
their enabled cached-read behavior; neither audience switch enables a connector or makes a request.
Recorded tool calls and last use come from local spans, with a link to Activity traces. The
connector's rolling network limit is distinguished from free cached reads. Issue creation and
model-initiated merging remain unavailable, with no pretend switches. Audience navigation is
retained with the existing Connections place. Profiles and Studio remain paused.

Regression checks cover loading/retry, duplicate writes, pending choice, save rollback, retained
selection/audience, off states, unsupported tools and no remote actions on navigation. Native tests
cover live revocation through scoped sources, legacy settings and allowed webview surfaces.
Desktop layout acceptance remains with the maintainer in the running app.

### GitHub account context - 2026-10-23

The existing GitHub poll includes the authenticated viewer's login in the same response, so
Settings can identify the `gh` account without a second request. The label is metadata: it does
not become a watched item or emit connector news, and status counts exclude it. No token, scopes,
or repository content is exposed by the status field. The remaining GitHub roadmap is staged in
the [GitHub for Luna implementation plan](plan-github-luna.md): authentication and capability diagnosis, complete bounded reads,
model tool access, guarded writes/reviews/Actions, merge checks, and specialist binding. Account
identity alone does not prove access to any particular repository or permission.


### GitHub pull request proposals - 2026-10-23

GH4 and GH5 extend the existing Chat tool cards with typed issue and pull request proposals.
Connections keeps the same For you / For Zeca audience and live switch; its copy now names
approved issue and PR changes, and its usage count includes the write tools. No additional editor
or independent review surface is introduced. Human cards show exact content and recipients,
current refs/SHAs and inline context. Ready transitions explain code-owner requests. Changed
heads, bases or anchors require a fresh proposal; unknown outcomes remain explicit in Activity
and Audit. Native visual acceptance of these cards remains pending.

### Agent refinement proposal - 2026-10-24

Pause specialist GitHub integration while refining agents with the maintainer. The agreed flow
separates profile identity/instructions/skills from its executor. Profiles will have per-agent
executor/model defaults, optional per-conversation project selection, the current searchable
skill catalog, and archive/restore for custom agents. Existing conversations retain their pinned
identity, revision, executor and source. Provider choice must not import Zeca's memory or tools.
API/local providers retain their existing capabilities; terminal execution is not added by a role.

Deliver the visual proposal before native persistence or executor changes. A development-only
entry in the existing Profiles view reuses the theme, sprites, Scene and production ChatPanel.
The proposal clones only preset data and owns in-memory examples: save, copy, defaults, skills,
archive/restore, project selection, example attachments and Chat replies make no native write or
provider request. A visible banner identifies those limits. Production specialist execution remains
unchanged. VITE_AGENTS_PREVIEW=1 opens this proposal on the development Nest navigation route;
without it, the normal Profiles view exposes the explicit preview button in development builds.

Acceptance before native implementation: review the catalogue, create/edit flow, animated identity,
skill selection and compact Chat header with the maintainer. Renderer regressions cover catalogue
immutability, search and filtered empty states, draft save/cancel, skills and defaults, retained
simulated messages/project, archive/restore and explicit preview exit. Build and translations must
pass; inspect the running app and retain any unperformed native matrix checks as pending.

The catalogue, editor and Chat proposal were inspected in the running app with Black at the
maintainer's Room size. This caught and corrected single-card stretching, native-select styling,
portrait/greeting overlap and prototype field styling leaking into the production composer.
Light and the 480 x 360 native layout matrix remain pending maintainer acceptance. Development
QA may start directly in the editor or Chat with VITE_AGENTS_PREVIEW_START=editor or chat;
normal preview startup always opens the catalogue.

Next after visual acceptance: amend ADR 0018; add profile preferences and archival metadata with
backward-compatible defaults; generalize specialist initialization/resume to the Chat's existing
Claude, Codex and API/local routes; preserve scoped drafts/history/approvals and validate project
selection. Finish provider/capability, revocation, restart and native layout checks before GH8.

### Agent refinement stage 2 - 2026-10-24

The maintainer accepted the visual proposal and requested the next stage. The shared workspace now
renders the real profile catalog, creation/copy/edit flow, searchable skills and preferences. The
legacy development navigation flag VITE_AGENTS_PREVIEW=1 now opens this real workspace; the isolated
preview remains a fixture for regression checks, without being the production Chat route.

Migration 33 stores versioned execution defaults and archive/restore metadata separately from
immutable content revisions and per-profile access. Content/defaults save in one transaction.
Stale updates reject without replacing the draft. New profiles/copies retain Ask. Built-in defaults
can change without creating a copy. Archived custom agents retain history and revisions; native
send/reset admission rejects them while Stop and pending human decisions remain available.
The Chat picker omits archived agents except its active recipient, without falling back to Zeca.

Claude defaults apply only before the first turn. Codex and API/local preferences are persisted
against the current provider registry, but an explicit notice blocks unsupported new specialist
conversations. Existing Claude conversations remain pinned. Multi-executor runtimes, optional
conversation projects and attachments are stage 3, followed by the integrated stage 4 checks.

Validation: Store 109 tests; app 401 passed / 1 ignored; targeted all-target Clippy with warnings
rejected; renderer build, surface continuity, Chat models, Settings, island geometry, formatting,
brand, English, translations, layers, network and README checks. Renderer regressions include CAS
conflicts, concurrent archive during edits, save retry/single flight, access rollback, registry IDs,
archived Stop/cards/draft retention and no executor fallback. The production catalogue was inspected
in the running native app with Black at Room size. Light, 480 x 360 and manual create/archive flows
remain for desktop acceptance; automated state/DOM tests do not replace that matrix. No commit or PR.

### Agent refinement stage 3.1 - 2026-10-25

Connect the existing API/local provider registry to specialist Chat with the exact pinned profile
brief on both API wires. The prompt includes neither Zeca's persona/name nor companion memory or
tools. Retained API messages are scoped by profile/provider/model, and malformed or foreign system
history is rejected. Defaults select the provider and model before the first turn, with the saved
model for that provider as fallback. Changing future defaults never relabels a bound conversation.
API agents have no project, file, terminal or GitHub capability; errors preserve editable drafts.

Claude specialists can explicitly select a project from the existing sessions' folder picker before
the first turn. Validate existing absolute paths, canonicalize them, and bind the source with the
conversation. Reset clears the old project. Isolated settings and the human approver remain in force;
selecting a folder imports no companion context, flock references, MCP or permission settings.

The Codex app-server currently inherits external configuration; its CLI consultation isolation is a
separate, tool-free path. Do not silently reuse either as a qualified specialist. Codex execution,
per-conversation executor/model overrides and attachments remain subsequent stage 3 slices. GH8 stays
paused. No model request, key, CLI configuration, dependency, commit or PR is created by these checks.

Checks: app 402 passed / 1 ignored; Chat 168 passed / 4 ignored; local HTTP captures on both wires
assert the exact system brief and absent tools; native round-trip covers provider/model/profile,
API history and project source rejection. Renderer checks cover explicit project selection, API
message routing/capability labels, default changes on a bound API chat, and archive continuity.
Build, targeted all-target Clippy, Chat models, surface continuity, Settings, geometry and repository
language/layer/network checks pass. Real cloud authentication/inference and the full Light/narrow
matrix remain manual acceptance. The existing development start=chat flag opens the real specialist
route for native inspection, without executing or simulating a turn.

Native inspection: the actual Developer Chat route was opened with the shared project selector, animated profile perch and composer. The screenshot exposed a CSS override of the Memory button hidden state; its selector now honors hidden. API-specific visual acceptance and Light/minimum-size checks remain pending.

### Agent refinement stage 3.2 - 2026-10-25

The installed CLI is now 0.164.0. Its app-server does not expose exec's ignore-user-config/rules
switches; a profile cannot inherit external configuration while claiming specialist isolation.
Retain the explicit Codex block, without falling back to another executor. Source inspection used
upstream [rust-v0.164.0 CLI](https://github.com/openai/codex/blob/rust-v0.164.0/codex-rs/cli/src/main.rs), [app-server](https://github.com/openai/codex/blob/rust-v0.164.0/codex-rs/app-server/src/lib.rs) and [config loader](https://github.com/openai/codex/blob/rust-v0.164.0/codex-rs/config/src/loader/mod.rs), not a model call,
forced login, credential copy or configuration edit. Native Codex activation is still unfinished.

Deliver the next independent planned slice: per-conversation executor/provider/model/CLI effort in
the existing Chat model menu. Native configuration is scoped by profile and source generation,
validated under the runtime lock and rejected after any saved first turn. Publishing advances the
generation, preserving draft text and profile identity; older sends and draft writes cannot inherit
the choice. Apply does not start a model or alter profile defaults, skills or access. Unsent choices
are retained with the runtime, not on disk. New/Open/history reset them; unavailable defaults may
be shown on an empty new conversation without running a substitute executor.

Renderer checks cover duplicate writes, failure/retry, typing retained, native profile/generation
scope, catalog immutability, reopening, fixed started conversations, and a delayed choice reply after
history reset. Native checks cover source ownership, draft preservation, changed generation, an
already saved but unstarted conversation and rejected foreign/companion sources. App: 403 passed,
1 ignored; targeted all-target Clippy, build and existing renderer/geometry/language checks pass.
Remaining stage 3: qualified Codex isolation/activation and explicitly chosen file attachments;
then stage 4 integrated acceptance and GH8. No commits or PR.

Native acceptance for this slice: inspected the real conversation setup popover in Black at Room size. Fixed its overflowing model input with border-box sizing and bounded horizontal overflow. Assigned unique focus identities to all setup controls; checkbox repaints do not attempt text caret restoration. Light, minimum-size desktop acceptance and manual authenticated inference remain pending. The app remains open; no model or configuration write was triggered by opening the menu.

### Agent refinements commit checkpoint - 2026-10-25

The maintainer authorized committing the accumulated refinements before the next implementation slice. Full workspace Clippy with warnings rejected, workspace tests, feature-enabled evals, renderer build, geometry, record search, saved close, Chat models, Flock continuity, surface continuity, Settings and repository checks passed. The requested check-task-actions.mjs script does not exist in this checkout; the available production surface checks were run. No dependencies changed. Light/minimum-size desktop and authenticated inference acceptance remain pending, rather than being marked as automated coverage. No PR or push is authorized here.

### Agent refinement stage 3.3 - 2026-10-25

After commit 9b4bfe95, add explicit specialist attachments through a generation/profile-scoped
chooser response rather than the companion files event. The control reuses ChatPanel's chips,
removal and composer. Zeca may stay off. Late responses cannot cross reset or recipient boundaries;
chooser failure/cancellation does not send, replace a draft or end a streaming turn.

Validate all rendered attachment paths at native send admission: immediate regular inbox copies,
no traversal, links, nested/foreign paths, duplicates or missing files. Bound each message to 16
files and 20 MB. New private copies use exclusive creation rather than overwriting a same-name
copy/link. Shared Chat marks attachments as outside input before any model/tool call, so a full
access choice cannot skip the outside-content card path. API history continues to strip file bytes.
No tools, project capability, configuration, key or model request is enabled by the chooser itself.

Tests cover path/size/count/duplicate/link/missing boundaries, private copies and original retention;
shared taint with no real model request; an intercepted local API payload and stripped disk history;
and the actual specialist adapter through pending/failure/retry, source reset, recipient change,
files-only submission, cancellation and streaming continuity. Native dialog completion, minimum-size
and Light visual acceptance remain manual. Codex activation remains blocked pending isolation;
file drag-and-drop is not routed to specialists. This new slice remains uncommitted for review.

Stage 3.3 validation: app 404 passed / 1 ignored; Chat 169 passed / 4 ignored; targeted all-target Clippy, build, surface continuity, Chat models, geometry, Settings and language/layer/network/README checks passed. Inspected the production Developer Chat in the running native app: its paperclip shares the existing composer without increasing the resting height. Actual chooser completion and authenticated model delivery remain manual acceptance; no user file was opened or sent during automated validation. Commit 9b4bfe95 contains the prior refinements only; these attachment changes remain local.


### Agent refinement stage 4.1 - 2026-10-25

Repair integrated history lifecycle before the remaining acceptance matrix. Native rename/delete
checks the stored recipient in the same store operation as its mutation; Zeca and specialists
cannot mutate each other's records. Deletion takes the runtime lock without waiting for a turn,
prepares its replacement, then deletes before clearing any runtime or draft. An archived agent's
current conversation can be deleted without invoking the archive-blocked New action. A successful
specialist deletion returns its new generation and latest identity to the existing Chat adapter.

The shared renderer serializes history writes, retains rename text and delete confirmation on
failure, shows translated errors for retry and keeps cached rows when listing fails. Successful
current deletion clears attachments, draft, project and conversation model override. Late results
cannot clear a newly opened source; stale list reads are discarded. Regression checks exercise
both shared surface modes and the production specialist adapter. Native ownership checks use a
temporary store, never user history.

Validation: app 404 passed / 1 ignored; targeted all-target Clippy, build, Chat models, surface
continuity, geometry, format and repository language/layer/network/README checks passed. Inspected
the updated Developer Chat in the running native app. Actual native history mutation, chooser
completion, authenticated model delivery, Light and minimum-size acceptance remain pending;
Codex isolation remains blocked. These changes and stage 3.3 remain local, with no push or PR.


### Agent refinement stage 4.2 - 2026-10-25

Fence New/Open transitions by recipient generation in both the shared renderer and native commands.
The native navigation guard takes the runtime lock without waiting and validates the expected
generation before reading/restoring history or closing any approval waiter. This also serializes
Open against Delete, so a previously fetched deleted record cannot become the active runtime.
Companion routes forward the same generation as specialist routes; stale or busy calls leave the
current runtime and draft untouched.

ChatPanel owns one pending navigation across New/Open, saves the draft before switching and blocks
sending or another navigation while it completes. Failure retains the draft/files with a translated
retry message. External source changes outrank delayed results. Specialist adapters return reset
identity unchanged; ChatPanel applies it only after its source check, including successful current
history deletion. This prevents an ignored reset from still overwriting the new recipient's name,
revision or appearance. Recent uses the same pending state without another navigation owner.

Regression checks cover both shared surfaces: duplicate New/Open, failed draft save and reset,
retry, generation forwarding, stale opened/reset results, retained newer draft/identity and a
successful latest-revision reset. A native guard check covers busy/stale rejection with intact
current record and draft. Native dialog/model and Light/minimum-size acceptance remain pending;
Codex activation remains blocked on CLI isolation. No push or PR.

Stage 4.2 validation: app 405 passed / 1 ignored; targeted all-target Clippy, renderer build,
Chat models (including initial-restore/external-reset race), surface continuity, geometry and
repository language/layer/network/README checks passed. The updated production Developer Chat
was inspected in the running native app at Room size. No user conversation was renamed, deleted
or sent for these checks; native command interaction and the remaining visual/manual matrix are
not marked as accepted. Changes remain local.


### Agent refinement: Codex and GH8 - 2026-10-25

The maintainer requested Codex specialist execution and independent GitHub enable/disable for
Developer and Reviewer. This explicitly supersedes the earlier Codex/GH8 pause. Codex uses an
app-owned CLI home and ephemeral external subscription auth from the existing CLI login, without
copying credentials or inheriting user/project configuration. An installed-CLI no-model canary
qualifies configuration isolation; an isolated error cannot fall back to native exec. Resume keeps
the saved profile/model/effort/session. Live authenticated inference remains acceptance work.

Profiles reuse the accepted workspace to show an exact repository, Enable/Disable GitHub and the
saved host/account. Each profile has its own versioned, default-off grant. Saving is CAS guarded;
a failed save keeps the repository input and prior choice. The Chat notice shows the selected
scope. GitHub tools are separate from Zeca and continue to require the connector, its budgets,
network traces, exact human write cards and live grant/account checks. Supported API tools run in the
host; local providers retain messaging only; Claude/Codex use a private per-turn same-user relay. No companion memory/flock tools or
global GitHub discovery are offered. Typed issue/PR/review/Actions/merge services remain shared.

Tests cover grant independence, repository and account changes, revocation/re-enable, human-only
writes, policy deny, MCP service cleanup/disconnect and UI duplicate/failure/retry. Workspace tests,
targeted/native qualification, renderer checks and Clippy pass. Cargo deny currently fails on the
pre-existing phone-signing crypto-bigint advisory RUSTSEC-2026-0387; it is not suppressed. Real
model/GitHub repository mutation and Light/minimum-size acceptance remain open. No commit/push/PR.

Delivery verification: workspace 1597 passed / 18 ignored; Chat 173 passed / 5 ignored; app 406
passed / 1 ignored; feature-enabled evals passed. The explicitly invoked installed-Codex canary
passed with no model request. Workspace all-target Clippy, build, surface continuity (including
Codex executor/model/effort forwarding), Chat models, Settings, geometry, records, saved close,
Flock placement and repository checks passed. Rechecked native Codex profile/session resume in a
temporary store. Inspected the production profile catalogue in the running native app at Room
size; the GitHub section is present in Developer's detail. The app remains open on Profiles for
maintainer acceptance. Live inference/repository writes and Light/minimum layout remain manual.
Cargo deny still reports the pre-existing signing dependency advisory; no ignore was added.


### PR readiness - 2026-10-25

The earlier dependency gate is resolved: p256 0.14 removes the vulnerable crypto-bigint 0.5 chain;
clipboard-win is updated to 5.4.3 with the existing scoped license exception reviewed and updated.
Cargo deny passes without a new advisory ignore. Phone signing retains SHA-256/ECDSA DER and SEC1
wire formats, checked with an independent OpenSSL signature fixture.

Managed Codex state is partitioned by profile and hashed principal/workspace. Thread ownership
rejects a different login, including another user in the same workspace, before spawning a resume.
A new conversation can use the new login; token rotation keeps the original identity. A fake CLI
integration checks start/resume, changed login, new conversation, external MCP rejection before
authentication and absence of native-exec fallback. The installed CLI no-model canary also checks
the explicitly scoped MCP configuration. No live inference or repository mutation was performed.

Final workspace verification: 1600 passed, 18 ignored, no failures; feature-enabled evals, all-target
Clippy, renderer checks and repository rules pass. Native Room profile inspection is recorded in
`assets/agent-github.png`. Live model/repository acceptance and Light/minimum layout remain manual;
GH9 remains open. The maintainer authorized commits, push and a PR for these accumulated changes.


### Dictation navigation and settings refinement - 2026-10-25

Start the maintainer-requested Dictation front-end refinement with the existing shared navigation,
without changing capture, insertion, provider consent or webview authority. Group work pages and
settings by purpose; search translated names/descriptions locally, preserve the query through
settings refresh and recover empty searches with Escape. The narrow picker uses the same groups.
Sidebar arrows and Home/End move focus without activating a page. Voice separates model/language,
microphone and sound/feedback, with advanced processing in a retained disclosure. All setting
saves, errors, drafts and native routes keep their existing owners.

Renderer checks cover grouping, search, no implicit writes/navigation, empty recovery, retained
query/focus and keyboard movement, alongside existing failure/retry and cross-surface checks.
This is the first refinement slice; the other work-page layouts and final native accessibility
acceptance remain separate.

Verification: renderer build, Settings (including grouped search and keyboard regressions),
surface continuity, dictation History/Words/Styles, geometry, records, saved close, Chat models and
Flock placement pass. Language, translation, brand, layer, network and README checks pass.
Inspected production Voice settings natively in Black and Light, including the 480 x 360 window;
the short-window pass caught excess heading space and now keeps the first action visible.
Restored the original window bounds and saved theme, removed all temporary inspection code,
and left the app open at Dictation > Voice using the existing user configuration. No microphone
recording, model download or permission change was made. Full human keyboard/screen-reader and
remaining work-page acceptance stay open. No commit, push or PR for this slice.


### Dictation History readability and accessible actions - 2026-10-25

Keep the delivered shared navigation and refine History/Voice commands on their existing owner.
Align the bounded transcript column and toolbar; show written text, delivery and destination first.
Original transcription, style/language and measurements live under Details. Today's figures start
folded in every layout and retain an explicit open/closed choice through refresh. The result count
shares that overview row, leaving the first record readable at 480 x 360. Local filtering includes
a count, Escape to clear and an explicit Clear search action on filtered empty results.

Bulk erasure moves into History options; commands still cannot erase ordinary dictations or replay.
Inline deletion focuses Cancel first, restores the originating row on cancellation and keeps safe
focus after a failed deletion. Successful non-destructive actions retain their row focus. The
renderer regression covers search recovery/counts, safe confirmation, failure/retry and retained
focus/statistics disclosure, plus the existing clipboard, stale reads and retention boundaries.

Inspected the running production History with existing records in Room and at 480 x 360 (Black),
and the filtered-empty view at 480 x 360 (Light). No retained transcript, clipboard, permission or
provider setting was changed. Temporary inspection code was removed and the original bounds/theme
restored. Native destructive clicks and a full screen-reader pass remain manual; their contracts
are covered by the renderer checks. Changes remain local on the same branch, with no PR.


### Dictionary and Snippets discovery and forms - 2026-10-25

Continue the same Dictation refinement branch on the existing Your words owner. Dictionary and
Snippets have independent local searches across keys and saved bodies, ignoring case/accents.
Filtering preserves form drafts, does no read/write, and offers Escape/clear recovery. Creation
forms precede the lists; grouped sections and wider multiline fields retain literal whitespace
and validation. Native forms prevent navigation on submit. Try a sentence starts folded without
recreating its editor or preview owner. Existing same-source replacement updates remain unchanged;
this slice adds no snippet update/rename contract.

Deletion keeps the safe confirmation and returns to the exact entry action on cancellation, with
search fallback when an entry is filtered away. Refresh restores a keyed entry focus; failed
mutations retain confirmation, drafts and safe focus. Read/save failures are announced as alerts.
Renderer checks cover independent key/body filtering, accent matching, no implicit reads/writes,
empty recovery, retained drafts, form navigation prevention and cancellation focus, alongside the
existing literal-body, pending/failure/retry and stale sentence-preview checks.

Inspected Dictionary/Snippets in the native app at Room size (Black), and both at 480 x 360
(Light), with Snippets also checked at that size in Black. Real saved entries were not modified.
The populated cases use renderer fixtures; physical keyboard/screen-reader and native destructive
acceptance remain manual. Restored the original bounds/theme and removed all temporary inspection
code. Build, Your words, History, Settings, surface continuity and repository checks pass. No
commit, push or PR; the app remains open on Dictionary for maintainer acceptance.


### Dictation prototype fidelity pass - 2026-10-25

The maintainer requested a closer match to the accepted prototype, rather than only general
usability improvements. Reviewed `design/ui-motion-proposal/dictation.js` and its `template.html`
styles: Voice/Your words/System navigation, the shortcut-and-figures history header, destination
marks, quiet icon actions, word chips and paired replacement/snippet tables. The shared navigation
now follows those groups, with the complete configuration inventory in a searchable disclosure.
Styles still opens its existing privileged editor; no webview capability moved. Saved panel widths
remain respected. Search and accessible narrow navigation remain available.

History displays the actual desktop-bound shortcut, never the prototype's hardcoded example.
Wide measured figures may open initially; unknown values are not fabricated and an explicit
choice is retained. Static decorative bars do not claim active recording. Rows reuse the production
icons and announce copy results; deletion still requires confirmation. Dictionary/Snippets retain
forms before bounded, named, keyboard-scrollable tables so large libraries do not bury creation.
Counts follow the local filter; table headers, literal bodies and unique removal targets remain
accessible. The reference's chips and theme roles replace the generic outlined panels.

Build and all 19 CI renderer checks pass, including live key display, measured-wide figures,
configuration navigation, search recovery, table content, focus and retained drafts. This round
does not claim a new native visual acceptance: the computer-use inventory exposed no browser or
app surface. Existing native development remains running and reloaded the production modules.
The prototype's normal generator currently exceeds its existing inline size budget with current
assets; a disposable inspection build was generated without changing that gate, then removed.
The temporary reference checkout and preview server were also removed. Final native comparison
with the maintainer remains open. No recording, model request, user-data mutation, commit or PR.


### Saved-meeting navigation and voice work-page hierarchy - 2026-10-25

Continue the prototype-aligned Dictation pass on existing owners. At 1100 px or wider and above
560 px high, a selected saved meeting retains its searchable list beside the detail. Narrow
layouts keep the existing back route and preserve the selected meeting through resize. Search
adds a local result count and Escape/Clear search recovery. Selected rows are marked, pending
mutations disable list actions, and deletion keeps Keep meeting before the destructive action.
Export, rename, source selection, history epochs and cancellation retain their existing owners.

Voice conversation separates its settings link/character in the heading, explicit current phase,
primary available turn action, recovered/reply text and grouped Conversation options. The existing
route, device, semantic/follow-up consent and start/finish/cancel listeners are retained. No option
opens capture and no new provider or microphone access is added. Production theme roles and bounded
scroll regions remain shared, including the short-window rules.

Regression checks cover wide/narrow detail selection, selected-row status, local search clearing,
voice settings outside the turn controls and the currently available primary action. Existing
meeting save/rename/delete/export failure and stale-read cases and voice late acknowledgements,
route/device choice, explicit consent, frozen sessions and departure cleanup still pass. This
round's native visual acceptance remains pending because the computer-use inventory exposes no
app/browser surface; the native development app remains running with renderer reloads. Changes
remain local in the same branch, without commits or PR.


### Styles discovery and Diagnostics hierarchy - 2026-10-25

Keep the accepted prototype's four descriptive style cards visible even when clean-up is off.
The new off-state cards are articles without mutation handlers, selected state or provider calls;
interactive choices still require the existing explicit clean-up switch and available provider.
The existing write guards, provider disclosures, quotas, app drafts and failed-save behavior remain.

Diagnostics retains its explicit six-second microphone owner, then presents measured stage timings
before system details. Technical counters move into a disclosure retaining its open/closed choice
through refresh; wide system facts use two columns and narrow pages retain the existing list.
Explicit diagnostic refresh restores its keyboard target after success/failure, without stealing
focus from another control or a departed page. Empty feedback takes no layout space. No missing
reading is converted to a zero or a Ready state.

Regression checks cover read-only off-state previews with no writes, preserved mutation gating,
report order, retained counters and refresh focus, along with existing provider failures, stale
reads, microphone late-start/stop and meter-focus cases. Build and all 19 CI renderer checks pass.
Native development remains running; fresh visual acceptance remains manual because this session's
computer-use inventory exposes no app/browser surface. No audio test, provider request or personal
configuration change was performed. Changes remain local on the same branch with no commit or PR.


### Dictation keyboard and paste configuration clarity - 2026-10-25

Group the shortcut page into global bindings and recording controls. Include the actual independent
voice-command binding next to dictation, copy-last and cancel, rather than requiring the separate
command-settings page to distinguish them. X11-only Space/Escape gestures keep their existing
platform guards in a disclosure with a stable, language-independent retention key. Hold/double-tap
and voice cancellation retain their existing save/pending/failure owners and visible scope notes.

Paste starts with a read-only configured-mode summary: clipboard only, unsupported insertion,
waiting for keyboard access or insertion enabled for supported destinations. Ready and temporary
Wayland access use the existing native state. The summary never asserts a successful delivery;
the final status remains authoritative, including clipboard failures. Existing insertion, portal
retry/token removal and spoken-code controls and their disclosures remain unchanged.

Renderer checks exercise explicit mode changes from permission/settings events without writes or
permission requests, the independent command binding and retained X11 disclosure across repaint.
Build and all 19 CI renderer checks pass alongside repository checks. Native development remains
running; visual acceptance remains pending with the existing lack of computer-use surfaces. No
keyboard permission, OS configuration, microphone, provider or personal history was changed.
Changes stay local on the same branch, without commits or PR.


### Command scope and privacy action refinement - 2026-10-25

Separate local command controls from the existing optional Chat handoff. The committed routing
choice is described as commands off, local-only unknown phrases or a possible handoff subject
to the existing Chat/source/budget guards. The phrase guide remains shared and read-only; work
page guidance is grouped and its focused summary survives relabeling. Settings guide expansion
and summary focus use stable identifiers. No example executes a command.

Privacy presents retention and erasure in separate sections. Erasure starts on Cancel, returns
to its request on cancellation, disables duplicate requests while pending and restores safe
confirmation focus on failure. Success/failure cannot override a newer field focus or another
settings page. Existing clear/retention semantics and explicit confirmation remain unchanged.

Regression checks cover the three routing choices, explicit clear confirmation, Escape return,
pending duplicate guard, failed-clear focus and a newer focus preserved after completion. Build
and all 19 CI renderer checks pass alongside repository checks. Native visual acceptance remains
manual with the current unavailable computer-use surfaces. Development remains running; no actual
command, deletion, permission change or provider request was performed. No commit, push or PR.


### Dictation exact work-page continuity - 2026-10-25

The integrated review found that Nest saved only the broad Dictation tab, so a recreated window
returned to History after selecting another work page. Dictation now reports its selected work
page and notifies the existing Nest navigation owner when an internal selection changes. Nest
saves the already-supported `dictation-<work-page>` route through its existing ordered save/flush
queue, including the Room/Full destination. No native command, persistence format, capture
capability or new private draft/search storage is introduced. Native navigation remains scoped
to the current app run; deeper page-specific selections keep their existing owners.

Integrated renderer checks select Commands, Snippets and Dictionary, flush the native destination,
recreate Nest and confirm the exact visible work page. Room/Full preserves Snippets identity.
The existing statistics destination, shared source/permission ownership, in-view drafts/searches
and no implicit actions assertions remain. Build and all 19 CI renderer checks pass alongside
repository checks. Native development remains running; physical desktop acceptance remains manual
with unavailable computer-use surfaces. No user data, consent or provider settings were changed.
The same branch remains local without commits or PR.

### Dictation navigation fold retention - 2026-10-25

The shared navigation reopened its configuration disclosure on every Settings refresh, including
an unchanged destination. Only entering a different configuration page now expands the group;
ordinary updates preserve the user's manual fold choice. Searching still reveals matching pages
and clearing the query restores that choice. No settings value, route authority or storage changes.

The Settings regression check reproduced the unwanted reopening before the fix and now covers
refresh, temporary search expansion, clearing and a different destination. Build, Settings and
surface-continuity checks pass, as do brand, English, i18n, layer, network, README and diff checks.
The native development app remains running for manual visual acceptance; no computer-use surface
is available for fresh desktop inspection. No commit, push or PR was made.

### Dictation asynchronous action focus - 2026-10-25

History actions and Your words saves restored their original focus unconditionally after an
asynchronous response. They now return focus only while the operation still owns it and the
page remains visible. A newer search, field or navigation target remains focused on success
and failure. Existing confirmation focus, keyboard return, single-flight writes, drafts and
literal snippet bodies remain unchanged; this changes no native execution or storage behavior.

Regression checks reproduced focus theft in both owners before the fix. They now cover History
success/failure, Dictionary success/failure and Snippets completion with a newer search focus.
Build and all 19 CI renderer checks pass. Native development remains open with renderer updates;
fresh desktop visual acceptance remains manual because no computer-use surface is available.
No actual clipboard action, user-data mutation, capture or provider request was performed.
Changes remain local in the same branch, without commit, push or PR.

### Dictation integrated workspace layout - 2026-10-25

Committed the preceding navigation, work-page and settings refinements as `c7863fb8` after build,
all 19 CI renderer checks, Rust formatting, workspace Clippy with warnings denied, workspace tests,
feature-enabled evals and repository checks passed. No dependency changed.

Continue on the same branch with a shared Dictation presentation stylesheet used by Nest and the
privileged Settings webview. Work pages and configuration share a 960 px reading bound, page
insets, compact title scale, existing decorative icons and quieter heading actions. Move the
previous per-page inset rules to this shared owner. Meetings and voice fields use the existing
theme roles, with bounded transcript scrolling, visible recording actions and a separated save
footer. No new page, capture action, permission, connection or storage behavior is introduced.

Container queries use the actual work-page width after the sidebar rather than only the window
width. History rows and figures, word forms, meeting save/export controls and Settings rows adapt
without changing their existing owners or drafts. Viewport-based compact fallbacks remain, and
short windows reduce page spacing. All titles keep semantic headings and the decorative icons
remain hidden from assistive technology. Existing black/light theme tokens are reused.

Build and all 19 renderer checks pass after the markup/layout changes, including translated
labels, saved navigation, search/drafts, async focus, recording controls, confirmation and provider
consent tests. SVG support was added to the two local DOM harnesses that now render title icons.
Repository checks pass. The native app remains running with HMR updates, but fresh native visual
acceptance of Room/Full, sidebar folding, long content and short windows remains manual because
this session exposes no computer-use surface. Automated renderer tests do not measure layout.
This next slice remains uncommitted for review; no push or PR was made.

### Saved meeting list return and keyboard continuity - 2026-10-25

Returning from a saved transcript now restores the list's prior scroll position and keyboard
focus to its originating meeting when that row is still available; otherwise search remains
usable. List rebuilds preserve the outer list scroll and independent wide-sidebar scroll, and
restore focused rows by meeting identity. A delayed list refresh cannot steal a newer search
focus. Existing filter, detail/source selection, explicit export/delete and stale-read guards
remain under the same archive owner. Invalidation clears the temporary position with the view.

The archive regression check reproduced the initial jump to zero, then separately reproduced
browser scroll clamping while replacing the list. Checks cover return position, originating
row focus, a delayed refresh after newer focus and sidebar scroll/focus across relabeling.
Build and the meeting renderer check pass; broader renderer/repository checks are recorded by
the development run. Native development remains open for manual visual acceptance because no
computer-use surface is exposed. No actual history, capture or provider action was performed.
Changes remain local with the integrated workspace refinement; no new commit, push or PR.

### Dictation configuration reading and navigation focus - 2026-10-25

Keep independent scroll positions for the bounded Dictation configuration destinations during
the current Settings window's lifetime. Existing same-page refresh retention remains; returning
to an already visited configuration page now restores its reading position. First visits and
other Settings pages retain their existing behavior. Positions are transient numbers with no
new disk storage, draft serialization, native requests or configuration writes.

Give the shared configuration disclosure summary a stable existing Settings focus key. The
native editor redraw already restores keyed controls after removing/reinserting its navigation;
the summary now participates instead of losing focus after the old tree is detached. The
existing expansion choice and search behavior remain unchanged.

Regression checks reproduced lost per-page scroll and then disclosure-focus loss separately.
They cover independent Paste/Privacy positions, refresh retention and real-DOM-style focus
loss when the root is detached, without settings writes. Build and the Settings renderer check
pass; the broader renderer/repository checks run with this development slice. Native visual
acceptance remains manual with no computer-use surface available; the app remains open.
Changes remain local alongside the pending workspace refinement, with no new commit or PR.

### Dictation consistent search and input-method Escape - 2026-10-25

Reuse one small text-folding function for Dictation navigation, History/commands, Dictionary/
Snippets and saved meeting titles. Searches ignore case and Latin combining accents consistently;
original displayed text, literal snippet bodies and saved values remain unchanged. Query folding
is computed once per words-page draw rather than once for every candidate. No extra IO or writes.

All four search owners now leave Escape to active character composition, including the legacy
229 key marker. Ordinary Escape retains the existing clear-search and focus behavior. This uses
the same guard already present in the local records search and adds no new keyboard shortcut.

Regression checks first reproduced accent-search failures in History and saved meetings and
composition Escape failures in Your words and navigation. All four owner checks now pass with
both composition signals and normal Escape recovery; existing local/no-write behavior remains.
Build passes. Broader renderer and repository checks run with the slice. The native app remains
open for manual visual acceptance with no computer-use surface available. Changes remain local
on the same branch with the pending workspace refinements; no commit, push or PR was made.


### Dictation refinement PR preparation - 2026-10-25

Consolidate the accepted work on this branch for review. Nest tabs and palette destinations now
follow Flock, Chat, Dictation, Connections, Activity; Alt+1 through Alt+5 follow that same order,
skipping hidden Chat as before. The shared Settings shortcut description and all three UI
catalogs and both Nest guides reflect the order.

Pre-commit validation passes: production UI build, all 19 CI renderer checks, Rust formatting,
workspace Clippy with warnings denied, workspace tests and feature-enabled evals. Brand, English,
i18n, layer, network, version, README translation and diff checks pass; changed user-guide source
hashes match their reviewed translations. No dependency or permission rule changed. The local
branch is based on current main without conflicts. Native development remains open; fresh desktop
visual acceptance and a current screenshot are still pending with no available computer-use
surface. Preserve this limitation in the PR checklist rather than claiming automated visual QA.
