# Research 042 — riir-gpu `gemma4_q4k_train/` Batch 38: the batched forward + both backwards

**Date:** 2026-08-15
**Batch:** 38 (Batch 37's deferred remainder — the Q4_K cluster is now FULLY MINED)
**Files:** `crates/riir-gpu/src/gemma4_q4k_train/forward_batched.rs` (1,077) + `backward.rs` (941)
**Corpus:** 290 → **294** (A 52→55, D 50→51)
**Bug yield:** riir-ai Issue 691

## 1. What this cluster is

- `forward_batched.rs` — the layer-major batched training forward (Issue 436): ALL
  positions through one layer at a time via `dispatch_gemv_q4k_batched` — the
  43,008 → 337 dispatch fix (127×). CPU element-wise (rayon-parallel, `RAYON_FWD`
  toggle), GPU batched GEMVs, CPU attention. `BatchedForwardScratch` = 16 flat
  `[block_size × dim]` buffers pre-sized at construction.
- `backward.rs` — TWO variants: the per-position `gemma4_backward_q4k_gpu`
  (8 transposed GEMV sites/layer, per-position) and the batched
  `gemma4_backward_q4k_gpu_batched` (~337 dispatches; `RefCell` flat scratch
  borrowed once per call; `valid_positions` compaction in Phase 1).

Dedup pressure was the predicted story and it held: this is the FIFTH mining
pass over a sibling Gemma training cluster (B34/B35/B36/B37), and most of the
file's structure is confirmed APPLICATIONS of existing rules (list in §4).

## 2. The 4 landed rules

1. **`env-flag-resolved-once-not-per-dispatch` (D)** — `std::env::var` on the
   hot path takes the global env lock + heap-allocates a String per lookup.
   Instances: `RAYON_FWD` at forward L203 (per step) AND L385 (per layer —
   48×/step, the same var read twice at different scopes), `FUSED_RMSNORM` at
   L382 (per layer), `GEMV_FORCE_TILED` at `gemv_autotune.rs` L121-131 —
   **in front of the papaya lock-free cache hit** (the env lock poisons the
   exact property the cache was chosen for), `OURO_*` at `kimi_k3_gpu_forward.rs`
   L1861-1948 (per token). The correct live-toggle form exists in-tree:
   `gemma2_cubecl/mod.rs` L1353-1358 orders the cheap `layer == 0 && pos <= 1`
   guard BEFORE the env read. Distinct from the B36-F5 falsification note
   (that killed "env var selects path A/B" as rule CONTENT; this rule's
   content is resolution timing + per-dispatch cost).
2. **`flat-activation-slots-as-readback-target` (A)** — nested `[pos] → Vec<f32>`
   activation slots (`Gemma4TrainingActivations`, shared with the CPU reference
   + backward) force a scratch + scatter two-copy tail after every batched
   readback: the dispatch's readback lands in a flat caller slice (`mod.rs`
   L753-755), then ~15 per-position save sweeps per layer scatter it into the
   nested slots. Flattening the container to `[seq × dim]` makes the slot the
   readback target — the container layout IS the copy count. Distinct from
   `preallocated-activation-slots-inplace-save` (which pre-sizes NESTED slots
   and keeps the per-position memcpy — nesting retained = scatter retained).
3. **`transient-spike-pins-persistent-scratch` (A)** — `bw_scratch_in` is a
   persistent `RefCell<Vec<f32>>` (mod.rs L377, `Vec::new()` at L426, never
   shrunk); backward.rs L409's Phase-1-ONLY `resize(batch × vocab)` ≈ 134 MB
   (vocab 262144) pins its capacity ~8× above the Phase-2 recurring max
   (L557: `batch × mlp × 2` ≈ 16 MB). Grow-only is right for RECURRING sizes
   and wrong for a non-recurring spike: transient oversized phases use a
   local buffer dropped at phase end.
4. **`transposed-consumption-accumulate-not-gather` (A)** — the backward's
   LoRA z-pass (`backward.rs` L923-930, dup at `mod.rs` L695-701) computes
   `z = Bᵀ·g` with k-outer column gather at stride `rank × 4B` (up to 16×
   read amplification) while the forward (`forward_batched.rs` L1013-1018)
   streams the SAME row-major B sequentially — the forward/backward asymmetry
   on the same buffer is the tell. Fix: i-outer row accumulation, same flops,
   sequential B. Caveat recorded in the rule: summation order changes — fine
   for grads, NOT for bit-identity gates.

## 3. The coordinator kill — backward-subagent R3 (zero-gradient-position elision)

The backward subagent proposed `zero-gradient-positions-elide-position-local-
backward` (D): Phase 1 guards the support set (`d_logits[p].is_empty()`), but
Phase 2's loops run ALL positions — elide the position-local half for prompt
positions, keep attention + wk/wv full-sequence. Magnitude claim: "~50% of
the (A)-stage work at n_in=seq/2".

**KILLED by coordinator math — the elision is INCORRECT below the topmost
layer.** Trace: at the topmost layer, prompt positions' `g_x_at_layer_out`
is exactly zero (Phase 1 skip) → its (A)-stage elision is safe. But attention
backward at that SAME layer re-introduces non-zero gradient at prompt
positions: **K/V-side gradients don't depend on `g_attn_out[p]` — they depend
on OTHER positions' `g_attn_out`** (support queries, at pred_pos ≥ any prompt
position, attend to prompt keys/values). That flows through wk/wv into
`g_normed_attn[p]` → `g_x_layer_in[p]` ≠ 0 → at every LOWER layer, the
position-local chain sees non-zero incoming gradient at prompt positions.
Eliding it as written would silently drop gradient contributions → wrong
LoRA grads for Down/Gate/Up/Wo at 47 of 48 layers. The narrow true kernel:
only the topmost layer's pre-attention chain + the head can skip — 1/48 of
the claimed win, already adjacent to `lm-head-loss-positions-only` (B37).

**Cross-check flag for Issue 687 H3** (gemma2_cubecl_train, filed by B34):
its claim "attention backward with zero `g_attn_out` rows contributes zero …
roughly halves Phase 2 cost" has the SAME flaw — it treats each position's
gradient in isolation and misses the cross-position K/V coupling. H3's
"skipping is mathematically safe" phrasing survives (skipping TRULY-zero
rows is safe); the identified skip set is ~1/48 of what it claims. Needs
re-review by whoever owns 687. (Batch 38 did NOT modify 687.)

## 4. Dedup kills + confirmed applications (the file IS the corpus, applied)

Killed as rules (all with evidence):
- **Backward RoPE `powf` table rebuild per layer** (L250-253, L738-741 — 48
  transcendental table builds/step while `rope_freq_tables[rope_table_idx]`
  sits cached, bit-identical incl. the Full-layer prefix) — third strike:
  B17 dropped cos/sin-table rules, B36-F4 re-cited `RopeCosSinCache`, B37-F1
  recorded the cache. HAZARD in Issue 691, not a rule.
- **Sibling-batch region packing** (inputs packed as offset regions L750-753
  vs 4 fresh output Vecs L583/L798-800) — composition of `superset-sized-
  scratch` + `slot-offset-partial-buffer-write`; hazard with counts.
- **resize churn** (shrink-regrow re-zero-fills, L409→493→557→643→750) —
  `superset-sized-scratch` unapplied.
- **Rayon attention direct-write into disjoint `attn_out` rows** (L642-687,
  eliminating the sequential path's `attn_pos_out` intermediary) — a SECOND
  in-file instance of `materialized-fast-path-bulk-slice-accessor` (B37's
  source is L208-227 of this same file) + `grid-over-output-thread-private-
  sweep` (B33). Strengthens B37.
- **LoRA-delta add-pass fusion** (7 separate `buf[i] += delta[i]` sweeps) —
  `fuse-residual-via-in-place-output-array` (B19) + rust_perf loop-fusion.
- **RefCell-scratch-under-`&self`** — ambient in 3 existing rules'
  opt_patterns; the enabler, not the insight.
- **Phase-2-processing-zero-grad-positions as a "hazard"** — see §3; the
  687-class claim itself is what needed killing.

Confirmed applications (cite-strengthens, no action): `op-major-backward-
loop-inversion` (B34 — the batched backward's structure IS it), `lm-head-
loss-positions-only` (B37 — backward Phase 1's `valid_positions` compaction
L405-439 is the rule applied in the backward; also the batch*vocab resize
corollary), `sibling-dispatches-before-any-readback` (B36 — a MEASURED
non-application: the `dispatch_gemv_q4k_batched_noread` + deferred-readback
experiment was run and Bench 437 found it ~16% SLOWER on Metal
memory-pressure; B29's `defer-readback-gated-by-live-output-size` gate
correctly predicts the loss — the 3 QKV outputs ≈ 4.7 MB simultaneously
live is an order above the gate's "large" regime), `host-cache-immutable-
device-weights` (B26), `first-readback-pays-queue-drain` (B29).

## 5. Bug yield — riir-ai Issue 691 (headline: the backward is UNREACHED)

1. **P2 — both backward variants have ZERO in-tree callers.** Grep across
   the workspace: only definitions + doc comments. issue436's G1 gate covers
   the forward only; the 941-LOC backward's math is unpinned (which is how
   hazard 2 survived). Fix: port the issue436 G1 pattern (batched backward
   vs `gemma4_train::backward` on a tiny config).
2. **P3 — dead `x_normed` compute in BOTH Phase 1s** (L93-100 per-position
   incl. a fresh n-Vec per valid position; L431-438 batched + the equally
   dead L410 resize): `rmsnorm_backward` takes `x_fnorm`, never x_normed.
   A forward-final-norm copy-paste leftover. Trivially deletable.
3. **P3 — 48× per-layer RoPE `powf` table rebuilds** (both variants) while
   the trainer caches bit-identical tables; one-line fix
   (`&self.rope_freq_tables[idx][..half_rot]`).
4. **P3 — clone storm**: `g_x_at_layer_out[p].clone()` per position per
   layer (L129/L495) is the row's last read; the container is wholesale-
   replaced (L338/L873) → `mem::take`. ~94 MB/step memcpy at n_embd=3840.
5. **P3 — 4 fresh batch×n output Vecs/layer** (L583, L798-800) into a
   provably-free `scratch_out` (freed at L705-706) ≈ 380 MB/step churn;
   plus the ~240K allocs/step helper-returns-Vec class
   (`rmsnorm_backward`, `gegelu_backward`, per-position temps) — the
   `caller-owned-split-scratch` rule unapplied.
6. **P3 — `hidden` uploaded 5× per layer** (Q/K/V + gate/up all pass
   `scratch.hidden`; `create_from_slice` per call at mod.rs L739) ≈ 380
   MB/step redundant upload. Nuance: Bench 437 measured the FULL
   noread+deferred combo as slower; upload-sharing ALONE (keeping immediate
   readbacks) was never tested.
7. **P3 — env flags per layer/per dispatch** (rule 1's instances).
8. **P3 — `bw_scratch_in` pinned at 134 MB** by the Phase-1-only resize
   (rule 3's instance; ~117 MB dead RSS).
9. **P3 — strided LoRA z-gather** at 2 sites (rule 4's instances; needs
   the Issue-691-#1 G1 gate first since summation order changes).
10. **P4** — dead `attn_scores` scratch field; stale comment L934-937 +
    `normed_atcn` typo L792; prose "43,008" vs table "43,136" (the table
    counts the 128 per-position lm_head dispatches the prose excludes);
    explicit `drop(scratch_*)` ceremony L876-879.

Suggested fix order: (1) the G1 gate (unlocks everything — the file is
unreached, fixes could regress silently); (2) the P4s + dead-x_normed
deletions; (3) one-liners (rope table, mem::take); (4) env-flag resolution
at construction; (5) scratch sizing (local Phase-1 buffer + superset-size
Phase-2) + output region packing; (6) the z-gather interchange (post-G1);
(7) the upload-sharing experiment (Bench 437 follow-up: upload-once variant
with immediate readbacks).

## 6. Process notes

- **The `ensure_ascii=False` lesson re-learned the hard way**: the first
  generator run used bare `json.dumps` → em-dashes became `\u2014` in Rust
  string literals → 33 compile errors (invalid escape; Rust wants
  `\u{2014}`). This is EXACTLY the 32c lesson, forgotten. Clean abort:
  `git restore` of the two files, regenerate with `ensure_ascii=False`,
  first compile clean. The lesson is now twice-observed — treat
  `ensure_ascii=False` as non-optional in the generator template.
- The "expect N occurrences" self-check needs to count ALL consumers
  (entry + category + rules_present + concepts + parses = 5, not 3).
- Post-write stale-count verification must whitelist line-number refs
  (`L290 (q_post_rope)`, `L288-290`) — over-eager asserts abort after a
  successful write and waste a cycle.
- A 17th count-pin site surfaced: the stale "282-rule corpus" comment in
  `kernel_categories_cover_all_corpus_rules_exhaustively` (B37 missed it;
  now 294). The count-pin list in the skill is updated.
- Clippy doc_markdown gotchas in `//!` batch blocks continue to bite:
  `Q4_K`, `powf`, `sibling-dispatches` (rule-id-shaped words), `RoPE`,
  `LoRA`, `opt_pattern` all needed backticks — same class as B36/B37.

## 7. Mining progress after Batch 38

riir-gpu ~81 of ~200 files mined. Corpus 294. `gemma4_q4k_train/` FULLY
MINED (37: mod.rs + weight_buffers.rs; 38: forward_batched.rs + backward.rs).
Next candidates: `gemma2_d2f*`, `kimi_k3_gpu_*` (4 files — the env-flag
instances found this batch make it partially surveyed), `weaver_gpu*`
(4 files), `kvarn/`, `hypernet/`, `maxsim/`. `ternary_deltanet_gpu_forward.rs`
+ `_cudarc.rs` still BLOCKED on sibling WIP.
