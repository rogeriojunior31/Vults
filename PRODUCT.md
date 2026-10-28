# Vults

<!-- impeccable:product-schema 1 -->

## Platform

web

Vults is a desktop application built with Tauri, Rust and a TypeScript/HTML/CSS
interface. The platform value describes its interface technology, not a hosted
website. Linux is the primary documented desktop target; platform and integration
capabilities must be checked against the current repository before making claims.

## Users

The primary audience is people who use AI agents, including developers, vibe
coders and other users of AI-assisted workflows. The audience is not restricted
to professional programmers. This scope was confirmed by the product owner.

Users need to follow agent activity, respond when attention is required, and use
voice, text and connected tools without repeatedly losing their working context.
No priority ranking between these audience segments has been established.

## Product Purpose

Vults brings agent activity, dictation, conversation and connected work into a
desktop companion. Users should understand what is happening, find the next
useful action, complete it and verify its result.

Dictation, voice conversation and meetings are useful workflows in their own
right. They should receive complete interaction design rather than becoming
secondary settings screens attached to agent monitoring.

## Positioning

Agent sessions are represented by animated vultures in a shared flock. Zeca is an
optional companion for conversation and context. The same desktop environment
provides agent attention and approvals, dictation into other applications, chat,
voice, meetings, GitHub connections and activity.

The flock and Zeca are established product identity. Their presence must help
users recognize the product and understand its state without obscuring the task.

## Operating Context

- Users work with agents, terminals, editors and other desktop applications.
  Some agent integrations can observe sessions without controlling or approving
  them; these capabilities are distinct.
- Vults has Mini, Quick, Room and full-screen presentations. Workflows must remain
  usable in compact and short windows as well as larger workspaces.
- Global dictation is used from a destination application. Copying, preparing text
  for the paste shortcut and supported insertion routes have different effects.
- Microphone input and audio output are separate devices. Voice interfaces must
  distinguish the selected route, system defaults, capture state and playback.
- The existing interface supports English, Brazilian Portuguese, Spanish and
  Simplified Chinese. User-facing wording must use the existing catalogs.

## Capabilities and Constraints

- Zeca is optional. Independent dictation and agent monitoring must not require
  users to enable him.
- Local transcription and optional provider processing are separate choices.
  Interfaces must identify when audio or text can leave the computer, which
  provider is involved and which spending controls apply.
- Recording, sending, validating a command and executing a command are different
  actions. Browsing examples or checking a phrase must not imply execution.
- Voice conversation never answers permission cards. Agent access, approvals and
  supported actions retain their existing ownership and explicit controls.
- Meeting capture, transcription, review and saving are separate stages. Browsing
  saved meetings retains the current review in memory. Explicit navigation away
  offers Save, Discard or Continue for unsaved text; it does not silently persist
  a draft. Forced hiding or locking still revokes capture and temporary work. Saving a meeting explicitly keeps a text-only snapshot.
- Style selection must explain its scope. Recognized sessions, application rules
  and built-in defaults can take precedence over the general style choice.
- Permanent dictation preferences belong together in the global Settings page.
  History, Commands, Conversation, Meetings, Dictionary, Snippets, Styles and
  Diagnostics remain available in the Dictation work area. Usage statistics belong
  in Activity rather than being duplicated in the work area.
- Current checkout capabilities are not automatically shipped or desktop-validated
  capabilities. Use the README, roadmap and acceptance evidence for release claims.

## Brand Commitments

The established names are Vults, Zeca and the flock (Bando in Portuguese).
Existing bird characters and their assets are part of the product's identity.

The product owner explicitly wants refinement that retains characteristic details
and improves understanding, configuration and validation. Moving features,
removing information or making a screen merely cleaner is not sufficient.

Visual implementation rules remain in DESIGN.md. This product record does not
replace its tokens, asset rules or interaction conventions.

## Evidence on Hand

- README.md and README.pt-br.md: product introduction, feature map and release
  qualifications.
- docs/guide/ and docs/pt-br/guide/: documented user workflows.
- docs/architecture.md and docs/adr/: capabilities, ownership and technical
  constraints.
- ui/src/character/ and docs/assets/: existing character renderers and captures.
  Historical captures may predate the current navigation and spacing.
- DESIGN.md: the incumbent interface rules.
- scripts/check-*.mjs: component behavior checks. Controlled render fixtures and
  passing checks do not establish that physical audio or native desktop behavior
  has been validated.

## Product Principles

1. Improve the complete task: preparing, acting, following progress, reviewing and
   recovering all belong to the same design responsibility.
2. Preserve recognizable product character while making the next action clear.
   Simplicity must improve understanding, not erase useful capability.
3. Show the actual effect and scope of a choice, including devices, style rules,
   processing destination, capture state and what is saved.
4. Keep daily tools close to their task and permanent preferences in the shared
   settings structure. Avoid duplicate editors and competing navigation systems.
5. Protect user work and explicit control across navigation, asynchronous updates
   and failures. Make any required discard or interruption understandable.

## Accessibility & Inclusion

Preserve the existing keyboard navigation, accessible control names, associated
input labels, visible focus, reduced-motion behavior and translated copy described
in DESIGN.md. Broad audience support must not assume that users know agent,
audio-system or desktop implementation terminology.
