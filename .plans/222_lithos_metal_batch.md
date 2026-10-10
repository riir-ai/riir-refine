# Plan 222 — Batch 223: the lithos-metal kernel_opt batch (11 rules + 3 riders)

**Status:** ACTIVE — conversion in flight (2026-10-11, shikuwa/4090). Research 249 (drafts) + Research 248 (quote authority); source `lithos-ai/lithos-metal @ 236475fa716434807668562cd878ae623c2c1fd5` (Apache-2.0). Clone `.raw/lithos-b223` HEAD == pin EXACTLY (zero drift, verified at session start). Plan-first per Protocol A step 0 (Research-236/239 precedent).

## Adjudicated landing shape

11 OWN rules (the twelve drafts minus R9 — folded into R2 as the sibling-axis clause; T1's eligibility law folded into R2 as the same conservative-default corroboration; T2's name-collision law folded into R8 as the secondary clause; T3 = research-note instance only) + 3 count-neutral riders (RD1 → `gdn-transition-replay-log-commit-many` [entries_gpu_08.rs], RD2 → `delay-rms-scale-past-gemv-epilogue` [entries_gpu_02.rs — the commuted-norm epilogue family, located by grep], RD3 → `lut-free-bit-spread-vadd4-unpack` [entries_gpu_33.rs]). Corpus 784 → 795.

DISCLOSURE: the Research-248 revised yield estimated 4–6 rules + 4–5 riders; every draft survived fresh dedup with zero owners (Research 249 prep-time greps + the B193 freshness re-verification at the pin this session), so the batch lands ABOVE the estimate with the fold set trimming twelve drafts to eleven rules. The falsification the estimate priced never materialized.

## Category deltas (tests.rs `expected` array + KERNEL_CATEGORIES)

A (MemoryAccess) +3 (R1 coherence-scope, R8 shared-partials, R10 derived-pack) 159→162 ·
C (Fusion) +1 (R4 fusion-scope-by-barrier-cost) 54→55 ·
D (Dispatch) +3 (R2 open-group barrier [+R9+T1 clauses], R5 gpu-resident control loop, R6 verify-length) 182→185 ·
E (Parallelism) +2 (R3 barrier-protocol ladder, R7 coprime-steal) 73→75 ·
H (MoeDispatchBackward) +1 (R12 moe-readiness-queue) 37→38 ·
K (AutotuneDispatch) +1 (R11 autotune-fidelity) 14→15. Sum 795.

## Slices (disjoint write sets; 3 authoring subagents + coordinator wiring)

- **Slice A** → `entries_gpu_85.rs`: R1, R2 (+R9 bus-first + T1 arena-eligibility clauses), R3, R4. Files: static_fusion.py (stage/fp8_tile_decoder), barriers.py, region_fusion.py, emit.py (_bus_first, #113, arena.py reuse_arenas), design docs (coherence granularity, fusion boundaries).
- **Slice B** → `entries_gpu_86.rs`: R5, R6, R7, R8 (+T2 params-identity clause). Files: advance.metal, spec_ops.metal (verify_select/accept_scan), steal.metal, probes p10, mixers.md, gqa_prefill.metal, shared_partials.py, emit.py scratch(shared=True) + #113.
- **Slice C** → `entries_gpu_87.rs`: R10, R11, R12. Files: fp8_tiles.py, nvfp4_tiles.py, moe_tiles.py, weight_windows.py (compact_weights), autotune.py, gemv_tuning.py, moe_ready.py, moe_down.py, moe_fusion.py.
- **Coordinator**: 3 riders (host files 08/02/33), mod.rs wiring (mod decls ×3 + extends ×3 + KERNEL_CATEGORIES rule_id additions ×11), tests.rs (count pins 784→795 ×4 sites, category array ×6 deltas, batch223 parse+needles gate, batch223 riders gate), docs.

## Tasks

- [x] T0 Protocol H (fetch in-sync at `19522892`; clone at pin HEAD-exact; worktrees clean; disk E:113G/C:40G owner-gated as-found)
- [x] T0.1 Read Research 248 (quote authority) + Research 249 (drafts) in full; locate rider hosts by grep (RD1=entries_gpu_08, RD2=entries_gpu_02 delay-rms, RD3=entries_gpu_33 lut-free); capture house style (entries_gpu_84) + gate format (batch218_riders gate) + KERNEL_CATEGORIES shape
- [x] T0.2 THIS PLAN (Protocol A step 0) + `.plans/.highwater` 221→222 + dual_allocation_gate at allocation
- [ ] T1 Claim marker row appended to `.distill/001_mining_queue_snapshot.md` BEFORE authoring (the Batch-221 mitigation candidate, second use) — commit+push before slices start
- [ ] T2 Fresh dedup re-greps at write time (Protocol A step 2): each rule_id + 3 vocabulary variants against src/domains/kernel_opt/corpus/ + rust_perf corpus — recorded in the slice briefs; slices re-confirm zero owners
- [ ] T3 Slice A authoring (4 rules, entries_gpu_85.rs) — every quote re-verified at the pin clone BEFORE the entry lands (the draft quote is a claim, the clone is the oracle); original Rust fixtures; NotBenchable annotations where the draft grades say so; distinct-from sets verified present
- [ ] T4 Slice B authoring (4 rules, entries_gpu_86.rs) — same discipline
- [ ] T5 Slice C authoring (3 rules, entries_gpu_87.rs) — same discipline
- [ ] T6 Riders RD1/RD2/RD3 appended to host gpu_concepts (verbatim quotes verified at the pin; `RIDER (Batch 223 / Plan 222 ...)` format + closing `Batch 223 rider / Plan 222.` marker; count-neutral)
- [ ] T7 Wiring: mod.rs (mod decls + extends + KERNEL_CATEGORIES ×11 rule_ids in A/C/D/E/H/K slices); tests.rs count pins 784→795 (×4 sites with honest labels) + category expected array (A 159→162, C 54→55, D 182→185, E 73→75, H 37→38, K 14→15) + `batch223_rules_parse` gate (11 rule_ids × needles) + `batch223_riders_landed_on_their_hosts` gate (3 hosts × needles)
- [ ] T8 Gates: batch223 gates FIRST-SHOT-or-fixed; kernel_opt+latent_retrieval lib suite (isolated CARGO_TARGET_DIR, polite posture under sibling lanes); default lib; clippy -D ×3 postures (default / all-features / no-default); retrieval floors HELD (no re-pin expected); freeze/score-bench NOT owed (zero clippy-corpus delta, P30)
- [ ] T9 Docs: AGENTS.md kernel_opt bullet + count (×2 sites); README kernel row + tree count; .docs/11_domains/kernel_opt.md ledger row; Research 249 Status → LANDED (+ landing addendum: quote re-verification results, any drift corrections); this plan COMPLETE
- [ ] T10 Snapshot landing row (`.distill/001_mining_queue_snapshot.md`) + ephemeral clone `.raw/lithos-b223` removed (the ephemeral-clone law) + commit `feat: batch 223: lithos-metal kernel_opt pack (Plan 222 / Research 249)` + push
- [-] T11 (defer) R3 optional staged GOAT (CubeCL-on-Metal polling-fan-in-ladder A/B, the B204→T11 pattern) — NotBenchable is the default posture; stage only if a scored lane needs the datum. Reason: no consumer lane is gated on it this cycle.

## Quote-verification ledger (the T1 duty — fill during T3–T6)

Coordinator spot-checks + slice-agent verification; corrections recorded here (the draft is a claim, the pin is the oracle):

- (to be filled at landing)
