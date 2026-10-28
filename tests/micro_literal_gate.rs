//! T10 (riir-dapps Plan 035) — the ×1_000_000 grep gate, riir-refine half.
//!
//! The lamport law's third rule: **`MICRO_PER_KAT` is the only µ↔KAT
//! conversion** — the bare `1_000_000` literal appears nowhere in money
//! paths. Wave 1 (riir-kat `89e31f2`) made that true at the TYPE level for
//! the economics constants; this gate keeps it true at the TEXT level for
//! this repo's non-test source: every bare million literal must either sit
//! on a line that NAMES its constant (the definition/pin of the two legal
//! millions) or live inside an allowlisted file with a recorded reason.
//!
//! The TWO-MILLIONS subtlety (the gate's founding lesson): token math
//! (`tokens / 1_000_000` for per-Mtok pricing) is a DIFFERENT million, not
//! the µKAT conversion. The token million is NAMED —
//! `riir_refine::kat_footer::TOKENS_PER_MTOK` — and every per-Mtok site
//! converts through it; the money million is
//! `riir_refine::kat_units::MICRO_PER_KAT`. Same value, different
//! dimension; the gate reds on the ANONYMOUS form of either so the next
//! one cannot arrive unconverted.
//!
//! What the scanner strips before matching (so prose and fixtures cannot
//! fake a finding, and fixtures cannot hide one):
//! - comments (`//`, `///`, `//!`, nested `/* */`) — a doc example is not
//!   a conversion;
//! - string + raw-string + char literals — the mined rule corpora embed
//!   whole source files inside strings (`entries_gpu_37`'s `ref_shape`);
//! - `#[cfg(test)]` / `#[cfg(all(test, …))]` `mod … { … }` bodies — test
//!   fixtures are µ/token values by construction, not unit decisions.
//!
//! Planted-fixture arms below prove the scanner FIRES (the bare, packed,
//! float-exponent and suffixed forms) and stays quiet on the exempt
//! shapes — the known-answer discipline; a green gate over a blind
//! scanner is the failure this file exists to prevent.
//!
//! ```bash
//! cargo test --test micro_literal_gate
//! ```

use std::path::Path;

// ── The scanner ────────────────────────────────────────────────────────────

/// One bare-million occurrence in non-exempt code text.
#[derive(Debug, PartialEq, Eq)]
struct Finding {
    line: usize,
    text: String,
}

/// A million literal in *stripped* code text: `1_000_000` (optionally with
/// an integer suffix), packed `1000000`, or the float-exponent form `1e6`
/// / `1E6` (the same million wearing a costume). `10_000_000`, `4_000_000`
/// and other magnitudes are different VALUES, not this gate's subject.
fn is_bare_million(token: &str) -> bool {
    let stripped: String = token.chars().filter(|c| *c != '_').collect();
    if stripped == "1000000" {
        return true;
    }
    // `1000000u64` / `1000000i128` / `1000000f32` … — value + suffix.
    match stripped.strip_prefix("1000000") {
        Some(rest)
            if !rest.is_empty()
                && rest.starts_with(['u', 'i', 'f'])
                && rest.chars().all(|c| c.is_ascii_alphanumeric()) =>
        {
            true
        }
        _ => stripped == "1e6" || stripped == "1E6",
    }
}

/// Strip comments, string/char literals, and `#[cfg(test)]` module bodies
/// from `src`, preserving line structure (skipped regions become spaces so
/// line numbers survive and tokens cannot join across a stripped span).
/// Length (in chars) of a test-only cfg attribute at the head of `rest`:
/// `#[cfg(test)]`, or `#[cfg(all(test, …))]` where `test` is a REQUIRED
/// conjunct (a feature-gated test module — `update_check.rs` carries one).
/// `any(test, …)` / `not(test)` are NOT test-only and stay scanned.
fn test_cfg_attr_len(rest: &[char]) -> Option<usize> {
    const BARE: &str = "#[cfg(test)]";
    const ALL: &str = "#[cfg(all(test";
    let head: String = rest.iter().take(BARE.len().max(ALL.len()) + 1).collect();
    if head.starts_with(BARE) {
        return Some(BARE.chars().count());
    }
    let after = head.strip_prefix(ALL)?;
    if !after.starts_with([',', ')']) {
        return None; // `all(tests_x, …)` — a different identifier
    }
    // Close at the attribute's own `]` (cfg predicates carry no brackets).
    rest.iter().position(|c| *c == ']').map(|p| p + 1)
}

fn strip_non_code(src: &str) -> String {
    let bytes: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;

    // `#[cfg(test)] mod NAME { … }` elision: on `#[cfg(test)]`, skip any
    // sibling attributes, require `mod NAME {`, then blank the brace-matched
    // body.
    let mut suppression_end: Option<usize> = None;

    while i < bytes.len() {
        if let Some(end) = suppression_end {
            if i < end {
                out.push(if bytes[i] == '\n' { '\n' } else { ' ' });
                i += 1;
                continue;
            }
            suppression_end = None;
        }

        if let Some(attr_len) = test_cfg_attr_len(&bytes[i..]) {
            let mut j = i + attr_len;
            loop {
                while j < bytes.len() && bytes[j].is_whitespace() {
                    j += 1;
                }
                if bytes[j..].starts_with(&['#', '[']) {
                    while j < bytes.len() && bytes[j] != ']' {
                        j += 1;
                    }
                    j += 1;
                    continue;
                }
                break;
            }
            let ahead: String = bytes[j..].iter().take(32).collect();
            if let Some(after_mod) = ahead.strip_prefix("mod ") {
                let name_len = after_mod
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(0);
                let tail = after_mod[name_len..].trim_start();
                if tail.starts_with('{') {
                    let mut depth = 0usize;
                    let mut k = j + "mod ".len() + name_len;
                    while k < bytes.len() {
                        match bytes[k] {
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            '"' => {
                                k += 1;
                                while k < bytes.len() {
                                    if bytes[k] == '\\' {
                                        k += 2;
                                        continue;
                                    }
                                    if bytes[k] == '"' {
                                        break;
                                    }
                                    k += 1;
                                }
                            }
                            _ => {}
                        }
                        k += 1;
                    }
                    suppression_end = Some(k + 1);
                    continue;
                }
            }
        }

        match bytes[i] {
            '/' if bytes.get(i + 1) == Some(&'/') => {
                while i < bytes.len() && bytes[i] != '\n' {
                    out.push(' ');
                    i += 1;
                }
            }
            '/' if bytes.get(i + 1) == Some(&'*') => {
                let mut depth = 1usize;
                out.push_str("  ");
                i += 2;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == '/' && bytes.get(i + 1) == Some(&'*') {
                        depth += 1;
                        out.push_str("  ");
                        i += 2;
                    } else if bytes[i] == '*' && bytes.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        out.push_str("  ");
                        i += 2;
                    } else {
                        out.push(if bytes[i] == '\n' { '\n' } else { ' ' });
                        i += 1;
                    }
                }
            }
            '"' => {
                out.push(' ');
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == '\\' {
                        out.push_str("  ");
                        i += 2;
                        continue;
                    }
                    let done = bytes[i] == '"';
                    out.push(if bytes[i] == '\n' { '\n' } else { ' ' });
                    i += 1;
                    if done {
                        break;
                    }
                }
            }
            'r' if bytes.get(i + 1) == Some(&'"') || bytes.get(i + 1) == Some(&'#') => {
                // Raw string: count the hashes.
                let mut hashes = 0usize;
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] == '#' {
                    hashes += 1;
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == '"' {
                    out.push_str(&" ".repeat(j - i + 1));
                    i = j + 1;
                    let mut closer = String::from("\"");
                    closer.extend(std::iter::repeat_n('#', hashes));
                    while i < bytes.len() {
                        if bytes[i] == '\n' {
                            out.push('\n');
                            i += 1;
                            continue;
                        }
                        let cl: Vec<char> = closer.chars().collect();
                        if bytes[i..].starts_with(&cl[..]) {
                            out.push_str(&" ".repeat(hashes + 1));
                            i += hashes + 1;
                            break;
                        }
                        out.push(' ');
                        i += 1;
                    }
                } else {
                    // Not a raw string — an identifier starting with `r`.
                    out.push('r');
                    i += 1;
                }
            }
            '\'' => {
                // Char literal vs lifetime: a char is `'x'` or `'\<esc>'`.
                let is_char = match bytes.get(i + 1) {
                    Some('\\') => true,
                    Some(&c) => bytes.get(i + 2) == Some(&'\'') && c != '\'',
                    None => false,
                };
                if is_char {
                    out.push_str("   ");
                    i += 2;
                    while i < bytes.len() {
                        let done = bytes[i] == '\'';
                        out.push(if bytes[i] == '\n' { '\n' } else { ' ' });
                        i += 1;
                        if done {
                            break;
                        }
                    }
                } else {
                    // Lifetime (or a stray quote) — ordinary code text.
                    out.push('\'');
                    i += 1;
                }
            }
            '\n' => {
                out.push('\n');
                i += 1;
            }
            _ => {
                out.push(bytes[i]);
                i += 1;
            }
        }
    }
    out
}

/// Scan stripped code text for bare-million literals.
fn scan_text(stripped: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (idx, line) in stripped.lines().enumerate() {
        let mut run = String::new();
        for ch in line.chars().chain(std::iter::once(' ')) {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                run.push(ch);
            } else {
                if is_bare_million(&run) {
                    findings.push(Finding {
                        line: idx + 1,
                        text: line.trim().to_string(),
                    });
                }
                run.clear();
            }
        }
    }
    findings.shrink_to_fit();
    findings
}

/// A literal ON a line that names its constant is that constant's
/// definition or pin — the gate bans the ANONYMOUS million, not the named
/// one.
fn line_names_its_constant(line: &str) -> bool {
    line.contains("MICRO_PER_KAT") || line.contains("TOKENS_PER_MTOK")
}

/// The per-line classification the gate applies: a stripped line is clean
/// when it holds no bare million, or when the literal it holds sits beside
/// its constant's NAME in code (the definition/pin exemption).
fn line_is_clean(stripped_line: &str) -> bool {
    scan_text(stripped_line).is_empty() || line_names_its_constant(stripped_line)
}

/// Allowlisted paths: (path substring, reason). Every real finding must sit
/// inside one of these files, or the gate reds. Membership with a reason —
/// never a count (a count is green on a swap).
const ALLOWED_PATHS: &[(&str, &str)] = &[
    (
        "src/domains/rust_perf/corpus/entries_",
        "mined perf-rule corpus: benchmark input-size ladders, not money",
    ),
    (
        "src/domains/rust_perf/bench.rs",
        "1_000_000 ns = the 1 ms bench-sample target — a TIME, not money",
    ),
    (
        "src/hotpath_report.rs",
        "ms → ns duration parsing (hotpath profile units) — a TIME, not money",
    ),
    (
        "src/validators/interp.rs",
        "DEFAULT_FUEL = a 1M interpreter STEP budget — a count of steps, not money",
    ),
    (
        "src/domains/kernel_opt/corpus/tests.rs",
        "kernel-corpus probe fixtures (f32 magnitudes incl. 1e6), not money",
    ),
];

fn allowed_for(path: &str) -> Option<&'static str> {
    ALLOWED_PATHS
        .iter()
        .find(|(prefix, _)| path.contains(prefix))
        .map(|(_, reason)| *reason)
}

fn collect_rs(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every bare-million finding over the real tree, classified only by the
/// structural exemptions (comments/strings/tests) — NOT by the allowlist.
fn scan_src_findings() -> (usize, Vec<(String, Finding)>) {
    let mut files = Vec::new();
    collect_rs(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files.sort();

    let mut all = Vec::with_capacity(files.len());
    for path in &files {
        let Ok(src) = std::fs::read_to_string(path) else {
            continue;
        };
        let stripped = strip_non_code(&src);
        let rel = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        // The allowlist keys are '/'-spelled; a Windows rel path carries '\' —
        // canonicalize so the gate classifies identically on every host (the
        // gate shipped macOS-only and this line is its first Windows run).
        let rel = rel.replace('\\', "/");
        for finding in scan_text(&stripped) {
            if !line_names_its_constant(&finding.text) {
                all.push((rel.clone(), finding));
            }
        }
    }
    (files.len(), all)
}

// ── The verdict ────────────────────────────────────────────────────────────

#[test]
fn no_anonymous_million_literal_in_non_test_source() {
    let (n_files, all) = scan_src_findings();
    // Blindness floors: a walk that finds no files, or a scanner that finds
    // nothing at all, is not a pass — the corpus allowlist is expected to
    // hold real findings, and their absence means the walk or lexer broke.
    assert!(n_files >= 150, "walk regressed: only {n_files} src files");
    assert!(
        all.len() >= 10,
        "scanner found only {} million literal(s) over {n_files} files — the lexer or walk went blind",
        all.len()
    );
    let unexpected: Vec<_> = all
        .iter()
        .filter(|(p, _)| allowed_for(p).is_none())
        .collect();
    assert!(
        unexpected.is_empty(),
        "anonymous 1_000_000/1000000/1e6 literal(s) in non-test source — \
         convert to MICRO_PER_KAT (µ↔KAT) or TOKENS_PER_MTOK (per-Mtok \
         pricing), or add an honest allowlist row with a reason:\n{}",
        unexpected
            .iter()
            .map(|(p, f)| format!("  {p}:{}  {}", f.line, f.text))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn allowlist_rows_stay_fresh_in_both_directions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&root, &mut files);
    let (_, all) = scan_src_findings();
    for (prefix, reason) in ALLOWED_PATHS {
        assert!(!reason.is_empty(), "allowlist row {prefix} lost its reason");
        // Stale-row direction: the row must still name a real file (the same
        // '/' canonicalization as the scan — a Windows walk carries '\\').
        assert!(
            files
                .iter()
                .any(|p| p.to_string_lossy().replace('\\', "/").contains(prefix)),
            "stale allowlist row: no file matches {prefix}"
        );
    }
    // Working-row direction: the corpus rows must still be doing real work
    // (carrying findings) — an entry whose findings vanished is a row to
    // delete, not a pass to sit on.
    for prefix in [
        "src/domains/rust_perf/corpus/entries_",
        "src/domains/kernel_opt/corpus/tests.rs",
    ] {
        assert!(
            all.iter().any(|(p, _)| p.contains(prefix)),
            "allowlist row {prefix} no longer carries findings — delete the row"
        );
    }
}

// ── Planted-fixture arms (the scanner's known-answer discipline) ───────────

#[test]
fn planted_bare_million_fires() {
    let src = "fn f() {\n    let x: u64 = 1_000_000;\n}\n";
    let findings = scan_text(&strip_non_code(src));
    assert_eq!(
        findings.len(),
        1,
        "the bare literal must fire: {findings:?}"
    );
    assert_eq!(findings[0].line, 2);
}

#[test]
fn planted_packed_and_exponent_and_suffixed_forms_fire() {
    let src = "let a = 1000000;\nlet b = t as f64 / 1e6;\nlet c = t as f64 / 1E6;\nlet d = 1_000_000u64;\n";
    let findings = scan_text(&strip_non_code(src));
    assert_eq!(
        findings.len(),
        4,
        "all four million costumes must fire: {findings:?}"
    );
}

#[test]
fn other_magnitudes_are_not_this_gates_subject() {
    let src = "let a = 10_000_000;\nlet b = 100_000;\nlet c = 4_000_000;\nlet d = 100_000_000;\n";
    assert!(
        scan_text(&strip_non_code(src)).is_empty(),
        "different VALUES pass — the gate pins the million, not every large number"
    );
}

#[test]
fn comments_and_strings_and_raw_strings_shield_their_text() {
    let src = concat!(
        "// let x = 1_000_000;\n",
        "/// µ example: `1_000_000` → 1 KAT\n",
        "let s = \"1_000_000\";\n",
        "let s = \"1e6 too\";\n",
        "let raw = r#\"1_000_000 in a raw string\"#;\n",
        "let ch = '6';\n",
        "fn g<'a>(x: &'a str) -> &'a str { x }\n",
    );
    assert!(
        scan_text(&strip_non_code(src)).is_empty(),
        "prose, fixtures and lifetimes must not fake findings"
    );
}

#[test]
fn cfg_test_module_bodies_are_elided_and_the_scan_resumes_after() {
    let src = concat!(
        "#[cfg(test)]\n",
        "mod tests {\n",
        "    const FIXTURE: u64 = 1_000_000;\n",
        "}\n",
        "\n",
        "fn live() -> u64 {\n",
        "    1_000_000\n",
        "}\n",
    );
    let findings = scan_text(&strip_non_code(src));
    assert_eq!(
        findings.len(),
        1,
        "the test body is elided, live code is not: {findings:?}"
    );
    assert_eq!(findings[0].line, 7, "line numbers survive the elision");
}

#[test]
fn attributes_may_sit_between_cfg_test_and_the_mod() {
    let src = concat!(
        "#[cfg(test)]\n",
        "#[allow(dead_code)]\n",
        "mod fixtures {\n",
        "    static N: u64 = 1_000_000;\n",
        "}\n",
        "fn live() -> u64 { 1_000_000 }\n",
    );
    let findings = scan_text(&strip_non_code(src));
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].line, 6);
}

#[test]
fn feature_gated_test_modules_are_elided_but_any_and_not_are_scanned() {
    // `all(test, …)`: test is a REQUIRED conjunct — test-only, elided.
    let gated = concat!(
        "#[cfg(all(test, feature = \"update_check\"))]\n",
        "mod tests {\n",
        "    const AT: u64 = 1_000_000;\n",
        "}\n",
        "fn live() -> u64 { 1_000_000 }\n",
    );
    let findings = scan_text(&strip_non_code(gated));
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].line, 5);
    // `any(test, …)`, `not(test)` and a look-alike ident compile into
    // non-test builds — every body stays scanned.
    for attr in [
        "#[cfg(any(test, feature = \"x\"))]",
        "#[cfg(not(test))]",
        "#[cfg(all(tests_x, feature = \"x\"))]",
    ] {
        let src = format!("{attr}\nmod m {{\n    const N: u64 = 1_000_000;\n}}\n");
        assert_eq!(scan_text(&strip_non_code(&src)).len(), 1, "{attr}");
    }
}

#[test]
fn a_named_constants_line_is_exempt_but_not_by_comment() {
    // The definition line names its constant — exempt.
    let def = "pub const TOKENS_PER_MTOK: u64 = 1_000_000;\n";
    assert!(line_is_clean(def.trim_end()));
    let pin = "assert_eq!(MICRO_PER_KAT, 1_000_000);\n";
    assert!(line_is_clean(pin.trim_end()));
    // …but a trailing COMMENT naming the constant does not save an
    // anonymous literal (comments are stripped; the name must be IN code).
    let sneaky = "let x = 1_000_000; // TOKENS_PER_MTOK\n";
    assert!(!line_is_clean(strip_non_code(sneaky).trim_end()));
    // And a code line naming NEITHER constant never exempts itself.
    let anonymous = "let x = 1_000_000;\n";
    assert!(!line_is_clean(anonymous.trim_end()));
}

#[test]
fn real_tree_micro_conversion_sites_stay_silent() {
    // The µ-conversion module the gate governs, read straight from disk —
    // a regression back to a bare literal there cannot hide behind an edit
    // to the fixture arms above.
    let footer = include_str!("../src/kat_footer.rs");
    let stripped = strip_non_code(footer);
    for (idx, line) in stripped.lines().enumerate() {
        if line_names_its_constant(line) {
            continue;
        }
        let findings = scan_text(line);
        assert!(
            findings.is_empty(),
            "kat_footer.rs:{} regressed to an anonymous million: {line}",
            idx + 1
        );
    }
}
