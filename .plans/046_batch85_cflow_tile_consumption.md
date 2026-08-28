# Plan 046 — Batch 85: cflow tile-consumption-order distill (rust_perf)

**Status:** OPENED 2026-08-28 (verdict Research 098) — plan-first per the 2026-08-18 discipline; ALL corpus edits build-gated (box saturated: two sibling test-suite runs + bench_791 sustained thermal A/B active, Plan 337). Execute at the next quiet-build window.

Source: arXiv:2608.23841 "Pipeline-Native Transformers" (Poperszky, CC BY 4.0) — distill verdict **YES B+ → rust_perf** (NOT kernel_opt — the CPU-trap routing note is in Research 098). Full candidate table + dedup evidence live in `.research/098_cflow_pipeline_native_distill.md`.

## Phase 1 — re-verification (grep-only, can run any time)

- [ ] Re-run the dedup greps at WRITE time (the folder re-scan lesson): `weight-tile|consumption-order|prefetch` against `src/domains/rust_perf/corpus.rs` + kernel_opt entries — corpus moved since verdict (sibling Batch 84 lands kernel_opt; verify zero new collisions).
- [ ] Confirm count pins baseline before any edit: `cargo test --features rust_perf_bench --lib` count + the annotated/benchable header numbers in corpus.rs (the 31-count-pin-sites lesson — record the pre-edit numbers in this file).

## Phase 2 — entry authoring

- [ ] Author rule `weight-tile-consumption-order` (category `layout`): buggy = row-major weight matrix streamed per-row in a single-token matvec (activation re-fetched per row-tile, L1 misses ∝ matrix size); fixed = weights repacked ONCE at setup into L2-sized tiles (e.g. 128×256 f32 or Q4-block groups) emitted in consumption order; the matvec walks tiles sequentially, activation slice L1-resident across the tile's rows, partial sums committed per output stripe. Original minimal fixtures (license hygiene — never paste source).
- [ ] In-rule differentiations: vs `transposed-weight-register-gemv` (B62 — register/accumulator granularity, composable), vs `loop-order-innermost-contiguous` (B64 — reorders loops over unchanged layout, this repacks the data), vs `soa-hot-field` (struct fields, not matvec weights), vs kernel_opt B14/B15 (GPU global-memory access-count rules — cousins, cite). Sibling-projection stripe interleave (QKV/gate-up by output stripe) as the variant note.
- [ ] Author rule `software-prefetch-storage-bound` (category `io`, annotation class per the `powi-vs-powf`/`varint` honest-loser precedent): "adding `_mm_prefetch`/prefetch threads to a bandwidth-bound streaming workload buys ≈nothing and can be ACTIVELY harmful" — source evidence: PF A/B ±0.1% at 64 MB / PF=1 4% WORSE at 4.7 GB direct-I/O; end-to-end the prefetch thread burned ~48% of decode time (stride-256 expert walks + cache pressure) — disabling it alone = 1.92×. Two-regime: compute-bound (prefetch can hide latency) vs bandwidth/storage-bound (adds cache pressure only). Suggestion-only or proposer-less by design (a losing rewrite never gets a `--fix` path — the `filter-count-to-any` rationale).
- [ ] Bench spec for candidate 1 (`rust_perf_bench`): matvec tiled vs row-major, N swept across L1/L2 boundaries (working sets 0.5×..32× L1-d; M3 P-core L1-d = 128 KB — parameterize, do NOT hardcode the source's 32 KB), identical arithmetic both arms, alloc-count axis exact, **rel-error gate not bit-identity** (tile accumulation order ≠ row-major order — the B64 gate shape), verdict-stable ×2.

## Phase 3 — candidate 2 verdict measurement (optional arm)

- [ ] PF A/B bench on the M3: sequential scan of a buffer ≫ LLC, with vs without aarch64 PRFM (`core::arch` asm) prefetch — IF the asm is clean; otherwise land the rule source-cited-only with the x86 numbers + the "no ARM re-gate yet" annotation (the `powi-vs-powf` cross-arch re-gate precedent). Defer-mark `[-]` with reason if asm friction dominates.

## Phase 4 — GOAT gates

- [ ] G1: rel-error equivalence (candidate 1) / deterministic verdict table (candidate 2).
- [ ] G2: Pareto verdict from the bench — candidate 1 expected ParetoDominant at working sets ≥ L1 (source: 1.12–1.36× wall-clock at 1–4× L1; PMU 7.29× fewer L1-d misses); if M3 lands Mixed (L1 4× larger changes the crossover), annotate honestly — never promote on a wrong-regime win.
- [ ] G3: default + bench-state lib tests + retrieval floors HELD (blended/clippy/perf @1 no re-pins) + clippy 0 in batch files + CI guard layers.
- [ ] G4: alloc-count axis exact + G-det stable ×2.

## Phase 5 — count pins + docs

- [ ] The standing count-pin walk: drafter token check, serde round-trip, annotated/unannotated counts, benchable header, `fifty/sixty_*` named tests, entries header comment, current-NNN snapshot comment, router annotation count if 4-domain state changed (expect unchanged).
- [ ] AGENTS.md rust_perf row (+1 or +2, 89→90/91) + `.docs/11_domains/rust_perf.md` table rows.
- [ ] Research 098 status → RECORD (batch landed) + queue-snapshot entry.
- [ ] Commit + push (feat: prefix; my files only — sibling Batch 84 WIP disjoint, commit-sweep guard before push).
