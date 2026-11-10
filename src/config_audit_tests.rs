use super::*;

fn one(src: &str) -> ConfigAuditReport {
    audit_sources(&[("lib.rs".to_string(), src.to_string())])
}

#[test]
fn inert_knob_fires() {
    let r = one(r"
pub const AUDIT_RING_K: usize = 64;
pub struct AuditConfig { pub ring_capacity: usize, pub live: usize }
impl Default for AuditConfig {
    fn default() -> Self { Self { ring_capacity: AUDIT_RING_K, live: AUDIT_RING_K } }
}
pub fn bound(_c: &AuditConfig) -> usize { AUDIT_RING_K }
pub fn live(c: &AuditConfig) -> usize { c.live }
");
    assert_eq!(r.inert_knobs.len(), 1, "ring_capacity fires");
    assert_eq!(r.inert_knobs[0].candidate.field_name, "ring_capacity");
    assert!(r.inert_knobs[0].const_reads >= 1);
    assert!(r.assert_only.is_empty());
}

#[test]
fn gpu_uniform_const_inited_field_downgrades() {
    // The seal-game-editor WaterParams shape (P28): the params struct is
    // the type of a #[uniform(0)]-bound field, its fields are const-inited
    // in Default and never read in Rust — the WGSL consumes the block.
    let r = one(r"
pub const RING: usize = 64;
#[derive(Clone, Copy)]
pub struct Params { pub ring: usize, pub live: usize }
impl Default for Params { fn default() -> Self { Self { ring: RING, live: RING } } }
pub struct Material { #[uniform(0)] pub params: Params }
pub fn bind(_m: &Material) -> usize { RING }
");
    assert!(
        r.inert_knobs.is_empty(),
        "gpu-bound struct downgrades out of [A]"
    );
    assert_eq!(r.gpu_uniform.len(), 2, "both const-inited fields annotate");
    assert!(r.render().contains("[gpu-uniform]"));
}

#[test]
fn gpu_uniform_downgrade_is_transitive_through_nested_shader_structs() {
    // A ShaderType struct nested in the uniform block is flattened into
    // it — the closure descends through field types.
    let r = one(r"
pub const INNER_K: usize = 8;
#[derive(Clone, Copy)]
pub struct Inner { pub k: usize }
impl Default for Inner { fn default() -> Self { Self { k: INNER_K } } }
#[derive(Clone, Copy)]
pub struct Params { pub inner: Inner }
impl Default for Params { fn default() -> Self { Self { inner: Inner::default() } } }
pub struct Material { #[uniform(0)] pub params: Params }
pub fn ext() -> usize { INNER_K }
");
    assert!(
        r.inert_knobs.is_empty(),
        "Inner is in the block transitively"
    );
    assert_eq!(r.gpu_uniform.len(), 1);
    assert_eq!(r.gpu_uniform[0].candidate.struct_name, "Inner");
}

#[test]
fn non_uniform_const_inited_field_still_fires() {
    // Precision guard: without the #[uniform(..)] bind the row stays [A].
    let r = one(r"
pub const RING: usize = 64;
#[derive(Clone, Copy)]
pub struct Params { pub ring: usize }
impl Default for Params { fn default() -> Self { Self { ring: RING } } }
pub struct Material { pub params: Params }
pub fn bind(_m: &Material) -> usize { RING }
");
    assert_eq!(r.inert_knobs.len(), 1);
    assert!(r.gpu_uniform.is_empty());
}

#[test]
fn gpu_uniform_c2_downgrades_to_weak_annotation() {
    let r = one(r"
#[derive(Clone, Copy)]
pub struct Params { pub a: u32, pub b: u32 }
impl Default for Params { fn default() -> Self { Self { a: 1, b: 2 } } }
pub struct Material { #[uniform(0)] pub params: Params }
");
    assert!(r.inert_knobs.is_empty());
    assert_eq!(r.self_referential.len(), 1);
    assert!(
        r.self_referential[0].gpu_uniform,
        "C2 downgrades, not suppresses"
    );
    assert!(r.render().contains("[C2·gpu-uniform]"));
}

/// The `[C2·test-rig]` downgrade (2026-09-16, the riir-ai
/// `Outcome`/`ProbeConfig` intake): a struct DECLARED at top level in
/// a tests-path file whose every read is under tests-path files is
/// definitionally confined — the C2 class targets PRODUCTION config
/// structs whose consumer never materialized. Downgrade-not-suppress:
/// the row stays visible at the weak annotation.
#[test]
fn tests_tree_struct_c2_downgrades_to_test_rig() {
    let r = audit_sources(&[(
        "tests/rig.rs".to_string(),
        r"
pub struct Rig { pub a: f32, pub b: f32 }
impl Default for Rig { fn default() -> Self { Self { a: 1.0, b: 2.0 } } }
fn probe(r: &Rig) -> f32 { if r.a > r.b { r.a } else { r.b } }
"
        .to_string(),
    )]);
    assert_eq!(r.self_referential.len(), 1);
    assert!(
        r.self_referential[0].test_rig,
        "tests-tree decl + all-tree reads downgrade, not suppress"
    );
    let rendered = r.render();
    assert!(rendered.contains("[C2·test-rig]"));
    assert!(!rendered.contains("[C2] tests/rig.rs"));
}

/// The distinguishing twin: a src/-declared struct whose reads are
/// cfg(test)-confined stays at FULL [C2] — the path axis (tests tree)
/// is separate from cfg(test) confinement, and the production-struct
/// class is what C2 exists for.
#[test]
fn src_cfg_test_struct_stays_full_c2() {
    let r = audit_sources(&[(
        "src/lib.rs".to_string(),
        r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 1.0 } } }
#[cfg(test)]
mod tests {
    fn probe(p: &super::P) -> f32 { p.a }
}
"
        .to_string(),
    )]);
    assert_eq!(r.self_referential.len(), 1);
    assert!(!r.self_referential[0].test_rig);
    let rendered = r.render();
    assert!(rendered.contains("[C2]"));
    assert!(!rendered.contains("[C2·test-rig]"));
}

/// The conservative guard: a tests-tree DECL alone does not downgrade
/// when the name-global read tally carries non-tree reads — a
/// same-named field on a src struct read inside cfg(test) is
/// indistinguishable from the rig's own reads, so eligibility
/// requires every read under tests-path files.
#[test]
fn tests_tree_decl_with_src_name_sharing_stays_full_c2() {
    let r = audit_sources(&[
        (
            "tests/rig.rs".to_string(),
            r"
pub struct Rig { pub done: bool }
impl Default for Rig { fn default() -> Self { Self { done: false } } }
fn probe(r: &Rig) -> bool { r.done }
"
            .to_string(),
        ),
        (
            "src/lib.rs".to_string(),
            r"
pub struct Other { pub done: bool }
#[cfg(test)]
mod t {
    fn probe(o: &super::Other) -> bool { o.done }
}
"
            .to_string(),
        ),
    ]);
    assert_eq!(r.self_referential.len(), 1);
    assert!(!r.self_referential[0].test_rig);
    assert!(!r.render().contains("[C2·test-rig]"));
}

#[test]
fn live_via_field_silent() {
    let r = one(r"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
pub fn use_it(c: &C) -> usize { c.f }
");
    assert!(r.inert_knobs.is_empty());
    assert!(r.assert_only.is_empty());
}

#[test]
fn live_via_destructuring_silent_the_blind_spot_guard() {
    let r = one(r"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
pub fn destructure(c: &C) -> usize { let C { f, .. } = c; f }
pub fn also_const() -> usize { K }
");
    assert!(r.inert_knobs.is_empty(), "destructuring counts as a read");
}

#[test]
fn live_via_macro_silent() {
    let r = one(r#"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
pub fn show(c: &C) { println!("{} {}", c.f, K); }
"#);
    assert!(
        r.inert_knobs.is_empty(),
        "field read inside non-assert macro counts as a read"
    );
}

#[test]
fn array_length_use_fires_with_note() {
    let r = one(r"
pub const RING_K: usize = 8;
pub struct C { pub ring: usize }
impl Default for C { fn default() -> Self { Self { ring: RING_K } } }
pub struct Entry { pub v: u32 }
pub fn slot(_c: &C) -> [Entry; RING_K] { [Entry { v: 0 }; RING_K] }
");
    assert_eq!(r.inert_knobs.len(), 1);
    assert!(
        r.inert_knobs[0].array_len_uses >= 1,
        "array-length use detected"
    );
    assert!(r.render().contains("structurally un-honorable"));
}

#[test]
fn assert_only_fires_class_b() {
    let r = one(r"
pub struct C { pub sc: bool }
impl Default for C { fn default() -> Self { Self { sc: true } } }
pub fn consume(_c: &C) -> u32 { 1 }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn fake_coverage() { let c = C::default(); assert!(c.sc); }
}
");
    assert!(r.inert_knobs.is_empty());
    assert_eq!(r.assert_only.len(), 1, "assert-only field fires class B");
    assert_eq!(r.assert_only[0].candidate.field_name, "sc");
    assert!(r.render().contains("[B]"));
}

/// The Issue-030-class intake downgrade: a class-B row whose FIELD doc
/// carries a deliberate-write-only marker reports at the weak
/// `[documented-write-only]` annotation instead (downgrade-not-suppress,
/// the `[wire-const]` precedent — still visible, never counted as [B]).
#[test]
fn documented_write_only_marker_downgrades_class_b() {
    let r = one(r"
pub struct GuardVerdict {
    /// Deliberate, not dead payload — the layout pin counts this field.
    pub redirect: Option<u8>,
}
impl Default for GuardVerdict {
    fn default() -> Self { Self { redirect: None } }
}
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn pin() { let v = GuardVerdict::default(); assert_eq!(v.redirect, None); }
}
");
    assert!(r.assert_only.is_empty(), "marked row downgrades out of [B]");
    assert_eq!(r.documented_write_only.len(), 1);
    let rendered = r.render();
    assert!(rendered.contains("[documented-write-only]"));
    assert!(
        rendered.contains("0 assert-only field(s) [B]"),
        "header [B] count is zero"
    );
}

/// The Issue-147 field-doc wording ("GOAT-gate territory") — the
/// vocabulary the canonical specimen's field doc carries TODAY.
#[test]
fn goat_gate_territory_wording_downgrades() {
    let r = one(r"
pub struct V {
    /// Phase-6 GOAT-gate territory — the channel is live and pinned.
    pub redirect: Option<u8>,
}
impl Default for V { fn default() -> Self { Self { redirect: None } } }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn pin() { let v = V::default(); assert_eq!(v.redirect, None); }
}
");
    assert!(r.assert_only.is_empty());
    assert_eq!(
        r.documented_write_only.len(),
        1,
        "the Issue-147 wording downgrades"
    );
}

/// The P29 class-A extension (the `HlaKarcConfig.delay_k` shape): a
/// const-defaulted knob whose FIELD doc carries a deliberate-write-only
/// marker reports at the weak `[A·documented-write-only]` annotation
/// instead of deletion-grade [A].
#[test]
fn documented_write_only_marker_downgrades_class_a() {
    let r = one(r"
pub const SHAPE_K: usize = 4;
pub struct ForecasterConfig {
    /// Deliberate write-only forward surface — a future dispatcher picks
    /// among pre-monomorphized shapes; the default forecaster never reads it.
    pub delay_k: usize,
}
impl Default for ForecasterConfig { fn default() -> Self { Self { delay_k: SHAPE_K } } }
pub fn shape(_c: &ForecasterConfig) -> usize { SHAPE_K }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn pin() { let c = ForecasterConfig::default(); assert_eq!(c.delay_k, SHAPE_K); }
}
");
    assert!(r.inert_knobs.is_empty(), "marked row downgrades out of [A]");
    assert_eq!(r.documented_write_only_knobs.len(), 1);
    assert_eq!(
        r.documented_write_only_knobs[0].candidate.field_name,
        "delay_k"
    );
    let rendered = r.render();
    assert!(rendered.contains("[A·documented-write-only]"));
    assert!(
        rendered.contains("0 inert knob(s) [A]"),
        "header [A] count is zero"
    );
}

/// Precedence: a gpu-bound struct's const-inited field with a marker doc
/// stays in the P28 gpu-uniform bucket — the shader-consumption mechanism
/// is the more specific annotation.
#[test]
fn gpu_uniform_takes_precedence_over_write_only_marker() {
    let r = one(r"
pub const RING: usize = 64;
#[derive(Clone, Copy)]
pub struct Params {
    /// Deliberate write-only forward surface.
    pub ring: usize,
}
impl Default for Params { fn default() -> Self { Self { ring: RING } } }
pub struct Material { #[uniform(0)] pub params: Params }
pub fn bind(_m: &Material) -> usize { RING }
");
    assert!(r.inert_knobs.is_empty());
    assert!(r.documented_write_only_knobs.is_empty(), "gpu-uniform wins");
    assert_eq!(r.gpu_uniform.len(), 1);
}

#[test]
fn canonical_token_downgrades() {
    let r = one(r"
pub struct V {
    /// documented-write-only: pinned wire surface.
    pub f: bool,
}
impl Default for V { fn default() -> Self { Self { f: true } } }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn pin() { let v = V::default(); assert!(v.f); }
}
");
    assert!(r.assert_only.is_empty());
    assert_eq!(r.documented_write_only.len(), 1);
}

/// The marker counts only on the FIELD doc — a struct-level doc speaks
/// for the whole type, not for one write-only member.
#[test]
fn struct_doc_marker_does_not_downgrade_field() {
    let r = one(r"
/// Deliberate forward surface struct.
pub struct V { pub f: bool }
impl Default for V { fn default() -> Self { Self { f: true } } }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn pin() { let v = V::default(); assert!(v.f); }
}
");
    assert_eq!(
        r.assert_only.len(),
        1,
        "struct doc does not speak for one field"
    );
    assert!(r.documented_write_only.is_empty());
}

/// A doc WITHOUT any marker vocabulary is just documentation — the row
/// stays a class-B finding.
#[test]
fn plain_doc_stays_class_b() {
    let r = one(r"
pub struct V {
    /// The redirect authority index.
    pub f: bool,
}
impl Default for V { fn default() -> Self { Self { f: true } } }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn pin() { let v = V::default(); assert!(v.f); }
}
");
    assert_eq!(r.assert_only.len(), 1, "unmarked doc stays [B]");
    assert!(r.documented_write_only.is_empty());
}

#[test]
fn class_a_precedence_over_b() {
    let r = one(r"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn a() { let c = C::default(); assert_eq!(c.f, K); }
}
");
    assert_eq!(
        r.inert_knobs.len(),
        1,
        "A wins over B (K is read inside the assert macro tokens)"
    );
    assert!(r.assert_only.is_empty());
}

#[test]
fn non_const_default_silent() {
    let r = one(r"
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: 7 } } }
pub fn x() -> usize { 7 }
");
    assert!(r.inert_knobs.is_empty());
    assert!(r.assert_only.is_empty(), "no asserts, no const path");
}

#[test]
fn default_own_const_read_does_not_satisfy_condition_3() {
    let r = one(r"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
pub fn consume(_c: &C) -> u32 { 1 }
");
    assert!(
        r.inert_knobs.is_empty(),
        "the default's own RHS is not an external consumer"
    );
}

#[test]
fn cross_file_index() {
    let r = audit_sources(&[
        (
            "config.rs".to_string(),
            r"
pub const RING_K: usize = 8;
pub struct C { pub ring: usize }
impl Default for C { fn default() -> Self { Self { ring: RING_K } } }
"
            .to_string(),
        ),
        (
            "consumer.rs".to_string(),
            r"pub fn bound() -> usize { crate::config::RING_K }".to_string(),
        ),
    ]);
    assert_eq!(
        r.inert_knobs.len(),
        1,
        "const read in another file satisfies condition 3"
    );
    assert_eq!(r.files_scanned, 2);
}

#[test]
fn qualified_default_no_cross_type_collision() {
    let r = one(r"
pub struct A { pub n: usize }
pub struct B { pub n: usize }
impl Default for A { fn default() -> Self { Self { n: B::COUNT } } }
impl Default for B { fn default() -> Self { Self { n: 1 } } }
impl B { pub const COUNT: usize = 2; }
pub fn other() -> usize { A::COUNT }
");
    // A::COUNT is read but is NOT B::COUNT — the qualified default must
    // not fire off a different type's const.
    assert!(r.inert_knobs.is_empty(), "no cross-type ::COUNT collision");
}

#[test]
fn struct_update_silent() {
    let r = one(r"
pub const K: usize = 4;
pub struct C { pub f: usize, pub g: usize }
impl Default for C { fn default() -> Self { Self { f: K, g: 1 } } }
pub fn mk(base: C) -> C { C { g: 2, ..base } }
");
    // K is read only by the Default itself (suppressed) and the
    // functional-update `..base` is not a read of `f` — but condition 3
    // needs an EXTERNAL const read, which does not exist here.
    assert!(r.inert_knobs.is_empty());
}

#[test]
fn behavioral_assert_counts_as_read_not_class_b() {
    // The riir-chain shape that forced the precision clause: asserts
    // against COMPUTED expectations are behavioral coverage, not
    // default-equality fake coverage.
    let r = one(r"
pub struct V { pub backoff_ticks: u32, pub other: u32 }
impl Default for V { fn default() -> Self { Self { backoff_ticks: 0, other: 0 } } }
pub fn severe() -> u32 { 9 }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn decay() {
        let v = V::default();
        assert_eq!(v.backoff_ticks as u64, severe() as u64);
        assert_eq!(v.backoff_ticks, 0);
        assert_eq!(v.other, None_other());
    }
}
fn None_other() -> u32 { 3 }
");
    assert!(
        r.assert_only.is_empty(),
        "behavioral asserts (computed expectations) are reads"
    );
    assert!(r.inert_knobs.is_empty());
}

#[test]
fn none_comparator_matches_none_default() {
    // assert_eq!(v.redirect, None) with a `None` default IS default-
    // equality — the riir-chain redirect row (accurate B).
    let r = one(r"
pub struct V { pub redirect: Option<u8> }
impl Default for V { fn default() -> Self { Self { redirect: None } } }
pub fn make() -> V { V::default() }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn a() { let v = make(); assert_eq!(v.redirect, None); }
}
");
    assert_eq!(
        r.assert_only.len(),
        1,
        "None-comparator against None default is class B"
    );
    assert_eq!(r.assert_only[0].candidate.field_name, "redirect");
}

#[test]
fn negated_bool_shape_tautological_fires() {
    let r = one(r"
pub struct C { pub off: bool }
impl Default for C { fn default() -> Self { Self { off: false } } }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn a() { let c = C::default(); assert!(!c.off); }
}
");
    assert_eq!(
        r.assert_only.len(),
        1,
        "assert!(!c.off) with default false is tautological"
    );
}

// ── Class C2 — the self-referential config struct (Plan 125) ──

#[test]
fn self_referential_validate_plus_tests_fires_c2() {
    // The PlasmaConfig shape: own validate() reads + test reads, nothing
    // external. Classes A/B are blind here (reads > 0).
    let r = one(r"
pub struct P { pub a: f32, pub b: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5, b: 0.25 } } }
impl P {
    pub fn validate(&self) -> bool { self.a > self.b }
}
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn smoke() { let p = P::default(); assert!(p.a > p.b); }
}
");
    assert_eq!(r.self_referential.len(), 1, "confined reads fire C2");
    assert_eq!(r.self_referential[0].struct_name, "P");
    assert_eq!(r.self_referential[0].fields.len(), 2);
    assert!(r.render().contains("[C2]"));
}

#[test]
fn never_plumbed_literal_default_fires_c2() {
    // Zero reads anywhere, literal init — A needs a const, B needs an
    // assert; only C2 sees the dead struct.
    let r = one(r"
pub struct Dead { pub x: u32 }
impl Default for Dead { fn default() -> Self { Self { x: 3 } } }
pub fn make() -> Dead { Dead::default() }
");
    assert_eq!(r.self_referential.len(), 1);
    assert!(r.inert_knobs.is_empty() && r.assert_only.is_empty());
}

#[test]
fn c2_documented_write_only_marker_downgrades() {
    // The seal-remake FeatureFlags shape: every field doc carries the
    // exact marker vocabulary, zero reads anywhere — the strong C2
    // verdict renders at the weak annotation instead.
    let r = one(r"
pub struct Flags {
    /// Forward surface: read from KV/env at load time once the runtime grows a loader (Issue 030).
    pub big: bool,
    /// Forward surface: read from KV/env at load time once the runtime grows a loader (Issue 030).
    pub live: bool,
}
impl Default for Flags { fn default() -> Self { Self { big: true, live: false } } }
");
    assert_eq!(r.self_referential.len(), 1);
    let s = &r.self_referential[0];
    assert!(s.documented_write_only, "all fields marked → downgrade");
    assert!(r.render().contains("[C2·documented-write-only]"));
    assert!(!r.render().contains("  [C2] "));
}

#[test]
fn c2_marker_needs_every_field_marked() {
    // One unmarked field keeps the strong verdict — the marker speaks
    // per field, the C2 verdict per struct.
    let r = one(r"
pub struct Mixed {
    /// Forward surface: read from KV/env once the loader lands.
    pub marked: bool,
    pub unmarked: bool,
}
impl Default for Mixed { fn default() -> Self { Self { marked: true, unmarked: false } } }
");
    assert_eq!(r.self_referential.len(), 1);
    let s = &r.self_referential[0];
    assert!(!s.documented_write_only, "one unmarked field → strong C2");
    assert!(r.render().contains("[C2]"));
}

#[test]
fn external_read_suppresses_c2() {
    let r = one(r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
impl P { pub fn validate(&self) -> bool { self.a > 0.0 } }
pub fn consume(c: &P) -> f32 { c.a * 2.0 }
");
    assert!(
        r.self_referential.is_empty(),
        "one external read keeps the struct live"
    );
}

#[test]
fn other_types_impl_reads_are_external() {
    // A read inside impl Engine (bool-returning, validate-SHAPED even) is
    // an EXTERNAL consumer of P — the impl-type check precedes the
    // method-shape check.
    let r = one(r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
pub struct Engine { pub p: P }
impl Engine {
    fn enabled(&self) -> bool { self.p.a > 0.0 }
}
");
    assert!(
        r.self_referential.is_empty(),
        "impl Engine reading .a is external for P"
    );
}

#[test]
fn trait_impl_reads_are_external_no_c2() {
    // FP-class #3 (2026-09-14): reads inside `impl Display for X` are
    // externally dispatchable — the impl'd trait IS the external
    // contract. Pre-fix these counted as validate-shaped internal reads
    // (`fmt::Result`'s last path segment is `Result`) and fired C2 —
    // the katgpt-rs 772 three-FP class, probe-reproduced at HEAD
    // (ColinearityBatchGate / RuleBasedVerifier / EntropyConflictDetector).
    let r = one(r#"
pub struct Gate { pub threshold: f32 }
impl Default for Gate { fn default() -> Self { Self { threshold: 0.5 } } }
impl core::fmt::Display for Gate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.threshold)
    }
}
"#);
    assert!(
        r.self_referential.is_empty(),
        "Display-impl reads are dispatchable, not self-referential: {:?}",
        r.self_referential
    );
}

#[test]
fn trait_impl_validate_shaped_reads_are_external() {
    // The RuleBasedVerifier/SchurSolver shape: a custom TRAIT's
    // bool-returning method reading the config's fields — validate-
    // shaped, but a trait impl: any generic/dyn caller dispatches
    // through it, so the reads are external.
    let r = one(r"
pub trait Verifier { fn accepts(&self) -> bool; }
pub struct RuleBased { pub limit: u32 }
impl Default for RuleBased { fn default() -> Self { Self { limit: 8 } } }
impl Verifier for RuleBased {
    fn accepts(&self) -> bool { self.limit > 0 }
}
");
    assert!(
        r.self_referential.is_empty(),
        "trait-impl validate-shaped reads are dispatchable: {:?}",
        r.self_referential
    );
}

#[test]
fn payload_result_reads_are_flow_out_no_c2() {
    // The SchurSolver class (riir-train 546's hand-verified FP): an
    // inherent method returning a PAYLOAD Result — the payload carries
    // the fields' influence out through the return type. Pre-fix the
    // last-segment "Result" heuristic read these as validate-shaped
    // internal checks and fired C2.
    let r = one(r#"
pub struct Solver { pub eps_reg: f32 }
impl Default for Solver { fn default() -> Self { Self { eps_reg: 1e-8 } } }
impl Solver {
    pub fn solve(&self, n: usize) -> Result<Vec<f32>, String> {
        if self.eps_reg > 0.0 { Ok(vec![0.0; n]) } else { Err("bad".into()) }
    }
}
"#);
    assert!(
        r.self_referential.is_empty(),
        "payload-Result reads flow the value out: {:?}",
        r.self_referential
    );
}

#[test]
fn unit_result_reads_still_confine_c2() {
    // The boundary pin: `Result<(), E>` is a CHECK (unit payload —
    // nothing derived from the fields can leave) and stays
    // validate-shaped internal. The canonical C2 shape keeps firing.
    let r = one(r#"
pub struct Chk { pub tol: f32 }
impl Default for Chk { fn default() -> Self { Self { tol: 0.5 } } }
impl Chk {
    pub fn check(&self) -> Result<(), String> {
        if self.tol < 0.0 { Err("neg".into()) } else { Ok(()) }
    }
}
"#);
    assert_eq!(
        r.self_referential.len(),
        1,
        "unit-Result checks stay internal"
    );
}

#[test]
fn inherent_validate_still_fires_alongside_trait_impl() {
    // The boundary discipline: an INHERENT validate() keeps counting as
    // internal when it is the ONLY confinement left (the C2 canonical
    // shape unchanged by the refinement).
    let r = one(r"
pub struct P { pub a: f32, pub b: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5, b: 0.25 } } }
impl P {
    pub fn validate(&self) -> bool { self.a > self.b }
}
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn smoke() { let p = P::default(); assert!(p.a > p.b); }
}
");
    assert_eq!(
        r.self_referential.len(),
        1,
        "inherent-only confinement still fires C2"
    );
}

#[test]
fn accessor_method_is_flow_out_not_internal() {
    // `fn a(&self) -> f32 { self.a }` exposes the value — the field IS
    // honored through the accessor; only validate-shaped returns confine.
    let r = one(r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
impl P { pub fn a(&self) -> f32 { self.a } }
");
    assert!(
        r.self_referential.is_empty(),
        "accessor return = flow-out = external"
    );
}

#[test]
fn external_destructuring_suppresses_c2() {
    let r = one(r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
impl P { pub fn validate(&self) -> bool { self.a > 0.0 } }
pub fn take(c: &P) -> f32 { let P { a, .. } = c; a }
");
    assert!(
        r.self_referential.is_empty(),
        "external destructuring reads the fields"
    );
}

#[test]
fn same_field_name_masks_c2_false_negative_direction() {
    // Name-matching: an unrelated struct's same-named field read
    // externally suppresses T's finding — the documented under-reporting
    // direction (never a false positive).
    let r = one(r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
pub struct Other { pub a: f32 }
impl Default for Other { fn default() -> Self { Self { a: 1.0 } } }
pub fn consume(o: &Other) -> f32 { o.a }
");
    assert!(
        r.self_referential.is_empty(),
        "name-masking = false negative, pinned"
    );
}

#[test]
fn test_file_path_is_test_context() {
    // A read in a tests/-path file does not keep the struct live.
    let r = audit_sources(&[
        (
            "src/lib.rs".to_string(),
            r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
"
            .to_string(),
        ),
        (
            "tests/p_integration.rs".to_string(),
            r"use mycrate::P;
#[test]
fn t() { let p = P::default(); assert!(p.a > 0.0); }"
                .to_string(),
        ),
    ]);
    assert_eq!(
        r.self_referential.len(),
        1,
        "tests/-file reads are test-context"
    );
}

// ── P25: the git-derived walk ─────────────────────────────────────

fn tmp_repo(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "cfgaudit_p25_{tag}_{}_{}",
        std::process::id(),
        line!()
    ));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&d)
            .args(["init", "-q"])
            .output()
            .is_ok_and(|o| o.status.success()),
        "git init (the P25 walk needs a real repository)"
    );
    d
}

fn tmp_plain(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "cfgaudit_p25n_{tag}_{}_{}",
        std::process::id(),
        line!()
    ));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn parse_ls_files_z_splits_joins_and_drops_missing() {
    let _base = Path::new("/base");
    let here = std::env::temp_dir().join(format!("cfgaudit_p25_exists_{}.rs", std::process::id()));
    fs::write(&here, b"x").unwrap();
    let rel = here
        .strip_prefix(std::env::temp_dir())
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    // NUL-separated: two nonexistent entries + one real file +
    // trailing NUL (the real one lives under TMPDIR — a per-terminal
    // dir under the agent harness, still inside `base`).
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(b"a.rs\0");
    bytes.extend_from_slice(rel.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(b"gone.rs\0");
    bytes.push(0);
    let got = parse_ls_files_z(&bytes, &std::env::temp_dir());
    let names: Vec<String> = got.iter().map(|p| p.display().to_string()).collect();
    assert_eq!(names.len(), 1, "nonexistent dropped, real kept: {names:?}");
    // The pid-discriminated name (f5ada0ec) — ends_with against the
    // FULL discriminated stem, not the pre-discriminator literal.
    let expect = format!("cfgaudit_p25_exists_{}.rs", std::process::id());
    assert!(names[0].ends_with(&expect), "{names:?}");
    let _ = fs::remove_file(&here);
}

#[test]
fn git_walk_excludes_gitignored_keeps_untracked_and_tracked() {
    let tmp = tmp_repo("excl");
    fs::write(tmp.join(".gitignore"), "ignored/\n*.gen.rs\n").unwrap();
    fs::write(tmp.join("tracked.rs"), "pub fn t() {}").unwrap();
    fs::write(tmp.join("wip.rs"), "pub fn w() {}").unwrap(); // untracked, not ignored
    fs::create_dir_all(tmp.join("ignored")).unwrap();
    fs::write(tmp.join("ignored/x.rs"), "pub fn x() {}").unwrap();
    fs::write(tmp.join("y.gen.rs"), "pub fn g() {}").unwrap();
    // tracked-but-later-ignored stays: add BEFORE the ignore rule lands.
    fs::write(tmp.join("tbi.rs"), "pub fn b() {}").unwrap();
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&tmp)
            .args(["add", "tracked.rs", "tbi.rs"])
            .output()
            .is_ok_and(|o| o.status.success())
    );
    fs::write(tmp.join(".gitignore"), "ignored/\n*.gen.rs\ntbi.rs\n").unwrap();
    let walk = read_sources_walk(std::slice::from_ref(&tmp));
    assert_eq!(walk.mode, WalkMode::Git { roots: 1 });
    let names: Vec<&str> = walk.sources.iter().map(|(f, _)| f.as_str()).collect();
    assert!(names.iter().any(|f| f.contains("tracked.rs")));
    assert!(
        names.iter().any(|f| f.contains("wip.rs")),
        "untracked non-ignored WIP kept"
    );
    assert!(
        names.iter().any(|f| f.contains("tbi.rs")),
        "tracked-but-later-ignored kept"
    );
    assert!(
        names.iter().all(|f| !f.contains("ignored/")),
        "gitignored dir excluded"
    );
    assert!(
        names.iter().all(|f| !f.contains("y.gen.rs")),
        "gitignored glob excluded"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn nested_foreign_repo_is_not_scanned() {
    let tmp = tmp_repo("nested");
    fs::write(tmp.join("own.rs"), "pub fn o() {}").unwrap();
    let nested = tmp.join("nested_repo");
    fs::create_dir_all(&nested).unwrap();
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(&nested)
            .args(["init", "-q"])
            .output()
            .is_ok_and(|o| o.status.success())
    );
    fs::write(nested.join("foreign.rs"), "pub fn f() {}").unwrap();
    let walk = read_sources_walk(std::slice::from_ref(&tmp));
    assert_eq!(walk.mode, WalkMode::Git { roots: 1 });
    let names: Vec<&str> = walk.sources.iter().map(|(f, _)| f.as_str()).collect();
    assert!(names.iter().any(|f| f.contains("own.rs")));
    assert!(
        names.iter().all(|f| !f.contains("foreign.rs")),
        "nested repo's files not ours"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn subdir_root_limits_to_subdir() {
    let tmp = tmp_repo("subdir");
    fs::write(tmp.join("top.rs"), "pub fn t() {}").unwrap();
    fs::create_dir_all(tmp.join("sub")).unwrap();
    fs::write(tmp.join("sub/inner.rs"), "pub fn i() {}").unwrap();
    let walk = read_sources_walk(&[tmp.join("sub")]);
    assert_eq!(walk.mode, WalkMode::Git { roots: 1 });
    let names: Vec<&str> = walk.sources.iter().map(|(f, _)| f.as_str()).collect();
    assert!(names.iter().any(|f| f.contains("inner.rs")));
    assert!(
        names.iter().all(|f| !f.contains("top.rs")),
        "cwd-limited listing, not repo-wide"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn non_repo_root_falls_back_to_plain_walk() {
    let tmp = tmp_plain("fallback");
    fs::write(tmp.join("a.rs"), "pub fn a() {}").unwrap();
    let walk = read_sources_walk(std::slice::from_ref(&tmp));
    assert_eq!(walk.mode, WalkMode::Plain);
    assert_eq!(walk.sources.len(), 1);
    assert!(walk.sources[0].0.ends_with("a.rs"));
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn explicit_ignored_file_is_scanned_anyway() {
    let tmp = tmp_repo("explicit");
    fs::write(tmp.join(".gitignore"), "b.rs\n").unwrap();
    fs::write(tmp.join("b.rs"), "pub fn b() {}").unwrap();
    let walk = read_sources_walk(&[tmp.join("b.rs")]);
    assert_eq!(
        walk.mode,
        WalkMode::Plain,
        "explicit file(s) only — no dir roots"
    );
    assert_eq!(
        walk.sources.len(),
        1,
        "caller intent wins over the ignore rule"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn entirely_ignored_dir_root_falls_back_to_intent() {
    let tmp = tmp_repo("igdir");
    fs::write(tmp.join(".gitignore"), "corpus/\n").unwrap();
    fs::create_dir_all(tmp.join("corpus")).unwrap();
    fs::write(tmp.join("corpus/x.rs"), "pub fn x() {}").unwrap();
    let walk = read_sources_walk(&[tmp.join("corpus")]);
    assert_eq!(
        walk.mode,
        WalkMode::Plain,
        "ignored dir named on purpose = intent"
    );
    assert_eq!(walk.sources.len(), 1);
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn repo_and_plain_roots_mix() {
    let repo = tmp_repo("mix");
    fs::write(repo.join("r.rs"), "pub fn r() {}").unwrap();
    let plain = tmp_plain("mix");
    fs::write(plain.join("p.rs"), "pub fn p() {}").unwrap();
    let walk = read_sources_walk(&[repo.clone(), plain.clone()]);
    assert_eq!(
        walk.mode,
        WalkMode::Mixed {
            git_roots: 1,
            plain_roots: 1
        }
    );
    assert_eq!(walk.sources.len(), 2);
    let _ = fs::remove_dir_all(&repo);
    let _ = fs::remove_dir_all(&plain);
}
