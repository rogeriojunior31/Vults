# Breeds and flocks: a project's bird

Internal board (2026-10-09), checked against the code at `2ee8da1`. Deleted when its last step
merges. It replaces item 4 of the list taken from the reference app (a color per bird): the
reference tells sessions apart by color; we have 23 vulture species drawn in code, so a species
says it with meaning.

## 1. Where we stand

- `crates/core/src/flock.rs` draws each session's species by a hash of its **session id** and the
  **season** (a number picked at start-up), from the pool the user chose in **Settings → Flock**:
  Brazil (4 species, the default), the Americas (6) or the World (22). The king vulture (`papa`) goes
  by role: the oldest session of a project with 3 or more.
- So a species says nothing: two sessions of one project rarely match, two projects often do, and
  a project changes bird at every start (*A new flock every time the app starts*).
- The color (the glow, the dot) says the **agent**: Claude Code, Codex, Gemini CLI, OpenCode, Qwen
  Code, another tool.
- **Zeca's species is already the user's choice** (Settings → Flock → Zeca, every species, grouped by
  family). Nothing to do there.
- Per-project choices exist (ADR 0011): `ProjectPrefs { mute, pin, hide }` keyed by the project's
  folder, in `settings.json` → `projects`, set from a session's quick actions or **Settings →
  Projects**.

## 2. Decisions (with the user, 2026-10-09)

- **D1. The breed is the project.** Every session of a project is one species; the project's
  sessions are its flock. The color keeps saying the agent, so a Claude Code and a Codex in one flock
  still differ.
- **D2. Stable.** A project's species comes from its folder, not the season: the same project gets
  the same bird across starts, so it can be learned at a glance.
- **D3. Few species, many projects: repeat, avoiding clashes.** The projects on the wire take
  different species while the pool has some left; past that, two projects share one and the
  project's name tells them apart.
- **D4. The user decides the flocks.** A project's species can be chosen, from **every** species,
  whatever the pool. Unchosen, the pool draws it (D2, D3).
- **D5. The king stays a role**, the oldest session of a flock of 3 or more: the flock's leader. It
  is not offered as a project's species, so a crown always means the same thing.

## 3. Steps

One step, one PR.

| # | Step | Size | Needs | Done when |
|---|---|---|---|---|
| R1 | **Core: species by project.** `flock::species` draws from the pool by a hash of the project's folder (`cwd`), without the season. Clashes: projects on the wire, in order of arrival, take their drawn species or the next free one in the pool; when none is free, the drawn one. A session with no folder yet draws by its own id and the season, as today. King unchanged. Settings → Flock text (*A new flock every time the app starts* goes); island guide (EN + pt-BR); CHANGELOG | S | | Unit tests: same folder, same species across seasons; two projects on the wire differ while the pool allows; the fifth project in Brazil repeats; a folderless session still draws; the king still crowns the oldest of 3 |
| R2 | **Choose a project's bird.** `ProjectPrefs.species: Option<String>` (an unknown id is ignored and logged; `papa` refused), so `ProjectPrefs` loses `Copy`. Settings schema version bump. A chosen species wins over the draw, and may clash (the user chose). **Settings → Projects**: a species picker per project (the Flock page's preview grid, plus *Automatic*). A session's quick actions: *This project's bird…* opens the same picker on the island (as Zeca's looks picker does). Settings reference + island guide (EN + pt-BR), CHANGELOG | M | R1 | Settings round-trip tests (absent, chosen, unknown, `papa`); core test: chosen beats drawn; visual test of the picker; the bird changes at once on the island |
| R3 | Optional, only if asked: **perch together**. A flock's sessions side by side on the wire and in the list (after pinned projects, by first arrival of the project) | S | R1 | Ordering test; flock visual baselines updated on purpose |

## 4. Risks

- **Baselines.** The lab builds its view in TypeScript with species set by hand, so R1 should move
  no screenshot. If one moves, it is a real change and gets accepted on purpose.
- **A project that moves.** The key is the folder: a renamed or moved project draws again (and loses
  a chosen species), as its mute, pin and hide do today.
- **Clash order.** With a clash, which project gets the free species depends on who arrived first:
  stable within a run, may differ across runs. A chosen species (R2) is the fix for a user who cares.

## 5. Out of scope

- A color per session (the reference's way): the species does that job.
- Zeca's species (already in Settings → Flock).
- Naming flocks (nicknames per project).
