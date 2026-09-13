//! Issue 099 T2 — the INTERACTING-error pair corpus.
//!
//! Every corpus the `--fix-compile` driver was measured on until now is built
//! from INDEPENDENT single mutations: even [`super::corpus_gen`]'s composed
//! scaffolds ([`super::corpus_gen::compose_multi`]) put each member in its own
//! `pub mod` over its own bindings — a member's diagnostics cannot depend on
//! another member's repair. Three threads stalled on exactly that property
//! (Bench 087 §6 zero flips / zero reverts; Bench 088's null ordering gain;
//! the floor-realism gap), so this module ships the one missing shape: pairs
//! over SHARED bindings where repairing member A changes member B's
//! diagnostics.
//!
//! ## The 2×2 is structural, never hand-copied
//!
//! Each family builds its four variants (`seed` = both fixed, `broken` = both
//! broken, `a_only`, `b_only`) by string-composing the SAME per-member
//! segment tables — member A's broken segment is byte-identical in `broken`
//! and `a_only`, member B's in `broken` and `b_only`. A hand-copied variant
//! set drifts (the probe pass caught exactly that: a variable-name drift
//! across variants reading as a fake interaction under the line-text key);
//! composing from shared segments makes the drift impossible.
//!
//! ## Admissibility is MEASURED, not asserted (Issue 099 T1)
//!
//! [`interacting_pairs`] emits the DESIGNED cases; `examples/interacting_gate.rs`
//! compiles all four variants of each and admits a case only if the pair
//! diagnostic multiset is not the sum of the singles (set tier), or the
//! playbook candidate ladder for one member's diagnostics differs between
//! the other member's broken/fixed states (ladder tier — Issue 099's
//! mechanism-2 claim). A committed case that fails admissibility is a gate
//! RED (rustc drift changed the interaction), not a silent degradation.
//! The E0505×E0597 "repair-class collision" family from the issue's
//! mechanism 2 was probed and measured NON-interacting on rustc 1.93.0
//! (both members' diagnostics and label sets are exactly additive; the
//! clone-at-move repair does not move the E0597's spans or suggestions) —
//! per the issue's own decision criteria that family is dropped and recorded
//! as a negative, not shipped in the hope the gate would somehow pass it.

use super::corpus_gen::GeneratedCase;
use super::floor::MutatorKind;

/// One interacting pair: four variants of one scaffold, composed from shared
/// segments so the 2×2 admissibility comparison is well-formed by
/// construction.
#[derive(Debug, Clone)]
pub struct InteractingCase {
    pub name: String,
    /// The code member A's mutation declares in `a_only` (the harness's
    /// `expect_code` — the code the cold `broken` variant visibly carries
    /// for member A).
    pub code_a: &'static str,
    /// The code member B's mutation declares in `b_only`. MAY be masked in
    /// the cold `broken` variant — masking is one of the admissible
    /// interaction mechanisms.
    pub code_b: &'static str,
    pub kind: MutatorKind,
    /// Both members fixed — compiles, ground-truth test passes.
    pub seed: String,
    /// Both members broken — the artifact the measurement arms run on.
    pub broken: String,
    /// A broken, B fixed — must declare `code_a`.
    pub a_only: String,
    /// A fixed, B broken — must declare `code_b`.
    pub b_only: String,
}

impl InteractingCase {
    /// The measurement form: the `broken` half plus the seed's known
    /// inverse, in the stock [`GeneratedCase`] shape every harness arm
    /// already consumes. `expect_code` carries member A's code (the cold
    /// variant's visible code), matching `compose_multi`'s inheritance rule.
    #[must_use]
    pub fn as_generated(&self) -> GeneratedCase {
        GeneratedCase {
            name: self.name.clone(),
            expect_code: self.code_a,
            kind: self.kind,
            seed: self.seed.clone(),
            broken: self.broken.clone(),
        }
    }
}

/// The designed interacting corpus (deterministic — same contract as
/// [`super::corpus_gen::generate_corpus`]: pure, table-driven, no RNG).
///
/// # Panics
///
/// Never: the tables are compile-time constants and every template is total.
#[must_use]
pub fn interacting_pairs() -> Vec<InteractingCase> {
    let mut out = Vec::new();
    e0502_e0499_head_first_copy(&mut out);
    e0502_e0499_head_first_string(&mut out);
    e0502_e0499_mid_read(&mut out);
    e0614_e0499_stale_star(&mut out);
    out
}

// ── F1/F2: the head-first masking shape (E0502 × E0499 over one `v`) ──
//
// Member A = hold an immutable read across the mutation (E0502 shape);
// member B = the mutable alias pair (E0499 shape). Measured interaction
// (rustc 1.93.0): with A broken, B's conflict renders as a SECOND E0502
// pointing at B's decl — B's own E0499 only exists once A is repaired.
// Repairing A therefore changes B's diagnostic set (masking), and the
// repair order matters: the E0502 playbook targets the shared immutable
// borrow (one candidate repairs both diagnostics at once), while repairing
// B first would still leave A's diagnostic on B's line.

/// F1 — Copy element stratum (the E0502 deref-removal strategy applies).
fn e0502_e0499_head_first_copy(out: &mut Vec<InteractingCase>) {
    // (type, initial vec, push x, push y, head expected)
    const TABLE: [(&str, &str, &str, &str); 12] = [
        ("i32", "1i32, 2, 3", "4i32", "5i32"),
        ("i32", "10i32, 20", "30i32", "40i32"),
        ("i32", "0i32, 0, 0", "1i32", "2i32"),
        ("u8", "1u8, 2, 3", "4u8", "5u8"),
        ("u8", "9u8", "1u8", "2u8"),
        ("u64", "1u64, 2, 3", "4u64", "5u64"),
        ("u64", "100u64, 200", "7u64", "8u64"),
        ("i16", "1i16, 2, 3", "4i16", "5i16"),
        ("i16", "50i16", "60i16", "70i16"),
        ("u32", "1u32, 2, 3", "4u32", "5u32"),
        ("u32", "11u32, 22", "33u32", "44u32"),
        ("i64", "1i64, 2, 3", "4i64", "5i64"),
    ];
    for (i, (ty, init, x, y)) in TABLE.iter().enumerate() {
        // The seed's return is head + v[0] after the pushes: the pushes
        // APPEND, so v[0] stays the first element and r = 2 * first.
        let first = init.split(',').next().unwrap_or_default().trim();
        let gt = gt_module(&format!(
            "let mut v = vec![{init}];\n        let r = super::probe(&mut v);\n        assert_eq!(r, 2 * {first});\n        assert_eq!(v.len(), {});",
            init.split(',').count() + 2
        ));
        // The four variants compose from the SAME three segments: the head
        // read (A's mutation site), the alias pair (B's mutation site), and
        // the shared scaffold.
        let a_fixed = "let head = v[0];".to_string();
        let a_broken = "let head = &v[0];".to_string();
        let b_fixed = format!("let a = &mut *v;\n    a.push({x});\n    let b = &mut *v;\n    b.push({y});");
        let b_broken = format!("let a = &mut *v;\n    let b = &mut *v;\n    a.push({x});\n    b.push({y});");
        let head_use_fixed = "head + v[0]";
        let head_use_broken = "*head + v[0]";
        let wrap = |head: &str, pair: &str, usex: &str| {
            format!(
                "pub fn probe(v: &mut Vec<{ty}>) -> {ty} {{\n    {head}\n    {pair}\n    {usex}\n}}\n{gt}"
            )
        };
        push_case(
            out,
            format!("ix_f1_e0502_e0499_copy_{i:02}_{ty}"),
            "E0502",
            "E0499",
            MutatorKind::AddBorrow,
            wrap(&a_fixed, &b_fixed, head_use_fixed),
            wrap(&a_broken, &b_broken, head_use_broken),
            wrap(&a_broken, &b_fixed, head_use_broken),
            wrap(&a_fixed, &b_broken, head_use_fixed),
        );
    }
}

/// F2 — String stratum (deref-removal cannot fire — clone-out is the only
/// E0502 repair; the head use goes through `.len()` like the committed
/// `e0502_len_borrow_across_push` anchor).
fn e0502_e0499_head_first_string(out: &mut Vec<InteractingCase>) {
    // (initial vec, pushes)
    const TABLE: [(&str, &str, &str, usize); 10] = [
        ("vec![String::from(\"aa\"), String::from(\"bb\")]", "String::from(\"x\")", "String::from(\"y\")", 2),
        ("vec![String::from(\"hello\")]", "String::from(\"w\")", "String::from(\"z\")", 1),
        ("vec![String::from(\"m\"), String::from(\"n\"), String::from(\"o\")]", "String::from(\"p\")", "String::from(\"q\")", 3),
        ("vec![String::from(\"first\"), String::from(\"second\")]", "String::from(\"third\")", "String::from(\"fourth\")", 2),
        ("vec![String::from(\"\")]", "String::from(\"a\")", "String::from(\"b\")", 1),
        ("vec![String::from(\"xx\"), String::from(\"yy\")]", "String::from(\"zz\")", "String::from(\"ww\")", 2),
        ("vec![String::from(\"one\")]", "String::from(\"two\")", "String::from(\"three\")", 1),
        ("vec![String::from(\"k1\"), String::from(\"k2\"), String::from(\"k3\"), String::from(\"k4\")]", "String::from(\"k5\")", "String::from(\"k6\")", 4),
        ("vec![String::from(\"left\"), String::from(\"right\")]", "String::from(\"up\")", "String::from(\"down\")", 2),
        ("vec![String::from(\"tok\")]", "String::from(\"a\")", "String::from(\"b\")", 1),
    ];
    for (i, (init, x, y, n)) in TABLE.iter().enumerate() {
        // r = head.len() + v[0].len() after the pushes = 2 * v[0].len()
        // (the pushes append; v[0] is unchanged).
        let gt = gt_module(&format!(
            "let mut v = {init};\n        let r = super::probe(&mut v);\n        assert_eq!(r, v[0].len() * 2);\n        assert_eq!(v.len(), {});",
            n + 2
        ));
        let a_fixed = "let head = v[0].len();".to_string();
        let a_broken = "let head = &v[0];".to_string();
        let b_fixed = format!("let a = &mut *v;\n    a.push({x});\n    let b = &mut *v;\n    b.push({y});");
        let b_broken = format!("let a = &mut *v;\n    let b = &mut *v;\n    a.push({x});\n    b.push({y});");
        let head_use_fixed = "head + v[0].len()";
        let head_use_broken = "head.len() + v[0].len()";
        let wrap = |head: &str, pair: &str, usex: &str| {
            format!("pub fn probe(v: &mut Vec<String>) -> usize {{\n    {head}\n    {pair}\n    {usex}\n}}\n{gt}")
        };
        push_case(
            out,
            format!("ix_f2_e0502_e0499_string_{i:02}"),
            "E0502",
            "E0499",
            MutatorKind::AddBorrow,
            wrap(&a_fixed, &b_fixed, head_use_fixed),
            wrap(&a_broken, &b_broken, head_use_broken),
            wrap(&a_broken, &b_fixed, head_use_broken),
            wrap(&a_fixed, &b_broken, head_use_fixed),
        );
    }
}

/// F3 — the mid-read shape: A's immutable read sits BETWEEN the two mutable
/// decls. Measured interaction (rustc 1.93.0): with B broken, even A's
/// FIXED form errors (the by-value `v[0]` index-read is an instant
/// immutable borrow conflicting with the live first mutable borrow), and
/// with both broken the second-decl conflict renders as B's E0499 while
/// A-repaired-alone renders it as an E0502 — B's diagnostic CODE depends on
/// A's state, the strongest masking form. The repair-order stakes are the
/// point: repairing B first (swap-second-decl) re-hangs an E0502 on B's
/// moved decl (no progress → revert), repairing A first (deref-removal)
/// clears both paths.
fn e0502_e0499_mid_read(out: &mut Vec<InteractingCase>) {
    const TABLE: [(&str, &str, &str, &str); 12] = [
        ("i32", "1i32, 2, 3", "4i32", "5i32"),
        ("i32", "7i32, 8", "9i32", "10i32"),
        ("i32", "0i32", "1i32", "2i32"),
        ("u8", "1u8, 2, 3", "4u8", "5u8"),
        ("u8", "42u8", "43u8", "44u8"),
        ("u16", "1u16, 2, 3", "4u16", "5u16"),
        ("u32", "5u32, 6, 7", "8u32", "9u32"),
        ("u32", "12u32, 13", "14u32", "15u32"),
        ("u64", "1u64, 2, 3", "4u64", "5u64"),
        ("i16", "3i16, 4", "5i16", "6i16"),
        ("i64", "9i64", "10i64", "11i64"),
        ("i64", "1i64, 2, 3, 4", "5i64", "6i64"),
    ];
    for (i, (ty, init, x, y)) in TABLE.iter().enumerate() {
        let gt = gt_module(&format!(
            "let mut v = vec![{init}];\n        let r = super::probe(&mut v);\n        assert_eq!(r, v[0] * 2);\n        assert_eq!(v.len(), {});",
            init.split(',').count() + 2
        ));
        // A's segment sits between the alias pair's two decls when broken;
        // after the pair's first use when fixed. B's fixed/broken forms are
        // the standard sequential/adjacent pair.
        let a_fixed_b_fixed = format!("let a = &mut *v;\n    a.push({x});\n    let head = v[0];\n    let b = &mut *v;\n    b.push({y});");
        let a_broken_b_fixed = format!("let a = &mut *v;\n    a.push({x});\n    let head = &v[0];\n    let b = &mut *v;\n    b.push({y});");
        let a_fixed_b_broken = format!("let a = &mut *v;\n    let head = v[0];\n    let b = &mut *v;\n    a.push({x});\n    b.push({y});");
        let a_broken_b_broken = format!("let a = &mut *v;\n    let head = &v[0];\n    let b = &mut *v;\n    a.push({x});\n    b.push({y});");
        let wrap = |pair: &str, usex: &str| {
            format!("pub fn probe(v: &mut Vec<{ty}>) -> {ty} {{\n    {pair}\n    {usex}\n}}\n{gt}")
        };
        push_case(
            out,
            format!("ix_f3_e0502_e0499_mid_{i:02}_{ty}"),
            "E0502",
            "E0499",
            MutatorKind::AddBorrow,
            wrap(&a_fixed_b_fixed, "head + v[0]"),
            wrap(&a_broken_b_broken, "*head + v[0]"),
            wrap(&a_broken_b_fixed, "*head + v[0]"),
            wrap(&a_fixed_b_broken, "head + v[0]"),
        );
    }
}

/// F4 — the stale-star stratum (Issue 100): member A's mutation puts a deref
/// star on a BY-VALUE binding's use (`head` → `*head`), the organic E0614;
/// member B is the alias pair (E0499). Measured interaction (rustc 1.98.1,
/// Issue 100's probes): the E0614 BLOCKS borrowck entirely — the broken pair
/// reports exactly the one E0614; B's E0499 exists only once the star is
/// removed. This is the family `e0614-deref-at-use` + the lexicographic keep
/// rule exist to resolve: star-removal is a KEEP at equal count (masking
/// 1→0 — the unmasking trade), then the swap finishes in one more keep.
fn e0614_e0499_stale_star(out: &mut Vec<InteractingCase>) {
    const TABLE: [(&str, &str, &str, &str); 12] = [
        ("i32", "2i32, 3, 5", "7i32", "11i32"),
        ("i32", "13i32, 17", "19i32", "23i32"),
        ("i32", "1i32, 1", "2i32", "3i32"),
        ("u8", "3u8, 5, 7", "11u8", "13u8"),
        ("u8", "17u8", "19u8", "23u8"),
        ("u64", "2u64, 4, 6", "8u64", "10u64"),
        ("i16", "4i16, 6, 8", "10i16", "12i16"),
        ("i16", "20i16", "30i16", "40i16"),
        ("u32", "6u32, 8, 10", "12u32", "14u32"),
        ("u32", "20u32, 22", "24u32", "26u32"),
        ("i64", "3i64", "4i64", "5i64"),
        ("i64", "1i64, 3, 5, 7", "9i64", "11i64"),
    ];
    for (i, (ty, init, x, y)) in TABLE.iter().enumerate() {
        // Same ground truth as F1: pushes APPEND, so v[0] stays the first
        // element and r = head + v[0] = 2 * first.
        let first = init.split(',').next().unwrap_or_default().trim();
        let gt = gt_module(&format!(
            "let mut v = vec![{init}];\n        let r = super::probe(&mut v);\n        assert_eq!(r, 2 * {first});\n        assert_eq!(v.len(), {});",
            init.split(',').count() + 2
        ));
        // A's segments: the binding is by-value in EVERY variant — the
        // mutation is the stale star at the USE (byte-identical in `broken`
        // and `a_only`, the shared-segment invariant).
        let a_binding = "let head = v[0];";
        let use_fixed = "head + v[0]";
        let use_broken = "*head + v[0]";
        let b_fixed = format!("let a = &mut *v;\n    a.push({x});\n    let b = &mut *v;\n    b.push({y});");
        let b_broken = format!("let a = &mut *v;\n    let b = &mut *v;\n    a.push({x});\n    b.push({y});");
        let wrap = |usex: &str, pair: &str| {
            format!(
                "pub fn probe(v: &mut Vec<{ty}>) -> {ty} {{\n    {a_binding}\n    {pair}\n    {usex}\n}}\n{gt}"
            )
        };
        push_case(
            out,
            format!("ix_f4_e0614_e0499_star_{i:02}_{ty}"),
            "E0614",
            "E0499",
            MutatorKind::AddDerefStar,
            wrap(use_fixed, &b_fixed),
            wrap(use_broken, &b_broken),
            wrap(use_broken, &b_fixed),
            wrap(use_fixed, &b_broken),
        );
    }
}

fn gt_module(body: &str) -> String {
    format!(
        "\n#[cfg(test)]\nmod ground_truth {{\n    #[test]\n    fn behavior() {{\n        {body}\n    }}\n}}\n"
    )
}

#[allow(clippy::too_many_arguments)]
fn push_case(
    out: &mut Vec<InteractingCase>,
    name: String,
    code_a: &'static str,
    code_b: &'static str,
    kind: MutatorKind,
    seed: String,
    broken: String,
    a_only: String,
    b_only: String,
) {
    out.push(InteractingCase {
        name,
        code_a,
        code_b,
        kind,
        seed,
        broken,
        a_only,
        b_only,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generation_is_deterministic() {
        let a = interacting_pairs();
        let b = interacting_pairs();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.name, y.name, "name order drifted");
            assert_eq!(x.seed, y.seed, "seed drifted for {}", x.name);
            assert_eq!(x.broken, y.broken, "broken drifted for {}", x.name);
            assert_eq!(x.a_only, y.a_only, "a_only drifted for {}", x.name);
            assert_eq!(x.b_only, y.b_only, "b_only drifted for {}", x.name);
        }
    }

    #[test]
    fn names_are_unique() {
        let set: HashSet<String> = interacting_pairs().iter().map(|c| c.name.clone()).collect();
        assert_eq!(set.len(), interacting_pairs().len());
    }

    #[test]
    fn every_variant_carries_a_ground_truth_test() {
        for c in interacting_pairs() {
            for which in [&c.seed, &c.broken, &c.a_only, &c.b_only] {
                assert!(
                    which.contains("#[cfg(test)]") && which.contains("fn behavior()"),
                    "{}: a variant lost its ground-truth test",
                    c.name
                );
            }
        }
    }

    #[test]
    fn braces_balance_in_all_four_variants() {
        for c in interacting_pairs() {
            for (which, src) in
                [("seed", &c.seed), ("broken", &c.broken), ("a_only", &c.a_only), ("b_only", &c.b_only)]
            {
                let open = src.matches('{').count();
                let close = src.matches('}').count();
                assert_eq!(
                    open, close,
                    "{} {which}: unbalanced braces ({open} vs {close})",
                    c.name
                );
            }
        }
    }

    /// The four variants must be pairwise distinct — a duplicated variant is
    /// a mis-composed table row and would silently degrade the 2×2.
    #[test]
    fn variants_are_pairwise_distinct() {
        for c in interacting_pairs() {
            let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
            let (s, b, a, o) = (
                norm(&c.seed),
                norm(&c.broken),
                norm(&c.a_only),
                norm(&c.b_only),
            );
            assert_ne!(s, b, "{}: seed == broken", c.name);
            assert_ne!(s, a, "{}: seed == a_only", c.name);
            assert_ne!(s, o, "{}: seed == b_only", c.name);
            assert_ne!(b, a, "{}: broken == a_only", c.name);
            assert_ne!(b, o, "{}: broken == b_only", c.name);
            assert_ne!(a, o, "{}: a_only == b_only", c.name);
        }
    }

    /// The 2×2 is structural: `broken` and `a_only` differ only in member
    /// B's segment (same for `broken`/`b_only` and A) — the composition
    /// invariant the admissibility gate's union comparison relies on.
    /// Checked by the cheap proxy: few lines of `a_only` may be absent
    /// from `broken`'s line multiset (exactly B's fixed-form lines).
    #[test]
    fn broken_and_a_only_share_all_lines_but_bs() {
        for c in interacting_pairs() {
            let a_lines: std::collections::BTreeMap<&str, usize> =
                c.a_only.lines().fold(std::collections::BTreeMap::new(), |mut m, l| {
                    *m.entry(l).or_default() += 1;
                    m
                });
            let b_lines: std::collections::BTreeMap<&str, usize> =
                c.broken.lines().fold(std::collections::BTreeMap::new(), |mut m, l| {
                    *m.entry(l).or_default() += 1;
                    m
                });
            let only_in_a: usize = a_lines
                .iter()
                .map(|(l, n)| n.saturating_sub(*b_lines.get(l).unwrap_or(&0)))
                .sum();
            assert!(
                only_in_a <= 4,
                "{}: a_only has {} lines absent from broken — composition drifted",
                c.name,
                only_in_a
            );
        }
    }

    #[test]
    fn as_generated_carries_the_broken_half_and_a_code() {
        for c in interacting_pairs() {
            let g = c.as_generated();
            assert_eq!(g.broken, c.broken);
            assert_eq!(g.seed, c.seed);
            assert_eq!(g.expect_code, c.code_a);
        }
    }
}
