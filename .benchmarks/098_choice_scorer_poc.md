# Bench 098 — Issue 125: the CUA-S1 specialist option-scorer PoC (REFUTED)

**Status:** COMPLETE 2026-09-20 — verdict **REFUTED for promotion** (both gates + latency fail; modelless `RerankMode::Structural` default stands). Harness + model land as the reusable measurement lane (`choice_scorer_poc` feature) — re-run as-is when the corpus grows an order of magnitude (the issue's reopen trigger).

**Source:** [Research 573](../katgpt-rs/.research/573_CUA_S1_Open_Jev_Recipe_Specialist_Option_Scorer.md) (CUA-S1 @ `83f142c4`, MIT) · [Issue 125](../HISTORY.md) (RESOLVED — REFUTED, the record row)

## What ran

The full Research-573 recipe on healer rule selection, both halves:

1. **Model** (`src/draft/choice_scorer.rs`, ~1650 lines): a ~719k-param byte-level tinyx — byte embed 257 + learned positions → 2-layer pre-LN encoder (width 128, 4 heads) over the context (≤224 B) → 1-layer over each option (≤96 B, `rule_id\nbuggy_code`) → final LN + mean-pool → the options-as-queries head (`attended_n = softmax_j(q_n·K_j/√r)·V`, `logit_n = q_n·attended_n/√r`). Hand-rolled forward/backward/AdamW (warmup+cosine), zero new deps. **Correctness gate:** finite-difference gradient checks (per-episode directional derivatives + per-index argmax FD at healthy SNR) + overfit + bit-determinism + cached-pool parity — 7/7 tests green; the FD gate caught one real bug pre-run (a transposed `dWq` accumulation) and correctly exonerated three finite-difference artifact classes (documented in the test).
2. **Harness** (`examples/choice_scorer_poc.rs`): renders choice rows from the shipped labeled fixtures (48 clippy + 48 perf oracle-verified spans + 16 clean excerpts = the skip rows; 231-rule option table), rule-disjoint 1/5 split (signature-disjoint OOD by construction — held-out rules never appear in ANY training option), confuser augmentation (forced same-family look-alikes, held-out rules excluded), threaded deterministic training (fixed 8 workers, ordered reduce, batch 32, 6 epochs, 3576 episodes), BLAKE3-checkpointed, then the full gate suite on production pools (canonical merge, PROBE_TOP_K=8/domain, ≤33 options + skip).

## Numbers (run 2 — fixed curriculum; run 1 was a curriculum defect, §V1)

| arm (top-1 on production pools) | in-dist (n=83) | OOD (n=18) |
|---|---|---|
| trained specialist (719k) | **23/83 = 27.7%** | **1/18 = 5.6%** |
| modelless structural baseline (pool[0]) | 73/83 = **88.0%** | 18/18 = **100%** |
| frozen control (untrained head) | 6/83 = 7.2% | — |
| shuffled-context control | 10/83 = 12.0% | — |

Gates: **G1 FAIL** (27.7% < 88.0%) · **G1-OOD FAIL** (gap +94.4pp) · no Bench-065 memorization signature (PASS, trivially — nothing memorized) · **G2 FAIL** (8868 µs/span single-thread CPU vs the ~1 ms bar; option pools cached) · shuffle control PASS (12.0% ≪ 27.7% — the harness's context-reading check works) · calibration: ECE 0.243→0.175 / Brier 0.157→0.131 after Platt-T (reported per the Issue-810 tie-in; far from calibrated either way).

Training: loss 2.20 → 1.53 over 6 epochs (1104 s wall, 8 workers, M3 Max); checkpoint BLAKE3 `ddecdc12…4fdbb`.

## Interpretation (why the class loses HERE, honestly)

- **Corpus scale, not architecture, is the binding constraint.** CUA-S1's 99.7%-vs-83.6% pricing was over ~150k synthetic rows with compositional variety; this run had 101 distinct gold contexts (×40 episodes). The specialist's training signal is ~1000× smaller than the recipe's regime, and the opponent is not a hosted general model but our structural rerank whose per-rule proposers encode hand-derived matching knowledge — an 88% bar on the same pools. The model DID learn (27.7% vs frozen 7.2%; loss −30%; shuffle degrades correctly) — it simply starves.
- **Zero-shot rule binding (G1-OOD) is the deepest failure** (5.6%): matching an unseen rule's `buggy_code` bytes to a span is exactly what 100 contexts cannot teach.
- **Latency**: the width-128 ctx encoder costs ~8.9 ms/span single-thread in this naive-but-correct implementation — 9× over the bar before any accuracy question. A GPU/serving-optimized path would clear 1 ms, but with G1 failing the point is moot.
- The `skip`-in-softmax abstention design worked as an arm: the model uses it (the honest decline rows — clean spans + gold-absent pools — trained without label noise after v1's fix).

## V1's curriculum defect (recorded for the rerun)

Run 1's curriculum mixed 25% gold-dropped-same-context rows ("train decline") — contradictory labels (the context still matched the gold pattern) collapsed the model to always-skip: 0/83 with skip at p≈1.00, WORSE than the frozen control (7.2%). Run 2 weights production-pool episodes ×8, drop_gold removed (decline comes only from honest sources: clean spans + gold-absent pools). Both runs' full outputs are reproducible: `cargo run --release --features choice_scorer_poc --example choice_scorer_poc -- all` (deterministic seeds; ~18 min on an M3 Max, CARGO_TARGET_DIR isolated).

## Verdict + reopen trigger

**REFUTED at current corpus scale** — the modelless default stands; nothing is wired into any heal path. Reopen trigger (per the issue): a corpus-order-of-magnitude change — fixseq rings at fleet scale or frontier-miner-generated oracle-verified spans reaching ~10⁴-10⁵ distinct labeled contexts — at which point the harness re-runs unchanged (it is committed, deterministic, and checkpoint-verified).

The metric-taxonomy half of the issue landed earlier (2026-09-20, `ChoiceTaxonomy` on `EvalReport`) and is unaffected by this verdict.
