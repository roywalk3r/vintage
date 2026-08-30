//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! Snake gameplay: keyboard input at $5800 must steer the snake.

use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::{Machine, KEY_UP};

#[test]
fn snake_turns_up_on_key() {
    let src = std::fs::read_to_string("software/snake.s").unwrap();
    let bin = assemble(&src).expect("snake.s must assemble");
    let mut rom = [0u8; 0x2000];
    let mut pokes: Vec<(u16, Vec<u8>)> = vec![];
    for (addr, bytes) in bin.segments.clone() {
        if addr >= 0xE000 {
            let base = addr as usize - 0xE000;
            rom[base..base + bytes.len()].copy_from_slice(&bytes);
        } else {
            pokes.push((addr, bytes));
        }
    }
    let mut m = Machine::new(rom);
    for (addr, bytes) in pokes {
        for (i, v) in bytes.iter().enumerate() {
            m.write(addr + i as u16, *v);
        }
    }
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    for n in 0..12 {
        if n == 5 {
            m.key(KEY_UP);
        }
        m.run_frame(&mut cpu);
    }
    let rd = |a: u16| m.read(a);
    let (hx, hy) = (rd(0x6100), rd(0x6140));
    assert_eq!(hx, 7, "head x after two steps (one right, one up)");
    assert_eq!(hy, 11, "head y after the up-turn");
    assert_eq!(rd(0xE0), 3, "direction register DIR should be up");
}
