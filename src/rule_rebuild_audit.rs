//! Issue 242 — the rule-rebuild description-fidelity audit lane (report-only).
//!
//! The iWSP text-bottleneck metric (arXiv:2611.18114; distill record
//! `katgpt-rs/.research/678`) applied to the rule corpus: **can a rule's
//! behavior be re-derived from its ≤200-word description alone?** For each of
//! the 30 frozen rules (10/10/10 `clippy_lints` / `rust_perf` / `kernel_opt`; the
//! freeze + composition law live in `assets/rule_rebuild_audit/README.md`),
//! TWO arms run paired:
//!
//! 1. **Description arm** — a fresh writer sees ONLY the rule's composed
//!    description (never the rule id, never the buggy-arm code).
//! 2. **Control arm** — the paper's description-less control: the same
//!    re-draft task with the buggy-arm snippet in the prompt instead.
//!
//! Each answer is spliced in wholesale as the fixture's fixed arm and scored
//! on five columns (the verdict-lane-negotiated design, 2026-11-25, 3 rounds):
//!
//! - `strict` — trimmed-text equality against the authored fixed arm. Known
//!   ASYMMETRIC bias: identifier closeness favors the control arm (it sees
//!   the buggy arm's identifiers) over the description arm (which must
//!   invent names). A column, never the bar.
//! - `alpha` — the PRIMARY semantic column: both sides pass through an
//!   alpha-normalizer (local bindings, params, generics, lifetimes renamed
//!   to positional names in source order; the std expression-macro family's
//!   comma-separated expression args visited by the same rules) and compare
//!   as printed token streams (comment- and whitespace-insensitive).
//! - `compiled` — MANDATORY: the redraft's non-test items compile standalone
//!   (`rustc --edition 2021 --test`).
//! - `tests` — the AUTHORED fixture tests spliced against the redraft's
//!   non-test items and executed: `pass` / `fail` / `vacuous` (the authored
//!   arm carries no test surface). Authored tests failing to COMPILE against
//!   the redraft = `UNDECIDED-signature` (a signature mismatch, counted per
//!   arm, never NO MATCH).
//! - `self` — the writer's own tests, disclosed only, never a verdict leg.
//!
//! Verdict ranking (pre-registered): **MATCH** = compiled AND tests pass (or
//! vacuous) AND (alpha OR strict); compiled + tests pass but neither =
//! **UNDECIDED-shape** (a differently-shaped — possibly still-equivalent —
//! transformation; counted + printed, never folded); not compiled OR tests
//! fail = **NO MATCH**. A behavioral failure can never be lifted to MATCH by
//! spelling. Rules whose authored fixed arm does not parse as Rust are
//! **UNSCOREABLE** — disclosed, excluded from Δ, count printed (the pre-run
//! census lives in `assets/rule_rebuild_audit/README.md`).
//!
//! Report-only: exit 0 on every successful read; no promotion path, no
//! corpus mutation. The hosted writer refuses loud unless the seat is armed
//! (an armed `hosted_expert` manifest row + the snippet egress consent) and
//! the run cap is recorded (`--rule-rebuild-cap`, ≤ $10 per the issue; the
//! effective cap is min(arg, env)).

use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Display, Write as _};
use std::path::Path;
use std::time::{Duration, Instant};

use blake3::Hasher;
use proc_macro2::{Punct, Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use serde::Deserialize;
use syn::parse::Parser as _;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::visit_mut::VisitMut;
use syn::{Expr, File, Item};

// ── The frozen inputs ────────────────────────────────────────────────────

/// The composed descriptions (frozen pre-run by zed-m3-i242-t1prep; any
/// post-run change lands only as a new digest version — the freeze law).
pub const DESCRIPTIONS_JSON: &str =
    include_str!("../assets/rule_rebuild_audit/descriptions_v1.json");

/// BLAKE3 over the embedded file's bytes — drift refuses loud (their
/// canonical digest, recorded in the file's `digest` field, is the
/// composition lane's own anchor; this pin seals the BYTES this lane reads).
pub const DESCRIPTIONS_BLAKE3: &str =
    "aee41f116721768dd82c8d4c0a8fca71862d12514078745dead1cc1c15cd5ba0";

/// The frozen sample (amo-idle-i242-preflight, 2026-11-21; the sample-digest
/// anchor `a5ad3f9f…` is recorded in the README). The loader asserts exact
/// set equality with the descriptions file — a corpus or file drift re-freezes
/// by the same procedure, never silently rescores.
const FROZEN_SAMPLE: &[(&str, &str)] = &[
    ("clippy_lints", "doc_lazy_continuation"),
    ("clippy_lints", "needless_bool"),
    ("clippy_lints", "filter_next"),
    ("clippy_lints", "unnecessary_lazy_evaluations"),
    ("clippy_lints", "cast_lossless"),
    ("clippy_lints", "needless_range_loop"),
    ("clippy_lints", "unnecessary_map_or"),
    ("clippy_lints", "map_or_identity"),
    ("clippy_lints", "match_bool"),
    ("clippy_lints", "manual_repeat_n"),
    ("rust_perf", "eager-pointer-collect-lazy-fetch"),
    ("rust_perf", "watchdog-deadline-moves-on-command-progress"),
    ("rust_perf", "bufread-scan-in-buffer-scratch-fragment"),
    (
        "rust_perf",
        "saturating-f2i-cast-proven-range-to-int-unchecked",
    ),
    ("rust_perf", "batch-reserve-monotonic-counters-offload-bulk"),
    ("rust_perf", "byte-flag-table-masked-run-scan"),
    ("rust_perf", "retain-inplace"),
    ("rust_perf", "work-removed-build-is-not-a-valid-lower-bound"),
    ("rust_perf", "regex-hoist-lazylock"),
    ("rust_perf", "smallvec-stack"),
    ("kernel_opt", "boot-identity-hash-read-once"),
    ("kernel_opt", "same-input-gemv-segments-single-launch"),
    (
        "kernel_opt",
        "walk-terminating-fill-for-unused-buffer-capacity",
    ),
    ("kernel_opt", "tile-coo-sparse-tile-aligned-format"),
    (
        "kernel_opt",
        "enumerable-domain-bit-identity-admission-gate",
    ),
    (
        "kernel_opt",
        "verify-fold-gqa-repeats-into-row-axis-tile-fill",
    ),
    (
        "kernel_opt",
        "defer-promoted-write-to-first-consumer-dispatch",
    ),
    (
        "kernel_opt",
        "dtype-branch-dispatch-parity-in-place-precondition",
    ),
    ("kernel_opt", "moe-group-selection-fused-into-topk"),
    (
        "kernel_opt",
        "block-scaled-mma-scale-register-owns-its-rows",
    ),
];

const DESCRIPTIONS_FORMAT_VERSION: u32 = 1;

/// The low-power bar (verdict round 3): below this scoreable count, the
/// report prints a warning — a 10-point Δ gap is only a couple of rules.
const LOW_POWER_SCOREABLE: usize = 20;

/// The hosted per-call generation budget (a complete module + prose).
const MAX_TOKENS_PER_CALL: u64 = 4096;

/// The hosted per-call wall budget (the leaf transport carries the hard
/// in-flight ceiling; this deadline declines pre-socket once expired).
const PER_CALL_DEADLINE: Duration = Duration::from_secs(180);

#[derive(Clone, Deserialize)]
struct DescriptionRow {
    rule_id: String,
    description: String,
    domain: String,
}

#[derive(Deserialize)]
struct DescriptionsFile {
    format_version: u32,
    digest: String,
    composed_at_head: String,
    rows: Vec<DescriptionRow>,
}

/// One sampled rule, resolved to its inline corpus fixture pair.
#[derive(Clone, Debug)]
pub struct Fixture {
    pub domain: Domain,
    pub rule_id: String,
    pub description: String,
    pub buggy: String,
    pub fixed: String,
}

/// The loaded, pinned audit inputs (descriptions + resolved fixtures).
#[derive(Clone, Debug)]
pub struct AuditInput {
    /// The composition lane's own canonical digest (provenance, printed).
    pub canonical_digest: String,
    pub composed_at_head: String,
    pub fixtures: Vec<Fixture>,
}

// ── Errors ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum AuditError {
    /// The embedded descriptions file's bytes do not match the pin.
    DescriptionsDigest {
        expected: String,
        actual: String,
    },
    /// The file does not parse / carry the expected shape.
    DescriptionsShape(String),
    /// The rows do not match the frozen 30-rule sample exactly.
    SamplePin {
        missing: Vec<String>,
        extra: Vec<String>,
    },
    /// A frozen id has no resolvable fixture pair in the corpus (the
    /// blind-derivation law: never score a silently-shrunk subset).
    BlindFixture(Vec<String>),
    Io(String),
}

impl Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DescriptionsDigest { expected, actual } => write!(
                f,
                "descriptions digest drift: expected blake3 {expected}, read {actual} — the \
                 frozen file moved; re-freeze by the README procedure, never re-pin silently"
            ),
            Self::DescriptionsShape(m) => write!(f, "descriptions file malformed: {m}"),
            Self::SamplePin { missing, extra } => {
                write!(f, "sample pin drift")?;
                if !missing.is_empty() {
                    write!(f, "; missing {}", missing.join(", "))?;
                }
                if !extra.is_empty() {
                    write!(f, "; unexpected {}", extra.join(", "))?;
                }
                Ok(())
            }
            Self::BlindFixture(missing) => write!(
                f,
                "blind derivation refused: {} frozen id(s) resolve to no corpus fixture pair: {}",
                missing.len(),
                missing.join(", ")
            ),
            Self::Io(m) => write!(f, "io: {m}"),
        }
    }
}

// ── Domains ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Domain {
    ClippyLints,
    RustPerf,
    KernelOpt,
}

impl Domain {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "clippy_lints" => Some(Self::ClippyLints),
            "rust_perf" => Some(Self::RustPerf),
            "kernel_opt" => Some(Self::KernelOpt),
            _ => None,
        }
    }
}

impl Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ClippyLints => "clippy_lints",
            Self::RustPerf => "rust_perf",
            Self::KernelOpt => "kernel_opt",
        })
    }
}

// ── The loader ───────────────────────────────────────────────────────────

/// Load the embedded descriptions (pins verified) + resolve every frozen id
/// to its inline corpus fixture pair.
///
/// # Errors
///
/// [`AuditError::DescriptionsDigest`] on byte drift, [`AuditError::SamplePin`]
/// on sample drift, [`AuditError::BlindFixture`] when a frozen id resolves to
/// no corpus pair — every refusal is loud and names the piece.
pub fn load_input() -> Result<AuditInput, AuditError> {
    load_input_from(DESCRIPTIONS_JSON, DESCRIPTIONS_BLAKE3)
}

/// The injectable core of [`load_input`] (the gates drive it directly with
/// tampered inputs; no test ever mutates the embedded bytes).
///
/// # Errors
///
/// The same taxonomy as [`load_input`].
pub fn load_input_from(json: &str, expected_blake3: &str) -> Result<AuditInput, AuditError> {
    let mut hasher = Hasher::new();
    hasher.update(json.as_bytes());
    let actual = hasher.finalize().to_hex().to_string();
    if actual != expected_blake3 {
        return Err(AuditError::DescriptionsDigest {
            expected: expected_blake3.to_string(),
            actual,
        });
    }
    let file: DescriptionsFile =
        serde_json::from_str(json).map_err(|e| AuditError::DescriptionsShape(e.to_string()))?;
    if file.format_version != DESCRIPTIONS_FORMAT_VERSION {
        return Err(AuditError::DescriptionsShape(format!(
            "format_version {} != {DESCRIPTIONS_FORMAT_VERSION}",
            file.format_version
        )));
    }
    // Sample pin: exact set equality with the frozen 30 (domain, rule_id).
    let mut want: std::collections::HashSet<(String, String)> = FROZEN_SAMPLE
        .iter()
        .map(|(d, r)| ((*d).to_string(), (*r).to_string()))
        .collect();
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    for row in &file.rows {
        let key = (row.domain.clone(), row.rule_id.clone());
        if !want.remove(&key) {
            extra.push(format!("{}/{}", row.domain, row.rule_id));
        }
    }
    for key in &want {
        missing.push(format!("{}/{}", key.0, key.1));
    }
    if !missing.is_empty() || !extra.is_empty() {
        return Err(AuditError::SamplePin { missing, extra });
    }
    // Resolve fixtures.
    let mut missing_fixtures = Vec::new();
    let mut fixtures = Vec::new();
    for row in &file.rows {
        let Some(domain) = Domain::parse(&row.domain) else {
            return Err(AuditError::DescriptionsShape(format!(
                "unknown domain {:?}",
                row.domain
            )));
        };
        match resolve_fixture(domain, &row.rule_id) {
            Some((buggy, fixed)) => fixtures.push(Fixture {
                domain,
                rule_id: row.rule_id.clone(),
                description: row.description.clone(),
                buggy,
                fixed,
            }),
            None => missing_fixtures.push(format!("{}/{}", row.domain, row.rule_id)),
        }
    }
    if !missing_fixtures.is_empty() {
        return Err(AuditError::BlindFixture(missing_fixtures));
    }
    Ok(AuditInput {
        canonical_digest: file.digest,
        composed_at_head: file.composed_at_head,
        fixtures,
    })
}

/// Resolve one rule's inline fixture pair: `(buggy, fixed)`. `None` = blind.
fn resolve_fixture(domain: Domain, rule_id: &str) -> Option<(String, String)> {
    match domain {
        Domain::ClippyLints => {
            let corpus = crate::domains::clippy_lints::corpus::build_seed_corpus();
            corpus.entries.iter().find_map(|e| {
                (e.lint_name == rule_id).then(|| (e.buggy_code.clone(), e.fixed_code.clone()))
            })
        }
        Domain::RustPerf => {
            let corpus = crate::domains::rust_perf::corpus::build_perf_seed_corpus();
            corpus.entries.iter().find_map(|e| {
                (e.rule_id == rule_id).then(|| (e.buggy_code.clone(), e.fixed_code.clone()))
            })
        }
        Domain::KernelOpt => {
            let corpus = crate::domains::kernel_opt::corpus::build_kernel_seed_corpus();
            corpus.entries.iter().find_map(|e| {
                (e.rule_id == rule_id).then(|| (e.ref_shape.clone(), e.opt_pattern.clone()))
            })
        }
    }
}

// ── Test-surface split ───────────────────────────────────────────────────

fn is_cfg_test(attr: &syn::Attribute) -> bool {
    matches!(&attr.meta, syn::Meta::List(list)
        if list.path.is_ident("cfg") && list.tokens.to_string().trim() == "test")
}

fn is_test_item(item: &Item) -> bool {
    match item {
        Item::Fn(f) => f
            .attrs
            .iter()
            .any(|a| a.path().is_ident("test") || a.path().is_ident("should_panic")),
        Item::Mod(m) => m.attrs.iter().any(is_cfg_test),
        _ => false,
    }
}

fn file_from(attrs: &[syn::Attribute], items: Vec<Item>) -> File {
    File {
        shebang: None,
        attrs: attrs.to_vec(),
        items,
    }
}

fn file_text(file: &File) -> String {
    file.to_token_stream().to_string()
}

/// Split a parsed file into (non-test file, test items with their attrs).
fn partition(file: File) -> (File, Vec<Item>) {
    let attrs = file.attrs.clone();
    let mut kept = Vec::new();
    let mut tests = Vec::new();
    for item in file.items {
        if is_test_item(&item) {
            tests.push(item);
        } else {
            kept.push(item);
        }
    }
    (file_from(&attrs, kept), tests)
}

// ── The alpha normalizer ─────────────────────────────────────────────────

/// The std expression-macro family whose comma-separated expression
/// arguments are visited by the same renaming rules (verdict round 2).
/// Every other macro stays opaque (tokens untouched; the renamed-equivalent
/// gate across all 30 frozen rules proves the coverage on the real corpus).
const RENAMEABLE_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "format",
    "print",
    "println",
    "eprint",
    "eprintln",
    "write",
    "writeln",
    "vec",
    "matches",
    "panic",
];

fn is_bindable(name: &str) -> bool {
    !matches!(name, "_" | "self" | "Self" | "crate" | "super")
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn lifetime_key(l: &syn::Lifetime) -> String {
    format!("'{}", l.ident)
}

/// Rename every local binding to its positional canonical name (`v0`, `v1`,
/// …) in source order, then print the file back. Scopes stack per fn /
/// closure; let-bindings insert into the current frame (shadowing-correct
/// for sequential rebinds; a `let x = x` init resolving against the NEW
/// binding is a documented quirk — deterministic-identical on both sides,
/// so alpha-equivalence is unaffected). Uppercase pattern idents stay raw
/// (consts / unit variants, not bindings); attributes and non-family macro
/// token streams stay raw.
struct AlphaNormalizer {
    scopes: Vec<HashMap<String, String>>,
    counter: usize,
}

impl AlphaNormalizer {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            counter: 0,
        }
    }

    fn bind(&mut self, name: &str) -> String {
        let canon = format!("v{}", self.counter);
        self.counter += 1;
        if let Some(top) = self.scopes.last_mut() {
            top.insert(name.to_string(), canon.clone());
        }
        canon
    }

    fn lookup(&self, name: &str) -> Option<String> {
        self.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }

    // Loop labels share the lifetime namespace (`'a: loop` + `break 'a`).
    fn bind_label(&mut self, label: Option<&syn::Label>) {
        if let Some(l) = label {
            if l.name.ident != "static" && l.name.ident != "_" {
                let key = lifetime_key(&l.name);
                let _canon = self.bind(&key);
            }
        }
    }

    fn rename_single_path(&self, path: &mut syn::Path) {
        if path.leading_colon.is_none() && path.segments.len() == 1 {
            let name = path.segments[0].ident.to_string();
            if is_bindable(&name) {
                if let Some(canon) = self.lookup(&name) {
                    path.segments[0].ident =
                        proc_macro2::Ident::new(&canon, path.segments[0].ident.span());
                    path.segments[0].arguments = syn::PathArguments::None;
                }
            }
        }
    }
}

impl VisitMut for AlphaNormalizer {
    // Attributes stay raw (cfg paths, derive hints, doc attrs).
    fn visit_attribute_mut(&mut self, _i: &mut syn::Attribute) {}

    // Item-level macro defs + invocations stay opaque.
    fn visit_item_macro_mut(&mut self, _i: &mut syn::ItemMacro) {}

    // ── binding sites ────────────────────────────────────────────────────

    fn visit_lifetime_param_mut(&mut self, p: &mut syn::LifetimeParam) {
        if p.lifetime.ident != "static" && p.lifetime.ident != "_" {
            let key = lifetime_key(&p.lifetime);
            let canon = self.bind(&key);
            p.lifetime.ident =
                proc_macro2::Ident::new(canon.trim_start_matches('\''), p.lifetime.ident.span());
        }
        syn::visit_mut::visit_lifetime_param_mut(self, p);
    }

    fn visit_type_param_mut(&mut self, p: &mut syn::TypeParam) {
        let name = p.ident.to_string();
        if is_bindable(&name) {
            let canon = self.bind(&name);
            p.ident = proc_macro2::Ident::new(&canon, p.ident.span());
        }
        syn::visit_mut::visit_type_param_mut(self, p);
    }

    fn visit_const_param_mut(&mut self, p: &mut syn::ConstParam) {
        let name = p.ident.to_string();
        if is_bindable(&name) {
            let canon = self.bind(&name);
            p.ident = proc_macro2::Ident::new(&canon, p.ident.span());
        }
        syn::visit_mut::visit_const_param_mut(self, p);
    }

    fn visit_pat_ident_mut(&mut self, p: &mut syn::PatIdent) {
        let name = p.ident.to_string();
        if is_bindable(&name) {
            let canon = self.bind(&name);
            p.ident = proc_macro2::Ident::new(&canon, p.ident.span());
        }
        syn::visit_mut::visit_pat_ident_mut(self, p);
    }

    fn visit_field_pat_mut(&mut self, f: &mut syn::FieldPat) {
        // Shorthand `Foo { x }` — the member doubles as a binding; the
        // member itself stays raw (field names are not scope bindings).
        if f.colon_token.is_none() {
            if let syn::Member::Named(ident) = &f.member {
                let name = ident.to_string();
                if is_bindable(&name) {
                    let _canon = self.bind(&name);
                }
            }
            return;
        }
        syn::visit_mut::visit_field_pat_mut(self, f);
    }

    // Scopes: one frame per fn / closure; the generic params, fn params and
    // body bind inside it in source order (syn's walk visits sig before
    // block, patterns before the bodies they guard).
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        self.scopes.push(HashMap::new());
        syn::visit_mut::visit_item_fn_mut(self, i);
        self.scopes.pop();
    }

    fn visit_expr_closure_mut(&mut self, e: &mut syn::ExprClosure) {
        self.scopes.push(HashMap::new());
        syn::visit_mut::visit_expr_closure_mut(self, e);
        self.scopes.pop();
    }

    // Loop labels share the lifetime namespace (`'a: loop` + `break 'a`).
    fn visit_expr_loop_mut(&mut self, e: &mut syn::ExprLoop) {
        self.bind_label(e.label.as_ref());
        syn::visit_mut::visit_expr_loop_mut(self, e);
    }

    fn visit_expr_while_mut(&mut self, e: &mut syn::ExprWhile) {
        self.bind_label(e.label.as_ref());
        syn::visit_mut::visit_expr_while_mut(self, e);
    }

    fn visit_expr_for_loop_mut(&mut self, e: &mut syn::ExprForLoop) {
        self.bind_label(e.label.as_ref());
        syn::visit_mut::visit_expr_for_loop_mut(self, e);
    }

    // ── use sites ────────────────────────────────────────────────────────

    fn visit_expr_path_mut(&mut self, e: &mut syn::ExprPath) {
        if e.qself.is_none() {
            self.rename_single_path(&mut e.path);
        }
        syn::visit_mut::visit_expr_path_mut(self, e);
    }

    fn visit_type_path_mut(&mut self, t: &mut syn::TypePath) {
        if t.qself.is_none() {
            self.rename_single_path(&mut t.path);
        }
        syn::visit_mut::visit_type_path_mut(self, t);
    }

    fn visit_lifetime_mut(&mut self, l: &mut syn::Lifetime) {
        if l.ident != "static" && l.ident != "_" {
            let key = lifetime_key(l);
            if let Some(canon) = self.lookup(&key) {
                l.ident = proc_macro2::Ident::new(canon.trim_start_matches('\''), l.ident.span());
            }
        }
    }

    fn visit_expr_macro_mut(&mut self, m: &mut syn::ExprMacro) {
        if rename_macro_args(&mut m.mac, self) {
            // The family's args were parsed, visited and re-tokenized.
        } else {
            // Opaque: the token stream stays raw (disclosed class).
        }
    }
}

fn rename_macro_args(mac: &mut syn::Macro, nz: &mut AlphaNormalizer) -> bool {
    let Some(last) = mac.path.segments.last() else {
        return false;
    };
    let name = last.ident.to_string();
    if !RENAMEABLE_MACROS.contains(&name.as_str()) {
        return false;
    }
    let Ok(mut exprs) = Punctuated::<Expr, Comma>::parse_terminated.parse2(mac.tokens.clone())
    else {
        return false;
    };
    for e in &mut exprs {
        nz.visit_expr_mut(e);
    }
    let mut ts = TokenStream::new();
    for (i, e) in exprs.iter().enumerate() {
        if i > 0 {
            ts.extend(std::iter::once(TokenTree::Punct(Punct::new(
                ',',
                Spacing::Alone,
            ))));
        }
        ts.extend(e.to_token_stream());
    }
    mac.tokens = ts;
    true
}

/// Alpha-normalize a Rust source string: parse → strip the test surface →
/// rename locals to positional names → print. Parse failure = `Err`.
///
/// # Errors
///
/// [`syn::Error`] when `src` does not parse as a Rust file.
pub fn normalize_source(src: &str) -> Result<String, syn::Error> {
    let file = syn::parse_file(src)?;
    let (mut items_file, _tests) = partition(file);
    let mut nz = AlphaNormalizer::new();
    nz.visit_file_mut(&mut items_file);
    Ok(file_text(&items_file))
}

// ── Signatures (the PUBLIC ITEM CONTRACT) ────────────────────────────────

fn type_name_of(ty: &syn::Type) -> String {
    if let syn::Type::Path(tp) = ty {
        if tp.qself.is_none() {
            if let Some(seg) = tp.path.segments.last() {
                return seg.ident.to_string();
            }
        }
    }
    String::from("_")
}

fn collect_sig_item(item: &Item, out: &mut Vec<String>) {
    match item {
        Item::Fn(f) => out.push(format!("fn {}/{}", f.sig.ident, f.sig.inputs.len())),
        Item::Struct(s) => out.push(format!("type {}", s.ident)),
        Item::Enum(e) => out.push(format!("type {}", e.ident)),
        Item::Union(u) => out.push(format!("type {}", u.ident)),
        Item::Trait(t) => out.push(format!("trait {}", t.ident)),
        Item::Type(t) => out.push(format!("type {}", t.ident)),
        Item::Const(c) => out.push(format!("const {}", c.ident)),
        Item::Static(s) => out.push(format!("static {}", s.ident)),
        Item::Impl(i) => {
            let owner = type_name_of(&i.self_ty);
            for item in &i.items {
                if let syn::ImplItem::Fn(f) = item {
                    out.push(format!(
                        "fn {}::{}({} args)",
                        owner,
                        f.sig.ident,
                        f.sig.inputs.len()
                    ));
                }
            }
        }
        Item::Mod(m) => {
            if let Some((_, items)) = &m.content {
                for item in items {
                    collect_sig_item(item, out);
                }
            }
        }
        _ => {}
    }
}

/// The authored arm's public item surface (names + arities) — pinned into
/// BOTH arms' prompts identically (verdict round 2: the authored tests can
/// only splice against a redraft that keeps the item names, and the pin
/// adds no fixture code to the description arm).
#[must_use]
pub fn item_signatures(file: &File) -> String {
    let mut lines = Vec::new();
    for item in &file.items {
        collect_sig_item(item, &mut lines);
    }
    if lines.is_empty() {
        String::from("(no items)")
    } else {
        lines.join("\n")
    }
}

// ── Authored arms + scoring ──────────────────────────────────────────────

/// The authored fixture's fixed arm, pre-split: items (the alpha/strict
/// baseline + the compile target), tests (the behavioral witness spliced
/// against redrafts), signatures (the prompt pin), and the pre-normalized
/// alpha baseline.
pub struct AuthoredArms {
    pub items_file: File,
    pub items_text: String,
    pub items_norm: Option<String>,
    pub tests_text: Option<String>,
    pub signatures: String,
}

/// Parse + split the authored arms. `Err` = the authored FIXED arm does not
/// parse as Rust → the rule is UNSCOREABLE (disclosed, excluded from Δ).
///
/// # Errors
///
/// [`syn::Error`] when the fixed arm does not parse as a Rust file.
pub fn authored_arms(buggy: &str, fixed: &str) -> Result<AuthoredArms, syn::Error> {
    let _ = buggy; // the buggy arm feeds prompts only; it need not parse
    let fixed_file = syn::parse_file(fixed)?;
    let file_attrs = fixed_file.attrs.clone();
    let (items_file, tests) = partition(fixed_file);
    let items_text = file_text(&items_file);
    let items_norm = normalize_source(&items_text).ok();
    let tests_text = if tests.is_empty() {
        None
    } else {
        Some(file_text(&file_from(&file_attrs, tests)))
    };
    let signatures = item_signatures(&items_file);
    Ok(AuthoredArms {
        items_file,
        items_text,
        items_norm,
        tests_text,
        signatures,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArmVerdict {
    /// compiled AND tests pass (or `tests_vacuous`) AND (alpha OR strict).
    Match { tests_vacuous: bool },
    /// compiled + tests pass but neither alpha nor strict: a differently-
    /// shaped — possibly still-equivalent — transformation.
    UndecidedShape,
    /// The authored tests failed to COMPILE against the redraft: a public
    /// item signature mismatch, counted per arm, never NO MATCH.
    UndecidedSignature,
    /// Not compiled, tests failed, or a writer refusal (fence / decline).
    NoMatch,
}

impl Display for ArmVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Match {
                tests_vacuous: true,
            } => f.write_str("MATCH(tests=vacuous)"),
            Self::Match {
                tests_vacuous: false,
            } => f.write_str("MATCH"),
            Self::UndecidedShape => f.write_str("UNDECIDED-shape"),
            Self::UndecidedSignature => f.write_str("UNDECIDED-signature"),
            Self::NoMatch => f.write_str("NO MATCH"),
        }
    }
}

/// One arm's score: the three columns (never pooled) + the test legs.
#[derive(Clone, Debug)]
pub struct ArmScore {
    pub strict: bool,
    pub alpha: bool,
    pub compiled: bool,
    /// The authored tests against the redraft items: `Some(passed)`, `None`
    /// = vacuous (the authored arm carries no test surface).
    pub tests: Option<bool>,
    /// The writer's own tests — disclosed only, never a verdict leg.
    pub self_tests: Option<bool>,
    pub verdict: ArmVerdict,
    pub note: String,
}

// ── The compiler seam ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct CompileOutcome {
    pub compiled: bool,
    pub tests_passed: Option<bool>,
    pub detail: String,
}

/// The compile oracle: compile `src` as a standalone test crate; when it
/// compiles, run the binary and report the test verdict. The production
/// arm shells out to `rustc`; the hermetic gates use [`FakeOracle`] (the
/// Issue-259 T3 fake-cargo pattern — no rustc inside unit tests).
pub trait CompileOracle {
    /// Compile + run one artifact. `label` names the artifact class
    /// (`items` / `splice` / `self_tests`) so fakes can discriminate.
    #[must_use]
    fn compile_and_run(&self, label: &str, src: &str) -> CompileOutcome;
}

/// The production oracle: `rustc --edition 2021 --test`, bounded waits.
pub struct RustcOracle {
    pub edition: &'static str,
}

impl Default for RustcOracle {
    fn default() -> Self {
        Self {
            edition: "2021", // the repo's edition, pinned + disclosed
        }
    }
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn tail(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut cut = text.len() - max;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    text[cut..].to_string()
}

/// Spawn + bounded wait (std has no timeout; a hung rustc would hang the
/// lane — kill at the deadline instead).
fn run_command(cmd: &mut std::process::Command, timeout_secs: u64) -> Option<(bool, String)> {
    use std::io::Read;
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().ok()?;
    let mut stderr_pipe = child.stderr.take()?;
    let drainer = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr_pipe.read_to_string(&mut buf);
        buf
    });
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break None,
        }
    };
    let stderr_text = drainer.join().unwrap_or_default();
    status.map(|s| (s.success(), stderr_text))
}

impl CompileOracle for RustcOracle {
    fn compile_and_run(&self, label: &str, src: &str) -> CompileOutcome {
        // Process-qualified temp dir (the shared_temp_path gate law).
        let dir = std::env::temp_dir().join(format!("rule_rebuild_audit_{}", std::process::id()));
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return CompileOutcome {
                compiled: false,
                tests_passed: None,
                detail: format!("temp dir {}: {e}", dir.display()),
            };
        }
        let stem = sanitize_label(label);
        let path = dir.join(format!("{stem}.rs"));
        let bin = dir.join(format!("{stem}_bin"));
        if let Err(e) = std::fs::write(&path, src) {
            return CompileOutcome {
                compiled: false,
                tests_passed: None,
                detail: format!("write {}: {e}", path.display()),
            };
        }
        let mut compile_cmd = std::process::Command::new("rustc");
        let compile_cmd = compile_cmd
            .arg(format!("--edition={}", self.edition))
            .arg("--test")
            .arg("-o")
            .arg(&bin)
            .arg(&path);
        let Some((ok, stderr)) = run_command(compile_cmd, 120) else {
            return CompileOutcome {
                compiled: false,
                tests_passed: None,
                detail: String::from("rustc timed out (120 s)"),
            };
        };
        if !ok {
            return CompileOutcome {
                compiled: false,
                tests_passed: None,
                detail: tail(stderr.trim(), 300),
            };
        }
        let mut run_cmd = std::process::Command::new(&bin);
        let Some((rok, _)) = run_command(&mut run_cmd, 30) else {
            return CompileOutcome {
                compiled: true,
                tests_passed: None,
                detail: String::from("test binary timed out (30 s)"),
            };
        };
        CompileOutcome {
            compiled: true,
            tests_passed: Some(rok),
            detail: String::new(),
        }
    }
}

/// A fake compile verdict: `(artifact label, source) -> compiles-and-passes?`.
pub type FakeVerdict = dyn Fn(&str, &str) -> bool + Send + Sync;

/// The hermetic oracle: `(label, src)` closures — no rustc inside unit
/// tests (the Issue-259 T3 fake-cargo pattern). `verdict` decides the
/// compile leg; `tests_fail_on` (checked FIRST) models a compiled binary
/// whose tests FAIL — the behavioral-witness leg the plain verdict cannot
/// express.
pub struct FakeOracle {
    pub verdict: Box<FakeVerdict>,
    pub tests_fail_on: Box<FakeVerdict>,
}

impl FakeOracle {
    /// The plain posture: everything the verdict accepts compiles AND
    /// passes; everything it refuses does not compile.
    #[must_use]
    pub fn plain(verdict: Box<FakeVerdict>) -> Self {
        Self {
            verdict,
            tests_fail_on: Box::new(|_, _| false),
        }
    }
}

impl CompileOracle for FakeOracle {
    fn compile_and_run(&self, label: &str, src: &str) -> CompileOutcome {
        if (self.tests_fail_on)(label, src) {
            CompileOutcome {
                compiled: true,
                tests_passed: Some(false),
                detail: String::from("fake oracle: tests fail"),
            }
        } else if (self.verdict)(label, src) {
            CompileOutcome {
                compiled: true,
                tests_passed: Some(true),
                detail: String::from("fake oracle"),
            }
        } else {
            CompileOutcome {
                compiled: false,
                tests_passed: None,
                detail: String::from("fake oracle: refused"),
            }
        }
    }
}

// ── The writer seam ──────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArmKind {
    Description,
    Control,
}

/// One re-draft request. `rule_index` is bookkeeping for replay writers —
/// the PROMPT builder must never include it (the description arm is
/// id-free by composition law; the control arm gains no hint either).
pub struct RedraftRequest {
    pub arm: ArmKind,
    pub rule_index: usize,
    pub description: String,
    pub buggy_arm: String,
    pub signatures: String,
}

/// The pinned application step (verdict round 2, condition 5) — identical
/// in both arms' prompts.
pub const APPLICATION_STEP: &str = "APPLICATION CONTRACT (binding): write the COMPLETE fixed Rust module \
    — every public item, at the same names and arities listed in the PUBLIC ITEM CONTRACT below — that \
    repairs the trigger described. Your output is spliced in wholesale as the fixture's fixed arm: the \
    grader compiles your items, splices the authored tests against them, and compares the result. Do not \
    write tests (the grader supplies them). Wrap the whole module in exactly one fenced rust code block; \
    no prose outside it.";

/// Build the prompt for one arm. Both arms carry the identical application
/// step + the identical PUBLIC ITEM CONTRACT; the payload differs (prose
/// description vs buggy-arm snippet).
#[must_use]
pub fn build_prompt(req: &RedraftRequest) -> String {
    let task = match req.arm {
        ArmKind::Description => format!(
            "TASK: Below is a prose description of a code-transformation rule's trigger and fix \
             shape. Re-derive the fix: write the complete fixed Rust module for the trigger it \
             describes.\n\nDESCRIPTION:\n{}",
            req.description
        ),
        ArmKind::Control => format!(
            "TASK: Below is a Rust module exhibiting a defect. Write the fixed module: the same \
             items, the defect repaired.\n\nBUGGY MODULE:\n```rust\n{}\n```",
            req.buggy_arm
        ),
    };
    format!(
        "{task}\n\n{APPLICATION_STEP}\n\nPUBLIC ITEM CONTRACT — your fixed module must define \
         exactly these items (names and arities):\n{}",
        req.signatures
    )
}

/// The writer seam: hosted at run time, replay in the hermetic gates.
pub trait RebuildWriter {
    /// One re-draft. `Err` = declined / refused — scored NO MATCH for that
    /// arm, counted (the message distinguishes `declined:` from the
    /// fence-refusal class).
    ///
    /// # Errors
    ///
    /// Declines (`declined: …`) and fence refusals (the fence-refusal
    /// class) — each scored NO MATCH for its arm, never a panic.
    fn redraft(&mut self, req: &RedraftRequest) -> Result<String, String>;

    #[must_use]
    fn label(&self) -> String;
}

/// Extract the single fenced rust code block. Zero or more than one block =
/// typed refusal (verdict round 2, condition 6: never silently take the
/// first block).
///
/// # Errors
///
/// A message naming the fence-refusal class when the block count is not
/// exactly one.
pub fn extract_rust_block(answer: &str) -> Result<String, String> {
    let mut blocks = Vec::new();
    let mut rest = answer;
    while let Some(start) = rest.find("```") {
        let after = &rest[start + 3..];
        let body_start = after.find('\n').map_or(0, |i| i + 1);
        let Some(end_rel) = after[body_start..].find("```") else {
            return Err(String::from(
                "unterminated fenced block (the fence-refusal class)",
            ));
        };
        let body = after[body_start..body_start + end_rel].trim();
        blocks.push(body.to_string());
        rest = &after[body_start + end_rel + 3..];
    }
    match blocks.len() {
        1 => Ok(blocks.remove(0)),
        0 => Err(String::from(
            "no fenced code block (the fence-refusal class)",
        )),
        n => Err(format!(
            "{n} fenced code blocks — the single-block contract refuses (the fence-refusal class)"
        )),
    }
}

/// The hosted writer: one [`HostedExpertL4`] transport (consent, deadline,
/// spend cap and the no-retention wire rule are all the existing
/// machinery's), one fenced-block extraction per answer.
pub struct HostedWriter {
    expert: crate::hosted_expert::HostedExpertL4,
    max_tokens: u64,
    per_call_deadline: Duration,
}

impl HostedWriter {
    #[must_use]
    pub fn new(
        expert: crate::hosted_expert::HostedExpertL4,
        max_tokens: u64,
        per_call_deadline: Duration,
    ) -> Self {
        Self {
            expert,
            max_tokens,
            per_call_deadline,
        }
    }
}

impl RebuildWriter for HostedWriter {
    fn redraft(&mut self, req: &RedraftRequest) -> Result<String, String> {
        let prompt = build_prompt(req);
        let deadline = Some(Instant::now() + self.per_call_deadline);
        let consent = crate::code_egress::EgressConsent::from_env();
        match self
            .expert
            .generate_redraft_checked_with(&prompt, deadline, self.max_tokens, consent)
        {
            Ok(answer) => extract_rust_block(&answer),
            Err(d) => Err(format!("declined: {d}")),
        }
    }

    fn label(&self) -> String {
        String::from("hosted")
    }
}

/// The hermetic replay writer (the gates' writer; the CLI's `replay`
/// self-test posture). Identity returns the authored fixed arm per index;
/// Mutated appends a parseable marker item (every column breaks).
pub struct ReplayWriter {
    mode: ReplayMode,
    authored_fixed: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReplayMode {
    Identity,
    Mutated,
}

pub const MUTATION_MARKER: &str = "\nfn __audit_mutation_marker() {}\n";

impl ReplayWriter {
    /// Resolve the frozen fixtures (the same loader `run_audit` uses, so
    /// indices align) and answer from them.
    ///
    /// # Errors
    ///
    /// The [`AuditError`] taxonomy (digest / sample / blind fixture).
    pub fn new(mode: ReplayMode) -> Result<Self, AuditError> {
        let input = load_input()?;
        Ok(Self {
            mode,
            authored_fixed: input.fixtures.into_iter().map(|f| f.fixed).collect(),
        })
    }
}

impl RebuildWriter for ReplayWriter {
    fn redraft(&mut self, req: &RedraftRequest) -> Result<String, String> {
        let Some(authored) = self.authored_fixed.get(req.rule_index) else {
            return Err(format!(
                "replay writer: index {} out of range",
                req.rule_index
            ));
        };
        match self.mode {
            ReplayMode::Identity => Ok(authored.clone()),
            ReplayMode::Mutated => Ok(format!("{authored}{MUTATION_MARKER}")),
        }
    }

    fn label(&self) -> String {
        format!("replay/{:?}", self.mode)
    }
}

// ── Scoring ──────────────────────────────────────────────────────────────

/// Score one redraft against one authored fixture (the verdict ranking,
/// pre-registered — never pooled columns, never a lifted behavioral
/// failure).
#[must_use]
pub fn score_arm(
    authored: &AuthoredArms,
    redraft: &Result<String, String>,
    oracle: &dyn CompileOracle,
) -> ArmScore {
    let refused = |note: String| ArmScore {
        strict: false,
        alpha: false,
        compiled: false,
        tests: None,
        self_tests: None,
        verdict: ArmVerdict::NoMatch,
        note,
    };
    let answer = match redraft {
        Ok(a) => a.clone(),
        Err(e) => return refused(e.clone()),
    };
    let redraft_file = match syn::parse_file(&answer) {
        Ok(f) => f,
        Err(e) => return refused(format!("redraft does not parse: {e}")),
    };
    let redraft_attrs = redraft_file.attrs.clone();
    let (items_file, own_tests) = partition(redraft_file);
    let items_text = file_text(&items_file);
    let strict = items_text.trim() == authored.items_text.trim();
    let alpha = normalize_source(&items_text).ok().as_ref() == authored.items_norm.as_ref();
    // (1) the redraft's items compile standalone — MANDATORY.
    let comp = oracle.compile_and_run("items", &items_text);
    if !comp.compiled {
        return ArmScore {
            strict,
            alpha,
            compiled: false,
            tests: None,
            self_tests: None,
            verdict: ArmVerdict::NoMatch,
            note: format!("compile: {}", comp.detail),
        };
    }
    // (2) the authored tests splice against the redraft items.
    let tests = match &authored.tests_text {
        Some(tests_text) => {
            let mut splice = items_text.clone();
            splice.push('\n');
            splice.push_str(tests_text);
            let sp = oracle.compile_and_run("splice", &splice);
            if !sp.compiled {
                return ArmScore {
                    strict,
                    alpha,
                    compiled: true,
                    tests: None,
                    self_tests: None,
                    verdict: ArmVerdict::UndecidedSignature,
                    note: format!(
                        "authored tests do not compile against the redraft: {}",
                        sp.detail
                    ),
                };
            }
            Some(sp.tests_passed == Some(true))
        }
        None => None, // vacuous: the authored arm carries no test surface
    };
    // (3) the writer's own tests — disclosed only.
    let self_tests = if own_tests.is_empty() {
        None
    } else {
        let mut self_src = items_text.clone();
        self_src.push('\n');
        self_src.push_str(&file_text(&file_from(&redraft_attrs, own_tests)));
        oracle.compile_and_run("self_tests", &self_src).tests_passed
    };
    if tests == Some(false) {
        return ArmScore {
            strict,
            alpha,
            compiled: true,
            tests,
            self_tests,
            verdict: ArmVerdict::NoMatch,
            note: String::from("authored tests fail against the redraft"),
        };
    }
    let tests_vacuous = tests.is_none();
    let verdict = if strict || alpha {
        ArmVerdict::Match { tests_vacuous }
    } else {
        ArmVerdict::UndecidedShape
    };
    ArmScore {
        strict,
        alpha,
        compiled: true,
        tests,
        self_tests,
        verdict,
        note: String::new(),
    }
}

// ── The audit driver ─────────────────────────────────────────────────────

/// One rule's paired outcome.
pub struct RuleOutcome {
    pub domain: Domain,
    pub rule_id: String,
    /// `Some(reason)` when the authored fixed arm does not parse as Rust —
    /// the rule is UNSCOREABLE (excluded from Δ, count printed).
    pub unscoreable: Option<String>,
    pub desc: Option<ArmScore>,
    pub ctrl: Option<ArmScore>,
}

/// The full paired audit over the frozen sample.
///
/// # Errors
///
/// The [`AuditError`] taxonomy (digest / sample / blind fixture / io).
pub fn run_audit(
    writer: &mut dyn RebuildWriter,
    oracle: &dyn CompileOracle,
    cap_lines: Vec<String>,
) -> Result<AuditReport, AuditError> {
    let input = load_input()?;
    let mut rules = Vec::new();
    for (idx, fx) in input.fixtures.iter().enumerate() {
        let authored = match authored_arms(&fx.buggy, &fx.fixed) {
            Ok(a) => a,
            Err(e) => {
                rules.push(RuleOutcome {
                    domain: fx.domain,
                    rule_id: fx.rule_id.clone(),
                    unscoreable: Some(format!("authored fixed arm does not parse as Rust: {e}")),
                    desc: None,
                    ctrl: None,
                });
                continue;
            }
        };
        let desc_req = RedraftRequest {
            arm: ArmKind::Description,
            rule_index: idx,
            description: fx.description.clone(),
            buggy_arm: fx.buggy.clone(),
            signatures: authored.signatures.clone(),
        };
        let ctrl_req = RedraftRequest {
            arm: ArmKind::Control,
            rule_index: idx,
            description: fx.description.clone(),
            buggy_arm: fx.buggy.clone(),
            signatures: authored.signatures.clone(),
        };
        let d = writer.redraft(&desc_req);
        let c = writer.redraft(&ctrl_req);
        rules.push(RuleOutcome {
            domain: fx.domain,
            rule_id: fx.rule_id.clone(),
            unscoreable: None,
            desc: Some(score_arm(&authored, &d, oracle)),
            ctrl: Some(score_arm(&authored, &c, oracle)),
        });
    }
    Ok(AuditReport {
        canonical_digest: input.canonical_digest,
        composed_at_head: input.composed_at_head,
        writer_label: writer.label(),
        cap_lines,
        rules,
    })
}

#[derive(Default)]
struct DomainStats {
    scoreable: usize,
    desc_match: usize,
    ctrl_match: usize,
    desc_vacuous: usize,
    ctrl_vacuous: usize,
    desc_ushape: usize,
    ctrl_ushape: usize,
    desc_usig: usize,
    ctrl_usig: usize,
    desc_nomatch: usize,
    ctrl_nomatch: usize,
    desc_fence: usize,
    ctrl_fence: usize,
    desc_declined: usize,
    ctrl_declined: usize,
}

impl DomainStats {
    fn rate(&self, desc: bool) -> Option<f64> {
        if self.scoreable == 0 {
            return None;
        }
        let m = if desc {
            self.desc_match
        } else {
            self.ctrl_match
        };
        Some(
            f64::from(u32::try_from(m).unwrap_or(u32::MAX)) * 100.0
                / f64::from(u32::try_from(self.scoreable).unwrap_or(u32::MAX)),
        )
    }

    fn accumulate(&mut self, other: &Self) {
        self.scoreable += other.scoreable;
        self.desc_match += other.desc_match;
        self.ctrl_match += other.ctrl_match;
        self.desc_vacuous += other.desc_vacuous;
        self.ctrl_vacuous += other.ctrl_vacuous;
        self.desc_ushape += other.desc_ushape;
        self.ctrl_ushape += other.ctrl_ushape;
        self.desc_usig += other.desc_usig;
        self.ctrl_usig += other.ctrl_usig;
        self.desc_nomatch += other.desc_nomatch;
        self.ctrl_nomatch += other.ctrl_nomatch;
        self.desc_fence += other.desc_fence;
        self.ctrl_fence += other.ctrl_fence;
        self.desc_declined += other.desc_declined;
        self.ctrl_declined += other.ctrl_declined;
    }
}

/// The audit report (render-only — nothing mutates, nothing promotes).
/// The oracle's disclosure rides `cap_lines` (the replay self-test labels
/// its fake oracle there; the hosted posture names rustc).
pub struct AuditReport {
    pub canonical_digest: String,
    pub composed_at_head: String,
    pub writer_label: String,
    pub cap_lines: Vec<String>,
    pub rules: Vec<RuleOutcome>,
}

impl AuditReport {
    fn count_note_markers(score: Option<&ArmScore>, fence: &mut usize, declined: &mut usize) {
        if let Some(s) = score {
            if s.note.contains("fence-refusal") {
                *fence += 1;
            }
            if s.note.starts_with("declined:") {
                *declined += 1;
            }
        }
    }

    /// The report text. Exit stays 0 on every successful read.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        let unscoreable: Vec<&RuleOutcome> = self
            .rules
            .iter()
            .filter(|r| r.unscoreable.is_some())
            .collect();
        let scoreable = self.rules.len() - unscoreable.len();
        self.render_header(&mut out);
        self.render_census(&mut out, &unscoreable, scoreable);
        let mut stats: BTreeMap<Domain, DomainStats> = BTreeMap::new();
        self.render_rows(&mut out, &mut stats);
        Self::render_summary(&mut out, &stats, unscoreable.len(), scoreable);
        out
    }

    fn render_header(&self, out: &mut String) {
        let _ = writeln!(
            out,
            "rule rebuild audit — issue 242 (the iWSP text-bottleneck fidelity metric)"
        );
        let _ = writeln!(
            out,
            "descriptions: canonical digest {} (composed at {}) · sample v1 freeze a5ad3f9f… · \
             {} rules (10/10/10 clippy_lints/rust_perf/kernel_opt)",
            self.canonical_digest,
            self.composed_at_head,
            self.rules.len()
        );
        let _ = writeln!(out, "writer: {}", self.writer_label);
        for line in &self.cap_lines {
            let _ = writeln!(out, "{line}");
        }
    }

    fn render_census(&self, out: &mut String, unscoreable: &[&RuleOutcome], scoreable: usize) {
        if unscoreable.is_empty() {
            let _ = writeln!(
                out,
                "UNSCOREABLE: none · scoreable {scoreable}/{}",
                self.rules.len()
            );
        } else {
            let names: Vec<String> = unscoreable
                .iter()
                .map(|r| format!("{}/{}", r.domain, r.rule_id))
                .collect();
            let _ = writeln!(
                out,
                "UNSCOREABLE (authored arms not Rust — excluded from Δ): {} · scoreable \
                 {scoreable}/{}",
                names.join(", "),
                self.rules.len()
            );
        }
        if scoreable < LOW_POWER_SCOREABLE {
            let _ = writeln!(
                out,
                "⚠ low-power warning: scoreable n={scoreable} < {LOW_POWER_SCOREABLE} — a 10-point \
                 Δ gap is only a couple of rules at this n; treat the bars accordingly"
            );
        }
    }

    fn render_rows(&self, out: &mut String, stats: &mut BTreeMap<Domain, DomainStats>) {
        let _ = writeln!(
            out,
            "\nper-rule (S=strict A=alpha C=compiled T=tests(pass/fail/vac) ST=self-tests):"
        );
        for r in &self.rules {
            let entry = stats.entry(r.domain).or_default();
            if let Some(reason) = &r.unscoreable {
                let _ = writeln!(out, "  {} {}\tUNSCOREABLE ({reason})", r.domain, r.rule_id);
                continue;
            }
            entry.scoreable += 1;
            let Some(d) = &r.desc else { continue };
            let Some(c) = &r.ctrl else { continue };
            let tests_str = |t: &Option<bool>| match t {
                Some(true) => "pass",
                Some(false) => "fail",
                None => "vac",
            };
            let st_str = |t: &Option<bool>| match t {
                Some(true) => "pass",
                Some(false) => "fail",
                None => "-",
            };
            let note = |s: &ArmScore| {
                if s.note.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", tail(&s.note, 80))
                }
            };
            let _ = writeln!(
                out,
                "  {} {}\tdesc: S={} A={} C={} T={} ST={} → {}{} | ctrl: S={} A={} C={} T={} \
                 ST={} → {}{}",
                r.domain,
                r.rule_id,
                u8::from(d.strict),
                u8::from(d.alpha),
                u8::from(d.compiled),
                tests_str(&d.tests),
                st_str(&d.self_tests),
                d.verdict,
                note(d),
                u8::from(c.strict),
                u8::from(c.alpha),
                u8::from(c.compiled),
                tests_str(&c.tests),
                st_str(&c.self_tests),
                c.verdict,
                note(c),
            );
            if matches!(d.verdict, ArmVerdict::Match { .. }) {
                entry.desc_match += 1;
                if d.tests.is_none() {
                    entry.desc_vacuous += 1;
                }
            }
            if matches!(c.verdict, ArmVerdict::Match { .. }) {
                entry.ctrl_match += 1;
                if c.tests.is_none() {
                    entry.ctrl_vacuous += 1;
                }
            }
            if d.verdict == ArmVerdict::UndecidedShape {
                entry.desc_ushape += 1;
            }
            if c.verdict == ArmVerdict::UndecidedShape {
                entry.ctrl_ushape += 1;
            }
            if d.verdict == ArmVerdict::UndecidedSignature {
                entry.desc_usig += 1;
            }
            if c.verdict == ArmVerdict::UndecidedSignature {
                entry.ctrl_usig += 1;
            }
            if d.verdict == ArmVerdict::NoMatch {
                entry.desc_nomatch += 1;
            }
            if c.verdict == ArmVerdict::NoMatch {
                entry.ctrl_nomatch += 1;
            }
            Self::count_note_markers(
                r.desc.as_ref(),
                &mut entry.desc_fence,
                &mut entry.desc_declined,
            );
            Self::count_note_markers(
                r.ctrl.as_ref(),
                &mut entry.ctrl_fence,
                &mut entry.ctrl_declined,
            );
        }
    }

    fn render_summary(
        out: &mut String,
        stats: &BTreeMap<Domain, DomainStats>,
        unscoreable_n: usize,
        scoreable: usize,
    ) {
        let _ = writeln!(out, "\nper-domain fix-match (scoreable n):");
        let mut overall = DomainStats::default();
        for (domain, s) in stats {
            let d_rate = s.rate(true);
            let c_rate = s.rate(false);
            let delta = match (d_rate, c_rate) {
                (Some(d), Some(c)) => format!("{:+.1} pts", d - c),
                _ => String::from("n/a"),
            };
            let _ = writeln!(
                out,
                "  {}: desc {} · ctrl {} · Δ {} (n={})",
                domain,
                fmt_rate(d_rate, s.desc_match),
                fmt_rate(c_rate, s.ctrl_match),
                delta,
                s.scoreable
            );
            overall.accumulate(s);
        }
        let d_rate = overall.rate(true);
        let c_rate = overall.rate(false);
        let delta = match (d_rate, c_rate) {
            (Some(d), Some(c)) => Some(d - c),
            _ => None,
        };
        let _ = writeln!(
            out,
            "OVERALL: desc {} · ctrl {} · Δ {} (scoreable n={scoreable}; UNSCOREABLE excluded: \
             {unscoreable_n})",
            fmt_rate(d_rate, overall.desc_match),
            fmt_rate(c_rate, overall.ctrl_match),
            delta.map_or_else(|| String::from("n/a"), |d| format!("{d:+.1} pts")),
        );
        let _ = writeln!(
            out,
            "undecided-shape: desc {} · ctrl {} | undecided-signature: desc {} · ctrl {} | \
             fence-refusals: desc {} · ctrl {} | declines: desc {} · ctrl {}",
            overall.desc_ushape,
            overall.ctrl_ushape,
            overall.desc_usig,
            overall.ctrl_usig,
            overall.desc_fence,
            overall.ctrl_fence,
            overall.desc_declined,
            overall.ctrl_declined
        );
        let _ = writeln!(
            out,
            "vacuous tests (MATCH rests on the alpha/strict columns alone): desc {} · ctrl {}",
            overall.desc_vacuous, overall.ctrl_vacuous
        );
        // The pre-registered bars.
        match delta {
            Some(d) if d < 10.0 => {
                let _ = writeln!(
                    out,
                    "KILL BAR FIRED (Δ {d:+.1} < 10 pts) — the descriptions carry no re-derivable \
                     signal beyond the task prompts; the audit closes negative (record it, retire \
                     the instrument)."
                );
            }
            Some(d) => {
                let _ = writeln!(
                    out,
                    "KILL BAR not fired (Δ {d:+.1} ≥ 10 pts). PASS BAR candidate — inspect the \
                     per-rule rows above for low-fidelity families; the only promotion-worthy \
                     outcome is a rule-quality defect the existing gates missed (a NEW \
                     pre-registration gates any promotion)."
                );
            }
            None => {
                let _ = writeln!(out, "bars: n/a (no scoreable rules)");
            }
        }
        let _ = writeln!(
            out,
            "report-only — no promotion path, no corpus mutation (issue 242 T1; the descriptions \
             freeze law holds: any description change lands only as a new digest version)"
        );
    }
}

fn fmt_rate(rate: Option<f64>, matches: usize) -> String {
    match rate {
        Some(r) => format!("{matches} ({r:.1}%)"),
        None => String::from("n/a"),
    }
}

// ── Seat preflight + the CLI entry ───────────────────────────────────────

struct SeatPreflight {
    label: String,
    endpoint: String,
    model: String,
}

/// Find the first armed `hosted_expert` row (label order = the manifest's
/// [`DOMAIN_LABELS`]); refuse naming the arming pieces otherwise.
///
/// # Errors
///
/// A message naming the arming pieces when no row qualifies.
fn seat_preflight_from(
    manifest: Option<&crate::escalation_manifest::EscalationManifest>,
) -> Result<SeatPreflight, String> {
    for label in crate::escalation_manifest::DOMAIN_LABELS {
        let Some(m) = manifest else {
            break;
        };
        if !m.is_armed(label) {
            continue;
        }
        let Some(row) = m.row(label) else {
            continue;
        };
        if row.target.kind != crate::escalation_manifest::TargetKind::HostedExpert.label() {
            continue;
        }
        let Some(endpoint) = row.target.endpoint.clone() else {
            continue;
        };
        let Some(model) = row.target.model.clone() else {
            continue;
        };
        return Ok(SeatPreflight {
            label: (*label).to_string(),
            endpoint,
            model,
        });
    }
    Err(String::from(
        "rule-rebuild-audit: no armed hosted_expert row in .refine/escalation_manifest.json — \
         arming needs a row { target { kind: \"hosted_expert\", endpoint, model } } on some domain \
         (the file ships ALL-EMPTY by design; the seat is the owner's credential act, issue 242)",
    ))
}

/// The CLI entry (`--rule-rebuild-audit`). Report text on `Ok`; `(message,
/// exit-code)` on a refusal (2 = posture/arming refusal, 1 = run failure).
///
/// # Errors
///
/// The refusal taxonomy: cap-not-recorded, seat-not-armed, consent-not-armed,
/// unknown writer mode (feature-absent is handled in the bin, pre-dispatch).
pub fn cli_run(cap_usd: Option<f64>, writer_mode: Option<&str>) -> Result<String, (String, i32)> {
    match writer_mode {
        Some("replay") => {
            let mut writer =
                ReplayWriter::new(ReplayMode::Identity).map_err(|e| (e.to_string(), 1))?;
            let oracle = FakeOracle::plain(Box::new(|_, _| true));
            let report = run_audit(
                &mut writer,
                &oracle,
                vec![String::from(
                    "oracle: FAKE (replay self-test — the compiled column is not evidence; \
                     this posture exists to print the UNSCOREABLE census + prove the pipeline)",
                )],
            )
            .map_err(|e| (e.to_string(), 1))?;
            Ok(report.render())
        }
        None | Some("hosted") => {
            let Some(cap) = cap_usd else {
                return Err((
                    String::from(
                        "rule-rebuild-audit: the hosted posture requires --rule-rebuild-cap <usd> \
                         — refuse to start without the cap recorded (issue 242; the dspy-wsp \
                         writer-spend datum bounds the whole run at $10)",
                    ),
                    2,
                ));
            };
            if !cap.is_finite() || cap < 0.0 {
                return Err((
                    format!("--rule-rebuild-cap expects a non-negative finite number, got {cap}"),
                    2,
                ));
            }
            let manifest_path = Path::new(crate::escalation_manifest::ESCALATION_MANIFEST_FILE);
            let manifest = crate::escalation_manifest::EscalationManifest::load(manifest_path).ok();
            let seat = seat_preflight_from(manifest.as_ref()).map_err(|m| (m, 2))?;
            // Consent preflight: EVERY call would decline pre-socket — one
            // loud refusal beats 60 declines.
            if let crate::code_egress::EgressConsent::Refused(r) =
                crate::code_egress::EgressConsent::from_env()
            {
                return Err((
                    format!(
                        "rule-rebuild-audit: code-egress consent is not armed ({r:?}) — the run \
                         sends rule descriptions/snippets to the seat; set \
                         RIIR_REFINE_CODE_EGRESS=snippet to arm it"
                    ),
                    2,
                ));
            }
            let env_cap = crate::llm_spend::cap_from_env();
            let effective = cap.min(env_cap);
            let ledger = crate::llm_spend::SpendLedger::open_at_with_cap(
                Path::new(".refine").join(crate::llm_spend::SpendLedger::FILE_NAME),
                effective,
            );
            let expert = crate::hosted_expert::HostedExpertL4::new(
                seat.endpoint.clone(),
                seat.model.clone(),
                ledger,
            );
            let token_note = if std::env::var(crate::hosted_expert::TOKEN_ENV)
                .is_ok_and(|t| !t.trim().is_empty())
            {
                String::from("token: present")
            } else {
                String::from(
                    "token: ABSENT (empty bearer — loopback experts only; a hosted tier will \
                     refuse at the wire)",
                )
            };
            let mut writer = HostedWriter::new(expert, MAX_TOKENS_PER_CALL, PER_CALL_DEADLINE);
            let cap_lines = vec![
                format!("cap: ${cap:.2} (arg) vs env ${env_cap:.2} → effective ${effective:.2}"),
                format!("seat: {} {} ({})", seat.label, seat.model, seat.endpoint),
                token_note,
                String::from("oracle: rustc --edition 2021 --test"),
            ];
            let report = run_audit(&mut writer, &RustcOracle::default(), cap_lines)
                .map_err(|e| (e.to_string(), 1))?;
            Ok(report.render())
        }
        Some(other) => Err((
            format!("unknown --rule-rebuild-writer {other:?} (replay | hosted)"),
            2,
        )),
    }
}
