# Research 130 — Batch 107: the uzu + lily defers batch (the B105 queue residue)

**Status:** RECORD (landed 2026-09-05, 4090 box, CPU-only)

- **Sources (both re-cloned at the verdict pins, HEAD-verified):**
  - trymirai/uzu @ `115609b6ef7ce5a94fc761bf395780f032b09f9b` (Release 0.5.23, **MIT**) — `.raw/uzu`, removed at landing.
  - perplexityai/pplx-garden `lily/` @ `1ed972ed3f0bd5616c997c9507c25616c63394fc` (**Apache-2.0**) — `.raw/pplx-garden`, removed at landing. Only 2 commits exist upstream (the engine landed in `d88b1e7`, the pin is the license fix); no tuning evolution to mine.
- **Method:** two parallel subagent lanes (A: 7 uzu defers, B: 5 lily defers) re-verifying all 12 B105-deferred candidates at the pins with byte-accurate quotes; coordinator spot-verified 6 load-bearing quote clusters (batch_topology.rs:25, model.rs:36 PREFILL_CHUNK, rng.h:116, counts_offsets_fused.metal:41/25, attention.metal:225-233, model.rs:1168-1226) — all byte-accurate.
- **Verdict:** 8 RULE / 1 RIDER / 3 DROP from 12 candidates (34% falsified/killed — the mine-time discipline doing its job).

## Landed rules (corpus 482 → 490; entries_gpu_20.rs)

| rule_id | Cat | Grade | Source | One-line |
|---|---|---|---|---|
| sparse-demand-paged-kv-cache | A | B+ | uzu | Placement-sparse KV buffers: full virtual extent, physical pages mapped as the gap-diff per prepare; address never moves; one shared heap pool; MTLHeapType::Sparse-is-for-textures trap. |
| gumbel-argmax-filtered-sampling-descent | C | B+ | uzu | Gumbel-max + in-kernel top-k/p/min-p filters + bounded descent on filter failure; clamped uniform ([2^-24, 1-2^-24]); fixed Philox field = fixed Plackett-Luce order. |
| packed-total-order-multi-field-argmax | E | B− | uzu | (score,index) argmax as ONE reduction via u64 packed key or total-order struct; the router_topk two-reduction + sentinel shape is the anti-instance. |
| integer-mma-span-epilogue-dequant | N | B− | uzu | Int fragments accumulate per SPAN = min(left, right group); dequant hoisted to the span boundary; activation zero-point from precomputed group sums; host-decided scale residency (parity-banked threadgroup vs registers). |
| single-threadgroup-fused-count-scan | H | C+ | uzu | MoE prologue (count + raking exclusive scan + total + tile partials) in ONE kernel, ONE threadgroup; honest decode-side regime; upstream E>128 defects recorded in-rule. |
| radix-select-ordered-key-topk | E | B− | uzu | Vocab-scale top-k as MSB-first radix SELECT over packed u64 keys; arrival-counter histogram merge (self-cleaning); pass count from index bits; NO explicit size router (verdict-time claim corrected). |
| prefill-chunk-command-buffer-capacity-coupled | D | C+ | lily | Chunk size == scratch capacity (PREFILL_CHUNK = 4096); boundary = only sync AND the upload window; no re-zero via write-before-read full-extent; drop-before-allocate growth. |
| tensor-op-strided-device-operand-binding | A | B− | lily | Metal-4 tensor ops bind operands by residence: device-resident → strided views (no k-loop at all), produced → smem; freed budget funds softmax state; ragged edges via slice bounds-checks. |

## Riders (folded into corpus text)

- **B63 `deferred-residual-fused-into-next-layer-norm`** += the lily decode-graph external instance (model.rs:1170-1174) + its MEASURED NUMERIC CONTRACT: stored residual bit-identical, normalized output ≤ 1 bf16 ULP, ULP COMPILER-DEPENDENT (g14 keeps the unrounded fp32 sum) — the lossy-surface gate is per-GPU-family (lily tests/unit/kernels/norm.rs:59-63).
- **B95 `whole-sequence-zero-barrier-single-dispatch-scan`** += gdn_prefill_regscan (gdn.metal:180-236) — SECOND independent implementation for the same recurrence — + the UNIFORMITY COMPLEMENT (attention.metal:691-692): simdgroup-uniform guards keep conditional consume blocks legal for simd_sum inside; no lane ever diverges.

## Drop ledger

| Candidate | Reason |
|---|---|
| flat-topology-fast-path (uzu) | Not model-graph topology — the BATCH TRIE (token tree for speculative batching). Mechanism = optional-buffer JIT specialization (B23 family) + a degenerate-structure predicate (`is_flat &= node.height == index`, batch_topology.rs:25); no bounded teachable fix-space. Trie-attention ancestor masking noted as a possible future candidate (adjacent to our Issue-721 work). |
| RIF-sentinel (lily) | Born dead at the pin: the RIF/SERIAL template machinery (attention.metal:623-624) has exactly ONE instantiation `<4,1,4,1,0,1>` = RIF=1/SERIAL=0 (identical at d88b1e7), and the 0xFFFFFFFF row sentinel can never occur (the cache is zero-filled, nothing writes 0xFFFF words). Salvage = the two B95 riders above. MoE block-map sentinel already covered by B105. |
| weight-hygiene cluster (lily) | concat_rows = B63 `same-input-gemv-weight-concat-at-upload` external instance (weights.rs:227-230); conv_weight transpose + a_log bf16→f32 = B31 `dtype-normalize-once-at-upload` instances (:324-326, :465-466); zero_centered_norm +1.0 undo = correctness hygiene with a precision proof, no perf decision; finish() exhaustiveness gate = genuinely good but loader-completeness, OUT of kernel_opt domain (report-only). |

## Upstream hazards (report-only, owner's call to file upstream)

1. **uzu `counts_offsets_fused.metal` — CONFIRMED + EXTENDED.** B105's find is byte-verified (`counts_shared[e0 + e]` writes up to 511 into `uint counts_shared[128]`, :100 vs :54 — OOB for 128 < E ≤ 512), and Lane A found a SECOND defect in the same kernel: the scan reads `counts_shared[lid]` instead of `counts_shared[base + lid]` (:118), so for E > 128 every chunk re-scans the first 128 counts and the carry accumulates the same block sum — offsets wrong for E > 128 even ignoring the OOB. Both invisible to the parity suite by construction (largest tested shape `(1024, 128)` at moe_counts_offsets_fused_test.rs:75; production wiring admits E = 512 via `E.div_ceil(512)` partials at moe/mod.rs:96). A DeepSeek-class E=256 model hits both. Fixture-population class — cf. our `tile-loop-back-edge-needs-multi-tile-bitexact-gate`.
2. **uzu `weaver_top_children.metal:25`** — the Gumbel uniform uses the full 32-bit word with NO clamp; `0xFFFFFFFF` converts to exactly 2^32 → u = 1.0 → noise −inf → that candidate silently ALWAYS LOSES the tree-children argmax (~1-in-4-billion per-token bias). Contrast: `rng.h:116-119` documents and fixes the OPPOSITE direction (+inf wins) for `unified_sampling` — the guard exists at one Gumbel site and not the other.
3. **uzu `unified_sampling.metal:232`** — `filters_passed || iteration == MAX_ITERS - 1` can emit a candidate that still violates top-k/top-p/min-p after 64 failed draws — a bounded-fallback SEMANTICS worth knowing when porting (design note, not a bug).
4. **lily `descending_blocks` knob** — shipped-but-unarmed: the NAX SDPA kernel supports reversed causal block order (attention.metal:198-200) and production passes 0 (attention.rs:276); no in-repo measurement. Report-only (our armed-default-off posture covers the pattern).

## En-route notes

- B105's lily GOAT-lead A/B fixture (Plan 075 T7 defer, `concurrent-dispatch-dependency-level-barriers`) remains armed for the M3 when the GPU lane quiets — unchanged by this batch.
- lily perf doc note (context, not a rule): prefill peaks at ctx 4096 (5,724 tok/s) and declines to 1,793 @131K; at ≥32K the newer MLX wins (0.991×/0.902×/0.801×) — Lily's advantage is ≤16K.
- pplx-garden history: only 2 commits at the pin — a repo this fresh carries churn risk; the pin is load-bearing.
- Process: the terminal on this box drops command output non-deterministically (the B100 hazard, re-confirmed; even single redirects need `> f 2>&1` ordering verification). Workaround: log to /tmp, `cp` into the repo target dir (gitignored), read with the file tools.

## GOAT (all green at final state)

- batch107 gates 3/3 (rules_present + concepts_covered + pseudocode_parses cfg(not(kernel_verify))); one concepts needle caught by the gate pre-landing (case mismatch "FIXED" — the B51 lesson, the gate doing its job).
- kernel_heal lib **1346/0/1i** (was 1343; +3 gates) · default lib **1078/0** (unchanged — the kernel_opt domain is kernel_heal-gated) · kernel_heal+latent **1454/0/2i** with `retrieval_precision_regression_gate` PASS — floors HELD, zero re-pins (the two rider text additions + 8 new rules did not decay retrieval).
- clippy `-D warnings` 0 in default + kernel_heal states (one doc_markdown fix en route: `counts_offsets_fused` backticks in the new //! header — the B37 lesson).
- Count pins walked: tests.rs total 482→490 + message + expected-array A/C/D/E/H/N + both sums + cover-all comment; router.rs expert total 482→490; mod.rs doc line 482→490 + 6 category comments (A 108, C 43, D 112, E 35, H 21, N 29 — the N comment was stale at 27 pre-batch, actual 28; repaired to 29).
