# Research 315 — Locals (Generated-Computers/Locals) distill verdict: the whole-step GDN decode window + device-side constraint recompute

**Status:** DISTILLED — pending owner decision (queued as Batch 278, plan 282, GATED on Batch 276's landing — the `entries_gpu_124.rs` serial-corpus discipline; the issue lane is riir-infer Issue 1108 T3)

## Source

`https://github.com/Generated-Computers/Locals` @ pin `32cf2feb07cd6e92cde4871347435773393f1812` (MIT, © 2026 Arip Asadulaev, commit dated 2026-10-22, v0.2.1 on PyPI — actively maintained). Ephemeral clone `.raw/locals` (full clone, sha checked out, removed at this verdict's commit). 140 Python files, ~26k LOC, ZERO Rust — an MLX/Metal serving stack for agent workloads (constrained decode, hybrid GDN+attention batch serving, small-M quantized kernels, prompt cache).

Provenance chain: the small-M kernel family (`qmv_multi.py`/`qmv_frag.py`/`mma_kernel.py`/`nax_kernel.py`) is DOWNSTREAM of ml-explore/mlx's own kernels and steel building blocks — `mma_kernel.py` states it "reuses MLX's `QuantizedBlockLoader` … and `steel::BlockMMA`", and the corpus's Batch-273 rule (`fewrow-mma-fragment-matvec-silicon-tiered-dispatch`, gpu_119) is cited from mlx @ `c6abf0db` #4712, the SAME upstream mechanism. The GDN step kernel cites mlx-lm's decomposition as its reference ("Rounding to the activation dtype happens at the same points as in mlx-lm"). The paper side (ReachUL, arXiv:2610.09139) is already distilled in riir-infer Research 014 / Issue 1108; THIS verdict adjudicates the IMPLEMENTATION.

Read method: full-tree read of the five mechanism files (`gdn_kernel.py` 205 lines, `walk.py` 219, `lazy.py` 200, `hybrid_batch.py` head 1879-line file, `engine.py` pipelined loop, `kernels.py` heads) + `docs/why-faster/README.md` (the measured ablation table) + coordinator corpus greps under both vocabularies (the Research-057 law, evidence inline below). Single-session read, no subagents — the five files are small enough to read whole.

## The measured ablation (source-reported, M2 Pro 19-core, Qwen3.5-4B 4-bit, 16 chat prompts, tokens/s summed across rows)

| Variant | 1 req | 4 | 8 | 16 |
|---|---|---|---|---|
| Plain mlx-lm | 60.5 | 87.0 | 86.2 | 156.8 |
| Locals | 65.6 | 160.8 | 272.7 | 345.8 |
| − small-batch kernels | 67.0 | 118.1 | 119.2 | 213.0 |
| − bfloat16 instead of float16 | 65.6 | 93.0 | 97.9 | 317.0 |
| − fused projections | 63.7 | 154.5 | 246.8 | 332.8 |
| − GDN kernel | 64.3 | 158.0 | 267.6 | 343.0 |
| − pipelining | 55.2 | 138.1 | 236.5 | 300.6 |

Honest reading: the GDN whole-step kernel is worth **1–2% at batch** ("the launches it saves matter most for a single stream" — their words); the small-M kernels are the big axis at batch; pipelining 13–16%. The f16-vs-bf16 arm (93 vs 273!) applies ONLY to M1/M2 (no native bf16) — M3+ keeps bf16, so it is NOT our regime (the league boxes are M3 Max + M5 Ultra). Provenance, never adoption bars (NotBenchable — see below).

## Verdict

**YES, B corpus (kernel_opt) / LANE INTEL MODERATE (riir-ai Issue 1113 confirmation + the Track-A reference design).**

Domain routing: **kernel_opt** — Metal kernels + host-orchestration around GPU. The routing correction vs the filing issue: Issue 1108 T3 named "rust_perf candidates = pipelined step enqueue with unevaluated device values, prompt-trie snapshot prefill" — under the routing table those are **host-orchestration around GPU = kernel_opt** (D-family dispatch), NOT rust_perf; rust_perf rejects GPU-adjacent patterns. Nothing here routes rust_perf (there is no pure-CPU Rust in the repo — zero `.rs` files).

## Ranked candidate table

| # | Pattern | Source file | Grade | Notes |
|---|---|---|---|---|
| 1 | **whole-step-gdn-decode-window-one-launch** — a recurrent layer's ENTIRE decode step (conv+silu+gates → q/k RMS norms → delta-rule recurrence → gated output RMS) as ONE Metal launch, one threadgroup per (sequence, value-head), the head's state rows register-resident ACROSS an S ≤ 16 token window (speculative/verify band), rounding points pinned to the reference decomposition ("results match the stock path up to summation order") | `locals/gdn_kernel.py` | **A−** | The corpus's fusion family owns pairwise/chain folds (`fused-conv-state-shift-dot-silu` owns the conv slide+dot+silu PIECE; `fuse-gate-up-gemv-activation` the MLP pair) and the G family owns recurrence INTERNALS (`row-owned-register-blocked-recurrence`, the simdgent row packing B98 rider) — ZERO owner of the whole-layer-step boundary or the S-token register-resident window. Distinct faces: (a) the whole-step admission boundary (fuse across the two norms, not just within chains); (b) the S ≤ 16 multi-token decode window with state in registers across steps (the verify-band carrier — connects to the L8/MTP window lane); (c) `supported()` gate (Dk==Dv==128, Hv%Hk==0) + the `states:` variant writing per-token state for draft rewind. NotBenchable (Metal/MLX; domain reason — see GOAT). |
| 2 | **constraint-row-recompute-on-device-not-table** — grammar constraints WITHOUT the V×|Q| transition table: one Metal kernel per decode step, a thread per vocab token walks the token's bytes through the byte automaton from the current state; memory O(256·|Q| + packed vocab bytes) whatever the grammar's size; the state stays device-resident so the engine stays pipelined | `locals/walk.py` | **B+** | Zero corpus owner of constrained-decoding/automaton machinery (grep `automaton|DFA|constrained decod` over both corpora: empty). Decision content: the space–time recompute-vs-table choice ON DEVICE with the pipelining-preservation constraint (host state would serialize); int16 state cap at 2^15 (loud refuse); the SPARSE_MAX=4096 arm (when a state allows ≤4096 tokens, compute the lm_head over those ROWS ONLY — `sparse_steps` counter — instead of masking the full logits). |
| 3 | **ragged-batch-hybrid-feed-two-contracts** — right-padded batch feeds over hybrid layers with PER-LAYER-KIND contracts: attention layers take `right_padding` (the cache rolls it left at finalize), recurrent layers take per-row `lengths` (masked positions leave the GDN state UNCHANGED — state-unobserved ≠ zero-fill), and a sliding-window layer needs the lengths too or the padding EVICTS a shorter row's in-window tokens (the measured Gemma bug: "11 of 24 empty, none alone") | `locals/hybrid_batch.py` `_prepare_ragged` | **B** | `ragged-chunk-pad` (J) owns chunk-pad for chunked reductions; `varlen-packing` owns varlen GEMM packing — neither owns the per-layer-kind contract split nor the window-eviction correctness hazard. The hazard anecdote is the decision content's teeth. |
| 4 | **overlapped-branch-speculation-commit-or-discard** — the decode loop builds step n+1 for the COMMON branch while the GPU runs step n; a forced-span branch DISCARDS the prebuilt step via cache snapshot/restore; the EARLY_BODY frequency gate (states with ≥3 visits and <25% forced rate run the body early even when a force is possible — the build overlaps the host wait, a rare force restores the snapshot) | `locals/engine.py` `_generate_pipelined` | **B−** | KINSHIP (compose, never merge): `pipelined-token-slot-ping-pong-submit-ahead` (gpu_18) owns the ON-DEVICE token dependency (greedy, constraint-free); `rotated-ring-undo-via-rebind-reference-snapshot` (B71) owns the snapshot-undo DISCIPLINE for rejected drafts; THIS owns the HOST-side branch speculation with the frequency-gated early-build arm — the trigger axis (common-branch build) is new. |
| 5 | lazy constraint rows — per-visited-state rows on demand (trie level-pass), bounded LRU (512 rows, O(V·visited)), successor rows prefetched in a background thread while the model runs the step | `locals/lazy.py` | **C+ rider** | `independent-cpu-work-fills-dispatch-read-window` (B36) owns CPU work inside dispatch windows — the prefetch thread is a B36 instance; the bounded row cache + unique-path recompute discipline is the thin new face. Rider, not own rule. |
| 6 | timed per-shape small-M kernel dispatch with background idle-time tuning + crash quarantine (a shape whose tuning crashed the process twice keeps the incumbent forever + the abort is loud) + shipped per-SKU preset + stale-timing guard (MLX version change ⇒ shapes re-time) | `locals/kernels.py` | **rider (dedup-heavy)** | The mechanism family dedups against `small-batch-matvec-gemm-bridge-rung` (B98), `fewrow-mma-fragment-matvec-silicon-tiered-dispatch` (gpu_119 — the UPSTREAM mlx rule), `batch-width-routed-dual-tile-twins` (B77), the K-family autotune tables. The crash-quarantine + preset-warm-start face is thin but real — rider on the K family. |
| 7 | f16 compute-dtype demotion on no-native-bf16 GPUs with a two-sided probe (same next token + near-same logits, else compare both against f32) | `locals/runtime.py` (via why-faster doc) | **rider (not our regime)** | `operand-precision-gated-by-consumer-invariance` (B118) owns consumer-gated precision; the probe's two-sided form is the thin delta. M1/M2-only — the league boxes (M3/M5) keep native bf16. Lane note only. |
| 8 | prompt-trie snapshot prefill (snapshots at template boundary points; the "reply" checkpoint keeps ONLY recurrent state + KV views cut to n, no copy) | `locals/hybrid_batch.py` | **DROP as corpus rule** | This is riir-ai Issue 742's prefix-cache lane ALREADY — Issue 1108 T2 folded the design delta into `qwen38_prefix_cache.rs`'s trigger notes on 2026-10-24 (the 83% cache-share / 2.4 s 138k numbers are in that note). Positive-instance of our own substrate documentation; not a healer rule. |
| 9 | fused projections (QKV → one GEMM, gate+up → one, the four GDN input projections → one) | `locals/fuse.py` | **DROP** | Positive instance of `same-input-consecutive-projections-one-dispatch-concatenated-tiles` (gpu_117, the splash wave) — the file IS rule N. Hazard-filing candidate at most. |

## Dedup-gate evidence (both vocabularies, per candidate)

1. Whole-step GDN: source terms `gdn`, `decode step`, `one kernel`, `fourteen kernels`; corpus operator terms `whole-layer`, `one-kernel-layer-pass`, `register-resident`, `fused-conv-state` — the hits are all PARTIAL owners (conv piece / recurrence internals / prefill residency); grep `conv.*silu.*recurrence|fourteen|launches per.*decode step` → gpu_05 (an unrelated 14-field gradient-bridge rule) only. CLEAR.
2. Walk recompute: `automaton|DFA|dfa|constrained decod|grammar.*mask` over BOTH corpora → ZERO. `walk` hits are decode-walk/replay-walk families (unrelated). CLEAR.
3. Ragged hybrid: `ragged` → `ragged-chunk-pad` + `ragged-pack-offset-table-fused-dispatch` + `zero-pad-ragged-tail-additive-identity` (all chunked-reduction/GEMM faces); `lengths`/`right_padding`/sliding-window eviction → ZERO. CLEAR (kinship noted).
4. Overlapped branch: `pipelined` → `pipelined-token-slot-ping-pong-submit-ahead` (kinship, different axis — on-device token vs host branch); `snapshot`/`restore` → the B71/undo family (kinship). CLEAR as a distinct face.
5. Lazy rows: `prefetch` → tile-prefetch occupancy rules (GPU tiles, not CPU row precompute); B36 owns the window-fill axis → RIDER posture, not own rule.

## GOAT candidate identification

Strongest: **whole-step-gdn-decode-window-one-launch**. Criterion check: (1) benchable — FAILS for a DOMAIN reason: Metal/MLX kernels, the corpus serves no Apple-GPU validator (the entries_gpu_65 standing disclosure; M3 Max has no NAX, the 4090 is not Metal) → **NotBenchable annotation, bench_spec: None, per the Batch-136 precedent — a per-rule annotation, never a demotion**; (2) exact semantics — the kernel pins its rounding points to the reference decomposition and states the residual ("up to summation order") — a stated tolerance strategy; (3) magnitude — 14 launches → 1 per linear-attention layer per token; at OUR measured amortized launch cost (~2.3 µs/launch, Bench 995) ≈ 30 µs/layer/token ≈ 0.7 ms/token over 24 GDN layers at batch-1 — a mechanism-level count argument, and the source's own ablation bounds it honestly (1–2% at batch, more at single stream); (4) deterministic — yes (fixed threadgroup layout, no atomics on the value path). Three of four pass outright; the one failure is domain-level, which the two-class split grades as annotation, not weakness.

## Lane-intel axis (mandatory)

- **riir-ai Issue 1113 (the m5ultra decode+prefill defense)** — MAPPED, CONFIRMATION + boundary refinement. Our own Bench-995 fold-tail+SwiGLU-rotate fusion saved −160 launches/token EXACT but measured only +1.40% e2e (below the 1.5% promote bar; launch class ~2.3 µs/launch amortized). The Locals GDN kernel is an INDEPENDENT consumer-grade datapoint for the same class on the same silicon family: whole-layer fusion worth 1–2% at batch, "most for a single stream". This REFINES the Issue-1113 lever pricing — the launch class pays at SINGLE-STREAM decode (our league decode rows ARE single-stream tg128!), and the Locals kernel additionally keeps state register-resident across the S-window (a memory-traffic axis, not just launches — the same "intermediate round-trips" shape Issue 1113 names in the deltanet split chain). NOT transferable as a bigger win than ours measured; it CONFIRMS the class is real and small-at-batch. Follow-up: cite Research 315 in Issue 1113's remaining-rungs section as the mlx-ecosystem external instance (source readable at the pin).
- **riir-infer Issue 1108 T1 (Track A, consumer-gated)** — the walk kernel + lazy rows + sparse-head-rows arm ARE the reference implementation of Track A's named shape ("Metal walk kernel per locals/walk.py's shape"): the verdict adds three implementation facts beyond the paper: the SPARSE_MAX head-rows-only lm_head arm, the budget-aware masking hook, and the int16 device-state cap. Recorded for the lane's trigger file; NO build (T1's consumer gate stands).
- **riir-ai Issue 742 / Issue 1108 T2 (prefix cache)** — already folded 2026-10-24; the implementation confirms the design, nothing new owed.

## Product-coverage axis

Not fired — the patterns live in MLX/Metal (Python), not a famous Rust-lib family; no tokio/rayon/serde class in play. The corpus's Apple/Metal family (uzu, mlx, splash, ds4, mlxfast — 10+ M-series entries) is the served family and this source feeds it.

## Yield estimate

**3–4 rules + 2–3 riders after ~50% coordinator falsification**: candidates 1–4 as own rules (NotBenchable-annotated), 5–6 as riders, 7 as a lane note. The filing issue's five named candidates survive as: 3 own rules (1–3), 1 reroute (4, rust_perf→kernel_opt D-family), 1 drop-to-lane-intel (the prompt-trie row, candidate 8).

## Honest caveats

- Source-reported numbers are M2 Pro / Qwen3.5-4B / mlx 0.32 — a different chip class and model than any corpus box; provenance, never bars.
- The repo is 12 days old (v0.2.0 2026-10-21) — young, but MIT, on PyPI, with a measured-ablation culture (the why-faster doc switches each piece off and re-measures — exactly the corpus's own evidence shape).
- `app.py`/`server.py`/`harness.py` (~5.6k LOC of CLI/server/menubar surface) skimmed for structure only, not read line-by-line — they carry no kernel mechanics; the five mechanism files were read whole.
- The Rust-side tie-in: zero `.rs` files — nothing routes rust_perf (the filing issue's two rust_perf names reroute, recorded above).

## Bottom line

Mine as **Batch 278** (plan 282, GATED on Batch 276's landing — the entries_gpu_124.rs serial discipline + the count pins move in one landing): lead with `whole-step-gdn-decode-window-one-launch` (A−, the verify-band carrier), then `constraint-row-recompute-on-device-not-table` (B+), `ragged-batch-hybrid-feed-two-contracts` (B), `overlapped-branch-speculation-commit-or-discard` (B−). Queue slot recorded; Issue 1108 T3 closes with this verdict as its record; Issue 1113 gains the external-instance citation at the batch landing.
