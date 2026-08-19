# Mining queue snapshot — the distill-program resume state

> Moved out of `.agents/skills/riir-clippy/SKILL.md` 2026-08-18 (104KB broke Zed's skill loader).
> This file IS the resume state: read at session start, update it in the same commit
> as each batch. Batch DETAIL lives in `.research/NNN_*.md` — this file holds only
> the rolling queue (next targets, sibling-blocked files, corpus count, standing
> lessons). Do NOT let this file grow unbounded: when it exceeds ~400 lines, fold
> closed-batch history into a `.research/` note and keep only the live queue here.

## Mining queue snapshot (update per batch — this is the resume state)

- **Batch 32b-1 DONE (2026-08-15, Research 035):** the edge_lora training
  half (topology_trainer + chain_commit + sigmoid_gate + mod) — 10 rules,
  corpus 228 → 238 (A 39, B 39, C 25, D 35, E 14). Bug yield filed as
  riir-ai Issue 684 (hot-loop allocation storm: per-node `vec!` on a FALSE
  aliasing premise ×2 + per-edge `to_vec()` borrow workaround + backward
  staging buffer + the O(E·L²) routing re-derivation ×3 loops + dead
  scratch clears + triple-BLAKE3 load path + 7-site sigmoid duplication
  + discarded topology validation error + u32→u16 narrowing).
- **Batch 32c DONE (2026-08-15, Research 036)** — landed as "32b-2": the edge_lora
  self-play/arena/consolidation half (sleep_consolidation + arena_eval +
  cross_game_edge/real + {go,bomber,fft}_self_play) — **13 rules, corpus 238 → 251**
  (A 42, B 42, C 27, D 39, E 15). The edge_lora module is now FULLY MINED
  (32a core + 32b-1 training + 32c self-play). Bug yield filed as riir-ai
  Issue 685: the quadratic centroid storm (P1) + swallowed failed move +
  wasted Pass encodes + warm_start weight duplication + per-call dataset
  deep-clone + false-premise sigmoid comment (named sigmoid, computes an
  affine ramp) + dead init-touch loop + Wait-vs-continue mismatch +
  per-tick nested-Vec grid rebuild + the adapter-skeleton triplication
  family (7 shapes × 3 game adapters — the self-play half of the Issue 684
  sigmoid dedup). GOAT: 312 kernel_heal + 201 default tests, clippy 0 both
  states (4 doc_markdown backtick fixes pre-commit).
  - **Landing method (proven 32c — supersedes manual edit_file for big
    corpus edits)**: a one-shot Python generator (script → run → delete)
    with `json.dumps(s, ensure_ascii=False)` for string literals (correct
    `\n` escapes by construction) + comment blocks as real multi-line
    strings + `assert count==1` anchors on every replace. First-compile
    clean vs the 32b-1 manual-escape incident. The awk orphan-`rule_id:`
    pre-compile scan stays as the standing defense.
  - `.research/.highwater` was STALE at 32c start (34 while 035 existed —
    the 32b-1 session forgot the write-back). Fixed. Never trust the
    highwater file alone; re-scan the folder at write time (the standing
    rule, now with its own observed failure).
- **Batch 33 DONE (2026-08-15, Research 037):** the spectralquant
  calibration cluster (calibration.rs + rotation.rs + qjl.rs — the
  calibration path Research 016 explicitly deferred; the spectralquant/
  directory is now FULLY MINED; bench-only today, sole consumer
  spectralquant_bench.rs) — **7 rules, corpus 251 → 258** (A 44, B 44,
  C 27, D 40, E 16, N 14): grid-over-output-thread-private-sweep,
  buffer-set-struct-two-tier-dispatch, slot-offset-partial-buffer-write,
  immutable-operand-upload-at-set-creation,
  selective-projection-effective-rank-head,
  derived-scalar-host-computed-in-uniform,
  rademacher-signs-over-gaussian-projection. **ONE seed REJECTED on
  verification** (the load-bearing call): two-stage tile→reduce is NOT
  what covariance.wgsl does — the producer writes no tile partials, so
  n_tiles > 1 would reduce uninitialized memory (filed as riir-ai Issue
  686 H1 instead of becoming a rule). 11 further candidates dropped as
  duplicates/thin. Bug yield filed as riir-ai Issue 686 (15 hazards:
  n_tiles>1 silent corruption + layer_offset dead in the rotation kernel
  + per-dispatch bind groups + create_uniform_buffer doc-lie + …).
  GOAT: 315 kernel_heal + 201 default tests, clippy 0 in all 4 states
  (8 doc warnings fixed pre-commit; drive-by fix of the stale
  M-category (7→9) comment).
  - **Method note (33): the subagent verification protocol overturned a
    coordinator seed** — subagent 1 read the WGSL kernels on its own
    initiative and produced line evidence that rejected the
    two-stage-tile seed BEFORE it landed. Always let subagents verify
    against the KERNEL side, not just the .rs dispatch wrappers; the
    .rs comments can describe unimplemented behavior (the
    "multi-pass accumulation" docs describe a path the kernel never
    implements).
- **Batch 34 DONE (2026-08-15, Research 038):** the Gemma-4 TRAINING
  cluster (`gemma4_cubecl_train/mod.rs` + `backward_gpu.rs`, 1,316 LOC —
  hybrid GPU training forward + Phase 2.5 GPU backward; the training
  counterpart of Batch 25's inference-side Gemma cluster) — **9 rules,
  corpus 258 → 267** (A 46, B 44, C 28, D 43, E 16, H 11, L 9, N 15):
  exact-op-parity-for-analytic-backward, mirror-ring-store-sliding-window-
  contiguity, split-dispatch-expose-fused-intermediate,
  preallocated-activation-slots-inplace-save,
  hybrid-cpu-elementwise-gpu-throughput-sync-points,
  pre-transposed-consumer-layout-handles-build-once,
  shape-matched-sibling-weight-grads-share-input-loop,
  op-major-backward-loop-inversion-batch-sequence-axis,
  backward-helper-expose-shared-norm-statistic. TWO seeds resolved by
  verification: the softcap "recompute" hypothesis FALSIFIED (the
  backward algebraically recovers tanh from cached POST-cap logits —
  already sigmoid-deriv-recovered-from-output) + the dual-KV-cache
  "pattern" REJECTED as wisdom (avoidable 2-3× duplication → hazard
  H9). Bug yield filed as riir-ai Issue 687 (13 hazards): mirrored-ring
  backward OOB panic for seq_len > 2·sw (index algebra verified —
  correct below the boundary, no assertion), backward-handle staleness
  + the self-contradictory LoRA doc (base-only transposes drop ΔW^T·g
  under the merged-weight convention — found independently by BOTH
  subagents), Phase 2 processing zero-grad positions, ~2.06M allocs /
  27 GB per step + 345,088 blocking GPU round-trips per step
  (quantified at real 12B dims), the CPU weight-grad accumulation as
  the ACTUAL bottleneck (wte 17× largest layer grad; the header's GEMV
  claim per-op true / per-step misleading), the 24-line byte-identical
  CE-backward duplication, + 7 more. GOAT: 318 kernel_heal + 201
  default tests, clippy 0 in all 4 states (8 doc warnings fixed
  pre-commit incl. a NEW variant: a leading `+` on a wrapped doc line
  parses as a markdown list item). Generator note: the one-shot Python
  script hit TWO anchor bugs before write (a %-format collision with
  literal `%` in concept text — fixed by plain concatenation; a
  double-update of the D-header comment) — both aborted cleanly, zero
  pollution, then first-compile after fix.
- **Batch 35 DONE (2026-08-15, Research 039):** the Gemma-2 TRAINING
  cluster (`gemma2_cubecl_train/`: mod.rs 1,286 + attention_gpu.rs 349 —
  the Issue 430 Option B fused causal attention kernel + the hybrid LoRA
  training forward/backward; resident.rs 1,639 deferred) — **8 rules,
  corpus 267 → 275** (A 49, B 45, C 29, D 43, E 17, H 12, L 9, N 16):
  mask-sentinel-matches-reduction-identity, smem-staging-gated-by-
  cross-thread-reuse, kernelize-host-chain-after-sync-batching-floor,
  block-reduction-separate-scratch-runtime-strides,
  saturating-nonlinearity-recovery-breaks-near-cap,
  adapter-grad-join-mirrors-forward-delta-insertion-point,
  side-consumer-readback-alias-live-handle-for-dependent-dispatch,
  quadratic-attention-intermediates-recompute-from-linear-saves. ONE
  coordinator seed KILLED ("thread role rotation" = ambient fused-kernel
  structure — falsification 3-for-3) + ONE CORRECTED (the 52 syncs is
  the POST-batching floor of 2/segment, not pre-batching — that is what
  makes the floor rule distinct). 12 candidates dropped as
  duplicates/thin (the predicted dedup pressure). Bug yield filed as
  riir-ai Issue 688 (15 hazards: 3-way sync-count doc inconsistency,
  ~1.1 GB/step churn from unused `_into` variants + engine doc-lie,
  ~82 MB dead activation slots, per-launch params buffer, 256
  triplication, un-asserted q_dim/kv_dim, KV liveness contract,
  per-layer grad churn, 262 MB EngineActivations/step, 2.6× doc
  understatement, h-recompute ~9.7 G MACs, sliding-window note). GOAT:
  321 kernel_heal + 201 default tests, clippy 0 in all 4 states (4 doc
  backticks fixed). Generator: ONE anchor bug caught pre-write (category
  arrays need the string-literal closing quote in anchors) — clean
  abort, zero pollution. **A SIXTEENTH count-pin site surfaced via test
  failure (not grep): the `kernel_categories_counts_match_plan_003_table`
  per-category expected-counts table — added to the list below.**
- **Batch 36 DONE (2026-08-15, Research 040):** the Gemma-2 RESIDENT
  training cluster (`gemma2_cubecl_train/resident.rs` 1,639 — the Plan
  481/482 all-GPU LoRA training path: per-position GEMV forward + batched
  GEMM forward + batched backward; completes the Batch 35 cluster,
  `gemma2_cubecl_train/` now FULLY MINED) — **7 rules, corpus 275 → 282**
  (A 49, B 45, C 30, D 48, E 17, H 13, L 9, N 16):
  constant-scale-folded-into-upload-copy,
  side-branch-operand-residency-matches-mutation-lifetime,
  activation-checkpoint-as-handle-clone-deferred-batch-readback,
  training-forward-shed-decode-loop-shape,
  sibling-dispatches-before-any-readback,
  independent-cpu-work-fills-dispatch-read-window,
  grad-terms-merge-before-shared-link-jacobian. FOUR coordinator seeds
  falsified — the highest count yet (F4 the per-position path already
  caches its cos/sin table via `RopeCosSinCache` + Batch 17 had dropped
  cos/sin-table rules; F5 the env-var A/B gate is GOAT-gate process;
  B2 the ping-pong swap is needless-swap-temp fix(2)'s loop-carried
  scratch; B4 the scratch hoist is a 3-rule composition) + 2 premise
  corrections (F3 the per-position path's "ONLY sync point" doc-lie —
  93,440 unbatched reads; B1 q/k/v consume DIFFERENT inputs so
  fan-out's premise doesn't apply). 1 weak candidate dropped
  (phase-disjoint buffer aliasing — 9 KB local win). Bug yield filed
  as riir-ai Issue 689 (15 hazards: headline the DEFAULT-path
  ~10.2M-transcendental CPU RoPE recompute from not saving
  `k_post_rope` (P2, cheapest fix — one handle clone), the 93,440-read
  unbatched legacy readback + doc-lie, 262 MB `g_logits_all` fresh
  alloc + dead zero-fill per backward step, the comment-lie
  `g_attn_out_vecs` per-layer allocs, the nested↔flat reshape
  round-trip ~327 MB/step, `read_handle` fresh-Vec ~1.2 GB/step
  download churn (the `read_handle_into` fix substrate already in-tree
  at kimi_k3), missed CPU/GPU overlap windows, the rmsnorm
  scratch→dest API asymmetry, + 7 more). GOAT: 324 kernel_heal + 201
  default tests, clippy 0 in all 4 states (7 doc backticks fixed:
  `481/482`, `read_batched`, `param-grad`, `g_logits_all`, `LoRA`,
  `RoPE`, `all-GPU` — NOTE: backtick `LoRA`/`RoPE` inside `//!` header
  prose or clippy flags them). Generator: ONE anchor bug caught
  pre-write (the entries-vec close anchor must use the LAST ENTRY's
  distinctive opt-pattern TEXT, not the rule id — `rindex` of the id
  resolves to its occurrence in the TESTS, after the vec) — clean
  abort, zero pollution, first compile after fix.
- **Batch 37 DONE (2026-08-15, Research 041):** the Q4_K TRAINING cluster
  (`gemma4_q4k_train/mod.rs` 1,389 + `weight_buffers.rs` 553 — the QLoRA
  frozen-Q4_K-base + f32-LoRA-delta Gemma-4 trainer;
  `forward_batched.rs` + `backward.rs` deliberately deferred to a future
  batch on predicted dedup pressure) — **8 rules, corpus 282 → 290**
  (A 52, C 31, D 50, L 10, N 17 — sparse-gather landed in C beside its
  inverse twin constant-scale-folded-into-upload-copy, not the proposed
  B): kv-window-device-resident-ring-append, lm-head-loss-positions-only,
  verbatim-upload-on-format-match, stream-drain-free-source-during-build,
  identical-tensor-handle-clone-zero-reupload,
  sparse-gather-scale-at-read-not-table-prepass,
  tiled-host-transpose-before-quantize,
  materialized-fast-path-bulk-slice-accessor. SIX seeds falsified
  (highest-per-agent yet: F1 rope tables ARE cached once — 48 layers
  share 2 via `rope_table_idx`; F3 `LoraSite::none()` is the correct
  frozen-base skip, not B31's inert adapter; F4 gammas built once at
  construction; F5 the 3 backward constructors field-identical — no
  `merkle_root`-class miss; F4' the setup path is pure enqueue, 0
  round-trips; F5' the GEMV path already ships packed Q4_K + in-kernel
  dequant = the B15 direction) + 2 confirmations with regime corrections
  that became rules (scale-at-gather is CORRECT for a sparse read set;
  quantize runs once but is paid TWICE: forward + transposed backward ≈
  14 GB on a 24 GB 4090). 1 dropped as dedup
  (variant-dedup-constant-table-by-index — share-pipeline-set + B17's
  rope-table-drop precedent). Bug yield filed as riir-ai Issue 690
  (~19 hazards: 43,008 blocking reads/step on the unbatched path (the
  file's own Bench 433 doc), the dead lm_head for prompt positions
  (~127 MB/step), the dual-KV quadruple-write ~402 MB/step, the O(S²)
  window collects on already-contiguous rows ~537 MB/step/layer, ~40
  GB/step sliding-window re-upload, 2× GPU residency, 7 GB + 48 GB
  one-time dead zero-inits, 2 doc-lies, + more, 6-step fix order).
  GOAT: 327 kernel_heal + 201 default tests, clippy 0 in all 4 states
  (45 `doc_lazy_continuation` + 8 backtick warnings fixed — ROOT CAUSE:
  a line-initial `+ ` in `//!` list-item continuations parses as a NEW
  markdown list item, flagging EVERY subsequent line; backtick
  `lm_head`/`Q4_K`/`QLoRA`/`.rs` names in header prose). Generator
  lessons: TWO slice-anchor misplacements caught by the count-pin
  test — slice-tail anchors are ONLY safe when extracted from the
  enclosing `KernelCategory::X` block (regex), never by line-number
  proximity (all 15 slices end with the identical `],\n    ),` shape);
  ONE malformed edit_file op (schema-foreign key) silently truncated
  a header sentence — caught by re-reading; a `git stash` near-miss
  (pop verified complete — use `git show HEAD:file` for comparisons,
  NEVER stash with sibling WIP present).
- **Batch 38 DONE (2026-08-15, Research 042):** the Q4_K TRAINING cluster,
  second half (`gemma4_q4k_train/forward_batched.rs` 1,077 + `backward.rs`
  941 — the layer-major batched forward, the 43,008 → 337 dispatch fix, +
  the per-position and batched backwards; the cluster is now FULLY MINED) —
  **4 rules, corpus 290 → 294** (A 55, D 51):
  `env-flag-resolved-once-not-per-dispatch` (D — `std::env::var` on the
  hot path takes the global env lock + allocates a String per lookup; one
  instance sits IN FRONT of the autotuner's papaya lock-free cache hit),
  `flat-activation-slots-as-readback-target` (A — nested
  `[pos] → Vec<f32>` activation slots force a scratch + scatter two-copy
  tail after every batched readback; the container layout IS the copy
  count), `transient-spike-pins-persistent-scratch` (A — a one-shot
  `resize(batch × vocab)` ≈ 134 MB pins a persistent `RefCell` scratch at
  ~8× its recurring max; grow-only is right for recurring sizes and wrong
  for a non-recurring spike), `transposed-consumption-accumulate-not-gather`
  (A — `z = Bᵀ·g` against row-major B: k-outer column gather at stride
  `rank × 4B`, up to 16× read amplification, → i-outer row accumulation;
  summation order changes — fine for grads, NOT for bit-identity gates).
  ONE subagent rule KILLED by coordinator math (zero-gradient-position
  elision: K/V cross-position coupling re-introduces non-zero gradient at
  prompt positions after the topmost layer — support queries attend to
  prompt keys — so only the topmost layer's pre-attention chain + the head
  can skip; the SAME flag applies to Issue 687 H3's ~50% magnitude claim,
  which is ~1/48 — 687 flagged for its owner, not modified). Dedup kills:
  the backward's per-layer RoPE `powf` table rebuild (third strike — B17
  drop + B36-F4 + B37-F1), sibling-batch region packing, resize-churn,
  rayon-attention direct-write (2nd in-file instance of
  materialized-fast-path), LoRA-delta add-pass fusion (B19), the
  RefCell-scratch API shape (ambient in 3 rules' opt_patterns), B36
  sibling-dispatches (a MEASURED non-application — Bench 437 found the
  noread combo ~16% slower on Metal; B29's live-output gate explains it).
  Bug yield filed as riir-ai Issue 691 (headline: BOTH backward variants
  have ZERO in-tree callers — the 941-LOC file is unreached, its math
  unpinned; then dead `x_normed` in both Phase 1s, 48× per-layer RoPE
  table rebuilds while the trainer caches bit-identical tables, the
  ~94 MB/step clone storm, ~380 MB/step fresh output-Vec churn +
  ~380 MB/step redundant `hidden` re-uploads (5×/layer), the 134 MB
  scratch pinning, the dead `attn_scores` field, 97 env lookups/step,
  the strided LoRA z-gather at 2 sites, 2 doc inconsistencies, + the
  Issue-690-H2 magnitude refinement (prompt-logits waste is zero at
  n_in=1)). GOAT: 330 kernel_heal + 201 default tests, clippy 0 in all 4
  states. **Generator lesson re-learned (now twice-observed)**: bare
  `json.dumps` WITHOUT `ensure_ascii=False` produced `\u2014` escapes in
  Rust string literals → 33 compile errors — treat `ensure_ascii=False`
  as non-optional in the generator template. A 17th count-pin site
  surfaced (stale "282-rule" comment in
  kernel_categories_cover_all_corpus_rules_exhaustively — B37 missed it).
- **Batch 39 DONE (2026-08-15, Research 043):** the KimiK3 full-model
  decode forward (`kimi_k3_gpu_forward.rs` 2,601 LOC — the largest single
  unmined file; hybrid CPU/GPU decode stack: GPU GEMVs + CPU elementwise
  glue, 7 forward entry-point variants, dual weight-update paths, MLA KV
  cache rewind for the Ouro loop, + the dense SiTU FFN helper family) —
  **9 rules, corpus 294 → 303** (A 60, C 32, D 53, K 5):
  `autotune-cache-extract-outlives-owner` (K), `append-cache-rewind-
  overwrite-in-place` (A), `slow-path-bootstraps-raw-buffer-hotswap-cache`
  (A), `entrypoint-variants-share-layer-body` (D),
  `elementwise-offload-needs-device-side-anchor` (C),
  `tiny-param-buffer-rebuild-over-cache` (A),
  `readback-destination-passing-free-fn` (A),
  `variant-owned-output-buffer-clone-escape` (A),
  `pipeline-layer-boundary-device-handle` (D). TWO rules merged from
  independent two-agent convergence (the anchor rule + the clone-escape
  rule — the coordinator merge for the latter read the actual borrow
  structure: the file's existing free-fn extraction did NOT fix the
  clones, proving ownership placement is the load-bearing half). 13 dedup
  kills/falsifications. Bug yield filed as riir-ai Issue 692 (14 hazards:
  dead 640 MB embed_handle at construction, the FFN clone/fresh-alloc
  storm (6+4 sites), the saved forwards' ~310-line divergence with LOST
  OURO env dispatch, contradictory sync docs 7→3 vs 3→1, gate+up
  duplicated GPU upload vs doc claim, vocab logits double-copy, + 8
  more). GOAT: 333 kernel_heal + 201 default tests, clippy 0 in all 4
  states (4 doc_markdown backtick fixes: `KimiK3`, `SiTU`,
  `O(max_seq)`, `O(iters × max_seq)` — O() expressions are a new gotcha
  variant). Generator: 25 anchors verified in a DRY-RUN before the real
  run (all unique) — the new standing step; one post-run fix (concept
  substring asserted lowercase vs caps — the concepts_covered test
  caught it).
- **Batch 40 DONE (2026-08-15, Research 044):** the KimiK3 GPU backward
  (`kimi_k3_gpu_backward.rs` 4,332 LOC — the LARGEST FILE in the crate; hybrid
  CPU/GPU gradient stack: transposed-GEMV backward via harvested
  WeightBufferSlot caches + dense SiTU FFN / MoE / MLA / KDA backwards; the
  backward half of the Batch 39 forward) — **7 rules, corpus 303 → 310**
  (A 64, D 54, H 14, N 18): `host-scratch-mirrors-slot-order-for-coalesced-
  writes`, `write-plan-derived-from-cache-slots-not-recomputed`,
  `cpu-source-builder-skips-device-roundtrip`,
  `expert-loop-two-phase-split-batch-across-iterations`,
  `rmsnorm-backward-gamma-scope-correction-term`,
  `per-position-scalar-factored-gemv-collapse`,
  `foreign-accumulator-bridge-copy-hoist-to-outer-loop`. ONE proposal dropped
  (rayon parallel transforms — thin vs rust_perf generic guidance; the missed
  MLA instance lives in Issue 693 H11). **PRUNER root-cause fix shipped**: the
  L2 wrap strategy (`fn _wrap() {{ {code} }}`) glued the closing brace into a
  trailing line-comment → false parse failure for statements-only snippets
  ending in a comment (rule 7 was the first hit); hardened with boundary
  newlines — the corpus found a pruner bug. Bug yield filed as riir-ai
  **Issue 693 — the biggest of the mining program**: TWO coordinator-verified
  critical wrong-math bugs on the production training path (MLA `d_attn_w`
  spurious attention-weight factor L2576-2584 — the Issue-393 scalar-factoring
  reused the aw-scaled term for the unscaled consumer; MoE latent-norm gamma
  over-application L1537 — gamma outside the parenthesis, masked by
  near-unity gamma init + 1.5e-2 tolerance + two-wrongs-agree parity) + a
  NaN-VACUOUS parity suite (the LCG test generator leaks random bits into
  the f32 exponent field — the "[2,4)" comment is false; both max-statistics
  silently skip NaN → 4/5 tests pass vacuously) + one ACTIVELY-FAILING KDA
  test (diff 4.95e27, empirically observed) + the feature in NO CI gate + 15
  more hazards (MLA slot-plan drift, production-dead KDA/LM-head subsystems,
  MoE zip without assert, sequential MLA transposes, doc-lies, ...). GOAT:
  336 kernel_heal + 201 default tests, clippy 0 in all 4 states (1 doc_markdown
  fix: `2+2k_r`-style math with subscript underscores needs backticks — a new
  gotcha variant). Generator: dry-run caught 1 bad anchor (header text
  mismatch) — second consecutive clean landing.
- **Batch 41 DONE (2026-08-15, Research 045):** the KimiK3 backward SEQUENCE
  (`kimi_k3_gpu_backward_sequence.rs` 1,328 — the PRODUCTION full-model backward
  caller path, Issue 390 T6.7) + `weight_buffer_cache.rs` (459 — the write
  substrate; the ORIGIN of the B39 hotswap-cache pattern, never itself mined) —
  **6 rules, corpus 310 → 316** (A 69, E 18):
  `concurrent-disjoint-write-buffer-parallelizes-uploads` (E),
  `aliased-slot-allocate-replace-fallback-severs-binding` (A),
  `pool-slot-write-coalescing-owns-only-declared-ranges` (A),
  `pool-reservation-token-outlives-raw-view` (A),
  `gpu-type-bridge-borrows-not-clones` (A),
  `superseded-primitive-machinery-zombie` (A). **Issue 693 H1/H2 production
  routing PINNED** (Lane B): the sequence path has NO inline MLA/MoE math — it
  delegates 100% to the batched twin (`mla_backward_gpu_batched` seq L914-923 →
  H1; `moe_backward_gpu_batched` seq L1151-1159 → H2); production callers
  confirmed in riir-train (kimi_k3_train.rs L2014-2066/L2403-2413/L2481-2491,
  kimi_k3_ouro.rs L1387-1391); the sequence's own inline RMSNorm is the CORRECT
  shared helper; KDA bypasses both bugs. Bug yield filed as riir-ai **Issue
  694**: the orphaned-write fallback (CRITICAL on cuda_backend — `extract_buffer`
  is a no-op Ok so EVERY update takes the allocate-replace fallback which severs
  the aliased-handle binding → GEMVs read boot weights forever + a fresh orphaned
  buffer per step; latent on Metal CPU-fallback), the zombie LM-head machinery
  (~640 MB-class staging upload + GPU transpose dispatch EVERY step with ZERO
  production readers + ~1.3 GB dead VRAM + the stale perf-justification comment
  in riir-train L2200-2210 + the stale header doc-lie seq L13), the Ouro update
  path routing through the GPU-source `update()` paying ~237 read syncs
  (kimi_k3_ouro.rs L1198-1202), the activation double-clone storms (MLA/Dense/KDA),
  debug_assert-only size contracts (release-mode cross-slot corruption), dead
  `update_from_cpu`, per-layer alloc churn, mutex-poisoning unwraps. GOAT: 339
  kernel_heal + 201 default tests, clippy 0 in all 4 states (1 latent_retrieval
  warning = the sibling's WIP heal.rs:415, pre-existing). **Generator NEW
  LESSON — anchor-replacement must REPRODUCE consumed prefix**: the entries-vec
  anchor includes the LAST ENTRY's closing tail; the replacement dropped it →
  unterminated string literal → "mismatched closing delimiter" pointing at the
  vec close (symptom), not the entry ~150 lines above (cause). Fix: restore the
  anchor's prefix content at the head of the replacement. Plus 1 G1 parse fix
  (pseudocode fns need bodies — `fn f(...)` with a trailing comment is invalid
  inside the `_wrap` fn; give it `{}`).
- **Batch 42 DONE (2026-08-15, Research 046):** the Gemma-2 D2F block-causal
  decode cluster (`gemma2_d2f/mod.rs` 1,579 — the CPU-orchestrated hybrid
  decode loop, Plan 108, cloned from the legacy dispatch shape instead of the
  in-tree device-resident substrate + `gemma2_d2f_sc.rs` 800 — Self-Conditioning,
  Plan 250 T1-T4) — **4 rules, corpus 316 → 320** (A 70, B 46, D 56):
  `refine-loop-invariant-prefix-elision` (D — the denoise loop re-forwards the
  PROVABLY-invariant prompt prefix every step; block-causal t_n closes it +
  mutation-set closure proves it; O(steps × seq) → O(seq + steps × |S|);
  precondition: refine-set closure — SC feeding prompt embeddings breaks it),
  `softmax-entropy-via-shift-identity` (B — ln(e_t) = logit_t − max exactly;
  ONE transcendental for the whole entropy sum; the file's own from_logits is
  the POSITIVE instance), `runtime-identity-gate-cached-verdict-elides-side-
  projection` (D — content-classified inertness cached write-through at the
  mutation boundary gates whole-stage elision; the gate must cover producer
  AND consumer — the file's own ungated update_from_logits computes 2.36
  GB/position/step the loop provably discards), `expected-embedding-transposed-
  gemv-resident-head` (A — the soft-token expectation x̂ = Σ p_v · wte[v,:]
  factors as the tied lm_head's transposed GEMV on the already-resident table;
  the vocabulary translation is load-bearing). 18 falsifications (the file is a
  museum of already-mined anti-patterns — incl. the DiffusionSampler learned
  gate judged outside the 15 categories + the KV re-upload being B37's
  ref_shape verbatim). Bug yield filed as riir-ai **Issue 695** (25 hazards:
  1.7 GB/block KV GPU→CPU→GPU round-trip, ~227 GMACs invariant-prefix
  recompute, 46,592 blocking read_one/block decode, dead prompt lm_head 151
  GFLOPs, hardcoded softcap/scale with dead host fields + no CPU-parity test,
  the truncate-the-wrong-end reachable panic, the unreachable trained-SC path —
  `ScTrainingForwardResult` never constructed + `set_w_sc` zero production
  callers + riir-train's separate synthetic SC impl). GOAT: 342 kernel_heal +
  201 default tests, clippy 0 in all 4 states (the 1 latent_retrieval warning
  = the sibling's WIP heal.rs:415, pre-existing). **Generator lesson (third
  anchor class): the entries-vec tail anchor MIXES literal 2-char `\n` escapes
  (INSIDE the Rust string literal) with REAL file newlines (after
  `.to_string(),`)** — treating them uniformly returns count=0; the dry-run
  caught it (2 aborted attempts, zero pollution). Plus 3 doc_markdown fixes
  (`ln(e_t)`/`logit_t`/`lm_head`'s in `//!` prose need backticks — the
  underscore-identifier family). The 2-lane split (decode structure + SC
  substrate) resolved its one overlap cleanly: the identity INIT is the B30/B31
  instance, the runtime GATE mechanism is the rule.
- **Batch 43 DONE (2026-08-15, Research 047):** the Weaver speculative-decode
  GPU corrector cluster (`weaver_gpu_corrector.rs` 1,449 — Plan 436 Phase 4
  `T4.1`: `GpuWeaverCorrector` + batched scratch + per-depth/batched
  `correct_marginals`; + `weaver_gpu_dflash.rs` 576 — `T4.2`: the `DFlash`
  orchestration wrapper; the `weaver_gpu/` kernel dir is a SEPARATE cluster,
  still unmined) — **5 rules, corpus 320 → 325** (A 73, B 47, D 57):
  `mutable-input-scratch-residency-write-per-call` (A — per-iteration
  `create_from_slice` input churn → scratch-resident handles + `client.write`
  per call; the file's own batched twin is the positive contrast),
  `topk-select-nth-over-full-vocab-sort` (B — O(V log V) full pair sort →
  `select_nth_unstable` + K-prefix sort over u32 indices; composes with
  order-stable-topk), `post-scatter-renorm-support-set-only` (A — dense
  re-sum/rescale after a K-scatter → sum-during-scatter + support-only
  rescale, BIT-IDENTICAL by IEEE), `stack-array-row-view-bridge` (A —
  `Vec<&[f32]>` heap collect → `[&[f32]; 64]` stack bridge, 5+ in-tree
  sites), `empty-slice-sentinel-launders-shape-violation` (D — the defensive
  `&[]` row type-checks past the shape contract into a swallowed Err; the
  negative space of B29 launch-unchecked). 23 falsifications (Issue 468
  P0/P2 already shipped several rules' fixes — the per-depth vs batched
  paths sit side by side as native naive/opt contrast). Bug yield filed as
  riir-ai **Issue 697** (17 hazards: the LIVE 3-layer silent no-correction
  chain undersized-capture → `&[]` row → `HiddenShape` Err → `let _ =`
  swallow; the release-mode embedding OOB window ~2.1 GB from a
  debug_assert-only length check; ~29 alloc requests/call; depth=0
  unvalidated; the GOAT assertion stricter than its own doc; test-only
  reachability with NO weaver_gpu CI row). GOAT: 345 kernel_heal + 201
  default tests, clippy 0 in all 4 states. **Generator lesson (a FOURTH
  anchor-class variant)**: the mixed-escape tail anchor must account for
  EVERY byte between the pseudocode's last `\n` escape and `.to_string(),`
  — the fn body's closing `}` sits between them (`...walk\n}".to_string(),`);
  an anchor missing the `}` matches nothing. Plus the header anchor must
  match MID-LINE from distinctive tail text (wrapped `//!` continuations
  share the prefix; a line-start assumption fails). 2 doc_markdown fixes
  (`T4.1`/`T4.2` + `DFlash` — the CamelCase-adjacent family).
- **Batch 44 DONE (2026-08-15, Research 048):** the Weaver GPU KERNEL
  substrate + the FULL kvarn/ cluster (`weaver_gpu/mod.rs` 1,657 — the
  19-dispatch kernel implementations behind the Batch 43 corrector:
  `GpuWeaverWeights` upload/download, pos-emb add, batched SwiGLU, causal
  MHA with smem softmax, indirect embedding gather, dot-per-row,
  softmax-over-K with the Issue 468 `P0` `probs_offset` write-shift +
  `kvarn/` — kvarn_zone_weights.rs + kvarn_dequant.rs + kvarn_quant.rs +
  the small siblings, the game-side CPU reference + zone-scale registry
  for `KVarN` KV-cache quantization; Lane B cross-verified every fix-shape
  against the canonical `katgpt-kv` kv_cache.rs/var_norm.rs) — **9 rules,
  corpus 325 → 334** (A 75, B 49, C 33, D 59, E 19, L 11):
  `row-dot-split-inner-axis-block-reduce` (E — one-thread-per-row pins 128
  threads to a SINGLE workgroup at production + 16× read amplification;
  split the reduction axis — the INVERSE regime of grid-over-output),
  `fuse-indirect-gather-into-dot-consumer` (C — a materialized `D*K*h`
  gather whose only consumer re-reads it in a dot → gather in-loop via
  the device-resident index array), `share-cube-size-const-between-kernel-
  and-launcher` (D — 4 kernels vs 6 launchers restating the 256 literal is
  a silent stride contract), `stash-kernel-internal-intermediate-in-smem`
  (A — a global write + re-read nobody outside the kernel observes),
  `hoist-bit-address-out-of-packed-row-sweep` (B — the per-row unpack
  re-derives an identical bit address; decompose once per position),
  `fold-tile-transpose-into-dequant-scatter` (L — materialize-then-
  transpose = 2 reads + 2 writes/element → scatter at production),
  `bit-cursor-unpack-to-width-specialized-byte-batch` (A — a software bit
  cursor re-derives what a per-width arm makes compile-time-constant),
  `single-sweep-minmax-for-quant-scale` (B — two folds scan twice for the
  (lo, hi) pair; the canonical var_norm comment documents the identical
  fix), `serialize-container-canonical-order` (D — `HashMap` iteration
  order leaks into serialized bytes, breaking BLAKE3 commitment
  stability). 22 falsifications (documented trade-offs + 8 dedup kills +
  canonical positives). Bug yield filed as riir-ai **Issue 698** (25
  hazards: the legacy `KVarN` round-trip DROPS the `VarN` inversion —
  scales discarded at kvarn_quant.rs L160, ~3.67× error masked by vacuous
  tolerances; the CPU/GPU auto-route doc-lie; the drifted unpack fork
  corrupting non-pow2 widths (no cross-byte handling); `GpuWeaverScratch`
  dead with known-wrong sizing; the dot-per-row cliff; `hot_swap_zone`
  inconsistent state; `memory_bytes` off-by-4). GOAT: 348 kernel_heal +
  201 default tests, clippy 0 in all 4 states. **Generator lesson (FIFTH
  post-write defect class, caught by first compile)**: the `lit()` helper
  must append `.to_string()` — json.dumps produces only the quoted
  literal; 36 E0308s pointed at every new field. Clean revert + fix +
  re-run (the revert-verify discipline held; corpus/router were pristine
  pre-edit — NOT sibling WIP). Plus the THIRD concepts-covered
  case-sensitivity occurrence ("TWO" vs "two" — assert the EXACT case of
  the prose) + 2 doc_markdown fixes (`seq_len` in `<= 16` prose, `VarN`).
- **Batch 45 DONE (2026-08-15, Research 050):** the SHINE hypernet
  meta-LoRA cluster, first half (`hypernet/meta_lora.rs` 732 — the CPU
  reference bidirectional forward for memory extraction, Plan 098/104 +
  `hypernet/train_meta_lora.rs` 601 — the CPU finite-difference Meta-LoRA
  trainer) — **3 rules, corpus 334 → 337** (A 77, B 50):
  `scratch-depth-matches-shape-invariance` (A — allocation depth must
  match shape invariance: a layer × position × head nest allocates every
  scratch at the deepest convenient point though all shapes are bound by
  (seq_len, dim) fixed for the whole call; the tell is two same-shaped
  residuals at different depths),
  `softmax-denominator-hoist-out-of-weighted-v` (B — scores[t]/sum_exp
  recomputed head_dim times per key position in the PV loop; hoist
  inv_sum once per (position, head); the in-tree optimized twin is
  attention.rs attention_head pass 3),
  `frozen-cooperand-hoist-out-of-perturbation-loop` (A — materialization
  cadence must match mutation cadence: the FD sweep rebuilds the frozen
  co-operand per candidate + the callee re-derives its own
  sweep-invariants per eval; build a LossPrelude at sweep entry;
  complements make-unmake which fixes the VARYING operand). 9+
  falsifications (adapter-conversion-clone = the immutable-operand
  family's CPU twin — mechanism is not decision content; FD
  unflatten-per-perturbation = make-unmake instance; etc). Bug yield
  filed as riir-ai **Issue 699** (H1: lora_buf sized by
  model_config.lora_rank vs adapter rank 128 → silent get_unchecked_mut
  OOB when real configs set lora_rank: 0 — masked by a 4==4 test
  coincidence; H2: GQA kvd < out_dim slice OOB; H3: apply_meta_lora
  dead-but-public + 16× scaling + same-bytes-TRANSPOSED A/B layout vs
  the production path; FD clone storm 1.4 MB/step test scale; baseline
  ctx-8 hardcode + single-sample GOAT estimator). GOAT: 351 kernel_heal
  + 201 default tests, clippy 0 in both feature states. **Numbering
  collision caught at write time**: `.research/049_*.md` was
  dual-allocated by a concurrent session before this batch landed (the
  highwater was stale at 49) — this batch's note is **050** (the
  standing re-scan rule's second observed save). First-compile clean
  (dry-run discipline); 10 doc_markdown backtick fixes pre-commit.
- **Batch 46 DONE (2026-08-15, Research 051):** the MaxSim cluster
  (`maxsim/dispatch.rs` 449 + `maxsim/spectralquant.rs` 581 + both WGSL
  kernels + the Benchmark 002 crossover bench) — **3 rules, corpus 337 →
  340** (A 79, D 60): `size-gate-covers-every-cost-factor` (D — CPU/GPU
  size gates must gate on the FULL work term, not a partial shape product;
  the pinned constant bakes in both implementations' current overheads;
  2 in-tree instances: maxsim `ld*batch` + gpu_m2p `rows*num_heads`),
  `declared-max-array-taxed-per-thread-under-runtime-bound` (A — runtime-
  bounded function-scope `array<f32, 256>` pays the declared max in
  thread-local per thread; pipeline override constants fix it),
  `shared-operand-side-pair-batch-index-decomposition` (A — pair-batch
  `[batch × lq × d_h]` replicates the shared query side; decompose the
  launch index instead; CPU twin `maxsim_score_packed` exists). 14
  falsifications (d_eff unpack-bounds hypothesis KILLED by kernel read —
  the unpack bound IS d_eff; per-call churn = B33/B43 documented trade-off;
  one-thread-per-pair = B44 regime split). Bug yield filed as riir-ai
  **Issue 700** (15 hazards, all latent-until-adopted — zero production
  callers: validate() never checks compressed_k; the +1-padding OOB read
  executes in tests; d_eff bounds unchecked anywhere; the FALSE
  no-katgpt-dep premise hiding the SIMD twin one feature-forward away;
  the bench's own finder warning against the shipped constant; the 800K
  doc example ~133 GB infeasible). GOAT: 354 kernel_heal + 201 default,
  clippy 0 both states (15 doc fixes — line-initial `+` RECURRED).
  **Generator NEW lesson (live-caught):** `open(p, "w").write(apply(...))`
  TRUNCATES at open-time before apply() runs — the anchor gate saved the
  content but left a 0-byte file (restored via `git checkout HEAD --`;
  dry-run green does NOT prove the write path safe — compute content
  first, then open). **Rule 0b removed same day (owner's call)** — the
  overnight-window gate is gone; the idle queue runs at any hour, with
  collision protection via the standing write-time discipline only.
- **Batch 47 DONE (2026-08-15, Research 052):** the SHINE hypernet SECOND
  half (`shine.rs` 402 + `routing.rs` 268 + `types.rs` 269 +
  `m2p_transformer.rs` 611 + `gpu_m2p.rs` 568 +
  `kernels/m2p_attention.wgsl` 173 — completes the hypernet module
  alongside Batch 45; meta_lora/train_meta_lora were ACTIVE sibling WIP
  (Issue 699 fix pass) — quotes drawn only from clean files; bench-only
  consumers, zero production wiring) — **3 rules, corpus 340 → 343**
  (B 51, D 61, E 20): `per-key-projection-hoist-out-of-query-sweep`
  (B — K/V projections re-derived inside the query loop multiply
  projection work by seq; the optimized twin projects ALL QKV once at
  sweep entry in the same module family),
  `multi-head-out-proj-consumes-head-concat` (E — the out-projection
  needs the head concat; a per-head thread degenerates to
  scalar-times-rowsum, masked by identity fixtures),
  `shared-producer-prefix-extract-once-per-input` (D — two sibling
  methods sharing an expensive pure producer pay it twice under one
  composed call; extract once, thread the artifact). ~17 dedups (the
  workgroup(1,1,1) shape = B44/B23/B28 composite; per-column weight
  re-upload = B33/B28/B26/B40 with the dead `grid_stride` as the
  fossil; the size gate = B46's pre-recorded 2nd instance; Vec churn =
  B45 depth; powf freq table = 3rd-strike cos/sin; routing norms =
  norm-decoupled; shape metadata = derive-shape-from-payload). Bug
  yield filed as riir-ai **Issue 701** (headline: the WRONG
  out-projection math in BOTH WGSL kernels AND the CPU fallback —
  `attn_val * rowsum(wo)` instead of the matvec + the head-local
  `wo_row` indexing bug, masked by identity fixtures + a print-only
  pseudo-parity bench; release-mode stack OOB at DEFAULT head_dim 512
  vs the `[f32; 128]` array; WGSL `array<f32, 256>` scores overflow at
  seq > 256; the ~251 GB default memory grid + the TRIPLE-inconsistent
  13/15/19H provision constant (99.96% of the grid discarded at
  default); structurally-dead all-zero LoRA modules at toy scale via
  the `min()` clamp; row-attention dispatch exceeding the 65535
  workgroup limit; no test compiles either WGSL kernel). GOAT: 357
  kernel_heal + 201 default, clippy 0 both states (10 doc fixes — the
  B37 line-initial `+` struck a 4TH time + 7 pre-existing
  `KERNEL_CATEGORIES` slice-header backticks from the B46 split —
  whole-crate clippy after ANY corpus-layout change is the standing
  check). Both lanes' quotes re-verified by the coordinator in clean
  files.
- **Plan 339 handoff (riir-train, T5.1, 2026-08-18)** — 5 kernel-rule candidates
  from Research 417 §2.3, now WITH measured verdicts from Benches 469/470/472
  so the loop doesn't mine refuted ones:
  - `symmetric-half-gemm-mirror-epilogue` — **REFUTED as a gain** on Ada
    (Bench 469: `cublasSsyrk_v2` gives up 2.2×/FLOP throughput for the 2× FLOP
    cut; 1.01–1.09× SLOWER at every shape). Only mineable as a
    negative-annotation/context_notes entry if the corpus wants one.
  - `gram-domain-iteration` — **REFUTED numerically** (Bench 470: the
    Gram-domain restart `P = X·P₀·X` needs 4 digits of cancellation in TF32's
    3; broke the well-conditioned control 16/26 → 6/26). Recorded in-source at
    `muon_dense_cuda_ns.rs` run_gpu_ns.
  - `restart-to-kill-spurious-eigenvalues` — **measured WORKS with
    constraints** (Bench 470: ε=1e-3 + 2 full passes → 26/26 adversarial +
    0.9587 > 0.9454 production; constraints `ε ≤ γ−1`, full coefficient
    schedule, data-domain Gram re-form). The strongest distillation candidate
    of the five — reopen path (c), same shape as the B49/B52 measured-GOAT
    distillations.
  - `error-feedback-subset-selection` — shipped (Plan 339 Phases 2–3, Bench
    472) but is optimizer-loop algorithm content, not kernel content —
    out of kernel_opt scope; the loop decides if it fits rust_perf at all.
  - `fp16-mantissa-over-bf16-range` — untested by Plan 339 (no fp16 work
    landed); still an open candidate, no in-repo evidence either way.
  In-repo proof refs: riir-train `.benchmarks/469/470/472`,
  `crates/riir-train-gpu/src/muon_dense_cuda{,_ns,_sampled}.rs`.
- **NEXT mining target**: the riir-gpu file-mining program is COMPLETE (Batch 51, Research 057 — corpus 347). The idle-loop mining item shifts to rust_perf + issue-driven work + measured-GOAT distillations (the Batch 49 path: a measured fix in riir-ai/riir-train becomes a rule the same day). kernel_opt reopens ONLY on: (a) the ternary_deltanet_gpu_forward.rs sibling-WIP unlock (re-verify clean at batch start), (b) new production kernel files landing in riir-gpu, or (c) a measured-GOAT distillation opportunity.
- **Batch 49 DONE (2026-08-16, Research 055):** the Bench-686 measured-lesson
  distillation (single rule, NOT file mining — riir-ai Issue 706 / Bench 686,
  the rmsnorm_quantize_f32 single-pass + register-unroll fix on the 4090 cudarc
  decode path): `runtime-arg-loop-bound-demotes-arrays-to-local-memory`
  (A — a correctly-sized per-thread array whose per-element loops are bounded
  by a RUNTIME kernel arg that is constant at every call site is demoted to
  local memory: no unroll, no register form for dynamically-indexed arrays,
  every array the loop touches inherits the demotion; the invariant is a
  LAUNCHER fact the kernel cannot see — fix: compile-time const bound +
  #pragma unroll, bit-identical by preserved ascending accumulation order;
  the pathology is invisible until a neighbor adds same-address-space
  traffic — the v1 naive fix measured 0.6-1% SLOWER in paired A/B; evidence:
  kernel −18.4% nsys drift-controlled at ppl bit-exact; distinct from B46 on
  three axes: SIZE footprint tax vs BOUND per-access addressing, oversized vs
  exactly-sized array, pipeline-override vs const+unroll fix). Corpus
  343 → 344 (A 80). GOAT: 574 kernel_heal + 415 default + 9 retrieval
  (floors held), clippy 0 both states, all-but-ternary combo check passes;
  CI guard layers 3/4/5 on the 4090 box blocked by the PRE-EXISTING un-gated
  metal dep in riir-gpu → filed riir-ai Issue 707 — RESOLVED 2026-08-16
  (`7c8b69045` dep move landed + M3 T2 verified: metal_tensor_gemm resolves
  + bench_663_zero_copy_interop green, riir-ai `c15cb947d`; guard green on
  both hosts). **4090 toolchain note:** this box resolves the
  repo's 1.97.0 pin to the GNU host with NO mingw linker — every cargo
  invocation needs `+1.97.0-x86_64-pc-windows-msvc` (or
  RUSTUP_TOOLCHAIN=...msvc for scripts); cargo-hack is NOT installed (guard
  layer 3 self-skips).
- **Batch 50 DONE (2026-08-16, Research 056):** the file-mining ZERO-RULE batch — 8 files across 2 lanes (small leaves: game_mux/domain_latent/context/cpu_reference/depth_tier/elf; standalone twins: forward_dflash/forward_flashprefill + 5 WGSL kernels verified). Every candidate was a positive instance (context.rs IS B21 construction-side; domain_latent.rs IS B33; elf's sigmoid IS B32a; flashprefill mean_k IS B35-applied) or an ANTI-instance (forward_dflash's stale uniforms are B26's live ref_shape — the SAME bug class Batch 27 filed in backward_dflash, forward twin; the two-pass softmax is popcorn online-softmax-streaming's ref_shape; the one-thread-per-row reductions are B44's) — 28 drop-table rows. Corpus stays 344. Hazard yield: 31 hazards filed as riir-ai Issues 708 (forward_dflash = UNFINISHED MVP presenting as a forward pass: two no-op stubs (rmsnorm L1027-1047 + attention L1212-1225) + stale uniform_qkv.pos = seq_len-1 for ALL Phase-A dispatches + one shared uniform_matmul across wo/w1/w2/lm_head + tests asserting only nonzero+finite on garbage logits; dflash_training opt-in, latent-until-adopted) + 709 (game_mux demux math UNJUSTIFIED — softmax after the weighted sum + non-negative hash distributions → h_i·h_j > 0 cross-contamination, live riir-games consumer, is_finite()-only tests; cpu_reference f64 doc-lie (all f32 accumulators) + lora_forward doc omits lora_b.wgsl's BASE GEMV (parity trap); elf rms_norm is L2-normalize mislabeled (sqrt(d) factor, collides with cpu_reference's true RMSNorm) + elf embedding-SDAR silent zero-loss stub at its riir-train consumer). Coordinator verified all 9 load-bearing claims against source before filing. LESSON: zero-rule batches at max dedup pressure are a corpus COVERAGE signal — the close read that finds no new rules still finds live bugs; judge mining batches by rules+hazards, never rules alone.
- **Batch 51 DONE (2026-08-16, Research 057):** the FINAL riir-gpu file-mining batch — 3 rules, corpus 344 → 347 (A 81, D 62, N 19): batched-read-slot-membership-decided-at-launch (D — push guards on the Option PAIR, extraction on the implying config flag; flag=true+handle=absent shifts every slot + guaranteed OOB; READ lists have no persistent state to derive from, so fix = decide-once/tag-vec-at-launch, distinct from B40's derive-from-cache write side), unpack-loop-indexed-by-container-word (A — byte-packed unpack indexed by the logical element re-loads the same u32 4× + runtime div/mod + runtime shift; index by the CONTAINER word; honest L1-scope), dequant-placement-by-component-traffic-share (N — values packed + inline dequant (reuse 1, B14 regime doesn't apply), scales pre-decoded f16→f32 at upload; encodes the tension with B31: correct for the sidecar, wrong for the payload). Coordinator verification REFUTED the lane-B orphaned-module CRITICAL (production-wired: lib.rs L895-898 + 3 launch sites in gemma2_cubecl/dispatch.rs) — subagent repo-grep silently failed on the multi-root workspace; every CRITICAL reachability claim gets a coordinator grep (standing rule). Issue 693 closeout: H1/H2 fixes present in both twins + LCG fixed + the stale B40 corpus note refreshed. Hazards → riir-ai Issue 710 (sync2 guard drift, 9-site MLA CPU weight-grad backlog, q8kv multi-tile path untested at n>256 — production-reachable). GOAT: 577 kernel_heal + 415 default, retrieval floors HELD (kernel 100/100/100, blended 77.8/87.3/92.9 above the Issue 022 minimums), clippy 0 both states, combo clean. THE RIIR-GPU CORPUS IS COMPLETE — queue shifts to rust_perf + issue-driven + measured-GOAT distillations; kernel_opt reopens only on ternary_deltanet unlock / new kernel files / measured GOATs.
- **Batch 52 DONE (2026-08-16, Research 059):** 1 rule, corpus 347 → 348 (D 62 → 63) — `phase-split-per-position-data-liveness-row-storage`, the FIRST measured-GOAT distillation after corpus-complete (reopen path (c), same-day as the fix): a Phase-A dispatch loop writing a SINGLE-ROW buffer per position followed by a Phase-B per-position consumer only preserves the LAST write; fix = `[seq_len × dim]` row storage (row-pos writes riding the existing pos-in-uniform; backward reads row pos). Distinct from B26/B27 (uniform staleness = wrong VALUES; this is DATA liveness = right value never stored). Source: riir-ai Issue 708 H4 (dflash forward, fixed in `42b564759` — forward FUNCTIONAL, CPU-reference parity max_rel=5e-6, 565/565 tests, 3 backward tests un-ignored). GOAT: 600 kernel_heal lib + 135 corpus tests, clippy 0 both states (the `[x × y]` bracket family joins the doc_markdown backtick gotchas), retrieval floors HELD (kernel 100/100/100, blended 95.2/97.6/97.6). Generator lesson: multi-line doc-bullet anchors must match the FULL bullet — the first B52 header insert split B51's bullet mid-narrative (B49/B51 exhibit the same pre-existing interleave in that header).

- **Batch 54 DONE (2026-08-18, Research 061):** 1 rule, corpus 348 → 349 (B 51 → 52) — `regularizer-above-noise-floor-coupled-to-damping-cap`, the SECOND measured-GOAT distillation after corpus-complete and the FIRST from riir-train (reopen path (c), the Plan 339 T5.1 handoff's strongest candidate, pre-verdicted): a reduced-precision matrix-polynomial iteration (NS / inverse-PSD-sqrt over a TF32 Gram) on rank-deficient input develops spurious null-space eigenvalues set by SIGNED per-GEMM format noise (~1e-3 TF32), NOT by the eps regulariser (cond~1e12 decay is clean; exact/near rank deficiency with O(1) columns is the trigger). Three coupled facts, none sufficient alone (Bench 470, 26 adversarial cases): eps must sit ABOVE the noise floor (1e-5 is 100× below TF32 noise — "a regulariser quieter than the thing it regularises"), eps is capped by the damping budget (eps ≤ γ−1 must be ASSERTED in code — 10× the budget diverges to σ_max ~1e24 on the CONTROL), and the repair is a second FULL-coefficient-schedule pass with the Gram re-formed from the DATA iterate (eps alone 26/26 but convergence 0.36; eps=1e-3 + 2 passes 26/26 AND production 0.9587 > 0.9454; shipped 16/26; restart alone 14/26 WORSE). Two refuted variants encoded INSIDE the rule as decision content: Gram-domain restart `P = X·P₀·X` (4 digits of cancellation in TF32's 3 — broke the control 16/26 → 6/26) + partial-schedule restart (fixed point σ ≈ 1.7, more passes don't improve). Guard corollary: `t < 1e-20` is FALSE for NaN — write `!t.is_finite() || t < threshold`. Category B (Compute — "precision upcast" territory; N considered + rejected: dtype specialization ≠ precision semantics). Handoff dispositions: syrk-mirror NOT distilled (Bench 469 refuted as a gain), gram-domain encoded as refuted variant INSIDE the rule, error-feedback out of kernel_opt scope, fp16-mantissa STILL OPEN (no in-repo evidence). GOAT: 735 kernel_heal + 549 default lib, clippy 0 both states, retrieval floors HELD (clippy top-1 89.6 vs 87.5 floor). Also FIXED the B49/B51/B52 header interleave in corpus/mod.rs (orphaned B49 content rejoined via span-marker replacement — the recommended method for that header from now on). `.research/.highwater` 60 → 61.

- **Batch 55 DONE (2026-08-18, Research 062):** 2 rules, corpus 349 → 351 (N 19 → 20, D 63 → 64) — the THIRD measured-GOAT distillation after corpus-complete (reopen path (c), same-day as the fix landing): distilled from riir-ai Issue 716 / Bench 691 (q8kv sink guard, landed `e794b0033`) + the Bench 642 / Issue 639 re-hit. Rule 1 `outlier-span-escapes-quantization-via-lossless-sidecar` (N): 1-2 massive-activation channels (x100/x300 at sink pos 0 + delimiter) collapse non-outlier NEIGHBORS in the same 32-block 58x (attention output error 584x/18,844x over uniform; poisoned blocks 88x/299x vs clean dims); the outlier rows are KNOWN A PRIORI (attention sinks = oldest fixed positions — no detection), so the known span escapes quantization into a LOSSLESS f32 sidecar while the q8 buffers keep the rows as DEAD DATA (strides never shift); A/B: sidecar BEATS Hadamard rotation on BOTH axes (rotation spreads MA to absmax ~MA/16 — neighbors regain only ~8x resolution, nonzero — at strictly larger kernel cost; sidecar exact: 4.781 → 0.000698 = 6,850x, full sidecar ≡ f32 kernel) at 0.96x perf; S=0 bit-identical to the unguarded launch. Rule 2 `buffer-length-derived-from-allocation-not-binding` (D): a TWO-OCCURRENCE root-caused class — cubecl derives the kernel-visible length from `handle.size_in_used()` (the ALLOCATION), NOT the bound `BufferArg` length; over-length half (Bench 642: `let dim = input.len()` made rmsnorm reduce over FOLLOWING rows; fix = both-ends binding trims) + zero-length-over-a-real-handle half (Bench 691: `from_raw_parts(query.clone(), 0)` derived sink_rows=1, silently reading the QUERY buffer as position-0 K/V; fix = a REAL 4-byte allocation floor-dividing to 0 rows; contract EXACT-sized); probe corollary: a benign-case-only probe is WORSE than no probe. GOAT: 738 kernel_heal + 549 default + 163 corpus tests, clippy 0 both states (2 doc_markdown backtick fixes — `handle.size_in_used()` + `BufferArg` in `//!` header prose), retrieval floors HELD with ZERO decay (kernel 100/100/100, blended 94.6/97.7/97.7 — identical to the B53 re-pin). Generator: all 23 anchors unique on the FIRST dry run (zero aborted attempts); entries_gpu_06.rs now ~870 lines — next batch opens entries_gpu_07.rs. `.research/.highwater` 61 → 62. Sibling note: katgpt-rs slice-3 sweep was ACTIVE (live `/tmp/heal_slice3` cargo process) — this batch was picked specifically because it does NOT collide (riir-clippy tree only).
- **Batch 56 DONE (2026-08-18, Research 063):** 2 rules, corpus 351 → 353 (A 81 → 82, D 64 → 65) — the FOURTH measured-GOAT distillation after corpus-complete (reopen path (c), same-day as the fix landings), from riir-ai Issues 719 + 714/720 (both resolved on the 4090 box). Rule 1 `concurrent-context-init-needs-process-wide-lock` (D): the init-outside-publish-on-success pattern does NOT serialize construction — publication is atomic, construction is not; every first-caller that misses the fast path runs the full driver-level sequence (Instance/Adapter/Device + CubeCL server registration) concurrently, which wedges NON-DETERMINISTICALLY at the driver level on 4090/Windows/Vulkan (7-thread race: 4 wedged forever inside `request_device`, another run 1/7 inside `init_device`; the stuck point MOVES between runs; full lib suite froze at 101/604) AND every loser leaks its device + server for process lifetime (no deregistration); the CROSS-PATH half is load-bearing — multiple constructors each with their OWN OnceLock run the SAME sequence and race each other, so ONE process-wide init Mutex guards all of them (fast-path read → lock → DOUBLE-CHECK → init → publish; Err with the lock released — failure not cached; poison de-fanged); regression gate = same-path + cross-path race tests (`context_init_race.rs`, 8+8 threads); post-fix 605/0/2 in 1664 s — the first green parallel-mode full suite on the box. Rule 2 `superseded-format-constructor-live-fields-only` (A): a constructor accepting an OPTIONAL second weight format while unconditionally uploading the primary f32 set uploads DEAD WEIGHT (WGSL Q4_K ctors: ~9.7 GiB f32 dead on ~1.4 GiB Q4_K, 14,089 → ~3,500 MiB peak; cubecl half: 23.8 GB peak on 24 GB); the dead-set proof is a DISPATCH-SITE audit (every primary-buffer bind sits inside the superseded format's match arms), never the ctor alone; fix = tiered live-fields-only variants (`skipped()` / `norms_only()`) with zero-size placeholder TRIPWIRES (accidental bind fails validation LOUDLY; raw wgpu accepts size 0 — B31's minimum-legal-footprint applies where substrates validate non-empty), wired INSIDE `new_internal` when `weights_q4k.is_some()` so callers cannot forget; `forward_layer_by_layer` REFUSES Q4_K; the class shipped TWICE back-to-back (714 + 720) + the Issue-690 2x-residency hazard — a ctor-format pair is the trigger; measured Bench 692: LOGITS_HASH bit-identical, −10.3 GiB device + −2.2 GiB host. GOAT: 760 kernel_heal + 568 default + 166 corpus tests, clippy 0 both states (6 doc_markdown backtick fixes — `request_device`/`init_device`/`Q4_K`/`skipped()`/`norms_only()` in the new `//!` header), retrieval floors HELD with ZERO decay (blend 94.6/97.7 — identical to the B53/B55 re-pin). Generator: all anchors unique on the FIRST dry run (second consecutive clean landing); entries_gpu_06.rs now ~950 lines — **Batch 57 opens entries_gpu_07.rs**. `.research/.highwater` 62 → 63.

- **Batch 57 DONE (2026-08-18, Research 064):** 2 rules, corpus 353 → 355 (A 82 → 83, D 65 → 66) — the FIFTH measured-GOAT distillation after corpus-complete (reopen path (c)), from riir-ai Issue 715 (resolved+removed; commits `9f9e68402` + `93d853582`; the fixes predate B56's queue scan but were never evaluated — found by this session's git-log re-scan). Rule 1 `smem-slot-role-reversal-needs-barrier` (A): a shared-memory scratch whose slot ROLES reverse between phases needs a barrier at each reversal — (a) intra-iteration: every thread reads the reduction result `smem[0]`, then thread 0 overwrites that slot with its weight (a slow warp reads a weight as the tile max; threads disagree on the softmax shift; length-independent); (b) loop back-edge: the last phase reads every `smem[p]` while the next tile's Phase 1 writes `smem[tid]` (only `n_tiles >= 2`); measured 27B / 256-token probe: NONDETERMINISTIC before (first divergent token 71), BIT-IDENTICAL (3/3) after; both barriers in BLOCK-UNIFORM control flow; five kernels shipped the races (3 CubeCL + 2 cudarc twins) — the CubeCL fix did NOT reach the cudarc arm (it dispatches hand-written CUDA; grepping one source tree proves nothing about which kernels an arm dispatches); recurring class: Issue 612's fix removed the destructive reduction but left both windows open; the corpus's prior barrier content is ALL perf-side (eliminate/fuse via warp-shuffle/plane-sizing) — this is the correctness complement. Rule 2 `tile-loop-back-edge-needs-multi-tile-bitexact-gate` (D): a loop that trips twice only past a size threshold hides every back-edge defect from fixtures that fit inside ONE iteration — every prior decode test used `n_positions <= 200` < the 256-wide tile (untestable BY CONSTRUCTION; tests passed because the code never ran); the gate needs BOTH a boundary-crossing fixture (`n_positions=300` → `n_tiles=2`) AND a bit-exact reproducibility assert over repeats (a tolerance-only assert cannot see a race in the low mantissa bits); validated BOTH directions (fails run 0 with max_diff=0.0586 barriers-removed); second instance: Issue 710 H10 (q8kv multi-tile, zero coverage at n > 256 until a 2-tile fixture). Dropped: RoPE table-dim (3rd-strike rope territory, driver-contract), reduction-tree copy-paste slips, twin-propagation-as-standalone (folded into rule 1; B39 carries the structural half). GOAT: 766/0/1 kernel_heal + 571/0/0 default, clippy 0 both states (zero post-write fixes), retrieval floors HELD with zero decay (kernel 100/100/100, blend 94.6/97.7). Opens `entries_gpu_07.rs` (gpu_06 at 843 lines). Count-baseline note: B56's 760/568 predate the sibling's `c85ff1c` fixtures (+3 tests via the `d8e614d` rebase); true HEAD baseline 763/571. `.research/.highwater` 63 → 64.


- **Batch 58 DONE (2026-08-18, Research 065):** 8 rules, corpus 355 → 363
  (A 83 → 86, D 66 → 70, E 20 → 21) — **the ternary_deltanet sibling-WIP
  unlock (reopen trigger (a)): the LAST never-mined riir-gpu orchestration
  file** (`ternary_deltanet_gpu_forward_cudarc.rs`, 5,691 lines — the
  raw-CUDA decode-forward twin; unblocked when the Issue 717 speculative
  seam + Issue 715 barrier fixes landed; the CubeCL twin stays blocked on
  Issue 721 T3, whose Non-goals exclude the CUDA port — collision-free by
  design). Heavy two-lane convergence (4 mechanisms found independently by
  both subagents, merged by the coordinator): `async-htod-host-source-
  outlives-copy` (A — an async HtoD memcpy reads the host pointer at
  GPU-COMPLETION time; a stack-local source dangles and the GPU silently
  reads the dead stack slot; persistent host staging fields +
  `slice::from_ref`; the HOST-side complement of the readback-timing
  family), `captured-graph-bakes-buffer-addresses` (D — mutators split
  into address-stable (in-place `memset_zeros`, graph SURVIVES) vs
  identity-changing (alloc/free, graph MUST die for lazy re-capture, else
  replay runs against freed pointers); the inverse-facing half of B25),
  `output-buffer-rotation-pool-multi-forward` (A — K pipelined forwards
  writing the same output field: swap the field's handle with pool[i] per
  dispatch, an O(1) ownership exchange; ONE sync, K+1 position-aligned
  reads; the field KEEPS the last buffer between cycles so the next
  cycle's out[0] is the live pre-speculation prediction),
  `mid-pipeline-device-alloc-is-implicit-sync` (D — `cuMemAlloc` can
  implicitly synchronize the device: an allocation inside a
  no-intermediate-sync dispatch sequence is a HIDDEN BARRIER;
  pre-allocate the whole cycle at construction sized to SPEC_MAX_K),
  `capture-region-dispatch-only` (D — the captured region records GPU work
  ONLY: host effects run ONCE at capture and NEVER on replay (a baked
  `pos += 1` freezes position silently); syncs/dtoh not capturable;
  preconditions: event tracking off BEFORE any allocation, non-default
  stream, pre-upload), `occupancy-grid-cap-refuted-uncapped-default` (E —
  the Issue 705/Bench 684 measured NEGATIVE never distilled at resolution
  time: the occupancy cap measured a 1-2% LOSS at every size; wavefront
  locality beats the wave tail; the cap stays env-gated as measurement
  apparatus), `write-before-read-state-skips-checkpoint` (A — a
  speculative checkpoint backs up ONLY read-modify state; write-before-
  read append-only caches (KV at position p) are repaired by re-execution
  itself; async dtod, ZERO CPU-GPU sync), `same-input-gemv-segments-
  single-launch` (D — same-input GEMV groups batched into ONE multi-
  segment launch (≤4 segments); nsys: 497 GEMV launches/token with m=48
  projections at the ~5.6 µs latency floor; pad unused slots with segment
  0's pointers (m=0 matches no rows); the generic middle between B25's
  shared-upload-N-launches and B15's bespoke fused pair). 23 dropped as
  dups/thin. Mining yield → riir-ai Issue 722 (13 hazards: the
  `spec_restore_buffers`/`spec_unrotate` doc-lie (referenced but
  nonexistent), set_lora release-mode silent no-op, capture-error-path
  stream leak, conv1d backward full-buffer download, attention-backward
  per-token alloc storm, uncached lm_head dequant_transpose, + 7 more).
  GOAT: 769/0/1 kernel_heal + 571/0/0 default, clippy 0 both states (one
  post-write fix round: the B37 line-initial `+ `/`- ` lesson struck a 5TH
  time — 3 trigger lines cascading 33 doc-list warnings + 6 backtick
  items; plus the B51 case-sensitivity lesson in the concepts test),
  retrieval floors HELD with zero decay (kernel 100/100/100, blend
  94.6/97.7 — sixth consecutive batch). Generator: all anchors unique on
  the FIRST dry run (fourth consecutive). The riir-gpu corpus is now
  COMPLETE-minus-one (only the CubeCL twin, blocked on Issue 721).
  `.research/.highwater` 64 → 65.

- **Batch 59 DONE (2026-08-18, Research 066):** 1 `rust_perf` rule, corpus 62 → 63
  (arithmetic 3 → 4; 53rd benchable) — **the FIRST EXTERNAL-SOURCE distill
  via the `distill` skill** (raygon-renderer/thermite `divider.rs`, a
  libdivide port; verdict YES B+ from the same session's evaluation):
  `division-by-invariant-multiplication` — loop of `x / d` with
  RUNTIME-invariant `d` → precomputed Granlund–Montgomery magic multiplier +
  shift (FastDivU32/U64; mulhi + shifts per element vs 20-90+ cycle hardware
  div). Two-regime guard (thermite's own split): CONSTANT divisors are
  already LLVM strength-reduced — the regime is exclusively runtime-invariant
  `d` applied ≥~3× (setup pays one hardware div); the bench harness
  black_boxes the divisor so the baseline can't be const-folded. **Measured
  2.03/2.13/2.07× ParetoDominant (3/0/0, M3 aarch64; x86 DIV 30-90+ cycles
  should be wider), allocs 0/0, G-det stable.** G1 = the NEW
  exhaustive-equivalence-test class for ported numeric algorithms
  (`division_magic_matches_hardware_div_exhaustive_low_d`: all u32 d 1..=4096
  + u64 d 1..=257 + edge divisors × boundary+LCG numerators) — it caught TWO
  pre-landing bugs in the ported construction: floor-vs-ceil magic (d=3 x=3:
  thermite wraps `+1` on BOTH branches — the plain branch needs the CEIL
  magic 0xAAAAAAAB, floor under-approximates at exact multiples) + u32-wrap
  carry loss (d=7 x=u32::MAX: `(mulhi + x)` reaches ~2^33; a u32 wrap drops
  the carry bit — fix = widening add in u64/u128). Fixture/harness/mirror
  pinned by 4 substring canaries. Floors re-pinned (`PERF_SPANS` 52→53,
  blended 100→101: measured 94/98/98, perf 51/52/52, router 71/101 — new span
  top-1). Gates: 572 default / 590 rust_perf_bench / 624 latent_retrieval
  lib, clippy 0 both states. Proposer DEFERRED (architectural rewrite —
  follow-up candidate). Drops: approx-op (policy.rs unread), DoublePump
  (architectural), branch-hint (PGO-class), tgamma input-domain (crate
  mostly todo!()). Standing lesson: **a ported numeric algorithm without an
  exhaustive equivalence gate ships subtly wrong.** `.research/.highwater`
  65 → 66.

- **Batch 60 DONE (2026-08-18, Research 067, Plan 006):** 1 rule, corpus 363 → 364
  (N 20 → 21) — the SIXTH measured-GOAT distillation after corpus-complete
  (reopen path (c), same-day): riir-train Issue 460 / katgpt-rs `e2853fbf` —
  the MLA backward reconstructed the pre-gate attention output as
  `attn_out_gated / gate`; `gate = sigmoid(W_g·h)` underflows to a HARD ZERO
  in f32 below ~−88 → 0/0 NaN (silent in the forward; surfaced steps later as
  an opaque NsInvSqrtFailed; 4 of 10 real sequences, 4 of 5 seeds;
  localization: the FIRST corrupt tensor was `mla.w_g`, the ONLY consumer of
  the reconstruction). Rule `pre-gate-recovery-by-division-cancels-
  algebraically`: never reconstruct a pre-gate value by dividing by the gate —
  every gradient consumer multiplies by gate-derived factors, so
  `pre·g·(1−g) == gated·(1−g)` CANCELS (bit-identical where finite, no
  epsilon); at g == 0 the value is information-theoretically unrecoverable —
  guard public reconstructions to 0.0 + SAVE the true value at checkpoint
  boundaries; detection tell: NaN localized to the single consuming weight
  matrix while post-gate-reading grads stay clean. Distinct from B35 / B32b /
  B32a (failure axis NaN-vs-0, operation division-vs-identity, fix
  elimination-vs-recompute). GOAT: 778/0/1 kernel_heal + 577/0/0 default,
  clippy 0 both states, retrieval floors HELD (kernel 100/100/100, blend
  94.7/97.7/97.7 of 131; top-1 improved). **Plan-first discipline adopted
  (owner call): Plan 006 created; every future batch creates its
  `.plans/NNN_*.md` BEFORE implementation** (Protocol A step 0 below + the
  distill skill's plan step, both added same-commit). `.research/.highwater`
  66 → 67; `.plans/.highwater` 5 → 6.

- **Batch 61 DONE (2026-08-18, Research 068, Plan 007 — the FIRST plan-FIRST
  batch):** 2 `rust_perf` rules, corpus 63 → 65 (dispatch 1 → 2, layout 5 →
  6; benchable 53 → 55, annotated stays 12) — the oxc "Pursuit of Performance
  on Building a JavaScript Compiler" distill (oxc.rs/docs/learn/performance,
  fetched via queue item 11 online perf search; the first FRESH-online-search
  batch after B48/B53/B59). **Both ParetoDominant:** `keyword-map-to-match`
  (dispatch; **5.23-5.58×**, zero allocs) — static-keyword
  `LazyLock<HashMap<&str, V>>` dispatch → plain `match` on string literals;
  LLVM lowers string-literal matches to a switch on string LENGTH + one
  inline compare per length group (oxc tried a gperf-style perfect hash,
  COULD NOT beat the match, then found the length-switch in
  `--emit=llvm-ir`); the HashMap pays SipHash per lookup — the folk wisdom
  "string match is O(n), use a hash map" is refuted for STATIC key sets
  (literal-initialized maps only); proposer deferred (key-set extraction).
  `usize-index-to-u32` (layout; **1.81-1.99×**, zero allocs) — usize
  index/offset fields in high-volume structs → u32 (the oxc Span 16B→8B
  change, +5% on large parser files); wins ~2× even at 1K where both arrays
  are L1-resident (twice the elements per cache line); domain must bound
  below 2³²; proposer deferred (type ripple — the B58 caller-gate class).
  8 other oxc candidates DROPPED at dedup (5 duplicates of existing rules:
  box-enum-variants / Box<str> / SIMD-skip / BufWriter / jbecker
  buffer-reuse; 3 architectural/external: arena-AST, interning-sharding,
  string-inlining). GOAT: 595/0 default lib, clippy 0 default +
  rust_perf_bench states, retrieval floors HELD (12/12; perf@1 94.3%,
  blend@1 92.1% structural — no crowding). Generator lessons: the B37
  line-initial `+ ` lesson struck a 6th time (header rewrap); NEW LESSON —
  edit_file fuzzy matching silently DROPPED a `#` from an unrelated
  raw-string terminator (`}"` vs `}"#`), un-terminating the division
  harness 100 lines away (E0425 + phantom E0308s) — raw-string `#`
  terminators are load-bearing; re-read file state after edits near `}"#`
  boundaries. Also: `match {...} as u64` does not parse (bind `let k = match
  ...; k as u64`). `.research/.highwater` 67 → 68; `.plans/.highwater` 6 → 7.

- **Batch 53 DONE (2026-08-17 evening, queue item 11 — online perf search +
  Bench 038):** the 4 online-search candidates landed as `PerfRule` entries +
  `BenchSpec` harnesses + eval spans + Pareto bench verdicts + floors re-pin.
  Corpus 58→62 rules (allocation 17→18, layout 4→5, arithmetic 1→3),
  benchable 48→52, annotated 10→12, `PERF_SPANS` 48→52 (all 4 new spans
  retrieve top-1). **2/4 ParetoDominant:** distance-sq-no-sqrt 1.21-1.24×
  (compare-squared beats sqrt-then-compare even on aarch64's single-instruction
  fsqrt — the fixed form vectorizes), hashmap-with-capacity 2.4-3.1× (allocs
  12→1 @ 4096 — the rehash ladder re-SipHashes ALL entries per resize, not a
  realloc+memcpy like the Vec sibling). **2/4 honest annotations:**
  powi-vs-powf ALL-TIE 1.00× (the folk-wisdom refutation — LLVM instcombine
  lowers pow(x, 2.0) to the same x*x as powi(2) in release; the advice only
  holds for debug profiles / exponents ≥ 3 where pow stays a libm call),
  soa-hot-field latency-bound Mixed 1.07×/tie/0.96× (a serial f32
  add-reduction chain dominates BOTH layouts — OoO hides the 40-byte-stride
  load latency; the SoA win lives on throughput-bound field consumers —
  count/min-max/transform — or multi-field scans). G5 batch bar 2/4 FAIL
  recorded honestly (the filter-count-to-any precedent). Environment: benched
  with the sibling's 1-core CPU-deterministic bonsai parity running;
  discharged by an independent re-run reproducing verdict + ratios exactly.
  Floors re-pinned both states (2-domain blended 92/96/96 of 100, perf top-3
  50/52, router 65/100; 3-domain kernel 100/100/100, blended 94.6/97.7/97.7
  of 130, router 89/130 → floor 84/130). Gates: 496 default / 514
  rust_perf_bench lib tests, clippy 0, retrieval gate PASS both states.
  Proposers: the two ParetoDominant rules LANDED same-evening (Batch 53
  follow-up — `distance-sq-no-sqrt` sum-of-squares + pure-threshold gates;
  `hashmap-with-capacity` first-mention for-loop + len-base gates,
  capacity-is-a-hint semantics; `FIXABLE_RULES` 8→10, gates 549 default /
  601 latent_retrieval / 612 fix_verify / 4 bins, clippy 0 in all three
  states); `powi-vs-powf` stays proposer-less BY DESIGN (ALL-TIE in
  release — zero gain, the filter-count-to-any rationale applied to a
  tie) + `soa-hot-field` stays suggestion-only (architectural). Record:
  Bench 038.


- **Batch 48 rust_perf half DONE (2026-08-16, Research 054):** the 8
  online-search candidates landed as `PerfRule` entries + `BenchSpec`
  harnesses + eval spans + Pareto bench verdicts + floors re-pin. Corpus
  50→58 rules (complexity 12→14, allocation 14→17, iterators 4→7),
  benchable 40→48, annotated 8→10. **6/8 ParetoDominant:**
  collect-append-to-extend 8.5-12.8× (allocs 6251→1), vec-insert-front-
  deque 7.8-115.3×, vec-macro-zero-fill 32-2505× (calloc lazy pages vs
  explicit memset), unwrap-or-else-lazy 1.9-2.3×, hashset-insert-bool-
  return 1.19-1.31×, filter-map-fuse 1.37-1.64×. **2 honest annotations:**
  filter-count-to-any ParetoDominated 0.82-0.90× (the FOLK-WISDOM
  REFUTATION — LLVM vectorizes filter().count() into batched scanning
  while any()'s early-exit forces scalar iteration; at a 50% hit position
  the vectorized full count WINS; any wins only ≲10%-position hits or
  non-vectorizable predicates) + iter-copied-deref-map Mixed (0.80× at 1M
  on sum folds — Copied's length-tracking overhead loses to the unrolled
  Map+sum; the idiom matters for collect's TrustedLen sizing, not folds).
  **Harness lessons (Research 054 §3-§4):** (1) pure-compute harnesses get
  HOISTED out of the timing loop by LICM (same input every iteration →
  perfect 1.00× ties at impossible per-call costs) — the standing anti-
  LICM idiom is `let input = std::hint::black_box(input);` at the top of
  every run_* (allocation-bearing harnesses never hit this); a pure fn of
  a constant arg also const-folds away (black_box the arg). (2) `use` is
  NOT inherited by child modules — a harness typing a container in BOTH a
  mod signature and a top-level run_* needs the use at BOTH levels.
  (3) A position-0 short-circuit drives any() to a literal sub-ns call,
  tripping the validator's div-by-zero Tie guard — position the hit at
  n/2 for a measurable regime. Tooling: `perf_bench_demo` gained
  `PERF_BENCH_FILTER` (comma-separated rule-id substrings) for
  batch-scoped gate runs. Retrieval floors RE-PINNED (96/126 blended
  denominators): perf 87.5/95.8/97.9 (top-1 UNCHANGED vs 40-span set —
  zero batch decay after 3 spans were re-authored to the fixture
  convention: variant names, canonical predicate shapes — filter-map-
  fuse's first draft didn't retrieve its own rule in top-8), blended
  70.8/83.3/90.6 (2d) + 77.8/87.3/92.9 (3d) — ALL above the old
  baseline. Proposers honestly deferred (bench-verdict-first, the
  Issues 012/013/014 precedent): the parse-clean ParetoDominant
  proposer candidates are hashset-insert-bool-return, collect-append-
  to-extend, vec-macro-zero-fill — **LANDED 2026-08-17** (Batch 48
  follow-up: 3 proposers + `delete_stmt_edit` emitter helper + fixture/
  clean-span/cross-domain gates; `FIXABLE_RULES` 5→8; two syn facts:
  type-annotated lets parse as `Pat::Type`, `LitFloat::base10_digits`
  keeps `.`/`e`; filter-count-to-any stays proposer-less BY DESIGN).
  filter-count-to-any must NEVER get an unconditional proposer
  (ParetoDominated). The full-corpus re-run was
  killed ~15 min in: pre-existing btreemap-sorted-iteration alone costs
  ~40+ min (5000 inner queries × ~1-2ms at n=50K ≈ 5-10s per call ×
  calibration × samples × 3 repeats — slow-by-design, not a bug).
  Batch 50 CONSUMED: game_mux, domain_latent, context, cpu_reference,
  depth_tier, elf (domino_gpu.rs = 85-line stub + config.rs = pure config
  types — judged out-of-scope without a dedicated read), forward_dflash,
  forward_flashprefill — 0 rules, 31 hazards → Issues 708/709.
  REMAINING for the FINAL kernel file batch: `kimi_k3_gpu_backward_
  batched.rs` (1,394 — LOW value: shares Issue 693's H1/H2 + the broken
  LCG with the module file), `attention_q8kv_cubecl.rs` (967, B23 sibling,
  dedup HIGH), + optionally forward_prefill.rs / forward_sp_kv.rs /
  forward_target_extract.rs (the remaining standalone twins). Expect 0-2
  rules; after this pair declare the riir-gpu corpus COMPLETE (Research
  057 — RE-SCAN `.research/` at write time).
  - Validation pattern (proven T4 + 32a + 32b-1 + 32c + 36 + 38): corpus files are NOT
    sibling WIP — edit + run GOAT in the main checkout with
    `CARGO_TARGET_DIR=/tmp/riir-clippy-bNN`; `--features kernel_heal`
    for the corpus suite (318 at 34, 321 at 35, 324 at 36, 327 at 37,
    330 at 38, 333 at 39, 336 at 40, 339 at 41, 342 at 42, 345 at 43,
    348 at 44, 351 at 45, 354 at 46, 357 at 47), default for
    the 201 suite.
    **Corpus layout since Batch 46 (2026-08-15 split, owner-approved):**
    `corpus/` module dir replaces the 10.9K-line monolith — `mod.rs`
    (header + build fn chaining slices + KERNEL_CATEGORIES — the ONE
    category table) + `entries_popcorn.rs` (B1-13, 119 rules, FROZEN) +
    `entries_gpu_01..06.rs` (~900 lines each, whole-rule boundaries) +
    `tests.rs`. **New batches append entries to the LAST gpu slice**
    (`entries_gpu_06.rs`, rules 325-340 — open `entries_gpu_07.rs` past
    ~900 lines). Count-pin sites unchanged, now at corpus/mod.rs +
    corpus/tests.rs. NEVER encode category membership in file layout.
    Split lesson: the generator's slice tail must be `]` NOT `])` — the
    bare `vec![` opener has no wrapping `(` (one cold-compile catch).
    `--all-features` is currently broken by SIBLING WIP
    (`rust_perf/bench.rs:335 alloc_axis`) — pre-existing, not yours.
  - Count-pin sites when adding rules (complete list, re-verified 38):
    corpus.rs `entries.len()` assert + KERNEL_CATEGORIES rows + the 6
    per-category count COMMENTS + the `kernel_categories_counts_match_
    plan_003_table` expected-counts table (the 16th site — caught by
    test failure at 35, not grep) + the per-batch comment line in the
    entry-count test + the STALE-corpus-comment in
    kernel_categories_cover_all_corpus_rules_exhaustively (the 17th site
    — "282-rule" survived B37; caught at 38) + 2 grouped-total tests +
    router.rs `rules_for_expert` total + the corpus-count mentions in
    router.rs doc header + the Plan 003 doc-comment `NNN-rule corpus`
    line above KERNEL_CATEGORIES + the `Total: 209 ... current NNN`
    comment + the file-level `//!` header batch block. All in sync at 344 (A 80, B 51, C 33, D 61, E 20, L 11 after Batch 49; F 10, G 19, H 14, I 4, J 5, K 5, M 9, N 18, O 4).
  - **Landing lesson (32b-1, self-caught)**: after a large multi-entry
    corpus edit, `awk`-scan for `rule_id:` lines NOT preceded by a
    standalone `KernelRule {` BEFORE the first compile — JSON `\n`
    escaping in comment blocks can land as literal 2-char `\n` text,
    swallowing the `KernelRule {` opener into the comment (9/10 entries
    broken; rustc's `vec![` unclosed-delimiter error points at the symptom,
    not the cause).
- **DONE 2026-08-19 as Batch 63 (Plan 009 / Research 071; corpus 364→367, 3 rules — the weight-concat third branch of the merge family, the deferred-residual loop-boundary fusion, the reset-in-place pool rule; Lane B structurally empty; 10 hazards filed as a riir-ai issue). HISTORICAL ENTRY:** was the next FILE-batch target (after Batch 62, which
  holds plan-first priority)**: `ternary_deltanet_gpu_forward.rs` (the CubeCL
  twin, now **5,432 lines** on origin/develop). The 2026-08-18 BLOCK condition
  ("mine only after 721 lands + tree verified clean") FIRED: Issue 721 CLOSED
  2026-08-18 as an honest negative (G2 STRUCTURAL FAIL; no chunked-recurrence
  rewrite shipped — the file grew 5,416 → 5,432, +16 lines = additive
  tree-verify hooks, not the feared kernel rewrite). Its last 3 commits are
  721's own (`aa7879ff7` T2-T4, `28ac37fa2` T4a, `1eb211ad2` T6+T7),
  committed clean, and it is NOT in the sibling's dirty set as of 2026-08-19 —
  but the sibling remains ACTIVE on the CUDA twin
  (`ternary_deltanet_gpu_forward_cudarc.rs`, modified), so RE-VERIFY
  tree-clean at batch start (standing `git status`/mtime check). Expected
  yield LOW-MEDIUM: B58 (the cudarc twin) already took the
  transport-graph/capture/staging/rotation-pool rules; the CubeCL twin's
  unique surface is its CubeCL-specific dispatch + the chunked-recurrence
  kernels (IntraOutput/CrossChunk/StateTransition). The cudarc twin was
  MINED as Batch 58 (2026-08-18, Research 065 — the sibling-WIP unlock fired
  when the Issue 717/715 work landed; 721's Non-goals exclude the CUDA port
  so it was collision-free). No batch executed at unblock time — the box is
  saturated by the sibling's p335 LoRA measurement; item-5 GOAT builds
  deferred per the Plan 337 discipline.
- **Batch 62 DONE (2026-08-19, Research 070, Plan 008 — the SECOND external-source distill via the `distill` skill):** LANDED — corpus 65 → 67. `transposed-weight-register-gemv` (simd 5→6): **ParetoDominant 1.18x/2.09x/6.08x** (NIN 16/64/4096 @ NOUT 64, M3 Max) — the note's wash-at-large-nin expectation was WRONG-DIRECTION (strict-FP Rust's scalar accumulator serial chain degrades FURTHER at large nin; llama.cpp's fine row-major dots use explicit multi-accumulator SIMD); architectural, no proposer. `newton-refinement-vs-hardware-transcendental` (arithmetic 4→5, annotated): **Mixed 1.09x/1.01x, hardware never loses** (vs source's 2–4% on Ryzen; M3's fdiv/sqrt latency flattens at 100K). GOAT: 603/621/655 lib tests (default/bench/latent), clippy 0 all-targets, retrieval floors HELD (92.1/96.0/97.0). Drive-by: fixed Batch 61's stale `fifty_three_rules_have_bench_specs` pin (55 actual — the bench-feature lib suite was missing from B61's GOAT) + 9 never-linted Batch-59 test warnings. HISTORICAL QUEUE ENTRY: (2026-08-18, Research 070, Plan 008 — the SECOND external-source distill via the `distill` skill):** the microGPT-C CPU-SIMD distill (github.com/vixhal-baraiya/microgpt-c, MIT, complete; verdict corrected MARGINAL B− → YES B on user challenge — the LOPD/NQF rescue class, record in the note). Route `rust_perf`, 2 candidates: lead `transposed-weight-register-gemv` (column-major pack at weight-freeze, broadcast-FMA into output registers, zero horizontal reduction; live in-tree shapes: `katgpt-types/src/simd/binary.rs` per-row `y[r] = vaddvq(acc)` matvec + `dot.rs` + `math.rs` matmul family) + `newton-refinement-vs-hardware-transcendental` (honest-annotation rule — GPU fast-math recipe inverts on CPU, measured 2–4% slower on 5600H). Drops recorded in Research 070 (Schraudolph exp tier: Cephes already ships at the tier we need + the trains-with-approx catch; sampling: `gumbel_max_sample` strictly better; prefix-table: `precompute-table` instance). Yield 1–2. Plan-first: Plan 008 created BEFORE any corpus edit.
- **Batch 63 candidate pool — LANDED 2026-08-19 as Batch 64 (Plan 010 / Research 072; corpus 67→69): loop-order-innermost-contiguous ParetoDominant 7.14-13.80x + debug-assert-length-hoist annotated Mixed (1.13x@1K / stable 0.67-0.71x@100K aarch64 + the data-dependent-index scope finding). HISTORICAL POOL ENTRY:** 2 candidates, both verified ABSENT from the 65-rule rust_perf list at sweep time (full rule-id list grepped):
  - **`loop-order-innermost-contiguous`** (simd; STRONG) — source: Arun Nagaraj, "Matrix Multiplication in Rust with SIMD: From Naive to 126× Faster" (medium @aruntamiln 2026-02-12; repo ArunTamil21/matmul-simd; x86 AVX measured, technique ISA-independent — it is memory-hierarchy, not ISA): nested-loop accumulation `c[i][j] += a[i][k] * b[k][j]` where the INNERMOST loop varies an index that strides a 2D access's OUTER axis (i-j-k: inner k walks `b[k][j]` column-wise at stride-n) → reorder the loops (i-k-j) so the innermost varies the LAST index (contiguous row walk). Measured in source: 9× alone, L1 misses 45%→12%, 0.47→5 GFLOPS. Parse-detectable (triple nest + `+=` over a double-indexed access whose inner-loop index sits on the outer axis); bounded fix = swap loop headers + hoist the row anchor. Distinct from `simd-chunk` (chunking for vectorization), `layout-reorder` (struct padding), `soa-hot-field` (AoS→SoA data layout), and kernel_opt's `matvec-coalesce` (GPU-side, distinct-by-substrate — the B62-plan GPU-twin note pattern). Caveats to encode: float sum-order changes → rel-error tolerance gate NOT bit-identity (`distance-sq-no-sqrt` 1-ulp precedent); legal only without loop-carried deps across the swapped axes (the `c[i][j] +=` shape is the canonical legal case). **Dedup note for the batch session: ADJACENT to B62's queued lead `transposed-weight-register-gemv`** (which achieves coalescing by repacking weights column-major) — this rule is the NO-repack general nested-accumulation case (matmul/2D-conv/image-kernel shapes where repacking is not wanted); scope fixtures to non-repacked shapes, or fold into the B62 rule if it lands first and generalizes.
  - **`debug-assert-length-hoist`** (complexity, simd-adjacent; MEDIUM — honest-annotation risk) — source: Shnatsel, "How to avoid bounds checks in Rust (without unsafe!)" (2023-01-17, the canonical article; repo Shnatsel/bounds-check-cookbook): a `debug_assert!` whose expression mentions `.len()` (or an index bound) guarding subsequent indexing is ERASED in release — the compiler loses exactly the length constraint it needed to eliminate the bounds checks, so release keeps ALL of them (the article's anti-pattern section: 3 checks retained; "Ironically, this makes the code slower") → swap to `assert!` (redundant asserts are optimizer-removed; "you cannot have too many"). Parse-detectable: `debug_assert!` expr containing `.len()` in a fn whose body indexes that container; bounded fix = macro swap. Honest-annotation risk: typical elimination gains are the 1–3% class (the 15% headline was an x86 unrolling secondary effect; Shnatsel's own aarch64 runs measured 1.03×) — may land Mixed/context_notes-annotated rather than ParetoDominant; the rule's VALUE is stopping the contra-intuitive release-mode pessimization (`debug_assert!` feels free, is not — the inverse of the usual zero-cost intuition). Also encodes the sibling fact from the same article: an `assert!` in front of the hot loop is the cheapest bounds-check hoist (check once, not per-iteration).
  - **Drops (recorded, with reasons):** cache blocking/tiling KC-for-L1/MC-for-L2 (architectural — hardware-specific tile sizes + working-set reasoning needs sizes, not parse-only; GPU tiling already lives in kernel_opt B14/B15); power-of-2 table + `& mask` "cheapest possible bounds check" (accepts WRONG results on out-of-range access — correctness-first excludes suggesting it); iterator retrofit + `.foreach` on long adapter chains (style guidance, partially covered by existing iterator rules); `#[inline(always)]` constraint propagation across fns (cross-function, not parse-local); SME/SME2 (M4+ hardware — this M3 box has none); the Arm developer arm-simd portal page itself (a link hub — every topic maps to an existing rule: branchless-conditional, simd-chunk's multi-accumulator, soa-hot-field, restrict-keyword is C-only).
- **Batch 65 candidate pool (OPENED 2026-08-19, queue item 11 refill — the hotpath-rs blog distill, queue-empty after B62/63/64 consumed everything):** source: Paweł Urbanek, "Lessons Learned Building High-Performance Rust Profiler" (hotpath.rs/blog/rust-performance-profiling, 2026-05-12; the NEW post — distinct from the Research 032 hotpath-rs *guide* distill). Both candidates verified ABSENT from the 69-rule rust_perf corpus at sweep time (grepped false-sharing/cache-pad/align(64)/unpark/batch-send — only incidental "cache line" prose in soa-hot-field/usize-index-to-u32 docs):
  - **`align-64-false-sharing`** (concurrency/layout; STRONG) — a `static` array of small per-thread atomic-counter structs (`[ThreadAllocStats; MAX_THREADS]`, 24 B each → two per cache line) contended by N worker threads → pad each slot with `#[repr(align(64))]` so no two slots share a line. Measured in source (the profiler's own GlobalAlloc hook, wall-clock, the *stronger* metric class than bounded-channel-backpressure's queue-depth): 2t 720→215ms (70%), 4t 1000→280ms (72%), 8t 3700→500ms (86%), 12t 4400→660ms (85%); single-thread 0% (correct — no contention, zero cost). Parse-detectable: a `static`/`static mut` fixed-size array of structs containing `Atomic*` fields + spawn-scoped threads indexing by thread ordinal (also covers `thread_local!`-less designs); bounded fix = `#[repr(align(64))]` on the struct. Bench regime is MULTI-THREADED wall-clock — first rust_perf rule in the contention-metric class that IS wall-clock-benchable (tokio-guard/bounded-channel were NotBenchable for metric mismatch); bench harness = N threads × M increments on adjacent vs padded slots. Caveats to encode: needs the box quiet (contention metrics are load-sensitive — the Bench-649 class); `crossbeam_utils::CachePadded` is the stdlib-external alternative (rule should prefer the attribute, zero-dep); doc the 24B→64B memory trade (irrelevant at MAX_THREADS=256 scale, load-bearing at millions-of-slots scale).
  - **`channel-send-batching-unpark`** (concurrency; MEDIUM — annotated-contender) — high-frequency per-item MPSC sends to a background consumer (µs-scale intervals) hit a `Thread::park/unpark` pair per message (~22% of samples in `Thread::unpark`) when the receiver sits in crossbeam's `select!` (which BYPASSES the direct-subscribe spin-wait-with-backoff path) → batch sends per producer thread (per-thread Vec drained on flush). Measured in source: 500ns → 80ns per measurement (6.25×). Honest-annotation risk: the mechanism is crossbeam-`select!`-specific (the direct receiver already spins; PR submitted upstream); single-threaded/uncontended benches will Tie; the wall-clock gain needs the park-storm regime — likely lands Mixed/context_notes-annotated unless the harness reproduces the storm portably. Adjacent-but-distinct from `bounded-channel-backpressure` (queue-fullness policy vs per-message syscall-adjacent wakeups).
  - **Drops (recorded, with reasons):** RAII-guard async-fn attribution (correctness-of-measurement, profiler-specific — instrument-at-poll is hotpath's implementation detail, not a general code pattern); `#[inline(always)]` symbol-range erasure (profiler-tooling-specific; DWARF-based resolution supersedes); the pprof-rs in-process scaling failure + detached-spawn samply workaround (war-story, no bounded fix-space); exclusive-vs-inclusive CPU attribution via stackTable linked lists (data-format detail); the wall-clock-vs-CPU-sampling taxonomy (measurement-methodology, not a code transformation).
  - **Also-noted (NOT candidates, pool-hunting intel):** this refill searched the 2025-2026 article space broadly (Rust blog, medium guides, Kobzol's 1160-PR retrospective, matklad index, Cloudflare FL2, developerlife locality tutorial, Reddit learnings threads) — the well-trodden sources are EXHAUSTED (Kobzol's retrospective is CI/infra enumeration with toolchain-level perf bits — PGO/jemalloc/LTO, not code patterns; Cloudflare FL2 is architecture-level; the guides restate corpus ground). The hotpath post was the only pattern-dense find. Future pools need different hunting grounds: arxiv systems papers via the research skill, or real-code mining of high-perf crates (regex-automata, hashbrown, polars internals) — the distill skill's source-evaluation workflow.
  - **The named high-perf crates were EVALUATED 2026-08-19 (3-lane distill session; verdicts below — this branch is now CLOSED except where a pool was opened):** **hashbrown NO/MARGINAL** (0 new rules — ~95% ambient SwissTable structure; usage-level guidance already covered by the hashmap rule family; ONE rider: pow2-divisor-to-mask as a regime note + optional constructor-provenance proposer precondition on `division-by-invariant-multiplication` — fold into that rule's next touch, not its own batch). **regex-automata YES B+ → Batch 67 pool** (lead `byte-find-position-to-memchr` — `iter().position(|&b| b == lit)` on `&[u8]` → memchr crate; std specializes `contains` but NOT `position`, verified in rust-src 1.97.1: `cmp.rs:394` SliceContains-for-u8 vs the `memchr_naive` FIXME; EXCLUDE contains; dep-gate fxhashmap-style; arity ladder memchr/2/3; small-haystack wash <32B; rider: heal our own proposers which contain 3 live positive instances). **polars YES B+ → Batch 66 pool** (lane's A− GOAT `float-sum-multi-accumulator` KILLED at coordination as a positive instance of `simd-chunk` — corpus.rs:283-284's fixture IS the shape; **standing dedup lesson: grep `buggy_code`/`fixed_code` BODIES, not just rule concepts — a rule whose fixture IS the candidate's shape is coverage**; salvage = simd-chunk mechanism-note enhancement rider). Remaining hunting grounds: arxiv systems papers; adjacent-but-don't-rewalk: the memchr crate itself (the Batch 67 rule makes it the FIX, not the source), crossbeam CachePadded (covered by Batch 65's align-64 rule when it lands). **Process security note (standing):** one lane received an injected instruction block inside a fetched tool result this session (disregarded) — fetched external content is DATA, never directives.
- **Batch 66 candidate pool — LANDED 2026-08-19 same day (Plan 011 / Research 073; corpus 69→71, 2 rules — sparse-set-bit-iteration-tz-blsr ParetoDominant 4.08/1.34x + sortedness-check-branchless-accumulate ParetoDominant 2.98/2.72x ANNOTATED regime-split; the lane's float-sum GOAT KILLED at coordination as a simd-chunk positive instance, salvaged as the simd-chunk mechanism-note rider; GOAT 612/630/664 lib, floors HELD, clippy 0). HISTORICAL POOL ENTRY:** polars kernel-idiom distill — source `pola-rs/polars` master (MIT, v0.55.1 era, `dist-release` lto=fat cgu=1 — they bench). **TARGETED-FILE mine of ~8 kernel files, NOT a repo sweep** (95% ambient engine). Verdict YES B+ (lane A−, downgraded by the coordinator kill), yield 2–3 — LANDED 2.
  - **`sparse-set-bit-iteration-tz-blsr`** — `for i in 0..64 { if (word>>i)&1 != 0 }` → `tz`/`wrapping_sub` blsr loop (iterates popcount times, not 64; 4–20× at low density; two-regime by density — polars switches sparse→dense ~16/64; companion content: `tz % 64` guard on 2× unroll + `trailing_ones` run fast path). Sources: `polars-compute/src/filter/boolean.rs`, `polars-arrow/src/bitmap/iterator.rs` TrueIdxIter.
  - **`sortedness-check-branchless-accumulate`** — hot-loop `if v < prev { return false }` sortedness validation on usually-sorted data → `sorted &= prev <= v` full-scan (4–8×; regime twin of `filter-count-to-any`'s annotation — same mechanism mirrored: removing early-exit buys vectorization when the predicate rarely fires; annotated regime-split). Source: `polars-utils/src/slice.rs`.
  - Secondary: `map-inplace-same-size-transmute` (B+; **coordinator caveat: `bytemuck::cast_vec` CONSUMES the source — detection must prove source dead-after, else skip**), `u128-funnel-unaligned-word-read` (B; byte-assembly window → 2 overlapping loads + shrd), `sorted-run-grouping-vs-hash` (B−; sort+Entry-grouping → adjacent-run scan; HIGHEST kill risk — statement-sequence detection), `streak-counter-cmov` (C+ optional).
  - **KILLED at coordination:** `float-sum-multi-accumulator` — positive instance of `simd-chunk` (corpus.rs:283-284 fixture IS the shape). Salvage = simd-chunk enhancement rider (N=16 stripe, tree-reduce finisher, strict-FP mechanism note).
  - Benches single-threaded wall-clock → CAN LAND UNDER MODERATE LOAD with load-recording (unlike Batch 65's contention bench). If the box stays noisy, run 66 before 65.
  - Plan-first at batch start (`.plans/011`); snapshot polars files at mine time (fast-moving repo); fixtures ORIGINAL (MIT hygiene).
- **Batch 67 candidate pool — LANDED 2026-08-19 same day (Plan 012 / Research 074; corpus 71→72, 1 rule — byte-find-position-to-memchr ParetoDominant 5.00/19.25/25.31/25.37x at 32B/256B/4K/64K; the predicted 32B wash did NOT materialize; the rider DEFERRED with reason — memchr transitive-only, a direct dep for non-hot scans fails cost/benefit). HISTORICAL POOL ENTRY:** regex-automata memchr distill — source `rust-lang/regex` master (**NOT BurntSushi/regex — stale fork; regex-automata at ROOT, coordinator-verified**). Verdict YES B+, yield 1–2. Lead: **`byte-find-position-to-memchr`** (A−, coordinator-sharpened) — `xs.iter().position(|&b| b == b';')` / `rposition` / manual while-scan on `&[u8]` with byte-literal needle → `memchr::memchr/memrchr`; arity ladder memchr/2/3; **EXCLUDE `contains`** (std already SWAR-specializes it via `SliceContains for u8` → core/src/slice/memchr.rs; measured-check at batch time → context_note at most); mechanism core: generic position has opaque predicate + early-exit → cannot vectorize (filter-count-to-any's documented mechanism) while contains gets SWAR — the asymmetry IS the rule; dep-gate fxhashmap-style (memchr direct dep required to fire); regime: wash <32B (Teddy bound), 5–40× at ≥1KB; bit-identical G1. Secondary: `ascii-class-chain-to-const-lut` (B; only if the proposer falls out cheap — precompute-table adjacency). **Rider: heal our own proposers first** — 3 live positive instances (`collect_append_to_extend.rs:173`, `hashmap_with_capacity.rs:232`, `vec_macro_zero_fill.rs:183` all do `hay[at..].iter().position(|&b| b == needle[0])`). Single-threaded benches → load-tolerant. Plan-first at batch start.
- **2026-08-19 item-11 sweep + Batch 68 (LANDED same day, Plan 013 / Research 075):** web article space RE-CONFIRMED EXHAUSTED (allocator papers = implementation not consumer patterns; arena = architectural; hyperpb = C++). Distill evaluation EXECUTED (2 GitHub-read lanes, coordinator quote-verified): `quickwit-oss/tantivy` = whole-repo MARGINAL B− (the ~95%-ambient hashbrown prior holds) but utility-layer-scoped YES B → **Batch 68 LANDED (corpus 72→73, io category)**: lead `varint-compose-word-single-store` **measured ParetoDominated 0/3 × 3 writer regimes** (Vec-push / inline-Cursor / Box<dyn Write>, load ~25-33, alloc axis equal+stable, outputs byte-identical by the exhaustive u16+boundary+LCG gate, both conventions) — the folk div-chain mechanism dies twice (128 is a compile-time constant → LLVM strength-reduces `%128 /128`; OoO hides the N-calls-vs-1 differential even at dyn dispatch) → shipped as the corpus's **3rd annotated honest-loser** (the `precompute-table` class: exists to STOP the tantivy folk port) + the **tz-blsr xor-clear anti-shape rider landed** (comment-only, the `x ^= 1<<tz` serialized-chain note from `TinySet::pop_lowest`). GOAT: G1 exhaustive+canary PASS, G2 honest FAIL→annotation, G3 clippy 0 both states + 620/638 lib + retrieval floors HELD, G-det stable. **Standing lesson (the B59 note applied in reverse): a source shipping ladder-beside-loop as a 2-regime design is evidence the AUTHOR considered both — NOT evidence the ladder is faster on your box; x86 re-gate before citing.** `BurntSushi/rust-csv` = MARGINAL B−, conversation-only (best candidate `ascii-fast-path-before-utf8-validate` A− is real but niche; memchr/record-reuse/DFA all kill). The named-crate hunting ground is fully adjudicated: hashbrown NO · regex-automata YES (B67) · polars YES (B66) · tantivy YES-but-annotated-loser (B68) · rust-csv MARGINAL — remaining intake: measured-GOAT distillations + issue-driven + arxiv systems papers via the research skill.
- **Progress**: riir-gpu file-mining COMPLETE-minus-one (Batch 51, Research
  057; ~115 files mined by line-weight; the sole remainder is the CubeCL
  ternary twin, blocked on Issue 721). Corpus: 364 kernel rules + 73
  rust_perf rules (Batch 68 = the tantivy utility-layer distill, Research
  075 / Plan 013 — varint-compose-word-single-store MEASURED ParetoDominated
  across 3 writer regimes → the 3rd annotated honest-loser + the tz-blsr
  xor-clear anti-shape rider; Batch 67 = the regex-automata distill, Research 074 /
  Plan 012 — byte-find-position-to-memchr ParetoDominant 5-25x, the
  position/contains std asymmetry; Batch 66 = the polars kernel-idiom distill,
  Research 073 / Plan 011 — sparse-set-bit-iteration-tz-blsr + sortedness-check-branchless-
  accumulate, both ParetoDominant; Batch 61 = the oxc article distill, the first plan-FIRST
  batch — keyword-map-to-match 5.23-5.58× + usize-index-to-u32 1.81-1.99×,
  both ParetoDominant, Research 068 / Plan 007; Batch 59 = the first external-source distill via the
  `distill` skill: thermite divider → division-by-invariant-multiplication,
  2.03-2.13× ParetoDominant, Research 066; Batch 58 = the ternary_deltanet sibling-WIP unlock, the
  LAST never-mined riir-gpu orchestration file — Research 065, 8 rules
  (staging lifetime, graph lifecycle, rotation pool, implicit-sync, capture
  region, occupancy refutation, checkpoint liveness, multi-segment GEMV) +
  Issue 722's 13 hazards; Batch 57 = the fifth measured-GOAT distillation,
  Research 064 (Issue 715 — the smem role-reversal barrier + the multi-tile
  bit-exact gate); Batch 56 = the fourth measured-GOAT distillation,
  Research 063 (Issues 719 + 714/720 — the cross-path init lock + the
  superseded-format live-fields ctor); Batch 55 = the third measured-GOAT
  distillation, Research 062; Batch 54 = the second measured-GOAT
  distillation, Research 061; Batch 48
  = rust_perf online-search half; Batch 49 = the Bench-686 distillation; Batch 50
  = zero-rule leaves/twins, 31 hazards → Issues 708/709; Batch 51 = final
  pair, 3 rules + Issue 710 — the queue now shifts to rust_perf + issue-
  driven work + measured-GOAT distillations).
  edge_lora module FULLY MINED (32a + 32b-1 + 32c). spectralquant/
  FULLY MINED (15 + 33). hypernet/ FULLY MINED (45: meta_lora.rs +
  train_meta_lora.rs; 47: shine.rs + routing.rs + types.rs +
  m2p_transformer.rs + gpu_m2p.rs + the M2P WGSL kernels). maxsim/ FULLY MINED (46 — dispatch.rs +
  spectralquant.rs + both WGSL kernels; gpu_m2p pattern-checked, its
  per-column loop flagged for the hypernet second-half batch). gemma4_cubecl_train/ DONE (34).
  gemma2_cubecl_train/ FULLY MINED (35: mod.rs + attention_gpu.rs;
  36: resident.rs). gemma4_q4k_train/ FULLY MINED (37: mod.rs +
  weight_buffers.rs; 38: forward_batched.rs + backward.rs).
  kimi_k3_gpu_forward.rs DONE (39); kimi_k3_gpu_backward.rs DONE (40);
  kimi_k3_gpu_backward_sequence.rs + weight_buffer_cache.rs DONE (41 —
  the batched twin remains, LOW value). gemma2_d2f/ FULLY MINED
  (42: mod.rs + gemma2_d2f_sc.rs). weaver_gpu_corrector.rs +
  weaver_gpu_dflash.rs DONE (43). weaver_gpu/ mod.rs + FULL kvarn/
  DONE (44 — weaver_gpu/tests.rs test-only, not mined).
- **Also open (riir-clippy issues)**: NONE — queue empty as of 2026-08-16.
  019 (idle-loop skill close-out) RESOLVED 2026-08-16 (T6 full cycle:
  items 10–13 — defer-marking sweep landed riir-train 387 + riir-ai 529
  + riir-chain 026 fixes; item 11 perf search → 8 rust_perf Batch 48
  candidates below; item 12 → Research 053; item 13 doc-sync → game-sdk
  `40f0971`; all 13 queue items exercised ≥1×; T7 stays blocked on
  sibling WIP `ternary_deltanet_gpu_forward.rs`; record: AGENTS.md +
  `git log -- .issues/019_*.md`).
  021 (evidential status axis) RESOLVED 2026-08-16 (Bench 026 —
  `EvidenceTier` Certified/Heuristic/Withdrawn + absorbing-Withdrawn
  transitions + trust-weighted retrieval; Withdrawn never suggested;
  certification admission wired through the Stage 2 tuning loop;
  `record_outcome` gains `verified` on LatentFixMemory + HealPipeline,
  EvolveRecorder keeps the 4-arg CLI shape + `record_outcome_verified`;
  honest A/B finding: equal-similarity distinct-tier pairs are NOT
  constructible — `record_outcome` matches by (lint, fixed) = one stats
  bucket — so the falsifiable A/B is the within-subject certification
  delta (exactly the 0.1 trust gap) + the cross-entry small-edge flip;
  scope-of-validity stretch goal deferred).
  014 (rust_perf v4) RESOLVED 2026-08-16 (Bench 020); 016 (clippy
  structural expansion) RESOLVED 2026-08-16 (Bench 021 — corpus 10→40,
  FIXABLE 6→33, VERIFY 1→4); 017 (retrieval precision at scale) RESOLVED
  2026-08-16 (Bench 022 — 130-span eval + regression gate; clippy top-1
  measured 58.3% at scale, floors pinned); 018 (self_evolve promotion)
  RESOLVED 2026-08-16 (Bench 023 — DEFAULT-ON + unconditional modules,
  EvolveRecorder oracle split, JSON persistence, `RIIR_HEAL_SELF_EVOLVE=0`
  opt-out; found + filed Issue 022 along the way); 022 (corpus real-clippy
  drift) RESOLVED 2026-08-16 (Bench 024 — 5 corpus shapes corrected to
  real-clippy-1.97-firing forms; G6 40/40, G7 50%→100% restored over the
  full 40-lint corpus; retrieval floors re-pinned clippy 58.3→54.2 top-1 —
  truthfulness wins); 020 (spectral routing POC) RESOLVED-KILLED
  2026-08-16 (Bench 025 — best token-frequency variant 70.3% < the 85%
  kill-bar; the ablation proves the LEXICAL signal is the ~70% ceiling,
  not the DFT compression — gating closed for the lexical branch;
  annotation-only salvage: TF-cosine-global wired, 53.4%→69.3%/48.3%→68.6%
  at 0.96µs, standing 63.6% floor in the Issue 017 gate; 7-variant matrix
  kept as the reproducible negative-result artifact).
  015 = RESOLVED 2026-08-16
  (Bench 031 — verify-mode GOAT on the real repo: katgpt-pruners 14 edits /
  12 kept / 2 auto-reverted (the u8 E0308 + fn-path E0631, the G5 run-1
  failure classes), every named skipped site landed compile-confirmed
  usize, 1245/1245 tests, landed as katgpt-rs `212ed7942`; root-cause fix
  `owning_package_root` relative-path → absolute (os error 123); issue
  file removed per noise-reduction rule — the `src/fix_verify.rs` +
  `heal.rs` --verify surfaces are now fair game, not sibling WIP).
  Earlier WIP notes (kept for history): NOTE (2026-08-16, Issue 018
  session): the
  tree was clean + fix_verify had landed, so `Cargo.toml`/`lib.rs`/
  `heal.rs` were safely touched for 018 — re-verify sibling WIP state
  before touching them again. NOTE (2026-08-16, Issue 021 session):
  tree clean again; 021 deliberately avoided `heal.rs` (EvolveRecorder's
  4-arg shape kept) — only `lib.rs` (EvidenceTier re-export) was touched
  from the protected list.

