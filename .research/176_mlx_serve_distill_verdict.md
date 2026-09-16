# Research 176 — ddalcu/mlx-serve distill verdict (v26.9.3)

**Status:** DISTILLED — pending owner decision (2026-09-16)

## Verdict

**YES — B+ corpus / LANE INTEL HIGH.** Route **`kernel_opt`** (GPU kernels +
host-orchestration-around-GPU), NOT `rust_perf` — the tempting wrong domain,
because the strongest candidates are host-side Zig controllers and the Batch-118
precedent already routes speculative-scheduling controllers to `kernel_opt`
(`speculation-scheduler-break-even-flip`, `entries_gpu_29.rs`).

## Source

- `https://github.com/ddalcu/mlx-serve`, release **v26.9.3**, inspected
  2026-09-16 at **`dcb0ede51dc5db3b291a7f9c5b52168a6b1f8a05`**
  (`.raw/mlx-serve`, shallow `--filter=blob:none`, removed at landing).
- **MIT** (+ Apache-2.0 components, NOTICE). 1.3k stars, active.
- Native Apple-Silicon LLM server: **Zig** core (105 `*.zig`), Metal kernels
  (`src/kernels/*.metal`), ObjC ANE bridge (`lib/ane/*.m`), Swift menu-bar app
  (528 files — no perf content). Single squashed commit: **no `git log`
  rationale exists for anything**.
- **The asset is not the code — it is `docs/gotchas/engine-mlx.md`**, a
  ~4.4k-line measured-gotchas record carrying dated, machine-named, same-session
  A/Bs next to nearly every decision. Inline `src/*.zig` doc comments carry the
  same discipline. Mine the prose, verify against the code.

## Provenance — this source is ALREADY in our graph, twice

1. **Research 171 (Batch 145, landed 2026-09-16)** records
   `oMLX (jundot) steel_qwen4_qsa_sparse_gqa.h <- mlx-serve MIT msv_attn_p256`.
   We mined a *descendant* of this repo's sparse-GQA attention kernel the same
   day, without knowing the ancestor was reachable.
2. mlx-serve **embeds ds4** ("ds4 September 14 head") and llama.cpp as alternate
   engines. **72 corpus rules already cite `antirez/ds4`** across 17 files. The
   Flash-Next speculative surface is therefore *adjacent to already-mined
   territory* — but the Zig core is mlx-serve's own code, and that is where the
   new content is.

Read both facts as a dedup WARNING, not a kill: the eight items below were
adjudicated individually against the 755-rule `rule_id` inventory.

## Ranked candidates

| # | Pattern | Site | Grade | Dedup verdict |
|---|---|---|---|---|
| A1 | **Prefill chunk LADDER priced per-request against a live memory ceiling, with a named under-billing margin and an asymmetric widen/step-down ratchet** | `server.zig:5566-5589`, `5684-5694`, `3285-3301` | **A−** (GOAT) | CLEAN. `prefill-chunk-command-buffer-capacity-coupled` is the *opposite* finding (ONE constant governing chunk+scratch; here ≥4 independent budgets); `prefault-admission-keyed-to-ram-headroom` is binary admit/decline, not a ladder |
| B6 | **A data format the fused kernels cannot read pays a whole-cache dequant per token** — TurboQuant removed not for accuracy but because every quantized kernel keys on the affine triple | `gotchas:130,135`; `kv_quant.zig:16,24` | **A−** | CLEAN. `fusion-payback-gated-on-per-dispatch-overhead` is per-dispatch overhead; nothing covers format-admission-by-fused-reader-existence |
| B2 | **An approximation confined to a coordinate the algorithm proves irrelevant is FREE** — sampled drafts drawn from an already-exact top-32 shortlist; Leviathan is exact for ANY q, so restricting q's support moves only the ACCEPTANCE RATE | `mtp.zig:1138-1146,3351`; `generate.zig:11128`; `gotchas:234-235` | **A−** | CLEAN. No `rule_id` match; distinct from `speculation-scheduler-break-even-flip` and `batch-verify-optimistic-aggregate-fallback-individual` |
| A4 | **When a fast path cannot be bit-identical, admit it on a RELATIVE tolerance against the incumbent's own error vs a third reference** (NAX ≤ 1.5× stock's error vs an f32 reference, or a BF16-ulp floor) — and run the probe **once per LAYOUT** | `transformer.zig:3284,3252-3266,4530-4552` | **A−** | SPLIT. The bit-exact probe (B) is a positive instance of `proven-shape-static-sibling-identical-results-fallback`; the **relative-tolerance** probe (A) is new decision content |
| A3a | **A gather-vs-mask crossover takes TWO values because the alternative arm's cost depends on the STORAGE FORMAT** — dense floor 32768, quantized floor 16384, because the mask arm dequantizes the whole range while the gather dequantizes only gathered rows | `transformer.zig:5865-5890,6185` | **B+** | ADJACENT to `kv-dequant-materialization-batch-threshold-gate` (that gates ONE conversion on size; this says quantization makes the FALLBACK dearer, so a quantized source LOWERS the threshold). Rule-or-rider, coordinator's call |
| B3 | **Asymmetric variance-aware break-even: price the candidate at +2σ and the baseline at −2σ, +5% switching margin, plus a per-row LATENCY VETO that overrides an aggregate throughput win** | `mtp_group_planner.zig:86-93,102-107,223-232` | **B+** | CLEAN — `variance-aware`/`2σ`/`latency veto` grep ZERO in the corpus. Distinct from `speculation-scheduler-break-even-flip` (cumulative extra-vs-saved, 110% margin, monotone permanent bypass) |
| A2′ | **A fusion's eligibility constant hardcoded to a DEFAULT that a config knob can exceed silently disables the fusion** — `hc_prefill.max_seq = 8192 + TAIL_MERGE_MAX - 1`, so `--prefill-chunk 16384` turns the fusion off with no diagnostic | `hc_prefill.zig:9-10,22-28` | **B** | Hazard shape; near `tuned-config-resource-limit-fallback`, not covered |
| A5′ | **Quantize a tuned parameter to the width of its own plateau when re-tuning has a fixed cost** — ANE share rounded to a 0.05 grid because the plateau is ~6% wide and a finer answer cold-compiles on every boot | `ane.zig:265-272` | **B** | CLEAN of `autotune-*` rules |

### Riders (not standalone rules)

- **B1** two-level chunk-max shortlist (`generate.zig:13010-13136`) — the proof
  (*a chunk whose max is not among the m largest chunk maxima holds no top-m
  element*) is **already shipped** as `tile-max-prefiltered-exact-topk`. The
  rider content: chunk width `sqrt(V/m)` clamped `[16, 2048]` balancing two
  argsorts, and the platform fact that Metal's `Partition::eval_gpu` *"direct
  partition to sort for now"*, so `mlx_topk`/`mlx_argpartition` ARE the full
  merge sort and buy nothing.
- **B1b** rank-cut vs value-cut ties → rider on
  `order-stable-topk-via-index-sort`. A value cutoff keeps every token tied with
  the k-th; **bf16 logits tie constantly at a 250k vocab, so `top_k: 1` stopped
  being greedy**. Determinism rests on mlx's argsort stability — an undocumented
  property they pin with a test.
- **B1c** f32 nucleus accumulation — mlx instantiates `cumsum` at the INPUT
  dtype; a bf16 accumulator **dropped 148 of 3720 nucleus columns**. The
  *inverse* of `operand-precision-gated-by-consumer-invariance`: here the
  consumer is a threshold cut and tail mass IS the deciding quantity.
- **B4** a shared filtered verify block requires **homogeneous sampling params
  and no per-row seed** ("the filter is per REQUEST, and one categorical cannot
  carry a per-row key") → rider on `batch-verify-rows-single-dispatch`.
- **B5** MTP head KV stays dense by default because at short context the
  quantized head cache falls **below the fused-kernel engagement floor** → rider
  on `operand-precision-gated-by-consumer-invariance`.
- **A3b** split-select at 24576 blocks (one threadgroup per row is latency-bound:
  0.72 ms at 215k blocks, ~20× under the bandwidth floor) → the **underfill**
  family; rider on Batch 145's `kv-split-decode-underfill-per-slot-validity`,
  whose upstream is this repo.

### Dropped

- **The HC+GDN prefill fusion itself.** Mechanism is readable (two *separate*
  fusions sharing a seq bound and a 65536-entry BF16 sigmoid LUT, not one fused
  op), but it is **the one optimization in the repo with no in-repo
  measurement** — the release note's `723 → 744` / `1749.5 → 1849.5` appear
  nowhere in the tree. Only the A2′ hazard above survives.
- Idle-evict, model picker, structured-output, cache-restore fixes — ambient.

## Claims that DO NOT survive the source read

Three release-note headlines are unsupported or wrong, and the third matters to
riir-ai Issue 769:

1. *"Sparse-attention block selection is 3× faster"* — in-repo evidence is
   **+4% at 40k blocks / +18% serial at 200k**. "3×" NOT FOUND.
2. *"speculated share goes from 19% to 98%"* — `CHANGELOG.md:29` only. No test,
   no benchmark, no gotchas entry. Unsourced.
3. ⛔ *"M5 Macs prefill on the new neural accelerators (+12% at 160k)"* —
   **conflates NAX (the GPU's matrix units) with the ANE.** ANE prefill is
   *affirmatively refused* on M5 (`ane.zig:538-542`) after measuring
   **−11% / −7.5% at 16k / 32k** (PR #223, two testers). The "+12% at 160k"
   figure is NOT FOUND in source or docs.

Also flagged: the "log names why a request runs alone" claim is true only at
8-tag `BatchVerdict` granularity; the nine MTP-group conjuncts
(`scheduler.zig:7280-7291`) are unobservable, and the log is one-shot per slot.
And an internal inconsistency worth not inheriting: `round_cost.zig:757` says
the persistence key includes the engine build, `gotchas:4350` says it does not.

## GOAT candidate

**A1 — the prefill chunk ladder.** Tested against the four criteria:

1. **Benchable** — pure host-side controller logic over a `(headroom, cost)`
   stream; runs on a perf harness, no Metal required. This is why it beats the
   kernel candidates: A3a/A3b are **Metal-only → unbenchable on our CUDA/CubeCL
   validator → leads, no GOAT** (the Research 122 precedent).
2. **Exact semantics** — chunk width must not change output; G1 is a
   bit-identity gate, trivially.
3. **Magnitude** — a descending ladder priced against a live ceiling
   (`mlx_get_active_memory` vs the Metal working-set limit, with the ANE's wired
   int8 copies added back and re-subtracted because they are invisible to MLX's
   accounting but genuinely gone from free RAM), a **+22% surcharge on the rung
   the long-context gate widened** because the estimator is documented to
   under-bill quantized prefill by about that, and an asymmetric controller:
   HOLD while headroom ≥ cost, STEP DOWN at once by as many rungs as it takes,
   WIDEN one rung only at 1.25× cost after 2 supporting probes, **never after a
   step-down** (a one-way ratchet).
4. **Deterministic** — yes, given the memory stream.

Runner-up: **B3**, the asymmetric ±2σ break-even, same host-side property.

## Yield estimate

**2–4 rules + 3–5 riders** after the usual ~50% coordinator falsification.
Above the recent 1–3 baseline because the gotchas doc supplies the regime
conditions and the measured negatives that normally die in verification.

## Caveats

- **Metal/Zig, not CUDA/Rust.** Most kernel-shaped candidates are leads only.
  The four A−/B+ rules are host-side and portable.
- **No git history** (single squashed commit) — every rationale is prose, so
  quotes must be pinned to `dcb0ede5`; a beta-quality fast-moving repo makes the
  pin load-bearing.
- **Their benchmark discipline is a citation asset in its own right**:
  `benchmarks.md:5` *"Apple M4 Max 128 GB ONLY … one mixed column poisons the
  whole history"*; `.claude/skills/bench/SKILL.md` carries "reproducible ≠ not
  variance", thermal-soak same-session-ratios-only, power-source-is-a-variable,
  and — our own lesson, independently rediscovered — *"an A/B arm is proven by
  ENGAGEMENT lines in its own log, never by its launch env"* (zsh not
  word-splitting `env $VAR` made a "composed" arm silently run the fast path).
- Absolute tok/s from this source is **never** league evidence: M4 Max, their
  own packs, their own harness.

## Lane intel — riir-ai

See the queue-snapshot line for the full mapping. Headline: **mlx-serve is in
neither the 8-row `watch_repo` table nor the 5-engine `EXPECTED_TABLE_ENGINES`
pin**, while being a named MIT upstream of a kernel Batch 145 mined the same
day. The coverage gate PASSES — correctly, by its own predicate — because
mlx-serve is not an opponents-table row. That is a POPULATION gap, not a gate
defect, and it is the Rapid-MLX precedent exactly.
