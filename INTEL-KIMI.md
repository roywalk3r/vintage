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