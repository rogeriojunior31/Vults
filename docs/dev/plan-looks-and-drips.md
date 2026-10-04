# Plan: looks aligned, and the drips

Working plan for the agents that fix how looks sit on every species and bring fourteen new looks
(the "drips": hats, shades, grills and chains) to Zeca. A shared board like `plan-fourth-review.md`
was: whoever picks a step marks it, refines it, and checks it off. Delete the file once every step
is merged, and record the result in `road-to-0.2.md` (the looks picker E17 stays a later step).

The gate is the usual one (CLAUDE.md, *Before every commit*): implementer → `flock-tester` (all
checks + the step's *Done when*) → `flock-reviewer` → at most two fix rounds → PR → CI green →
merge. The steps touch the same files (`zeca.py`, `looks.ts`): they run one after the other.

## Board

| # | Step | Size | Depends on | Status |
|---|---|---|---|---|
| L1 | Looks follow each species' eye; a pose never drops a look's column | S | | done (#85) |
| L2 | Pieces: hats, eyewear, grill; the five head-only drips | M | L1 | doing (feat/drips-head) |
| L3 | The neck slot with a swinging pendant; the nine drips with chains | M | L2 | todo |

## What exists today

- A look is a `LOOKS` entry in `design/mascots/zeca/zeca.py`: a profile grid (`side`, drawn on
  the resting head) and a front grid (`front`, on `head_front`), each with its top-left on that
  head. `looks()` derives where every pose wears it (`POSE_SHIFT`, the mirror for `head_back`,
  `FLY_SHIFT` for flight) and writes `looks` into `zeca.json`.
- `ui/src/character/looks.ts` `dress(set, id)` bakes a look into a copy of a species' heads and
  flight frames, growing every head by the same rows on top. `zeca.py`'s `dress()` is its twin for
  the review sheet (`looks.png`): change both together.
- The core picks what Zeca wears (`crates/core/src/looks.rs` `Outfit`: `Auto` follows the
  calendar, `None`, or one look for good); the setting `zeca_look` (`app/src/settings.rs`) and
  the select in `ui/src/settings.ts` (`value`/`label` pairs) carry it; `ui/src/view.gen.ts` is
  generated from the Rust types (a test checks it is fresh). Docs: `docs/reference/settings.md`
  (`zeca_look`), `docs/ANIMATIONS.md` (looks table), `docs/guide/island.md`.
- Visual tests: `tests/visual/looks.spec.ts` (each look in idle, approval, fly, and on the island);
  the lab takes `?look=<id>&species=<id>`.

## L1. Looks follow each species' eye; a pose never drops a look's column

### What

Two bugs in how a look sits on a head.

1. **The condor wears its glasses on the forehead.** A look's anchors are relative to the top of
   the resting head. The Andean condor (`vultur`) has a comb that adds a row on top of every head,
   so its eye is one row lower than Zeca's, and every look sits one row too high (sunglasses on
   the forehead; a hat floating). Fix in `looks.ts` `dress()`: for each pose, shift the look by the
   distance between this species' eye and Zeca's eye on the same part (the first `E` cell of
   `set.parts[pose]` vs `ZECA.parts[pose]`; for `fly` use `glide`). For flight, subtract the
   centring `dress()` already applies to a wider frame, so the shift is the eye's alone. No eye
   found (a part with no `E`) means no shift. Species that keep Zeca's heads get a shift of zero,
   so nothing else changes.
2. **A pose drops a look's left column.** `head_down` and `head_up` move the crown one cell
   left (`POSE_SHIFT` (-1, 1)), which puts a look's column 0 at x = -1, and `dress()` skips
   cells left of the grid. The witch hat's brim and the Santa hat's pompom lose a column while
   he reads or looks up. Fix in `zeca.py` `looks()`: clamp each pose's x at 0 (the look slides a
   cell forward when he leans, which reads as a hat slipping, and nothing is lost). Keep the
   `head_back` mirror math as it is (it uses the resting head's x).

### Also in the PR

- `zeca.py`'s `dress()` twin follows (it reads `look["on"]`, so the clamp is free; the species eye
  shift is not needed there, the sheet shows Zeca only).
- A test in `tests/visual/looks.spec.ts`: `/lab/?still=1&t=1500&look=sunglasses&species=vultur`
  idle card at 2x, and `look=witch-hat` on the `read` clip card (the brim whole).
- `docs/ANIMATIONS.md` looks section: one sentence that a look follows the species' eye.
- Visual baselines: only the new screenshots plus any look baseline that changes because of the
  clamp (`read`/`think` are not in the looks spec today; check with `npm run test:visual`).

### Done when

- On `vultur`, `sunglasses` cover the eye in idle, approval (front) and flight; `witch-hat` sits
  on the crown. On `atratus` and `aura` every look's screenshot is unchanged except where the
  clamp applies.
- A script (not committed) over all 23 species × 5 looks × the 7 head poses reports zero look
  cells dropped left of the grid, and the eye row under the shades on every species.
- All CLAUDE.md checks pass; `looks.ts` and `zeca.py` `dress()` agree (the sheet `looks.png`
  regenerated).

### Notes

- Studied on the lookbook before this plan: the eye-based shift was validated on all 23 species
  with 14 outfits; the clamp fixed nine outfits' brims, tails and locks in the reading pose.

## L2. Pieces: hats, eyewear, grill; the five head-only drips

### What

Looks become compositions of **pieces**: a hat, eyewear, a face mark, a grill, so a new look is one
line and the pieces can be mixed later (E17's picker). In `zeca.py`:

- `HATS`, `EYES` tables, each piece drawn like a look today (`side` on the resting head, `front`
  on `head_front`, with top-left offsets). `GRILL`: two gold cells on the lower beak, with a place
  per pose, because the beak moves more than the crown (in the hiss it reads as gold teeth in the
  open beak; in flight it sits on the small flight head).
- `OUTFITS`: a list of `dict(id, label, hat=, eyes=, grill=, neck=, notes=)`. `looks()` stamps
  an outfit's pieces per pose into one look grid (hat first, eyes over it, grill last), so the
  renderer reads them as looks: no change to `looks.ts` for the head.
- Height: no look rises more than 5 rows over the head (the witch hat's); `build()` asserts it.
- New palette letters are symbols, never letters (species recolor letters; digits are the
  seasonal looks'): see the palette below.

The five head-only drips ship in this step: `west-coast`, `fitted-cap`, `mountain-hat`, `headband`, `dreads`.
The other nine need the neck slot (L3) and stay out of `OUTFITS` until then.

Then the plumbing, one variant per drip: `Outfit` in `crates/core/src/looks.rs` (kebab-case ids,
listed in its round-trip test), `ui/src/view.gen.ts` regenerated, the select in
`ui/src/settings.ts` (labels below), `docs/reference/settings.md` `zeca_look` row, the looks
table in `docs/ANIMATIONS.md`, a line in `docs/guide/island.md`. Visual tests: the five ids in
`tests/visual/looks.spec.ts` `LOOKS`.

Ids and labels describe the style; no artist is named anywhere in the repo.

### The pieces (verbatim)

Palette (add to `PALETTE`, with the same short comments):

```
    "=": "#2456b8",
    "^": "#163a86",
    "@": "#16161c",
    "+": "#3a3a46",
    "~": "#bdb7aa",
    "%": "#b8892a",
    "*": "#fff1a8",
    "!": "#7a1520",
    "(": "#8a5a2b",
    ")": "#b07a40",
    "_": "#4a2e14",
    "&": "#d9a877",
    "|": "#d4d7de",
    "'": "#8c909a",
    "#": "#4a2f18",
    ":": "#7a5230",
```

Hats (`HATS`):

```
  "bandana-back": {"side": ((0, -2), [
      "...===....",
      "..=6==^...",
      ".==^=6==..",
      "=^=====...",
      "^=........",
      "=^........",
      ".=........",
    ]), "front": ((0, -2), [
      "..====..",
      ".=6==6=.",
      "==^==^==",
      "^======^",
    ])},
  "bandana-knot": {"side": ((0, -3), [
      "........4.",
      "...444.4..",
      ".4464644..",
      "44444!!...",
      "!!!!!.....",
    ]), "front": ((0, -3), [
      "..4..4..",
      "...44...",
      ".44!!44.",
      "46444464",
      "!!!!!!!!",
    ])},
  "durag": {"side": ((0, -1), [
      "...666....",
      ".66~666...",
      "6666666...",
      "6~........",
      "66........",
      "~6........",
      ".~........",
    ]), "front": ((0, -1), [
      "..6666..",
      ".66~666.",
      "66666666",
      "~......~",
    ])},
  "fitted": {"side": ((0, -3), [
      "..++++....",
      ".+@6666...",
      ".@@6666...",
      "0000000000",
    ]), "front": ((0, -3), [
      "..++++..",
      ".+6666+.",
      ".@6666@.",
      "00000000",
    ])},
  "headphones": {"side": ((1, -2), [
      "..|||...",
      ".|...|..",
      ".|...|..",
      "||'.....",
      "|'|.....",
      "||'.....",
    ]), "front": ((0, -2), [
      "..||||..",
      ".|....|.",
      "|'....'|",
      "||....||",
      "|'....'|",
    ])},
  "crown": {"side": ((2, -5), [
      "*.....",
      "7.*.*.",
      "7.7.7.",
      "77777.",
      "74747.",
      "%%%%%.",
    ]), "front": ((1, -4), [
      "*..*..*",
      "7..7..7",
      "7777777",
      "7477747",
      "%%%%%%%",
    ])},
  "bucket": {"side": ((0, -4), [
      "...444....",
      "..44444...",
      ".444444...",
      ".!!!!!!!..",
      "!!!!!!!!!.",
      "!.......!.",
    ]), "front": ((0, -4), [
      "..4444..",
      ".444444.",
      ".444444.",
      "!!!!!!!!",
      "!......!",
    ])},
  "mountain": {"side": ((0, -5), [
      "...)))....",
      "..)((()...",
      "..(((((...",
      "..(((((...",
      "..(___(...",
      "(((((((((.",
      "(.......(.",
    ]), "front": ((0, -5), [
      "..))))..",
      ".)(((().",
      ".((((((.",
      ".((((((.",
      ".(____(.",
      "((((((((",
      "(......(",
    ])},
  "headband": {"side": ((1, 0), [
      "..~~~.",
      "666666",
    ]), "front": ((0, 0), [
      "..~~~~..",
      "~666666~",
    ])},
  "dreads": {"side": ((0, -2), [
      "..#:##....",
      ".######...",
      "#:#####...",
      "###:......",
      "#:#.......",
      "###.......",
      ":#........",
      "##........",
    ]), "front": ((0, -2), [
      "..####..",
      ".#:##:#.",
      "########",
      "#:....:#",
      "#......#",
      ":......:",
      "#......#",
      "#......#",
    ])},
```

Eyewear and face marks (`EYES`):

```
  "locs": {"side": ((2, 2), [
      "000*0",
    ]), "front": ((1, 2), [
      "00*00*0",
    ])},
  "white-frames": {"side": ((2, 1), [
      "...666",
      "66600*",
      "...666",
    ]), "front": ((1, 1), [
      ".66.66.",
      "6006006",
    ])},
  "shutter": {"side": ((1, 1), [
      "...6666",
      "6660006",
      "...6666",
    ]), "front": ((1, 1), [
      "6666666",
      "0006000",
      "666.666",
    ])},
  "patch": {"side": ((1, 0), [
      "..%%%..",
      ".%...%.",
      "....@@.",
      "....@@.",
    ]), "front": ((0, 0), [
      "..%%%%..",
      ".%....%.",
      "%....@@%",
      ".....@@.",
    ])},
  "plaster": {"side": ((3, 3), [
      "&&&",
    ]), "front": ((4, 3), [
      "&&&",
    ])},
```

Grill cells per pose (`GRILL`; `(x, y)` on that head part, `fly` on the glide frame):

```
  "head": [(6, 4), (7, 4)],
  "head_hiss": [(6, 4), (7, 4)],
  "head_tilt": [(7, 4), (8, 4)],
  "head_down": [(5, 6), (6, 6)],
  "head_up": [(7, 2), (7, 3)],
  "fly": [(21, 6), (22, 6)],
  "head_front": [(3, 4), (4, 4)],
```

Outfits (`OUTFITS`; `neck=` entries are L3's, keep them commented out until then):

```
 dict(id="west-coast", label="West coast bandana", hat="bandana-back", eyes="locs",
      notes="A blue paisley bandana tied at the back, its tails down the nape, and wraparound shades."),
 dict(id="front-knot", label="Red bandana, front knot", hat="bandana-knot", neck="cross",
      notes="A red bandana knotted at the forehead with its ends up, and a gold cross on a thin chain."),
 dict(id="durag", label="Durag and grill", hat="durag", grill=True, neck="chain",
      notes="A white silk durag with its tail down the back, a gold grill, a chain with a medallion."),
 dict(id="fitted-cap", label="Fitted cap", hat="fitted", eyes="locs",
      notes="A black fitted cap, brim forward, a white wordmark across the front, and wraparound shades."),
 dict(id="crown", label="Crown and chain", hat="crown", neck="chain",
      notes="A gold crown with red stones, a little tilted, and a chain with a medallion."),
 dict(id="bucket-hat", label="Bucket hat and rope", hat="bucket", neck="rope",
      notes="A red bucket hat with a soft brim, and a thick gold rope chain."),
 dict(id="clock-chain", label="Clock chain", eyes="white-frames", neck="clock",
      notes="Big white-framed shades and a clock on a chain, swinging when he dances."),
 dict(id="headphones", label="Headphones", hat="headphones", neck="chain",
      notes="Big silver headphones, the cup over the ear, and a gold chain."),
 dict(id="mountain-hat", label="Mountain hat", hat="mountain",
      notes="The tall felt hat with a dented crown, a dark band and a wide brim."),
 dict(id="shutter-shades", label="Shutter shades", eyes="shutter", neck="chain",
      notes="White slatted shades and a medallion."),
 dict(id="headband", label="Headband", hat="headband", eyes="plaster",
      notes="A white headband and a plaster under one eye."),
 dict(id="dreads", label="Dreads and grill", hat="dreads", grill=True,
      notes="Locks down the nape and framing the face, and a gold grill."),
 dict(id="chrome-chain", label="Chrome chain", neck="chrome",
      notes="A thick chrome chain with a big square medallion."),
 dict(id="eye-patch", label="Eye patch and chains", eyes="patch", neck="stack",
      notes="An eye patch with a gold strap, and a stack of gold chains."),
```

How `looks()` composes an outfit, from the lookbook's generator (keep the behavior, write it in
`zeca.py`'s style):

```
def stamp(*pieces):
    """Pieces (x, y, rows) on one grid; returns its top-left and rows."""
    cells = {}
    for x, y, rows in pieces:
        for j, r in enumerate(rows):
            for i, c in enumerate(r):
                if c != ".": cells[(x + i, y + j)] = c
    x0, y0 = min(x for x, _ in cells), min(y for _, y in cells)
    w, h = max(x for x, _ in cells) - x0 + 1, max(y for _, y in cells) - y0 + 1
    g = [["."] * w for _ in range(h)]
    for (x, y), c in cells.items(): g[y - y0][x - x0] = c
    return x0, y0, ["".join(r) for r in g]

def outfit_look(o):
    pieces = [p for p in (HATS.get(o.get("hat")), EYES.get(o.get("eyes"))) if p]
    parts, on = {}, {}
    poses = {**POSE_SHIFT, "fly": FLY_SHIFT}
    for pose, (dx, dy) in poses.items():
        ps = [(p["side"][0][0] + dx, p["side"][0][1] + dy, p["side"][1]) for p in pieces]
        ps += [(x, y, ["7"]) for x, y in GRILL[pose]] if o.get("grill") else []
        x, y, g = stamp(*ps)
        parts[pose] = g; on[pose] = [pose, max(0, x), y]   # the L1 clamp
    hx, hy, hg = on["head"][1], on["head"][2], parts["head"]
    parts["back"] = mirror(hg); on["head_back"] = ["back", HEAD_W - hx - max(map(len, hg)), hy]
    ps = [(p["front"][0][0], p["front"][0][1], p["front"][1]) for p in pieces]
    ps += [(x, y, ["7"]) for x, y in GRILL["head_front"]] if o.get("grill") else []
    x, y, g = stamp(*ps); parts["front"] = g; on["head_front"] = ["front", x, y]
    return {"parts": parts, "on": on}
```

Note the difference from the seasonal looks: an outfit has one grid **per pose** (the grill
moves with the beak), where a seasonal look has one `side` grid placed per pose. `looks()`
emits both shapes into `zeca.json`; `looks.ts` already reads `on[pose] = [part, x, y]` with any
part name, so it needs no change for this.

### Done when

- The five drips show in the lab (`?look=<id>`) on every pose: idle, looking back, reading,
  thinking, the hiss, asleep, facing you, in flight; the shades cover the eye in all of them, the
  grill sits on the lower beak in each pose.
- On all 23 species, no piece floats (a script: every look cell touches the head or another look
  cell), no look rises over 5 rows, no cell is dropped.
- Settings offer them, with the labels above; `zeca_look` round-trips each id; docs updated;
  visual tests for the five looks added and green; all CLAUDE.md checks pass.

## L3. The neck slot with a swinging pendant; the nine drips with chains

### What

A second slot: something worn on the body, anchored on the agent's band, drawn **under the head**
so a lowered head covers it the way it would. In `zeca.py`:

- `NECK`: pieces with, per view (`side` for perched frames, `front` for the sunning pose), the
  strand's top-left relative to the band and its rows, and optionally a pendant with its own
  offset and rows. Nothing in flight.
- `looks()` writes an outfit's `neck` into its look as
  `"neck": {"side": {"at": [x, y], "strand": [...], "pat": [x, y] | null, "pendant": [...] | null}, "front": {...}}`.

In `ui/src/character/looks.ts` `dress()`: when the look has a `neck`, add the strand part (and the
pendant part) to `parts`, and in every frame that has a `band` layer insert, right after it, the
strand at `band + at` (the front view when the frame has `sunning`, else the side view) and the
pendant at `band + at + pat`, **lagging one cell behind when the body sways forward**
(`frame.dx > 0` → x - 1; the chest holds it on the way back). The `zeca.py` `dress()` twin does
the same for the sheet. Frames without a band (flight) get nothing.

The nine drips with chains join `OUTFITS`: `front-knot`, `durag`, `crown`, `bucket-hat`, `clock-chain`, `headphones`, `shutter-shades`, `chrome-chain`, `eye-patch`; their
plumbing (Outfit variants, settings labels, docs, visual tests) as in L2.

### The pieces (verbatim)

Neck pieces (`NECK`; per view: `(at, strand, pat, pendant)`):

```
  "chain": {
    "side": ((0, 1), ["7..", ".7.", ".7.", ".%."], (0, 4), ["777", "7*7", "%7%"]),
    "front": ((-2, 1), ["7....7", ".7..7.", "..77.."], (1, 3), ["7**7", "%77%"]),
  },
  "cross": {
    "side": ((0, 1), ["%..", ".%.", ".%."], (0, 3), [".7.", "7*7", ".7.", ".%."]),
    "front": ((-2, 1), ["%....%", ".%..%.", "..%%.."], (1, 3), [".*7.", "7777", ".77.", ".%7."]),
  },
  "rope": {
    "side": ((0, 1), ["7%.", "%7.", ".7%", ".%7", ".7%", "7%."], None, None),
    "front": ((-2, 1), ["7%..%7", "%7..7%", ".%7%7.", "..7%.."], None, None),
  },
  "clock": {
    "side": ((0, 1), ["7..", ".7.", ".7."], (0, 3), ["777", "767", "7@7", "767", "777"]),
    "front": ((-2, 1), ["%....%", ".%..%."], (0, 2), [".7777.", "766667", "76@667", "76@@67", "766667", ".7777."]),
  },
  "chrome": {
    "side": ((0, 1), ["|'.", ".|'", ".'|"], (0, 4), ["|||", "|'|", "|||"]),
    "front": ((-2, 1), ["|'..'|", ".|'.|.", "..||.."], (0, 4), ["||||", "|''|", "||||"]),
  },
  "stack": {
    "side": ((0, 1), ["7%.", "%7%", "7%7", ".7%"], (0, 4), ["777", "7*7", "%7%"]),
    "front": ((-2, 1), ["7%..%7", "%7%%7%", ".7%%7."], (0, 3), ["%7777%", ".7**7.", "..%%.."]),
  },
```

### Done when

- The nine drips show in the lab on every perched pose and the front pose; the strand sits on
  the neck, the pendant on the chest; in `dance` and `fail` the pendant swings (x differs between
  the forward sway and the rest); nothing is drawn in flight.
- On all 23 species, every neck cell lies on the body in every perched frame of every clip
  (a script over the built sets; the approval pose included), the strand never covers the band.
- `looks.ts` and `zeca.py` `dress()` agree (`looks.png` shows the chains); settings, docs,
  visual tests, and all CLAUDE.md checks pass.

### Notes

- From the lookbook: the pendants were narrowed so the approval pose keeps them on the body of
  the smallest species; the clock lost a column for the same reason.
