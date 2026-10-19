# Research 291 — Batch 256: the TensorFold `9ccf4727..d1018b31` +988-commit staged campaign

**Status:** RECORD — converted as Batch 256 (Plan 260), 2026-10-19, shikuwa/4090 idle loop.

## Source

`ashhart/tensorfold` @ `d1018b318fb51981c4a18f12e40269a4d3ee9713` (= origin/main tip at clone time; the pool did NOT move past the B255 staging pin). Window `9ccf4727..d1018b31` = 988 commits, +431,482/−18,771 across 3,566 files. **The defining event of the window: TensorFold rewrote its serving engine in Zig** — `zig/kernels/{cuda,metal}/` (GPU kernels ported to Zig-embedded sources), `zig/src/core` (host core), `zig/src/families/` (deepseek_v41, glm, nemotron, qwen3_5, flashnext, drama3), `zig/src/server`, plus an Electron app (out of corpus scope). Apache-2.0 throughout.

The B249 slices had converted the pool to `9ccf4727` (the python-era engine, corpus 848→878). The named leads from the B255 staging record — `018e73c6` (digest-cache) + `3c0d6d3c` (GLM prefix-move-before-evict) — were re-verified AT THE PIN FIRST (quote-level), per the standing discipline.

## Method

3 parallel read agents over disjoint slices (kernels+src/cuda 252 commits / host core+server+fabric+native 433 / families 425), full-message reads on ~150 perf-relevant subjects, full-diff reads on ~42, dedup greps over BOTH corpora at 3+ vocabulary variants per candidate. **Convergence datum: the window's strongest kernel lead (the softmax-state once-per-block exchange, c05ce72c) was independently discovered by agents A and C** — a product-coverage signal for the slice split. 5 A− candidates + ~10 B+ identified; 7 rules converted (4 kernel + 3 rust_perf), the B+ remainder staged for B257 (list at the tail).

## Quote verification

All brief quotes re-verified by the authoring agents at the pin via `git show` — **zero drift** (a handful of precision upgrades: commit-message vs in-code attribution splits, the `pool_move.zig` file discovery, PH=128 phasing within the block). Two honest corrections AT AUTHORING:
1. **digest-cache corrupt-index semantics** — the staged brief said "corrupt index ignored, degrades to recompute"; the source's corruption fixture is "refuses corruption **without returning or recomputing** a trusted value" → `error.CorruptDigestCache`, reads stay at 1. Only ABSENT (→ empty) or foreign-key (→ miss) indexes recompute. The rule carries the three-way semantics.
2. **pin-lifecycle HEAD drift** — at pin HEAD the mechanism ships renamed (`pins`→`borrowers`, `dead`→`expired`, `RestoresPending`→`CacheBorrowed`, collector in `prompt_cache_lifetime.zig`); the quoted commits are the protocol's landing, disclosed in-rule.

## Converted (7 rules)

kernel_opt (entries_gpu_110.rs): `attention-softmax-state-exchange-once-per-block` (F, c05ce72c+80d1aba4) · `expert-prompt-row-batch-amortizes-weight-pool-stream` (A, 7bcab59f — the first owner of the weight-STREAM amortization axis the corpus explicitly dropped as not-a-rule at the Yukon conversion) · `glm-selected-prefix-move-before-evict` (D, 3c0d6d3c+df4289c7 — the NAMED lead, fixes #966) · `mtp-ring-rebase-covers-rollback-window` (G, 89aa09a7).

rust_perf (entries_36.rs): `persistent-digest-memo-stat-identity-keyed` (caching, `018e73c6`+`20bca340` — the NAMED lead) · `launch-cost-scales-with-allocation-count-pack-one-arena` (allocation, `84c58d3f` — 1.2 ms/launch over 1,900 buffers, load 25 s vs 200 s, RTX 4090 source-reported) · `pinned-entry-tombstone-freed-at-last-unpin` (concurrency, `8c6b40a3`+`3aedf373`).

## Gates (all green)

batch256 gates 3/3 (kernel needles + rust_perf parse + rust_perf needles; one needle fixed — "never prompt text or tokens") · default lib 2700/0 · kernel_opt lib 3171/0 (1 ignored = the documented freeze-regen e2e) · rust_perf_bench lib 2731/0 (6 ignored) · retrieval floors HELD no re-pin (blended 90.6/94.2/95.3, kernel self-variants 100%, 21/21 tests, 292.5 s) · clippy −D ×4 postures (7 doc_markdown mechanicals healed via cargo refine, compile-gated; the CRLF edit-lane hazard instance #9 caught pre-commit — entries_gpu_110.rs written CRLF by the agent lane, normalized) · count pins 906→910 ×17 + the category table ×4 + router experts + the annotated-split pins 179→182 ×3 (surfaced by the full-lib run — the B255 pin-hiding class repeating) · score-bench #191 (heal 0.9636 FLAT, parse_safety 1.0; frontier snapshot gen 7 appended) · freeze NOT owed (zero clippy-corpus delta — the M4/M5 generations key on the clippy seed corpus). Fixture proof: throwaway crates pre-landing 25/25 + 18/18 green (the corpus never compiles fixtures; the B255 practice).

## STAGED-NOT-CONVERTED — the B257 pool (from the read agents' reports, quotes re-verify at the pin)

1. `cluster-dsmem-shared-head-one-row-decode-attention` (01b7f3bb, drama_tower.cu — one decode head spread across a CTA cluster, scores gathered over dsmem, one-CTA-order replay; zero owners; NotBenchable).
2. `pdl-admission-by-image-set-marker` (bef432bb — PDL only when both embedded images hold pdl_marker; the deployment-level admission vs grid-dependency-wait's in-kernel scoping; zero owners).
3. `first-message-sized-to-smallest-job` (10ff8779 — TP first message sized to the modal smallest job, 165,440 → 10,560 B/step; measured).
4. `copy-source-quarantine-unproven-free-pinned` (2ba81a06 — freeing async GPU memory requires WAIT PROOF; unprovable drops enter a charged quarantine; completes the custody family).
5. `fill-selector-starvation-aging-bound` (48277078 — RIDER on B249's fill-foreground-then-fewest: after 8 skipped choices oldest-wait wins, bounded 8+N).
6. `draft-cost-timed-at-served-widths-not-extrapolated` (abb860c5 cluster — "the rule's extension of the window table priced them at twice the truth": measure the served widths, interpolate, never extrapolate; + the env-fingerprint cost-cache persistence).
7. `checkpoint-reads-skip-page-cache-with-refusal-fallback` (221c2b5d Windows NO_BUFFERING + 6bb9daac Linux EINVAL fallback — O_DIRECT with a buffered fallback that clears the flag on the fd).
8. `reserve-stream-caches-at-admission-warm-request-paths-at-load` (84c58d3f sibling — the server knows its stream count at admission; `reserve` allocates and warms each stream's caches).
9. `cache-budget-first-sessions-take-remainder-never-reshrink` (406b2090).
10. `prefill-batch-atomic-fallback-individual` (c4995966) · `teardown-takes-list-whole-not-copying-dupe` (30f05c6d) · `drain-device-work-before-free` (98571a84) · `mtp-ring-rebase` (landed) · row-keyed-chunk-width rider face (79bdb93d/02d3e2ed) · `verify-row-tiles-share-loaded-weight-panel` (4d1a3f15).

## Watch / lane intel

- The Zig rewrite is a NEW LANGUAGE surface for the workspace's mine sources — the corpus's first Zig-sourced rules landed this batch (7 of 7). Prior TensorFold batches mined the Python engine; the same LAWS now have Zig spellings (several re-derivations adjudicated instances: Nagle → owned tcp-nodelay; residency-set → owned metal-keepalive rider; conv-in-place → owned fused-conv family; cuBLASLt qualification → owned canary-admission).
- RTX 4090 provenance IN-SOURCE (the arena rule: "On an RTX 4090: the 27B decodes at 37 tok/s in the runner…, 210 tok/s across eight streams") — this box's class, source-reported.
- `a739444c` pinned-host prompt states + the 8-GiB host-cache sizing rule (families) — read at B257 staging if the pool reopens.
- Wholesale-revert-to-next-minor discipline (f1531196/cc626365/2561150d) — a release-process watch row, not a rule.

## Provenance

- Clone: `.raw/tensorfold-b256`, detached at pin `d1018b31`, REMOVED at close (the ephemeral-clone law).
- Box state: Windows 11 / i7-13700K / RTX 4090, CPU-only batch (StartLux stack resident+idle), CARGO_TARGET_DIR=/tmp/b256-target (removed at close).
- En-route hazards: the CRLF edit-lane class ×2 more instances (#8 the snapshot rewrite at claim time, caught post-commit + fixed-forward; #9 entries_gpu_110.rs caught pre-commit) — standing reminder: normalize LF before every commit on this box.
