# Bench 091 — Issue 101: the E0502 read-relocate — F3 cleared (interacting corpus 100%), the first ordering IMPROVEMENT (learned arms 10 reverts vs static 24)

**Status:** RECORD — the Bench-090 residual (F3's 12 stalls) is cleared by the
hoist-to-first-use template, the interacting corpus reaches **100% fix rate**,
and the ordering A/B prints its **first IMPROVEMENT verdict**: the learned arms
(shape-conditional, nonergodic per-run) resolve 46/46 with **10 reverts vs
static's 24** — the Bench-089 counter-poisoning is fully repaired and the
learning finally converts into fewer wasted attempts. The design that SHIPPED
is the second one — the first (relocate-after-conflict's-last-use) was measured
dead before landing: the E0499 swap can never keep on cold F3.

**Date:** 2026-09-13 · **Box:** 4090 RTX box "shikuwa" (Windows 11, i7-13700K), cargo quiet (no sibling compute) · **Toolchain:** rustc 1.98.1 (48a229cea 2026-09-01) · **Profile:** `--release`.

> **Issue:** [101](../.issues/101_e0502_read_relocate.md) · **Predecessors:** [Bench 090](090_e0614_unmasking_trade.md) (the F3 residual + the honest-revert repair this builds on) · **Gate instrument:** `examples/interacting_gate.rs`.

## The measured premise correction (design 1 → design 2, both probed)

**Design 1 (dead):** relocate the read AFTER the conflicting borrow's last use
(the diag's `mutable borrow later used here` anchor). Measured false on cold F3:
the E0499 swap — the rung that was supposed to clear the way — **reverts at
2→2**, because head's immutable borrow SPANS to the tail, so fixing the E0499
surfaces a SECOND E0502 (head vs b) at b's decl. The post-swap state the
template targeted is unreachable (T3 lane re-run: F3 ring unchanged,
`swap=REV(2->2)`). The derived 6-attempt ladder path was wrong with it; the
`max_iters` 5→8 bump it justified was REVERTED (see commit history — no
justification survives its measured path dying).

**Design 2 (shipped):** hoist the borrowed decl to JUST BEFORE ITS FIRST USE,
by value — rustc's own documented E0502 fix shape. On cold F3 the first use is
the tail (after every borrow region ends), so the hoist clears the E0502 while
the E0499 remains: KEEP 2→1, then the swap KEEP 1→0. Four attempts, under the
stock `max_iters = 5`. One diag anchor (`immutable borrow occurs here` → the
ref-local) + syn (first referencing statement by ident-boundary scan; a plain
`contains` would fire on `head2`); the oracle arbitrates non-`Copy` strata
(E0507 revert → clone-out stays the String repair) and still-conflicting
regions (re-fire → revert).

## What landed

- **`e0502-read-relocate`** (`playbooks/borrowck.rs`, template C): full-line
  deletion of the decl + by-value re-insert immediately before the first-use
  statement (its own line, indent mirrored) + star deletions — one atomic
  candidate. `semantic_note`: the value read differs if the statements crossed
  mutated the indexed place. Guards: first use strictly after the decl;
  single-line comment-free decl and use stmts; the use stmt starts its own
  line. Min-delta ordering makes it the THIRD rung (deref-removal, clone-out
  first) — it fires only where they revert or stay silent.
- **`FRONTIER_LANDINGS`** row (`E0502`, `e0502-read-relocate`) — the evolve
  ledger ratchet red until the row landed in the same commit (its second catch).

## Lanes run

| lane | command | result |
|---|---|---|
| cheap structural tests | `cargo test --features rustc_errors --lib` | **1279/1279** |
| admissibility gate | `interacting_gate` | **46/46 ADMITTED** (19.2s) |
| T3 flip prevalence | `fixseq_prevalence -- --interacting --runs` | below |
| T4 ordering A/B | `rustc_errors_floor -- --only ab --interacting` | below |
| floor + driver lanes | `rustc_errors_floor -- --only floor,driver` | G-floor 41.5% (unchanged); driver **66.7%**; E0502 row 33/38/**0**; **G1 172/172**; G2 p50 129ms (p99 323) |
| full-corpus integrity | `RIIR_FULL_CORPUS_INTEGRITY=1 … full_corpus` | **PASS over 258** (165.2s) |
| nonergodic A/B tests | `--test nonergodic_driver_ab` | **6/6** |
| clippy (touched files) | `cargo clippy --features rustc_errors --all-targets` | clean (one PRE-EXISTING finding at `evolve.rs:716`, untouched) |

## T3 — flip prevalence over the interacting corpus (n=46)

```
runs 46 · attempts 116 · depth hist {2: 34, 4: 12} · runs ≥3 attempts: 12
flips: 0 on all three keys (power: 12 runs ≥ 3 attempts)
invariant: HOLDS over all 116 attempts (the lex reading)
repair quality: 100.0% total — E0502 34: 12 matched / 22 diverged / 0 unfixed
```

The F3 ring (every run identical):
`E0502:e0502-deref-removal=REV(2->2)  E0502:e0502-clone-out=REV(2->1),m0->1
E0502:e0502-read-relocate=KEEP(2->1)  E0499:e0499-swap-second-decl=KEEP(1->0)`
— the two honest reverts (Issue 100's lex rule), then the hoist, then the
swap. F3 heals DIVERGED-from-seed (the hoisted decl order differs from the
seed's) — behaviorally equal, G1-verified.

## T4 — the ordering A/B (n=46): the first IMPROVEMENT verdict

| arm | clean | iterations | reverts | attempts |
|---|---:|---:|---:|---:|
| static (min-delta) | 46/46 | 116 | 24 | 116 |
| flat counters | 46/46 | 114 | 22 | 114 |
| shape-conditional | 46/46 | **102** | **10** | **102** |
| nonergodic shared | 46/46 | 109 | 17 | 109 |
| nonergodic per-run | 46/46 | **102** | **10** | **102** |

`verdict: shape-conditional vs static — clean 46 vs 46, reverts 10 vs 24 —
IMPROVEMENT (T4.2 earns its keep)` — the first time any learned ordering beats
static on this corpus (Bench 089: all learned arms REGRESSED 22→10; Bench 090:
parity). The mechanism is the completed feedback loop: the counters now SEE
the doomed deref-removal/clone-out attempts reverting on the F3 strata (the
Issue-100 lex rule made them honest reverts), so the shape-conditional and
per-run arms skip them. The gain axis is WASTED ATTEMPTS, not clean rate —
same 46/46 everywhere, 14 fewer iterations per full-corpus pass.

## Floor movement (N unchanged, 258)

- driver lane: **66.7%** (was 62.0%; numerator 160→172 = the 12 F3 heals);
  E0502 row 33 matched / **38 diverged** / **0 unfixed** (was 33/26/12) —
  row fix-rate 83%→100%; **UNCONTESTED stratum 139/139** (was 127/139).
- **G1 semantics 172/172** (was 160/160): every F3 heal keeps the seed's
  ground-truth behavior (the hoisted read lands after the pushes, which
  append — v[0] unchanged).
- G2 p50 129ms/case (stable); p99 253→323ms — the F3 depth-4 tail is real
  (two reverts + two keeps per run), an honest cost for the 12 cleared cases.
- G-floor 41.5% unchanged (the harness fixer does not gain E0502 coverage —
  its numerator was already stable through Issues 099/100/101).

## Honest limits

- All numbers are SYNTHETIC-corpus numbers (the standing label); the organic
  `.heal/fixseq/` surface still has zero rings across 33 repos.
- F3 heals DIVERGED (not seed-matched): the hoisted order differs from the
  seed. G1 verifies behavior; a stricter matched-only metric would score F3
  at 0 — recorded so nobody reads "100%" as "seed-text-identical".
- The hoist crosses every statement between decl and first use — the
  semantic note names the class, G1 verifies the corpus, but an organic case
  whose crossed statements MUTATE the indexed place can change behavior while
  compiling. Same standing as clone-out (Bench 041's contested-opening
  discipline: the note names it, the oracle gates compile, tests gate
  behavior the corpus can see).
- The IMPROVEMENT verdict's gain is wasted-attempt reduction measured on a
  synthetic corpus with 2 doomed rungs; `static` remains the shipped default
  ordering (T4.2's promotion needs the owner pull evidence per the standing
  Issue 085 discipline).
