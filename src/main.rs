//! VINTAGE-1
//! Author: roywalk3r
//! Repo: https://github.com/roywalk3r/vintage
//! License: MIT
//! VINTAGE-1 toolchain CLI: assemble, run, disassemble.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vintage::asm::{assemble, Binary, Error};
use vintage::cpu::{Bus, Cpu};
use vintage::dis::disasm_one;
use vintage::machine::Machine;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let Some(cmd) = args.first().cloned() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let result = match cmd.as_str() {
        "asm" => cmd_asm(&args[1..]),
        "run" => cmd_run(&args[1..]),
        "disasm" => cmd_disasm(&args[1..]),
        _ => Err(format!("unknown command '{cmd}'\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("vintage: {msg}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "VINTAGE-1 toolchain

usage:
  vintage asm <file.s> [-o <file.vin>]     assemble to a V1 segment container
  vintage run <file.s> [--frames N] [--ppm <file>]
                                            assemble, run N frames (default 120),
                                            dump the framebuffer as PPM
  vintage disasm <file.bin> [--base $ADDR]  list instructions (default base 0)";

struct Opts {
    positional: Vec<PathBuf>,
    flags: std::collections::HashMap<&'static str, String>,
}

/// Tiny flag parser: `--frames 30`, `--ppm out.ppm`, `-o out.vin`.
fn parse_opts(args: &[String], value_flags: &[&'static str]) -> Result<Opts, String> {
    let mut opts = Opts {
        positional: Vec::new(),
        flags: std::collections::HashMap::new(),
    };
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-o" || value_flags.iter().any(|f| a == *f) {
            let key = if a == "-o" {
                "-o"
            } else {
                value_flags.iter().find(|f| a == *f).unwrap()
            };
            let Some(v) = args.get(i + 1) else {
                return Err(format!("{a} needs a value"));
            };
            opts.flags.insert(key, v.clone());
            i += 2;
        } else if a.starts_with('-') {
            return Err(format!("unknown option {a}"));
        } else {
            opts.positional.push(PathBuf::from(a));
            i += 1;
        }
    }
    Ok(opts)
}

fn input(opts: &Opts, cmd: &str) -> Result<PathBuf, String> {
    opts.positional
        .first()
        .cloned()
        .ok_or_else(|| format!("{cmd} needs an input file\n{USAGE}"))
}

fn die(e: Error) -> String {
    format!("line {}: {}", e.line, e.msg)
}

fn cmd_asm(args: &[String]) -> Result<(), String> {
    let opts = parse_opts(args, &[])?;
    let src_path = input(&opts, "asm")?;
    let src = fs::read_to_string(&src_path).map_err(|e| format!("{}: {e}", src_path.display()))?;
    let bin = assemble(&src).map_err(die)?;

    let out_path = opts.flags.get("-o").map_or_else(
        || {
            if bin.extra_banks.is_empty() {
                src_path.with_extension("vin")
            } else {
                src_path.with_extension("v1b")
            }
        },
        PathBuf::from,
    );
    let banks = write_container(&out_path, &bin).map_err(|e| format!("{}: {e}", out_path.display()))?;
    println!(
        "wrote {} ({} segment{}, {} bank{})",
        out_path.display(),
        bin.segments.len(),
        if bin.segments.len() == 1 { "" } else { "s" },
        banks,
        if banks == 1 { "" } else { "s" }
    );
    Ok(())
}

/// V1 container: magic "V1", u16 segment count, then per segment
/// u16 address, u16 length, bytes. V1B adds cartridge banks: magic "V1B",
/// u16 bank count, then per bank the same segment list as V1. Bank 0 first.
/// All little-endian. Returns the bank count for the CLI's report line.
fn write_container(path: &Path, bin: &Binary) -> std::io::Result<usize> {
    let mut banks: Vec<&Vec<(u16, Vec<u8>)>> = vec![&bin.segments];
    banks.extend(bin.extra_banks.iter());
    let mut out = Vec::new();
    if banks.len() == 1 {
        out.extend_from_slice(b"V1");
        out.extend_from_slice(&(bin.segments.len() as u16).to_le_bytes());
        for (addr, bytes) in &bin.segments {
            out.extend_from_slice(&addr.to_le_bytes());
            out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
            out.extend_from_slice(bytes);
        }
    } else {
        out.extend_from_slice(b"V1B");
        out.extend_from_slice(&(banks.len() as u16).to_le_bytes());
        for bank in banks.iter() {
            out.extend_from_slice(&(bank.len() as u16).to_le_bytes());
            for (addr, bytes) in bank.iter() {
                out.extend_from_slice(&addr.to_le_bytes());
                out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
                out.extend_from_slice(bytes);
            }
        }
    }
    fs::write(path, out)?;
    Ok(banks.len())
}

fn cmd_run(args: &[String]) -> Result<(), String> {
    let opts = parse_opts(args, &["--frames", "--ppm"])?;
    let src_path = input(&opts, "run")?;
    let src = fs::read_to_string(&src_path).map_err(|e| format!("{}: {e}", src_path.display()))?;
    let bin = assemble(&src).map_err(die)?;

    let frames: u32 = match opts.flags.get("--frames") {
        Some(v) => v.parse().map_err(|_| format!("bad --frames {v}"))?,
        None => 120,
    };

    // Cartridge banks from $E000–$FFFF segments; non-ROM segments are
    // poked into RAM through the bus once the machine exists.
    let mut rom = [0u8; 0x2000];
    let mut ram_pokes: Vec<(u16, Vec<u8>)> = Vec::new();
    for &(addr, ref bytes) in &bin.segments {
        match addr {
            0xE000..=0xFFFF => copy_bank_segment(&mut rom, addr, bytes)?,
            _ => ram_pokes.push((addr, bytes.clone())),
        }
    }
    let mut banks: Vec<[u8; 0x2000]> = vec![rom];
    for seg in &bin.extra_banks {
        banks.push(rom_image(seg)?);
    }
    let mut machine = Machine::with_banks(banks);
    for (addr, bytes) in &ram_pokes {
        for (i, b) in bytes.iter().enumerate() {
            machine.write(addr + i as u16, *b);
        }
    }

    let mut cpu = Cpu::new();
    cpu.reset(&mut machine);
    for _ in 0..frames {
        machine.run_frame(&mut cpu);
    }

    let ppm_path = opts.flags.get("--ppm").map_or_else(
        || src_path.with_extension("ppm"),
        PathBuf::from,
    );
    write_ppm(&ppm_path, machine.fb(), machine.palette())
        .map_err(|e| format!("{}: {e}", ppm_path.display()))?;
    println!(
        "wrote {} after {} frames ({} cycles)",
        ppm_path.display(),
        frames,
        cpu.cycles
    );
    Ok(())
}

/// Map one bank's $E000–$FFFF segments into a full 8K image.
fn rom_image(segments: &[(u16, Vec<u8>)]) -> Result<[u8; 0x2000], String> {
    let mut img = [0u8; 0x2000];
    for (addr, bytes) in segments {
        copy_bank_segment(&mut img, *addr, bytes)?;
    }
    Ok(img)
}

fn copy_bank_segment(img: &mut [u8; 0x2000], addr: u16, bytes: &[u8]) -> Result<(), String> {
    let end = addr as usize + bytes.len();
    if addr < 0xE000 || end > 0x1_0000 {
        return Err(format!("segment ${addr:04X} is outside the $E000-$FFFF window"));
    }
    img[addr as usize - 0xE000..end - 0xE000].copy_from_slice(bytes);
    Ok(())
}

/// Phosphor palettes: (on-colour, off-colour) RGB triples.
fn palette(p: u8) -> ([(u8, u8, u8); 2], u8) {
    const MAX: u8 = 3;
    match p % MAX {
        0 => ([(51, 255, 51), (6, 12, 6)], 255),
        1 => ([(255, 176, 0), (16, 12, 0)], 255),
        _ => ([(230, 230, 230), (10, 10, 10)], 230),
    }
}

fn write_ppm(path: &Path, fb: &[u8], p: u8) -> std::io::Result<()> {
    let (colors, maxval) = palette(p);
    let mut out = format!("P6\n256 192\n{maxval}\n").into_bytes();
    out.reserve(256 * 192 * 3);
    for &byte in fb {
        for bit in (0..8).rev() {
            let c = colors[usize::from((byte >> bit) & 1 == 0)];
            out.extend_from_slice(&[c.0, c.1, c.2]);
        }
    }
    fs::write(path, out)
}

fn cmd_disasm(args: &[String]) -> Result<(), String> {
    let opts = parse_opts(args, &["--base"])?;
    let path = input(&opts, "disasm")?;
    let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let base: u16 = match opts.flags.get("--base") {
        Some(v) => parse_hex(v).ok_or_else(|| format!("bad --base {v}"))?,
        None => 0,
    };

    let mut pc = base;
    let mut i = 0;
    while i < bytes.len() {
        if let Some((len, text)) = disasm_one(&bytes[i..], pc) {
            let len = len as usize;
            let raw: Vec<String> = bytes[i..i + len].iter().map(|b| format!("{b:02X}")).collect();
            println!("${pc:04X}  {:<8} {text}", raw.join(" "));
            i += len;
            pc = pc.wrapping_add(len as u16);
        } else {
            println!("${pc:04X}  {:<8} .byte ${:02X}", format!("{:02X}", bytes[i]), bytes[i]);
            i += 1;
            pc = pc.wrapping_add(1);
        }
    }
    Ok(())
}

/// `$C000`, `0xC000`, or bare `C000` → 0xC000.
fn parse_hex(s: &str) -> Option<u16> {
    let t = s
        .trim()
        .trim_start_matches('$')
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    u16::from_str_radix(t, 16).ok()
}