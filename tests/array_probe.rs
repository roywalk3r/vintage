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
fn basic_array_dim_read_write() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DIM A(9)\r");
    type_keys(&mut m, &mut cpu, b"20 LET A(3)=7\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT A(3)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "7", "LET stored the element, PRINT read it");
}
#[test]
fn basic_array_undimensioned_read_is_zero() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 PRINT A(2)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "0", "an undimensioned read returns 0");
}
#[test]
fn basic_array_dim_over_10_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DIM A(11)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "ERR", "DIM size > 10 is ERR");
}
#[test]
fn basic_array_index_over_10_errors() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A(11)=1\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "ERR", "an element index > 10 is ERR");
}
#[test]
fn basic_array_comma_list_dim() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DIM A(9),B(8)\r");
    type_keys(&mut m, &mut cpu, b"20 LET B(5)=4:PRINT B(5)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "4", "DIM's comma list continues the ':' chain");
}
#[test]
fn basic_array_index_expr() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A(4)=7:PRINT A(2*2)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "7", "the index is a full expression");
}
#[test]
fn basic_array_direct_dim() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"DIM B(7)\r");
    type_keys(&mut m, &mut cpu, b"PRINT B(5)\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "0", "direct DIM runs immediately, reads stay 0");
}
#[test]
fn basic_array_redim_keeps_values() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DIM A(5)\r");
    type_keys(&mut m, &mut cpu, b"20 LET A(0)=9\r");
    type_keys(&mut m, &mut cpu, b"30 DIM A(10):PRINT A(0)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "9", "re-DIM is harmless, values survive");
}
#[test]
fn basic_array_chained_statements_on_one_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 DIM A(9)\r");
    type_keys(&mut m, &mut cpu, b"20 LET A(2)=5:PRINT A(2)\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT A(3)\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "5", "the DIM/LET/PRINT chain runs in order");
    assert_eq!(term_row(&m, 3), "0", "unwritten elements read 0");
}
#[test]
fn basic_array_element_in_expr_and_if() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"10 LET A(0)=2:PRINT A(0)+1\r");
    type_keys(&mut m, &mut cpu, b"20 IF A(1)>2 THEN 40\r");
    type_keys(&mut m, &mut cpu, b"30 PRINT 0\r");
    type_keys(&mut m, &mut cpu, b"40 PRINT 9\r");
    type_keys(&mut m, &mut cpu, b"RUN\r");
    for _ in 0..8 {
        m.run_frame(&mut cpu);
    }
    assert_eq!(term_row(&m, 2), "3", "array elements fold like any factor");
    assert_eq!(term_row(&m, 3), "0", "untaken IF falls through");
    assert_eq!(term_row(&m, 4), "9", "THEN <line> repositioned");
}
