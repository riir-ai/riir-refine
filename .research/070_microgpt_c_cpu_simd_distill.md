# Research 070: microGPT-C — CPU SIMD GEMV layout + exp-tier distill verdict

**Status:** RECORD — Batch 62 landed 2026-08-19 (corpus 65→67; both rules + measured verdicts in the corpus header + AGENTS.md). Measured corrections to this note's expectations: the lead rule's "wash at large nin" caveat was WRONG-DIRECTION (measured 1.18×/2.09×/**6.08×** — the win GROWS at large nin; in strict-FP Rust the row-dot's scalar accumulator is a serial dependency chain LLVM cannot vectorize without reassociation — the serial-chain latency, not the horizontal reduce, dominates; llama.cpp's fine row-major dots use explicit multi-accumulator SIMD, not portable scalar code); Rule 2 measured Mixed 1.09×/1.01× (hardware never loses; the M3's fdiv/sqrt latency flattens the gap at 100K vs the source's 2-4% on Ryzen).

**Source:** https://github.com/vixhal-baraiya/microgpt-c (MIT, 616★, 6 commits, complete — not WIP)
**Inspected:** 2026-08-18 — 100% of `src/microgpt.c` (~1,300 LOC, the only source file) + `docs/PERFORMANCE.md` + README. No unread files.
**Provenance chain:** karpathy's microgpt reimplemented in pure dependency-free C (char-level transformer, 4,192 params, N_EMBD=16, single layer; training forward + specialized single-token inference forward; AVX2 + NEON backends; 6.9M tok/s Ryzen 5 5600H / 10.1M tok/s M5 Pro — regime-bound to tiny dims, which is precisely where kernel shape dominates). All headline techniques are published prior art, cited by the source itself or canonical: Schraudolph 1999 (*A Fast, Compact Approximation of the Exponential Function*, Neural Computation) + the 2000 follow-up (RMS-optimal c=6.0801, ±3% rel error for the basic form); Fabian Giesen, *A small note on SIMD matrix-vector multiplication* (ryg blog, 2015 — the canonical transposed-lane Mv writeup); Gumbel-max sampling (Gumbel 1954 / Maddison 2014). Novelty claims below are **corpus-relative only** (verified by grep), never absolute.

## Verdict — YES, grade B → Batch 62 (rust_perf), Plan 008

**Verdict correction record (the LOPD/NQF user-challenge rescue class):** the initial verdict was MARGINAL B− (conversation-only). Corrected to YES B on user challenge after three specific errors were owned:

1. **A bar the corpus doesn't have.** The implicit requirement of a 2–5× GOAT anchor to open a batch conflates batch *leadership* with batch *viability*. Recent batch history is 1–2 rules as the norm (B49=1, B52=1, B54=1, B60=1, B61=2), and the corpus ships 1.1–1.5× rules routinely (`entry-vs-contains-insert` 1.5×, `box-str-immutable` 1.1–1.3×, `cow-conditional-alloc` 1.32–1.49×, `retain-inplace` 1.3–2.0×). A 1.1–1.4× regime-conditional rule is landable by the corpus's own standards.
2. **"No consumer" was an inference, never grep-verified — and the grep refutes it.** Live in-tree shapes the lead rule targets: `katgpt-types/src/simd/binary.rs` (`neon_binary_matvec` / `avx2_binary_matvec`: per-row loop ending `y[r] = vaddvq_f32(acc0)` / `horizontal_sum_256(acc0)` — "Single horizontal sum at the end (like ternary)", i.e. the ternary matvec shares the shape), `katgpt-types/src/simd/dot.rs` (row-dot kernels), and the `katgpt-types/src/math.rs` matmul family (`matmul`, `matmul_parallel`, `matmul_relu`, `matmul_f16*` → `simd_matmul_rows*`). CPU SIMD is a first-class maintained surface (AVX2/NEON/**wasm32** kernel families) — the browser peer runs on it.
3. **"Yield 0–1" treated as disqualifying when it IS the norm** — the skill's own falsification math (~50% kills, recent batches land 1–3) makes 1–2 expected, not disqualifying. Thermite precedent: mined at B+ with a 5–10× GOAT → landed exactly 1 rule.

Corpus rules do not need current consumers to be valuable — they are a pattern library fired by `--suggest`/`--fix` sweeps across all repos (thermite's `division-by-invariant-multiplication` had zero in-repo consumers at mining time). Consumer evidence raises confidence; absence was never a valid disqualifier.

## Ranked candidates

| Pattern | Source site | Grade | Disposition |
|---|---|---|---|
| **Transposed-weight register GEMV** — pack weights column-major once at weight-freeze; per input element broadcast x[c] and FMA into output-vector accumulators; zero horizontal reduction anywhere; 16/8/4-row tail tiles | `pack_t`/`infer_pack_weights`/`mv16_blk16v(4)`/`mv16_blk8/4` | **B+ — LEAD** | Mine (Batch 62) |
| **Newton-refined recip/rsqrt loses to hardware on CPU** — measured 2–4% slower than single-instruction `fdiv`/`fsqrt` on Ryzen 5 5600H; the GPU fast-math recipe (approx + refine, because GPU div is multi-cycle) inverts regimes on CPU; relaxed-precision estimate intrinsics (`rsqrtss` 12-bit, `frecpe`) are a separate axis valid only where accuracy can relax | PERFORMANCE.md dead-ends | **B (context/annotated)** | Mine as honest-annotation rule (the `filter-count-to-any` class: stops a plausible-looking GPU→CPU port) |
| Schraudolph bit-trick exp (`(int)(12102203.16·x·log₂e) + 0x3F800000`, no mantissa correction, ≈±3% rel err) | `fexpf`/`vfexpq` | C+ | Drop from batch; regime knowledge recorded below |
| Deferred RMS scale s² through positively-homogeneous MLP chain (fc1+ReLU²+fc2) | `rmsnorm_scale` + `gpt_forward_infer` | C+ | Drop; extends kernel_opt `delay-rms-scale-past-gemv-epilogue` (B29) but SiLU/GELU (our SwiGLU mainline) break homogeneity — inapplicable today |
| (token,pos) prefix table — embed + 2 norms + layer-0 Q/K/V precomputed over bounded vocab×BLOCK at pack time | `build_pretok` | Drop | Positive instance of existing `precompute-table` at macro granularity (B50 discipline: instances are kills); optionally enrich that rule's context_notes with the macro regime |
| Fused one-pass sampling (SIMD max + exp + temp + pad-with-logits[0]/sum-correction + CDF scan) | `sample_logits` | Drop | Covered strictly better: `katgpt_core::ac_prefix::gumbel_max_sample` needs no normalizer at all; `branch_routing` ships the Logistic-noise sigmoid analog |
| Split accumulator sets (2/4 FMA chains) | `mv16_blk16v4` | Drop | Ambient SIMD folklore; mechanism already documented in `f32-counter-to-int`'s rationale; `split-sign-accumulators` (B17) covers the corpus-worthy variant. Source's own caveat adds the register-pressure regime: x86-64 has 16 vector regs vs ARM64's 32, so the same source gives x86 half the chains; 4→8 chains measured flat on 5600H |
| Vocab pad-to-4 tail tiles | `mv16_blk4/8/16` | Drop | Thin/ambient |

Other measured dead-ends from PERFORMANCE.md worth carrying as rule context: fp16 weights (FMLAL halves bytes not ops → +1 cycle of 241; fp16 accumulate +5% for 3 orders of magnitude logit-accuracy loss — the accuracy axis dominates); breaking token-to-token dependency (no gain — nothing waits on it); batching is the ceiling-lifter (batch-1 GEMV has zero weight reuse).

## Dedup-grep evidence (both vocabularies, 2026-08-18)

- `schraudolph|fast[_-]?exp|exp[_-]?approx|12102203|1065353216|fexpf` over `riir-clippy/src/**` → **0 hits**. But `katgpt-types/src/simd/activations.rs` ships `fast_exp = cephes_exp_scalar` (~1 ULP, ~1.7× libm aarch64, `(n+127)<<23` bit-trick for the 2ⁿ scale only) + `fast_sigmoid` + full AVX2/NEON/wasm32 exp/sigmoid/tanh families — the **high-accuracy tier is shipped and pervasive** (Research 315 documents the sigmoid-not-softmax enforcement on this infra).
- `column[_-]major|transposed weight|horizontal reduction|lane[_-]strided|plane_sum|gemv_plane` over `riir-clippy/src/**` → hits ONLY in kernel_opt (the GPU plane-per-row family: `single-plane-sufficient-reduction`, `workgroup-equals-plane-native-reduction`, `epilogue-at-register-write-phase`, `row-tile-amortize-input-load` B18). **Zero rust_perf hits** — the CPU-side rule is new to the corpus; the GPU twins are cited as distinct-by-substrate.
- `accumulator (set|chain)|split accumulator|fma chain|dependency chain|register pressure|independent chain` → kernel_opt `split-sign-accumulators` + rust_perf `f32-counter-to-int` rationale. Ambient → kill upheld.
- `o1-lookup|lookup table|memoiz` → `o1-lookup` (data-structure query) + `precompute-table` (table-vs-recompute, honestly annotated ParetoDominated for cheap ops) + `prefix-sum-2d` (mathematical pre-computation). The prefix-table = macro instance of `precompute-table` → kill upheld.
- `gumbel|multinomial|fn sample_logits|weighted choice|sample_token` over katgpt-core → `ac_prefix::gumbel_max_sample` + `branch_routing` perturbed-argmax. Sampling covered → kill upheld.
- `newton|rsqrt|reciproc` over riir-clippy/src → only `regularizer-above-noise-floor-coupled-to-damping-cap` (Newton-Schulz iteration robustness — different concern) + popcorn `upcast-before-reduction`. **No CPU transcendental-refinement rule** → the annotated rule is new.

## Why the Schraudolph tier stays out (record for future re-opens)

Our tier is covered by Cephes at the accuracy we need. The lower tier's real catch, visible only by reading the source: microgpt **trains** with `fexpf` in attention softmax, so the model adapts to the ±3% approximation; we infer externally-trained frozen weights and cannot adapt — swapping exp tiers changes semantics measurably on quality-sensitive paths. Additional hazards if ever reopened: the unclamped trick produces **negative garbage floats** for x < −87.3 (int bit pattern goes negative); it is monotone (ranking survives — relevant to the FV sigmoid-ranking theorems); RMS-optimal constant per the 2000 follow-up. Re-open trigger: a measured need for a sub-Cephes exp tier in a latency-bound, accuracy-tolerant path.

## Caveats

- **No measured A/B in the source** between row-major and column-major layout (training forward is row-major dot16; inference column-major; never compared). Magnitude for the lead rule is mechanism-level (eliminates per-row horizontal reduction + enables output-register accumulation + sequential weight streaming; ryg 2015 analysis) — our Pareto bench supplies the measurement. Expect regime-conditional verdicts (wins at small/mid nin where reduction cost is material; wash at large nin — llama.cpp row-major dots are fine at 4096+). If the bench lands Mixed, the rule ships WITH the honest annotation (the `soa-hot-field` precedent) — the framework working as designed.
- **Float sum-order changes** under transposition → NOT bit-identical. The bench gate needs a rel-error tolerance strategy, stated up front (precedent: `distance-sq-no-sqrt` 1-ulp boundary caveat).
- Ternary/binary **bit-packed** weights are a different layout problem — the rule targets the f32/f16 family (`math.rs` matmul, `binary.rs` f32 matvec path); applicability to the bit-packed ternary matvec is out of scope.
- The rule is architectural (weight storage packs transposed at model-load; per-call `weight: &[f32]` params can't pack in place) — same class as `soa-hot-field`.
- Drive-by finding (not a corpus item): `katgpt-dec/src/simd.rs::fast_sigmoid` uses libm `(-x).exp()` while `katgpt-types` uses the shared Cephes kernel — same-shaped duplicate at the slower tier; ~2-line heal when someone is next in that file.

## Queue slot

**Batch 62** (rust_perf), **Plan 008**, lead `transposed-weight-register-gemv`. Expected yield after coordinator falsification: **1–2 rules** (+ optional `precompute-table` context-notes enrichment).
