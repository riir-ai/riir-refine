//! Plan 134 T0.5 — the shader_perf domain's control gates.
//!
//! Four things are pinned here, all in the "wrong in the safe direction"
//! discipline of the docker T5 control (`tests/docker_t5_control.rs`):
//!
//! 1. **Fixture firing** — every scanner pattern fires on its embedded
//!    buggy fixture and is SILENT on the fixed twin (the buggy→fixed pair
//!    is miner-detectable).
//! 2. **The negative control** (the garbage-in guard): ZERO findings over
//!    the sibling riir-shader vgpu port corpus (190 tracked `.wgsl` at
//!    landing; the 28/28 hero-verified ports). Any finding must be
//!    adjudicated REAL or the rule demoted/killed before this gate is
//!    updated — a false positive on a clean port poisons the consumer gate
//!    this domain exists to serve. Sibling absent (fresh clone, CI without
//!    neighbours) ⇒ SKIP with a printed line, never a silent pass.
//! 3. **The refusal table** — every named refusal is present, non-empty,
//!    and the bounded proposer's output honors them (idempotence: a healed
//!    source refuses a second guard).
//! 4. **Determinism + population floors** — same scan, same output; the
//!    negative-control walk must see >= 150 files or it says so loudly
//!    (a walk regression must not print a vacuous zero).
//!
//! Run: `cargo test --features shader_perf --test shader_perf_gates`
//! (`--features shader_perf,shader_verify` adds the naga-tier arms.)

#![cfg(feature = "shader_perf")]

use riir_clippy::domains::shader_perf::proposers::{
    collect_all_edits, FIXABLE_SHADER, DEFAULT_GUARD_BOUND,
};
use riir_clippy::domains::shader_perf::{refuses, ShaderPerfDomain, ShaderScanner};
use riir_clippy::edit_emitter::apply_edits;
use riir_clippy::HealDomain;

fn rule_ids(src: &str) -> Vec<&'static str> {
    ShaderScanner.scan(src).into_iter().map(|f| f.rule_id).collect()
}

// ── Embedded fixtures (always run — the scanner is never fully unpinned) ──

/// The clean control: a plausible, well-formed fragment shader — every
/// pattern must be SILENT on it.
const CLEAN_FRAGMENT: &str = "@fragment\nfn fs_main(\n  t: texture_2d<f32>,\n  s: sampler,\n  uv: vec2<f32>,\n) -> @location(0) vec4<f32> {\n  let base = textureSample(t, s, uv);\n  let detail = textureSampleLevel(t, s, uv * 4.0, 1.0);\n  let hi = select(0.5, 1.0, uv.x > 0.5);\n  return base * hi + detail * 0.25;\n}\n";

/// The unbounded-loop buggy side (the TDR class).
const BUGGY_LOOP: &str = "@fragment\nfn march(uv: vec2<f32>) -> @location(0) vec4<f32> {\n  var p = uv;\n  loop {\n    let d = scene(p);\n    p = p + vec2<f32>(0.01, 0.0);\n  }\n  return vec4<f32>(p, 0.0, 1.0);\n}\nfn scene(p: vec2<f32>) -> f32 { return length(p) - 0.5; }\n";

/// The bounded twin — the guard the proposer emits (manually written here;
/// the proposer test below asserts its own output matches the shape).
const FIXED_LOOP: &str = "@fragment\nfn march(uv: vec2<f32>) -> @location(0) vec4<f32> {\n  var p = uv;\n  var shader_heal_guard: u32 = 0u;\n  loop {\n        shader_heal_guard = shader_heal_guard + 1u;\n        if (shader_heal_guard >= 256u) { break; }\n    let d = scene(p);\n    p = p + vec2<f32>(0.01, 0.0);\n  }\n  return vec4<f32>(p, 0.0, 1.0);\n}\nfn scene(p: vec2<f32>) -> f32 { return length(p) - 0.5; }\n";

/// The loop-carried-sample buggy side.
const BUGGY_LOOP_SAMPLE: &str = "@fragment\nfn glow(t: texture_2d<f32>, s: sampler, uv: vec2<f32>) -> @location(0) vec4<f32> {\n  var acc = vec4<f32>(0.0);\n  var i = 0u;\n  loop {\n    i = i + 1u;\n    if (i >= 64u) { break; }\n    acc = acc + textureSample(t, s, uv + vec2<f32>(f32(i) * 0.003, 0.0));\n  }\n  return acc / 64.0;\n}\n";

/// The branch-divergent buggy side (exactly one arm samples).
const BUGGY_BRANCH: &str = "@fragment\nfn detail(t: texture_2d<f32>, s: sampler, uv: vec2<f32>, k: f32) -> @location(0) vec4<f32> {\n  if (k > 0.5) {\n    return textureSample(t, s, uv * 4.0);\n  } else {\n    return vec4<f32>(k, k, k, 1.0);\n  }\n}\n";

#[test]
fn clean_fragment_fires_nothing() {
    assert_eq!(rule_ids(CLEAN_FRAGMENT), Vec::<&'static str>::new());
}

#[test]
fn unbounded_loop_pair_fires_and_heals() {
    let buggy = rule_ids(BUGGY_LOOP);
    assert!(
        buggy.contains(&"unbounded-render-loop-guard"),
        "must fire on the buggy side: {buggy:?}"
    );
    let fixed = rule_ids(FIXED_LOOP);
    assert!(
        !fixed.contains(&"unbounded-render-loop-guard"),
        "must be silent on the guarded twin: {fixed:?}"
    );
}

#[test]
fn loop_carried_sample_pair() {
    let buggy = rule_ids(BUGGY_LOOP_SAMPLE);
    assert!(
        buggy.contains(&"loop-carried-sample-to-explicit-lod"),
        "{buggy:?}"
    );
    // The loop HAS an exit (break at 64) — not the TDR class.
    assert!(!buggy.contains(&"unbounded-render-loop-guard"));
}

#[test]
fn branch_divergent_pair() {
    let buggy = rule_ids(BUGGY_BRANCH);
    assert!(buggy.contains(&"branch-divergent-sample-select"), "{buggy:?}");
}

#[test]
fn scans_are_deterministic() {
    for src in [CLEAN_FRAGMENT, BUGGY_LOOP, BUGGY_LOOP_SAMPLE, BUGGY_BRANCH] {
        assert_eq!(ShaderScanner.scan(src), ShaderScanner.scan(src));
    }
}

// ── The corpus ─────────────────────────────────────────────────────────────

#[test]
fn corpus_entries_are_unique_and_nonempty() {
    let corpus = ShaderPerfDomain::build_corpus();
    assert!(!corpus.is_empty(), "empty corpus panics the DomainRouter");
    let mut ids: Vec<&str> = corpus.iter().map(|r| r.rule_id.as_str()).collect();
    let n = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), n, "duplicate rule_id in the corpus");
}

/// Every scanner rule fronts a corpus entry (the citation contract — a
/// finding naming a rule the corpus does not carry is an unbooked citation).
#[test]
fn every_pattern_cites_a_corpus_rule() {
    let corpus: Vec<String> = ShaderPerfDomain::build_corpus()
        .into_iter()
        .map(|r| r.rule_id)
        .collect();
    for src in [BUGGY_LOOP, BUGGY_LOOP_SAMPLE, BUGGY_BRANCH] {
        for f in ShaderScanner.scan(src) {
            assert!(
                corpus.iter().any(|id| id == f.rule_id),
                "finding cites unknown rule {}",
                f.rule_id
            );
        }
    }
}

// ── The refusal table + the proposer's discipline ──────────────────────────

#[test]
fn refusals_are_named_and_reasoned() {
    let r = refuses();
    assert!(r.contains(&"one-sided-branch-sample-rewrite"));
    assert!(r.contains(&"guessed-lod-substitution"));
    assert!(r.contains(&"wrapping-guard-counter"));
    // And the decision rule is NOT in the fixable set — suggest-only
    // forever (the DL3059 discipline).
    assert!(!FIXABLE_SHADER.contains(&"branch-divergent-sample-select"));
    assert!(!FIXABLE_SHADER.contains(&"loop-carried-sample-to-explicit-lod"));
}

#[test]
fn proposer_is_idempotent_and_tier0_clean() {
    let edits = collect_all_edits(BUGGY_LOOP);
    assert!(!edits.is_empty());
    let healed = apply_edits(BUGGY_LOOP, edits.iter().map(|re| re.edit.clone()).collect());
    // Idempotence (the guard-name refusal).
    assert!(collect_all_edits(&healed).is_empty());
    // The scanner agrees the loop is now bounded.
    assert!(!rule_ids(&healed).contains(&"unbounded-render-loop-guard"));
    // Tier-0 invariants via the domain's own validator.
    let v = ShaderPerfDomain::validate("unbounded-render-loop-guard", &healed);
    assert!(v.parses && v.entry_points_intact && v.bindings_unchanged);
    // The guard breaks BEFORE the wrap (the wrapping-guard-counter
    // refusal, by construction: bound 256 << 2^32).
    assert!(healed.contains(&format!(">= {DEFAULT_GUARD_BOUND}u")));
}

// ── The negative control (the garbage-in guard) ────────────────────────────

/// The sibling port corpus root. The 267 files under `.raw/vgpu` are the
/// UNTRACKED upstream reference — excluded (the control pins what ships).
const PORT_ROOT: &str = "../riir-shader/crates";

/// ZERO findings over the shipped port corpus. A finding here is either a
/// real anti-pattern someone should adjudicate (update this test with the
/// adjudication note) or a FALSE POSITIVE that demotes/kills the rule —
/// never a silent pass.
#[test]
fn negative_control_zero_findings_on_vgpu_port_corpus() {
    let mut files = Vec::new();
    walk_wgsl(std::path::Path::new(PORT_ROOT), &mut files);
    if files.is_empty() {
        println!(
            "SKIP negative control: sibling corpus absent at {PORT_ROOT} \
             (degrading, not passing silently)"
        );
        return;
    }
    assert!(
        files.len() >= 150,
        "walk regression? only {} .wgsl files under {PORT_ROOT} (expect ~190)",
        files.len()
    );
    let scanner = ShaderScanner;
    let mut offenses: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        // Render-stage files only — @compute files are the kernel lane's.
        if !riir_clippy::domains::shader_perf::entry_stages(&src).has_render() {
            continue;
        }
        scanned += 1;
        for f in scanner.scan(&src) {
            offenses.push(format!("{}:{} [{}] {}", path.display(), f.line, f.rule_id, f.excerpt));
        }
    }
    assert!(
        scanned >= 100,
        "render-stage population collapsed: {scanned} of {} files",
        files.len()
    );
    assert!(
        offenses.is_empty(),
        "negative control FAILED — adjudicate or demote:\n{}",
        offenses.join("\n")
    );
}

fn walk_wgsl(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_wgsl(&p, out);
        } else if p.extension().is_some_and(|x| x == "wgsl") {
            out.push(p);
        }
    }
}

// ── G2 latency (release-gated, the wgsl_scan bar) ──────────────────────────

#[cfg(not(debug_assertions))]
#[test]
fn g2_scan_latency_under_1ms_per_file() {
    use std::time::Instant;
    let scanner = ShaderScanner;
    // Warm the caches once.
    let _ = scanner.scan(BUGGY_LOOP_SAMPLE);
    let mut worst = std::time::Duration::ZERO;
    for _ in 0..64 {
        let t = Instant::now();
        let _ = scanner.scan(BUGGY_LOOP_SAMPLE);
        worst = worst.max(t.elapsed());
    }
    assert!(
        worst.as_nanos() < 1_000_000,
        "scan latency {worst:?} exceeds the 1 ms/file bar (fixture ~20 lines)"
    );
}
