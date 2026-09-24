//! Issue 134 T3 re-arm — `shader_perf` domain eval fixtures (hand-authored
//! variants of the 8 corpus anti-patterns; the `eval_fixtures_perf`
//! convention — variant function/identifier names, canonical WGSL shapes,
//! one span per rule).
//!
//! Pure data; consumed by `super::retrieval_eval` under the
//! `latent_retrieval` feature.

// Uniform `r#"..."#` raw-string style is deliberate for fixture data.
#![allow(clippy::needless_raw_string_hashes)]

/// (`rule_id`, `span_code`) — one span per rule; each genuinely instantiates
/// the rule's anti-pattern (semantic label, authored against the corpus entry).
pub const SHADER_SPANS: &[(&str, &str)] = &[
    (
        "unbounded-render-loop-guard",
        r#"@fragment
fn trace_rays(coord: vec2<f32>) -> vec4<f32> {
  loop {
    let hit = probe_scene(coord);
    if (hit.w < 0.0005) { }
    coord = coord + vec2<f32>(0.0, 0.02);
  }
  return vec4<f32>(coord, 0.0, 1.0);
}
"#,
    ),
    (
        "loop-carried-sample-to-explicit-lod",
        r#"@fragment
fn bloom(src: texture_2d<f32>, samp: sampler, base: vec2<f32>) -> vec4<f32> {
  var sum = vec4<f32>(0.0);
  var n = 0u;
  loop {
    n = n + 1u;
    if (n >= 48u) { break; }
    let drift = f32(n) * 0.004;
    sum = sum + textureSample(src, samp, base + vec2<f32>(drift, 0.0));
  }
  return sum / 48.0;
}
"#,
    ),
    (
        "branch-divergent-sample-select",
        r#"@fragment
fn sharpen(map: texture_2d<f32>, samp: sampler, base: vec2<f32>, strength: f32) -> vec4<f32> {
  if (strength > 0.25) {
    return textureSample(map, samp, base * 2.0);
  } else {
    return vec4<f32>(strength, strength, strength, 1.0);
  }
}
"#,
    ),
    (
        "const-fold-loop-bound",
        r#"var<uniform> cfg : Cfg;
@fragment
fn blur_taps(coord: vec2<f32>) -> vec4<f32> {
  var total = vec4<f32>(0.0);
  for (var i = 0; i < cfg.taps; i++) {
    total = total + gather(coord, i);
  }
  return total;
}
"#,
    ),
    (
        "fragment-sample-budget-annotation",
        r#"@fragment
fn blend_layers(l0: texture_2d<f32>, l1: texture_2d<f32>, l2: texture_2d<f32>, samp: sampler, coord: vec2<f32>) -> vec4<f32> {
  let one = textureSample(l0, samp, coord);
  let two = textureSample(l1, samp, coord);
  let three = textureSample(l2, samp, coord);
  let four = textureSample(l1, samp, coord * 2.0);
  let five = textureSample(l2, samp, coord * 4.0);
  let six = textureSample(l0, samp, coord * 8.0);
  let seven = textureSample(l1, samp, coord * 16.0);
  let eight = textureSample(l2, samp, coord * 32.0);
  let nine = textureSample(l0, samp, coord * 64.0);
  let ten = textureSample(l1, samp, coord * 128.0);
  return one * 0.3 + two * 0.2 + three * 0.1 + four * 0.1 + five * 0.08
       + six * 0.07 + seven * 0.05 + eight * 0.04 + nine * 0.03 + ten * 0.02;
}
"#,
    ),
    (
        "vertex-stage-texture-fetch-annotation",
        r#"@vertex
fn elevate(terrain: texture_2d<f32>, samp: sampler, base: vec3<f32>, coord: vec2<f32>) -> vec3<f32> {
  let h = textureSampleLevel(terrain, samp, coord, 0.0).r;
  let corner = textureLoad(terrain, vec2<i32>(0), 0).r;
  return base + vec3<f32>(0.0, h + corner, 0.0);
}
"#,
    ),
    (
        "derivative-in-nonuniform-flow",
        r#"@fragment
fn rim(normal: vec3<f32>, mask: f32) -> f32 {
  var width = 0.0;
  if (mask > 0.2) {
    width = fwidth(normal.x);
  } else {
    width = 0.5 * dpdy(normal.y);
  }
  return width;
}
"#,
    ),
    (
        "invariant-recompute-to-varying",
        r#"@vertex
fn vs_main(coord: vec2<f32>) -> FOut {
  var o: FOut;
  o.uv = coord;
  o.pos = vec4<f32>(coord * 2.0 - 1.0, 0.0, 1.0);
  return o;
}
// @fragment — recomputes the SAME hemispheric normal per fragment:
fn fs_main(v: FOut) -> @location(0) vec4<f32> {
  let nz = sqrt(1.0 - v.uv.x * v.uv.x - v.uv.y * v.uv.y);
  let n = vec3<f32>(v.uv.x, v.uv.y, nz);
  return vec4<f32>(n * 0.5 + 0.5, 1.0);
}
"#,
    ),
];
