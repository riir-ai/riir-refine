# Research 421 — anemll-forge ANE/GPU hybrid prefill distill

**Status:** DISTILLED — Claude verdict AGREE (2 rounds, session c676122d) recorded 2026-11-18; staged for the next batch window (Plan 387 carries the tasks)

**Prior verdict reconciliation (the 2026-09-30 verdict is the controlling record):** the queue snapshot already carries a DISTILL VERDICT on this exact repo — 2026-09-30, `@ 117ded62a79806fa96554b5a34eb2d5c055b15eb` (MIT, the then-main tree): **MARGINAL C+ corpus / LANE INTEL HIGH — no batch, no plan, no riir-refine note**, lane record `../riir-ai/.research/392_Anemll_Forge_ANE_GDN_Numerics_Distill.md` (@ 391→392, + the Research-388 f16-GDN-state addendum). That verdict ALREADY adjudicated most of what the historical imports carry, and this pass RE-READS those adjudications rather than re-claiming them: the absorption identity was extracted as the QUEUED LEAD `operand-rescale-exact-norm-absorption` (B− content, "queued opportunistic at the next GDN-kernel touchpoint, no slot consumed"); the silu re-fusion hazard was extracted as "the NEW half: coremltools `mil_backend::fuse_activation_silu` FUSES the x·sigmoid(x) workaround back — check the COMPILED graph"; the lazy-commit GDN rollback was folded as "gdn-transition-replay-log-commit-many B178/B182 second instance" (the rule EXISTS — grep verified this session; this note's K1 below is downgraded accordingly); the multifunction weight-duplication finding was lane intel PRICING the laya six-BC1S-bucket design (the per-stage-tap CPU_ONLY-vs-CPU_AND_NE debugging method and the fp16 numerics triple were named there too); the league classification ("the FIFTH Qwen3.8-family Apple runtime, FIRST ANE-class — non-scoring reference") predates this note. **What is genuinely NEW at this pin is the `v8-gpu-ane-prefill` branch itself** — the V8 hybrid research, SPLIT+GPU decode work and server findings are dated 13–17 Nov 2026 and did not exist at the 09-30 pin. This note therefore RE-GRADES THE BRANCH DELTA: 7 new-atom candidates (A4, A6, A8, A9, A10, R2, A7) + 2 new faces (F3, F2's partial-sum pole) + the new-owner face opportunities (A5/F2 target rules that LANDED after 09-30: entries_74 at B358 2026-11-13, ane R2 at Plan 377 2026-11-17), while A1/F1 and the league classification are tagged RE-READ of the 09-30 record. The upgrade MARGINAL C+ → YES B− is carried by the branch delta alone; re-grade provenance per candidate in the table below. T0 of Plan 387 additionally diffs the two pins (`git diff --stat 117ded62..912925f -- research/ *.md`) to prove the NEW/RE-READ tags at mine time.

Source: `Anemll/anemll-forge`, branch `v8-gpu-ane-prefill`, shallow clone `.raw/anemll-forge`,
PIN `912925f7f0579c0918786267a7457f7abb36e597` — tip commit-dated 2026-11-17T18:33:00-08:00
("Docs recheck: decode write-up status, README links to the article and build"), **MIT** (ANEMLL
contributors, 2026). User-directed @distill+@research of the repo plus the two linked research docs
(`research/v8_prefill/README.md`, `research/v8_prefill/PREFILL_RESEARCH.md`); zero cargo, zero GPU
(m3 docs-only lane). Clone REMOVED at close of the recording commit.

## Verdict

**YES B− corpus — BRANCH-DELTA RE-GRADE** (the 2026-09-30 verdict @ `117ded62` judged the then-main tree MARGINAL C+ corpus / LANE INTEL HIGH with one queued lead and no batch; the `v8-gpu-ane-prefill` branch — 13–17 Nov V8 research, SPLIT+GPU decode, server findings — is new material this pass, carrying 7 new-atom candidates + 2 new faces; the historical-import atoms are RE-READS of the 09-30 adjudication, tagged per candidate below) — mixed transcription batch (ane-domain majority + rust_perf caching; nominal window after the staged polars lane; Plan 387) /
**LANE INTEL HIGH** (updates the 09-30 classification "FIFTH Qwen3.8-family Apple runtime, FIRST ANE-class — non-scoring reference" with the fresh same-box tables; feeds riir-infer Issue 1179's paused specdec lane — a lane that did not exist at the 09-30 verdict — + one bench-hygiene blind spot in the GPU-exclusivity law: ANE load moves GPU numbers +43–45%, invisible to GPU monitoring and to pmset) /
**product-coverage YES** (the ane family — 6 liquidcodec rules, landed 2026-11-17 via Plan 377 — gains its second source; neither the family nor entries_74 existed at the 09-30 verdict, so the face opportunities are new-owner, not re-claims).

## What the source is

ANEMLL's research forge for Qwen3.8-27B on Apple silicon: the v8 branch carries the **V8 hybrid**
(13–17 Nov 2026, M6 32 GB) — every FFN on the Neural Engine, every mixer (48 Gated DeltaNet + 16
attention layers' cores) on the GPU, pipelined prompt blocks so both units stay busy — plus an
OpenAI-compatible server, DFlash2 specdec (all3 drafter), and a per-variant measured research
log. Published result: V8 prefill beats splash 1.3.2 **+23–37% at every length** on the same box
(586/658/620/541/420/347 tok/s at 2K/8K/16K/32K/64K/95K vs splash 476/480/454/404/329/283);
SPLIT+GPU decode ties splash at long context (141/149 ms per cycle at 64K/95K).

The repo also carries five measured Apple-feedback-grade findings
(`FEEDBACK_ANE_MULTIFUNCTION_MEMORY.md`, `ANE_DELTANET_NUMERICS.md`) and a full Metal
decode-attention kernel census (`pipe/unit/DECODE_GPU_ATTENTION.md`). The top-level research notes
are historical imports ("results were not rerun"); the v8_prefill docs are fresh and served as of
17 Nov.

## Ranked candidates

All suggestion-only **NotBenchable-by-domain** (GOAT criterion 1 fails for a DOMAIN reason — the
win metrics live in the ANE/CoreML runtime: wired-memory deltas, ANE call latency, model KL — no
corpus validator hosts them; the dial9/Batch-136 annotation class, never a demotion). Rust
fixtures are original reproductions per the transcription law; every quote below was read
directly at the pin this session.

| # | candidate | domain | grade | source file | notes |
|---|---|---|---|---|---|
| A1 | `ane-stage-parity-gate-cpu-vs-ane-same-model` | ane | **B− (RE-READ — demoted from lead)** | `ANE_DELTANET_NUMERICS.md` | The validity-gate law: whole-model gates are BLIND to per-stage accelerator numerics bugs — "Neither bug is visible in cosine-similarity checks of the whole model (the model still generates fluent text), in the M3U KL runs (PyTorch), or in small test graphs (they run on the CPU)". The method: "Look at per-stage tensors of the ANE program, compare CPU_ONLY vs CPU_AND_NE of the SAME compiled model, and check magnitudes (subnormal fraction) and near-zero activation accuracy." Measured: two fp16 bugs made every DeltaNet layer ~50% wrong while end-to-end NLL looked *better* (the "copying artifact", −0.906 nats): native silu ~1e-3 absolute error near 0 → 12% relative on conv outputs (99% of values in [−0.5, 0.5]), and fp16 subnormal q·S (61% subnormal) → 20–51% error. Fix: tanh-form silu + GDN_SQ=16/GDN_SV=64 prescale → 51%→1.3% rel error, trace ppl 2.651→2.420, **cost none measurable** ("the extra elementwise ops disappear in the weight-bandwidth-bound chunk time"). UPSTREAM: their own landed fix (`ane7f`/`ane7i` — decision content, never a PR). **RE-READ tag: the 09-30 verdict ALREADY named the per-stage-tap method and the fp16 numerics triple as lane intel for laya** (`../riir-ai/.research/392`: "supplies the per-stage-tap debugging method (CPU_ONLY vs CPU_AND_NE of the SAME compiled chunk + subnormal-fraction checks)") — what remains NEW-CORPUS is only the RULE FORM (a stage-parity gate as an ane-domain corpus entry; the 09-30 pass kept this material out of the corpus at MARGINAL C+). Grade carries that history; the batch lane decides whether the rule form earns a slot or rides the 09-30 lane-intel classification. Dedup: liquidcodec R6 owns RETENTION (palette depth); THIS owns per-stage VALIDITY — axis distinct, both greps recorded. |
| A4 | `ane-int8-matmul-shape-silent-wrong-gate` | ane | **B (LEAD)** | `DFLASH2_ANE_PLAN.md` §5 + `PREFILL_RESEARCH.md` lesson 3 | "Gate and up as separate K=5120 matmuls compute **wrong results** (rel 0.98) at any width, and so does 17x1024 without the K split at 1024 rows (rel 1.24), with nothing to signal it. Always check ANE INT8 output against a host reference." The law: an ANE INT8 matmul's correctness is SHAPE-dependent (K-split required past a threshold; fused gate+up required) and fails SILENT. **NEW-ATOM** (not in the 09-30 record; T0 confirms via the pin diff whether the §5 text predates `117ded62` — either way it was never adjudicated). Distinct from A1 (fp16 execution-path wrongness found by stage taps vs quantized-form shape wrongness found by direct host check) — merge decision at authoring with the two regimes carried in-rule. |
| A8 | `ane-hybrid-pipeline-block-granularity-not-layer-interleave` | ane | B− | `PREFILL_RESEARCH.md` (V8 vs splash) | **NEW-ATOM** (the v8 branch, 13–17 Nov). The headline finding, as a placement law: a dual-accelerator partition that splits WITHIN a layer idles the second unit — splash's ANE "takes 76% of each FFN's channels inside each layer and waits while the GPU runs the mixers (busy about 42% of each 2,048-row chunk)" (hybrid speedup 1.31–1.51× over GPU-only), while V8 pipelines whole ops per unit across prompt BLOCKS (FFN→ANE 1.56× better there, mixers→GPU; both units stay busy; speedup 1.60–2.08×). The per-layer comparative-advantage table (ANE LUT FFN 23.2 µs vs GPU 36.3 µs per token-layer; mixers ~equal) is the admission rule's shape. Adjacent-distinct: liquidcodec R3/R4 own placement REPORTING and graph restructure, not the pipeline-granularity decision. |
| A5 | `ane-entry-point-weight-share-verify-per-runtime` | rust_perf (face-or-rule on entries_74) | B− | `FEEDBACK_ANE_MULTIFUNCTION_MEMORY.md` | **RE-READ finding, NEW-OWNER face**: the 09-30 verdict already carried this finding as lane intel ("Core ML wires N× weights for N resident buckets; Core AI entry points SHARE one copy — Apple's own AFM package does the same" — pricing the laya six-BC1S-bucket design). What is new since: the landed rust_perf rule `one-program-many-functions-accelerator-service-memory` (B358, 2026-11-13) did not exist at 09-30 — the finding can now land as a FACE on it (0.69 GB chunk: +~1.0–1.05 GB per added function, all three load paths measured; Core AI +0.04–0.09 GB per added entry, up to 4 entries). The law: deduplicated-on-disk ≠ shared-in-memory — residency sharing is a per-runtime property; measure the second function's wired delta before sizing. FACE on entries_74 (count-neutral) preferred — decision at authoring. |
| A6 | `ane-host-output-pool-unbounded-call-bound` | ane | B− | `FEEDBACK_ANE_MULTIFUNCTION_MEMORY.md` §2 | **NEW-ATOM-STAGED** (the doc existed at `117ded62` and was read; this section was never adjudicated — T0's pin diff proves the atom's provenance). Long-running ANE inference loops die: "every call allocates new output NDArrays (12 per chunk call), the runtime's output pool keeps growing (per-chunk call time 8.7 -> 12.9 ms, wired memory +0.5 GB over 600 calls), and after ~850 function calls the process aborts" (NDArray+Pool fatal). No caller-supplied outputs API exists; reload costs 60–80 s. The rule: bound call counts + watch wired memory + budget the recycle; the leak is host-runtime-owned (cannot be fixed at the call site). |
| R2 | `snapshot-state-kv-prefix-consistency-drop-invalid` | rust_perf (caching) | B− | `V8_SERVER.md` + server section of `PREFILL_RESEARCH.md` | The consistency law: "a summary request (another system prompt) rewrites the KV rows under the main conversation's 'system' snapshot. The V8 cold run did not replace that snapshot, so the next main request paired the main system prompt's DeltaNet state with the summary's KV rows" — silent wrong state. Fix, verbatim: "Snapshots that are not a prefix of the KV cache's tokens are now dropped, so that request runs cold." Plus the reuse half ('sysk': keep the system prompt's KV rows across summary requests — 8.4K tokens restored in 17 ms vs ~13 s cold) = a SECOND engine instance of the STAGED `hybrid-state-prefix-snapshot-reuse` candidate (11-08 splash blog entry; fold together at the batch lane). Dedup: `wal-watermark-pinned-before-snapshot` (entries_80) owns capture ORDER; `snapshot-then-release-shared-drafter-context` (entries_81) owns release ordering; THIS owns joint state/KV consistency under a second writer — zero owners (greps recorded). |
| K1 | ~~`gdn-speculative-lazy-commit-pending-rows`~~ → **CORROBORATION, not staged** | kernel_opt | — | `DFLASH2_ANE_PLAN.md` §4 | **RE-READ — the 09-30 verdict already folded this exact design**: "lazy-commit GDN rollback (unit-lower-triangular exactness + min(·,0) NaN guard + β=g=0 exact-noop masking) → `gdn-transition-replay-log-commit-many` B178/B182 second instance" — the owner rule EXISTS (grep verified this session: `rule_id: "gdn-transition-replay-log-commit-many"` in the kernel_opt corpus). This note's original K1 staging was a dedup miss (the corpus grep covered B337 but missed B178/B182); the row is DEMOTED to a second-instance confirmation recorded beside the owner rule at the batch lane's discretion, never a new entry. The supporting quotes stand as read: "Exact for any k: the system is unit lower-triangular, so the solution's leading rows do not depend on later rows"; the `min(., 0)` clamp; the EIR constraint (matmul of state-derived data fails -14; N² = I − (I−N)(I+N)) — all in the 09-30 record's source read. |
| A9 | `ane-prediction-gap-fewer-bigger-calls` | ane | B− | `PREFILL_RESEARCH.md` (lessons + V8 fixes) | "Fewer, bigger ANE predictions. Every evaluation has a fixed 0.11-0.23 ms gap that no submission trick hides." Only fewer/bigger calls help — "`ffn2` entries run two 64-row blocks in one function, saving one gap per pair" (2K 566-582) — but "128-row ANE calls stay 19% worse per token than 64-row ones": the gap is per CALL, so batch rows per call up to the per-row efficiency cliff. Kin: entries_74 `one-program-many-functions...` owns the program-count memory axis; THIS owns the call-latency axis. The Instruments evidence (0.23 ms idle gap before every ANE evaluation, present in splash's own method too) is the regime proof. |
| A10 | `ane-concurrent-same-function-silent-corruption` | ane | B | `PREFILL_RESEARCH.md` (V8 fixes) | "one ANE thread per pair: two threads calling the same Core AI function concurrently (different bindings) corrupt outputs at random (KL spikes 0.6 to 4)" — a silent-corruption concurrency hazard (lesson 2: "Never call the same Core AI function from two threads at once, even with different bindings"). Fix-space: serialize per function handle / one call thread per program; the corruption is undetectable at the call site (random KL spikes), so the rule is the guard, not the check. |
| A7 | `ane-package-size-build-gate-before-kernel-panic` | ane | C+ | `FEEDBACK_ANE_MULTIFUNCTION_MEMORY.md` §4 | **NEW-ATOM-STAGED** (doc predates `117ded62`; this section was never adjudicated — T0 pin diff proves provenance). 8+ layer / 1.1 GB Core AI packages panic the KERNEL on load ("panic(cpu 7...): Break 0x0800... panicked task `aned`"), reproducibly even standalone; 4-layer (0.52–0.72 GB) stable for thousands of calls; a package-format version mismatch "segfaults the process instead of failing with a version error"; watchdog resets during 32K/64K compiles. The rule: gate size/complexity at BUILD time and version-check before load — the panic class is uncatchable at runtime. Thin Rust shape; face-or-rule at authoring. |

**Faces (count-neutral, ride their owners):**

- **F1** — **RE-READ, recorded beside the 09-30 rider**: the re-fusion hazard was already extracted by the 09-30 verdict as "the NEW half: coremltools `mil_backend::fuse_activation_silu` FUSES the x·sigmoid(x) workaround back — check the COMPILED graph" (routed to the transcendental-form family). This pass adds nothing new beyond the tanh-form survival datum — the face rides the 09-30 record, never landed as a new claim.
- **F2** — **FOLDED into the 09-30 QUEUED LEAD `operand-rescale-exact-norm-absorption`**: the absorption identity "rms(s o, eps s^2) = s o / sqrt(s^2 mean(o^2) + s^2 eps) = rms(o, eps)" IS that lead (B− content, "queued opportunistic at the next GDN-kernel touchpoint, no slot consumed" — quote from the 09-30 queue entry). The NEW delta this pass contributes to the lead's notes: (a) the SUBNORMAL FLOOR pole (scale UP: q ×16 / v ×64 escapes 61%-subnormal q·S — the 09-30 record carried the magnitude datum but not the pole framing), and (b) the PARTIAL-SUM overflow pole, never adjudicated: "the ANE accumulates (unscaled LUT values x input) and applies the per-channel scale afterwards, so the pre-scale partial sums are ~1/scale (~100x) larger than the output" — static down-input ×64 overflowed from layer 32 on (+0.71 nats end-to-end), the dynamic variant broke further (NLL 12) and cost ~1.5 ms per verify call. Input prescale on an accumulate-then-scale backend is UNSAFE; scale at the weights/outputs or absorb downstream. Landing: the queued lead consumes these deltas at its GDN-kernel touchpoint (no separate slot).
- **F3** on rust_perf entries_74 `ane-split-calibration-fit-bandwidth-interference-term`: **NEW** (the coupling section is 13–17 Nov, absent from the 09-30 record). The M6
  pair decomposition refines the interference term from scalar-bandwidth to WORK-CLASS-dependent:
  bandwidth is shared ~additively (GPU stream alone 156 GB/s, ANE alone 121, both 87+77=164 of
  170) while compute interference is super-additive and class-dependent — "The GPU attention call
  at 60K takes 18.7 ms alone and 26.8-27.1 ms (+43-45%) under an ANE load. The ANE FFN slows 3 to
  20% under GPU load"; "A 153 GB/s GPU memory stream costs the ANE only 7% and ALU FMA 3%, but
  the int8 DeltaNet loop costs 20%"; "In V8 the GPU slows only once the ANE is busier than about
  50%". Likely cause "a shared power or clock limit on the matrix units (not measured)". Plus the
  wake/spin interference: a GPU warm-up kick helps GPU-only (0.25→0.09 ms) but "with the ANE
  running, the GPU waking mid-call makes the ANE call ~1.3 ms longer. A spinning CPU core does the
  same (+1.5-2 ms)" — the keepalive family's idle-window obligations gain a fourth datum (wake
  work during co-execution interferes).
- **F4 positive instances** (record, never land): the engagement-proof law at home — an earlier
  decode-kernel batch "had silently run the unmodified kernel because zsh does not word-split
  `env $cfg`"; corrected with "one command buffer of 50 copies, output checksum per variant" =
  our Issue-1124 T1.5 law + the checksummed-arm fixture discipline, independently arrived at.
  Interleaved A/B thermal discipline ("both units slow under sustained load with no pmset
  warning... The first rep after a cool-down is about 5% fastest. Configs are compared only
  interleaved (A B A B)") = the bench_preflight box-state law's positive instance, with one new
  face: ANE thermal does NOT surface in pmset (the box-state detector's blind spot).

**Lane-intel annotation (not a rule):** the decode-attention kernel census — all flash-decode
variants plateau at 0.86–1.04 ms/1K keys vs the 0.22 ms/1K read floor ("All of them stay near
35 GB/s: latency, not bandwidth or compute"; serial per-threadgroup key walk, one row tile,
occupancy limit); the served shape census (16 rows × 4 simdgroups, 256-key tiles best of
{32,64,128,256,512}); every prefetch form measured NEGATIVE (+11–86%: "the lines are evicted
before the matmuls read them"; double-buffering +86%; "the extra plain loads cost even without
extra traffic"); softmax costs nothing as math (no-exp == FP32 == FP8-P within 0.7%; ~8% is the
S/P threadgroup round trips). Metal-specific — stays in the note as the decode-kernel
measured-negative family, not a corpus entry.

## Lane mapping (the mandatory axis — fires HIGH)

1. **League reference rows (third engine).** anemll-forge serves the SAME model family as the
   league's qwen3.8 rows (Qwen3.8-27B) on M-class silicon with published per-context tables vs
   splash 1.3.2 on the same box: prefill V8 +23–37% at every length; decode cycle anatomy (drafter
   13–16 ms + ANE verify 85–90% + ~1 ms host); cold acceptance 2.4–3.7 tokens/cycle (splash
   2.6–4.4); SPLIT+GPU ties splash at 64K/95K. Absolute numbers are M6 provenance, never headroom
   (the league law). Recommendation: the league doc's splash block gains an anemll reference row
   (named, not executed — charter lane).
2. **riir-infer Issue 1179 (qwen3.8 specdec verify lane) — PAUSED 2026-11-18, verified live this
   session** (Status addendum, delegated-owner verdict d9c2bb28: T1 closed — uzu DFlash mean_j
   1.27 < the 2.5 bar, Path A closes; re-open triggers (1) trained head ≥3.2 held-out mean_j,
   (2) serving-side consumer, (3) new draft-head proposal pre-registering its own stop line).
   The anemll feed for trigger (3): their DFlash2 measures **2.345 mean accepted / 3.345
   tokens-per-target-call on the bf16 target** (16 mixed prompts, thinking on; math 3.5–4.9, code
   1.5–4.5, explanations 1.6–2.0, Chinese 1.0) — and their own open item names the lever our
   closed T1 implies: "a short distillation of the drafter on our (quantized) target's own
   features and outputs could raise acceptance". The head-sizing sweep prices the trade:
   full 248K head −1.6% tokens/call vs bf16, 32K head −10.9% — "Verify dominates the cycle, so
   acceptance is worth more than drafter milliseconds" (the ≤8 ms drafter variants LOSE). The
   ANE drafter call lands at 9.5–16 ms ≈ 12% of the cycle. Named update for 1179, not executed
   (paused lane, no claim).
3. **Static-shape ANE serving crossover law** (design feed, no open lane): ANE verify entries are
   a STEP FUNCTION of context (16K/32K/64K/96K entries: 84.6/92.5/107.6/122.4 ms per verify
   regardless of exact length); the GPU split follows actual length (~92 ms + 0.24 ms/1K keys) and
   wins just past entry boundaries, loses below; entry count is capped by the ~51-bound-tensor
   runtime limit ("the 96K entry already binds 5 segments per attention layer (the binding count
   limit: about 51 per function)"). The dispatch rule (pick the cheaper of quantized-entry vs
   dynamic-split at the actual length) is the shape any future ANE serving lane in riir-infer-ane
   would adopt.
4. **Bench-hygiene blind spot for the GPU-exclusivity law** (our lanes): ANE activity moves GPU
   numbers (+43–45% on attention calls, measured) and is INVISIBLE to GPU monitoring — CoreML/ANE
   background work during a league bout or correctness gate would contaminate without any GPU-side
   signal, and ANE thermal does not surface in pmset. Named face for
   `riir-reflex/scripts/bench_preflight.sh` + the riir-ai GPU-exclusivity AGENTS row; not executed
   from this verdict.
5. **NOT transferable (the closes):** the Core AI/HWX zero-copy posture needs SIP-off + a root ACL
   on the aned cache and GPU tables pinned to one compiler build ("they match only this compiler's
   output (M6, macOS 27.3 build 26C5035r)") — a research-only posture, not a shipping lane; their
   export (vq3pA 3-bit VQ MLP + LUT mixers, Core AI programs) is not our GGUF lanes; M6 silicon;
   Core AI's own GPU path is 20–100× off (`ffn2_L0` 89.5 ms vs 4.6 ms in their Metal engine — the
   GPU half MUST be a real Metal engine, which we have).

## Dedup evidence (greps this session)

- ane corpus = 6 rules (`ane-fixed-window-mask-stateless-streaming`,
  `ane-fp16-norm-pow2-prescale-overflow`, `ane-compute-plan-pin-cpu-and-neural-engine`,
  `ane-graph-restructure-for-ane-lowering`, `ane-precision-dsp-stays-cpu-fp32`,
  `ane-weight-palette-depth-retention-gate`) — zero hits for
  stage/parity/validity/silu/concurrent/unordered/output-pool/panic/size-limit/prediction-gap/
  placement-granularity/prefix-consistency across ane + rust_perf + kernel_opt + sec corpora.
- **The workspace-research sweep (the internal-first rule — run AFTER the verdict round 1 caught
  its omission):** `Anemll/anemll-forge` already carries a 2026-09-30 distill verdict (queue
  snapshot, MARGINAL C+ / LANE INTEL HIGH) + the lane record `../riir-ai/.research/392_Anemll_Forge_ANE_GDN_Numerics_Distill.md`.
  That record owns: the absorption identity (queued lead `operand-rescale-exact-norm-absorption`),
  the silu re-fusion rider, the lazy-commit→`gdn-transition-replay-log-commit-many` B178/B182
  fold (grep-verified: the rule exists), the multifunction lane-intel, the per-stage-tap method,
  the fp16 numerics triple, and the league classification. All RE-READ tags above cite it; the
  internal sweep of OTHER workspace `.research/` folders for this source name found no further
  prior distills.
- The Research-345 rust_perf five **LANDED at B358** (entries_74: `one-program-many-functions...`
  + `ane-split-calibration-fit-bandwidth-interference-term`) — my A5/F3 faces target the LANDED
  rules, which did not exist at the 09-30 verdict (B358 = 2026-11-13).
- silu appears in sec only via B352's activation-expansion rule; subnormal via B352's rescale rule
  — both now also owned record-side by the 09-30 riders (F1/F2 dispositions above).
- kernel_opt owns `gdn-state-snapshot-per-position-verify-early-exit` (B337) AND
  `gdn-transition-replay-log-commit-many` (B178/B182) — K1's adjudication now cites the latter as
  the actual owner.
- rust_perf owns `wal-watermark-pinned-before-snapshot` (entries_80) +
  `snapshot-then-release-shared-drafter-context` (entries_81) — R2 adjudicated in-rule both
  directions; the STAGED `hybrid-state-prefix-snapshot-reuse` (11-08 splash blog entry) is the
  reuse-half fold target.

## Yield estimate (restated from NEW atoms only)

NEW-ATOM candidates: A4 (B), A8 (B−), A9 (B−), A10 (B), R2 (B−), A6 (B−), A7 (C+) = 7; new faces:
F3 + F2's partial-sum pole (folded into the 09-30 queued lead); new-owner faces: A5 on entries_74,
the queued lead's own landing at its GDN-kernel touchpoint. RE-READS (never staged as new): A1
(rule-form question rides the batch lane), K1 (corroboration of `gdn-transition-replay-log-commit-many`
B178/B182), F1 (the 09-30 rider), the league classification (updated numbers only). After ~50%
coordinator falsification: **3–5 rules + 2–3 faces from the branch delta** (A4 near-certain; A8,
A10, R2 strong; A9, A6 contestable; A7 thin). All suggestion-only, all `bench_spec: None`
NotBenchable-by-domain disclosed per rule.

## Honest caveats

- The top-level research notes are historical imports ("results were not rerun during
  localization"); the v8_prefill docs are fresh and served as of 17 Nov — quote vintage differs
  per file, recorded per row.
- Fast-moving branch; re-clone + re-pin at mine time (T0).
- Silicon/OS-pinned: GPU tables match one M6 compiler build; absolute numbers never transfer.
- One run per cell in places (their own disclosure: "One run per point; different weights").
- UNREAD (re-read rows in Plan 387 T0b): `QUANTIZATION_NOTES.md`, `RESULTS_M5_MAX_INT8.md`,
  `COREAI_PORT_NOTES.md`, `research/v8_prefill/pipe/V8_SERVER.md` body (grepped, not fully read),
  `coreai/` + `scripts/` code, `prototypes/*.py`, the article (`ARTICLE_ANE_GPU_M6.md`).
- Research-skill notes: no training-track classification (inference-serving distill); the one
  training-flavored finding (drafter-on-own-target distillation) is a named feed under 1179's own
  paused lane, nothing filed from it here — adversarial panel not fired (no optimizer/loss/RL
  framing in the classified surface). No katgpt-rs primitive candidate (the mechanisms route to
  the ane/rust_perf/kernel_opt corpora and riir-infer lanes, not generic modelless math).

## Queue slot

Nominal next Protocol-A transcription batch window (after the staged polars lane); claim
plan-first per Plan 387. NOTHING INVENTED — every quote read directly at the pinned clone this
session. Session: zed-m3-anemll-distill, epoch 1795007987.
