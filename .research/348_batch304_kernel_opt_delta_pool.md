# Research 348 — Batch 304: the kernel_opt delta pool

**Status:** RECORD — Batch 304 landed from this note (Plan 309); sources re-cloned at pins, removed at close.

## Provenance

Five sources, all permissive, re-verified at clone (plan 309 T0):

| repo | licence | commits mined |
|---|---|---|
| ggml-org/llama.cpp (master) | MIT | `9b8446b` #30072, `4d71b42` #31179, `c3df4b7` #31201, `cf24317b` #31336, `b996a43f`→mlx |
| ml-explore/mlx (main) | MIT | `b996a43f` #4717 (+ kin `9cb9c06f` #4437 adjudicated) |
| vllm-project/vllm (main) | Apache-2.0 | `b42f25b` #63382, `4da2a82` #63791, `d48202060` #63827 |
| ashhart/tensorfold (main) | Apache-2.0 | `2922b93`+`2e04524` (+ instrument `c6193ca`/`dd0a9cc`), `8abd450` #1478 |
| lithos-ai/lithos-metal (main) | Apache-2.0 | `8ed1611`, `cd220ab`, `2d51c4a` |

Clones at `.raw/b304/` (blobless), REMOVED at close per the ephemeral-clone law.

## The landed set — 13 own rules (corpus 1007 → 1020)

| # | rule_id | cat | source | one-line law |
|---|---|---|---|---|
| 1 | concat-row-space-thread-split-not-singleton-outer-dim | E | llamacpp `4d71b42` | split a host copy's work over the FLAT row space, never a singleton-capable outer dim (i2==1 → 1/nth utilization) |
| 2 | strided-source-row-tile-element-outer-walk | A | llamacpp `4d71b42` | transposed-source copies walk 32-row tiles with the ELEMENT index outer so tile rows read neighboring addresses |
| 3 | codebook-nearest-midpoint-rank-branch-free | B | llamacpp `c3df4b7` | nearest-codebook over a fixed monotone table = branch-free midpoint rank, tie-rule-equivalent per float |
| 4 | parsed-artifact-index-range-check-wide-offsets-load-boundary | D | llamacpp `cf24317b` a | an index parsed from a foreign artifact is untrusted input: range-check at the load boundary + address-width offsets |
| 5 | merge-tile-whole-iteration-bounded-write-domain | A | llamacpp `cf24317b` b | a hardcoded inner unroll inside an outer stride overruns on non-multiple grids; iterate whole merge tiles |
| 6 | grid-ceiling-sliced-dispatch-per-slice-buffer-rebind | E | mlx `b996a43f` | a grid past the platform total-thread ceiling is SLICED with per-slice buffer rebinds (Metal silently truncates the tail) |
| 7 | fused-pair-replacement-writes-every-legacy-output-every-path | C | vllm `b42f25b` | a fused kernel replacing a producer pair inherits EVERY output the pair wrote, on every admitted path (NaN-logits fix) |
| 8 | silent-zero-launch-row-chunked-past-grid-z-ceiling | E | vllm `4da2a82` | a launch past gridDim.z's 65535 fails WITHOUT raising; chunk rows at the ceiling + the boundary sweep |
| 9 | page-reuse-fenced-behind-reader-completion | A | vllm `d48202060` | reuse-eligibility and read-eligibility are different clocks; fence new writers behind the readers' completion step |
| 10 | device-primitive-dedup-under-sass-identity | M | tensorfold `2922b93`+`2e04524` | source dedup lands ONLY under per-function compiled-output identity, carried by an instrument with planted controls |
| 11 | dual-bank-scratch-retention-per-round-class | A | tensorfold `8abd450` | two disjoint round-class row regimes → a per-class scratch bank swapped on class flip, allocation-free |
| 12 | bit-exact-constrained-geometry-search | K | lithos `8ed1611` | a measured geometry table's admission predicate is DETERMINISM — only bit-exact geometries admitted |
| 13 | statistic-producer-row-split-single-threadgroup | E | lithos `cd220ab` | a row split on a statistic-writing op is confined to ONE threadgroup so the statistic keeps its partials byte-for-byte |

Riders (count-neutral):
- **widen-index-multiply-to-64bit (gpu_02)** += the SILENT-NARROW-INDEX + SWEEP-EVERY-STORE-PATH faces (llamacpp `9b8446b` #30072) — the widening law itself was owned (Batch 24 + the Batch-225 vendor-view rider); the sweep completed the file's four int-narrowed store paths + two 16-bit index types.
- **bit-exact-constrained-geometry-search** carries the posture-scope rider face in-rule (lithos `2d51c4a`: a tuned geometry applies only in the posture it was timed in — variant-beside-default).
- Cross-kin added both ways: rules 6 and 8 are the SAME law's two platform halves (Metal truncates the grid; CUDA accept-and-no-op) with opposite failure modes — each names the other.

## Honest drops with provenance (the cross-pass dedup)

- **#30553 Stream-K cap / #30573 WAR hazard** — ALREADY MINED (Batch 253, `stream-k-grid-capped-at-useful-work-quanta` + `loop-back-edge-scratch-war-trailing-warp-barrier`, entries_gpu_108); verified present at T0c.
- **mlx #4712 qmv_mma few-row** — ALREADY MINED (Batch 273, `fewrow-mma-fragment-matvec-silicon-tiered-dispatch` + the #4682 riders, entries_gpu_119).
- **#30804 graph chunking** — rider-faced (Batch 274 gpu_08 SIZE-PLACED-CUT + Batch 280's axis adjudication).
- **#30795 moe_reduce dst-alias race** — the FOUNDING SOURCE of `fused-read-write-dst-input-overlap-refusal` (gpu_109, Batch 254) + the Batch-299 rider; verified in-rule quoting this exact commit. Nothing to add.
- **#29882 rms_norm subgroup reductions** — the 10-27 audit verdict (NO-GO as a standalone rung: decode is launch-bound; the bit-identity law pins the 256-slot tree; the norm-fold lane closed 7/7 negative) STANDS; no re-open.
- **#31255 pragma-unroll removal** — dropped by Batch 299 (no measurement, no predicate).
- **cubecl #1914/#1915/#1907 measurement pair + vllm Zipf #64167 / warmup #63819** — route to the rust_perf `measurement` category (B284's), NOT kernel_opt; rust_perf is behind the bbtree B305 head + B300 foreign claim → formally out of this batch's scope, staged where they sit.

## Drift disclosures (staged-row claims the diffs contradicted)

1. **#4717's remedy is NOT a grid-stride loop** — it is host-side dispatch slicing with per-slice buffer byte-offset rebind; grid-stride cited only as the kernel-cooperative alternative (the SYCL shrink spelling). Rule authored as slicing+rebind.
2. **#30072's sweep scope** — the tensor path was ALREADY wide from #28748 (the Batch-225 rider); the sweep covered the four non-tensor store paths + two index types, not "every store path in the file" as staged.
3. **#31179's "22.4→0.14 ms" figure is NOT in the commit** (the staging carried it from the p11 addendum); no number cited in-rule.
4. **Slice B's initial rule-10 quote joined two non-adjacent commit-message fragments** — caught by the slice's own quote verifier, re-pointed to the true adjacent fragments.

## Gates

- G1: batch304 needles + parses 2/2 FIRST-RUN (after the fixture quote-class repair below)
- G2: kernel_opt lib 3282/0; default lib 2760/0
- G3: clippy −D ×3 postures clean (26 needless_raw_string_hashes healed post-assembly — the documented manual class; 27 doc mechanicals healed by the cargo-refine healer, compile-gated)
- T7: retrieval floors HELD both postures (28/0 at kernel_opt+latent 206.6 s; 27/0 at default 50.5 s) — no re-pin
- freeze/score-bench NOT owed (kernel_opt-only, P30); binary rebuild NOT owed (kernel_opt outside the release set)

## En-route

- **The B298 missed cross-domain pin repaired**: `batch249_slice3_rust_perf_riders` still read the rust_perf corpus at 274 after Batch 298's bbtree partial landing moved it to 275 — the batch-266/271 missed-pin class its own message documents. Repaired with honest attribution (count-neutral for kernel_opt).
- **The embedded-quote defect class fired once more** (the B299 twice-fired lesson): the gpu_02 rider text carried raw `"` around the PR title and the 32,768-token quote — compile caught it, escaped same-session.
- Count pins re-pinned: 2 primary asserts + 35 era-witness sites + the router expert total + 7 per-category sites (+4 A, +1 B, +1 C, +1 D, +4 E, +1 K, +1 M) + the sum comment.
- Fixtures: 26 ref/opt pairs, all original Rust reproductions, zero string literals, compile+run-verified by the authoring slices (landed bytes re-extracted and test-run green).

## Queue after

Batch 304 pool EMPTY for kernel_opt. The rust_perf residue (TF borrowed-pool `f3eb4834`, bevy #26336, TF `b5accc3`, vllm #62748, uzu #1027, the splash ANE/host five staged in Research 345) serializes behind the bbtree B305 head (10-29T09:44Z) and B300 (perfect-hash, foreign to 10-30T09:34Z); B301 leviathan Phase 3 (plan 306, foreign to 10-30T13:41Z). The 1.99 pin bump LANDED in this repo 2026-10-29 (AHEAD_OF_PIN_1_99 empty per the bump contract) — the post-pin-bump rust_perf slot is OPEN once the heads clear.
