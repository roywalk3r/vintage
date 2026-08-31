//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The shipped banks demo, run headless: the RAM dispatcher flips the
//! cartridge every 128 frames, so the framebuffer alternates between bank 1
//! (horizontal bars) and bank 0 (vertical bars).

use vintage::asm::assemble;
use vintage::cpu::Cpu;
use vintage::machine::Machine;

fn image(segments: &[(u16, Vec<u8>)]) -> [u8; 0x2000] {
    let mut img = [0u8; 0x2000];
    for &(addr, ref bytes) in segments {
        img[addr as usize - 0xE000..addr as usize - 0xE000 + bytes.len()]
            .copy_from_slice(bytes);
    }
    img
}

fn classify(fb: &[u8; 0x1800]) -> u8 {
    if fb.iter().all(|&b| b == 0) {
        return 0;
    }
    if fb[0] != 0 && fb[1] == 0 && fb[2] != 0 && fb[49] == 0 {
        return 1;
    }
    if fb[0] != 0 && fb[32] != 0 && fb[128] == 0 && fb[256] != 0 {
        return 2;
    }
    3
}
#[test]
fn banks_demo_toggles_cartridge_every_dwell() {
    let src = include_str!("../software/banks.s");
    let bin = assemble(src).unwrap();
    assert_eq!(bin.extra_banks.len(), 1);
    let mut banks: Vec<[u8; 0x2000]> = vec![image(&bin.segments)];
    for seg in &bin.extra_banks {
        banks.push(image(seg));
    }
    let mut m = Machine::with_banks(banks);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    let mut runs: Vec<(u8, u32)> = Vec::new();
    for _ in 0..500 {
        m.run_frame(&mut cpu);
        let c = classify(m.fb());
        match runs.last_mut() {
            Some((last, n)) if *last == c => *n += 1,
            _ => runs.push((c, 1)),
        }
    }
    let heads: Vec<u8> = runs.iter().map(|r| r.0).collect();
    assert!(
        heads.starts_with(&[0, 2, 1, 2][..]),
        "pattern runs {:?}",
        heads
    );
    for &(kind, n) in runs.iter().take(3) {
        if kind != 0 {
            assert!(n >= 100, "run length {} for class {}", n, kind);
        }
    }
}
