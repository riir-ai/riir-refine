# Plan 185 — Batch 189: open-jev-fast kernel pack (staged)

**Status:** COMPLETE — Batch 189 LANDED 2026-09-30 (the quiet cargo slot opened after the sibling's round-5 measurement retired; all gates green)

Distill source: `lyuyiqi/open-jev-fast @ c52b8bb958c1f0d241d4eb7fce4ecd8d885bf1e4` (MIT) — verdict + full grading: `.research/221_open_jev_fast_cuda_gdn_serving_distill.md`. Reflex-side lane candidate: `riir-reflex/.issues/054` (separate repo, already filed).

## Phase 0 — preconditions

- [x] Confirm Batch 188 (Plan 184, TensorFold/MLX) has LANDED before this batch's conversion touches the corpus — one batch at a time
- [x] Confirm a quiet cargo slot (Plan-337 load discipline released; no sibling compute owning the box) — load 4.04 at start, the round-5 probe retired

## Phase 1 — dedup re-verification (read the four kinship rules before authoring)

- [x] Read `row-owned-register-blocked-recurrence` (G) — confirm candidate 1 (`two-chunk-state-independent-overlap-serial-chunk-recurrence`) is scheduling, not per-head decomposition — CONFIRMED: that rule decomposes state ownership to (head, row); cand 1 changes no ownership, it SCHEDULES two chunks' state-independent halves into each other's barrier intervals
- [x] Read `first-order-affine-recurrence-associative-scan` (G) — confirm candidate 1 is barrier-scheduling, not scan-algebra — CONFIRMED: that rule's own text routes dxd-state recurrences (GDN) to chunking; cand 1 is the scheduling win INSIDE the chunked form (composes)
- [x] Read `bit-exact-dispatch-fusion-boundary-emulation` + `fused-vector-site-transcendental-form-nonportable` — confirm candidate 3 (`exact-bf16-domain-activation-lut`) decision content is distinct — CONFIRMED: table substitution on the full 16-bit domain vs form-selection-within-inline-evaluation (and immune to its hazard: the table IS the validated output); also distinct-from `exhaustive-domain-input-patch-bit-identity` (patching an approximation vs full substitution) added at conversion
- [x] Read `multiplexed-grid-axis-partition-modes-exclusive` (CloudRift) — adjudicate candidate 5 (split-K-in-epilogue) as standalone rule vs rider row vs drop — ADJUDICATED: RIDER on `split-k-two-pass-attention-decode-long-context` (that rule owns the split-then-combine decision space; the rider adds the THIRD placement — fold-at-consumer epilogue — beside its separate-combine and the CloudRift atomic-accumulate arms)
- [x] Re-run the both-vocabulary greps at the post-188 corpus state — done at 679: ZERO exact-id hits for all four candidates; nearest concepts (ILP overlap, LUT families) distinct-from'd in-rule

## Phase 2 — entry authoring (`entries_gpu_57.rs` or next free)

- [x] Author candidate 1 `two-chunk-state-independent-overlap-serial-chunk-recurrence` (G) — decision content + mechanism arithmetic (11 vs 24 barriers, state-chain 1–4 warps, bit-identity argument); NOT-BENCHABLE annotation + measured-GOAT re-measurement citation (`bench/logs/r4/v6_e2e.log`, 74→36.5 µs/layer on B300)
- [x] Author candidate 2 `ancestor-mask-bit-visibility-block-skip-tree-attention` (F) — the measured cross-over IS the regime arm (≤384 rows wins, 576 loses to cuDNN — source-measured); anti: full attention → vendor SDPA/FA path + the fattn2 prep-fusion negative as the second anti-arm
- [x] Author candidate 3 `exact-bf16-domain-activation-lut` (B) — 65,536-entry SiLU/sigmoid tables; regime = scalar unary transcendental on 16-bit inputs; anti-arm cites the source's OWN negative (gated-RMSNorm folding into GDN epilogue slower, GDN_VER 5)
- [x] Author candidate 4 `shape-bucketed-graph-cache-lru` (D) — cites the positive instance of `cuda-graph-scalar-via-device-pointer` (copy-in at replay)
- [x] Candidate 5: per Phase-1 adjudication — RIDER row on `split-k-two-pass-attention-decode-long-context` (three-placement combine decision: separate-combine / fold-into-consumer / atomic-accumulate)
- [x] Measured negatives → the `fused-serving-stack-measured-negative-ledger` annotation row (D, refutation-type: L2 prefetch +0.18 ms · PDL rejected · exhaustive cuBLASLt unnecessary · FP8 rejected on numerics · gated-RMSNorm slower) + anti-arms on the related rules
- [x] ALL fixtures ORIGINAL minimal reproductions — never paste source (license hygiene + binary-only distribution rule)

## Phase 3 — gates

- [x] Batch-189 parse gate (rule parse + rider-presence arms, the B176+ shape) — `batch189_pseudocode_parses` (5 ids) + `batch189_riders_present`
- [x] Kernel slice clippy gate — GREEN 2766/0 (1 ignored pre-existing)
- [x] Lib default + clippy `-D` ×2 — 2385/0 + clean at default/no-default/all-features (after healing B188's 27 stale doc-lints in entries_gpu_56.rs — PRE-EXISTING at HEAD, the B188 record's "clippy both postures clean" over-claimed; disclosed in the batch row + 2 manual backtick-span repairs the healer will not make)
- [x] Retrieval floors HELD (21/21 regression gate) — 210.86s, no decay
- [x] LUT candidate: lands as decision-content (pseudocode fixtures, parse-gated — the corpus's G1 form; the Batch-59 exhaustive-arm law binds rules shipping REAL numeric Rust, none here). The bit-exactness claim is source-attested; our exhaustive arm rides any actual adoption (the measured-GOAT offer).
- [x] Score-bench + freeze tables: NOT owed — zero clippy-corpus delta (P30 verified: kernel_opt-only batch)
- [x] Count pins: kernel_opt 679 → 684 (G 29→30, F 16→17, B 74→75, D 147→149), `--corpus-stats` at rebuild + the router.rs:472 expert pin (STALE at 675 since B188 — caught by this batch's kernel-slice run, fixed to 684 with the honest label)

## Phase 4 — records + closeout

- [x] Research 221 status → RECORD (batch landed) in the same commit
- [x] Queue snapshot: Batch 189 row appended
- [x] Global binary rebuild + repoint (corpus is compile-time) — DONE: release-set build (latent_retrieval,kat_service,kat_transport,fix_verify) in an isolated CARGO_TARGET_DIR, stamp `release set: complete`, `--corpus-stats` kernel_opt **684**, copied over the symlink target, temp dirs removed
- [x] Lane-intel follow-ups stay DETECTION-ONLY: riir-ai GDN prefill lead + riir-infer GDN reference + reflex Issue 054 (hardware-gated) — this batch does not implement any of them
- [x] Commit + push (`feat:` prefix, develop) — `95456c9f` pushed (8d6aa421..95456c9f)
- [-] DEFERRED: any measured-GOAT re-measurement on our hardware (riir-ai/riir-infer owning sessions' call, not this batch)

## Constraints carried from the verdict

- B300-only numbers are provenance, never adoption bars — our 4090/M3 re-measure before any arm lands on our kernels
- Shape-generic rules; the source's Qwen3.8-27B hardcoded shapes are reference material
- Never a Cargo/path dep into the source; never modify `.raw` (clones already deleted post-verdict)
