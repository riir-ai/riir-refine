//! Plan 019 T6.1 — the generated known-inverse mutation corpus (N ≥ 200).
//!
//! The committed 12 anchors ([`super::floor::MUTANT_CASES`]) are hand-authored
//! `from`/`to` pairs. This module scales the corpus to the plan's N ≥ 200 via
//! PARAMETRIC seed templates: fixed tables of (type, value, identifier)
//! tuples expanded over each error family, so every case is a compiling seed
//! carrying a `#[cfg(test)]` ground-truth behavior test plus the exact
//! rewrite that breaks it. The generator is pure and table-driven — no RNG,
//! no environment reads — so the corpus is byte-stable per build (pinned by
//! `generation_is_deterministic`), which is the "committed for determinism"
//! contract without a 4,000-line literal table.
//!
//! Why the behavior test matters: the harness scores `matched-inverse` by
//! whitespace-normalized text, and the plan records that a compiling fix can
//! still change behavior. Every generated seed therefore pins its own output
//! (`assert_eq!(super::probe(..), ..)`), so the post-fix `cargo test` pass
//! (the G1 run) verifies SEMANTICS, not just compilation, for every fix the
//! arms land — including the semantically-equal-but-textually-diverged ones.
//!
//! Stratification (the anti-inflation rule): uncontested families
//! (E0597/E0499/E0502 — codes where `cargo fix` measures 0%) are the product
//! head; contested families (E0505/E0596/E0382/E0507/E0373) are carried so
//! the floor's numbers stay honest at scale, but coverage claims are
//! reported PER TERRITORY, never blended.

use super::floor::MutatorKind;

/// Which side of the `cargo fix` territory line a code sits on (Bench 041).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Territory {
    /// rustc ships no machine-applicable fix (E0597/E0499/E0502) — any
    /// coverage here is net-new.
    Uncontested,
    /// rustc already repairs these; we ship a template only where the floor
    /// measured a divergence (E0505) or delegate (the rest).
    Contested,
}

/// The uncontested code set (kept in one place so the strata can never
/// drift from the premise probe).
#[must_use]
pub fn territory_of(code: &str) -> Territory {
    match code {
        "E0597" | "E0499" | "E0502" => Territory::Uncontested,
        _ => Territory::Contested,
    }
}

/// One generated case: the owned form the harness runs (the committed
/// anchors borrow `&'static str`; generated seeds are built with
/// `format!`).
#[derive(Debug, Clone)]
pub struct GeneratedCase {
    pub name: String,
    pub expect_code: &'static str,
    pub kind: MutatorKind,
    pub seed: String,
    pub broken: String,
}

/// The full generated corpus (deterministic — see the module docs).
///
/// # Panics
///
/// Never: the parameter tables are compile-time constants and every
/// template is total.
#[must_use]
pub fn generate_corpus() -> Vec<GeneratedCase> {
    let mut out = Vec::with_capacity(200);
    e0597_string_borrow(&mut out);
    e0597_vec_slice(&mut out);
    e0597_as_str(&mut out);
    e0597_format(&mut out);
    e0597_vec_string_index(&mut out);
    e0499_flatten(&mut out);
    e0502_copy_index(&mut out);
    e0502_string_len(&mut out);
    e0505_drop_clone(&mut out);
    e0505_nonclone_borrow_move(&mut out);
    e0382_nonclone_use_after_move(&mut out);
    e0382_clone_use_after_move(&mut out);
    e0596_drop_mut(&mut out);
    e0382_drop_clone(&mut out);
    e0507_drop_clone(&mut out);
    e0373_drop_move(&mut out);
    out
}

/// Compose `k` single-mutation cases into ONE scaffold, each body wrapped in
/// its own `pub mod cN` — so a run carries `k` SIMULTANEOUS errors instead of
/// one (Issue 91 T2 / Bench 087's "power lever", lifted here so more than one
/// harness can use it).
///
/// Why it exists: [`generate_corpus`] is one-mutation-per-scaffold, so the
/// `--fix-compile` driver makes exactly ONE apply→recheck decision per run.
/// Every within-run property — a stateful ordering's feedback (Issue 093 T2),
/// an oscillation flip (Issue 91 T2), the revive policy — needs ≥2 decisions,
/// and is therefore unreachable BY CONSTRUCTION on the stock corpus. A zero
/// measured there is a fact about the corpus, not about the mechanism.
///
/// `k <= 1` returns the input unchanged (cloned). The composed case inherits
/// `chunk[0]`'s `expect_code` — the harness's mutant-validity check only asks
/// that the declared code is AMONG the pre-fix diagnostics, which holds.
#[must_use]
pub fn compose_multi(cases: &[GeneratedCase], k: usize) -> Vec<GeneratedCase> {
    if k <= 1 {
        return cases.to_vec();
    }
    cases
        .chunks(k)
        .enumerate()
        .map(|(g, chunk)| {
            let wrap = |pick: fn(&GeneratedCase) -> &str| -> String {
                use std::fmt::Write as _;
                let mut out = String::new();
                for (i, c) in chunk.iter().enumerate() {
                    let body = pick(c).replace('\n', "\n    ");
                    // Infallible: writing into a String never errors.
                    let _ = write!(out, "pub mod c{i} {{\n    {body}\n}}\n");
                }
                out
            };
            GeneratedCase {
                name: format!("multi{k}_{g:03}"),
                expect_code: chunk[0].expect_code,
                kind: MutatorKind::NarrowScope,
                seed: wrap(|c| c.seed.as_str()),
                broken: wrap(|c| c.broken.as_str()),
            }
        })
        .collect()
}

/// Compose like [`compose_multi`], but draw the `k` members by STRIDE
/// instead of from a contiguous chunk — so one scaffold carries `k`
/// different mutation FAMILIES rather than `k` variants of one.
///
/// Why both exist (Bench 088's measurement): [`generate_corpus`] emits
/// families contiguously, so a chunked group of 5 is usually five variants
/// of the SAME template — the driver's candidate menu for that scaffold then
/// has ONE distinct `strategy_id`, and any per-strategy ordering policy is
/// the identity on it regardless of how many attempts the run makes. The
/// strided form is the only one that puts a genuinely WIDE menu in front of
/// the ordering seam, which is what an ordering A/B needs in order to be
/// able to measure anything at all.
#[must_use]
pub fn compose_multi_strided(cases: &[GeneratedCase], k: usize) -> Vec<GeneratedCase> {
    if k <= 1 || cases.len() < k {
        return cases.to_vec();
    }
    let groups = cases.len() / k;
    (0..groups)
        .map(|g| {
            let members: Vec<&GeneratedCase> = (0..k).map(|i| &cases[i * groups + g]).collect();
            let wrap = |pick: fn(&GeneratedCase) -> &str| -> String {
                use std::fmt::Write as _;
                let mut out = String::new();
                for (i, c) in members.iter().enumerate() {
                    let body = pick(c).replace('\n', "\n    ");
                    let _ = write!(out, "pub mod c{i} {{\n    {body}\n}}\n");
                }
                out
            };
            GeneratedCase {
                name: format!("strided{k}_{g:03}"),
                expect_code: members[0].expect_code,
                kind: MutatorKind::NarrowScope,
                seed: wrap(|c| c.seed.as_str()),
                broken: wrap(|c| c.broken.as_str()),
            }
        })
        .collect()
}

fn gt_module(body: &str) -> String {
    format!(
        "\n#[cfg(test)]\nmod ground_truth {{\n    #[test]\n    fn behavior() {{\n        {body}\n    }}\n}}\n"
    )
}

fn push_case(
    out: &mut Vec<GeneratedCase>,
    name: String,
    expect_code: &'static str,
    kind: MutatorKind,
    seed: String,
    broken: String,
) {
    out.push(GeneratedCase {
        name,
        expect_code,
        kind,
        seed,
        broken,
    });
}

// ── E0597: borrow outlives owner (uncontested head) ───────────────────

/// `let owned = String::from(..); let r: &str = &owned;` narrowed so the
/// owner drops inside an inner block.
fn e0597_string_borrow(out: &mut Vec<GeneratedCase>) {
    const LITERALS: [(&str, usize); 10] = [
        ("hello", 5),
        ("world!", 6),
        ("rustc", 5),
        ("oracle", 6),
        ("borrow", 6),
        ("lifetime", 8),
        ("mutant", 6),
        ("seed", 4),
        ("fix", 3),
        ("compile", 7),
    ];
    for (i, (lit, len)) in LITERALS.iter().enumerate() {
        let seed = format!(
            "pub fn probe() -> usize {{\n    let owned = String::from(\"{lit}\");\n    let r: &str = &owned;\n    r.len()\n}}\n{}",
            gt_module(&format!("assert_eq!(super::probe(), {len});"))
        );
        let broken = format!(
            "pub fn probe() -> usize {{\n    let r: &str;\n    {{\n        let owned = String::from(\"{lit}\");\n        r = &owned;\n    }}\n    r.len()\n}}\n{}",
            gt_module(&format!("assert_eq!(super::probe(), {len});"))
        );
        push_case(
            out,
            format!("gen_e0597_str_borrow_{i:02}_{lit}"),
            "E0597",
            MutatorKind::NarrowScope,
            seed,
            broken,
        );
    }
}

/// `let owned = vec![..]; let r: &[T] = &owned[..];` narrowed.
fn e0597_vec_slice(out: &mut Vec<GeneratedCase>) {
    const TABLE: [(&str, &str, usize, usize); 10] = [
        // (type, elems, elem_count, expected_len)
        ("u8", "1u8, 2, 3, 4", 4, 4),
        ("u8", "9u8, 8, 7, 6, 5, 4, 3", 7, 7),
        ("i32", "10i32, 20, 30", 3, 3),
        ("i32", "1i32, 2, 3, 4, 5", 5, 5),
        ("u64", "100u64, 200", 2, 2),
        ("u16", "1u16, 2, 3, 4, 5, 6", 6, 6),
        ("u32", "7u32, 8, 9", 3, 3),
        ("i16", "3i16, 1, 4, 1", 4, 4),
        ("f32", "1.5f32, 2.5", 2, 2),
        ("u8", "0u8", 1, 1),
    ];
    for (i, (ty, elems, _, len)) in TABLE.iter().enumerate() {
        let gt = gt_module(&format!("assert_eq!(super::probe(), {len});"));
        let seed = format!(
            "pub fn probe() -> usize {{\n    let owned = vec![{elems}];\n    let r: &[{ty}] = &owned[..];\n    r.len()\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe() -> usize {{\n    let r: &[{ty}];\n    {{\n        let owned = vec![{elems}];\n        r = &owned[..];\n    }}\n    r.len()\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0597_slice_{i:02}_{ty}"),
            "E0597",
            MutatorKind::NarrowScope,
            seed,
            broken,
        );
    }
}

/// `owned.as_str()` narrowed (the borrow is implicit in the return type).
fn e0597_as_str(out: &mut Vec<GeneratedCase>) {
    const LITERALS: [(&str, usize); 5] = [
        ("alpha", 5),
        ("bravo", 5),
        ("charlie", 7),
        ("delta", 5),
        ("echo-fox", 8),
    ];
    for (i, (lit, len)) in LITERALS.iter().enumerate() {
        let gt = gt_module(&format!("assert_eq!(super::probe(), {len});"));
        let seed = format!(
            "pub fn probe() -> usize {{\n    let owned = String::from(\"{lit}\");\n    let r = owned.as_str();\n    r.len()\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe() -> usize {{\n    let r: &str;\n    {{\n        let owned = String::from(\"{lit}\");\n        r = owned.as_str();\n    }}\n    r.len()\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0597_as_str_{i:02}_{lit}"),
            "E0597",
            MutatorKind::NarrowScope,
            seed,
            broken,
        );
    }
}

/// `format!`-built owner narrowed.
fn e0597_format(out: &mut Vec<GeneratedCase>) {
    // (prefix, arg, result len): template = "{prefix}{{}}", so
    // `format!("x-{}", 7)` builds "x-7" (len 3).
    const TABLE: [(&str, &str, usize); 5] = [
        ("x-", "7", 3),
        ("y-", "14", 4),
        ("z-", "23", 4),
        ("w-", "105", 5),
        ("q-", "2", 3),
    ];
    for (i, (prefix, arg, len)) in TABLE.iter().enumerate() {
        let gt = gt_module(&format!("assert_eq!(super::probe(), {len});"));
        let template = format!("{prefix}{{}}");
        let seed = format!(
            "pub fn probe() -> usize {{\n    let owned = format!(\"{template}\", {arg});\n    let r: &str = &owned;\n    r.len()\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe() -> usize {{\n    let r: &str;\n    {{\n        let owned = format!(\"{template}\", {arg});\n        r = &owned;\n    }}\n    r.len()\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0597_fmt_{i:02}"),
            "E0597",
            MutatorKind::NarrowScope,
            seed,
            broken,
        );
    }
}

/// `&owned[idx]` over `Vec<String>` narrowed.
fn e0597_vec_string_index(out: &mut Vec<GeneratedCase>) {
    // (elems expr, idx, idx_len) — lens MECHANICALLY verified (see the
    // e0502_len table's lesson note).
    const TABLE: [(&str, usize, usize); 10] = [
        ("vec![String::from(\"aa\"), String::from(\"bbb\")]", 1, 3),
        ("vec![String::from(\"aa\"), String::from(\"bbb\")]", 0, 2),
        (
            "vec![String::from(\"x\"), String::from(\"yy\"), String::from(\"zzz\")]",
            2,
            3,
        ),
        (
            "vec![String::from(\"x\"), String::from(\"yy\"), String::from(\"zzz\")]",
            1,
            2,
        ),
        ("vec![String::from(\"\")]", 0, 0),
        ("vec![String::from(\"four\"), String::from(\"five\")]", 1, 4),
        ("vec![String::from(\"six\"), String::from(\"seven\")]", 0, 3),
        (
            "vec![String::from(\"eight8\"), String::from(\"nine9\")]",
            1,
            5,
        ),
        (
            "vec![String::from(\"ten10\"), String::from(\"eleven11\")]",
            1,
            8,
        ),
        (
            "vec![String::from(\"twelve12\"), String::from(\"th\")]",
            0,
            8,
        ),
    ];
    for (i, (elems, idx, ilen)) in TABLE.iter().enumerate() {
        let gt = gt_module(&format!("assert_eq!(super::probe(), {ilen});"));
        let seed = format!(
            "pub fn probe() -> usize {{\n    let owned = {elems};\n    let r: &str = &owned[{idx}];\n    r.len()\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe() -> usize {{\n    let r: &str;\n    {{\n        let owned = {elems};\n        r = &owned[{idx}];\n    }}\n    r.len()\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0597_str_idx_{i:02}"),
            "E0597",
            MutatorKind::NarrowScope,
            seed,
            broken,
        );
    }
}

// ── E0499: two `&mut` alive at once (uncontested head) ────────────────

/// Block-scoped borrow flattened so both `&mut` are live simultaneously.
fn e0499_flatten(out: &mut Vec<GeneratedCase>) {
    // (type, x, y) — the pushed pair; ground truth asserts final contents.
    const TYPES: [&str; 5] = ["u8", "i32", "u64", "i16", "u32"];
    const PAIRS: [(u32, u32); 5] = [(1, 2), (3, 4), (5, 7), (10, 20), (100, 200)];
    for (ti, ty) in TYPES.iter().enumerate() {
        for (pi, (x, y)) in PAIRS.iter().enumerate() {
            let gt = gt_module(&format!(
                "let mut v = vec![0{ty}];\n        super::probe(&mut v);\n        assert_eq!(v, vec![0{ty}, {x}{ty}, {y}{ty}]);"
            ));
            let seed = format!(
                "pub fn probe(v: &mut Vec<{ty}>) {{\n    {{\n        let a = &mut *v;\n        a.push({x}{ty});\n    }}\n    let b = &mut *v;\n    b.push({y}{ty});\n}}\n{gt}"
            );
            let broken = format!(
                "pub fn probe(v: &mut Vec<{ty}>) {{\n    let a = &mut *v;\n    let b = &mut *v;\n    a.push({x}{ty});\n    b.push({y}{ty});\n}}\n{gt}"
            );
            push_case(
                out,
                format!("gen_e0499_flat_{ti:02}_{pi:02}_{ty}"),
                "E0499",
                MutatorKind::FlattenScope,
                seed,
                broken,
            );
        }
    }
}

// ── E0502: immutable borrow across mutation (uncontested head) ───────

/// Copy element borrowed across a `push` (index + deref at the use site —
/// the committed anchor's integrity lesson: a missing `*` measures E0606,
/// not E0502). The idx/expected pairs are EXPLICIT per row — a derived
/// index can run off the end of the initial vec and panic the seed's own
/// ground-truth test (idx 2 on a 2-element vec), which the brace-balance
/// net cannot catch.
// The 20-row table is the data — 118 lines under `kat_transport` (the
// function carries no cfg, but the sibling crates' line-count baseline
// shifts with the feature graph), 100 is the pedantic budget. Splitting it
// would separate the table from its consumer for nothing.
#[allow(clippy::too_many_lines)]
fn e0502_copy_index(out: &mut Vec<GeneratedCase>) {
    // (ty, init elems, push val, idx, expected, final vec)
    const TABLE: [(&str, &str, &str, usize, &str, &str); 20] = [
        ("u8", "4u8, 7", "9u8", 1, "7u8", "vec![4u8, 7, 9u8]"),
        ("u8", "4u8, 7", "9u8", 0, "4u8", "vec![4u8, 7, 9u8]"),
        ("u8", "1u8, 2, 3", "4u8", 2, "3u8", "vec![1u8, 2, 3, 4u8]"),
        ("u8", "1u8, 2, 3", "4u8", 0, "1u8", "vec![1u8, 2, 3, 4u8]"),
        (
            "i32",
            "10i32, 20, 30",
            "40i32",
            1,
            "20i32",
            "vec![10i32, 20, 30, 40i32]",
        ),
        (
            "i32",
            "10i32, 20, 30",
            "40i32",
            2,
            "30i32",
            "vec![10i32, 20, 30, 40i32]",
        ),
        ("i32", "5i32, 6", "7i32", 0, "5i32", "vec![5i32, 6, 7i32]"),
        ("i32", "5i32, 6", "7i32", 1, "6i32", "vec![5i32, 6, 7i32]"),
        ("u16", "1u16, 2", "3u16", 1, "2u16", "vec![1u16, 2, 3u16]"),
        ("u16", "1u16, 2", "3u16", 0, "1u16", "vec![1u16, 2, 3u16]"),
        (
            "u16",
            "8u16, 9, 10",
            "11u16",
            2,
            "10u16",
            "vec![8u16, 9, 10, 11u16]",
        ),
        (
            "u16",
            "8u16, 9, 10",
            "11u16",
            0,
            "8u16",
            "vec![8u16, 9, 10, 11u16]",
        ),
        (
            "i64",
            "100i64, 200",
            "300i64",
            1,
            "200i64",
            "vec![100i64, 200, 300i64]",
        ),
        (
            "i64",
            "100i64, 200",
            "300i64",
            0,
            "100i64",
            "vec![100i64, 200, 300i64]",
        ),
        (
            "i64",
            "7i64, 8, 9",
            "10i64",
            2,
            "9i64",
            "vec![7i64, 8, 9, 10i64]",
        ),
        (
            "u32",
            "5u32, 6, 7",
            "8u32",
            1,
            "6u32",
            "vec![5u32, 6, 7, 8u32]",
        ),
        (
            "u32",
            "5u32, 6, 7",
            "8u32",
            0,
            "5u32",
            "vec![5u32, 6, 7, 8u32]",
        ),
        (
            "u32",
            "5u32, 6, 7",
            "8u32",
            2,
            "7u32",
            "vec![5u32, 6, 7, 8u32]",
        ),
        ("u8", "0u8, 1", "2u8", 1, "1u8", "vec![0u8, 1, 2u8]"),
        (
            "i32",
            "1i32, 2, 3",
            "4i32",
            1,
            "2i32",
            "vec![1i32, 2, 3, 4i32]",
        ),
    ];
    for (i, (ty, init, push, idx, expected, final_vec)) in TABLE.iter().enumerate() {
        let gt = gt_module(&format!(
            "let mut v = vec![{init}];\n        let got = super::probe(&mut v);\n        assert_eq!(got, {expected});\n        assert_eq!(v, {final_vec});"
        ));
        let seed = format!(
            "pub fn probe(v: &mut Vec<{ty}>) -> {ty} {{\n    let first = v[{idx}];\n    v.push({push});\n    first\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe(v: &mut Vec<{ty}>) -> {ty} {{\n    let first = &v[{idx}];\n    v.push({push});\n    *first\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0502_copy_{i:02}_{ty}"),
            "E0502",
            MutatorKind::AddBorrow,
            seed,
            broken,
        );
    }
}

/// `items[0].len()` read through a reference across a `push` (non-Copy
/// elements — the len must be computed through `&items[0]`).
fn e0502_string_len(out: &mut Vec<GeneratedCase>) {
    // (initial elements, idx-0 len, pushed literal) — lens MECHANICALLY
    // verified (`python3 -c 'print(len(..))'`); hand-counted literals are
    // how the first draft shipped 7 wrong expectations that only the
    // cargo-test G1 pass caught (the seeds compiled — the TESTS were
    // wrong, which check-time integrity cannot see).
    const TABLE: [(&str, usize, &str); 15] = [
        ("vec![String::from(\"abc\")]", 3, "\"new\""),
        (
            "vec![String::from(\"hello\"), String::from(\"x\")]",
            5,
            "\"new\"",
        ),
        ("vec![String::from(\"\")]", 0, "\"z\""),
        (
            "vec![String::from(\"abcd\"), String::from(\"ef\")]",
            4,
            "\"w3\"",
        ),
        ("vec![String::from(\"seven7\")]", 6, "\"new\""),
        (
            "vec![String::from(\"one\"), String::from(\"two\")]",
            3,
            "\"pushed\"",
        ),
        ("vec![String::from(\"12345\")]", 5, "\"9\""),
        (
            "vec![String::from(\"a\"), String::from(\"bb\"), String::from(\"ccc\")]",
            1,
            "\"new\"",
        ),
        ("vec![String::from(\"eleven=11\")]", 9, "\"x\""),
        ("vec![String::from(\"twelve12\")]", 8, "\"yy\""),
        ("vec![String::from(\"thirteen13\")]", 10, "\"n\""),
        ("vec![String::from(\"fourteen14\")]", 10, "\"m\""),
        ("vec![String::from(\"fifteen15\")]", 9, "\"k\""),
        ("vec![String::from(\"sixteen16\")]", 9, "\"j\""),
        ("vec![String::from(\"seventeen17\")]", 11, "\"i\""),
    ];
    for (i, (init, len, push_lit)) in TABLE.iter().enumerate() {
        let gt = gt_module(&format!(
            "let mut items = {init};\n        let got = super::probe(&mut items);\n        assert_eq!(got, {len});\n        assert_eq!(items.len(), {});",
            // initial count + 1 pushed — computed from the table shape by
            // counting `String::from` occurrences at generation time.
            init.matches("String::from").count() + 1
        ));
        let push_val = format!("String::from({push_lit})");
        let seed = format!(
            "pub fn probe(items: &mut Vec<String>) -> usize {{\n    let head = items[0].len();\n    items.push({push_val});\n    head\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe(items: &mut Vec<String>) -> usize {{\n    let head = &items[0];\n    items.push({push_val});\n    head.len()\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0502_len_{i:02}"),
            "E0502",
            MutatorKind::AddBorrow,
            seed,
            broken,
        );
    }
}

// ── E0505: move out of borrowed (contested opening — our one template) ─

fn e0505_drop_clone(out: &mut Vec<GeneratedCase>) {
    const TABLE: [(&str, &str, usize); 20] = [
        ("u8", "1u8, 2, 3", 3),
        ("u8", "9u8", 1),
        ("u8", "1u8, 1, 1, 1, 1, 1", 6),
        ("u8", "2u8, 4, 6, 8", 4),
        ("u8", "5u8, 5", 2),
        ("u8", "0u8, 0, 0", 3),
        ("u8", "3u8, 1, 4, 1, 5", 5),
        ("u8", "7u8, 7, 7, 7, 7, 7, 7", 7),
        ("u8", "8u8, 9, 10", 3),
        ("u8", "11u8", 1),
        ("i32", "1i32, 2", 2),
        ("i32", "10i32, 20, 30", 3),
        ("i32", "5i32", 1),
        ("i32", "1i32, 1, 1, 1", 4),
        ("i32", "100i32, 200", 2),
        ("u64", "1u64, 2, 3", 3),
        ("u64", "42u64", 1),
        ("u64", "10u64, 20, 30, 40", 4),
        ("u64", "7u64, 7", 2),
        ("u64", "1u64, 2, 3, 4, 5", 5),
    ];
    for (i, (ty, elems, n)) in TABLE.iter().enumerate() {
        let expected = 2 * n;
        let gt = gt_module(&format!(
            "assert_eq!(super::probe(vec![{elems}]), {expected});"
        ));
        let seed = format!(
            "pub fn probe(v: Vec<{ty}>) -> usize {{\n    let r = &v;\n    let w = v.clone();\n    r.len() + w.len()\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe(v: Vec<{ty}>) -> usize {{\n    let r = &v;\n    let w = v;\n    r.len() + w.len()\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0505_clone_{i:02}_{ty}"),
            "E0505",
            MutatorKind::DropCloneCall,
            seed,
            broken,
        );
    }
}

// ── E0505/E0382: the non-Clone saddle stratum (Plan 432 corpus expansion) ──
//
// The borrowck ladder's second rungs (`e0505-use-before-move`,
// `e0382-use-before-move`) exist so a site can SADDLE: the clone rung is
// tried first (min-delta ordering) and REVERTS on non-`Clone` types (E0599
// at equal count — no strict lexicographic decrease), then the swap rung
// keeps. These families manufacture that fail→resolve shape on codes the
// interacting corpus never covers — the tuple-diversity expansion the
// RLTL;DR teacher pass needs (riir-train plan 432's diversity tripwire:
// the 10-01 corpus was E0502-only).

/// E0505 on a deliberately non-`Clone` struct: the seed reads through the
/// borrow BEFORE the move; the broken form swaps those statements so the
/// borrow is live across the move. Clone-at-move reverts (E0599);
/// use-before-move recovers — the saddle.
fn e0505_nonclone_borrow_move(out: &mut Vec<GeneratedCase>) {
    // (field type, field value) — `Tkn` derives NOTHING (not Clone, not
    // Copy): the clone rung's `t.clone()` is E0599 here.
    const TABLE: [(&str, &str); 12] = [
        ("u8", "7u8"),
        ("u8", "13u8"),
        ("u16", "9u16"),
        ("u16", "21u16"),
        ("u32", "11u32"),
        ("u32", "35u32"),
        ("u64", "7u64"),
        ("u64", "42u64"),
        ("i16", "5i16"),
        ("i32", "19i32"),
        ("i64", "23i64"),
        ("usize", "17usize"),
    ];
    for (i, (ty, val)) in TABLE.iter().enumerate() {
        let gt = gt_module(&format!(
            "assert_eq!(super::probe(Tkn {{ v: {val} }}), 2 * {val});"
        ));
        let wrap = |pair: &str| {
            format!(
                "pub struct Tkn {{ pub v: {ty} }}\n\npub fn probe(t: Tkn) -> {ty} {{\n    {pair}\n}}\n{gt}"
            )
        };
        let seed = wrap("let r = &t;\n    let n = r.v;\n    let w = t;\n    n + w.v");
        let broken = wrap("let r = &t;\n    let w = t;\n    let n = r.v;\n    n + w.v");
        push_case(
            out,
            format!("gen_e0505_ncswap_{i:02}_{ty}"),
            "E0505",
            MutatorKind::SwapMoveUse,
            seed,
            broken,
        );
    }
}

/// E0382 on the same non-`Clone` struct: the seed reads the field BEFORE
/// the move; the broken form uses after it. Same ladder, one code over —
/// clone reverts (E0599), the statement swap recovers.
fn e0382_nonclone_use_after_move(out: &mut Vec<GeneratedCase>) {
    const TABLE: [(&str, &str); 12] = [
        ("u8", "3u8"),
        ("u8", "29u8"),
        ("u16", "15u16"),
        ("u16", "31u16"),
        ("u32", "13u32"),
        ("u32", "37u32"),
        ("u64", "11u64"),
        ("u64", "50u64"),
        ("i16", "7i16"),
        ("i32", "23i32"),
        ("i64", "31i64"),
        ("usize", "19usize"),
    ];
    for (i, (ty, val)) in TABLE.iter().enumerate() {
        let gt = gt_module(&format!(
            "assert_eq!(super::probe(Tkn {{ v: {val} }}), 2 * {val});"
        ));
        let wrap = |pair: &str| {
            format!(
                "pub struct Tkn {{ pub v: {ty} }}\n\npub fn probe(t: Tkn) -> {ty} {{\n    {pair}\n}}\n{gt}"
            )
        };
        let seed = wrap("let n = t.v;\n    let w = t;\n    n + w.v");
        let broken = wrap("let w = t;\n    let n = t.v;\n    n + w.v");
        push_case(
            out,
            format!("gen_e0382_ncswap_{i:02}_{ty}"),
            "E0382",
            MutatorKind::SwapMoveUse,
            seed,
            broken,
        );
    }
}

/// E0382 on `Clone` types — the CONTRAST stratum: the clone rung KEEPS
/// here (no saddle; the first-tried candidate resolves), which keeps the
/// ladder's ordering honest rather than tuned for saddles alone.
fn e0382_clone_use_after_move(out: &mut Vec<GeneratedCase>) {
    // (kind, init-expr, len-expr) — Vec/String are Clone.
    const TABLE: [(&str, &str, usize); 6] = [
        ("u8", "vec![1u8, 2, 3]", 3),
        ("u8", "vec![9u8]", 1),
        ("u32", "vec![10u32, 20]", 2),
        ("u32", "vec![5u32]", 1),
        ("u64", "vec![7u64, 8, 9]", 3),
        ("u64", "vec![100u64]", 1),
    ];
    for (i, (ty, init, n)) in TABLE.iter().enumerate() {
        let gt = gt_module(&format!(
            "assert_eq!(super::probe({init}), 2 * {n});"
        ));
        let wrap = |pair: &str| {
            format!("pub fn probe(v: Vec<{ty}>) -> usize {{\n    {pair}\n}}\n{gt}")
        };
        let seed = wrap("let n = v.len();\n    let w = v;\n    n + w.len()");
        let broken = wrap("let w = v;\n    let n = v.len();\n    n + w.len()");
        push_case(
            out,
            format!("gen_e0382_clone_swap_{i:02}_{ty}"),
            "E0382",
            MutatorKind::SwapMoveUse,
            seed,
            broken,
        );
    }
}

// ── E0596: borrow as mutable (contested — delegated to cargo fix) ─────

fn e0596_drop_mut(out: &mut Vec<GeneratedCase>) {
    // Vec variants ×15 + String variants ×5 (both tables up front — items
    // after statements is the confusing-form clippy class).
    const VEC_TABLE: [(&str, &str, &str, usize); 15] = [
        // (ty, initial, push, expected_len)
        ("u8", "vec![5u8]", "1u8", 2),
        ("u8", "vec![1u8, 2]", "3u8", 3),
        ("u8", "vec![]", "7u8", 1),
        ("u8", "vec![9u8]", "9u8", 2),
        ("u8", "vec![0u8, 0]", "0u8", 3),
        ("i32", "vec![10i32]", "5i32", 2),
        ("i32", "vec![]", "1i32", 1),
        ("i32", "vec![1i32, 2, 3]", "4i32", 4),
        ("u16", "vec![1u16]", "2u16", 2),
        ("u16", "vec![7u16, 8]", "9u16", 3),
        ("u32", "vec![100u32]", "200u32", 2),
        ("u32", "vec![]", "42u32", 1),
        ("u64", "vec![1u64]", "2u64", 2),
        ("u64", "vec![3u64, 4]", "5u64", 3),
        ("i16", "vec![9i16]", "8i16", 2),
    ];
    // String variants ×5
    const STR_TABLE: [(&str, &str, usize); 5] = [
        // (initial, push_str, expected_len)
        ("a", "bc", 3),
        ("x", "yz!", 4),
        ("", "hello", 5),
        ("seed", "ed", 6),
        ("q", "", 1),
    ];
    for (i, (ty, init, push, len)) in VEC_TABLE.iter().enumerate() {
        let gt = gt_module(&format!("assert_eq!(super::probe({init}), {len});"));
        let seed = format!(
            "pub fn probe(v: Vec<{ty}>) -> usize {{\n    let mut x = v;\n    x.push({push});\n    x.len()\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe(v: Vec<{ty}>) -> usize {{\n    let x = v;\n    x.push({push});\n    x.len()\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0596_mut_{i:02}_{ty}"),
            "E0596",
            MutatorKind::DropMutBinding,
            seed,
            broken,
        );
    }
    for (i, (init, tail, len)) in STR_TABLE.iter().enumerate() {
        let gt = gt_module(&format!("assert_eq!(super::probe(), {len});"));
        let seed = format!(
            "pub fn probe() -> usize {{\n    let mut s = String::from(\"{init}\");\n    s.push_str(\"{tail}\");\n    s.len()\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe() -> usize {{\n    let s = String::from(\"{init}\");\n    s.push_str(\"{tail}\");\n    s.len()\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0596_str_{i:02}"),
            "E0596",
            MutatorKind::DropMutBinding,
            seed,
            broken,
        );
    }
}

// ── E0382: use of moved value (contested — delegated) ─────────────────

fn e0382_drop_clone(out: &mut Vec<GeneratedCase>) {
    const TABLE: [(&str, &str, usize); 20] = [
        ("u8", "1u8, 2", 2),
        ("u8", "1u8, 2, 3", 3),
        ("u8", "9u8", 1),
        ("u8", "1u8, 1, 1, 1", 4),
        ("u8", "5u8, 6, 7", 3),
        ("u8", "0u8", 1),
        ("u8", "2u8, 4, 6, 8", 4),
        ("u8", "3u8, 5", 2),
        ("i32", "1i32, 2, 3", 3),
        ("i32", "10i32", 1),
        ("i32", "1i32, 1", 2),
        ("i32", "7i32, 8, 9, 10", 4),
        ("i64", "1i64, 2", 2),
        ("i64", "100i64, 200, 300", 3),
        ("u16", "1u16, 2, 3", 3),
        ("u16", "5u16", 1),
        ("u32", "1u32, 2", 2),
        ("u32", "3u32, 4, 5", 3),
        ("u64", "8u64, 9", 2),
        ("u64", "1u64, 2, 3, 4", 4),
    ];
    for (i, (ty, elems, n)) in TABLE.iter().enumerate() {
        let expected = 2 * n;
        let gt = gt_module(&format!(
            "assert_eq!(super::probe(vec![{elems}]), {expected});"
        ));
        let seed = format!(
            "pub fn probe(v: Vec<{ty}>) -> usize {{\n    let w = v.clone();\n    v.len() + w.len()\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe(v: Vec<{ty}>) -> usize {{\n    let w = v;\n    v.len() + w.len()\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0382_clone_{i:02}_{ty}"),
            "E0382",
            MutatorKind::DropCloneCall,
            seed,
            broken,
        );
    }
}

// ── E0507: cannot move out of `&` (contested — delegated) ─────────────

fn e0507_drop_clone(out: &mut Vec<GeneratedCase>) {
    // (field, ty, elems)
    const TABLE: [(&str, &str, &str); 15] = [
        ("v", "u8", "1u8, 2"),
        ("v", "u8", "9u8"),
        ("v", "i32", "1i32, 2, 3"),
        ("w", "u8", "4u8, 5, 6"),
        ("w", "i64", "100i64"),
        ("w", "u16", "1u16, 2"),
        ("data", "u8", "7u8, 8, 9, 10"),
        ("data", "i32", "5i32"),
        ("data", "u32", "1u32, 2, 3"),
        ("data", "u64", "3u64, 4"),
        ("items", "u8", "1u8, 1, 1"),
        ("items", "i16", "9i16, 8"),
        ("items", "u8", "2u8, 2, 2, 2"),
        ("buf", "u8", "0u8, 1, 0"),
        ("buf", "i32", "6i32, 7, 8"),
    ];
    for (i, (field, ty, elems)) in TABLE.iter().enumerate() {
        let struct_name = "S";
        let gt = gt_module(&format!(
            "assert_eq!(super::probe(&super::{struct_name} {{ {field}: vec![{elems}] }}), vec![{elems}]);"
        ));
        let seed = format!(
            "pub struct {struct_name} {{\n    pub {field}: Vec<{ty}>,\n}}\n\npub fn probe(s: &{struct_name}) -> Vec<{ty}> {{\n    s.{field}.clone()\n}}\n{gt}"
        );
        let broken = format!(
            "pub struct {struct_name} {{\n    pub {field}: Vec<{ty}>,\n}}\n\npub fn probe(s: &{struct_name}) -> Vec<{ty}> {{\n    s.{field}\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0507_field_{i:02}_{field}_{ty}"),
            "E0507",
            MutatorKind::DropCloneCall,
            seed,
            broken,
        );
    }
}

// ── E0373: closure may outlive captured value (contested — delegated) ─

fn e0373_drop_move(out: &mut Vec<GeneratedCase>) {
    // String captures ×10 + Vec sum captures ×15 (both tables up front).
    const STRS: [(&str, usize); 10] = [
        ("hello", 5),
        ("world", 5),
        ("move", 4),
        ("closure", 7),
        ("capture", 7),
        ("box", 3),
        ("dyn", 3),
        ("lifetime", 8),
        ("outlives", 8),
        ("probe", 5),
    ];
    const SUMS: [(&str, &str, &str, u64); 15] = [
        // (ty, elems, sum_ty_annotation, expected)
        ("u32", "1u32, 2, 3", "u32", 6),
        ("u32", "10u32, 20", "u32", 30),
        ("u32", "5u32", "u32", 5),
        ("u32", "1u32, 1, 1, 1", "u32", 4),
        ("u32", "0u32, 0", "u32", 0),
        ("i64", "1i64, 2, 3", "i64", 6),
        ("i64", "100i64, 200, 300", "i64", 600),
        ("i64", "42i64", "i64", 42),
        ("u16", "1u16, 2, 3, 4", "u16", 10),
        ("u16", "7u16, 8", "u16", 15),
        ("i32", "4i32, 5, 6", "i32", 15),
        ("i32", "9i32", "i32", 9),
        ("u8", "1u8, 2, 3, 4, 5", "u8", 15),
        ("u8", "250u8", "u8", 250),
        ("u64", "1u64, 2, 3, 4", "u64", 10),
    ];
    for (i, (lit, len)) in STRS.iter().enumerate() {
        let gt = gt_module(&format!("assert_eq!(super::probe()(), {len});"));
        let seed = format!(
            "pub fn probe() -> Box<dyn Fn() -> usize> {{\n    let s = String::from(\"{lit}\");\n    Box::new(move || s.len())\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe() -> Box<dyn Fn() -> usize> {{\n    let s = String::from(\"{lit}\");\n    Box::new(|| s.len())\n}}\n{gt}"
        );
        push_case(
            out,
            format!("gen_e0373_str_{i:02}_{lit}"),
            "E0373",
            MutatorKind::DropMoveKeyword,
            seed,
            broken,
        );
    }
    // Vec sum captures ×15
    for (i, (ty, elems, ann, sum)) in SUMS.iter().enumerate() {
        let gt = gt_module(&format!("assert_eq!(super::probe()(), {sum});"));
        let seed = format!(
            "pub fn probe() -> Box<dyn Fn() -> {ann}> {{\n    let data = vec![{elems}];\n    Box::new(move || data.iter().sum())\n}}\n{gt}"
        );
        let broken = format!(
            "pub fn probe() -> Box<dyn Fn() -> {ann}> {{\n    let data = vec![{elems}];\n    Box::new(|| data.iter().sum())\n}}\n{gt}"
        );
        let _ = ty; // elems carry their own suffixes; kept for the name
        push_case(
            out,
            format!("gen_e0373_sum_{i:02}_{ty}"),
            "E0373",
            MutatorKind::DropMoveKeyword,
            seed,
            broken,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn corpus_meets_the_n200_target() {
        let corpus = generate_corpus();
        assert!(
            corpus.len() >= 200,
            "Plan 019 T6.1 requires N>=200; generated {}",
            corpus.len()
        );
    }

    #[test]
    fn generation_is_deterministic() {
        let a = generate_corpus();
        let b = generate_corpus();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.name, y.name, "name order drifted");
            assert_eq!(x.seed, y.seed, "seed drifted for {}", x.name);
            assert_eq!(x.broken, y.broken, "broken drifted for {}", x.name);
            assert_eq!(x.expect_code, y.expect_code);
        }
    }

    #[test]
    fn names_are_unique() {
        let corpus = generate_corpus();
        let set: HashSet<&str> = corpus.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(set.len(), corpus.len(), "duplicate case names");
    }

    #[test]
    fn corpus_is_stratified_per_family() {
        let corpus = generate_corpus();
        let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for c in &corpus {
            *counts.entry(c.expect_code).or_default() += 1;
        }
        // Every family the plan probes is carried at N>=10 so per-code
        // rates cannot hinge on one or two seeds.
        for code in [
            "E0597", "E0499", "E0502", "E0505", "E0596", "E0382", "E0507", "E0373",
        ] {
            assert!(
                counts.get(code).copied().unwrap_or(0) >= 10,
                "{code} underrepresented: {counts:?}"
            );
        }
        // The uncontested head (the product claim) must be the majority of
        // the corpus — coverage numbers are reported per territory, and the
        // uncontested families are what the driver is FOR.
        let uncontested: usize = corpus
            .iter()
            .filter(|c| territory_of(c.expect_code) == Territory::Uncontested)
            .count();
        assert!(
            uncontested >= 90,
            "uncontested stratum too thin: {uncontested}"
        );
    }

    #[test]
    fn every_seed_carries_a_behavior_test() {
        for c in generate_corpus() {
            assert!(
                c.seed.contains("#[cfg(test)]")
                    && c.seed.contains("fn behavior()")
                    && c.seed.contains("assert_eq!"),
                "{} has no ground-truth test",
                c.name
            );
        }
    }

    #[test]
    fn broken_always_differs_from_seed() {
        for c in generate_corpus() {
            assert_ne!(c.seed, c.broken, "{}: mutation is a no-op", c.name);
        }
    }

    /// Braces balance in both halves — a cheap syntax sanity net that
    /// catches template typos BEFORE the (slow) cargo integrity run. The
    /// full declared-code check needs cargo and lives in the measurement
    /// example + the env-gated heavy test.
    #[test]
    fn braces_balance_in_both_halves() {
        for c in generate_corpus() {
            for (which, src) in [("seed", &c.seed), ("broken", &c.broken)] {
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
}
