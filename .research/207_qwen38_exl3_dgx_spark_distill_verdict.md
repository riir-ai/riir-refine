# Research 207 — vcruz305/Qwen3.8-Flash-Next-EXL3-DGX-Spark-recipe distill verdict

**Status:** RECORD — mined as **Batch 177** (2026-09-25, Plan 170). Mine-time
verification appendix at the end of this note.

**Verdict:** **YES, B — corpus** (route `kernel_opt`, with one `rust_perf`
measurement-honesty member) **/ LANE INTEL HIGH — riir-infer quantization zoo**
/ product-coverage does NOT fire.

Queue slot: nominal **Batch 177**, plan **170**. Lead with
`aa-control-prices-the-fidelity-instrument`.

## Source

- **Repo:** https://github.com/vcruz305/Qwen3.8-Flash-Next-EXL3-DGX-Spark-recipe
- **Pin:** `@ e0ebee8ddc4391b5e66d86fdce993b0756e00986` (verified by `rev-parse`
  on the clone; tip commit dated 2026-09-20, *"Move custom OpenAI wrapper to
  beta_testing/, TabbyAPI is the default API"*).
- **License:** MIT (Copyright (c) 2026 Victor Cruz, vcruz305).
- **Inspected:** 2026-09-24, shallow clone at `.raw/qwen38-flash-next-exl3-dgx-spark`,
  **removed at landing** per the `.raw` discipline. Quotes re-verify at the pin.
- **Upstream provenance chain, three deep** — the patches are `git format-patch`
  of the author's own fork [vcruz305/exllamav3 PR #2](https://github.com/vcruz305/exllamav3/pull/2)
  (fork `master` @ `523ecd3`), which forks
  [turboderp-org/exllamav3](https://github.com/turboderp-org/exllamav3) (MIT),
  whose EXL3 format is a streamlined variant of **QTIP** (Cornell RelaxML,
  NeurIPS 2024, arXiv:2406.11235) — trellis-coded vector quantization with
  procedural codebooks and incoherence processing. Cite the fork for the CUDA
  shape, turboderp for the format, QTIP for the math.
- **Sibling repo, not inspected:** `vcruz305/Qwen3.8-Flash-Next-EXL3-SGLang-DGX-Spark-recipe`
  (the SGLang twin, MTP EAGLE fix + native EXL3 draft, 0.47–0.72 accept,
  22–31 tok/s). Named as a follow-up lead, **not graded** — unread files are
  never graded.

## What it is

A serving recipe for turboderp's Qwen3.8-Flash-Next EXL3 pack on **one NVIDIA
DGX Spark (GB10, 128 GB unified aarch64)**, two paths: the exllamav3 **native
engine** (primary, tuned) and **vLLM + vllm-exl3** (secondary). 81 tracked
files; a 72 KB README that is the bulk of the asset, 3 merged kernel/engine
patches, ~20 Python/shell measurement harnesses, and a `logs/` tree of raw
matrices.

Model geometry (README-verbatim): `Qwen4ExpForConditionalGeneration`, 48 layers
(36 linear attention / 12 full, `full_attention_interval: 4`), **512 experts
top-10**, hidden 2560, 2 KV heads at head_dim 256, one MTP layer, a per-layer
n-gram (PLE) embedding table of **320,001,536 rows × 160 at K=5**, and a vision
tower. `max_position_embeddings` 262,144.

⚠ **This is a different model from the Research 120/139 arena backbone.** That
one is Qwen 3.8 **27B** (64 layers, hidden 5120, vocab 248320) on MLX/Metal.
This is Flash-Next on CUDA/GB10. Same *family name*, different geometry,
different silicon, different runtime — no measurement crosses between them.

**Source class:** serving-recipe / measured-config, which the skill grades
corpus-MARGINAL *by construction*, with the escape clause "mine them only when a
component survives the dedup gate as a new rule." **Five components survived**,
three of them carrying the author's own merged kernel diffs with quoted
before/after numbers — which is why this grades above the C+ recipe precedent
(`humanrouter/qwen38-flash-6bit-m3-ultra-recipe`, 2026-09-05, ~250 LOC of
helpers that were nearly all instances of B92–B106).

## Dedup-gate evidence (both vocabularies, over `src/domains/`)

Source vocabulary: `trellis` **0** · `exl3` **0** · `exllama` **0** ·
`GatedResidual` / `gated residual` **0** · `int8 mixer` **0** ·
`pruned.*lm_head` **0** · `device-resident draft` **0** · `unified memory` **0**.

Corpus operator-name variants (the load-bearing half):

| probe | hits | adjudication |
|---|---:|---|
| `A/A` / `null control` / `control.*indistinguishable` | 0 | novel |
| `nondeterministic.*temperature` / `engine.*nondetermin` | 0 | novel |
| `observer.*perturb` / `instrument.*overhead.*attribut` | 0 | novel |
| `dequant.bound` / `decode.bound` / `bandwidth.bound` | 0 | novel |
| `residency.*audit` / `bytes by dtype` / `unquantized.*minority` | 0 | novel |
| `per-row scale` / `scale.*factors out` | 0 | novel |
| `column slice` / `asymmetric.*draft.*verify` | 0 | novel |
| `acceptance.*collapse` / `draft.*precision.*floor` | 0 | novel |
| `piecewise` / `graph.*split` / `graph.break` | 0 | novel (see LEAD below) |
| `pinned` | 68 | **LEAD, read:** the pinned-vs-pageable H2D staging in patch 0003 is a positive instance of the existing pinned-memory family. KILL as a rule. |
| `page.cache` / `fadvise` | 4 | **LEAD, read:** hits are `rust_perf` io + one `kernel_opt` entry. The *device-allocatable* framing is the delta; re-read at mine time before landing #7. |
| `argmax` (25) / `MTP` (25) | — | the B115 family; see the #5 adjudication below |
| `readout-trim-needs-committed-id-coverage` | **LANDED** | `entries_gpu_26.rs` (B115 addendum). Candidate #5 is a **rider correcting its scope**, not a duplicate — see below. |
| `dead-result-timing-loop-folded-under-lto` (B109) · `env-gate-reaches-and-arms-the-scored-process` (B115) | LANDED | the measurement-honesty family's members 1 and 2. Candidate #1 is member **3** and is about the *comparator*, where those are about the *work* and the *config*. |

Cross-checked `graph`/`bandwidth` hits by reading them rather than counting:
the CUDA-graph entries are B110 capture-time work omission
(`entries_gpu_22.rs:37`), B110 degenerate selection, and B58/B22
scalar-baking — none is about *a host sync forcing a PIECEWISE-only graph and
the split carrying its own separable price*. The bandwidth entries are B114
recurrence-prefill restructure (`entries_gpu_24.rs:26`) and B05 upload
parallelism / GEMV — none is about *dequant compute displacing bandwidth as the
decode bound*. Both distinct.

## Ranked candidate table

| # | Candidate rule | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | **`aa-control-prices-the-fidelity-instrument`** — before using output-text equality as a fidelity gate, run the **A/A control** (the same config against itself) and compare its disagreement rate to the A/B's. Measured here: the engine is non-deterministic at temperature 0 with a fixed seed (**4 distinct outputs from 5 identical requests** on an open-ended prompt; **1 of 5** on a constrained one), and **spec-on vs spec-on scored the same 2-of-6 identical as spec-on vs spec-off** — *"the control is indistinguishable from the test, so a text diff measures engine noise."* Fall back to a counter-based signal (acceptance counters from `/metrics`). Second clause: the divergence is **prompt-class-conditional** — open-ended generations diverge, tightly constrained ones do not, *"which is what near-ties in the logits resolved by varying reduction order would look like"* — so a fidelity suite of constrained prompts reports a clean instrument that has no power. | README §"How things were measured"; `scripts/probe_greedy.py` (which the author marks as **predating** the finding — the superseded instrument is left in-tree with the reason, the honest form) | **A− (GOAT lead)** | Measurement-honesty **#3**, and the first member about the COMPARATOR rather than the work (B109) or the config (B115). Zero dedup hits. Directly load-bearing at home: our G1 bit-identity gates assume a deterministic reference, and the rule states the precondition that assumption needs. |
| 2 | **`quantized-pack-residency-is-set-by-the-unquantized-minority`** — audit parameter bytes by `(class, dtype)` FIRST on any new pack: the fp16 hyperconnection mixers in an otherwise **3-bit** pack are **3.2 GB resident**, 13.2 MB × 96 sites = **1.27 GB read per decode round, ~20% of the round at ~44% of spec bandwidth** (each `(M+1, R)` block re-reads its weight row per stream). The headline bpw describes the majority and predicts neither residency nor round time. Fix regime: int8 + per-row fp32 scale quantized at load — and the reason it is exact is the placement, **the scale is constant along the contracted dim so it factors out of the k-loop**, leaving the fmaf chains and reduction order byte-identical to the fp16 kernel (only the weight load changes). The fp16 fused-path copies are then freeable (1.6 GB back); the prefill path is untouched. **The precision ladder is a measured two-sided bound:** int6 rejected for lane-aligned loads, int4 rejected because a simulated int4 mixer **collapsed acceptance to 20%**. | patch `0001` commit body + `hc_mix.cu` (+256 ln), `hyperconnections.py`; `tuning/mixaudit.py` (*"How the 3.2 GB of fp16 GatedResidual weights in a 3-bit pack was found. Run this first on any new pack."*), `mixq.py` (quantize→dequantize simulation that **decides whether a kernel is worth writing** before it is written) | **A−/B+** | Measured 74.0→79.1 / 55.5→62.5 / 49.0→53.0 t/s (code/devops/prose); parity 1–2e-4 rel mixed, ~1e-7 post, R=1..8; isolated per-call 1.14–1.34×. Two rules may be right here (the AUDIT discipline and the SCALE-FACTORS-OUT exactness placement) — coordinator's call. |
| 3 | **`bpw-reduction-buys-memory-not-decode-when-dequant-bound`** — on a trellis/codebook format the **per-weight dequant cost barely grows with K**, so decode is bound by dequantization rather than bytes moved and the usual "lower bpw ⇒ faster decode" heuristic **inverts**. Measured 3.05 vs 4.05 bpw, same build and harness: decode **52.22→48.70** @4k, **50.53→51.31** @32k, cold prefill **1,142→1,137** — all inside the sample spread. The cost is entirely memory and it is severe: **+22 GiB resident** (79→100.81 GiB), utilisation forced 0.80→**0.92**, context ceiling **halved 262,144→131,072**, MemAvailable 17–18 GiB → **3.4 GiB**. The one real gain is **acceptance 68%→74%** — a higher-precision target agrees with its draft more often, which is a *speculation* win, not a bandwidth one. | README §"The 4.05 bpw revision" | **B+** | Two-regime, mechanism-level, the corpus's favourite shape. The rule names WHICH axis a bpw change buys on, which is exactly the question an "add EXL3" decision asks. |
| 4 | **`host-gather-splits-the-graph-and-the-split-is-the-separable-price`** — moving a 36 GiB table off-device to memory-mapped checkpoint views makes every lookup a **host synchronization point**, so it cannot sit inside a full CUDA graph: disk mode runs **PIECEWISE-only with the lookup as the splitting op**. The decision content is that the graph-mode downgrade is **priced independently of the mechanism that forced it** — PIECEWISE alone measured **5–7%** on the same model with the table still resident (48.25 / 48.01 against 52.22 / 50.53 at k=3) — so the gather's own cost is the residual, not the whole delta. On top, rows **first-touch page in from NVMe**, which is why short-prompt samples start slow (32.6–46.0 tok/s across four) while the 32k figure lands where PIECEWISE alone would put it. Net: a real configuration, ~10% slower, **+15 GiB headroom**, and 262k reachable where the resident table cannot reach it. | README §"The 4.05 bpw revision" (NVMe table half); `vllm-exl3` `NGRAM_TABLE=disk` | **B** | Zero dedup hits on the graph-split axis; the landed CUDA-graph rules (B110/B58/B22) are capture-time work-set and address-stability, a different axis. Composes with #3 as the memory-side arm. |
| 5 | **`asymmetric-draft-verify-precision-escapes-the-coverage-guard`** — the landed `readout-trim-needs-committed-id-coverage` (B115 addendum) says a readout trim is valid only if every **committable** id lies inside the retained slice. This source is the **architectural escape from that guard**: trim the **DRAFT** head only (a 65,536-column EXL3 slice of the shared 248k head: **397 MB / 1.65 ms → 105 MB / 0.39 ms**), keep **VERIFY** on the full head — then an out-of-slice draft *"becomes a rejection, not an error"*, accepted output is unchanged, and a coverage miss degrades into **acceptance loss rather than a correctness loss**. Supported by a measured prior: 97% of drafted argmaxes fall in ids < 65536 on code/prose/devops. Second clause, and it is the subtle one: the dynamic-draft **confidence must be read from the slice too** — identical when in-slice, *lower* otherwise, so dynamic drafting stops earlier, i.e. the error is forced into the conservative direction by construction. | patch `0002`, `qwen4_exp_mtp.py` (+45 ln) | **B (strong RIDER)** | **Not a duplicate — a scope correction.** The landed rule reads as an absolute constraint on trims; this names the structural condition under which the constraint does not bind. The Batch-115-addendum precedent (*"corrects this batch's claim that the arm was already covered"*) is exactly how this lands. Coordinator decides rule-vs-rider. |
| 6 | **`observer-sync-bias-differs-by-change-class`** — a harness's own per-token console write **is a host sync**, so an in-process harness reads *"~4 tok/s high for kernel changes and ~10 for host-side ones."* The instrument bias is **not a constant offset — it scales with the class of change under test**, so the same patch measures +12 t/s in-process and **inside variance (±2 t/s)** through `chat.py`. The author's resolution is the rule: quote the harness that matches the **deployed consumer** (*"kept because in-process and server (TabbyAPI-class) callers see the full effect"*), and say which one you quoted. Companion clause from the same section: under speculation **TTFT absorbs k+1 tokens that still count in `n`**, so decode figures run ~2% high — *"uniform across configurations, so comparisons hold"*, i.e. a known-uniform bias licenses the RATIO and never the absolute. | `tuning/README.md` header (*"Quote `chat.py` numbers, not in-process ones"*); patch `0003` commit body; README §"How things were measured" | **B−** | Rider on the measurement-honesty family alongside #1. The "bias scales with change class" half is the novel content — a constant offset would cancel in a ratio, and this one does not. |
| 7 | **`unified-memory-page-cache-is-device-allocatable`** — on unified-memory parts the host page cache **competes with GPU allocation**: without `posix_fadvise(DONTNEED)` on every pack file *"the autosplit loader refuses to load after any large file write"*, and back-to-back loads need **≥8 s** apart *"or the second dies to a workspace race."* The same cache is also the speedup — cold load **9.5 min from NVMe vs 2.5 min from page cache** — so it is a resource to be *dropped deliberately at the right moment*, not minimised or maximised. | `tuning/drop-model-cache.sh`; `tuning/README.md` | **C+** | 4 existing `page.cache`/`fadvise` hits — **re-read them at mine time before landing.** The device-allocatable framing is the claimed delta; if the existing io entries already carry it, this is a rider. |

### Drop rows (killed candidates, recorded for transparency)

- **Patch `0003`'s three sync removals** — `EXL3_BATCH_VERIFY` (one sampler call
  over all q verify positions + on-device draft compare with a single readback),
  `EXL3_MTP_DEVICE_DRAFT` / `EXL3_EMBED_GPU` (draft ids stay on the draft
  device), and the **pinned staging** replacing pageable `torch.zeros` whose
  every H2D upload was a blocking memcpy (1.65 ms × ndt per round). Positive
  instances of the B25/B26/B27 deferred-download family and the existing
  pinned-memory family (68 corpus hits). **KILL as rules**; the *measurement*
  around them is candidate #6, which is the part that is new.
- **"Levers that measured empty" table** — `--max-num-seqs 2 vs 4` (inside
  noise), CUDA graph mode (already `FULL_AND_PIECEWISE` on every boot),
  `--max-num-batched-tokens 16384` (+2% prefill), `--kv-cache-dtype fp8`
  (refused: *"Qwen4Exp QSA requires a BF16 main KV cache"*), MTP k=4 (wedges the
  engine after `torch.compile`). Config-knob table, ambient structure. **KILL as
  rules** — this is lane evidence and belongs in the riir-infer issue, not the
  corpus.
- **vLLM/TabbyAPI serving plumbing**, `serve_openai.py`, the Qwen-XML →
  `tool_calls` shim, `preflight.py`, `prepare_pack.sh` — ambient. **KILL.**
- **`rename_unsharded_ngram.py`** — a workaround for plugin builds before
  `94c29ba` misfiling one unsharded `I16 [320001536, 61]` tensor as a dense
  linear (boot OOM in ~100 s). A specific upstream bug's workaround, not a
  pattern. **KILL.**
- **`--gpu-memory-utilization` ceiling lore** (*"Do not go above 0.85"*, driver
  allocation failures in the kernel log, earlyoom's 3% trigger, a 0.93 attempt
  killed by the watchdog during load) — box-specific operational tuning.
  **KILL as a rule**, cite in the lane note.

## Mine-time verification appendix (Batch 177, 2026-09-25)

**Landed:** 6 rules + 1 rider (the plan's T4 fork resolved to ONE rule for
candidate #2 — see the dedup catch below).

- **#1 `aa-control-prices-the-fidelity-instrument`** — landed `rust_perf`
  (complexity, NotBenchable annotation). Quotes re-verified at the pin:
  the README measurement section + `probe_greedy.py` (still in-tree, still
  marked as superseded).
- **#2 `quantized-pack-residency-audit-unquantized-minority-first`** —
  landed `kernel_opt` (N) as **ONE rule, not two**. The T2 re-verification
  (grep under both vocabularies at current corpus HEAD, per the plan's
  mandate) caught what the verdict's phrase-grep could not see:
  `factor-affine-quant-scale-out-of-element-loop` (entries_gpu_01, Batch 18)
  IS the loop-stage half of the exactness placement — the verdict probed
  `scale.*factors out` (0 hits) while the rule's ID spells the mechanism.
  Resolution per the plan's T4 fork: the AUDIT discipline is the rule; the
  scale placement rides as its fix-regime clause with an explicit kinship
  row (format-stage CREATES the constant vs loop-stage EXPLOITS it — they
  compose). New quotes verified from the patch body this batch: the
  in-code factors-out comment with its honest fp32-rounding caveat (parity
  MEASURED at 1-2e-4/1e-7, greedy diverges from fp16 at token 157/34/14 —
  never bit-identity), the 8-byte-int8-load = same-8-wide-step note (the
  real lane-aligned reason int6 lost), the two scale granularities (per-row
  `fn` / per-channel `up`, each constant along ITS contracted dim, the
  `_quantize_int8` docstring verbatim), the mixq int8-sim acceptance floor
  (58-63% vs 63%), and the i8 A/B logs re-read (`i8_int8-on_dds_code.log`
  78.95 t/s @ 72.93% vs off 73.80 @ 73.12% — the commit body's rounded
  74.0→79.1 / 73.1→72.9 exactly).
- **#3 `bpw-reduction-buys-memory-not-decode-when-dequant-bound`** — landed
  `kernel_opt` (N). Numbers re-verified against `docs/benchmark-data.json`
  (schema v2: context_curve 52.22 @3060 / 50.53 @24354; 79 GiB resident;
  MemAvailable 17-18).
- **#4 `host-gather-splits-the-graph-and-the-split-is-the-separable-price`**
  — landed `kernel_opt` (D).
- **#5 `asymmetric-draft-verify-precision-escapes-the-coverage-guard`** —
  landed as the **rider on `readout-trim-needs-committed-id-coverage`**
  (entries_gpu_26), per the verdict's own rule-vs-rider fork: the landed
  rule's guard is economic (forced rejects vs trim savings) and this names
  the structural condition under which it does not bind. Patch 0002 read
  in FULL this batch: the fail-closed `_pruned_head_cache = False` guards
  (non-trellis / biased / scale-free heads take the full path) verified in
  the diff body.
- **#6 `observer-sync-bias-differs-by-change-class`** — landed `rust_perf`
  (complexity, NotBenchable). The tuning/README.md header quote verified
  verbatim at the pin, plus the rider clause the verdict also carried (the
  sampler-pinning discipline: `temperature-0.8 moves MTP acceptance 58-75%
  run to run`) folded into the annotation.
- **#7 `unified-memory-page-cache-is-device-allocatable`** — landed `kernel_opt`
  (D) as a RULE, not a rider: the 4 existing `page.cache`/`fadvise` hits were
  READ at current HEAD (B123 read-ahead duplication, B94 cacheability-gated
  coalescing, B118 parallel cold page-ins, + mmap/prefault mentions) — none
  carries the device-allocatable framing; `drop-model-cache.sh`'s header
  comment verified verbatim (the mem_get_info() refusal + the 9.5/2.5 min
  cells + the >=8 s workspace race).

**Dropped as adjudicated** (no changes): patch 0003's three sync removals
(pinned staging = the 271-hit pinned family; batched verify + device-resident
ids = B25/B26/B27), the levers-that-measured-empty table, the vLLM plumbing,
`rename_unsharded_ngram.py`, the gpu-memory-utilization lore.

**Quotes that failed re-verification:** none — every load-bearing quote
re-verified at `e0ebee8d`. The repo moved +2 restructure commits past the
pin (TabbyAPI-primary, vLLM to plugin dir; no measurement content in the
subjects) — the pin proved load-bearing exactly as the verdict warned.

**Gates:** batch177 presence/concepts/parses ×3 (kernel) + presence+annotation
(rust_perf) green; lib default 2286/0; kernel_heal 2645/0; kernel_heal+latent
2818/0; retrieval floors 21/21 (209 s); clippy `-D warnings` clean both
postures (4 doc_markdown backticks healed mechanically via `cargo heal`, 4
edits compile-gated); score-bench row #165 heal 0.96363634 / created 0.0182
— byte-identical to B174's baseline (the clippy corpus untouched; no
freeze-table regen owed). Pins walked: kernel 627→631 (router + per-category
table D 136→138 / N 44→46 + two 627s inside legacy batch tests — the B129
lesson again), rust_perf 153→155 (total + without_spec 57→59 + three
annotated-count 98→100 sites + header + build doc).

## GOAT candidate test (#1, `aa-control-prices-the-fidelity-instrument`)

1. **Benchable** — **NO, for a DOMAIN reason**, and that is an *annotation, not
   a demotion*: the rule's content is that a fidelity comparator has a noise
   floor which must be measured before the comparator is trusted, so there is no
   latency quantity to Pareto-ratio. `bench_spec: None` / `NotBenchable`, the
   **B109 precedent in this exact family** (whose own annotation reads *"the
   rule's content is that the baseline folds to zero, so there is no nonzero
   quantity to ratio"*) and the Batch-136 precedent for landing NotBenchable
   decision-content rules as first-class.
2. **Exact semantics** — n/a for a ratio; the machine-checkable surrogate is
   the **A/A disagreement rate**, which is a counted quantity with a known
   direction (an A/A rate at or above the A/B rate ⇒ the instrument has no
   power). That is the same shape as B109's zero-baseline assert.
3. **Magnitude story** — mechanism-level and measured: 4-of-5 distinct outputs
   at temperature 0 with a fixed seed, and an A/A that scores *identically* to
   the A/B (2 of 6 both ways). The failure is not a small bias, it is **the
   entire signal**.
4. **Deterministic** — the rule is; its subject is precisely a system that is
   not, which is the point.

Criterion 1 fails for a **domain** reason, so per the skill's two-class split
(the 2026-09-17 dial9 correction) this is an ANNOTATION and the verdict stays
**YES**. Candidates #2, #3 and #4 carry ordinary measured numbers and are
benchable in the measured-GOAT distillation shape (B49/B52/B54/B55) if the
coordinator wants a benched lead instead.

## Lane intel — the axis FIRES, and it is the owner's actual question

The ask that opened this session was *"we need EXL3 support"*. The mapping is
exact and it is **not** katgpt-rs:

- **`riir-infer` OWNS the target surface.** `riir-infer/BOUNDARY.md`: *"the
  quantization zoo (q2k → q8kv, q2_0 ternary, PTQ/TurboQuant)"* under
  `riir-infer-core`. **Zero** `exl3` / `exllama` / `trellis` / `qtip` hits in
  that repo — format absent, home unambiguous.
- **katgpt-rs is the WRONG home by its own contract.** EXL3 is a weight format;
  the modelless-first mandate and the BOUNDARY domain test (*"is this a
  modelless inference primitive with no riir dep?"*) put a weight-format loader
  outside it. File in riir-infer.
- **The dispatch question is already settled in our corpus.** riir-ai Research
  360 (`.research/360_M4_Prefill_Engine_Metal_Kernel_Distill.md:118`) already
  names **"EXL3 codebook"** as one of six formats in a Universal Quant Router
  and adjudicated the router shape a *positive instance of B29
  `enum-dispatch-separate-kernel-per-variant` + B23, "No action."* So the
  **shape** of adding a format is decided; only the format itself is missing.
- **The lineage is already known on the research side.** katgpt-rs Research 502
  places **QTIP** in the AVQ2 / affine-lattice / VPTQ / additive-quantization
  lineage. EXL3 is the streamlined QTIP variant — so an EXL3 issue inherits an
  existing literature read rather than opening one.

**The transferable intel — and it is a warning, not an encouragement:**
candidate #3 is the decision-relevant finding. **EXL3's bpw advantage converts
to memory headroom, not decode speed, when the decode is dequant-bound.** An
"add EXL3 support" issue written around expected decode wins would be promising
the wrong axis; the defensible promise is *context ceiling and residency*, with
acceptance as a secondary speculation-side effect. Candidate #4 is the second
half: if the pack does not fit, the host-gather escape exists and costs ~10%,
of which 5–7 points are the graph-mode downgrade and not the gather.

**NOT transferable, each with its closer:**

- **Absolute tok/s** (79 / 62 / 53 native; 52.22 @4k vLLM; 1,142 cold prefill) —
  GB10 aarch64 unified 128 GB, a 512-expert top-10 MoE, EXL3 3.05 bpw. Our
  league cells are 4090/M3 on Bonsai ternary. The league's non-scoring
  discipline owns this: **format/model/box-crossed rows are never a headline and
  never a gate change.**
- **MTP / speculation economics** (acceptance 68–74%, draft depth sweeps, the
  `-dds -dc 0.6` dynamic-draft policy) — riir-ai **721/717 closed our
  speculation path structurally**. Evidence for that verdict, not a reopen.
- **The vLLM / SGLang plugin surface** (`vllm-exl3`, `sglang-exl3`,
  `--quantization exl3`, `tensor-parallel size 1 only`) — not our runtime.
- **The Research 120/139 arena numbers** — *different model and different
  silicon from this repo*, despite the shared "Qwen3.8" name. Do not let the
  name join them.

**Follow-up offered (named, NOT executed — a verdict does not edit sibling
repos):** file `riir-infer/.issues/NNN_exl3_trellis_format_support.md` carrying
(a) the format reference — turboderp-org/exllamav3 `exl3.md`, MIT, QTIP
arXiv:2406.11235 as the math; (b) the **dequant-bound caveat from candidate #3
as the issue's own scoping clause**, so the issue promises headroom rather than
speed; (c) the Research 360 router adjudication as the already-decided dispatch
shape; (d) this note's pin `e0ebee8d` as the measured-configuration reference.
**Take the next issue number from that repo's `.issues/.highwater` at write
time** — not from this note.

## Product-coverage axis — does NOT fire, stated rather than skipped

The source's patterns live in **CUDA C++ / torch extension / Python serving**,
not in a top-tier Rust ecosystem library (tokio, rayon, serde, regex, bevy).
`cargo heal` consumers do not write this code. No family is seeded and none is
gapped; the axis is recorded as checked and empty so the next reader does not
re-derive it. (Candidate #1 is the partial exception — it is *domain-general
measurement discipline* that fires on any A/B harness, Rust included — but it
seeds no library family.)

## Caveats

- **Recipe-class source.** The unit of value is a qualified CONFIGURATION; the
  corpus value is the five surviving components, and that is a narrower asset
  than the file count suggests. 72 KB of the repo is one README.
- **Single box, single author, n=1 per cell.** Every number is one GB10. The
  author's own patch `0001` says *"Default off pending a second box's
  numbers"* (later flipped on in fork PR #3) — the honesty is a good signal and
  the sample size is still one.
- **Fast-moving beta upstream** — exllamav3, `vllm-exl3` (`94c29ba`) and the
  fork all move. **The pin is load-bearing**; re-verify every quote at
  `e0ebee8d` at mine time.
- **The engine is non-deterministic** (candidate #1 IS this caveat). Any
  re-measurement of this source's numbers inherits that; use the acceptance
  counters, not text diffs.
- **Unread / ungraded:** the SGLang sibling repo (named as a lead), the full
  bodies of `hc_mix.cu` (+256 ln — header and commit body read, kernel body
  skimmed), `docs/benchmark-data.json`, and the `logs/` matrices. **Re-read at
  mine time; nothing unread is graded here.**
- **License hygiene:** MIT source, and the corpus rule stands regardless —
  fixtures are ORIGINAL minimal reproductions, never pasted CUDA.
- **Candidate #2 may be two rules** (audit discipline vs exactness placement)
  and **#5 may be a rider rather than a rule.** Both are flagged for the
  coordinator rather than pre-decided here.

## Yield estimate

7 candidates in. After the ~50% coordinator falsification this corpus's history
predicts: **3–4 rules + 2–3 riders.** #1 and #3 are near-certain (zero dedup
contact, strong independent decision content). #2 is near-certain as at least
one rule. #4 carries falsification risk against the CUDA-graph family on a
coordinator re-read. #5 is a rider by expectation. #6 is a rider. #7 is the most
likely outright drop — it has live dedup contact.

## Bottom line

**Mine as one batch — nominal Batch 177, Plan 170 — leading with
`aa-control-prices-the-fidelity-instrument`.** Separately and independently of
the batch, the lane deliverable is the **riir-infer EXL3 issue**, whose most
important content is the candidate-#3 caveat: on this evidence EXL3 buys
**memory headroom and context ceiling, not decode throughput**.
