# Plan 261 — Batch 257: the TensorFold staged remainder conversion

**Status:** COMPLETE 2026-10-20 — Batch 257 landed (shikuwa/4090 idle loop; Research 293 is the record; 292 was taken mid-flight by the sibling security-audit-skill verdict).

## Source

`ashhart/tensorfold` @ `d1018b318fb51981c4a18f12e40269a4d3ee9713` (origin/main tip — re-verified this session, ZERO drift past the B256 staging pin). Pool = Research 291 §"STAGED-NOT-CONVERTED" (the B256 read agents' B+ remainder, ~13 candidates) + the `a739444c` watch lead (pinned-host prompt states + 8-GiB host-cache sizing). Zig-era engine; Apache-2.0.

## Working inventory (homes adjudicated at authoring)

kernel-class (→ kernel_opt unless dedup says otherwise):
- [x] R1 `cluster-dsmem-shared-head-one-row-decode-attention` (01b7f3bb, drama_tower.cu)
- [x] R2 `pdl-admission-by-image-set-marker` (bef432bb)
- [x] R3 `copy-source-quarantine-unproven-free-pinned` (2ba81a06)
- [x] R4 `prefill-batch-atomic-fallback-individual` (c4995966)
- [x] R5 `teardown-takes-list-whole-not-copying-dupe` (30f05c6d)
- [x] R6 `drain-device-work-before-free` (98571a84)
- [x] R7 `verify-row-tiles-share-loaded-weight-panel` (4d1a3f15)
- [x] RD1 row-keyed-chunk-width rider face (79bdb93d/02d3e2ed)
- [x] RD2 fill-selector-starvation-aging-bound rider (48277078 — on B249's fill-foreground-then-fewest)

rust_perf-class (→ rust_perf unless dedup says otherwise):
- [x] R8 `first-message-sized-to-smallest-job` (10ff8779, measured)
- [x] R9 `draft-cost-timed-at-served-widths-not-extrapolated` (abb860c5 cluster)
- [x] R10 `checkpoint-reads-skip-page-cache-with-refusal-fallback` (221c2b5d + 6bb9daac)
- [x] R11 `reserve-stream-caches-at-admission-warm-request-paths-at-load` (84c58d3f sibling)
- [x] R12 `cache-budget-first-sessions-take-remainder-never-reshrink` (406b2090)
- [x] R13 `a739444c` pinned-host prompt states + 8-GiB host-cache sizing (watch lead, read at staging)

Honest drops allowed — thin candidates grade out at the at-pin read (the B243 transform-borrow precedent); every drop carries its reason in Research 292.

## Tasks

- [x] T0 claim row + this plan + highwater bumps committed & pushed BEFORE authoring
- [x] T1 clone TensorFold at pin `.raw/tensorfold-b257`, rev-parse EXACT; every quote re-verified at the pin via `git show` during authoring
- [x] T2 dedup greps at 3+ vocabulary variants per candidate over BOTH corpora (src/domains/kernel_opt/ + src/domains/rust_perf/), zero owners confirmed per surviving rule
- [x] T3 authoring — 2-3 parallel subagents over disjoint slices (kernel slice / rust_perf slice / riders), fixtures proven in throwaway crates pre-landing where the rule carries a code atom
- [x] T4 wiring — entries files (next: entries_gpu_111.rs / entries_37.rs as needed), category counts, router experts pin, mod registration
- [x] T5 count pins re-pinned ×all sites (kernel_opt 910→N, rust_perf 235→M) — tests.rs, mod.rs doc, AGENTS.md, README, .docs/11_domains
- [x] T6 gates — batch257 parse+needles gates; default lib; kernel_opt lib; rust_perf_bench lib; clippy −D ×4 postures (default / no-default / kernel_opt / rust_perf_bench); retrieval floors re-measure; score-bench appended; freeze NOT owed unless clippy-corpus delta
- [x] T7 Research 293 written (.research/293_batch257_tensorfold_remainder.md — 292 was allocated mid-flight by the sibling security-audit-skill distill verdict, which yielded 257/261 to this claim); plan ticked all-[x]
- [x] T8 maintained binary rebuilt at the release feature set + `cp` to `~/.cargo/bin/cargo-refine.exe` BEFORE push (§0 bin-age; verify `--version` build == HEAD + `--corpus-stats` counts)
- [x] T9 commit `feat: batch 257: ...` + push + snapshot landing row (same commit); `.raw/tensorfold-b257` removed at close; CRLF-normalize every touched file pre-commit (instances #8/#9 standing)

## Box state

Windows 11 / i7-13700K / RTX 4090; CPU-only batch (StartLux stack resident to 2026-10-20T13:30Z, idle — no GPU compute spawned); CARGO_TARGET_DIR=/tmp/b257-target for the isolated gate runs if the main target is contended, removed at close.
