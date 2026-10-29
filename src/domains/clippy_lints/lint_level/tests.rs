use super::*;
use crate::domains::clippy_lints::proposers::{FIXABLE_LINTS, VERIFY_LINTS};

fn fix_set() -> Vec<&'static str> {
    let mut v: Vec<&str> = FIXABLE_LINTS.iter().chain(VERIFY_LINTS).copied().collect();
    v.sort_unstable();
    v.dedup();
    v
}

fn levels(ds: &[(&str, Level)]) -> LintLevels {
    LintLevels {
        directives: ds.iter().map(|(n, l)| directive(n, *l)).collect(),
    }
}

/// A per-process scratch crate (the shared-temp-path rule: a fixed name is
/// truncated by any concurrent process running the same test).
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lint_level_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

const PKG: &str = "[package]\nname = \"x\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

// ── T1: the group table ─────────────────────────────────────────────────

#[test]
fn table_is_sorted_and_unique() {
    // `default_group` binary-searches — an unsorted row is a silent miss.
    for w in LINT_GROUPS.windows(2) {
        assert!(
            w[0].0 < w[1].0,
            "LINT_GROUPS not strictly sorted at {:?} / {:?}",
            w[0],
            w[1]
        );
    }
}

#[test]
fn table_covers_exactly_the_fix_set() {
    let set = fix_set();
    for lint in &set {
        assert!(
            default_group(lint).is_some(),
            "{lint} is in FIXABLE_LINTS/VERIFY_LINTS with no LINT_GROUPS row — derive its group \
             from `clippy-driver -W help` at the pinned toolchain"
        );
    }
    for (lint, _) in LINT_GROUPS {
        assert!(
            set.contains(lint),
            "LINT_GROUPS row {lint} is not in the fix set (dead row)"
        );
    }
}

#[test]
fn allow_by_default_blast_radius_is_pinned_by_membership() {
    // Measured against clippy 0.1.98 (Issue 135): 15 FIXABLE + 5 VERIFY.
    // The issue's corpus-derived count said 13 and named two wrong lints —
    // a set, not a count, is what this pins.
    let mut got: Vec<&str> = fix_set()
        .into_iter()
        .filter(|l| is_allow_by_default(l))
        .collect();
    got.sort_unstable();
    // Batch 230 note: the two new pedantic rows (assert_is_empty,
    // manual_bit_width) are AHEAD-OF-PIN (clippy 1.99) — their rows are
    // derived from the isolated +1.99.0 driver, not the pinned 1.98.1
    // (see AHEAD_OF_PIN_1_99); the membership this test pins is unchanged
    // in KIND: both are allow-by-default pedantic lints.
    // P51a note (2026-10-14): match_same_arms + used_underscore_binding
    // join — both allow-by-default pedantic rows per the pinned 1.98.1
    // driver, the same KIND.
    let mut want = vec![
        "assert_is_empty",
        "cast_lossless",
        "collapsible_else_if",
        "doc_markdown",
        "format_collect",
        "items_after_statements",
        "manual_let_else",
        "manual_bit_width",
        "manual_string_new",
        "map_unwrap_or",
        "match_bool",
        "match_same_arms",
        "needless_pass_by_ref_mut",
        "range_plus_one",
        "redundant_closure_for_method_calls",
        "semicolon_if_nothing_returned",
        "single_char_pattern",
        "single_match_else",
        "string_lit_as_bytes",
        "suboptimal_flops",
        "uninlined_format_args",
        "unreadable_literal",
        "use_self",
        "used_underscore_binding",
    ];
    want.sort_unstable();
    assert_eq!(got, want);
    assert!(
        !is_allow_by_default("unnecessary_map_or"),
        "style at 1.98 — the stale corpus label"
    );
    assert!(!is_allow_by_default("needless_return"));
}

#[test]
fn corpus_groups_agree_with_the_table() {
    let corpus = crate::domains::clippy_lints::corpus::build_seed_corpus();
    let mut bad = Vec::new();
    for e in &corpus.entries {
        if let Some(g) = default_group(&e.lint_name) {
            if g != e.lint_group {
                bad.push(format!(
                    "{}: corpus {} vs pinned {g}",
                    e.lint_name, e.lint_group
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "corpus lint_group disagrees with LINT_GROUPS:\n{}",
        bad.join("\n")
    );
}

#[test]
fn table_and_corpus_match_installed_driver() {
    // Live half of T1: when the pinned driver is on PATH, every table row
    // AND every corpus label must match its `-W help`. Any other state is
    // printed as UNSEEN — never read as a pass.
    let Ok(out) = std::process::Command::new("clippy-driver")
        .args(["-W", "help"])
        .stdin(std::process::Stdio::null())
        .output()
    else {
        eprintln!("UNSEEN: clippy-driver not spawnable — the driver cross-check did not run");
        return;
    };
    let version = std::process::Command::new("clippy-driver")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    if !out.status.success() || !version.contains("0.1.98") {
        eprintln!(
            "UNSEEN: clippy-driver is `{}` — not the pinned {PINNED_TOOLCHAIN}",
            version.trim()
        );
        return;
    }
    let surface = crate::corpus_coverage::parse_lint_help(&String::from_utf8_lossy(&out.stdout));
    assert!(
        surface.blind.is_none(),
        "driver surface blind: {:?}",
        surface.blind
    );
    let driver: HashMap<&str, &str> = surface
        .lints
        .iter()
        .map(|r| (r.name.as_str(), r.group.as_str()))
        .collect();
    let mut bad = Vec::new();
    for (lint, g) in LINT_GROUPS {
        // AHEAD-OF-PIN rows (Batch 230): the pinned 1.98.1 driver cannot
        // know them — they are verified against the +1.99.0 driver by
        // `ahead_of_pin_rows_match_the_1_99_driver` below, not here.
        if !driver.contains_key(lint) {
            assert!(
                AHEAD_OF_PIN_1_99.contains(lint),
                "table {lint} is listed AHEAD-OF-PIN but the pinned driver already knows it — re-merge the row"
            );
            continue;
        }
        match driver.get(lint) {
            Some(d) if d == g => {}
            other => bad.push(format!("table {lint}: {g} vs driver {other:?}")),
        }
    }
    let corpus = crate::domains::clippy_lints::corpus::build_seed_corpus();
    for e in &corpus.entries {
        if let Some(d) = driver.get(e.lint_name.as_str()) {
            if *d != e.lint_group {
                bad.push(format!(
                    "corpus {}: {} vs driver {d}",
                    e.lint_name, e.lint_group
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "re-derive from the driver:\n{}",
        bad.join("\n")
    );
}

/// The AHEAD-OF-PIN half of the driver check (Batch 230): every
/// [`AHEAD_OF_PIN_1_99`] row is verified against the ISOLATED `+1.99.0`
/// clippy driver when it is installed (this box: `rustup toolchain install
/// 1.99.0`); on a box without it the check reads UNSEEN — never a pass.
#[test]
fn ahead_of_pin_rows_match_the_1_99_driver() {
    let Ok(out) = std::process::Command::new("clippy-driver")
        .args(["+1.99.0", "-W", "help"])
        .stdin(std::process::Stdio::null())
        .output()
    else {
        eprintln!(
            "UNSEEN: clippy-driver +1.99.0 not spawnable — the ahead-of-pin check did not run"
        );
        return;
    };
    let version = std::process::Command::new("clippy-driver")
        .args(["+1.99.0", "--version"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    if !out.status.success() || !version.contains("0.1.99") {
        eprintln!(
            "UNSEEN: clippy-driver +1.99.0 is `{}` — not the expected 0.1.99",
            version.trim()
        );
        return;
    }
    let surface = crate::corpus_coverage::parse_lint_help(&String::from_utf8_lossy(&out.stdout));
    assert!(
        surface.blind.is_none(),
        "1.99 driver surface blind: {:?}",
        surface.blind
    );
    let driver: HashMap<&str, &str> = surface
        .lints
        .iter()
        .map(|r| (r.name.as_str(), r.group.as_str()))
        .collect();
    let mut bad = Vec::new();
    for lint in AHEAD_OF_PIN_1_99 {
        let want = default_group(lint).expect("ahead-of-pin row missing from LINT_GROUPS");
        match driver.get(lint) {
            Some(d) if *d == want => {}
            other => bad.push(format!("ahead row {lint}: {want} vs 1.99 driver {other:?}")),
        }
    }
    assert!(
        bad.is_empty(),
        "re-derive ahead-of-pin rows from the +1.99.0 driver:\n{}",
        bad.join("\n")
    );
}

// ── T2: effective level ─────────────────────────────────────────────────

#[test]
fn allow_by_default_lint_is_off_without_a_directive() {
    assert!(!LintLevels::default().enables("doc_markdown"));
    assert!(LintLevels::default().enables("needless_return"));
}

#[test]
fn group_and_lint_directives_resolve_in_order() {
    assert!(levels(&[("pedantic", Level::Warn)]).enables("doc_markdown"));
    assert!(levels(&[("doc_markdown", Level::Deny)]).enables("doc_markdown"));
    assert!(levels(&[("doc_markdown", Level::Expect)]).enables("doc_markdown"));
    assert!(
        !levels(&[("pedantic", Level::Warn), ("doc_markdown", Level::Allow)])
            .enables("doc_markdown")
    );
    assert!(
        levels(&[("doc_markdown", Level::Allow), ("pedantic", Level::Warn)])
            .enables("doc_markdown")
    );
    // A different allow-by-default group does not reach it.
    assert!(!levels(&[("nursery", Level::Warn)]).enables("doc_markdown"));
    assert!(levels(&[("nursery", Level::Warn)]).enables("use_self"));
    // Allowing a default-on lint turns it off; `all` raises default-on only.
    assert!(!levels(&[("needless_return", Level::Allow)]).enables("needless_return"));
    assert!(!levels(&[("all", Level::Warn)]).enables("doc_markdown"));
    assert!(!levels(&[("all", Level::Allow)]).enables("needless_return"));
}

#[test]
fn forbid_is_sticky() {
    assert!(
        levels(&[("pedantic", Level::Forbid), ("doc_markdown", Level::Allow)])
            .enables("doc_markdown")
    );
}

#[test]
fn manifest_section_and_inline_table_forms() {
    let text = "[package]\nname = \"x\"\n\n[lints.clippy]\n\
                doc_markdown = \"allow\" # trailing comment\n\
                pedantic = { level = \"warn\", priority = -1 }\n";
    let (ds, inherits) = parse_manifest_lints(text, false);
    assert!(!inherits);
    // Group at priority -1 sorts BEFORE the lint at 0 — the lint wins.
    assert_eq!(
        ds,
        vec![
            directive("pedantic", Level::Warn),
            directive("doc_markdown", Level::Allow)
        ]
    );
    assert!(!LintLevels { directives: ds }.enables("doc_markdown"));
}

#[test]
fn manifest_equal_priority_puts_groups_first() {
    let text = "[lints.clippy]\ndoc_markdown = \"allow\"\npedantic = \"warn\"\n";
    let (ds, _) = parse_manifest_lints(text, false);
    assert_eq!(ds[0].name, "pedantic");
    assert!(!LintLevels { directives: ds }.enables("doc_markdown"));
}

#[test]
fn manifest_dotted_keys_workspace_flag_and_workspace_table() {
    let pkg =
        "[lints]\nworkspace = true\nclippy.match-bool = \"warn\"\nrust.unsafe_code = \"deny\"\n";
    let (ds, inherits) = parse_manifest_lints(pkg, false);
    assert!(inherits);
    assert_eq!(ds, vec![directive("match_bool", Level::Warn)]);

    let ws = "[workspace]\nmembers = []\n[workspace.lints.clippy]\npedantic = \"warn\"\n\
              [lints.clippy]\nnursery = \"warn\"\n";
    assert_eq!(
        parse_manifest_lints(ws, true).0,
        vec![directive("pedantic", Level::Warn)]
    );
    assert_eq!(
        parse_manifest_lints(ws, false).0,
        vec![directive("nursery", Level::Warn)]
    );
    // Other sections never leak in.
    assert_eq!(
        parse_manifest_lints("[dependencies]\npedantic = \"warn\"\n", false).0,
        Vec::<Directive>::new()
    );
}

#[test]
fn inner_attrs_parse_clippy_paths_only() {
    let f = syn::parse_file(
        "#![warn(clippy::all, clippy::pedantic)]\n\
         #![allow(clippy::doc_markdown, reason = \"prose\")]\n\
         #![deny(unsafe_code)]\n\
         #![cfg_attr(test, warn(clippy::nursery))]\n\
         #[warn(clippy::match_bool)]\nfn f() {}\n",
    )
    .unwrap();
    assert_eq!(
        parse_inner_attrs(&f),
        vec![
            directive("all", Level::Warn),
            directive("pedantic", Level::Warn),
            directive("doc_markdown", Level::Allow),
        ]
    );
}

#[test]
fn crate_roots_follow_cargo_layout() {
    let pkg = scratch("layout");
    let p = |s: &str| pkg.join(s);
    // No lib.rs, no main.rs → a src module has no known root.
    assert_eq!(crate_root_for(&pkg, &p("src/a.rs")), None);
    write(&p("src/main.rs"), "");
    assert_eq!(
        crate_root_for(&pkg, &p("src/a/b.rs")),
        Some(p("src/main.rs"))
    );
    write(&p("src/lib.rs"), "");
    assert_eq!(crate_root_for(&pkg, &p("src/a.rs")), Some(p("src/lib.rs")));
    assert_eq!(
        crate_root_for(&pkg, &p("src/bin/t.rs")),
        Some(p("src/bin/t.rs"))
    );
    assert_eq!(
        crate_root_for(&pkg, &p("src/bin/t/m.rs")),
        Some(p("src/bin/t/main.rs"))
    );
    assert_eq!(
        crate_root_for(&pkg, &p("tests/it.rs")),
        Some(p("tests/it.rs"))
    );
    assert_eq!(
        crate_root_for(&pkg, &p("benches/b/x.rs")),
        Some(p("benches/b/main.rs"))
    );
    assert_eq!(crate_root_for(&pkg, &p("build.rs")), Some(p("build.rs")));
    assert_eq!(crate_root_for(&pkg, &p("other/x.rs")), None);
    let _ = std::fs::remove_dir_all(&pkg);
}

fn resolve(file: &Path) -> LintLevels {
    let parsed = syn::parse_file(&std::fs::read_to_string(file).unwrap()).unwrap();
    LevelResolver::new().levels_for(file, &parsed)
}

#[test]
fn resolver_issue_135_repro_crate_does_not_enable_doc_markdown() {
    // The issue's minimal repro: a plain crate, no clippy config at all.
    let pkg = scratch("repro");
    write(&pkg.join("Cargo.toml"), &format!("{PKG}[workspace]\n"));
    let lib = pkg.join("src/lib.rs");
    write(&lib, "//! Regen.\n//!\n//!   cargo run -p healrepro --bin gen_corpus\n\npub fn one() -> u32 { 1 }\n");
    let lv = resolve(&lib);
    assert!(!lv.enables("doc_markdown"));
    assert!(lv.enables("needless_return"));
    let _ = std::fs::remove_dir_all(&pkg);
}

#[test]
fn resolver_reads_crate_root_attrs_for_a_module_file() {
    let pkg = scratch("rootattr");
    write(&pkg.join("Cargo.toml"), PKG);
    write(
        &pkg.join("src/lib.rs"),
        "#![warn(clippy::pedantic)]\nmod m;\n",
    );
    let m = pkg.join("src/m.rs");
    write(&m, "pub fn f() {}\n");
    let mut r = LevelResolver::new();
    let parsed = syn::parse_file("pub fn f() {}\n").unwrap();
    assert!(r.levels_for(&m, &parsed).enables("doc_markdown"));
    // The module's own scope overrides the root.
    let parsed = syn::parse_file("#![allow(clippy::doc_markdown)]\npub fn f() {}\n").unwrap();
    assert!(!r.levels_for(&m, &parsed).enables("doc_markdown"));
    // The root itself, resolved after a module (cache shared) — same answer.
    assert!(resolve(&pkg.join("src/lib.rs")).enables("doc_markdown"));
    let _ = std::fs::remove_dir_all(&pkg);
}

#[test]
fn resolver_reads_package_and_inherited_workspace_lints() {
    let ws = scratch("wslints");
    write(
        &ws.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\n[workspace.lints.clippy]\npedantic = \"warn\"\n",
    );
    let a = ws.join("a");
    write(
        &a.join("Cargo.toml"),
        &format!("{PKG}[lints]\nworkspace = true\n"),
    );
    write(&a.join("src/lib.rs"), "pub fn f() {}\n");
    let b = ws.join("b");
    write(
        &b.join("Cargo.toml"),
        &format!("{PKG}[lints.clippy]\nnursery = \"warn\"\n"),
    );
    write(&b.join("src/lib.rs"), "pub fn f() {}\n");
    assert!(resolve(&a.join("src/lib.rs")).enables("doc_markdown"));
    let lb = resolve(&b.join("src/lib.rs"));
    assert!(
        !lb.enables("doc_markdown"),
        "b does not inherit the workspace table"
    );
    assert!(lb.enables("use_self"));
    let _ = std::fs::remove_dir_all(&ws);
}

#[test]
fn gated_lints_honours_crate_level_and_group_opt_in() {
    let none = LintLevels::default();
    let all_gated = gated_lints(&none, &[]);
    assert!(all_gated.contains(&"doc_markdown") && all_gated.contains(&"use_self"));
    assert!(
        !all_gated.contains(&"needless_return"),
        "default-on lints are never gated"
    );
    assert_eq!(
        all_gated.len(),
        24,
        "the pinned allow-by-default fix-set membership"
    );

    let ped = gated_lints(&none, &["pedantic".to_string()]);
    assert!(!ped.contains(&"doc_markdown") && ped.contains(&"use_self"));
    assert_eq!(gated_lints(&none, &["all".to_string()]), Vec::<&str>::new());

    let crate_ped = levels(&[("pedantic", Level::Warn), ("doc_markdown", Level::Allow)]);
    let g = gated_lints(&crate_ped, &[]);
    assert!(
        g.contains(&"doc_markdown"),
        "an explicit allow keeps it gated"
    );
    assert!(!g.contains(&"match_bool"), "the crate enables pedantic");
    assert!(g.contains(&"use_self"), "nursery is not pedantic");
}
