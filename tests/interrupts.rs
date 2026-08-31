// VINTAGE-1
// Author: roywalk3r
// Repo: https://github.com/roywalk3r/vintage
// License: MIT
//! Vsync IRQ: one maskable interrupt per frame, vectored through $FFFE.

use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

fn machine(src: &str) -> Machine {
    let bin = assemble(src).unwrap();
    let mut rom = [0u8; 0x2000];
    for (addr, bytes) in &bin.segments {
        let base = *addr as usize - 0xE000;
        rom[base..base + bytes.len()].copy_from_slice(bytes);
    }
    Machine::new(rom)
}

#[test]
fn cli_program_sees_one_vsync_irq_per_frame() {
    let src = "
            .org $E000
    entry:  cli
    loop:   jmp loop

            .org $F000
    handler:
            inc $40
            rti

         .org $FFFA
            .word handler, entry, handler
        ";
    let mut m = machine(src);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    m.run_frame(&mut cpu);
    m.run_frame(&mut cpu);
    assert_eq!(m.read(0x40), 2, "one vsync IRQ per frame once cli'd");
}

#[test]
fn irq_is_masked_while_i_flag_set() {
    let src = "
            .org $E000
    entry:  sei
    loop:   jmp loop
            .org $F000
    handler:
            inc $40
            rti
         .org $FFFA
            .word handler, entry, handler
        ";
    let mut m = machine(src);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    m.run_frame(&mut cpu);
    m.run_frame(&mut cpu);
    assert_eq!(m.read(0x40), 0, "sei must mask the vsync IRQ");
}
