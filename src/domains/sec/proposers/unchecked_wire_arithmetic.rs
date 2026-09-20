//! `unchecked-wire-arithmetic` — SUGGEST-ONLY detector
//! (Batch 165, Plan 156 T3; Research 193's staged lead, sink-level novelty).
//!
//! A wire-byte-origin integer (parsed via `from_{be,le,ne}_bytes` or a
//! header `.parse()`) flowing through a BARE arithmetic operator —
//! `+` `-` `*` `<<` `>>` (and their compound-assign spellings) — with no
//! intervening bound check is an unchecked overflow: debug builds PANIC,
//! release builds WRAP silently, and a wrapped length/corrupts the frame
//! parse downstream (a `checked_add`-class repair invents an error path
//! the fn may not have — the length-prefix shape).
//!
//! ## Sink-level novelty (Plan 156 T2, recorded)
//!
//! The taint walk is the SHARED `wire_taint` substrate — one walker, two
//! sinks. This module contributes the ARITHMETIC sink; the
//! `length-prefix-validate-before-alloc` rule owns the ALLOC sink. The
//! sinks are deliberately NOT gated against each other (no composition
//! gate): the defects are distinct classes — allocation-size `DoS` vs
//! overflow/wrap — with distinct repairs, and the walker's clearing rule
//! already yields the correct behavior in every ordering. The witness
//! case is the derived-validated shape (`n = len * ROW; if n > MAX; ..`):
//! the alloc rule is correctly silent, this rule CORRECTLY still fires,
//! because the overflow already happened at the multiply and the post-hoc
//! range check validates the WRAPPED value.
//!
//! ## Why suggest-only (never a fix)
//!
//! The bounded repair is a checked op (`checked_add`/`checked_sub`/
//! `checked_mul`/`checked_shl`/`checked_shr`) plus an INVENTED error path
//! (`?`/`ok_or(..)`) whose type the fn's signature may not carry — a
//! protocol decision, never mechanical. NO `FIXABLE_SEC` membership, NO
//! `collect_edits` arm — the suggestion surfaces through retrieval and
//! review (the Batch-131 suggest-only shape, `--fix` never touches it).
//!
//! ## Boundary vs existing lints (measured, Plan 156 T1 + T7 refinement)
//!
//! Real clippy 0.1.98 at the pinned 1.98.1 over the probe crates: ZERO
//! default-group safety lints fire on the buggy forms. `arithmetic_side_effects`
//! TRUSTS literal and const-path operands — SILENT on `len + 8` and
//! `len + CONST` (the canonical shapes this rule targets); where it does
//! fire (variable operands: `len * n`, `len += 8`) it is blanket — every
//! `x * y` site in every codebase, no suggestion, no wire-origin scoping,
//! restriction/allow-by-default. `manual_checked_ops` is `BinOp::Div`-only
//! at this pin. `Div`/`Rem` are NOT this sink (unsigned division cannot
//! overflow; the divisor-zero panic class stays theirs).
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Wire-byte origins only**: the operand must be a bare ident the
//!   shared walker tracks to a parse site in the SAME fn (cross-fn and
//!   field/method origins are out — the honest recall limits).
//! - **NO intervening validation**: a comparison/match/assert involving
//!   the ident first suppresses the site (the shared clearing rule).
//! - **Checked/wrapping/saturating forms are silent**: they are method
//!   calls, not bare operators — the bounded twin never fires.
//! - **`#[cfg(test)]` mods skipped** (the house comment guard).

use std::ops::Range;

use syn::{BinOp, Expr, File};

use super::wire_taint::{self, FnState, WireSink};

/// One detected unchecked wire-origin arithmetic site.
#[derive(Debug, Clone)]
pub struct WireArithFinding {
    /// Byte range of the arithmetic expression.
    pub arith_span: Range<usize>,
    /// The arithmetic expression's source text.
    pub arith_text: String,
    /// The tracked wire-origin ident at the site.
    pub origin_ident: String,
    /// The checked-op repair this site maps to (`checked_add` class).
    pub checked_op: &'static str,
    /// The advice line (the bounded form + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "wire-byte-origin arithmetic can overflow (panic in \
debug, silent wrap in release): use the checked op + an error path, or \
wrapping_*/saturating_* if intentional (Batch 165, suggest-only)";

/// The overflow-prone bare operators → their checked-op repair. syn 2
/// parses compound assignment (`len += 8`) as `Expr::Binary` with the
/// `assign`-suffixed `BinOp` variants, so BOTH spellings route through this
/// one map.
fn checked_op(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::Add(_) | BinOp::AddAssign(_) => Some("checked_add"),
        BinOp::Sub(_) | BinOp::SubAssign(_) => Some("checked_sub"),
        BinOp::Mul(_) | BinOp::MulAssign(_) => Some("checked_mul"),
        BinOp::Shl(_) | BinOp::ShlAssign(_) => Some("checked_shl"),
        BinOp::Shr(_) | BinOp::ShrAssign(_) => Some("checked_shr"),
        // Div/Rem are NOT this sink: unsigned division cannot overflow;
        // the divisor-zero panic class is `manual_checked_ops` territory
        // (Plan 156 T2(b), re-verified Div-only at this pin).
        _ => None,
    }
}

/// The arithmetic sink: a bare overflow-prone operator — `+ - * << >>` or
/// a compound-assign spelling (`+=` etc., syn 2's `assign`-suffixed `BinOp`
/// variants) — with at least one tracked, UNVALIDATED bare-ident operand.
struct ArithSink<'a> {
    findings: &'a mut Vec<WireArithFinding>,
}

impl WireSink for ArithSink<'_> {
    fn check(&mut self, source: &str, expr: &Expr, state: &FnState) {
        // ONE arm covers both spellings: syn 2 parses compound assignment
        // (`len += 8`, `total += wire_len`) as `Expr::Binary` with an
        // `*Assign` BinOp variant. Either operand is the bare tracked
        // ident (paren-unwrapped; `len + 8` and `8 + len` both fire, the
        // left-most tracked operand is reported).
        if let Expr::Binary(b) = expr {
            if let Some(checked) = checked_op(&b.op) {
                self.fire(source, expr, state, checked, [&b.left, &b.right]);
            }
        }
    }
}

impl ArithSink<'_> {
    fn fire(
        &mut self,
        source: &str,
        expr: &Expr,
        state: &FnState,
        checked: &'static str,
        operands: [&Expr; 2],
    ) {
        let Some(origin) = operands.iter().find_map(|e| {
            wire_taint::path_ident(e)
                .filter(|id| state.is_tracked(id) && !state.validated.contains(id))
        }) else {
            return;
        };
        let Some(span) = wire_taint::bounded_span(expr, source) else {
            return;
        };
        self.findings.push(WireArithFinding {
            arith_text: source[span.clone()].to_string(),
            arith_span: span,
            origin_ident: origin,
            checked_op: checked,
            advice: ADVICE,
        });
    }
}

/// Detect unchecked arithmetic on wire-byte-origin integers (suggest-only;
/// no edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<WireArithFinding> {
    let mut findings = Vec::new();
    wire_taint::walk_file(source, file, &mut ArithSink { findings: &mut findings });
    findings
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::WireArithFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    /// Batch 165 gate — the UNBOUNDED wire-frame shape (the corpus
    /// `buggy_code`) MUST fire: parsed length + a constant header size.
    #[test]
    fn batch165_unbounded_wire_add_fires() {
        let src = "pub const FRAME_OVERHEAD: usize = 8;\n\n\
             pub fn framed_payload_len(header: [u8; 4]) -> usize {\n    \
             let len = u32::from_be_bytes(header) as usize;\n    \
             len + FRAME_OVERHEAD\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "one finding expected: {f:?}");
        assert_eq!(f[0].origin_ident, "len");
        assert_eq!(f[0].checked_op, "checked_add");
        assert_eq!(f[0].advice, super::ADVICE);
    }

    /// A parsed magnitude THROUGH A CHAIN fires at every bare-op site:
    /// `count = len * 4` then `total = count - 2` are two overflow sites.
    #[test]
    fn batch165_chain_sites_fire_individually() {
        let src = "pub fn f(prefix: [u8; 4]) -> usize {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let count = len * 4;\n    \
             let total = count - 2;\n    \
             total\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 2, "{f:?}");
        assert_eq!(f[0].origin_ident, "len");
        assert_eq!(f[0].checked_op, "checked_mul");
        assert_eq!(f[1].origin_ident, "count");
        assert_eq!(f[1].checked_op, "checked_sub");
    }

    /// Shift forms are the same sink (`checked_shl` is the repair).
    #[test]
    fn batch165_shl_fires() {
        let src = "pub fn f(prefix: [u8; 4]) -> usize {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let _tag = len << 24;\n    \
             len\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].checked_op, "checked_shl");
    }

    /// The compound-assign spelling is the same site's second form.
    #[test]
    fn batch165_assignop_fires() {
        let src = "pub fn f(prefix: [u8; 4]) -> usize {\n    \
             let mut len = u32::from_be_bytes(prefix) as usize;\n    \
             len += 8;\n    \
             len\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].origin_ident, "len");
        assert_eq!(f[0].checked_op, "checked_add");
    }

    // ── NEGATIVE SUITE — every class must stay SILENT (zero-FP bar) ──

    /// The checked twin (`checked_add`) is a METHOD call, never a bare
    /// operator — the bounded form never fires.
    #[test]
    fn batch165_checked_twin_is_silent() {
        let src = "pub fn framed_payload_len(header: [u8; 4]) -> Option<usize> {\n    \
             let len = u32::from_be_bytes(header) as usize;\n    \
             len.checked_add(FRAME_OVERHEAD)\n}\n\
             pub const FRAME_OVERHEAD: usize = 8;\n";
        assert!(findings(src).is_empty());
    }

    /// Wrapping/saturating intents are method calls too — silent.
    #[test]
    fn batch165_wrapping_and_saturating_are_silent() {
        let src = "pub fn f(prefix: [u8; 4]) -> (usize, usize) {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             (len.wrapping_add(8), len.saturating_mul(4))\n}\n";
        assert!(findings(src).is_empty());
    }

    /// A validated length (comparison BEFORE the arithmetic) is silent.
    #[test]
    fn batch165_validated_len_is_silent() {
        let src = "pub const MAX: usize = 1 << 20;\n\
             pub fn f(prefix: [u8; 4]) -> usize {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             if len > MAX {\n        return 0;\n    }\n    \
             len + 8\n}\n";
        assert!(findings(src).is_empty(), "validated len must not fire");
    }

    /// A comparison SUPPRESSING its own operand: `if len + 8 > MAX` — the
    /// inner add sits under a comparison that validates `len` first.
    #[test]
    fn batch165_comparison_operand_is_silent() {
        let src = "pub const MAX: usize = 1 << 20;\n\
             pub fn f(prefix: [u8; 4]) -> usize {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             if len + 8 > MAX {\n        return 0;\n    }\n    \
             len\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Non-wire integer origins never fire: loop counters, config consts,
    /// trusted `buf.len()` arithmetic.
    #[test]
    fn batch165_non_wire_origins_are_silent() {
        let src = "pub const STRIDE: usize = 16;\n\
             pub fn f(buf: &[u8]) -> usize {\n    \
             let mut acc = 0usize;\n    \
             for i in 0..buf.len() {\n        acc += i * STRIDE;\n    }\n    \
             acc + buf.len() + 8\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Const-foldable expressions (literal operands only) are silent.
    #[test]
    fn batch165_const_foldable_is_silent() {
        let src = "pub fn overhead() -> usize {\n    \
             let base = 4 + 8 * 2;\n    \
             base << 3\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Field/method origins (`self.header_len`) are out — bare idents only.
    #[test]
    fn batch165_field_origin_is_silent() {
        let src = "pub struct Wire { pub header_len: usize }\n\n\
             impl Wire {\n    \
             pub fn framed(&self) -> usize {\n        self.header_len + 8\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Cross-fn origins are out (the helper parses, the caller adds).
    #[test]
    fn batch165_cross_fn_origin_is_silent() {
        let src = "fn parse_len(prefix: [u8; 4]) -> usize {\n    \
             u32::from_be_bytes(prefix) as usize\n}\n\n\
             pub fn framed(prefix: [u8; 4]) -> usize {\n    \
             parse_len(prefix) + 8\n}\n";
        assert!(findings(src).is_empty());
    }

    /// Div/Rem are NOT this sink (no unsigned overflow; the divisor-zero
    /// class is `manual_checked_ops` territory — the T2(b) boundary).
    #[test]
    fn batch165_div_rem_are_silent() {
        let src = "pub fn f(prefix: [u8; 4]) -> (usize, usize) {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             (len / 4, len % 16)\n}\n";
        assert!(findings(src).is_empty());
    }

    /// #[cfg(test)] mods are skipped (the house comment guard).
    #[test]
    fn batch165_test_mod_is_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n    \
             #[test]\n    fn t() {\n        \
             let n: usize = \"8\".parse().unwrap();\n        \
             let _ = n + 8;\n    }\n}\n";
        assert!(findings(src).is_empty());
    }

    /// The findings carry byte spans into the source (span integrity).
    #[test]
    fn batch165_finding_spans_slice_the_source() {
        let src = "pub fn f(prefix: [u8; 4]) -> usize {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             len + 8\n}\n";
        let f = findings(src);
        assert_eq!(f.len(), 1);
        assert_eq!(&src[f[0].arith_span.clone()], "len + 8");
    }

    // ── THE T2 COMPOSITION WITNESSES (sink-distinct, recorded) ──

    /// The composition shape (`len + 8` feeding `vec![0u8; framed]`, no
    /// validation): BOTH rules fire on their OWN sinks with disjoint
    /// spans — two true findings, two distinct repairs. No composition
    /// gate (Plan 156 T2).
    #[test]
    fn batch165_composition_both_sinks_fire_disjointly() {
        let src = "pub fn read(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let framed = len + 8;\n    \
             let mut buf = vec![0_u8; framed];\n    \
             buf\n}\n";
        let arith = findings(src);
        let file = syn::parse_file(src).expect("parses");
        let alloc = super::super::length_prefix_alloc::detect(src, &file);
        assert_eq!(arith.len(), 1, "arith sink: {arith:?}");
        assert_eq!(arith[0].origin_ident, "len");
        assert_eq!(alloc.len(), 1, "alloc sink: {alloc:?}");
        assert_eq!(alloc[0].length_ident, "framed");
        // Disjoint spans: the `+` vs the `vec!`.
        assert!(
            arith[0].arith_span.end <= alloc[0].alloc_span.start,
            "spans must be disjoint: {arith:?} vs {alloc:?}"
        );
    }

    /// THE sink-distinct witness: the derived value is validated AFTER the
    /// arithmetic — the alloc rule is correctly silent (the alloc is
    /// bounded), the arithmetic rule CORRECTLY still fires, because the
    /// overflow already happened at the multiply and the post-hoc range
    /// check validates the WRAPPED value (\"check the multiplication, not
    /// just the result\"). A composition gate would suppress this true
    /// finding — exactly why the sinks are distinct.
    #[test]
    fn batch165_derived_validated_still_fires_arithmetic() {
        let src = "pub const ROW: usize = 16;\n\n\
             pub fn f(prefix: [u8; 4]) -> Vec<u8> {\n    \
             let len = u32::from_be_bytes(prefix) as usize;\n    \
             let n = len * ROW;\n    \
             if n > 1 << 20 {\n        return Vec::new();\n    }\n    \
             vec![0_u8; n]\n}\n";
        let arith = findings(src);
        let file = syn::parse_file(src).expect("parses");
        let alloc = super::super::length_prefix_alloc::detect(src, &file);
        assert_eq!(alloc.len(), 0, "alloc sink correctly silent: {alloc:?}");
        assert_eq!(arith.len(), 1, "the wrap already happened: {arith:?}");
        assert_eq!(arith[0].checked_op, "checked_mul");
    }
}
