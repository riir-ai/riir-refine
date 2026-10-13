# Plan 233 — Batch 231: the P52+P53 conversion (Protocol A)

**Status:** LANDED

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
- [x] Fetch both `.patch` pins; re-verify EVERY quoted line from the P52/P53
      intake rows against the real diffs (all verified; the third P53 quote
      needed an exact-case fix in the rider draft — the verbatim form
      'two Q2_0 blocks per step, each into its own accumulator (two FMA
      chains)').
- [x] Dedup re-grep: pinning law zero hits; MADV/WILLNEED owned by the
      entries_15/05 prose only (no guard-silent-else owner); AVX2/maddubs
      prose-consumers only — no Q2_0 x86 dot owner (see Research 262).
- [x] P52a: the NEW kernel rule authored (entries_gpu_90.rs append via
      subagent A; corpus 803 → 804; A-slice + A-comment 165 + module doc).
- [x] P52b: the rust_perf rider authored (entries_15.rs context_notes via
      subagent B; count-neutral).
- [x] P53: the kernel rider authored (entries_gpu_33.rs gpu_concept).
- [x] G1: `batch231_rules_parse_and_needles_landed` (kernel) +
      `batch231_rider_landed_on_madvise_host` (rust_perf) — both pass.
- [x] G2: kernel_opt+rust_perf gated lib 3095/0/1i; default lib 2650/0;
      count pins moved at SEVEN sites (incl. the per-category A-table pin
      the first gated run caught — the eighth site greps miss).
- [x] G3: clippy 0 default; all-features 0 code warnings (the pre-existing
      `block v0.1.6` dep notice only).
- [x] Retrieval-precision re-measure: kernel 100.0%, blended 91.2/94.7/95.3
      — floors HELD, no re-pin.
- [x] No freeze regen owed (zero clippy-corpus delta — P30; batch 230
      `3f2d87a6` + P51a `3ff0013f` both landed their tails — the cycle-74
      "one freeze tail" expectation was WRONG, recorded in Research 262).
- [x] Research note 262 (`.research/262_batch231_p52_p53_conversion.md`),
      `.highwater` 261 → 233-line (262 written).
- [x] AGENTS.md corpus count bump (kernel 803 → 804 + the batch-231 line;
      rust_perf 198 stands with the rider note).
- [x] `.docs/11_domains/kernel_opt.md` (inline B231 row) + `rust_perf.md`
      (Batch 231 rider block).
- [x] Snapshot landing row (P52/P53 marked landed w/ commit hash; intake
      cleared) — python splice, LF-preserving.
- [x] Commit `feat: batch 231: ...` + push.
- [x] Fold the 4090 lane's companion x86 GOAT (022eaf27, bench 007/116,
      PARETO-PASS — run ahead of this landing against the claim) into the
      rider text + AGENTS + kernel_opt.md + Research 262 + snapshot; rebase
      resolved (both snapshot sides kept), gates re-run on the combined tree.

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
