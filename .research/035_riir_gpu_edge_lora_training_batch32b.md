# Research 035 — riir-gpu Batch 32b: the edge_lora training half (topology_trainer + chain_commit + sigmoid_gate + mod)

**Date:** 2026-08-15
**Batch:** 32b (corpus 228 → **238**)
**Sources:** `riir-ai/crates/riir-gpu/src/edge_lora/topology_trainer.rs` (1468 lines) + `chain_commit.rs` (394) + `sigmoid_gate.rs` (394) + `mod.rs` (249) — the topology-evolution trainer + chain commitment + per-edge additive gating. Two parallel mining subagents (A: topology_trainer, B: chain_commit+sigmoid_gate+mod) + synthesizer re-verification of every load-bearing quote (all quotes verified exact against the files; one subagent quote omission caught and corrected during verification).

**Split note:** the snapshot's suggested 32b-1/32b-2 split — this is **32b-1** (topology_trainer + training remnants). 32b-2 remains: the self-play/arena/consolidation half (`sleep_consolidation.rs` 615L, `arena_eval.rs` 604L, `{go,bomber,fft}_self_play.rs` 730+521+506L, `cross_game_{edge,real}.rs` 504+224L) + `kernels/` Rust wrappers.

## The headline: a false-premise aliasing comment motivating a hot-loop allocation storm

`forward_with_vertex` + `forward_cached` (topology_trainer.rs L627-639 + L758-769) allocate `let mut tmp = vec![0.0f32; hd];` **per node per layer per training step** — and the temp is unnecessary. The comment claims "Vertex reads from next_node_hidden, writes back into the same slot (in-place). We use a small swap buffer to avoid aliasing" — but the read is from `scratch.next_node_hidden` and the write to `scratch.node_hidden`: **different buffers**. The false premise motivated the temp; the fix is a direct write (zero temps, zero extra copies). Compounding it: `forward_cached`'s edge loop pays `let edge_out_owned: Vec<f32> = edge_out.to_vec();` per edge per step (L744) — a borrow-checker workaround (the comment says so outright: "The cleanest fix is to split the sig copy BEFORE the aggregation, but the borrow checker can't see that"). And `backward` fills a `grad_h_src` staging buffer per edge (L998-1039) that a direct `grad_node[src_slot + j] +=` eliminates. Together: three per-iteration allocation/pass sites on the hottest path of the trainer — plus the **O(E·L²) routing re-derivation**: the `layer_of` closure (an O(n_layers) linear probe per edge) is rebuilt and re-scanned in ALL THREE hot loops (L569-579, L680-689, L830-839), while `edge_source_layers()` (L1063-1079) already computes the exact mapping that only the cold LR path consumes. Filed as **riir-ai Issue 684** together with: the dead `clear()` lanes (`gates_scratch`/`edge_outputs` zeroed every call, never read by any production path — the gated composition is unwired Phase 2 per mod.rs L7-9), the redundant double-memset (`scratch.clear()` then the consumer re-zeroes its live window), the triple-BLAKE3 load path (`deserialize` verifies → `edges_from_bundle` re-verifies → per-artifact `recompute_hash` — up to 3 full payload passes per load), the 7-site `sigmoid` duplication across the crate (grep-verified), the `recipe_to_topology` discarded-error (`let _ = e;` — every topology failure mislabeled as JSON), and the silent u32→u16 narrowing on artifact import.

## Rules landed (10)

| Rule | Cat | Source | Concept |
|---|---|---|---|
| `static-classification-hoist-to-routing-table` | D | topology_trainer.rs L569-604 + L1046-1056 | Immutable topology re-classified per call via an O(L) scan per edge + a fresh Vec per call; classify ONCE at construction into CSR-style buckets so each transition visits only its active edges. Notably `edge_source_layers()` already computed the mapping — only a cold path consumed it. |
| `sigmoid-deriv-recovered-from-output` | A | topology_trainer.rs L966-987 + L171-182 | σ'(x) = σ(x)·(1−σ(x)) makes the derivative a pure function of the OUTPUT: cache only the post-activation, recover the derivative algebraically incl. through a scale factor (`scale·σ' = sig − sig²/scale`, degenerate guarded). Halves activation-cache traffic. |
| `fuse-weight-grad-input-grad-single-matrix-visit` | C | topology_trainer.rs L932-963 | The weight-grad outer product WRITES `grad_B` (never reads B) while the input-grad transposed matvec READS B — one visit per B row serves both roles, loading sig[]/go once for both. |
| `needless-swap-temp-hoist-to-scratch` | A | topology_trainer.rs L627-639 + L758-769 | A 'defensive' per-iteration `vec![0.0; n]` motivated by a FALSE aliasing premise (read buffer ≠ write buffer): write the destination directly; genuine ping-pong temps are loop-carried state in the persistent scratch. |
| `scoped-init-at-consume-site` | A | topology_trainer.rs L400-421 + L581-594 | Blanket whole-scratch `clear()` memsets dead lanes + buffers the consumer re-zeroes anyway: only ACCUMULATION targets need pre-zeroing, scoped to the live window at the consume site. O(total scratch) → O(active window). |
| `monomorphize-per-element-trait-calls` | B | topology_trainer.rs L558-563 + L812-819 | Per-node `&dyn` vtable indirection in the hot loop → generic `+ ?Sized` (the same file's backward is ALREADY generic — the inconsistent pair was the evidence). `?Sized` keeps `&dyn` callers compiling: a strict generalization. |
| `additive-sigmoid-gates-not-softmax-routing` | E | sigmoid_gate.rs L3-16 + L70-83 + L141-147 | Softmax routing forces a cross-edge reduction barrier (max+sum) before ANY gate is final; per-edge independent sigmoid gates compose additively — embarrassingly parallel, multiple edges simultaneously near 1.0 by design. The house sigmoid-not-softmax rule, as a kernel-corpus rule. |
| `commit-over-maintained-item-digests` | B | chain_commit.rs L66-76 + L83-92 | Container commitment digests the maintained 32-byte per-item digests: O(items), not O(total payload bytes) per commit; full payload hashing only at trust boundaries. |
| `verify-once-at-trust-boundary` | D | chain_commit.rs L156-166 + L236-243 | The load path re-verifies at every layer (deserialize → `edges_from_bundle` → per-artifact re-hash): verify ONCE where untrusted bytes become trusted state, return a capability type (`Verified<Bundle>`) — the type carries the proof. |
| `derive-shape-from-payload-not-cached-field` | A | sigmoid_gate.rs L31-38 + L57-61 + L70-77 | No cached `in_dim` field that can drift: `gate_vectors.first().map(|v| v.len())` at the API boundary is the single source of shape truth; the merkle_root-class desync bug becomes structurally impossible. |

Category deltas: A 35→39, B 37→39, C 24→25, D 33→35, E 13→14. Total **238** across 15 categories.

## Candidates dropped (dedup, with the covering rule)

- **Per-edge `edge_out.to_vec()` borrow-release workaround** (topology_trainer.rs L744) — `caller-owned-split-scratch` (32a, same module family); also fixable by reordering the sig copy after the aggregation under NLL. Recorded as a bug instead.
- **Worst-case scratch sizing (`max_width`/`max_rank`/`max_in_dim`)** (L324-397) — `superset-sized-scratch-across-variants` (32a).
- **Manual 8-wide unrolled inner loops** (L939-963, L1004-1028) — `unroll-fixed-inner`.
- **Borrowed-view return of `&cache.output[..hd]`** (L660-661, L776-777) — the view-return half of `caller-owned-split-scratch`.
- **Forward activation caching (`forward_cached`)** — `activation-recompute-budget`.
- **Per-layer LR multipliers** (`sgd_step_per_layer`, L271-302) — `per-layer-grad-norm-lr-equalization` (32a).
- **BLAKE3 `recompute_hash()` after mutation** (L267, L300) — the naive form that `incremental-multi-part-hash-no-concat` (32a) replaces.
- **`Vec<Vec<f32>>` per-edge caches → flat arena + offsets** (`ForwardCache`, `TopologyScratch.edge_scratch`, `EdgeGradients`) — `ragged-pack-offset-table-fused-dispatch` core; the persistent-layout framing is a thin variant.
- **Struct field ordering for padding** (L426-428) — ordinary layout hygiene, not rule-grade.
- **8-lane chunked dot/axpy in sigmoid_gate** (L144-159, L181-195) — `unroll-fixed-inner` (subagent B).
- **Caller-provided `gates_scratch`** (L99-105) — `caller-owned-split-scratch` thin variant (subagent B).
- **Gate-scalar pre-pass feeding the bandwidth phase** (L125-126) — `split-reduction-scalar-prepass` + `hoist-group-invariant-scale-out-of-word-loop` (subagent B).
- **Empty-workset early exit returning identity output** (L109-114) — `predicate-gated-skip-empty-workset-step` (subagent B).
- **Early-return shape validation at API boundary** (L71-77 + L177) — `launch-unchecked-caller-owned-size-contract` (subagent B).
- **`out += g * eo` absorbing broadcast scale** (L141-159) — `inplace-gate-scale-at-funnel-bottleneck` (32a) (subagent B).
- **`Box<[f32]>` exact-size storage** — too ordinary; nearest `block-contiguous-weight-layout` (subagent B).
- **`Vec<Box<[f32]>>` slice-of-slices instead of flat+ offsets** — the regime INVERSE of `ragged-pack-offset-table-fused-dispatch` (packing pays only when a fused dispatch consumes it) — a caveat, not a rule (subagent B).
- **Magic/version/length header rejection** — leaf-crate concern, below the bar (subagent B).
- **Deterministic serialization** — correctness property, not a perf transformation (subagent B).
- **u32-on-wire / u16-in-memory split** — wire-stability concern, perf delta is noise (subagent B).

## GOAT gate

- **G1 (parses):** `batch32b_pseudocode_parses` — 10/10 rules' ref_shape + opt_pattern syn-parse. PASS.
- **G2 (tests):** kernel_opt lib suite **309 tests PASS** (was 306; +3: `batch32b_rules_present`, `batch32b_concepts_covered`, `batch32b_pseudocode_parses`); default suite 201 PASS.
- **G3 (no-regression):** clippy clean in BOTH compiled feature states (default + kernel_opt; fixed 41 doc warnings in the new batch narrative before commit — 40 `doc list item without indentation` caused by continuation lines starting with `+ ` (a CommonMark list marker!) + 1 missing-backticks). `--all-features` still broken by the SIBLING's uncommitted WIP (`rust_perf/bench.rs:335` `alloc_axis`) — pre-existing, verified at 32a, not this batch's failure.
- **G4:** unchanged (no hot-path additions — corpus is static data).
- **G5 (corpus):** 238 flat → G5-drafter reduction maintained.

**Landing incident (self-caught, same class as 32a's typo):** the big corpus edit initially failed to compile — 9 of the 10 `KernelRule {` opening lines were swallowed into the preceding `// Source:` comment lines because the edit's JSON newlines inside comment blocks landed as literal `\n` text (2-char sequence), merging the comment with the next line. rustc's "unclosed delimiter `vec![`" pointed at the symptom; the diagnostic ladder was: quote-count per line (all even — strings fine) → entry-structure scan (9 rule_ids with no `KernelRule {` open line) → tail-bytes of the merged comment (literal `\n        KernelRule {`). Fixed with a targeted perl split (only comment lines, only the literal `\n` before `KernelRule {`). Lesson for the next batch: after a large multi-entry corpus edit, `awk`-scan for `rule_id:` lines not preceded by a standalone `KernelRule {` BEFORE the first compile — it localizes this failure class in one pass.

## State

`riir-gpu` mined: ~62 of ~200 files. Next: **Batch 32b-2** — the self-play/arena/consolidation half (`sleep_consolidation.rs` 615L, `arena_eval.rs` 604L, `{go,bomber,fft}_self_play.rs` 730+521+506L, `cross_game_{edge,real}.rs` 504+224L) + `kernels/` Rust wrappers. Note: the three `sigmoid_outcome_reward` fns are duplicated across the self-play files (part of the 7-site sigmoid family in Issue 684) — dedup candidate, not a rule candidate. `ternary_deltanet_gpu_forward.rs` still BLOCKED on sibling WIP.
