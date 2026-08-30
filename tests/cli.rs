//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! End-to-end CLI tests: drive the real `vintage` binary.

use std::fs;
use std::process::Command;

fn vintage() -> Command {
    Command::new(env!("CARGO_BIN_EXE_vintage"))
}

fn tmp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("vintage-cli-{name}"))
}

#[test]
fn run_renders_framebuffer_to_ppm() {
    let ppm = tmp("minimal.ppm");
    let _ = fs::remove_file(&ppm);
    let out = vintage()
        .args([
            "run",
            "tests/fixtures/minimal.s",
            "--frames",
            "2",
            "--ppm",
        ])
        .arg(&ppm)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let data = fs::read(&ppm).unwrap();
    let header = b"P6\n256 192\n255\n";
    assert!(data.starts_with(header), "not a raw PPM: {:?}", &data[..20]);
    assert_eq!(data.len(), header.len() + 256 * 192 * 3);

    // framebuffer byte 0 = $81, MSB-left: pixel x=0 (bit 7) lit, x=1 dark
    let body = header.len();
    let px = |x: usize, y: usize| &data[body + (y * 256 + x) * 3..body + (y * 256 + x) * 3 + 3];
    assert_eq!(px(0, 0), &[51, 255, 51], "x=0 (MSB of $81) should be lit green");
    assert_eq!(px(1, 0), &[6, 12, 6], "x=1 should be phosphor-dark");
}

#[test]
fn asm_writes_v1_container() {
    let bin = tmp("minimal.vin");
    let _ = fs::remove_file(&bin);
    let out = vintage()
        .args(["asm", "tests/fixtures/minimal.s", "-o"])
        .arg(&bin)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let data = fs::read(&bin).unwrap();
    assert_eq!(&data[0..2], b"V1", "magic");
    assert_eq!(u16::from_le_bytes([data[2], data[3]]), 2, "segment count");

    // first segment: $E000, then the lda/sta/jmp code
    assert_eq!(u16::from_le_bytes([data[4], data[5]]), 0xE000);
    let len = u16::from_le_bytes([data[6], data[7]]) as usize;
    assert_eq!(&data[8..8 + len], &[0xA9, 0x81, 0x8D, 0x00, 0x40, 0x4C, 0x05, 0xE0]);

    // second segment: $FFFC reset vector
    let off = 8 + len;
    assert_eq!(u16::from_le_bytes([data[off], data[off + 1]]), 0xFFFC);
    let vlen = u16::from_le_bytes([data[off + 2], data[off + 3]]) as usize;
    assert_eq!(vlen, 2);
    assert_eq!(&data[off + 4..off + 6], &[0x00, 0xE0]);
}

#[test]
fn disasm_lists_instructions() {
    let raw = tmp("snippet.bin");
    fs::write(&raw, [0xA9u8, 0x41, 0x8D, 0x34, 0x12, 0xEA]).unwrap();
    let out = vintage()
        .args(["disasm"])
        .arg(&raw)
        .args(["--base", "$C000"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("$C000"), "listing should show addresses: {text}");
    assert!(text.contains("lda #$41"), "{text}");
    assert!(text.contains("sta $1234"), "{text}");
    assert!(text.contains("nop"), "{text}");
}

#[test]
fn bad_source_reports_line() {
    let out = vintage()
        .args(["run", "tests/fixtures/broken.s"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "broken source must fail");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("line 2"), "stderr: {err}");
}