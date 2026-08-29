//! VINTAGE-1
//! Copyright 2026 roywalk3r
//! SPDX-License-Identifier: MIT
//! 6502 CPU core — the silicon of VINTAGE-1.
//!
//! Models the original NMOS 6502, bug-for-bug: decimal-mode ADC/SBC with the
//! NMOS flag quirks, page-crossing cycle penalties, the JMP ($xxFF) page-wrap
//! bug, and zero-page index wraparound. Correctness is gated on Klaus
//! Dormann's functional test suite (see `tests/klaus.rs`).

use crate::isa::{decode, read_cycles, rmw_cycles, store_cycles, Mode, Op};

pub const FLAG_C: u8 = 0b0000_0001;
pub const FLAG_Z: u8 = 0b0000_0010;
pub const FLAG_I: u8 = 0b0000_0100;
pub const FLAG_D: u8 = 0b0000_1000;
pub const FLAG_B: u8 = 0b0001_0000;
pub const FLAG_U: u8 = 0b0010_0000;
pub const FLAG_V: u8 = 0b0100_0000;
pub const FLAG_N: u8 = 0b1000_0000;

pub const NMI_VECTOR: u16 = 0xFFFA;
pub const RESET_VECTOR: u16 = 0xFFFC;
pub const IRQ_VECTOR: u16 = 0xFFFE;

pub trait Bus {
    fn read(&self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub s: u8,
    pub pc: u16,
    pub p: u8,
    pub cycles: u64,
    pub irq_line: bool,
    nmi_pending: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            s: 0xFD,
            pc: 0,
            p: FLAG_U | FLAG_I,
            cycles: 0,
            irq_line: false,
            nmi_pending: false,
        }
    }

    pub fn reset(&mut self, bus: &mut dyn Bus) {
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.s = 0xFD;
        self.p = FLAG_U | FLAG_I;
        self.pc = self.read_vector(bus, RESET_VECTOR);
        self.cycles += 7;
        self.nmi_pending = false;
    }

    /// Execute one instruction (or a pending interrupt), returning the cycles
    /// it consumed. Also accumulates into `self.cycles`.
    pub fn step(&mut self, bus: &mut dyn Bus) -> u32 {
        let before = self.cycles;
        if self.nmi_pending {
            self.nmi_pending = false;
            self.interrupt(bus, NMI_VECTOR, false);
            return (self.cycles - before) as u32;
        }
        if self.irq_line && self.p & FLAG_I == 0 {
            self.interrupt(bus, IRQ_VECTOR, false);
            return (self.cycles - before) as u32;
        }
        let opcode = self.fetch8(bus);
        let (op, mode) = decode(opcode);
        self.execute(op, mode, bus);
        (self.cycles - before) as u32
    }

    /// Raise an NMI (edge-triggered; taken before the next instruction).
    pub fn nmi(&mut self) {
        self.nmi_pending = true;
    }

    // ---- Fetch / stack primitives -------------------------------------

    fn fetch8(&mut self, bus: &mut dyn Bus) -> u8 {
        let v = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        v
    }

    fn fetch16(&mut self, bus: &mut dyn Bus) -> u16 {
        let lo = u16::from(self.fetch8(bus));
        let hi = u16::from(self.fetch8(bus));
        lo | hi << 8
    }

    fn read_vector(&mut self, bus: &mut dyn Bus, v: u16) -> u16 {
        let lo = u16::from(bus.read(v));
        let hi = u16::from(bus.read(v + 1));
        lo | hi << 8
    }

    fn push(&mut self, bus: &mut dyn Bus, v: u8) {
        bus.write(0x0100 | u16::from(self.s), v);
        self.s = self.s.wrapping_sub(1);
    }

    fn pop(&mut self, bus: &mut dyn Bus) -> u8 {
        self.s = self.s.wrapping_add(1);
        bus.read(0x0100 | u16::from(self.s))
    }

    fn push16(&mut self, bus: &mut dyn Bus, v: u16) {
        self.push(bus, (v >> 8) as u8);
        self.push(bus, (v & 0xFF) as u8);
    }

    fn pop16(&mut self, bus: &mut dyn Bus) -> u16 {
        let lo = u16::from(self.pop(bus));
        let hi = u16::from(self.pop(bus));
        lo | hi << 8
    }

    // ---- Flags ----------------------------------------------------------

    fn set_flag(&mut self, f: u8, on: bool) {
        if on {
            self.p |= f;
        } else {
            self.p &= !f;
        }
    }

    fn set_zn(&mut self, v: u8) {
        self.set_flag(FLAG_Z, v == 0);
        self.set_flag(FLAG_N, v & 0x80 != 0);
    }

    // ---- Addressing ------------------------------------------------------

    /// Resolve an addressing mode to an effective address, consuming operand
    /// bytes. Returns `(ea, crossed_page)`; the bool is the extra-cycle
    /// penalty for indexed reads.
    fn resolve(&mut self, mode: Mode, bus: &mut dyn Bus) -> (u16, bool) {
        match mode {
            Mode::Imp | Mode::Acc | Mode::Rel => (0, false),
            Mode::Imm => {
                let ea = self.pc;
                self.pc = self.pc.wrapping_add(1);
                (ea, false)
            }
            Mode::Zp => (u16::from(self.fetch8(bus)), false),
            Mode::Zpx => (u16::from(self.fetch8(bus).wrapping_add(self.x)), false),
            Mode::Zpy => (u16::from(self.fetch8(bus).wrapping_add(self.y)), false),
            Mode::Abs => (self.fetch16(bus), false),
            Mode::Abx => {
                let b = self.fetch16(bus);
                let ea = b.wrapping_add(u16::from(self.x));
                (ea, ((b ^ ea) & 0xFF00) != 0)
            }
            Mode::Aby => {
                let b = self.fetch16(bus);
                let ea = b.wrapping_add(u16::from(self.y));
                (ea, ((b ^ ea) & 0xFF00) != 0)
            }
            Mode::Izx => {
                let z = self.fetch8(bus).wrapping_add(self.x);
                let lo = bus.read(u16::from(z));
                let hi = bus.read(u16::from(z.wrapping_add(1)));
                (u16::from(lo) | u16::from(hi) << 8, false)
            }
            Mode::Izy => {
                let z = self.fetch8(bus);
                let lo = bus.read(u16::from(z));
                let hi = bus.read(u16::from(z.wrapping_add(1)));
                let b = u16::from(lo) | u16::from(hi) << 8;
                let ea = b.wrapping_add(u16::from(self.y));
                (ea, ((b ^ ea) & 0xFF00) != 0)
            }
            Mode::Ind => {
                let ptr = self.fetch16(bus);
                // NMOS bug: the pointer's high byte never increments — $xxFF
                // wraps back to $xx00 for the second byte.
                let lo = bus.read(ptr);
                let hi = bus.read((ptr & 0xFF00) | (ptr.wrapping_add(1) & 0x00FF));
                (u16::from(lo) | u16::from(hi) << 8, false)
            }
        }
    }

    // ---- Execution -------------------------------------------------------

    fn execute(&mut self, op: Op, mode: Mode, bus: &mut dyn Bus) {
        use Op::*;

        match op {
            Lda | Ldx | Ldy | Adc | Sbc | And | Ora | Eor | Cmp | Cpx | Cpy | Bit | Nop => {
                let (ea, cross) = self.resolve(mode, bus);
                let m = bus.read(ea);
                self.cycles += read_cycles(mode) + u64::from(cross);
                match op {
                    Lda => {
                        self.a = m;
                        self.set_zn(m);
                    }
                    Ldx => {
                        self.x = m;
                        self.set_zn(m);
                    }
                    Ldy => {
                        self.y = m;
                        self.set_zn(m);
                    }
                    Adc => self.adc(m),
                    Sbc => self.sbc(m),
                    And => {
                        self.a &= m;
                        self.set_zn(self.a);
                    }
                    Ora => {
                        self.a |= m;
                        self.set_zn(self.a);
                    }
                    Eor => {
                        self.a ^= m;
                        self.set_zn(self.a);
                    }
                    Cmp => self.compare(self.a, m),
                    Cpx => self.compare(self.x, m),
                    Cpy => self.compare(self.y, m),
                    Bit => {
                        self.set_flag(FLAG_Z, self.a & m == 0);
                        self.set_flag(FLAG_N, m & 0x80 != 0);
                        self.set_flag(FLAG_V, m & 0x40 != 0);
                    }
                    _ => {}
                }
            }

            Sta | Stx | Sty => {
                let (ea, _) = self.resolve(mode, bus);
                let v = match op {
                    Sta => self.a,
                    Stx => self.x,
                    _ => self.y,
                };
                bus.write(ea, v);
                self.cycles += store_cycles(mode);
            }

            Inc | Dec | Asl | Lsr | Rol | Ror => {
                if mode == Mode::Acc {
                    let r = self.rmw_value(op, self.a);
                    self.a = r;
                    self.set_zn(r);
                    self.cycles += 2;
                } else {
                    let (ea, _) = self.resolve(mode, bus);
                    let v = bus.read(ea);
                    // NMOS read-modify-write puts the old value on the bus
                    // before the new one; I/O registers see both writes.
                    bus.write(ea, v);
                    let r = self.rmw_value(op, v);
                    bus.write(ea, r);
                    self.set_zn(r);
                    self.cycles += rmw_cycles(mode);
                }
            }

            Jmp => {
                let (ea, _) = self.resolve(mode, bus);
                self.pc = ea;
                self.cycles += if mode == Mode::Ind { 5 } else { 3 };
            }

            Jsr => {
                let target = self.fetch16(bus);
                // Return address points at the third byte of the instruction.
                let ret = self.pc.wrapping_sub(1);
                self.push16(bus, ret);
                self.pc = target;
                self.cycles += 6;
            }

            Rts => {
                self.pc = self.pop16(bus).wrapping_add(1);
                self.cycles += 6;
            }

            Rti => {
                self.p = self.pop(bus) & !FLAG_B | FLAG_U;
                self.pc = self.pop16(bus);
                self.cycles += 6;
            }

            BrPl => self.branch(self.p & FLAG_N == 0, bus),
            BrMi => self.branch(self.p & FLAG_N != 0, bus),
            BrVc => self.branch(self.p & FLAG_V == 0, bus),
            BrVs => self.branch(self.p & FLAG_V != 0, bus),
            BrCc => self.branch(self.p & FLAG_C == 0, bus),
            BrCs => self.branch(self.p & FLAG_C != 0, bus),
            BrNe => self.branch(self.p & FLAG_Z == 0, bus),
            BrEq => self.branch(self.p & FLAG_Z != 0, bus),

            Inx => {
                self.x = self.x.wrapping_add(1);
                self.set_zn(self.x);
                self.cycles += 2;
            }
            Iny => {
                self.y = self.y.wrapping_add(1);
                self.set_zn(self.y);
                self.cycles += 2;
            }
            Dex => {
                self.x = self.x.wrapping_sub(1);
                self.set_zn(self.x);
                self.cycles += 2;
            }
            Dey => {
                self.y = self.y.wrapping_sub(1);
                self.set_zn(self.y);
                self.cycles += 2;
            }

            Tax => {
                self.x = self.a;
                self.set_zn(self.x);
                self.cycles += 2;
            }
            Tay => {
                self.y = self.a;
                self.set_zn(self.y);
                self.cycles += 2;
            }
            Txa => {
                self.a = self.x;
                self.set_zn(self.a);
                self.cycles += 2;
            }
            Tya => {
                self.a = self.y;
                self.set_zn(self.a);
                self.cycles += 2;
            }
            Tsx => {
                self.x = self.s;
                self.set_zn(self.x);
                self.cycles += 2;
            }
            // TXS is the only transfer that does not touch flags.
            Txs => {
                self.s = self.x;
                self.cycles += 2;
            }

            Pha => {
                let v = self.a;
                self.push(bus, v);
                self.cycles += 3;
            }
            Php => {
                let v = self.p | FLAG_B | FLAG_U;
                self.push(bus, v);
                self.cycles += 3;
            }
            Pla => {
                self.a = self.pop(bus);
                self.set_zn(self.a);
                self.cycles += 4;
            }
            Plp => {
                self.p = self.pop(bus) & !FLAG_B | FLAG_U;
                self.cycles += 4;
            }

            Clc => {
                self.p &= !FLAG_C;
                self.cycles += 2;
            }
            Sec => {
                self.p |= FLAG_C;
                self.cycles += 2;
            }
            Cli => {
                self.p &= !FLAG_I;
                self.cycles += 2;
            }
            Sei => {
                self.p |= FLAG_I;
                self.cycles += 2;
            }
            Cld => {
                self.p &= !FLAG_D;
                self.cycles += 2;
            }
            Sed => {
                self.p |= FLAG_D;
                self.cycles += 2;
            }
            Clv => {
                self.p &= !FLAG_V;
                self.cycles += 2;
            }

            Brk => {
                // BRK is a two-byte instruction: skip the signature byte.
                self.pc = self.pc.wrapping_add(1);
                self.interrupt(bus, IRQ_VECTOR, true);
            }
        }
    }

    fn rmw_value(&mut self, op: Op, v: u8) -> u8 {
        use Op::*;
        match op {
            Inc => v.wrapping_add(1),
            Dec => v.wrapping_sub(1),
            Asl => {
                self.set_flag(FLAG_C, v & 0x80 != 0);
                v << 1
            }
            Lsr => {
                self.set_flag(FLAG_C, v & 0x01 != 0);
                v >> 1
            }
            Rol => {
                let c = self.p & FLAG_C != 0;
                self.set_flag(FLAG_C, v & 0x80 != 0);
                (v << 1) | u8::from(c)
            }
            Ror => {
                let c = self.p & FLAG_C != 0;
                self.set_flag(FLAG_C, v & 0x01 != 0);
                (v >> 1) | u8::from(c) << 7
            }
            _ => v,
        }
    }

    fn branch(&mut self, taken: bool, bus: &mut dyn Bus) {
        let off = self.fetch8(bus) as i8;
        self.cycles += 2;
        if taken {
            let base = self.pc;
            let target = (base as i16 + i16::from(off)) as u16;
            self.cycles += 1 + u64::from(((base ^ target) & 0xFF00) != 0);
            self.pc = target;
        }
    }

    fn compare(&mut self, reg: u8, m: u8) {
        let r = reg.wrapping_sub(m);
        self.set_flag(FLAG_C, reg >= m);
        self.set_zn(r);
    }

    // ---- Arithmetic ------------------------------------------------------

    /// ADC. NMOS decimal mode: V from the binary sum, N from the
    /// pre-correction intermediate, Z/C from the decimal result.
    fn adc(&mut self, m: u8) {
        let a = self.a;
        let c_in = u16::from(self.p & FLAG_C != 0);
        let bin = a as u16 + m as u16 + c_in;
        let bin_b = bin as u8;
        self.set_flag(FLAG_V, ((bin_b ^ a) & (bin_b ^ m) & 0x80) != 0);
        if self.p & FLAG_D != 0 {
            let mut lo = u16::from(a & 0x0F) + u16::from(m & 0x0F) + c_in;
            if lo > 9 {
                lo += 6;
            }
            let mut hi = u16::from(a & 0xF0) + u16::from(m & 0xF0) + (lo & 0xF0);
            self.set_flag(FLAG_N, hi & 0x80 != 0);
            let carry = hi >= 0xA0;
            if carry {
                hi += 0x60;
            }
            let r = ((hi & 0xF0) as u8) | (lo as u8 & 0x0F);
            self.set_flag(FLAG_C, carry);
            self.set_flag(FLAG_Z, r == 0);
            self.a = r;
        } else {
            self.set_flag(FLAG_C, bin > 0xFF);
            self.set_zn(bin_b);
            self.a = bin_b;
        }
    }

    /// SBC. NMOS decimal mode: all flags from the binary result; only the
    /// value is decimal-corrected (nines-complement borrow adjustment).
    fn sbc(&mut self, m: u8) {
        let a = self.a;
        let c_in = i16::from(self.p & FLAG_C != 0);
        let bin = a as i16 - m as i16 - (1 - c_in);
        let bin_b = bin as u8;
        self.set_flag(FLAG_C, bin >= 0);
        self.set_flag(FLAG_V, ((a ^ m) & (a ^ bin_b) & 0x80) != 0);
        self.set_zn(bin_b);
        if self.p & FLAG_D != 0 {
            let al = (a & 0x0F) as i16 - (m & 0x0F) as i16 + c_in - 1;
            let al = if al < 0 {
                ((al - 6) & 0x0F) - 0x10
            } else {
                al
            };
            let mut r = (a & 0xF0) as i16 - (m & 0xF0) as i16 + al;
            if r < 0 {
                r -= 0x60;
            }
            self.a = r as u8;
        } else {
            self.a = bin_b;
        }
    }

    // ---- Interrupts ------------------------------------------------------

    fn interrupt(&mut self, bus: &mut dyn Bus, vector: u16, brk_flag: bool) {
        let ret = self.pc;
        self.push16(bus, ret);
        let p = if brk_flag {
            self.p | FLAG_B | FLAG_U
        } else {
            self.p & !FLAG_B | FLAG_U
        };
        self.push(bus, p);
        self.p |= FLAG_I;
        self.pc = self.read_vector(bus, vector);
        self.cycles += 7;
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test RAM is pre-filled with NOP (0xEA) so over-stepping a program
    /// cannot disturb flags; every byte the CPU touches beyond the program
    /// is an explicit load.
    struct Ram([u8; 0x10000]);

    impl Ram {
        fn load(&mut self, addr: u16, bytes: &[u8]) {
            let a = addr as usize;
            self.0[a..a + bytes.len()].copy_from_slice(bytes);
        }
    }

    impl Bus for Ram {
        fn read(&self, addr: u16) -> u8 {
            self.0[addr as usize]
        }
        fn write(&mut self, addr: u16, val: u8) {
            self.0[addr as usize] = val;
        }
    }

    fn run_at(base: u16, bytes: &[u8], steps: usize) -> (Cpu, Ram) {
        let mut cpu = Cpu::new();
        let mut ram = Ram([0xEA; 0x10000]);
        ram.load(base, bytes);
        cpu.pc = base;
        for _ in 0..steps {
            cpu.step(&mut ram);
        }
        (cpu, ram)
    }

    fn run(bytes: &[u8], steps: usize) -> (Cpu, Ram) {
        run_at(0x1000, bytes, steps)
    }

    #[test]
    fn adc_binary_sets_overflow_on_positive_sum_overflow() {
        let (cpu, _) = run(&[0xA9, 0x50, 0x69, 0x50], 2);
        assert_eq!(cpu.a, 0xA0);
        assert!(cpu.p & FLAG_V != 0);
        assert!(cpu.p & FLAG_N != 0);
        assert!(cpu.p & FLAG_C == 0);
        assert!(cpu.p & FLAG_Z == 0);
    }

    #[test]
    fn adc_binary_sets_carry_and_zero() {
        let (cpu, _) = run(&[0xA9, 0xFF, 0x69, 0x01], 2);
        assert_eq!(cpu.a, 0x00);
        assert!(cpu.p & FLAG_C != 0);
        assert!(cpu.p & FLAG_Z != 0);
        assert!(cpu.p & FLAG_V == 0);
        assert!(cpu.p & FLAG_N == 0);
    }

    #[test]
    fn adc_decimal_classic_58_plus_46() {
        let (cpu, _) = run(&[0xF8, 0xA9, 0x58, 0x69, 0x46], 3);
        assert_eq!(cpu.a, 0x04);
        assert!(cpu.p & FLAG_C != 0);
        // NMOS quirk: N and V come from the pre-adjust binary sum ($9E).
        assert!(cpu.p & FLAG_N != 0);
        assert!(cpu.p & FLAG_V != 0);
        assert!(cpu.p & FLAG_Z == 0);
    }

    #[test]
    fn adc_decimal_99_plus_01_sets_n_and_z_together() {
        let (cpu, _) = run(&[0xF8, 0xA9, 0x99, 0x69, 0x01], 3);
        assert_eq!(cpu.a, 0x00);
        assert!(cpu.p & FLAG_C != 0);
        // The famous NMOS case: N from binary intermediate ($9A) while Z from
        // the decimal result ($00) — both set at once.
        assert!(cpu.p & FLAG_N != 0);
        assert!(cpu.p & FLAG_Z != 0);
    }

    #[test]
    fn sbc_decimal_borrows_via_nines_complement() {
        let (cpu, _) = run(&[0xF8, 0xA9, 0x00, 0x38, 0xE9, 0x01], 4);
        assert_eq!(cpu.a, 0x99);
        assert!(cpu.p & FLAG_C == 0);
        // NMOS: SBC flags come from the binary result ($FF).
        assert!(cpu.p & FLAG_N != 0);
        assert!(cpu.p & FLAG_Z == 0);
    }

    #[test]
    fn sbc_decimal_plain_subtraction() {
        let (cpu, _) = run(&[0xF8, 0xA9, 0x46, 0x38, 0xE9, 0x12], 4);
        assert_eq!(cpu.a, 0x34);
        assert!(cpu.p & FLAG_C != 0);
    }

    #[test]
    fn jmp_indirect_wraps_within_page() {
        let (mut cpu, mut ram) = (Cpu::new(), Ram([0xEA; 0x10000]));
        ram.load(0x0800, &[0x6C, 0xFF, 0x10]);
        ram.load(0x10FF, &[0x78]);
        ram.load(0x1000, &[0x34]);
        cpu.pc = 0x0800;
        cpu.step(&mut ram);
        assert_eq!(cpu.pc, 0x3478);
    }

    #[test]
    fn indexed_read_charges_page_cross_penalty() {
        let (cpu, _) = run(&[0xA2, 0x01, 0xBD, 0xFF, 0x0F], 2);
        assert_eq!(cpu.cycles, 7);
        let (cpu, _) = run(&[0xA2, 0x00, 0xBD, 0x00, 0x0F], 2);
        assert_eq!(cpu.cycles, 6);
    }

    #[test]
    fn indexed_store_never_charges_page_penalty() {
        let (cpu, _) = run(&[0xA2, 0x01, 0x9D, 0xFF, 0x0F], 2);
        assert_eq!(cpu.cycles, 7);
    }

    #[test]
    fn branch_cycles_charge_on_take_and_page_cross() {
        let (cpu, _) = run(&[0xD0, 0x05], 1);
        assert_eq!(cpu.cycles, 3);
        assert_eq!(cpu.pc, 0x1007);

        let (cpu, _) = run_at(0x10F8, &[0xD0, 0x0E], 1);
        assert_eq!(cpu.cycles, 4);
        assert_eq!(cpu.pc, 0x1108);

        let (cpu, _) = run(&[0xA9, 0x00, 0xD0, 0x05], 2);
        assert_eq!(cpu.cycles, 4);
        assert_eq!(cpu.pc, 0x1004);
    }

    #[test]
    fn zero_page_indexed_wraps_at_page_boundary() {
        let (mut cpu, mut ram) = (Cpu::new(), Ram([0xEA; 0x10000]));
        ram.load(0x1000, &[0xA2, 0x02, 0xB5, 0xFF]);
        ram.load(0x0001, &[0x42]);
        cpu.pc = 0x1000;
        for _ in 0..2 {
            cpu.step(&mut ram);
        }
        assert_eq!(cpu.a, 0x42);
    }

    #[test]
    fn indirect_y_pointer_wraps_at_page_boundary() {
        let (mut cpu, mut ram) = (Cpu::new(), Ram([0xEA; 0x10000]));
        ram.load(0x1000, &[0xA0, 0x01, 0xB1, 0xFF]);
        ram.load(0x00FF, &[0x34]);
        ram.load(0x0000, &[0x12]);
        ram.load(0x1235, &[0x77]);
        cpu.pc = 0x1000;
        for _ in 0..2 {
            cpu.step(&mut ram);
        }
        assert_eq!(cpu.a, 0x77);
    }

    #[test]
    fn php_sets_break_plp_masks_it() {
        let (cpu, ram) = run(&[0x08, 0x28], 2);
        // S starts at $FD after reset, so the first push lands at $01FD.
        assert_eq!(ram.0[0x01FD], 0x34);
        assert_eq!(cpu.s, 0xFD);
        assert_eq!(cpu.p, 0x24);
    }

    #[test]
    fn brk_pushes_pc_plus_two_and_sets_i() {
        let (mut cpu, mut ram) = (Cpu::new(), Ram([0xEA; 0x10000]));
        ram.load(0x1000, &[0x00]);
        ram.load(0xFFFE, &[0x00, 0x20]);
        cpu.pc = 0x1000;
        cpu.step(&mut ram);
        assert_eq!(cpu.pc, 0x2000);
        assert_eq!(cpu.s, 0xFA);
        assert_eq!(ram.0[0x01FD], 0x10);
        assert_eq!(ram.0[0x01FC], 0x02);
        assert_eq!(ram.0[0x01FB], 0x34);
        assert!(cpu.p & FLAG_I != 0);
        assert_eq!(cpu.cycles, 7);
    }

    #[test]
    fn rti_restores_p_and_pc_masking_break() {
        let (mut cpu, mut ram) = (Cpu::new(), Ram([0xEA; 0x10000]));
        ram.load(0x1000, &[0x40]);
        ram.load(0x01FE, &[0x15]);
        ram.load(0x01FF, &[0x00]);
        ram.load(0x0100, &[0x20]);
        cpu.pc = 0x1000;
        cpu.step(&mut ram);
        assert_eq!(cpu.pc, 0x2000);
        assert_eq!(cpu.p, 0x25);
        assert_eq!(cpu.cycles, 6);
    }

    #[test]
    fn rol_shifts_carry_in_and_out() {
        let (cpu, _) = run(&[0xA9, 0x80, 0x38, 0x2A], 3);
        assert_eq!(cpu.a, 0x01);
        assert!(cpu.p & FLAG_C != 0);
        assert!(cpu.p & FLAG_N == 0);
        assert!(cpu.p & FLAG_Z == 0);
    }

    #[test]
    fn lsr_moves_bit_zero_to_carry() {
        let (cpu, _) = run(&[0xA9, 0x01, 0x4A], 2);
        assert_eq!(cpu.a, 0x00);
        assert!(cpu.p & FLAG_C != 0);
        assert!(cpu.p & FLAG_Z != 0);
        assert!(cpu.p & FLAG_N == 0);
    }

    #[test]
    fn stack_pointer_wraps_within_page_one() {
        let (cpu, ram) = run(&[0xA9, 0x02, 0xAA, 0x9A, 0x08, 0x08, 0x08, 0x08], 7);
        assert_eq!(cpu.s, 0xFE);
        assert_eq!(ram.0[0x0100], 0x34);
    }

    #[test]
    fn reset_loads_vector_and_initializes_state() {
        let (mut cpu, mut ram) = (Cpu::new(), Ram([0xEA; 0x10000]));
        ram.load(0xFFFC, &[0x00, 0xF0]);
        cpu.reset(&mut ram);
        assert_eq!(cpu.pc, 0xF000);
        assert_eq!(cpu.s, 0xFD);
        assert!(cpu.p & FLAG_I != 0);
        assert!(cpu.p & FLAG_U != 0);
    }

    #[test]
    fn nmi_takes_vector_pushing_break_clear() {
        let (mut cpu, mut ram) = (Cpu::new(), Ram([0xEA; 0x10000]));
        ram.load(0x1000, &[0xEA]);
        ram.load(0xFFFA, &[0x00, 0xE0]);
        cpu.pc = 0x1000;
        cpu.nmi();
        cpu.step(&mut ram);
        assert_eq!(cpu.pc, 0xE000);
        assert_eq!(cpu.s, 0xFA);
        assert_eq!(ram.0[0x01FB], 0x24);
        assert!(cpu.p & FLAG_I != 0);
        assert_eq!(cpu.cycles, 7);
    }

    #[test]
    fn irq_masked_by_i_flag_then_taken_after_cli() {
        let (mut cpu, mut ram) = (Cpu::new(), Ram([0xEA; 0x10000]));
        ram.load(0x1000, &[0x58, 0xEA]);
        ram.load(0xFFFE, &[0x00, 0x30]);
        cpu.pc = 0x1000;
        cpu.irq_line = true;

        cpu.step(&mut ram);
        assert_eq!(cpu.pc, 0x1001);
        assert_eq!(cpu.cycles, 2);

        cpu.step(&mut ram);
        assert_eq!(cpu.pc, 0x3000);
        assert_eq!(cpu.cycles, 9);
        assert!(cpu.p & FLAG_I != 0);
    }
}