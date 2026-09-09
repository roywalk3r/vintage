// VINTAGE-1
// Author: roywalk3r
// Repo: https://github.com/roywalk3r/vintage
// License: MIT
//! The dodge game, headless: boot the ROM and verify the player steers with
//! the arrow keys, the rock falls and respawns from random columns, dodging
//! scores and speeds the fall up, and landing the rock on the player's
//! column crashes and resets the score.

use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::{Machine, KEY_LEFT, KEY_RIGHT};

fn boot() -> (Machine, Cpu) {
    let src = include_str!("../software/dodge.s");
    let bin = assemble(src).unwrap();
    let mut m = Machine::new(image(&bin.segments));
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    // init (score stamp + latches) is quick; two frames gets the main loop
    // through its first body runs
    for _ in 0..2 {
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

// '0' glyph row 0 — the only digit whose first row is $7C, so a changed
// score byte always reads as a different value
const ZERO_TOP: u8 = 0x7C;

#[test]
fn game_boots_with_player_rock_and_score() {
    let (mut m, mut cpu) = boot();
    m.run_frame(&mut cpu);
    assert_eq!(m.read(0x5810), 3, "both sprites enabled");
    assert_eq!(m.read(0x5808), 120, "player x centered");
    assert_eq!(m.read(0x5809), 176, "player y on the bottom rows");
    let ry = m.read(0x580D);
    assert!(ry <= 183, "rock y {ry} must be on screen");
    let fb = m.fb();
    assert_eq!(fb[0], ZERO_TOP, "score tens digit stamped at fb (0,0)");
    assert_eq!(fb[1], ZERO_TOP, "score ones digit stamped at fb (8,0)");
}

#[test]
fn arrow_keys_move_the_player() {
    let (mut m, mut cpu) = boot();
    m.run_frame(&mut cpu);
    let x0 = m.read(0x5808);
    m.key(KEY_LEFT);
    m.run_frame(&mut cpu);
    assert_eq!(m.read(0x5808), x0 - 8, "left arrow steps the player left");
    m.key(KEY_RIGHT);
    m.run_frame(&mut cpu);
    assert_eq!(m.read(0x5808), x0, "right arrow steps the player right");
    // hammering left clamps at the wall instead of wrapping
    for _ in 0..20 {
        m.key(KEY_LEFT);
        m.run_frame(&mut cpu);
    }
    assert_eq!(m.read(0x5808), 0, "left wall clamp");
    m.key(KEY_RIGHT);
    m.run_frame(&mut cpu);
    assert_eq!(m.read(0x5808), 8, "right arrow works from the wall");
}

#[test]
fn rock_falls_and_respawns() {
    let (mut m, mut cpu) = boot();
    let mut spawns = 0;
    for _ in 0..800 {
        m.run_frame(&mut cpu);
        let y = m.read(0x580D);
        assert!(y <= 183, "rock y {y} must never leave the screen");
        if y == 0 {
            spawns += 1;
        }
    }
    assert!(spawns >= 2, "rock must respawn, saw {spawns} spawn frames");
}

#[test]
fn dodging_scores_and_speeds_up() {
    let (mut m, mut cpu) = boot();
    let mut scored = false;
    let mut sped_up = false;
    let mut score_drew = false;
    for _ in 0..800 {
        m.run_frame(&mut cpu);
        if m.read(0x46) >= 1 {
            scored = true; // SCORE
        }
        if m.read(0x44) == 3 {
            sped_up = true; // RSPEED floor
        }
        if m.fb()[1] != ZERO_TOP {
            score_drew = true; // ones digit stopped being '0'
        }
    }
    assert!(scored, "no dodge scored within 800 frames");
    assert!(sped_up, "RSPEED never reached its floor of 3");
    assert!(score_drew, "score glyph never left '0'");
}

#[test]
fn forced_column_crash_resets_score() {
    let (mut m, mut cpu) = boot();
    m.run_frame(&mut cpu);
    // park the rock on the player's column, just above the band: it must
    // crash there instead of scoring
    m.write(0x42, 120); // RX = PX
    m.write(0x43, 160); // RY
    let mut crashed = false;
    for _ in 0..400 {
        m.run_frame(&mut cpu);
        if m.read(0x43) == 0 {
            crashed = true; // respawned, i.e. the crash path ran
            break;
        }
    }
    assert!(crashed, "rock never respawned — crash path never ran");
    assert_eq!(m.read(0x46), 0, "score resets on crash");
    assert_eq!(m.read(0x5807), 30, "crash blip is sounding");
}
