# Proposal 014 — The `shader_perf` Domain: WGSL Render-Pipeline Bounded-Fix Healing as the Eighth HealDomain
Status: **ACCEPTED — owner GO 2026-09-16, executing as [Plan 134](../.plans/134_shader_perf_domain_phase0_batch1.md)** (originally DRAFT filed 2026-09-12, superseding the same-day mining-queue verdict's "defer until Tier-2 rejects exist" half — the re-grep that produced this proposal found three facts that change the sequencing argument, listed in §The problem this solves. The frame-time caveat from that verdict SURVIVES and is carried as caveat 6 + a Phase-2 gate. GO-2026-09-16 adds a consumer fact: the upcoming **shader-toy editor** — owner-stated, repo placement TBD — a live WGSL authoring surface whose submit/save gate is the natural promotion pull, strengthening the caveat-1 consumer argument beyond synthetic Tier-2 fixtures.)
Branch: `develop` (per global rule — no feature branches)
Owner: unassigned
Fusion of: Proposal 001 (HealDomain meta-architecture) × Proposal 004 + Proposal 006 (the fourth/fifth-domain playbook: boundary gate, negative control, close condition, oracle ladder) × Issue 025's `WgslScanner` (the scanner substrate — resolved, git history) × [riir-dapps Proposal 002](../../riir-dapps/.proposals/002_shader_asset_marketplace.md) (the consumer: the Stage-2 mint gate names this domain's rule surface verbatim) × the riir-shader vgpu port corpus (negative control + fixture mine — the Batch 49 "mine our own artifacts" path)
Related: [riir-dapps Proposal 002](../../riir-dapps/.proposals/002_shader_asset_marketplace.md) §Stage-2 (the mint gate: "naga validation + texture/loop caps"), [riir-shader Plan 001](../../riir-shader/.plans/001_vgpu_port.md) (the 15+ hero-verified ports; the naga-vs-Tint `ptr<workgroup>` divergence gotcha), the 2026-09-12 mining-queue verdict line (`.distill/001_mining_queue_snapshot.md`)

## TL;DR

Open an eighth healing domain — `shader_perf` — for **WGSL render-pipeline source
patterns with bounded fix-spaces**: texture-sample budgets, loop boundedness,
vertex↔fragment stage traffic, branch-vs-sample divergence, uniform layout,
derivative safety. Detection-only at the source level is an **empty field**:
tint/naga validate semantics and stop; spirv-opt fixes at the IR where the
source author never sees it; Android AGI/RenderDoc/Nsight measure frames after
the fact (prior art below). The consumer is real and named — riir-dapps
Proposal 002's Stage-2 mint gate is literally "naga validation + **texture/loop
caps**", and its Tier-2 generation lane emits the WGSL these rules grade. This
is an in-house invention in the healer pattern (the L0 corpus discipline), fed
by a CC0 technique source (halisavakis water shader — the 2026-09-12 queue
verdict) and the workspace's own hero-verified render corpus.

## The problem this solves

1. **The marketplace mints live GPU code with no static perf gate.** Proposal
   002 Stage 2 gates mint on "capture (offscreen, frame-budgeted, corr/
   region-stats) + naga validation + texture/loop caps". Today the "caps" half
   is prose — no mechanized rule set exists to grade a generated WGSL artifact
   before it costs a capture run (or a buyer's frame rate). An NFT plugin that
   tanks a game's frame time is a broken product: **perf is a correctness
   property of a live consumable**, and beauty-only gating ships regressions.
2. **The 2026-09-12 queue verdict deferred this domain on "no consumer + no
   oracle".** The re-grep for this proposal found three facts that move the
   consumer-surface argument: **(a)** Issue 025's `WgslScanner` already SHIPPED
   (`src/domains/kernel_opt/wgsl_scan.rs` — regex/line-level, corpus-cited,
   determinism + <1 ms/file G2 gate, wired into the cargo-heal `.wgsl` walk,
   suggest-only): the scanner substrate is BUILT, not greenfield. **(b)** The
   riir-shader pattern crate (Issue 007) SHIPPED (determinism goldens, 40 ns
   sampling, zero-alloc, fuzz-tested) — Tier-1 generation is live, Tier-2 WGSL
   is the next lane in the same proposal, so generated-WGSL volume is the
   trajectory, not a hypothesis. **(c)** The vgpu port corpus gives the domain
   what docker got from in-tree Dockerfiles: **15+ hero-verified effects of
   real render WGSL** (atmosphere alone = 26 files; corr 0.99+, byte-identical
   re-captures) — a ready negative control AND fixture mine.
3. **Nobody else will ship it.** gpuweb discussion #4832 asks "what static code
   analyzers are available for WGSL?" — the community answer is the compilers
   themselves (tint/naga, error-only). The LLVM "static analysis for GPU
   program performance" thread is an unbuilt proposal. spirv-opt optimizes IR
   silently and reports nothing to the author. The source-level advisory layer
   with a visual-neutrality oracle does not exist anywhere we can adopt from.

## The proposed design

### `ShaderPerfDomain: HealDomain` — the trait holds (the sec/docker precedent, now twice-proven)

```rust
pub struct ShaderPerfDomain;

impl HealDomain for ShaderPerfDomain {
    type Entry = ShaderRule;    // { rule_id, shader_concept, buggy_code,
                                //   fixed_code, family: ShaderFamily }
    type Verdict = ShaderVerdict; // { parses, entry_points_intact, bindings_unchanged,
                                  //   naga_agrees: Option<bool> }
}
// ShaderFamily: SampleBudget | LoopBoundedness | StageTraffic |
//               BranchCoherence | UniformLayout | DerivativeSafety
```

Feature `shader_perf` (opt-in, default-off). **Zero mandatory new
dependencies** — the scanner is regex/line-level, the third no-`syn` scanner
after docker and dist, extending the `wgsl_scan.rs` discipline (corpus
citation per finding, `strip_line_comment`, determinism test, latency gate).
The `.wgsl` collection walk already exists in `bin/heal.rs` (Issue 025 T4);
under `shader_perf` those files route to this domain's rule table.

### The routing split (the kernel_opt boundary, made mechanical)

`@compute` entry points → `kernel_opt` (the 607-rule compute corpus — dispatch,
buffers, occupancy). `@vertex`/`@fragment` entry points → `shader_perf`.
Mixed files (fft-ocean: compute sim + fragment shade in one module) are scanned
by both; each finding routes by its rule's family. The classifier is the
entry-point attribute set — mechanical, no heuristic.

### Rule families + seed (post-falsification landing zone 8–12)

**Class A — bounded fixes (~6–8 candidates):**
- `unbounded-render-loop-guard` — a `loop`/`while` without a compile-time
  bound in `@fragment`/`@vertex` (the TDR class; the mint gate's "loop caps"
  verbatim). Fix: max-iteration clamp + break. Visually neutral by
  construction when the bound exceeds the semantic exit.
- `loop-carried-sample-to-explicit-lod` — `textureSample` inside a dynamic
  loop (implicit derivatives in non-uniform loop trip = implementation-defined
  WGSL) → `textureSampleLevel` at the derivable LOD. The fix shape is subtle —
  explicit-LOD selection is semantics-preserving ONLY where the LOD is
  reconstructible; where it is not, the rule fires suggest-only. This is the
  domain's flagship two-regime decision content.
- `branch-divergent-sample-select` — sample-in-one-arm → sample-both + select,
  OR the reverse. Genuine two-regime: divergence cost vs always-sample
  bandwidth; ships as an annotated decision rule (the `context_notes`
  machinery), never a one-sided rewrite — the DL3059 anti-cargo-cult lesson
  from docker.
- `invariant-recompute-to-varying` — a value recomputed per-fragment that is
  invariant across the primitive → hoist to `@vertex`, pass as interpolant.
  Bounded; the StageTraffic family lead.
- `uniform-struct-tail-pad` — the kernel corpus's `pad-uniform-pod` rule,
  render-side. **Dedup expected**: may land as a citation/rider of the
  existing kernel_opt rule rather than a new entry — decided at mine time by
  the dedup gate, as always.
- `const-fold-loop-bound` — a loop bound read from a uniform buffer that is
  constant for the pipeline's lifetime → `const`-specialize. Suggest-lean:
  shader-permutation cost vs per-invocation load (the
  `hardcoded-fast-path-jit-specialization` two-regime, render-side).

**Class B — suggest-only (~4–6 candidates):** fragment sample-budget
annotation (N samples per invocation, mobile bandwidth class), vertex-stage
texture-fetch cost annotation, `dpdx`/`fwidth` inside non-uniform control flow
(a correctness hazard naga partially catches — the advisory fires
pre-render), render-adjacent compute `workgroup_size` (the existing
`single_thread_workgroup` class applied to post/bloom passes).

### Oracle ladder (`shader_verify`, graceful degradation per the Issue 018 precedent)

| Tier | Check | Cost | When |
|---|---|---|---|
| 0 | line/regex scan + text round-trip invariants (entry-point set intact, binding numbers untouched) | free | always |
| 1 | naga validation (feature-gated `dep:naga`, pure Rust, in-process) — module parses + validates post-fix | ~10 ms class | `--verify`, both fix classes |
| 2 | capture-fidelity protocol — **lives in the consumer repo, never a riir-clippy dep**: capture pre/post-heal at pinned frame + seed in riir-shader's harness; byte-identical on deterministic scenes, corr ≥ pinned threshold otherwise | seconds | G5-real, marketplace mint gate |

Tier 1 off → degrade to Tier 0 with one log line. naga is chosen over tint as
the Tier-1 validator because it is the pure-Rust crate already live in the
consumer's loop — with the KNOWN divergence pinned on record (riir-shader
Plan 001: `ptr<workgroup>` params are Tint-legal, naga-rejected): the
validator tier pins naga, records divergences, and treats the browser (Tint)
smoke as final truth — the exact oracle-split discipline of hadolint-vs-
BuildKit in docker.

## Honest caveats — READ BEFORE IMPLEMENTING (MANDATORY)

1. **The live Tier-2 consumer does not exist yet.** The pattern crate
   (Tier-1, AST-driven) shipped; Tier-2 "new WGSL" generation is designed but
   unstarted. Until it emits, this domain's standing consumer is the vgpu
   port corpus (negative control) + synthetic Tier-2-shaped fixtures (LLM
   failure classes: unbounded loops, budget-blind sampling). G5-real heals a
   synthetic artifact honestly labeled as such. If Tier-2 never ships, the
   domain still serves hand-authored effect WGSL — but say the pull is
   speculative at promotion time.
2. **The frame-time axis does not ship in the capture harness.** riir-shader
   has display FPS (`FrameTimeDiagnosticsPlugin`, HUD-only) and byte-exact
   visual determinism, but no recorded capture-side frame metric — grep
   verified 2026-09-12. So the perf half of "not just beauty but also fast"
   is, at Phase 1, **static advisory only** (sample counts, loop bounds,
   stage traffic — statically decidable costs). Measured frame-time gating
   waits on the riir-shader frame-time axis (a prerequisite this proposal
   names but does not ship — it is theirs, ours is the rules).
3. **Regex scanning on a graphics language is precision-critical.** A false
   positive on a hero-verified port poisons the mint gate (the garbage-in
   guard, the exact reason `wgsl_scan` is suggest-only today). The negative
   control is load-bearing: zero findings on the shipped port corpus, pinned
   as a gate; a real FP on a clean file must demote or kill the rule.
4. **naga is a real dependency.** Feature-gated and pure Rust, but not
   weightless. Default build stays clean (`shader_perf` off ⇒ no naga); the
   release-set question is decided at promotion, not now.
5. **The kernel_opt boundary will blur in practice.** Workgroup sizing,
   uniform layout, and staging patterns exist on both sides of the
   `@compute`/`@fragment` line. The entry-point classifier routes findings,
   but the DEDUP gate runs across BOTH corpora — expect uniform-layout
   candidates to collapse into citations of existing kernel rules (stated in
   the seed table).
6. **Visual neutrality is a protocol, not a proof.** Tier 2 lives outside
   this repo; CI drift in the consumer can silently weaken the G5 oracle.
   The bench doc must pin the harness commit alongside the numbers.
7. **The halisavakis source is technique-grade, not corpus-grade.** The
   2026-09-12 queue verdict graded it C+ corpus: its strongest perf pattern
   (distance-based tessellation LOD) is untranslatable to WebGPU (no
   hull/domain stages); the survivors (displacement-field normal
   reconstruction, gradient-LUT coloring) are one candidate each, not a
   batch. This domain's corpus is mined from the workspace's own render
   code + WGSL perf hygiene; the article is provenance, not seed.
8. **Promotion follows the docker T1.6 rule**: default-on requires a standing
   consumer running `cargo heal --domain shader_perf` in CI (riir-shader's
   lane or the marketplace mint gate). One-shot artifacts do not promote.

## Fusion lineage

- **Proposal 001** — the HealDomain architecture; sec and docker proved the
  trait holds for no-`syn` scanners twice. This is the third.
- **Proposals 004 + 006** — the domain-opening playbook verbatim: boundary
  gate, real-repo negative control, falsification discipline, close condition
  (< 6 bounded rules survive ⇒ the domain closes), oracle ladder with honest
  degradation.
- **Issue 025's `WgslScanner`** — the scanner substrate: regex rules,
  corpus-citation-per-finding, determinism + latency gates, the `.wgsl` walk
  already in `bin/heal.rs`. `shader_perf` extends the discipline; it does not
  re-build it.
- **riir-dapps Proposal 002** — the consumer contract: the Stage-2 mint gate
  names this domain's rule surface ("texture/loop caps") and supplies the
  capture-fidelity oracle half.
- **The vgpu port corpus (riir-shader)** — the docker-precedent "mine our own
  artifacts" seed: 15+ hero-verified effects of real render WGSL, corr-pinned
  and byte-identical-deterministic — simultaneously the negative control and
  the fixture mine.

## GOAT gate

- **G1**: every fixture fires its rule; **negative control: zero findings on
  the shipped vgpu port WGSL** (nonzero falsifies the scanner — the
  precision-first bias); bounded-fix outputs pass Tier-0 invariants
  (entry-point set intact, binding numbers byte-identical) + Tier-1 naga
  validation; the anti-cargo-cult refusal test (a one-sided
  branch-divergent rewrite is REFUSED) green.
- **G2**: scan latency < 1 ms/file — the existing `wgsl_scan` gate bar.
- **G3**: all lib tests + feature-combo checks green; other domains'
  retrieval floors unchanged; the N-domain router annotation re-pinned per
  the Issue 020 contract (docker's measured finding: an off-vocabulary
  centroid ABSORBS spans — expect accuracy to hold or rise); every count-pin
  site walked.
- **G4**: scan-path alloc-free (`SuggestScratch` precedent).
- **G5-real**: heal a synthetic Tier-2-shaped shader end-to-end via the
  consumer protocol: scan → fix → naga-validate → capture pre/post at pinned
  frame+seed in riir-shader's harness → byte-identical (deterministic scene)
  or corr ≥ pinned threshold. Bench doc pins the harness commit.
- **Promotion** (default-on): measured standing consumer pull — riir-shader
  CI or the mint gate running the domain in-loop.

## What ships now (riir-clippy) vs deferred

### Ships now — Phase 0 + Phase 1
`src/domains/shader_perf/` (mod + corpus + scanner extending the `wgsl_scan`
discipline + proposers), features `shader_perf` (+ `shader_verify` →
`dep:naga`), DomainRouter eighth-domain wiring + CLI routing by entry-point
classifier, boundary + negative-control gates, first batch: the Class A core
(loop-guard, explicit-LOD, invariant-to-varying) + Class B annotations, docs.

### Deferred — Phase 2 (gated on Phase 1 ≥ 6 bounded rules landed)
The f16-eligibility family, MSAA/derivative edge rules, the marketplace
Stage-2 wiring (mint-gate integration in riir-dapps — theirs), the frame-time
budget tier (gated on riir-shader's frame axis shipping first).

### Explicitly NOT shipped
Pipeline-state healing (blend/ZWrite/draw-order — host-side Rust, that is
riir-shader's own code and rust_perf territory if anything), SPIR-V/naga IR
optimization passes (source-level only), GLSL/HLSL (the workspace ships
WGSL), aesthetic judgment of any kind (the capture-fidelity gate is the
marketplace's, not the healer's), frame-time measurement infrastructure
(names the riir-shader prerequisite; ships there, not here).

## Phased rollout (sketch — a plan would expand)

### Phase 0 — scaffold
- [ ] T0.1 domain + corpus + feature + scanner (rule table reusing
      `wgsl_scan` helpers) + router/CLI wiring + control gates
- [ ] T0.2 negative-control gate over the vgpu port corpus (zero findings)

### Phase 1 — seed batch
- [ ] T1.1 dedup gate across kernel_opt + rust_perf corpora (uniform-layout
      collapse expected); author Class A core + Class B annotations
- [ ] T1.2 `shader_verify` naga tier + degradation path
- [ ] T1.3 GOAT G1–G4 + G5-real on a synthetic Tier-2-shaped shader via the
      consumer protocol + bench doc
- [ ] T1.4 docs/AGENTS/queue updates; close condition check (≥ 6 bounded)

### Phase 2 — gated (see Deferred)

## Risks

1. **Precision risk (highest)**: a scanner FP on clean render WGSL poisons
   the mint gate the domain exists to serve. Mitigation: the pinned
   negative-control gate + suggest-only until the bounded fix proves itself
   under the capture protocol.
2. **Correctness risk**: WGSL fixes that move `textureSample` calls change
   implicit derivatives — a naive hoist is a visual-semantics break. The
   explicit-LOD rule encodes this trap as its core decision content; the
   capture oracle is the backstop.
3. **Architectural risk**: kernel_opt/shader_perf boundary drift. Mitigation:
   the mechanical entry-point classifier + the cross-corpus dedup gate.
4. **Dependency risk**: naga's tree (feature-gated, default-off, decided at
   promotion).
5. **Consumer risk (honest)**: Tier-2 generation may never ship; the domain
   then serves a small hand-authored corpus. The close condition bounds the
   loss.

## Out of scope

Host-side render-graph healing; overdraw/blending pipeline state; IR-level
optimization; non-WGSL languages; the frame-time axis itself; mint/settlement
flow (riir-dapps Proposal 002's territory); any judgment of artistic quality.

## References

1. gpuweb discussion #4832 — "What static code analyzers are available for
   WGSL kernels?" (the empty-field confirmation; cited-only).
2. tint / tintd — the WebGPU reference compiler; validation-errors-only
   (cited-only).
3. naga — gfx-rs WGSL validator, bevy's front-end; the in-loop validator with
   the pinned `ptr<workgroup>` Tint divergence (riir-shader Plan 001,
   fft-ocean-surface; cited-only).
4. SPIRV-Tools `spirv-opt` — IR-level peephole optimization; fixes without
   advisory, invisible at source (cited-only).
5. LLVM Discourse — "Static Analysis for GPU Program Performance in LLVM"
   (an unbuilt proposal; the same gap on the IR side; cited-only).
6. Android AGI Frame Profiler / RenderDoc / Nsight — dynamic frame
   measurement, post-hoc (cited-only).
7. AI Co-Artist, [arXiv:2512.08951](https://arxiv.org/abs/2512.08951) +
   ShaderGPT — LLM shader GENERATION (the consumer of this gate; cited in
   riir-dapps Proposal 002; cited-only here).
8. halisavakis, "My take on shaders: water shader" (2019, CC0) — the
   triggering source; graded C+ corpus / product-lane HIGH in the 2026-09-12
   queue verdict (distilled there).
9. [riir-dapps Proposal 002](../../riir-dapps/.proposals/002_shader_asset_marketplace.md)
   — the consumer contract (distilled, this session).
10. [riir-shader Plan 001](../../riir-shader/.plans/001_vgpu_port.md) — the
    port corpus, capture oracle mechanics, naga divergence gotcha
    (distilled, this session).

## TL;DR

Ship it staged: Phase 0 + Phase 1 now (scanner substrate exists, negative
control exists, consumer contract exists), Tier-2 live volume and the
frame-time axis as the Phase-2 gates. Open Plan (next `.plans/NNN`) on
owner GO; close condition ≥ 6 bounded rules surviving falsification, else
the domain closes as a sec-style marginal and the rules fold into kernel_opt
citations.
