# Plan 411 — Batch 412: the mlxfast staged pool (Research 437's 8 candidates)

**Status:** IN FLIGHT — claimed 2026-11-24T04:44Z by zed-amo-i412@amo (epoch 1795495491), expires 2026-11-26T04:44Z.
**Claimed:** zed-amo-i412@amo, 1795495491, expires 2026-11-26T04:44Z

## Source

Research 437's own staging (the 8 candidates queued behind Batch 411's landing, per
`.distill/001_mining_queue_snapshot.md` line "Research 437's 8 mlxfast staged candidates stay
queued behind"): the mlxfast-bonsai2-27b-engine surfaces left unmined after Batch 402's
un-owned-residue pass. Source: Layr-Labs/mlxfast-bonsai2-27b-engine, MIT, pin `0c7c3923`
(Research 437 verified tip UNMOVED vs the Batch 401 pin; T0 re-clones + re-verifies — the clone
was torn down at Batch 402 close).

The 8 name-staged candidates:
1. `speculative-substitution-prefix-dominance`
2. `route-selection-frozen-at-construction`
3. `speculative-prefab-adopt-or-drop`
4. `unit-arity-sibling-verb`
5. `tested-ceiling-explicit-per-decoder`
6. `device-readiness-spin-deadline-bounded`
7. `per-step-shared-scalars-host-advanced-one-array`
8. `calibration-band-two-sided-bounded-zero-cv-refused`

The names are stage labels, not landed rules — T0 discovers the source surfaces each names,
re-verifies every quote in-source (the names may have drifted vs what the files actually carry;
a candidate whose mechanism is already owned by a corpus rule dies at T1 dedup — the
Batch-409 prior-coverage lesson: a staged lead's claim expires against the corpus, re-grep at
claim time).

## Tasks

- [ ] Claim pushed BEFORE authoring (this plan + the snapshot claim row, one commit).
- [ ] T0: clone Layr-Labs/mlxfast-bonsai2-27b-engine at `0c7c3923` into `.raw/mlxfast-27b`;
      MIT license verified at the pin; locate each of the 8 candidates' source surfaces
      (file + line windows) via the names' mechanism vocabulary.
- [ ] T1: dedup grep per candidate against `src/domains/kernel_opt/` + `src/domains/rust_perf/`
      + `sec` (3+ vocabulary variants; 3+-vocabulary greps + in-row reads per the Batch 402
      lesson). Kill owned mechanisms, keep owners cited for in-rule adjudication.
- [ ] T2: 2 parallel reader subagents over the surviving candidates (pattern, bounded
      fix-space, category, rationale, source quote); coordinator re-verifies EVERY quoted line
      against the real file (Batch 31 discipline).
- [ ] T3: author corpus entries (expected home: rust_perf `entries_99` for the
      measurement-honesty/concurrency class; kernel `entries_gpu_168` if any kernel-surface
      candidate survives; sec per candidate as dedup admits). Suggestion-only/NotBenchable
      annotations carried where the win metric has no validator host (the dialed/B136 class).
- [ ] T4: fixtures — standalone-compiled AND executed BEFORE entry (debug AND -O), four-arm
      discipline per rule; expected name `b412_fixtures` (+ domain-specific files as authored).
- [ ] T5: gates — default lib; kernel+latent lib; retrieval floors BOTH lanes (30/0, no re-pin
      expected — corpus delta is not clippy_lints; if rust_perf delta moves nothing in the
      router floors, no re-pin); clippy -D ×3 postures clean; fmt (bare rustfmt, touched
      files); numbering gate + dual-allocation; pins_manifest --write; count web one-landing
      (AGENTS head + README + .docs/11_domains heads + health_views + router witness +
      KERNEL_CATEGORIES + category table + census if kernel delta).
- [ ] T6: Research note (`.research/445_*`), landing row in the snapshot, commit `feat: batch
      412: ...` + push; teardown (.raw/mlxfast-27b, /tmp target dirs, fixture binaries).

## Yield expectation

~4-5 rules + faces after ~50% falsification (the staged-pool convention; 8 candidates − T1
dedup kills − authoring drops). All suggestion-only unless a benchable GOAT lead with a
validator host survives.
