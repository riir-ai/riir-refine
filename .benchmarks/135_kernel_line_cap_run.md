# Bench 135 — RUN RECORD: the per-span line-query cap A/B (Bench 134's registration, executed) — cost leg FAIL again, the cap cut the tail 2.2× with PERFECT utility neutrality (0 rank-1 flips), and the residual is query-count-INVARIANT: the whole-span query on giant spans is the named tail driver

**Status:** RUN COMPLETE 2026-11-05 (zed-m3-i208pregate, the Bench-134 lease) — per the
pre-pinned arithmetic the G5-cost fence is **NOT cleared** (p95 FAIL in every arm). The
run's own finding supersedes the prereg's projection: the line cap delivered a real 2.2×
tail cut, then flatlined between cap 4 and cap 2 — the residual ~44 ms is
query-count-invariant. No verdict call requested: the numbers reconcile with the
mechanism (the invariant component is the whole-span query's embed+BM25 over the corpus
entries' giant fns, which no per-line cap touches).

## The verdict table (one session, five arms, minutes apart)

| arm | added p50 ms (≤ 1.0) | added p95 ms (≤ 5.0) | files > 1 ms | cost | utility floor |
|---|---|---|---|---|---|
| OFF | baseline | baseline | — | reference | n/a |
| GATE | 0.148 ✅ | 124.4 ❌ | 182/600 | FAIL | 15/15 HOLDS (1,483 spans; 597/1056) |
| SCOPE | 0.291 ✅ | 101.3 ❌ | 175/600 | FAIL | 15/15 HOLDS (660 spans; 458/1056) |
| **CAP4** | 0.187 ✅ | **46.5** ❌ | 169/600 | FAIL | 15/15 HOLDS (660 spans; 458/1056) |
| **CAP2** | 0.289 ✅ | **44.3** ❌ | 185/600 | FAIL | 15/15 HOLDS (660 spans; 458/1056) |

- **The fence verdict: FAIL — `kernel_opt` stays opt-in.** No ceiling re-registration
  (forbidden by D2; the residual is ~9× the ceiling).
- **The cap is UTILITY-NEUTRAL — the strongest quality result this lane has produced:**
  paired rank-1 compare (SCOPE ↔ CAP4 ↔ CAP2, same span population, the compare that
  APPLIED here unlike Bench 132's): **0 flips / 660 spans, 0 lost, 0 cited-set
  shrinkage** for BOTH caps. The stride selection changed no ranking outcome at all, and
  the whole-span query carries the signal. The cap ships with a perfect quality story.
- **The invariant-cost finding (this run's contribution):** CAP4 (5 queries/span) vs
  CAP2 (3 queries/span) moved p95 only ~5% (46.5 → 44.3). Line-query count is NOT the
  binding constraint anymore. The tail is the WHOLE-SPAN query — one per span, always
  run, whose `query_for` embeds + BM25-tokenizes the ENTIRE span text. The tail files
  are the kernel-corpus entries (`src/domains/kernel_opt/corpus/entries_*.rs`), whose
  rule texts live as kilobyte string literals inside single giant fns — one fn = one
  giant span = one massive whole-span query, invariant under every per-line cap.
- GATE same-session drift vs Bench 132 (disclosed, cross-session): p50 0.148 vs 0.191,
  p95 124.4 vs 122.5 — inside the load-drift band (Bench 129's sequential-process
  caveat); SCOPE 101.3 vs 92.4 (+10%, load rising during the session).

## The verdict arithmetic outcome

Per Bench 134's pre-pinned arms: both cap arms FAIL p95 → the fence stays FAIL. The
prereg named the substrate-side lever (score-only retrieval) as the next step on a
double-fail; THIS RUN'S attribution sharpens that: the residual is span-TEXT mass, so
the re-targeted next lever is (i) **in-repo: a whole-span query-text cap** — bound the
whole-span query's text (a pinned truncation budget for giant spans; the span's lines
already carry the fine scale) — a NEW design change, its own pre-registration, same
ceilings; and (ii) the substrate score-only retrieval lane (riir-neuron-db's) as the
complementary per-query-cost cut. Neither is requested here; both are owner-visible
design changes.

## Provenance (T1 bar 2 — Bench 132's block applies, deltas only)

- Box: m3, AC, powermode 2, load 3.28 falling from 5.97 at start, 2 cargo processes
  (this lane's own builds) at start, zero others.
- Commit `8eccfbdc8` (the cap implementation; tree clean at run time), walk 600 files
  (+1 vs Bench 132's 599 — the sibling session's kernel_opt distill work grew the tree).
- Release builds (isolated target dirs `/tmp/i208_g5_{off,on}`): 2m00s + 1m59s; the CAP
  arms are the GATE binary + env (`RIIR_KERNEL_PREGATE_SCOPE=1`,
  `RIIR_KERNEL_PREGATE_LINE_CAP=4|2` — the pinned literals; one binary, five runs).
- Runs 22:21–22:33 local, sequential minutes apart, one untimed warmup + three timed
  passes each, per-file medians, top_k=5, nearest-rank p50/p95 + tail support — the
  standing instrument verbatim (`kernel_goat_g5` + the runner's adjudication math).
- No marginal pass exists (the best tail is ~9× over) — the alone-rerun rule does not
  trigger.

## Reproduce

```sh
CARGO_TARGET_DIR=/tmp/i208_g5_off cargo build --release --example kernel_goat_g5 --features latent_retrieval
CARGO_TARGET_DIR=/tmp/i208_g5_on  cargo build --release --example kernel_goat_g5 --features latent_retrieval,kernel_opt,kernel_pregate
OFF=.../kernel_goat_g5; ON=.../kernel_goat_g5
$OFF --root src --top-k 5 --out g5_off.json
$ON  --root src --top-k 5 --out g5_gate.json
RIIR_KERNEL_PREGATE_SCOPE=1 $ON --root src --top-k 5 --out g5_scope.json
RIIR_KERNEL_PREGATE_SCOPE=1 RIIR_KERNEL_PREGATE_LINE_CAP=4 $ON --root src --top-k 5 --out g5_cap4.json
RIIR_KERNEL_PREGATE_SCOPE=1 RIIR_KERNEL_PREGATE_LINE_CAP=2 $ON --root src --top-k 5 --out g5_cap2.json
# adjudicate: the runner script's paired added-cost python over the five JSONs
```

## What lands from this run

1. **The cap ships (opt-in, evidence-carrying):** utility-neutral by paired measurement
   (0/660 rank-1 flips), tail 2.2×, median green — `kernel_pregate` + the line-cap env
   is the best measured posture of the lane.
2. **NOT claimable:** the G5-cost fence — p95 FAIL in all arms; `kernel_opt` stays
   opt-in.
3. **The named next lever re-targeted:** the whole-span query-text cap (in-repo) or the
   substrate score-only lane — fresh pre-registrations, same ceilings, same floors.

## Refs

- Bench 134 (this run's registration — nothing amended), Issue 208 T6 (ticked by this
  record), Bench 132 (the pregate run: p50 green, p95 the residual), Bench 129
  (lever (a) naming — the cap is that lever, now measured), Bench 131 (the pregate
  registration), Bench 125 (the instrument + floors), Issue 191 (the re-campaign
  sequence: T6-GPU ∧ the cost lane → G1+G3+G4+G5), riir-neuron-db Issue 642 (the
  substrate lever's home).
