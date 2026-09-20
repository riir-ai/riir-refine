# Research 194: FWHT Staging Ladder — the Second Butterfly Instance Landed (Staged Lead Promoted)

> **Source:** the staged opponent-watch lead in `.distill/001` (2026-09-17 grading: PrismML-Eng/llama.cpp #166 `1bc46a8a7` "FWHT block-per-row for widths ≥512", rider `6f1c6908d` ">2048 widths") + the in-tree promotion trigger: riir-ai Plan 600 / Issue 980 (the folded-prefill rotation lane, landed 2026-09-19, read-only citation).
> **Date:** 2026-09-21
> **Status:** RECORD — the 09-17 stage condition ("stage if a second butterfly-transform instance lands") is **MET**; the FWHT staging-ladder rule is **PROMOTED** from staged-lead to the kernel_opt mining queue (next batch candidate). No corpus change this session.
> **UPDATE 2026-09-21 (later same day) — BATCH 166 LANDED:** the rule shipped as `fwht-butterfly-staging-ladder-warp-smem-register` (entries_gpu_45.rs, B: Compute 65→66; corpus 619→620) — see the ADDENDUM at the bottom.
> **Classification:** Private (riir-clippy)
> **Related Research:** 569 (Hadamard MLP / Monarch / Kronecker — the CPU butterfly map: `kvarn::hadamard`, `meld::walsh_hadamard_in_place_normalized`, `kron_tile` WHT fast path), 65 (RotorQuant — WHT-butterfly quant-rotation lane), `.distill/001` watch rows 09-07 / 09-10 / 09-17 (the fork series; #137 CSE, #161 Metal threadgroup FWHT, #166 CUDA ladder).

## TL;DR

Two days after the 09-17 grading said "NO runtime Hadamard in our production path — STAGE, don't admit", riir-ai Plan 600/Issue 980 landed exactly the second instance the staging condition waited for: a **default-on (`bonsai2_hadamard` in riir-gpu, promoted 09-19) GPU runtime-Hadamard lane** — `deltanet_rotation_cudarc.rs` (CUDA) + `deltanet_rotation_cubecl.rs` (the Metal twin), a batched `[p × width]` fwht fwd/inverse pair, and a fused rotate+quantize family whose every member reproduces the same butterfly verbatim. The benchable axis is now **ours** (sm_89 decode + the M3 Metal twin), not the fork's H100. The staged rule — the FWHT staging ladder (shuffle below warp width, ≤3 stages through smem, top stages register-resident) — is a real kernel_opt batch candidate with a measurable in-repo consumer. This note distills the rule's decision content, prices it against published prior art, and maps applicability; the batch itself is the next kernel_opt unit.

## 1. The decision content (the staging ladder)

For a width-W in-block butterfly (W = block_size, one transform per block):

| Stage span | Fork's placement | Cost mechanism |
|---|---|---|
| span < 32 (warp) | warp shuffles | no `__syncthreads`, no smem round-trip |
| warp ≤ span ≤ ~3 stages up to block width | smem | cross-warp communication only where required |
| top stages | **register-resident** (each lane holds N/32 contiguous elements) | zero syncs on the most-data-heavy stages |

Fork evidence: the incumbent warp kernel held N/32 registers per lane and synced 2×/stage — the register-resident top is the win; block-per-row for W ≥ 512 measured **+17% / +11% decode** (9B/2B, H100, folded bands); Hadamard was the **largest single decode kernel (25.8% of GPU time at block 4096)** in their profile. Riders: `6f1c6908d` extends the family to W > 2048; #161 is the Metal threadgroup variant for folded widths 512–2048 (**+4–36% decode on M5 Pro, but +2.2–4.8% pp — a measured quality cost on their side**).

## 2. Published prior art (the class is well-published — priced, not novel-claimed)

- **HadaCore** (arXiv:2412.16471, Dec 2024; Meta/PyTorch blog) — tensor-core-accelerated FWHT, SOTA on A100/H100. Different execution axis (tensor cores / matrix-formulation vs the SIMT register ladder) — and the direct published anchor for the observation that our own prefill lane converged to the same direction (§5).
- "Optimized Fast Walsh-Hadamard Transform on GPUs" (massively-parallel, memory-hierarchy-exploiting FWHT) and the CUDA-SDK-era variants — the classic prior art for the smem/shuffle staging idea itself.
- J. Andersson, Chalmers 2025 thesis — numerically stable GPU WHT implementations (the numerics axis: stage reordering changes summation order).
- **QuaRot** (arXiv:2404.00456, NeurIPS 2024) and **QuIP#** (arXiv:2402.04396) — the *consumer class*: online Hadamard rotations for outlier suppression in quantized LLM inference; upstream llama.cpp carries a QuaRot support PR — the runtime-FWHT serving lane is mainstream, which is exactly why the fork carries its own kernels and we now carry ours.
- `hadipourh/fwht` (C99, CPU) — CPU-side reference, adjacent to our shipped CPU butterflies (Research 569's map; no intake).

Verdict: corpus rules are decision content + bounded fix-space, not novelty claims — the ladder is admissible on mechanism + measured in-repo applicability. The prior art's role here is expectation-pricing (the class is real and active) and naming the tensor-core arm as a future GOAT competitor (§5).

## 3. The promotion trigger, verified in-tree (2026-09-21)

riir-ai `crates/riir-gpu/src/deltanet_rotation_cudarc.rs` (cited read-only; sibling-owned, no edits this session):

- `fwht_rotate_forward_f32` / `fwht_rotate_inverse_f32` — the split decode kernels: **one Hadamard block per CUDA block, 256 threads, `__shared__ float smem[1024]`** — sign-first load (`smem[i] = v * signs[base + i]`), **all log₂(1024) = 10 stages through smem** (`(a±b) * inv_sqrt2` per step), writeback (inverse multiplies signs on store). This is precisely the all-smem shape the ladder replaces.
- Batched prefill variants: `fwht_rotate_forward_batched_f32`, `fwht_rotate_inverse_batched_f32`, `fwht_forward_copy_batched_f32`.
- The fused rotate+quantize family — `rmsnorm_sign_fwht_quantize_f32`, `gdn_input_rotabq_f32`, `gdn_out_norm_gate_permute_rotq_f32`, `gate_sigmoid_sign_fwht_quantize_f32`, `swiglu_sign_fwht_quantize_f32` — each documents a **bit-identity contract: "butterfly: fwht_rotate_forward_f32 verbatim"**. Any ladder rewrite must either rewrite all five fused bodies in lockstep or keep the shared butterfly a device-inline function both call.
- `gemm_dense_ab_batched_f32` — the **prefill winner**: dense a/b GEMM formulation, 2668 → 3402 → 3606 tok/s (Plan 600 C0.5/C0.5-rung2/985), **1.1269× the fork's folded prefill, the ≥1.10 league gate GREEN** (`f27136a8c`).
- `bonsai2_hadamard` is **default-on in riir-gpu** (manifest note: 2026-09-19, Plan 600 B5/T5 promotion). The lane is production posture, not an experiment.

## 4. Applicability map (where the rule's fix-space bites)

1. **Decode split kernels** (`fwht_rotate_{forward,inverse}_f32`): 10 sync-separated smem stages today → ladder cuts ~4 to shuffles, keeps ~3 in smem, moves the top ~3 register-resident. The fork's +17/+11% was measured on exactly this shape class (W ≥ 512, block-per-row).
2. **The fused family (5 kernels)**: same butterfly inline — the rule applies via the shared device-inline body; the bit-identity contract (§3) is the gate, not an obstacle.
3. **The Metal/CubeCL twin** (`deltanet_rotation_cubecl.rs`): the M3 is this box's silicon; fork #161 measured the Metal variant at +4–36% decode on M5 Pro (folded widths 512–2048) — the second benchable axis, locally runnable.
4. **W > 2048 rider**: none of our kernels exceed W=1024 today (`block_size: 1024` in the G1 parity test); the rider lands only with a wider-block model.
5. **Inverse-embedding + GDN permute arms** ride the same family; no separate rule.

## 5. Fusion

- **HadaCore × our dense-ab convergence (the interesting one):** our prefill lane independently converged toward HadaCore's thesis — express the Hadamard as GEMM work (`gemm_dense_ab_batched_f32`, 1.1269× GREEN) rather than a butterfly. Complementary regimes, now both published-and-shipped: GEMM/tensor-core formulation for prefill (large-batch, tensor-core-rich), the SIMT register ladder for decode (latency-bound, small-batch). The rule's bounded fix-space stays the ladder; a tensor-core FWHT arm on sm_89 is recorded as a **future GOAT competitor axis**, not this batch's scope.
- **Healer-consumer answer (investment #2):** the rule lands in the kernel_opt corpus (619 → 620 on admission); GOAT = G1 syn-parse + G2 lib tests + G3 clippy both feature states; retrieval-floor re-measure per the kernel-batch protocol. No drafter/retrieval/trajectory consumer exists — stated, not silently skipped; corpus growth IS the healer surface for kernel rules.
- **Game-context answer:** no per-NPC scalar; the surface is the inference-perf league (investment #3) — the decode-hold defense (the 2.8% margin) is the standing driver, same lane intel as the 09-17 row.
- **Dedup state:** `fwht|walsh|hadamard|butterfly` corpus grep → **0 entries** (re-verified 2026-09-21). The 09-17 row's rider (b)(i) — "EXACT sign-vector MUL folds into the neighboring transform kernel's LOAD path" — is **already shipped** in our kernels (sign-first smem load, and signs-on-store for the inverse): retired at intake, not a batch rider.

## 6. Numerics caveats the batch must carry

- **FFMA contraction precedent:** riir-ai `d07542ee3` — the warp-butterfly ULP divergence was ptxas FFMA contraction, asm-guarded. The ladder *regroups arithmetic per stage*; the G1 parity/byte-identity pins (`g1_bonsai2_rotation_parity`, the fused-family bit-exact tests) are the gate. A ladder rewrite that cannot hold them verbatim is a numerics change and needs the lossy-surface walk (per-family conditional retention), never "looks fine".
- **Fork's measured quality cost on Metal:** #161's +2.2–4.8% pp is *their* measurement on their variant; it prices the risk, it does not transfer. Our parity pins decide.
- The fork's +17/+11% are H100 numbers — expectation, not evidence. Our GOAT numbers come from the batch's own gate on sm_89 + the M3 Metal twin.

## 7. Next unit (not this session)

Protocol A batch, plan-first: dedup (done, §5), entry authoring from the .distill staged row + §1 table, standing syn-parse gate extension, kernel_opt GOAT (G1–G3 + retrieval floors), score-bench near-FLAT expectation, research-note cross-links, `.distill` + AGENTS count updates, commit+push. Batch number: read `.plans/.highwater` at write time (dual-allocation discipline — a sibling landed Batch 165 / Plan 156 on 09-20).

## What is NOT claimed

No measured gain on our silicon yet — §1's numbers are the fork's; §3's 1.1269× is the prefill GEMM cell, not the ladder. The batch's GOAT gate is where the claim gets made or dies. riir-ai is sibling-owned: this session edited only riir-clippy files.

## ADDENDUM (2026-09-21, Batch 166 landing — Plan 157)

⛔ **The §5 dedup record was WRONG and the batch protocol caught it.** The claim "`fwht|walsh|hadamard|butterfly` corpus grep → 0 entries (re-verified 2026-09-21)" is false: the RECURSIVE grep over `src/domains/kernel_opt/` finds `fwht-shuffle-xor-butterfly-register-resident` (entries_gpu_11.rs — Batch 96 / Research 114 / Plan 060, distilled from the SAME fork's Metal FWHT kernel) plus butterfly-reduction riders in entries_gpu_01/04. Likely mechanism: a grep whose path glob does not recurse (`src/domains/kernel_opt/*.rs` matches only top-level files, missing the `corpus/` subdirectory) — a false CLEAN, the worse direction, and this workspace's own most-repeated lesson (a rule landed in one instrument but the sweep half of the dedup discipline is the grep ITSELF: run it recursively or it lies). The Batch-30 load-bearing-dedup filter exists for exactly this; the false clean would have shipped an UNADJUDICATED near-neighbor as "0 dedup hits".

**Adjudication (not a duplicate — complementary regime):** the Batch-96 rule owns the ALL-REGISTER transform (N ≤ LANES × register budget, zero smem, zero barriers; instantiations 64..2048 at 32 lanes) and explicitly punts the wider class — "transforms larger than the budget fall to smem tiles or multi-pass schemes" — WITHOUT content. The staging ladder IS that fallback's decision content. The landed entry carries the distinctness + composition note in-rule ("compose: all-register below the budget, the ladder above it"), pinned by `batch166_concepts_covered`.

**Landed state:** `fwht-butterfly-staging-ladder-warp-smem-register` (entries_gpu_45.rs; B: Compute 65→66; corpus 619→620). GOAT: G1 batch gates 3/3 + corpus suite 305/0; G2 lib kernel_heal 2390/0 + default 2037/0; G3 clippy `-D warnings` clean both states (one doc-comment `+`-at-line-start firing `doc list item without indentation` — fixed by rewording to "plus"); retrieval floors HELD (18/18 tests green; sweep clippy top-1 89.6% ≥ 80% bar; blended op-bigram 94.7/96.2 vs the 77.1/86.4/92.4 floors); score-bench heal_rate 96.4% ≈ FLAT vs the recorded 96.2% (kernel rules don't move the clippy fixture bench — the expected near-flat, the Batch-146/149 precedent). **Count pins walked ×9** — six in tests.rs (the total ×2 with messages, the exhaustive comment, the Compute pin 65→66, the sum, the grouped), one in router.rs:472 ("experts cover all 619" — found by the G2 full-suite run, NOT by reading: the 8th site was invisible to the grep that found the first seven), one mod.rs doc comment, the B-category header (which was ALREADY stale — comment said 64 while the pinned truth was 65; fixed en passant to 66 with a note), AGENTS.md ×2, .docs/11_domains ×2. Freeze tables untouched (kernel_opt corpus ≠ `build_seed_corpus()`). The ladder's OWN gain remains unclaimed until a riir-gpu parity-holding rewrite is measured — the corpus rule prices the decision, the riir-ai lanes make the claim.
