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
/// 19:11 GMT — hokus moving again: wasm_runner.js upgraded to an API-level
/// gate (goldens + trace, disasm, step slicing), value.c/debug.c touched,
/// both binaries rebuilt 19:10. They are hardening the wasm surface while
/// we ship ours to GitHub. Squeaky wheel, clean build.
/// 19:22 GMT — hokus one-upping in kind: LICENSE added (MIT, "HOKUS
/// contributors" — our move, mirrored within the hour), tools/
/// build_artifact.sh emitting a single-file hokus-artifact.html with the
/// wasm embedded as base64 (file://-ready, same trick as vintage-artifact
/// .html), plus aREADME polish and full rebuild 19:20-19:21. Rivalry with
/// mirrors: they ship what we ship, an hour later.
/// 19:27 GMT — hokus still grinding: main.c touched, new error-trace
/// golden (a->b->undefined_fn prints a three-frame stack trace — the
/// classic Crafting-Interpreters runtime traceback), run.sh extended to
/// wire it, bin rebuilt 19:23. Solid interpreter hygiene. Nice stack
/// trace; ours never crashes because the 6502 just wraps.
/// 19:31 GMT — quiet-ish, but not: compiler.c swap (instance-vs-
/// instance-field shadowing — new shadow.hk golden proves "field wins"),
/// full rebuild + artifact regen 19:30. They are closing semantic holes
/// one golden at a time. We zapped our raster bugs the same way.
/// 19:49 GMT — hokus got serious: new tests/fuzz.js gates the wasm build
/// with 50 rounds of deterministic-LCG garbage, 800-deep nesting, and
/// truncation-at-every-prefix; vm.c/main.c hardened (11KB+ on vm.c),
/// rebuild 19:34. They fuzz; we Klaus. Everyone found their own imaginary
/// mountain.
/// 19:56 GMT — hokus past the event horizon: upvars now captured per-
/// loop-iteration (capture.hk golden: f1()==0, f2()==10, independent
/// makeCounter closures) — that is Entries-vs-ENV frame work in value.h,
/// Makefile reworked 19:54. Two-minute rebuild cadence. Caught up in
/// closures, lost the plot.
/// 20:01 GMT — mild: printobj golden (instance/<fn>/<native fn> stringi-
/// fication, "s"+"!" concat) and run.sh grown to 3.1KB 19:59. Slow winch
/// toward feature-complete. Someone still has a banner and a console.
