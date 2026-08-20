# DFlash 2 — Draft-Only Distillation (and why "drafter alone" cannot generate)

**Date:** 2026-08-20
**Sources:** Inco AI blog ["DFlash 2: Keep Drafting Parallel"](https://inco.ai/blog/dflash2/) · ggml-org/llama.cpp PR [#27342](https://github.com/ggml-org/llama.cpp/pull/27342) @ `5ecbe1a` (built locally at `/Users/katopz/git/llama.cpp`, branch `dflash2`, Metal) · `z-lab/Qwen3.8-27B-DFlash2-GGUF` Q4_K_M (1.14 GB, on disk at `riir-train/data/`) · DFlash paper (Chen et al., ICML 2026)
**Status:** DISTILLED — POC-A plan filed ([Plan 017](../.plans/017_dflash2_selection_headroom_poc.md) + [Issue 034](../.issues/034_dflash2_selection_headroom_poc.md)); the artifact-based L4 arm (POC-B) is **DEFERRED — show-stopper** (owner call, 2026-08-20: parasitism + the in-house DSpark precedent, §2.1). Only the DFlash2 *primitive* (selection headroom) stays active. Weights kept on disk, unused.

## TL;DR

DFlash 2 is a block-diffusion *drafter* for speculative decoding: it predicts a whole
8-token block in parallel, keeps the **top-16 candidates per position**, and a +2.0M-param
**path selector** traces one coherent path through them (`S_t(a,b) = U_t(b) + ⟨A(a)⊙H(h_t), B(b)⟩`,
a low-rank bilinear score over adjacent candidate pairs). A +16.5M-param two-tap dynamic
depthwise convolution fixes suffix decay. Net: +1 token/pass (~21%) over DFlash at +1.3% cycle latency.

The owner's ask — *run the drafter alone (no Qwen3.8-27B target) as the `cargo heal` L4* — is
**architecturally blocked by the released artifact**, not merely quality-degraded (§2, source
receipts). What survives without any target weights is the blog's load-bearing *insight*:
**selection headroom** — the right answer is usually already in the top-K; top-1 selection is
the bottleneck. Our healer exhibits the exact same phenomenon (Bench 024 pool crowding). POC-A
ports the selector mechanism onto our modelless candidate flow; POC-B (deferred) does the
artifact-based L4 with the target.

## 1. What DFlash 2 is (blog distill)

Two additions over DFlash (v1), both measured on five-layer Qwen3-4B/GSM8K:

1. **Path selector** (+2.0M params, +0.6% latency). DFlash predicts every block position
   independently; picks are individually plausible but jointly incoherent (stutters die at
   verification). Evidence the fix is *selection, not generation*: position-0 **Recall@1 = 85.4%
   but Recall@16 = 99.5%** — an oracle picker over the existing top-16 lifts acceptance 4.27 → 6.79.
   The selector keeps top-16 per position and scores every **adjacent pair** in one parallel shot
   (predecessor embedding `A(a)`, context gate `H(h_t)`, candidate embedding `B(b)`, 256-dim) —
   "low-rank bilinear attention over adjacent candidates". Beats DSpark's sequential correction
   head (+77.8M, +9.6%) with ~40× fewer params and 16× lower overhead.
2. **Two-tap dynamic depthwise conv** (+16.5M, +3%). Suffix decay (per-position recall fading
   toward block end) is a *local* problem: within-block attention mass already collapses
   30%→8% by layer 5. `Conv_k(x)_t = k_{t,0}⊙x_t + k_{t,1}⊙x_{t-1}` inserted before/after each
   sublayer recovers most of what +10 layers buy (77.61% vs 78.73% at the last position) at
   1/5 the cost.

Qwen3.8-27B drafter, block 8: mean acceptance **4.80 vs MTP 4.28 / DSpark 3.62**; code benchmarks
(HumanEval 4.39, MBPP 4.79) lead too. GGUF quants: Q4_K_M acceptance **5.39** (≥ BF16's 5.28) —
the Q4 is the recommended operating point. Throughput: 2.7–3.4× AR at bs=1 (SGLang), ~1.8× on
Apple M-series via llama.cpp PR #27342.

## 2. Why the drafter cannot run alone (source receipts)

The GGUF is parasitic on the target **by construction** — three independent blockers, all verified
in the llama.cpp source at `5ecbe1a`:

| # | Blocker | Receipt |
|---|---|---|
| 1 | No token embedding table — pulled from the target at graph build | `dflash.cpp:539-547`: `tok_embd` via `ctx_other`, `GGML_ASSERT(cparams.ctx_other != nullptr)` |
| 2 | No LM head — same ("reduced-vocab drafts ship their own, full-vocab drafts share the target's") | `dflash.cpp:203` + `650-658`: `output` via `ctx_other`, same assert |
| 3 | The encoder's **only** context input is the target's hidden states | `dflash.cpp:25`: `n_embd_inp_enc = target_layers × n_embd` = **5×5120 = 25,600 floats/position**; encoder = `fc` fusion → RMS norm (`dflash.cpp:266-278`) |

Context understanding flows **entirely** through path 3: the decoder's per-cycle batch is
`[last verified token, mask × (block_size−1)]` (`common/speculative.cpp:1187`) — the drafter never
sees the prompt as tokens, only as target features. Standalone context creation refuses outright:
`llama-context.cpp:157` `"dflash requires ctx_other to be set"` (independently reproduced in the
PR thread by ThConMan). A zero-state or self-state "cheat" patch produces an unconditional LM that
knows only the last token — information-theoretically blind to the span being healed.

**Consequence:** ~1.5B params of embedding+head live only in a Qwen3.8-27B checkpoint. Any
artifact-based use of this drafter (including POC-B) requires the target — which the owner has
explicitly deferred (weights that exist locally are kept but unused: the DFlash2 Q4_K_M drafter
for POC-B; `Qwen3.6-27B-UD-Q4_K_XL` was ruled out — wrong model generation, distillation targets
Qwen3.8-27B states specifically).

### 2.1 Precedent — we already ran this experiment class with DSpark (closed-negative, all axes)

The owner's show-stopper call is backed by in-house measurement, not just the source analysis.
The DSpark drafter artifact (`Ternary-Bonsai-27B-dspark-Q4_1.gguf`, the 6-layer block-denoising
drafter + trained Markov head) is the **same architectural family** — katgpt-rs Bench 656 names
it explicitly: *"**DSpark = DFlash pattern**"*, and its failure-mode table marks the
**separate-drafter family: DFlash, DSpark** as **STRUCTURAL** failure mode 2 — *"drafter forward
cost doesn't amortize at batch-1"* (loses on Apple Silicon; 1.34× only on 4090). Our L4 heal
loop is exactly batch-1 single-request usage.

The full artifact chain was already ported and measured (riir-ai Issue 717, Benches 693-697):
the llama.cpp DSpark contract end-to-end (encoder fusion of target layers `[1,16,31,46,61]` →
per-layer KV injection → non-causal noise block → markov-bias greedy chain + confidence head,
with `forward_token_with_layer_capture` extracting the target features — the same target
parasitism as §2), and the verdict was split-negative: TRUE acceptance with the target beat the
baseline (+17.4%) but the **corpus-proxy quality lost to the modelless bigram table** (0.185 vs
0.065) and the **wall-clock closed-negative at the chain seam on BOTH GPU substrates** (G3a
0.326x/0.252x Metal; G3b 0.84x/0.32x vs the 87 tok/s CUDA-Graphs baseline on 4090). The
modelless replacement (bigram Markov head, Bench 663: 23 ns/step vs ~129 µs/step, ~5,600×
cheaper) shipped as the winning drafter primitive.

**Lesson applied here:** a target-conditioned separate drafter in a batch-1 loop is
structurally wrong for our economics, twice over — it cannot run standalone (§2) and it does
not amortize with the target (Bench 656/696). The DFlash2 Q4_K_M artifact is this same class
with a newer selector; no new test is needed to re-establish the class verdict. Reopens only
if a standalone-capable drafter ships (own embd+head) or a concrete multi-request consumer
changes the batch-1 premise.

## 3. Fusion — selection headroom is a phenomenon we already measured

**Vocabulary translation** (paper → our stack):

| DFlash 2 term | riir-clippy equivalent |
|---|---|
| draft block | fix candidate for a span |
| top-16 candidates per position | retrieval pool (`RERANK_POOL = 12` post-Issue 024) |
| Recall@1 vs Recall@16 | top-1 vs top-K rule-hit on the eval fixture sets |
| path selector `S_t(a,b)` | pairwise coherence between candidate fixes (absent today) |
| acceptance length | heal_rate / verified-fix rate |
| verifier (target) | `clippy_verify` / `fix_verify` cargo gate |

**Bench 024 is our Table 1.** The cross-domain switch-cost investigation found the correct rule
crowded to raw-similarity rank 9 by loop-shaped rules scoring 3× its solo winner — Recall@1
failed, Recall@12 held — and deepening the re-rank pool rescued it (0.0pp both directions
post-fix). Issue 017 measured top-1 58.3% at scale before the T3.5.5 instantiation re-ranker
lifted it to 87.5% (structural) / 96.0% (composite) — i.e. **~29-38pp of our healer's own
history was selection headroom, converted by a selector**. The T3.5.5 re-ranker selects by
*individual* instantiation fit; the DFlash 2 mechanism it does NOT implement is **pairwise
adjacent-candidate coherence** — scoring candidate *pairs* under a context gate and tracing a
path, which is what multi-edit composition (overlapping spans, conflicting patterns across the
fixpoint pass) needs. That is POC-A.

The honest caveat, recorded up front: post-T3.5.5 residual headroom is the remaining ~4-13%
(top-1 87.5-96.0%), so the gate must be sized to that residual (Plan 017 T1 measures it first;
stop rule if < 2pp).

## 4. Routing verdict

- **POC-A — Gain, modelless, this repo, now.** DFlash 2's selector mechanism (keep-K +
  bilinear pair coherence + path trace) over our retrieval pool, composed with `BetaPosterior`
  selection (not replacing it). Gates in [Plan 017](../.plans/017_dflash2_selection_headroom_poc.md).
  No model, no target weights, stays µs-scale.
- **POC-B — DEFERRED, show-stopper (owner call 2026-08-20).** DFlash2-accelerated Qwen3.8-27B
  as an `L4Fallback` (llama-server HTTP), re-running the Bench 039 reachable-set harness where
  the bonsai adapter scored WIN 5 / REGRESSION 4. Blocked by §2 (parasitism: needs the ~17 GB
  target for embd+head+features) AND the §2.1 precedent (the DSpark artifact class measured
  closed-negative in our stack — structural batch-1 non-amortization + wall-clock FAIL on both
  substrates). Staged infra kept for a future reopen: llama.cpp dflash2 build
  (`/Users/katopz/git/llama.cpp` @ `5ecbe1a`, Metal) + drafter Q4_K_M (`riir-train/data/`,
  acceptance 5.39 per model card). The "draft quality" question for the *artifact* stays
  unanswered by choice — the class verdict answers it.
- **katgpt-rs side-note:** the selector/conv ideas are also relevant to the Issue 717/721
  speculative-decode line (tree-drafting reopen path), but the no-GOAT-without-consumer rule
  applies — out of scope for this note.

## PoC addendum

(to be filled when Plan 017 gates run — raw numbers, confirmed vs refuted axes)
