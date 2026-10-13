# Plan 233 — Batch 231: the P52+P53 conversion (Protocol A)

**Status:** IN FLIGHT

Claim `68be73c3` (m3 idle cycle 75). Two graded intake rows converted per
Protocol A; both sources llama.cpp, MIT.

- **P52** — `b9f3599f` #30405 "llama : do not map foreign ranges into a
  backend buffer": (a) NEW kernel_opt rule (grade B+, io/alloc) — the
  host-pointer-buffer pinning law; (b) rust_perf RIDER on
  `madvise-random-for-key-gather-tables` (entries_15) — the macOS
  MADV_WILLNEED prefetch no-op (platform-guard-silent-else class).
- **P53** — `0a731cce` #30447 "ggml-cpu : add AVX2 dot product for Q2_0":
  kernel_opt RIDER on `lut-free-bit-spread-vadd4-unpack` (entries_gpu_33) —
  the x86 SIMD face of the CPU 2-bit spread (OR-not-add spread + maddubs
  algebra + two-FMA chains).

## Tasks

- [x] Claim row spliced + pushed (`68be73c3`).
- [ ] Fetch both `.patch` pins; re-verify EVERY quoted line from the P52/P53
      intake rows against the real diffs (discipline: quotes verbatim at the
      pin, never from the intake prose).
- [ ] Dedup re-grep (corpus moved since grading): pinning law + MADV/WILLNEED
      + AVX2/maddubs/cvtepu8 concepts over both corpora.
- [ ] P52a: author the NEW kernel rule (entries_gpu_90.rs append — the
      batch-227 convention: new rules join the latest slice; corpus 803 →
      804) with ref_shape + opt_pattern pseudocode.
- [ ] P52b: author the rust_perf rider into `madvise-random-for-key-gather-tables`
      (entries_15.rs context_notes; count-neutral).
- [ ] P53: author the kernel rider into `lut-free-bit-spread-vadd4-unpack`
      (entries_gpu_33.rs gpu_concept; count-neutral).
- [ ] G1: `batch231` needle test + pseudocode-parse asserts in
      `src/domains/kernel_opt/corpus/tests.rs` (+ rust_perf corpus tests
      needle row if the rider format needs one — follow the batch-224 rider
      precedent).
- [ ] G2: `cargo test --lib` default features green; corpus count pin 804.
- [ ] G3: clippy clean — default + all-features postures.
- [ ] Retrieval-precision re-measure (kernel batch owed):
      `CARGO_TARGET_DIR=/tmp/riir-refine-b231 cargo test --features
      kernel_opt,latent_retrieval --lib retrieval` — floors kernel 100.0%,
      blended 91.2/94.7/95.3 must hold.
- [ ] No freeze regen owed (zero clippy-corpus delta — P30; batch 230
      `3f2d87a6` + P51a `3ff0013f` both landed their tails). Score-bench not
      owed (same P30 line, batch-229 precedent).
- [ ] Research note 262 (`.research/262_batch231_p52_p53_conversion.md`) —
      re-scan `.research/` at write time.
- [ ] AGENTS.md corpus count bump (kernel_opt 803 → 804) + the batch-231
      line; rust_perf count unchanged (198) with the rider note.
- [ ] `.docs/11_domains/kernel_opt.md` + `rust_perf.md` rows.
- [ ] Snapshot landing row (P52/P53 marked landed w/ commit hash; intake
      cleared) — python splice, LF-preserving.
- [ ] Commit `feat: batch 231: the P52+P53 conversion ...` + push.

## Notes

- NotBenchable postures: P52a (the pinning law is a residency/IO contract —
  decision content, source-reported figures only); P52b rider (page-fault
  vs madvise warm-up — same class as the host rule's NotBenchable note;
  M3 GOAT staged candidate, not run this batch); P53 (AVX2 literals do not
  compile on this aarch64 box — NotBenchable-on-M3, x86 GOAT staged for the
  4090 lane; the host rule B212's CPU-face lesson applies: the anti-LUT
  verdict is `__constant__`-divergent-lane-specific, this is the third ISA
  spelling).
- The batch-230 commit message says the intake's own grading expected "one
  freeze tail" — VERIFIED NOT OWED: both prior clippy deltas landed their
  regens, and this batch adds zero clippy-corpus entries.
