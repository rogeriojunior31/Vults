# Vults UI rules

Use these rules for UI changes and new prototypes. The production components and
the shared theme define the visual baseline. ADRs define behavior and authority;
this document defines how the UI presents them.

## Identity and theme

- Read `ui/src/themes.css` before choosing colors. Reuse its roles in every surface,
  including prototypes, dialogs, settings and embedded workspaces.
- Keep the black background, neutral chrome, subtle borders and light primary
  action. Green communicates a result, Zeca or an agent identity; it is not a new
  background or navigation theme.
- Keep species, accessories, palettes and sprite proportions from the existing
  character renderer. Scale pixel art with integer drawing scales and
  `image-rendering: pixelated`; do not stretch a bird to fill a card.
- Reuse the existing icon vocabulary and font stack. Do not introduce a second
  icon set, downloaded font, component framework or theme for a prototype.

| Role | Production token | Current value |
| --- | --- | --- |
| Ground | `--bg` | `#000000` |
| Chrome | `--chrome` | `#070809` |
| Panel | `--panel` | `#141518` |
| Floating content | `--float` | `#0e0f11` |
| Raised control | `--raised` | `#1c1e22` |
| Border / chrome line | `--border` / `--chrome-line` | `#1d1f23` / `#202127` |
| Main / secondary text | `--fg` / `--fg-dim` | `#f5f6f8` / `#9398a1` |
| Primary / selected mark | `--accent` / `--mark` | `#f5f6f8` / `#e2e6da` |
| Success / warning / error text | `--ok` / `--warn` / `--error-text` | `#22c55e` / `#f5a524` / `#ff8d97` |

The values above describe the current theme. The CSS tokens remain the source of
truth when that theme changes. Keep `--agent-*` colors as agent identity.

## Scale and spacing

- Use a small spacing scale: 4, 6, 8, 12, 16, 20 and 24 CSS pixels.
- Use 12-14 px body text, 11-12 px controls and supporting text, and 15-20 px local
  titles. Reserve smaller text for optional metadata; do not hide a decision,
  command, essential state or action behind tiny type.
- Use the production `--font` and `--mono` stacks. Code, paths and identifiers use
  the monospace stack and retain their complete value through wrapping or detail.
- Keep control corners near 6-7 px, cards near 9-10 px and the current surface frame.
  Borders express grouping; avoid drawing a card around every label.
- Desktop controls have at least 28-32 px of usable height; primary decision
  controls use about 38-40 px. Tiny signals receive a larger, separate hit area.
  Mobile controls follow platform touch-target requirements.
- Align related controls and use content width deliberately. Do not stretch a
  short metadata panel to the height of an unrelated decision.

## Surface roles and continuity

- Mini shows presence and attention and opens Quick. Keep one quiet horizontal
  composition; approvals require the full card in an authorized surface.
- Quick supports a decision, a conversation and short summaries. Use progressive
  detail to keep ordinary use within the initial view.
- Room and Full use the same Nest content. Additional space reveals useful lists,
  context and details, rather than enlarging every control.
- Preserve selected session, area, filters, source, conversation and unsent draft
  across layout changes. A resize does not create a conversation or execution.
- Honor the current registered capabilities and size rules in ADR 0021. The
  prototype's paired widths (560/680/760 px), Mini height (56 px) and Quick height
  (600 px) are layout references until native window, zoom, placement and input
  region validation lands. Production currently uses Mini 340 x 38 and Quick
  440 CSS px internally (492 in Chat), with a 1.14 UI zoom inside the fixed 720 x 640
  logical-pixel overlay; display scaling must be accounted for.
- Settings remains the configuration destination. Present the destination clearly
  when another surface needs a privileged setting.

## Priority and disclosure

1. A decision or failure needing attention.
2. Its project, agent, operation and scope.
3. The next useful action and its alternative.
4. Supporting context, history and metrics.

- Give each decision one primary action. Put maintenance and uncommon actions in
  a labeled disclosure, while keeping them available by keyboard.
- Keep the approval and its actions above the fold in short windows. Reduce the
  flock scene before reducing decision readability.
- Destructive requests emphasize Deny and expose no permanent approval. Show the
  command, folder and scope before the user responds.
- Explanations, suggestions and review verdicts do not answer permissions. Batch
  selection retains each request and leaves unselected requests pending.
- Show the selected session's own stage and status. Do not reuse another session's
  counters or describe a running session as already approved.
- Name the current item once per local region. Avoid repeated titles, identity
  rows, context buttons and identical suggested actions.

## Scroll and responsive layout

- Give the content region `min-height: 0` where it must shrink. Keep the header,
  navigation and Chat composer independent of long content.
- In the proposed Quick widths, initial Flock, Week, quick settings and a small
  agent directory should fit without scrolling. Empty Chat should also fit.
- Long transcripts, lists, editors, evidence and detailed settings scroll in their
  own bounded region. Their actions and a return path stay discoverable.
- Use a list/detail pair only when both remain useful. At narrow widths, open the
  detail in its own view or dialog and restore selection and focus on return.
- Avoid horizontal page scrolling. Truly wide tables and code can have a named,
  local scroll region; wrap long paths and preserve full values in detail.
- Validate both width and height. Nest must remain usable at 480 x 360; compact
  density does not justify a clipped composer or save bar.

## Chat and Zeca

- Use a short header for recipient, executor, conversation history and creation.
  Group source, context and memory controls together; avoid duplicating them above
  and below the transcript. Zeca shares the header in a small slot; do not reserve
  a portrait column or keep an empty-state title above a populated conversation.
- Keep an editable draft, attachments and Send visible. Quick provides at least
  one comfortable draft line that grows with its content. Keep its actions beside
  the draft and retain at least 40 px of transcript space in the minimum Nest.
- Suggested prompts fill a draft and leave sending to the user. Show them in the
  empty state; a growing transcript should not compete with a persistent row of
  the same prompts.
- Keep history, context and memory on the existing conversation. Looking at
  another conversation preserves each one's own draft and bound source.
- Show which profile, skills and source were pinned when a run started. Keep
  executor, specialist identity and tool connections distinguishable.
- Zeca's portrait may react to hover, click, work, attachment and voice states.
  Reactions communicate presence and never grant approval authority.
- With Zeca off, preserve the current independent flock, tasks and other surfaces.
  New specialist Chat behavior must follow its agreed capability contract.

## Agents, tasks and integrations

- A specialist row explains name, role and executor and offers Chat and Copy/Edit.
  Put full instructions, appearance, skills and scope in its detail or editor.
- Separate immutable presets and user profiles. Copying a preset creates a user
  profile; appearance and a role label do not grant permissions.
- A task puts goal, stage, status and criteria before IDs and lifecycle controls.
  Keep one execution identity across task, conversation, review and flock views.
- Retain the current issue, budget, workspace, continuation, handoff and review
  behavior when changing a task's presentation.
- Keep save/discard visible in long editors. Distinguish stopping owned work,
  deleting a task and removing its workspace.
- An integration draft, planned tool, verified capability and active connection
  have distinct states. Show Watch, Answer and Drive separately for CLI support.
- Imported skills retain source and revision. Import, profile activation and
  writing an agent configuration are separate, explicit steps.

## Motion and feedback

- Use the existing clips and director. Motion explains state and never moves a
  command, button or focus target away from the user.
- Prefer brief transitions around 120-200 ms. Preserve sprite phase on ordinary
  redraws; hold a finished pose instead of celebrating indefinitely.
- Respect reduced motion and manual pause. Stop unnecessary animation while a
  surface is hidden or inactive; reuse the existing clock and lifecycle controls.
- Show loading, empty, filtered-empty, failed, stale, disconnected and expired
  states distinctly, with a useful next action when available.
- Combine state text and a shape or icon with color. Keep usage warnings consistent:
  70-89% warns; 90% and above is critical, with an explicit label.
- Distinguish API estimates, subscription usage, observed cost and missing data.
  Missing measurements are unknown, not zero.

## Accessibility and wording

- Every control has an explicit accessible name. Associate labels with inputs;
  name icon actions by purpose and item.
- Tab lists expose selection and a panel relationship and support arrows,
  Home/End and one Tab entry. Keep existing application shortcuts working.
- Give dialogs a title, usable focus, Escape/close behavior and focus restoration.
  Present validation errors inside the active dialog, next to the relevant fields.
- Retain drafts, scroll position and selection when opening detail. Check that
  fixed or sticky actions do not cover the last row or editor fields.
- Write short, complete English UI sentences and translate them through the
  existing catalogs. Explain effects and scope before advanced implementation
  details. Pair technical identifiers with readable names.

## Prototype fidelity and validation

- Reuse actual renderers, theme roles and existing interaction owners. A prototype
  may add illustrative states without presenting them as shipped capabilities.
- Mark simulated execution and planned features explicitly. Readiness requires
  its real verification; a reference catalog is not a working engine inventory.
- Use controlled data to check production renderers and verify changes in the
  running application. Cover
  empty and populated content, pending and destructive requests, long values,
  errors, disabled Zeca, reduced motion and layout continuity.
- Review the default and narrow/short layouts, keyboard access, overflow and
  visible primary actions. The repository has no browser Lab or screenshot suite;
  keep small regression checks on the production components where useful.
- Record the renderer checks performed and the native checks still required.
