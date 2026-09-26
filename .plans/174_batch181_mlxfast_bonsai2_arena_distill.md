# Plan 174 — Batch 181: the mlxfast-bonsai2 arena distill (Research 212)

**Status:** STAGED — nominal Batch 181, queued BEHIND Batch 180 (Plan 173, qvac Bonsai CUDA). One-batch-per-day cadence holds; this plan is created BEFORE any corpus edit per the plan-first law. Verdict note: [`.research/212_mlxfast_bonsai2_arena_distill_verdict.md`](../.research/212_mlxfast_bonsai2_arena_distill_verdict.md) (Status: DISTILLED — pending owner decision; Claude verdict ping-pong closed AGREE round 3, session `419201be`).

**Source:** `Layr-Labs/mlxfast-bonsai2-27b-engine` @ `b3050f0f8c11c239efa9661aafcee62741eb7115` (MIT). T0 re-clones at the pin (the `.raw` clone was removed at verdict close). Re-verify every quote at mine time (the B177/B178 law).

## Phase 1 — pre-batch verification
- [ ] T1.1 Re-clone at the pin; re-verify the LEAD quotes byte-for-byte (`Qwen35.swift:5617–5624` signed-codes comment + kill-switch; `Qwen35.swift:5310/5401` the two matmul2d descriptors; `DFlash2Draft.swift:~924/935/962/1052` DFlash2TensorMatmul + STACK_GATEUP; the mask-memo and warm-prefill doc comments).
- [ ] T1.2 Dedup re-verification: LEAD-4 against all 7 rust_perf caching rules by name; LEAD-2 against the FWHT ladder family (B116/#29094/#29095) confirming the writer-vs-transform layer split; LEAD-1 family table re-read (B107 uzu `entries_gpu_20.rs`, B96, Research 211 LEAD-2).
- [ ] T1.3 Owner decision gate: confirm B181 proceeds (owner gate on the batch slot + any LEAD trimming).

## Phase 2 — corpus entries
- [ ] T2.1 **LEAD-1 `signed-activation-codes-drop-dequant-offset-term`** (kernel_opt, B+): rule text carries BOTH regime conditions — (a) FP32 mantissa budget `bits(scale)+bits(colsum) ≤ 24` with the worked numbers (2-bit g128 = 20 exact; 4-bit ≈22 exact; 8-bit 26 NOT exact; FP32 scale breaks earlier) and (b) consumer ISA (u8×s8 lanes VNNI `vpdpbusd`/AVX2 `vpmaddubsw` = forced-offset anti-regime; CUDA dp4a / NEON sdot / Metal staged-int8 MMA = paying lanes). Cite the family table (B107 consumer-lookup, B96 bit-plane, R211-LEAD-2 weight-side) — cross-referenced, landed separately.
- [ ] T2.2 **LEAD-2 `fused-transform-quantize-writer`** (kernel_opt, B+): rotation+group-quant in ONE writer kernel (codes+scales+scaledSums); boundary variant (RMSNorm+quant); activation-fn carriers (`rotatedSwiGLU` class). Kinship note vs the FWHT ladder family (different layer).
- [ ] T2.3 **LEAD-3 `tensor-core-mma-staged-quantized-codes-double-buffered`** (kernel_opt, B): both instances cited (prompt 64×64×128 `simdgroups<4>` + verify 16×TN; the drafter BF16 16×32×256 `multiply_accumulate` variant — the Issue-846 chat-row-relevant one); STACK_GATEUP as rider (weight-stack family).
- [ ] T2.4 **LEAD-4 `mask-memo-hoist-to-owner-lifetime`** (rust_perf, B−): caching category; graph-reuse enabler framing.
- [ ] T2.5 **LEAD-5 `seed-warm-the-scored-seam`** (NotBenchable decision-content, B−): measurement-honesty family; `bench_spec: None`.
- [ ] T2.6 Riders: §6.1 cast-hazard annotation (HadamardQuantizedLinear IS-A QuantizedLinear → raw `.weight` reads skip the transform); per-depth-oracle measurement-law annotation.

## Phase 3 — fixtures + gates
- [ ] T3.1 LEAD-1 fixture: bit-identical INSIDE the regime (2-bit g128 / NEON or CUDA-shape) **+ the mandatory NEGATIVE arm** (8-bit or FP32-scale shape where the equality breaks).
- [ ] T3.2 Per-ISA A/B note: x86 u8×s8 lane documented as the anti-regime (no win claimed there); NEON/CUDA lanes benched.
- [ ] T3.3 Standing gates: corpus count pins (router.rs + per-category tables), corpus tests, kernel retrieval floors, clippy `-D` both postures, score row appended, freeze-table regen IF the clippy_lints corpus moved (it should not — kernel_opt/rust_perf only), global binary rebuild at the new corpus.

## Phase 4 — records + close
- [ ] T4.1 Research 212 status → RECORD with the landing paragraph (batch number, landed rules, pin re-verification note).
- [ ] T4.2 Queue line: B181 landed row in `.distill/001_mining_queue_snapshot.md`.
- [ ] T4.3 Pins walked: AGENTS.md kernel_opt/rust_perf bullets + the domain doc seed rows + mod.rs header counts if moved.
- [ ] T4.4 League/lane follow-ups recorded (do NOT auto-file sibling issues; name them): Plan 602 B5 writer-side fusion analog; Issue 846 chat-row engine-side levers; the per-depth-oracle pin discipline for our spec lanes; the Issue-846-lane-intel line in the queue snapshot if not already there.
- [ ] T4.5 Commit + push (`feat(batch181):` per convention), `Session:` trailer.

## Cross-references
- **Plan 173 (B180, qvac Bonsai CUDA):** Research 211 LEAD-2 (weight-side deferred-bias ternary dp4a + exact isum) is the MIRROR IMAGE of this plan's LEAD-1 (activation-side signed-code elimination). OPTION at mine time: one shared offset-family fixture with a common A/B across the two batches, so the family is not counted twice. Whichever batch mines second MUST cite the first's landed rule in its entry's distinct-from clause.
