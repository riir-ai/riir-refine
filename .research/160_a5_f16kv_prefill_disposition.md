# Research 160 — A5 dtype-asymmetry lead dispositioned: f16-KV prefill on M3 is low-prior (the currency is load requests, not bytes)

**Status:** RECORD 2026-09-09 — a disposition note, not a new probe. The "both opponents run f16 KV while our M3 prefill reads f32" observation looks like the obvious A5 answer from the outside; this note records why three adjacent measurements bound it low, what the one unmeasured cell would need, and the reopen condition. Zero-build unit: greps + existing-bench reads only. League intel for 844/851; nothing filed cross-repo.

- **Trigger:** continuing Research 159's A5 narrowing (KV-traffic efficiency at Θ(P²) scale). External-symmetric observation: the M3 opponents run **f16 KV by default** (llama.cpp GGUF default KV = f16; MLX default = f16) while our production M3 prefill kernels are hardwired `*_f32` (`qwen_attention_prefill_tiled_m16/m32_cubecl.rs`, no dtype arm). Same Θ(P²) KV traffic, half the bytes per element — a structural asymmetry shared by BOTH opponents, growing exactly where our gap grows (0.546× @16K → 0.507× @32K).

## The disposition chain (why it's low-prior)

1. **Decode-side f16 was measured a LOSS.** `QWEN38_KV_DTYPE=f16` ships as an opt-in on the CUDA dense path (`qwen38_dense_cudarc.rs`, Issue 753; Benches 751/756) — Plan 548 P2-1 **DECIDED 2026-08-29: NO FLIP — decode row −3.4% under f16**; the opt-in survives only for the long-ctx C-axis consumer, with the recorded reopen condition ("a production long-ctx verify-loop consumer").
2. **The M3 q8-KV prefill cell was a WASH — with a confound that does NOT rescue f16.** Bench 837 (Issue 771 T2c-a): quartering KV bytes (f32→q8) on the plane kernel = steady wash @16K and @32K; "the K/V-byte-term premise does not convert on this substrate." The recorded confound is the O(P)-per-chunk requantize handing traffic back — f16 has no requantize (write-time dtype only), so the wash alone can't kill f16. But:
3. **The decisive prior: the promotions that WON attacked load COUNT, not bytes.** m16 (÷2 rows-per-load) → 1.207× e2e @16K; m32 (÷4) → the Bench-845 long-pass + re-cells (+3.7%-class) — all at **unchanged f32 bytes**. Meanwhile T1 (Bench 843) measured the kernel at **~194–230 GB/s over unique KV bytes ≈ half DRAM peak** — a kernel that leaves 2× bandwidth on the table is **latency/request-bound, not bytes-bound**; and "−KV floors at 2.3–2.8 TF (the serial chain wall)" confirms the loads serialize. f16 halves bytes per element but keeps **request count, addresses, and MLP width identical** (a 16-byte load is 8 halves or 4 floats — same transaction class). The currency this kernel is short of is not bytes.
4. **ALU is bounded away as the f16-conversion cost anyway** — T1: dots/exp ≤13% of the kernel; there is no dequant ALU to pay for f16 (unlike q8), but there is also no ALU win of consequence to harvest.

**Net:** on the current M3 m16/m32 shape, expected f16-KV gain ≈ small; the measured league gap needs ~3.5× on the attention stage to close at fixed GEMM (Research 159 arithmetic), which no bytes-halving delivers under a latency-bound regime.

## The one unmeasured cell (if anyone wants it)

An **m32-f16 arm** (KV cache + kernel loads at half, accumulate f32, no requantize pass, no dequant) is the only honest test of the byte axis on the production path — B837's plane-kernel q8 result cannot substitute (confounded; different kernel; quarter-vs-half). Cost: kernel dtype variant + cache allocation arm + the **lossy-surface gate set** (bits move ⇒ FNV anchors re-pin; argmax-identity + max_abs band per the Bench-773 rule + the NLL gates the q8 route already cleared at LOWER precision in B878's class) — i.e. exactly a T2c-class 844 build task, P3. **Do not build without a NEW mechanism argument** — the B837 closing rule applies verbatim: the prior is now lower than when q8 was tried, because m32 already harvested the request-count axis f16 cannot touch.

## Honest caveats

- Opponent KV dtype stated from engine defaults (GGUF/MLX f16), not verified against the pinned fork binary's cache allocation for this model; the fork SHIPS optional KV-quant features (per-channel K mean-centering, #51/#53) that the league harness does not enable — if a future opponent-watch pass finds KV-quant flags in the league manifest, this note's asymmetry premise flips and the cell re-ranks.
- B837's wash is one kernel (plane) + one dtype (q8) + one confound (requantize); this note's low-prior rests on the THREE-way agreement (loss on decode + wash on prefill + the m16/m32 request-currency evidence), not on any single one.
- The cmma smem-staging compounding hint from B837 ("q8+smem-staged-tiles would compound") is MOOT on the default path: cmma was demoted to opt-in by Bench 841, and m16/m32 read KV to registers without threadgroup staging.

## Dedup

- Issue 844's hypothesis list carried "f16-class **score math**" (compute dtype — bounded ≤13% by T1's ALU map); this note is about f16-KV **storage/traffic** — a different axis the lane's docs never dispositioned explicitly. Issue 844 non-goals kill the Q8-KV retread; this note extends the same wall to f16 with the request-currency argument.
- Research 159 (triangular-exact) + 157 (streaming design transfer) + this note = the A5 dtype/structure/class waste hypotheses now all carry recorded dispositions; what remains standing is 844's own live T2 (KV-traffic amortization, m64-class) and the sustained-thermal counter-decomposition question (157's candidate a).

**Highwater bumped 159 → 160 in the same commit.**
