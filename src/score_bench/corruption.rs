//! Issue 137 T1/T3 — the CRB-style compile-clean / test-failing corruption
//! engine (generation v9), ported from arXiv:2512.15596's Code Revision
//! Benchmark instrument (Research 215).
//!
//! Three pieces live here:
//!
//! 1. **The corruption engine** — over a table of small seed programs
//!    (compile-clean, test-passing), syn-derived mutation SITES in the three
//!    CRB classes (operator table `+↔-`, `*↔/`, `<↔<=`, `==↔!=`, `&&↔||`;
//!    same-type literal; identifier-from-same-scope), a seeded deterministic
//!    candidate builder at |E| ∈ {1, 2, 4}, and the pre-cargo no-op gate.
//!    The two-tier cargo filter itself (`cargo check` must stay CLEAN, then
//!    `cargo test` must FAIL) spawns real cargo and lives in the freeze e2e
//!    (`tests/corruption_corpus_freeze.rs`), not in unit tests.
//! 2. **The committed corpus** — `super::corruption_corpus_v9` is GENERATED
//!    by the e2e's write mode and pinned by a blake3 digest. Hand-editing it
//!    without regenerating reds [`v9_freeze_status`] (the append-refusal
//!    law, asserted by the always-on pin test).
//! 3. **The generation row** — [`generation_v9`] / [`v9_freeze_status`],
//!    the same freeze-status → fixtures contract as generations v2–v8, with
//!    per-|E| count floors firing DOWNWARD only.
//!
//! Deliberate exclusions (the hang-safety law): no sites are collected
//! inside `while` conditions (a flipped loop predicate can spin forever —
//! the seed programs use `for` loops exclusively and the engine still
//! refuses the class), `#[cfg(test)]` items are never corrupted (the grader
//! is trusted; errors belong to program logic), and compound assignments /
//! floats are outside the v1 table. The e2e additionally rejects
//! signal-killed (abort/stack-overflow) and timed-out candidates — only a
//! clean non-zero `cargo test` verdict counts as "test-failing".
//!
//! Token-count note: every replacement is one source token for one source
//! token (`proc_macro2` may split `<=` into two punct chars — byte spans are
//! what the metrics consume, so this is cosmetic).

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{BinOp, ExprLit, ItemFn, ItemMod, Lit};

use super::corruption_corpus_v9;

/// The generation this module owns (v8 is the shaped-verdict ladder; v9 is
/// the next free row — verified free at implementation time).
pub const GENERATION: u8 = 9;

/// The |E| ladder (replaced-token counts; the gen2/gen4 k convention).
pub const LADDER_KS: [usize; 3] = [1, 2, 4];

/// Per-|E| admission floors — fire DOWNWARD only (a silent generation
/// regression empties the population loudly). Measured at the landing run
/// (16 per rung at the seed table × cap 2): 10 ≈ 62% of achieved.
pub const PER_K_FLOORS: &[(usize, usize)] = &[(1, 10), (2, 10), (4, 10)];

/// The corruption classes (one tag string each in the committed corpus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorruptionClass {
    Operator,
    Literal,
    Ident,
}

impl CorruptionClass {
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::Literal => "literal",
            Self::Ident => "ident",
        }
    }

    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "operator" => Some(Self::Operator),
            "literal" => Some(Self::Literal),
            "ident" => Some(Self::Ident),
            _ => None,
        }
    }
}

// ── seed programs ───────────────────────────────────────────────────────

/// One seed program: a standalone `lib.rs` body (compile-clean,
/// test-passing — the e2e's seed gate verifies both) with asymmetric tests
/// so same-type identifier swaps flip behavior.
#[derive(Debug, Clone, Copy)]
pub struct SeedProgram {
    pub id: &'static str,
    pub code: &'static str,
}

/// The seed table. Small on purpose: every program must keep its
/// `for`-loop-only, no-while shape so no mutation site can spin a loop, and
/// every test must assert exact values from asymmetric arguments (a
/// symmetric function like `max` would make parameter swaps semantically
/// inert and the tier-2 filter would reject every such draw).
pub static SEED_PROGRAMS: &[SeedProgram] = &[
    SeedProgram {
        id: "range_gates",
        code: r"pub fn clamp_range(value: i64, low: i64, high: i64) -> i64 {
    if value < low {
        low
    } else if high < value {
        high
    } else {
        value
    }
}

pub fn in_open_range(value: i64, low: i64, high: i64) -> bool {
    low < value && value < high
}

pub fn midpoint(low: i64, high: i64) -> i64 {
    low + (high - low) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_hits_all_arms() {
        assert_eq!(clamp_range(-5, 0, 10), 0);
        assert_eq!(clamp_range(5, 0, 10), 5);
        assert_eq!(clamp_range(50, 0, 10), 10);
    }

    #[test]
    fn open_range_excludes_bounds() {
        assert!(in_open_range(1, 0, 10));
        assert!(!in_open_range(0, 0, 10));
        assert!(!in_open_range(10, 0, 10));
    }

    #[test]
    fn midpoint_is_interior() {
        assert_eq!(midpoint(0, 10), 5);
        assert_eq!(midpoint(-4, 4), 0);
    }
}
",
    },
    SeedProgram {
        id: "fib_pairs",
        code: r"pub fn fib(n: u64) -> u64 {
    let mut a = 0u64;
    let mut b = 1u64;
    for _ in 0..n {
        let next = a + b;
        a = b;
        b = next;
    }
    a
}

pub fn fib_sum(n: u64) -> u64 {
    let mut total = 0u64;
    for i in 0..n {
        total += fib(i + 1);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fib_values() {
        assert_eq!(fib(0), 0);
        assert_eq!(fib(1), 1);
        assert_eq!(fib(10), 55);
    }

    #[test]
    fn fib_sum_values() {
        assert_eq!(fib_sum(5), 12);
        assert_eq!(fib_sum(1), 1);
        assert_eq!(fib_sum(0), 0);
    }
}
",
    },
    SeedProgram {
        id: "slice_search",
        code: r"pub fn binary_search(haystack: &[i64], needle: i64) -> Option<usize> {
    if haystack.is_empty() {
        return None;
    }
    let mid = haystack.len() / 2;
    if haystack[mid] < needle {
        match binary_search(&haystack[mid + 1..], needle) {
            Some(i) => Some(i + mid + 1),
            None => None,
        }
    } else if haystack[mid] == needle {
        Some(mid)
    } else {
        binary_search(&haystack[..mid], needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_each_quadrant() {
        let data = [1, 3, 5, 7, 9];
        assert_eq!(binary_search(&data, 1), Some(0));
        assert_eq!(binary_search(&data, 7), Some(3));
        assert_eq!(binary_search(&data, 9), Some(4));
        assert_eq!(binary_search(&data, 4), None);
        assert_eq!(binary_search(&data, 0), None);
        assert_eq!(binary_search(&data, 42), None);
    }

    #[test]
    fn empty_haystack_misses() {
        assert_eq!(binary_search(&[], 1), None);
    }
}
",
    },
    SeedProgram {
        id: "rolling_stats",
        code: r"pub fn sum(values: &[i64]) -> i64 {
    let mut total = 0i64;
    for v in values {
        total += *v;
    }
    total
}

pub fn mean(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    sum(values) / values.len() as i64
}

pub fn peak(values: &[i64]) -> i64 {
    let mut best = values[0];
    for v in values {
        if best < *v {
            best = *v;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_and_mean() {
        assert_eq!(sum(&[2, 4, 6]), 12);
        assert_eq!(mean(&[2, 4, 6]), 4);
        assert_eq!(mean(&[]), 0);
        assert_eq!(sum(&[]), 0);
    }

    #[test]
    fn peak_of_mixed() {
        assert_eq!(peak(&[3, 9, 1]), 9);
        assert_eq!(peak(&[-7, -2, -5]), -2);
    }
}
",
    },
    SeedProgram {
        id: "bool_gates",
        code: r"pub fn both_or_neither(a: bool, b: bool) -> bool {
    (a && b) || (!a && !b)
}

pub fn either_high(left: bool, right: bool) -> bool {
    left || right && !left
}

pub fn evenly_divides(numerator: i64, denominator: i64) -> bool {
    denominator != 0 && numerator % denominator == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xnor_truth_table() {
        assert_eq!(both_or_neither(true, true), true);
        assert_eq!(both_or_neither(false, false), true);
        assert_eq!(both_or_neither(true, false), false);
        assert_eq!(both_or_neither(false, true), false);
    }

    #[test]
    fn either_truth_table() {
        assert_eq!(either_high(true, false), true);
        assert_eq!(either_high(false, true), true);
        assert_eq!(either_high(true, true), true);
        assert_eq!(either_high(false, false), false);
    }

    #[test]
    fn divides_guards_zero() {
        assert_eq!(evenly_divides(8, 4), true);
        assert_eq!(evenly_divides(7, 3), false);
        assert_eq!(evenly_divides(5, 0), false);
    }
}
",
    },
    SeedProgram {
        id: "char_count",
        code: r#"pub fn count_char(haystack: &str, needle: char) -> usize {
    let mut count = 0usize;
    for c in haystack.chars() {
        if c == needle {
            count += 1;
        }
    }
    count
}

pub fn bracketed(text: &str, edge: char) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        false
    } else {
        bytes[0] == edge as u8 && bytes[bytes.len() - 1] == edge as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_are_exact() {
        assert_eq!(count_char("banana", 'a'), 3);
        assert_eq!(count_char("banana", 'z'), 0);
        assert_eq!(count_char("", 'a'), 0);
    }

    #[test]
    fn bracketed_edges() {
        assert_eq!(bracketed("aba", 'a'), true);
        assert_eq!(bracketed("ab", 'a'), false);
        assert_eq!(bracketed("ab", 'b'), false);
        assert_eq!(bracketed("", 'a'), false);
        assert_eq!(bracketed("x", 'x'), true);
    }
}
"#,
    },
    SeedProgram {
        id: "grid_steps",
        code: r"pub fn manhattan(ax: i64, ay: i64, bx: i64, by: i64) -> i64 {
    (ax - bx).abs() + (ay - by).abs()
}

pub fn inside_square(x: i64, y: i64, half: i64) -> bool {
    -half < x && x < half && -half < y && y < half
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manhattan_asymmetric() {
        assert_eq!(manhattan(1, 5, 4, 1), 7);
        assert_eq!(manhattan(0, 0, 0, 0), 0);
        assert_eq!(manhattan(-3, 2, 1, -1), 7);
    }

    #[test]
    fn inside_edges() {
        assert_eq!(inside_square(0, 0, 5), true);
        assert_eq!(inside_square(5, 0, 5), false);
        assert_eq!(inside_square(4, 4, 5), true);
        assert_eq!(inside_square(0, -6, 5), false);
    }
}
",
    },
    SeedProgram {
        id: "edge_counts",
        code: r"pub fn rising_edges(bits: &[bool]) -> usize {
    let mut count = 0usize;
    let mut prev = false;
    for b in bits {
        if *b && !prev {
            count += 1;
        }
        prev = *b;
    }
    count
}

pub fn majority(a: bool, b: bool, c: bool) -> bool {
    (a && b) || (b && c) || (a && c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_count_transitions() {
        assert_eq!(rising_edges(&[false, true, true, false, true]), 2);
        assert_eq!(rising_edges(&[true, false, false]), 1);
        assert_eq!(rising_edges(&[]), 0);
        assert_eq!(rising_edges(&[false]), 0);
    }

    #[test]
    fn majority_truth() {
        assert_eq!(majority(true, false, false), false);
        assert_eq!(majority(true, true, false), true);
        assert_eq!(majority(false, true, true), true);
    }
}
",
    },
];

/// Read-only access for the e2e + tests.
#[must_use]
pub fn seed_programs() -> &'static [SeedProgram] {
    SEED_PROGRAMS
}

// ── the site engine ─────────────────────────────────────────────────────

/// One single-token mutation site (operator or literal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutSite {
    pub start: usize,
    pub end: usize,
    pub class: CorruptionClass,
    pub replacement: String,
}

/// One identifier-swap plan: every use of `from` inside its function body
/// becomes `to` (both same-type parameters; `spans` are the use sites, so
/// the plan's |E| is `spans.len()`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentPlan {
    pub from: String,
    pub to: String,
    pub spans: Vec<Range<usize>>,
}

/// The mutation inventory of one seed program.
#[derive(Debug, Clone, Default)]
pub struct ProgramSites {
    pub singles: Vec<MutSite>,
    pub plans: Vec<IdentPlan>,
}

/// Deterministic `SplitMix64` (the unseeded-global-RNG gate compliant — every
/// draw is seeded; the modulo is disclosed as the sampling primitive).
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[must_use]
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            // `n` is a small sampling bound — no truncation at any real width
            #[allow(clippy::cast_possible_truncation)]
            let bound = n as u64;
            #[allow(clippy::cast_possible_truncation)]
            let sampled = (self.next_u64() % bound) as usize;
            sampled
        }
    }
}

fn mix(seed: u64, program: usize, k: usize, draw: usize) -> u64 {
    let mut h = SplitMix64::new(seed);
    let a = h.next_u64() ^ ((program as u64).rotate_left(17));
    let b = h.next_u64() ^ ((k as u64) << 32) ^ (draw as u64);
    a ^ b
}

/// The operator swap table (`+↔-`, `*↔/`, `<↔<=`, `==↔!=`, `&&↔||`) — the
/// CRB operator class, 1 source token for 1 source token. Returns the
/// replacement text.
fn op_swap(op: &BinOp) -> Option<&'static str> {
    Some(match op {
        BinOp::Add(_) => "-",
        BinOp::Sub(_) => "+",
        BinOp::Mul(_) => "/",
        BinOp::Div(_) => "*",
        BinOp::Lt(_) => "<=",
        BinOp::Le(_) => "<",
        BinOp::Eq(_) => "!=",
        BinOp::Ne(_) => "==",
        BinOp::And(_) => "||",
        BinOp::Or(_) => "&&",
        _ => return None,
    })
}

/// `#[cfg(test)]` detection over a module's attrs (syn 2 nested-meta parse —
/// errors mean "not a cfg(test)" and stay false).
fn module_is_cfg_test(module: &ItemMod) -> bool {
    module.attrs.iter().any(|attr| {
        if !attr.path().is_ident("cfg") {
            return false;
        }
        let mut found = false;
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("test") {
                found = true;
            }
            Ok(())
        });
        found
    })
}

/// A comparable key for the simple types the seed programs use (bare paths,
/// references, slices, parens, unit). `None` = "don't pair this type".
fn type_key(ty: &syn::Type) -> Option<String> {
    Some(match ty {
        syn::Type::Path(p)
            if p.qself.is_none()
                && p.path.segments.len() == 1
                && p.path.segments[0].arguments.is_none() =>
        {
            format!("path:{}", p.path.segments[0].ident)
        }
        syn::Type::Reference(r) => format!(
            "ref{}:{}",
            if r.mutability.is_some() { "mut" } else { "" },
            type_key(&r.elem)?
        ),
        syn::Type::Slice(s) => format!("slice:{}", type_key(&s.elem)?),
        syn::Type::Paren(p) => type_key(&p.elem)?,
        syn::Type::Tuple(t) if t.elems.is_empty() => "unit".to_string(),
        _ => return None,
    })
}

/// Body walker: collects `let`-style bindings (shadow guard) and per-name
/// path-use spans. Nested `fn` items are skipped (their names are not this
/// function's parameters); closure params COUNT as binds (they shadow).
struct BodyWalker {
    binds: BTreeSet<String>,
    uses: BTreeMap<String, Vec<Range<usize>>>,
}

impl Visit<'_> for BodyWalker {
    fn visit_item_fn(&mut self, _i: &ItemFn) {
        // nested fn: skip entirely (its identifiers are not this scope's)
    }

    fn visit_expr_while(&mut self, e: &syn::ExprWhile) {
        // loop-condition uses are excluded with the operator sites (hang law)
        self.visit_block(&e.body);
    }

    fn visit_pat(&mut self, p: &syn::Pat) {
        if let syn::Pat::Ident(pi) = p {
            self.binds.insert(pi.ident.to_string());
        }
        syn::visit::visit_pat(self, p);
    }

    fn visit_expr_path(&mut self, e: &syn::ExprPath) {
        if e.qself.is_none() {
            if let Some(seg) = e.path.segments.last() {
                if seg.arguments.is_none() {
                    let name = seg.ident.to_string();
                    let range = seg.ident.span().byte_range();
                    self.uses.entry(name).or_default().push(range);
                }
            }
        }
        syn::visit::visit_expr_path(self, e);
    }
}

/// The identifier-swap plans of one function: every ordered pair of
/// distinct same-type parameters (no shadowing binds for either name,
/// `from` used at least once). Conservative by construction — the cargo
/// tiers are the real authority, this just keeps the candidate space clean.
fn ident_plans_for_fn(f: &ItemFn) -> Vec<IdentPlan> {
    let mut by_type: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for arg in &f.sig.inputs {
        if let syn::FnArg::Typed(pt) = arg {
            if let syn::Pat::Ident(pi) = &*pt.pat {
                if pi.ident == "self" {
                    continue;
                }
                if let Some(key) = type_key(&pt.ty) {
                    by_type.entry(key).or_default().push(pi.ident.to_string());
                }
            }
        }
    }
    if !by_type.values().any(|v| v.len() >= 2) {
        return Vec::new();
    }
    let mut walker = BodyWalker {
        binds: BTreeSet::new(),
        uses: BTreeMap::new(),
    };
    walker.visit_block(&f.block);
    let mut plans = Vec::new();
    for names in by_type.values() {
        if names.len() < 2 {
            continue;
        }
        for from in names {
            for to in names {
                if from == to || walker.binds.contains(from) || walker.binds.contains(to) {
                    continue;
                }
                let spans = walker.uses.get(from).cloned().unwrap_or_default();
                if spans.is_empty() {
                    continue;
                }
                plans.push(IdentPlan {
                    from: from.clone(),
                    to: to.clone(),
                    spans,
                });
            }
        }
    }
    plans
}

struct ProgramVisitor {
    sites: ProgramSites,
}

impl Visit<'_> for ProgramVisitor {
    fn visit_item_mod(&mut self, module: &ItemMod) {
        if module_is_cfg_test(module) {
            // the grader is trusted — never corrupted
            return;
        }
        syn::visit::visit_item_mod(self, module);
    }

    fn visit_item_fn(&mut self, f: &ItemFn) {
        self.sites.plans.extend(ident_plans_for_fn(f));
        syn::visit::visit_item_fn(self, f);
    }

    fn visit_expr_while(&mut self, e: &syn::ExprWhile) {
        // the hang-safety law: no sites inside loop conditions
        self.visit_block(&e.body);
    }

    fn visit_expr_binary(&mut self, e: &syn::ExprBinary) {
        if let Some(replacement) = op_swap(&e.op) {
            let range = e.op.span().byte_range();
            self.sites.singles.push(MutSite {
                start: range.start,
                end: range.end,
                class: CorruptionClass::Operator,
                replacement: replacement.to_string(),
            });
        }
        syn::visit::visit_expr_binary(self, e);
    }

    fn visit_expr_lit(&mut self, e: &ExprLit) {
        match &e.lit {
            Lit::Int(li) => {
                let digits = li.base10_digits();
                let suffix = li.suffix();
                let decimal = digits.bytes().all(|b| b.is_ascii_digit());
                let plain_suffix = suffix.bytes().all(|b| b.is_ascii_alphanumeric());
                if decimal && plain_suffix {
                    if let Ok(value) = digits.parse::<u128>() {
                        let replacement = format!("{}{}", value ^ 1, suffix);
                        let range = li.span().byte_range();
                        self.sites.singles.push(MutSite {
                            start: range.start,
                            end: range.end,
                            class: CorruptionClass::Literal,
                            replacement,
                        });
                    }
                }
            }
            Lit::Bool(lb) => {
                let replacement = if lb.value() { "false" } else { "true" };
                let range = lb.span().byte_range();
                self.sites.singles.push(MutSite {
                    start: range.start,
                    end: range.end,
                    class: CorruptionClass::Literal,
                    replacement: replacement.to_string(),
                });
            }
            _ => {}
        }
        syn::visit::visit_expr_lit(self, e);
    }
}

/// The mutation inventory of one program (`None` = does not parse).
#[must_use]
pub fn collect_sites(code: &str) -> Option<ProgramSites> {
    let file = syn::parse_file(code).ok()?;
    let mut visitor = ProgramVisitor {
        sites: ProgramSites::default(),
    };
    visitor.visit_file(&file);
    visitor.sites.singles.sort_by_key(|s| s.start);
    Some(visitor.sites)
}

/// One generated candidate (pre-cargo — the two-tier filter decides its
/// fate).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub seed_id: &'static str,
    pub k: usize,
    pub draw: usize,
    pub code: String,
    /// The replaced-token byte spans in CORRUPTED-code coordinates
    /// (recomputed post-edit — replacements may differ in width), ascending.
    pub spans: Vec<(usize, usize)>,
    pub classes: Vec<CorruptionClass>,
    /// `true` when the candidate is one identifier plan (|E| = all its uses).
    pub via_plan: bool,
}

/// Apply replacements and derive the error spans in CORRUPTED-code
/// coordinates. Replacements may differ in width from the token they
/// replace (`<`↔`<=`, ident pairs with different lengths), so the spans the
/// metrics consume are recomputed during the walk — never reused from the
/// seed's coordinates.
fn apply_edits_with_spans(
    base: &str,
    mut edits: Vec<(Range<usize>, String)>,
) -> (String, Vec<(usize, usize)>) {
    edits.sort_by_key(|(range, _)| range.start);
    let mut out = String::with_capacity(base.len());
    let mut cursor = 0usize;
    let mut spans = Vec::with_capacity(edits.len());
    for (range, replacement) in &edits {
        out.push_str(&base[cursor..range.start]);
        let new_start = out.len();
        out.push_str(replacement);
        spans.push((new_start, out.len()));
        cursor = range.end;
    }
    out.push_str(&base[cursor..]);
    (out, spans)
}

/// The pre-cargo gate: a candidate whose text equals its seed is a no-op
/// corruption and is REJECTED before cargo ever runs (the planted no-op
/// proof — the filter must discriminate).
#[must_use]
pub fn is_noop(candidate_code: &str, seed_code: &str) -> bool {
    candidate_code == seed_code
}

/// Draw the seeded candidate grid: `draws_per_k` draws per
/// (seed program, |E|). Deterministic — same inputs, byte-identical
/// candidates (pinned by test). About one draw in four takes an identifier
/// plan when one matches |E|; the rest sample |E| distinct single-token
/// sites.
#[must_use]
pub fn build_candidates(seed: u64, draws_per_k: usize) -> Vec<Candidate> {
    let mut out = Vec::with_capacity(LADDER_KS.len());
    for (program_index, program) in SEED_PROGRAMS.iter().enumerate() {
        let Some(sites) = collect_sites(program.code) else {
            continue;
        };
        for &k in &LADDER_KS {
            let plans_k: Vec<&IdentPlan> = sites
                .plans
                .iter()
                .filter(|plan| plan.spans.len() == k)
                .collect();
            for draw in 0..draws_per_k {
                let mut rng = SplitMix64::new(mix(seed, program_index, k, draw));
                let route_plan = !plans_k.is_empty() && rng.below(4) == 0;
                let built = if route_plan {
                    let plan = &plans_k[rng.below(plans_k.len())];
                    let (code, spans) = apply_edits_with_spans(
                        program.code,
                        plan.spans
                            .iter()
                            .map(|r| (r.clone(), plan.to.clone()))
                            .collect(),
                    );
                    Some(Candidate {
                        seed_id: program.id,
                        k,
                        draw,
                        code,
                        spans,
                        classes: vec![CorruptionClass::Ident],
                        via_plan: true,
                    })
                } else if sites.singles.len() >= k {
                    let mut order: Vec<usize> = (0..sites.singles.len()).collect();
                    for i in (1..order.len()).rev() {
                        let j = rng.below(i + 1);
                        order.swap(i, j);
                    }
                    let mut edits = Vec::with_capacity(k);
                    let mut class_set = BTreeSet::new();
                    for &index in &order[..k] {
                        let site = &sites.singles[index];
                        edits.push((site.start..site.end, site.replacement.clone()));
                        class_set.insert(site.class);
                    }
                    let (code, spans) = apply_edits_with_spans(program.code, edits);
                    Some(Candidate {
                        seed_id: program.id,
                        k,
                        draw,
                        code,
                        spans,
                        classes: class_set.into_iter().collect(),
                        via_plan: false,
                    })
                } else {
                    None
                };
                if let Some(candidate) = built {
                    out.push(candidate);
                }
            }
        }
    }
    out.shrink_to_fit();
    out
}

// ── the committed corpus + generation row ───────────────────────────────

/// One committed corruption fixture (headline or control arm). All fields
/// `&'static` — the corpus is generated data, pinned by the digest.
#[derive(Debug, Clone, Copy)]
pub struct CorruptionSpec {
    pub id: &'static str,
    pub k: usize,
    pub code: &'static str,
    /// The replaced-token byte spans (the known error positions).
    pub spans: &'static [(usize, usize)],
    /// Sorted distinct class tags.
    pub classes: &'static [&'static str],
    /// blake3 over `id ‖ k ‖ code ‖ spans ‖ classes` (the per-fixture pin).
    pub blake3: &'static str,
    pub seed_program: &'static str,
}

/// The headline corpus (compile-clean, test-failing — NO oracle marks these
/// spans; this is where localization is a genuine question).
#[must_use]
pub fn corpus() -> &'static [CorruptionSpec] {
    corruption_corpus_v9::CORRUPTION_SPECS_V9
}

/// The control arm (compile-FAILING corruption — rustc marks those spans,
/// so hit@K there measures oracle anchoring and is expected near-trivial).
#[must_use]
pub fn control_corpus() -> &'static [CorruptionSpec] {
    corruption_corpus_v9::CONTROL_SPECS_V9
}

/// The per-fixture blake3 pin (single source of truth — the e2e's writer
/// calls this over its records so the committed pins are this module's
/// exact arithmetic).
#[must_use]
pub fn spec_blake3(
    id: &str,
    k: usize,
    code: &str,
    spans: &[(usize, usize)],
    classes: &[&str],
) -> String {
    let mut input = Vec::new();
    input.extend_from_slice(id.as_bytes());
    input.push(0);
    input.extend_from_slice(&(k as u64).to_le_bytes());
    input.push(0);
    input.extend_from_slice(code.as_bytes());
    input.push(0);
    for (start, end) in spans {
        input.extend_from_slice(&(*start as u64).to_le_bytes());
        input.extend_from_slice(&(*end as u64).to_le_bytes());
    }
    input.push(0);
    for class in classes {
        input.extend_from_slice(class.as_bytes());
        input.push(0);
    }
    blake3::hash(&input).to_hex().to_string()
}

/// The canonical digest over the whole corpus (headline + control, id
/// order, arm-tagged) — the append-refusal pin.
#[must_use]
pub fn corpus_digest() -> String {
    let mut input = Vec::new();
    let mut rows: Vec<(&str, bool, &CorruptionSpec)> = Vec::new();
    for spec in corpus() {
        rows.push((spec.id, false, spec));
    }
    for spec in control_corpus() {
        rows.push((spec.id, true, spec));
    }
    rows.sort_by_key(|(id, control, _)| (*id, *control));
    for (id, control, spec) in rows {
        input.extend_from_slice(if control { b"C" } else { b"H" });
        input.extend_from_slice(id.as_bytes());
        input.push(0);
        input.extend_from_slice(&(spec.k as u64).to_le_bytes());
        input.extend_from_slice(spec.code.as_bytes());
        input.push(0);
        for (start, end) in spec.spans {
            input.extend_from_slice(&(*start as u64).to_le_bytes());
            input.extend_from_slice(&(*end as u64).to_le_bytes());
        }
        input.push(0);
        for class in spec.classes {
            input.extend_from_slice(class.as_bytes());
            input.push(0);
        }
    }
    blake3::hash(&input).to_hex().to_string()
}

/// Structural verification of a corpus slice (digest is checked by
/// [`v9_freeze_status`] over both arms).
fn verify_specs(table: &[CorruptionSpec], arm: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for spec in table {
        if !seen.insert(spec.id) {
            return Err(format!("{arm}: duplicate id {}", spec.id));
        }
        if !spec.id.starts_with("gen9-") {
            return Err(format!("{arm}: id {} lacks the gen9- prefix", spec.id));
        }
        if !LADDER_KS.contains(&spec.k) {
            return Err(format!("{}: k {} outside the ladder", spec.id, spec.k));
        }
        if spec.spans.is_empty() {
            return Err(format!(
                "{}: zero error spans (a no-op cannot be a fixture)",
                spec.id
            ));
        }
        let mut previous_end = 0usize;
        for (start, end) in spec.spans {
            if start >= end || *end > spec.code.len() {
                return Err(format!("{}: span {start}..{end} out of bounds", spec.id));
            }
            if *start < previous_end {
                return Err(format!("{}: spans overlap at {start}..{end}", spec.id));
            }
            previous_end = *end;
        }
        if spec.classes.is_empty() {
            return Err(format!("{}: no class tags", spec.id));
        }
        let mut class_set = BTreeSet::new();
        for class in spec.classes {
            if CorruptionClass::from_tag(class).is_none() {
                return Err(format!("{}: unknown class tag {class}", spec.id));
            }
            class_set.insert(*class);
        }
        if class_set.len() != spec.classes.len() {
            return Err(format!("{}: duplicate class tags", spec.id));
        }
        if spec.blake3.len() != 64 {
            return Err(format!("{}: blake3 pin malformed", spec.id));
        }
        if syn::parse_file(spec.code).is_err() {
            return Err(format!("{}: code does not parse", spec.id));
        }
    }
    Ok(())
}

/// Freeze readiness for generation v9: Ok(admitted headline count) when the
/// generated corpus is present, structurally sound, digest-consistent, and
/// above every per-|E| floor; Err(why) otherwise (the empty corpus is an
/// Err, never a green zero).
///
/// # Errors
///
/// The un-frozen states: corpus not generated, digest mismatch (hand-edited
/// data — regenerate via the e2e write mode), a structural defect, or a
/// per-|E| floor breach.
pub fn v9_freeze_status() -> Result<usize, String> {
    let table = corpus();
    if table.is_empty() {
        return Err("generation v9 corpus not generated — run \
             `cargo test --features score_bench --test corruption_corpus_freeze -- --ignored \
             --test-threads=1` with RIIR_CRB_GEN_WRITE=1 and commit the result"
            .to_string());
    }
    let recorded = corruption_corpus_v9::CORPUS_V9_DIGEST;
    let computed = corpus_digest();
    if recorded != computed {
        return Err(format!(
            "generation v9 corpus digest drift — the committed table was hand-edited \
             without regenerating (recorded {recorded}, computed {computed}); \
             re-run the freeze e2e write mode"
        ));
    }
    verify_specs(table, "headline")?;
    verify_specs(control_corpus(), "control")?;
    for &(k, floor) in PER_K_FLOORS {
        let count = table.iter().filter(|spec| spec.k == k).count();
        if count < floor {
            return Err(format!(
                "generation v9 per-|E| floor breached at k={k}: {count} < {floor}"
            ));
        }
    }
    Ok(table.len())
}

/// The admitted generation-v9 fixture set (headline arm). Callers check
/// [`v9_freeze_status`] first — this panics on a freeze hole, the
/// generations-v2..v8 contract.
///
/// # Panics
///
/// On any freeze-status failure (empty corpus, digest drift, floor breach).
#[must_use]
pub fn generation_v9() -> Vec<CorruptionSpec> {
    match v9_freeze_status() {
        Ok(_) => corpus().to_vec(),
        Err(why) => panic!("generation v9 freeze hole: {why}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sites_are_collected_across_all_three_classes() {
        let program = &SEED_PROGRAMS[0];
        let sites = collect_sites(program.code).expect("seed parses");
        assert!(
            sites.singles.len() >= 6,
            "expected a real operator/literal surface"
        );
        let operators = sites
            .singles
            .iter()
            .filter(|s| s.class == CorruptionClass::Operator)
            .count();
        let literals = sites
            .singles
            .iter()
            .filter(|s| s.class == CorruptionClass::Literal)
            .count();
        assert!(operators >= 4, "operator sites missing");
        assert!(literals >= 1, "literal sites missing");
        assert!(
            sites
                .plans
                .iter()
                .any(|p| p.from == "low" && p.to == "high"),
            "same-type identifier plan missing"
        );
        // byte-ordered, non-overlapping singles
        for pair in sites.singles.windows(2) {
            assert!(pair[0].end <= pair[1].start, "sites overlap");
        }
    }

    #[test]
    fn test_module_is_never_corrupted() {
        let program = &SEED_PROGRAMS[0];
        let sites = collect_sites(program.code).expect("seed parses");
        // `mod tests` carries plenty of `assert_eq!` `==` sites and int
        // literals — none of them may appear in the inventory.
        let test_region = program
            .code
            .find("mod tests")
            .expect("seed has a test module");
        for site in &sites.singles {
            assert!(
                site.end <= test_region,
                "site at {} reaches into the test module",
                site.start
            );
        }
    }

    #[test]
    fn while_conditions_yield_no_sites() {
        let code = "pub fn f(n: usize) -> usize {\n    let mut i = 0usize;\n    while i < n {\n        i += 1;\n    }\n    i + 1\n}\n";
        let sites = collect_sites(code).expect("parses");
        assert!(
            !sites.singles.iter().any(|s| &code[s.start..s.end] == "<"),
            "while-condition operator must be excluded (hang law)"
        );
        // the body's `i + 1` IS a site
        assert!(sites.singles.iter().any(|s| s.replacement == "-"));
    }

    #[test]
    fn candidates_are_deterministic_per_seed() {
        let a = build_candidates(0x4352_4239, 3);
        let b = build_candidates(0x4352_4239, 3);
        assert_eq!(a, b, "seeded draws must be byte-identical");
        assert!(!a.is_empty(), "candidate grid empty");
        let c = build_candidates(0x4352_4238, 3);
        assert_ne!(a, c, "different seeds must diverge");
    }

    #[test]
    fn candidate_grid_covers_k_ladder_and_spans_are_disjoint() {
        let candidates = build_candidates(7, 4);
        for candidate in &candidates {
            assert!(LADDER_KS.contains(&candidate.k));
            assert_eq!(candidate.spans.len(), candidate.k);
            let mut spans: Vec<(usize, usize)> = candidate.spans.clone();
            spans.sort_by_key(|(start, end)| (*start, *end));
            for pair in spans.windows(2) {
                assert!(pair[0].1 <= pair[1].0, "spans overlap");
            }
            for (start, end) in &candidate.spans {
                assert!(
                    candidate.code.is_char_boundary(*start)
                        && candidate.code.is_char_boundary(*end),
                    "span {start}..{end} off a char boundary"
                );
                assert!(start < end);
                assert!(*end <= candidate.code.len());
            }
            assert_ne!(candidate.classes, [] as [CorruptionClass; 0]);
        }
    }

    #[test]
    fn plan_candidates_replace_exactly_one_parameter_name() {
        let candidates = build_candidates(11, 16);
        let plan_candidates: Vec<&Candidate> = candidates.iter().filter(|c| c.via_plan).collect();
        assert!(!plan_candidates.is_empty(), "no plan route drawn");
        for candidate in &plan_candidates {
            assert_eq!(candidate.classes, vec![CorruptionClass::Ident]);
            // every replaced span must hold ONE name (the plan's `to`), and
            // that name must be a swap target of the seed's plan table.
            let names: BTreeSet<&str> = candidate
                .spans
                .iter()
                .map(|(s, e)| &candidate.code[*s..*e])
                .collect();
            assert_eq!(names.len(), 1, "plan candidate replaced multiple names");
            let replaced = *names.iter().next().expect("one name");
            let program = SEED_PROGRAMS
                .iter()
                .find(|p| p.id == candidate.seed_id)
                .expect("seed id");
            let sites = collect_sites(program.code).expect("parses");
            assert!(
                sites.plans.iter().any(|p| p.to == replaced),
                "replaced name {replaced} is not a plan target"
            );
        }
    }

    #[test]
    fn noop_corruption_is_rejected_before_cargo() {
        let program = &SEED_PROGRAMS[0];
        assert!(is_noop(program.code, program.code));
        let candidates = build_candidates(3, 2);
        let corrupted = candidates
            .iter()
            .find(|c| c.seed_id == program.id)
            .expect("a candidate for the first seed");
        assert!(!is_noop(&corrupted.code, program.code));
    }

    #[test]
    fn class_tags_round_trip() {
        for class in [
            CorruptionClass::Operator,
            CorruptionClass::Literal,
            CorruptionClass::Ident,
        ] {
            assert_eq!(CorruptionClass::from_tag(class.tag()), Some(class));
        }
        assert_eq!(CorruptionClass::from_tag("nonsense"), None);
    }

    #[test]
    fn width_changing_spans_are_recomputed_in_corrupted_coordinates() {
        // `a < b` corrupted to `a <= b`: the recorded span must cover BOTH
        // characters of `<=` in the corrupted text, and any later site must
        // be shifted by the +1 delta — the metrics consume corrupted-code
        // coordinates, never the seed's.
        let code = "pub fn lt(a: usize, b: usize) -> bool {\n    let early = a < b;\n    let late = a + b > 0;\n    early && late\n}\n";
        let sites = collect_sites(code).expect("parses");
        let edits: Vec<(Range<usize>, String)> = sites
            .singles
            .iter()
            .take(3)
            .map(|s| (s.start..s.end, s.replacement.clone()))
            .collect();
        let (corrupted, spans) = apply_edits_with_spans(code, edits.clone());
        assert_eq!(spans.len(), edits.len());
        for ((start, end), (_, replacement)) in spans.iter().zip(edits.iter()) {
            assert_eq!(&corrupted[*start..*end], replacement);
        }
        // the whole edit set applied: at least one widened operator present
        assert!(
            corrupted.contains("<=")
                || corrupted.contains(">=")
                || corrupted.contains("!=")
                || corrupted.contains("||"),
            "expected a width-changing swap in the corrupted text"
        );
    }

    // The always-on corpus pin (the append-refusal law): the committed
    // corpus must be present, digest-consistent, structurally sound, and
    // above every per-|E| floor. Red until the freeze e2e's write mode has
    // landed the generated table — by design. The control arm may be empty
    // (the v1 type-preserving engine cannot produce compile-failing
    // candidates on these seeds) — disclosed, never demanded.
    #[test]
    fn corpus_v9_pins_hold() {
        match v9_freeze_status() {
            Ok(admitted) => {
                assert!(admitted > 0);
                assert!(control_corpus().len() <= 8, "control arm oversized");
            }
            Err(why) => panic!("generation v9 corpus pins: {why}"),
        }
    }
}
