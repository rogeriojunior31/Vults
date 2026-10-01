#!/usr/bin/env python3
"""Draws the app icon: Zeca's head in 8-bit phosphor green, as PNGs (stdlib only)."""
import struct, zlib, pathlib

# 16x16. G = phosphor body, D = dark outline, E = eye, B = beak, . = transparent
ART = """
................
.....DDDDDD.....
....DGGGGGGD....
...DGGGGGGGGD...
...DGGEEGGGGD...
...DGGEEGGGGBB..
...DGGGGGGGBBBB.
...DGGGGGGGGBB..
....DGGGGGGD....
.....DGGGGD.....
....DGGGGGGD....
...DGGGGGGGGD...
..DGGGGGGGGGGD..
..DGGGGGGGGGGD..
...DDDDDDDDDD...
................
""".strip().splitlines()

COLORS = {
    "G": (0x5C, 0xFF, 0x9D, 255),
    "D": (0x0B, 0x1F, 0x14, 255),
    "E": (0x0B, 0x1F, 0x14, 255),
    "B": (0xF2, 0xC1, 0x4E, 255),
    ".": (0x10, 0x14, 0x12, 255),
}


def png(size: int) -> bytes:
    scale = size // 16
    rows = []
    for y in range(size):
        line = ART[y // scale]
        row = bytearray([0])
        for x in range(size):
            row += bytes(COLORS[line[x // scale]])
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
