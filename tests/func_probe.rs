//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The BASIC app, headless: boot the ROM, type program lines and direct
//! commands through the one-key buffer, and assert on the scrolling
//! terminal's ASCII mirror at $2500 (8 rows x 33 bytes per row). This
//! suite covers the 26 fixed 11-element arrays: DIM validation, element
//! reads as expression factors, LET element stores, chained statements,
//! and direct-mode DIM.

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
fn basic_func_abs_direct() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT ABS(-7)\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "7", "ABS of a negative literal");
}
#[test]
fn basic_func_abs_expr() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT ABS(3-9)\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "6", "ABS folds the inner expression first");
}
#[test]
fn basic_func_sgn_zero() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT SGN(0)\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "0", "basic_func_sgn_zero");
}
#[test]
fn basic_func_sgn_pos() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT SGN(9)\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "1", "basic_func_sgn_pos");
}
#[test]
fn basic_func_sgn_neg_wrap() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT SGN(-4)+1\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "0", "SGN(-4)=$FFFF; +1 wraps to 0");
}
#[test]
fn basic_func_asc_literal() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT ASC(\"A\")\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "65", "basic_func_asc_literal");
}
#[test]
fn basic_func_asc_empty() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT ASC(\"\")\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "0", "empty string composes to 0");
}
#[test]
fn basic_func_asc_var() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A$=\"AB\"\r");
    type_keys(&mut m, &mut cpu, b"PRINT ASC(A$)\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "65", "basic_func_asc_var");
}
#[test]
fn basic_func_val_digits() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT VAL(\"123\")\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "123", "basic_func_val_digits");
}
#[test]
fn basic_func_val_negative() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT VAL(\"-2\")-65534\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "0", "VAL folds then negates: $FFFE - 65534 = 0");
}
#[test]
fn basic_func_val_empty() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT VAL(\"\")\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "0", "basic_func_val_empty");
}
#[test]
fn basic_func_val_stops_at_letter() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT VAL(\"12A3\")\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "12", "the walk stops at the first non-digit");
}
#[test]
fn basic_func_val_leading_zero() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT VAL(\"007\")\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "7", "basic_func_val_leading_zero");
}
#[test]
fn basic_func_val_from_var() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET B$=\"5\"\r");
    type_keys(&mut m, &mut cpu, b"PRINT VAL(B$)\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "5", "basic_func_val_from_var");
}
#[test]
fn basic_func_val_in_if() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A$=\"17\"\r");
    type_keys(&mut m, &mut cpu, b"1 IF VAL(A$)>5 THEN 3 ELSE 5\r");
    type_keys(&mut m, &mut cpu, b"3 PRINT 4\r");
    type_keys(&mut m, &mut cpu, b"5 PRINT 8\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "4", "VAL folds inside an IF condition");
}
#[test]
fn basic_func_dispatch_falls_through() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET VB=5\r");
    type_keys(&mut m, &mut cpu, b"PRINT VB\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "5", "the (V,B) pair is not a keyword: plain variable");
}
#[test]
fn basic_func_mixed_row() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"PRINT LEN(\"AB\")+ABS(2)\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "4", "new factors coexist with LEN in one item");
}
