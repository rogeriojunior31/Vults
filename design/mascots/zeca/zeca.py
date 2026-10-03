#!/usr/bin/env python3
"""Zeca, the black vulture (Coragyps atratus): the source of his sprites.

Parts are palette-indexed pixel grids; clips are frames that stack parts at integer offsets,
so a head pose or a blink is drawn once and reused. Each clip follows a real behavior
(docs/ANIMATIONS.md). Run it after any change:

    design/mascots/zeca/zeca.py      # writes ui/src/character/zeca/zeca.json and clips.png
"""
import json, pathlib, sys

ROOT = pathlib.Path(__file__).resolve().parents[3]

PALETTE = {
    "K": "#0e0d11", "B": "#1d1c22", "b": "#2c2a33", "s": "#4b4858",
    "H": "#4f4f56", "h": "#77777f", "w": "#34343a",
    "E": "#0a0705", "e": "#e8e2d6", "N": "#17171b",
    "P": "#5f5c58", "p": "#d6ceb9", "R": "#2a1416",
    "L": "#c2c2c6", "l": "#8f8f95",
    "W": "#e2e3e8", "v": "#a6a7ae",
    "-": "#3a3a40",
    "A": "#d97757",
    # Emotes: the island's state colors, so a mark over his head reads like the badge it echoes.
    "Y": "#f5a524", "C": "#22d3ee", "G": "#4ade80", "D": "#7cc4ff", "r": "#f4505e",
}

PARTS = {
  # Folded body, perched, facing right. Neck socket at (14, 4) where heads attach.
  "body": [
    "......KBBBK.............",
    ".....KBbbbBK............",
    "....KBbsbbbBK...........",
    "....BbsbbbbbBK..........",
    "...KBbsbbbbbbB..........",
    "...BbsbbbbbbbbK.........",
    "...BbsbbbbbbbbBB........",
    "..KBbsbbbbbbbbbBB.......",
    "..BBbsbbbbbbbbbBB.......",
    "..BBbsbbsbbbbbbBB.......",
    "..BBbbsbbsbbbbBBB.......",
    "..KBBbbsbbsbbbBBK.......",
    "...BBbbbsbbsbBBB........",
    "...KBBbbbsbbsBBK........",
    "....KBBbbbsbbBK.........",
    ".....KBBBbbsBK..........",
    "....KBBKBBBBK...........",
    "...KBBK.................",
    "..KBK...................",
  ],
  # Feathers fluffed: cold, asleep, or threatened.
  "body_puff": [
    ".....KBBBBK.............",
    "....KBbbbbBK............",
    "...KBbsbbbbBK...........",
    "...BbsbbbbbbBK..........",
    "..KBbsbbbbbbbB..........",
    "..BbsbbbbbbbbbK.........",
    "..BbsbbbbbbbbbBB........",
    ".KBbsbbbbbbbbbbBB.......",
    ".BBbsbbbbbbbbbbBBK......",
    ".BBbsbbsbbbbbbbBBK......",
    ".BBbbsbbsbbbbbBBBK......",
    ".KBBbbsbbsbbbbBBK.......",
    "..BBbbbsbbsbbBBB........",
    "..KBBbbbsbbsbBBK........",
    "...KBBbbbsbbBBK.........",
    "....KBBBbbsBBK..........",
    "....KBBKBBBBK...........",
    "...KBBK.................",
    "..KBK...................",
  ],
  # The agent's band at the base of the neck, in the accent color.
  "band": [
    ".A",
    "AA",
  ],
  # A wing raised over the back in a stretch, primaries splayed, pale underside showing.
  "wing_up": [
    "W.W.W.W.......",
    "WvWvWvWv......",
    "vWvWvWvWK.....",
    ".vWvWvbbBK....",
    "..vvWbbbbBK...",
    "..KvbbsbbbBK..",
    "...KbbbsbbbBK.",
    "....KBbbsbbBK.",
    ".....KBbbsbBK.",
    "......KBbbbBK.",
    ".......KKBBK..",
  ],


  "legs": [
    "L..L..",
    "Ll.Ll.",
    "LlL.LlL",
  ],
  "legs_step": [
    "L...L.",
    "Ll..Ll",
    "LlL.LlL",
  ],
  # Heads: 10x7 boxes; the neck joins the body at the box's bottom-left.
  "head": [
    "...hhh....",
    ".hHHHHh...",
    "hHHwHEeP..",
    "HHHHHHNPPp",
    "wHHHHHPPpp",
    ".wHHw...p.",
  ],
  "head_down": [
    "..........",
    "..hhh.....",
    ".hHHHh....",
    "hHHHEeh...",
    "HHwHHHHP..",
    ".wHHHHNPP.",
    "..wHHPPPp.",
    "....PPpp..",
    ".....pp...",
  ],
  "head_up": [
    "........p.",
    "..hhh..Pp.",
    ".hHHHhPP..",
    "hHHwEeNP..",
    "HHHHHHH...",
    "wHHHHHw...",
    ".wHHw.....",
  ],
  "head_tilt": [
    "....hh....",
    "..hhHHh...",
    ".hHHwHEe..",
    "hHHHHHHNP.",
    "wHHHHHPPPp",
    ".wHHHw..pp",
    "..ww......",
  ],
  "head_hiss": [
    "...hhh....",
    ".hHHHHh...",
    "hHHwHEeP..",
    "HHHHHHNPPp",
    "wHHHHHRR.p",
    ".wHHHPPp..",
    "..wHw.....",
  ],
  "head_front": [
    "..hhhh..",
    ".hHHHHh.",
    ".HEeHEe.",
    ".HHPPHH.",
    "..HPPH..",
    "...pp...",
    "..wHHw..",
  ],
  # Front-facing body for sunning (wings spread), 32 wide; head socket at (12, 0).
  "sunning": [
    "...KBBBBK.....wHHw.....KBBBBK...",
    "..KBbbbbbBBK.BBBBBB.KBBbbbbbBK..",
    ".KBbsbsbsbbbBBbbbbBBbbbsbsbsbBK.",
    "KWBbsbsbsbbbbBbbbbBbbbbsbsbsbBWK",
    "KWvBbsbsbbbbbBbbbbBbbbbbsbsbBvWK",
    "KWWvBbbbbbbbbBbbbbBbbbbbbbbBvWWK",
    ".KWvWvBbbbbbBBbbbbBBbbbbbBvWvWK.",
    "..KWvWvBBBBK.BbbbbB.KBBBBvWvWK..",
    "...KvWKvK....KBbbBK....KvKWvK...",
    "....KK.KK.....KBBK.....KK.KK....",
  ],
  "legs_front": [
    "L..L",
    "Ll.lL",
  ],
}

# Flight, seen from below and slightly behind, heading right: how a vulture looks in the sky.
# Broad wings with the pale underside of the primaries and splayed "fingers"; a short square tail.
def _sym(left, center):
    return [l + c + l[::-1] for l, c in zip(left, center)]

_CENTER = list("....KBbbbBBBK.")
_GLIDE = [
    "..................", "..................", "K.K...............", "WKWK.K............",
    "vWvWKBK.......KKKK", ".vWvWvBKKKKKKKBBBB", "..vWvvBbbbbbbbbbbb", "...KvvBbsbsbsbbbbb",
    "....KKBBbbbbbbbbbB", "......KKKKBBBBBBBB", ".........KKKKKKBBB", "..............KKBB",
    "...............KKK", "..................",
]
_UP = [
    "......K.K.........", ".....WKWK.........", "....vWvWKK........", ".....vWvBBK.......",
    "......vvBbBK..KKKK", ".......KBbbBKKBBBB", "........KBbbbbbbbb", ".........KBbsbbbbb",
    "..........KBBbbbbB", "...........KKBBBBB", "..............KBBB", "..............KKBB",
    "...............KKK", "..................",
]
_DOWN = [
    "..................", "..................", "..................", "..................",
    "..............KKKK", "........KKKKKKBBBB", ".....KKBbbbbbbbbbb", "...KKBbsbsbsbbbbbb",
    "..KvvBbbbbbbbbbbbB", ".KvWvBBBBKKBBBBBBB", "KvWvWKKK...KKKKBBB", "vWvWK.........KKBB",
    "WKWK...........KKK", "K.K...............",
]
_FLY_HEAD = ["..hhh...", ".hHHHh..", ".HHHEePp", ".wHHHPPp", "..wHw..."]

def _with_head(rows, head=_FLY_HEAD, x=16, y=3):
    grid = [list(r) for r in ["." * 37] * 3 + rows]
    for j, r in enumerate(head):
        for i, c in enumerate(r):
            if c != ".":
                grid[y + j][x + i] = c
    return ["".join(r) for r in grid]

PARTS.update({
    "fly_up": _with_head(_sym(_UP, _CENTER)),
    "glide": _with_head(_sym(_GLIDE, _CENTER)),
    "fly_down": _with_head(_sym(_DOWN, _CENTER)),
})

def blink(rows):
    return [r.replace("E", "w").replace("e", "w") for r in rows]

for name in [n for n in PARTS if n.startswith("head")]:
    PARTS[name + ":blink"] = blink(PARTS[name])

def mirror(rows):
    w = max(len(r) for r in rows)
    return [r.ljust(w, ".")[::-1] for r in rows]

PARTS["head_back"] = mirror(PARTS["head"])
PARTS["head_back:blink"] = blink(PARTS["head_back"])

# ── Clips ──────────────────────────────────────────────────────────────────────
# Perched frames are relative to the body's top-left; the head socket is (14, 1), the feet (9, 17).
HEAD = (14, 1)
FEET = (9, 17)

BAND = (13, 5)

def perch(head="head", hx=0, hy=0, body="body", legs="legs", dx=0, dy=0, under=(), extra=()):
    layers = [*under, [body, 0, 0], [legs, FEET[0], FEET[1]], ["band", BAND[0], BAND[1]],
              [head, HEAD[0] + hx, HEAD[1] + hy], *extra]
    return {"dx": dx, "dy": dy, "layers": layers}

def f(ms, frame):
    return {**frame, "ms": ms}

CLIPS = {
  # Watching: long holds, a blink, a look back over the shoulder.
  "idle": {"loop": True, "frames": [
    f(1800, perch()), f(110, perch("head:blink")), f(1400, perch()),
    f(700, perch("head_back", hx=-5, hy=-1)), f(110, perch("head_back:blink", hx=-5, hy=-1)), f(500, perch("head_back", hx=-5, hy=-1)),
    f(1200, perch()),
    f(160, perch("head_down", hx=-4, hy=3)), f(120, perch("head_down", hx=-6, hy=5)),
    f(110, perch("head_down", hx=-5, hy=4)), f(120, perch("head_down", hx=-6, hy=5)),
    f(110, perch("head_down", hx=-5, hy=4)), f(140, perch("head_down", hx=-6, hy=6)),
    f(180, perch("head_down", hx=-3, hy=2)), f(600, perch("head", hy=1)),
  ]},
  # Thinking: head drawn up and still, slow blinks, a small settle.
  "think": {"loop": True, "frames": [
    f(1400, perch("head_up", hy=-1)), f(160, perch("head_up:blink", hy=-1)), f(900, perch("head_up", hy=-1)),
    f(700, perch("head_up", hy=0)), f(160, perch("head_up:blink", hy=0)), f(1100, perch("head_up", hy=-1)),
  ]},
  # Reading: head down, scanning along the line and dropping to the next one.
  "read": {"loop": True, "frames": [
    f(320, perch("head_down", hx=1, hy=1)), f(320, perch("head_down", hx=2, hy=1)), f(320, perch("head_down", hx=3, hy=1)),
    f(420, perch("head_down", hx=1, hy=2)), f(320, perch("head_down", hx=2, hy=2)), f(120, perch("head_down:blink", hx=2, hy=2)),
    f(320, perch("head_down", hx=3, hy=2)),
  ]},
  # Searching: neck out, quick turns, a tilt to look closer.
  "search": {"loop": True, "frames": [
    f(500, perch("head", hx=1, hy=-1)), f(380, perch("head_back", hx=-5, hy=-1)), f(500, perch("head_tilt", hx=1)),
    f(300, perch("head", hx=2, hy=-1)), f(110, perch("head:blink", hx=2, hy=-1)), f(420, perch("head_back", hx=-5, hy=-1)),
  ]},
  # Editing: the feeding motion — lean in, strike, tear back, swallow.
  "edit": {"loop": True, "frames": [
    f(220, perch()), f(80, perch("head_down", hx=1, hy=3, dy=1)), f(90, perch("head_down", hx=1, hy=5, dy=1)),
    f(130, perch("head_down", hx=0, hy=2)), f(160, perch("head", hx=-1, hy=0)), f(240, perch()),
  ]},
  # Running a command: quick tugs at the wire, feet shuffling for grip.
  "run": {"loop": True, "frames": [
    f(110, perch("head_down", hx=1, hy=4, dy=1)), f(110, perch("head_down", hx=0, hy=2, legs="legs_step")),
    f(110, perch("head_down", hx=1, hy=4, dy=1)), f(160, perch("head_down", hx=-1, hy=1)),
    f(110, perch("head_down", hx=1, hy=4, dy=1, legs="legs_step")), f(260, perch("head", hx=0)),
  ]},
  # Needs a human: the sunning pose, facing you, head bobbing.
  "approval": {"loop": True, "frames": [
    f(700, {"dx": 0, "dy": 0, "layers": [["sunning", -4, 4], ["band", 11, 4], ["head_front", 8, -2], ["legs_front", 10, 14]]}),
    f(500, {"dx": 0, "dy": 0, "layers": [["sunning", -4, 4], ["band", 11, 4], ["head_front", 8, -1], ["legs_front", 10, 14]]}),
    f(120, {"dx": 0, "dy": 0, "layers": [["sunning", -4, 4], ["band", 11, 4], ["head_front:blink", 8, -1], ["legs_front", 10, 14]]}),
    f(500, {"dx": 0, "dy": 0, "layers": [["sunning", -4, 4], ["band", 11, 4], ["head_front", 8, -2], ["legs_front", 10, 14]]}),
  ]},
  # A question: the curious head tilt, held.
  "question": {"loop": True, "frames": [
    f(900, perch("head_tilt", hx=1, hy=-1)), f(120, perch("head_tilt:blink", hx=1, hy=-1)),
    f(700, perch("head_tilt", hx=1, hy=-1)), f(500, perch("head", hx=0, hy=0)),
  ]},
  # Done: a hop, a stretch with one wing raised over the back, settle.
  "done": {"loop": False, "frames": [
    f(160, perch(dy=1)), f(120, perch(dy=-2)), f(120, perch(dy=-1)), f(140, perch()),
    f(140, perch("head", hy=-1, extra=[["wing_up", 1, -7]])),
    f(520, perch("head_up", hy=-1, extra=[["wing_up", 0, -9]])),
    f(160, perch("head", extra=[["wing_up", 1, -6]])),
    f(900, perch()),
  ]},
  # Failed: feathers up, hiss, a shake.
  "fail": {"loop": True, "frames": [
    f(140, perch("head_hiss", body="body_puff", hx=-1, dx=-1)), f(140, perch("head_hiss", body="body_puff", hx=-1, dx=1)),
    f(140, perch("head_hiss", body="body_puff", hx=-1, dx=-1)), f(700, perch("head", body="body_puff", hx=-1)),
  ]},
  # Listening to the user (the chat's mic is open): head cocked toward them, small nods as they
  # speak, a blink now and then.
  "listen": {"loop": True, "frames": [
    f(700, perch("head_tilt", hx=1, hy=-1)), f(180, perch("head_tilt", hx=1, hy=0)),
    f(600, perch("head_tilt", hx=1, hy=-1)), f(120, perch("head_tilt:blink", hx=1, hy=-1)),
    f(500, perch("head_tilt", hx=2, hy=-1)), f(180, perch("head_tilt", hx=2, hy=0)),
    f(700, perch("head_tilt", hx=1, hy=-1)),
  ]},
  # Music playing: a bob on every beat (112 BPM, about 536 ms), swaying side to side, head
  # tilting with it and a foot tapping.
  "dance": {"loop": True, "frames": [
    f(268, perch("head", hy=1, dy=1)), f(268, perch("head_up", hy=-1, dy=-1, legs="legs_step")),
    f(268, perch("head_tilt", hx=1, dy=1, dx=1)), f(268, perch("head_up", hy=-1, dy=-1, dx=1)),
    f(268, perch("head", hy=1, dy=1)), f(268, perch("head_up", hy=-1, dy=-1, legs="legs_step")),
    f(268, perch("head_back", hx=-5, dy=1, dx=-1)), f(268, perch("head_back:blink", hx=-5, hy=-1, dy=-1, dx=-1)),
  ]},
  # Asleep: fluffed, head sunk into the shoulders, eyes shut.
  "sleep": {"loop": True, "frames": [
    f(1600, perch("head_down:blink", body="body_puff", hx=-3, hy=3)), f(1600, perch("head_down:blink", body="body_puff", hx=-3, hy=4, dy=1)),
  ]},
  # A file dropped on him: down to the wire, pick it up, toss the head back and gulp it.
  "swallow": {"loop": False, "frames": [
    f(140, perch("head_down", hx=1, hy=3, dy=1)), f(160, perch("head_down", hx=1, hy=5, dy=1)),
    f(120, perch("head_down", hx=0, hy=2)), f(200, perch("head_up", hy=-2, dy=-1)),
    f(140, perch("head_up:blink", hy=-3, dy=-1)), f(140, perch("head_up", hy=-2)),
    f(140, perch("head_up:blink", hy=-1)), f(500, perch("head")),
  ]},
  # Preen: a bout of grooming, head into the wing and quick nibbles, once.
  "preen": {"loop": False, "frames": [
    f(160, perch("head_down", hx=-4, hy=3)), f(120, perch("head_down", hx=-6, hy=5)),
    f(110, perch("head_down", hx=-5, hy=4)), f(120, perch("head_down", hx=-6, hy=5)),
    f(110, perch("head_down", hx=-5, hy=4)), f(140, perch("head_down", hx=-6, hy=6)),
    f(110, perch("head_down", hx=-5, hy=4)), f(140, perch("head_down", hx=-6, hy=5)),
    f(180, perch("head_down", hx=-3, hy=2)), f(400, perch("head", hy=1)), f(300, perch()),
  ]},
  # Startled (a click): a jump, wings flung up, head high, then down again.
  "startle": {"loop": False, "frames": [
    f(80, perch(dy=1)),
    f(110, perch("head_up", hy=-1, dy=-3, extra=[["wing_up", 1, -8]])),
    f(160, perch("head_up", hy=-1, dy=-2, extra=[["wing_up", 0, -10]])),
    f(140, perch("head", dy=-1, extra=[["wing_up", 1, -6]])),
    f(120, perch(dy=1)), f(500, perch()),
  ]},
  # Hello: he turns his head to you and waves a wing.
  "hello": {"loop": False, "frames": [
    f(320, perch("head_front", hx=-2, hy=-1)),
    f(200, perch("head_front", hx=-2, hy=-1, extra=[["wing_up", 1, -7]])),
    f(200, perch("head_front", hx=-2, hy=-2, extra=[["wing_up", 0, -9]])),
    f(200, perch("head_front", hx=-2, hy=-1, extra=[["wing_up", 1, -7]])),
    f(200, perch("head_front", hx=-2, hy=-2, extra=[["wing_up", 0, -9]])),
    f(260, perch("head_front", hx=-2, hy=-1)), f(120, perch("head_front:blink", hx=-2, hy=-1)),
    f(500, perch("head_front", hx=-2, hy=-1)), f(400, perch()),
  ]},
  # Gape: something is being dragged over him. Head up, bill open, waiting for it to drop in.
  "gape": {"loop": True, "frames": [
    f(500, perch("head_hiss", hx=1, hy=-2, dy=-1)), f(400, perch("head_hiss", hx=1, hy=-1)),
    f(120, perch("head_hiss:blink", hx=1, hy=-1)), f(500, perch("head_hiss", hx=1, hy=-2, dy=-1)),
  ]},
  # Flight: three quick stiff flaps (up, level, down, level), then a short flat glide.
  "fly": {"loop": True, "frames": [
    *[fr for _ in range(3) for fr in (
      f(70, {"dx": 0, "dy": 0, "layers": [["fly_up", 0, 0]]}),
      f(55, {"dx": 0, "dy": 0, "layers": [["glide", 0, 0]]}),
      f(80, {"dx": 0, "dy": 1, "layers": [["fly_down", 0, 0]]}),
      f(55, {"dx": 0, "dy": 0, "layers": [["glide", 0, 0]]}),
    )],
    f(1100, {"dx": 0, "dy": 0, "layers": [["glide", 0, 0]]}),
  ]},
}

# ── Emotes: a mark over the head that says the state at a glance ─────────────────
# Drawn above the head layer of whatever pose he is in; each loops on its own.
PARTS.update({
  # A thought bubble, its tail down to the head; the dots come one by one.
  "bubble": [
    ".WWWWWWW.",
    "WWWWWWWWW",
    "WWWWWWWWW",
    "WWWWWWWWW",
    ".WWWWWWW.",
    "..WW.....",
    "...W.....",
  ],
  "dot": ["K"],
  "bang": [
    "YY",
    "YY",
    "YY",
    "YY",
    "..",
    "YY",
  ],
  "ask": [
    ".CCC.",
    "C...C",
    "...C.",
    "..C..",
    "..C..",
    ".....",
    "..C..",
  ],
  # A twinkle: a diamond with a bright core, and a smaller one going out.
  "spark": [
    "..G..",
    ".GWG.",
    "GWWWG",
    ".GWG.",
    "..G..",
  ],
  "spark_small": [
    ".G.",
    "GWG",
    ".G.",
  ],
  "spark_out": [
    "G.G",
    "...",
    "G.G",
  ],
  "curse": [
    ".r.r.",
    "rrrrr",
    ".r.r.",
    "rrrrr",
    ".r.r.",
  ],
  "drop": [
    ".D.",
    ".D.",
    "DDD",
    ".D.",
  ],
  "zee": [
    "WWWW",
    "..W.",
    ".W..",
    "WWWW",
  ],
  # An eighth note.
  "note": [
    "..WW",
    "..WW",
    "..W.",
    "..W.",
    "WWW.",
    "WWW.",
  ],
})

def mark(ms, *layers, dx=0, dy=0):
    return {"ms": ms, "dx": dx, "dy": dy, "layers": [list(l) for l in layers]}

BUBBLE = ("bubble", 0, 0)
EMOTES = {
  # Thinking: the dots fill in, then hold.
  "think": {"loop": True, "frames": [
    mark(260, BUBBLE), mark(260, BUBBLE, ("dot", 2, 2)), mark(260, BUBBLE, ("dot", 2, 2), ("dot", 4, 2)),
    mark(700, BUBBLE, ("dot", 2, 2), ("dot", 4, 2), ("dot", 6, 2)),
  ]},
  # A permission: the bang hops.
  "alert": {"loop": True, "frames": [mark(420, ("bang", 0, 0)), mark(160, ("bang", 0, 0), dy=-1), mark(420, ("bang", 0, 0))]},
  # A question: the mark sways.
  "ask": {"loop": True, "frames": [mark(500, ("ask", 0, 0)), mark(500, ("ask", 0, 0), dx=1)]},
  # Done: two sparkles trade places.
  "done": {"loop": True, "frames": [
    mark(360, ("spark", 0, 2), ("spark_small", 6, 0)), mark(140, ("spark_small", 1, 3), ("spark", 5, 0)),
    mark(360, ("spark_out", 1, 3), ("spark", 5, 0)), mark(140, ("spark", 0, 2), ("spark_out", 6, 0)),
  ]},
  # Failed: a curse mark, shaking.
  "fail": {"loop": True, "frames": [mark(140, ("curse", 0, 0)), mark(140, ("curse", 0, 0), dx=1), mark(500, ("curse", 0, 0))]},
  # A usage limit: a drop of sweat runs down.
  "sweat": {"loop": True, "frames": [mark(220, ("drop", 0, 0)), mark(220, ("drop", 0, 1)), mark(220, ("drop", 0, 2)), mark(500, ("drop", 0, 2))]},
  # Music: a note drifts up and sways.
  "music": {"loop": True, "frames": [
    mark(268, ("note", 0, 4)), mark(268, ("note", 1, 3)), mark(268, ("note", 2, 2)), mark(268, ("note", 1, 1)),
  ]},
  # Asleep: a z drifts up, then another.
  "sleep": {"loop": True, "frames": [
    mark(500, ("zee", 0, 5)), mark(500, ("zee", 1, 3)), mark(500, ("zee", 2, 1), ("zee", 0, 5)), mark(500, ("zee", 1, 3)),
  ]},
}

def build(out_json):
    json.dump({"palette": PALETTE, "parts": PARTS, "clips": CLIPS, "emotes": EMOTES}, open(out_json, "w"), separators=(",", ":"))

# ── Review sheet: every clip as a strip of frames ───────────────────────────────
def sheet(out_png, scale=6, cw=40, ch=30, ox=6, oy=6):
    import struct, zlib
    rgb = {k: tuple(int(v[i:i+2], 16) for i in (1, 3, 5)) for k, v in PALETTE.items()}
    maxf = max(len(c["frames"]) for c in CLIPS.values())
    W, H = maxf * cw, len(CLIPS) * ch
    img = [[(0, 0, 0)] * W for _ in range(H)]
    for ci, (name, clip) in enumerate(CLIPS.items()):
        for fi, fr in enumerate(clip["frames"]):
            bx, by = fi * cw + ox + fr["dx"], ci * ch + oy + fr["dy"]
            for x in range(fi * cw, fi * cw + cw - 1):
                img[ci * ch + oy + 20][x] = rgb["-"] if name not in ("fly",) else (0, 0, 0)
            for part, px, py in fr["layers"]:
                for y, row in enumerate(PARTS[part]):
                    for x, c in enumerate(row):
                        if c in rgb and c != ".":
                            X, Y = bx + px + x, by + py + y
                            if 0 <= X < W and 0 <= Y < H:
                                img[Y][X] = rgb[c]
    big = []
    for row in img:
        line = bytes(c for px in row for c in px * 1)
        line = b"".join(bytes(px) * scale for px in row)
        big.extend([b"\0" + line] * scale)
    chunk = lambda k, d: struct.pack(">I", len(d)) + k + d + struct.pack(">I", zlib.crc32(k + d))
    open(out_png, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W * scale, H * scale, 8, 2, 0, 0, 0))
                              + chunk(b"IDAT", zlib.compress(b"".join(big), 6)) + chunk(b"IEND", b""))
    print(out_png, list(CLIPS))

if __name__ == "__main__":
    build(ROOT / "ui/src/character/zeca/zeca.json")
    sheet(ROOT / "design/mascots/zeca/clips.png")
