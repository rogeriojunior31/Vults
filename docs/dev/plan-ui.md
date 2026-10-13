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

**The first implementation task is UX0: migrate the existing lab to the new UI.** Then
deliver the small fixes to existing features in UX1-UX5. UX6-UX7 change surface
behavior and require native checks. UX8-UX10 are new feature work. UX11-UX12 land
with their owning K/M/W/L/G steps, rather than as an unrelated redesign.

| Order | Task | Priority / size | Dependency | Delivery |
| --- | --- | --- | --- | --- |
| 1 | UX0 Lab migration | P0 / medium | None | The new UI becomes the main development lab for subsequent steps; the old lab may remain separately as `old` |
| 2 | UX1 Names and keyboard | P1 / small | UX0 | Existing controls become accessible without changing authority |
| 3 | UX2 Narrow and short layouts | P1 / small slices | UX0 | Decisions, composer, settings and return actions remain usable |
| 4 | UX3 Cost warning consistency | P2 / small | UX0 | One set of thresholds and explicit cost provenance |
| 5 | UX4 Quick presentation | P2 / small slices | UX1, UX2 | Refined Chat, Flock and Week on the existing owners |
| 6 | UX5 Existing detail and settings polish | P2 / small slices | UX1, UX2 | Better task actions, records, connection detail and setup guidance |
| 7 | UX6 Cross-surface continuity | P1 / medium | UX0, UX2 | Selection and drafts survive the real island/Nest handoff |
| 8 | UX7 Paired Mini/Quick extension | P2 / large | UX0, UX6; ADR 0021 reconciliation | Shared horizontal extension validated on Linux |
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

## UX0 - migrate the lab to the new UI, first

- [ ] Transform the existing dev lab into the new UI's development entry, with
  a frontend-only command and no hook or Rust build before opening. The default
  lab opens the new UI and becomes the working base for subsequent steps.
- [ ] If the old lab is retained, keep it at a separate entry explicitly named
  `old`, for checking previous behavior. Do not combine old and new UIs in one
  review shell, side-by-side view or comparison toggle; do not require parallel
  development of both labs.
- [ ] Reuse the existing lab fixtures, settings bridge pattern and production
  components as the migration foundation. Use `createIsland`/`createNest` and
  their typed contracts where applicable; evolve the renderers with their owning
  UX steps. The settings bridge fixture in
  `design/compact-nest-review/inspect.mjs` is a setup reference.
- [ ] Use the generated Compact/Nest proposal as the design reference for the
  migration, including Profiles, Tasks, Studio, Chat and all 13 roadmap examples.
  Keep its build script as the reference owner; do not embed the standalone
  proposal as a second UI inside the new lab. Future views remain clearly labeled
  simulations until their owning native steps are implemented.
- [ ] Add deep links for surface, width/height, tab, subsection, scenario, locale
  and motion. Include Reset and a way to copy the exact test location.
- [ ] Extend the existing frozen-clock behavior (`still=1`, `t=1500`) with fixed
  fixture dates and seeds. Use named scenarios and stable IDs shared by assertions
  and visual checks.
- [ ] Cover normal/destructive approval, text question, busy/stop/retry, source and
  task context, memory, long names/paths, multiline drafts, many attachments,
  empty/error/stale states, Zeca off and reduced motion. Add the future examples
  as simulated fixtures with availability labels. Keep fixture IO local; GitHub,
  model, microphone and relay tests use recorded responses, with live integration
  checks remaining a separate, explicit native test.
- [ ] Reuse `tests/visual/playwright.config.ts` and the existing screenshot
  workflow. Add focused interaction/geometry checks for primary actions, overflow,
  draft continuity and consent. Keep baseline changes deliberate and reviewed.
- [ ] Exercise every page and subdivision before treating the lab as ready.
  Subsequent steps evolve this new lab and add scenarios there instead of creating
  another lab or maintaining an old/new comparison system.

**Files:** `ui/lab/`, `ui/vite.config.ts`, `package.json`, `tests/visual/`,
`design/compact-nest-review/build.mjs` and its verification entry.

**Done when:** one documented local command opens the new UI lab by default;
both of us can open every view and reproduce a shared deep link; the lab is ready
to evolve with subsequent steps. Any retained old lab has a separate `old` entry.
Fixtures do not launch providers, access credentials, write configs or enable
background requests; production build contains no lab entry. Include an
English/localized layout check and an intentional visual baseline review.
UX0 establishes the frontend development base; native features and runtime
validation remain with their owning steps.

## UX1 - accessible names and keyboard, small

- [ ] Recheck B03/B04/B09: settings switches, task status/owner and connection
  dismissal have specific labels. Fix the shared helper or actual owner once.
- [ ] Recheck B07/B08: the existing island/Nest navigation announces selection;
  tab lists implement arrows, Home/End, roving focus and panel association.
- [ ] Keep shortcuts, disabled states and Zeca-off navigation working. Name a
  dismissal by its item, not only its icon.

**Files:** settings controls, `ui/src/surfaces/nest/{head,tasks}.ts`,
`ui/src/island/render.ts`, existing i18n catalogs.

**Done when:** no visible unnamed control; a keyboard-only pass reaches every
existing area and returns from detail. Add focused accessibility assertions to
UX0. **Reference:** B03, B04, B07, B08, B09; D04.

## UX2 - responsive repairs, small PRs

- [ ] Chat (B01/R11): bounded transcript/composer, narrow memory return and saved
  draft; remove composer/footer overlap at 480 x 360.
- [ ] Settings (B02): adapt the page selector and stack wide controls without
  losing advanced options or page selection.
- [ ] Approval (R01): shorten the flock scene in low windows so the current card
  and both decision actions are visible before scrolling.
- [ ] Targets (R12): enlarge the usable area of small session/size controls.
  Preserve sprite scale and separation between adjacent targets.

**Files:** `ui/src/island/chat.ts`, `ui/src/surfaces/nest/nest.css`, settings CSS
and shared controls. **Done when:** all ten settings pages fit horizontally;
composer/save bars stay inside the frame; pending/destructive cards are usable at
480 x 360 and the existing Quick width. **Reference:** B01, B02, R01, R11, R12.

## UX3 - cost warnings, small

- [ ] Recheck B06. Use the same usage thresholds in Cost and Flock: warning at
  70-89%, critical at 90%+, and a readable status in addition to color.
- [ ] Retain API estimate versus subscription usage distinctions. Missing usage
  and cost remain unknown; do not replace them with zero or measured totals.

**Files:** `ui/src/surfaces/nest/{records,flock,budget}.ts`.
**Done when:** fixtures at 69/70/89/90/100% agree between views, including labels.
Observed terminal telemetry belongs to W9b, not this fix. **Reference:** B06.

## UX4 - existing Quick content, small slices

- [ ] Chat: short recipient/executor/history header; one set of context/memory
  controls beside the companion; empty-state prompts; two comfortable draft
  lines where height permits. Keep attachments, voice, skills, send/stop and the
  current conversation/source owners.
- [ ] Flock: use a short selected-session summary and on-demand detail; preserve
  the actual session state and keep a pending decision first. Remove repeated
  labels and tall metadata wrappers.
- [ ] Week: period and metrics precede explanation, with concise export/detail
  actions. Use local scroll only for content that actually exceeds the region.
- [ ] Reuse the native theme and readable identity labels; do not replace the
  production palette with the older prototype's sage theme.

**Files:** `ui/src/island/{chat,quick,render}.ts`, shared theme, Nest Chat/Week.
**Done when:** defaults fit at the current size and proposed lab widths; long
content stays editable; history/context return preserves the draft. New Routine
controls remain tied to K4, and new specialist Chat remains tied to UX8.
**Reference:** R02, R05, R13, R19; C03/C09; D01/D02; E01-E03 (prototype revision 05).

## UX5 - existing detail and setup, separate small slices

- [ ] Tasks: goal/status/next step first; lifecycle and maintenance under labeled
  disclosure. Preserve issue actions, criteria, budget, run/workspace handling,
  continuation, handoff, independent review and confirmation for removal.
- [ ] Connections: below the useful two-column width, selection opens detail in
  its own view; Back restores item, repository filter and focus.
- [ ] Activity: short project summaries; searchable/grouped History and Audit;
  tools inside their Trace turn; metrics distinct from retention settings.
- [ ] Settings: concise effects, advanced disclosures, per-agent connection
  review, rule scope/revocation and clear phone setup prerequisites.
- [ ] Empty and filtered-empty states expose a real first step or clear-filter
  action. Added filters use existing data; heavy new queries get their own slice.

**Files:** `ui/src/surfaces/nest/{tasks,connections,records,roosts}.ts`, settings.
**Done when:** detail return preserves state; grouping hides no working action;
delete task and remove workspace remain distinct. **Reference:** R03, R06-R10,
R14-R19; D04/D05 where they refine already-existing task behavior.

## UX6 - continuity with native ownership

- [ ] Reproduce B05 in the real app. Carry Place/tab/subsection, selected session,
  repository filter and conversation through island/Nest transitions.
- [ ] Keep one conversation/run owner: resize and return do not reset an unsent
  draft, duplicate a run or detach its source. Preserve pending-card routing.
- [ ] Add the integration check for Room/Full/Quick, minimization, returning to
  the island, and Zeca off; make the lab's transition fixtures exercise the same
  state contract.

This native slice can start after UX2 while renderer-local UX3-UX5 land; it does
not need to wait for the future Agents or Studio features.

**Files:** `app/src/{panel,lib}.rs`, `ui/src/bridge.ts`, island/Nest entry points.
**Done when:** a desktop pass confirms continuity and authorized card handling;
renderer-only results cannot close this task. **Reference:** B05, R04, C01/C06.

## UX7 - shared extension, native feature work

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
[original Studio specification](../../design/agent-studio/design.md).

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
production task. A finding already fixed on the current head becomes a lab
regression instead of another implementation ticket.

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
| D02 | Theme baseline in UX0/UX4 and root DESIGN.md, not a new theme feature |
| D03/D04 | UX8; accessibility/disclosure rules reusable in UX1/UX5 |
| D05 | Existing task polish in UX5, specialist identity in UX8 |
| D06-D10/D13 | UX11, with their original roadmap owners |
| D11/D12 | UX12; W9b cost provenance in UX11 |

## Review and completion

### Handoff for the next model — 2026-10-13

Continue on `feat/ui-roadmap`. The verified implementation reference is
`22199c12`, following `138829cb`, `96e0eb4e`, `30b865a0`, `da65ce6e`,
`2a16059b`, `06c5aa33`, `ddea9584` and the UX1/UX3 commits. At that head,
`npm run build`, `scripts/check-i18n.sh` and all 280 visual tests passed.
This is evidence for the existing renderer slices, not completion of UX0 or
validation of the native app. Check the current Git state before starting;
there are parallel uncommitted design, roadmap and Vite changes.

Already delivered: named settings/task controls, Nest keyboard navigation,
island selection announcements, consistent usage warnings, short Chat composer,
narrow Settings navigation, short Flock decision priority, narrow Memory return
with draft/focus preservation, and a visible localized Memory label. Session
and size targets were refined from 28 to 24 px after visual review; session
dots remain 9 px visually. Treat 24 px as the current compact-control choice,
not a blanket replacement for the larger desktop-control guidance in DESIGN.md.

The old audit checkboxes above remain reproduction prompts. Do not implement a
duplicate repair merely because its checkbox is unchecked. UX0 still needs its
own acceptance review; do not mount the entire generated proposal in the lab
or combine old/new surfaces to make it appear complete.

### Next slices and focused acceptance

Deliver one row per implementation/commit. Inspect its current behavior first;
if the problem is already solved, record the evidence and move to the next row.

| Order / owner | Reproduce in the existing lab | What must hold after the change | Existing tests to run |
| --- | --- | --- | --- |
| 1 — UX4 Chat | `/lab/?still=1&t=1500&state=chat&open=1`; `/lab/nest.html?still=1&tab=chat` | Long source/model/folder labels wrap without overlap; context/history/Memory have one clear entry each; a multiline draft survives opening and returning from those panels; send/stop, attachments and voice remain reachable. Two draft lines where height permits; composer and footer fit at 480×360. | `island.spec.ts`, `nest.spec.ts`, `webkit.spec.ts`, filtered with `[Cc]hat\|memory\|conversation\|paperclip\|talk shortcut` |
| 2 — UX4 Flock | `/lab/?still=1&t=1500&state=busy-flock&open=1`; `/lab/?still=1&t=1500&state=destructive-approval&open=1`; `/lab/nest.html?still=1` | Selected session, project, agent and next action are clear; detail is available on demand; switching session updates the actual owner; pending approval stays first at 480×360. Deny/Allow and text-question answers remain usable, and destructive requests gain no permanent permission. | `island.spec.ts`, `nest.spec.ts`, `webkit.spec.ts`, filtered with `flock\|session\|permission\|approval\|question\|quiet` |
| 3 — UX4 Week | `/lab/?still=1&t=1500&state=idle-flock&open=1`, then Activity; `/lab/nest.html?still=1&tab=week`; `/lab/activity.html` | Period and metrics precede supporting explanation; previous/current week work; export is reachable; empty week remains explicit; no new page overflow at 480×360. | `sizes.spec.ts`, `activity.spec.ts`, `nest.spec.ts`, `webkit.spec.ts`, filtered with `week\|Week\|Activity\|history` |
| 4 — UX5 Tasks | `/lab/nest.html?still=1&tab=tasks` | Goal/status/next action stay visible before maintenance; disclosures retain issue, criteria, budget, run/workspace, continuation, handoff and review actions. Task removal and workspace removal remain distinct; canceling confirmation changes nothing. | `nest.spec.ts`, `webkit.spec.ts`; inspect task fixtures and test names before choosing the exact filter |
| 5 — UX5 Connections | `/lab/nest.html?still=1&tab=connections` | At 480 px, selection opens readable detail; Back restores selection, repository filter and focus. Loading, off, stale/error and empty states remain distinct; merge still requires confirmation and operates on the selected item. | `nest.spec.ts`, filtered with `connection\|connector\|merge`; add one return-state assertion if absent |
| 6 — UX5 Records/settings | `/lab/nest.html?still=1&tab=activity`; Settings through the mocked `settings.spec.ts` setup | Choose one subsection per slice. Search/grouping hides no retained action; Trace tools stay in their turn; settings effects and advanced options remain understandable and accessible. Check empty and filtered-empty states separately. | Relevant cases in `activity.spec.ts`, `nest.spec.ts`, `settings.spec.ts` and locale tests; discover actual titles before filtering |

The table lists review routes, not an assertion that every edge-case fixture
exists. Extend the smallest existing fixture when needed; do not create a new
lab or a parallel component system. UX6 onward additionally needs native
ownership/window checks and must not be declared validated by these fixtures.

### Fast validation loop

1. Record the owning UX ID, exact route, affected surface and the observable
   before/after behavior. Read DESIGN.md and the component's real action owner.
2. Run the relevant existing tests in compare mode before editing. Inspect each
   failure. A missing locator is not automatically a product bug, and a changed
   screenshot is not automatically an acceptable design change.
3. After the edit, run build, i18n and the focused interaction/layout tests.
   Inspect screenshots at Quick's real size, Nest 760×640, Nest 480×360 and a
   wide window when that surface changes. Include English and pt-BR, long
   labels, keyboard focus and reduced motion where relevant.
4. Review actual versus previous images before updating a baseline. Check
   hierarchy, spacing, target separation, sprite scale and action visibility;
   geometry assertions alone do not prove the UI looks good. Update only the
   affected tests with the official container runner, then compare again.
5. Run the full visual suite once before closing a slice that changes shared
   CSS/helpers, header/footer, navigation or multiple surfaces; otherwise use
   the focused suite and run full coverage at the UX milestone. Do not repeat a
   passing full suite without a subsequent change or unresolved failure.
6. Report exact checks/results, remaining native limitations and the commit.
   Never claim success from partial console output; await the process exit.
   Preserve unrelated working-tree changes when staging.

Example for the next Chat slice (the pipe is a quoted Playwright regex):

```sh
npm run test:visual -- island.spec.ts nest.spec.ts webkit.spec.ts --grep '[Cc]hat|memory|conversation|paperclip|talk shortcut'
npm run build
scripts/check-i18n.sh
git diff --check
# Only after inspecting the intended image changes:
npm run test:visual -- island.spec.ts nest.spec.ts webkit.spec.ts --grep '[Cc]hat|memory|conversation|paperclip|talk shortcut' -u
# Repeat the first command to compare against the reviewed references.
```

Use `npm run test:visual` for both update and comparison, so browser versions
and fonts match. Do not regenerate the entire screenshot directory for an
isolated change. When shared chrome legitimately affects many baselines,
review representative sizes/locales and inspect the complete changed-file list.
Add a regression assertion only for meaningful behavior not already covered
(draft/focus continuity, consent, reachable actions or actual overlap). Prefer
accessible names over positional button indexes, especially for confirmations.

For each slice, record the owning ID, affected views, fixture/deep link, source
contract, changes and verification. Use [DESIGN.md](../../DESIGN.md) for UI rules;
keep task priority and completion status here and in Plan Zeca.

Renderer changes need relevant lab interactions and deliberately reviewed visual
baselines. Native state/placement and real provider/mobile changes additionally
need their owning runtime checks. Run the project's required checks before a
commit; preserve English/i18n and translated public docs. Do not broaden a small
layout PR into a driver, model, theme, catalog port or orchestration framework.

Current reference commands (already available):

```sh
node design/compact-nest-review/build.mjs
node design/compact-nest-review/inspect.mjs --proposal
npx vite ui --host 127.0.0.1 --port 1430 --strictPort
```

The new lab's default entry and frontend-only command are UX0 deliverables;
the migration is not implemented yet. The command should reuse Vite directly;
`npm run dev` currently builds the Rust hook through `predev`.
