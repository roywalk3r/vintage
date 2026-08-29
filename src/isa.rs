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

/// Official-opcode inverse of `decode`. Returns `None` for combinations the
/// NMOS 6502 never shipped (e.g. `sta $12,y`, `(zp,x)` for stores is fine but
/// `(abs)` only exists for JMP). The assembler uses this to pick a legal
/// fallback mode, and to reject impossible ones.
pub const fn encode(op: Op, mode: Mode) -> Option<u8> {
    use Mode::*;
    use Op::*;
    Some(match (op, mode) {
        (Lda, Imm) => 0xA9, (Lda, Zp) => 0xA5, (Lda, Zpx) => 0xB5, (Lda, Abs) => 0xAD,
        (Lda, Abx) => 0xBD, (Lda, Aby) => 0xB9, (Lda, Izx) => 0xA1, (Lda, Izy) => 0xB1,
        (Ldx, Imm) => 0xA2, (Ldx, Zp) => 0xA6, (Ldx, Zpy) => 0xB6, (Ldx, Abs) => 0xAE,
        (Ldx, Aby) => 0xBE,
        (Ldy, Imm) => 0xA0, (Ldy, Zp) => 0xA4, (Ldy, Zpx) => 0xB4, (Ldy, Abs) => 0xAC,
        (Ldy, Abx) => 0xBC,
        (Sta, Zp) => 0x85, (Sta, Zpx) => 0x95, (Sta, Abs) => 0x8D, (Sta, Abx) => 0x9D,
        (Sta, Aby) => 0x99, (Sta, Izx) => 0x81, (Sta, Izy) => 0x91,
        (Stx, Zp) => 0x86, (Stx, Zpy) => 0x96, (Stx, Abs) => 0x8E,
        (Sty, Zp) => 0x84, (Sty, Zpx) => 0x94, (Sty, Abs) => 0x8C,
        (Adc, Imm) => 0x69, (Adc, Zp) => 0x65, (Adc, Zpx) => 0x75, (Adc, Abs) => 0x6D,
        (Adc, Abx) => 0x7D, (Adc, Aby) => 0x79, (Adc, Izx) => 0x61, (Adc, Izy) => 0x71,
        (Sbc, Imm) => 0xE9, (Sbc, Zp) => 0xE5, (Sbc, Zpx) => 0xF5, (Sbc, Abs) => 0xED,
        (Sbc, Abx) => 0xFD, (Sbc, Aby) => 0xF9, (Sbc, Izx) => 0xE1, (Sbc, Izy) => 0xF1,
        (And, Imm) => 0x29, (And, Zp) => 0x25, (And, Zpx) => 0x35, (And, Abs) => 0x2D,
        (And, Abx) => 0x3D, (And, Aby) => 0x39, (And, Izx) => 0x21, (And, Izy) => 0x31,
        (Ora, Imm) => 0x09, (Ora, Zp) => 0x05, (Ora, Zpx) => 0x15, (Ora, Abs) => 0x0D,
        (Ora, Abx) => 0x1D, (Ora, Aby) => 0x19, (Ora, Izx) => 0x01, (Ora, Izy) => 0x11,
        (Eor, Imm) => 0x49, (Eor, Zp) => 0x45, (Eor, Zpx) => 0x55, (Eor, Abs) => 0x4D,
        (Eor, Abx) => 0x5D, (Eor, Aby) => 0x59, (Eor, Izx) => 0x41, (Eor, Izy) => 0x51,
        (Cmp, Imm) => 0xC9, (Cmp, Zp) => 0xC5, (Cmp, Zpx) => 0xD5, (Cmp, Abs) => 0xCD,
        (Cmp, Abx) => 0xDD, (Cmp, Aby) => 0xD9, (Cmp, Izx) => 0xC1, (Cmp, Izy) => 0xD1,
        (Cpx, Imm) => 0xE0, (Cpx, Zp) => 0xE4, (Cpx, Abs) => 0xEC,
        (Cpy, Imm) => 0xC0, (Cpy, Zp) => 0xC4, (Cpy, Abs) => 0xCC,
        (Inc, Zp) => 0xE6, (Inc, Zpx) => 0xF6, (Inc, Abs) => 0xEE, (Inc, Abx) => 0xFE,
        (Dec, Zp) => 0xC6, (Dec, Zpx) => 0xD6, (Dec, Abs) => 0xCE, (Dec, Abx) => 0xDE,
        (Asl, Acc) => 0x0A, (Asl, Zp) => 0x06, (Asl, Zpx) => 0x16, (Asl, Abs) => 0x0E,
        (Asl, Abx) => 0x1E,
        (Lsr, Acc) => 0x4A, (Lsr, Zp) => 0x46, (Lsr, Zpx) => 0x56, (Lsr, Abs) => 0x4E,
        (Lsr, Abx) => 0x5E,
        (Rol, Acc) => 0x2A, (Rol, Zp) => 0x26, (Rol, Zpx) => 0x36, (Rol, Abs) => 0x2E,
        (Rol, Abx) => 0x3E,
        (Ror, Acc) => 0x6A, (Ror, Zp) => 0x66, (Ror, Zpx) => 0x76, (Ror, Abs) => 0x6E,
        (Ror, Abx) => 0x7E,
        (Bit, Zp) => 0x24, (Bit, Abs) => 0x2C,
        (Jmp, Abs) => 0x4C, (Jmp, Ind) => 0x6C,
        (Jsr, Abs) => 0x20,
        (Rts, Imp) => 0x60, (Rti, Imp) => 0x40,
        (BrPl, Rel) => 0x10, (BrMi, Rel) => 0x30, (BrVc, Rel) => 0x50, (BrVs, Rel) => 0x70,
        (BrCc, Rel) => 0x90, (BrCs, Rel) => 0xB0, (BrNe, Rel) => 0xD0, (BrEq, Rel) => 0xF0,
        (Inx, Imp) => 0xE8, (Iny, Imp) => 0xC8, (Dex, Imp) => 0xCA, (Dey, Imp) => 0x88,
        (Tax, Imp) => 0xAA, (Tay, Imp) => 0xA8, (Txa, Imp) => 0x8A, (Tya, Imp) => 0x98,
        (Tsx, Imp) => 0xBA, (Txs, Imp) => 0x9A,
        (Pha, Imp) => 0x48, (Php, Imp) => 0x08, (Pla, Imp) => 0x68, (Plp, Imp) => 0x28,
        (Clc, Imp) => 0x18, (Sec, Imp) => 0x38, (Cli, Imp) => 0x58, (Sei, Imp) => 0x78,
        (Cld, Imp) => 0xD8, (Sed, Imp) => 0xF8, (Clv, Imp) => 0xB8,
        (Brk, Imp) => 0x00, (Nop, Imp) => 0xEA,
        _ => return None,
    })
}

pub const fn instruction_len(mode: Mode) -> u8 {
    use Mode::*;
    match mode {
        Imp | Acc => 1,
        Imm | Zp | Zpx | Zpy | Izx | Izy | Rel => 2,
        Abs | Abx | Aby | Ind => 3,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_are_inverse() {
        for byte in 0..=255u8 {
            let (op, mode) = decode(byte);
            if let Some(encoded) = encode(op, mode) {
                assert_eq!(
                    decode(encoded),
                    (op, mode),
                    "opcode {byte:#04x} re-encodes to {encoded:#04x}, which decodes differently"
                );
            }
        }
    }

    #[test]
    fn encode_covers_exactly_the_151_official_opcodes() {
        use Mode::*;
        const OPS: [Op; 56] = [
            Op::Lda, Op::Ldx, Op::Ldy, Op::Sta, Op::Stx, Op::Sty,
            Op::Adc, Op::Sbc, Op::And, Op::Ora, Op::Eor, Op::Cmp, Op::Cpx, Op::Cpy,
            Op::Inc, Op::Dec, Op::Asl, Op::Lsr, Op::Rol, Op::Ror, Op::Bit,
            Op::Jmp, Op::Jsr, Op::Rts, Op::Rti,
            Op::BrPl, Op::BrMi, Op::BrVc, Op::BrVs, Op::BrCc, Op::BrCs, Op::BrNe, Op::BrEq,
            Op::Inx, Op::Iny, Op::Dex, Op::Dey,
            Op::Tax, Op::Tay, Op::Txa, Op::Tya, Op::Tsx, Op::Txs,
            Op::Pha, Op::Php, Op::Pla, Op::Plp,
            Op::Clc, Op::Sec, Op::Cli, Op::Sei, Op::Cld, Op::Sed, Op::Clv,
            Op::Brk, Op::Nop,
        ];
        const MODES: [Mode; 13] = [
            Imp, Acc, Imm, Zp, Zpx, Zpy, Abs, Abx, Aby, Ind, Izx, Izy, Rel,
        ];
        let count: usize = OPS
            .iter()
            .map(|op| {
                MODES
                    .iter()
                    .filter(|mode| encode(*op, **mode).is_some())
                    .count()
            })
            .sum();
        assert_eq!(count, 151, "official NMOS 6502 has exactly 151 opcodes");
    }

    #[test]
    fn undocumented_nops_never_reencode() {
        // The assembler must never emit these; encode() has no entry for them.
        for byte in [0x04u8, 0x14, 0x1C, 0x34, 0x44, 0x54, 0x64, 0x74, 0x80, 0x82, 0x89, 0x0C, 0xC2, 0xD4, 0xE2, 0xF4, 0xFC] {
            let (op, mode) = decode(byte);
            if (op, mode) != (Op::Nop, Mode::Imp) {
                assert_eq!(encode(op, mode), None, "{byte:#04x} must not re-encode");
            }
        }
    }
}