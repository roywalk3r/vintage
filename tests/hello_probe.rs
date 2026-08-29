//! Hello banner probes: the redesigned banner must place the title on
//! screen row 6 with NOTHING lit above it, and the subtitle on row 10.

use std::fs;
use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

fn hello_rom() -> [u8; 0x2000] {
    let src = fs::read_to_string("software/hello.s").unwrap();
    let bin = assemble(&src).expect("assemble hello.s");
    let mut rom = [0u8; 0x2000];
    for (addr, bytes) in bin.segments.clone() {
        let base = addr as usize - 0xE000;
        rom[base..base + bytes.len()].copy_from_slice(&bytes);
    }
    rom
}

/// A banner whose rows collapsed toward the top of the screen (all four
/// messages overlapping in rows 0-4) is broken; the title lives on row 6.
#[test]
fn banner_starts_at_row_6_nothing_above() {
    let rom = hello_rom();
    let mut m = Machine::new(rom);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    for _ in 0..3 {
        m.run_frame(&mut cpu);
    }
    let fb = m.fb();
    let lit_rows: Vec<usize> = (0..24)
        .filter(|&r| fb[r * 32 * 8..(r + 1) * 32 * 8].iter().any(|&b| b != 0))
        .collect();
    let first = *lit_rows.first().expect("banner drew nothing");
    assert!(
        first >= 6,
        "text collapsed to row {first}: lit rows {lit_rows:?}"
    );
    assert!(
        lit_rows.contains(&6),
        "row 6 (title) empty: lit rows {lit_rows:?}"
    );
}