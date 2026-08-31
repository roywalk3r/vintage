// VINTAGE-1
// Author: roywalk3r
// Repo: https://github.com/roywalk3r/vintage
// License: MIT
//! 2bpp color mode: PALETTE ($5804) bit 7 selects 128x192 fat-pixel video;
//! bits 1:0 pick the four-color scheme. Sprite pixels invert the 2-bit index
//! of each covered fat pixel (index ^= 3), so sprites display brightest.

use vintage::cpu::{Bus, Cpu};
use vintage::machine::Machine;

#[test]
fn sprite_xor_inverts_2bpp_pixel_index() {
    let mut m = Machine::new([0u8; 0x2000]);
    for i in 0..8u16 {
        m.write(0x0400 + i, 0xFF);
    }
    m.write(0x5804, 0x80); // 2bpp, scheme 0
    m.write(0x5808, 8); // x = 8 fat pixels
    m.write(0x580B, 0x04); // pattern pointer = $0400
    m.write(0x5810, 0x01); // sprite 0 on
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    m.run_frame(&mut cpu);
    assert_eq!(m.fb()[2], 0xFF, "fat pixels 8..15 flip to index 3");
    assert_eq!(m.fb()[3], 0xFF, "byte 3 covered too");
    assert_eq!(m.fb()[1], 0x00, "bytes left of the sprite stay dark");
}

#[test]
fn fb_plane_untouched_by_sprites_in_2bpp() {
    let mut m = Machine::new([0u8; 0x2000]);
    m.write(0x5804, 0x80); // 2bpp, scheme 0
    m.write(0x4000, 0x40); // index 1 for fat pixels 0..3
    m.write(0x0400, 0x80); // one pattern bit -> leftmost fat pixel
    m.write(0x580B, 0x04);
    m.write(0x5810, 0x01);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    m.run_frame(&mut cpu);
    assert_eq!(m.fb()[0], 0x80, "index 1 inverts to index 2 under sprite");
    assert_eq!(m.read(0x4000), 0x40, "fb plane keeps the background");
}

#[test]
fn one_bit_mode_regression_guard() {
    let mut m = Machine::new([0u8; 0x2000]);
    for i in 0..8u16 {
        m.write(0x0400 + i, 0xFF);
    }
    m.write(0x5808, 8); // x = 8 narrow pixels -> byte 1
    m.write(0x580B, 0x04);
    m.write(0x5810, 0x1);
    let mut cpu = Cpu::new();
    cpu.reset(&mut m);
    m.run_frame(&mut cpu);
    assert_eq!(m.fb()[1], 0xFF, "narrow pixels 8..15 -> byte 1 in 1bpp");
    assert_eq!(m.fb()[2], 0x00, "byte 2 untouched in 1bpp");
}
