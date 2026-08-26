# Research 084 — Batch 72: the frguthmann/NVIDIA compute-shader external distill (1 rule + 1 rider)

**Status:** RECORD (batch landed same day)
**Plan:** `.plans/026_dirty_encoding_write_elision_batch72.md`
**Sources (both fetched + read in full):**
- frguthmann.github.io "Exploring ways to optimize compute shaders — Part 1" (F. Guthmann, 2020-09-26) — GLSL/Vulkan Game-of-Life, Nsight GPU-trace P3-driven, honest failed-experiments list.
- developer.nvidia.com "Optimizing Compute Shaders for L2 Locality using Thread-Group ID Swizzling" (L. Bavoil, NVIDIA, 2020-07-16) — Battlefield V DXR denoiser measurement, GDC 2019 talk distill.

## Verdict

1 NEW rule + 1 spec-enhancement rider; corpus **372 → 373** (A 88 → 89).

## The rule — `skip-unchanged-writes-via-dirty-encoding` (A)

Per-element write elision for ping-pong state buffers. The naive pass
unconditionally copies unchanged elements (`dst[i] = src[i], "so the data is
there next step"`) — on the source's 9-read-1-write stencil that copy alone
was **20% of all L2 traffic**. The fix encodes the dirty bit IN the element
value (0/1 plain dead/alive, 2/3 just-updated): liveness stays readable
because dirty = plain + 2 preserves `state % 2`; write back only elements
that transitioned (store dirty) or still carry the bit (store value − 2).

**The lazy-cleanup accounting (derived, sharper than the source's own
comment):** the source says "if its value is greater than 1 … it was updated
in the last rotation" — imprecise (the bit may be older). The write-back is
idempotent (always lands the correct plain value), which is exactly why the
lazy schedule is sound. Cost model per element class:

| element class | writes vs naive |
|---|---|
| never-changed | **0** (vs 1/step) |
| changed-once-then-stable | alternate rotations (~1/2) |
| changed-every-step | same count **+** encoding overhead (strictly worse) |

**REGIME-GATED both directions (measured, Nsight, RTX 2070, 20k×20k):**
sparse dynamics (single glider) **~20% faster** (11.5ms → ~9.5ms; L2 write
share collapses — 99.7% of L2 traffic becomes L1TEX reads; SM becomes the
75% top-throughput unit). Dense dynamics (random fill) **measured WORSE**
(13.5ms vs 11.5ms). Apply only on structurally small changed fractions
(event-driven sims, sparse updates, converge-and-idle iterations). The
regime gate is encoded in-concept (kernel_opt rules carry no
`context_notes`; the B49 occupancy precedent).

## The rider — `l2-grouped-tile-schedule` (popcorn corpus) spec enhancement

The NVIDIA thread-group-tiling candidate was **KILLED at dedup**: the
existing rule's fixture IS the grouped-tiling swizzle shape (momoe_tl.py
grouped_tile_map ≡ Bavoil's rectangular tiling). Per the B66 precedent
(float-sum killed as a simd-chunk positive instance, salvaged as a spec
rider), the decision content the existing thin concept lacked now lives in
the entry's doc comment:

- **Applicability conditions**: (1) VRAM is the top-throughput limiter,
  (2) L2 load hit well below 80%, (3) adjacent groups' footprints
  significantly overlap — wide kernels (denoisers, ray queries); NOT narrow
  stencils (row-major already reuses neighbor data; the swizzle measured
  irrelevant there — frguthmann's own case).
- **N=16** tile width (measured sweet spot, BFV DXR denoiser 1440p, RTX
  2080): **47% faster, L2 read hit 63% → 86%**, workload moved
  VRAM-latency-limited → SM-limited.
- **Morton order loses** to simple rectangular tiling in practice.
- Re-profile after applying — once SM-bound, further locality work is
  misdirected.

## Dedup record (Protocol A step 2)

- `dirty` / `write-elision` / `skip-unchanged` / `conditional store` /
  `unchanged write` — zero corpus hits (nearest: `predicate-gated-skip-
  empty-workset-step` B30 = whole-STEP host-side elision; the ternary
  sparse-word skips = COMPUTE skip on zero input words;
  `write-before-read-state-skips-checkpoint` B58 = CHECKPOINT elision — all
  distinct, distinctness encoded in-rule).
- `swizzle` / `morton` / `l2 locality` / `launch order` → **hit:
  `l2-grouped-tile-schedule`** (entries_popcorn.rs) — the KILL above.

## Pool drops (with reasons)

- wgpu discussion #6688 (rcoreilly/axon experience report) — read in full;
  substance is version-regression tracking (wgpu 23 kludge, #6662/#6521,
  v25-vs-v23 platform flips ±20%), not transferable code patterns. The one
  regime observation (small models = dispatch-overhead-bound, large =
  codegen-bound) is covered by the dispatch-batching/residency families.
  No decision content — rejected per the distill discipline.
- frguthmann Part 2 — does not exist (404; series unfinished). The
  register-island teaser is unminable from a teaser.
- 1D-dispatch-beats-2D measurement — thin (author: "quite hard to know why
  for sure"); mechanism is the same launch-order locality the rider now
  documents; folded as context, not a rule.
- Threadgroup ≥ 64 (warp/wavefront granularity) — textbook occupancy
  hygiene; the corpus's occupancy content (B28 workgroup sizing, B58
  occupancy-cap-refuted) already carries the sizing decision discipline.
- GPUOpen "large thread groups" article — not fetched; the pool was already
  at the established external-batch yield (1-3) with stronger candidates.

## GOAT

- **G1** syn-parse: `batch72_pseudocode_parses` ✓ (with the B70 lesson's
  `#[cfg(not(feature = "kernel_verify"))]` guard — re-hit once during the
  run, fixed in-WIP).
- **G2** lib tests: default **677/0**, kernel_heal **890/0/1i**, all-features
  **1221/0/13i**.
- **G3** clippy 0 both feature states (default + kernel_heal +
  all-features `-D warnings` via the guard); **ci_feature_guard.sh ALL
  LAYERS PASS**; retrieval floors **HELD zero decay** (13/13; blend@1 94.7,
  clippy@1 89.6, perf@1 96.2 — the B53/B55/B56/B57/B58/B71 pins).

## Unblock chain (3 pre-existing latent breaks fixed en route, separate commits)

The guard had not run Layer 5a/5b since the KAT lane landed; the Batch 72
GOAT chain surfaced + fixed (each attributed, none from this batch):

1. `a8da8b5` — `heal.rs:1422` point-free `and_then(owning_package_root)`
   (2512ef4) dropped the `&PathBuf → &Path` deref coercion → E0631; the
   bin compiles ONLY under `--all-features` (required-features), so
   lib-only validation missed it.
2. `bc8501e` — `examples/plan342_t37_test_labeled_corpus.rs` (35d4735) had
   no `[[example]] required-features` row → E0432 under `--all-targets
   --no-default-features`.
3. `a69aee6` — `heal.rs:1686` `map_or(false, …)` → `is_ok_and` (clippy's
   own suggestion; also 2512ef4) — fails Layer 5a's `-D warnings`.

## Notes

- **`.research/` 083 dual-allocation collision pre-exists** (083_elo_
  selection_gate.md + 083_omlx_external_distill_batch71.md, both referenced
  from AGENTS.md). This batch uses 084. Renumbering either 083 breaks live
  references — owner call, flagged not fixed.
- Generator lessons re-learned: (a) the B37 line-initial `+ ` markdown-list
  strike (9th recurrence) in the mod.rs ledger — reworded to "plus"/commas;
  (b) the B51 concepts-needle case-sensitivity (4th recurrence) — needles
  must match in-concept casing exactly; (c) NEW: `rfind("}")`-based EOF
  insertion on `tests.rs` is UNSAFE — the file has NO wrapper mod braces
  (it is a path-included module body), so the last `}` in the file is the
  last FN's close and the insertion nested the gates INSIDE that fn
  ("cannot test inner items" + "never used" — caught because the lib-test
  count did not include them). Always insert relative to a named anchor
  (`fn <last_test>`), never a bare brace search.
