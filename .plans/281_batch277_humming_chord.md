# Plan 281 — Batch 277: the Humming + Chord quantized-GEMM pair (gated on Batch 276's landing)

**Claimed:** shikuwa/4090 idle loop, 1792874338, expires 2026-10-26T20:38:00Z

**Status:** CLAIMED — staged pair re-targeted to Batch 277 after the Batch-276 dead-heat
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

- [ ] T0 — this claim (plan-first, pushed BEFORE authoring)
- [ ] T0b — clone `.raw/b277/{humming,chord}` at pins (full shas resolved); licence
      chain verified at clone: humming LICENSE, chord LICENSE + DeepGEMM MIT +
      CUTLASS submodule BSD-3 read at pin `59b61c60` (API/partial clone; init not
      required for mining chord's own code) — MAY run pre-gate, read-only
- [ ] T1 — read the ONE open chord issue (conversion reads it per the staging)
- [ ] T2 — at-pin quote re-verification of the 10 Chord + 5 Humming staged contents
      (byte-level at the pins; corrections recorded) — MAY run pre-gate, read-only
- [ ] T3 — dedup greps per candidate: B251 swap-AB/cudafast-pack, Batch-220
      rounding-order, B273 fp32-partials-single-rounding rider, gpu_101 residency,
      B130 l2-grouped-schedule + B98 bridge-rung, B116 crossover, magic-div family,
      B9/B242 embedding, TMA producer faces (gpu_12/B16 staged loader), fusion families
- [ ] T4 — author own rules + count-neutral rider faces into the NEXT free slice
      (124 nominal at claim time; re-read after B276 lands — it may take 124)
      with silicon-distance + NotBenchable annotations honest (staged cells are
      H200/B300; ours SM89)
- [ ] T5 — gates: batch277 needle+parse; kernel_opt lib; default lib; clippy −D ×3
      postures; count pins at EVERY site re-read post-B276 (category tuples, router
      total, Plan-003 sum pins, cross-domain rust_perf pin — the batch-274
      pin-set-wider lesson: the failed-lib list is the only reliable enumerator);
      retrieval floors re-measure (isolated target if moved; re-pin with attribution
      only if moved)
- [ ] T6 — docs: AGENTS ×2 counts, README row, `.docs/11_domains/kernel_opt.md`,
      Research note (highwater re-read at landing), plan ticks, `.distill/001`
      landing row
- [ ] T7 — commit `feat: batch 277: ...` + push; `.raw/b277/` removed at close;
      claim discharged by the landing row

Adjudicated NOT owed: freeze regen + score-bench (kernel_opt-only corpus, P30);
maintained-binary rebuild (kernel_opt outside the release set).
