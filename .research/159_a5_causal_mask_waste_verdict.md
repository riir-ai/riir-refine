# Research 159 — A5 hypothesis eliminated: our prefill attention is already triangular-exact (no causal-mask waste to reclaim)

**Status:** NEGATIVE (source-read refutation, 2026-09-09) — the "our kernel wastes ~2× FLOPs on causally-masked positions" class of A5 explanation is DEAD for the M3 league path. Zero-build unit: pure kernel-source read + arithmetic; no probe needed or run. League intel for the A5 question (851/876/844 lanes + Research 157's trigger); nothing filed cross-repo — the 844 owner may cite this note as A5-narrowing.

- **Trigger:** the A5 open question — ours sits 0.546× @16K / 0.507× @32K vs BOTH llama.cpp and MLX uniformly, so the gap is in something both opponents share at long P. A candidate hypothesis of exactly that shape: a ~2× gap at from-scratch prefill is what you get if our attention scans the full S_q×S_kv rectangle while both opponents block-skip the causal mask (exact-triangular work is p²/2 of the rectangle). Checked before anyone proposes "add triangular skipping" as an A5 fix.
- **Method:** kernel-source read of the M3 production prefill-attention family (`riir-ai/crates/riir-gpu/src/qwen_attention_prefill_m16_cubecl.rs`, `..._m32_cubecl.rs`; the plain tiled kernel shares the structure per the m16 module doc) + arithmetic. No build, no run — the kernel structure itself is the proof, and its no-op semantics are already test-pinned (FNV anchors `99a0733c45a0e663` @2048 / `c152813ee93aaa2a` @16K asserted by `tests/bench_808_issue771_flash_m16.rs`).

## The verdict

**The skip is shipped.** Every kernel in the family bounds its KV scan per query-cube, not per model pass:

```text
q_last = q_tile * Q_PER_CUBE + (Q_PER_CUBE - 1)
n_loop = min(q_last, p - 1) + q_offset + 1     // m16 L167-172; m32 L204-213 same
```

A cube covering rows `[16c, 16c+15]` (m16) scans exactly to its last row's causal frontier `16c+15+q_offset+1` — NOT to `p`. Summed over all cubes the scanned work is `p²/2 + O(p)`, i.e. exact-triangular up to the cube-widening edge:

| kernel | rows/cube | wasted iterations per cube | total waste | share of p²/2 @16K | @32K |
|---|---|---|---|---|---|
| plain tiled | 8 | 28 | 3.5·p | 0.04% | 0.02% |
| m16 | 16 | 120 | 7.5·p | 0.09% | 0.05% |
| m32 | 32 | 496 | 15.5·p | 0.19% | 0.09% |

Out-of-causal iterations that DO run (the ≤15/31 intra-cube edge) are bit-exact no-ops by construction — masked = −1e30 → w = exp(−1e30 − finite) = 0, correction = exp(0) = 1, and `x·1 + 0` / `o·1 + 0·v` are exact in IEEE (m16 module doc L28-33; m32 L21-22) — so they cost only the shared K-row load (already paid once per cube) plus the per-row dot/exp FLOPs counted in the table above. Cache-continuation cells (base_pos ≫ p, q_offset large) have no mask at all — every query row genuinely attends to the whole prefix — so there is nothing for US to skip and nothing the opponents could be skipping either.

## Corollaries

1. **"Add triangular skipping" would be a no-op.** Any future A5 brainstorm or heuristic-mining pass that proposes causal block-skipping for our prefill is re-discovering shipped substrate (the m16 doc even notes the plain tiled kernel's `n_loop` already spans the whole cube). Record here so nobody spends a probe on it.
2. **Where the A5 gap still lives** — consistent with the already-measured decomposition, all cited from the kernel module docs themselves: attention = 63.2% of prefill wall @16K (Bench 792), tiled kernel ≈2.7× above its compute floor @16K (Bench 800), KV loads 41→63% of flash wall monotone in P (844). I.e. **KV-traffic DRAM efficiency at Θ(P²) scale**, not FLOP waste — exactly the lane 844 is working (m16 traffic-÷2 → 1.207× e2e @16K; m32 ÷4; the streaming counter-decomposition candidate per Research 157 for the sustained-thermal regime). This note narrows; it does not redirect.
3. **Scoping caveat:** the verdict covers the Metal tiled-flash family — the M3 league cells (the 0.546×/0.507× numbers). The CUDA twins (`att_pf_mq8*` in `prefill_cuda_attention.rs`, the 4090 lane) were NOT audited this pass and feed different league cells; same-shaped check there is a 15-min follow-up if a 4090 long-P cell ever opens a similar question.

## Dedup

- Not a ripgrep-vocabulary case: the mechanism ships under its literal name (`n_loop`, "loop-bound widening", "bit-exact no-op") in the kernel docs — but it was recorded there as a NUMERICS/design claim, never as A5 league intel (grep of `riir-ai/.plans/844*` for triangular/causal/waste/n_loop: zero hits, 2026-09-09). This note is the league-facing record of the elimination; the kernel docs remain the implementation truth.
- Research 157's A5 framing quoted the open question; this closes one of its hypothesis classes. No conflict — 157's candidates stand.

**Highwater bumped 158 → 159 in the same commit.**
