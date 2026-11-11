# Bench 149 — plan 633 T3.2: the G3 re-run under the issue-994 tie rule, over FRESH windows (the Bench-148 store is lost)

**Status:** EXECUTED 2026-11-11 (amo; instrument `examples/adoption_g3_cell.rs` at the α-flip-crossover disclosure added this cell — see the instrument note below; two full runs byte-identical, `cmp` clean). **Verdict: honest split — the T3.1 pre-registered bars: bar (1) the α=1 endpoint class PASSES (zero-rejection windows hold REFUSE through α=1, the Bench-148 G3-b defect family is gone); bar (2) zero tie-PROCEEDs at the default α PASSES (no exact-tie verdict exists anywhere in the census); bar (3) accepted-carrier demotes stay REFUSE PASSES (every demote of a rule carrying accepted fixes refuses at the default α). The instrument's aggregate G3-b/G3-c/G3-d lines FAIL on mechanisms OUTSIDE the T3.1 pre-registration: interior α-crossovers (evidence-driven, not ties), thin-window noise fragility (unchanged from Bench 148), and the abstain path unexercised. `adoption_gate` STAYS OPT-IN — the tie-fix is confirmed working; the store is too young for the wider bars.**

## The store substitution (read first)

The Bench-148 store (m3's five per-repo `.refine/adoption_outcomes.jsonl`) is **LOST**: the windows were never committed to any checkout (git log --all on all five repos: zero history), and the m3 working-tree `.refine/` dirs are gone (absent on amo, bmo, 4090 probes). This cell therefore runs on **FRESH windows accumulated on amo 2026-11-11** by the cell's own declared growth loop (Bench 148 Consequences #5): adoption-armed fix runs (`adoption_gate` + the release set, build `a62b88f21a54`) over eight repos, each sweep landing as a conventional `fix:` commit in its repo (viewbridge `a7f7ab7`, reflexer `f07c629`, dapps `8b06a45`, neuron-db `f0688a9`, rethink `03ab27a`; kat/deployer/instinct produced revert-only rows — nothing to commit).

| window | rows | accepted / rejected | tags |
|---|---|---|---|
| riir-dapps | 48 | 41 / 7 | 6 |
| riir-neuron-db | 7 | 5 / 2 | 3 |
| riir-viewbridge | 3 | 3 / 0 | 3 |
| riir-deployer | 2 | 0 / 2 | 1 |
| riir-kat | 1 | 0 / 1 | 1 |
| riir-instinct | 1 | 0 / 1 | 1 |
| riir-reflexer | 1 | 1 / 0 | 1 |
| riir-rethink | 1 | 1 / 0 | 1 |
| **total** | **64** | **51 / 13** | **17 tags across 8 windows** |

Both verdict shapes exist on real data (51 accepted fixes, 13 verify-reverts — the dapps/neuron-db sweeps carried the revert class). Replayed from `/tmp/g3rerun/*.jsonl` copies; the instrument never writes.

## The T3.1 pre-registered bars (plan 633, the ONLY bars this re-run was for)

### Bar (1) — "G3-b (the α=1 endpoint) MUST PASS on the re-run" — **PASS**

Bench 148's G3-b failure family: on zero-rejection windows the challenger and incumbent scores meet EXACTLY at α=1 and the then-pinned tie→challenger rule ADOPTED, demoting productive rules on no evidence. Under the issue-994 rule (ties are no-evidence and KEEP, decline-first): **viewbridge (3×1 accepted, zero-rejection) holds all three demote REFUSEs across the whole α grid including α=1** — the arithmetic: challenger score(1) = 1−rep_ch = 1.0, incumbent score(1) = 1.0, strict `>` refuses the tie. The endpoint class that produced all 5 Bench-148 flips is **extinct on the new windows**. Derivation + measurement agree.

### Bar (2) — "ZERO tie-PROCEEDs at the default α in the α-flip census" — **PASS**

No consult in the 8-window × 34-consult census produces an exact tie at the default α. (Bench 148 had exactly one — instinct's `hashmap-with-capacity` demote. The new census: zero.)

### Bar (3) — "the demote of a rule carrying accepted fixes stays REFUSE" — **PASS**

Every demote consult on a rule whose window rows include accepted fixes REFUSES at the default α: dapps `collapsible_match`/`doc_overindented_list_items`/`redundant_closure`/`shrink-to-fit-after-build`/`vec-with-capacity` (retained 0.9062–0.9688), neuron-db `excessive_precision`/`shrink-to-fit-after-build`, viewbridge all three, reflexer/rethink/instinct/kat single-tag windows (retained 0 — the NoEvidence-on-incumbent-rings refusal shape from Bench 148's pin correction #2, reproduced). **13 demote refusals / 6 readmit refusals across the census, every one matching the decline-first law.**

## The instrument's aggregate lines — FAIL, and what each failure ACTUALLY is

### G3-b aggregate: 6 α-flips — **FAIL, but none are the tie class**

The α-flip-crossover disclosure added to the instrument for this cell names each crossover:

| window | consult | crossover | mechanism |
|---|---|---|---|
| dapps | demote `collapsible_match` | α 0.80→0.90 | **interior score-line crossing**: challenger retained 0.9375 vs incumbent 1.0; challenger repeated 0.1667 vs incumbent 0.1842; the lines cross at α≈0.7825 (hand-verified) — the challenger's lower retention is outweighed by its lower repetition past that point. Evidence-driven, α-weighting doing its job — NOT a tie. |
| dapps | demote `unnecessary_get_then_check` | α 0.90→1.00 | interior crossing (same shape) |
| deployer | demote `explicit_auto_deref` | α 0.00→0.10 | **the α=0 identity**: window = 2 rejected rows of the consult's own tag; challenger score(0) = 0 = incumbent score(0) — exact equality at the α=0 endpoint refuses, then strict-improvement adopts. Derivation matches measurement. This is the α=0 mirror of Bench 148's α=1 class (an endpoint identity, not a productive-rule demotion: the rule carried ONLY reverts — demoting it is the gate's correct second reading, per Bench 148's deployer row). |
| neuron-db | demote `collapsible_match` | α 0.40→0.50 | interior crossing (its rows are 2 reverts + audit-filed accepted) |
| kat / instinct | demote `collapsible_match` / 1-row | α 0.00→0.10 | α=0 endpoint identity (single-revert windows, same deployer shape) |

**The reading:** the pre-registered bar was about the α=1 tie family; it PASSES. The aggregate census ALSO surfaces (a) interior crossovers — the score model doing exactly what α means, on rejection-bearing windows Bench 148 never had, and (b) the α=0 endpoint identity — the symmetric boundary case, present in Bench 148's store too but never grid-probed (its grid started at 0.1 with the default census at 0.5 only). Both are findings for a NEXT pre-registration, not defects of the tie fix. ⛔ No promotion claim rides this.

### G3-c aggregate: dominant min stability 0.688 < 0.90, LOO survived: false — **FAIL, unchanged mechanism from Bench 148**

The dominant refusal in thin windows (viewbridge's 1-row carriers) flips under its own row's noise — Bench 148's "evidentiary, not mechanical" finding, reproduced exactly on independent data. The quotientable set: neuron-db `shrink-to-fit-after-build` demote (0.938/0.969/1.000 across the p arms — meets the bar), rethink + reflexer demotes (0.969 at p=0.1, below at p=0.2), dapps consults (stable at both p arms in the census arms run). Per-consult quotability, never pooled.

### G3-d aggregate: abstains=0 — **FAIL (green-zero class)**

The abstain path (readmit on audit-filed-only tags) never fires: the new windows' audit-fifth filings (stride 5, dapps rows 5/10/15/…) never landed as a tag's ONLY rows. Bench 148 had one abstain (instinct's audit-filed `hashmap` readmit); this store has none. A next accumulation should mix repos with ≥10-row windows so audit-fifths produce single-row audit-only tags.

### Seat leg: 0 seat rows — **UNMEASURED (disclosed, unchanged)**

Waits for armed seat traffic, same as Bench 148.

## Determinism

Two full runs byte-identical (`cmp` clean). The `from_rows` replay self-check passed on every window (it gates the run). Box state: amo, AC plugged, quiet (the sweeps and the cell ran back-to-back on the same box; the cell is exact f64 arithmetic over ≤48-row windows, not a latency measurement).

## Instrument note (disclosed change)

`examples/adoption_g3_cell.rs` gained an α-flip **crossover disclosure** (this cell): when a consult's verdict shape changes between adjacent grid points, the crossing interval + shapes print (`[α-flip] … crosses between α=X and α=Y: A → B`). Zero behavior change to any bar computation — the census counter, the pre-registered bars, and every existing line are byte-identical (the Bench-148 invocation shape reproduced identically when run without the new lines' trigger conditions). Report-only example, never a library surface.

## Consequences

- `adoption_gate` stays **opt-in** — the T3.1 bars confirm the tie fix; the wider promotion posture still lacks the G3-d non-green-zero + quotable-dominant evidence on a grown store.
- **The three T3.1 pre-registered bars are all PASS** — the tie-rule decision (issue 994, Option A global) is CONFIRMED working on real data; plan 633 T3.2 adjudicates this row.
- The store-growth loop continues: dapps (48 rows) is the seed window; the five original Bench-148 repos remain sweepable once their lint surfaces regrow.
- Interior α-crossovers + the α=0 endpoint identity are filed as NEXT-CELL pre-registration material (never mid-cell edits — the Bench-148 law).
- The α-flip crossover disclosure rides the instrument from this cell forward.

Commits: the accumulating sweeps are in their own repos (hashes above); this record + the instrument change + the plan-633 T3.2 fold land in riir-refine + katgpt-rs. Session: katgpt-rs-33@amo, 1794417000.
