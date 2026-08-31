// VINTAGE-1
// Author: roywalk3r
// Repo: https://github.com/roywalk3r/vintage
// License: MIT
//! Sprite unit: two 8x8 1-bpp sprites, XOR-composited over the framebuffer
//! at each vsync, driven through the $5808 block.
//!
//! Registers:
//!   $5808/$5809  sprite 0 x/y latches (pixels, 0-based top-left)
//!   $580A/$580B  sprite 0 pattern pointer (lo/hi; 8 pattern bytes fetched
//!                through the bus each frame, MSB-left, one byte per row)
//!   $580C/$580D  sprite 1 x/y latches
//!   $580E/$580F  sprite 1 pattern pointer
//!   $5810  SPR_CTRL: bit0 sprite 0 enable, bit1 sprite 1 enable

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

/// Sprite 0 at (10,20), pattern rows $81/…/$81, enabled: rows 0 and 7 give
/// lit pixels at x=10 and x=17 on fb rows y=20 and y=27.
#[test]
fn sprite_composes_xor_over_dark_framebuffer() {
    let mut m = machine(
        "
            .org $E000
    entry:  lda #10
            sta $5808        ; x = 10
            lda #20
            sta $5809        ; y = 20
            lda #<pat
            sta $580A
            lda #pat>>8
            sta $580B
            lda #1
            sta $5810        ; enable sprite 0

    halt:   jmp halt

            .org $FFFC
            .word entry

            .org $F000
    pat:    .byte $81, 0, 0, 0, 0, 0, 0, $81
        ",
    );
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    m.run_frame(&mut cpu);
    // pattern row 0 → display row 20: x=10 → byte 1 bit $20, x=17 → byte 2 $40
    let disp = m.fb();
    assert_eq!(disp[20 * 32 + 1], 0x20, "pixel (10,20) lit");
    assert_eq!(disp[20 * 32 + 2], 0x40, "pixel (17,20) lit");
    assert_eq!(disp[20 * 32], 0x00, "no spurious pixels");
    assert_eq!(disp[20 * 32 + 3], 0x00, "no spurious pixels");
}

/// A fully lit sprite row over an already-lit fb row XORs the row dark. The
/// pattern lives in RAM ($0400, uploaded through the bus) to prove the
/// pointer is bus-fetched, not a hardcoded ROM read.
#[test]
fn sprite_xor_erases_lit_background_pixels() {
    let mut m = machine(
        "
            .org $E000
    entry:  lda #1
            sta $5810        ; enable sprite 0
    halt:   jmp halt
        ",
    );
    // Pattern $FF x8 at $0400 (RAM, bus-fetched) and fb row 0 lit at $4000,
    // so the XOR has something to erase. Sprite at (0,0) covers exactly that.
    for i in 0..8u16 {
        m.write(0x0400 + i, 0xFF);
        m.write(0x4000 + i, 0xFF);
    }
    m.write(0x580A, 0x00);
    m.write(0x580B, 0x04); // pattern pointer = $0400
    m.write(0x5810, 0x01);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    m.run_frame(&mut cpu);
    // disp = fb ^ pattern = $FF ^ $FF = $00 under the sprite; the fb plane
    // keeps the background so the sprite never blinks across frames.
    assert_eq!(m.fb()[0], 0x00, "display row XORed dark under the sprite");
    assert_eq!(m.read(0x4000), 0xFF, "fb plane keeps the background");
}

/// Disabled sprites do not touch the fb; past-edge rows and columns are
/// clipped, not wrapped.
#[test]
fn disabled_sprite_and_edge_clipping() {
    let mut m = machine(
        "
            .org $E000
    entry:  lda #254
            sta $5808        ; sprite 0 x = 254 (cols 254..261 → 254,255 only)
            lda #190
            sta $5809        ; sprite 0 y = 190 (rows 190..197 → 190,191 only)
            lda #<pat
            sta $580A
            lda #pat>>8
            sta $580B
    halt:   jmp halt

            .org $FFFC
            .word entry

            .org $F000
    pat:    .byte $80, $40, $80, $80, $80, $80, $80, $80
        ",
    );
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    // Disabled (SPR_CTRL stays 0): nothing drawn.
    m.run_frame(&mut cpu);
    assert_eq!(m.fb()[190 * 32 + 31], 0x00, "disabled -> untouched");

    // Enable: rows 190/191 painted, rows 192+ clipped; past-256 bits clip.
    m.write(0x5810, 0x01);
    m.run_frame(&mut cpu);
    let disp = m.fb();
    assert_eq!(disp[190 * 32 + 31], 0x02, "pixel (254,190) lit");
    assert_eq!(disp[191 * 32 + 31], 0x01, "pixel (255,191) lit");
    assert_eq!(disp[189 * 32 + 31], 0x00, "row above stays dark");
}