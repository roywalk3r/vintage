// VINTAGE-1
// Author: roywalk3r
// Repo: https://github.com/roywalk3r/vintage
// License: MIT
//! .vst save states: full machine + CPU serialization. The cartridge banks
//! ride along in the image, so a .vst restores a resumable machine with no
//! companion ROM needed.

use vintage::asm::assemble;
use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

/// Map one bank's segments into a full 8K image.
fn image(segments: &[(u16, Vec<u8>)]) -> [u8; 0x2000] {
    let mut img = [0u8; 0x2000];
    for &(addr, ref bytes) in segments {
        let at = addr as usize - 0xE000;
        img[at..at + bytes.len()].copy_from_slice(bytes);
    }
    img
}

/// Counter program: bumps $20 on every pass, forever.
const COUNTER: &str = "
        .org $E100
entry:  inc $20
        jmp entry

        .org $FFFC
        .word entry
";

#[test]
fn save_then_restore_resumes_to_identical_continuation() {
    let bin = assemble(COUNTER).unwrap();
    let mut m = Machine::new(image(&bin.segments));
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    // Warm up, then freeze. Both machines must share this exact past.
    for _ in 0..2 {
        m.run_frame(&mut cpu);
    }
    let snap = m.save_state(&cpu);
    let frozen = m.read(0x0020);
    let frames = m.read(0x5802);

    // Side A: five frames of divergence.
    for _ in 0..5 {
        m.run_frame(&mut cpu);
    }
    let count_a = m.read(0x0020);
    let frame_a = m.read(0x5802);
    let cycle_a = cpu.cycles;

    // Side B: rebuild from the snapshot and run the same five frames.
    let mut mb = Machine::new([0; 0x2000]);
    let mut cb = Cpu::new();
    mb.restore_state(&mut cb, &snap).unwrap();
    for _ in 0..5 {
        mb.run_frame(&mut cb);
    }
    assert_eq!(mb.read(0x0020), count_a, "same continuation, same count");
    assert_eq!(mb.read(0x5802), frame_a);
    assert_eq!(cb.cycles, cycle_a, "cycle-exact continuation");
    assert!(count_a > frozen, "the frozen state still advances");
    assert_eq!(frame_a, frames + 5);
}
#[test]
fn snapshot_captures_registers_and_io() {
    let mut m = Machine::with_banks(vec![[0xAA; 0x2000], [0xBB; 0x2000]]);
    m.key(b'Q');
    m.write(0x5804, 0x81);
    m.write(0x5806, 1);
    m.write(0x5807, 90);
    m.write(0x5808, 100);
    m.write(0x5809, 50);
    m.write(0x580B, 0x12);
    m.write(0x5810, 0x02);
    m.write(0x4000, 0xA5);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    let snap = m.save_state(&cpu);
    let old = (cpu.a, cpu.x, cpu.y, cpu.s, cpu.p, cpu.pc, cpu.cycles);

    let mut m2 = Machine::new([0x11; 0x2000]);
    let mut c2 = Cpu::new();
    m2.restore_state(&mut c2, &snap).unwrap();
    assert_eq!(m2.read(0x5804), 0x81, "palette");
    assert_eq!(m2.read(0x5806), 1, "bank register");
    assert_eq!(m2.read(0xE000), 0xBB, "bank 1 still visible");
    assert_eq!(m2.read(0x5807), 90, "beeper");
    assert_eq!(m2.read(0x5808), 100, "spr0 x");
    assert_eq!(m2.read(0x5809), 50, "spr0 y");
    assert_eq!(m2.read(0x580A), 0x00, "spr0 pat lo");
    assert_eq!(m2.read(0x580B), 0x12, "spr0 pat hi");
    assert_eq!(m2.read(0x5810), 0x02, "sprite ctrl");
    assert_eq!(m2.read(0x5800), b'Q', "pending key survives");
    assert_eq!(m2.fb()[0], 0xA5, "fb survives");
    assert_eq!(c2.pc, old.5, "cpu pc");
    assert_eq!(c2.cycles, old.6, "cpu cycles");
}

#[test]
fn bad_state_files_are_rejected() {
    let mut m = Machine::new([0xEA; 0x2000]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    let snap = m.save_state(&cpu);

    let mut bad = snap.clone();
    bad[2] = b'X';
    assert!(mb_restore_fails(&bad));

    // every truncation point of the payload must fail
    for cut in 3..snap.len() {
        assert!(mb_restore_fails(&snap[..cut]), "truncation at {cut} accepted");
    }

    // plus one byte short is not enough either... trailing junk is rejected
    let mut junk = snap.clone();
    junk.push(0);
    assert!(mb_restore_fails(&junk), "trailing byte rejected");
}

fn mb_restore_fails(data: &[u8]) -> bool {
    let mut m = Machine::new([0; 0x2000]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    m.restore_state(&mut cpu, data).is_err()
}
