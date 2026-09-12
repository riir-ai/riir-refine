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
//! `#![cfg(test)]`) and (b) the struct's own validate-shaped methods
//! (`bool`/`Result`-returning — `validate()`, `is_ok()`; a
//! `fmt::Result`-returning trait impl reads the same way). Classes A/B
//! require zero reads, so an internally-read, externally-orphaned config
//! struct is invisible to them; `dead_code` cannot see it either (`pub` =
//! assumed used). Also catches the never-plumbed literal default (zero reads
//! anywhere, literal init — A needs a const, B needs an assert).
//!
//! C2 read attribution: a read inside `impl X` where X ≠ T (another type's
//! method consuming the config) is EXTERNAL — it keeps T live. A read inside
//! `impl T` is internal ONLY when the enclosing method is validate-shaped;
//! any other return type (accessor `fn a(&self) -> f32 { self.a }`, builders)
//! is FLOW-OUT and counts as external — the value leaves the struct through
//! the method, so the field is honored. C2 is per-STRUCT and may co-fire
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

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, Macro};

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
    pub fields: Vec<SelfRefField>,
}

/// The audit result (data only — rendering is [`ConfigAuditReport::render`]).
#[derive(Debug, Default)]
pub struct ConfigAuditReport {
    pub files_scanned: usize,
    pub candidates: usize,
    pub inert_knobs: Vec<InertKnob>,
    pub assert_only: Vec<AssertOnly>,
    pub self_referential: Vec<SelfReferentialConfig>,
}

/// Class-B comparator context: field name → the default representations it
/// can assert against (const last segment, init token text).
type DefaultsByField = BTreeMap<String, Vec<(Option<String>, Option<String>)>>;

/// Scan (filename, source) pairs — the entry the unit tests drive;
/// [`audit_paths`] is the file-walking wrapper. Unparseable files are
/// skipped silently (a report must not die on one broken fixture file).
#[must_use]
pub fn audit_sources(sources: &[(String, String)]) -> ConfigAuditReport {
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

    // Class-B comparator context: field name → the default representations
    // it can assert against (const last segment / init token text).
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
            test_depth: 0,
            impl_stack: Vec::new(),
            validate_depth: 0,
        };
        v.visit_file(file);
    }

    classify(&candidates, &ix, files.len())
}

/// Walk `paths` (dirs recursively / files), parse every `.rs`, audit.
#[must_use]
pub fn audit_paths(paths: &[PathBuf]) -> ConfigAuditReport {
    let mut rust_files: Vec<PathBuf> = Vec::new();
    for p in paths {
        collect_rust_files(p, &mut rust_files);
    }
    let sources: Vec<(String, String)> = rust_files
        .iter()
        .filter_map(|p| {
            fs::read_to_string(p)
                .ok()
                .map(|src| (p.display().to_string(), src))
        })
        .collect();
    audit_sources(&sources)
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
    files_scanned: usize,
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
                report.inert_knobs.push(InertKnob {
                    candidate: cand.clone(),
                    const_reads,
                    array_len_uses,
                });
                continue;
            }
        }
        if asserts > 0 {
            report.assert_only.push(AssertOnly {
                candidate: cand.clone(),
                assert_interactions: asserts,
            });
        }
    }
    classify_self_referential(candidates, ix, &mut report);
    report
}

/// Class C2: group candidates per struct (insertion order) and fire when
/// every field's reads are test-confined or own-validate-impl-confined —
/// `external = total − tests − in_impl(T)` clamped at 0.
fn classify_self_referential(
    candidates: &[FieldCandidate],
    ix: &InteractionIndex,
    report: &mut ConfigAuditReport,
) {
    let mut seen: Vec<String> = Vec::new();
    for cand in candidates {
        if seen.contains(&cand.struct_name) {
            continue;
        }
        seen.push(cand.struct_name.clone());
        let mut fields: Vec<SelfRefField> = Vec::new();
        let mut all_confined = true;
        for f in candidates.iter().filter(|c| c.struct_name == cand.struct_name) {
            let total = ix.field_reads.get(&f.field_name).copied().unwrap_or(0);
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
                fields,
            });
        }
    }
}

/// Cross-file interaction tallies (pass-2 output).
#[derive(Debug, Default)]
struct InteractionIndex {
    /// Field name → read touches.
    field_reads: BTreeMap<String, usize>,
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
    /// Depth inside `#[cfg(test)]` mods / `#[test]`-like fns.
    test_depth: usize,
    /// Self-type names of the enclosing impl blocks (innermost last).
    impl_stack: Vec<String>,
    /// Depth inside validate-shaped (`bool`/`Result`-returning) methods.
    validate_depth: usize,
}

/// One field-read touch, attributed for class C2: the A/B total always
/// increments; the test / own-impl tallies only inside their contexts.
/// `internal_impl` is `Some(T)` when the read sits in a validate-shaped
/// method of `impl T` (test context takes precedence).
fn tally_field_read(
    ix: &mut InteractionIndex,
    name: &str,
    in_test: bool,
    internal_impl: Option<&str>,
) {
    *ix.field_reads.entry(name.to_string()).or_insert(0) += 1;
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
fn attr_is_cfg_test(attr: &syn::Attribute) -> bool {
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
fn attr_is_test_fn(attr: &syn::Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|s| s.ident == "test" || s.ident == "bench")
}

fn attrs_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(attr_is_cfg_test)
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
    /// own-impl; own-impl requires a validate-shaped method frame). Returns
    /// OWNED data — callers pair it with a mutable `ix` borrow.
    fn read_ctx(&self) -> (bool, Option<String>) {
        let in_test = self.file_is_test || self.test_depth > 0;
        let internal_impl = if in_test {
            None
        } else if self.validate_depth > 0 {
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
        syn::visit::visit_item_impl(self, impl_block);
        self.impl_stack.pop();
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
            tally_field_read(ix, &name, in_test, internal_impl.as_deref());
        }
        syn::visit::visit_expr_field(self, f);
    }

    fn visit_pat_struct(&mut self, pat: &'ast syn::PatStruct) {
        let (in_test, internal_impl) = self.read_ctx();
        let ix = &mut *self.ix;
        for pf in &pat.fields {
            if let Some(name) = field_member_name(&pf.member) {
                tally_field_read(ix, &name, in_test, internal_impl.as_deref());
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
                    tally_field_read(ix, &ident, in_test, internal_impl.as_deref());
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
                    tally_field_read(ix, &ident, in_test, internal_impl.as_deref());
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

fn field_member_name(member: &syn::Member) -> Option<String> {
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
fn sig_returns_validate_shape(sig: &syn::Signature) -> bool {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return false;
    };
    let syn::Type::Path(tp) = ty.as_ref() else {
        return false;
    };
    let last = tp
        .path
        .segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default();
    last == "bool" || last == "Result"
}

impl ConfigAuditReport {
    /// Render the human report (what `--config-audit` prints).
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "config-audit: {} file(s) parsed, {} Default-impl field candidate(s): {} inert knob(s) [A], {} assert-only field(s) [B], {} self-referential config struct(s) [C2]",
            self.files_scanned,
            self.candidates,
            self.inert_knobs.len(),
            self.assert_only.len(),
            self.self_referential.len()
        );
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
        for s in &self.self_referential {
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
        let _ = writeln!(
            out,
            "scope: detector sees only the scanned set; consumers in sibling/upstream crates are under-reported"
        );
        out
    }
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
}

