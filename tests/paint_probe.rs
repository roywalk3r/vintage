//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The paint app, headless: boot the 2bpp color ROM, post keys, and assert
//! on framebuffer bytes — cursor stamp/restore, plot/erase, the color
//! cycle, canvas clear, and the $6000 shadow save/load roundtrip.

use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

fn image(segments: &[(u16, Vec<u8>)]) -> [u8; 0x2000] {
    let mut img = [0u8; 0x2000];
    for &(addr, ref bytes) in segments {
        img[addr as usize - 0xE000..addr as usize - 0xE000 + bytes.len()]
            .copy_from_slice(bytes);
    }
    img
}

fn boot() -> (Machine, Cpu) {
    let src = include_str!("../software/paint.s");
    let bin = assemble(src).unwrap();
    let mut m = Machine::with_banks(vec![image(&bin.segments)]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    // frames 0-1 are reset warm-up; boot completes in frame 2, and the
    // newest-wins key buffer must be empty before the first posted key
    for _ in 0..4 {
        m.run_frame(&mut cpu);
    }
    (m, cpu)
}

/// One key per 3 frames: the app polls once per frame (gated on $5802),
/// so a key posted before frame N is consumed at the start of frame N+1;
/// the third frame guarantees every assertion lands after the handler and
/// the cursor re-stamp, while the loop is quiescent in its spin.
fn type_keys(m: &mut Machine, cpu: &mut Cpu, keys: &[u8]) {
    for &k in keys {
        m.key(k);
        for _ in 0..3 {
            m.run_frame(cpu);
        }
    }
}

/// Walk the cursor ($11 up, $12 down, $13 left, $14 right).
fn walk(m: &mut Machine, cpu: &mut Cpu, right: usize, down: usize, left: usize, up: usize) {
    let mut keys = Vec::new();
    keys.extend(std::iter::repeat(0x14).take(right));
    keys.extend(std::iter::repeat(0x12).take(down));
    keys.extend(std::iter::repeat(0x13).take(left));
    keys.extend(std::iter::repeat(0x11).take(up));
    type_keys(m, cpu, &keys);
}

fn fb_byte(m: &Machine, fx: u16, fy: u16) -> u8 {
    m.read(0x4000 + fy * 32 + fx / 4)
}

#[test]
fn paint_boots_with_cursor_at_origin() {
    let (m, _cpu) = boot();
    assert_eq!(fb_byte(&m, 0, 0), 0xC0, "cursor = color 3 at (0,0), shift 6");
}

#[test]
fn paint_plots_erases_and_cycles_colors() {
    let (mut m, mut cpu) = boot();
    // (5,0) is byte $4001 sub 1; (8,0)/(9,0)/(10,0)/(11,0) live in $4002
    walk(&mut m, &mut cpu, 5, 0, 0, 0);
    type_keys(&mut m, &mut cpu, b"z"); // plot color 1
    assert_eq!(fb_byte(&m, 5, 0), 0x30, "cursor (3) covers the plot at the same sub-pixel");
    type_keys(&mut m, &mut cpu, &[0x14; 3]); // right x3 -> (8,0)
    assert_eq!(fb_byte(&m, 5, 0), 0x10, "plotted pixel persists");
    assert_eq!(fb_byte(&m, 8, 0), 0xC0, "cursor at (8,0), sub 0");
    type_keys(&mut m, &mut cpu, b"c"); // color 2
    type_keys(&mut m, &mut cpu, b"z");
    assert_eq!(fb_byte(&m, 8, 0), 0xC0, "cursor covers the color-2 plot at the same sub-pixel");
    type_keys(&mut m, &mut cpu, &[0x14]); // -> (9,0)
    assert_eq!(fb_byte(&m, 9, 0), 0xB0, "color 2 sub 0 | cursor sub 1 ($30)");
    type_keys(&mut m, &mut cpu, b"c"); // color 3
    type_keys(&mut m, &mut cpu, b"z");
    assert_eq!(fb_byte(&m, 9, 0), 0xB0, "color 3 ($30) re-plotted under cursor");
    type_keys(&mut m, &mut cpu, &[0x14]); // -> (10,0)
    assert_eq!(fb_byte(&m, 10, 0), 0xBC, "color-3 plot at sub 1 | cursor sub 2 ($0C)");
    type_keys(&mut m, &mut cpu, &[0x13]); // left -> (9,0)
    type_keys(&mut m, &mut cpu, b"x"); // erase (9,0)
    type_keys(&mut m, &mut cpu, &[0x14]); // right -> (10,0)
    assert_eq!(fb_byte(&m, 10, 0), 0x8C, "sub 1 cleared, sub 0 and cursor left");
    type_keys(&mut m, &mut cpu, b"c"); // 3 -> wrap -> 1
    type_keys(&mut m, &mut cpu, b"z"); // plot color 1 at (10,0)
    type_keys(&mut m, &mut cpu, &[0x14]); // -> (11,0)
    assert_eq!(fb_byte(&m, 11, 0), 0x87, "color 1 sub 2 ($04) | cursor sub 3 ($03)");
}

#[test]
fn paint_clear_zeroes_canvas_except_cursor() {
    let (mut m, mut cpu) = boot();
    walk(&mut m, &mut cpu, 127, 191, 0, 0); // park in the corner
    type_keys(&mut m, &mut cpu, b"n");
    let fb = m.fb();
    let nonzero: Vec<usize> = (0..0x1800).filter(|&i| fb[i] != 0).collect();
    assert_eq!(nonzero, vec![0x17FF], "only the cursor byte at $57FF survives");
    assert_eq!(fb[0x17FF], 0x03, "cursor sub 3 at (127,191)");
}

#[test]
fn paint_save_load_roundtrip_through_6000_shadow() {
    let (mut m, mut cpu) = boot();
    walk(&mut m, &mut cpu, 127, 191, 0, 0);
    type_keys(&mut m, &mut cpu, &[0x13, b'z']); // left, plot color 1 at (126,191)
    type_keys(&mut m, &mut cpu, &[0x14]); // back to the corner
    // (126,191) is sub 2 of $57FF: 1<<2 = $04; cursor sub 3 = $03
    assert_eq!(m.read(0x57FF), 0x07, "plotted 1 at sub 2 | cursor sub 3");
    type_keys(&mut m, &mut cpu, b"s"); // save to $6000
    type_keys(&mut m, &mut cpu, b"n"); // clear
    assert_eq!(m.read(0x57FF), 0x03, "clear wiped the plot, cursor only");
    type_keys(&mut m, &mut cpu, b"l"); // load from $6000
    assert_eq!(m.read(0x57FF), 0x07, "load restored the plotted pixel");
    // the $6000 shadow itself holds the saved image — cursor-free, since
    // fbcopy runs between unstamp and stamp
    assert_eq!(m.read(0x77FF), 0x04, "shadow $6000+0x17FF mirrors the cursor-free image");
}

