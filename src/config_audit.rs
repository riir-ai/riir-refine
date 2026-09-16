//! The config-hygiene audit — intake classes A + B (Plan 124).
//!
//! Report-first, gates nothing (the `--frontier-report` law): a report that
//! reds is a gate nobody runs. The report itself always exits 0.
//!
//! **Class A — `config-default-is-a-named-constant-and-the-consumer-reads-the-constant`**
//! (intake 2026-09-07, measured 3x in one repo by the riir-ai Issue-888
//! consumer session): a `pub` config field whose hand-written `Default` init
//! is a bare path to a named/associated const, where consuming code reads the
//! CONST (not `self.field`). Silent-wrong configuration — the caller sets the
//! knob, gets the constant, and any A/B over that dimension measures nothing.
//! Invisible to `cargo clippy` at every feature state: a `pub` field is
//! assumed used-by-API in a lib crate (probe evidence, clippy 1.98: zero
//! findings on the inert shape in BOTH lib and bin crate forms — Plan 124 T1;
//! the gap is measured, not assumed).
//!
//! **Class B — `assert-on-default-config-value`** (intake 2026-09-07): a
//! Default-initialized field whose ONLY interactions in the scanned set are
//! `assert*!` macros comparing it against **its own Default value** (the
//! literal / const / `None` it inits to, or a tautological bool) — asserting
//! `Default` reproduces `Default` proves nothing about the field being
//! *used*; an inert knob reads as covered. Asserts against anything else are
//! BEHAVIORAL (a computed expectation) and count as reads — the intake: “the
//! catching test is always behavioral”. This precision clause was added by
//! T5 hand-verification (riir-chain `GuardVerdict.backoff_ticks`: 8 asserts,
//! all against computed decay curves — a behavioral suite, not fake
//! coverage).
//!
//! **Class C2 — `self-referential-config-struct`** (Plan 125; intake class C
//! sub-shape C2, the riir-ai Issue-888 T18 escalation; measured once:
//! riir-games `PlasmaConfig`, deleted at riir-ai `dbb939b26`): a struct with
//! a hand-written `impl Default` whose EVERY candidate field has zero
//! EXTERNAL reads — reads are confined to (a) test context (`#[cfg(test)]`
//! mods, `#[test]`/`#[bench]` fns, `tests/`/`benches/` files, file-level
//! `#![cfg(test)]`) and (b) the struct's own INHERENT validate-shaped
//! methods — `bool`-returning or UNIT-`Result<(), E>`-returning checks
//! (`validate()`, `is_ok()`); a PAYLOAD `Result<T, E>` carries the
//! fields' influence out through the return type (the `SchurSolver` class:
//! `solve(..) -> Result<Array1<f32>, E>` reads tolerance knobs to
//! PRODUCE the answer — flow-out, riir-train 546's hand-verified FP). Reads
//! inside `impl Trait for T` are EXTERNAL regardless of return shape —
//! the impl'd trait IS the external contract: any generic/dyn caller
//! dispatches through it (`Display`/`Debug` formatting, solver/verifier
//! traits). The FP-class #3 refinement (2026-09-14): measured on
//! katgpt-rs 772's three C2 FPs (`ColinearityBatchGate`/
//! `RuleBasedVerifier`/`EntropyConflictDetector` — all probe-reproduced
//! pre-fix) + riir-train 546's `SchurSolver`/`OpenEndedMetrics`. Classes A/B
//! require zero reads, so an internally-read, externally-orphaned config
//! struct is invisible to them; `dead_code` cannot see it either (`pub` =
//! assumed used). Also catches the never-plumbed literal default (zero reads
//! anywhere, literal init — A needs a const, B needs an assert).
//!
//! C2 read attribution: a read inside `impl X` where X ≠ T (another type's
//! method consuming the config) is EXTERNAL — it keeps T live. A read inside
//! an INHERENT `impl T` is internal ONLY when the enclosing method is
//! validate-shaped; any other return type (accessor `fn a(&self) -> f32 {
//! self.a }`, builders) is FLOW-OUT and counts as external — the value
//! leaves the struct through the method, so the field is honored. A read
//! inside `impl Trait for T` is ALWAYS external (dispatchable through the
//! trait — the FP-class #3 refinement). C2 is per-STRUCT and may co-fire
//! with per-field A/B on an all-inert struct; the row aggregates them (the
//! whole struct is a deletion candidate, not just the one knob).
//!
//! ## What the detector is NOT (honest limits)
//!
//! - NOT a lint engine — the value is the report + citations; consumers
//!   decide (suggest-only; every row carries a fix direction).
//! - NOT crate-complete: consumers in UPSTREAM/sibling crates are invisible —
//!   under-reporting direction, stated in the report footer (the intake's
//!   reverse lesson: 7 of 22 rows in the consumer's heuristic turned out live
//!   in a sibling repo).
//! - NOT typed: field reads match by NAME across all types. Over-counting
//!   reads can only cause false NEGATIVES (a same-named field elsewhere masks
//!   a real inert knob) — never false positives. Destructuring binds DO count
//!   as reads: the documented blind spot that put three LIVE fields in the
//!   consumer-script's most-confident bucket; counting them is this
//!   detector's precision edge.
//! - Macro bodies are token-scanned: `.ident` after a non-`.` punct counts as
//!   a field touch (assert-family → assert touch; anything else → read
//!   touch, so `println!("{:?}", c.f)` does not read as "unread"); a bare
//!   SCREAMING ident counts as a const read (`assert_eq!(c.f, K)` reads K —
//!   condition 3 stays correct inside macros). Destructuring inside macro
//!   bodies is invisible (rare; false-negative direction).
//! - Const reads are counted everywhere EXCEPT inside `impl Default` blocks
//!   themselves: the default's own RHS is not an external consumer (the
//!   "const IS read elsewhere" condition must not be self-satisfied).
//!
//! ## Firing conditions
//!
//! Class A: (1) hand-written `impl Default for T` inits the field with a bare
//! const path (last segment `SCREAMING_CASE`; `AUDIT_RING_K`,
//! `LoraTarget::COUNT`); (2) zero read interactions for the field in the
//! scanned set; (3) ≥1 external read of that const (full-path match when the
//! default was qualified — kills the cross-type `::COUNT` collision;
//! last-segment match when unqualified). Array-length uses of the const are
//! counted AND flagged: a const used as an array length makes the field
//! *structurally un-honorable* — you cannot wire the knob without changing
//! array types.
//!
//! Class B: zero reads, ≥1 assert touch (class A takes precedence when both).
//!
//! Class C2: for every candidate field f of struct T,
//! `reads_total(f) − reads_tests(f) − reads_in_impl(T, f) == 0` — all reads
//! test-confined or T-own-validate-confined. Name-matching masking (a
//! same-named field on another struct read externally hides T's orphan) is a
//! FALSE NEGATIVE, same direction as A/B; never a false positive from
//! name-matching.
//!
//! ## The gpu-uniform downgrade (P28, 2026-09-16)
//!
//! A struct whose fields ride a shader uniform block (a `#[uniform(…)]` bind
//! attribute — bevy `AsBindGroup`/`ShaderType`) has its bytes consumed by the
//! WGSL even when no Rust code reads the field: struct-literal writes and
//! per-frame buffer uploads are invisible to the member-read tally, and the
//! instrument does not parse WGSL. Measured specimen: seal-game-editor
//! `WaterParams.waves`/`.misc` (2 [A] rows) — written per-frame by the canvas
//! (`show_animated_water`), read in the vertex/fragment shaders through the
//! `#[uniform(0)]` block; the sibling field-expr writes in
//! `apply_water_light` are exactly why `sun`/`sun_color`/`ambient` escaped
//! [A] (the name-matched tally counts field-expr assignment as a touch).
//! [A] and [C2] rows on such structs downgrade to the weak `[gpu-uniform]`
//! annotation (downgrade-not-suppress, the `[wire-const]` precedent): a
//! declared-but-unread WGSL field would be schema-dead weight, so the row
//! stays visible with a confirm-the-shader fix line instead of disappearing.
//! The bind type set is closed transitively through nested `ShaderType`
//! struct fields (a nested struct is flattened into the same block).

//! ## The test-rig downgrade (2026-09-16)
//!
//! A struct DECLARED at top level in a `tests/`/`benches/`-path file whose
//! every read sits under tests-path files is definitionally confined — it
//! cannot be production config (tests files do not compile into the lib),
//! and the C2 class was built for PRODUCTION config structs whose consumer
//! never materialized (the `PetAiConfig` canonical TP). Measured specimens:
//! riir-ai `Outcome` (`riir-engine` `tests/cgsp_rewind_recovery_poc.rs`) and
//! `ProbeConfig` (`riir-gpu` `tests/support/gpu_exclusivity.rs`) — both with
//! LIVE in-rig reads that read as confined test context only because the
//! whole file is a test. Such rows downgrade to the weak `[C2·test-rig]`
//! annotation (downgrade-not-suppress): the fix line points at the rig
//! (prune it with its test), not at a production schema. Eligibility is
//! CONSERVATIVE — top-level decl in the tests tree AND every read under
//! tests-path files: a src/-declared struct whose reads are cfg(test)-
//! confined stays at FULL [C2] (the path axis is separate from cfg(test)
//! confinement), and a same-named field read outside the tree (the
//! name-global masquerade) blocks the downgrade. Test-rig outranks
//! gpu-uniform when both apply — a tests-tree struct is not production
//! schema either way.

//! ## The walk (P25, 2026-09-15)
//!
//! [`read_sources_walk`] — shared by `--config-audit`, `--orphan-report`,
//! and `--platform-audit` — derives the scanned file set from GIT when the
//! scan root is inside a repository: `git ls-files --cached --others
//! --exclude-standard -- '*.rs'` (one subprocess per directory root). That
//! is tracked + untracked-NON-IGNORED — GITIGNORED files are excluded, the
//! measured driver being the P24 v3 workspace sweep's 3 standing riir-train
//! rows: name-global masquerades against `data/mining_v3/files/
//! rust-lang__rust/*.rs`, a GITIGNORED rustc-snapshot corpus (`git
//! check-ignore` → `.gitignore:149`) no repo owns — the trap-audit lesson
//! ("walking the filesystem instead reported 25 findings in a gitignored
//! vendored drop") hitting this walk. Untracked WIP files STAY in the scan
//! (the goal is gitignore-exclusion, not tracked-purity — a report that
//! silently misses fresh files would be a regression), and a TRACKED file
//! matching a later-added ignore rule stays too (git cannot retroactively
//! ignore the index). Nested foreign checkouts (worktrees, submodule
//! gitlinks, untracked nested repos) are naturally excluded — v2's
//! symlink/`.git`-file walk prune remains only for the fallback. Explicitly
//! named FILES and entirely-ignored directory ROOTS are scanned anyway
//! (caller intent wins). Roots outside a repository (fixtures, /tmp probes,
//! git absent from PATH) fall back to the v2 filesystem walk; the derived
//! [`WalkMode`] is printed by every consuming report so the distinction is
//! never silent.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, Item, Macro};

/// One Default-impl field candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldCandidate {
    /// The implementing struct (`Self` resolved to the impl type).
    pub struct_name: String,
    /// The field name.
    pub field_name: String,
    /// Full path string of the const RHS as written in the Default impl
    /// (`AUDIT_RING_K`, `LoraTarget::COUNT`); `None` for non-const inits.
    pub const_path: Option<String>,
    /// Token text of a LITERAL or simple-ident (`None`) Default init — the
    /// class-B default-equality comparator.
    pub init_token: Option<String>,
    /// `file:line` of the field's init in the Default impl.
    pub origin: String,
}

/// A class-A finding: a knob that cannot be turned.
#[derive(Debug, Clone)]
pub struct InertKnob {
    pub candidate: FieldCandidate,
    /// External reads of the const path across the scanned set.
    pub const_reads: usize,
    /// How many of those are array-length positions.
    pub array_len_uses: usize,
}

/// A class-B finding: asserted-only coverage.
#[derive(Debug, Clone)]
pub struct AssertOnly {
    pub candidate: FieldCandidate,
    pub assert_interactions: usize,
}

/// One field's read breakdown inside a class-C2 finding.
#[derive(Debug, Clone)]
pub struct SelfRefField {
    pub field_name: String,
    pub total_reads: usize,
    pub test_reads: usize,
    pub self_impl_reads: usize,
}

/// A class-C2 finding: the whole struct's candidate fields have zero
/// external reads (test + own-validate-impl confined).
#[derive(Debug, Clone)]
pub struct SelfReferentialConfig {
    pub struct_name: String,
    /// `file:line` of the FIRST candidate field's Default-impl init.
    pub origin: String,
    /// `#[uniform(…)]` bind — the C2 verdict renders at the weak
    /// `[C2·gpu-uniform]` annotation (P28; the shader consumes the
    /// block's bytes, so "delete the struct" is not a safe fix line).
    pub gpu_uniform: bool,
    /// Declared at top level in a tests-path file AND every read under a
    /// tests-path file — the C2 verdict renders at the weak
    /// `[C2·test-rig]` annotation (2026-09-16, the riir-ai
    /// `Outcome`/`ProbeConfig` intake): the C2 class was built for
    /// PRODUCTION config structs whose consumer never materialized
    /// (`PetAiConfig`); a struct that lives wholly in the tests tree is
    /// definitionally confined — if the rig is dead, delete the file.
    /// Outranks `gpu_uniform` when both apply (a tests-tree struct is not
    /// production schema either way).
    pub test_rig: bool,
    pub fields: Vec<SelfRefField>,
}

/// The audit result (data only — rendering is [`ConfigAuditReport::render`]).
#[derive(Debug, Default)]
pub struct ConfigAuditReport {
    pub files_scanned: usize,
    /// How the file set was derived (P25) — rendered under the header so
    /// the git-derived vs filesystem distinction is never silent.
    pub walk: WalkMode,
    pub candidates: usize,
    pub inert_knobs: Vec<InertKnob>,
    /// The P28 intake downgrade: [A]-shaped rows on a struct whose fields
    /// ride a shader uniform block (`#[uniform(…)]` bind attribute — the
    /// WGSL consumes the block's bytes without a Rust read). Weak severity,
    /// never hidden (the detector does not parse WGSL — the
    /// `[documented-write-only]`/`[wire-const]` precedent:
    /// downgrade-not-suppress).
    pub gpu_uniform: Vec<InertKnob>,
    /// Class-A rows whose FIELD doc carries a deliberate-write-only marker —
    /// the knob is documented as intentionally not consulted (e.g. a
    /// const-generic forecaster keeps the field for a future dispatcher that
    /// picks among pre-monomorphized shapes; the `HlaKarcConfig.delay_k`
    /// shape). Weak severity, never hidden (downgrade-not-suppress — the
    /// doc's claim is not verified; if it is NOT deliberate, wire the
    /// consumers to the field or drop the marker wording).
    pub documented_write_only_knobs: Vec<InertKnob>,
    pub assert_only: Vec<AssertOnly>,
    /// The Issue-030-class intake downgrade: class-B rows whose FIELD doc
    /// carries a deliberate-write-only marker. Weak severity, never hidden
    /// (the detector cannot verify the doc's claim — the `[wire-const]`
    /// precedent: downgrade-not-suppress).
    pub documented_write_only: Vec<AssertOnly>,
    pub self_referential: Vec<SelfReferentialConfig>,
}

/// One field with zero EXTERNAL reads — the class-A condition-2 set at field
/// granularity (Plan 126, the orphan-report current leg). `external =
/// total − tests − own_impl` clamped at 0, so test-confined and
/// validate-confined reads do NOT keep a field out of this set (the intake's
/// mandate: "check the field still has a non-test reader").
#[derive(Debug, Clone)]
pub struct ZeroReadField {
    pub candidate: FieldCandidate,
    pub total_reads: usize,
    pub test_reads: usize,
    pub self_impl_reads: usize,
}

/// The zero-external-read field scan — input to the C1 history leg.
#[derive(Debug, Default)]
pub struct ZeroReadScan {
    pub files_scanned: usize,
    pub candidates: usize,
    pub fields: Vec<ZeroReadField>,
    /// (struct, field) pairs whose FIELD doc carries a deliberate-write-only
    /// marker (the config-audit vocabulary). The C1 classifier downgrades a
    /// marked row from ORPHANED/stillborn severity to the weak
    /// `[documented-write-only]` annotation — visible, never verified.
    pub marked_write_only: BTreeSet<(String, String)>,
}

/// Class-B comparator context: field name → the default representations it
/// can assert against (const last segment, init token text).
type DefaultsByField = BTreeMap<String, Vec<(Option<String>, Option<String>)>>;

/// Scan (filename, source) pairs — the entry the unit tests drive;
/// [`audit_paths`] is the file-walking wrapper. Unparseable files are
/// skipped silently (a report must not die on one broken fixture file).
#[must_use]
pub fn audit_sources(sources: &[(String, String)]) -> ConfigAuditReport {
    let (candidates, ix, files) = scan_core(sources);
    let field_docs = collect_field_docs(&files);
    let gpu_uniform = collect_gpu_uniform_structs(&files);
    let test_rig = collect_test_rig_decls(&files);
    classify(
        &candidates,
        &ix,
        &field_docs,
        files.len(),
        &gpu_uniform,
        &test_rig,
    )
}

/// Fields whose reads are all test- or own-impl-confined (the C1 candidate
/// set, Plan 126): `external(f) = total − tests − own_impl(T, f) == 0` per
/// field. Unlike class A (which requires total==0, not even test reads) this
/// is the intake's exact "no NON-TEST reader at HEAD" condition.
///
/// The Plan-126 canonical case forced one extension over the A/B/C2
/// candidate net: `BehaviorConfig.alarm_radius` (riir-games-civ, the
/// YAML designer surface a const-table optimization orphaned) has NO
/// `impl Default` — it is constructed by serde + handwritten struct
/// literals. So this scan ALSO collects constructor-init candidates from
/// serde-backed structs (`#[serde(…)]` anywhere on the item, or a derive
/// naming `Deserialize`; Serialize-only structs are excluded — their
/// fields are read by the derive, invisible to the member-read tally).
///
/// The measured serde boundary (riir-ai Issue 934 first-contact, the
/// `ConversationEntry` false positive): fields carrying
/// `#[serde(skip_serializing_if = …)]` are EXCLUDED — conditionally-emitted
/// output-schema surface whose reader is the Serialize derive (invisible
/// here). Plain `#[serde(default)]` fields in the SAME Ser+De struct stay
/// candidates: on the full flagged sample the skip-attrs fields were
/// exactly the two false positives and the plain-default fields
/// (`alarm_radius`, `boids`, …) the true positives — persistence mirrors
/// whose functional read is what the report measures. `#[serde(skip)]`
/// (never serialized NOR deserialized) is NOT excluded — the derive never
/// touches it, so a never-member-read skip field is genuinely dead.
#[must_use]
pub fn zero_external_read_fields(sources: &[(String, String)]) -> ZeroReadScan {
    let (mut candidates, ix, files) = scan_core(sources);
    let files_scanned = files.len();
    let (serde_structs, skip_ser_fields) = collect_serde_config_structs(&files);
    if !serde_structs.is_empty() {
        let local_crates = scan_local_crate_roots(sources);
        let local_fields = collect_struct_field_sets(&files);
        let mut extra: Vec<FieldCandidate> = Vec::new();
        for (name, file) in &files {
            let scope = FileScope::of(file, &local_crates);
            let mut v = StructLitCollector {
                serde_structs: &serde_structs,
                local_fields: &local_fields,
                scope: &scope,
                file: name,
                out: &mut extra,
            };
            v.visit_file(file);
        }
        for c in extra {
            if !candidates
                .iter()
                .any(|e| e.struct_name == c.struct_name && e.field_name == c.field_name)
            {
                candidates.push(c);
            }
        }
        candidates.sort_by(|a, b| a.struct_name.cmp(&b.struct_name).then(a.field_name.cmp(&b.field_name)));
    }
    // The measured serde boundary (Issue 934's `ConversationEntry` FP):
    // `skip_serializing_if` fields are conditionally-emitted output schema —
    // the Serialize derive reads them, invisibly to this tally. Drop them
    // from the C1 net (both candidate paths — Default-impl AND literal).
    candidates.retain(|c| {
        !skip_ser_fields
            .get(&c.struct_name)
            .is_some_and(|fields| fields.contains(&c.field_name))
    });
    let mut fields = Vec::with_capacity(candidates.len());
    for cand in &candidates {
        let total = ix.field_reads.get(&cand.field_name).copied().unwrap_or(0);
        let tests = ix
            .field_reads_tests
            .get(&cand.field_name)
            .copied()
            .unwrap_or(0);
        let own = ix
            .field_reads_in_impl
            .get(&(cand.field_name.clone(), cand.struct_name.clone()))
            .copied()
            .unwrap_or(0);
        if total.saturating_sub(tests).saturating_sub(own) == 0 {
            fields.push(ZeroReadField {
                candidate: cand.clone(),
                total_reads: total,
                test_reads: tests,
                self_impl_reads: own,
            });
        }
    }
    ZeroReadScan {
        files_scanned,
        candidates: candidates.len(),
        fields,
        marked_write_only: marked_write_only_fields(&files),
    }
}

/// The shared scan core (DRY across `audit_sources` and the orphan-report's
/// current leg): parse → Default-impl candidates → interaction tally.
fn scan_core(
    sources: &[(String, String)],
) -> (Vec<FieldCandidate>, InteractionIndex, Vec<(String, syn::File)>) {
    let files: Vec<(String, syn::File)> = sources
        .iter()
        .filter_map(|(name, src)| syn::parse_file(src).ok().map(|f| (name.clone(), f)))
        .collect();

    // Pass 1: collect Default-impl field candidates.
    let mut candidates: Vec<FieldCandidate> = Vec::new();
    for (name, file) in &files {
        let mut v = DefaultImplVisitor {
            file: name.clone(),
            out: &mut candidates,
        };
        v.visit_file(file);
    }
    dedup_candidates(&mut candidates);

    // Class-B comparator context: field name → the default representations it
    // can assert against (const last segment / init token text).
    let mut defaults_by_field: DefaultsByField = BTreeMap::new();
    for c in &candidates {
        let last = c
            .const_path
            .as_ref()
            .map(|p| p.rsplit("::").next().unwrap_or(p).to_string());
        defaults_by_field
            .entry(c.field_name.clone())
            .or_default()
            .push((last, c.init_token.clone()));
    }

    // Pass 2: tally interactions across the whole set.
    let mut ix = InteractionIndex::default();
    for (name, file) in &files {
        let mut v = InteractionVisitor {
            ix: &mut ix,
            suppress_const: false,
            defaults_by_field: &defaults_by_field,
            file_is_test: file_is_test_path(name) || attrs_cfg_test(&file.attrs),
            file_in_tests_tree: file_is_test_path(name),
            test_depth: 0,
            impl_stack: Vec::new(),
            impl_is_trait: Vec::new(),
            validate_depth: 0,
        };
        v.visit_file(file);
    }

    (candidates, ix, files)
}

/// Serde-backed config surface for the C1 net (Plan 126): struct names
/// (any `#[serde(…)]` attribute on the item or one of its fields, or a
/// derive naming `Deserialize`) PLUS the per-struct set of fields carrying
/// `#[serde(skip_serializing_if = …)]` (the FP exclusion — see
/// [`zero_external_read_fields`]). Serialize-only structs are deliberately
/// absent — their fields are read by the generated code, invisible to the
/// member-read tally, so they would all read as stillborn false positives.
fn collect_serde_config_structs(
    files: &[(String, syn::File)],
) -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
    let mut out = BTreeSet::new();
    let mut skip_ser_fields = BTreeMap::new();
    for (_name, file) in files {
        let mut v = SerdeStructMarker {
            out: &mut out,
            skip_ser_fields: &mut skip_ser_fields,
        };
        v.visit_file(file);
    }
    (out, skip_ser_fields)
}

struct SerdeStructMarker<'a> {
    out: &'a mut BTreeSet<String>,
    skip_ser_fields: &'a mut BTreeMap<String, BTreeSet<String>>,
}

/// Scan-global field sets for every TOP-LEVEL `struct` declaration, unioned
/// across same-named decls (the Issue-109 re-sight, the riir-shader bevy-0.19
/// FP): `FileScope::literal_is_foreign` cannot decide a file importing BOTH a
/// foreign glob (`use bevy::prelude::*`) and a local glob
/// (`use riir_shader_core::prelude::*`) — the documented conservative
/// direction reads such a literal local. The field sets close the class from
/// the other side: a literal naming a field that NO local same-named decl has
/// provably cannot be any of them — a construction of the local type must
/// compile against its real fields, so the mismatch is a foreign type
/// masquerading under the same name. Mod-nested decls are excluded
/// (consistent with [`FileScope::of`]'s top-level rule — a bare literal in
/// another file can never name one without a path).
#[must_use]
pub fn collect_struct_field_sets(
    files: &[(String, syn::File)],
) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (_name, file) in files {
        for item in &file.items {
            if let syn::Item::Struct(s) = item {
                let entry = out.entry(s.ident.to_string()).or_default();
                for f in &s.fields {
                    if let Some(ident) = &f.ident {
                        entry.insert(ident.to_string());
                    }
                }
            }
        }
    }
    out
}

/// Does this literal name a field that NO local same-named top-level struct
/// declares? `true` = provably not the scanned set's type (skip it as
/// evidence — both the candidate net and the wire pass); `false` = local,
/// unknown type, no named fields, or no local decl — never rejected on a
/// guess (the conservative direction).
#[must_use]
pub fn literal_is_field_mismatch(
    local_fields: &BTreeMap<String, BTreeSet<String>>,
    ty: &str,
    named_fields: &[String],
) -> bool {
    match local_fields.get(ty) {
        Some(fields) => {
            !named_fields.is_empty() && named_fields.iter().any(|f| !fields.contains(f))
        }
        None => false,
    }
}

/// Field-level doc index — (struct name, field name) → concatenated field
/// doc text. The class-B downgrade evidence (Issue-030-class intake): a
/// deliberate-write-only marker counts only on the FIELD doc; the struct's
/// own doc speaks for the whole type, not for one write-only member.
type FieldDocs = BTreeMap<(String, String), String>;

fn collect_field_docs(files: &[(String, syn::File)]) -> FieldDocs {
    let mut out = FieldDocs::new();
    for (_name, file) in files {
        let mut v = FieldDocVisitor { out: &mut out };
        v.visit_file(file);
    }
    out
}

/// The (struct, field) pairs whose FIELD doc carries a deliberate-write-only
/// marker — the shared downgrade evidence. The config-audit class-B lane
/// consumes it via `collect_field_docs` directly; the orphan-report's C1/
/// stillborn lanes consume this precomputed set (P26: downgrade-not-suppress,
/// the `[wire-const]` precedent — marked rows render at the weak
/// `[documented-write-only]` annotation instead of strong severity).
pub(crate) fn marked_write_only_fields(files: &[(String, syn::File)]) -> BTreeSet<(String, String)> {
    collect_field_docs(files)
        .into_iter()
        .filter(|(_, doc)| is_deliberate_write_only_marker(doc))
        .map(|(key, _)| key)
        .collect()
}

struct FieldDocVisitor<'a> {
    out: &'a mut FieldDocs,
}

impl Visit<'_> for FieldDocVisitor<'_> {
    fn visit_item_struct(&mut self, item: &syn::ItemStruct) {
        for f in &item.fields {
            let Some(fname) = &f.ident else { continue };
            let mut doc = String::new();
            for a in &f.attrs {
                if a.path().is_ident("doc") {
                    if let syn::Meta::NameValue(nv) = &a.meta {
                        doc.push_str(&nv.value.to_token_stream().to_string());
                    }
                }
            }
            if !doc.is_empty() {
                self.out
                    .entry((item.ident.to_string(), fname.to_string()))
                    .or_default()
                    .push_str(&doc);
            }
        }
        syn::visit::visit_item_struct(self, item);
    }
}

/// The deliberate-write-only marker vocabulary (case-insensitive): the
/// canonical token, the Issue-142 wording ("deliberate, not dead
/// payload"), and the Issue-147 wording ("deliberate forward surface" /
/// "GOAT-gate territory" — the adjudication the redirect field doc now
/// carries). Matched on the field doc only, reported at weak severity.
/// The class-B downgrade evidence (Issue-030-class intake): a deliberate-
/// write-only marker counts only on the FIELD doc; the struct's own doc
/// speaks for the whole type, not for one write-only member.
/// Shared with the orphan-report's C1/stillborn lanes (the P26 downgrade) —
/// the marker vocabulary has exactly one home: [`is_deliberate_write_only_marker`].
pub(crate) fn is_deliberate_write_only_marker(doc: &str) -> bool {
    let d = doc.to_lowercase();
    d.contains("documented-write-only")
        || d.contains("deliberate")
        || d.contains("forward surface")
        || d.contains("goat-gate territory")
}

impl Visit<'_> for SerdeStructMarker<'_> {
    fn visit_item_struct(&mut self, item: &syn::ItemStruct) {
        let derives_deserialize = item.attrs.iter().any(|a| {
            a.path().is_ident("derive")
                && a.to_token_stream().to_string().contains("Deserialize")
        });
        let has_serde_attr = item
            .attrs
            .iter()
            .any(|a| a.path().is_ident("serde"))
            || item.fields.iter().any(|f| {
                f.attrs.iter().any(|a| a.path().is_ident("serde"))
            });
        if derives_deserialize || has_serde_attr {
            self.out.insert(item.ident.to_string());
            let skip_fields = self.skip_ser_fields.entry(item.ident.to_string()).or_default();
            for f in &item.fields {
                if f.attrs.iter().any(|a| {
                    a.path().is_ident("serde")
                        && a.to_token_stream().to_string().contains("skip_serializing_if")
                }) {
                    skip_fields.insert(f.ident.as_ref().map_or_default(ToString::to_string));
                }
            }
        }
        syn::visit::visit_item_struct(self, item);
    }
}

/// GPU-uniform bind surface (the P28 intake class): struct names whose
/// fields ride a shader uniform block. A field carrying `#[uniform(…)]`
/// (bevy's `AsBindGroup` bind attribute) makes its FIELD TYPE the uniform
/// block root; a `ShaderType` struct nested inside another is flattened
/// into the same block, so the closure descends through the field types of
/// collected structs and stops at foreign types (whose fields no scan
/// names). The Rust-side member-read tally cannot see the WGSL reads —
/// the block's bytes are consumed by the shader even when no Rust code
/// reads the field — so `classify` downgrades the [A]/[C2] rows of these
/// structs to the weak `[gpu-uniform]` annotation (downgrade-not-suppress:
/// a declared-but-unread shader field would be schema-dead weight, and the
/// instrument does not parse WGSL).
pub(crate) fn collect_gpu_uniform_structs(files: &[(String, syn::File)]) -> BTreeSet<String> {
    // struct name → every path-segment type ident of its fields (only for
    // structs that exist in the scanned set).
    let mut field_types: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut direct: BTreeSet<String> = BTreeSet::new();
    for (_name, file) in files {
        for item in &file.items {
            let Item::Struct(s) = item else { continue };
            let sname = s.ident.to_string();
            for f in &s.fields {
                let mut idents = Vec::new();
                collect_type_idents(&f.ty, &mut idents);
                if f.attrs.iter().any(|a| a.path().is_ident("uniform")) {
                    direct.extend(idents.iter().cloned());
                }
                field_types
                    .entry(sname.clone())
                    .or_default()
                    .extend(idents);
            }
        }
    }
    let mut out = BTreeSet::new();
    let mut queue: Vec<String> = direct.into_iter().collect();
    while let Some(n) = queue.pop() {
        if !out.insert(n.clone()) {
            continue;
        }
        if let Some(types) = field_types.get(&n) {
            for t in types {
                if !out.contains(t) {
                    queue.push(t.clone());
                }
            }
        }
    }
    out
}

/// Every path-segment ident in a type, descending generic arguments
/// (`Option<Inner>` yields `Option` and `Inner` — the inner name is the one
/// the closure consumes; the wrapper name matches nothing and is inert).
fn collect_type_idents(ty: &syn::Type, out: &mut Vec<String>) {
    match ty {
        syn::Type::Path(p) => {
            for seg in &p.path.segments {
                out.push(seg.ident.to_string());
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(t) = arg {
                            collect_type_idents(t, out);
                        }
                    }
                }
            }
        }
        syn::Type::Reference(r) => collect_type_idents(&r.elem, out),
        syn::Type::Group(g) => collect_type_idents(&g.elem, out),
        syn::Type::Paren(p) => collect_type_idents(&p.elem, out),
        _ => {}
    }
}

/// Collects constructor-init candidates from struct literals whose type is
/// a serde-backed config struct (anywhere in the file — `hardcoded_config()`
/// tables, registry builders). `Self`-typed literals are skipped: outside
/// an `impl` the type name cannot be resolved, and inside `impl Default`
/// the scan-core pass already collected them with the impl type resolved.
///
/// Issue 109 specimen A: the literal must also pass the per-file scope
/// check ([`FileScope::literal_is_foreign`]) — a bare `Name { … }` literal
/// never resolves to a sibling module's same-named struct, so a foreign
/// literal (bevy's `Node` through `use bevy::prelude::*`) is not evidence
/// for the local serde struct of the same name. Issue 109 re-sight: the
/// scope check alone cannot decide the foreign-glob + local-glob file, so
/// the literal must ALSO pass [`literal_is_field_mismatch`] against the
/// scan-global field sets — a field no local same-named decl has is
/// provably not that decl.
struct StructLitCollector<'a> {
    serde_structs: &'a BTreeSet<String>,
    local_fields: &'a BTreeMap<String, BTreeSet<String>>,
    scope: &'a FileScope,
    file: &'a str,
    out: &'a mut Vec<FieldCandidate>,
}

impl<'ast> Visit<'ast> for StructLitCollector<'_> {
    fn visit_expr_struct(&mut self, lit: &'ast syn::ExprStruct) {
        if let Some(ty) = struct_lit_type(&lit.path) {
            let named: Vec<String> = lit
                .fields
                .iter()
                .filter_map(|fv| field_member_name(&fv.member))
                .collect();
            if self.serde_structs.contains(&ty)
                && !self.scope.literal_is_foreign(&ty)
                && !literal_is_field_mismatch(self.local_fields, &ty, &named)
            {
                for fv in &lit.fields {
                    let Some(field) = field_member_name(&fv.member) else {
                        continue;
                    };
                    self.out.push(FieldCandidate {
                        struct_name: ty.clone(),
                        field_name: field,
                        const_path: const_path_of(&fv.expr),
                        init_token: init_token_of(&fv.expr),
                        origin: format!("{}:{}", self.file, fv.expr.span().start().line),
                    });
                }
            }
        }
        syn::visit::visit_expr_struct(self, lit);
    }
}

/// Per-file name-resolution scope for struct-literal attribution (Issue 109
/// specimen A): within one Rust file, a bare `Name { … }` literal resolves
/// ONLY to a type declared in that file, an explicit import, or a glob
/// import — it NEVER silently resolves to a same-named struct declared in a
/// sibling module. The serde-literal candidate net (and the orphan-report's
/// wire pass) match type NAMES file-globally, so a foreign literal
/// (`bevy_ui::Node` reached through `use bevy::prelude::*`) masquerades as
/// construction evidence for a local same-named serde struct — the measured
/// riir-shader FP (4 stillborn rows, zero true findings).
/// [`FileScope::literal_is_foreign`] is the repair: a provably-foreign
/// literal is not evidence for or against the local type.
///
/// Precedence mirrors rustc name resolution at file granularity: the file's
/// own top-level declaration wins (shadows globs), then explicit imports
/// (an in-crate `crate::`/`self::`/`super::` path can be the local struct; a
/// foreign path cannot), then globs (a foreign glob with no local glob and
/// no local decl/import means the name is foreign-or-unresolved — never the
/// sibling module's declaration). Known over-approximations, both
/// documented: `use` items are collected file-globally regardless of
/// fn/mod nesting (a fn-local import marks the whole file — the rare
/// file-local-decl + fn-local-foreign-import mix reads local, the
/// conservative direction), and glob-imported names are only decidable as
/// a class, not per-name (a foreign glob alongside a local glob reads
/// local — the conservative direction). That last undecidable shape is
/// closed one level up by the scan-global field sets
/// ([`collect_struct_field_sets`] + [`literal_is_field_mismatch`]): a
/// literal the import shape cannot decide still names fields, and a field
/// no local same-named decl has is provably foreign.
#[derive(Debug, Default)]
pub struct FileScope {
    /// Type idents declared at file top level (struct/enum/union/type alias).
    declared: BTreeSet<String>,
    /// Names explicitly imported via a crate-internal path.
    local_imports: BTreeSet<String>,
    /// Names explicitly imported from a path outside the scan's crate set.
    foreign_imports: BTreeSet<String>,
    /// A glob from outside the scan's crate set is present
    /// (`use bevy::prelude::*`).
    foreign_glob: bool,
    /// A crate-internal (or workspace-sibling) glob is present
    /// (`use crate::x::*`, `use editor_core::prelude::*`).
    local_glob: bool,
    /// Workspace-sibling crate names (dash-normalized path components of
    /// the scan set — see [`scan_local_crate_roots`]).
    local_crates: BTreeSet<String>,
}

impl FileScope {
    /// Extract the scope from a parsed file: top-level type declarations
    /// plus every `use` item in the file (nested ones included — see the
    /// type-level note on the over-approximation). `local_crates` carries
    /// the scan's workspace-sibling crate names
    /// ([`scan_local_crate_roots`]) — a workspace scan sees `use
    /// editor_core::types::XRow` from a sibling crate whose declaration IS
    /// in the scanned set, which is LOCAL evidence, not foreign.
    #[must_use]
    pub fn of(file: &syn::File, local_crates: &BTreeSet<String>) -> Self {
        let mut scope = Self {
            local_crates: local_crates.clone(),
            ..Self::default()
        };
        for item in &file.items {
            match item {
                Item::Struct(i) => {
                    scope.declared.insert(i.ident.to_string());
                }
                Item::Enum(i) => {
                    scope.declared.insert(i.ident.to_string());
                }
                Item::Union(i) => {
                    scope.declared.insert(i.ident.to_string());
                }
                Item::Type(i) => {
                    scope.declared.insert(i.ident.to_string());
                }
                _ => {}
            }
        }
        let mut v = UseScopeVisitor { scope: &mut scope };
        v.visit_file(file);
        scope
    }

    /// Is a bare `Name { … }` literal in this file PROVABLY not the scanned
    /// set's local same-named struct? (False = local, or unknown — collect,
    /// the conservative direction.)
    #[must_use]
    pub fn literal_is_foreign(&self, name: &str) -> bool {
        if self.declared.contains(name) || self.local_imports.contains(name) {
            return false;
        }
        if self.foreign_imports.contains(name) {
            return true;
        }
        self.foreign_glob && !self.local_glob
    }
}

/// Walks `use` trees accumulating explicit binding names + glob presence,
/// split by crate-internal vs foreign first path segment.
struct UseScopeVisitor<'a> {
    scope: &'a mut FileScope,
}

impl UseScopeVisitor<'_> {
    fn walk(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                self.walk(&p.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(n) => {
                prefix.push(n.ident.to_string());
                self.record(prefix);
                prefix.pop();
            }
            syn::UseTree::Rename(r) => {
                // The BINDING is the rename; the path's first segment (already
                // in `prefix`) decides foreignness — `r.ident` cannot change it.
                let binding = r.rename.to_string();
                self.scope.bind(&binding, prefix);
            }
            syn::UseTree::Glob(_) => {
                // An empty-prefix glob (`use *;`) is not valid Rust — ignore.
                if let Some(first) = prefix.first() {
                    if self.scope.is_local_root(first) {
                        self.scope.local_glob = true;
                    } else {
                        self.scope.foreign_glob = true;
                    }
                }
            }
            syn::UseTree::Group(g) => {
                for t in &g.items {
                    self.walk(t, prefix);
                }
            }
        }
    }

    fn record(&mut self, prefix: &[String]) {
        let Some((binding, path)) = prefix.split_last() else {
            return;
        };
        self.scope.bind(binding, path);
    }
}

impl<'ast> Visit<'ast> for UseScopeVisitor<'_> {
    fn visit_item_use(&mut self, i: &'ast syn::ItemUse) {
        let mut prefix = Vec::new();
        self.walk(&i.tree, &mut prefix);
    }
}

impl FileScope {
    fn bind(&mut self, binding: &str, path: &[String]) {
        let Some(first) = path.first() else { return };
        if self.is_local_root(first) {
            self.local_imports.insert(binding.to_string());
        } else {
            self.foreign_imports.insert(binding.to_string());
        }
    }

    fn is_local_root(&self, first: &str) -> bool {
        is_in_crate_segment(first) || self.local_crates.contains(first)
    }
}

/// `crate`, `self`, `super` — the self-referential path roots. Everything
/// else names another crate (or is a 2015-style root that behaves the same
/// for this purpose: not provably this crate's module tree).
fn is_in_crate_segment(seg: &str) -> bool {
    matches!(seg, "crate" | "self" | "super")
}

/// The scan's workspace-sibling crate-name candidates (Issue 109): every
/// dash-normalized path component of the scanned sources. A workspace scan
/// sees `use editor_core::types::XRow` from the sibling whose declaration
/// sits at `crates/editor-core/…` — IN the scanned set — so treating it as
/// foreign would suppress every cross-crate row literal (measured: the
/// seal-game-editor sweep lost 207 candidates and all 26 C1 rows to that
/// over-suppression before this table existed). Components like `src`,
/// `crates`, `benches` are harmless — no import roots collide with them in
/// practice, and over-inclusion is the collect (conservative) direction.
/// Single-crate scans are safe by construction: a cross-crate import of a
/// scanned-declared name can only appear in files OUTSIDE the declaring
/// crate, which such a scan does not carry.
#[must_use]
pub fn scan_local_crate_roots(sources: &[(String, String)]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (name, _) in sources {
        for comp in name.split(['/', '\\']) {
            let norm = comp.replace('-', "_");
            if !norm.is_empty() {
                out.insert(norm);
            }
        }
    }
    out
}

/// Walk `paths` (dirs recursively / files), parse every `.rs`, audit.
#[must_use]
pub fn audit_paths(paths: &[PathBuf]) -> ConfigAuditReport {
    let walked = read_sources_walk(paths);
    let mut report = audit_sources(&walked.sources);
    report.walk = walked.mode;
    report
}

/// How the scanned file set was derived (P25 — the git-derived walk).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WalkMode {
    /// Every directory root resolved inside a git repository; the file set
    /// came from `git ls-files --cached --others --exclude-standard` —
    /// GITIGNORED files excluded (the mining-corpus masquerade class),
    /// untracked non-ignored WIP files kept.
    Git {
        roots: usize,
    },
    /// No directory root inside a git repository (or git missing from
    /// PATH, or explicit file arguments only) — plain filesystem walk,
    /// nothing excluded.
    #[default]
    Plain,
    /// Some roots git-derived, some filesystem-walked.
    Mixed {
        git_roots: usize,
        plain_roots: usize,
    },
}

impl WalkMode {
    /// The one-line render (each consuming audit prints it under its
    /// population line — the distinction must never be silent).
    #[must_use]
    pub fn note(&self) -> String {
        match *self {
            Self::Git { roots } => format!(
                "walk: git-derived ({roots} root(s)) — tracked + untracked-non-ignored; \
                 GITIGNORED files excluded (P25)"
            ),
            Self::Plain => "walk: filesystem (root(s) outside a git repo, git absent, or \
                 explicit file(s) only) — gitignored files NOT excluded"
                .to_string(),
            Self::Mixed {
                git_roots,
                plain_roots,
            } => format!(
                "walk: mixed — {git_roots} git-derived root(s) (gitignored excluded), \
                 {plain_roots} filesystem root(s) (not excluded)"
            ),
        }
    }
}

/// The walked file set + how it was derived.
pub(crate) struct SourcesWalk {
    pub sources: Vec<(String, String)>,
    pub mode: WalkMode,
}

/// Walk `paths` (dirs recursively / files), read every `.rs` — shared by
/// `--config-audit`, `--orphan-report`, and `--platform-audit` (P25:
/// git-derived where the root is inside a repository).
pub(crate) fn read_sources_walk(paths: &[PathBuf]) -> SourcesWalk {
    let mut rust_files: Vec<PathBuf> = Vec::new();
    let mut git_roots = 0usize;
    let mut plain_roots = 0usize;
    for p in paths {
        if p.is_dir() {
            match git_derived_rust_files(p) {
                Some(files) if !files.is_empty() => {
                    git_roots += 1;
                    rust_files.extend(files);
                }
                Some(_) => {
                    // In a repo but git names no `.rs` under this root — an
                    // entirely-ignored directory the caller named on
                    // purpose: explicit intent wins (the explicitly-named
                    // FILE rule), fall back to the filesystem walk. A root
                    // that is empty on disk too counts as neither.
                    let before = rust_files.len();
                    collect_rust_files(p, &mut rust_files);
                    if rust_files.len() > before {
                        plain_roots += 1;
                    }
                }
                None => {
                    plain_roots += 1;
                    collect_rust_files(p, &mut rust_files);
                }
            }
        } else if p.extension().is_some_and(|e| e == "rs") {
            // Explicit file: caller intent wins, ignored or not.
            rust_files.push(p.clone());
        }
    }
    rust_files.sort();
    rust_files.dedup();
    let mode = match (git_roots, plain_roots) {
        (g, 0) if g > 0 => WalkMode::Git { roots: g },
        (0, p) if p > 0 => WalkMode::Plain,
        (g, p) if g + p > 0 => WalkMode::Mixed { git_roots: g, plain_roots: p },
        _ => WalkMode::Plain,
    };
    let sources = rust_files
        .iter()
        .filter_map(|p| {
            fs::read_to_string(p)
                .ok()
                .map(|src| (p.display().to_string(), src))
        })
        .collect();
    SourcesWalk { sources, mode }
}

/// One `git ls-files` per directory root (P25): repo membership via
/// `rev-parse --show-toplevel` (works in a repo with unborn HEAD — the
/// index answers `ls-files` without a single commit), then the
/// non-ignored set (`--cached --others --exclude-standard`, NUL-separated)
/// filtered to `*.rs`. `None` when the root is outside a repository or git
/// cannot answer (missing from PATH, subprocess failure) — the caller
/// falls back to the filesystem walk.
fn git_derived_rust_files(dir: &Path) -> Option<Vec<PathBuf>> {
    let top = git_output(dir, &["rev-parse", "--show-toplevel"])?;
    if !top.status.success() {
        return None;
    }
    let listed = git_output(
        dir,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.rs",
        ],
    )?;
    if !listed.status.success() {
        return None;
    }
    Some(parse_ls_files_z(&listed.stdout, dir))
}

/// Pure half of the git walk (unit-testable without a repository):
/// NUL-separated `ls-files` output → EXISTING files joined onto `base`,
/// sorted + deduped. Non-existent entries are dropped (staged-then-rm).
fn parse_ls_files_z(bytes: &[u8], base: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = bytes
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| base.join(String::from_utf8_lossy(s).as_ref()))
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    files.dedup();
    files
}

/// Run `git -C <dir> <args>`, capturing output. `None` when the binary
/// cannot be spawned.
fn git_output(dir: &Path, args: &[&str]) -> Option<std::process::Output> {
    Command::new("git").arg("-C").arg(dir).args(args).output().ok()
}

fn collect_rust_files(p: &Path, out: &mut Vec<PathBuf>) {
    if p.is_dir() {
        let Ok(entries) = fs::read_dir(p) else {
            return;
        };
        let mut children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        children.sort();
        for c in children {
            let name = c.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "target" | ".git" | "node_modules") {
                continue;
            }
            // A NESTED foreign checkout is not this repo's scan surface:
            // (1) a symlinked directory (seal-game-editor/.wt/riir-ai — an
            // alias to a sibling repo root; the first platform-audit
            // workspace sweep duplicated riir-ai's findings under the
            // editor's scan this way, P24 (c)); (2) a dir whose `.git` is
            // a FILE — a git worktree or submodule. Both belong to THAT
            // repo's own scan. The scan ROOT is never tested, so scanning
            // a worktree/symlink path directly still works.
            if c.is_symlink() {
                continue;
            }
            if c.is_dir() && c.join(".git").is_file() {
                continue;
            }
            collect_rust_files(&c, out);
        }
    } else if p.extension().is_some_and(|e| e == "rs") {
        out.push(p.to_path_buf());
    }
}

/// Same (struct, field) can be initialized in more than one struct literal
/// inside one Default impl — keep the first origin, upgrade to a const path
/// if any init was const-path.
fn dedup_candidates(candidates: &mut Vec<FieldCandidate>) {
    let mut out: Vec<FieldCandidate> = Vec::with_capacity(candidates.len());
    for c in candidates.drain(..) {
        if let Some(prev) = out
            .iter_mut()
            .find(|p| p.struct_name == c.struct_name && p.field_name == c.field_name)
        {
            if prev.const_path.is_none() {
                prev.const_path = c.const_path;
            }
        } else {
            out.push(c);
        }
    }
    *candidates = out;
}

fn classify(
    candidates: &[FieldCandidate],
    ix: &InteractionIndex,
    field_docs: &FieldDocs,
    files_scanned: usize,
    gpu_uniform: &BTreeSet<String>,
    test_rig: &TestRigDecls,
) -> ConfigAuditReport {
    let mut report = ConfigAuditReport {
        files_scanned,
        candidates: candidates.len(),
        ..ConfigAuditReport::default()
    };
    for cand in candidates {
        let reads = ix.field_reads.get(&cand.field_name).copied().unwrap_or(0);
        let asserts = ix.field_asserts.get(&cand.field_name).copied().unwrap_or(0);
        if reads > 0 {
            continue;
        }
        if let Some(const_path) = &cand.const_path {
            let (const_reads, array_len_uses) = ix.const_reads_of(const_path);
            if const_reads > 0 {
                let knob = InertKnob {
                    candidate: cand.clone(),
                    const_reads,
                    array_len_uses,
                };
                if gpu_uniform.contains(&cand.struct_name) {
                    // DOWNGRADE-not-suppress (the [wire-const] precedent):
                    // the shader consumes the uniform block's bytes without
                    // any Rust read; WGSL is not parsed, so the row stays
                    // visible at annotation severity.
                    report.gpu_uniform.push(knob);
                } else if field_docs
                    .get(&(cand.struct_name.clone(), cand.field_name.clone()))
                    .is_some_and(|d| is_deliberate_write_only_marker(d))
                {
                    // P29: same downgrade when the FIELD doc carries the
                    // deliberate-write-only marker (the class-B arm below is
                    // the precedent); the doc's claim is not verified, so the
                    // row stays visible at annotation severity.
                    report.documented_write_only_knobs.push(knob);
                } else {
                    report.inert_knobs.push(knob);
                }
                continue;
            }
        }
        if asserts > 0 {
            let row = AssertOnly {
                candidate: cand.clone(),
                assert_interactions: asserts,
            };
            let marked = field_docs
                .get(&(cand.struct_name.clone(), cand.field_name.clone()))
                .is_some_and(|d| is_deliberate_write_only_marker(d));
            if marked {
                // DOWNGRADE-not-suppress (the [wire-const] precedent): the
                // row stays visible at annotation severity because the
                // detector cannot verify the doc's deliberateness claim.
                report.documented_write_only.push(row);
            } else {
                report.assert_only.push(row);
            }
        }
    }
    classify_self_referential(candidates, ix, &mut report, gpu_uniform, test_rig);
    report
}

/// Class C2: group candidates per struct (insertion order) and fire when
/// every field's reads are test-confined or own-validate-impl-confined —
/// `external = total − tests − in_impl(T)` clamped at 0.
fn classify_self_referential(
    candidates: &[FieldCandidate],
    ix: &InteractionIndex,
    report: &mut ConfigAuditReport,
    gpu_uniform: &BTreeSet<String>,
    test_rig: &TestRigDecls,
) {
    let mut seen: Vec<String> = Vec::new();
    for cand in candidates {
        if seen.contains(&cand.struct_name) {
            continue;
        }
        seen.push(cand.struct_name.clone());
        let mut fields: Vec<SelfRefField> = Vec::new();
        let mut all_confined = true;
        // Every read under a tests-path file (the path axis — distinct
        // from cfg(test) confinement): the `[C2·test-rig]` input.
        let mut all_reads_in_tests_tree = true;
        for f in candidates.iter().filter(|c| c.struct_name == cand.struct_name) {
            let total = ix.field_reads.get(&f.field_name).copied().unwrap_or(0);
            let tree = ix
                .field_reads_tests_tree
                .get(&f.field_name)
                .copied()
                .unwrap_or(0);
            if total != tree {
                all_reads_in_tests_tree = false;
            }
            let tests = ix
                .field_reads_tests
                .get(&f.field_name)
                .copied()
                .unwrap_or(0);
            let own = ix
                .field_reads_in_impl
                .get(&(f.field_name.clone(), f.struct_name.clone()))
                .copied()
                .unwrap_or(0);
            if total.saturating_sub(tests).saturating_sub(own) > 0 {
                all_confined = false;
            }
            fields.push(SelfRefField {
                field_name: f.field_name.clone(),
                total_reads: total,
                test_reads: tests,
                self_impl_reads: own,
            });
        }
        if all_confined {
            report.self_referential.push(SelfReferentialConfig {
                struct_name: cand.struct_name.clone(),
                origin: cand.origin.clone(),
                gpu_uniform: gpu_uniform.contains(&cand.struct_name),
                test_rig: all_reads_in_tests_tree && test_rig.is_eligible(&cand.struct_name),
                fields,
            });
        }
    }
}

/// Top-level struct decls split by whether their file sits under a
/// `tests/`/`benches/` path — the `[C2·test-rig]` eligibility input. A
/// name declared in BOTH trees is NOT eligible (the conservative
/// direction — same-name decls cannot be told apart by the name-global
/// read tally). Nested (non-top-level) decls are out of scope, mirroring
/// `collect_gpu_uniform_structs`.
#[derive(Default)]
struct TestRigDecls {
    tests_tree: BTreeSet<String>,
    non_tests: BTreeSet<String>,
}

impl TestRigDecls {
    fn is_eligible(&self, name: &str) -> bool {
        self.tests_tree.contains(name) && !self.non_tests.contains(name)
    }
}

fn collect_test_rig_decls(files: &[(String, syn::File)]) -> TestRigDecls {
    let mut out = TestRigDecls::default();
    for (name, file) in files {
        let in_tree = file_is_test_path(name);
        for item in &file.items {
            if let syn::Item::Struct(s) = item {
                if in_tree {
                    out.tests_tree.insert(s.ident.to_string());
                } else {
                    out.non_tests.insert(s.ident.to_string());
                }
            }
        }
    }
    out
}

/// Cross-file interaction tallies (pass-2 output).
#[derive(Debug, Default)]
struct InteractionIndex {
    /// Field name → read touches.
    field_reads: BTreeMap<String, usize>,
    /// Field name → read touches in files under a `tests/`/`benches/` PATH
    /// (the path axis alone — the `[C2·test-rig]` input; distinct from
    /// cfg(test) inline confinement, which keeps src/-declared structs at
    /// full [C2]).
    field_reads_tests_tree: BTreeMap<String, usize>,
    /// Field name → read touches inside TEST context.
    field_reads_tests: BTreeMap<String, usize>,
    /// (field name, impl self-type) → read touches inside validate-shaped
    /// methods of that type's own impls (class C2's confinement tally).
    field_reads_in_impl: BTreeMap<(String, String), usize>,
    /// Field name → assert-macro touches.
    field_asserts: BTreeMap<String, usize>,
    /// Const path AS READ (full string) → count.
    const_reads_full: BTreeMap<String, usize>,
    /// Const LAST segment → count.
    const_reads_last: BTreeMap<String, usize>,
    /// Const last segment → array-length-position count.
    const_array_len: BTreeMap<String, usize>,
}

impl InteractionIndex {
    /// `(reads, array-len uses)` for a const path as written in a Default.
    fn const_reads_of(&self, as_written: &str) -> (usize, usize) {
        let last = as_written.rsplit("::").next().unwrap_or(as_written);
        let array_len = self.const_array_len.get(last).copied().unwrap_or(0);
        if as_written.contains("::") {
            // Qualified default: full-path matches only. An unqualified read
            // of the same last segment counts only when unambiguous (no
            // OTHER qualified read of that segment exists) — kills the
            // cross-type `::COUNT` collision.
            let full = self.const_reads_full.get(as_written).copied().unwrap_or(0);
            let has_other_qualified = self
                .const_reads_full
                .keys()
                .any(|k| k.rsplit("::").next() == Some(last) && k.as_str() != as_written);
            let unqualified = if has_other_qualified {
                0
            } else {
                self.const_reads_last.get(last).copied().unwrap_or(0)
            };
            (full + unqualified, array_len)
        } else {
            (
                self.const_reads_last.get(last).copied().unwrap_or(0),
                array_len,
            )
        }
    }
}

/// Pass 1: collect field candidates from `impl Default for T` blocks.
struct DefaultImplVisitor<'a> {
    file: String,
    out: &'a mut Vec<FieldCandidate>,
}

impl<'ast> Visit<'ast> for DefaultImplVisitor<'_> {
    fn visit_item_impl(&mut self, impl_block: &'ast syn::ItemImpl) {
        if !is_default_impl(impl_block) {
            syn::visit::visit_item_impl(self, impl_block);
            return;
        }
        let mut sub = StructLitVisitor {
            impl_ty: type_name(&impl_block.self_ty),
            file: self.file.clone(),
            out: self.out,
        };
        sub.visit_item_impl(impl_block);
    }
}

fn is_default_impl(impl_block: &syn::ItemImpl) -> bool {
    impl_block.trait_.as_ref().is_some_and(|(_, p, _)| {
        p.segments
            .last()
            .is_some_and(|s| s.ident == "Default")
    })
}

/// Walks one `impl Default for T`, recording struct-literal fields.
struct StructLitVisitor<'a> {
    impl_ty: String,
    file: String,
    out: &'a mut Vec<FieldCandidate>,
}

impl<'ast> Visit<'ast> for StructLitVisitor<'_> {
    fn visit_expr_struct(&mut self, lit: &'ast syn::ExprStruct) {
        if struct_lit_type(&lit.path).is_none_or(|t| t == self.impl_ty) {
            for fv in &lit.fields {
                let Some(field) = field_member_name(&fv.member) else {
                    continue;
                };
                self.out.push(FieldCandidate {
                    struct_name: self.impl_ty.clone(),
                    field_name: field,
                    const_path: const_path_of(&fv.expr),
                    init_token: init_token_of(&fv.expr),
                    origin: format!("{}:{}", self.file, fv.expr.span().start().line),
                });
            }
        }
        syn::visit::visit_expr_struct(self, lit);
    }
}

/// Pass 2: tally interactions. `suppress_const` is set inside `impl Default`
/// subtrees — the default's own const RHS is not an external consumer.
/// `defaults_by_field` carries the class-B comparator context. The C2
/// context (test / impl / validate-shaped method) reclassifies each READ
/// touch without touching the A/B `field_reads` semantics.
struct InteractionVisitor<'a> {
    ix: &'a mut InteractionIndex,
    suppress_const: bool,
    defaults_by_field: &'a DefaultsByField,
    /// File-level test heuristic (`tests/`/`benches/` path or `#![cfg(test)]`).
    file_is_test: bool,
    /// The PATH axis alone (`tests/`/`benches/` file) — feeds the
    /// `[C2·test-rig]` downgrade, distinct from cfg(test) confinement.
    file_in_tests_tree: bool,
    /// Depth inside `#[cfg(test)]` mods / `#[test]`-like fns.
    test_depth: usize,
    /// Self-type names of the enclosing impl blocks (innermost last).
    impl_stack: Vec<String>,
    /// Whether each enclosing impl block is a TRAIT impl (`impl Trait for
    /// T`), parallel to `impl_stack` (innermost last). Trait-impl reads are
    /// external — dispatchable through the trait (FP-class #3).
    impl_is_trait: Vec<bool>,
    /// Depth inside validate-shaped (`bool`/`Result`-returning) methods.
    validate_depth: usize,
}

/// One field-read touch, attributed for class C2: the A/B total always
/// increments; the test / own-impl tallies only inside their contexts.
/// `internal_impl` is `Some(T)` when the read sits in a validate-shaped
/// method of `impl T` (test context takes precedence). `tests_tree` is
/// the PATH axis alone (a `tests/`/`benches/` file) — kept separate from
/// `in_test` so a src/-declared rig (cfg(test)-confined) stays at full
/// [C2] while a tests-tree one downgrades to `[C2·test-rig]`.
fn tally_field_read(
    ix: &mut InteractionIndex,
    name: &str,
    in_test: bool,
    internal_impl: Option<&str>,
    tests_tree: bool,
) {
    *ix.field_reads.entry(name.to_string()).or_insert(0) += 1;
    if tests_tree {
        *ix
            .field_reads_tests_tree
            .entry(name.to_string())
            .or_insert(0) += 1;
    }
    if in_test {
        *ix.field_reads_tests.entry(name.to_string()).or_insert(0) += 1;
    } else if let Some(ty) = internal_impl {
        *ix
            .field_reads_in_impl
            .entry((name.to_string(), ty.to_string()))
            .or_insert(0) += 1;
    }
}

/// `#[cfg(test)]`-style attribute (path `cfg`, tokens containing `test` and
/// not `not` — `cfg(not(test))` is production-only, not test).
pub(crate) fn attr_is_cfg_test(attr: &syn::Attribute) -> bool {
    if let syn::Meta::List(list) = &attr.meta {
        let is_cfg = list
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "cfg");
        if is_cfg {
            let tok = list.tokens.to_string();
            return tok.contains("test") && !tok.contains("not");
        }
    }
    false
}

/// `#[test]` / `#[bench]` / `#[tokio::test]`-shaped attribute (any attr
/// whose LAST path segment is `test` or `bench`).
pub(crate) fn attr_is_test_fn(attr: &syn::Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|s| s.ident == "test" || s.ident == "bench")
}

pub(crate) fn attrs_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attr_is_cfg_test)
}

/// Any `#[test]`-like attribute on the item (shared with `platform_audit`).
pub(crate) fn attrs_test_fn(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attr_is_test_fn)
}

/// Cargo-layout heuristic: a file under `tests/` or `benches/` is test
/// context regardless of its items.
fn file_is_test_path(name: &str) -> bool {
    name.contains("/tests/")
        || name.contains("/benches/")
        || name.starts_with("tests/")
        || name.starts_with("benches/")
}

impl InteractionVisitor<'_> {
    /// The C2 attribution of the current position (test context wins over
    /// own-impl; own-impl requires a validate-shaped method frame INSIDE an
    /// INHERENT impl — a trait impl's reads are dispatchable through the
    /// trait, hence external). Returns OWNED data — callers pair it with a
    /// mutable `ix` borrow.
    fn read_ctx(&self) -> (bool, Option<String>) {
        let in_test = self.file_is_test || self.test_depth > 0;
        let innermost_is_trait = self.impl_is_trait.last().is_some_and(|b| *b);
        let internal_impl = if in_test {
            None
        } else if self.validate_depth > 0 && !innermost_is_trait {
            self.impl_stack.last().cloned()
        } else {
            None
        };
        (in_test, internal_impl)
    }

    fn enter_fn_ctx(&mut self, enter_test: bool, validate_shaped: bool) {
        if enter_test {
            self.test_depth += 1;
        } else if validate_shaped && !self.impl_stack.is_empty() {
            self.validate_depth += 1;
        }
    }

    fn exit_fn_ctx(&mut self, enter_test: bool, validate_shaped: bool) {
        if enter_test {
            self.test_depth -= 1;
        } else if validate_shaped && !self.impl_stack.is_empty() {
            self.validate_depth -= 1;
        }
    }
}

impl<'ast> Visit<'ast> for InteractionVisitor<'_> {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let enter_test = attrs_cfg_test(&item.attrs);
        if enter_test {
            self.test_depth += 1;
        }
        syn::visit::visit_item_mod(self, item);
        if enter_test {
            self.test_depth -= 1;
        }
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        let enter_test = item.attrs.iter().any(attr_is_test_fn);
        let validate_shaped = !enter_test && sig_returns_validate_shape(&item.sig);
        self.enter_fn_ctx(enter_test, validate_shaped);
        syn::visit::visit_item_fn(self, item);
        self.exit_fn_ctx(enter_test, validate_shaped);
    }

    /// Methods are `syn::ImplItem::Fn` in syn 2 — `visit_item_fn` never fires
    /// inside impl blocks, so the validate-shape context needs this hook too
    /// (the C2 probe caught the empty `field_reads_in_impl`).
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let enter_test = item.attrs.iter().any(attr_is_test_fn);
        let validate_shaped = !enter_test && sig_returns_validate_shape(&item.sig);
        self.enter_fn_ctx(enter_test, validate_shaped);
        syn::visit::visit_impl_item_fn(self, item);
        self.exit_fn_ctx(enter_test, validate_shaped);
    }

    fn visit_item_impl(&mut self, impl_block: &'ast syn::ItemImpl) {
        let is_default = is_default_impl(impl_block);
        let prev = self.suppress_const;
        if is_default {
            self.suppress_const = true;
        }
        self.impl_stack.push(type_name(&impl_block.self_ty));
        self.impl_is_trait.push(impl_block.trait_.is_some());
        syn::visit::visit_item_impl(self, impl_block);
        self.impl_stack.pop();
        self.impl_is_trait.pop();
        self.suppress_const = prev;
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        if !self.suppress_const {
            if let Expr::Path(p) = expr {
                if let Some(last_seg) = p.path.segments.last() {
                    let last = last_seg.ident.to_string();
                    if is_screaming(&last) {
                        let full = path_string(&p.path);
                        *self.ix.const_reads_full.entry(full).or_insert(0) += 1;
                        *self.ix.const_reads_last.entry(last).or_insert(0) += 1;
                    }
                }
            }
        }
        syn::visit::visit_expr(self, expr);
    }

    fn visit_expr_field(&mut self, f: &'ast syn::ExprField) {
        if let Some(name) = field_member_name(&f.member) {
            let (in_test, internal_impl) = self.read_ctx();
            let ix = &mut *self.ix;
            tally_field_read(
                ix,
                &name,
                in_test,
                internal_impl.as_deref(),
                self.file_in_tests_tree,
            );
        }
        syn::visit::visit_expr_field(self, f);
    }

    fn visit_pat_struct(&mut self, pat: &'ast syn::PatStruct) {
        let (in_test, internal_impl) = self.read_ctx();
        let tests_tree = self.file_in_tests_tree;
        let ix = &mut *self.ix;
        for pf in &pat.fields {
            if let Some(name) = field_member_name(&pf.member) {
                tally_field_read(ix, &name, in_test, internal_impl.as_deref(), tests_tree);
            }
        }
        syn::visit::visit_pat_struct(self, pat);
    }

    fn visit_type_array(&mut self, ta: &'ast syn::TypeArray) {
        if !self.suppress_const {
            if let Expr::Path(p) = &ta.len {
                if let Some(last_seg) = p.path.segments.last() {
                    let last = last_seg.ident.to_string();
                    if is_screaming(&last) {
                        *self.ix.const_array_len.entry(last).or_insert(0) += 1;
                    }
                }
            }
        }
        syn::visit::visit_type_array(self, ta);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        let is_assert = matches!(
            name.as_str(),
            "assert" | "assert_eq" | "assert_ne" | "debug_assert" | "debug_assert_eq" | "debug_assert_ne"
        );
        let is_ne = name.ends_with("_ne");
        let suppress_const = self.suppress_const;
        let (in_test, internal_impl) = self.read_ctx();
        let tests_tree = self.file_in_tests_tree;
        let defaults = self.defaults_by_field;
        let ix = &mut *self.ix;
        let mut sink = |touch: TokenTouch| match touch {
            TokenTouch::Field {
                ident,
                comparator,
                negated,
            } => {
                if !is_assert {
                    // A field reference inside any other macro is a read.
                    tally_field_read(ix, &ident, in_test, internal_impl.as_deref(), tests_tree);
                    return;
                }
                // Assert family: default-equality (per the intake spec —
                // "an equality/boolean assertion against the literal
                // appearing in that type's Default impl") is the class-B
                // touch; anything else is a BEHAVIORAL assert and counts
                // as a read (the intake: the catching test is behavioral).
                // `assert_ne` against the default proves non-default —
                // behavioral either way.
                let is_default_eq = !is_ne
                    && defaults.get(&ident).is_some_and(|reprs| {
                        reprs.iter().any(|(const_last, init_token)| {
                            match (comparator.as_deref(), init_token.as_deref()) {
                                // assert_eq!(c.f, <init token/const>)
                                (Some(c), Some(t)) => c == t,
                                (Some(c), None) => {
                                    const_last.as_deref() == Some(c)
                                }
                                // assert!(c.f) / assert!(!c.f): truthiness —
                                // default-eq iff the bool default agrees.
                                (None, Some(t)) => {
                                    (t == "true" && !negated) || (t == "false" && negated)
                                }
                                (None, None) => false,
                            }
                        })
                    });
                if is_default_eq {
                    *ix.field_asserts.entry(ident).or_insert(0) += 1;
                } else {
                    tally_field_read(ix, &ident, in_test, internal_impl.as_deref(), tests_tree);
                }
            }
            TokenTouch::Const(ident) => {
                if !suppress_const {
                    *ix.const_reads_last.entry(ident).or_insert(0) += 1;
                }
            }
        };
        scan_macro_tokens(&mac.tokens, &mut sink);
        // Macro bodies are token-scanned; do not recurse (unparsed).
    }
}

/// A token-level touch found inside a macro body.
enum TokenTouch {
    /// `.ident` — a field reference, with the assert-eq comparator (the
    /// token after the following comma, `None` for the bool shape) and
    /// whether the receiver was negated (`assert!(!c.f)`).
    Field {
        ident: String,
        comparator: Option<String>,
        negated: bool,
    },
    /// A bare SCREAMING ident (a const reference in macro tokens).
    Const(String),
}

/// Token scan of a macro body: `.ident` pairs (guarding against range
/// operators `a..b` / `a..=b`) and bare SCREAMING idents. The callback is
/// dyn (not generic) — group recursion must not mint new closure types.
fn scan_macro_tokens(tokens: &proc_macro2::TokenStream, f: &mut dyn FnMut(TokenTouch)) {
    use proc_macro2::TokenTree;
    let trees: Vec<TokenTree> = tokens.clone().into_iter().collect();
    let mut prev_is_dot = false;
    let mut i = 0;
    while i < trees.len() {
        let tree = &trees[i];
        match tree {
            TokenTree::Punct(p) => {
                prev_is_dot = p.as_char() == '.';
            }
            TokenTree::Ident(id) => {
                let s = id.to_string();
                if prev_is_dot {
                    let (comparator, negated) = assert_shape(&trees, i);
                    f(TokenTouch::Field {
                        ident: s,
                        comparator,
                        negated,
                    });
                } else if is_screaming(&s) {
                    f(TokenTouch::Const(s));
                }
                prev_is_dot = false;
            }
            TokenTree::Group(g) => {
                prev_is_dot = false;
                scan_macro_tokens(&g.stream(), f);
            }
            TokenTree::Literal(_) => {
                prev_is_dot = false;
            }
        }
        i += 1;
    }
}

/// For `.field` at `trees[i]` (the ident after the dot): the assert-eq
/// comparator — the token after the following comma — and whether the
/// receiver was negated (`assert!(!c.f)`; `!` sits before the receiver
/// ident at `i - 2`).
fn assert_shape(trees: &[proc_macro2::TokenTree], i: usize) -> (Option<String>, bool) {
    use proc_macro2::TokenTree;
    let comparator = trees.iter().skip(i + 1).find_map(|t| match t {
        TokenTree::Punct(p) if p.as_char() == ',' => None,
        other => Some(other.to_string()),
    });
    // Only treat it as a comparator when a comma PRECEDES it (the eq-family
    // shape); otherwise it is just the next expression token (bool shape).
    let had_comma = trees[i + 1..]
        .iter()
        .take_while(|t| !matches!(t, TokenTree::Group(_)))
        .any(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ','));
    let comparator = if had_comma { comparator } else { None };
    // `assert!(!c.f)`: `!` sits three tokens back (`!` recv `.` field).
    let negated = i >= 3
        && matches!(&trees[i - 3], TokenTree::Punct(p) if p.as_char() == '!')
        && matches!(&trees[i - 2], TokenTree::Ident(_))
        && matches!(&trees[i - 1], TokenTree::Punct(p) if p.as_char() == '.');
    (comparator, negated)
}

#[must_use]
pub fn field_member_name(member: &syn::Member) -> Option<String> {
    match member {
        syn::Member::Named(ident) => Some(ident.to_string()),
        syn::Member::Unnamed(_) => None,
    }
}

/// A bare path RHS naming a const: LAST segment `SCREAMING_CASE`; leading
/// segments (if any) type-ish (start uppercase) — `AUDIT_RING_K`,
/// `LoraTarget::COUNT`. Calls/methods/literals are not.
fn const_path_of(expr: &Expr) -> Option<String> {
    let Expr::Path(p) = expr else {
        return None;
    };
    let segs = &p.path.segments;
    let last = segs.last()?;
    if !is_screaming(&last.ident.to_string()) {
        return None;
    }
    if segs.len() > 1 {
        let leading_ok = segs.iter().rev().skip(1).all(|s| {
            s.ident
                .to_string()
                .chars()
                .next()
                .is_some_and(char::is_uppercase)
        });
        if !leading_ok {
            return None;
        }
    }
    Some(path_string(&p.path))
}

/// Token text of a LITERAL (`0`, `true`, `9`) or simple-ident (`None`)
/// Default init — the class-B default-equality comparator. Calls
/// (`Some(0)`, `all().to_vec()`) and paths yield `None`.
fn init_token_of(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(l) => Some(l.lit.to_token_stream().to_string()),
        Expr::Path(p) if p.path.segments.len() == 1 && p.attrs.is_empty() => {
            let s = p.path.segments[0].ident.to_string();
            // Only keyword-ish single idents (`None`); SCREAMING ones are
            // const paths (already captured by `const_path_of`).
            (!is_screaming(&s)).then_some(s)
        }
        _ => None,
    }
}

/// `SCREAMING_CASE`: non-empty, only uppercase/underscore/digits, ≥1 alphabetic.
fn is_screaming(seg: &str) -> bool {
    !seg.is_empty()
        && seg.chars().any(char::is_alphabetic)
        && seg.chars().all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
}

fn path_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn struct_lit_type(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .filter(|s| s != "Self")
}

fn type_name(ty: &syn::Type) -> String {
    if let syn::Type::Path(tp) = ty {
        return tp
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
    }
    String::new()
}

/// Validate-shaped signature output: `bool` or a `Result` (incl.
/// `fmt::Result`) — reads inside such methods of `impl T` are T-internal
/// (checks that consume the value without exposing it). Any other return
/// type (accessors, builders, `Default::default`) is FLOW-OUT — the value
/// leaves through the method, so the read stays external. Shared by free
/// fns (`ItemFn`) and methods (`ImplItemMethod`).
///
/// `Result` is split by PAYLOAD (the `SchurSolver` class, riir-train 546's
/// hand-verified FP): `Result<(), E>` is a check (unit payload — nothing
/// derived from the fields can leave); `Result<Payload, E>` is COMPUTATION
/// — the payload carries the fields' influence out through the return
/// type (a solver's `solve(..) -> Result<Array1<f32>, E>` reads its
/// tolerance knobs to PRODUCE the answer). Bare undecidable `Result`
/// (non-angle-bracketed, or a non-type first arg) reads conservatively as
/// a check (validate-shaped) — the quiet direction.
fn sig_returns_validate_shape(sig: &syn::Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };
    let syn::Type::Path(tp) = ty.as_ref() else {
        return false;
    };
    let Some(seg) = tp.path.segments.last() else {
        return false;
    };
    match seg.ident.to_string().as_str() {
        "bool" => true,
        "Result" => result_payload_is_unit(seg),
        _ => false,
    }
}

/// The `Result` payload test: `Result<(), E>` (empty-tuple Ok type) is a
/// unit check; any other payload type flows the field influence out.
fn result_payload_is_unit(seg: &syn::PathSegment) -> bool {
    let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
        return true; // undecidable payload — conservative (check)
    };
    match args.args.first() {
        Some(syn::GenericArgument::Type(syn::Type::Tuple(t))) => t.elems.is_empty(),
        Some(syn::GenericArgument::Type(_)) => false, // real payload — flow-out
        _ => true, // undecidable — conservative (check)
    }
}

impl ConfigAuditReport {
    /// Render the human report (what `--config-audit` prints).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "config-audit: {} file(s) parsed, {} Default-impl field candidate(s): {} inert knob(s) [A], {} write-only knob(s) [A·documented-write-only], {} assert-only field(s) [B], {} documented-write-only (annotation), {} gpu-uniform (annotation), {} self-referential config struct(s) [C2]",
            self.files_scanned,
            self.candidates,
            self.inert_knobs.len(),
            self.documented_write_only_knobs.len(),
            self.assert_only.len(),
            self.documented_write_only.len(),
            self.gpu_uniform.len(),
            self.self_referential.len()
        );
        let _ = writeln!(out, "{}", self.walk.note());
        for k in &self.inert_knobs {
            let _ = writeln!(
                out,
                "  [A] {} — {}.{} defaults to const {} (field read 0x, const read {}x)",
                k.candidate.origin,
                k.candidate.struct_name,
                k.candidate.field_name,
                k.candidate.const_path.as_deref().unwrap_or("?"),
                k.const_reads
            );
            if k.array_len_uses > 0 {
                let _ = writeln!(
                    out,
                    "      note: {} array-length use(s) of the const — the field is structurally un-honorable (wiring it needs array-type changes)",
                    k.array_len_uses
                );
            }
            let _ = writeln!(
                out,
                "      fix: wire consumers to the field (or document the const as the contract)"
            );
        }
        for k in &self.documented_write_only_knobs {
            render_write_only_knob_row(&mut out, k);
        }
        for k in &self.gpu_uniform {
            render_gpu_uniform_row(&mut out, k);
        }
        for b in &self.assert_only {
            let _ = writeln!(
                out,
                "  [B] {} — {}.{} only ever asserted ({} assert touch(es), 0 reads)",
                b.candidate.origin,
                b.candidate.struct_name,
                b.candidate.field_name,
                b.assert_interactions
            );
            let _ = writeln!(
                out,
                "      fix: replace the default-equality assert with a behavioral one"
            );
        }
        for b in &self.documented_write_only {
            let _ = writeln!(
                out,
                "  [documented-write-only] {} — {}.{} only ever asserted ({} assert touch(es), 0 reads) but the FIELD doc carries a deliberate-write-only marker (weak severity — the doc's claim is not verified; if it is NOT deliberate, drop the marker wording)",
                b.candidate.origin,
                b.candidate.struct_name,
                b.candidate.field_name,
                b.assert_interactions
            );
        }
        for s in &self.self_referential {
            render_self_referential(&mut out, s);
        }
        let _ = writeln!(
            out,
            "scope: detector sees only the scanned set; consumers in sibling/upstream crates are under-reported; struct-literal evidence is per-file scope-resolved (Issue 109) and additionally skips a literal naming a field that NO local same-named top-level struct declares — provably a foreign type under the same name (the foreign-glob + local-glob shape is undecidable by import shape alone)"
        );
        out
    }
}

/// One `[A·documented-write-only]` row (P29): the class-A fact plus the
/// unverified deliberate-write-only marker and its fix line.
fn render_write_only_knob_row(out: &mut String, k: &InertKnob) {
    let _ = writeln!(
        out,
        "  [A·documented-write-only] {} — {}.{} defaults to const {} (field read 0x, const read {}x) but the FIELD doc carries a deliberate-write-only marker (weak severity — the knob is documented as intentionally not consulted; the doc's claim is not verified)",
        k.candidate.origin,
        k.candidate.struct_name,
        k.candidate.field_name,
        k.candidate.const_path.as_deref().unwrap_or("?"),
        k.const_reads
    );
    if k.array_len_uses > 0 {
        let _ = writeln!(
            out,
            "      note: {} array-length use(s) of the const — wiring the field needs array-type changes",
            k.array_len_uses
        );
    }
    let _ = writeln!(
        out,
        "      fix: keep only if the surface is genuinely deliberate (future dispatcher); otherwise wire consumers to the field or drop the marker"
    );
}

/// One `[gpu-uniform]` row (P28): the class-A fact plus the
/// shader-consumption mechanism and its confirm-the-WGSL fix line.
fn render_gpu_uniform_row(out: &mut String, k: &InertKnob) {
    let _ = writeln!(
        out,
        "  [gpu-uniform] {} — {}.{} defaults to const {} (field read 0x, const read {}x) but {} rides a #[uniform(..)] bind — the shader consumes the block's bytes without a Rust read (weak severity: WGSL is not parsed, a declared-but-unread shader field would be schema-dead weight — if the shader does not read it, treat as [A])",
        k.candidate.origin,
        k.candidate.struct_name,
        k.candidate.field_name,
        k.candidate.const_path.as_deref().unwrap_or("?"),
        k.const_reads,
        k.candidate.struct_name
    );
    let _ = writeln!(
        out,
        "      fix: confirm the WGSL reads the field and leave it, or delete the knob"
    );
}

/// One [C2] row (or its `[C2·test-rig]` / `[C2·gpu-uniform]` weak
/// downgrades) with its per-field tally and fix line.
fn render_self_referential(out: &mut String, s: &SelfReferentialConfig) {
    if s.test_rig {
        let _ = writeln!(
            out,
            "  [C2·test-rig] {} — {} ({} Default-impl field(s)) has ZERO external Rust reads and lives wholly in the tests tree (declared + read under tests/-path files; weak severity — the C2 class targets PRODUCTION config structs whose consumer never materialized. If the rig is dead, delete the file)",
            s.origin,
            s.struct_name,
            s.fields.len()
        );
        for f in &s.fields {
            let _ = writeln!(
                out,
                "      .{} — {} read(s): {} test, {} own-validate-impl",
                f.field_name, f.total_reads, f.test_reads, f.self_impl_reads
            );
        }
        let _ = writeln!(
            out,
            "      fix: prune the rig with its test, or plumb a real consumer if the struct was meant to graduate"
        );
        return;
    }
    if s.gpu_uniform {
        let _ = writeln!(
            out,
            "  [C2·gpu-uniform] {} — {} ({} Default-impl field(s)) has ZERO external Rust reads, but the struct rides a #[uniform(..)] bind (weak severity — the shader consumes the block's bytes; WGSL is not parsed)",
            s.origin,
            s.struct_name,
            s.fields.len()
        );
        for f in &s.fields {
            let _ = writeln!(
                out,
                "      .{} — {} read(s): {} test, {} own-validate-impl",
                f.field_name, f.total_reads, f.test_reads, f.self_impl_reads
            );
        }
        let _ = writeln!(
            out,
            "      fix: confirm the WGSL reads the fields; do not delete on this row alone"
        );
        return;
    }
    let _ = writeln!(
        out,
        "  [C2] {} — {} ({} Default-impl field(s)) has ZERO external reads",
        s.origin,
        s.struct_name,
        s.fields.len()
    );
    for f in &s.fields {
        let _ = writeln!(
            out,
            "      .{} — {} read(s): {} test, {} own-validate-impl",
            f.field_name, f.total_reads, f.test_reads, f.self_impl_reads
        );
    }
    let _ = writeln!(
        out,
        "      fix: delete the struct, or plumb a real consumer (reads inside other types' impls / accessors keep it live)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(src: &str) -> ConfigAuditReport {
        audit_sources(&[("lib.rs".to_string(), src.to_string())])
    }

    #[test]
    fn inert_knob_fires() {
        let r = one(
            r"
pub const AUDIT_RING_K: usize = 64;
pub struct AuditConfig { pub ring_capacity: usize, pub live: usize }
impl Default for AuditConfig {
    fn default() -> Self { Self { ring_capacity: AUDIT_RING_K, live: AUDIT_RING_K } }
}
pub fn bound(_c: &AuditConfig) -> usize { AUDIT_RING_K }
pub fn live(c: &AuditConfig) -> usize { c.live }
",
        );
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
        let r = one(
            r"
pub const RING: usize = 64;
#[derive(Clone, Copy)]
pub struct Params { pub ring: usize, pub live: usize }
impl Default for Params { fn default() -> Self { Self { ring: RING, live: RING } } }
pub struct Material { #[uniform(0)] pub params: Params }
pub fn bind(_m: &Material) -> usize { RING }
",
        );
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
        let r = one(
            r"
pub const INNER_K: usize = 8;
#[derive(Clone, Copy)]
pub struct Inner { pub k: usize }
impl Default for Inner { fn default() -> Self { Self { k: INNER_K } } }
#[derive(Clone, Copy)]
pub struct Params { pub inner: Inner }
impl Default for Params { fn default() -> Self { Self { inner: Inner::default() } } }
pub struct Material { #[uniform(0)] pub params: Params }
pub fn ext() -> usize { INNER_K }
",
        );
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
        let r = one(
            r"
pub const RING: usize = 64;
#[derive(Clone, Copy)]
pub struct Params { pub ring: usize }
impl Default for Params { fn default() -> Self { Self { ring: RING } } }
pub struct Material { pub params: Params }
pub fn bind(_m: &Material) -> usize { RING }
",
        );
        assert_eq!(r.inert_knobs.len(), 1);
        assert!(r.gpu_uniform.is_empty());
    }

    #[test]
    fn gpu_uniform_c2_downgrades_to_weak_annotation() {
        let r = one(
            r"
#[derive(Clone, Copy)]
pub struct Params { pub a: u32, pub b: u32 }
impl Default for Params { fn default() -> Self { Self { a: 1, b: 2 } } }
pub struct Material { #[uniform(0)] pub params: Params }
",
        );
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
        let r = audit_sources(&[
            (
                "tests/rig.rs".to_string(),
                r#"
pub struct Rig { pub a: f32, pub b: f32 }
impl Default for Rig { fn default() -> Self { Self { a: 1.0, b: 2.0 } } }
fn probe(r: &Rig) -> f32 { if r.a > r.b { r.a } else { r.b } }
"#
                .to_string(),
            ),
        ]);
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
        let r = audit_sources(&[
            (
                "src/lib.rs".to_string(),
                r#"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 1.0 } } }
#[cfg(test)]
mod tests {
    fn probe(p: &super::P) -> f32 { p.a }
}
"#
                .to_string(),
            ),
        ]);
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
                r#"
pub struct Rig { pub done: bool }
impl Default for Rig { fn default() -> Self { Self { done: false } } }
fn probe(r: &Rig) -> bool { r.done }
"#
                .to_string(),
            ),
            (
                "src/lib.rs".to_string(),
                r#"
pub struct Other { pub done: bool }
#[cfg(test)]
mod t {
    fn probe(o: &super::Other) -> bool { o.done }
}
"#
                .to_string(),
            ),
        ]);
        assert_eq!(r.self_referential.len(), 1);
        assert!(!r.self_referential[0].test_rig);
        assert!(!r.render().contains("[C2·test-rig]"));
    }

    #[test]
    fn live_via_field_silent() {
        let r = one(
            r"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
pub fn use_it(c: &C) -> usize { c.f }
",
        );
        assert!(r.inert_knobs.is_empty());
        assert!(r.assert_only.is_empty());
    }

    #[test]
    fn live_via_destructuring_silent_the_blind_spot_guard() {
        let r = one(
            r"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
pub fn destructure(c: &C) -> usize { let C { f, .. } = c; f }
pub fn also_const() -> usize { K }
",
        );
        assert!(r.inert_knobs.is_empty(), "destructuring counts as a read");
    }

    #[test]
    fn live_via_macro_silent() {
        let r = one(
            r#"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
pub fn show(c: &C) { println!("{} {}", c.f, K); }
"#,
        );
        assert!(r.inert_knobs.is_empty(), "field read inside non-assert macro counts as a read");
    }

    #[test]
    fn array_length_use_fires_with_note() {
        let r = one(
            r"
pub const RING_K: usize = 8;
pub struct C { pub ring: usize }
impl Default for C { fn default() -> Self { Self { ring: RING_K } } }
pub struct Entry { pub v: u32 }
pub fn slot(_c: &C) -> [Entry; RING_K] { [Entry { v: 0 }; RING_K] }
",
        );
        assert_eq!(r.inert_knobs.len(), 1);
        assert!(r.inert_knobs[0].array_len_uses >= 1, "array-length use detected");
        assert!(r.render().contains("structurally un-honorable"));
    }

    #[test]
    fn assert_only_fires_class_b() {
        let r = one(
            r"
pub struct C { pub sc: bool }
impl Default for C { fn default() -> Self { Self { sc: true } } }
pub fn consume(_c: &C) -> u32 { 1 }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn fake_coverage() { let c = C::default(); assert!(c.sc); }
}
",
        );
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
        let r = one(
            r"
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
",
        );
        assert!(r.assert_only.is_empty(), "marked row downgrades out of [B]");
        assert_eq!(r.documented_write_only.len(), 1);
        let rendered = r.render();
        assert!(rendered.contains("[documented-write-only]"));
        assert!(rendered.contains("0 assert-only field(s) [B]"), "header [B] count is zero");
    }

    /// The Issue-147 field-doc wording ("GOAT-gate territory") — the
    /// vocabulary the canonical specimen's field doc carries TODAY.
    #[test]
    fn goat_gate_territory_wording_downgrades() {
        let r = one(
            r"
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
",
        );
        assert!(r.assert_only.is_empty());
        assert_eq!(r.documented_write_only.len(), 1, "the Issue-147 wording downgrades");
    }

    /// The P29 class-A extension (the `HlaKarcConfig.delay_k` shape): a
    /// const-defaulted knob whose FIELD doc carries a deliberate-write-only
    /// marker reports at the weak `[A·documented-write-only]` annotation
    /// instead of deletion-grade [A].
    #[test]
    fn documented_write_only_marker_downgrades_class_a() {
        let r = one(
            r"
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
",
        );
        assert!(r.inert_knobs.is_empty(), "marked row downgrades out of [A]");
        assert_eq!(r.documented_write_only_knobs.len(), 1);
        assert_eq!(r.documented_write_only_knobs[0].candidate.field_name, "delay_k");
        let rendered = r.render();
        assert!(rendered.contains("[A·documented-write-only]"));
        assert!(rendered.contains("0 inert knob(s) [A]"), "header [A] count is zero");
    }

    /// Precedence: a gpu-bound struct's const-inited field with a marker doc
    /// stays in the P28 gpu-uniform bucket — the shader-consumption mechanism
    /// is the more specific annotation.
    #[test]
    fn gpu_uniform_takes_precedence_over_write_only_marker() {
        let r = one(
            r"
pub const RING: usize = 64;
#[derive(Clone, Copy)]
pub struct Params {
    /// Deliberate write-only forward surface.
    pub ring: usize,
}
impl Default for Params { fn default() -> Self { Self { ring: RING } } }
pub struct Material { #[uniform(0)] pub params: Params }
pub fn bind(_m: &Material) -> usize { RING }
",
        );
        assert!(r.inert_knobs.is_empty());
        assert!(r.documented_write_only_knobs.is_empty(), "gpu-uniform wins");
        assert_eq!(r.gpu_uniform.len(), 1);
    }

    #[test]
    fn canonical_token_downgrades() {
        let r = one(
            r"
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
",
        );
        assert!(r.assert_only.is_empty());
        assert_eq!(r.documented_write_only.len(), 1);
    }

    /// The marker counts only on the FIELD doc — a struct-level doc speaks
    /// for the whole type, not for one write-only member.
    #[test]
    fn struct_doc_marker_does_not_downgrade_field() {
        let r = one(
            r"
/// Deliberate forward surface struct.
pub struct V { pub f: bool }
impl Default for V { fn default() -> Self { Self { f: true } } }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn pin() { let v = V::default(); assert!(v.f); }
}
",
        );
        assert_eq!(r.assert_only.len(), 1, "struct doc does not speak for one field");
        assert!(r.documented_write_only.is_empty());
    }

    /// A doc WITHOUT any marker vocabulary is just documentation — the row
    /// stays a class-B finding.
    #[test]
    fn plain_doc_stays_class_b() {
        let r = one(
            r"
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
",
        );
        assert_eq!(r.assert_only.len(), 1, "unmarked doc stays [B]");
        assert!(r.documented_write_only.is_empty());
    }

    #[test]
    fn class_a_precedence_over_b() {
        let r = one(
            r"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn a() { let c = C::default(); assert_eq!(c.f, K); }
}
",
        );
        assert_eq!(r.inert_knobs.len(), 1, "A wins over B (K is read inside the assert macro tokens)");
        assert!(r.assert_only.is_empty());
    }

    #[test]
    fn non_const_default_silent() {
        let r = one(
            r"
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: 7 } } }
pub fn x() -> usize { 7 }
",
        );
        assert!(r.inert_knobs.is_empty());
        assert!(r.assert_only.is_empty(), "no asserts, no const path");
    }

    #[test]
    fn default_own_const_read_does_not_satisfy_condition_3() {
        let r = one(
            r"
pub const K: usize = 4;
pub struct C { pub f: usize }
impl Default for C { fn default() -> Self { Self { f: K } } }
pub fn consume(_c: &C) -> u32 { 1 }
",
        );
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
        assert_eq!(r.inert_knobs.len(), 1, "const read in another file satisfies condition 3");
        assert_eq!(r.files_scanned, 2);
    }

    #[test]
    fn qualified_default_no_cross_type_collision() {
        let r = one(
            r"
pub struct A { pub n: usize }
pub struct B { pub n: usize }
impl Default for A { fn default() -> Self { Self { n: B::COUNT } } }
impl Default for B { fn default() -> Self { Self { n: 1 } } }
impl B { pub const COUNT: usize = 2; }
pub fn other() -> usize { A::COUNT }
",
        );
        // A::COUNT is read but is NOT B::COUNT — the qualified default must
        // not fire off a different type's const.
        assert!(r.inert_knobs.is_empty(), "no cross-type ::COUNT collision");
    }

    #[test]
    fn struct_update_silent() {
        let r = one(
            r"
pub const K: usize = 4;
pub struct C { pub f: usize, pub g: usize }
impl Default for C { fn default() -> Self { Self { f: K, g: 1 } } }
pub fn mk(base: C) -> C { C { g: 2, ..base } }
",
        );
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
        let r = one(
            r"
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
",
        );
        assert!(r.assert_only.is_empty(), "behavioral asserts (computed expectations) are reads");
        assert!(r.inert_knobs.is_empty());
    }

    #[test]
    fn none_comparator_matches_none_default() {
        // assert_eq!(v.redirect, None) with a `None` default IS default-
        // equality — the riir-chain redirect row (accurate B).
        let r = one(
            r"
pub struct V { pub redirect: Option<u8> }
impl Default for V { fn default() -> Self { Self { redirect: None } } }
pub fn make() -> V { V::default() }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn a() { let v = make(); assert_eq!(v.redirect, None); }
}
",
        );
        assert_eq!(r.assert_only.len(), 1, "None-comparator against None default is class B");
        assert_eq!(r.assert_only[0].candidate.field_name, "redirect");
    }

    #[test]
    fn negated_bool_shape_tautological_fires() {
        let r = one(
            r"
pub struct C { pub off: bool }
impl Default for C { fn default() -> Self { Self { off: false } } }
#[cfg(test)]
mod t {
    use super::*;
    #[test]
    fn a() { let c = C::default(); assert!(!c.off); }
}
",
        );
        assert_eq!(r.assert_only.len(), 1, "assert!(!c.off) with default false is tautological");
    }

    // ── Class C2 — the self-referential config struct (Plan 125) ──

    #[test]
    fn self_referential_validate_plus_tests_fires_c2() {
        // The PlasmaConfig shape: own validate() reads + test reads, nothing
        // external. Classes A/B are blind here (reads > 0).
        let r = one(
            r"
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
",
        );
        assert_eq!(r.self_referential.len(), 1, "confined reads fire C2");
        assert_eq!(r.self_referential[0].struct_name, "P");
        assert_eq!(r.self_referential[0].fields.len(), 2);
        assert!(r.render().contains("[C2]"));
    }

    #[test]
    fn never_plumbed_literal_default_fires_c2() {
        // Zero reads anywhere, literal init — A needs a const, B needs an
        // assert; only C2 sees the dead struct.
        let r = one(
            r"
pub struct Dead { pub x: u32 }
impl Default for Dead { fn default() -> Self { Self { x: 3 } } }
pub fn make() -> Dead { Dead::default() }
",
        );
        assert_eq!(r.self_referential.len(), 1);
        assert!(r.inert_knobs.is_empty() && r.assert_only.is_empty());
    }

    #[test]
    fn external_read_suppresses_c2() {
        let r = one(
            r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
impl P { pub fn validate(&self) -> bool { self.a > 0.0 } }
pub fn consume(c: &P) -> f32 { c.a * 2.0 }
",
        );
        assert!(r.self_referential.is_empty(), "one external read keeps the struct live");
    }

    #[test]
    fn other_types_impl_reads_are_external() {
        // A read inside impl Engine (bool-returning, validate-SHAPED even) is
        // an EXTERNAL consumer of P — the impl-type check precedes the
        // method-shape check.
        let r = one(
            r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
pub struct Engine { pub p: P }
impl Engine {
    fn enabled(&self) -> bool { self.p.a > 0.0 }
}
",
        );
        assert!(r.self_referential.is_empty(), "impl Engine reading .a is external for P");
    }

    #[test]
    fn trait_impl_reads_are_external_no_c2() {
        // FP-class #3 (2026-09-14): reads inside `impl Display for X` are
        // externally dispatchable — the impl'd trait IS the external
        // contract. Pre-fix these counted as validate-shaped internal reads
        // (`fmt::Result`'s last path segment is `Result`) and fired C2 —
        // the katgpt-rs 772 three-FP class, probe-reproduced at HEAD
        // (ColinearityBatchGate / RuleBasedVerifier / EntropyConflictDetector).
        let r = one(
            r#"
pub struct Gate { pub threshold: f32 }
impl Default for Gate { fn default() -> Self { Self { threshold: 0.5 } } }
impl core::fmt::Display for Gate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.threshold)
    }
}
"#,
        );
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
        let r = one(
            r"
pub trait Verifier { fn accepts(&self) -> bool; }
pub struct RuleBased { pub limit: u32 }
impl Default for RuleBased { fn default() -> Self { Self { limit: 8 } } }
impl Verifier for RuleBased {
    fn accepts(&self) -> bool { self.limit > 0 }
}
",
        );
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
        let r = one(
            r#"
pub struct Solver { pub eps_reg: f32 }
impl Default for Solver { fn default() -> Self { Self { eps_reg: 1e-8 } } }
impl Solver {
    pub fn solve(&self, n: usize) -> Result<Vec<f32>, String> {
        if self.eps_reg > 0.0 { Ok(vec![0.0; n]) } else { Err("bad".into()) }
    }
}
"#,
        );
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
        let r = one(
            r#"
pub struct Chk { pub tol: f32 }
impl Default for Chk { fn default() -> Self { Self { tol: 0.5 } } }
impl Chk {
    pub fn check(&self) -> Result<(), String> {
        if self.tol < 0.0 { Err("neg".into()) } else { Ok(()) }
    }
}
"#,
        );
        assert_eq!(r.self_referential.len(), 1, "unit-Result checks stay internal");
    }

    #[test]
    fn inherent_validate_still_fires_alongside_trait_impl() {
        // The boundary discipline: an INHERENT validate() keeps counting as
        // internal when it is the ONLY confinement left (the C2 canonical
        // shape unchanged by the refinement).
        let r = one(
            r"
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
",
        );
        assert_eq!(r.self_referential.len(), 1, "inherent-only confinement still fires C2");
    }

    #[test]
    fn accessor_method_is_flow_out_not_internal() {
        // `fn a(&self) -> f32 { self.a }` exposes the value — the field IS
        // honored through the accessor; only validate-shaped returns confine.
        let r = one(
            r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
impl P { pub fn a(&self) -> f32 { self.a } }
",
        );
        assert!(r.self_referential.is_empty(), "accessor return = flow-out = external");
    }

    #[test]
    fn external_destructuring_suppresses_c2() {
        let r = one(
            r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
impl P { pub fn validate(&self) -> bool { self.a > 0.0 } }
pub fn take(c: &P) -> f32 { let P { a, .. } = c; a }
",
        );
        assert!(r.self_referential.is_empty(), "external destructuring reads the fields");
    }

    #[test]
    fn same_field_name_masks_c2_false_negative_direction() {
        // Name-matching: an unrelated struct's same-named field read
        // externally suppresses T's finding — the documented under-reporting
        // direction (never a false positive).
        let r = one(
            r"
pub struct P { pub a: f32 }
impl Default for P { fn default() -> Self { Self { a: 0.5 } } }
pub struct Other { pub a: f32 }
impl Default for Other { fn default() -> Self { Self { a: 1.0 } } }
pub fn consume(o: &Other) -> f32 { o.a }
",
        );
        assert!(r.self_referential.is_empty(), "name-masking = false negative, pinned");
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
        assert_eq!(r.self_referential.len(), 1, "tests/-file reads are test-context");
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
        let here = std::env::temp_dir().join("cfgaudit_p25_exists.rs");
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
        assert!(names[0].ends_with("cfgaudit_p25_exists.rs"));
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
        assert!(Command::new("git")
            .arg("-C")
            .arg(&tmp)
            .args(["add", "tracked.rs", "tbi.rs"])
            .output()
            .is_ok_and(|o| o.status.success()));
        fs::write(tmp.join(".gitignore"), "ignored/\n*.gen.rs\ntbi.rs\n").unwrap();
        let walk = read_sources_walk(std::slice::from_ref(&tmp));
        assert_eq!(walk.mode, WalkMode::Git { roots: 1 });
        let names: Vec<&str> = walk.sources.iter().map(|(f, _)| f.as_str()).collect();
        assert!(names.iter().any(|f| f.contains("tracked.rs")));
        assert!(names.iter().any(|f| f.contains("wip.rs")), "untracked non-ignored WIP kept");
        assert!(names.iter().any(|f| f.contains("tbi.rs")), "tracked-but-later-ignored kept");
        assert!(names.iter().all(|f| !f.contains("ignored/")), "gitignored dir excluded");
        assert!(names.iter().all(|f| !f.contains("y.gen.rs")), "gitignored glob excluded");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn nested_foreign_repo_is_not_scanned() {
        let tmp = tmp_repo("nested");
        fs::write(tmp.join("own.rs"), "pub fn o() {}").unwrap();
        let nested = tmp.join("nested_repo");
        fs::create_dir_all(&nested).unwrap();
        assert!(Command::new("git")
            .arg("-C")
            .arg(&nested)
            .args(["init", "-q"])
            .output()
            .is_ok_and(|o| o.status.success()));
        fs::write(nested.join("foreign.rs"), "pub fn f() {}").unwrap();
        let walk = read_sources_walk(std::slice::from_ref(&tmp));
        assert_eq!(walk.mode, WalkMode::Git { roots: 1 });
        let names: Vec<&str> = walk.sources.iter().map(|(f, _)| f.as_str()).collect();
        assert!(names.iter().any(|f| f.contains("own.rs")));
        assert!(names.iter().all(|f| !f.contains("foreign.rs")), "nested repo's files not ours");
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
        assert!(names.iter().all(|f| !f.contains("top.rs")), "cwd-limited listing, not repo-wide");
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
        assert_eq!(walk.mode, WalkMode::Plain, "explicit file(s) only — no dir roots");
        assert_eq!(walk.sources.len(), 1, "caller intent wins over the ignore rule");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn entirely_ignored_dir_root_falls_back_to_intent() {
        let tmp = tmp_repo("igdir");
        fs::write(tmp.join(".gitignore"), "corpus/\n").unwrap();
        fs::create_dir_all(tmp.join("corpus")).unwrap();
        fs::write(tmp.join("corpus/x.rs"), "pub fn x() {}").unwrap();
        let walk = read_sources_walk(&[tmp.join("corpus")]);
        assert_eq!(walk.mode, WalkMode::Plain, "ignored dir named on purpose = intent");
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
}
