//! Generation-4 composite measurement for the score bench.
//!
//! A generation-4 case composes `k` generated error members into ONE
//! `lib.rs` — each wrapped as `mod gen_{i} { ... }` — and measures the
//! `--fix-compile` driver against the whole composite: the initial check is
//! a label fence (the coded-diagnostic multiset must equal the expected
//! codes), then the write-mode driver runs, and each member is scored
//! independently by whether its expected error code still attributes to
//! that member's region at the final check (per-member code-reduction
//! scoring).
//!
//! This module is deliberately decoupled from the corpus generator: it
//! takes composed source text + expected codes, nothing else. Textual seed
//! comparison is deliberately NOT used — some playbook fixes are textually
//! diverged from the seed (compiling, different bytes), and a byte diff
//! would score those correct repairs as failures.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::diag::parse_error_diags;
use super::driver::CompileDriver;
use super::playbooks::Playbooks;
use crate::compile_oracle::CheckRunner;

/// One measured generation-4 composite.
#[derive(Debug, Clone)]
pub struct CompositeOutcome {
    /// The case id the measurement was filed under (unsanitized).
    pub spec_id: String,
    /// Member count the case declared (`expect_codes.len()`).
    pub k: usize,
    /// cargo ran at all (false: cargo missing / check errored at spawn).
    pub ran: bool,
    /// Initial check produced exactly the expected code SET with
    /// `count == k`.
    pub label_ok: bool,
    /// Total member regions the case declared.
    pub members_total: usize,
    /// Members whose expected code is absent from their region at the
    /// final check.
    pub members_reduced: usize,
    /// Non-initial coded errors attributed inside member regions (plus
    /// unattributed non-initial codes — conservative).
    pub created_total: usize,
    /// Final check FAILED with ZERO coded diagnostics — the file no longer
    /// parses; never count that as clean.
    pub parse_unsafe: bool,
    /// Driver-applied + kept fix count (`report.applied.len()`).
    pub edits: usize,
    /// Coded errors at the final check whose primary line fell in NO member
    /// region (reported, not asserted).
    pub unattributed: usize,
    /// `(code, primary line)` multiset from the initial check, diagnostic
    /// order. Line `0` = the diagnostic carried no primary span (kept so
    /// the multiset count stays exact).
    pub initial_codes: Vec<(String, usize)>,
    /// Same, final check.
    pub final_codes: Vec<(String, usize)>,
    /// Per-code unreduced member counts (BTreeMap-stable order).
    pub persist: Vec<(String, usize)>,
    /// Per-member reduced flags, aligned with the caller's `expect_codes`
    /// order (member i ↔ region i). The bench's per-territory split reads
    /// this — territory is a per-member property and a composite can mix.
    pub member_reduced: Vec<bool>,
}

/// Extract the `mod gen_{index} { ... }` regions of a composed source.
///
/// Returns `(module_index, line_start, line_end)` — 1-based INCLUSIVE line
/// numbers, sorted by module index. A header is a line whose trimmed form
/// starts with `mod gen_` and carries the (decimal) index before the next
/// whitespace or `{`. Region bodies are brace-matched line by line; the
/// composition contract guarantees member code never carries a brace inside
/// a string literal, so a plain character count is exact there. If the count
/// never closes (corrupt source), the region falls back to ending at the
/// LAST line consisting of a single `}`.
///
/// The driver's kept edits change member bodies but not module boundaries,
/// so callers recompute regions on the final source with this same
/// function.
#[must_use]
pub fn module_regions(source: &str) -> Vec<(usize, usize, usize)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut regions: Vec<(usize, usize, usize)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let Some(idx) = parse_mod_index(lines[i]) else {
            i += 1;
            continue;
        };
        let end = region_end(&lines, i).unwrap_or_else(|| {
            lines[i..]
                .iter()
                .rposition(|l| l.trim() == "}")
                .map_or(lines.len(), |off| i + off + 1)
        });
        regions.push((idx, i + 1, end));
        i = end;
    }
    regions.sort_unstable_by_key(|(idx, _, _)| *idx);
    regions.shrink_to_fit();
    regions
}

/// Scaffold `composed_source` under `work_root/<sanitized spec_id>/` as a
/// standalone `[workspace]` lib crate (edition 2021 — the
/// `tests/fix_compile.rs` scaffold pattern), run ONE `cargo check`, and
/// parse the coded diagnostics.
///
/// Returns the `(code, primary_line)` pairs in diagnostic order; line `0`
/// marks a diagnostic with no primary span. Returns `None` if the check
/// could not run. Does NOT clean up the scaffold — the caller owns the
/// work root.
#[must_use]
pub fn check_composite_labels(
    spec_id: &str,
    composed_source: &str,
    work_root: &Path,
) -> Option<Vec<(String, usize)>> {
    let dir = scaffold_dir(spec_id, composed_source, work_root).ok()?;
    let outcome = CheckRunner::isolated().run_check(&dir).ok()?;
    Some(coded_lines(&outcome.json))
}

/// Full generation-4 bench measurement: scaffold → initial check (label
/// fence) → [`CompileDriver::isolated`] write-mode loop with
/// [`Playbooks::production`] → final check → per-member region attribution.
///
/// `expect_codes[i]` is member i's expected error code, in the composite's
/// `mod gen_{i}` order (module index i ↔ member i — the composition
/// contract). On any pre-measurement failure the outcome degrades
/// honestly: `ran = false` before the first check completes; `label_ok =
/// false` skips the driver entirely (measuring an invalid composition is
/// meaningless — final = initial, every member unreduced).
#[must_use]
pub fn measure_composite(
    spec_id: &str,
    expect_codes: &[&str],
    composed_source: &str,
    work_root: &Path,
) -> CompositeOutcome {
    let k = expect_codes.len();
    let mut out = CompositeOutcome {
        spec_id: spec_id.to_owned(),
        k,
        ran: false,
        label_ok: false,
        members_total: k,
        members_reduced: 0,
        created_total: 0,
        parse_unsafe: false,
        edits: 0,
        unattributed: 0,
        initial_codes: Vec::new(),
        final_codes: Vec::new(),
        persist: full_persist(expect_codes),
        member_reduced: vec![false; k],
    };

    // Scaffold; a write failure means nothing ran.
    let Ok(dir) = scaffold_dir(spec_id, composed_source, work_root) else {
        return out;
    };
    let lib_rs = dir.join("src").join("lib.rs");

    // Initial check = the label fence.
    let Ok(initial) = CheckRunner::isolated().run_check(&dir) else {
        return out;
    };
    out.ran = true;
    out.initial_codes = coded_lines(&initial.json);
    out.label_ok = labels_match(expect_codes, &out.initial_codes);
    if !out.label_ok {
        // Skip the driver: final = initial, all members unreduced.
        out.final_codes.clone_from(&out.initial_codes);
        return out;
    }

    // Write-mode loop; disk holds exactly the kept fixes on return.
    let report = CompileDriver::isolated().run(
        std::slice::from_ref(&lib_rs),
        &Playbooks::production(),
        true,
    );
    out.edits = report.applied.len();

    // Final state: re-read what the driver left on disk, recheck.
    let final_source =
        std::fs::read_to_string(&lib_rs).unwrap_or_else(|_| composed_source.to_owned());
    let (final_codes, final_exit_ok) = match CheckRunner::isolated().run_check(&dir) {
        Ok(o) => (coded_lines(&o.json), o.exit_ok),
        // Spawn failure at the final check: degrade to the initial state
        // rather than invent codes; `exit_ok = true` keeps `parse_unsafe`
        // false — we do not KNOW the file stopped parsing.
        Err(_) => (out.initial_codes.clone(), true),
    };
    out.final_codes = final_codes;
    out.parse_unsafe = !final_exit_ok && out.final_codes.is_empty();
    finalize_attribution(&mut out, expect_codes, &final_source);
    out
}

/// The header predicate: trimmed line starts with `mod gen_`, carries the
/// decimal index before the next whitespace or `{`, and opens a body.
fn parse_mod_index(line: &str) -> Option<usize> {
    let rest = line.trim().strip_prefix("mod gen_")?;
    if !rest.contains('{') {
        return None;
    }
    let digits_end = rest.find(|c: char| c.is_whitespace() || c == '{')?;
    rest[..digits_end].parse::<usize>().ok()
}

/// 1-based inclusive end line of the region whose header sits at 0-based
/// `start`, by plain brace count; `None` = never closed (lone-`}` fallback).
fn region_end(lines: &[&str], start: usize) -> Option<usize> {
    let mut open: usize = 0;
    let mut close: usize = 0;
    for (offset, line) in lines.iter().enumerate().skip(start) {
        open += line.matches('{').count();
        close += line.matches('}').count();
        if close >= open {
            return Some(offset + 1);
        }
    }
    None
}

/// Multiset label fence: every expected code's count must match the
/// observed count, and the totals must agree (line numbers do not matter
/// here — only codes and their multiplicity).
fn labels_match(expected: &[&str], observed: &[(String, usize)]) -> bool {
    if observed.len() != expected.len() {
        return false;
    }
    let mut want: Vec<&str> = expected.to_vec();
    want.sort_unstable();
    let mut got: Vec<&str> = observed.iter().map(|(code, _)| code.as_str()).collect();
    got.sort_unstable();
    want == got
}

/// Attribute each `(code, line)` diagnostic to the region containing its
/// primary line (bounds inclusive). Returns the owning module index per
/// diagnostic, `None` when the line is absent or outside every region.
fn attribute(
    regions: &[(usize, usize, usize)],
    diags: &[(String, Option<usize>)],
) -> Vec<Option<usize>> {
    diags
        .iter()
        .map(|(_, line)| {
            line.and_then(|l| {
                regions
                    .iter()
                    .find(|(_, start, end)| *start <= l && l <= *end)
                    .map(|(idx, _, _)| *idx)
            })
        })
        .collect()
}

/// Fill the per-member scoring fields from the final code set + regions.
///
/// Member i is reduced iff NO diagnostic with `code == expect_codes[i]`
/// attributes to region i. `created_total` counts attributed diagnostics
/// whose code differs from their region's expectation, plus every
/// unattributed diagnostic (conservative: a fix introduced it somewhere).
/// A `parse_unsafe` final state has an empty code set, so both loops
/// naturally report all members unreduced with the persist set full.
fn finalize_attribution(out: &mut CompositeOutcome, expect_codes: &[&str], final_source: &str) {
    let regions = module_regions(final_source);
    let diags: Vec<(String, Option<usize>)> = out
        .final_codes
        .iter()
        .map(|(code, line)| (code.clone(), nonzero(*line)))
        .collect();
    let owners = attribute(&regions, &diags);

    out.unattributed = owners.iter().filter(|o| o.is_none()).count();
    for (owner, (code, _)) in owners.iter().zip(&out.final_codes) {
        let created = match owner {
            Some(idx) => expect_codes.get(*idx).copied() != Some(code.as_str()),
            None => true,
        };
        if created {
            out.created_total += 1;
        }
    }

    let mut persist: BTreeMap<&str, usize> = BTreeMap::new();
    out.member_reduced = vec![false; expect_codes.len()];
    for (i, expected) in expect_codes.iter().enumerate() {
        // Reduced iff NO final diagnostic of the member's expected code
        // attributes to its region — an error-free final check reduces every
        // member (the vacuous-`any` trap: an empty final code set must read
        // as all-fixed, not all-unproven).
        let still_present = owners
            .iter()
            .zip(&out.final_codes)
            .any(|(owner, (code, _))| *owner == Some(i) && code == expected);
        if still_present {
            *persist.entry(expected).or_insert(0) += 1;
        } else {
            out.members_reduced += 1;
            out.member_reduced[i] = true;
        }
    }
    out.persist = persist
        .into_iter()
        .map(|(code, n)| (code.to_owned(), n))
        .collect();
}

/// `Some(line)` for a real 1-based line; `None` for the line-0 sentinel.
fn nonzero(line: usize) -> Option<usize> {
    (line > 0).then_some(line)
}

/// Coded diagnostics of a check's JSON, as `(code, primary line)`; line 0
/// = no primary span (the count stays exact for the label fence).
fn coded_lines(json: &str) -> Vec<(String, usize)> {
    dedupe_pairs(
        parse_error_diags(json)
            .into_iter()
            .map(|d| (d.code, d.primary.map_or(0, |(_, line)| line)))
            .collect(),
    )
}

/// Collapse exact `(code, line)` duplicates, first occurrence wins.
///
/// `cargo check --all-targets` compiles the lib TWICE (normal + the `--test`
/// harness profile), so every error is reported twice — the same artifact
/// the driver itself handles with `diag::dedupe_by_site`. Within one member
/// region a same-(code, line) pair is the same logical error; distinct
/// members sit at distinct lines, so the composite multiset is preserved.
fn dedupe_pairs(pairs: Vec<(String, usize)>) -> Vec<(String, usize)> {
    let mut seen: std::collections::HashSet<(String, usize)> = std::collections::HashSet::new();
    pairs
        .into_iter()
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

/// The all-unreduced persist set: per-code member counts over the full
/// expectation (BTreeMap-stable order).
fn full_persist(expect_codes: &[&str]) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for code in expect_codes {
        *counts.entry(code).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .map(|(code, n)| (code.to_owned(), n))
        .collect()
}

/// Directory-safe form of a spec id: keep alphanumeric, `-`, `_`, `+`;
/// every other char becomes `_`.
fn sanitize_spec_id(spec_id: &str) -> String {
    spec_id
        .chars()
        .map(|c| match c {
            c if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '+') => c,
            _ => '_',
        })
        .collect()
}

/// Valid crate name derived from the sanitized spec id: `+` → `_`,
/// lowercased, ≤ 60 chars, not digit-leading, no trailing separators.
fn package_name(spec_id: &str) -> String {
    let mut name: String = sanitize_spec_id(spec_id)
        .chars()
        .map(|c| match c {
            '+' => '_',
            other => other.to_ascii_lowercase(),
        })
        .collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, 'g');
    }
    name.truncate(60);
    while name.ends_with(['-', '_']) {
        name.pop();
    }
    if name.is_empty() {
        name.push_str("gen");
    }
    name
}

/// Write the standalone lib-crate scaffold; both files are overwritten on
/// re-run, and no cleanup happens here (the caller owns the work root).
fn scaffold_dir(
    spec_id: &str,
    composed_source: &str,
    work_root: &Path,
) -> std::io::Result<PathBuf> {
    let dir = work_root.join(sanitize_spec_id(spec_id));
    std::fs::create_dir_all(dir.join("src"))?;
    let pkg = package_name(spec_id);
    std::fs::write(
        dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{pkg}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[workspace]\n"
        ),
    )?;
    std::fs::write(dir.join("src").join("lib.rs"), composed_source)?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-composed 3-module source; boundary lines verified against the
    /// explicit line numbering in the comments.
    #[test]
    fn module_regions_three_modules_boundaries() {
        let lines: Vec<&str> = vec![
            "pub fn probe() -> usize {",           //  1 — outside every region
            "    let v = Vec::new();",             //  2
            "    v.push(1);",                      //  3
            "    v.len()",                         //  4
            "}",                                   //  5
            "",                                    //  6
            "mod gen_0 {",                         //  7
            "    pub fn probe() -> usize {",       //  8
            "        let v = Vec::new();",         //  9
            "        v.push(1);",                  // 10
            "        v.len()",                     // 11
            "    }",                               // 12
            "}",                                   // 13
            "",                                    // 14
            "mod gen_1 {",                         // 15
            "    pub fn snap(s: &str) -> usize {", // 16
            "        let t = s.to_string();",      // 17
            "        t.len()",                     // 18
            "    }",                               // 19
            "}",                                   // 20
            "",                                    // 21
            "mod gen_2 {",                         // 22
            "    pub fn keep(v: Vec<u8>) -> u8 {", // 23
            "        let r = &v;",                 // 24
            "        r[0]",                        // 25
            "    }",                               // 26
            "}",                                   // 27
        ];
        let src = lines.join("\n");
        assert_eq!(
            module_regions(&src),
            vec![(0, 7, 13), (1, 15, 20), (2, 22, 27)]
        );
        // Content spot check: region 0 starts at its own header line.
        let all: Vec<&str> = src.lines().collect();
        assert_eq!(all[8], "        let v = Vec::new();");
    }

    #[test]
    fn module_regions_zero_modules() {
        let src =
            "pub fn probe() -> usize {\n    let v = Vec::new();\n    v.push(1);\n    v.len()\n}\n";
        assert!(module_regions(src).is_empty());
    }

    #[test]
    fn module_regions_single_and_oneline_modules() {
        // A one-line module self-closes on its header line; the index may
        // sit flush against the brace.
        let src = "mod gen_5 { pub fn f() -> u8 { 1 } }\n\nmod gen_6 {\n    pub fn g() -> u8 {\n        2\n    }\n}\n";
        assert_eq!(module_regions(src), vec![(5, 1, 1), (6, 3, 7)]);
    }

    #[test]
    fn module_regions_unbalanced_falls_back_to_last_lone_close() {
        // Never closes by brace count → end at the last lone-`}` line.
        let src = "mod gen_0 {\n    pub fn f() -> u8 {\n        1\n    }\n";
        assert_eq!(module_regions(src), vec![(0, 1, 4)]);
    }

    #[test]
    fn labels_match_multiset_semantics() {
        let observed = vec![
            ("E0596".to_owned(), 3),
            ("E0502".to_owned(), 7),
            ("E0596".to_owned(), 12),
        ];
        assert!(labels_match(&["E0596", "E0502", "E0596"], &observed));
        // Count mismatch at equal length: two E0502 expected, one observed.
        assert!(!labels_match(&["E0596", "E0502", "E0502"], &observed));
        // Length mismatch.
        assert!(!labels_match(&["E0596"], &observed));
        assert!(labels_match(&[], &[]));
    }

    #[test]
    fn attribute_maps_lines_to_regions_inclusive_bounds() {
        let regions = vec![(0, 7, 13), (1, 15, 20)];
        let diags = vec![
            ("E0596".to_owned(), Some(9)),  // inside gen_0
            ("E0502".to_owned(), Some(13)), // boundary: region end inclusive
            ("E0596".to_owned(), Some(15)), // boundary: region start
            ("E0382".to_owned(), Some(1)),  // outside every region
            ("E0382".to_owned(), None),     // no primary span
        ];
        assert_eq!(
            attribute(&regions, &diags),
            vec![Some(0), Some(0), Some(1), None, None]
        );
    }

    #[test]
    fn sanitizer_keeps_only_path_safe_chars() {
        assert_eq!(sanitize_spec_id("gen4/alpha+beta v1"), "gen4_alpha+beta_v1");
        assert_eq!(sanitize_spec_id("ok-id_2+3"), "ok-id_2+3");
        assert_eq!(sanitize_spec_id("a/b\\c:d"), "a_b_c_d");
    }

    #[test]
    fn package_name_is_a_valid_crate_name() {
        assert_eq!(package_name("Gen4/Alpha+Beta"), "gen4_alpha_beta");
        // Digit-leading names get a `g` prefix.
        assert_eq!(package_name("9lives"), "g9lives");
        let name = package_name(&"x-".repeat(80));
        assert!(name.len() <= 60);
        assert!(!name.ends_with(['-', '_']));
        assert!(!name.is_empty());
    }
}
