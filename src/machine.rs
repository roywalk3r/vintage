//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! The VINTAGE-1 machine: memory map, memory-mapped video, and I/O registers.
//!
//! ```text
//! $0000–$3FFF  RAM (16K)          $5800  keyboard   (read clears)
//! $4000–$57FF  framebuffer (6K)   $5802  frame counter lo/hi
//!   256×192, 1bpp, MSB-left       $5804  phosphor palette (green/amber/white)
//! $5800–$5FFF  I/O                $5805  LFSR random
//! $6000–$DFFF  RAM (32K)          $5807  beeper period (0 = silence)
//! $E000–$FFFF  ROM (8K, vectors)  2 MHz → 33,333 cycles/frame @ 60 Hz
//! ```

use std::cell::Cell;

use crate::cpu::{Bus, Cpu, FLAG_I};

pub const CYCLES_PER_FRAME: u32 = 33_333;
pub const FB_BASE: u16 = 0x4000;
pub const FB_LEN: u16 = 0x1800;
pub const SCREEN_W: usize = 256;
pub const SCREEN_H: usize = 192;

pub const KEY_UP: u8 = 0x11;
pub const KEY_DOWN: u8 = 0x12;
pub const KEY_LEFT: u8 = 0x13;
pub const KEY_RIGHT: u8 = 0x14;

pub struct Machine {
    ram_lo: [u8; 0x4000],
    fb: [u8; FB_LEN as usize],
    ram_hi: [u8; 0x8000],
    /// Cartridge banks. `banks[0]` is the boot image; software picks the
    /// visible one through the $5806 register. One bank = the 8K window.
    banks: Vec<[u8; 0x2000]>,
    bank_sel: usize,
    // Registers with read side effects need interior mutability: `Bus::read`
    // takes `&self`, exactly like a debugger peeking at live hardware.
    key_pending: Cell<u8>,
    frame: u16,
    palette: u8,
    lfsr: Cell<u16>,
    beeper_period: u8,
    /// Vsync IRQ latch: raised at frame start, taken (or masked out) once.
    vsync_pending: bool,
    /// Sprite latches. Patterns are 8 bytes fetched through the bus from
    /// `spr_ptr`, XORed into the display buffer at vsync.
    spr_x: [u8; 2],
    spr_y: [u8; 2],
    spr_ptr: [u16; 2],
    spr_ctrl: u8,
    /// What hosts render: the fb with sprites XOR-composited at vsync.
    /// The fb itself stays background-only; not CPU-visible.
    disp: [u8; FB_LEN as usize],
}

impl Machine {
    pub fn new(rom: [u8; 0x2000]) -> Self {
        Self::with_banks(vec![rom])
    }

    /// Build a machine from one image per cartridge bank. The first entry
    /// boots; higher banks become visible via $5806.
    pub fn with_banks(banks: Vec<[u8; 0x2000]>) -> Self {
        Self {
            ram_lo: [0; 0x4000],
            fb: [0; FB_LEN as usize],
            ram_hi: [0; 0x8000],
            bank_sel: 0,
            banks,
            key_pending: Cell::new(0),
            frame: 0,
            palette: 0,
            lfsr: Cell::new(0xACE1),
            beeper_period: 0,
            vsync_pending: false,
            spr_x: [0; 2],
            spr_y: [0; 2],
            spr_ptr: [0; 2],
            spr_ctrl: 0,
            disp: [0; FB_LEN as usize],
        }
    }

    /// Post a keypress into the one-key buffer (newest wins).
    pub fn key(&mut self, code: u8) {
        self.key_pending.set(code);
    }

    /// Advance the video frame counter (host calls this 60×/s).
    pub fn tick_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// Drive the CPU for one video frame's worth of cycles, then tick.
    ///
    /// Vsync pulses the IRQ line once per frame: latched at frame start,
    /// taken at the first instruction boundary with I clear, and dropped
    /// for the rest of the frame once serviced. A program holding `sei`
    /// loses the pulse — that is the documented contract.
    pub fn run_frame(&mut self, cpu: &mut Cpu) {
        self.vsync_pending = true;
        let mut budget = CYCLES_PER_FRAME as u64;
        while budget > 0 {
            cpu.irq_line = self.vsync_pending && cpu.p & FLAG_I == 0;
            let spent = u64::from(cpu.step(self));
            if cpu.irq_line {
                self.vsync_pending = false;
            }
            budget = budget.saturating_sub(spent);
        }
        cpu.irq_line = false;
        self.compose_sprites();
        self.tick_frame();
    }

    /// XOR-composite the two sprites over a fresh copy of the framebuffer.
    /// The fb stays background-only, so a sprite that holds position across
    /// frames does not blink; software always sees the background plane.
    fn compose_sprites(&mut self) {
        self.disp.copy_from_slice(&self.fb);
        for s in 0..2 {
            if self.spr_ctrl & (1 << s) == 0 {
                continue;
            }
            for row in 0..8u16 {
                let y = self.spr_y[s] as usize + row as usize;
                if y >= SCREEN_H {
                    continue;
                }
                let bits = self.read(self.spr_ptr[s] + row);
                for bit in 0..8 {
                    if bits & (0x80 >> bit) == 0 {
                        continue;
                    }
                    let x = self.spr_x[s] as usize + bit;
                    if x >= SCREEN_W {
                        continue;
                    }
                    let idx = y * 32 + (x >> 3);
                    self.disp[idx] ^= 0x80 >> (x & 7);
                }
            }
        }
    }

    /// The framebuffer as the host renderer sees it: 6144 bytes, one bit per
    /// pixel, MSB leftmost, 32 bytes per scanline, sprites composited.
    pub fn fb(&self) -> &[u8; FB_LEN as usize] {
        &self.disp
    }

    pub fn palette(&self) -> u8 {
        self.palette
    }

    pub fn beeper_period(&self) -> u8 {
        self.beeper_period
    }

    /// 16-bit Galois LFSR, taps 0x002D — every read steps it.
    fn rand(&self) -> u8 {
        let mut l = self.lfsr.get();
        l = (l >> 1) ^ (0x002D * (l & 1));
        self.lfsr.set(l);
        l as u8
    }
}

impl Bus for Machine {
    fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3FFF => self.ram_lo[addr as usize],
            0x4000..=0x57FF => self.fb[(addr - FB_BASE) as usize],
            0x5800 => {
                let k = self.key_pending.get();
                self.key_pending.set(0);
                k
            }
            0x5802 => self.frame as u8,
            0x5803 => (self.frame >> 8) as u8,
            0x5804 => self.palette,
            0x5805 => self.rand(),
            0x5806 => self.bank_sel as u8,
            0x5807 => self.beeper_period,
            0x5808..=0x580F => {
                let s = (addr >> 2) as usize & 1;
                match addr & 3 {
                    0 => self.spr_x[s],
                    1 => self.spr_y[s],
                    2 => self.spr_ptr[s] as u8,
                    _ => (self.spr_ptr[s] >> 8) as u8,
                }
            }
            0x5810 => self.spr_ctrl,
            0x6000..=0xDFFF => self.ram_hi[(addr - 0x6000) as usize],
            0xE000..=0xFFFF => self.banks[self.bank_sel][(addr - 0xE000) as usize],
            _ => 0,
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x3FFF => self.ram_lo[addr as usize] = val,
            0x4000..=0x57FF => self.fb[(addr - FB_BASE) as usize] = val,
            0x5808..=0x580F => {
                let s = (addr >> 2) as usize & 1;
                match addr & 3 {
                    0 => self.spr_x[s] = val,
                    1 => self.spr_y[s] = val,
                    2 => self.spr_ptr[s] = (self.spr_ptr[s] & 0xFF00) | val as u16,
                    _ => self.spr_ptr[s] = (self.spr_ptr[s] & 0x00FF) | ((val as u16) << 8),
                }
            }
            0x5810 => self.spr_ctrl = val,
            0x5804 => self.palette = val,
            0x5806 => {
                if (val as usize) < self.banks.len() {
                    self.bank_sel = val as usize;
                }
            }
            0x5807 => self.beeper_period = val,
            0x6000..=0xDFFF => self.ram_hi[(addr - 0x6000) as usize] = val,
            // ROM and unmapped I/O holes silently ignore writes.
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine() -> Machine {
        Machine::new([0xEA; 0x2000])
    }

    #[test]
    fn memory_map_decodes_all_regions() {
        let mut m = machine();
        m.write(0x0000, 0x11);
        m.write(0x3FFF, 0x22);
        m.write(0x4000, 0x33);
        m.write(0x57FF, 0x44);
        m.write(0x6000, 0x55);
        m.write(0xDFFF, 0x66);
        assert_eq!(m.read(0x0000), 0x11);
        assert_eq!(m.read(0x3FFF), 0x22);
        assert_eq!(m.read(0x4000), 0x33);
        assert_eq!(m.read(0x57FF), 0x44);
        assert_eq!(m.read(0x6000), 0x55);
        assert_eq!(m.read(0xDFFF), 0x66);
        assert_eq!(m.read(0xE000), 0xEA);
        assert_eq!(m.read(0xFFFF), 0xEA);
    }

    #[test]
    fn rom_is_write_protected() {
        let mut m = machine();
        m.write(0xE000, 0x42);
        m.write(0xFFFC, 0x99);
        assert_eq!(m.read(0xE000), 0xEA);
        assert_eq!(m.read(0xFFFC), 0xEA);
    }

    #[test]
    fn vectors_live_in_rom() {
        let mut rom = [0; 0x2000];
        rom[0x1FFC] = 0x00;
        rom[0x1FFD] = 0xE0;
        let m = Machine::new(rom);
        assert_eq!(m.read(0xFFFC), 0x00);
        assert_eq!(m.read(0xFFFD), 0xE0);
    }

    #[test]
    fn framebuffer_is_memory_mapped() {
        let mut m = Machine::new([0xEA; 0x2000]);
        m.write(0x4000, 0xFF);
        m.write(0x57FF, 0x81);
        assert_eq!(m.read(0x4000), 0xFF);
        assert_eq!(m.read(0x57FF), 0x81);
        // fb() is the display plane: the fb is copied into it at each vsync
        // by compose_sprites, so one run_frame makes the writes visible.
        let mut cpu = Cpu::new();
        cpu.reset(&mut m);
        m.run_frame(&mut cpu);
        assert_eq!(m.fb()[0], 0xFF);
        assert_eq!(m.fb()[FB_LEN as usize - 1], 0x81);
    }

    #[test]
    fn keyboard_read_clears() {
        let mut m = machine();
        m.key(b'A');
        assert_eq!(m.read(0x5800), b'A');
        assert_eq!(m.read(0x5800), 0);
    }

    #[test]
    fn keyboard_newest_key_wins() {
        let mut m = machine();
        m.key(b'A');
        m.key(b'Z');
        assert_eq!(m.read(0x5800), b'Z');
    }

    #[test]
    fn frame_counter_ticks_and_reads_little_endian() {
        let mut m = machine();
        for _ in 0..3 {
            m.tick_frame();
        }
        assert_eq!(m.read(0x5802), 3);
        assert_eq!(m.read(0x5803), 0);

        // wraps at 65536 frames (~18 minutes of uptime)
        for _ in 0..(0x10000 - 3) {
            m.tick_frame();
        }
        assert_eq!(m.read(0x5802), 0);
        assert_eq!(m.read(0x5803), 0);
        m.tick_frame();
        assert_eq!(m.read(0x5802), 1);
    }

    #[test]
    fn palette_register_roundtrip() {
        let mut m = machine();
        m.write(0x5804, 2);
        assert_eq!(m.read(0x5804), 2);
        assert_eq!(m.palette(), 2);
    }

    #[test]
    fn random_reads_are_pseudorandom() {
        let m = machine();
        let mut values = [0u8; 64];
        for v in values.iter_mut() {
            *v = m.read(0x5805);
        }
        let distinct = {
            let mut v = values.to_vec();
            v.sort_unstable();
            v.dedup();
            v.len()
        };
        assert!(distinct >= 20, "only {distinct} distinct values in 64 reads");
    }

    #[test]
    fn beeper_register_roundtrip() {
        let mut m = machine();
        m.write(0x5807, 100);
        assert_eq!(m.read(0x5807), 100);
        assert_eq!(m.beeper_period(), 100);
        m.write(0x5807, 0);
        assert_eq!(m.beeper_period(), 0);
    }

    #[test]
    fn run_frame_consumes_budget_and_ticks() {
        let mut m = machine();
        let mut cpu = Cpu::new();
        cpu.reset(&mut m);
        m.run_frame(&mut cpu);
        assert_eq!(m.read(0x5802), 1);
        assert!(
            cpu.cycles >= CYCLES_PER_FRAME as u64
                && cpu.cycles < CYCLES_PER_FRAME as u64 + 8,
            "cycles = {}",
            cpu.cycles
        );
    }

    #[test]
    fn unmapped_io_holes_read_zero_and_ignore_writes() {
        let mut m = machine();
        assert_eq!(m.read(0x5801), 0);
        assert_eq!(m.read(0x5FFF), 0);
        m.write(0x5801, 0xFF);
        m.write(0x5FFF, 0xFF);
        assert_eq!(m.read(0x5801), 0);
        assert_eq!(m.read(0x5FFF), 0);
    }

    #[test]
    fn bank_register_selects_the_rom_window() {
        let mut m = Machine::with_banks(vec![[0xAA; 0x2000], [0xBB; 0x2000]]);
        assert_eq!(m.read(0xE000), 0xAA);
        m.write(0x5806, 1);
        assert_eq!(m.read(0x5806), 1);
        assert_eq!(m.read(0xE000), 0xBB);
        m.write(0x5806, 0);
        assert_eq!(m.read(0xE000), 0xAA);
    }

    #[test]
    fn bank_window_stays_write_protected() {
        let mut m = Machine::with_banks(vec![[0xEA; 0x2000], [0x42; 0x2000]]);
        m.write(0x5806, 1);
        m.write(0xE000, 0x99);
        assert_eq!(m.read(0xE000), 0x42);
    }

    #[test]
    fn out_of_range_bank_writes_are_ignored() {
        let mut m = Machine::with_banks(vec![[0xEA; 0x2000], [0x42; 0x2000]]);
        m.write(0x5806, 7);
        assert_eq!(m.read(0x5806), 0);
        assert_eq!(m.read(0xE000), 0xEA);
    }
}