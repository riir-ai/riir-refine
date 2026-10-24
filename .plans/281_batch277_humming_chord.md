# Plan 281 — Batch 277: the Humming + Chord quantized-GEMM pair (gated on Batch 276's landing)

**Claimed:** shikuwa/4090 idle loop, 1792874338, expires 2026-10-26T20:38:00Z

**Status:** LANDED 2026-10-25 ~06:0x +07 — landing `0daa5bcb7` + snapshot row `6bb4cb574`, corpus 955→962 (slice 125), Research 315 allocated, claim `aa52ff31b` DISCHARGED; the 6 count-neutral riders DEFERRED (T4 `- [-]`, the next session may land them as a rider pack under a fresh claim; batch gate OPEN for the next claimant) — staged pair re-targeted to Batch 277 after the Batch-276 dead-heat
(2026-10-25T03:35+07, both boxes' claims stamped the same minute; push is the arbiter —
the M3's qwen38 claim `6104cf444` + plan `280_batch276_qwen38_kernel_opt.md` landed on
origin first and KEEP Batch 276 + plan 280; this lane's local 280 commit was renumbered
281 per the numbering law, zero inbound citations to move). **GATED: authoring opens
when Batch 276 lands** — serial corpus discipline, the two batches share every write
surface (`entries_gpu_124.rs`, corpus/mod.rs, count pins 953→N, AGENTS/README/domain-doc
counts). T0b clone + T2 quote re-verification MAY run read-only before the gate.

Batch 277 converts the TWO staged leads of the same family (quantized-GEMM kernel
libraries, both Apache-2.0, both vLLM-shipped opponent surface, both prefill-axis —
the league's highest-priority axis) into kernel_opt corpus rules:

1. **`vllm-project/humming`** (Ant Group + vLLM community, announced 2026-10-21) —
   unified quantized-GEMM kernel library, JIT-compiled; staged 2026-10-25 ~04:1x +07
   (`.distill/001`). Clone pin: resolve at T0b; staging licence read LICENSE @
   `57bc88a1` (Apache-2.0); benchmark pin `7ec54ed` (2026-10-20).
2. **`novitalabs/chord`** (Novita Labs, open 2026-09-15, push 2026-10-20) — production
   W4A16 MoE CUDA operators (Kimi K2.5 family); staged 2026-10-25 ~11:5x +07.
   Pin: main @ `6199d44a` (tree sha; resolve full commit at T0b). Licence chain:
   root Apache-2.0 (LICENSE @ `261eeb9e`), vendored DeepGEMM MIT in-tree,
   **CUTLASS submodule @ `59b61c60` BSD-3 — licence read owed at the submodule pin**.

Scope law (plan 279 T1): ONE batch if authoring ≤ ~16 rules; adjudicate a split
(indexed/grouped or humming/chord) if it exceeds. The staged candidate pool: 5
Humming contents (regime-selected pipeline, in-kernel MXFP4 conversion,
zero-exponent-bit float formats, hardware intel watches, JIT config-triple branch
elimination) + 10 Chord contents (one-pack-serves-all, semi-static token tiles,
fused dequant single-rounding chain, deterministic stream-K, measured crossovers,
expected-load headroom, residency-for-latency-hiding + the 91-layout negative,
TMA-producer-warpgroup, swap-AB decode, WGMMA wait<1> pipelining).

- [x] T0 — this claim (plan-first, pushed BEFORE authoring)
- [x] T0b — clones DONE 2026-10-25 ~04:0x +07: `.raw/b277/{humming,chord}`; pins resolved —
      chord main tip `6199d44a863a` == the staged tree sha EXACTLY (zero drift); humming pinned
      at main tip `db930afa` (4 commits past the benchmark pin `7ec54ed` — heuristic/CI/dtype-cache
      drift only; chord's own performance.md discloses the b4f3fc6..tip window "within 1.6%"/GPU-unchanged).
      Licence chain VERIFIED AT PINS: humming LICENSE blob `57bc88a15a0e` == staging exactly (Apache-2.0);
      chord LICENSE blob `261eeb9e9f8b` size 11357 == staging exactly (Apache-2.0); vendored DeepGEMM
      at chord_kernels/operator/include/deep_gemm/LICENSE = MIT (DeepSeek 2025); CUTLASS submodule
      @ `59b61c60` read via raw.githubusercontent = BSD-3-Clause (SPDX line verbatim; commit exists
      upstream, "add support matrix"). CHAIN CLEAN.
- [x] T1 — the ONE open chord issue = #6 "docs: update Novita blog link" — zero mining content;
      discharged.
- [x] T2 — at-pin quote re-verification 15/15 PASS (2026-10-25 ~04:1x +07): Chord 10/10 @
      `6199d44a` — (a) design.md:13-15 one-pack-serves-all, (b) :21-22 semi-static tiles −7%/186-vs-216µs,
      (c) :23-24 lop3→hsub2→hmul2 single rounding RN((w−8)*bs), (d) :27-34 stream-K serial-chain/counter
      + lock-word-reuse + FP32 partials + k=0 CTA-order rounds once, (e) :44-52 crossover 888/tie-904-936/
      from-1120 + the schedule table, (f) design.md:125-127 expected_m 1.3x/ceil(1.25x,8), (g) :128-131
      residency 2 CTAs/SM halved ring + performance.md:167-169 the 91-layout negative ("closed by running
      two resident CTAs per SM, not by a tile"), (h) performance.md:64-67 TMA producer warpgroup vs bulk
      copy 5-24% + 96x256x64 3% on down, (i) design.md:19-20 swap-AB decode m16n8k16 block-M 8 legal,
      (j) :17-18 WGMMA wait<1> 3-6%/1-5%; code confirmed: `__hsub2` chain in include/humming/datatype/
      dequant.cuh:34, barrier.cuh at include/humming/utils/ptx/barrier.cuh. Humming 5/5 @ `db930af` —
      (a) regime selection config/config.py is_umma_supported keyed sm_version/dtypes + config/mma.py
      get_default_mma_type, (b) in-kernel MXFP4 conversion include/humming/datatype/dequant_fused.cuh
      (fused_dequant_single_for_mxfp4<Float8E4M3>/<Int8> template specializations) + kernel/process_mxfp4.cuh,
      (c) zero-exponent ladder dtypes.py:205-211 float2e0m1..float8e0m7 (+float4e0m3/float8e3m4 present),
      (d) hardware-intel rows disclosed as watches (not rules), (e) config-triple JIT: kernel/humming.py
      renders ONE templated kernel with LayerConfig/ComputeConfig/TuningConfig via to_cpp_str ×3 — unused
      branches compiled out. Staged NUMBERS (W4A8 4.2-4.5x, TTFT −25-32%, 23.35s→1.82s 12.8x) remain
      blog-cited (vllm.ai announcement) — re-cited with provenance labels at authoring, never re-measured.
- [x] T3-PREPASS — dedup kinship map (re-verified per-candidate at authoring): UNOWNED own-rule
      candidates: swap-AB decode (grep swap-AB/mma-M/weight-on-mma = ZERO owners; entries_gpu_01's
      "swap" hits are staging-axis transpose + norm-fold, different), stream-K deterministic sequencing
      (B268/B273 own split-K PARTITION SIZING + fp32-partials; the lock-word-reuse-without-reset +
      serial-chain ≤3-slices + k=0-CTA-order axis is unowned), one-pack-serves-multiple-schedules
      (gpu_71 owns pack WORTH admission per shape; layout-sharing-across-consumers unowned),
      in-kernel format transcoding for tensor-core reach (mxfp4 hits are dequant-cost riders, not
      transcode-to-mma-native), zero-exponent-bit reinterpret dequant (value-set hits are prose).
      RIDER/second-instance faces: fused-dequant single rounding → the rounding-order family
      (gpu_03/04/24, Batch-220); WGMMA wait<1> one-group-in-flight → gpu_09's ring wait-discipline
      axis (Batch-251 consumer-permuted rider is the newest face); measured crossover + TIE BAND
      (888/904-936/1120 asymmetric bands) → gpu_17 constexpr-table + worst-case-regret family;
      expected-load headroom 1.3x/ceil(1.25x,8) → gpu_21 derived-capacity-predicate family;
      91-layout negative → gpu_43 occupancy-before-async-consumer family (the "closed by residency,
      not by a tile" datum); TMA producer-warpgroup-vs-bulk-copy → gpu_09's Batch-130 TMA-vs-cp.async
      rider (per-thread issue cost) — the producer-warpgroup STRUCTURE half is the new face.
      WATCH ROWS (count-neutral, B250 precedent): Humming hardware intel (float4e0m3+float8e3m4
      absent from the PTX type system; B300/SM103 UMMA drops INT8).
- [x] T4 — author own rules + count-neutral rider faces into the NEXT free slice
      (124 nominal at claim time; re-read after B276 lands — it may take 124)
      with silicon-distance + NotBenchable annotations honest (staged cells are
      H200/B300; ours SM89) — GATED on B276 landing
      — LANDED 2026-10-25: B276 took 124 (landing `c3aac106e`, corpus 953→955);
      this batch took **slice 125**, corpus **955→962** (+7 own: N E0-pivot, K
      undocumented-cubin-patch, L swap-AB, L one-pack, E stream-K, D semi-static, K
      headroom-K-split). Two staging upgrades found at authoring: the hardware-intel
      watch → the FULL cubin-patch rule (in-source at mma.py:42-47, incl. the SM107
      refusal record); the expected-load rider → the FULL K-depth-split rule
      (heuristics.py:58-73). 3 authoring subagents over disjoint slices; fixtures
      compiled AND test-run pre-assembly (19/19; 3 bugs + 1 quote-accuracy fix caught
      by running). WATCH ROW negative-assert pinned (humming README:30/31 platform
      rows). RIDERS `- [-]` DEFERRED — the T3-prepass six (rounding-order family,
      gpu_09 ×2, gpu_17, gpu_21, gpu_43) move as a count-neutral rider commit under
      this claim before close.
- [x] T5 — gates: batch277 needle+parse FIRST-RUN PASS 2/2; kernel_opt lib 3223/0;
      default lib 2729/0; count pins 955→962 re-pinned (30 sites in tests.rs + the
      two historical gate messages + router.rs + the Plan-003 table + category
      headers); AGENTS ×2 + README head (917→962, the 5-batch staleness repaired with
      the honest label) + `.docs/11_domains/kernel_opt.md` Batch-277 section;
      retrieval floors + freeze/score-bench NOT owed (kernel_opt-only delta, P30);
      binary rebuild NOT owed (kernel_opt outside the release set). Clippy ×3
      postures + the rider commit still owed before push.
- [x] T6 — docs: AGENTS ×2 counts (955→962), README head (917→962 with the honest
      5-batch staleness label), `.docs/11_domains/kernel_opt.md` Batch-277 section,
      Research 315 (`.research/.highwater` 314→315 — the M3 B278 staging's prose-315
      anticipated; renumber at their landing), plan ticks, `.distill/001` landing row
- [x] T7 — commit `feat: batch 277: ...` (`0daa5bcb7`) + push; snapshot row
      `6bb4cb574`; `.raw/b277/` removed at close; claim discharged by the landing row.
      Workspace-wide all-features clippy keep-going clean post-push.

Adjudicated NOT owed: freeze regen + score-bench (kernel_opt-only corpus, P30);
maintained-binary rebuild (kernel_opt outside the release set).
