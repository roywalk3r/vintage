# VINTAGE-1 Roadmap

Where the machine is, and where it's going. Ground truth for "done": a
feature is shipped when it has a regression test, appears in the console or
the reference, and lives on `main`.

## Shipped

- **CPU** — full NMOS 6502, 151-opcode encode/decode inverse table, gated on
  Klaus Dormann's functional test suite (the whole point: correctness proven
  by an external judge, not by my own tests).
- **Assembler** — two-pass, deterministic zero-page/absolute sizing,
  expression grammar, round-trips with the disassembler.
- **Disassembler** — full assembler roundtrip on every official opcode.
- **Machine** — VINTAGE-1 memory map: 48K RAM, 6K framebuffer, I/O bank
  (keyboard, frame counter, palette, LFSR random, beeper).
- **CLI** — `vintage asm / run / disasm`, headless PPM framebuffer dumps for
  tests.
- **Web console** — wasm build of the whole machine; canvas blit, CPU panel,
  cycle-budget slider, ROM file loading, demo buttons.
- **Demos** — hello (8×8 font), snake (playable), cube (3D wireframe),
  tune (32-note beeper melody, actual audio in the console).
- **Audio** — `$5807` beeper: square-wave voice in the console, unlocked on
  first gesture; machine-level test samples the register across frames.
- **Palette** — `$5804` selects green/amber/white phosphor in console and
  PPM renderer alike.
- **Docs** — `docs/REFERENCE.md` (memory map, I/O, assembler dialect),
  README quickstart.

## Next (in order)

1. **Breakout** — the first demo that uses everything at once: keyboard,
   framebuffer rects, ball physics, and per-event beeps. Also the first
   demo with a machine-level behavioral test (inject keys, watch the beeper
   and brick bitmask change).
2. **Cycle-count discipline** — per-frame cycle accounting surfaced in the
   console (the budget slider exists; make the cost visible per demo).
3. **Disk / storage** — a simple cartridge-bank format for multi-load
   programs (`.vin` already has segments; extend to banked ROMs).
4. **Interrupts** — real NMI/IRQ vectors wired to a vsync flag, so games can
   sleep on hardware instead of polling the frame counter.

## Later / speculative

- Sprites: a scanline sprite unit in `machine.rs` (2 hardware sprites with
  x/y latches) — the next hardware step beyond 1bpp framebuffers.
- Color mode: 2bpp framebuffer variant selected by `$5804` high bit.
- Cassette-style save states: serialize `Machine` + `Cpu` to a `.vst` file
  for the console.
- In-browser assembler: paste source, run — closes the loop on the console
  without installing Rust.

## Non-goals

- Cycle-exact video timing emulation (the frame-budget model is the spec).
- Binary-compatible anything — VINTAGE-1 is deliberately its own machine.