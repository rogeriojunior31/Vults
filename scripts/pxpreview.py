#!/usr/bin/env python3
"""Renders palette-indexed pixel grids (design/mascots/*.json) to a PNG sheet, stdlib only.
Usage: scripts/pxpreview.py <sprites.json> <out.png> [scale]"""
import json, struct, sys, zlib

def hexrgb(h):
    h = h.lstrip("#"); return tuple(int(h[i:i+2], 16) for i in (0, 2, 4))

def render(spec_path, out_path, scale=10, pad=2, bg="#000000"):
    spec = json.load(open(spec_path))
    pal = {k: hexrgb(v) for k, v in spec["palette"].items()}
    sprites = spec["sprites"]
    cell_w = max(len(r) for s in sprites.values() for r in s) + pad * 2
    cell_h = max(len(s) for s in sprites.values()) + pad * 2
    cols = min(len(sprites), spec.get("columns", 4)); rows = -(-len(sprites) // cols)
    W, H = cols * cell_w * scale, rows * cell_h * scale
    bgc = hexrgb(bg); img = [[bgc] * W for _ in range(H)]
    for i, (name, grid) in enumerate(sprites.items()):
        cx, cy = (i % cols) * cell_w + pad, (i // cols) * cell_h + pad
        for y, row in enumerate(grid):
            for x, ch in enumerate(row):
                if ch in pal:
                    for dy in range(scale):
                        line = img[(cy + y) * scale + dy]
                        for dx in range(scale):
                            line[(cx + x) * scale + dx] = pal[ch]
    raw = b"".join(b"\0" + bytes(c for px in line for c in px) for line in img)
    ch = lambda k, d: struct.pack(">I", len(d)) + k + d + struct.pack(">I", zlib.crc32(k + d))
    open(out_path, "wb").write(b"\x89PNG\r\n\x1a\n" + ch(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
                               + ch(b"IDAT", zlib.compress(raw, 6)) + ch(b"IEND", b""))
    print(out_path, W, H, list(sprites))

if __name__ == "__main__":
    render(sys.argv[1], sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 10)
