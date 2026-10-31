# Plan 294 — Batch 290: the resin serving-engine conversion (rust_perf)

**Status:** STAGED — authoring gated behind the live mining-lane claims (plan-289 bbtree lane holds the slot; B284 burn authoring pending); verdict landed as Research 328.
**Lease closed:** zed-m3-294t0, epoch 1793003333, was expires 2026-10-28T08:28:53+00:00 (staleness fold — the plan stays STAGED behind the live mining-lane claims; verdict stands as Research 328; re-claim at the slot)

Source: ekzhang/resin @ `c14ee9d0e84a79cf0168fa117745787de968f63f` (Apache-2.0, pinned 2026-10-26). Verdict: **YES B corpus / lane-intel HIGH / product-coverage LOW-MODERATE** — see `.research/328_resin_serving_engine_distill.md` (the ranked table + dedup evidence IS the working inventory).

## Tasks

- [x] T0 — re-verify at the pin: re-clone `.raw/resin/` (shallow) if removed, `rev-parse` == the pinned sha, licence re-read at the root; re-read the verdict's UNVERIFIED files (`paged_kv.rs`, `paged.rs` 300–820 placement internals, `engine.rs` unread bodies) and re-verify EVERY staged quote byte-for-byte (the B275 verify_quotes instrument). **DISCHARGED 2026-10-26 (zed-m3-294t0): quotes 8 PASS / 0 DRIFT / 0 MISSING; UNVERIFIED surfaces read via sweep — 0 contradictions, 3 enrichments, ~10 NEW candidates (A/B strong, C–H medium, I–P thin) recorded in the Research-328 §T0 addendum; yield revised 3–5 → 6–9.**
- [ ] T1 — dedup re-verification at authoring-time corpus generation (3+-vocabulary greps over rust_perf + kernel_opt; the Research-328 §Dedup lines are the starting evidence, not the verdict).
- [ ] T2 — authoring slices over disjoint entry files (≤3 rules per slice, the B251 slice discipline), each slice's fixtures standalone-compiled + executed BEFORE landing:
  - [ ] Slice A (dispatch): `overlap-continuation-lease-reuse-under-lookahead-window` (B+) + `pre-accept-rejection-restores-post-accept-failure-quarantines` (B−; adjudicate rider-vs-own against the sonar B272 fail-stop leads' landing state at authoring).
  - [ ] Slice B (caching/allocation): `incremental-prefix-publication-extends-from-weak-hint` (B) + `prepare-fallible-metadata-then-commit-without-allocation` (B−).
  - [ ] Slice C (dispatch/complexity + rider): `epoch-guarded-selection-identity` (B−, NotBenchable) + `atomic-span-aware-chunk-placement` (B−) + `merge-returning-batch-sorted-into-aged-queue` (B−, the GOAT lead — benchable on the Pareto validator) + the B200 rider face (block-granular strict extension + checkpoint-aligned chunk ends, count-neutral). **+ T0-found candidates (Research-328 §T0 addendum A–P) join the T1 re-adjudication — slices re-cut there, disjoint entry files per slice.**
- [ ] T3 — NotBenchable annotations landed per grade (#1/#4/#5/#6 + the annotation reason in context_notes; the #7 GOAT bench on rust_perf_bench at `--release` with box-state provenance per the G2 law).
- [ ] T4 — GOAT (G1 exactness on #7: merge order bit-identical vs a full-sort reference on exhaustive small queues + LCG batches; G2 Pareto + alloc axis; G3 no-regression; G4 alloc-free) — only #7 is claimed benchable; an honest drop if the validator Pareto-dominates it (the B243 transform-borrow precedent).
- [ ] T5 — batch needle + parse gates (needles keyed to landed text — the Batch-259 lesson).
- [ ] T6 — count pins via ONE shape-asserted python pass (rust_perf total, with_spec, annotated, category counts, mod.rs header + AGENTS.md + README + `.docs/11_domains/rust_perf.md`) — at authoring-time counts, never staged numbers.
- [ ] T7 — standing gates: default lib, rust_perf_bench lib, clippy `-D` ×3 postures, retrieval floors RE-MEASURE (corpus delta).
- [ ] T8 — freeze/score-bench OWED (rust_perf delta → plan-137 T7.5 gen2+gen3 regeneration + score-bench run) — unlike the kernel_opt-only batches.
- [ ] T9 — release-set binary REBUILT + installed BEFORE push (rust_perf is release-set surface; the §0 bin-age law — verify the exe MTIME + `--version` stamp, the locked-exe class).
- [ ] T10 — research-note addendum (landing record + drift disclosures) + queue-snapshot landing row + AGENTS.md/README/.docs count updates in the same commit; `rm -rf .raw/resin` at close (the ephemeral law).
- [ ] T11 — commit + push on `develop` (`feat:` prefix, named paths only — never `-A`; the worktree carries sibling WIP).

## Honest scope note

4–6 staged candidates → **3–5 rules** expected after coordinator falsification. If the bbtree (plan-289) or B284 lanes' landings consume the same serving families, re-adjudicate each candidate as rider-vs-drop at T1 (the collision-merge precedent: both batches may land; disjoint entry files by slice).
