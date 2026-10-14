//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The BASIC app, headless: boot the ROM, type program lines and direct
//! commands through the one-key buffer, and assert on the scrolling
//! terminal's ASCII mirror at $2500 (8 rows x 33 bytes per row). This
//! suite covers IF...THEN...ELSE: the untaken IF's quote-aware scan to
//! the line's standalone ELSE, the ELSE <line> jump and inline statement
//! forms, and a taken IF never seeing its own ELSE.

use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

fn image(segments: &[(u16, Vec<u8>)]) -> [u8; 0x2000] {
    let mut img = [0u8; 0x2_000];
    for &(addr, ref bytes) in segments {
        img[addr as usize - 0xE000..addr as usize - 0xE000 + bytes.len()]
            .copy_from_slice(bytes);
    }
    img
}

fn boot() -> (Machine, Cpu) {
    let src = include_str!("../software/basic.s");
    let bin = assemble(src).unwrap();
    let mut m = Machine::with_banks(vec![image(&bin.segments)]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    for _ in 0..4 {
        m.run_frame(&mut cpu);
    }
    (m, cpu)
}

fn type_keys(m: &mut Machine, cpu: &mut Cpu, keys: &[u8]) {
    for &k in keys {
        m.key(k);
        m.run_frame(cpu);
    }
}

fn term_row(m: &Machine, r: usize) -> String {
    let base = 0x2500 + r * 33;
    let mut s = String::new();
    for c in 0..32 {
        s.push(m.read((base + c) as u16) as char);
    }
    s.trim_end().to_string()
}

#[test]
fn basic_else_false_line_jump() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"1 IF 1=2 THEN 3 ELSE 5\r");
    type_keys(&mut m, &mut cpu, b"3 PRINT 8\r");
    type_keys(&mut m, &mut cpu, b"5 PRINT 1\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "1", "the untaken IF runs the ELSE branch");
}
#[test]
fn basic_else_true_skips_else() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"1 IF 1=1 THEN 3 ELSE 5\r");
    type_keys(&mut m, &mut cpu, b"3 PRINT 8\r");
    type_keys(&mut m, &mut cpu, b"5 PRINT 1\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "8", "basic_else_true_skips_else");
    assert_eq!(term_row(&m, 3), "1", "basic_else_true_skips_else");
}
#[test]
fn basic_else_untaken_no_else_skips_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"1 IF 1=2 THEN 3\r");
    type_keys(&mut m, &mut cpu, b"2 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"3 PRINT 8\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "9", "basic_else_untaken_no_else_skips_line");
    assert_eq!(term_row(&m, 3), "8", "basic_else_untaken_no_else_skips_line");
}
#[test]
fn basic_else_untaken_else_statement() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"1 IF 1=2 THEN 3 ELSE PRINT 8\r");
    type_keys(&mut m, &mut cpu, b"2 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"3 PRINT 8\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "8", "basic_else_untaken_else_statement");
    assert_eq!(term_row(&m, 3), "9", "basic_else_untaken_else_statement");
    assert_eq!(term_row(&m, 4), "8", "basic_else_untaken_else_statement");
}
