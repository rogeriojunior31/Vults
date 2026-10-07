#!/usr/bin/env python3
"""Zeca, the black vulture (Coragyps atratus): the source of his sprites.

Parts are palette-indexed pixel grids; clips are frames that stack parts at integer offsets,
so a head pose or a blink is drawn once and reused. Each clip follows a real behavior
(docs/ANIMATIONS.md). Run it after any change:

    design/mascots/zeca/zeca.py      # writes ui/src/character/zeca/zeca.json, clips.png, looks.png
                                     # and the tray icon's frames (app/icons/tray/)
"""
import json, pathlib, sys

ROOT = pathlib.Path(__file__).resolve().parents[3]

PALETTE = {
    # One step above the island's own black, so he stays a black vulture and not a hole in it.
    "K": "#17161c", "B": "#27262e", "b": "#383641", "s": "#58555f",
    "H": "#4f4f56", "h": "#77777f", "w": "#3f3f46",
    # The lit edge (light from above) and the deep shadow under the folded wing: the species derive
    # their own from their body color (ui/src/character/flock/index.ts).
    "i": "#6c6a78", "d": "#1f1e25",
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
  # The lit edge (i) and the wing's shadow (d) stay above row 11: the king, Egyptian and palm-nut
  # vultures recolor the rows from 11 (14 for the palm-nut) cell by cell (flock/species.ts).
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
  # Feathers fluffed: cold, asleep, or threatened.
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
    ".hhHHHh...",
    "hHHwHEeP..",
    "HHwHHHNPPp",
    "wHHHHwPPpp",
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
    ".hhHHhh.",
    ".HEeHEe.",
    ".HHPPHH.",
    "..wPPw..",
    "...pp...",
    "..wHHw..",
  ],
  # Front-facing body for sunning (wings spread), 32 wide; head socket at (12, 0).
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

def perch(head="head", hx=0, hy=0, body="body", legs="legs", dx=0, dy=0, under=(), extra=(), sink=0):
    # A breath: the body, band and head sink `sink` rows while the feet keep the wire.
    layers = [*under, [body, 0, sink], [legs, FEET[0], FEET[1]], ["band", BAND[0], BAND[1] + sink],
              [head, HEAD[0] + hx, HEAD[1] + hy + sink], *extra]
    return {"dx": dx, "dy": dy, "layers": layers}

def f(ms, frame):
    return {**frame, "ms": ms}

def crouch(sink, head="head", hx=0, hy=0, body="body_puff"):
    """A landing squash: the body sinks `sink` rows, wider (fluffed), onto bent legs that stay on
    the wire, drawn under it. Existing parts only, so every species rig and look applies."""
    return {"dx": 0, "dy": sink, "layers": [["legs", FEET[0], FEET[1] - sink], [body, 0, 0],
                                            ["band", BAND[0], BAND[1]], [head, HEAD[0] + hx, HEAD[1] + hy]]}

CLIPS = {
  # Watching: long holds, a blink, a breath now and then, a shuffle of the feet, a look back over
  # the shoulder.
  "idle": {"loop": True, "frames": [
    f(1800, perch()), f(110, perch("head:blink")), f(1080, perch()), f(320, perch(sink=1)),
    f(130, perch(legs="legs_step")),
    f(700, perch("head_back", hx=-5, hy=-1)), f(110, perch("head_back:blink", hx=-5, hy=-1)), f(500, perch("head_back", hx=-5, hy=-1)),
    f(880, perch()), f(320, perch(sink=1)),
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
  # Hello: he lands with a squash, bounces up stretched tall, lands again softer, then turns his
  # head to you and waves a wing.
  "hello": {"loop": False, "frames": [
    f(90, crouch(2, hy=1)),
    f(110, perch("head_up", hy=-2, dy=-2)), f(90, perch("head_up", hy=-1, dy=-3)),
    f(80, perch("head", dy=-1)),
    f(90, crouch(1, hy=1)), f(120, perch()),
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

# ── Looks: what Zeca wears for the season (crates/core/src/looks.rs picks the day) ─────────
# The looks with no season are composed of pieces, further down.
# Digits are the looks' colors: no species recolors them.
PALETTE.update({
    "1": "#3b2752", "2": "#664a8c", "3": "#f08c2e",   # witch hat: purple, its shine, the band
    "4": "#d0353c", "5": "#93222c", "6": "#f2efe8",   # Santa hat: red, its shade; fur and pompoms
    "7": "#f5c542", "8": "#e0409a",                   # party hat: gold and magenta stripes
    "9": "#f2a0b8",                                   # inside a bunny's ear
    "0": "#101016",                                   # dark lenses
})

# Each look is drawn twice: in profile ("side", on the resting head) and facing you ("front", on
# head_front), each with where its top-left sits on that head (y < 0 is above it). The other
# poses follow from where their crown and eye moved; head_back wears the profile mirrored, and in
# flight the profile sits on the small flight head. The renderer (ui/src/character/looks.ts)
# grows every head by the same rows on top, so a pose swap mid-clip keeps the hat on.
LOOKS = {
  # Halloween: a witch's hat, its tip bent back.
  "witch-hat": {"side": ((0, -5), [
    "1........",
    ".11......",
    "..121....",
    "..1221...",
    "..3333...",
    "11111111.",
  ]), "front": ((0, -5), [
    ".....1..",
    "....11..",
    "...121..",
    "..1221..",
    "..3333..",
    "11111111",
  ])},
  # Christmas: a Santa hat, the pompom hanging back.
  "santa-hat": {"side": ((0, -4), [
    "66.......",
    "665544...",
    "..544444.",
    ".6666666.",
  ]), "front": ((0, -4), [
    "......66",
    "...45566",
    "..44444.",
    "66666666",
  ])},
  # New Year: a striped party cone.
  "party-hat": {"side": ((2, -5), [
    "..6..",
    "..7..",
    ".787.",
    ".878.",
    "78787",
  ]), "front": ((1, -5), [
    "..6..",
    "..7..",
    ".787.",
    ".878.",
    "78787",
  ])},
  # Easter: a bunny's ears.
  "bunny-ears": {"side": ((1, -4), [
    "66.66.",
    "96.96.",
    "96.96.",
    ".66.66",
  ]), "front": ((0, -4), [
    ".66..66.",
    ".69..96.",
    ".69..96.",
    "..6..6..",
  ])},
  # Summer, on request: dark glasses in a gold frame.
  "sunglasses": {"side": ((1, 2), [
    "777060",
    "...00.",
  ]), "front": ((1, 2), [
    "0607060",
    ".00.00.",
  ])},
}

# ── Pieces: a hat, eyewear, a grill, a chain, composed into a look ───────────────────────
# Symbols are the pieces' colors: the species recolor letters, and the digits are the seasonal looks'.
PALETTE.update({
    "=": "#2456b8", "^": "#163a86",                   # a blue bandana and its paisley
    "@": "#16161c", "+": "#3a3a46",                   # black cloth and its lit edge
    "~": "#bdb7aa",                                   # white cloth in the shade
    "%": "#b8892a", "*": "#fff1a8",                   # gold in the shade, and its glint
    "!": "#7a1520",                                   # red cloth in the shade
    "(": "#8a5a2b", ")": "#b07a40", "_": "#4a2e14",   # felt: brown, its lit crown, the band
    "&": "#d9a877",                                   # a plaster
    "|": "#d4d7de", "'": "#8c909a",                   # chrome and its shade
    "#": "#4a2f18", ":": "#7a5230",                   # locks, and the strands the light catches
})

# A piece is drawn like a look: in profile on the resting head and facing you on head_front, each
# with where its top-left sits on that head.
HATS = {
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
}

# Eyewear and face marks.
EYES = {
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
}

# Two gold cells on the lower beak, placed per pose: the beak moves more than the crown (in the
# hiss they read as gold teeth in the open beak; in flight they sit on the small flight head).
GRILL = {
  "head": [(6, 4), (7, 4)],
  "head_hiss": [(6, 4), (7, 4)],
  "head_tilt": [(7, 4), (8, 4)],
  "head_down": [(5, 6), (6, 6)],
  "head_up": [(7, 2), (7, 3)],
  "fly": [(21, 6), (22, 6)],
  "head_front": [(3, 4), (4, 4)],
}

# Worn on the neck, hanging from the agent's band: per view (`side` on the perched body, `front` on
# the sunning pose), the strand's top-left from the band and its rows, then the pendant's top-left
# from the strand and its rows (None: a chain with no pendant). Nothing in flight.
NECK = {
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
  # The clock hangs a link higher than the medallion: its bottom row would leave the narrowest
  # chests (the Egyptian and palm-nut vultures) at rest.
  "clock": {
    "side": ((0, 1), ["7..", ".7."], (0, 2), ["777", "767", "7@7", "767", "777"]),
    "front": ((-2, 1), ["%....%", ".%..%."], (0, 2), [".7777.", "766667", "76@667", "76@@67", "766667", ".7777."]),
  },
  "chrome": {
    "side": ((0, 1), ["|'.", ".|'", ".'|"], (0, 3), ["|||", "|'|", "|||"]),
    "front": ((-2, 1), ["|'..'|", ".|'.|.", "..||.."], (0, 3), ["||||", "|''|", "||||"]),
  },
  "stack": {
    "side": ((0, 1), ["7%.", "%7%", "7%7", ".7%"], (0, 4), ["777", "7*7", "%7%"]),
    "front": ((-2, 1), ["7%..%7", "%7%%7%", ".7%%7."], (0, 3), ["%7777%", ".7**7.", "..%%.."]),
  },
}

# An outfit is one line: its pieces, and the label the settings show. The ids are the setting's
# values (crates/core/src/looks.rs), so they never change.
OUTFITS = [
 dict(id="west-coast", label="West coast bandana", hat="bandana-back", eyes="locs",
      notes="A blue paisley bandana tied at the back, its tails down the nape, and wraparound shades."),
 dict(id="fitted-cap", label="Fitted cap", hat="fitted", eyes="locs",
      notes="A black fitted cap, brim forward, a white wordmark across the front, and wraparound shades."),
 dict(id="mountain-hat", label="Mountain hat", hat="mountain",
      notes="The tall felt hat with a dented crown, a dark band and a wide brim."),
 dict(id="headband", label="Headband", hat="headband", eyes="plaster",
      notes="A white headband and a plaster under one eye."),
 dict(id="dreads", label="Dreads and grill", hat="dreads", grill=True,
      notes="Locks down the nape and framing the face, and a gold grill."),
 dict(id="front-knot", label="Red bandana, front knot", hat="bandana-knot", neck="cross",
      notes="A red bandana knotted at the forehead with its ends up, and a gold cross on a thin chain."),
 dict(id="durag", label="Durag and grill", hat="durag", grill=True, neck="chain",
      notes="A white silk durag with its tail down the back, a gold grill, a chain with a medallion."),
 dict(id="crown", label="Crown and chain", hat="crown", neck="chain",
      notes="A gold crown with red stones, a little tilted, and a chain with a medallion."),
 dict(id="bucket-hat", label="Bucket hat and rope", hat="bucket", neck="rope",
      notes="A red bucket hat with a soft brim, and a thick gold rope chain."),
 dict(id="clock-chain", label="Clock chain", eyes="white-frames", neck="clock",
      notes="Big white-framed shades and a clock on a chain, swinging when he dances."),
 dict(id="headphones", label="Headphones", hat="headphones", neck="chain",
      notes="Big silver headphones, the cup over the ear, and a gold chain."),
 dict(id="shutter-shades", label="Shutter shades", eyes="shutter", neck="chain",
      notes="White slatted shades and a medallion."),
 dict(id="chrome-chain", label="Chrome chain", neck="chrome",
      notes="A thick chrome chain with a big square medallion."),
 dict(id="eye-patch", label="Eye patch and chains", eyes="patch", neck="stack",
      notes="An eye patch with a gold strap, and a stack of gold chains."),
]

# Where each profile pose's crown and eye sit, from the resting head's.
POSE_SHIFT = {"head": (0, 0), "head_hiss": (0, 0), "head_down": (-1, 1), "head_up": (-1, 1), "head_tilt": (1, 0)}
# The flight head (_with_head) puts the resting head's crown 15 cells right and 3 down.
FLY_SHIFT = (15, 3)
HEAD_W = len(PARTS["head"][0])
# The tallest look (the witch hat's rows over the head): every head grows by this much at most.
RISE = 5

def stamp(*pieces):
    """Pieces (x, y, rows) on one grid, the later ones over the earlier: its top-left and rows."""
    cells = {(x + i, y + j): c for x, y, rows in pieces for j, r in enumerate(rows) for i, c in enumerate(r) if c != "."}
    x0, y0 = min(x for x, _ in cells), min(y for _, y in cells)
    w, h = max(x for x, _ in cells) - x0 + 1, max(y for _, y in cells) - y0 + 1
    grid = [["."] * w for _ in range(h)]
    for (x, y), c in cells.items():
        grid[y - y0][x - x0] = c
    return x0, y0, ["".join(r) for r in grid]

def outfit(o):
    """An outfit as a look: one grid per pose (the grill moves with the beak, not with the crown),
    the hat first, the eyewear over it, the grill last. The neck piece goes out as drawn: the
    renderer hangs it from the band of each frame."""
    pieces = [p for p in (HATS.get(o.get("hat")), EYES.get(o.get("eyes"))) if p]
    grill = lambda pose: [(x, y, ["7"]) for x, y in GRILL[pose]] if o.get("grill") else []
    parts, on = {}, {}
    # Only a chain: nothing on any head, so no pose wears a grid (stamp() needs cells).
    if pieces or o.get("grill"):
        for pose, (dx, dy) in {**POSE_SHIFT, "fly": FLY_SHIFT}.items():
            # The renderer clamps a look's x at 0 as one grid, which would carry the grill off the beak
            # on a leaning head: clamp each piece instead. The grill is placed on each pose's beak.
            placed = [(max(0, x + dx), y + dy, rows) for (x, y), rows in (p["side"] for p in pieces)]
            x, y, parts[pose] = stamp(*placed, *grill(pose))
            on[pose] = [pose, x, y]
        _, hx, hy = on["head"]
        parts["back"] = mirror(parts["head"])
        on["head_back"] = ["back", HEAD_W - hx - len(parts["head"][0]), hy]
        placed = [(x, y, rows) for (x, y), rows in (p["front"] for p in pieces)]
        x, y, parts["front"] = stamp(*placed, *grill("head_front"))
        on["head_front"] = ["front", x, y]
    look = {"parts": parts, "on": on}
    if o.get("neck"):
        look["neck"] = {view: {"at": at, "strand": strand, "pat": pat, "pendant": pendant}
                        for view, (at, strand, pat, pendant) in NECK[o["neck"]].items()}
    return look

def looks():
    """The looks as the renderer reads them: parts, and where each pose wears one. A seasonal look
    is one profile grid placed per pose; an outfit's pieces are stamped into a grid per pose."""
    out = {}
    for name, look in LOOKS.items():
        (sx, sy), side = look["side"]
        (fx, fy), front = look["front"]
        # The renderer drops cells left of the grid: a leaning head slides the hat a cell forward instead.
        on = {pose: ["side", max(0, sx + dx), sy + dy] for pose, (dx, dy) in POSE_SHIFT.items()}
        on["head_front"] = ["front", fx, fy]
        on["head_back"] = ["back", HEAD_W - sx - max(map(len, side)), sy]
        on["fly"] = ["side", sx + FLY_SHIFT[0], sy + FLY_SHIFT[1]]
        out[name] = {"parts": {"side": side, "front": front, "back": mirror(side)}, "on": on}
    for o in OUTFITS:
        out[o["id"]] = outfit(o)
    return out

def dress(parts, look, clips=CLIPS):
    """Zeca's parts and clips wearing `look`, as ui/src/character/looks.ts dresses them; the review
    sheet's copy. Every head grows on top and its layers move up as much; what hangs from the neck
    is a layer under the band (the agent's mark stays whole) in every frame that has one. The
    species' eye shift is left out on purpose: the sheet is Zeca's, where it is zero."""
    out, lift = dict(parts), {}
    heads = [n for n in parts if n.startswith("head")]
    fly = [n for n in parts if n in ("fly_up", "glide", "fly_down")]
    pad = lambda poses: max([0] + [-at[2] for pose, at in look["on"].items() if pose in poses])
    for names, poses in ((heads, set(look["on"]) - {"fly"}), (fly, {"fly"})):
        grow = pad(poses)
        for n in names:
            pose = "fly" if n in fly else max((p for p in poses if n.startswith(p)), key=len, default=None)
            w = max(len(r) for r in parts[n])
            grid = [list("." * w) for _ in range(grow)] + [list(r.ljust(w, ".")) for r in parts[n]]
            if pose and pose in look["on"]:
                part, x, y = look["on"][pose]
                x += (w - 37) // 2 if pose == "fly" else 0
                for j, row in enumerate(look["parts"][part]):
                    for i, c in enumerate(row):
                        # A pose whose crown moved left may push a brim's edge off the grid.
                        if c == "." or x + i < 0 or not 0 <= y + grow + j < len(grid):
                            continue
                        line = grid[y + grow + j]
                        line.extend("." * (x + i + 1 - len(line)))
                        line[x + i] = c
            out[n], lift[n] = ["".join(r) for r in grid], grow
    neck = look.get("neck", {})
    for view, n in neck.items():
        out["strand_" + view] = n["strand"]
        if n["pendant"]:
            out["pendant_" + view] = n["pendant"]
    def frame(fr):
        layers = [[p, x, y - lift.get(p, 0)] for p, x, y in fr["layers"]]
        band = next((i for i, (p, _, _) in enumerate(layers) if p == "band"), None)
        if neck and band is not None:
            view = "front" if any(p == "sunning" for p, _, _ in layers) else "side"
            n, (_, bx, by) = neck[view], layers[band]
            x, y = bx + n["at"][0], by + n["at"][1]
            worn = [["strand_" + view, x, y]]
            if n["pendant"]:
                # The pendant lags a cell behind a forward sway; the chest holds it on the way back.
                worn.append(["pendant_" + view, x + n["pat"][0] - (1 if fr["dx"] > 0 else 0), y + n["pat"][1]])
            layers[band:band] = worn
        return {**fr, "layers": layers}
    return out, {name: {**clip, "frames": [frame(fr) for fr in clip["frames"]]} for name, clip in clips.items()}

def build(out_json):
    # A letter outside the palette is drawn as a hole, silently: fail here instead. Every piece is
    # checked, worn by an outfit today or not.
    pieces = [(f"{kind}:{n}:{view}", p[view][1]) for kind, table in (("hat", HATS), ("eyes", EYES)) for n, p in table.items() for view in ("side", "front")]
    pieces += [(f"neck:{n}:{view}", rows) for n, p in NECK.items() for view, (_, strand, _, pendant) in p.items() for rows in (strand, pendant) if rows]
    for name, grid in [*PARTS.items(), *pieces, *((f"{n}:{p}", g) for n, l in looks().items() for p, g in l["parts"].items())]:
        for c in {c for row in grid for c in row} - {"."} - set(PALETTE):
            raise SystemExit(f"part {name!r} uses {c!r}, which is not in PALETTE")
    # The heads grow by the tallest look's rows: a taller one would push the mark over his head off the card.
    for name, look in looks().items():
        rise = max((-y for _, _, y in look["on"].values()), default=0)
        if rise > RISE:
            raise SystemExit(f"look {name!r} rises {rise} rows over the head; {RISE} is the most")
    json.dump({"palette": PALETTE, "parts": PARTS, "clips": CLIPS, "emotes": EMOTES, "looks": looks()},
              open(out_json, "w"), separators=(",", ":"))

def looks_sheet(out_png):
    """Every look on a few clips that show each pose: resting, looking back, reading, the
    approval's front head, and in flight."""
    rows = {}
    for name, look in looks().items():
        parts, clips = dress(PARTS, look)
        for clip in ("idle", "read", "question", "approval", "fly"):
            rows[f"{name}:{clip}"] = (parts, clips[clip]["frames"][:8])
    _strip(out_png, rows)

# ── Review sheet: every clip as a strip of frames ───────────────────────────────
def sheet(out_png):
    _strip(out_png, {name: (PARTS, clip["frames"]) for name, clip in CLIPS.items()})

def _strip(out_png, rows, scale=6, cw=40, ch=30, ox=6, oy=6):
    """One row per entry, its frames side by side, on a wire (flight has none)."""
    import struct, zlib
    rgb = {k: tuple(int(v[i:i+2], 16) for i in (1, 3, 5)) for k, v in PALETTE.items()}
    maxf = max(len(frames) for _, frames in rows.values())
    W, H = maxf * cw, len(rows) * ch
    img = [[(0, 0, 0)] * W for _ in range(H)]
    for ci, (name, (parts, frames)) in enumerate(rows.items()):
        for fi, fr in enumerate(frames):
            bx, by = fi * cw + ox + fr["dx"], ci * ch + oy + fr["dy"]
            if not name.endswith("fly"):
                for x in range(fi * cw, fi * cw + cw - 1):
                    img[ci * ch + oy + 20][x] = rgb["-"]
            for part, px, py in fr["layers"]:
                for y, row in enumerate(parts[part]):
                    for x, c in enumerate(row):
                        if c in rgb and c != ".":
                            X, Y = bx + px + x, by + py + y
                            if 0 <= X < W and 0 <= Y < H:
                                img[Y][X] = rgb[c]
    big = []
    for row in img:
        line = b"".join(bytes(px) * scale for px in row)
        big.extend([b"\0" + line] * scale)
    chunk = lambda k, d: struct.pack(">I", len(d)) + k + d + struct.pack(">I", zlib.crc32(k + d))
    open(out_png, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W * scale, H * scale, 8, 2, 0, 0, 0))
                              + chunk(b"IDAT", zlib.compress(b"".join(big), 6)) + chunk(b"IEND", b""))
    print(out_png, list(rows))

# ── Tray icon: a few frames per state, for the panel (app/src/tray.rs picks them by core's
# Attention). Three sizes, each drawn pixel for pixel, so the panel picks the one it shows and
# nothing is scaled: 22 px (Plasma's default panel), 24 and 32. At 32 he is the island's own
# frame; at 22 and 24 a smaller Zeca drawn for them. Every state but idle wears a round badge with
# a dark glyph: busy dots on white (the island has no working color: cyan there means a question),
# a bang on its warning amber, a check on green, a cross on red. A light rim keeps the black
# vulture visible on a dark panel.
TRAY_SIZES = (22, 24, 32)
TRAY_RIM = (226, 227, 232, 150)
# The badges, "o" the fill, "x" the glyph (tray_badge swaps them). 12 x 12 with 2 px strokes at
# 32 px (a desktop that only has that size scales it down); 9 x 9 with 1 px strokes below.
_DISC = {
    12: [
        "....KKKK....",
        "..KKooooKK..",
        ".KooooooooK.",
        ".KooooooooK.",
        "KooooooooooK",
        "KooooooooooK",
        "KooooooooooK",
        "KooooooooooK",
        ".KooooooooK.",
        ".KooooooooK.",
        "..KKooooKK..",
        "....KKKK....",
    ],
    9: [
        "...KKK...",
        ".KKoooKK.",
        ".KoooooK.",
        "KoooooooK",
        "KoooooooK",
        "KoooooooK",
        ".KoooooK.",
        ".KKoooKK.",
        "...KKK...",
    ],
}
# disc -> glyph -> (first row, rows), drawn over the disc.
_GLYPHS = {
    12: {
        "dots": (5, ["..xx.xx.xx..",
                     "..xx.xx.xx.."]),
        "dots2": (5, ["..xx.xx.....",
                      "..xx.xx....."]),
        "bang": (2, [".....xx.....",
                     ".....xx.....",
                     ".....xx.....",
                     ".....xx.....",
                     ".....xx.....",
                     "............",
                     ".....xx.....",
                     ".....xx....."]),
        "check": (3, ["........xx..",
                      ".......xx...",
                      "..xx..xx....",
                      "...xxxx.....",
                      "....xx......"]),
        "cross": (2, ["..xx....xx..",
                      "...xx..xx...",
                      "....xxxx....",
                      ".....xx.....",
                      ".....xx.....",
                      "....xxxx....",
                      "...xx..xx...",
                      "..xx....xx.."]),
    },
    9: {
        "dots": (4, ["..x.x.x.."]),
        "dots2": (4, ["..x.x...."]),
        "bang": (2, ["....x....",
                     "....x....",
                     "....x....",
                     ".........",
                     "....x...."]),
        "check": (2, ["......x..",
                      ".....x...",
                      "..x.x....",
                      "...x....."]),
        "cross": (2, ["..x...x..",
                      "...x.x...",
                      "....x....",
                      "...x.x...",
                      "..x...x.."]),
    },
}

def tray_badge(disc, fill, glyph, ink="E", edge="K"):
    top, marks = _GLYPHS[disc][glyph]
    rows = [list(r) for r in _DISC[disc]]
    for y, line in enumerate(marks):
        for x, c in enumerate(line):
            if c == "x":
                rows[top + y][x] = "x"
    swap = {"o": fill, "x": ink, "K": edge}
    return ["".join(swap.get(c, c) for c in r) for r in rows]

# The small Zeca, for 22 and 24 px: perched facing right (band at the neck, feet on the wire), his
# head lowered as he works, puffed up and hissing as it fails, and wings spread (front on) when he
# needs you.
TRAY_SMALL = {
    "perch": [
        "...KBBK.........",
        "..KiisBK..hhh...",
        ".KisbbbBKhHHEeP.",
        ".isbsbbbBHwHHNPp",
        "KisbbbbBdwHHwPPp",
        "isbsbbBdAAw...p.",
        "ibsbbBdbbbB.....",
        "BbsbBdbbbBB.....",
        "BBbsBsbbBBK.....",
        ".BBbBbsbBB......",
        ".KBBsBsBBK......",
        "KBBKBBBBK.......",
        "BK...L.L........",
        ".....LlLl.......",
    ],
    "perch_down": [
        "...KBBK.........",
        "..KiisBK........",
        ".KisbbbBK.......",
        ".isbsbbbBK......",
        "KisbbbbBdBhhh...",
        "isbsbbBdAhHHEe..",
        "ibsbbBdbbHwHHNP.",
        "BbsbBdbbbwHHPPp.",
        "BBbsBsbbBBwPPp..",
        ".BBbBbsbBB...p..",
        ".KBBsBsBBK......",
        "KBBKBBBBK.......",
        "BK...L.L........",
        ".....LlLl.......",
    ],
    "puff": [
        "...KBBBK........",
        "..KiisbBK.hhh...",
        ".KisbbbbBhHHEeP.",
        ".isbsbbbBHwHHNPp",
        "KisbbbbbBwHHRR.p",
        "isbsbbbBdAwPPp..",
        "ibsbbbBdbbB.....",
        "BbsbbBdbbbBB....",
        "BBbsbBsbbBBK....",
        ".BBbsBbsbBB.....",
        ".KBBbBsBBBK.....",
        "KBBKBBBBBK......",
        "BK...L.L........",
        ".....LlLl.......",
    ],
    "spread": [
        ".........hhh.........",
        "........hEHEh........",
        "........HHPHH........",
        ".KBBK....wpw....KBBK.",
        "KBiisBB.BAAB.BBsiiBK.",
        "WBbsbsbdBbbBdbsbsbBW.",
        "WvBbsbbdBbbBdbbsbBvW.",
        "WWvBbbbbBbbBbbbbBvWW.",
        "KWvWvBBBBbbBBBBvWvWK.",
        ".KvWKvK.KbbK.KvKWvK..",
        "..KK.KK..KK..KK.KK...",
        ".........L.L.........",
    ],
}
TRAY_SMALL["perch:blink"] = blink(TRAY_SMALL["perch"])
TRAY_SMALL["spread:flash"] = TRAY_SMALL["spread"]

def _composite(fr, ox=0, oy=0):
    """An island frame as one grid of palette keys, its top-left at (ox, oy) of the frame."""
    cells = {}
    for part, px, py in fr["layers"]:
        for y, row in enumerate(PARTS[part]):
            for x, c in enumerate(row):
                if c != ".":
                    cells[(fr["dx"] + px + x - ox, fr["dy"] + py + y - oy)] = c
    assert min(x for x, _ in cells) >= 0 and min(y for _, y in cells) >= 0
    w = max(x for x, _ in cells) + 1
    h = max(y for _, y in cells) + 1
    return ["".join(cells.get((x, y), ".") for x in range(w)) for y in range(h)]

# state -> per size: (the bird's frames, where he sits), then the badge on each frame or None.
# The badge sits in the top right corner; every bird sits a little left of center for it, idle
# too, so a change of state does not shift him. Needs you is the loudest: spread wings, and a
# badge that flashes dark.
def _big(clip, *frames, origin=(0, 0)):
    return [_composite(CLIPS[clip]["frames"][n], *origin) for n in frames]

_S = TRAY_SMALL
TRAY = {
    "idle": ({32: (_big("idle", 0, 1), (2, 11)),
              24: ([_S["perch"], _S["perch:blink"]], (2, 10)),
              22: ([_S["perch"], _S["perch:blink"]], (1, 8))},
             None),
    "working": ({32: (_big("edit", 0, 2), (1, 10)),
                 24: ([_S["perch"], _S["perch_down"]], (1, 10)),
                 22: ([_S["perch"], _S["perch_down"]], (1, 8))},
                [("W", "dots"), ("W", "dots2")]),
    "needs-you": ({32: (_big("approval", 0, 1, origin=(-4, -2)), (-1, 13)),
                   24: ([_S["spread"], _S["spread:flash"]], (1, 12)),
                   22: ([_S["spread"], _S["spread:flash"]], (0, 10))},
                  [("Y", "bang"), ("E", "bang", "Y", "Y")]),
    "done": ({32: (_big("idle", 0, 0), (1, 11)),
              24: ([_S["perch"], _S["perch"]], (1, 10)),
              22: ([_S["perch"], _S["perch"]], (1, 8))},
             [("G", "check"), ("G", "check")]),
    "failed": ({32: (_big("fail", 0, 1), (2, 11)),
                24: ([_S["puff"], _S["puff"]], (1, 10)),
                22: ([_S["puff"], _S["puff"]], (1, 8))},
               [("r", "cross"), ("r", "cross")]),
}

def tray_frame(state, size, n):
    """One frame as rows of RGBA, rim included."""
    rgb = {k: tuple(int(v[i:i+2], 16) for i in (1, 3, 5)) for k, v in PALETTE.items()}
    sizes, badges = TRAY[state]
    frames, (bx, by) = sizes[size]
    img = [[None] * size for _ in range(size)]
    def put(rows, X0, Y0):
        for y, row in enumerate(rows):
            for x, c in enumerate(row):
                X, Y = X0 + x, Y0 + y
                if c != "." and 0 <= X < size and 0 <= Y < size:
                    img[Y][X] = (*rgb[c], 255)
    put(frames[n], bx, by)
    if badges:
        disc = 12 if size >= 32 else 9
        put(tray_badge(disc, *badges[n]), size - disc, 0)
    return [[img[y][x] or (TRAY_RIM if any(
        0 <= x + dx < size and 0 <= y + dy < size and img[y + dy][x + dx]
        for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))) else (0, 0, 0, 0))
        for x in range(size)] for y in range(size)]

def tray(out_dir):
    for size in TRAY_SIZES:
        (out_dir / str(size)).mkdir(parents=True, exist_ok=True)
        for state in TRAY:
            for n in range(2):
                _png_rgba(out_dir / str(size) / f"{state}-{n}.png", tray_frame(state, size, n))
    print(out_dir, TRAY_SIZES, list(TRAY))

def _png_rgba(path, rows):
    import struct, zlib
    h, w = len(rows), len(rows[0])
    raw = b"".join(b"\0" + b"".join(bytes(px) for px in row) for row in rows)
    chunk = lambda k, d: struct.pack(">I", len(d)) + k + d + struct.pack(">I", zlib.crc32(k + d))
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
                     + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))

if __name__ == "__main__":
    build(ROOT / "ui/src/character/zeca/zeca.json")
    sheet(ROOT / "design/mascots/zeca/clips.png")
    looks_sheet(ROOT / "design/mascots/zeca/looks.png")
    tray(ROOT / "app/icons/tray")
