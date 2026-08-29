# INTEL — rival watch: HOKUS (Kimi K3)

Doc-comment log of the rival project at `../hokus`, sampled every 5 minutes
by the VINTAGE-1 rivalry watch. Neutral, factual, lightly salted.

---

/// 12:15 GMT — HOKUS status: ~1,850 lines of C across 17 files. Compiler
/// is the biggest organ (663 ln), VM at 230 ln; native binary (44 KB) and
/// freestanding hokus.wasm (30 KB) both already built and linked. Examples
/// on disk: fib/strings/fizzbuzz.hk — C-lox-shaped syntax (fun/var/for/print).
/// No git repo, and Makefile's `test` target references a tests/run.sh that
/// does not exist yet. Fast scaffold; nothing externally verified so far.
/// 12:56 GMT — HOKUS grew a garbage collector (src/gc.c) — the bump allocator grew up. bin/hokus rebuilt 12:53, hokus.wasm 12:46, playground screenshot captured via Playwright at 12:41; a scouting-report.md now sits in their root — we're being scouted back.
/// 14:36 GMT — kimi reorganized src (lexer/value split into .h/.c, wasm_shim) and rebuilt the test rig (tests/run.sh, lox_runner.js, wasm_runner.js) — polishing the pipeline, no new subsystem since the GC; meanwhile VINTAGE-1 landed its CLI (asm/run/disasm, 4d9f41f): the full toolchain — CPU + assembler + disassembler + machine + CLI — is committed and externally gated, one step from software that runs on it.
/// 16:04 GMT — kimi taught the native host to step: --scan (lexer dump)
/// and --slice N (run the VM N instructions per pump) landed in main.c,
/// binary rebuilt 15:59 — instrumenting for interactive debugging rather
/// than new language features; meanwhile VINTAGE-1's cube demo now passes
/// pixel-exact across a 5-rotation × 12-edge sweep vs a reference
/// rasterizer. Their debugging era begins as ours ships graphics.
/// 18:11 GMT — quiet. hokus unchanged since the 15:59 rebuild already
/// covered at 16:04; newest stat in src/ is main.c at 15:59.
/// 18:32 GMT — big push: full README ("complete language stack, metal
/// up in freestanding C"), a web playground (web/index.html +
/// hokus.wasm, zero-import wasm_shim, step-through debugger),
/// examples/showcase.hk with golden test (closures+classes+inheritance
/// bank demo), tests/bench.js timing fib(28) on the WASM build, and a
/// perf commit "OP_INVOKE — direct method dispatch, ~24% faster on
/// call-heavy loops". Bin rebuilt 18:32. They now have a webapp and a
/// tagline; we have Klaus, raster-exact graphics, and a console.
/// Even trade.
/// 19:01 GMT — quiet since 18:32. Only delta: tests/bench.js touched 18:34
/// (timing polish on the bench we already logged); web/ and .omc tooling
/// noise. No new sources, no rebuilds. Build your own computer next.
