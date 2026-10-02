# Research 055 — Batch 49: the Bench-686 measured-lesson distillation (runtime-arg loop bounds → local memory)

**Date:** 2026-08-16
**Batch:** 49 (single-rule batch — a measured-GOAT distillation, not file mining)
**Corpus:** 343 → **344** kernel rules (A 80)
**Source:** riir-ai Issue 706 / [Bench 686](../../riir-ai/.benchmarks/686_rmsnorm_single_pass_goat.md) — the `rmsnorm_quantize_f32` single-pass + register-unroll GOAT on the 4090 cudarc path.

## Why this batch exists (departure from the file-mining protocol)

The mining protocol mines unmined source clusters. Batch 49 instead distills a rule from a
**measured GOAT** landed the same day (Issue 706 / Bench 686): the v1→v2 iteration of the
rmsnorm fix surfaced a pre-existing CUDA pathology with an exceptional evidence trail — a
paired interleaved A/B that caught the naive fix measuring SLOWER (v1), an nsys
same-code-drift-controlled kernel measurement (−18.4% against +13-21% ambient drift), and ppl
bit-exactness at every level. A lesson with a measured falsification of its own naive fix is
stronger evidence than most file-mined patterns; it belongs in the corpus immediately rather
than waiting for the next cluster that happens to contain the shape.

## The rule landed (A: memory access)

**`runtime-arg-loop-bound-demotes-arrays-to-local-memory`** — a correctly-sized per-thread
array whose per-element loops are bounded by a RUNTIME kernel argument is demoted to local
memory even when the argument is constant at every call site: the compiler cannot unroll a
runtime-bounded loop, and a dynamically-indexed per-thread array has no register form — every
access becomes a local-memory (spill) round-trip, and every array the loop touches inherits
the demotion. The invariant is a LAUNCHER fact the kernel cannot see (the host passes a Rust
const as an int arg); the fix hoists it into the kernel as a compile-time constant +
`#pragma unroll` (statically-indexed arrays promote to registers; unrolling preserves the
ascending accumulation order → bit-identical; keep the `(i < len)` tail-guard for partial
tiles). The pathology is invisible by default — it hides under whatever global traffic the
kernel already does, and surfaces only when a neighboring change ADDS traffic to the same
address space: trading L1-hot global reads for more local-memory traffic measures SLOWER.
Detection instrument: paired interleaved A/B; quantifier: nsys local-memory counters.

Source quotes re-verified by the coordinator against the live file
(`riir-gpu cudarc_kernels/mod.rs` L193-235 current; the pre-fix form via
`git show d11a9732e~1`):

- pre-fix (Issue-623 era): `float vals[16];` + `for (int i = 0; i < ablock; i++)` — `ablock`
  a kernel arg, 16 at every call site.
- shipped v2: `#pragma unroll` + `for (int i = 0; i < 16; i++)` + the in-source comment
  (L217-221): "a runtime bound blocks unrolling, which forces vals[]/gam[] into local memory
  (indexed addressing has no register form). `#pragma unroll` + constant bound keeps both
  arrays in registers."

## Dedup analysis (the load-bearing step)

Zero corpus matches for `local.mem|unroll|register|spill|dynamically.index` across the kernel
corpus. The ONE adjacent rule — `declared-max-array-taxed-per-thread-under-runtime-bound`
(Batch 46, WGSL) — is a **distinct sibling**, distinguished on three axes:

| axis | Batch 46 (`declared-max-array…`) | Batch 49 (this rule) |
|---|---|---|
| pathology | array **SIZE** declared at a hard max (`array<f32,256>`) >> runtime use (d_eff=6) | array **exactly sized** (`vals[16]`); the LOOP BOUND is the runtime value |
| tax | thread-local **footprint** at the declared max (1 KB/thread always paid) | per-ACCESS **local-memory addressing** + lost unrolling (traffic, not footprint) |
| fix | pipeline override constants (specialize the SIZE, one pipeline per class) | hoist the BOUND to a compile-time const + `#pragma unroll` |

Also distinct from `bit-cursor-unpack-to-width-specialized-byte-batch` (B44 — per-width arms
make BIT ADDRESSES compile-time-constant; here the loop bound itself is the runtime value) and
`hardcoded-fast-path-jit-specialization` (B23 — throughput constant-folding; this is
address-space placement of per-thread state). All three distinctions are encoded in the rule's
`gpu_concept` text.

## GOAT

- **G1 parse/syn:** `batch49_pseudocode_parses` PASS (ref_shape + opt_pattern syn-parse).
- **G2 lib tests:** 574 kernel_opt / 415 default / 9 retrieval-gate — 0 failures.
- **G3 no-regression:** clippy **0 warnings** in default + kernel_opt states; the
  all-but-ternary combo check (`clippy_lints,clippy_verify,rust_perf,rust_perf_verify,
  rust_perf_bench,kernel_opt,kernel_verify,ruliology_search,latent_retrieval,self_evolve,
  fix_verify,wasm_pruner,clippy_lora`) PASSES (9.44s).
- **CI guard caveat (pre-existing, filed):** layers 3/4/5 fail on Windows ONLY via
  `ternary_inference` → riir-gpu's un-gated optional `metal` dep (L80 general `[dependencies]`
  instead of the macOS target section) → `core-foundation` compile failure. Diagnosed via
  `cargo tree -i core-foundation` and filed as **riir-ai Issue 707** with the one-line fix.
  Not caused by this batch (dependency-graph fact; the batch touches corpus data only).
  cargo-hack is not installed on this box, so layer 3 was skipped by the script itself —
  per-feature coverage provided by the states above.
- **Retrieval floors (Issue 017 gate):** `retrieval_precision_regression_gate` PASS — kernel
  self-variants 100%, blended floors held (9/9 tests incl. determinism; no decay from the
  new rule).

## Toolchain note for this box (4090 / Windows)

The riir-clippy dir resolves its `rust-toolchain.toml` pin (1.97.0) to the **GNU** host
(`x86_64-pc-windows-gnu`) whose mingw linker is absent — plain `cargo` fails at build-script
linking (`x86_64-w64-mingw32-gcc: command not found`). All builds on this box must use
`cargo +1.97.0-x86_64-pc-windows-msvc` (installed) or `RUSTUP_TOOLCHAIN=1.97.0-x86_64-pc-windows-msvc`
for scripts. Recorded here so future 4090 sessions don't re-diagnose.

## What's next

The Batch 48/49 queue candidates (small unmined riir-gpu leaves, standalone `forward*/` twins,
`kimi_k3_gpu_backward_batched.rs` low-value, `attention_q8kv_cubecl.rs` high-dedup) remain for
Batch 50. `ternary_deltanet_gpu_forward.rs` stays sibling-blocked.
