# Research 395 — LoNE (Liquid on Neural Engine) distill verdict

**Status:** DISTILLED-BATCH-LANDED — Batch 352 / Plan 362 landed 2026-11-13 (4 rules into the sec Numerics family, all suggest-only `NotBenchable-by-domain`: the subnormal rescale + the wide precision split + the activation expansion + the position-keyed cache mask; T0 re-verified at the disclosed newer pin `afb99463` — tip moved `4806171d`→`afb99463` on ring-cache plumbing only, all four mechanism sites byte-identical)

**Source:** https://github.com/AJ-Gonzalez/LoNE @ `4806171daa1af35333d9bc291d8fc5f478984bf8` (main tip, 2026-11-09) — **Apache-2.0**. Inspection date 2026-11-12 (amo, user-directed distill lane). Clone inspected in `.raw/LoNE` (shallow), removed after recording per the `.raw` lifecycle; the pin above is the provenance for every quote.

## What the source is

Python (2,065 LOC across 11 modules) — LFM3/LFM2.5 on the Apple Neural Engine via Core ML: converter (`torch_model.py` builds static-shape TorchScript graphs, `convert.py` traces + palettizes), runtime (`runtime.py` owns cache tensors, drives predictions), OpenAI-compatible server, and a parity verifier (`verify.py`). **Serving-recipe / measured-config source class** — its unit of value is the QUALIFIED CONFIGURATION plus four measured numerics rules, not kernels. Graded on the lane axis first, corpus axis second (the class's own construction makes it corpus-thin; it cleared anyway — see below).

The four headline findings, all measured on LFM3-1.7B / LFM2.5 (M1, 8 GB, Core ML → ANE fp16):

1. **fp16 subnormal products** — the ANE's linear cores lose precision when input×weight products fall into fp16's subnormal range (conv `out_proj`: input absmax 0.066 × weights ~0.02). A power-of-two weight rescale (×2^4, undone ×1/16 on the output) is exact in real arithmetic and restores normal-range accumulation: operator cosine 0.99916 → 0.9999998. Applied to ANY projection with reduction dim k > 1024 (`_WIDE = 1024`, `scaled_weight()`).
2. **Wide reductions split** — one wide output projection loses precision vs two half-width ones summed (k=6656 → 0.9972 cosine; k=3328 → 0.999994). FFN fan-in > 4096 splits into 2 (`ffn_split`); the LM head splits at hidden > 1024 (hidden 2048 on one linear measured 0.992).
3. **Fused-activation approximation** — Core ML maps `aten::silu` (and `x * sigmoid(x)`) to an NE activation with ~1e-3 relative error; the explicit exp form `x / (1 + exp(-x))` keeps the op off that path: gate 0.99935 → 0.999991.
4. **Depth compounds drift** — per-graph depth is the parity axis, not per-op parity: 1 layer/graph 0.99986 logits cosine, 2/graph 0.9968 (230M); over LFM3's 24 layers hidden cosine falls 0.999995 → 0.99958 even with every rule applied (greedy diverges from HF at token 6/12 despite 0.99977 logits cosine — the honest limit line).

Plus the quantization ladder (coremltools `OpPalettizerConfig` kmeans palettization, applied post-conversion on the mlpackage, NE-native LUT reads): fp16 0.99977 @ 13.2 tok/s · LUT8 2.2 GB **0.9985** @ 23.1 tok/s (1.75×, the only post-training format near the bar) · LUT6 1.8 GB 0.9526 (needs QAT) · LUT4 1.4 GB 0.786 (dead). The measured cliff: each extra LUT bit buys ~0.04 logits cosine in the 6–8 range. And the serving facts: ANE decode varies **2–13 tok/s with weight residency** (mmap-backed weights page out under pressure; `--wire-caches` pins caches with `mlock(2)`), and the M1 `MLComputePlan` **declines the small LFM2.5 graphs** (230M/350M ran the CPU path; only LFM3-1.7B placed on the NE).

## Verdict

**YES B− corpus / LEAGUE INTEL HIGH** (riir-infer ANE lanes). Product-coverage axis: does not fire — the patterns live in Core ML/ANE conversion, not a famous-lib family.

**Domain routing: `sec` domain, Numerics family** (the Issue-185 precedent: `sigmoid-minus-one` / `softmax-onehot-collapse` / `neg-ln-softmax-prob` are already Numerics-family suggest-only entries — "numeric hazard with a bounded repair" is an established genus here; a backend-arithmetic-parity hazard is the same shape). The tempting WRONG domain is `kernel_opt` ("ANE is an accelerator") — rejected: the rules are conversion-graph numerics transforms with no kernel we author; kernel_opt fixtures are kernel-shaped Rust and its numerics entries (`compiled-vs-eager-per-op-exactness-admission`) own the serving-admission axis, not the conversion-transform axis. `rust_perf` rejected: no perf claim exists in any candidate (the LUT8 speedup is lane evidence, not a code pattern).

**GOAT identification (the two-class split):** all four candidates fail GOAT criterion 1 for a DOMAIN reason — the win metric is logits cosine / operator parity, which no validator hosts, and no candidate claims a perf win. That is the per-rule `NotBenchable` annotation (`bench_spec: None`), not a demotion (the Batch-136/258/291 precedent). Graded on decision content × novelty × dedup clearance × yield.

**Provenance note:** this is primary research (the author's own measurements, no upstream chain to cite) — every number quoted above is from the README's measured tables or in-code comments at the pin. UPSTREAM-state (P113): every fix-shaped row is **present by construction** — their own landed decisions at the pin; decision content only, never PR material.

## Dedup gate (grep evidence, both vocabularies)

| pattern | source term | corpus-term grep | verdict |
|---|---|---|---|
| subnormal-product rescale | `subnormal`, `_SCALE`, `scaled_weight` | `subnormal` → entries_25 (F16→F32 widening **lane-merge perf** rule — the word appears only in conversion impl detail), gpu_106/97/50 (GPU accumulator contexts) | CLEAN — nobody owns the backend-subnormal-product rescale decision |
| wide-reduction split | `ffn_split`, `_WIDE`, wide reduction | `reduction split\|split reduction` → gpu_127/04/116/60/01 — split-K/work-partition **perf** class | CLEAN — parity-motivated splitting is unowned; in-rule adjudication vs the split-K family required at authoring |
| fused-activation expansion | `silu`, `aten::silu` | `silu\|swish` → gpu_96/35/126/147 — GLU/activation **fusion** perf rules | CLEAN-but-LEAD: opposite axis (those FUSE for speed, this EXPANDS for parity); the vocabulary collision makes an explicit in-rule adjudication mandatory |
| position-keyed cache mask | `attention_mask`, `past \|\| new` | (no hits — grep position-keyed mask vocabulary) | CLEAN |
| ANE/Core ML class | `ane`, `coreml`, `neural engine` | zero domain-corpus hits (src/ hits were non-domain files) | CLEAN — no ANE-domain rules exist |

## Ranked candidates

| # | pattern | source | grade | notes |
|---|---|---|---|---|
| 1 | `fp16-subnormal-product-power-of-two-rescale` | `torch_model.py` `_SCALE`/`scaled_weight` | **B** | weight ×2^k (k=4) at conversion, output ×2^-k; exactness argument carried in-source ("power-of-two scaling is exact in fp16 unless it overflows"); threshold k > 1024; measured 0.99916 → 0.9999998. Rust fixture: `half::f16` wide-dot with subnormal-range products vs the rescaled path, cosine/abs-error assert. UPSTREAM: present by construction. |
| 2 | `wide-reduction-precision-split` | `torch_model.py` `DecoderLayer.ffn_split` / `Head.split` | **B−** | split a projection at a width threshold into halves summed at the output; thresholds 4096 (FFN) / 1024 (head) / 1024 (any reduction, via rule 1's gate). Measured: 6656-wide 0.9972 vs two 3328 0.999994. Distinct from split-K (parity motivation, sum-at-end, no work-partition content). Fixture: f16 wide matvec vs two-half sum. |
| 3 | `fused-activation-approximation-expansion` | `torch_model.py` `silu()` | **B−** | when the backend runtime substitutes an approximate fused activation (~1e-3), expand to the primitive composition to escape the approximation path. ⛔ in-rule adjudication vs the GLU-fusion family MANDATORY (opposite axis — fuse-for-speed vs expand-for-parity). Fixture: `approx_silu` stand-in vs exp-form, error bound assert. |
| 4 | `position-keyed-additive-cache-mask` | `layout.py` `attention_mask()` | **C+** | caches preallocated to full ctx → additive mask keyed by ABSOLUTE position, never buffer index (fp16 sentinel −65504); small but bounded + authorable. May ride as a face on #2 if the batch wants 3 entries. |

Drops / annotations (recorded, not graded rules):
- **Graph-depth parity budget** (1 layer/graph; 24-layer compounding) — annotation on #2's admission context: per-op parity does not compose; depth is the compounding axis. Fix-space is conversion partitioning (a lane decision), not a code pattern.
- **Two-phase parity verifier** (subprocess reference + `.npz` handoff for memory-bound boxes; BAR 0.999; per-layer drift curve; first greedy divergence) — we own G5 (`tests/laya_ane_parity.rs`: top-1 ≥ 99.9% + near-tie band + published p-drift); corroboration, not a rule.
- **`mlock` weight wiring** (`--wire-caches`, the 2–13 tok/s residency swing) — lane intel (Plan 616 axis), not a Rust fix pattern.
- **Runtime-owned caches / no Core ML state buffers / `past‖new` concat** — TorchScript-era Core ML design constraints; ambient for our host (which owns its own gather/dequant seams).
- **LUT palettization ladder** — lane intel (Plans 612/615), not a code pattern; see the lane-intel section.

## Lane-intel output (the HIGH half)

**Mapped lanes (riir-infer):**

1. **Plan 612 — laya ANE e8 int8-table stack** (`FINAL 2026-10-10: promotion DECLINED` — G1 FAIL english top-1 0.9545 < 0.999, max prob err 12.7×/5.9× fp16; G2 FAIL typed +6.9% paired delta; recorded rescue conditions: scale-axis regen + a typed-perf mechanism, each alone insufficient). **LoNE adds a THIRD rescue axis:** kmeans-codebook palettization at 8 bits measured 0.9985 logits cosine across a whole 24-layer model ON THE ANE (LFM3-1.7B, M1) at 1.75× fp16 decode — codebook-adaptive quant vs our per-row linear-symmetric int8 (`ane_convert.py`: "closed-form min/max, no codebooks"). Honest mechanism caveat: LoNE's LUT8 is INSIDE the ANE graph (NE-native LUT reads, all weights); our e8 is a HOST-side gather dequant of the embedding table only — the transferable datum is the codebook-vs-linear axis + 8-bit viability, not a drop-in fix. **Follow-up offered:** file the datum as a plan-612 addendum / new riir-infer `.issues/NNN` (content: this section, pin sha, Apache-2.0) — the sibling lane acts, this verdict does not edit riir-infer.
2. **Plan 615 — LUT grid lane** (CLOSED NEGATIVE 2026-10-01: 2-bit Lloyd-Max, ppl 6.8e5 vs base 49.8, "4 orders of magnitude past the usability floor") — LoNE's measured cliff independently corroborates the closure and prices the operating points: 2/4-bit dead (LUT4 0.786), 6-bit QAT-only (0.9526), **8-bit the only post-training near-bar format** (0.9985). No re-open on this datum alone (2-bit ≠ 8-bit; the lane's closure stands), but the cliff numbers belong in the lane record if it is ever revisited.
3. **Plan 616 — laya q8 device residency** (COMPLETE) — LoNE's residency swing (mmap-backed weights paging out → ANE decode collapses 6×, 13.2 → ~2 tok/s) and the `mlock` wire option are the same concern axis from the ANE-serving side; the wire-the-resident-tier idea is transferable if the ANE lane reopens on memory-bound boxes.
4. **Plan 619 — clef-text 0.6b ANE lane** — the M1 planner-decline finding (small graphs declined → silent CPU path) applies to our SMALL checkpoint class: **check at filing** whether clef-0.6b graphs place on M1/M3-class silicon or silently ride CPU. Our own `MLComputePlan`-equivalent posture summary (`convert`'s `plan_summary` + `check_plan=True` in LoNE) is the diagnostic shape.
5. **riir-reflex `ane_convert.py`** — carries NONE of the four precision rules (verified by grep this pass: no silu expansion, no wide-reduction split, no subnormal scaling — only per-row int8 quant, rope scale, LayerNorm handling). The acceptance authority for any such transform is the ANE parity gate (top-1 ≥ 99.9% + near-tie band). Whether our laya/clef shapes TRIP the thresholds (hidden > 1024? FFN > 4096?) is a filing-time check — laya/clef hidden dims needed.

**Not transferable:** absolute tok/s (different model + silicon — league law: only same-box cells own comparisons); LUT4/6 postures (below bar by LoNE's own measurement); the specific thresholds as universals (model-family-measured on LFM2.5/LFM3; carried as regime conditions, and ANE generations differ — M1 NE behavior is not guaranteed on M3/M5-class NE; our parity gate re-measures regardless).

## Caveats

- Python source — fine under the transcription law (fixtures are always original Rust); the language was never a gate.
- Young project (11 stars, 23 commits, v0.0.1, Homebrew tap) — but Apache-2.0, completed modules, benched postures, and the numbers are internally consistent (verify harness + drift curves + paired power windows). Grade at module level: everything mined here is complete.
- Single-model-family evidence: LFM2.5/LFM3 only. Thresholds and even the NE-approximation magnitude may differ per arch and per ANE generation. The rules carry regime conditions; our fixtures reproduce the CLASS, not LFM3.
- Unread files (structure-fetched only, re-read at mine time): `cli.py`, `cli_options.py`, `config.py`, `metrics.py`, `server.py`. The mechanism files (`torch_model.py`, `layout.py`, `verify.py`, `convert.py`, `runtime.py`) were read in full at the pin.
- The ANE lane context on our side is opt-in/quiet (clef-ANE compiles empty by default; e8 promotion declined; LUT-grid closed) — the corpus transcription stands on its own decision content independent of whether the lane reopens.

## Yield estimate

4 candidates → expect **2–3 rules landing** after ~50% coordinator falsification (the silu-expansion adjudication and the mask's smallness are the likely squeeze points). All suggest-only `NotBenchable-by-domain` sec/Numerics entries — zero freeze-table delta expected (no clippy-lint surface), score-bench expected FLAT, release-set binary NOT owed.

## Queue slot + plan

Nominal **Batch 352** — plan **362** (`.plans/362_lone_ane_numerics_transcription_batch.md`, this commit). Queue line recorded at the head of `.distill/001_mining_queue_snapshot.md` (same commit). Corpus counts at verdict time: derived from `pins_manifest` at authoring (the live counts were NOT re-derived for this verdict — no corpus edit was made; the batch plan re-verifies at T1).
