# Plan 185 — Batch 189: open-jev-fast kernel pack (staged)

**Status:** STAGED — conversion cargo-deferred behind the sibling's in-flight Batch 188 + Plan-337 load discipline (the Plan-184 precedent)

Distill source: `lyuyiqi/open-jev-fast @ c52b8bb958c1f0d241d4eb7fce4ecd8d885bf1e4` (MIT) — verdict + full grading: `.research/221_open_jev_fast_cuda_gdn_serving_distill.md`. Reflex-side lane candidate: `riir-reflex/.issues/054` (separate repo, already filed).

## Phase 0 — preconditions

- [ ] Confirm Batch 188 (Plan 184, TensorFold/MLX) has LANDED before this batch's conversion touches the corpus — one batch at a time
- [ ] Confirm a quiet cargo slot (Plan-337 load discipline released; no sibling compute owning the box)

## Phase 1 — dedup re-verification (read the four kinship rules before authoring)

- [ ] Read `row-owned-register-blocked-recurrence` (G) — confirm candidate 1 (`two-chunk-state-independent-overlap-serial-chunk-recurrence`) is scheduling, not per-head decomposition
- [ ] Read `first-order-affine-recurrence-associative-scan` (G) — confirm candidate 1 is barrier-scheduling, not scan-algebra
- [ ] Read `bit-exact-dispatch-fusion-boundary-emulation` + `fused-vector-site-transcendental-form-nonportable` — confirm candidate 3 (`exact-bf16-domain-activation-lut`) decision content is distinct (full-16-bit-input-domain exact replacement vs dispatch/fusion-boundary emulation)
- [ ] Read `multiplexed-grid-axis-partition-modes-exclusive` (CloudRift) — adjudicate candidate 5 (split-K-in-epilogue) as standalone rule vs rider row vs drop
- [ ] Re-run the both-vocabulary greps at the post-188 corpus state (the corpus moves between now and conversion)

## Phase 2 — entry authoring (`entries_gpu_57.rs` or next free)

- [ ] Author candidate 1 `two-chunk-state-independent-overlap-serial-chunk-recurrence` — decision content + mechanism arithmetic (11 vs 24 barriers, state-chain 1–4 warps, bit-identity argument); category: dispatch/scheduling; NOT-BENCHABLE annotation (validator does not host a GDN recurrence) + measured-GOAT re-measurement citation (`bench/logs/r4/v6_e2e.log`, 74→36.5 µs/layer on B300)
- [ ] Author candidate 2 `ancestor-mask-bit-visibility-block-skip-tree-attention` — the measured cross-over IS the regime arm (≤384 rows wins, 576 loses to cuDNN — source-measured); anti: full attention → vendor SDPA/FA path
- [ ] Author candidate 3 `exact-bf16-domain-activation-lut` — 65,536-entry SiLU/sigmoid tables; regime = scalar unary transcendental on 16-bit inputs; anti-arm cites the source's OWN negative (gated-RMSNorm folding into GDN epilogue slower, GDN_VER 5)
- [ ] Author candidate 4 `shape-bucketed-graph-cache-lru` (dispatch) — cite the positive instance of `cuda-graph-scalar-via-device-pointer` (copy-in at replay)
- [ ] Candidate 5: per Phase-1 adjudication — rule, rider row on the CloudRift split-K rule, or annotation-only
- [ ] Measured negatives (gated-RMSNorm-in-GDN-epilogue slower · L2 weight prefetch +0.18 ms · PDL rejected · exhaustive cuBLASLt search unnecessary · FP8 rejected on numerics · prep-fused-into-attention rejected) → anti-régime clauses on the related rules + one annotation row
- [ ] ALL fixtures ORIGINAL minimal reproductions — never paste source (license hygiene + binary-only distribution rule)

## Phase 3 — gates

- [ ] Batch-189 parse gate (rule parse + rider-presence arms, the B176+ shape)
- [ ] Kernel slice clippy gate (the `entries_gpu_*` slice)
- [ ] Lib default + clippy `-D` ×2
- [ ] Retrieval floors HELD (21/21 regression gate)
- [ ] LUT candidate (if it lands as a numeric-construction rule): boundary equivalence arm over the full bf16 domain (table vs reference across all 65,536 inputs — the Batch-59 exhaustive-gate law)
- [ ] Score-bench + freeze tables: NOT owed if zero clippy-corpus delta (the B187 P30 determination) — verify at conversion
- [ ] Count pins: kernel_opt 675 → 675+N (A/B/D buckets per final grades), `--corpus-stats` confirms

## Phase 4 — records + closeout

- [ ] Research 221 status → RECORD (batch landed) in the same commit
- [ ] Queue snapshot: Batch 189 row appended
- [ ] Global binary rebuild + repoint (corpus is compile-time) — at a cargo slot
- [ ] Lane-intel follow-ups stay DETECTION-ONLY: riir-ai GDN prefill lead + riir-infer GDN reference + reflex Issue 054 (hardware-gated) — this batch does not implement any of them
- [ ] Commit + push (`feat:` prefix, develop)
- [-] DEFERRED: any measured-GOAT re-measurement on our hardware (riir-ai/riir-infer owning sessions' call, not this batch)

## Constraints carried from the verdict

- B300-only numbers are provenance, never adoption bars — our 4090/M3 re-measure before any arm lands on our kernels
- Shape-generic rules; the source's Qwen3.8-27B hardcoded shapes are reference material
- Never a Cargo/path dep into the source; never modify `.raw` (clones already deleted post-verdict)
