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
fn dbg_mem_dump() {
    let cases: Vec<(&str, Vec<&str>)> = vec![
        ("STORE", vec!["10 LET A(3)=7\r", "RUN\r"]),
        ("CHAIN", vec!["10 DIM A(9):LET A(2)=5:PRINT A(2)\r", "RUN\r"]),
        ("REDIM", vec!["10 DIM A(5):LET A(0)=9:DIM A(10):PRINT A(0)\r", "RUN\r"]),
        ("ELEMIF", vec!["10 LET A(0)=2:LET A(1)=3:IF A(1)>A(0) THEN PRINT A(0)+A(1)\r", "RUN\r"]),
    ];
    for (label, lines) in cases {
        let (mut m, mut cpu) = boot();
        for line in lines {
            type_keys(&mut m, &mut cpu, line.as_bytes());
        }
        for _ in 0..8 {
            m.run_frame(&mut cpu);
        }
        println!("=== {} ===", label);
        for r in 2..5 {
            println!("  row{}: |{}|", r, term_row(&m, r));
        }
        let mut s = String::new();
        for a in 0x1300..0x1310 {
            s.push_str(&format!("{:02X} ", m.read(a)));
        }
        println!("  ARY $1300-$130F: {}", s);
        let mut s2 = String::new();
        for a in 0x2000..0x2040 {
            s2.push_str(&format!("{:02X} ", m.read(a)));
        }
        println!("  SLOT0: {}", s2);
        let mut s3 = String::from("  VARS: ");
        for a in 0x1100..0x1108 {
            s3.push_str(&format!("{:02X} ", m.read(a)));
        }
        println!("  {}", s2);
        println!("  {}", s3);
    }
}
