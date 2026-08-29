//! VINTAGE-1
//! Copyright 2026 roywalk3r
//! SPDX-License-Identifier: MIT
//! Per-edge cube probe: patch cube.s so a single edge draws per run, then
//! diff that edge's raster against a reference rasterizer. Sweeps several
//! rotation steps to cover every Bresenham branch combination.

use vintage::asm::assemble;
use vintage::cpu::Cpu;
use vintage::machine::Machine;

fn lit(fb: &[u8]) -> std::collections::HashSet<(usize, usize)> {
    let mut s = std::collections::HashSet::new();
    for (i, &b) in fb.iter().enumerate() {
        for bit in 0..8 {
            if b & (0x80 >> bit) != 0 {
                s.insert(((i % 32) * 8 + bit, i / 32));
            }
        }
    }
    s
}

/// Reference rasterizer mirroring the 6502 `line` in software/cube.s:
/// y-order swap, +/-1 step, err starts at big/2, exit after the final step
/// (so the trailing endpoint pixel is not plotted — both implementations
/// leave it to the shared-vertex pixels covered by the other edges).
fn ref_line(
    pts: &mut std::collections::HashSet<(i32, i32)>,
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
) {
    let (mut x0, mut y0, mut x1, mut y1) = (ax, ay, bx, by);
    if y1 < y0 {
        std::mem::swap(&mut x0, &mut x1);
        std::mem::swap(&mut y0, &mut y1);
    }
    let mut dxa = x1 - x0;
    let sxf = if dxa < 0 { dxa = -dxa; -1 } else { 1 };
    let dya = y1 - y0;
    let horiz = dxa >= dya;
    let (big, small) = if horiz { (dxa, dya) } else { (dya, dxa) };
    let mut err = big / 2;
    loop {
        pts.insert((x0, y0));
        err -= small;
        if err < 0 {
            err += big;
            if horiz {
                y0 += 1;
            } else {
                x0 += sxf;
            }
        }
        if horiz {
            x0 += sxf;
        } else {
            y0 += 1;
        }
        if (horiz && x0 == x1) || (!horiz && y0 == y1) {
            break;
        }
    }
}

const EDGES: [(usize, usize); 12] = [
    (0, 1), (1, 3), (3, 2), (2, 0),
    (4, 5), (5, 7), (7, 6), (6, 4),
    (0, 4), (1, 5), (2, 6), (3, 7),
];

/// One-edge-per-run: patch cube.s so edge k (at rotation `step`) draws once,
/// run 6 frames, return lit pixels.
fn edge_run(k: usize, step: usize) -> Vec<(usize, usize)> {
    let src = std::fs::read_to_string("software/cube.s").unwrap();
    let src = {
        let old = " lda #0\n sta FCTR\n sta CNT\n sta STEP";
        let new = format!(
            " lda #0\n sta FCTR\n sta CNT\n lda #{step}\n sta STEP"
        );
        assert!(src.contains(old), "STEP init site");
        src.replace(old, &new)
    }
    .replace(" lda #0\n sta EI", &format!(" lda #{k}\n sta EI"))
    .replace(" cmp #12", &format!(" cmp #{}", k + 1));
    let bin = assemble(&src).expect("assemble");
    let mut rom = [0u8; 0x2000];
    for (addr, bytes) in bin.segments.clone() {
        let base = addr as usize - 0xE000;
        rom[base..base + bytes.len()].copy_from_slice(&bytes);
    }
    let mut m = Machine::new(rom);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    let mut v: Vec<(usize, usize)> = lit(m.fb()).into_iter().collect();
    v.sort();
    v
}

#[test]
fn cube_per_edge() {
    for step in [0usize, 5, 12, 20, 31] {
        for (k, &(a, b)) in EDGES.iter().enumerate() {
            let drawn = edge_run(k, step);
            let rom_tbl = tbl_bytes();
            let base = step * 16;
            let ax = rom_tbl[base + a * 2] as i32;
            let ay = rom_tbl[base + a * 2 + 1] as i32;
            let bx = rom_tbl[base + b * 2] as i32;
            let by = rom_tbl[base + b * 2 + 1] as i32;
            let mut expected: std::collections::HashSet<(i32, i32)> = Default::default();
            ref_line(&mut expected, ax, ay, bx, by);
            let drawn_i: std::collections::HashSet<(i32, i32)> =
                drawn.iter().map(|&(x, y)| (x as i32, y as i32)).collect();
            let extra: Vec<_> = drawn_i.difference(&expected).copied().collect();
            let missing: Vec<_> = expected.difference(&drawn_i).copied().collect();
            assert!(
                extra.is_empty() && missing.is_empty(),
                "step {step} edge {k} ({a}-{b}): extra {extra:?} missing {missing:?}"
            );
        }
    }
}

/// Vertex coordinate pairs read back from the assembled cube ROM.
fn tbl_bytes() -> Vec<u8> {
    let src = std::fs::read_to_string("software/cube.s").unwrap();
    let bin = assemble(&src).expect("assemble");
    let mut rom = [0u8; 0x2000];
    for (addr, bytes) in bin.segments.clone() {
        let base = addr as usize - 0xE000;
        rom[base..base + bytes.len()].copy_from_slice(&bytes);
    }
    let sig = [0x74u8, 0x6C, 0x8C, 0x6C, 0x74, 0x54, 0x8C, 0x54];
    let off = rom
        .windows(8)
        .position(|w| w == sig)
        .expect("TBL signature");
    rom[off..off + 512].to_vec()
}
