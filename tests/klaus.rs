//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The correctness gate: Klaus Dormann's 6502 functional test must reach its
//! success trap before anything is built on this CPU.
//!
//! Binary is fetched by `scripts/fetch-klaus.sh` into `.cache/`.

use vintage::cpu::{Bus, Cpu};

struct FlatRam([u8; 0x10000]);

impl Bus for FlatRam {
    fn read(&self, addr: u16) -> u8 {
        self.0[addr as usize]
    }
    fn write(&mut self, addr: u16, val: u8) {
        self.0[addr as usize] = val;
    }
}

const TEST_BASE: u16 = 0x0400;
// The test signals success by trapping in a tight jump-to-self loop. The .bin
// is a full 64KB memory image (verified: $3469 is `JMP $3469` in it; the
// $04xx/$07xx/$37xx self-jmps are per-test error handlers).
const SUCCESS_PC: u16 = 0x3469;

fn is_self_jmp(ram: &FlatRam, pc: u16) -> bool {
    let lo = ram.read(pc.wrapping_add(1));
    let hi = ram.read(pc.wrapping_add(2));
    ram.read(pc) == 0x4C && u16::from_le_bytes([lo, hi]) == pc
}

#[test]
fn klaus_functional_test_reaches_success_trap() {
    let bin = match std::fs::read(".cache/6502_functional_test.bin") {
        Ok(b) => b,
        Err(e) => panic!("Klaus binary missing ({e}) — run scripts/fetch-klaus.sh"),
    };

    assert_eq!(bin.len(), 0x10000, "expected a full 64KB image");
    let mut ram = FlatRam([0; 0x10000]);
    ram.0.copy_from_slice(&bin);

    let mut cpu = Cpu::new();
    cpu.pc = TEST_BASE;

    let mut instructions = 0u64;
    loop {
        cpu.step(&mut ram);
        instructions += 1;
        if is_self_jmp(&ram, cpu.pc) {
            break;
        }
        assert!(
            instructions < 100_000_000,
            "no trap after 100M instructions — the CPU is lost, not the test"
        );
    }

    assert_eq!(
        cpu.pc, SUCCESS_PC,
        "trapped at a failure address, not the success trap"
    );
}