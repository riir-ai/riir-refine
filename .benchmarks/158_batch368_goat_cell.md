# 158 — Batch 368 GOAT cell: the kache build-cache distill benchable lead (Plan 382)

**Status:** R1 `restore-via-cow-clone-not-copy` **GOAT PASS** per the plan's pre-registered bar (the batch's only benchable rule — R2 `singleflight-identical-work-per-content-key` landed NotBenchable-by-domain: the win is a concurrency-liveness property the single-threaded harness cannot price; the entries_77 fallback — decision content IS the record, and the 15/15 standalone preflight demonstrates the law).

PROVENANCE (bench_preflight, quoted verbatim): `power=AC Power load=3.98 swap=0.00M canary=skipped powermode=2(high)` at run 1; `power=AC Power load=4.40 swap=0.00M canary=skipped powermode=2(high)` at run 2. Box: m3 (M3 Max 16-core), AC, High Power mode DISCLOSED (powermode=2 — the detector's three-state enum), load under the 6.0 MAX_LOAD bar at both runs (a sibling session's build had drained by the first cell; the earlier 7.76 refusal is why the bench waited for this window). The validator's own cargo invocation builds a temp bench crate per rule; timing medians, dynamically calibrated; `run_bench_with_determinism(rid, spec, 2)` runs the binary twice per invocation and asserts the alloc axis bit-identical — green on both.

## Bars (pre-registered at claim time, BEFORE the first run — Plan 382 T4)

- G1: restored-byte census identical both arms (asserted in `make_input` at every run, both postures).
- G2: ParetoDominant on bytes-written + wall at [64 MB, 256 MB, 1 GB], two-run stable. The MODEL harness stands in for the syscall: the copy arm allocates + writes every word (the validator's alloc-bytes axis measures the full O(size) write), the clone arm touches metadata only (0 data bytes). The real clonefile(2) gap is LARGER than the modeled gap (a syscall vs an in-process memcpy), so the modeled verdict bounds the real one from below in gap magnitude; the direction is identical.
- G3: no-regression — the copy arm IS the incumbent; the clone arm must never be slower on any size.
- G4: alloc-free both arms (the clone arm allocates nothing in steady state).
- A missed bar DEMOTES in the landing commit (the ungoat law).

## Cell — `restore-via-cow-clone-not-copy`: PASS (two-run stable)

Instrument: `cargo test -p riir-refine --lib --features rust_perf_bench batch368_goat -- --ignored --nocapture`.

Run 1 (load 3.98):

| size | buggy ns | fixed ns | speedup | allocs (buggy→fixed) | bytes (buggy→fixed) | outcome |
|---|---|---|---|---|---|---|
| 67,108,864 (64 MB) | 8,287,875 | 4,668,500 | 1.78× | 1→0 | 67,108,864→0 | Faster |
| 268,435,456 (256 MB) | 33,406,541 | 18,649,625 | 1.79× | 1→0 | 268,435,456→0 | Faster |
| 1,073,741,824 (1 GB) | 138,034,834 | 74,801,709 | 1.85× | 1→0 | 1,073,741,824→0 | Faster |

Run 2 (load 4.40):

| size | buggy ns | fixed ns | speedup | allocs (buggy→fixed) | bytes (buggy→fixed) | outcome |
|---|---|---|---|---|---|---|
| 67,108,864 (64 MB) | 8,230,583 | 4,647,625 | 1.77× | 1→0 | 67,108,864→0 | Faster |
| 268,435,456 (256 MB) | 33,580,333 | 18,676,209 | 1.80× | 1→0 | 268,435,456→0 | Faster |
| 1,073,741,824 (1 GB) | 137,716,750 | 74,776,042 | 1.84× | 1→0 | 1,073,741,824→0 | Faster |

Verdict class: **ParetoDominant 3/0/0 two-run stable** — every size Faster on both runs, medians within ~1% across runs, the alloc + bytes axes bit-identical (determinism leg asserted). G1 ✓ G2 ✓ G3 ✓ G4 ✓.

## Honest disclosures

- The harness is a MODEL of the mechanism (a modeled FS: copy = per-word write; clone = metadata-only), never pasted kache source — the license law. The bytes-written collapse to 0 is the model of the CoW share; the wall speedups 1.77–1.85× measure the model's memcpy-vs-skip, a LOWER bound in ratio than the real syscall-class gap the upstream measured in its own adoption pilot (bench 157: per-worktree restore disk 92 MB → ≈0, warm-store walls ~3.7–4.3× under contention).
- The parity census (G1) folds the consumer-visible restored bytes; the dirty-page counts are the METRIC axis, not the census (the first draft folded dirty-page XORs into the census and the gate red it — fixed pre-landing).
- powermode=2 (High Power) is disclosed; the bars are relative (both arms same process, interleaved calibration) so a power-mode wall shift moves both arms together.
- The G3 no-regression bar reads on the model: the ladder's clone rung DEGRADES to copy on a no-CoW volume (the anti-regime arm in b368_fixtures F1 proves the fallback is byte-identical + correct), so the real-world never-regression property rests on the loud-not-fatal fallback law (link.rs:685-689, quoted in-rule), not on the model bench.

## Landing state

rust_perf 356 → 358 (entries_83: R1 BENCHABLE + R2 NotBenchable) + 3 count-neutral FACES (R3 onto kernel_opt `compiled-handle-direct-launch-key-completeness`, R4 onto `latency-histogram-split-on-structural-populations`, R5 onto `pending-claim-registry-take-consumes-drop-clears`). Fixtures b368 15/15 standalone pre-entry. Plan 382 / Research 414.
