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

#[test]
fn basic_for_next_sum() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET S=0\r20 FOR I=1 TO 5\r30 LET S=S+I\r40 NEXT I\r50 PRINT S\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "15");
}

#[test]
fn basic_for_next_negative_step() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 FOR I=10 TO 1 STEP -3\r20 PRINT I\r30 NEXT I\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    // the loop spans several frames; let the run settle before asserting
    for _ in 0..4 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "10");
    assert_eq!(term_row(&m, 3), "7");
    assert_eq!(term_row(&m, 4), "4");
    assert_eq!(term_row(&m, 5), "1");
}

#[test]
fn basic_for_next_nested() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET S=0\r20 FOR I=1 TO 2\r30 FOR J=1 TO 3\r40 LET S=S+1\r50 NEXT J\r60 NEXT I\r70 PRINT S\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "6");
}

#[test]
fn basic_next_without_for_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 NEXT\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "ERR");
}

#[test]
fn basic_rnd_poke_collect() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 FOR I=0 TO 19\r20 POKE 4608+I, RND\r30 NEXT I\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    let mut vals: Vec<u8> = Vec::new();
    for i in 0..20u16 {
        vals.push(m.read(0x1200 + i));
    }
    let mut uniq = vals.clone();
    uniq.sort();
    uniq.dedup();
    assert!(uniq.len() >= 4, "20 RND reads must vary, got {vals:?}");
}

#[test]
fn basic_poke_peek_roundtrip() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 POKE 4608,42\r20 PRINT PEEK(4608)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "42");
}

#[test]
fn basic_poke_peek_framebuffer() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 POKE 16416,255\r20 PRINT PEEK(16416)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "255");
    assert_eq!(m.fb()[32], 255, "POKE $4020 must land in the framebuffer");
}

#[test]
fn basic_input_assigns_var() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 INPUT A\r20 PRINT A\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    type_keys(&mut m, &mut cpu, b"42\r");
    // the assignment and the PRINT settle over a few frames
    for _ in 0..4 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "42");
}

#[test]
fn basic_parens_and_unary_minus() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT (2+3)*4\r");
    assert_eq!(term_row(&m, 2), "20");
    type_keys(&mut m, &mut cpu, b"PRINT -5\r");
    assert_eq!(term_row(&m, 3), "65531");
    type_keys(&mut m, &mut cpu, b"PRINT 2*-3\r");
    assert_eq!(term_row(&m, 4), "65530");
}
