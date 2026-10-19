# Plan 256 — Batch 252: the #369 prism verification kernel + the upstream leads conversion

**Status:** LANDED 2026-10-19 — +2 own rules + 5 rider/second-instance faces + 1 watch disclosure, corpus 901→903 (C 57→58, F 20→21); gates all green (batch252 needles 2/2 FIRST-RUN, kernel_opt lib 3163/0, default lib 2696/0, clippy −D ×3 clean, retrieval HELD 100%/90.6); freeze/score-bench NOT owed (kernel_opt-only, P30). Claimed by shikuwa/4090 idle loop, 2026-10-19 ~14:1x +07, expires 2026-10-21T14:1x+07 (the push is the arbiter). Sources staged by the 10-19 ~08:1x intake row; every quote read at the local pins this session.

Batch 252 converts the standing watch's staged leads (`.distill/001_mining_queue_snapshot.md` intake row 2026-10-19 ~08:1x) into corpus: **+2 own rules + 5 count-neutral rider/second-instance faces + 1 watch disclosure**. Target: corpus kernel_opt 901 → 903. GPU-window note: StartLux stack resident+idle (issue 635 lane, owner-gated stop) — this batch is CPU-only (corpus authoring + lib tests + clippy), no conflict.

## Sources (all read at local pins this session)

- **Prism fork #369** `1180918489` (origin/prism tip, UNCHANGED since the intake check; newest tag still prism-b10777-e3de715 → decode re-pin NOT triggered): `ggml/src/ggml-cuda/gated_delta_net_l8.cuh` +219 + `gated_delta_net.cu` +19 + `licenses/yukon-cudafast` MIT notice. Provenance: adapted from the Yukon engine `36c9536` (B251's own source) — SECOND-STACK instances of two B251 laws + three new integration faces. Clone: `E:/git/vendor/prismml-llama.cpp` (league vendor clone, fetched).
- **Upstream `ff5a2337` #30602** "cuda : fuse qwen4exp's SCALE -> SIGMOID -> SCALE -> hc_post chain" (port of Vulkan #29520): `dsv4-hc.cu` +31, `dsv4-hc.cuh` +4, `ggml-cuda.cu` +25 — the gated template arm computing `gate_scale_out * sigmoid(gate_scale_in * p)` from the RAW weights. Clone: `F:/llama.cpp` (origin/master @ ff5a2337).
- **Upstream `5aeac37a` #30572** "fattn do not check oob if k_VKQ_max = (k_VKQ_max - nbatch_fa)": `fattn-tile.cuh` 1-line `<` → `<=` — the unchecked fast-path loop takes the last FULLY-COVERED tile.
- **Upstream `0878f7c3` #28851** non-contiguous SUM (CPU/CUDA/Vulkan/WebGPU): op-generality, NO perf mechanism — WATCH disclosure in the research note, no corpus entry (the intake's own verdict).

## Landing shape

**Own rules (2) → `entries_gpu_107.rs`:**
1. `sigmoid-gate-chain-folded-into-consumer-template-arm` — **C (fusion)**: a producer chain of SCALE→SIGMOID→SCALE building gate weights (2*sigmoid(inject/hc)) feeding a consumer kernel folds INTO the consumer as a `gated` template arm (`p = gate_out * (1/(1+expf(-(gate_in * p))))`) reading the RAW pre-chain weights (`post = gate_scale_in->src[0]`), scalars via op params — four kernels become one. Kinship: `moe-router-weight-fold-into-reduce-never-materialize` (B220 — that folds multiply-weights into a REDUCE; this folds a sigmoid GATE-CHAIN into a pointwise consumer with source-rewiring), `gate-activation-precompute-pass-token-threshold-gated` (B251 — WHICH pass activates; this ELIDES materialization entirely).
2. `oob-free-loop-inclusive-of-last-full-tile` — **F (flash-attention tiling)**: the `oob_check = false` fast-path loop bound must be INCLUSIVE of the last fully-covered tile (`k_VKQ_0 <= k_VKQ_max - nbatch_fa`); a strict `<` silently routes one fully-covered tile through the checked path. Kinship: `dense-fast-path-bounded-by-selector-coverage` (B26 — the fast path's ADMISSION bounds; this is the fast path's LOOP-BOUND within an admitted regime), `pad-tail-to-fixed-tile` (pad-into-regime vs bound-into-regime).

**Riders (3 count-neutral faces) + second instances (2):**
- R1 on `packed-code-cpasync-stage-per-use-prmt-decode` (entries_gpu_41): the 4-byte element-granular `cp.async.ca` stage with a computed per-element dst map — stage NARROW so compute reads WIDE (float4) with no transpose pass (L8V k-staging, `dst_i = (r * 8 + n_) * 4 + m`).
- R2 on `verify-ubatch-full-prestage-lane-token-gate-broadcast` (entries_gpu_105): the fixed-token-window face — SMEM sized by the window MAX (`stage[GDN_L8V_MAX_TOKENS][TOK_SM]`, MAX=16) with the dispatch window gate (`n_tokens >= GDN_L8V_MIN_TOKENS && n_tokens <= GDN_L8V_MAX_TOKENS`), padding lanes staged-skip (`continue // padding`).
- R3 on `gate-activation-precompute-pass-token-threshold-gated` (entries_gpu_105): the template-arm specialization face — the L8V kernel reuses the SAME `<RAW, G_PRECOMPUTED>` arms with `if constexpr (RAW)` in-kernel activation, zero-cost static dispatch (the B251 pairing assert's law carried to a second stack).
- R4 second instance on `lane-permuted-column-reduce-same-butterfly-tree-bit-identity` (entries_gpu_105): `gdn_l8_reduce` = the same xor-tree-preserving fold across the 4-old-lanes remap (`__fadd_rn(__fadd_rn(p[0], p[2]), __fadd_rn(p[1], p[3]))` + xor 4/2/1 in-group) — same law, same source commit (`36c9536`), second stack (product-coverage validation).
- R5 second instance on `host-dispatch-predicate-mirrors-kernel-compiled-arch` (entries_gpu_105): the #369 dispatch gates on `ggml_cuda_highest_compiled_arch(cc) >= GGML_CUDA_CC_AMPERE` host-side vs `__CUDA_ARCH__ >= GGML_CUDA_CC_AMPERE` device-side (+ NVIDIA/warp_size/cols_per_warp/token-window predicates) — the one-predicate-two-sides law, second stack.

## Tasks

- [x] T0 claim + this plan committed & pushed BEFORE authoring (the push is the arbiter) — `90015e92`
- [x] T1 Dedup greps (DONE at plan time): oob/bounds families (no owner of the loop-bound law), hc_post/sigmoid-chain (no owner), swizzle stage (entries_gpu_41 host identified), window SMEM (B251 verify-window host), template arms (B251 gate-activation host)
- [x] T2 Own-rule drafts: 2 parallel subagents (one per rule) from the read sources; coordinator re-verifies every quote at the pins (corrections: fattn cite :955→:958-959; l8.cuh spans :9-10→:10-11, :11-19→:13-22, :37→:53, :72-73→:78-79, :93→:99, :126-134→:124-132, :137→:132; the 1/hc + 2.0f chain shape verified at the Vulkan origin #29520 test builder, not assumed)
- [x] T3 `entries_gpu_107.rs` authored (2 rules + module doc) + mod.rs wiring (mod decl + entries() extend + C/F category-list additions)
- [x] T4 Riders R1–R5 appended to the four host entries (R1 gpu_41 cp.async 4-byte swizzle stage; R2 gpu_105 fixed-token-window SMEM-by-MAX; R3 gpu_105 template-arm specialization; R4 gpu_105 lane-permuted SECOND INSTANCE; R5 gpu_105 host-dispatch SECOND INSTANCE) — every line cite re-verified at the pin
- [x] T5 Gates: `batch252_rules_landed` (2 needle sets + the #28851 WATCH negative assert) + `batch252_riders_landed` (5 hosts) in corpus/tests.rs — FIRST-RUN PASS
- [x] T6 Count pins: 18 sites in tests.rs 901→903 (each annotated) + C 57→58 + F 20→21 + router.rs:472 experts pin + mod.rs C/F count comments (F was STALE at 19 vs the table's 20 — repaired to 21 with the note)
- [x] T7 Gates run: kernel_opt lib 3163/0 + default lib 2696/0 + clippy −D ×3 postures clean + retrieval floors HELD (kernel 100%, blended 90.6%, no re-pin); freeze/score-bench NOT owed (kernel_opt-only, P30 precedent)
- [x] T8 Docs: Research 287 + AGENTS.md kernel_opt bullet + `.docs/11_domains/kernel_opt.md` batch row + queue-snapshot landing row
- [x] T9 Commit `feat: batch 252: ...` + push; vendor clones stay (league-owned, not ephemeral)
