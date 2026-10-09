# Bench 110 — the repr(align(8)) store-forwarding M3-Max GOAT (Batch 211, Plan 204 T11 pattern)

**Status:** MEASURED 2026-10-10 ~07:5x–08:1x +07 — verdict **NOT-REPRODUCED-LOCALLY** (the honest outcome, recorded as such; the rule `repr-align-register-partition-store-forwarding` STAYS source-reported, no corpus edit owed). Highwater 109→110 (dual_allocation_gate clean 0/0/0 at allocation).

**What this is:** the staged local evidence for the Batch 211 rule — the bevyengine/bevy #26042 `EntityLocation` distill (`#[repr(align(8))]` tips LLVM's register partition so the joined (row, table_row) read forwards in-register instead of round-tripping through a spill). The rule is NotBenchable as a corpus rule (every figure in its context_notes is a FOREIGN source-reported bench); the Batch 211 queue note staged THIS bench as the local evidence gatherer: `benches/bench_006_repr_align_m3_goat.rs` — the Plan 204 T11 pattern (hand-written, self-contained, correctness-gated, provenance-quoted).

**The A/B:** the rule's own fixture shapes, hand-written TWICE (the textual duplication IS the A/B — identical bodies, the ONLY difference the struct's `#[repr(align(8))]`):

- **Shape F** — the rule's `store_then_key` fixture verbatim: push the 4×u32 location into a Vec (the aggregate store) then compute `cell_key` (the joined (row, table_row) read).
- **Shape C** — the bevy construction loop: locations loaded from a table, stored into a DISTINCT destination aggregate (same 4-field shape, the `UnsafeEntityCell` stand-in), keys computed from the loaded location.

Both driven over 4 table sizes {4k, 16k, 32k, 64k}, interleaved best-of-5 with 2 warmup passes, correctness-coupled sinks (every timed pass re-asserts its accumulator against the G1 reference — a deleted loop FAILS, never reports a fast zero).

**Gates:**

- **G1 PASS** — bit-identity across the attribute: `cell_key` per-element + both shape drivers' accumulators + the stored Cell fields, all identical (4096-fixture sweep + field-by-field check).
- **G4 PASS** — zero allocations in all four measured paths (counting allocator armed; sinks pre-sized).
- **G2 measured, PROVENANCE-gated** — the gate posture (`REPR_ALIGN_BENCH_REQUIRE_PROVENANCE=1`) refused to produce citable numbers until preflight PASSED.

**The citable runs (four total: 1 provisional under load 6.96, then 3 gate-posture runs — the last AFTER the clippy `needless_range_loop` heal (index loop → `iter().enumerate()`, semantically identical), re-run to confirm the recorded numbers hold on the committed binary):**

```
PROVENANCE: power=AC Power load=3.04 swap=19984.50M canary=skipped powermode=2(high)   # run 2
PROVENANCE: power=AC Power load=2.96 swap=19984.50M canary=skipped powermode=2(high)   # run 3
PROVENANCE: power=AC Power load=2.46 swap=19984.50M canary=skipped powermode=2(high)   # run 4 (the committed binary)
```

Run 2 (gate posture, PASSED preflight):

| n | shape | align4 (µs) | align8 (µs) | ratio |
|---|---|---|---|---|
| 4096 | F | 7.000 | 7.125 | 1.0179 |
| 4096 | C | 6.500 | 6.292 | 0.9680 |
| 16384 | F | 29.042 | 29.250 | 1.0072 |
| 16384 | C | 26.958 | 27.000 | 1.0016 |
| 32768 | F | 59.375 | 59.417 | 1.0007 |
| 32768 | C | 54.833 | 52.875 | 0.9643 |
| 65536 | F | 118.042 | 119.083 | 1.0088 |
| 65536 | C | 109.875 | 110.000 | 1.0011 |

Run 3 (gate posture, PASSED preflight):

| n | shape | align4 (µs) | align8 (µs) | ratio |
|---|---|---|---|---|
| 4096 | F | 7.042 | 7.125 | 1.0118 |
| 4096 | C | 6.583 | 6.583 | 1.0000 |
| 16384 | F | 29.208 | 29.334 | 1.0043 |
| 16384 | C | 27.000 | 26.959 | 0.9985 |
| 32768 | F | 57.333 | 57.500 | 1.0029 |
| 32768 | C | 52.916 | 52.916 | 1.0000 |
| 65536 | F | 115.042 | 115.333 | 1.0025 |
| 65536 | C | 106.042 | 105.417 | 0.9941 |

Run 4 (the committed binary, after the clippy heal — the record's citable table):

| n | shape | align4 (µs) | align8 (µs) | ratio |
|---|---|---|---|---|
| 4096 | F | 7.041 | 7.167 | 1.0179 |
| 4096 | C | 6.625 | 6.583 | 0.9937 |
| 16384 | F | 28.500 | 28.583 | 1.0029 |
| 16384 | C | 26.375 | 26.375 | 1.0000 |
| 32768 | F | 57.292 | 57.791 | 1.0087 |
| 32768 | C | 52.917 | 52.916 | 1.0000 |
| 65536 | F | 114.750 | 114.791 | 1.0004 |
| 65536 | C | 105.958 | 106.000 | 1.0004 |

(The provisional run 1 under load 6.96 showed the same picture — all deltas sub-1.5% — and is NOT evidence; the preflight refused it.)

**The verdict: NOT-REPRODUCED-LOCALLY.** Every align-8 delta across all three citable runs sits in the ±1.8% band; the two apparent "wins" (run 2: shape C at 4096 −3.2%, 32768 −3.6%) do not repeat in runs 3-4 (same cells: 0.0%/0.0% and −0.6%/0.0%) — sub-noise flip-flopping, not effect. The one "behind" cell (shape F at 4096, +1.0/+1.8%) fails to reach a consistent magnitude beyond the band. The local fixtures do not exhibit the partition stall on this toolchain (M3 Max, rust 1.98.1 pinned toolchain, `bench` profile = release + `bench = true`).

**What this does and does not claim:**

- It does NOT contradict the source. The bevy PR's stall arose in the FULL `UnsafeEntityCell` construction context after the #25632 layout change (their bench bisect found it, their assembly read located it) — a context a stand-in cannot reproduce by construction. The rule's own anti-regime clause said exactly this before this bench existed: "an attribute without a failing benchmark is speculative layout churn (measure first, the CSE crossover rule's two-sided law)."
- It DOES close the staged evidence item: the local A/B was priced and run; the attribute shows no local gain on the fixture shapes; the rule stays source-reported (provenance figures only, never local bars), exactly as its corpus annotation already reads. No corpus edit owed (the annotation already says NotBenchable-on-this-box — this bench is the receipt for that claim, not a correction to it).
- The bench file stays in-tree as the re-measurement instrument: a future LLVM/toolchain bump (the pinned toolchain moves) re-runs it in one command (`REPR_ALIGN_BENCH_REQUIRE_PROVENANCE=1 cargo bench --bench bench_006_repr_align_m3_goat`) — if a toolchain change ever exposes the partition stall locally, the same bench flips to REPRODUCED and the rule gains its local evidence retroactively.

**En-route notes:**

- The first macro-generated draft (`fixture_fns!`) hit the macro-path-hygiene wall (`expected identifier, found metavariable` on `use super::$loc as Loc;`) — rewritten as hand-written twin modules per the B202 macro-trap precedent (the textual duplication is arguably BETTER for an A/B bench: a reviewer sees both bodies are identical).
- `const _: () = { assert!(size_of/align_of ...) }` pins the layout contract (16 B both forms; align 4 vs 8) — the A/B's premise is asserted at compile time, never assumed.
- Box state across the session: the hyperthink census exited 05:10 (T1 premise FAIL — its own pre-registered negative, recorded in riir-infer `.plans/612` as the window marker); two sibling cargo spikes (7.95, 6.96) were RIDDEN OUT, never averaged in — every citable number above is from a PASSED preflight at load 2.9-3.0.

**Queue after (unchanged standing order):** the conversion set — Leads 1+2 (llama.cpp #29608 pinned-ring upload staging + #30087 GDN four-columns-per-warp) + the P45 rider + Lead 3 (#28713 top-k selection) first, then 4–8, 9/10/11; Lead 12 at the head of kernel_opt staging; the cubecl k=8-MMA lead (Research 250) at the rider tier; then the lithos batch (transcription-ready per Research 249, plan-first Protocol A step 0, re-clone at pin `236475fa`); walk #14 Tue 10-13; mlxfast floor ~10-11 (local-fetch instrument); nnethercote ~11-01.
