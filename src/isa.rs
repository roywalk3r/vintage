//! The 6502 instruction set — opcodes, addressing modes, and timing tables,
//! shared by the CPU core, the disassembler, and the assembler.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Imp,
    Acc,
    Imm,
    Zp,
    Zpx,
    Zpy,
    Abs,
    Abx,
    Aby,
    Ind,
    Izx,
    Izy,
    Rel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Lda, Ldx, Ldy, Sta, Stx, Sty,
    Adc, Sbc, And, Ora, Eor, Cmp, Cpx, Cpy,
    Inc, Dec, Asl, Lsr, Rol, Ror, Bit,
    Jmp, Jsr, Rts, Rti,
    BrPl, BrMi, BrVc, BrVs, BrCc, BrCs, BrNe, BrEq,
    Inx, Iny, Dex, Dey,
    Tax, Tay, Txa, Tya, Tsx, Txs,
    Pha, Php, Pla, Plp,
    Clc, Sec, Cli, Sei, Cld, Sed, Clv,
    Brk, Nop,
}

pub const fn decode(opcode: u8) -> (Op, Mode) {
    use Mode::*;
    use Op::*;
    match opcode {
        0xA9 => (Lda, Imm), 0xA5 => (Lda, Zp), 0xB5 => (Lda, Zpx), 0xAD => (Lda, Abs),
        0xBD => (Lda, Abx), 0xB9 => (Lda, Aby), 0xA1 => (Lda, Izx), 0xB1 => (Lda, Izy),
        0xA2 => (Ldx, Imm), 0xA6 => (Ldx, Zp), 0xB6 => (Ldx, Zpy), 0xAE => (Ldx, Abs),
        0xBE => (Ldx, Aby),
        0xA0 => (Ldy, Imm), 0xA4 => (Ldy, Zp), 0xB4 => (Ldy, Zpx), 0xAC => (Ldy, Abs),
        0xBC => (Ldy, Abx),
        0x85 => (Sta, Zp), 0x95 => (Sta, Zpx), 0x8D => (Sta, Abs), 0x9D => (Sta, Abx),
        0x99 => (Sta, Aby), 0x81 => (Sta, Izx), 0x91 => (Sta, Izy),
        0x86 => (Stx, Zp), 0x96 => (Stx, Zpy), 0x8E => (Stx, Abs),
        0x84 => (Sty, Zp), 0x94 => (Sty, Zpx), 0x8C => (Sty, Abs),
        0x69 => (Adc, Imm), 0x65 => (Adc, Zp), 0x75 => (Adc, Zpx), 0x6D => (Adc, Abs),
        0x7D => (Adc, Abx), 0x79 => (Adc, Aby), 0x61 => (Adc, Izx), 0x71 => (Adc, Izy),
        0xE9 => (Sbc, Imm), 0xE5 => (Sbc, Zp), 0xF5 => (Sbc, Zpx), 0xED => (Sbc, Abs),
        0xFD => (Sbc, Abx), 0xF9 => (Sbc, Aby), 0xE1 => (Sbc, Izx), 0xF1 => (Sbc, Izy),
        0x29 => (And, Imm), 0x25 => (And, Zp), 0x35 => (And, Zpx), 0x2D => (And, Abs),
        0x3D => (And, Abx), 0x39 => (And, Aby), 0x21 => (And, Izx), 0x31 => (And, Izy),
        0x09 => (Ora, Imm), 0x05 => (Ora, Zp), 0x15 => (Ora, Zpx), 0x0D => (Ora, Abs),
        0x1D => (Ora, Abx), 0x19 => (Ora, Aby), 0x01 => (Ora, Izx), 0x11 => (Ora, Izy),
        0x49 => (Eor, Imm), 0x45 => (Eor, Zp), 0x55 => (Eor, Zpx), 0x4D => (Eor, Abs),
        0x5D => (Eor, Abx), 0x59 => (Eor, Aby), 0x41 => (Eor, Izx), 0x51 => (Eor, Izy),
        0xC9 => (Cmp, Imm), 0xC5 => (Cmp, Zp), 0xD5 => (Cmp, Zpx), 0xCD => (Cmp, Abs),
        0xDD => (Cmp, Abx), 0xD9 => (Cmp, Aby), 0xC1 => (Cmp, Izx), 0xD1 => (Cmp, Izy),
        0xE0 => (Cpx, Imm), 0xE4 => (Cpx, Zp), 0xEC => (Cpx, Abs),
        0xC0 => (Cpy, Imm), 0xC4 => (Cpy, Zp), 0xCC => (Cpy, Abs),
        0xE6 => (Inc, Zp), 0xF6 => (Inc, Zpx), 0xEE => (Inc, Abs), 0xFE => (Inc, Abx),
        0xC6 => (Dec, Zp), 0xD6 => (Dec, Zpx), 0xCE => (Dec, Abs), 0xDE => (Dec, Abx),
        0x0A => (Asl, Acc), 0x06 => (Asl, Zp), 0x16 => (Asl, Zpx), 0x0E => (Asl, Abs),
        0x1E => (Asl, Abx),
        0x4A => (Lsr, Acc), 0x46 => (Lsr, Zp), 0x56 => (Lsr, Zpx), 0x4E => (Lsr, Abs),
        0x5E => (Lsr, Abx),
        0x2A => (Rol, Acc), 0x26 => (Rol, Zp), 0x36 => (Rol, Zpx), 0x2E => (Rol, Abs),
        0x3E => (Rol, Abx),
        0x6A => (Ror, Acc), 0x66 => (Ror, Zp), 0x76 => (Ror, Zpx), 0x6E => (Ror, Abs),
        0x7E => (Ror, Abx),
        0x24 => (Bit, Zp), 0x2C => (Bit, Abs),
        0x4C => (Jmp, Abs), 0x6C => (Jmp, Ind),
        0x20 => (Jsr, Abs),
        0x60 => (Rts, Imp), 0x40 => (Rti, Imp),
        0x10 => (BrPl, Rel), 0x30 => (BrMi, Rel), 0x50 => (BrVc, Rel), 0x70 => (BrVs, Rel),
        0x90 => (BrCc, Rel), 0xB0 => (BrCs, Rel), 0xD0 => (BrNe, Rel), 0xF0 => (BrEq, Rel),
        0xE8 => (Inx, Imp), 0xC8 => (Iny, Imp), 0xCA => (Dex, Imp), 0x88 => (Dey, Imp),
        0xAA => (Tax, Imp), 0xA8 => (Tay, Imp), 0x8A => (Txa, Imp), 0x98 => (Tya, Imp),
        0xBA => (Tsx, Imp), 0x9A => (Txs, Imp),
        0x48 => (Pha, Imp), 0x08 => (Php, Imp), 0x68 => (Pla, Imp), 0x28 => (Plp, Imp),
        0x18 => (Clc, Imp), 0x38 => (Sec, Imp), 0x58 => (Cli, Imp), 0x78 => (Sei, Imp),
        0xD8 => (Cld, Imp), 0xF8 => (Sed, Imp), 0xB8 => (Clv, Imp),
        0x00 => (Brk, Imp), 0xEA => (Nop, Imp),
        // Stable undocumented opcodes, treated as NOPs that still consume
        // their operands — keeps the PC sane if a program strays into them.
        0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => (Nop, Imm),
        0x04 | 0x44 | 0x64 => (Nop, Zp),
        0x14 | 0x54 | 0x74 | 0xD4 | 0xF4 => (Nop, Zpx),
        0x0C => (Nop, Abs),
        0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => (Nop, Abx),
        _ => (Nop, Imp),
    }
}

/// Base cycles for operand-reading instructions; the caller adds the
/// page-cross penalty from `resolve`.
pub const fn read_cycles(mode: Mode) -> u64 {
    use Mode::*;
    match mode {
        Imm => 2,
        Zp => 3,
        Zpx | Zpy => 4,
        Abs => 4,
        Abx | Aby => 4,
        Izx => 6,
        Izy => 5,
        _ => 2,
    }
}

pub const fn store_cycles(mode: Mode) -> u64 {
    use Mode::*;
    match mode {
        Zp => 3,
        Zpx | Zpy => 4,
        Abs => 4,
        Abx | Aby => 5,
        Izx => 6,
        Izy => 6,
        _ => 2,
    }
}

pub const fn rmw_cycles(mode: Mode) -> u64 {
    use Mode::*;
    match mode {
        Acc => 2,
        Zp => 5,
        Zpx | Zpy => 6,
        Abs => 6,
        Abx | Aby => 7,
        _ => 2,
    }
}