#!/usr/bin/env python3
"""Draws the app icon from Zeca's sprite data, as PNGs and an ICO (stdlib only)."""
import json, pathlib, struct, zlib

# The icon is Zeca himself: his perched body and head, cut from the sprite data, on nothing.
ZECA = json.loads((pathlib.Path(__file__).resolve().parent.parent / "ui/src/character/zeca/zeca.json").read_text())
GRID = 28
CLEAR = (0, 0, 0, 0)


def art() -> list[list[tuple]]:
    pal = {k: tuple(int(v[i:i + 2], 16) for i in (1, 3, 5)) + (255,) for k, v in ZECA["palette"].items()}
    pal["A"] = pal["K"]  # no agent band on the app icon
    grid = [[CLEAR] * GRID for _ in range(GRID)]
    # The perched bird is 24 cells wide and 21 tall: centred, with a cell of margin.
    ox, oy = 2, 3
    frame = ZECA["clips"]["idle"]["frames"][0]
    for part, x, y in frame["layers"]:
        for j, row in enumerate(ZECA["parts"][part]):
            for i, c in enumerate(row):
                if c != "." and c in pal and 0 <= oy + y + j < GRID and 0 <= ox + x + i < GRID:
                    grid[oy + y + j][ox + x + i] = pal[c]
    return grid


ART = art()


def png(size: int) -> bytes:
    rows = []
    for y in range(size):
        line = ART[y * GRID // size]
        row = bytearray([0])
        for x in range(size):
            row += bytes(line[x * GRID // size])
        rows.append(bytes(row))
    raw = zlib.compress(b"".join(rows), 9)

    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", raw) + chunk(b"IEND", b"")


out = pathlib.Path(__file__).resolve().parent.parent / "app" / "icons"
for name, size in [("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 512)]:
    (out / name).write_bytes(png(size))

# Windows: an ICO holding PNG images (supported since Vista); a size byte of 0 means 256.
images = [png(n) for n in (32, 256)]
offset = 6 + 16 * len(images)
ico = struct.pack("<HHH", 0, 1, len(images))
for n, data in zip((32, 256), images):
    ico += struct.pack("<BBBBHHII", n % 256, n % 256, 0, 0, 1, 32, len(data), offset)
    offset += len(data)
(out / "icon.ico").write_bytes(ico + b"".join(images))
print("icons written to", out)
