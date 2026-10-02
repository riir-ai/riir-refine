# Research 149 — Batch 127: the PrismML llama.cpp CUDA ternary MMQ mma distill

**Status:** RECORD 2026-09-07 — Batch 127 landed: kernel_opt +6 (559 → 565: A 118→120, B 59→60, D 129→130, H 26→27, N 38→39; C/E/F/G/I–M/O unchanged). Gates: batch127 3/3 (parse gate caught two C-flavored ref_shapes + one CUDA-launch-syntax snippet — syn reads Rust, every snippet fn-wrapped Rust-shaped per the B122/B123 lesson) + lib default 1167/0 + kernel_opt 1486/0/1i + kernel_opt+latent 1603/0/2i + retrieval re-run kernel 100/100/100 blended 94.7/96.9/97.7 (byte-identical to B118–B125) + clippy 0 ×3 (3 doc_markbacktick in the new module doc, healed) + score-bench #76 FLAT 96.2/1.9/100/100. Source `PrismML-Eng/llama.cpp` @ `e311ed38fe7ab8fb577a5435b049d48b7d040923` (MIT; fresh clone in `.raw/prismml-cuda`, HEAD re-verified == pin, LICENSE verified, removed at landing). Plan 108 / this note.

## The source and the lead

The league opponent's CUDA ternary MMQ mma family — the UNMINED CUDA side of the PrismML fork (prior contact B96 Metal extraction / B113 fa-vec tables / B116 PTQ1_0 format). Lead filed 2026-09-07 by the 4090 box's riir-ai B877 opponent autopsy (Bench 877: our `gemm_ternary_i8_mma_cuda_raw` runs ~23% of Ada int8 peak vs the fork's implied ~50–54% — a 2.2–2.4× e2e gap). **Vendor-checkout hazard recorded:** the shared `../vendor/prismml-llama.cpp` checkout is NOT at the pin (pin not an ancestor of its HEAD; `mmq.cu` differs) — the league-watch lane's moving state; the mine used a fresh `.raw` clone only. Repo-name trap: the org is `PrismML-Eng`, not `PrismML` (the lead's shorthand 404s).

## What landed (6 rules)

| # | Rule | Cat | Source site | One-line decision content |
|---|---|---|---|---|
| 1 | `lut-free-bit-spread-vadd4-unpack` | B | mmq-hopper-q1.cu:213–215 | No `__constant__` LUT in a hot-loop bit→byte expansion: real weight bits are high-entropy → divergent lane indices serialize the constant cache (~1.7× e2e); mask/shift spread + one `__vadd4`/`__vsub4`. **The benchmark trap is in-rule: SYNTHETIC UNIFORM DATA HIDES THE LOSS** — gates for this class must bench real weights |
| 2 | `per-block-int32-chunk-rescale-fp32-accum` | N | mmq-hopper-q1.cu:267–272 | Exact-int32 tensor-core accumulation per 128-block k-chunk (chunk == weight block == activation group → one constant scale pair), one rescale-FMA per element into fp32, clear per chunk; never fp16 accums, never per-element dequant. Q2_0's 64-block doesn't align → excluded. Accuracy gate family-conditional (per-128 self-flagged for KLD) |
| 3 | `one-time-dense-repack-keyed-sync-publish` | D | mmq-hopper-q1.cu:40–63 | Static interleaved micro-bit weights repack ONCE to dense arrays; the guards are the content — tuple key (device, ptr, N, K, **wbits**) and publish-after-`cudaStreamSynchronize` (cross-stream consumers otherwise read uninitialized buffers — silent-wrong-answer class) |
| 4 | `sm90a-window-shape-gated-wgmma-dispatch` | A | mmq-hopper-q1.cu:453–456 | wgmma admission = EXACT arch window `900 ≤ cc < BLACKWELL` ("no GGML_CUDA_CC_HOPPER macro in this tree" — `cc >= 900` alone silently breaks on Blackwell/tcgen05) + type/fp32/2D-contiguous/%128 gates + opt-in + fall-through false |
| 5 | `streamk-starved-tile-grid-dispatch` | A | mmq-hopper-q1.cu:494+472–493 | Starvation cutover `ntiles < 8·nsm` → persistent stream-K (uniform-cost tiles — the occupancy complement of B102's data-dependent-cost decomposition); owned-tile direct-store vs atomicAdd-of-partials only for split tiles; per-device SM-count + dynamic-SMEM caches; **fp32-atomicAdd flush is run-to-run non-deterministic** — deterministic-replay consumers pin the fixed grid |
| 6 | `moe-broadcast-quantize-once-scatter` | F | mmq.cu:194–196 | MoE gate/up broadcast (ne11==1, n_expert_used>1): quantize the shared fp32 row ONCE, scatter the cheap int8 artifact to expert slots (ids map = the inverse addressing) — dedup the expensive transform, duplicate the cheap copy |

## Riders + confirmations (no entries)

- **B98 rider (dequant-in-tensor-op-GEMM):** all three ternary types expand to int8 bytes in the tile load then run the STOCK `vec_dot_q8_0_q8_1` on both dp4a (`mmq.cuh:547/553/559`) and mma (`:717/723/729`, DS layout D4) — the skeleton confirmed externally; the loaders are the only type-specific code.
- **B113 rider (per-arch tables):** Ampere ternary rows are geometry-identical to Q8_0 rows (256 thr, occ 1, I=128, stream_k=true for ALL types) — ternary rides Q8_0 geometry; only the loader differs.
- **B20 rider (byte-perm):** the two-level `__byte_perm` crumb decode (`mmq-load-tiles.cuh:49–51`, selector table 0x11100100 then value map) = the 2-bit instantiation of the permutation-immediate family.
- **cp.async E-class rider (measured):** the synchronous dense A-copy costs ~1.8× e2e — the SIMT-unpacked operand is the latency-hiding compute (`mmq-hopper-q1.cu:197–199`).
- **CDNA4 xf32 note (held, not ruled):** gfx950 REMOVED the CDNA3 xf32 MFMA path (`mma.cuh:1110`) — "never key a fast-instruction path on `>= generation`"; narrow (AMD-only), promotes on a second instance.

## Porting notes (load-bearing, recorded for riir-ai Issue 884 T6)

1. The dispatch fast-path (`ggml-cuda.cu:1867`) restricts to the two 128-block formats (Q1_0/PQ2_0) + ne11 ≥ 128 + in-path M/N/K % 128 + contiguity + the cc window + `GGML_HOPPER_Q1` env + build-time CUTLASS include — **doubly opt-in: default league runs never execute this path** (league-fairness note for any opponent comparison).
2. **`src1_q8_1` staging is sized by `get_J_max`** — a walk down from min(ne11, 512) in steps of 8 to the largest REALIZABLE config-J; sizing by ne11 alone under-allocates (silent OOB) whenever ne11 falls between config J rows.
3. **Determinism:** the stream-K flush is fp32 `atomicAdd` into pre-zeroed C — non-deterministic accumulation; bit-identical-replay stacks pin the fixed-grid path for committed ticks (our sync-boundary rule).
4. `fallback = ne01 % 128 != 0` threads a bounds-check-free specialization through every kernel (tile I=128, not the quant group).
5. Unexplored: `mmvq.cu:942` gives ternary types a CDNA-specific MMVQ branch — follow-up read if the corpus wants the CDNA GEMV carve-out.
6. The D4-vs-DS4 `ds_layout` split (ternary/Q8_0/Q5_0/MXFP4 → D4 scales-only; Q4_0/Q4_1/Q5_1/IQ1_S → DS4 with y partial sums) — the Q4_0-keeps-DS4-vs-Q5_0 divergence was NOT verified; left unproposed.

## League intel

The wgmma path is doubly opt-in (build + env) — the scored league cells never ran it; the B877 autopsy's 2.2–2.4× gap attribution to this family needs the caveat that the fork's DEFAULT path is the standard MMQ (the gap is then dp4a/MMQ-shape efficiency, not wgmma). No new league row; the pin (`e311ed38f`) and upstream watch are unchanged by this landing.

## Provenance

- Clone: `.raw/prismml-cuda` @ `e311ed38fe7ab8fb577a5435b049d48b7d040923` (tag `prism-b10660-e311ed3`; HEAD == pin verified at stage time 2026-09-07; LICENSE MIT verified; all 9 coordinator quote checks via sed at the pin; clone removed at landing).
- Lanes: 2 parallel read lanes (A: MMQ kernels + dispatch; B: mma primitives + arch configs), 12 proposals total → coordinator landed 6 + 5 riders; convergent independent reads on rules 1–3 (identical quotes from both lanes).
- Count pins walked: tests.rs totals ×4 sites (incl. the stale-mutable 559 hiding in `batch116_rules_present` — the count-pin grep's exact-word trap), per-category expected table (A/B/D/H/N), router.rs:461, mod.rs category comments ×3 (incl. two PRE-EXISTING stale headers E 45→46 and the B125-era A/B drift), AGENTS.md 559→565, `.docs/11_domains/kernel_opt.md` count row.
- Score-bench combo fix en route: `src/bin/heal.rs` score_bench arm had a pre-existing signature-drift break (`v5_is_removal_rule(&str)` filtered as `fn(&String)` — the B125 landing changed the signature without building the score_bench combo; fixed with a closure, build re-verified).
