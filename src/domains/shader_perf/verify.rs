//! The `shader_perf` oracle ladder (Plan 134 T1.5).
//!
//! | Tier | Check | Cost | When |
//! |---|---|---|---|
//! | 0 | [`tier0_verdict`] — text round-trip invariants: balanced braces, entry-point multiset intact, `@group/@binding` multiset byte-identical | free | always |
//! | 1 | `naga` validation (feature-gated `dep:naga`, pure Rust, in-process) — the module parses + validates post-fix | ~10 ms class | `shader_verify` builds, both fix classes |
//! | 2 | capture-fidelity protocol — lives in the CONSUMER repo (riir-shader's harness / the shader-toy editor gate), never a riir-clippy dep | seconds | G5-real, the mint gate |
//!
//! Tier 1 off → degrade to Tier 0 with ONE log line (the Issue 018
//! oracle-split): "we could not check" and "the check failed" are different
//! facts, and neither is ever coerced to a pass.
//!
//! ## The `naga`-vs-Tint divergence, pinned (riir-shader Plan 001)
//!
//! `ptr<workgroup>` function params are Tint-legal, `naga`-rejected — the
//! fft-ocean gotcha. [`validate_naga`] records divergences of that class as
//! `Some(false)` with the error text; the CALLER (the consumer protocol)
//! treats the browser (Tint) smoke as final truth for shapes `naga` rejects
//! but the browser accepts — the exact oracle-split discipline of
//! hadolint-vs-BuildKit in docker.

use super::scanner::{entry_point_multiset, group_binding_multiset};

/// Tier 0: the always-available structural verdict (no deps). A bounded
/// fix must (a) keep braces balanced, (b) not add/remove entry points,
/// (c) not move the pipeline layout. `naga_agrees` is `None` here — Tier 1
/// is a separate question (see [`validate_naga`]).
#[must_use]
pub fn tier0_verdict(buggy: &str, fixed: &str) -> super::ShaderVerdict {
    let parses = braces_balanced(fixed) && !entry_point_multiset(fixed).is_empty();
    super::ShaderVerdict {
        parses,
        entry_points_intact: entry_point_multiset(buggy) == entry_point_multiset(fixed),
        bindings_unchanged: group_binding_multiset(buggy) == group_binding_multiset(fixed),
        naga_agrees: None,
    }
}

/// Tier 1: `naga` validation of the fixed source. `Ok(Some(true))` = parses
/// + validates; `Ok(Some(false))` = `naga` rejected; `Ok(None)` = reserved.
///
/// Compiled only under `shader_verify` — a plain `shader_perf` build
/// stands on Tier 0 with ONE degradation line (the Issue 018 oracle-split:
/// "could not check" and "the check failed" are different facts).
///
/// # Errors
///
/// Never returned today: a `naga` parse failure AND a validation failure
/// both map to `Ok(Some(false))` ("naga disagrees" is a verdict, not an
/// error). The `Result` shape reserves the oracle-contract error lane.
#[cfg(feature = "shader_verify")]
pub fn validate_naga(source: &str) -> Result<Option<bool>, String> {
    let module = match naga::front::wgsl::parse_str(source) {
        Ok(m) => m,
        Err(_e) => return Ok(Some(false)), // parse failure: naga disagrees
    };
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module);
    match info {
        Ok(_) => Ok(Some(true)),
        Err(_e) => Ok(Some(false)), // validation failure: naga disagrees
    }
}

/// Brace balance over the raw source (strings masked).
fn braces_balanced(source: &str) -> bool {
    let mut depth: i64 = 0;
    for raw in source.lines() {
        let masked = super::mask_string_literals(raw);
        for ch in masked.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }
    }
    depth == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLEAN: &str = "@fragment\nfn f() -> @location(0) vec4<f32> {\n  return vec4<f32>(0.0);\n}\n";

    #[test]
    fn tier0_passes_a_structural_identity() {
        let v = tier0_verdict(CLEAN, CLEAN);
        assert!(v.parses && v.entry_points_intact && v.bindings_unchanged);
        assert_eq!(v.naga_agrees, None);
    }

    #[test]
    fn tier0_catches_a_removed_entry_point() {
        let broken = "fn f() -> vec4<f32> {\n  return vec4<f32>(0.0);\n}\n";
        let v = tier0_verdict(CLEAN, broken);
        assert!(!v.entry_points_intact);
    }

    #[test]
    fn tier0_catches_a_moved_binding() {
        let a = "@group(0) @binding(0) var<uniform> u: U;\n@fragment\nfn f() -> @location(0) vec4<f32> {\n  return vec4<f32>(0.0);\n}\n";
        let b = "@group(0) @binding(1) var<uniform> u: U;\n@fragment\nfn f() -> @location(0) vec4<f32> {\n  return vec4<f32>(0.0);\n}\n";
        let v = tier0_verdict(a, b);
        assert!(!v.bindings_unchanged);
    }

    #[cfg(feature = "shader_verify")]
    #[test]
    fn naga_validates_a_clean_module_and_rejects_a_broken_one() {
        assert_eq!(validate_naga(CLEAN).ok(), Some(Some(true)));
        let broken = "@fragment\nfn f( -> @location(0) vec4<f32> {\n  return vec4<f32>(0.0);\n}\n";
        assert_eq!(validate_naga(broken).ok(), Some(Some(false)));
    }
}
