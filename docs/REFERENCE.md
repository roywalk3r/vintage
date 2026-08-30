# VINTAGE-1 Programmer's Reference

The complete contract between you and the machine: memory map, I/O registers,
the assembler dialect, program shapes, and the toolchain.

## Memory map

| Range          | Region                        |
|----------------|-------------------------------|
| `$0000–$3FFF`  | RAM (16K)                     |
| `$4000–$57FF`  | framebuffer (6K)              |
| `$5800–$5FFF`  | I/O bank                      |
| `$6000–$DFFF`  | RAM (32K)                     |
| `$E000–$FFFF`  | ROM (8K, vectors at the top)  |

Frame timing: **2 MHz → 33,333 cycles/frame @ 60 Hz**. The host runs
`run_frame` once per video frame; a game loop must either rely on the frame
counter for pacing or spend the full budget in `run_frame`.

## I/O bank ($5800–$5FFF)

Only the registers below exist; every other address in the bank reads `$00`
and drops writes.

| Addr    | Name    | R/W | Behavior |
|---------|---------|-----|----------|
| `$5800` | KEY     | R   | Latest keypress, **read clears**. Codes: `$11` up, `$12` down, `$13` left, `$14` right. |
| `$5802` | FRAME   | R   | frame counter, low byte (wraps after ~18 min at 60 Hz) |
| `$5803` | FRAME+1 | R   | frame counter, high byte |
| `$5804` | PALETTE | R/W | phosphor color: 0 = green, 1 = amber, 2 = white |
| `$5805` | RANDOM  | R   | 16-bit Galois LFSR (taps 0x002D); **every read steps it** |
| `$5807` | BEEPER  | R/W | square-wave half-period in CPU cycles; **0 = silence**. Frequency = 120,000 / (2 × period) Hz. |

## Assembler dialect

Two-pass, deterministic sizing: operand shape picks the addressing mode, and
`<expr` forces zero-page sizing. Expressions support `+ - * / & | ^ << >>`
with C-like precedence, unary minus, and undelimited literals — `$` hex, `%`
binary, `'c'` char literals.

| Directive | Meaning |
|-----------|---------|
| `.org $E000`  | set the assembly address |
| `.byte 1, $FF` | emit expression values, one byte each |
| `.word start` | emit little-endian 16-bit values |
| `.text "hello"` | emit string bytes (no terminator) |
| `.res 16`     | skip n bytes (reserves, emits nothing) |
| `.align 256`  | advance to the next multiple of n |
| `NAME = expr` | define an equate |

Comments start with `;`.

## Program shape

A ROM is an 8K image mapped at `$E000`–$FFFF. The reset vector at `$FFFC`
points at your code:

```asm
 .org $E000
start:
 lda #0
sta  ...

 .org $FFFC
 .word start
```

Demos live in `software/*.s` — `hello.s` (text via 8×8 font table), `snake.s`
(playable), `cube.s` (wireframe, uses `.align`), `tune.s` (beeper melody).
Start from whichever is closest to what you want.

## Toolchain

```sh
cargo run -- asm software/snake.s -o out.vin                     # to V1 container
cargo run -- run software/snake.s --frames 60 --ppm out.ppm      # headless run
cargo run -- disasm out.vin --base $E000                         # disassemble
```

A `.vin` file is `V1` magic, a segment count, then `(addr:u16, len:u16,
payload)` segments — enough to relink a scatter-loaded ROM.