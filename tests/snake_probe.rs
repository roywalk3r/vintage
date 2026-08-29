//! VINTAGE-1
//! Copyright 2026 roywalk3r
//! SPDX-License-Identifier: MIT
//! Snake gameplay probes: wall death must not let the head enter a wall
//! cell, and '+'/'-' must retune the step divider live.

use std::fs;
use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

fn snake_rom() -> [u8; 0x2000] {
    let src = fs::read_to_string("software/snake.s").unwrap();
    let bin = assemble(&src).expect("assemble snake.s");
    let mut rom = [0u8; 0x2000];
    for (addr, bytes) in bin.segments.clone() {
        let base = addr as usize - 0xE000;
        rom[base..base + bytes.len()].copy_from_slice(&bytes);
    }
    rom
}

/// Snake begins at (6,12) heading right. Holding RIGHT must crash the snake
/// on the right wall with the head never drawn over the wall cell x=31,
/// detected as the game restarting (SX[0] back at 6).
#[test]
fn snake_head_never_enters_wall_cell() {
    let rom = snake_rom();
    let mut m = Machine::new(rom);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    let mut ever_31 = false;
    let mut progressed = false;
    let mut restarted = false;
    for _ in 0..600 {
        m.key(0x14); // right, every frame
        m.run_frame(&mut cpu);
        let hx = m.read(0x6100);
        if hx > 10 {
            progressed = true;
        }
        if hx == 31 {
            ever_31 = true;
        }
        if progressed && hx == 6 {
            restarted = true;
            break;
        }
    }
    assert!(restarted, "snake never crashed into the wall");
    assert!(!ever_31, "head pixel landed on the wall cell x=31");
}

/// '+' (0x15) tightens the move divider, '-' (0x16) loosens it. Observable
/// effect: at max speed the snake covers ground ~8x faster than at min
/// speed — assert head x advanced more within the same frame budget.
#[test]
fn snake_speed_keys_change_step_rate() {
    let rom = snake_rom();
    let run = |key: u8| -> u8 {
        let mut m = Machine::new(rom);
        let mut cpu = Cpu::new();
        cpu.reset(&mut m);
        for _ in 0..3 {
            m.key(0x14); // steady right while boot settles
            m.run_frame(&mut cpu);
        }
        m.key(key); // one clean frame for the speed key
        m.run_frame(&mut cpu);
        for _ in 0..36 {
            m.key(0x14);
            m.run_frame(&mut cpu);
        }
        m.read(0x6100)
    };
    let fast = run(0x15); // '+' pressed once: divider 4 -> 3
    let slow = run(0x16); // '-' pressed once: divider 4 -> 5
    assert!(fast > slow, "speed keys had no effect: fast {} slow {}", fast, slow);
}