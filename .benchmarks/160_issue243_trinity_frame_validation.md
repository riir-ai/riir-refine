# Bench 160 — Issue 243's frame-validation pass: the evaluation trinity (T1 × T2 × T3) over the committed Bench-150 windows

**Status:** EXECUTED 2026-11-17 (amo, release binary at this record's landing commit; two full runs byte-identical on program stdout, `cmp` clean; the instrument's stability numbers reproduce Bench 150's disclosed fractions exactly — see Cross-check). **Verdict: the falsifier resolves NOT-NEGATIVE — T2/T3 surface real verdict-relevant content beyond Bar C's census (the readmit-PROCEED permission class and the demote-PROCEED class are NEVER qualified in the executed record; 12/17 readmit-PROCEED verdicts rest on single-row evidence; the population hit rate at the 0.90 bar is 8/48). The mechanism is Bar C's own (thin-window noise fragility → per-consult qualification, never aggregate pooling) — the trinity WIDENS the quotability contract to every verdict class rather than finding a new mechanism. The arms join the next G3 cell's pre-registration (issue 243 stays open for exactly that adoption); T1 stays PROXY-labeled; no executed bar of Benches 148/149/150 was recomputed or re-adjudicated.**

## The pass (pre-registered in issue 243)

- Inputs: the COMMITTED Bench-150 windows (`.benchmarks/150_plan633_t33_crossover_alpha0_growth_data/windows/*.jsonl` — 8 windows, 78 rows, 24 tags, 48 consults). Nothing re-swept, nothing re-executed.
- Instrument: `src/trinity_frame.rs` (`compute_frame`) + `examples/adoption_trinity_frame.rs` (thin CLI). The perturbation ensemble reproduces the executed cell's procedure exactly — same `XorShift`, same per-(p, arm, seed) seeding formula (the `bool as u64` spelled `u64::from`, same 0/1), same arm filters, same eligible-row rule, same 1.0-on-no-eligible-arm convention — so every stability number is directly comparable to Bench 150's disclosed fractions. Defaults are the executed posture: α₀ = 0.5, noise arms 0.1/0.2, 32 seeds, bar 0.90.
- Replay self-check (`from_rows` consults identically) passed on every window before any number was read.
- Arms self-test: `tests/adoption_gate_gates.rs::trinity_arms_compute_and_are_deterministic` — the fixture-planting survival clause (the arms compute at all), never the falsifier.
- Full output: `run_output.txt` beside this record (committed, store-preservation discipline).

## Cross-check (the comparability proof)

Every fraction the executed record disclosed for its Bar-C set reproduces exactly: dapps `demote collapsible_match` 0.719, `doc_overindented_list_items` 0.781, `redundant_closure` 0.625; dapps `readmit redundant_closure` 13/32 = 0.406; the dominant refusal (dapps `demote unnecessary_get_then_check`) 1.000. The ensemble is the executed one.

## What the arms measured (48 consults, both kinds × 24 tags)

- **T1 (PROXY arm — disclosed; committed windows carry no forward rows):** hits **8 / 48** winner-carrying consults at the 0.90 bar; **40 misses**; 0 abstain-excluded. Per window: dapps 4/12, deployer 1/2, instinct 0/2, kat 0/2, neuron-db 3/20, reflexer 0/2, rethink 0/2, viewbridge 0/6.
- **T2 (the mass table, all verdict classes, nothing pooled):** the readmit-PROCEED class — the gate's PERMISSION verdicts — is **17 consults fleet-wide, 12 single-row-backed** (dapps 2/4, neuron-db 5/8, viewbridge 3/3, reflexer 1/1, rethink 1/1), min stability 0.406. The demote-PROCEED class includes the three all-rejected single-row carriers (deployer/kat/instinct `collapsible_match` — the α=0 endpoint class, Bar B's subject) at 0.562–0.781 stability, plus neuron-db `demote collapsible_match` at **0.125**. The demote-REFUSE class carries the record's known shape (dapps mass 1..38; neuron-db 5/9 single-row; viewbridge 3/3 single-row).
- **T3 (identity):** 42/48 consults flip under some perturbation. The cross-class join shows fragility is TAG-concentrated — the same tag is fragile in demote+readmit in 15 of the 17 fragile tags — and the single most fragile verdict (neuron-db `demote collapsible_match`, 0.125) sits 0.004 above its own mechanically-derived crossing α* = 0.4958 (Bench 150's Bar A table): the verdict is a hair from its flip point, which the LOO census now shows row-by-row (16 single rows move it).

## The adjudication (the falsifier)

The executed record qualifies ONE class: its consequence line says *"the gate refuses demotion of productive rules" is quotable ONLY for multi-row carriers*. The aggregate lines give readmit-refusals=7 as a count; the run output lists per-arm below-bar names (raw, unconsolidated). What the record NEVER says:

1. **The readmit-PROCEED class is never qualified** — and it is the permission direction (re-admit a demoted rule): 12/17 rest on single-row evidence, the worst at 0.406 stability. An operator reading the record could believe the readmit path is solid. The symmetric contract ("re-admission permitted — quotable only for multi-row clean histories") is missing.
2. **The demote-PROCEED class is never qualified** — both the all-rejected endpoint verdicts (the G1 happy path: "demoting a pure-reject adopts" is 1-row-evidence fragile) and the α*-adjacent PROCEED (0.125).
3. **The population hit rate (8/48)** — per-consult reliability at the 0.90 bar is the exception, not the rule, on this store.

That is NOT "nothing the aggregate lines + Bar C don't already say" — so the pre-registered negative close does NOT fire. Equally honestly: **no new mechanism** — the mechanism is Bar C's own (thin windows → noise-fragile verdicts), and the remedy is Bar C's own (per-consult qualification, never pooling). The verdict-relevant delta is COVERAGE: the quotability contract extends to every verdict class, which is exactly what the arms tabulate.

**Consequence (the issue's task):** the three arms join the NEXT pre-registered G3 cell's bar set unchanged from their validated form (T1 on the forward-outcome reference where forward rows exist, the proxy labeled as proxy; T2's mass table across all classes; T3's flip identity). The executed Benches 148/149/150 stay untouched — their refusals-side findings stand as recorded; this pass adds the missing qualification HALF (permission-side verdicts) without re-adjudicating them. No gate code, no scorer, no posture changed; `adoption_gate` stays opt-in (Plan 633's standing verdict).

## Determinism + box

Two full runs byte-identical (`cmp` clean). Exact f64 arithmetic over ≤48-row windows — not a latency measurement; box contention immaterial (disclosed: amo, AC, a sibling cargo build running concurrently; release build in an isolated `CARGO_TARGET_DIR=/tmp/i243`, removed at teardown).

Commits: riir-refine — instrument + record (this landing). Session: amo-idle-i243, 1794927066.
