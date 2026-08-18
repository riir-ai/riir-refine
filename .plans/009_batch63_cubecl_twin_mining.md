# Plan 009 — Batch 63: the CubeCL twin file-mining batch (`ternary_deltanet_gpu_forward.rs`)

**Created:** 2026-08-19 (plan-first discipline — BEFORE any corpus edit)
**Source:** `crates/riir-gpu/src/ternary_deltanet_gpu_forward.rs` @ origin/develop `42cc13e0d`, **5,432 lines**, committed-clean (the sibling is active on the CUDA twin only — re-verified at batch start; extracted byte-exact to `target/b63/cubecl_twin.rs` for lane stability).
**Precedent:** Batch 58 (Research 065) mined the cudarc twin (`ternary_deltanet_gpu_forward_cudarc.rs`, 5,691 lines → 8 rules, corpus 355→363). The CubeCL twin shares the algorithmic core but differs in dispatch substrate (CubeCL runtime + WGSL/`#[cube]` kernels vs raw CUDA + cudarc) — B58 took the TRANSPORT/capture/staging/rotation-pool mechanisms common to both; this batch's unique surface is **CubeCL-specific dispatch + the chunked-recurrence kernels (IntraOutput/CrossChunk/StateTransition)** + the 721-era tree-verify hooks.

## Expected yield

LOW-MEDIUM (the queue's honest estimate): B58 already took the shared mechanisms. Realistic: 2–5 rules. A ZERO-rule landing is a legitimate outcome (B50 precedent — the file may be a museum of already-mined anti-patterns; the hazard yield is then the value).

## Phases

### Phase 0 — dedup re-verification (mine-time)
- [x] Two-lane read — DONE (Lane A: 3 candidates + 11 dedup drops + 7 C-drops + 10 hazards; Lane B: 0 candidates — structurally empty, the kernels live in sibling modules mined at B28/B58; 24 examinations all duplicates). Reports in Research 071. of `target/b63/cubecl_twin.rs` (5,432 lines): Lane A = dispatch/orchestration/pooling half; Lane B = kernels/chunked-recurrence/tree-verify half. Each lane: candidate mechanisms named + one-line evidence quotes + line refs.
- [x] Coordinator verification — DONE (both A-grades verified against the actual B58/B9 rule texts; the Lane-A-vs-B concat-family disagreement adjudicated: segments = launch-time segment table needing a launcher API, concat = upload-time layout transform using the stock kernel — genuinely distinct branches). of every candidate's quote against the file (the B51 lesson: coordinator verifies load-bearing claims before landing).
- [x] Dedup — DONE (both lanes ran both-vocabulary greps; the 18th count-pin site found: router.rs experts-cover pin). ( BOTH vocabularies: source terms + operator names) + the B58 rule set specifically (async-htod, captured-graph-bakes, output-buffer-rotation-pool, mid-pipeline-device-alloc, capture-region-dispatch-only, occupancy-grid-cap-refuted, write-before-read-state-skips-checkpoint, same-input-gemv-segments).
- [x] Sibling state re-verified — the CUDA twin dirty (theirs), the CubeCL twin committed-clean; corpus edits confined to riir-clippy (clean repo). (the CUDA twin stays dirty; the CubeCL twin + corpus files must be clean).

### Phase 1 — entry authoring (entries_gpu_07.rs continues; open 08 only past ~950 lines)
- [x] Rules authored — 3 landed (same-input-gemv-weight-concat-at-upload D; deferred-residual-fused-into-next-layer-norm C; reset-state-in-place-fill-never-realloc A) in entries_gpu_07.rs.; zero-rule outcome lands as a research note + queue update, no corpus edit).
- [x] Distinct-from clauses — in every gpu_concept (B58 segments / B15 fusion for rule 1; B9 / B19 + the composition ladder for rule 2; B27 / B39 for rule 3). (esp. vs the B58 twins + the B28 deltanet CubeCL cluster — `deltanet_cubecl.rs` was mined at B28: row-owned-register-blocked-recurrence, workgroup-equals-plane, smem-broadcast-then-scatter, token-flattened-modulo, fused-conv-shift-dot-silu, softplus-clamp, reverse-running-decay, split-decay-outputs, carry-dual-layout, nonlinearity-forces-raw-buffers).

### Phase 2 — standing gates
- [x] batch63_rules_present + batch63_concepts_covered + batch63_pseudocode_parses — all green. (pseudocode_parses / rules_present / concepts_covered — the standing trio).
- [x] Existing tests green — 811/0/1i kernel_heal, 607/0 default. (counts, dedup, retrieval).

### Phase 3 — GOAT
- [x] G1 — DONE (syn-parse + coordinator-verified quotes). `buggy_code`/`fixed_code` + pseudocode parse + reflect REAL file content (coordinator-verified quotes).
- [x] G3 — DONE (811 kernel_heal + 607 default + clippy 0 all-targets both states — 3 doc-backtick fixes in the header line). — the Batch-62 lesson: lint the TEST target too).
- [x] G4 — no new deps (corpus + tests only).
- [x] Retrieval floors — HELD (gate green, no re-pin). re-pin only if legitimately re-ranked (floors = baseline − 1).

### Phase 4 — count pins + records
- [x] Count pins — tests.rs 364→367 x4 + per-category table (A 87 / C 34 / D 71) + sum x2 + mod.rs category comments + exhaustive doc + router.rs experts pin (the 18th site). (corpus header, `kernel_categories_counts_match_plan_003_table` where categories shift, tests' expected counts — the 17+ lesson).
- [x] Research 071 + AGENTS.md Batch 63 entry + mining queue snapshot — DONE in the landing commit. + AGENTS.md Batch 63 entry + `.docs/` + mining queue snapshot (QUEUED → DONE); NEVER SKILL.md (instructions-only since 94655ac).
- [x] Hazard yield — filed as a riir-ai issue via the worktree dance (10 hazards, Research 071 §Hazard yield). filed as a riir-ai issue via the worktree dance (the repo is sibling-active with a stale local line).
- [x] .plans/.highwater = 9; .research/.highwater = 71.; `.research/.highwater` 70 → 71.
- [x] Commit + push on develop. + push on `develop`.

## Non-goals

- No perf benches (kernel rules are corpus patterns, not Pareto-benched — the standing kernel_opt discipline).
- No proposer work (kernel domain has no proposers).
- The CUDA twin stays the sibling's territory — no edits to `*_cudarc.rs`.
