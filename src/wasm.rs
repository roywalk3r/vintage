//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! C-ABI surface for the web console (wasm32-unknown-unknown).
//!
//! One global VINTAGE-1 instance: a Machine plus its Cpu. JS talks to the
//! machine purely through linear-memory pointers and scalar calls — no
//! wasm-bindgen, so the console stays dependency-free.

use crate::cpu::{Bus, Cpu};
use crate::machine::Machine;

static mut MACH: Option<Machine> = None;
static mut CPUS: Option<Cpu> = None;

#[inline]
fn m() -> &'static mut Machine {
    unsafe {
        // WASM is single-threaded; lazily boot a blank machine so early
        // calls never see a None.
        let slot = (&raw mut MACH).cast::<Option<Machine>>();
        if (*slot).is_none() {
            *slot = Some(Machine::new([0; 0x2000]));
        }
        (*slot).as_mut().unwrap_unchecked()
    }
}

#[inline]
fn c() -> &'static mut Cpu {
    unsafe {
        let slot = (&raw mut CPUS).cast::<Option<Cpu>>();
        if (*slot).is_none() {
            *slot = Some(Cpu::new());
        }
        (*slot).as_mut().unwrap_unchecked()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vin_reset() {
    unsafe {
        (&raw mut MACH).write_volatile(Some(Machine::new([0; 0x2000])));
        let mut cpu = Cpu::new();
        cpu.reset(m());
        (&raw mut CPUS).write_volatile(Some(cpu));
    }
}

/// Load a raw 8K ROM image (caller must pass exactly 0x2000 bytes).
#[unsafe(no_mangle)]
// Clippy can't see the JS-side invariant that `ptr` is a live heap slice.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vin_load_rom(ptr: *const u8, len: usize) {
    if ptr.is_null() || len != 0x2000 {
        return;
    }
    unsafe {
        let mut rom = [0u8; 0x2000];
        std::ptr::copy_nonoverlapping(ptr, rom.as_mut_ptr(), 0x2000);
        (&raw mut MACH).write_volatile(Some(Machine::new(rom)));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vin_step() -> u32 {
    c().step(m())
}

#[unsafe(no_mangle)]
pub extern "C" fn vin_run_frame() {
    m().run_frame(c());
}

#[unsafe(no_mangle)]
pub extern "C" fn vin_fb_ptr() -> *const u8 {
    m().fb().as_ptr()
}

/// Post a keypress (KEY_UP/DOWN/LEFT/RIGHT codes).
#[unsafe(no_mangle)]
pub extern "C" fn vin_key(code: u8) {
    m().key(code);
}

/// Current beeper period ($5807), 0 = silence. The console maps this to a
/// square wave: audible frequency is 120,000 / (2 × period) Hz.
#[unsafe(no_mangle)]
pub extern "C" fn vin_beeper() -> u8 {
    m().beeper_period()
}

/// Active phosphor ($5804): 0 green, 1 amber, 2 white.
#[unsafe(no_mangle)]
pub extern "C" fn vin_palette() -> u8 {
    m().palette()
}

#[unsafe(no_mangle)]
pub extern "C" fn vin_rd(a: u16) -> u8 {
    m().read(a)
}

#[unsafe(no_mangle)]
pub extern "C" fn vin_wr(a: u16, v: u8) {
    m().write(a, v);
}

/// Reboot the CPU against the currently loaded ROM.
#[unsafe(no_mangle)]
pub extern "C" fn vin_cpu_reset() {
    unsafe {
        let mut cpu = Cpu::new();
        cpu.reset(m());
        (&raw mut CPUS).write_volatile(Some(cpu));
    }
}

/// Snapshot [a, x, y, s, pc_lo, pc_hi, flags, cycles_lo] for the JS debugger.
#[unsafe(no_mangle)]
pub extern "C" fn vin_cpu_state_ptr() -> *const u8 {
    static mut STATE: [u8; 8] = [0; 8];
    unsafe {
        let cpu = c();
        STATE[0] = cpu.a;
        STATE[1] = cpu.x;
        STATE[2] = cpu.y;
        STATE[3] = cpu.s;
        STATE[4] = cpu.pc as u8;
        STATE[5] = (cpu.pc >> 8) as u8;
        STATE[6] = cpu.p;
        STATE[7] = (cpu.cycles & 0xFF) as u8;
        (&raw const STATE).cast()
    }
}
