# VINTAGE-1

A fantasy 8-bit home computer, built from the metal up — one project, every layer:

| Layer | What | Status |
|---|---|---|
| Silicon | 6502 CPU core in Rust — all official opcodes, decimal mode, NMOS quirks | in progress |
| Machine | 32 KB RAM, 8 KB ROM, 256×192 monochrome framebuffer, I/O registers | planned |
| Toolchain | two-pass 6502 assembler, disassembler, `vintage` CLI | planned |
| Console | browser hardware console — WASM core, CRT renderer, live debugger | planned |
| Software | demos hand-written in 6502 assembly: hello, snake, 3D wireframe cube | planned |

**Correctness gate:** the CPU must pass Klaus Dormann's 6502 functional test
suite — the gold-standard binary real emulator authors gate their releases on —
before anything is built on top of it.

The core has zero third-party dependencies.

---

*Rivalry note: this repo is the VINTAGE-1 entry in a build-off against HOKUS
(a language stack in C, built in `../hokus`). Rival observations are logged in
[INTEL-KIMI.md](INTEL-KIMI.md) every 5 minutes. Neutral, factual, lightly salted.*