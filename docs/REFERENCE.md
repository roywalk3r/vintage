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
| `$E000–$FFFF`  | ROM window (8K; a banked cartridge maps one of N 8K banks here, vectors at the top)  |

Frame timing: **2 MHz → 33,333 cycles/frame @ 60 Hz**. The host runs
`run_frame` at vsync: a **maskable IRQ** is latched once per frame and taken
at the first instruction boundary with I clear (vector `$FFFE`; the handler
runs with I set and `rti` restores the interrupted context). A program
holding `sei` never sees the pulse, and it is not re-issued later in the
frame. Programs without `cli` never notice; all demos ship with I set.
There is no NMI source.

## I/O bank ($5800–$5FFF)

Only the registers below exist; every other address in the bank reads `$00`
and drops writes.

| Addr    | Name    | R/W | Behavior |
|---------|---------|-----|----------|
| `$5800` | KEY     | R   | Latest keypress, **read clears**. Codes: `$11` up, `$12` down, `$13` left, `$14` right. |
| `$5802` | FRAME   | R   | frame counter, low byte (wraps after ~18 min at 60 Hz) |
| `$5803` | FRAME+1 | R   | frame counter, high byte |
| `$5804` | PALETTE | R/W | bit 7 = 0: 1bpp, low bits select phosphor color (0 = green, 1 = amber, 2 = white). bit 7 = 1: **2bpp color mode** — 128×192 fat pixels, 4 per byte; low 2 bits select the four-color scheme (0 = green, 1 = amber, 2 = white, 3 = mixed). |
| `$5805` | RANDOM  | R   | 16-bit Galois LFSR (taps 0x002D); **every read steps it** |
| `$5806` | BANK    | R/W | current cartridge bank (0–255). Writing a value **and the next fetch already comes from the new bank**; continuation code must exist in the target bank at the same PC, or execution must jump before switching. Read returns the current bank. The window stays write-protected. |
| `$5807` | BEEPER  | R/W | square-wave half-period in CPU cycles; **0 = silence**. Frequency = 120,000 / (2 × period) Hz. |
| `$5808/$5809` | SPR0_X/Y | R/W | sprite 0 position, 0-based top-left pixel |
| `$580A/$580B` | SPR0_PAT | R/W | sprite 0 pattern pointer (lo/hi); 8 pattern bytes are **fetched through the bus** at vsync — one byte per row, MSB leftmost |
| `$580C/$580D` | SPR1_X/Y | R/W | sprite 1 x/y latches |
| `$580E/$580F` | SPR1_PAT | R/W | sprite 1 pattern pointer |
| `$5810` | SPR_CTRL | R/W | bit 0 enables sprite 0, bit 1 enables sprite 1 |

Sprite composition: at each vsync the host composits both sprites over a
fresh copy of the framebuffer — set bit = XOR. The fb itself stays
background-only (not modified by sprites, so a static sprite never blinks),
and `fb()` — what hosts render — is the composited result. In 2bpp a sprite
hit **inverts** the covered fat pixel's palette index (index ^= 3), so
sprites display brightest. Sprites clip at the video edges (x ≥ 128 fat
pixels in 2bpp, x ≥ 256 in 1bpp; y ≥ 192): no wraparound. Patterns are
exactly 8 bytes, fetched through the bus so a sprite can be defined
anywhere (RAM or ROM).

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

Banked cartridges use **V1B**: same `V1` prefix, third byte `'B'`, a u16
bank count, then per bank the same segment list as V1 (bank 0 first). The
console and `vintage run` load every bank into the cartridge; `$5806` picks
the visible one. `vintage asm` picks the container flavor automatically.

## Save states

`save .vst` / `load .vst` in the console produce and consume **.vst files**:
`V1S` magic, then CPU registers (A, X, Y, S, P, PC, cycle count), the full
54 KB of RAM (both planes), the 6K framebuffer, the cartridge banks, bank
select, sprite latches + ctrl, frame counter, palette, LFSR seed, beeper
period, pending key, and the vsync latch. The banked RAM is included, so a
`.vst` is a self-contained resumable machine: load it into the console with
no companion ROM. Execution resumes cycle-exact. A malformed file is
rejected before any field is assigned — it can never half-restore.