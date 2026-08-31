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

/// Load a multi-bank cartridge: `nbanks` consecutive 8K images at `ptr`.
/// Bank 0 boots; higher banks become visible through $5806.
#[unsafe(no_mangle)]
// Clippy can't see the JS-side invariant that `ptr` is a live heap slice.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vin_load_banks(ptr: *const u8, nbanks: usize) {
    if ptr.is_null() || nbanks == 0 || nbanks > 256 {
        return;
    }
    unsafe {
        let mut banks = Vec::with_capacity(nbanks);
        for i in 0..nbanks {
            let mut img = [0u8; 0x2000];
            std::ptr::copy_nonoverlapping(ptr.add(i * 0x2000), img.as_mut_ptr(), 0x2000);
            banks.push(img);
        }
        (&raw mut MACH).write_volatile(Some(Machine::with_banks(banks)));
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

/// Total CPU cycles since the current ROM booted, for the console's
/// cycle-accounting panel.
#[unsafe(no_mangle)]
pub extern "C" fn vin_cycles() -> u64 {
    c().cycles
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

/// Snapshot [a, x, y, s, pc_lo, pc_hi, flags, cycles_lsb] for the JS debugger.
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

/// Full-machine save state (.vst). It is parked in a scratch Vec because it
/// is ~63 KB: JS copies it out through linear memory in one go.
static mut SCRATCH: Vec<u8> = Vec::new();
/// Assembler output and error text, parked the same way for JS.
static mut ASM_OUT: Vec<u8> = Vec::new();
static mut ASM_ERR: Vec<u8> = Vec::new();

/// Serialize the live machine + CPU. Call vin_save_ptr() after this to get
/// the bytes' location.
#[unsafe(no_mangle)]
pub extern "C" fn vin_save_state() -> usize {
    unsafe {
        let s = m().save_state(c());
        let slot = (&raw mut SCRATCH).cast::<Vec<u8>>();
        (*slot).clear();
        (*slot).extend_from_slice(&s);
        s.len()
    }
}

/// Assemble `len` bytes of 6502 source at `ptr`. Returns the `.vin`
/// container length in bytes, or 0 on error — the message (with source line)
/// is parked for vin_asm_err_ptr()/vin_asm_err_len().
#[unsafe(no_mangle)]
// Clippy can't see the JS-side invariant that `ptr` is a live heap slice.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vin_asm(ptr: *const u8, len: usize) -> usize {
    if ptr.is_null() {
        return 0;
    }
    let data = unsafe { std::slice::from_raw_parts(ptr, len) };
    let result = match std::str::from_utf8(data) {
        Ok(src) => match crate::asm::assemble(src) {
            Ok(bin) => Ok(bin.to_container()),
            Err(e) => Err(format!("line {}: {}", e.line, e.msg)),
        },
        Err(_) => Err("source is not UTF-8".to_string()),
    };
    match result {
        Ok(container) => {
            let slot = (&raw mut ASM_OUT).cast::<Vec<u8>>();
            unsafe {
                (*slot).clear();
                (*slot).extend_from_slice(&container);
            }
            container.len()
        }
        Err(msg) => {
            let slot = (&raw mut ASM_ERR).cast::<Vec<u8>>();
            unsafe {
                (*slot).clear();
                (*slot).extend_from_slice(msg.as_bytes());
            }
            0
        }
    }
}

/// Pointer to the container produced by the last successful vin_asm().
#[unsafe(no_mangle)]
pub extern "C" fn vin_asm_ptr() -> *const u8 {
    unsafe {
        let slot = (&raw mut ASM_OUT).cast::<Vec<u8>>();
        (*slot).as_ptr()
    }
}

/// Pointer to the error text parked by the last failed vin_asm().
#[unsafe(no_mangle)]
pub extern "C" fn vin_asm_err_ptr() -> *const u8 {
    unsafe {
        let slot = (&raw mut ASM_ERR).cast::<Vec<u8>>();
        (*slot).as_ptr()
    }
}

/// Length of the error text parked by the last failed vin_asm().
#[unsafe(no_mangle)]
pub extern "C" fn vin_asm_err_len() -> usize {
    unsafe {
        let slot = (&raw mut ASM_ERR).cast::<Vec<u8>>();
        (*slot).len()
    }
}

/// Pointer to the bytes written by the last vin_save_state() call.
#[unsafe(no_mangle)]
pub extern "C" fn vin_save_ptr() -> *const u8 {
    unsafe {
        let slot = (&raw mut SCRATCH).cast::<Vec<u8>>();
        (*slot).as_ptr()
    }
}

/// Restore the live machine + CPU from a .vst image at `ptr`. Returns 1 on
/// success, 0 on any parse error (the image is rejected before any field is
/// assigned, so a bad file never half-restores).
#[unsafe(no_mangle)]
// Clippy can't see the JS-side invariant that `ptr` is a live heap slice.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vin_load_state(ptr: *const u8, len: usize) -> u8 {
    if ptr.is_null() {
        return 0;
    }
    let data = unsafe { std::slice::from_raw_parts(ptr, len) };
    match m().restore_state(c(), data) {
        Ok(()) => 1,
        Err(_) => 0,
    }
}
