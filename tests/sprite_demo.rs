//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The sprites demo, headless: boot the ROM and verify the IRQ-driven
//! bounce — positions advance every frame inside the 0..248 / 0..184
//! box, every velocity eventually flips sign at a wall, and the composited
//! display differs from the raw fb plane under each sprite.

use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

fn boot() -> (Machine, Cpu) {
    let src = include_str!("../software/sprites.s");
    let bin = assemble(src).unwrap();
    let mut m = Machine::new(image(&bin.segments));
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    // the checkerboard fill is a ~5-frame init; let it finish and the IRQ
    // take over before a test starts sampling
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    (m, cpu)
}

fn image(segments: &[(u16, Vec<u8>)]) -> [u8; 0x2000] {
    let mut img = [0u8; 0x2_000];
    for &(addr, ref bytes) in segments {
        img[addr as usize - 0xE000..addr as usize - 0xE000 + bytes.len()]
            .copy_from_slice(bytes);
    }
    img
}

// display byte at fb row r (0..191), byte column c (0..31)
fn disp(m: &Machine, r: u16, c: u16) -> u8 {
    m.fb()[r as usize * 32 + c as usize]
}

// raw background plane byte (sprites never touch it)
fn plane(m: &Machine, r: u16, c: u16) -> u8 {
    m.read(0x4000 + r * 32 + c)
}

#[test]
fn sprites_move_every_frame_via_irq() {
    let (mut m, mut cpu) = boot();
    m.run_frame(&mut cpu);
    let x0 = m.read(0x5808);
    let x1 = m.read(0x580C);
    m.run_frame(&mut cpu);
    assert_ne!(m.read(0x5808), x0, "sprite 0 x must advance each frame");
    assert_ne!(m.read(0x580C), x1, "sprite 1 x must advance each frame");
}

#[test]
fn sprites_stay_inside_the_bounce_box() {
    let (mut m, mut cpu) = boot();
    for _ in 0..400 {
        m.run_frame(&mut cpu);
        let x0 = m.read(0x5808);
        let y0 = m.read(0x5809);
        let x1 = m.read(0x580C);
        let y1 = m.read(0x580D);
        assert!(x0 <= 248, "sprite 0 x {x0} past the right wall");
        assert!(y0 <= 184, "sprite 0 y {y0} past the bottom wall");
        assert!(x1 <= 248, "sprite 1 x {x1} past the right wall");
        assert!(y1 <= 184, "sprite 1 y {y1} past the bottom wall");
        assert!(m.read(0x5809) <= 184);
    }
}

#[test]
fn every_velocity_flips_at_a_wall() {
    let (mut m, mut cpu) = boot();
    let mut seen_pos = [false; 4];
    let mut seen_neg = [false; 4];
    for _ in 0..400 {
        m.run_frame(&mut cpu);
        // velocities live in zero page $44-$47
        for (i, addr) in [0x44u16, 0x45, 0x46, 0x47].iter().enumerate() {
            let b = m.read(*addr);
            if b & 0x80 == 0 && b != 0 {
                seen_pos[i] = true;
            }
            if b & 0x80 != 0 {
                seen_neg[i] = true;
            }
        }
    }
    for i in 0..4 {
        assert!(seen_pos[i] && seen_neg[i], "velocity {i} never flipped: {seen_pos:?} {seen_neg:?}");
    }
}

#[test]
fn checkerboard_is_drawn_and_sprites_xor_it() {
    let (mut m, mut cpu) = boot();
    m.run_frame(&mut cpu);
    // checkerboard: parity of (col>>1) ^ (row>>2) picks $F0/$0F
    assert_eq!(plane(&m, 0, 0), 0xF0, "row 0 col 0: parity 0 -> $F0");
    assert_eq!(plane(&m, 4, 0), 0x0F, "row 4: row parity 1 -> $0F");
    assert_eq!(plane(&m, 0, 2), 0x0F, "col 2: col parity 1 -> $0F");
    assert_eq!(plane(&m, 4, 2), 0xF0, "row 4 col 2: parity 0 -> $F0");
    // the sprite's display rows must differ from the raw plane (XOR)
    let sy = m.read(0x5809) as u16;
    let mut diff = 0;
    for r in sy..sy + 8 {
        for c in 0..32u16 {
            if disp(&m, r, c) != plane(&m, r, c) {
                diff += 1;
            }
        }
    }
    assert!(diff > 0, "sprite rows must show the XOR composited sprite");
}