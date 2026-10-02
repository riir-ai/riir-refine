# Research 056 — riir-gpu Batch 50: small leaves + DFlash twins (zero-rule batch, hazard-rich)

**Date:** 2026-08-16
**Batch:** 50 (file-mining kernel half)
**Lanes:** A = small unmined leaves (game_mux.rs 359, domain_latent.rs 275,
context.rs 283, cpu_reference.rs 390, depth_tier.rs 327, elf.rs 578);
B = standalone forward twins (forward_dflash.rs 1,345,
forward_flashprefill.rs 497 + 5 WGSL kernels).
**Corpus:** 344 → **344 (zero rules)** — an honest verdict at maximum dedup
pressure, per the queue's own prediction ("dedup pressure very high after 47
batches").
**Bug yield:** filed as riir-ai Issues **708** (dflash non-functional forward +
stale-uniform wrong-math classes + flashprefill doc-lies, 20 hazards) and
**709** (game_mux demux math, cpu_reference f64/lora_b doc-lies, elf
mislabeled norm + silent SDAR no-op, 11 hazards).

## 1. Why zero rules is the correct verdict

Both lanes were read in full by independent subagents with the complete
344-rule dedup list, kernel-side verification of every referenced WGSL file
(the Batch 33 lesson), and consumer tracing across riir-ai + riir-train. The
coordinator re-verified every load-bearing hazard claim against source before
filing (9/9 verified: both dflash no-op stubs, both stale-uniform structures,
the f64 doc-lie, the lora_b.wgsl base-GEMV mismatch, the elf L2-vs-RMSNorm
mislabel, the game_mux softmax-then-dot demux, the SDAR zero-loss stub).

The drop tables (18 rows Lane B + 10 rows Lane A) show a consistent pattern:
**every candidate concept was either (a) a positive instance of an existing
rule** (context.rs IS B21; domain_latent.rs IS B33; elf's sigmoid IS B32a;
flashprefill's mean_k staging IS B35 applied correctly) **or (b) an
anti-instance of an existing rule** (forward_dflash's stale uniforms are the
live ref_shape of B26; the two-pass softmax is the ref_shape of popcorn
`online-softmax-streaming`; the one-thread-per-row reductions are B44's
ref_shape). Encoding instances of existing rules as new rules would dilute the
corpus — the exact failure mode the dedup gate exists to prevent.

## 2. Lane B headline — forward_dflash is an unfinished MVP presenting as a forward pass

The single most valuable finding of the batch: `dflash_training` (opt-in) ships
a "Bidirectional forward pass" whose module doc claims working
target-conditioned attention, while the code contains:

1. **Two no-op stubs** — `dispatch_rmsnorm_manual` (L1027-1047) writes a
   uniform and dispatches nothing; `dispatch_attention_bidirectional_manual`
   (L1212-1225) returns Ok immediately. In-source comments admit both ("This
   is a placeholder", "This is a stub for the MVP").
2. **Two live stale-uniform wrong-math classes** — `uniform_qkv.pos` written
   per call via `queue.write_buffer` inside the `for layer { for pos }` loop
   with ONE submit at the end → every K/V-fill dispatch reads `pos =
   seq_len-1`; and ONE shared `uniform_matmul` across wo/w1/w2/lm_head →
   every matmul except the last-written reads the wrong dims. This is the
   Batch-27 `backward_dflash` bug class, forward twin — the SAME file family
   has now produced the same bug twice, which is itself evidence for B26 being
   load-bearing corpus wisdom.
3. **Structurally masked tests** — `backward_dflash` tests assert
   "nonzero + finite" on gradients computed from garbage logits; they pass
   green on a non-functional forward.

`forward_flashprefill` is the honest contrast: orchestration is B26/B27-clean
at its production call site, but the module doc's "online softmax" claim is a
two-pass full-recompute, and the "matches metal()" config claim is false (the
katgpt-core twin is identical; the REAL divergences — tail_window hardcoded
to 0, score_reduction/budget_adaptation dropped — are undocumented).

## 3. Lane A headline — doc-lies on the validation substrate

`cpu_reference.rs` exists to be the GOAT-parity ground truth, and it ships two
doc-lies that weaken exactly that purpose: the f64-accumulation claim (all
accumulators are f32) and the lora_forward "LoRA delta only, matches the
two-dispatch GPU pattern" claim (`lora_b.wgsl` computes the BASE GEMV too —
a parity test would differ by the entire base term). Plus the elf.rs
L2-normalize-under-the-name-RMSNorm collision with cpu_reference's true
RMSNorm in the same crate.

The game_mux finding is the cross-contamination class: a demux whose math
(softmax after the weighted sum + non-negative hash distributions →
`hᵢ·hⱼ > 0` for all pairs) cannot recover per-state values, validated only by
`is_finite()` tests, with a live consumer in riir-games
(`game_mux_bridge.rs`). Encoding the mux/demux as a corpus rule was REJECTED
on this basis — it would have put a broken mechanism in the corpus as wisdom
(the Batch 33 rejected-seed precedent).

## 4. Drop tables (condensed)

### Lane B (18 dropped)
| Candidate | Killed by |
|---|---|
| Two-pass softmax w/ full dot recompute | `online-softmax-streaming` (popcorn B2) |
| Per-pos `queue.write_buffer` uniforms during encoder build | `per-slot-uniform-buffer-write-ordering` (B26) |
| 15 identical bind groups per layer | `prebuilt-bind-groups-fixed-buffer-combos` (B26) |
| Concat-KV written, noise half copied back | `consumer-shaped-output-split` (B28) |
| One 64B superset uniform shared by 4 stages | `pack-scalar-params-into-combined-buffer` (B29) + `identical-uniforms-share-single-slot` (B27) |
| block_score → block_select same-granularity round-trip | `epilogue-at-register-write-phase` (B29) |
| −1 tail padding never read | too thin + `scoped-init-at-consume-site` (B32b) |
| Per-(layer,pos) staging slice copies | `derive-params-from-global-invocation-id` (B26) + `slot-offset-partial-buffer-write` (B33) ambient |
| LM head temp + copy per position | `slot-offset-partial-buffer-write` (B33) |
| mean_k staged vs mean_q inline | `smem-staging-gated-by-cross-thread-reuse` (B35) — POSITIVE instance |
| Phase-A/B per-pos loop shape | `training-forward-shed-decode-loop-shape` (B36) + B26 |
| Fresh zero Vec per call + full-cache zeroing | `persistent-zeros-source-encoder-copy-clear` (B27) + `scoped-init-at-consume-site` (B32b) |
| wte/wpe host clones for CPU adds | `host-cache-immutable-device-weights` (B26) + `hybrid-cpu-elementwise-gpu-throughput-sync-points` (B34) ambient |
| gpu_label! duplicated in 8 files | `debug-only-gpu-label-macro` (B31) — ambient DRY debt |
| Dedicated encoder for one copy | `merge-dispatches-into-single-compute-pass` (B26) ambient |
| `array<f32, 256>` w/ runtime head_dim bounds (×3) | `declared-max-array-taxed-per-thread-under-runtime-bound` (B46) — 3rd+4th in-tree instance |
| One-thread-per-row serial reductions (×2 kernels) | `row-dot-split-inner-axis-block-reduce` (B44) ref_shape |
| Negative-score alpha threshold | shared CPU/GPU semantics — not GPU decision content |

### Lane A (10 dropped)
| Candidate | Killed by |
|---|---|
| game_mux superposition N-forwards→1 | out of scope (pure CPU closure) + demux math broken (Issue 709 H1) |
| CubeCL-on-shared-wgpu-device construction | `same-queue-implicit-ordering-no-sync` (B21) construction side |
| domain_latent 4-buffer optimizer struct | `buffer-set-struct-two-tier-dispatch` (B33) positive instance |
| elf `elf_sigmoid` | `sign-branch-stable-sigmoid` (B32a) — already optimal in-source |
| elf pre-allocated norm scratch | `scratch-depth-matches-shape-invariance` (B45) positive |
| depth_tier fixed histogram | CPU hot-path (rust_perf territory, trivial counting) |
| depth_tier entropy→tier routing | CPU game-logic config |
| cpu_reference panic-vs-verdict contract | test-harness behavior → hazard (Issue 709 H10) |
| cpu_reference "f64 ground truth" discipline | doc-lie, not a rule (Issue 709 H2) |
| mux_kl_loss softmax-recompute | out-of-lane + likely B35 dedup (Issue 709 H11) |

## 5. Mining queue impact

- Files closed: game_mux, domain_latent, context, cpu_reference, depth_tier,
  elf, forward_dflash, forward_flashprefill — **mined (0 rules, hazard-filed)**.
- `kimi_k3_gpu_backward_batched.rs` (1,394 — LOW value, shares Issue 693's
  H1/H2 + broken LCG) and `attention_q8kv_cubecl.rs` (967 — HIGH dedup, B23
  sibling) remain the last unmined files of note; given Batch 50's zero-rule
  result on fresher files, the kernel file-mining program is approaching its
  natural end — the remaining candidates are predicted lower-yield than what
  just produced zero.
- Recommendation for the next kernel batch: treat
  `kimi_k3_gpu_backward_batched.rs` + `attention_q8kv_cubecl.rs` as the FINAL
  file-mining batch (expect 0-2 rules), then declare the riir-gpu corpus
  COMPLETE (~110 of ~200 files mined; the unmined remainder is the
  sibling-blocked `ternary_deltanet_gpu_forward.rs` cluster + test files +
  forward/ dir internals already covered at B26) and shift the queue to the
  rust_perf side + Issue-driven work.
- `ternary_deltanet_gpu_forward.rs` stays sibling-blocked (re-verify clean at
  batch start).

## 6. GOAT for a zero-rule batch

No corpus files were touched (corpus stays 344; no count-pin sites to update).
The standing gates were spot-run to confirm the tree is green post-Batch-49
(no changes since `80f0b9e`, so this is a confirmation, not a re-gate):
default + kernel_opt lib tests pass, clippy 0 both states — see the commit's
validation block. Retrieval floors: unchanged corpus ⇒ floors mechanically
hold (no re-measure needed; the Issue 017 gate runs on corpus content, which
did not change).

## 7. Lessons

1. **Zero-rule batches are a corpus-health signal.** When both lanes of a
   properly-scoped mining batch return zero with detailed drop tables, the
   corpus has achieved coverage on this codebase's pattern distribution —
   further file-mining in the same crate has negative expected value per
   token spent.
2. **The hazard yield IS the batch value.** Batch 50's 31 hazards (5 CRITICAL
   + 2 HIGH among them) are a direct product of the mining discipline — the
   same close read that finds no new rules finds live bugs. A mining batch
   should never be judged by rule count alone.
3. **Repeat-bug evidence for corpus rules.** `forward_dflash` producing the
   EXACT stale-uniform class that Batch 27 filed in `backward_dflash` (same
   file family, both directions) is empirical confirmation that B26 encodes
   a real recurring failure mode, not a one-off.
4. **Doc-lie clusters concentrate on validation substrates.** The two
   cpu_reference doc-lies + the flashprefill "online softmax" claim all sit
   on code whose PURPOSE is to validate other code — the place where a false
   claim does the most damage per line.
