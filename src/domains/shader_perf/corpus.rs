//! The `shader_perf` seed corpus (Plan 134 T1.2/T1.3).
//!
//! Eight rules: one bounded (loop-guard, proposer-backed), one two-regime
//! (explicit-LOD, suggest-only v1 — the LOD is reconstructible only in the
//! screen-space-uniform class), one decision (branch-divergent-select,
//! suggest-only forever by the DL3059 refusal), one suggest-lean
//! (const-fold), three Class B annotations, and one corpus-only entry
//! (`invariant-recompute-to-varying` — not line-expressible in v1; the
//! fixture pair exists for retrieval, the scanner pattern waits for a real
//! buggy file to widen it, the `wgsl_scan` precedent).
//!
//! ## Dedup ledger (T1.1, checked against the LIVE corpora 2026-09-16)
//!
//! - `uniform-struct-tail-pad` (Proposal 014's seed candidate) did NOT
//!   ship as a new entry: `kernel_opt`'s `pad-uniform-pod` family owns the
//!   uniform-layout territory (compute-side authored, render-side shapes
//!   are the same layout question) — the citation/rider disposition the
//!   proposal predicted.
//! - `render-adjacent-compute-workgroup-size` (Proposal 014's Class B
//!   candidate) did NOT ship: `kernel_opt`'s
//!   `row-dot-split-inner-axis-block-reduce` already fires on
//!   `@workgroup_size(1…)` — a second pattern on the same textual tell
//!   would double-cite every post-process pass. The kernel scanner's
//!   findings stand; this domain adds nothing there.
//!
//! ## The `shader_concept` field carries the two-regime annotations
//!
//! The `context_notes` machinery (Bench 013) reaches the consumer through
//! `ShaderRule`'s concept text — every suggest-only rule's concept names
//! the regime in which the fix would be WRONG, so the annotation rides the
//! citation itself (the `HealEntry::context_notes` default of `None` is
//! correct here: these rules are unconditional *findings*, and the DECISION
//! text is the payload).

use super::{ShaderFamily, ShaderRule};

/// The seed corpus (BLAKE3-committable, freeze/thaw-ready — the shared
/// `Corpus<ShaderPerfDomain>` substrate).
#[derive(Debug, Clone)]
pub struct ShaderSeedCorpus {
    /// The entries.
    pub entries: Vec<ShaderRule>,
}

/// Positional shorthand (the docker/dist convention — the dashboard's
/// count needle is `    rule(`).
#[allow(clippy::too_many_arguments)]
fn rule(
    rule_id: &str,
    shader_concept: &str,
    buggy_code: &str,
    fixed_code: &str,
    family: ShaderFamily,
) -> ShaderRule {
    ShaderRule {
        rule_id: rule_id.to_string(),
        shader_concept: shader_concept.to_string(),
        buggy_code: buggy_code.to_string(),
        fixed_code: fixed_code.to_string(),
        family,
    }
}

/// Build the seed corpus.
#[must_use]
pub fn build_shader_seed_corpus() -> ShaderSeedCorpus {
    ShaderSeedCorpus {
        entries: vec![
            // ── Class A: bounded-fix core ────────────────────────────────
            rule(
                "unbounded-render-loop-guard",
                "A loop/while(true) in a @vertex/@fragment function with NO break token is provably unbounded at the text level — the TDR (timeout-detection-recovery) class that hangs the GPU on a driver-bounced iteration count, and the mint gate's 'loop caps' verbatim. Bounded fix: an iteration-count guard that breaks BEFORE the u32 wrap (a wrapping counter un-bounds the loop again at 2^32).",
                "@fragment\nfn march(uv: vec2<f32>) -> vec4<f32> {\n  loop {\n    let p = sample_scene(uv);\n    if (p.w < 0.001) { }\n    uv = uv + vec2<f32>(0.01, 0.0);\n  }\n  return vec4<f32>(uv, 0.0, 1.0);\n}",
                "@fragment\nfn march(uv: vec2<f32>) -> vec4<f32> {\n  var guard: u32 = 0u;\n  loop {\n    guard = guard + 1u;\n    if (guard >= 256u) { break; }\n    let p = sample_scene(uv);\n    if (p.w < 0.001) { }\n    uv = uv + vec2<f32>(0.01, 0.0);\n  }\n  return vec4<f32>(uv, 0.0, 1.0);\n}",
                ShaderFamily::LoopBoundedness,
            ),
            rule(
                "loop-carried-sample-to-explicit-lod",
                "textureSample inside a potentially-unbounded loop in a @fragment function: implicit derivatives are implementation-defined WGSL where the trip count is non-uniform (per-pixel divergence). TWO-REGIME — explicit-LOD (textureSampleLevel) is semantics-preserving ONLY where the LOD is reconstructible (the screen-space-uniform / fullscreen-quad class, LOD 0); everywhere else the substitution GUESSES the LOD and is a visual break wearing a perf fix's clothes. v1 ships the citation with this annotation; the bounded proposer is refused on the non-reconstructible shapes (the guessed-lod-substitution refusal).",
                "@fragment\nfn glow(t: texture_2d<f32>, s: sampler, uv: vec2<f32>) -> vec4<f32> {\n  var acc = vec4<f32>(0.0);\n  var i = 0u;\n  loop {\n    i = i + 1u;\n    if (i >= 64u) { break; }\n    let off = f32(i) * 0.003;\n    acc = acc + textureSample(t, s, uv + vec2<f32>(off, 0.0));\n  }\n  return acc / 64.0;\n}",
                "@fragment\nfn glow(t: texture_2d<f32>, s: sampler, uv: vec2<f32>) -> vec4<f32> {\n  var acc = vec4<f32>(0.0);\n  var i = 0u;\n  loop {\n    i = i + 1u;\n    if (i >= 64u) { break; }\n    let off = f32(i) * 0.003;\n    // explicit LOD 0: valid ONLY in the screen-space-uniform class —\n    // annotate + verify the scene is a fullscreen quad before taking.\n    acc = acc + textureSampleLevel(t, s, uv + vec2<f32>(off, 0.0), 0.0);\n  }\n  return acc / 64.0;\n}",
                ShaderFamily::SampleBudget,
            ),
            // ── Class A: decision rules (suggest-only FOREVER — the DL3059
            //    refusal table pins that no proposer ever rewrites these) ──
            rule(
                "branch-divergent-sample-select",
                "An if/else in a @fragment function where EXACTLY ONE arm contains textureSample: sampling on one arm makes the implicit derivatives divergent across the warp where the branch is non-uniform; hoisting the sample above the branch (both-arms) restores uniform derivatives at the cost of a wasted fetch on the not-taken path. A GENUINE DECISION (divergence cost vs bandwidth), never a one-sided rewrite — select() of two samples or a hoisted sample changes what is fetched AND the derivative semantics. Annotated citation only.",
                "@fragment\nfn detail(t: texture_2d<f32>, s: sampler, uv: vec2<f32>, k: f32) -> vec4<f32> {\n  if (k > 0.5) {\n    return textureSample(t, s, uv * 4.0);\n  } else {\n    return vec4<f32>(k, k, k, 1.0);\n  }\n}",
                "// decision, not a rewrite — the author picks per effect:\n// @fragment\nfn detail(t: texture_2d<f32>, s: sampler, uv: vec2<f32>, k: f32) -> vec4<f32> {\n  // hoist (uniform derivatives, +1 fetch on the not-taken path):\n  let hi = textureSample(t, s, uv * 4.0);\n  if (k > 0.5) {\n    return hi;\n  } else {\n    return vec4<f32>(k, k, k, 1.0);\n  }\n}",
                ShaderFamily::BranchCoherence,
            ),
            rule(
                "const-fold-loop-bound",
                "A render-stage loop whose condition reads a field of a var<uniform>: the bound is constant for the pipeline's lifetime, so the driver cannot unroll or specialize without a runtime check every iteration. Const-specializing trades per-invocation loads for shader permutations (pipeline recompiles per bound value) — a decision on permutation cost vs iteration cost, suggest-lean by design.",
                "var<uniform> params : Params;\n@fragment\nfn accumulate(uv: vec2<f32>) -> vec4<f32> {\n  var acc = vec4<f32>(0.0);\n  for (var i = 0; i < params.steps; i++) {\n    acc = acc + tap(uv, i);\n  }\n  return acc;\n}",
                "var<uniform> params : Params;\n// @fragment — specialization decision, applied via a pipeline override\n// constant or a generated permutation, never by editing the uniform:\nfn accumulate(uv: vec2<f32>) -> vec4<f32> {\n  var acc = vec4<f32>(0.0);\n  const STEPS = 16; // the specialized permutation's bound\n  for (var i = 0; i < STEPS; i++) {\n    acc = acc + tap(uv, i);\n  }\n  return acc;\n}",
                ShaderFamily::LoopBoundedness,
            ),
            // ── Class B: annotations (suggest-only) ──────────────────────
            rule(
                "fragment-sample-budget-annotation",
                "A @fragment function with >= 10 texture-fetch calls per invocation — the EXCESS class above what hero-verified ports carry (measured port max: 9, three-tsl/surface; flare-composite 7 legitimately): every fetch is a memory round trip per pixel, and the mobile bandwidth class multiplies fill-rate cost. Annotation only: which fetches are load-bearing is effect knowledge.",
                "@fragment\nfn composite(t1: texture_2d<f32>, t2: texture_2d<f32>, t3: texture_2d<f32>, s: sampler, uv: vec2<f32>) -> vec4<f32> {\n  let a = textureSample(t1, s, uv);\n  let b = textureSample(t2, s, uv);\n  let c = textureSample(t3, s, uv);\n  let d = textureSample(t2, s, uv * 2.0);\n  let e = textureSample(t3, s, uv * 4.0);\n  return a * 0.4 + b * 0.2 + c * 0.2 + d * 0.1 + e * 0.1;\n}",
                "// review the taps: adjacent-scale fetches of the same texture are\n// mipmap-ladder candidates (one textureSampleLevel chain), and t2/t3\n// repeats may collapse into single fetches with swizzled use:\n// @fragment\nfn composite(t1: texture_2d<f32>, t2: texture_2d<f32>, t3: texture_2d<f32>, s: sampler, uv: vec2<f32>) -> vec4<f32> {\n  let a = textureSample(t1, s, uv);\n  let b = textureSample(t2, s, uv);\n  let c = textureSample(t3, s, uv);\n  let d = textureSampleLevel(t2, s, uv * 2.0, 1.0);\n  let e = textureSampleLevel(t3, s, uv * 4.0, 2.0);\n  return a * 0.4 + b * 0.2 + c * 0.2 + d * 0.1 + e * 0.1;\n}",
                ShaderFamily::SampleBudget,
            ),
            rule(
                "vertex-stage-texture-fetch-annotation",
                "A texture fetch inside a @vertex function: vertex texture fetches run through a scalar/limited texture path on many GPUs (no derivative hardware in the vertex stage — textureSample is invalid there, which naga owns) and each fetch stalls the vertex pipeline where the fetch is not latency-hidden by more vertices. Annotation only: displacement/look-ahead fetches are often load-bearing.",
                "@vertex\nfn displace(hmap: texture_2d<f32>, s: sampler, pos: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {\n  let h = textureSampleLevel(hmap, s, uv, 0.0).r;\n  let h2 = textureLoad(hmap, vec2<i32>(0), 0).r;\n  return pos + vec3<f32>(0.0, h + h2, 0.0);\n}",
                "// review the fetch pair: h2 reads a CONSTANT texel (0,0) every\n// invocation — hoist it to a uniform (or bake it) instead of fetching\n// per-vertex; h is the load-bearing displacement fetch:\n// @vertex\nfn displace(hmap: texture_2d<f32>, s: sampler, pos: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {\n  let h = textureSampleLevel(hmap, s, uv, 0.0).r;\n  return pos + vec3<f32>(0.0, h + baked_zero, 0.0);\n}",
                ShaderFamily::StageTraffic,
            ),
            rule(
                "derivative-in-nonuniform-flow",
                "dpdx/dpdy/fwidth inside a loop or branch in a @fragment function: derivatives are computed by differencing neighboring invocations, so their value in non-uniform control flow is quad-dependent garbage that naga's uniformity analysis only partially rejects (the analysis is conservative both ways). The advisory fires pre-render on the shapes validation still lets through. Annotation only — the correct restructuring (hoist the derivative, or restructure the branch) is effect knowledge.",
                "@fragment\nfn edge(n: vec3<f32>, k: f32) -> f32 {\n  var w = 0.0;\n  if (k > 0.5) {\n    w = fwidth(n.x);\n  } else {\n    w = 0.5 * dpdy(n.y);\n  }\n  return w;\n}",
                "// hoist the derivative above the branch — computed unconditionally,\n// selected after (one extra derivative op, uniform quad math):\n// @fragment\nfn edge(n: vec3<f32>, k: f32) -> f32 {\n  let wx = fwidth(n.x);\n  let wy = 0.5 * dpdy(n.y);\n  var w = 0.0;\n  if (k > 0.5) {\n    w = wx;\n  } else {\n    w = wy;\n  }\n  return w;\n}",
                ShaderFamily::DerivativeSafety,
            ),
            // ── Corpus-only (not line-expressible in v1) ─────────────────
            rule(
                "invariant-recompute-to-varying",
                "A value recomputed per-fragment that is invariant across the primitive (depends only on builtin/interpolated inputs, not on fragment-dependent state) burns ALU in every fragment where the vertex stage would compute it 3 times per triangle and interpolate for free. The StageTraffic family lead. NOT LINE-EXPRESSIBLE in v1: the discriminator needs dataflow from the vertex outputs (which inputs are varying vs derived), which the text does not carry — the fixture pair exists for retrieval; the scanner pattern waits for a real buggy file to widen it (the wgsl_scan precedent).",
                "@vertex\nfn vs(uv: vec2<f32>) -> VOut {\n  var o: VOut;\n  o.uv = uv;\n  o.pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);\n  return o;\n}\n// @fragment — recomputes the SAME spherical normal per fragment:\nfn fs(i: VOut) -> @location(0) vec4<f32> {\n  let z = sqrt(1.0 - i.uv.x * i.uv.x - i.uv.y * i.uv.y);\n  let n = vec3<f32>(i.uv.x, i.uv.y, z);\n  return vec4<f32>(n * 0.5 + 0.5, 1.0);\n}",
                "// @vertex — computed 3x per triangle, interpolated for free:\nfn vs(uv: vec2<f32>) -> VOut {\n  var o: VOut;\n  o.uv = uv;\n  let z = sqrt(1.0 - uv.x * uv.x - uv.y * uv.y);\n  o.n = vec3<f32>(uv.x, uv.y, z);\n  o.pos = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);\n  return o;\n}\n// @fragment\nfn fs(i: VOut) -> @location(0) vec4<f32> {\n  return vec4<f32>(i.n * 0.5 + 0.5, 1.0);\n}",
                ShaderFamily::StageTraffic,
            ),
        ],
    }
}
