# Research 247 — the Research-239 follow-up pool conversion (arm quants.c CPU decode)

**Status:** DISTILLED — landed as Batch 212 / Plan 212 (rider `c<hash>`), the pool is now EMPTY.

## What the pool was

[Research 239](239_prismml_neon_pq2_0_vec_dot_distill.md) §Follow-up pool survived Batch 204's conversion with four sources: PrismML's `ggml_vec_dot_q2_0_q8_0` (the replication trick's in-fork origin), ggml-org upstream `ggml-cpu/arch/arm/quants.c` NEON kernels, the karusrus `arm-neon-ptq1_0` branch (the i8mm/SMMLA reference), and the SYCL XMX arm (watch-only, different ISA class). The pool carried the accumulation clause (sources read before the batch window fold in).

## Disposition table (every source re-verified at its pin 2026-10-08)

| Source | Pin | Disposition |
|---|---|---|
| PrismML `ggml_vec_dot_q2_0_q8_0` | PrismML-Eng HEAD `4fbda129` | **DEDUP** — the kernel's decode IS the B204 rule's owned structure (the in-file comment B204 quotes — "Same 2-bit codec as the Q8_0 path above" — refers to THIS kernel); its accumulation (`sdot` per 32 → `vmlaq_n_f32(sumv, vcvtq_f32_s32(p1), d0*d1)`) is the B127 per-block-int32-chunk-rescale-fp32-accum contract, which the B204 rule already cites as kin with the instance carried. Both mechanisms owned; nothing left to lift. |
| PrismML HEAD itself | `4fbda129` | **Already mined** — HEAD is the #290 merge ("arm: NEON + i8mm vec_dot for PTQ1_0"), the exact commit Batch 211's Rider 2 converted (identical numbers, identical identity). The fork's perf-relevant surface beyond it is the q1_0/q5/b2b family (below). |
| karusrus branch `arm-neon-ptq1_0` | `f9078a62` (2026-09-26) | **COVERED** — the branch is the pre-merge original of PrismML #290 (same author karusrus, same message body, same Snapdragon numbers). B211's Rider 2 owns the content; "pin before reading" satisfied — pinned, then deduped without re-mining. |
| ggml-org upstream arm quants.c | path HEAD `0c6a6a7c` (2026-09-28) | **RIDER + checked-not-mined** — the q1_0/q5 b2b-LUT family is the surviving lead (landed as the Batch 212 rider on `lut-free-bit-spread-vadd4-unpack`, below). The newest path commit (`0c6a6a7c`, "Enables Windows ARM64 build with MSVC cl.exe #28362") is C-syntax portability for cl.exe (compound-literal class: `(float32x4_t){...}` initializers) — off-domain for a Rust-idiom perf corpus; checked-not-mined. Older path commits (Q2_0 CPU #24448 2026-07-07, NVFP4 UE4M3-LUT #25331 2026-07-06, Q1_0 #21273/#21636 April) carry the kernels the fork file also carries — covered via the fork read at `4fbda129`. |
| SYCL XMX PQ2_0/PTQ1_0 `a14c7de9` | watch-only | **STAYS WATCH** — the pool's own disposition (DPAS 16-wide, different ISA class). |

## The rider (the batch's single conversion)

**`lut-free-bit-spread-vadd4-unpack` (kernel_opt) += the CPU face.** The law was mined from PrismML's CUDA `mmq-hopper-q1.cu` @ e311ed38: expanding packed sub-byte weight fields into byte lanes in a hot loop must NOT use a `__constant__` LUT — divergent per-lane indices serialize the constant cache (~1.7× end-to-end); ALU bit-spread instead. The same fork's CPU `quants.c` uses exactly the LUT form for the same expansion class — and the contrast is the content, not a contradiction:

- **The mechanism is GPU-specific.** `__constant__`-memory serialization requires (a) constant-memory semantics and (b) DIVERGENT per-lane indices in a warp. The CPU b2b form (`table_b2b_0[bits[k]]` → u64 → `vcreate_u8`) is SCALAR sequential byte-indexed loads: one index per load, no divergence, no constant cache — a 2 KB table pair resident in L1.
- **What it buys:** 8 expanded lanes per scalar load, replacing an 8-op shift/mask/sign-fold chain; the `{0,1} → {−1,+1}` fold then runs per VECTOR (`vsub/vadd` on int8x8), not per element.
- **One table pair, two consumers:** q1_0_q8_0 (full 1-bit value form, `table_b2b_0 = { B8(00, 10) }`) and q5_0/q5_1's 5th-bit mask via the INVERTED table (`static const uint64_t table_b2b_1[1 << 8] = { B8(10, 00) }; // (!b) << 4`, in-kernel comment verbatim "extract the 5th bit via lookup table ((!b) << 4)"; scalar `tmp[]` u64 staging then `vld1q_s8`).
- **The boundary is density-aware in-tree, not ISA-loyal:** the ternary kernels stay pure ALU (`tq1_0`'s `vhaddq_u8(q, q>>1) >> 6` base-3 identity — the same identity B211's rider carries; 17 `vhaddq_u8` sites; `tq2_0` shifts). 1-bit → LUT (8 lanes per load); 3-state → arithmetic.
- **The uniform-data benchmark trap does NOT transfer:** it exists because uniform bits make LUT indices converge and hide constant-cache serialization; the CPU face has no serialization mechanism for uniform bits to hide.
- **Upstream lineage:** #21273 (Q1_0 CPU, 2026-04-06) + #21636 (the follow-up that made the b2b-NEON form standing by replacing the scalar ARM impl with the generic fallback, 2026-04-20).

Count-neutral: kernel_opt stays 777; no rust_perf rule (a new rule would add a THIRD staged M3 GOAT to the quiet-box queue while two wait — the B209 single-rider precedent). All quotes read at the pin this session; clone `.raw/b212/prismml` (sparse, blob-filtered) removed at close.

## Adjacent records (checked, not mined)

- **`nvfp4_q8_0`'s masked-nibble decode** (`ggml_vqtbl1q_s8(values, vandq_u8(q4bits, m4b))` + `vshrq_n_u8`): B194 (`fp4-nibble-gather-lut-replicated-index-width`) already owns the improvement — its naive form is arithmetic per-code dequant, its fixed form replicates the table across index width to DELETE the pre-mask. The nvfp4 arm sits between the two (table + retained mask): an un-taken application of an owned law, upstream-side. No corpus edit owed.
- **`pq2_0_q8_0`** (fork-only packed variant between q2_0 and pq2_0_q8_K): same replication family as the two owned kernels; the B204 rule + the q2_0 dedup cover the mechanism class.
- **The tq1_0 `k_shift {1,3,9,27}` + multiply-by-{3,9,27,81} upper-trit ladder:** the base-3 SWAR family — owned by `swar-lane-widened-base3-dp4a-dot` (B116 family + the B211 rider).

## Yield vs the research's estimate

Research 239 estimated "1 rust_perf rule + 3 riders after ~50% coordinator falsification" for the ORIGINAL pool (Batch 204 landed exactly that). The follow-up pool yielded **1 rider, 0 rules, 0 additional riders** — the pool was mostly convergence: two of four sources had already been mined under other pins (#290 ↔ the karusrus branch), and the named q2_0 source dissolved into owned mechanisms on the read. The honest lesson for pool bookkeeping: a follow-up pool written at conversion time should re-check CONVERGENCE (has the upstream/fork graph collapsed the sources?) before pricing the batch — the yield collapse here was visible at pin time, not at read time.

## Gates (all green, census-polite posture)

- batch212 needle gate FIRST SHOT; batch20x family 47/47
- default lib 2477/0 (floor held)
- kernel_opt+latent_retrieval lib 3089/0 (+1 = the new gate; 3 ignored = the documented freeze-regen e2es); retrieval 21/21 floors HELD, no re-pin
- clippy `-D` clean at default + all-features (the `block v0.1.6` future-incompat note is the documented pre-existing one)

Batch 212 / Plan 212. Corpus: kernel_opt 777 (unchanged), rust_perf 184 (unchanged).
