# Plan: Zeca polished (0.1.1)

Working plan for the agents that ship the polish of Zeca's sprite and the 0.1.1 release that
carries it. Same rig, same silhouette, same anchors: every clip, look and species keeps working.
It is a shared board, like `plan-fourth-review.md`: whoever picks a step marks it, refines it,
and checks it off. Delete the file once the release is out, and note the release in
`road-to-0.2.md`.

The gate is the usual one (CLAUDE.md, *Before every commit*): implementer → `flock-tester` (all
checks + the step's *Done when*) → `flock-reviewer` → at most two fix rounds → PR → CI green →
merge.

## Board

| # | Step | Size | Depends on | Status |
|---|---|---|---|---|
| Z1 | The rig polished: light and a wing on the body, the head, two new tones, lifted blacks, life in the idle; species inherit the tones | S | | doing (feat/zeca-polish) |
| R1 | Release 0.1.1: versions, boards, tag, draft release, docs | S | Z1 | todo |

## Z1. The rig polished

### What

Three changes, all in `design/mascots/zeca/zeca.py` plus one rule in
`ui/src/character/flock/index.ts`.

**1. Body and wing.** Two new palette letters: `i`, a lit edge (lighter than `s`), and `d`, a
deep shadow (between `B` and `K`). The back gets a lit edge along its top (light from above), the
folded wing gets a trailing edge running down toward the tail with the shadow under it, the three
feather streaks sit inside the wing panel, the belly is plain. The head gets a lit crown, a
wrinkle on the nape and a darker cheek under the eye (letter `w`, already there). `body_puff`,
`head_front` and `sunning` get the same treatment. **Every grid keeps its silhouette cell for
cell** (a `.` stays a `.`, a cell stays a cell): the species transforms (`rig.ts`) address cells
by coordinate.

`i` and `d` stay above row 11 of the bodies: the king and Egyptian vultures recolor Zeca's grid
from row 11 (`species.ts` `recolor(..., 11)`), the palm-nut from row 14, and the gyps recolor
their tall bodies from row 16; a lit cell in a recolored area would be a stripe. The tall bodies
(`tallGrid` in `rig.ts`) carry the same lit edge and wing fold, derived by rule.

The grids:

```
  "body": [
    "......KBBBK.............",
    ".....KiisbBK............",
    "....KisbsbbBK...........",
    "....isbsbbbbBK..........",
    "...KisbsbbbbdB..........",
    "...isbsbbbbbBdK.........",
    "...isbsbbbbBdbBB........",
    "..KibsbbbbBdbbbBB.......",
    "..BibbsbbBdbbbbBB.......",
    "..BBbbsbBdbbbbbBB.......",
    "..BBbbsbBsbbbbBBB.......",
    "..KBBbbsBbsbbbBBK.......",
    "...BBbbbBbbsbBBB........",
    "...KBBbsBsbbsBBK........",
    "....KBBsBbsbbBK.........",
    ".....KBBBsbsBK..........",
    "....KBBKBBBBK...........",
    "...KBBK.................",
    "..KBK...................",
  ],
  "body_puff": [
    ".....KBBBBK.............",
    "....KiisbbBK............",
    "...KisbsbbbBK...........",
    "...isbsbbbbbBK..........",
    "..KisbsbbbbbdB..........",
    "..isbsbbbbbbBdK.........",
    "..isbsbbbbbbBdBB........",
    ".KibsbbbbbbBdbbBB.......",
    ".BibbsbbbbBdbbbBBK......",
    ".BBbbsbbbBdbbbbBBK......",
    ".BBbbbsbBdbbbbBBBK......",
    ".KBBbbbsBsbbbbBBK.......",
    "..BBbbbbBbsbbBBB........",
    "..KBBbbsBsbsbBBK........",
    "...KBBbsBsbbBBK.........",
    "....KBBBbbsBBK..........",
    "....KBBKBBBBK...........",
    "...KBBK.................",
    "..KBK...................",
  ],
  "head": [
    "...hhh....",
    ".hhHHHh...",
    "hHHwHEeP..",
    "HHwHHHNPPp",
    "wHHHHwPPpp",
    ".wHHw...p.",
  ],
  "head_front": [
    "..hhhh..",
    ".hhHHhh.",
    ".HEeHEe.",
    ".HHPPHH.",
    "..wPPw..",
    "...pp...",
    "..wHHw..",
  ],
  "sunning": [
    "...KBBBBK.....wHHw.....KBBBBK...",
    "..KBiisbbBBK.BBBBBB.KBBbbsiiBK..",
    ".KBibsbsbbbbBBbbbbBBbbbbsbsbiBK.",
    "KWBbsbsbsbbbdBbbbbBdbbbsbsbsbBWK",
    "KWvBbsbsbbbbdBbbbbBdbbbbsbsbBvWK",
    "KWWvBbbbbbbbbBbbbbBbbbbbbbbBvWWK",
    ".KWvWvBbbbbbBBbbbbBBbbbbbBvWvWK.",
    "..KWvWvBBBBK.BbbbbB.KBBBBvWvWK..",
    "...KvWKvK....KBbbBK....KvKWvK...",
    "....KK.KK.....KBBK.....KK.KK....",
  ],
```

The other heads (`head_down`, `head_up`, `head_tilt`, `head_hiss`) and the flight frames keep
their grids: they differ from the resting head only in crown and beak placement, and a second
pass can bring the wrinkle and the cheek to them.

**2. Lifted blacks.** Zeca's darkest colors move one step up: he stays a black vulture, he stops
being the island's own black. In `PALETTE`:

```
"K": "#17161c", "B": "#27262e", "b": "#383641", "s": "#58555f", "w": "#3f3f46",
"i": "#6c6a78", "d": "#1f1e25",
```

`i` and `d` are new entries. The species override `K`, `B`, `b` and `s` with their own colors,
so only Zeca and the species that keep his colors change.

**3. Species inherit the tones.** In `ui/src/character/flock/index.ts`, right after
`Object.assign(set.palette, s.palette)`: a species that sets its own `b` and does not set `i` or
`d` gets `i` = its `b` moved 30% toward white and `d` = its `b` at 58% (toward black). One small
helper, with a comment saying why (the lit edge and the wing shadow come from the body's color).
No species defines `i` or `d` today; `g` is taken (the ruffs), which is why the lit edge is `i`.

**4. Life in the idle.** Two small things in the `idle` clip, clip edits only:

- a breath: body, band and head sink one cell for 320 ms, the feet stay. `perch()` gets a
  `sink=0` argument that adds it to the y of the body, band and head layers (not the legs).
  Two breaths per loop: split the first hold (`f(1800, perch())`) into `f(1480, perch())`,
  `f(320, perch(sink=1))`, and the `f(1400, perch())` after the blink into `f(1080, perch())`,
  `f(320, perch(sink=1))`;
- a shuffle of the feet: `f(130, perch(legs="legs_step"))` right before the look back over the
  shoulder (the `f(700, perch("head_back", ...))` frame).

Keep the loop's feel: long holds, nothing busy.

### Also in the PR

- Run `design/mascots/zeca/zeca.py`: it rewrites `ui/src/character/zeca/zeca.json`,
  `design/mascots/zeca/clips.png` and `looks.png`. Commit all three.
- A guard in `zeca.py` `build()`: every letter used in every part is in `PALETTE` (a missing
  letter is drawn as a hole). Fail loudly.
- `docs/ANIMATIONS.md`: the `idle` row says "Watching, blinking, a breath now and then, a shuffle
  of the feet, a look back over the shoulder".
- Visual baselines: `npm run test:visual -- -u` (every clip, look and species screenshot changes
  on purpose). `flock.spec.ts`'s visitors test is flaky on this machine (times out about half
  the time); rerun it alone before calling it a failure.
- No `Co-Authored-By` trailer in commits (a hook rejects it); keep the `Claude-Session` line.

### Done when

- `scripts/check-brand.sh`, `scripts/check-english.sh`, `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `npm run build`, `npm run test:visual` all pass.
- For each of the five grids, the silhouette equals `main`'s cell for cell (a script in the
  tester's report, not committed).
- In the lab (`/lab/?species=<id>`), every species draws with no holes: for all 23, every letter
  in the built parts has a color in the built palette (the tester checks it with a small
  Playwright or node script, not committed).
- At 3× the lit edge along the back, the wing's trailing edge and the lit crown are visible on
  the black vulture, the turkey vulture and a griffon (its tall body carries the tones through
  `tallGrid`); the griffons' flight feathers (rows 16+ of the tall body) are unchanged.
- The idle loop breathes twice and shuffles once per loop; the legs never move with the breath.

### Notes

- Studied on the lab page and species sheets before this step: the two new tones only read when
  derived from each species' own body color; with Zeca's grey they were wrong on brown birds.
- The lit edge is `i`, not `g`: `g` is the ruff color of three species.
- Round 1: tall bodies (`tallGrid`) carried none of the tones; the palm-nut's rect leaked `i`;
  `d` scaled by luminance for white birds; breaths moved off t=1500.

## R1. Release 0.1.1

After Z1 is merged. One PR, then the tag.

- Versions: `0.1.1` in `Cargo.toml` (`[workspace.package]`), `package.json`,
  `app/tauri.conf.json`; `scripts/check-version.sh v0.1.1` passes. The AUR `pkgver()` reads
  `Cargo.toml` (check `packaging/aur/PKGBUILD` has no hard-coded version left).
- Boards: this file's board marks Z1 `done (#PR)` and R1 `doing`; `road-to-0.2.md` *Releases*
  gets a line "0.1.1: Zeca polished (plan-zeca-polish.md)". Once the release is out, this file is
  deleted and that line is the record.
- Merge, then `git tag -a v0.1.1 -m "Vultures AI 0.1.1"` on `main` and push the tag: the
  release workflow checks the version, builds the packages, drafts the release and publishes
  the docs.
- Release notes (the draft): "Zeca polished: a lit back and folded wing with two new tones, his
  blacks one step up, a breath and a shuffle in his idle; every species inherits the light."
- Smoke: nothing but sprites and one palette rule changed, so the visual tests are the smoke
  check; the R3 checklist (island, card, chat, voice, GitHub) is the user's call for this one.

### Done when

- The draft release `v0.1.1` exists with the Linux packages, the AppImage and the Windows
  installer attached, and the docs site shows 0.1.1.
