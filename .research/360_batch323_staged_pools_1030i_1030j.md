# Research 360 — Batch 323: the 10-30i + 10-30j staged leads conversion

**Status:** RECORD — landed (Batch 323 / Plan 323; the landing commit is the provenance).

## What converted

Two sweep windows, one batch (the b322 pool shape). Corpus **1044 → 1051** (+7 kernel_opt rules in `entries_gpu_147.rs`) + **6 count-neutral rider faces**. T0: 14/14 leads byte-verified at the pins by three parallel subagents (TensorFold @ `316b3879`, vllm @ `d90e35cf`, PrismML-Eng/llama.cpp @ `02447444` — the PRIMARY `PrismML/llama.cpp` spelling 404s; PrismML-Eng is canonical). Licenses: TensorFold Apache-2.0 (b322's correction re-confirmed), vllm Apache-2.0 (some csrc files carry no header — flashinfer/sglang provenance, MIT only via the tilelang grandparent), the fork MIT.

## Own rules (7)

| Rule | Cat | Source | The law |
|---|---|---|---|
| `nan-preserving-select-over-fmaxf-in-scale-reductions` | N | TF cef366c9+2b587959 | comparison-select keeps NaN; fmaxf launders to 0; `max_nan` amax gives a NaN block a NaN scale that propagates (no refusal gate — T0 nuance) |
| `banded-launch-order-shares-weight-tiles` | D | TF 19c491ce | launch order = pure permutation; kGemmBand=4 consecutive row-blocks share one column tile's weights in L2; byte-identical (digest rig, 14 shapes) |
| `quant-block-keyed-index-space` | L | vllm 6c19299fc #64509 | the tile lattice mirrors the quant format's [G, 8, 2] (block, pair, lane) nesting; amax = axis fold; e2m1x2 pack = adjacent pair; no reshapes |
| `simdgroup-fragment-whole-store-generation-keyed` | A | TF 0afc366c+bfb0f5a2 #1590 | element-by-element fragment stores crawl on M1-class (0.04 TFLOPS, 62–78×); whole vec-reinterpret store bit-identical; admission PER GPU GENERATION (13/14 — M3-class regressed 1.8–2.3%, the else branch deliberate; M2 admitted unmeasured) |
| `stream-through-load-no-l1-allocate-once-read` | A | TF acb21860+839778d4+5e151065 #1534 | L1 allocation keyed on REUSE not read-only-ness; once-read stream → `ld.global.nc.L1::no_allocate` (value bit-identical, reply-digest proven); prefill revisited tiles keep `__ldg` |
| `inner-loop-select-division-shift-mask-vgpr-pressure` | B | TF 33b443da #1663 | select-per-element + runtime division in the K-loop = a VGPR-pressure cliff (256 VGPRs, spilled inside the loop); shift+mask (`__builtin_ctz`) + uniform first-product branch when per ∈ {1,2,4} — the precondition is load-bearing |
| `activation-layout-mirrors-weight-bit-serialism` | L | PrismML-Eng 01ea7fcf #420 | the ACTIVATION layout permuted to mirror the consumer's bit decomposition (word j = elements j, 8+j, 16+j, 24+j); bit j of weight byte i pairs with byte i of activation word j; eight aligned dp4a; exact int32; sign algebra rides the existing isum half-slot |

## Faces (6 hosts, count-neutral)

- **gpu_99** `bit-equal-fusion-mirrors-reference-reduction-regime` ×2: the vllm DTYPE-BOUNDARY rounding-points face (#64509: the fusion rounds to the activation dtype before quantizing + the scale chain's `rcp.approx.ftz.f32` asm matches the CUDA kernel) + the prism PINNED-REFERENCE + WINDOW-DOWNGRADE faces (#418: the default float4 path pins its order with explicit intrinsics — an unpinned reference is unmirrorable; `tail_fused_exact(n,11)` fail → the window downgrades to 9, never a mix).
- **B309** `runtime-trip-count-defeats-unroll-template-the-bound`: the ALL-SLICES-ISSUED face (TF nemotron #548 — the MoE router scores load every slice's partial before the sums, fold in slice order `__fadd_rn`, MAXSK=8; Issue-940 ordered-reduction corroboration).
- **gpu_111** `pdl-admission-by-image-set-marker`: the DEPENDENTS-BEFORE-LOADS early-trigger face (vllm TRIGGER_EARLY — `gdc_launch_dependents()` fires before this kernel's loads, M ≤ 128 gate; legality keyed on the DEPENDENT's data-dependence — stated as the PDL contract, not an in-file quote per T0) + the fork #417 PDL-on-5120 arm.
- **gpu_146** `checkpoint-boot-staged-upload-bounded-bytes`: the PINNED-MAPPED STAGING face (TF GLM loader — 192 MiB mapped stage, 8 packers, 16 read-ahead readers, 99→61→51 s; the reader-depth half YIELDS to plan-310 as io intel).
- **gpu_51** `dtype-native-source-read-with-shared-admission-predicate`: the PERSISTENT-TOPK face (vllm #64138 — three legs: fold-in widening at vector loads `__uint_as_float(raw.x << 16)`, convert-at-key-site `convert_to_uint32_v2(to_float(x))`, the caller `takes_bf16_logits()` copy elimination; port guards: 128 KiB smem, stride(1)==1, MAX_VEC, float internal dtype).
- **B310** `warp-per-row-small-blocks-cta-under-load-law`: the ARCH-CONDITIONAL-ROWS face (#417: Hopper `nrows_x <= 17408` two-row gate) + the FUSION-CONTEXT-GEOMETRY face (#419: tall-mat-vec types drop to default geometry when a gate is fused — "there 8 rows per CTA ran slower").

## T0 adjudications (graded calls)

- **S-j1 split CLEAN**: (a) rounding-reproduction → face on gpu_99; (b) index-space → own rule. The vllm T0 verifier confirmed the atoms occupy disjoint code regions.
- **S-i6 face, not own-law**: the prism verifier suggested "strongest new-law candidate"; the sweep's corpus-kin reading wins — gpu_99's mirror law owns the axis, the pinned-reference + downgrade are its reference-side and refusal-arm faces.
- **S-j5 face, not own-rule**: the enriched three-leg atom is one admission axis (no materialized upcast, conversion at consumption) — gpu_51 is the host.
- **S-i5 own-law**: the T0 prism verifier's SPLIT adjudication — the producer-side activation permutation is not derivable by composition (B11 owns the weight-side dot, gpu_49 the isum transport; the corpus's planar rule is coalescing-keyed, not decomposition-keyed). Grep evidence: no owner.
- **Honesty carries**: S-i1's live training-path fmaxf + the deliberate contract negative-control; S-j2's unmeasured M2; S-j3's "not the whole difference" + the commit-vs-changelog number split (8.49→5.78 vs 8.60→5.83 — commit message cited); S-j4's per ∈ {1,2,4} precondition.

## Gates (isolated /tmp/b323_target, nice'd, under the i1117 rho legs — Plan-337 posture)

- batch323 needles+parse: **2/2**
- kernel_opt lib: **3333/0** (1 ignored)
- default lib: **2792/0**
- clippy −D ×3 postures (default / kernel_opt / all-features): **0** (6 doc_markdown mechanicals healed via `cargo refine --fix --verify --verify-args "--features kernel_opt"`; 2 PR-title embedded-quote escapes)
- kernel_opt+latent retrieval: **28/0** floors HELD (3 priced-ignored, no re-pin)
- Layer 5b2 default retrieval: **27/0**
- kernel_opt+latent lib: **3530/0** (5 ignored; 1891 s wall under the legs)
- dual_allocation 0/0/0; rustfmt clean in all batch regions (tests.rs line-291 drift is PRE-EXISTING at HEAD — another lane's region, left alone)
- Fixtures: **7/7 standalone-compiled AND executed** (test-harness + standalone main, both green) BEFORE entry
- freeze-table + score-bench NOT owed (P30 kernel_opt-only); binary rebuild NOT owed (kernel_opt outside the release set)

## Provenance

- TensorFold `ashhart/TensorFold` @ 316b3879 (pin), commits cef366c9, 2b587959, 19c491ce, 0afc366c, bfb0f5a2, acb21860, 839778d4, 5e151065, 33b443da, 7737cd33+5fde7dee+9637983a+40d063cf+f3dbae73, ab366b1e+a72b9d00+af2909ce — Apache-2.0.
- vllm `vllm-project/vllm` @ d90e35cf (pin), commits 6c19299fc (#64509), e31cf3dec (#64138) — Apache-2.0.
- PrismML-Eng/llama.cpp @ 02447444 (pin), commits 01ea7fcf (#420), b2f181d3 (#418), e20db821 (#417), 02bd091e (#419) — MIT.
- Staged rows: the 10-30i sweep (`9ec5019a1`, zed-m3-leads1031c) + the 10-30j sweep (`c24e59a30`, zed-m3-leads1031d).
- Session: zed-m3-b323, epoch 1793405869; claim `59b1ba69c`.
