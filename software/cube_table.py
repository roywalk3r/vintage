#!/usr/bin/env python3
"""Precompute the 32-step projected-cube vertex table for software/cube.s.

Vertices (±1,±1,±1) spin about the Y axis, 32 steps/turn; each step stores
8 vertices as (px,py) pairs, 16 bytes per step, 512 bytes total.
"""
import math

CX, CYV, K, DIST = 128, 96, 40, 4.2
VX = [(sx, sy, sz) for sz in (-1, 1) for sy in (-1, 1) for sx in (-1, 1)]

print("TBL:")
rows = []
for step in range(32):
    t = step * math.pi / 16
    pts = []
    for (x, y, z) in VX:
        xr = x * math.cos(t) + z * math.sin(t)
        zr = -x * math.sin(t) + z * math.cos(t) + DIST
        px = CX + round(K * xr / zr)
        py = CYV - round(K * y / zr)
        pts += [px, py]
    rows.append(pts)

xs = [p for r in rows for p in r[0::2]]
ys = [p for r in rows for p in r[1::2]]
print(f"; bbox x {min(xs)}..{max(xs)}  y {min(ys)}..{max(ys)}")
for i, r in enumerate(rows):
    line = ",".join(f"${v:02X}" for v in r)
    print(f" .byte {line}" + (f" ; step {i}" if i % 8 == 0 else ""))
