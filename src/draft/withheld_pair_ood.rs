//! Issue 062 T1/T2 — withheld-(role, filler) OOD generalization.
//!
//! ## The axis nothing here measured
//!
//! Every retrieval floor this repo ships is **in-distribution by
//! construction**: the query is a variant of a corpus entry whose
//! (rule, shape) pair is present in the index. Research 527
//! (arXiv:2608.29530) predicts the opposite regime is where such systems
//! break — a store of (role, filler) *conjunctions* memorizes atoms and
//! fails on **unseen combinations of seen roles and seen fillers**.
//!
//! - **role** = the rule identity (clippy lint name / rustc error code);
//! - **filler** = a **shape-class** of the code the rule is embedded in,
//!   derived deterministically from the span text.
//!
//! The protocol (preregistered in `.benchmarks/062_withheld_pair_ood.md`
//! BEFORE any run) withholds a whole (role, filler) **cell from the INDEX**
//! — not merely from the eval set — and reports in-distribution vs
//! withheld-pair ranking side by side, paired per query, against an
//! atomic-memorization null.
//!
//! ## Why the shape-class is a CONTEXT feature
//!
//! If the shape-class were read off the lint-identifying construct
//! (`match` for `match_bool`, `for … in` for `needless_range_loop`), role and
//! filler would be **collinear**: every item of a role would land in one
//! cell, no unseen combination could exist, and the protocol would be
//! vacuous. So [`ShapeClass`] is derived from the *containing* context
//! (method vs free fn, generic, closure, iterator chain, macro, loop,
//! multi-item), which is orthogonal to the anti-pattern for most rules. The
//! rules for which it is NOT orthogonal are reported as
//! [`WithheldReport::collinear_rules`] — that is itself a corpus finding.
//!
//! ## Three arms, one run, paired
//!
//! | arm | index | what it says |
//! |---|---|---|
//! | `ID` | `P \ {q}` | the pair IS represented (leave-one-out) |
//! | `OOD` | `P \ cell(q)` | the pair is ABSENT (leave-cell-out) |
//! | `NOROLE` | `P \ {items with q's rule}` | the ROLE is absent — must score 0 |
//!
//! `NOROLE` is the dead-instrument canary. It is 0.0% **by construction**
//! (an unindexed rule cannot be retrieved), so a nonzero `NOROLE` means the
//! harness is not rebuilding the index / not reading the rank — a defect,
//! not a finding. It disambiguates "OOD at ceiling" (a real result) from
//! "withholding never happened" (a broken instrument), which `ID == OOD`
//! alone cannot.
//!
//! Both scored arms run **in the same call over the same query**, and the
//! report exposes per-row ranks so the summary is a paired statistic, never
//! `median(A)` vs `median(B)`.
//!
//! Modelless: deterministic string classification + the shipped retrieval
//! stack. No weights, no training, no GPU.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::corpus_index::{RetrievalConfig, RuleIndex};
use super::domain_router::DomainId;
use super::instantiation_rerank::{mark_instantiation, rerank, RerankMode, RERANK_POOL};

// ── shape classes ───────────────────────────────────────────────────────

/// Structural **context** class of a span — the filler axis.
///
/// Priority is most-specific-first and **first match wins**; the order is
/// part of the specification (pinned by `shape_classes_are_pinned`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShapeClass {
    /// Invokes a macro (`format!`, `println!`, `write!`, `vec!`, `assert*!`, `panic!`).
    Macro,
    /// An iterator/adapter chain (`.iter()`, `.map(`, `.filter(`, `.collect(`, `.sort_by`).
    IteratorChain,
    /// Carries a closure header (`|x|`, `||`, `move |`).
    Closure,
    /// Generic or lifetime parameters (`<'`, `<T`, `impl Fn`).
    Generic,
    /// The rule sits inside an `impl` block (a method, not a free fn).
    ImplBlock,
    /// Inside a `for … in` / `while` loop.
    Loop,
    /// Two or more top-level items.
    MultiItem,
    /// None of the above — a single free fn body.
    PlainFn,
}

impl ShapeClass {
    /// Fixed order = the classifier's priority order.
    pub const ALL: [Self; 8] = [
        Self::Macro,
        Self::IteratorChain,
        Self::Closure,
        Self::Generic,
        Self::ImplBlock,
        Self::Loop,
        Self::MultiItem,
        Self::PlainFn,
    ];

    /// Stable label — the report key and the atomic-null signature prefix.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Macro => "macro",
            Self::IteratorChain => "iter-chain",
            Self::Closure => "closure",
            Self::Generic => "generic",
            Self::ImplBlock => "impl-block",
            Self::Loop => "loop",
            Self::MultiItem => "multi-item",
            Self::PlainFn => "plain-fn",
        }
    }
}

/// Count of non-overlapping occurrences of `needle` in `hay`.
fn count(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

/// Iterator/adapter-chain markers — the `IteratorChain` predicate's table.
const ITER_MARKERS: [&str; 8] = [
    ".iter()",
    ".into_iter()",
    ".iter_mut()",
    ".map(",
    ".filter(",
    ".collect(",
    ".sort_by",
    ".chars()",
];

/// Does the text invoke one of the known macros?
fn has_macro(code: &str) -> bool {
    const MACROS: [&str; 10] = [
        "format!",
        "println!",
        "print!",
        "eprintln!",
        "write!",
        "writeln!",
        "vec!",
        "assert!",
        "assert_eq!",
        "panic!",
    ];
    MACROS.iter().any(|m| code.contains(m))
}

/// Does the text carry a closure header? `|` used as a closure delimiter,
/// deliberately excluding the bit-or / pattern-or spellings ` | `.
fn has_closure(code: &str) -> bool {
    if code.contains("move |") {
        return true;
    }
    let b = code.as_bytes();
    for (i, &c) in b.iter().enumerate() {
        if c != b'|' {
            continue;
        }
        // `||` (nullary closure or logical-or): closure iff followed by a
        // block/expression start rather than an operand — approximate by
        // requiring the previous non-space byte to be `(`, `,` or `=`.
        let prev = b[..i].iter().rev().find(|c| !c.is_ascii_whitespace());
        let opens = matches!(prev, Some(b'(' | b',' | b'=' | b'{'));
        if !opens {
            continue;
        }
        // `|x|` / `||`
        if b.get(i + 1) == Some(&b'|') {
            return true;
        }
        if let Some(rest) = code.get(i + 1..) {
            if let Some(close) = rest.find('|') {
                let inner = &rest[..close];
                if !inner.is_empty()
                    && inner.chars().all(|c| {
                        c.is_ascii_alphanumeric() || c == '_' || c == ',' || c == ' ' || c == '&'
                    })
                {
                    return true;
                }
            }
        }
    }
    false
}

/// The shape-class of a span — deterministic, first-match-wins.
///
/// See [`ShapeClass`] for the class list and the module docs for why the
/// features are *context* features rather than the anti-pattern itself.
#[must_use]
pub fn classify_shape(code: &str) -> ShapeClass {
    if has_macro(code) {
        return ShapeClass::Macro;
    }
    if ITER_MARKERS.iter().any(|m| code.contains(m)) {
        return ShapeClass::IteratorChain;
    }
    if has_closure(code) {
        return ShapeClass::Closure;
    }
    if code.contains("<'") || code.contains("<T") || code.contains("impl Fn") {
        return ShapeClass::Generic;
    }
    if code.contains("impl ") {
        return ShapeClass::ImplBlock;
    }
    if (code.contains("for ") && code.contains(" in ")) || code.contains("while ") {
        return ShapeClass::Loop;
    }
    let items =
        count(code, "fn ") + count(code, "struct ") + count(code, "enum ") + count(code, "const ");
    if items >= 2 {
        return ShapeClass::MultiItem;
    }
    ShapeClass::PlainFn
}

/// The repair-relevant PAYLOAD of a `rustc_errors` corpus case: the broken
/// source with the appended `#[cfg(test)] mod ground_truth { … }` behaviour
/// harness removed.
///
/// The ground-truth module is generator scaffolding present on **every**
/// case, so classifying with it included makes the filler an artifact of the
/// harness rather than of the borrow pattern. It stays in the INDEXED text
/// (that is the corpus datum as shipped) and is excluded only from the
/// classifier.
#[must_use]
pub fn rustc_payload(code: &str) -> &str {
    code.split("\n#[cfg(test)]").next().unwrap_or(code)
}

/// Drop every `#[…]` attribute span. Without this, `#[test]` reads as an
/// index expression `[test]` — the measured defect that made `index-expr`
/// absorb four of eight error codes on the first run.
fn strip_attributes(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let b = code.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        if b[i] == b'#' && matches!(b.get(i + 1), Some(b'[' | b'!')) {
            // Skip to the matching `]` (attributes do not nest in practice,
            // but count depth so `#[cfg(any(a, b))]` is handled).
            let mut depth = 0usize;
            while i < b.len() {
                match b[i] {
                    b'[' => depth += 1,
                    b']' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            continue;
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

/// Structural class of a **borrow-checker** span — the T2 filler axis.
///
/// Same discipline as [`ShapeClass`] (first match wins, priority = order),
/// but the features are the borrow-relevant *data carrier* rather than the
/// generic code context: what is being borrowed decides which repair
/// playbook applies, so it is the natural filler for the `rustc_errors` role
/// (the error code).
///
/// Applied to [`rustc_payload`] with attributes stripped — see those docs
/// for the two artifacts this avoids.
#[must_use]
pub fn classify_rustc_shape(code: &str) -> &'static str {
    let payload = strip_attributes(rustc_payload(code));
    let code = payload.as_str();
    if has_closure(code) {
        return "closure-capture";
    }
    if code.contains("format!") {
        return "format-macro";
    }
    if code.contains("[..]") {
        return "slice-borrow";
    }
    if code.contains(".as_str()") || code.contains(".as_slice()") || code.contains(".as_ref()") {
        return "as-view";
    }
    // An index expression `[<ident-or-int>]` — distinguished from the slice
    // form above, which already returned.
    if code.split('[').skip(1).any(|s| {
        s.split(']').next().is_some_and(|i| {
            !i.is_empty() && i.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    }) {
        return "index-expr";
    }
    if code.contains("&mut ") {
        return "mut-borrow";
    }
    if code.contains("struct ") {
        return "struct-field";
    }
    if code.contains("vec![") || code.contains("Vec<") {
        return "vec-owner";
    }
    if code.contains("String") {
        return "string-owner";
    }
    "plain-binding"
}

// ── the atomic-memorization null ────────────────────────────────────────────

// `pub(crate)`: Issue 186's synth-rename transform reuses the same keyword
// table (a rename that touched a keyword would change the atomic key).
pub(crate) const RUST_KEYWORDS: [&str; 36] = [
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while",
];

/// Normalize a span to its structural token stream: keywords kept verbatim,
/// every other identifier collapsed to `I`, every literal to `L`, and each
/// maximal punctuation run kept as one `op:<run>` token. Comments dropped.
///
/// This is the memorizer's key material: it erases naming but preserves the
/// exact structural conjunction, which is precisely what an atomic store
/// remembers.
#[must_use]
pub fn normalized_tokens(code: &str) -> Vec<String> {
    let b = code.as_bytes();
    let mut out = Vec::with_capacity(code.len() / 3);
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // line comment
        if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // string / char literal → one `L`
        if c == b'"' || c == b'\'' {
            let quote = c;
            let mut j = i + 1;
            let mut closed = false;
            while j < b.len() {
                if b[j] == b'\\' {
                    j += 2;
                    continue;
                }
                if b[j] == quote {
                    closed = true;
                    j += 1;
                    break;
                }
                j += 1;
            }
            if closed {
                out.push("L".to_string());
                i = j;
                continue;
            }
            // Unterminated (a lifetime `'a` reads as an open char literal):
            // fall through to punctuation handling for the quote byte.
        }
        if c.is_ascii_alphanumeric() || c == b'_' {
            let start = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let word = &code[start..i];
            if word.starts_with(|ch: char| ch.is_ascii_digit()) {
                out.push("L".to_string());
            } else if RUST_KEYWORDS.contains(&word) {
                out.push(word.to_string());
            } else {
                out.push("I".to_string());
            }
            continue;
        }
        let start = i;
        while i < b.len() {
            let d = b[i];
            if d.is_ascii_whitespace() || d.is_ascii_alphanumeric() || d == b'_' {
                break;
            }
            i += 1;
        }
        out.push(format!("op:{}", &code[start..i]));
    }
    out.shrink_to_fit();
    out
}

/// A pure memorizer: `BLAKE3(shape_label ‖ 0x00 ‖ key)` → the set of rules
/// stored under that exact conjunction.
///
/// Two keys, both shape-class-prefixed:
/// - `sig` — the ordered [`normalized_tokens`] stream;
/// - `bag` — the same tokens **sorted** (order-insensitive, strictly
///   coarser, therefore a strictly stronger null).
///
/// Because the shape label is part of the key, signature equality implies
/// shape equality — which is what makes the withheld-pair score of this null
/// **zero by construction** (see [`WithheldReport`]).
pub struct AtomicNull {
    sig: HashMap<[u8; 32], BTreeSet<String>>,
    bag: HashMap<[u8; 32], BTreeSet<String>>,
}

fn key(shape: &str, tokens: &[String]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(shape.as_bytes());
    h.update(&[0u8]);
    for t in tokens {
        h.update(t.as_bytes());
        h.update(&[0u8]);
    }
    *h.finalize().as_bytes()
}

/// The two atomic keys of one span.
#[must_use]
pub fn atomic_keys(code: &str, shape: &str) -> ([u8; 32], [u8; 32]) {
    let toks = normalized_tokens(code);
    let sig = key(shape, &toks);
    let mut sorted = toks;
    sorted.sort_unstable();
    let bag = key(shape, &sorted);
    (sig, bag)
}

impl AtomicNull {
    /// Build over an index population.
    #[must_use]
    pub fn build<'a>(items: impl Iterator<Item = &'a Binding>) -> Self {
        let mut sig: HashMap<[u8; 32], BTreeSet<String>> = HashMap::new();
        let mut bag: HashMap<[u8; 32], BTreeSet<String>> = HashMap::new();
        for it in items {
            let (s, g) = atomic_keys(&it.text, it.shape);
            sig.entry(s).or_default().insert(it.rule.clone());
            bag.entry(g).or_default().insert(it.rule.clone());
        }
        Self { sig, bag }
    }

    /// Does the memorizer answer `rule` for this query? `(sig_hit, bag_hit)`.
    #[must_use]
    pub fn hits(&self, code: &str, shape: &str, rule: &str) -> (bool, bool) {
        let (s, g) = atomic_keys(code, shape);
        (
            self.sig.get(&s).is_some_and(|r| r.contains(rule)),
            self.bag.get(&g).is_some_and(|r| r.contains(rule)),
        )
    }
}

// ── population ──────────────────────────────────────────────────────────

/// One binding: a rule id, its shape-class label, the span text, and where
/// the datum came from.
///
/// **The two consumers assign the role/filler axes OPPOSITELY, on purpose.**
/// This protocol's withheld-pair arms treat `rule` as the thing being
/// retrieved (the "role" in [`ArmRow::role_fillers`]); the TPR fit in
/// [`crate::draft::tpr_fit`] does the reverse — role = `shape`, filler =
/// `rule` — because `n_bind_slots == arity` in `TprScheme::Orthogonal` caps
/// the role count at the number of blocks, while the filler count is
/// unconstrained, and only the filler axis can carry `kernel_opt`'s 426 rule
/// ids (Bench 063 §2, the forced scheme decision). Check which axis a
/// consumer means before reading a census off it.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The rule id / lint name / error code. TPR **filler** axis.
    pub rule: String,
    /// A shape-class label (see [`classify_shape`]). TPR **role** axis.
    pub shape: &'static str,
    /// The span text (what gets indexed / queried).
    pub text: String,
    /// Provenance tag for the report (`corpus`, `fixture`, `real-repo`, …).
    pub source: &'static str,
}

impl Binding {
    /// Classify `text` with the clippy-domain [`classify_shape`].
    #[must_use]
    pub fn clippy(rule: impl Into<String>, text: impl Into<String>, source: &'static str) -> Self {
        let text = text.into();
        let shape = classify_shape(&text).label();
        Self {
            rule: rule.into(),
            shape,
            text,
            source,
        }
    }

    /// Classify `text` with the borrowck [`classify_rustc_shape`].
    #[must_use]
    pub fn rustc(rule: impl Into<String>, text: impl Into<String>, source: &'static str) -> Self {
        let text = text.into();
        let shape = classify_rustc_shape(&text);
        Self {
            rule: rule.into(),
            shape,
            text,
            source,
        }
    }
}

/// The compiled-in half of the T1 population: the `clippy_lints` corpus
/// entries + the single-lint eval fixtures.
///
/// The 20 oracle-labeled REAL-repo spans (Plan 049 T0.1,
/// `.benchmarks/data/049_t01/spans.jsonl`) are appended by the caller — the
/// library never reads the filesystem.
///
/// `eval_fixtures_composite::COMPOSITE_SPANS` is deliberately NOT indexable:
/// those spans are multi-label, and [`RuleIndex::build_with`] deduplicates
/// identical chunk content by `zone_hash`, so the same text cannot carry two
/// rule ids — the second binding would silently vanish.
#[cfg(feature = "clippy_lints")]
#[must_use]
pub fn clippy_population() -> Vec<Binding> {
    use crate::domain::{HealDomain, HealEntry};
    let mut out = Vec::with_capacity(128);
    for e in crate::domains::clippy_lints::ClippyLintDomain::build_corpus() {
        out.push(Binding::clippy(e.rule_id(), e.buggy_code(), "corpus"));
    }
    for (lint, code) in super::eval_fixtures_clippy::CLIPPY_SPANS {
        out.push(Binding::clippy(*lint, *code, "fixture"));
    }
    out.shrink_to_fit();
    out
}

/// The T2 population: the 200-case stratified generated corpus + the 12
/// committed anchors, role = error code, filler = borrowck shape-class,
/// text = the BROKEN source.
///
/// # Panics
///
/// Panics if a committed anchor's declared mutation site is missing from its
/// seed — corpus-integrity failure, never silently skipped.
#[cfg(feature = "rustc_errors")]
#[must_use]
pub fn rustc_errors_population() -> Vec<Binding> {
    use crate::domains::rustc_errors::{corpus_gen, floor};
    let mut out = Vec::with_capacity(220);
    for g in corpus_gen::generate_corpus() {
        out.push(Binding::rustc(g.expect_code, g.broken, "generated"));
    }
    for c in floor::MUTANT_CASES {
        let broken = c.broken().expect("anchor mutation site present");
        out.push(Binding::rustc(c.expect_code, broken, "anchor"));
    }
    out.shrink_to_fit();
    out
}

/// The `kernel_opt` population (Issue 062 T5b): 426 GPU-optimization rules
/// over their declared categories. Role = the [`KernelCategory`], filler = the
/// rule id, text = the reference (slow) kernel shape.
///
/// This arm is the one that can answer T5's untested prediction ("`kernel_opt`
/// categories possibly weaker than rule-level"), and it is **methodologically
/// the strongest of the three** for a reason worth stating: its role is
/// DECLARED corpus metadata, not a label derived from the span text. The
/// clippy and rustc arms compute their shape class from the same text the
/// state is hashed from, which is Bench 063 §11.7's stated-not-fixed confound
/// — a role there is partly readable off the state. Here it is not, so a
/// structure verdict on this arm is not exposed to that objection.
///
/// [`KernelCategory`]: `crate::domains::kernel_opt::KernelCategory`
#[cfg(feature = "kernel_opt")]
#[must_use]
pub fn kernel_opt_population() -> Vec<Binding> {
    let cats = crate::domains::kernel_opt::corpus::build_corpus_categorized();
    let mut out = Vec::with_capacity(cats.iter().map(|(_, rs)| rs.len()).sum());
    for (cat, rules) in cats {
        for r in rules {
            out.push(Binding {
                rule: r.rule_id,
                shape: cat.as_str(),
                text: r.ref_shape,
                source: "corpus",
            });
        }
    }
    out.shrink_to_fit();
    out
}

// ── the protocol ────────────────────────────────────────────────────────

/// One paired measurement: the same query scored against three indexes.
#[derive(Debug, Clone)]
pub struct ArmRow {
    pub rule: String,
    pub shape: &'static str,
    pub source: &'static str,
    /// `|cell(rule, shape)|` in the full population.
    pub cell_size: usize,
    /// How many DISTINCT fillers this role appears with in the population.
    /// The mechanistically informative covariate: a role seen with one
    /// remaining filler has nothing to compose from.
    pub role_fillers: usize,
    /// How many DISTINCT roles this filler appears with.
    pub filler_roles: usize,
    /// Rank (0-based) of the true rule with the pair PRESENT (`P \ {q}`).
    pub id_rank: Option<usize>,
    /// Rank with the whole pair WITHHELD (`P \ cell`).
    pub ood_rank: Option<usize>,
    /// Rank with the whole ROLE withheld — the dead-instrument canary.
    pub norole_rank: Option<usize>,
    /// Fused score of the true rule in the ID arm (0.0 when not retrieved).
    pub id_score: f32,
    /// Fused score of the true rule in the OOD arm.
    pub ood_score: f32,
    /// Atomic-null verdicts: `(sig, bag)` per arm.
    pub id_null: (bool, bool),
    pub ood_null: (bool, bool),
}

/// Recall@k over a set of ranks.
#[allow(clippy::cast_precision_loss)] // counts are ~10^2
fn recall(ranks: impl Iterator<Item = Option<usize>>, k: usize, n: usize) -> f32 {
    if n == 0 {
        return 0.0;
    }
    let hits = ranks.filter(|r| r.is_some_and(|r| r < k)).count();
    hits as f32 / n as f32 * 100.0
}

/// The full T1/T2 report for one re-rank mode.
#[derive(Debug, Clone)]
pub struct WithheldReport {
    pub mode_label: &'static str,
    /// Index population size (distinct-text bindings).
    pub population: usize,
    pub n_rules: usize,
    pub n_shapes: usize,
    pub n_cells: usize,
    /// Cells passing the three eligibility conditions.
    pub eligible_cells: usize,
    /// `(rule, [(shape, count)])`, both sorted — the printed population matrix.
    pub matrix: Vec<(String, Vec<(&'static str, usize)>)>,
    /// Rules present under exactly ONE shape-class: role/filler collinear,
    /// so no unseen combination of that role can exist.
    pub collinear_rules: Vec<String>,
    pub rows: Vec<ArmRow>,
}

impl WithheldReport {
    #[must_use]
    pub fn n(&self) -> usize {
        self.rows.len()
    }
    #[must_use]
    pub fn id_topk(&self, k: usize) -> f32 {
        recall(self.rows.iter().map(|r| r.id_rank), k, self.n())
    }
    #[must_use]
    pub fn ood_topk(&self, k: usize) -> f32 {
        recall(self.rows.iter().map(|r| r.ood_rank), k, self.n())
    }
    #[must_use]
    pub fn norole_topk(&self, k: usize) -> f32 {
        recall(self.rows.iter().map(|r| r.norole_rank), k, self.n())
    }
    /// `(sig, bag)` atomic-null top-1 with the pair present.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn null_id(&self) -> (f32, f32) {
        let n = self.n().max(1) as f32;
        let s = self.rows.iter().filter(|r| r.id_null.0).count() as f32;
        let b = self.rows.iter().filter(|r| r.id_null.1).count() as f32;
        (s / n * 100.0, b / n * 100.0)
    }
    /// `(sig, bag)` atomic-null top-1 with the pair withheld — 0.0 by construction.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn null_ood(&self) -> (f32, f32) {
        let n = self.n().max(1) as f32;
        let s = self.rows.iter().filter(|r| r.ood_null.0).count() as f32;
        let b = self.rows.iter().filter(|r| r.ood_null.1).count() as f32;
        (s / n * 100.0, b / n * 100.0)
    }
    /// Paired drop in percentage points at top-k.
    #[must_use]
    pub fn drop_pp(&self, k: usize) -> f32 {
        self.id_topk(k) - self.ood_topk(k)
    }
    /// Uniform-random top-k over the role vocabulary — the CHANCE floor.
    ///
    /// The atomic null is zero by construction on withheld pairs, so it
    /// cannot say whether an OOD score is *informative*. Chance can: an OOD
    /// top-1 at `k / n_rules` is a coin flip over the vocabulary, whatever
    /// the null says.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn chance_topk(&self, k: usize) -> f32 {
        if self.n_rules == 0 {
            return 0.0;
        }
        (k.min(self.n_rules) as f32) / (self.n_rules as f32) * 100.0
    }
    /// Per-row rank pairs that DIFFER between the arms — a paired count, the
    /// statistic the workspace's A/B discipline requires instead of two medians.
    #[must_use]
    pub fn paired_moves(&self) -> (usize, usize, usize) {
        let mut worse = 0;
        let mut same = 0;
        let mut better = 0;
        for r in &self.rows {
            let a = r.id_rank.unwrap_or(usize::MAX);
            let b = r.ood_rank.unwrap_or(usize::MAX);
            match b.cmp(&a) {
                std::cmp::Ordering::Greater => worse += 1,
                std::cmp::Ordering::Equal => same += 1,
                std::cmp::Ordering::Less => better += 1,
            }
        }
        (better, same, worse)
    }
    /// OOD top-1 grouped by how many distinct fillers the role is seen
    /// with: `(role_fillers, n, id_top1, ood_top1)`.
    ///
    /// This is the covariate a role-composition mechanism (Phase 2's TPR
    /// role table) would act on — a role with only one remaining filler has
    /// nothing to compose from, so its OOD floor is structural, not a
    /// ranking defect.
    #[must_use]
    pub fn by_role_fillers(&self) -> Vec<(usize, usize, f32, f32)> {
        let mut groups: BTreeMap<usize, Vec<&ArmRow>> = BTreeMap::new();
        for r in &self.rows {
            groups.entry(r.role_fillers).or_default().push(r);
        }
        groups
            .into_iter()
            .map(|(k, rws)| {
                let n = rws.len();
                (
                    k,
                    n,
                    recall(rws.iter().map(|r| r.id_rank), 1, n),
                    recall(rws.iter().map(|r| r.ood_rank), 1, n),
                )
            })
            .collect()
    }

    /// Are the per-row ID scores all identical? A constant score vector is a
    /// harness bug, not a finding (workspace failure ledger).
    #[must_use]
    pub fn scores_are_uniform(&self) -> bool {
        let mut it = self.rows.iter().map(|r| r.id_score);
        let Some(first) = it.next() else {
            return true;
        };
        it.all(|s| (s - first).abs() < f32::EPSILON)
    }
}

/// Build the withheld index's patterns. Shared with the band axis
/// (`band_ood`) — one index builder, never two.
pub(crate) fn build_index(items: &[&Binding]) -> RuleIndex {
    let patterns: Vec<(String, String)> = items
        .iter()
        .map(|b| (b.rule.clone(), b.text.clone()))
        .collect();
    RuleIndex::build_with(patterns, RetrievalConfig::default()).expect("index builds")
}

/// Rank of `rule` under the set-admission policy: the rule's position in
/// the admitted slate (None when not admitted). Recovery-fired slates rank
/// corpus rules that may never have been retrieved — the withheld-pair
/// question ("is the role recoverable from the pattern alone?") is exactly
/// what the recovery fan re-answers.
#[cfg(feature = "set_rerank")]
#[allow(clippy::too_many_arguments)] // index + items + query + cfg + scratch pair — the substrate's own shape
fn rank_of_set(
    index: &RuleIndex,
    items: &[&Binding],
    code: &str,
    rule: &str,
    cfg: &super::set_rerank::SetRerankConfig,
    k: usize,
    scratch: &mut katgpt_core::set_admission::AdmissionScratch,
    fan: &mut katgpt_core::set_admission::FanScratch,
) -> (Option<usize>, f32) {
    use riir_rag::{query::LATENT_DIM, Embedder, ModellessEmbedder};

    let mut rules = index.suggest_deep(code, RERANK_POOL);
    super::instantiation_rerank::rerank(&mut rules);

    // The corpus = this index's own patterns (the withheld set).
    let entries: Vec<(String, DomainId, String)> = items
        .iter()
        .map(|b| (b.rule.clone(), DomainId::Clippy, b.text.clone()))
        .collect();
    let corpus = super::set_rerank::RuleLatentCorpus::build(&entries);

    let by_domain = vec![(DomainId::Clippy, rules)];
    let latents = &corpus.latents;
    let ids = &corpus.rule_ids;
    let lookup = move |id: &str| {
        ids.binary_search_by(|p| p.as_str().cmp(id))
            .ok()
            .map(|i| cfg.latent.project(latents[i]))
    };
    let query: [f32; LATENT_DIM] = ModellessEmbedder.embed(code);
    let slate = super::set_rerank::counter_anchored_slate(
        cfg, &by_domain, lookup, &corpus, query, k, scratch, fan,
    );
    let rank = slate.admitted.iter().position(|m| m.rule_id == rule);
    let score = rank.map_or(0.0, |i| slate.admitted[i].score.unwrap_or(0.0));
    (rank, score)
}

/// Rank of `rule` for `code`, plus its fused score (0.0 when unretrieved).
/// Shared with the band axis (`band_ood`) — one ranking pipeline, never two.
pub(crate) fn rank_of(
    index: &RuleIndex,
    code: &str,
    rule: &str,
    probe: Option<DomainId>,
    mode: RerankMode,
) -> (Option<usize>, f32) {
    let mut rules = index.suggest_deep(code, RERANK_POOL);
    if let Some(d) = probe {
        mark_instantiation(d, code, &mut rules, mode);
    }
    rerank(&mut rules);
    let rank = rules.iter().position(|r| r.rule_id == rule);
    let score = rank.map_or(0.0, |i| rules[i].score);
    (rank, score)
}

/// Run the withheld-pair protocol over `population`.
///
/// `probe` selects the domain whose proposer visitors re-rank the pool
/// (`None` = pure retrieval, no instantiation evidence). `mode` is the
/// shipped [`RerankMode`] knob.
///
/// # Panics
///
/// Panics if a withheld index still contains the withheld pair (the
/// withholding assertion — never assumed), or if index construction fails.
#[must_use]
pub fn run_withheld_pair(
    population: Vec<Binding>,
    probe: Option<DomainId>,
    mode: RerankMode,
    mode_label: &'static str,
) -> WithheldReport {
    run_withheld_pair_under(
        population,
        |index, _items, code, rule| rank_of(index, code, rule, probe, mode),
        mode_label,
    )
}

/// Issue 121: the withheld-pair protocol under the counter-anchored
/// set-admission policy — the OOD-rig A/B arm. Same protocol, same arms,
/// same assertions; only the ranking differs (see [`rank_of_set`]).
#[cfg(feature = "set_rerank")]
#[must_use]
pub fn run_withheld_pair_set_admission(
    population: Vec<Binding>,
    cfg: super::set_rerank::SetRerankConfig,
    k: usize,
    mode_label: &'static str,
) -> WithheldReport {
    let mut admission = katgpt_core::set_admission::AdmissionScratch::new();
    let mut fan = katgpt_core::set_admission::FanScratch::new();
    run_withheld_pair_under(
        population,
        |index, items, code, rule| {
            rank_of_set(index, items, code, rule, &cfg, k, &mut admission, &mut fan)
        },
        mode_label,
    )
}

/// The protocol body, parameterized over the ranking policy (the Issue 121
/// refactor: the shipped closure vs the set-admission closure run the SAME
/// arms so the A/B rows are comparable).
// One function on purpose: the population census, the eligibility filter and
// the three-arm loop share the cell/role/filler maps, and splitting them would
// either recompute those maps or hand them around as a struct that exists only
// to be destructured. The protocol is one measurement.
#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::too_many_lines)]
fn run_withheld_pair_under(
    population: Vec<Binding>,
    mut rank: impl FnMut(&RuleIndex, &[&Binding], &str, &str) -> (Option<usize>, f32),
    mode_label: &'static str,
) -> WithheldReport {
    // Distinct texts only — `RuleIndex` dedupes identical chunk content, so a
    // duplicate text would silently drop one binding.
    let mut seen = BTreeSet::new();
    let pop: Vec<Binding> = population
        .into_iter()
        .filter(|b| seen.insert(*blake3::hash(b.text.as_bytes()).as_bytes()))
        .collect();

    let mut cells: BTreeMap<(String, &'static str), Vec<usize>> = BTreeMap::new();
    let mut by_rule: BTreeMap<String, BTreeSet<&'static str>> = BTreeMap::new();
    let mut by_shape: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    for (i, b) in pop.iter().enumerate() {
        cells.entry((b.rule.clone(), b.shape)).or_default().push(i);
        by_rule.entry(b.rule.clone()).or_default().insert(b.shape);
        by_shape.entry(b.shape).or_default().insert(b.rule.clone());
    }

    let matrix: Vec<(String, Vec<(&'static str, usize)>)> = by_rule
        .keys()
        .map(|rule| {
            let mut row: Vec<(&'static str, usize)> = by_shape
                .keys()
                .filter_map(|s| cells.get(&(rule.clone(), *s)).map(|v| (*s, v.len())))
                .collect();
            row.sort_unstable();
            (rule.clone(), row)
        })
        .collect();

    let collinear_rules: Vec<String> = by_rule
        .iter()
        .filter(|(_, shapes)| shapes.len() < 2)
        .map(|(r, _)| r.clone())
        .collect();

    // Eligibility: |cell| >= 2, role survives outside the cell, filler
    // survives outside the cell. Exactly the paper's "unseen combination of
    // a seen role and a seen filler".
    let eligible: Vec<&(String, &'static str)> = cells
        .iter()
        .filter(|((rule, shape), members)| {
            members.len() >= 2
                && by_rule.get(rule).is_some_and(|s| s.len() >= 2)
                && by_shape.get(*shape).is_some_and(|r| r.len() >= 2)
        })
        .map(|(k, _)| k)
        .collect();
    let eligible_cells = eligible.len();

    let mut rows = Vec::new();
    for cell_key in eligible {
        let (rule, shape) = (cell_key.0.clone(), cell_key.1);
        let members = &cells[cell_key];

        // OOD index: the whole cell removed. Shared by every member.
        let ood_items: Vec<&Binding> = pop
            .iter()
            .filter(|b| !(b.rule == rule && b.shape == shape))
            .collect();
        assert!(
            !ood_items.iter().any(|b| b.rule == rule && b.shape == shape),
            "withholding leak: ({rule}, {shape}) still in the OOD index"
        );
        assert!(
            ood_items.len() + members.len() == pop.len(),
            "withheld count mismatch for ({rule}, {shape})"
        );
        let ood_index = build_index(&ood_items);
        let ood_null = AtomicNull::build(ood_items.iter().copied());

        // NOROLE canary index: every item of the role removed.
        let norole_items: Vec<&Binding> = pop.iter().filter(|b| b.rule != rule).collect();
        let norole_index = build_index(&norole_items);

        for &qi in members {
            let q = &pop[qi];
            let id_items: Vec<&Binding> = pop
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != qi)
                .map(|(_, b)| b)
                .collect();
            let id_index = build_index(&id_items);
            let id_null = AtomicNull::build(id_items.iter().copied());

            let (id_rank, id_score) = rank(&id_index, &id_items, &q.text, &rule);
            let (ood_rank, ood_score) = rank(&ood_index, &ood_items, &q.text, &rule);
            let (norole_rank, _) = rank(&norole_index, &norole_items, &q.text, &rule);

            rows.push(ArmRow {
                rule: rule.clone(),
                shape,
                source: q.source,
                cell_size: members.len(),
                role_fillers: by_rule[&rule].len(),
                filler_roles: by_shape[&shape].len(),
                id_rank,
                ood_rank,
                norole_rank,
                id_score,
                ood_score,
                id_null: id_null.hits(&q.text, shape, &rule),
                ood_null: ood_null.hits(&q.text, shape, &rule),
            });
        }
    }

    WithheldReport {
        mode_label,
        population: pop.len(),
        n_rules: by_rule.len(),
        n_shapes: by_shape.len(),
        n_cells: cells.len(),
        eligible_cells,
        matrix,
        collinear_rules,
        rows,
    }
}

/// The `--nocapture` table.
pub fn print_report(r: &WithheldReport) {
    println!(
        "\n== Issue 062 withheld-(role, filler) OOD — arm: {} ==",
        r.mode_label
    );
    println!(
        "  population {} bindings | {} roles | {} fillers | {} cells | {} ELIGIBLE cells | n={} queries",
        r.population, r.n_rules, r.n_shapes, r.n_cells, r.eligible_cells, r.n()
    );
    println!(
        "  role/filler-collinear roles (1 shape only): {}",
        r.collinear_rules.len()
    );
    println!(
        "\n  {:<34} {:>8} {:>8} {:>8}",
        "arm", "top-1", "top-3", "top-5"
    );
    for (name, f) in [
        ("ID  (pair present)", 0usize),
        ("OOD (pair withheld)", 1),
        ("NOROLE (canary, must be 0)", 2),
    ] {
        let g = |k: usize| match f {
            0 => r.id_topk(k),
            1 => r.ood_topk(k),
            _ => r.norole_topk(k),
        };
        println!(
            "  {:<34} {:>7.1}% {:>7.1}% {:>7.1}%",
            name,
            g(1),
            g(3),
            g(5)
        );
    }
    println!(
        "  {:<34} {:>7.1}% {:>7.1}% {:>7.1}%",
        "CHANCE (uniform over roles)",
        r.chance_topk(1),
        r.chance_topk(3),
        r.chance_topk(5)
    );
    let (ns, nb) = r.null_id();
    let (os, ob) = r.null_ood();
    println!("\n  atomic null (sig / bag)  ID: {ns:.1}% / {nb:.1}%   OOD: {os:.1}% / {ob:.1}%");
    let (better, same, worse) = r.paired_moves();
    println!("  paired rank moves ID->OOD: better {better} | same {same} | worse {worse}");
    println!(
        "  PAIRED DROP top-1 {:+.1}pp | top-3 {:+.1}pp | top-5 {:+.1}pp",
        r.drop_pp(1),
        r.drop_pp(3),
        r.drop_pp(5)
    );
    println!("\n  -- OOD top-1 by role's distinct-filler count --");
    println!(
        "  {:<16} {:>5} {:>8} {:>8}",
        "role fillers", "n", "ID", "OOD"
    );
    for (k, n, id1, ood1) in r.by_role_fillers() {
        println!("  {k:<16} {n:>5} {id1:>7.1}% {ood1:>7.1}%");
    }
    println!("\n  -- per-(role, filler) cells --");
    let mut by_cell: BTreeMap<(String, &'static str), Vec<&ArmRow>> = BTreeMap::new();
    for row in &r.rows {
        by_cell
            .entry((row.rule.clone(), row.shape))
            .or_default()
            .push(row);
    }
    for ((rule, shape), rws) in &by_cell {
        let n = rws.len();
        let id1 = recall(rws.iter().map(|r| r.id_rank), 1, n);
        let ood1 = recall(rws.iter().map(|r| r.ood_rank), 1, n);
        println!(
            "  {:<34} {:<12} n={:<3} ID {:>5.1}%  OOD {:>5.1}%  Δ {:+.1}pp",
            rule,
            shape,
            n,
            id1,
            ood1,
            id1 - ood1
        );
    }
}

/// Per-row rank dump — the paired raw data behind every aggregate, and the
/// only way to localize a run-to-run flip to a specific query.
pub fn print_rows(r: &WithheldReport) {
    println!(
        "\n  -- per-row ranks ({}): rank None = not in the pool --",
        r.mode_label
    );
    println!(
        "  {:<30} {:<12} {:<10} {:>6} {:>6} {:>10}",
        "role", "filler", "source", "ID", "OOD", "id_score"
    );
    let fmt = |o: Option<usize>| o.map_or("-".to_string(), |v| v.to_string());
    for row in &r.rows {
        println!(
            "  {:<30} {:<12} {:<10} {:>6} {:>6} {:>10.4}",
            row.rule,
            row.shape,
            row.source,
            fmt(row.id_rank),
            fmt(row.ood_rank),
            row.id_score
        );
    }
}

/// Print the population matrix (role × filler occupancy).
pub fn print_matrix(r: &WithheldReport) {
    println!("\n  -- population matrix (role × filler) --");
    for (rule, row) in &r.matrix {
        let cells: Vec<String> = row.iter().map(|(s, n)| format!("{s}:{n}")).collect();
        println!("  {:<36} {}", rule, cells.join("  "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_classes_are_pinned() {
        // Fixed inputs → fixed classes. This is the classifier's spec.
        let cases: [(&str, ShapeClass); 8] = [
            ("fn f() -> String { format!(\"x\") }", ShapeClass::Macro),
            (
                "fn f(v: &[u8]) -> usize { v.iter().count() }",
                ShapeClass::IteratorChain,
            ),
            ("fn f() -> u8 { run(|x| x + 1) }", ShapeClass::Closure),
            (
                "fn head<'a>(t: &'a String) -> &'a str { t.as_str() }",
                ShapeClass::Generic,
            ),
            ("impl A { fn f(&self) -> u8 { 1 } }", ShapeClass::ImplBlock),
            (
                "fn f(v: &[u8]) { for i in 0..v.len() { let _ = i; } }",
                ShapeClass::Loop,
            ),
            (
                "struct A { x: u8 }\nfn f(a: A) -> u8 { a.x }",
                ShapeClass::MultiItem,
            ),
            ("fn f() -> u8 { return 1; }", ShapeClass::PlainFn),
        ];
        for (code, want) in cases {
            assert_eq!(classify_shape(code), want, "classify_shape({code:?})");
            // Determinism: the same text always classifies the same way.
            assert_eq!(classify_shape(code), classify_shape(code));
        }
    }

    #[test]
    fn rustc_shape_classes_are_pinned() {
        let cases: [(&str, &str); 5] = [
            ("pub fn probe() { let c = move || 1; }", "closure-capture"),
            ("let s = format!(\"{}\", 1);", "format-macro"),
            ("let r: &[u8] = &owned[..];", "slice-borrow"),
            ("let r: &str = owned.as_str();", "as-view"),
            ("let r = &mut v;", "mut-borrow"),
        ];
        for (code, want) in cases {
            assert_eq!(
                classify_rustc_shape(code),
                want,
                "classify_rustc_shape({code:?})"
            );
        }
    }

    #[test]
    fn attributes_do_not_masquerade_as_index_expressions() {
        // The measured first-run defect: `#[test]` inside the generated
        // ground-truth module read as an index expression, so `index-expr`
        // absorbed four of the eight error codes.
        let with_gt = "pub fn probe(v: Vec<u8>) -> usize {\n    let r = &v;\n    let w = v;\n    r.len() + w.len()\n}\n\n#[cfg(test)]\nmod ground_truth {\n    #[test]\n    fn behavior() {\n        assert_eq!(super::probe(vec![9u8]), 2);\n    }\n}\n";
        assert_eq!(classify_rustc_shape(with_gt), "vec-owner");
        assert!(!rustc_payload(with_gt).contains("#[cfg(test)]"));
        assert_eq!(strip_attributes("#[test]\nfn f() {}"), "\nfn f() {}");
    }

    #[test]
    fn normalization_erases_names_and_keeps_structure() {
        let a = normalized_tokens("fn alpha() -> u8 { return 1; }");
        let b = normalized_tokens("fn beta() -> u16 { return 7; }");
        assert_eq!(a, b, "identifier/literal renaming must not change the key");
        let c = normalized_tokens("fn alpha() -> u8 { 1 }");
        assert_ne!(a, c, "structure change must change the key");
        assert!(a.contains(&"return".to_string()), "keywords survive: {a:?}");
    }

    #[test]
    fn atomic_null_is_shape_prefixed() {
        // Same tokens, different shape label → different key. This is what
        // makes the withheld-pair null zero by construction.
        let code = "fn f() -> u8 { return 1; }";
        let (s1, _) = atomic_keys(code, "plain-fn");
        let (s2, _) = atomic_keys(code, "impl-block");
        assert_ne!(s1, s2);
    }
}
