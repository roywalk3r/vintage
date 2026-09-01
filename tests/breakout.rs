//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! Breakout: unattended play keeps bricks and paddle intact, and the
//! paddle-bounce aim tiers (45° middle, shallow 2px quarters) and the
//! slope-aware wall reflection are probed by crafting ball/paddle state
//! directly in zero page.

use std::fs;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

fn rom_for(src: &str) -> [u8; 0x2000] {
    let s = fs::read_to_string(src).unwrap();
    let bin = vintage::asm::assemble(&s).expect("demo must assemble");
    let mut rom = [0u8; 0x2000];
    for (addr, data) in &bin.segments {
        let off = *addr as usize - 0xE000;
        rom[off..off + data.len()].copy_from_slice(data);
    }
    rom
}

fn lit_bits(m: &Machine, y0: usize, y1: usize) -> usize {
    let fb = m.fb();
    let mut n = 0usize;
    for y in y0..y1 {
        for b in &fb[y * 32..(y + 1) * 32] {
            n += b.count_ones() as usize;
        }
    }
    n
}

// Zero-page map from software/breakout.s
const PX: u16 = 0xE4;
const OPX: u16 = 0xE5;
const BX: u16 = 0xE6;
const BY: u16 = 0xE7;
const DXF: u16 = 0xE8;
const DYF: u16 = 0xE9;
const SCD: u16 = 0xEA;
const HRATE: u16 = 0xF4;

fn boot() -> (Machine, Cpu) {
    let mut m = Machine::new(rom_for("software/breakout.s"));
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    // frames 0-1 are reset warm-up; init completes during frame 2, leaving
    // the main loop parked in wait_frame with the ball at spawn
    for _ in 0..3 {
        m.run_frame(&mut cpu);
    }
    (m, cpu)
}

// Craft a falling ball and force exactly one ball-step: the frame counter
// ticks at the END of run_frame, so the first call spins in wait_frame and
// the second call's pass sees SCD=1, steps once, and self-clears.
fn step_once(m: &mut Machine, cpu: &mut Cpu) {
    m.write(SCD, 1);
    m.run_frame(cpu); // frame counter advances (CPU idle in wait_frame)
    m.run_frame(cpu); // main-loop pass: SCD=1 → exactly one step_ball
}

#[test]
fn breakout_paddle_aim_tiers() {
    // (ball x, expected direction, expected slope) at paddle left edge 80
    for (bx, want_dxf, want_hrate) in
        // bx pre-steps horizontally (DXF=1, HRATE=1) before the zone check:
        // 110 would land on 111 = T1+31 exactly, the miss boundary
        [(84u8, 0u8, 2u8), (90, 0, 1), (100, 1, 1), (108, 1, 2)]
    {
        let (mut m, mut cpu) = boot();
        m.write(PX, 10); // paddle left edge = 10*8 = 80
        m.write(OPX, 10);
        m.write(BX, bx);
        m.write(BY, 177); // just entered the paddle zone
        m.write(DYF, 1); // falling
        step_once(&mut m, &mut cpu);
        assert_eq!(m.read(DXF), want_dxf, "aim: bx={bx}");
        assert_eq!(m.read(HRATE), want_hrate, "slope: bx={bx}");
        assert_eq!(m.read(BY), 176, "ball parks on the paddle: bx={bx}");
        assert_eq!(m.read(DYF), 0, "ball must bounce up: bx={bx}");
    }
}

#[test]
fn breakout_wall_reflect_honors_hrate() {
    // right wall at slope 2: 254 + 2 = 256 → reflected to 253, moving left
    let (mut m, mut cpu) = boot();
    m.write(BX, 254);
    m.write(BY, 100);
    m.write(DYF, 1);
    m.write(DXF, 1);
    m.write(HRATE, 2);
    step_once(&mut m, &mut cpu);
    assert_eq!(m.read(BX), 253, "right wall reflects at slope 2");
    assert_eq!(m.read(DXF), 0, "direction flips at the right wall");
    assert_eq!(m.read(BY), 101, "ball keeps falling past the wall");

    // left wall at slope 2: 1 - 2 = -1 → reflected to 1, moving right
    // (mirrors the right wall: overshoot px from the wall, r=256→253 ↔ r=-1→1)
    m.write(BX, 1);
    m.write(DXF, 0);
    step_once(&mut m, &mut cpu);
    assert_eq!(m.read(BX), 1, "left wall reflects at slope 2");
    assert_eq!(m.read(DXF), 1, "direction flips at the left wall");
}

#[test]
fn breakout_plays_without_input() {
    let (mut m, mut cpu) = boot();
    for _ in 0..320 {
        m.run_frame(&mut cpu);
    }
    let bricks = lit_bits(&m, 16, 48);
    assert!(bricks > 0, "some bricks must remain after 320 frames");
    assert!(bricks < 32 * 32 * 8, "at least one brick must break on its own trajectory");
    assert!(lit_bits(&m, 176, 184) > 0, "paddle must stay drawn");
}