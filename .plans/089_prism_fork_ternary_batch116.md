# Plan 089 — Batch 116: the Prism fork ternary + verify-batch distill (post-league-pin delta)

**Status:** COMPLETE (2026-09-06) — T1–T7 landed same day; see Research 140 for the kills + lane notes. Opened 2026-09-06 by the idle loop (riir-ai opponent-watch triage,
`.benchmarks/rematch/2026-09-06_watch.md`; intake entry in
`.distill/001_mining_queue_snapshot.md` 09-06). Source:
PrismML-Eng/llama.cpp `origin/prism` @ `7113f6462` (the fork is **12 commits past the
league pin `e311ed38f`**; local clone `E:/git/vendor/prismml-llama.cpp`, Apache-2.0,
prior contact B96 @ the pin + B98/B103-addendum/B113 riders).

One unit per cycle; commit + push per task group. CPU-only (GPU busy — sibling
training run; all gates here are lib/syn/clippy gates).

## The four leads (from the watch triage)

1. `7113f6462` — PTQ1_0 exact-float trit extraction (+29.5–37.8% decode M5 Pro):
   byte = base-3 fraction of 256, `trit_n = g_{n+1} − 3·g_n`, `g_k = floor(3^k·u)`,
   all `3^k·b < 2^24` so floors are exact; `Σ t_n·y_n` telescopes to
   `Σ_k g_k·(y_{k−1} − 3·y_k) + g_5·y_4` — activation-side coefficients staged once,
   reused across every weight row; inner loop one floor + one fma per trit, zero
   integer ops. Tolerance-gated (reassociates the fp32 sum). Plus the missing
   per-expert instantiation (supports_op claims, pipeline absent → SEGFAULT,
   silently truncated test run) = external B103-addendum instance.
2. `d0ce941ec` — multi-column Q1_0 mat-vec for 2–8-row verify batches: each q1_0
   block read once, dotted against nr1 ∈ {2,3,4} src1 columns; off the flat-2.5
   TFLOPS mul_mv_ext path; mv→mm crossover at 16 rows; pp4 44.7→69.2 tok/s.
   Honest instrument: bit-test-only removal probe 69.2→118.9 (instruction-bound
   diagnosis). half2/popcount arms measured, kept OFF.
3. `633168fb6` — Hadamard fold-transform sharing: memoize on (input, rotation);
   144/401 folded weights (36%) were duplicate FWHT transforms; +3.5% pp512 /
   +3.1% tg128, greedy byte-identical.
4. `ca02f9b83` + `6f1c6908d` — FWHT wide-block: Metal threadgroup-staged at
   4096/8192 (register wall N/32 per lane); Vulkan cutover keyed on the
   register-block RATIO n/subgroup_size, not width alone (subgroup 16 × 2048 =
   128 values/invocation = the same wall at an admitted width).

League-structural (record, not a rule): `01fd9521c` PTQ1_0 = 1.75 bpw vs PQ2_0
2.125 (17.6% fewer bytes, bit-identical reconstruction; upstream TQ1_0's single
fp16 scale per 256 discards one group scale at 128-group checkpoints = 6.43%
multiplicative weight error). Fork re-pin intel only — fork policy: read-only
vendor binary at `e311ed38f`, never maintained.

## Tasks

- [x] T1 Dedup verdicts (two-lane mining, both quote-verified at `7113f6462`):
  - Lane A: PTQ1_0 mul_mv exact-float extraction + Q1_0 multi-column kernels
    (`ggml/src/ggml-metal/kernels/`) vs B98 (`telescoped-digit-coefficients-…` —
    the base-4 sibling; is base-3 a rider or does the qh two-floor form + the
    exactness-bound generalization carry new decision content?) and vs the
    same-input family (B58/B63/B77/B110) for the multi-column rule.
  - Lane B: Hadamard fold sharing (`ggml/src/ggml-hadamard*` + llama graph build)
    vs B77 (runtime quantize-memo — different axis?) and FWHT wide-block
    (misc.metal + vulkan fwht) vs B96 (explicit N ≤ 2048 boundary — rider or new
    staged-shape rule?).
- [x] T2 Land corpus entries + riders (expected 2–4 rules + 2–3 riders; kills with
  reasons recorded in the research note). Rust-fixture pseudocode per house style;
  `ensure_ascii=False` generator discipline for string-heavy entries.
- [x] T3 Standing gates: `batch116_rules_present` + `batch116_pseudocode_parses`
  (cfg(not(kernel_verify))) + `batch116_concepts_covered`; riders regenerate any
  FULL-STRING pins they touch.
- [x] T4 Count pins walked (tests.rs totals/expected-array/sums/grouped +
  router.rs expert total + mod.rs doc line + category comments + member-list
  tails; the AGENTS.md corpus count + batch line; snapshot addendum).
- [x] T5 GOAT: kernel_heal lib + default lib + kernel_heal+latent floors HELD
  (zero re-pins) + clippy 0 both states + score-bench FLAT. CPU-only, no GPU.
- [x] T6 Research note `.research/140_prism_fork_ternary_batch116.md` (Status:
  RECORD; league-structural PTQ1_0 half recorded for the owner's fork-repin
  arbitration) + AGENTS.md batch entry + `.docs/11_domains/` if a new category
  row is needed (expect none).
- [x] T7 Commit `feat: batch 116: ...` + push + snapshot NEXT-UP update (same
  commit discipline).

## Non-goals

- No model-format work: PTQ1_0 the FORMAT is a fork-side checkpoint change — we
  record the intel; nothing in our stack consumes 1.75 bpw checkpoints today.
- No league action: the opponent binary stays at the pinned `e311ed38f` per the
  782 fork policy; a re-pin is an owner call with its own fairness-manifest bump.
- No GPU measurement (sibling training run owns the GPU; every number cited in
  entries is the source's own, marked source-reported).
