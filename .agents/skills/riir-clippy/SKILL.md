---
name: riir-clippy
description: Auto-continue idle loop + riir-clippy batch mining/heal workflow for the riir-* workspace. Activate when the user says "continue", "auto continue", "keep going", "nothing to do", "idle", "next", "next batch", "Batch 32+", or when a session summary shows remaining riir-clippy batches; when asked to fix clippy across repos, mine new rust/kernel perf rules, dry-run `cargo heal --suggest/--fix` on workspace repos and adapt proposers when suggestions are wrong, find/finish issues or plans, run boundary-guard/substrate-first routine checks, run benchmark regression sweeps, search online to unblock, or find related papers. Chains doc-sync + research skills at the right steps.
---

# riir-clippy — the auto-continue idle loop

You are the idle-time worker for the workspace. When activated without a
specific task, run the priority queue below, **one unit per cycle**,
re-scanning after each completed unit. **Commit + push each completed unit
before starting the next.**

**Rule 0 — session context outranks the queue.** User-reported
regressions/bugs/questions in the current conversation always go first.
The queue is for idle time only.

## The idle priority queue

| # | Item | How |
|---|------|-----|
| 1 | **Fix open issues** | Scan `.issues/` across repos, oldest first, skip deferred. Fix → validate → doc → remove file (noise-reduction rule). |
| 2 | **Finish in-flight plans** | `.plans/` with unchecked `- [ ]` — complete the next task group, honor `- [-]` defers. |
| 3 | **Clippy sweep, all repos** | Heal-driven where possible (see heal protocol below), manual `cargo clippy --all-targets` otherwise. |
| 4 | **Resolved-issue hygiene** | Find issue files whose work is verifiably landed but file remains → doc at `.docs`/AGENTS.md → `rm` the file. |
| 5 | **Batch mining + heal dry-run** | The centerpiece — see the two protocols below. |
| 6 | **boundary-guard routine check** | Run `scripts/ci_boundary_guard.sh` where present (riir-mmorpg-examples); otherwise the 7-surface grep table from the boundary-guard skill. File violations as `.issues/` — do NOT fix in the same commit as detection. Record the run in the skill's Run log. |
| 7 | **substrate-first drift audit** | Mode 2 of the substrate-first skill: inventory substrate primitives, grep consumers for parallel systems (3+ vocabulary variants!), classify (false positive / POC simplification / DRY violation / architectural), file issues. |
| 8 | **Benchmark regression sweep** | GPU-exclusivity check FIRST (see discipline), then pinned suites; compare vs `.benchmarks/` baselines; drift → bisect → root-cause fix → new bench file. |
| 9 | **Online unblock search** | For each blocked/`[-]` item: web search the error/blocker, fetch via `https://r.jina.ai/...` per global Search rules. |
| 10 | **Defer-marking sweep** | Plans/issues with silently-stalled tasks → mark `- [-]` WITH a reason + unblock condition; file a follow-up issue when the blocker is external. |
| 11 | **Online rust/kernel perf search** | rust-optimize checklist, nnethercote/perf-book, bevy, CubeCL/burn, wgpu, CUDA best practices → candidate rules → into the batch pipeline. |
| 12 | **Related-paper research** | Hand off to the `research` skill (its pre-flight + prior-art gate are mandatory — ad-hoc "search online" is the #1 false-PASS mode). |
| 13 | **doc-sync pass** | Invoke the doc-sync skill against repos with landed-but-undocumented work. |

Items 6 + 7 run BEFORE item 8 (benchmarks) by design — boundary/DRY
findings are cheaper to fix before you benchmark around them.

## Protocol A — batch mining (queue item 5)

1. Read the **mining queue snapshot** below + `ls` the target tree. Pick
   the next unmined cluster (1–2 files, ~8–12 rules). NEVER mine a
   sibling-blocked file.
2. **Dedup grep BEFORE proposing** — grep the existing corpus
   (`src/domains/kernel_opt/`, `src/domains/rust_perf/`) for each
   candidate concept. Batch 30 dropped 13 candidates as
   duplicates/adjacent; that filter is load-bearing.
3. Spawn **2 parallel subagents** to read the target file(s) + propose
   rules (pattern, bounded fix-space, category, rationale, source
   quote). **Re-verify every quoted line against the real file** —
   Batch 31's full-quote re-verification caught drift before it landed.
4. Add corpus entries + extend the standing syn-parse gate
   (`batchNN_pseudocode_parses`).
5. **GOAT** — a batch is NOT done at corpus-add time:
   - G1: parse/syn check (all new rules)
   - G2: lib tests pass
   - G3: no-regression — clippy 0 in BOTH feature states + CI guard
   - rust_perf rules: the Pareto bench (`rust_perf_bench` feature) +
     `context_notes` annotations on regime-fragile/Mixed verdicts;
     demote/annotate losers. Never promote a perf gain on a wrong result.
6. Write the Research note (`.research/NNN_*.md` — next free number),
   bump the corpus count in `AGENTS.md`, update `.docs/11_domains/`.
7. Commit `feat: batch NN: ...` + push. Update the mining queue snapshot
   in THIS file (same commit).

## Protocol B — heal dry-run on workspace repos (queue items 3 + 5)

```bash
cd /Users/katopz/git/riir-clippy
cargo run --features latent_retrieval --bin cargo-heal -- --suggest <target-repo>/src/
cargo run --features latent_retrieval --bin cargo-heal -- --fix <target-repo>/src/          # preview
cargo run --features latent_retrieval --bin cargo-heal -- --fix --write <target-repo>/src/  # apply
```

- Validate in the TARGET repo: `cargo clippy --all-targets` +
  `cargo test --lib`. Use `CARGO_TARGET_DIR=/tmp/<repo>-heal` if the
  repo's target dir is locked by a sibling build; clean it up after.
- **Feature-aware validation (the bevy_shared lesson, 2026-08-15):** a
  cfg-gated edit site is INVISIBLE to default `cargo check`/`--all-targets`
  when the gate is a non-test feature — an E0432 sat green in
  `riir-game-sdk/src/backend/bevy_shared.rs` until validated with
  `--features backend_bevy`. After a heal, compile the edited files under
  the features that actually include them (grep the `#[cfg(...)]` on the
  enclosing `mod`); a passing default check proves nothing about gated
  code. Same class as the count-assertion rule below.
- cfg-gated test files: assert COUNTS, not exit codes — a green run can
  mean "ran nothing" (riir-chain lesson).
- **Wrong suggestion → fix the proposer at root cause** (rule/pattern in
  `src/domains/*/proposers/`), add a fixture regression test, re-run.
  NEVER suppress the lint, NEVER hand-edit past it without a fixture —
  the healer must adapt each iteration, not accumulate suppressed lints.
- Commit separately: `fix: heal <repo> ...` (target repo) and
  `fix: proposer ...` (riir-clippy).

## Discipline (non-negotiable)

- **Commit + push per unit** — `feat:`/`fix:`/`docs:` prefixes; stay on
  `develop` (`main` for riir-train); rebase non-interactive only.
- **Numbering**: read `.highwater`, +1, AND re-scan the folder at WRITE
  time — concurrent sessions dual-allocated 675 AND 677 in one night.
- **Sibling safety**: never `git stash` sibling WIP; commit only your own
  hunks; isolate builds under `/tmp`; before blaming CI on riir-clippy,
  check whether `src/domains/*/bench.rs` WIP is the sibling's.
- **GPU exclusivity before benches/gates**: `ps aux | grep -E
  "cargo|riir|katgpt" | grep -v grep` — no second compute consumer
  (Unity Editor + Zed are exempt). Bench 649: 12.3× correctness swing
  under contention.
- Don't fix unrelated bugs — note them in the final summary.
- Low confidence → STOP, emit the 4-part `## Summary` (the auto_prompt
  harness depends on that exact format), let the user re-aim.
- **All queues empty → idle report** (what was checked, all clean). Do
  NOT invent work.

## Skill homes (canonical-in-repo + global symlink — never copies)

| Skill | Canonical home | Global |
|---|---|---|
| `riir-clippy` (this) | `riir-clippy/.agents/skills/riir-clippy/` | symlink ✓ |
| `boundary-guard` | `katgpt-rs/.agents/skills/boundary-guard/` | symlink ✓ |
| `substrate-first` | `katgpt-rs/.agents/skills/substrate-first/` | symlink ✓ |
| `doc-sync`, `rust-optimize` | global-only real dirs (no repo home yet — pending) | — |
| `feature-gate-audit`, `goat-audit`, `proposal`, `research` | `katgpt-rs/.agents/skills/` (tracked) | no symlink yet |

Copies drift — the substrate-first global duplicate (identical today,
diff-verified 2026-08-15) was removed for exactly this reason. When
creating a new skill: canonical home = the repo that owns the workflow;
global = symlink only.

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
- **NEXT mining target**: `ls` the riir-gpu tree for the next unmined
  cluster (~83 of ~200 files mined). Candidates:
  `kimi_k3_gpu_backward_batched.rs` (1,394) +
  `kimi_k3_gpu_backward_sequence.rs` (1,328) — the 2 backward siblings
  (NOTE: the batched twin shares the 2 wrong-math bugs + the broken LCG per
  Issue 693 — its mining value is now lower; the sequence file is the
  PRODUCTION backward caller path); `gemma2_d2f*` (mod 1,579 + _sc 800),
  `weaver_gpu*` (dflash 576 + corrector 1,449), `kvarn/`, `hypernet/`,
  `maxsim/`, `weight_buffer_cache.rs` (the write substrate — flagged as
  unmined by Batch 40's lane A). Pick 1-2 files, ~6-10 rules (dedup pressure
  high), Batch 41 (Research 045).
  - Validation pattern (proven T4 + 32a + 32b-1 + 32c + 36 + 38): corpus files are NOT
    sibling WIP — edit + run GOAT in the main checkout with
    `CARGO_TARGET_DIR=/tmp/riir-clippy-bNN`; `--features kernel_heal`
    for the corpus suite (318 at 34, 321 at 35, 324 at 36, 327 at 37,
    330 at 38, 333 at 39, 336 at 40), default for
    the 201 suite.
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
    comment + the file-level `//!` header batch block. All in sync at 310 (A 64, D 54, H 14, N 18 after Batch 40).
  - **Landing lesson (32b-1, self-caught)**: after a large multi-entry
    corpus edit, `awk`-scan for `rule_id:` lines NOT preceded by a
    standalone `KernelRule {` BEFORE the first compile — JSON `\n`
    escaping in comment blocks can land as literal 2-char `\n` text,
    swallowing the `KernelRule {` opener into the comment (9/10 entries
    broken; rustc's `vec![` unclosed-delimiter error points at the symptom,
    not the cause).
- **BLOCKED**: `ternary_deltanet_gpu_forward.rs` (+ `_cudarc.rs` — sibling
  WIP STILL ACTIVE as of 2026-08-15 evening: qv_lora.rs + cudarc_kernels/
  + riir-gpu/lib.rs + ternary_deltanet_gpu_forward_cudarc.rs all dirty)
  — mine only after their WIP lands.
- **Progress**: riir-gpu ~83 of ~200 files mined. Corpus: 310 rules.
  edge_lora module FULLY MINED (32a + 32b-1 + 32c). spectralquant/
  FULLY MINED (15 + 33). gemma4_cubecl_train/ DONE (34).
  gemma2_cubecl_train/ FULLY MINED (35: mod.rs + attention_gpu.rs;
  36: resident.rs). gemma4_q4k_train/ FULLY MINED (37: mod.rs +
  weight_buffers.rs; 38: forward_batched.rs + backward.rs).
  kimi_k3_gpu_forward.rs DONE (39); kimi_k3_gpu_backward.rs DONE (40 —
  the 2 batched/sequence siblings remain).
- **Also open (riir-clippy issues)**: 014 (rust_perf v4, 12 tasks), 016
  (clippy structural expansion, 6), 017 (retrieval precision at scale,
  5), 018 (self_evolve promotion, 6). 015 = sibling's ACTIVE WIP
  (fix_verify mode — do not touch `src/bin/heal.rs`, `src/fix_verify.rs`,
  `src/lib.rs`, `Cargo.toml`, proposers mod.rs/vec_with_capacity, bench.rs,
  clippy proposers mod.rs).

## Manual invocation

In Zed: `/riir-clippy` in the agent panel, or ask "run the riir-clippy
skill". From a terminal, the underlying healer is `cargo-heal`
(see Protocol B).
