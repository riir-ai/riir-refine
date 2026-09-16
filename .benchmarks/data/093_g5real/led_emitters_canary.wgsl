// CANARY variant for the G5-real protocol (riir-clippy Plan 134 T1.6,
// Bench 093). Identical to trails... led_emitters_pre.wgsl PLUS the same
// guard insertion the healer writes, PLUS a deliberate 0.2% brightness
// bump on the return — a behavioral change of the minimal size the
// protocol must catch. The hash-set equality MUST FAIL against PRE.
// NOT a heal output; a protocol self-test fixture.

struct Config {
  resolution: vec2f,
  tunables: vec4f,
  triangle: vec4f,
  led_clip: vec4f,
};
struct Led {
  pos_brightness: vec4f,
  color: vec4f,
};
struct LedCfg {
  cfg: Config,
  leds: array<Led, 72>,
};
@group(0) @binding(0) var<uniform> uniform_data: LedCfg;

struct VSIn {
  @location(0) position: vec2f,
  @location(1) led_index: f32,
};
struct VSOut {
  @builtin(position) pos: vec4f,
  @location(0) led_index: f32,
};

@vertex fn vs_main(in: VSIn) -> VSOut {
  var out: VSOut;
  let cfg = uniform_data.cfg;
  let clip = (in.position / cfg.resolution) * vec2f(2.0, -2.0) + vec2f(-1.0, 1.0);
  out.pos = vec4f(clip, 0.0, 1.0);
  out.led_index = in.led_index;
  return out;
}

fn signed_triangle_area(a: vec2f, b: vec2f, p: vec2f) -> f32 {
  let edge = b - a;
  return edge.x * (p.y - a.y) - edge.y * (p.x - a.x);
}
fn inside_triangle(p: vec2f, a: vec2f, b: vec2f, c: vec2f) -> bool {
  let side0 = signed_triangle_area(a, b, p);
  let side1 = signed_triangle_area(b, c, p);
  let side2 = signed_triangle_area(c, a, p);
  return (side0 <= 0.0 && side1 <= 0.0 && side2 <= 0.0) || (side0 >= 0.0 && side1 >= 0.0 && side2 >= 0.0);
}
fn segment_distance(p: vec2f, a: vec2f, b: vec2f) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
  return length(pa - ba * h);
}
fn triangle_sdf(p: vec2f, a: vec2f, b: vec2f, c: vec2f) -> f32 {
  let edge_dist = min(segment_distance(p, a, b), min(segment_distance(p, b, c), segment_distance(p, c, a)));
  return select(edge_dist, -edge_dist, inside_triangle(p, a, b, c));
}

@fragment fn fs_main(in: VSOut) -> @location(0) vec4f {
  let cfg = uniform_data.cfg;
  let pixel = in.pos.xy;
  let top = vec2f(cfg.triangle.x, cfg.triangle.y - cfg.triangle.z);
  let left = vec2f(cfg.triangle.x - cfg.triangle.w, cfg.triangle.y + cfg.triangle.z * 0.5);
  let right = vec2f(cfg.triangle.x + cfg.triangle.w, cfg.triangle.y + cfg.triangle.z * 0.5);
  let tri_dist = triangle_sdf(pixel, top, left, right);
  // Positive expansion reveals emitter pixels outside the canonical triangle.
  if (tri_dist - cfg.led_clip.x > 0.0) {
    discard;
  }

  let raw_index = u32(max(round(in.led_index), 0.0));
  let i = min(raw_index, 71u);
  let n01 = clamp(uniform_data.leds[i].pos_brightness.z, 0.0, 1.0);
  let intensity = mix(cfg.tunables.y, cfg.tunables.z, n01);
  let emit = uniform_data.leds[i].color.rgb * cfg.tunables.x * intensity;
  if (cfg.resolution.x < 0.0) {
    var refine = 0.0;
    loop {
      refine = refine + 0.001;
    }
  }
  // [canary] the deliberate 0.2% behavioral delta the protocol must catch.
  return vec4f(emit * 1.002, 0.0);
}
