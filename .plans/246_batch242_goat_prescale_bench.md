# Plan 246 — the Batch-242 GOAT bench fixture (portable CPU-exact prescale)

**Status:** IN_FLIGHT — claimed 2026-10-16 by the shikuwa/4090 idle lane (the claim row in
`.distill/001`, pushed plan-first per protocol)

The Batch-242 GOAT rule `masked-in-place-field-prescale-other-operand` (entries_gpu_98,
kernel_opt N-category) carries its own benchability clause: "Benchable via a Rust-idiom
fixture (the x86-GOAT precedent B231/116 — portable CPU-exact exponent-field +
integer-mask logic; a fixture comparing shift-down decode vs masked-in-place + prescale
over random packed words pins both exactness and op-count)". The landing deferred the
battery to a fresh cycle ("deserves the full G1/G2/G4 battery; not rushed there") — this
is that cycle. Portable by design: no ISA gate (the B231/116 precedent's portable
sibling — pure integer masks + fp32 bit patterns, runs on both boxes).

## Tasks

- [ ] **T0 — claim row + plan-first push** (this commit).
- [ ] **T1 — author `benches/bench_008_prescale_masked_field_goat.rs` + the Cargo
  `[[bench]]` row** (harness = false, no required-features — std-only). Four arms: the
  4-bit wpre arm (off = 4p, 8 fields/word), the 8-bit wpre arm (off = 8p, 4 fields/word),
  the 2-bit ALU-twin arm (the qmv_nax2 scale-ladder form: `sv * {2^0, 2^-2, 2^-4, 2^-6}`
  staged per group, codes masked at byte offsets), and the 6-bit shift-down exception arm
  (wpre = 1, the two-word straddle form verbatim) — G1-only, no perf claim.
- [ ] **T2 — run the battery**: G1 bit-identity (the exponent-cancellation proof: the
  prescaled x·2^-off and the masked field·2^off are each exact in-regime, so the product
  rounds the same true value; random + edge + straddle + the 2^-98 boundary fixtures);
  G4 zero-alloc both arms (staging into a pre-allocated scratch); G2 rows×K sweep
  (interleaved best-of-5 min, correctness-coupled sinks, strictly-faster bar at every
  shape for the 4-bit + 8-bit arms) + the op-count ledger line (R×K shifts eliminated
  for K staged prescale multiplies). Provenance-gated citable posture
  (`PRESCALE_BENCH_REQUIRE_PROVENANCE=1`).
- [ ] **T3 — record**: `.benchmarks/117_*` bench note with the measured numbers; the
  GOAT rule's in-rule bench verdict (count-neutral append to `entries_gpu_98` — the
  "Benchable via…" clause resolves to the landed fixture); AGENTS kernel_opt bullet;
  `.docs/11_domains/kernel_opt.md` Batch-242 row tail.
- [ ] **T4 — gates**: the batch242 needle test still green (append-only corpus text);
  default lib tests green; clippy ×3 postures (default / no-default-features /
  kernel_opt+latent_retrieval) clean; retrieval floors re-measured (kernel-corpus text
  delta — expect HELD, no re-pin; the rider precedent).
- [ ] **T5 — snapshot landing row + commit + push** (`feat: bench 008 — the Batch-242
  GOAT fixture (portable CPU-exact prescale)`).

## Yield ledger

G1 pass = the exactness proof pinned by execution. G2 pass = the amortization law
measured (the shift leaves the hot FMA loop for K staged multiplies). G2 fail at any
shape = an honest recorded outcome (PARETO-FAIL, exit 1) — the fixture still pins
exactness + the op-count ledger; the rule text then carries the honest measured label.

## Anti-goals

- No corpus-count change (one count-neutral append only).
- No freeze/score-bench regen (zero clippy-corpus delta, P30).
- No ISA-gated code paths (the portable point of the law).
