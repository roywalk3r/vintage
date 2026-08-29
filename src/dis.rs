//! VINTAGE-1
//! Copyright 2026 roywalk3r
//! SPDX-License-Identifier: MIT
//! 6502 disassembler — renders bytes back to assembler syntax, so
//! `assemble(disasm(b))` round-trips for every official opcode.

use crate::isa::{decode, instruction_len, Mode, Op};

/// Disassemble one instruction from the start of `bytes`, which lives at
/// `addr` in memory. Returns `(byte length, text)`, or `None` if the operand
/// bytes are truncated. The text is exactly the assembler's syntax.
pub fn disasm_one(bytes: &[u8], addr: u16) -> Option<(u8, String)> {
    let (&opcode, rest) = bytes.split_first()?;
    let (op, mode) = decode(opcode);
    let len = instruction_len(mode);
    let name = mnemonic_name(op);
    let text = match mode {
        Mode::Imp => name.into(),
        Mode::Acc => format!("{name} a"),
        Mode::Imm => format!("{name} #${:02X}", rest.first()?),
        Mode::Zp => format!("{name} ${:02X}", rest.first()?),
        Mode::Zpx => format!("{name} ${:02X},x", rest.first()?),
        Mode::Zpy => format!("{name} ${:02X},y", rest.first()?),
        Mode::Izx => format!("{name} (${:02X},x)", rest.first()?),
        Mode::Izy => format!("{name} (${:02X}),y", rest.first()?),
        Mode::Abs => format!("{name} ${:04X}", word(rest)?),
        Mode::Abx => format!("{name} ${:04X},x", word(rest)?),
        Mode::Aby => format!("{name} ${:04X},y", word(rest)?),
        Mode::Ind => format!("{name} (${:04X})", word(rest)?),
        Mode::Rel => {
            let off = *rest.first()? as i8 as i32;
            let target = addr as i32 + 2 + off;
            format!("{name} ${:04X}", target as u16)
        }
    };
    Some((len, text))
}

fn word(operand: &[u8]) -> Option<u16> {
    Some(u16::from(*operand.first()?) | u16::from(*operand.get(1)?) << 8)
}

fn mnemonic_name(op: Op) -> &'static str {
    use Op::*;
    match op {
        Lda => "lda", Ldx => "ldx", Ldy => "ldy", Sta => "sta", Stx => "stx", Sty => "sty",
        Adc => "adc", Sbc => "sbc", And => "and", Ora => "ora", Eor => "eor",
        Cmp => "cmp", Cpx => "cpx", Cpy => "cpy",
        Inc => "inc", Dec => "dec", Asl => "asl", Lsr => "lsr", Rol => "rol", Ror => "ror",
        Bit => "bit", Jmp => "jmp", Jsr => "jsr", Rts => "rts", Rti => "rti",
        BrPl => "bpl", BrMi => "bmi", BrVc => "bvc", BrVs => "bvs",
        BrCc => "bcc", BrCs => "bcs", BrNe => "bne", BrEq => "beq",
        Inx => "inx", Iny => "iny", Dex => "dex", Dey => "dey",
        Tax => "tax", Tay => "tay", Txa => "txa", Tya => "tya", Tsx => "tsx", Txs => "txs",
        Pha => "pha", Php => "php", Pla => "pla", Plp => "plp",
        Clc => "clc", Sec => "sec", Cli => "cli", Sei => "sei",
        Cld => "cld", Sed => "sed", Clv => "clv",
        Brk => "brk", Nop => "nop",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::assemble;

    fn dis(bytes: &[u8]) -> String {
        disasm_one(bytes, 0x8000).unwrap().1
    }

    #[test]
    fn implied() {
        let (len, text) = disasm_one(&[0xE8], 0).unwrap();
        assert_eq!((len, text.as_str()), (1, "inx"));
    }

    #[test]
    fn accumulator() {
        assert_eq!(dis(&[0x0A]), "asl a");
    }

    #[test]
    fn immediate() {
        assert_eq!(dis(&[0xA9, 0x41]), "lda #$41");
    }

    #[test]
    fn zero_page_and_indexed() {
        assert_eq!(dis(&[0xA5, 0x10]), "lda $10");
        assert_eq!(dis(&[0xB5, 0x10]), "lda $10,x");
        assert_eq!(dis(&[0xB6, 0x10]), "ldx $10,y");
    }

    #[test]
    fn absolute_and_indexed() {
        assert_eq!(dis(&[0xAD, 0x34, 0x12]), "lda $1234");
        assert_eq!(dis(&[0xBD, 0x34, 0x12]), "lda $1234,x");
        assert_eq!(dis(&[0x99, 0x34, 0x12]), "sta $1234,y");
    }

    #[test]
    fn indirect_forms() {
        assert_eq!(dis(&[0x6C, 0x34, 0x12]), "jmp ($1234)");
        assert_eq!(dis(&[0xA1, 0x10]), "lda ($10,x)");
        assert_eq!(dis(&[0xB1, 0x10]), "lda ($10),y");
    }

    #[test]
    fn branch_forward() {
        // $8000 + 2 + $05 = $8007
        let (len, text) = disasm_one(&[0xD0, 0x05], 0x8000).unwrap();
        assert_eq!((len, text.as_str()), (2, "bne $8007"));
    }

    #[test]
    fn branch_backward() {
        // $8000 + 2 - $04 = $7FFE
        let (len, text) = disasm_one(&[0xF0, 0xFC], 0x8000).unwrap();
        assert_eq!((len, text.as_str()), (2, "beq $7FFE"));
    }

    #[test]
    fn truncated_operand_is_none() {
        assert_eq!(disasm_one(&[0xAD], 0), None);
        assert_eq!(disasm_one(&[], 0), None);
        assert_eq!(disasm_one(&[0xA5], 0), None);
    }

    #[test]
    fn every_official_opcode_roundtrips_through_the_assembler() {
        let mut covered = 0;
        for opcode in 0..=255u8 {
            let (op, mode) = decode(opcode);
            let Some(reencoded) = crate::isa::encode(op, mode) else {
                continue; // undocumented or non-canonical: not our syntax
            };
            if reencoded != opcode {
                continue; // illegal opcode that decodes to (nop, implied)
            }
            let operand: Vec<u8> = match mode {
                Mode::Imp | Mode::Acc => vec![],
                Mode::Imm | Mode::Zp | Mode::Zpx | Mode::Zpy | Mode::Izx | Mode::Izy => {
                    vec![0x10]
                }
                Mode::Abs | Mode::Abx | Mode::Aby | Mode::Ind => vec![0x34, 0x12],
                Mode::Rel => vec![0x05],
            };
            let mut bytes = vec![opcode];
            bytes.extend_from_slice(&operand);
            let Some((len, text)) = disasm_one(&bytes, 0x8000) else {
                panic!("{opcode:#04x} failed to disassemble");
            };
            assert_eq!(
                instruction_len(mode),
                len,
                "{opcode:#04x} wrong length"
            );
            let bin = assemble(&format!(" .org $8000\n {text}"))
                .unwrap_or_else(|e| panic!("{text}: {e:?}"));
            assert_eq!(
                bin.segments[0].1, bytes,
                "{text} did not reassemble to {bytes:02x?}"
            );
            covered += 1;
        }
        assert_eq!(covered, 151);
    }
}