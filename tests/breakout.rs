//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! Breakout: the ball breaks at least one brick on its own serve trajectory,
//! and the paddle stays drawn through 320 frames of unattended play.

use std::fs;

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

fn lit_bits(m: &vintage::machine::Machine, y0: usize, y1: usize) -> usize {
    let fb = m.fb();
    let mut n = 0usize;
    for y in y0..y1 {
        for b in &fb[y * 32..(y + 1) * 32] {
            n += b.count_ones() as usize;
        }
    }
    n
}

#[test]
fn breakout_plays_without_input() {
    let mut m = vintage::machine::Machine::new(rom_for("software/breakout.s"));
    let mut cpu = vintage::cpu::Cpu::new();
    cpu.reset(&mut m);
    for _ in 0..200 {
        cpu.step(&mut m); // let init finish before the first gated frame
    }
    for _ in 0..320 {
        m.run_frame(&mut cpu);
    }
    let bricks = lit_bits(&m, 16, 48);
    assert!(bricks > 0, "some bricks must remain after 320 frames");
    assert!(bricks < 32 * 32 * 8, "at least one brick must break on its own trajectory");
    assert!(lit_bits(&m, 176, 184) > 0, "paddle must stay drawn");
}