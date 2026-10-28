# Research 344 — splash return visit: the un-mined residue + the ANE lane intel (incoai/splash @ cabbad27)

**Status:** DISTILLED — pending owner decision (verdict landed with plan 308, nominal Batch 303, staged behind plan 307's live claim; no corpus edits made by this verdict)

## The source

incoai/splash — "A local inference engine for Apple silicon, built around the model"
(https://github.com/incoai/splash). Swift host + C++ ops + MSL Metal kernels + Apple
Neural Engine (private AppleNeuralEngine client). Apache-2.0; GGUF table decode follows
llama.cpp's Metal kernels with the MIT notice carried in THIRD_PARTY_NOTICES (original
Rust fixtures only at mine time, per the skill constraint). Inspection 2026-10-28,
user-directed @distill. Pin **`cabbad2732b027540f5a141511544e0bb62224b7`** (shallow clone
into `.raw/splash`, read + removed at the verdict commit; HEAD = merge "docs/split",
ahead of the 10-28 tfdelta sweep's cover point `ed79621`). Two inspection lanes with
disjoint scopes (kernels / host+ANE), coordinator byte-verified the headline quotes at
the pin. 31 `.metal` files + ~363 C++/ObjC++/Swift sources; zero `.rs` — per the
transcription law, language is never a gate.

**This is a RETURN VISIT to an approved, heavily-mined source:**

| Prior owner | What landed |
|---|---|
| B236 (`entries_gpu_95`) | `mid-prefill-recurrent-state-capture-boundary-unsplit` |
| B237 / B247 riders | discretionary-cache-growth; KV-ALIKE + retained-rows oracles |
| **B268** (Research 305 / Plan 273, `entries_gpu_117`) | **11 own kernel_opt rules** from the `be3b083c..origin/main` release wave (205 non-merge commits): apple10-narrow-grid-register-tile, splitk-tier-ladder, mma-lane-map-whole-run-vector-loads, constexpr-unroll-capacity-bound, prefix-active-split-axis-linearized, moe-lone-tile-register-gemv, moe-unverified-rows-route-zero-experts, block-sparse-union-lists, same-input-consecutive-projections, consumer-prepares-consumed-table, wide-row-tile-under-bpw-floor |
| B270/271 (Plans 274/275, Research 306) | the wave's rust_perf serving pool (KV-tier restore-point chain, spill faces) + landed `calibrate-verify-window-on-text-not-proposer-chain` (the VerifyBudget mechanism), `bounded-peer-handoff-wait` (CommandWatchdog), `constrained-unconstrained-lanes-one-decode-command` |
| 10-28 tfdelta sweep | `ed79621` determinism corroboration of the Issue-940 law (record-only); post-cover delta "CI + 2 product merges" at commit-log granularity |

The verdict's delta is NOT commits — it is the **deep-read residue the release wave
dropped or never read** (B268's own A-slice ledger shows ~17 drops), plus the **ANE host
surface, which has no corpus owner** (verified by grep: zero owner for the calibration
fit, the breaker, or the one-program law; `ane-bank-split-ladder` is our own B114/oMLX
rule). Prior cover point `origin/main` ≈ 2026-10-21; `docs/internals.md` (1,640 lines,
the design doc) and `docs/performance.md` carried the decisive mechanism text.

## Verdict

**YES B+ corpus (return-visit residue) / LEAGUE INTEL HIGH (plan-643/769/887 mapping) /
product-coverage MODERATE (the serving-pool family is seeded; the ANE micro-family is
new).** Rationale: 10 zero-owned kernel_opt candidates + ~3 open ANE host candidates +
12 rider-magnitude findings for existing rules, from a source with exceptional
measured-claims density (~20+ distinct measured numbers in kernel comments alone), on
silicon we serve (M3/M5-class Metal), already proven mineable (B268's 11 landed).

## Ranked candidates — kernel_opt (lane A; every dedup grep zero-hit under both vocabularies)

| # | rule id | Source : quote (byte-verified at pin) | Regime | Magnitude | Measured? |
|---|---|---|---|---|---|
| 1 | `golden-ratio-decode-tile-phase-spread` | `gguf_staged_tile.h:41-53`: "Walking in lockstep (spread 0), every column tile of a projection fetches the same slice of the input at each step, which the operator before has just written and no cache holds yet, so the fetch sits on every tile's critical path. Spread (1), tile t starts (t * 40503 mod 2^16) / 2^16 of the way in" — one multiply, one `& 0xFFFF`, one shift (`staged_first_step`), wrapping walk | Wins: decode tiles streaming the SAME freshly-written activation. **Anti-regime named IN SOURCE: "the MoE expert tiles and the prefill tile walk in lockstep"**; loses when L2 already holds the operand | Hides first-touch DRAM latency across every tile of a dispatch; host knob `LinearConfig::spread` is the rule's gate | Design-level |
| 2 | `fp32-pipe-mma-role-split-biased-bf16-operands` | `decode/linear_gguf_sgmatrix.metal:1-13`: "The 8x8x8 MMA runs on the FP32 pipe there, so the kernel keeps every other FP32 operation to the minimum a group scale needs, and does the rest on the integer pipe: weights enter the MMA as exact bf16: 128 + code … 160 + code - zero (one add)" + seeded per-span input sums close the chain (`gguf_sgmatrix.h`) | Wins: Apple9-family register-tile GEMV where MMA rides FP32 pipe. Loses: Apple10 (different pipe), formats outside bf16-exact code range | One fp32 multiply per 32-input group instead of per-element dequant on the scarce pipe | Design-level; kin `one-pack-serves-all-schedules-nibble-order-contract` + `integer-mma-span-epilogue-dequant` own the bias tricks — the ROLE SPLIT + seeded chains are the new content |
| 3 | `gdn-decode-tape-deferred-retained-row-fold` | `decode/gdn.metal:22-25`: "Which of a step's rows a lane retains is known only after the step, so the state a step writes is the one before its rows: it leaves the rows on the lane's tape, and the lane's next step folds the retained ones into the state before its own rows. A step reads and writes the state once" | Wins: recurrent decode under speculative verify (step-dependent retained counts). Loses: non-spec decode, prefill (own capture path, mined B236) | Halves worst-case state traffic (one R+W per step vs per-row RMW) | Part of the measured claim below |
| 4 | `gdn-head-threadgroup-pipeline-tg-handoff` (rider-grade NEW magnitude on the mined `whole-step-gdn-decode-window-one-launch`) | `decode/gdn.metal:13-20`: "The phases therefore hand each other their operands in threadgroup memory instead of device memory, issue their device loads before their stores … and the gate runs one simdgroup per row without barriers: over the 27B's 48 layers with DRAM-cold states that takes 11-28% less time at one to four lanes … with the same bits" | DRAM-cold recurrent state, 1–4 lanes | **11–28% over 48 layers, M3 Max + M5 Pro, bit-identical** | **Yes — source-measured** |
| 5 | `metal-fma-carries-fast-math-reassociate-off-cannot-pin` | `gguf_column_tile.h:17-19`: "The chain is written as multiplies and adds, whose order `reassociate(off)` keeps: Metal's fma() is emitted with every fast-math flag, reassociation among them, which the pragma does not clear" | Any Metal kernel needing bitwise-stable fp32 chains — the ENABLER for every determinism rule on Metal; loses on perf-critical loops not needing order pins | Without it order pins silently fail (fma contraction) | Indirect — the GDN parity claim below is its proof |
| 6 | `gdn-cross-phase-gate-bitwise-parity` | `decode/gdn.metal:292-295`: "the lane assignment and the channel-order sum of the four simd_sum partials reproduce the prefill gate … with fast-math reassociation this shape rounded about one output in 10^5 differently" | Any op family with separate decode/prefill kernels feeding the same state | ~1-in-10⁵ rounding divergence decode-vs-prefill without it | **Yes — the 1-in-10⁵ figure is source-measured** |
| 7 | `conditional-pv-accumulator-rescale` | `paged_attention_tile.h:241-248` + `:367-374`: online-softmax rescale of the M×D PV accumulator gated on a `rescale` flag (`if (scale != 1.0f) atomic_store_explicit(rescale, 1u, …)` → accumulate multiply reads the flag) | Wins: long stable-context pages where max rarely moves. Loses: first pages / high-dynamic scores (one atomic store of pure overhead) | Skips one FMA pass over M×D fp32 registers per page when unchanged; arithmetic-preserving by construction | No; structural |
| 8 | `kv-stage-reuse-overlapped-softmax` | `sparse_attention_tile.h:36-38`: staged keys/values 8 KiB leaves the 96-row tile's scores+probs room in 32 KiB; `:341-344`: "The values replace the keys in the stage while the softmax reads only the scores and scales" | Threadgroup-memory-bound attention tiles; loses when SMEM headroom ample | Halves KV stage bytes → bigger token steps | Budget arithmetic explicit |
| 9 | `apple9-coop-tensor-destination-init-lost-runtime-format` | `gguf_staged_tile.h:19-20`: "returning an initialized cooperative tensor loses its initial values on Apple9 in runtime-format kernels (also with shader validation)" — must zero element-wise | Apple9 + run-time format selection; correctness class | Garbage accumulators otherwise | Found via test failure |
| 10 | `quad-lane-softmax-two-shuffle-16b-store` | `paged_attention_tile.h:160-165`: "Four lanes own one fused row, eight consecutive tokens each, so the row maximum and sum are two xor shuffles instead of a 32-lane reduction per row and the eight bf16 probabilities leave as one 16-byte store" | 32-token pages, 256-thread TG (N==32 asserted); lane geometry co-designed for reduction depth AND store width | 2 xor shuffles vs butterfly per row; one 16B store per lane | Structural |

Second tier (micro, real): `unsigned-window-mask-single-compare` (start-counting so one
unsigned compare tests both window ends, `paged_attention_tile.h:198-204`) ·
`lane-binding-macro-select-not-function` ("A macro, not a function: … turns a function's
into branches", `lane_bindings.h:5-8`) · `e4m3-exponent-shift-half-decode` (LUT-free fp8
by exponent-field placement, `quant_formats.h:43-47`; CUDA-only kin `lop3/prmt-*`) ·
`residency-end-noop-kick` ("Metal applies endResidency at its next GPU operation… the
backend dispatches this kernel as that operation", `residency.metal:5-8`) ·
`thread-local-dynamic-byte-index-cost` ("indexing a thread-local byte array … by it costs
~8% of the eight-row staged kernel on Apple10", `quant_formats.h:101-104`).

**Riders (magnitude for EXISTING rules, feed at next re-pin):** split-order reduce vs
spec-decode acceptance ("An earlier parallel denominator reduction passed numerical tests
but changed speculative acceptance enough to reduce throughput on some prompts; it was
rejected" — `dev/benchmarks/remaining-decode-optimizations.md`, rides
`splitk-reduce-one-launch-slice-order-pinned`); fp32 router scores (bf16-rounded top-8
flips 12–15% of rows — `moe.metal:15-19`, rides `router-math-stays-fp32`); indirect-grid
dead threadgroups 0.8–4.8% of decode cycle (`Moe.cpp:222-226`); ragged chunk row-skip
1.7× + "branch-free copy for full tiles is no faster" (`gguf_linear.metal:31-35`); 32-row
tile over padded lanes 3–15% (`gguf_linear.metal:62-66`); MoE prefill 32-row tiles 17–30%
(`moe_gguf.metal:16-17`); gate/up one pass 1.1–5.1% (`moe_gguf.metal:94-99`); run-time
format dispatch −11..+8% vs per-format (`moe_gguf.metal:11-12`); `noinline` column tile
fixing a shader-validated miscompile on Apple9 (`moe_gguf.metal:100-104` — correctness
face); meta-unit-crossing prefetch skip. **`remaining-decode-optimizations.md` is a model
distill artifact**: ABBA protocol, box-state disclosure, and a NEGATIVE-RESULTS section
(register-resident attention slower; 2-KV-pages/iter slower on M5; cooperative Q reuse
compiler-rejected; four numerator chains no gain) — anti-regime clause gold.

**PQ2_0 answered** (our league model's format): `FmtTwoBit` = GGUF type 142, 2-bit codes
in Q2_K layout, one half `d` per 128-element meta unit, `value = d * (q - 1)` — just
another QuantLinear format in the MDGG0001 tile-major plane image; no ternary-specific
kernel (stock llama.cpp lineage for tables, splash-original for the plane image +
repack-at-load).

## Ranked candidates — ANE/host residue (lane B; NOT landed, zero corpus owner)

Routing: the wave's serving leads landed rust_perf (B270/271) — the ANE host patterns
join that family; kernel_opt's "host-orchestration around GPU" clause is the alternative
home, the batch adjudicates.

| # | rule id | Source | Pattern + regime |
|---|---|---|---|
| 1 | `one-program-many-functions-accelerator-service-memory` | `internals.md:760-762` + `AneFfn.cpp`: 13 one-function programs held the ANE service's intermediate memory 13× (3.25 GB) vs **0.67 GB for one program with 13 row-length functions** sharing one surface set; compile once per source hash, ~0.1 s warm load | Group compiled accelerator procedures into ONE program when the service bills intermediate memory per program. Regime: any NPU/accelerator runtime with per-program residency (maps 1:1 to our 96-program bank collapse, below) |
| 2 | `ane-split-calibration-fit-bandwidth-interference-term` | `AneFfnCalibration.cpp`: `T(s) = max(A(s), G(s) + u·A(s))` — u = "the bandwidth the ANE's part takes from the GPU's beside it" (clamp of low-share excess ÷ A; M6 0.1–0.33, M5 Pro 3–7%, M5 Max 0); choice = argmin over ±1 unit with a **5% gain floor**; per-function minimum-rows chain at 2% chunk noise ("640 rows take 93% of 768's"); median-of-3 with a GPU-alone FFN stand-in subtraction | Live, per-device calibrated split fraction — NOT a static config knob. Remember-to-cache, forget-on-breaker |
| 3 | `accelerator-lost-breaker-forgets-calibration` | `AneFfnCalibration.cpp:181-194` + `AneFfnStartup.cpp`: over each 8 commands, accumulated slow-agent time ≥ accumulated GPU-alone baseline line → retire the split AND delete the remembered calibration so the next start re-derives | Any accelerator fast-path that can silently lose to the baseline needs a bounded window + baseline-line comparison + calibration amnesia. Distinct from `bounded-peer-handoff-wait` (progress give-up) and B219's persisted-table repair |
| 4 | `writeclass-share-partition-inflight-writes` | `KvPageTier.hpp:219-231`: 64 in-flight total; reclaim all, persist ¼, spill ½ — "one kind never starves another and reclaim always keeps a quarter" | B− two-regime starvation prevention |
| 5 | `spin-keep-awake-checked-capability-degrade-loud` | `internals.md:968-979`: a GPU-thread spin keeps clocks up; the spin needs a memory scope the public headers lack, so it builds separately and the backend **checks the spin actually sees the CPU end it, else warns and runs without** | The CHECKED-DEGRADE half is the rule |

Already-landed-adjacent (do NOT re-claim): VerifyBudget adaptive verify depth =
`calibrate-verify-window-on-text-not-proposer-chain` (the draft-agnostic machinery — our
MTP head could consume it verbatim; honest cost documented, run-dependence would need an
Issue-940 disposition); CommandWatchdog = `bounded-peer-handoff-wait`; serving-mark/
hysteresis faces are siblings of `resume-admission-sized-to-per-command-window` (entries_39).

## GOAT candidate

`golden-ratio-decode-tile-phase-spread`: (1) kernel_opt entries gate on needles+parse +
retrieval floors (kernel_opt is not Pareto-benched — its full GOAT is pending per
AGENTS.md), and this rule has the cleanest decision content of the set; (2) semantics
exact (an integer start offset — no numerics change; the walk is a reordering of tile
starts, not of arithmetic); (3) magnitude story is mechanism-level (first-touch fetch off
every tile's critical path; the source names both regimes itself); (4) deterministic
(same tile id → same offset). The determinism pair (#5+#6) is the strongest
workspace-resonance candidate — it pins the Issue-940 class on Metal, where our own
determinism law has no Metal-shaped corpus entry yet.

## Lane-intel axis — HIGH (the reason this verdict matters beyond the corpus)

**1. plan-643 / 769 / 887 (hybrid ANE/Metal prefill for folded Bonsai-2).** Splash is a
SHIPPED hybrid ANE implementation on our silicon class. The mechanism deltas, each with
file:line evidence at the pin:

- **One-program-many-functions** (0.67 GB vs 3.25 GB): direct prior art for our T2
  bank collapse (96 programs / 12.61 GB / 45.6 ms per-program switch / the 0x50004
  cascade). Splash proves the service bills intermediate memory PER PROGRAM — collapse
  per-layer programs into one program of per-shape functions.
- **Rotation-in-program dissolves the Plan-602-B2 premise** ("compiled ANE programs
  cannot carry the Hadamard rotation"): splash bakes a seeded random-signed Hadamard into
  BOTH sides — weights get it at GPU requant time (one layer ahead, double-buffered
  IOSurfaces), the input gets a cheap GPU rotate pre-op, and **the intermediate rotation
  lives INSIDE the ANE program as a grouped 1×1 conv whose fp16 weights are a constant
  blob** (`rotationBlob`, `AneFfn.cpp:131-142`, `:350-357`). Orthogonality makes
  int8-in-rotated-basis exact. Our option A (GPU-side sandwich) and splash's answer are
  composable, not exclusive.
- **The T10 wait-share wall** (85–87%): splash's answer is NOT a second queue (our P2
  fence-split refuted at 1.002×). It is commit-time batch enqueue of all ANE jobs (one
  XPC burst, CPU never per-op), GPU-event ordering, one-layer-ahead weight staging
  widening the overlap window, and fail-fast retire (2 s bound < Metal's 5 s event-wait
  kill). Untested by us; a cheap T10-P6-shaped probe can test commit-time-enqueue +
  event-chaining on the folded body.
- **887's inert channel_fraction has a complete live design**: per-Mac calibrated T(s)
  fit with the bandwidth-interference term, 5% floor, remember/forget-on-breaker.
  Splash's measured reality (~1.3–1.45× on the split FFN layer ≈ 1.15–1.25× e2e prefill)
  brackets our composed 1.87× ceiling (Bench 846) as the optimistic edge.
- **Scope law**: splash splits FFN-only, dense-only — never a mixer (the recurrent-state
  cap taken to its structural conclusion; matches our oMLX recurrent-safe finding).
- **Numerics decisions for any future W8A8 arm**: SiLU via tanh ("the ANE's sigmoid
  differs 6.5% RMS at small gate pre-activations vs 1.6% through tanh");
  dequant-by-1/128 so fp16 cannot overflow; differentiated Hadamard blocks (input 128 /
  intermediate 512 — "larger span spreads their larger outliers further"); fp16 range
  ±65504 bounds gate pre-activations; int8 leaves 2.5–2.8%, split-vs-GPU KL 1e-4..7e-4;
  verify-with-NaN-canary at startup (NaN-prefill the ANE output so unwritten rows show).
- **Bonsai-2 PQ2_0 gets NO ANE in splash**: the split refuses model-level-rotated
  projections (PQ2_0 is `gguf_embed_rotated_pq20`) — their SSD stress test served Bonsai
  PQ2_0 GPU-alone (`performance.md:109-110`). No splash ANE row exists for our league
  model; our ANE lane would be differentiating.
- Compile cache: FNV-keyed dir under DARWIN_USER_CACHE_DIR, purge-on-failed-load
  recompile, 120 s compile limit, private-API pinned by arm64 type-encoding checks that
  fail loud. Warm load ~0.1 s vs 9–30 s cold (our bank: 226.3 s every start; T5(a)
  plans the decomposition).

Recommended artifacts (mapped lane's to execute, NOT this verdict's): plan-643 T-R1
addendum citing the in-program-rotation prior art; issue 887 addendum naming the live
calibration design; a T10-P6-shaped commit-time-enqueue probe.

**2. Decode league.** Splash claims 74–92 tok/s on Qwen3.8-27B decode (M5 Pro 20-core /
M3 Max 40-core) vs llama.cpp 16–17 / +MTP 20–27. ⛔ NOT league-comparable: their llama.cpp
pin is **upstream `e6ab7c1`, not our PrismML fork** (our fork cells score far above
16–17), different model variant (dense Qwen3.8 vs our GDN Bonsai), and absolute numbers
on different engine+box are provenance, never headroom (the league law). The honest
reading: their decode lead comes from DFlash2 speculation + width-4 fixed-shape batching
(`SPLASH_MAXIMUM_BATCH_WIDTH == 4`, only the four real M8/M16/M24/M32 shapes) + the MPP
staged tile — i.e., speculation is THE decode lever on Metal, which is exactly where our
own lane just landed (Bench 924: the freerun MTP head at 0.808× of the ordinary pair).
Their accuracy method is unusually rigorous (16,384 positions, top-1 agreement
99.30–99.45% on 27B; differing positions are near-ties, median 0.03–0.10 nats apart) —
more Issue-940 corroboration (the 10-28 tfdelta row already records the near-tie-flip
sentence). A league-opponent add (splash as a scored row on our boxes) is an OWNER call
(perf-rematch charter); this verdict only notes eligibility: Apache-2.0, brew-installable,
serves our exact model PQ2_0, M3 Max covered in their own tables.

**3. Prefill gap (riir-infer Issue 1146, 0.65×/0.55× of llama.cpp at w512).** Splash's
prefill lead over the SAME upstream llama.cpp is 1.49× (27B M5 Pro) — the remaining edge
lives in host-side decision content (one-wave law, staged-vs-register tile pick by
family/cores, the ANE FFN split), not exotic kernels. Supports 1146's premise that the
prefill gap is host-composition, not kernel-quality, alone.

**4. Product-coverage.** The serving-pool family is seeded (B270/271 landed 11+ splash
serving leads); the ANE-host micro-family is new (5 candidates above). Follow-up pool
(accumulation clause): splash (this), mlx, ddalcu/mlx-serve (B242, 31 rules),
Layr-Labs/mlxfast-qwen38-125b-a6b-engine (B276 rider), oMLX (B114) — the Apple-silicon
serving family is one of the corpus's most productive veins; splash is its
production-grade center.

## Yield estimate

10 kernel candidates + 5 micro + 5 ANE/host candidates. After ~50% coordinator
falsification (the B268 wave itself landed 11 of 28 staged, and dropped 17): **5–8 own
rules + 12 riders**. Above the usual 1–3, justified by breadth + the measured-claims
density + zero-owned dedup on every candidate.

## Honest caveats

- Unread (marked unverified — re-read at mine time): `Scheduler.cpp` (819 lines; hpp +
  internals.md admission section read), `Engine.cpp` (2,603 lines; decode-cycle
  composition via targeted greps + docs), `KvPool.cpp`/`KvCache.cpp`/`PageStorage.cpp`
  bodies (headers + doc), `server/*.py` bodies, `DFlashDraft.cpp`, the VerifyBudget
  consume-site in `Runtime.cpp`, `ops/Linear.cpp` `takesPlaneViews()` body (the
  "unrotated projections" semantics is inferred from `unsupported()`'s message +
  internals.md — labeled inference).
- The shallow clone has no git history — new-vs-old adjudicated by corpus content, not
  commit range. The 10-28 tfdelta sweep's "post-cover delta EMPTY" was commit-log-level;
  this verdict's candidates are corpus-dedup-level (zero owners), which is the binding
  gate.
- Splash's Bonsai-PQ2_0 ANE exclusion follows from the unsupported() reading, not an
  explicit statement in docs (labeled inference; their performance.md Bonsai row used
  the GPU alone either way).
- Production-grade source, no WIP signals in scope; fault-injection instrumentation,
  sanitizer-run calibration tests, and a "measured gates, not a guarantee" honesty
  posture throughout.
- Splash moves fast (release-grade, multiple merges/day at times) — quotes pinned at
  `cabbad27`; re-verify at mine time per the skill.

## Queue slot

Plan 308 (nominal **Batch 303**), staged behind plan 307's live claim (Batch 302 pool,
expires 2026-10-30T16:2xZ) and the B300 perfect-hash slot. Zero candidate overlap with
plan 307's inventory (verified by grep at landing). The batch's landing duties: the
standard rust_perf/kernel_opt delta set (freeze regen only if rust_perf lands;
retrieval floors re-measure at both postures; clippy ×3 postures; count pins — the
B299 lesson: ~37 asserts, not 5).
