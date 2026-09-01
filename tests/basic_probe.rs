//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The BASIC app, headless: boot the ROM, type program lines and direct
//! commands through the one-key buffer, and assert on the scrolling
//! terminal's ASCII mirror at $2500 (8 rows x 33 bytes per row).

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
    // let the ROM reach the keyboard poll before the first key is posted,
    // or the newest-wins one-key buffer drops it
    for _ in 0..4 {
        m.run_frame(&mut cpu);
    }
    (m, cpu)
}

// One key per frame: the poll loop reads $5800 continuously, so any key
// posted mid-frame is caught; hsubmit's full processing shares the frame.
fn type_keys(m: &mut Machine, cpu: &mut Cpu, keys: &[u8]) {
    for &k in keys {
        m.key(k);
        m.run_frame(cpu);
    }
}

// TERM mirrors every printed row as the 32 cell bytes at $2500 + r*33;
// a fresh row is spaces, so trim to the text.
fn term_row(m: &Machine, r: usize) -> String {
    let base = 0x2500 + r * 33;
    let mut s = String::new();
    for c in 0..32 {
        s.push(m.read((base + c) as u16) as char);
    }
    s.trim_end().to_string()
}

fn input_row(m: &Machine) -> String {
    let mut s = String::new();
    for c in 0..32 {
        s.push(m.read(0x2600 + c as u16) as char);
    }
    s.trim_end().to_string()
}

#[test]
fn basic_if_equal_taken_and_less_false() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 5=5 GOTO 30\r20 PRINT 8\r30 PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "7");
}

#[test]
fn basic_if_greater_false_advances() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 10>20 GOTO 30\r20 PRINT 8\r30 PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "8");
    assert_eq!(term_row(&m, 3), "7");
}

#[test]
fn basic_boot_shows_banner_ready() {
    let (m, _cpu) = boot();
    assert_eq!(term_row(&m, 0), "VINTAGE-1 BASIC");
    assert_eq!(term_row(&m, 1), "READY");
    assert_eq!(input_row(&m), "?");
}

#[test]
fn basic_print_expression_precedence() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT 2+3*4\r");
    // */ must bind tighter than +-: 2+(3*4) = 14, not (2+3)*4 = 20
    assert_eq!(term_row(&m, 2), "14");
}

#[test]
fn basic_let_and_print_var() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"LET A=7\r");
    type_keys(&mut m, &mut cpu, b"PRINT A\r");
    assert_eq!(term_row(&m, 2), "7");
}

#[test]
fn basic_program_run_and_list() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A=1\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT A\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "1");
    type_keys(&mut m, &mut cpu, b"LIST\r");
    assert_eq!(term_row(&m, 3), "10 LET A=1");
    assert_eq!(term_row(&m, 4), "20 PRINT A");
}

#[test]
fn basic_program_run_order() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT 1\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 2\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "1");
    assert_eq!(term_row(&m, 3), "2");
}

#[test]
fn basic_goto_missing_line_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 GOTO 999\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "ERR");
}

#[test]
fn basic_if_taken_skips_to_target() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1<2 GOTO 30\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "7");
}

#[test]
fn basic_if_false_advances() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 5<1 GOTO 30\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "9");
    assert_eq!(term_row(&m, 3), "7");
}

#[test]
fn basic_backspace_fixes_typed_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT 6");
    assert_eq!(input_row(&m), "? PRINT 6");
    type_keys(&mut m, &mut cpu, b"\x085\r");
    assert_eq!(term_row(&m, 2), "5");
}