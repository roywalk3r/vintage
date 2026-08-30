//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! Cartridge banking end to end: a two-bank program where bank 0 boots and
//! bank 1 does the work, selected through the $5806 register.
//!
//! The bank switch takes effect on the very next fetch, so the instruction
//! after `sta $5806` comes from the NEW bank at the same PC — the trampoline
//! at $E005 lives in bank 1, not bank 0.

use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

/// Map one bank's segments into a full 8K image.
fn image(segments: &[(u16, Vec<u8>)]) -> [u8; 0x2000] {
    let mut img = [0u8; 0x2000];
    for &(addr, ref bytes) in segments {
        img[addr as usize - 0xE000..addr as usize - 0xE000 + bytes.len()]
            .copy_from_slice(bytes);
    }
    img
}

#[test]
fn two_bank_program_executes_across_banks() {
    let src = "
        .org $E000
boot:   lda #1
        sta $5806        ; bank 1 is visible from the very next fetch

 .bank 1
        .org $E005       ; trampoline: execution continues here, in bank 1
        jmp $F000

        .org $F000
        lda #$33
        sta $4000
hang:   jmp hang

 .bank 0
        .org $FFFA
        .word boot, boot, boot
    ";
    let bin = assemble(src).unwrap();
    assert_eq!(bin.extra_banks.len(), 1);
    let mut banks: Vec<[u8; 0x2000]> = vec![image(&bin.segments)];
    for seg in &bin.extra_banks {
        banks.push(image(seg));
    }
    let mut m = Machine::with_banks(banks);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    for _ in 0..40 {
        cpu.step(&mut m);
    }
    assert_eq!(m.read(0x4000), 0x33);
    assert_eq!(m.read(0x5806), 1);
}