//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The scratchpad editor, headless: boot the ROM, post keys, and assert on
//! the 8-line text block at $1000 (8 lines x 28 chars + NUL per line).

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
    let src = include_str!("../software/editor.s");
    let bin = assemble(src).unwrap();
    let mut m = Machine::with_banks(vec![image(&bin.segments)]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    // run past the boot render so the newest-wins one-key buffer doesn't
    // swallow the first key
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

/// The text block as 8 NUL-trimmed strings.
fn lines(m: &Machine) -> Vec<String> {
    (0..8)
        .map(|l| {
            let base = 0x1000 + l * 29;
            (0..28)
                .map(|i| m.read((base + i) as u16))
                .take_while(|&b| b != 0)
                .map(|b| b as char)
                .collect()
        })
        .collect()
}

fn cx(m: &Machine) -> u8 {
    m.read(0x12)
}

#[test]
fn editor_boots_empty_with_title_on_screen() {
    let (m, _cpu) = boot();
    assert_eq!(lines(&m), vec![""; 8]);
    // the title banner is pasted into the framebuffer
    // 'V' of the title, row 0 col 0: its top scanline byte is nonzero
    let fb = m.fb();
    assert!(fb[0] != 0, "title row should have pixels set");
}

#[test]
fn editor_types_a_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"HELLO");
    assert_eq!(lines(&m)[0], "HELLO");
}

#[test]
fn editor_arrow_left_then_insert_splices() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"AB");
    type_keys(&mut m, &mut cpu, &[0x13]); // left
    type_keys(&mut m, &mut cpu, b"X");
    assert_eq!(lines(&m)[0], "AXB");
}

#[test]
fn editor_backspace_deletes_left() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"AB");
    type_keys(&mut m, &mut cpu, &[0x08]);
    assert_eq!(lines(&m)[0], "A");
    assert_eq!(cx(&m), 1); // the cursor followed the deleted cell
    // left, then two backspaces at column 0: both no-ops
    type_keys(&mut m, &mut cpu, &[0x13, 0x08, 0x08]);
    assert_eq!(lines(&m)[0], "A");
}

#[test]
fn editor_enter_starts_next_line() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"AB");
    type_keys(&mut m, &mut cpu, &[0x0D]);
    type_keys(&mut m, &mut cpu, b"CD");
    assert_eq!(lines(&m)[0], "AB");
    assert_eq!(lines(&m)[1], "CD");
    // the cursor sat on line 1, col 2; up-arrow returns to a real cell
    type_keys(&mut m, &mut cpu, &[0x11]);
    type_keys(&mut m, &mut cpu, &[0x14, 0x14]);
    assert_eq!(m.read(0x12), 4); // cx = 4 on line 0
    assert_eq!(m.read(0x1004), 0); // text line 0 is still "AB"
    assert_eq!(lines(&m)[0], "AB");
}

#[test]
fn editor_full_line_ignores_overflow() {
    let (mut m, mut cpu) = boot();
    let full: Vec<u8> = b"0123456789ABCDEFGHIJklmnopqrst"  // 30 chars
        .to_vec();
    type_keys(&mut m, &mut cpu, &full);
    assert_eq!(lines(&m)[0], "0123456789ABCDEFGHIJklmnopqr"); // 28
    assert_eq!(bytes_of(&m, 0)[28], 0);
}

fn bytes_of(m: &Machine, l: usize) -> Vec<u8> {
    (0..29).map(|i| m.read((0x1000 + l * 29 + i) as u16)).collect()
}

#[test]
fn editor_cursor_clamps_at_column_edges() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, &[0x13, 0x13]); // left twice at col 0
    assert_eq!(cx(&m), 0);
    type_keys(&mut m, &mut cpu, &[0x14]); // right one
    assert_eq!(cx(&m), 1);
}

#[test]
fn editor_down_arrow_moves_lines_keeps_column() {
    let (mut m, mut cpu) = boot();
    type_keys(&mut m, &mut cpu, b"AB");
    type_keys(&mut m, &mut cpu, &[0x12]); // down
    assert_eq!(m.read(0x13), 1); // CY
    assert_eq!(m.read(0x12), 2); // CX kept
    type_keys(&mut m, &mut cpu, b"XY");
    // the gap to the kept column is space-padded so the line stays
    // NUL-terminated and renderable
    assert_eq!(lines(&m)[1], "  XY");
}