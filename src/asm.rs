//! Two-pass 6502 assembler for VINTAGE-1.
//!
//! Syntax: `label:` definitions, `name = expr` equates, `$` hex, `%` binary,
//! `'c'` chars, full expression grammar with precedence. Addressing modes
//! are chosen by operand shape; symbolic operands always assemble absolute
//! (never zero-page) so a single sizing pass is deterministic — use `<expr`
//! to force zero page.

use std::collections::HashMap;

use crate::isa::{encode, instruction_len, Mode, Op};

/// Assembled output: contiguous runs of bytes at their addresses.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Binary {
    pub segments: Vec<(u16, Vec<u8>)>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub msg: String,
}

fn err(line: usize, msg: impl Into<String>) -> Error {
    Error {
        line,
        msg: msg.into(),
    }
}

// ---------------------------------------------------------------- expressions

#[derive(Clone, Debug, PartialEq, Eq)]
enum Expr {
    Num(i32),
    Sym(String),
    Neg(Box<Expr>),
    Lo(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

/// True if the expression references a named symbol (not `*`). Decides the
/// zp-vs-absolute rule: symbol-bearing operands always go absolute.
fn has_sym(e: &Expr) -> bool {
    match e {
        Expr::Num(_) => false,
        Expr::Sym(s) => s != "*",
        Expr::Neg(a) | Expr::Lo(a) => has_sym(a),
        Expr::Bin(_, a, b) => has_sym(a) || has_sym(b),
    }
}

/// Evaluate the expression tree with the symbols known so far. Pure AST
/// walk — no code execution; `None` means an unknown symbol, a forward
/// reference, or division by zero.
fn resolve(e: &Expr, syms: &HashMap<String, i32>, pc: u32) -> Option<i32> {
    match e {
        Expr::Num(v) => Some(*v),
        Expr::Sym(s) => {
            if s == "*" {
                Some(pc as i32)
            } else {
                syms.get(s).copied()
            }
        }
        Expr::Neg(a) => resolve(a, syms, pc).map(|v| -v),
        Expr::Lo(a) => resolve(a, syms, pc).map(|v| v & 0xFF),
        Expr::Bin(op, a, b) => {
            let (x, y) = (resolve(a, syms, pc)?, resolve(b, syms, pc)?);
            match op {
                BinOp::Add => x.checked_add(y),
                BinOp::Sub => x.checked_sub(y),
                BinOp::Mul => x.checked_mul(y),
                BinOp::Div => x.checked_div(y),
                BinOp::And => Some(x & y),
                BinOp::Or => Some(x | y),
                BinOp::Xor => Some(x ^ y),
                BinOp::Shl => x.checked_shl(y.unsigned_abs()),
                BinOp::Shr => x.checked_shr(y.unsigned_abs()),
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    Num(i32),
    Sym(String),
    /// `*` — multiply in operator position, current address in primary
    /// position; the parser decides by where it appears.
    Star,
    LParen,
    RParen,
    Plus,
    Minus,
    Slash,
    Amp,
    Pipe,
    Caret,
    Shl,
    Shr,
    Lt,
}

fn tokenize(s: &str, line: usize) -> Result<Vec<Tok>, Error> {
    let b: Vec<char> = s.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        match c {
            _ if c.is_whitespace() => i += 1,
            '$' | '%' | '0'..='9' => {
                let radix = match c {
                    '$' => 16,
                    '%' => 2,
                    _ => 10,
                };
                let start = i + usize::from(c == '$' || c == '%');
                let mut j = start;
                while j < b.len() && b[j].is_digit(radix) {
                    j += 1;
                }
                if j == start {
                    return Err(err(line, "empty numeric literal"));
                }
                let text: String = b[start..j].iter().collect();
                toks.push(Tok::Num(
                    i32::from_str_radix(&text, radix).map_err(|_| err(line, "bad number"))?,
                ));
                i = j;
            }
            '\'' => {
                if i + 2 >= b.len() || b[i + 2] != '\'' || b[i + 1] == '\'' {
                    return Err(err(line, "bad character literal"));
                }
                toks.push(Tok::Num(b[i + 1] as i32));
                i += 3;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let mut j = i;
                while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == '_') {
                    j += 1;
                }
                toks.push(Tok::Sym(b[i..j].iter().collect()));
                i = j;
            }
            '*' => {
                toks.push(Tok::Star);
                i += 1;
            }
            '(' => {
                toks.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                toks.push(Tok::RParen);
                i += 1;
            }
            '+' => {
                toks.push(Tok::Plus);
                i += 1;
            }
            '-' => {
                toks.push(Tok::Minus);
                i += 1;
            }
            '/' => {
                toks.push(Tok::Slash);
                i += 1;
            }
            '&' => {
                toks.push(Tok::Amp);
                i += 1;
            }
            '|' => {
                toks.push(Tok::Pipe);
                i += 1;
            }
            '^' => {
                toks.push(Tok::Caret);
                i += 1;
            }
            '<' => {
                if i + 1 < b.len() && b[i + 1] == '<' {
                    toks.push(Tok::Shl);
                    i += 2;
                } else {
                    toks.push(Tok::Lt);
                    i += 1;
                }
            }
            '>' if i + 1 < b.len() && b[i + 1] == '>' => {
                toks.push(Tok::Shr);
                i += 2;
            }
            _ => return Err(err(line, format!("unexpected character '{c}'"))),
        }
    }
    Ok(toks)
}

/// Recursive descent over the token list, loosest level first:
/// `| ^ & << >> + - * /`, with unary `-` and `<`.
fn parse_expr(s: &str, line: usize) -> Result<Expr, Error> {
    let mut p = Parser {
        toks: tokenize(s, line)?,
        pos: 0,
    };
    let e = p
        .bin(0)
        .ok_or_else(|| err(line, "syntax error in expression"))?;
    if p.pos != p.toks.len() {
        return Err(err(line, "trailing tokens in expression"));
    }
    Ok(e)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn bin(&mut self, level: u8) -> Option<Expr> {
        if level == 6 {
            return self.unary();
        }
        let mut lhs = self.bin(level + 1)?;
        while let Some(op) = self.toks.get(self.pos).and_then(|t| binop_at(level, t)) {
            self.pos += 1;
            let rhs = self.bin(level + 1)?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Some(lhs)
    }

    fn unary(&mut self) -> Option<Expr> {
        match self.toks.get(self.pos) {
            Some(Tok::Minus) => {
                self.pos += 1;
                Some(Expr::Neg(Box::new(self.unary()?)))
            }
            Some(Tok::Lt) => {
                self.pos += 1;
                Some(Expr::Lo(Box::new(self.unary()?)))
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Option<Expr> {
        let t = self.toks.get(self.pos).cloned()?;
        self.pos += 1;
        match t {
            Tok::Num(v) => Some(Expr::Num(v)),
            Tok::Sym(s) => Some(Expr::Sym(s)),
            Tok::Star => Some(Expr::Sym("*".into())),
            Tok::LParen => {
                let e = self.bin(0)?;
                if self.toks.get(self.pos) != Some(&Tok::RParen) {
                    return None;
                }
                self.pos += 1;
                Some(e)
            }
            _ => None,
        }
    }
}

fn binop_at(level: u8, t: &Tok) -> Option<BinOp> {
    use BinOp::*;
    Some(match (level, t) {
        (0, Tok::Pipe) => Or,
        (1, Tok::Caret) => Xor,
        (2, Tok::Amp) => And,
        (3, Tok::Shl) => Shl,
        (3, Tok::Shr) => Shr,
        (4, Tok::Plus) => Add,
        (4, Tok::Minus) => Sub,
        (5, Tok::Star) => Mul,
        (5, Tok::Slash) => Div,
        _ => return None,
    })
}

// ------------------------------------------------------------------- parsing

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Index {
    None,
    X,
    Y,
}

#[derive(Debug, PartialEq)]
enum Operand {
    None,
    Imm(Expr),
    Abs(Expr, Index),
    Izx(Expr),
    Izy(Expr),
    Ind(Expr),
    A,
}

#[derive(Debug)]
enum Stmt {
    Empty,
    Org(Expr),
    Byte(Vec<Expr>),
    Word(Vec<Expr>),
    Text(String),
    Res(Expr),
    Align(Expr),
    Equate(String, Expr),
    Instr(Op, Operand),
}

/// Drop a `;` comment, honouring double-quoted strings.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_string = !in_string,
            ';' if !in_string => return &line[..i],
            _ => {}
        }
    }
    line
}

fn parse_line(no: usize, raw: &str) -> Result<(Option<String>, Stmt), Error> {
    let t = strip_comment(raw).trim_start();
    if t.is_empty() {
        return Ok((None, Stmt::Empty));
    }
    let word_len = t
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(t.len());
    if t.starts_with(|c: char| c.is_ascii_alphabetic()) && word_len > 0 {
        let word = &t[..word_len];
        let after = &t[word_len..];
        if let Some(rest) = after.strip_prefix(':') {
            return Ok((Some(word.to_string()), parse_statement(no, rest)?));
        }
        if after.trim_start().starts_with('=') {
            let expr = parse_expr(after.trim_start()[1..].trim(), no)?;
            return Ok((None, Stmt::Equate(word.to_string(), expr)));
        }
    }
    Ok((None, parse_statement(no, t)?))
}

fn parse_statement(no: usize, s: &str) -> Result<Stmt, Error> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(Stmt::Empty);
    }
    let name_len = t
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
        .unwrap_or(t.len());
    let name = t[..name_len].to_lowercase();
    let operand = t[name_len..].trim();
    match name.as_str() {
        ".org" => Ok(Stmt::Org(parse_expr(operand, no)?)),
        ".byte" => Ok(Stmt::Byte(parse_items(operand, no)?)),
        ".word" => Ok(Stmt::Word(parse_items(operand, no)?)),
        ".text" => Ok(Stmt::Text(parse_string(operand, no)?)),
        ".res" => Ok(Stmt::Res(parse_expr(operand, no)?)),
        ".align" => Ok(Stmt::Align(parse_expr(operand, no)?)),
        n if n.starts_with('.') => Err(err(no, format!("unknown directive '{n}'"))),
        _ => {
            let Some(op) = mnemonic(&name) else {
                return Err(err(no, format!("unknown mnemonic '{name}'")));
            };
            Ok(Stmt::Instr(op, parse_operand(operand, no)?))
        }
    }
}

fn parse_items(s: &str, line: usize) -> Result<Vec<Expr>, Error> {
    if s.is_empty() {
        return Err(err(line, "directive needs at least one item"));
    }
    s.split(',').map(|item| parse_expr(item.trim(), line)).collect()
}

fn parse_string(s: &str, line: usize) -> Result<String, Error> {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        Ok(t[1..t.len() - 1].to_string())
    } else {
        Err(err(line, "expected \"quoted string\""))
    }
}

fn parse_operand(s: &str, line: usize) -> Result<Operand, Error> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(Operand::None);
    }
    if t.eq_ignore_ascii_case("a") {
        return Ok(Operand::A);
    }
    if let Some(rest) = t.strip_prefix('#') {
        return Ok(Operand::Imm(parse_expr(rest.trim(), line)?));
    }
    if t.starts_with('(') {
        let Some(close) = t.rfind(')') else {
            return Err(err(line, "unbalanced parentheses"));
        };
        let suffix = t[close + 1..].trim();
        let inner = t[1..close].trim();
        if let Some((head, idx)) = split_index(inner) {
            if idx == Index::X {
                return Ok(Operand::Izx(parse_expr(head.trim(), line)?));
            }
            return Err(err(line, "(zp,y) does not exist; only (zp,x) and (zp),y"));
        }
        return match suffix.to_lowercase().as_str() {
            "" => Ok(Operand::Ind(parse_expr(inner, line)?)),
            ",y" => Ok(Operand::Izy(parse_expr(inner, line)?)),
            _ => Err(err(line, "malformed indirect operand")),
        };
    }
    if let Some((head, idx)) = split_index(t) {
        return Ok(Operand::Abs(parse_expr(head.trim(), line)?, idx));
    }
    Ok(Operand::Abs(parse_expr(t, line)?, Index::None))
}

/// Split a trailing `,x` or `,y` (case-insensitive) off the operand text.
fn split_index(s: &str) -> Option<(&str, Index)> {
    let t = s.trim_end();
    let lower = t.to_lowercase();
    for (suffix, idx) in [(",x", Index::X), (",y", Index::Y)] {
        if lower.ends_with(suffix) {
            let cut = t.len() - 2;
            return Some((&t[..cut], idx));
        }
    }
    None
}

fn mnemonic(name: &str) -> Option<Op> {
    use Op::*;
    Some(match name {
        "lda" => Lda, "ldx" => Ldx, "ldy" => Ldy, "sta" => Sta, "stx" => Stx, "sty" => Sty,
        "adc" => Adc, "sbc" => Sbc, "and" => And, "ora" => Ora, "eor" => Eor,
        "cmp" => Cmp, "cpx" => Cpx, "cpy" => Cpy,
        "inc" => Inc, "dec" => Dec, "asl" => Asl, "lsr" => Lsr, "rol" => Rol, "ror" => Ror,
        "bit" => Bit, "jmp" => Jmp, "jsr" => Jsr, "rts" => Rts, "rti" => Rti,
        "bpl" => BrPl, "bmi" => BrMi, "bvc" => BrVc, "bvs" => BrVs,
        "bcc" => BrCc, "bcs" => BrCs, "bne" => BrNe, "beq" => BrEq,
        "inx" => Inx, "iny" => Iny, "dex" => Dex, "dey" => Dey,
        "tax" => Tax, "tay" => Tay, "txa" => Txa, "tya" => Tya, "tsx" => Tsx, "txs" => Txs,
        "pha" => Pha, "php" => Php, "pla" => Pla, "plp" => Plp,
        "clc" => Clc, "sec" => Sec, "cli" => Cli, "sei" => Sei,
        "cld" => Cld, "sed" => Sed, "clv" => Clv,
        "brk" => Brk, "nop" => Nop,
        _ => return None,
    })
}

fn is_shift(op: Op) -> bool {
    matches!(op, Op::Asl | Op::Lsr | Op::Rol | Op::Ror)
}

fn is_branch(op: Op) -> bool {
    matches!(
        op,
        Op::BrPl | Op::BrMi | Op::BrVc | Op::BrVs | Op::BrCc | Op::BrCs | Op::BrNe | Op::BrEq
    )
}

// ------------------------------------------------------------- two-pass core

enum Rec {
    Instr {
        line: usize,
        op: Op,
        mode: Mode,
        expr: Option<Expr>,
    },
    /// `.byte` / `.word` items — each keeps its expression for pass 2.
    Data {
        line: usize,
        items: Vec<(DataItem, Expr)>,
    },
    /// `.text` / `.res` / `.align` — fully resolved during pass 1.
    Raw(Vec<u8>),
}

#[derive(Clone, Copy)]
enum DataItem {
    Byte,
    Word,
}

/// Pick the addressing mode now, once, so byte counts are frozen before any
/// operand is fully evaluated. Symbol-bearing operands always go absolute;
/// `<expr` forces the zero-page family; fitting literals use zero page when
/// the opcode exists there, otherwise absolute.
fn decide_mode(
    line: usize,
    op: Op,
    operand: &Operand,
    syms: &HashMap<String, i32>,
    pc: u32,
) -> Result<(Mode, Option<Expr>, u32), Error> {
    let size = |mode: Mode| -> u32 {
        u32::from(instruction_len(mode))
    };
    let require = |mode: Mode| -> Result<Mode, Error> {
        encode(op, mode)
            .map(|_| mode)
            .ok_or_else(|| err(line, "no such addressing mode for this instruction"))
    };
    let (mode, expr) = match operand {
        Operand::None => {
            let m = if is_shift(op) { Mode::Acc } else { Mode::Imp };
            (require(m)?, None)
        }
        Operand::A if is_shift(op) => (require(Mode::Acc)?, None),
        Operand::Imm(e) => (require(Mode::Imm)?, Some(e.clone())),
        Operand::Ind(e) if op == Op::Jmp => (require(Mode::Ind)?, Some(e.clone())),
        Operand::Ind(_) => return Err(err(line, "(abs) indirect is only valid on jmp")),
        Operand::Izx(e) => (require(Mode::Izx)?, Some(e.clone())),
        Operand::Izy(e) => (require(Mode::Izy)?, Some(e.clone())),
        Operand::Abs(e, idx) => {
            if is_branch(op) {
                if *idx != Index::None {
                    return Err(err(line, "branches take a plain target, not an index"));
                }
                (require(Mode::Rel)?, Some(e.clone()))
            } else if matches!(e, Expr::Lo(_)) {
                let m = match idx {
                    Index::None => Mode::Zp,
                    Index::X => Mode::Zpx,
                    Index::Y => Mode::Zpy,
                };
                (require(m)?, Some(e.clone()))
            } else {
                let fits_zp = !has_sym(e)
                    && matches!(resolve(e, syms, pc), Some(0..=0xFF));
                if fits_zp {
                    let zp = match idx {
                        Index::None => Mode::Zp,
                        Index::X => Mode::Zpx,
                        Index::Y => Mode::Zpy,
                    };
                    if encode(op, zp).is_some() {
                        (zp, Some(e.clone()))
                    } else {
                        let abs = match idx {
                            Index::None => Mode::Abs,
                            Index::X => Mode::Abx,
                            Index::Y => Mode::Aby,
                        };
                        (require(abs)?, Some(e.clone()))
                    }
                } else {
                    let abs = match idx {
                        Index::None => Mode::Abs,
                        Index::X => Mode::Abx,
                        Index::Y => Mode::Aby,
                    };
                    (require(abs)?, Some(e.clone()))
                }
            }
        }
        Operand::A => {
            return Err(err(line, "'a' operand is only valid on shifts"))
        }
    };
    Ok((mode, expr, size(mode)))
}

/// Parse `src` into a loadable binary. Fails with a 1-based line number.
pub fn assemble(src: &str) -> Result<Binary, Error> {
    // ---- parse all lines first; errors here need no symbol context
    let mut lines = Vec::new();
    for (i, raw) in src.lines().enumerate() {
        let (label, stmt) = parse_line(i + 1, raw)?;
        lines.push((i + 1, label, stmt));
    }

    // ---- pass 1: define symbols, fix every instruction's size
    let mut syms: HashMap<String, i32> = HashMap::new();
    let mut recs: Vec<(u32, Rec)> = Vec::new();
    let mut pc: u32 = 0;

    for &(no, ref label, ref stmt) in &lines {
        if let Some(name) = label {
            if syms.contains_key(name) {
                return Err(err(no, format!("duplicate symbol '{name}'")));
            }
            syms.insert(name.clone(), pc as i32);
        }
        match stmt {
            Stmt::Empty => {}
            Stmt::Equate(name, e) => {
                if syms.contains_key(name) {
                    return Err(err(no, format!("duplicate symbol '{name}'")));
                }
                let v = resolve(e, &syms, pc)
                    .ok_or_else(|| err(no, "equate references symbols not yet defined"))?;
                syms.insert(name.clone(), v);
            }
            Stmt::Org(e) => {
                pc = resolve(e, &syms, pc)
                    .ok_or_else(|| err(no, ".org needs a resolvable address"))? as u32;
            }
            Stmt::Instr(op, operand) => {
                let (mode, expr, size) = decide_mode(no, *op, operand, &syms, pc)?;
                recs.push((
                    pc,
                    Rec::Instr {
                        line: no,
                        op: *op,
                        mode,
                        expr,
                    },
                ));
                pc += size;
            }
            Stmt::Byte(items) => {
                recs.push((
                    pc,
                    Rec::Data {
                        line: no,
                        items: items.iter().map(|e| (DataItem::Byte, e.clone())).collect(),
                    },
                ));
                pc += items.len() as u32;
            }
            Stmt::Word(items) => {
                recs.push((
                    pc,
                    Rec::Data {
                        line: no,
                        items: items.iter().map(|e| (DataItem::Word, e.clone())).collect(),
                    },
                ));
                pc += items.len() as u32 * 2;
            }
            Stmt::Text(s) => {
                recs.push((pc, Rec::Raw(s.bytes().collect())));
                pc += s.len() as u32;
            }
            Stmt::Res(e) => {
                let n = resolve(e, &syms, pc)
                    .ok_or_else(|| err(no, ".res needs a resolvable count"))?;
                if n < 0 {
                    return Err(err(no, ".res count is negative"));
                }
                recs.push((pc, Rec::Raw(vec![0; n as usize])));
                pc += n as u32;
            }
            Stmt::Align(e) => {
                let n = resolve(e, &syms, pc)
                    .ok_or_else(|| err(no, ".align needs a resolvable value"))?;
                if n <= 0 {
                    return Err(err(no, ".align must be positive"));
                }
                let pad = (n - (pc as i32 % n)) % n;
                recs.push((pc, Rec::Raw(vec![0; pad as usize])));
                pc += pad as u32;
            }
        }
        if pc > 0x1_0000 {
            return Err(err(no, "program runs past $FFFF"));
        }
    }

    // ---- pass 2: emit bytes with the complete symbol table
    let mut segments: Vec<(u16, Vec<u8>)> = Vec::new();
    for (addr, rec) in recs {
        let mut a = addr as usize;
        let put = |a: usize, b: u8, segments: &mut Vec<(u16, Vec<u8>)>| {
            if let Some(last) = segments.last_mut() {
                if usize::from(last.0) + last.1.len() == a {
                    last.1.push(b);
                    return;
                }
            }
            segments.push((a as u16, vec![b]));
        };
        match rec {
            Rec::Raw(bytes) => {
                for b in bytes {
                    put(a, b, &mut segments);
                    a += 1;
                }
            }
            Rec::Data { line, items } => {
                for (kind, e) in items {
                    let v = resolve(&e, &syms, a as u32).ok_or_else(|| {
                        err(line, "undefined symbol or unresolvable expression")
                    })?;
                    match kind {
                        DataItem::Byte => {
                            put(a, v as u8, &mut segments);
                            a += 1;
                        }
                        DataItem::Word => {
                            put(a, v as u8, &mut segments);
                            put(a + 1, (v >> 8) as u8, &mut segments);
                            a += 2;
                        }
                    }
                }
            }
            Rec::Instr {
                line,
                op,
                mode,
                expr,
            } => {
                // `encode` was validated in pass 1, so this cannot fail.
                put(addr as usize, encode(op, mode).unwrap(), &mut segments);
                let operand_at = addr as usize + 1;
                let operand = || -> Result<i32, Error> {
                    resolve(expr.as_ref().unwrap(), &syms, addr)
                        .ok_or_else(|| err(line, "undefined symbol or unresolvable expression"))
                };
                match mode {
                    Mode::Imp | Mode::Acc => {}
                    Mode::Imm | Mode::Zp | Mode::Zpx | Mode::Zpy | Mode::Izx | Mode::Izy => {
                        put(operand_at, operand()? as u8, &mut segments);
                    }
                    Mode::Abs | Mode::Abx | Mode::Aby | Mode::Ind => {
                        let v = operand()?;
                        put(operand_at, v as u8, &mut segments);
                        put(operand_at + 1, (v >> 8) as u8, &mut segments);
                    }
                    Mode::Rel => {
                        let target = operand()?;
                        let off = target - (addr as i32 + 2);
                        if !(-128..=127).contains(&off) {
                            return Err(err(line, "branch target out of range"));
                        }
                        put(operand_at, off as u8, &mut segments);
                    }
                }
            }
        }
    }
    Ok(Binary { segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asm(src: &str) -> Vec<u8> {
        assemble(src).unwrap().segments[0].1.clone()
    }

    #[test]
    fn immediate() {
        assert_eq!(asm("lda #$41"), vec![0xA9, 0x41]);
    }

    #[test]
    fn zero_page_from_literal() {
        assert_eq!(asm("lda $10"), vec![0xA5, 0x10]);
        assert_eq!(asm("ldx $10"), vec![0xA6, 0x10]);
        assert_eq!(asm("sta $20,x"), vec![0x95, 0x20]);
    }

    #[test]
    fn absolute_is_little_endian() {
        assert_eq!(asm("lda $1234"), vec![0xAD, 0x34, 0x12]);
        assert_eq!(asm("jmp $1234"), vec![0x4C, 0x34, 0x12]);
        assert_eq!(asm("jsr $1234"), vec![0x20, 0x34, 0x12]);
    }

    #[test]
    fn symbolic_operands_are_absolute() {
        let bin = assemble("ptr = $1234\n lda ptr").unwrap();
        assert_eq!(bin.segments[0].1, vec![0xAD, 0x34, 0x12]);
    }

    #[test]
    fn low_byte_operator_forces_zero_page() {
        let bin = assemble("ptr = $1234\n lda <ptr").unwrap();
        assert_eq!(bin.segments[0].1, vec![0xA5, 0x34]);
    }

    #[test]
    fn missing_mode_falls_back_to_absolute() {
        // LDA has no zp,Y form, so `lda $12,y` must become absolute,Y; LDX
        // does have zp,Y and uses it.
        assert_eq!(asm("lda $12,y"), vec![0xB9, 0x12, 0x00]);
        // LDX does have zp,Y.
        assert_eq!(asm("ldx $12,y"), vec![0xB6, 0x12]);
    }

    #[test]
    fn indexed_absolute() {
        assert_eq!(asm("lda $1234,x"), vec![0xBD, 0x34, 0x12]);
        assert_eq!(asm("sta $1234,y"), vec![0x99, 0x34, 0x12]);
    }

    #[test]
    fn indirect_modes() {
        assert_eq!(asm("lda ($10,x)"), vec![0xA1, 0x10]);
        assert_eq!(asm("lda ($10),y"), vec![0xB1, 0x10]);
        assert_eq!(asm("jmp ($1234)"), vec![0x6C, 0x34, 0x12]);
    }

    #[test]
    fn implied_and_accumulator() {
        assert_eq!(asm("inx"), vec![0xE8]);
        assert_eq!(asm("asl"), vec![0x0A]);
        assert_eq!(asm("asl a"), vec![0x0A]);
    }

    #[test]
    fn branch_forward_and_backward() {
        let bin = assemble(" beq end\n nop\nend: nop").unwrap();
        assert_eq!(bin.segments[0].1, vec![0xF0, 0x01, 0xEA, 0xEA]);

        let bin = assemble("top: nop\n beq top").unwrap();
        assert_eq!(bin.segments[0].1, vec![0xEA, 0xF0, 0xFD]);
    }

    #[test]
    fn branch_out_of_range_fails() {
        let err = assemble(" beq far\n .res 200\nfar: nop").unwrap_err();
        assert_eq!(err.line, 1);
    }

    #[test]
    fn labels_and_forward_references() {
        let bin = assemble(" jmp start\n .res 3\nstart: nop").unwrap();
        assert_eq!(bin.segments[0].1[0..3], [0x4C, 0x06, 0x00]);
        assert_eq!(bin.segments[0].1.len(), 7);
    }

    #[test]
    fn origin_directive_sets_base() {
        let bin = assemble(" .org $C000\n nop").unwrap();
        assert_eq!(bin.segments[0].0, 0xC000);
        assert_eq!(bin.segments[0].1, vec![0xEA]);
    }

    #[test]
    fn byte_word_text_res_align() {
        let bin = assemble(
            " .byte 1,2,$03\n .word $1234, label\n .text \"hi\"\n .res 2\n .align 8\nlabel: nop",
        )
        .unwrap();
        assert_eq!(
            bin.segments[0].1,
            vec![
                0x01, 0x02, 0x03, // .byte
                0x34, 0x12, 0x10, 0x00, // .word $1234, label($0010)
                b'h', b'i', // .text
                0x00, 0x00, // .res 2
                0x00, 0x00, 0x00, 0x00, 0x00, // .align 8: 5 pad bytes to $10
                0xEA,
            ]
        );
    }

    #[test]
    fn expressions_with_precedence() {
        let bin = assemble(" lda #(2+3*4)").unwrap();
        assert_eq!(bin.segments[0].1, vec![0xA9, 0x0E]);
        let bin = assemble(" lda #((2+3)*4)").unwrap();
        assert_eq!(bin.segments[0].1, vec![0xA9, 0x14]);
        let bin = assemble(" lda #'A'+1").unwrap();
        assert_eq!(bin.segments[0].1, vec![0xA9, 0x42]);
    }

    #[test]
    fn multiple_origin_segments() {
        let bin = assemble(" .org $C000\n nop\n .org $FFFC\n .word $C000").unwrap();
        assert_eq!(bin.segments.len(), 2);
        assert_eq!(bin.segments[0], (0xC000, vec![0xEA]));
        assert_eq!(bin.segments[1], (0xFFFC, vec![0x00, 0xC0]));
    }

    #[test]
    fn unknown_mnemonic_fails_with_line() {
        let err = assemble(" lda #$41\n bogus").unwrap_err();
        assert_eq!(err.line, 2);
    }

    #[test]
    fn undefined_symbol_fails_with_line() {
        let err = assemble(" lda nowhere").unwrap_err();
        assert_eq!(err.line, 1);
    }

    #[test]
    fn comments_and_case_insensitivity() {
        let bin = assemble(" ; leading comment\n LDA #$41 ; trailing\n NOP").unwrap();
        assert_eq!(bin.segments[0].1, vec![0xA9, 0x41, 0xEA]);
    }

    #[test]
    fn current_address_symbol() {
        let bin = assemble(" here = *\n .byte here, here+1").unwrap();
        assert_eq!(bin.segments[0].1, vec![0x00, 0x01]);
    }
}