//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The calculator app, headless: boot the ROM, post keys into the one-key
//! buffer, and assert on the ASCII display field the app mirrors at $2010.

use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

fn image(segments: &[(u16, Vec<u8>)]) -> [u8; 0x2000] {
    let mut img = [0u8; 0x2000];
    for &(addr, ref bytes) in segments {
        img[addr as usize - 0xE000..addr as usize - 0xE000 + bytes.len()]
            .copy_from_slice(bytes);
    }
    img
}

fn boot() -> (Machine, Cpu) {
    let src = include_str!("../software/calc.s");
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

// Type a sequence of keys, one machine frame per key (the poll loop reads
// $5800 continuously, so any key posted mid-frame is caught).
fn type_keys(m: &mut Machine, cpu: &mut Cpu, keys: &[u8]) {
    for &k in keys {
        m.key(k);
        m.run_frame(cpu);
    }
}

fn field(m: &Machine) -> String {
    let mut s = String::new();
    for a in 0x2010..0x201F {
        let c = m.read(a) as char;
        if c.is_ascii_graphic() || c == ' ' {
            s.push(c);
        } else {
            s.push('?');
        }
    }
    s
}

fn show(n: u16) -> String {
    // right-aligned like the app's 14-cell number field
    format!("{:>14}", n.to_string())
}

#[test]
fn calc_boot_draws_zero() {
    let (m, _cpu) = boot();
    assert_eq!(field(&m), format!(" {}", show(0)));
}

#[test]
fn calc_adds_committed_operands() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"12+");
    // mid-expression: the pending op shows in the op cell
    assert_eq!(field(&m), format!("+{}", show(12)));
    type_keys(&mut m, &mut cpu, b"30=");
    assert_eq!(field(&m), format!(" {}", show(42)));
}
#[test]
fn calc_subtracts() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"50-8=");
    assert_eq!(field(&m), format!(" {}", show(42)));
}

#[test]
fn calc_multiplies() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"12*25=");
    assert_eq!(field(&m), format!(" {}", show(300)));
}

#[test]
fn calc_divides() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"100/7=");
    assert_eq!(field(&m), format!(" {}", show(14)));
}

#[test]
fn calc_err_on_div_by_zero() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"5/0=");
    assert_eq!(&field(&m)[12..], "ERR");
}

#[test]
fn calc_backspace_edits_entry() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"123\x08");
    assert_eq!(field(&m), format!(" {}", show(12)));
}
