# Research 103 — Batch 87: the TTT-KVB associative-scan boundary rules (kernel_opt)

**Status:** RECORD (batch landed 2026-08-29; Plan 048 / Issue 054 closed in the same commit).

## Source

arXiv:2602.21204 *"Test-Time Training with KV Binding Is Secretly Linear Attention"* (Liu, Elflein, Litany, Gojcic, Li — NVIDIA · Univ. Toronto · Vector Institute · Technion, ICML 2026), **Appendix I** (the non-reducibility analysis) + the §6 reduction trajectory. Corpus record: `../katgpt-rs/.research/516_TTT_KVB_Secretly_Linear_Attention.md` (katgpt-rs commit `513088cd`), which holds the full theorem/paradox/trajectory distill — this note records only the BATCH landing.

## What landed (corpus 398 → 400, G 23 → 25)

The **boundary complement** of B79's `first-order-affine-recurrence-associative-scan` (B79 = the positive case: affine ⇒ scan; this batch = what BREAKS the scan):

1. **`state-normalization-between-updates-breaks-associative-scan`** (G) — `Norm(A+B) ≠ Norm(A)+Norm(B)`: normalizing the recurrence **state** between updates voids associativity outright even though the mechanism still reads as linear attention. The pair-space scan (B79's opt shape) returns the **raw affine trajectory** — a wrong answer at scan speed, never an error. LaCT weight-norm is the live anti-shape; the paper's App-H parallel form exists only for the static-kernel + no-state-norm + dot-product-loss corner. Fix-space: (a) normalize q/k instead (phi-side — a static per-token transform, scan-SAFE; the in-tree negative control is riir-gpu's `expand_and_l2_normalize_heads_f32`, which normalizes Q/K heads PRE-recurrence → in-tree SAFE, no firing population), or (b) keep state normalization and accept the strictly sequential form.
2. **`dynamic-kernel-params-break-prefix-scan`** (G) — kernel parameters Θ updated INSIDE the recurrence (TTT-style inner-loop fitting) make φ history-dependent through nested-nonlinearity chains (silu′ nesting): no per-token pair exists, the composition is not associative, and the scan port silently computes a different operator. Fix-space: freeze Θ at layer load; when adaptation is wanted it happens BETWEEN forwards (outer loop), and the paper's V1/V2 reduction rungs (update-only-last-layer 15.93 ppl; +no-weight-norm 16.31 — best-or-near AND the first shape admitting the chunk-parallel form) are the design table.
3. **B79 rider** (comment-level, zero count impact) — SCAN-ELIGIBILITY BOUNDARY sentences appended to B79's `gpu_concept`: qk/head-side normalization is part of φ and does NOT break the scan; only state-norm (rule 1) or dynamic-Θ (rule 2) void it.

## Authoring-time falsification / dedup

- Subagent drift caught + corrected before landing (the Batch-31 discipline): R2's draft said V2 is "the **only** shape that admits the chunk-parallel form" — R516's Table 2 shows V3–V6 with parallel tok/s, so V2 is the **first/minimal** such shape (weight-norm removal is what unlocks parallelism; later rungs keep it). Landed text corrected.
- R1's "every output from t=1 disagrees" softened — the scalar toy differs at t=0 too; "diverge from the production sequential loop" is the accurate claim.
- Dedup: no existing rule covers recurrence-STATE normalization (all "normaliz" hits are RMSNorm fusion / quant unit-norms) or dynamic kernel params (zero hits); `chunk-reset-cumsum` is the accumulator-ZEROING complement (reset composes with chunked scans; normalization RESCALES every step and breaks associativity) — the distinct-from sentence encodes the contrast.
- Issue 054 close condition met the strong way: R1+R2 stood as rules (did not collapse into B79 regime notes), so R3 landed as the rider alongside both.

## Gates (GOAT)

| Gate | Result |
|---|---|
| G1 parse | `batch87_pseudocode_parses` PASS (both rules, ref + opt; `#[cfg(not(feature = "kernel_verify"))]` per the B70 lesson) |
| G2 lib tests | kernel_opt 984/0/1i (was 982+1i pre-batch: +2 gates); default 752/0 unchanged-path |
| G3 no-regression | clippy 0 default + kernel_opt (3 self-inflicted doc_markdown backticks fixed — the B37 lesson, `LaCT`/`gpu_concept`); `kernel_verify` state compiles (the cfg'd-out gate shape) |
| G4 content | NEW `batch87_state_norm_breaks_associative_scan` — the falsifiable breaker gate: with h0 = 0 the scan computes the raw trajectory `B_t` while the truth is `l2n(B_t)`; asserts (a) truth states are unit-norm (the normalization is live), (b) `|scan − truth| > 1e-3 · scale` at some position (material, structural wrongness — not fp association noise, which B79's own rel-error gate already bounds) |
| Retrieval floors | 14/14 HELD (kernel-inclusive run, ~17 min) |

**CI guard: deliberately NOT run this cycle** — the shared checkout carried the sibling's in-flight B85/docker WIP (`rust_perf/corpus.rs`, `docker/rules.rs`), so a full-guard run would compile + verdict THEIR mid-flight code and hammer the box during their GOAT runs. Targeted states (default, kernel_opt, kernel_verify) all validated instead; the guard should re-run at the sibling's next quiet window.

## Provenance discipline

Grade B: the paper's throughput (4.0× parallel-form / 1.19× training) AND quality numbers (15.93/16.31/16.43 ppl) are LaCT end-to-end runs at 90M–760M from-scratch pretrains — both rules carry the never-cite-isolated discipline in-rule. Zero in-tree firing population (paper-distilled precedent = B79 itself). The 4-probe paradox kit + the full reduction trajectory stay note-only in R516 (no in-tree consumer — Issue 054 Deferred).

## Files

- `src/domains/kernel_opt/corpus/entries_gpu_08.rs` — the 2 entries (+ header)
- `src/domains/kernel_opt/corpus/entries_gpu_07.rs` — the B79 rider
- `src/domains/kernel_opt/corpus/mod.rs` — G-slice +2, category count 25, history block, 400-rule doc
- `src/domains/kernel_opt/router.rs` — 400 pins
- `src/domains/kernel_opt/corpus/tests.rs` — count pins (400 / G 25) + the 2 gates
- `.docs/11_domains/kernel_opt.md` — Batch 87 section + total
- `.plans/048_batch87_ttt_kvb_scan_boundary.md`, `.issues/054_*` (removed at close per the noise-reduction rule)
