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
fn basic_if_then_line_taken() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1=1 THEN 30\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "7");
    assert_ne!(term_row(&m, 3), "9");
}

#[test]
fn basic_if_then_line_not_taken() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1=2 THEN 30\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "9");
    assert_eq!(term_row(&m, 3), "7");
}

#[test]
fn basic_if_then_inline_print_advances() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1=1 THEN PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "7");
    assert_eq!(term_row(&m, 3), "9");
}

#[test]
fn basic_if_then_inline_goto_repositions() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1=1 THEN GOTO 30\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "7");
}

#[test]
fn basic_if_then_inline_skipped_when_false() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1=2 THEN PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "9");
}

#[test]
fn basic_if_then_string_cond_inline() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A$=\"X\"\r");
    type_keys(&mut m, &mut cpu, b"20 IF A$=\"X\" THEN PRINT 3\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT 4\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "3");
    assert_eq!(term_row(&m, 3), "4");
}

#[test]
fn basic_if_then_missing_line_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1=1 THEN 99\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "ERR");
}

#[test]
fn basic_if_then_unknown_statement_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1=1 THEN A\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "ERR");
}

#[test]
fn basic_direct_if_then_print_runs_inline() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"IF 1=1 THEN PRINT 5\r");
    let hit = (0..8).any(|r| term_row(&m, r) == "5");
    assert!(hit, "direct THEN PRINT must execute immediately");
    type_keys(&mut m, &mut cpu, b"PRINT 6\r");
    let hit6 = (0..8).any(|r| term_row(&m, r) == "6");
    assert!(hit6, "prompt must recover after a direct THEN PRINT");
}

#[test]
fn basic_direct_if_then_line_jumps_into_run() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT 7\r");
    type_keys(&mut m, &mut cpu, b"IF 1=1 THEN 20\r");
    let hit7 = (0..8).any(|r| term_row(&m, r) == "7");
    assert!(hit7, "direct THEN <line> must start a run at that line");
    assert!(!(0..8).any(|r| term_row(&m, r) == "9"), "line 10 must not run: jump starts at 20");
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

// A direct command must retire the input line (hcln) even when its handler
// returns through a bare rts (LIST/END/direct-IF/RUN-with-no-program used to
// unwind past hcln, leaving the submitted text in IBUF so the next typed
// line concatenated with it).
#[test]
fn basic_direct_list_retires_the_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT 1\r");
    type_keys(&mut m, &mut cpu, b"LIST\r");
    assert_eq!(m.read(0x12), 0, "IBLEN must clear after LIST");
    assert_eq!(input_row(&m), "?");
    type_keys(&mut m, &mut cpu, b"PRINT 2\r");
    let hit = (0..8).any(|r| term_row(&m, r) == "2");
    assert!(hit, "follow-up PRINT must run cleanly after LIST");
}

#[test]
fn basic_direct_end_retires_the_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"END\r");
    assert_eq!(m.read(0x12), 0, "IBLEN must clear after END");
    type_keys(&mut m, &mut cpu, b"PRINT 3\r");
    let hit = (0..8).any(|r| term_row(&m, r) == "3");
    assert!(hit, "follow-up PRINT must run cleanly after END");
}

#[test]
fn basic_direct_run_empty_program_retires_the_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(m.read(0x12), 0, "IBLEN must clear after RUN with no program");
    type_keys(&mut m, &mut cpu, b"PRINT 4\r");
    let hit = (0..8).any(|r| term_row(&m, r) == "4");
    assert!(hit, "follow-up PRINT must run cleanly after an empty RUN");
}

#[test]
fn basic_direct_if_taken_retires_the_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"IF 1=1 GOTO 10\r");
    assert_eq!(m.read(0x12), 0, "IBLEN must clear after a taken direct IF");
    type_keys(&mut m, &mut cpu, b"PRINT 5\r");
    let hit = (0..8).any(|r| term_row(&m, r) == "5");
    assert!(hit, "follow-up PRINT must run cleanly after a direct IF");
}

// Strings live at STRV = $1200: 26 slots of 8 bytes (7 chars + NUL), slot
// for A$ first, so slot(c) reads the C-string at $1200 + 8*(c-'A').
fn slot(m: &Machine, c: u8) -> String {
    let base = 0x1200 + 8 * (c - b'A') as u16;
    (0..8)
        .map(|i| m.read((base + i) as u16))
        .take_while(|&b| b != 0)
        .map(|b| b as char)
        .collect()
}

#[test]
fn basic_let_string_assigns_slot() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A$=\"HI\"\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(slot(&m, b'A'), "HI");
    assert_eq!(slot(&m, b'B'), "", "unassigned slots must stay empty");
}

#[test]
fn basic_direct_print_string() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT \"HI\"\r");
    assert_eq!(term_row(&m, 2), "HI");
}

#[test]
fn basic_print_mixed_semicolon_list() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A=7\r20 LET B$=\"X\"\r30 PRINT \"P\";B$;A\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "PX7");
}

#[test]
fn basic_concat_appends() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A$=\"AB\"\r20 LET A$=A$+\"CD\"\r30 PRINT A$\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "ABCD");
}

#[test]
fn basic_len_counts_chars() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A$=\"AB\"\r20 PRINT LEN(A$)\r30 PRINT LEN(B$)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "2");
    assert_eq!(term_row(&m, 3), "0", "LEN of an empty string must be 0");
}

#[test]
fn basic_input_string_assigns_slot() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 INPUT A$\r20 PRINT A$\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    type_keys(&mut m, &mut cpu, b"HELLO\r");
    // the assignment and the PRINT settle over a few frames
    for _ in 0..4 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "HELLO");
    assert_eq!(slot(&m, b'A'), "HELLO");
}

#[test]
fn basic_string_truncates_at_seven() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A$=\"12345678\"\r20 LET A$=A$+\"9\"\r30 PRINT A$\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(
        term_row(&m, 2),
        "1234567",
        "literals and concats cap at 7 chars"
    );
    assert_eq!(slot(&m, b'A'), "1234567");
}

#[test]
fn basic_unclosed_quote_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT \"AB\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "ERR");
}

// IF accepts string conditions: both sides are full string exprs compared
// with = < >, first differing char decides, lengths decide at prefix-equal.
#[test]
fn basic_if_string_equal_taken() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A$=\"HI\"\r20 IF A$=\"HI\" GOTO 40\r30 PRINT 0\r40 PRINT 1\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "1");
}

#[test]
fn basic_if_string_not_equal_advances() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A$=\"HI\"\r20 IF A$=\"HO\" GOTO 40\r30 PRINT 8\r40 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "8");
    assert_eq!(term_row(&m, 3), "9");
}

#[test]
fn basic_if_string_prefix_decides() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF \"A\"<\"AB\" GOTO 40\r20 PRINT 0\r30 PRINT 1\r40 PRINT 2\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "2", "\"A\" < \"AB\": taken, so only line 40 prints");
}

#[test]
fn basic_if_string_empty_equals_empty() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF B$=\"\" GOTO 30\r20 PRINT 8\r30 PRINT 1\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "1", "two empty strings compare equal");
}

#[test]
fn basic_if_string_direct_taken_retires() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"IF B$=\"\" GOTO 10\r");
    assert_eq!(m.read(0x12), 0, "IBLEN must clear after a taken direct string IF");
    type_keys(&mut m, &mut cpu, b"PRINT 5\r");
    let hit = (0..8).any(|r| term_row(&m, r) == "5");
    assert!(hit, "follow-up PRINT must run cleanly after a direct string IF");
}

#[test]
fn basic_gosub_return_roundtrip() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 GOSUB 100\r20 PRINT 1\r100 PRINT 2\r110 RETURN\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "2", "GOSUB runs the callee first");
    assert_eq!(term_row(&m, 3), "1", "RETURN resumes after the GOSUB line");
}

#[test]
fn basic_gosub_nested() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 GOSUB 100\r20 PRINT 1\r30 END\r100 GOSUB 150\r110 PRINT 2\r120 RETURN\r150 PRINT 3\r160 RETURN\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    // the nesting spans frames; let the run settle before asserting
    for _ in 0..4 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "3");
    assert_eq!(term_row(&m, 3), "2");
    assert_eq!(term_row(&m, 4), "1");
}

#[test]
fn basic_return_without_gosub_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 RETURN\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "ERR");
}

#[test]
fn basic_gosub_caller_for_survives_return() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 FOR I=1 TO 3\r20 GOSUB 50\r30 NEXT I\r40 PRINT A\r45 END\r50 LET A=A+1\r60 FOR J=1 TO 5\r70 RETURN\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..4 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "3", "RETURN restores the caller's FOR depth");
}

#[test]
fn basic_direct_return_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"RETURN\r");
    assert_eq!(term_row(&m, 2), "ERR", "direct RETURN = RETURN without GOSUB");
}

#[test]
fn basic_direct_gosub_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"GOSUB 100\r");
    assert_eq!(term_row(&m, 2), "ERR", "direct GOSUB has no return stack");
}

#[test]
fn basic_data_read_fills_variables() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DATA 42,7\r20 READ A,B\r30 PRINT A;B\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "427", "A=42 then B=7 on one shared row");
}

#[test]
fn basic_read_spans_multiple_data_lines() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DATA 1\r20 DATA 2\r30 READ A,B\r40 PRINT A;B\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "12", "READ walks the data cursor across DATA lines");
}

#[test]
fn basic_read_strings_quoted_and_bare() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DATA HI,\"8 BIT\"\r20 READ A$,B$\r30 PRINT A$;B$\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "HI8 BIT", "bare word and quoted literal both read");
}

#[test]
fn basic_read_negative_number() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DATA -3\r20 READ A\r30 PRINT A\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "65533", "-3 mod 65536, like every other op");
}

#[test]
fn basic_restore_rewinds_data() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DATA 1,2\r20 READ A\r30 RESTORE\r40 READ B\r50 PRINT A;B\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "11", "RESTORE rewinds: B re-reads the first item");
}

#[test]
fn basic_run_resets_data_cursor() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DATA 9\r20 READ A\r30 PRINT A\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "9");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 3), "9", "second RUN re-reads from the top, not ERR");
}

#[test]
fn basic_data_lines_never_execute() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DATA 1,2\r20 READ A\r30 PRINT A\rRUN\r");
    // one submission: typing RUN as part of the same frame batch would be
    // fine either way, but keep the run on its own frame
    type_keys(&mut m, &mut cpu, b"RUN\r");
    assert_eq!(term_row(&m, 2), "1", "DATA as a statement is an inert no-op");
}

#[test]
fn basic_read_past_data_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 READ A\rRUN\r");
    assert_eq!(term_row(&m, 2), "ERR", "no DATA anywhere: out of data");
}

// PRINT's item list: tputc owns y, so pitem's string copy must index SSCR
// with x — indexing with y compared the source position against TBLEN and
// dropped chars from every second-and-later string item.
#[test]
fn basic_print_second_string_item_keeps_spaces() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT \"AB\";\"CD\"\r");
    assert_eq!(term_row(&m, 2), "ABCD");
    type_keys(&mut m, &mut cpu, b"PRINT \"8 BIT\";\"OK\"\r");
    assert_eq!(term_row(&m, 3), "8 BITOK");
}

#[test]
fn basic_print_numeric_then_numeric() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT 42;7\r");
    assert_eq!(term_row(&m, 2), "427", "nump parks the parse index before appending digits");
}

#[test]
fn basic_read_type_mismatch_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DATA HI\r20 READ A\rRUN\r");
    assert_eq!(term_row(&m, 2), "ERR", "a bare word is not a numeric literal");
}

// PLOT writes the fb bit at $4000 + y*32 + x/8, mask $80 >> (x&7) — MSB
// leftmost, 32 bytes per scanline. Row 100 keeps clear of the terminal's
// glyph rows. Direct mode plots immediately; a program loop fills bytes.
#[test]
fn basic_plot_direct_sets_fb_bit() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PLOT 0,100\r");
    assert_eq!(m.fb()[100 * 32], 0x80, "(0,100) is the MSB of the byte");
    type_keys(&mut m, &mut cpu, b"PLOT 8,100\r");
    assert_eq!(m.fb()[100 * 32 + 1], 0x80, "x=8 lands in the next byte");
    type_keys(&mut m, &mut cpu, b"UNPLOT 0,100\r");
    assert_eq!(m.fb()[100 * 32], 0x00, "UNPLOT clears the same bit");
}

#[test]
fn basic_plot_expression_coords() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PLOT 3+5,100\r");
    assert_eq!(m.fb()[100 * 32 + 1], 0x80, "x=8: second byte, bit 7");
    type_keys(&mut m, &mut cpu, b"PLOT 8/2,100\r");
    assert_eq!(m.fb()[100 * 32], 0x08, "x=4: first byte, bit $80>>4");
}

#[test]
fn basic_plot_out_of_range_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PLOT 0,192\r");
    assert_eq!(term_row(&m, 2), "ERR", "y tops out at 191");
    let (mut m2, mut cpu2) = boot();
    type_keys(&mut m2, &mut cpu2, b"PLOT 256,0\r");
    assert_eq!(term_row(&m2, 2), "ERR", "x must fit one byte");
}

#[test]
fn basic_plot_program_row() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 FOR I=0 TO 7\r20 PLOT I,100\r30 NEXT I\r40 PRINT \"DONE\"\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(m.fb()[100 * 32], 0xFF, "PLOT I,100 for I=0..7 fills the byte");
    assert_eq!(term_row(&m, 2), "DONE", "the loop completed without aborting");
}
#[test]
fn basic_colon_print_chain() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT \"A\":PRINT \"B\"\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "A", "the slot's first statement prints");
    assert_eq!(term_row(&m, 3), "B", "':' resumes at the next statement");
}

#[test]
fn basic_colon_for_next_same_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 FOR I=1TO 3:PRINT I:NEXT\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "1", "body prints before NEXT on the first pass");
    assert_eq!(term_row(&m, 3), "2");
    assert_eq!(term_row(&m, 4), "3", "body re-runs mid-slot via the resume vector");
}

#[test]
fn basic_colon_loop_exit_continues_chain() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 FOR I=1TO 2:NEXT:PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "9", "after NEXT exhausts, the ':' chain continues");
}

#[test]
fn basic_colon_gosub_mid_slot_resume() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 GOSUB 100:PRINT \"R\"\r");
    type_keys(&mut m, &mut cpu, b"100 PRINT \"S\"\r");
    type_keys(&mut m, &mut cpu, b"110 RETURN\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "S", "the subroutine runs first");
    assert_eq!(term_row(&m, 3), "R", "RETURN resumes at the ':' continuation, not the next slot");
}

#[test]
fn basic_colon_then_chain_taken() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1=1THEN PRINT7:PRINT8\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "7", "the THEN body runs");
    assert_eq!(term_row(&m, 3), "8", "the chain after the THEN body runs too");
}

#[test]
fn basic_colon_if_untaken_skips_rest() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 IF 1=2THEN PRINT7:PRINT8\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "", "an untaken IF skips the whole slot");
}

#[test]
fn basic_colon_goto_repositions_out_of_chain() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT \"A\":GOTO 30\r");
    type_keys(&mut m, &mut cpu, b"20 PRINT \"B\"\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT \"C\"\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "A");
    assert_eq!(term_row(&m, 3), "C", "GOTO repositioned CPTR out of the chain");
}

#[test]
fn basic_colon_empty_body_loop_terminates() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 FOR I=1TO 2:NEXT:PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "9", "an empty loop body terminates instead of hanging");
}

#[test]
fn basic_colon_nested_inner_same_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 FOR I=1 TO 2\r");
    type_keys(&mut m, &mut cpu, b"20 FOR J=1TO 2:PRINT J:NEXT\r");
    type_keys(&mut m, &mut cpu, b"30 NEXT I\r");
    type_keys(&mut m, &mut cpu, b"40 PRINT \"END\"\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "1", "outer pass 1, inner pass 1");
    assert_eq!(term_row(&m, 3), "2", "outer pass 1, inner pass 2");
    assert_eq!(term_row(&m, 4), "1", "outer pass 2 re-runs the slot");
    assert_eq!(term_row(&m, 5), "2", "outer pass 2, inner pass 2");
    assert_eq!(term_row(&m, 6), "END", "outer NEXT I exits cleanly from a mid-slot resume");
}

#[test]
fn basic_colon_trailing_is_harmless() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT \"X\":\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..6 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "X", "trailing ':' is consumed, then the NUL ends the line");
}