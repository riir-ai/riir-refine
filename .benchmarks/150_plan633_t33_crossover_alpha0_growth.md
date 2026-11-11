# Bench 150 — plan 633 T3.3: the third G3 cell — mechanical α-flip classification, the α=0 endpoint identity, and the abstain-bearing growth attempt

**Status:** EXECUTED 2026-11-11 (amo, AC, quiet; instrument `examples/adoption_g3_cell.rs` at `224b4fe9` + the binary `224b4fe9dc8f`; two full runs byte-identical on program stdout, `cmp` clean — the only first-run divergence was cargo's own compile chatter into stderr). **Verdict: honest split — bars A and B PASS (the pre-registered mechanical classification works and the α=0 endpoint identity is exactly the derivation), bars C and D FAIL (thin-carrier noise fragility FORMALIZED on the accepted-carrier set; the abstain path stays green-zero after real growth). `adoption_gate` stays opt-in — the promotion posture is unchanged; what changed is that the aggregate census failures are now CLASSIFIED findings with exact derivations, not unexplained flips.**

## The cell's inputs (all pre-registered in katgpt-rs plan 633 T3.3 BEFORE any sweep or run — claims `a1da6b41a`, pre-registration `311cf3c6f`, Bar-D correction `203e4f9a2`)

- **Bars A–D** pinned in the plan (the linear-score fact: every demote consult's score difference is linear in α, so every flip has an exact crossing α* = Δr/(Δr+Δp)).
- **Growth loop declared:** sweeps over riir-dao/riir-auth/riir-grams (zero Bench-149 rows) + regrowth re-sweeps of the eight existing windows.
- **Store preservation declared:** this record pins its own replay inputs — the windows are snapshotted under `windows/` in THIS directory and committed WITH the record (the Bench-148 store-loss fix; amo-local-only accumulation is now bounded to one cell's delta).
- **Instrument changes declared then landed:** mechanical α* + per-flip classification + prediction-agreement check + accepted-carrier stability + abstain audit-ring classification (`1c61ca70`), landed before the run; validated on the pre-growth store (reproduced Bench 149's 6-flip census exactly, prediction-mismatches 0).

## What actually happened during growth (the sweep found a rulewright bug instead of rows)

The growth sweeps were mostly **honest zeros**: riir-dao (220 spans, 0 oracle-anchored, clippy-clean at its own posture), riir-auth (0), riir-grams (0), riir-ai's four fresh issue-1199 files (0 — the landing was clippy-gated), riir-game-sdk's cas.rs (not swept — the landing message already carried clippy --all-features clean), riir-infer (docs/issues only, no code), and six of the eight window repos (0 new findings). **riir-neuron-db regrew** (11 files / 25 edits in the first pass).

Then the postures validation caught the sweep breaking `--all-features` with 7 errors (E0004/E0053) — the sweep posture had compiled the cfg-gated files OUT (the documented `--verify-args` divergence; the lane's cfg audit DID name it, the operator must compile under the features per Protocol B). Three files reverted. Root-cause hunting found **THREE proposer defect classes**, all fixed, fixture-pinned, score-benched, and landed in riir-refine this cell (P124 in the mining-queue snapshot):

1. `collapsible_match` — guard-hoist fires only when a later arm is a covering no-op wildcard (`fa6f64f8`): guard-failure falls to later arms; specific-pattern later arms are E0004 (the lifecycle.rs/kg_axiom.rs specimens), non-empty wildcard bodies shift behavior.
2. `doc_lazy_continuation` — CommonMark ordinal discipline (`fa6f64f8`): an ordered marker interrupts an open paragraph only at ordinal 1 (sibling markers at indent < cc continue at any ordinal). A wrapped sentence's `003)` tail opened a phantom list item whose content col armed a 5-space lazy insert into the NEXT line — the over-indent clippy then warned on the fix's OWN output (self-churn class; caught by hand before landing).
3. `ptr_arg` — trait-impl/trait-declaration refusal (`224b4fe9`): a coherence-bound trait's signatures live anywhere (here katgpt-core); rewriting only the impl's `&Vec<f32>` params is E0053. The re-sweep with the fixed binary re-fired exactly this → guard landed → third sweep clean.

The re-sweeps with the fixed binary landed the sound remainder: neuron-db `1b271cc` (3 float-literal sites). The window grew **64 → 78 rows / 17 → 24 tags** (dapps 48, neuron-db 21, viewbridge 3, deployer 2, kat/instinct/reflexer/rethink 1 each; 8 windows — the floor held, nothing exceeded it because dao/auth/grams/ai are clippy-clean at their own postures: the fleet's landed surfaces are maintained clean, which is itself the finding — growth by sweep only works where regrowth exists).

**Window honesty note:** the first sweep's 25 rows include rows for edits later reverted at the all-features validation (the three broken files' verdicts) — the window records the sweep posture's compile-gate verdict, not the final repo state. Disclosed; the re-sweep rows supersede the class on the same tags where the fixed binary re-consulted.

## The pre-registered bars — adjudicated

### Bar A (α-flip classification, mechanical) — **PASS**

```
[bar T3.3-A] flips=7 endpoint0=3 interior=4 prediction-mismatches=0
             tie-PROCEEDs=0 no-facts-flips=0 readmit-grid-flips=0 — PASS
```

Every α-flip carries its instrument-derived exact crossing; the linear model predicts every observed grid point (48 consults × 11 grid points, zero mismatches — this doubles as the replay-gate fidelity check); the extinct tie→PROCEED class is still extinct; readmit consults are grid-invariant as the model says. The four INTERIOR crossings:

| window | consult | bracket | α* (mechanical) | Δr | Δp |
|---|---|---|---|---|---|
| dapps | demote `collapsible_match` | 0.80→0.90 | **0.8298** | −0.0625 | −0.0128 |
| dapps | demote `unnecessary_get_then_check` | 0.90→1.00 | **0.9164** | −0.7500 | −0.0684 |
| neuron-db | demote `collapsible_match` | 0.40→0.50 | **0.4958** | −0.1429 | −0.1453 |
| neuron-db | demote `explicit_auto_deref` | 0.80→0.90 | **0.8654** | −0.1429 | −0.0222 |

Bench 149's hand-verified crossing (collapsible_match α*≈0.7825 on the OLD 48-row store) is now 0.8298 on the grown store — the window changed, the mechanism (lower repetition outweighing lower retention past α*) is identical, and the derivation is now mechanical and reproducible instead of hand arithmetic. The α=0 endpoint identities (deployer + kat + instinct, all-rejected tags, REFUSE@0 → PROCEED@α>0) are Bar B's subject.

### Bar B (α=0 endpoint identity) — **PASS**

```
[bar T3.3-B] endpoint0 flips=3 on-all-rejected-tags=3 shape-verified=3
             on-accepted-carriers=0 (bar 0) — PASS
```

The derivation held exactly: for a tag whose window rows are all rejected, score_inc(α) = 0 ∀α and score_ch(α) = α → exact equality at α=0 (tie → REFUSE under strict-`>`), strict challenger win at every α>0. All three endpoint flips verified on all-rejected tags; zero endpoint flips on accepted-carriers.

### Bar C (accepted-carrier stability — the quotability upgrade) — **FAIL, and that is the finding**

```
[bar T3.3-C] refusals=20 min-stability=0.625 (bar ≥ 0.90) below-bar=15 — FAIL
```

15 of the 20 accepted-carrier demote refusals fall below the 0.90 per-consult noise-stability bar (min 0.625 at the p=0.2 arm). **The thin-window noise fragility Bench 148/149 observed on aggregate refusals is now FORMALIZED on the accepted-carrier set specifically**: the carriers are 1-row tags (viewbridge ×3, reflexer, rethink, instinct, kat) whose single accepted row IS the entire evidence — flip that one verdict bit and the refusal moves. The growth did NOT cure this because the new rows created NEW single-row tags rather than deepening the thin ones. The quotable accepted-carrier set is the complement: 5 consults (the multi-row dapps/neuron-db carriers), stable at both arms. Consequence: **"the gate refuses demotion of productive rules" is quotable ONLY for multi-row carriers** — a 1-row carrier's refusal is noise-fragile evidence, disclosed per-consult in the run output. This upgrades Bench 148's "evidentiary, not mechanical" from an observation to a per-consult quotability contract.

### Bar D (G3-d, abstain non-green-zero) — **FAIL (green-zero persists, honestly)**

```
[bar T3.3-D] windows=8 (≥8) tags=24 (≥17) demote-refusals=20 readmit-refusals=7
             abstains=0 audit-only=0 ring-eviction=0 (bar audit-only ≥ 1) — FAIL
```

The store GREW (78 rows / 24 tags) and the abstain path STILL never fired. The Bar-D correction's classification machinery is in place and exercised (it verified the zero: no readmit consult abstained, so there is nothing to classify — audit-only=0, ring-eviction=0 both measured zeros, not unexplored). Why the lottery missed: stride-5 audit filings landed inside already-multi-row tags (dapps/neuron-db) — never an audit-ONLY tag. Ring-eviction also measured zero: dapps' 48-row window has 9 evicted accepted digests, but every affected tag retains ≥1 non-evicted digest in the T rings. The abstain path on REAL data now has a formally verified green zero with the classification machinery live — the next accumulation needs either (a) windows where a tag's FIRST accepted row lands on a stride-5 boundary (1-in-5 per new single-row tag, observed 0/7 this cycle) or (b) a deliberately seeded instrument posture (would be a different, disclosed cell).

### G3-c (unchanged bar) — **FAIL/DISCLOSE, same mechanism as 148/149**

Dominant-carrier refusal min noise stability 0.688 (bar ≥ 0.90), leave-one-out survived: false — the dominant refusal on 1-row evidence flips under its own row's noise, reproduced on independent data for the third time. Per-consult quotability (Bar C's set) is the quotientable form; the aggregate line stays FAIL by design.

### Seat leg — UNMEASURED (disclosed, unchanged from 148/149)

0 seat rows across all windows; waits for armed seat traffic.

## Determinism

Two full runs byte-identical on program stdout (`cmp` clean; run 1's stderr carried cargo's release-build chatter — separated for the comparison). The `from_rows` replay self-check passed on every window. Box: amo, AC, quiet; the cell is exact f64 arithmetic over ≤48-row windows (not a latency measurement).

## Consequences

- `adoption_gate` stays **opt-in** — the promotion posture still lacks the G3-d non-green-zero abstain evidence, and the accepted-carrier quotability contract (Bar C) says the aggregate "gate protects productive rules" claim needs per-consult qualification.
- **Bars A/B confirm the classification machinery and the tie-rule decision on a grown store** — the census is now fully derived, not enumerated.
- **The sweep lane found and fixed three real rulewright defects** (P124) — the growth loop's value this cell was the bug harvest; the store grew 14 rows / 7 tags as the by-product.
- **Store preservation landed**: this record commits its own replay inputs (`windows/*.jsonl`) — the Bench-148 loss mode (uncommitted amo-local store) is closed for the cell's evidence; between-cell accumulation stays amo-local (bounded loss disclosed).
- The abstain path's next-cell material: audit-only tags need single-accepted-row tags landing on stride-5 boundaries — the observed rate this cycle was 0/7 new single-row tags. A cell that needs the abstain evidence should declare a seeding posture up front (different cell, pre-registered) or keep accumulating.
- The three clippy-clean repos (dao/auth/grams) and riir-ai/riir-game-sdk remain UNWINDOWED — the sweep-growth model only produces rows where regrowth exists.

Commits: riir-refine — instrument `1c61ca70`, proposers `fa6f64f8` + `224b4fe9`, intake `37b19cd9`, this record; katgpt-rs — claim `a1da6b41a`, pre-registration `311cf3c6f` + `203e4f9a2`, the T3.3 fold (this record's landing); neuron-db — sweeps `7705fd5` + `1b271cc`. Session: katgpt-rs-34@amo, 1794407889.
