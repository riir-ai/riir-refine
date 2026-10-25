//! Issue 171 T0b GOAT gate — package-scoped re-checks in the `--fix --verify`
//! loop.
//!
//! A real three-package cargo workspace: `a` (one GOOD edit + one BAD edit),
//! `b` (independent, one good edit), `c` (depends on `a`, one good edit).
//! Round 1 is dirty in `a` only; round 2 must re-check `a` (its file was
//! reverted) and `c` (it depends on `a`) — and only `b` may be skipped.
//!
//! - **G1 (soundness + yield):** the kept/reverted sets and the final tree
//!   bytes are IDENTICAL with the mechanism on and off.
//! - **G2 (cost):** on-arm `n_checks` = off-arm − skipped, skipped > 0;
//!   paired interleaved wall-clock printed (`--nocapture`), metadata cost
//!   INCLUDED in the on arm. Counts are asserted; time is disclosed, never
//!   asserted (a shared box measures the scheduler).
//! - **G3 (dependency clause):** a lib-level bad edit in `a` makes `c` dirty
//!   too; `c` is never skipped while it depends on a reverted package.
//!
//! Spawns real cargo (seconds) — a dedicated target, run by name:
//! `cargo test --features fix_verify,scoped_recheck --test scoped_recheck_goat -- --nocapture`.

use std::path::{Path, PathBuf};
use std::time::Instant;

use riir_refine::fix_verify::{EditAtLine, FileEditSet, FixVerifier, VerifyReport};

fn edit(line: usize, replacement: &str, restore: &str) -> EditAtLine {
    EditAtLine {
        rule_id: "goat-rule".to_string(),
        final_line: line,
        replacement: replacement.to_string(),
        restore: restore.to_string(),
        rejected: Vec::new(),
    }
}

struct Fixture {
    root: PathBuf,
    sets: Vec<FileEditSet>,
}

const A_ORIG: &str = "pub fn one() -> usize {\n    1\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n        let x: usize = 1;\n        assert_eq!(x, super::one());\n    }\n}\n";
const B_ORIG: &str = "pub fn two() -> usize {\n    2\n}\n";
const C_ORIG: &str = "pub fn three() -> usize {\n    a::one() + 2\n}\n";

/// `lib_level_bad`: the bad edit lands in `a`'s LIB code (so `c`'s check
/// fails too) instead of `a`'s test code (so `c`'s check is clean).
fn fixture(tag: &str, lib_level_bad: bool) -> Fixture {
    let root =
        std::env::temp_dir().join(format!("scoped_recheck_goat_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for (name, deps) in [("a", ""), ("b", ""), ("c", "a = { path = \"../a\" }\n")] {
        let dir = root.join(name).join("src");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            root.join(name).join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps}"
            ),
        )
        .unwrap();
    }
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\", \"c\"]\nresolver = \"2\"\n",
    )
    .unwrap();

    let (a_fixed, a_edits) = if lib_level_bad {
        // Bad on lib line 2 (`1u8` returned as usize: E0308 reported in
        // a's file by BOTH a's and c's checks); good on test line 9.
        (
            A_ORIG.replacen("    1\n", "    1u8\n", 1).replacen(
                "let x: usize = 1;",
                "let x: usize = (1);",
                1,
            ),
            vec![edit(2, "1u8", "1"), edit(9, "(1)", "1")],
        )
    } else {
        (
            A_ORIG.replacen("    1\n", "    (1)\n", 1).replacen(
                "let x: usize = 1;",
                "let x: usize = 1u8;",
                1,
            ),
            vec![edit(2, "(1)", "1"), edit(9, "1u8", "1")],
        )
    };
    let sets = vec![
        FileEditSet {
            path: root.join("a/src/lib.rs"),
            original: A_ORIG.to_string(),
            fixed: a_fixed,
            edits: a_edits,
        },
        FileEditSet {
            path: root.join("b/src/lib.rs"),
            original: B_ORIG.to_string(),
            fixed: B_ORIG.replacen("    2\n", "    (2)\n", 1),
            edits: vec![edit(2, "(2)", "2")],
        },
        FileEditSet {
            path: root.join("c/src/lib.rs"),
            original: C_ORIG.to_string(),
            fixed: C_ORIG.replacen("    a::one() + 2\n", "    (a::one() + 2)\n", 1),
            edits: vec![edit(2, "(a::one() + 2)", "a::one() + 2")],
        },
    ];
    let fx = Fixture { root, sets };
    reset(&fx);
    fx
}

fn reset(fx: &Fixture) {
    for s in &fx.sets {
        std::fs::write(&s.path, &s.original).unwrap();
    }
}

fn snapshot(fx: &Fixture) -> Vec<String> {
    fx.sets
        .iter()
        .map(|s| std::fs::read_to_string(&s.path).unwrap())
        .collect()
}

fn reverted_keys(r: &VerifyReport, root: &Path) -> Vec<(String, usize, bool)> {
    let mut v: Vec<_> = r
        .reverted
        .iter()
        .map(|x| {
            (
                x.path
                    .strip_prefix(root)
                    .unwrap_or(&x.path)
                    .display()
                    .to_string(),
                x.line,
                x.whole_file,
            )
        })
        .collect();
    v.sort();
    v
}

fn run(fx: &Fixture, on: bool) -> (VerifyReport, Vec<String>, f64) {
    reset(fx);
    let t = Instant::now();
    let report = FixVerifier::new().with_scoped_recheck(on).verify(&fx.sets);
    let secs = t.elapsed().as_secs_f64();
    (report, snapshot(fx), secs)
}

fn goat_case(tag: &str, lib_level_bad: bool) {
    // The verifier inherits the environment: an inherited CARGO_TARGET_DIR
    // (the outer `cargo test`'s) would make both cases' `a`/`b`/`c`
    // packages share — and race on — one target dir. Each fixture builds
    // into its own `<root>/target` instead.
    std::env::remove_var("CARGO_TARGET_DIR");
    let fx = fixture(tag, lib_level_bad);
    // Warm the target dir once so neither arm pays the cold build.
    let _ = run(&fx, false);

    let mut off_t = Vec::new();
    let mut on_t = Vec::new();
    let mut last: Option<(VerifyReport, VerifyReport)> = None;
    for pair in 0..3 {
        // Interleave, alternating which arm goes first.
        let (first_on, second_on) = if pair % 2 == 0 {
            (false, true)
        } else {
            (true, false)
        };
        let r1 = run(&fx, first_on);
        let r2 = run(&fx, second_on);
        let (off, on) = if first_on { (r2, r1) } else { (r1, r2) };
        assert!(off.0.error.is_none(), "off arm errored: {:?}", off.0);
        assert!(on.0.error.is_none(), "on arm errored: {:?}", on.0);
        // G1: identical outcome, byte-for-byte.
        assert_eq!(off.1, on.1, "[{tag}] final tree bytes differ on vs off");
        assert_eq!(
            off.0.kept_edits, on.0.kept_edits,
            "[{tag}] kept edits differ"
        );
        assert_eq!(
            off.0.kept_files, on.0.kept_files,
            "[{tag}] kept files differ"
        );
        assert_eq!(
            reverted_keys(&off.0, &fx.root),
            reverted_keys(&on.0, &fx.root),
            "[{tag}] reverted sets differ"
        );
        off_t.push(off.2);
        on_t.push(on.2);
        last = Some((off.0, on.0));
    }
    let (off, on) = last.unwrap();
    // The bad edit was caught and the good ones kept.
    assert!(
        !off.reverted.is_empty(),
        "[{tag}] the bad edit must revert: {off:?}"
    );
    assert_eq!(off.skipped_checks, 0);
    // G2: `b` skipped exactly once (round 2); counts reconcile.
    assert_eq!(
        on.skipped_checks, 1,
        "[{tag}] exactly b skipped in round 2: {on:?}"
    );
    assert_eq!(
        on.n_checks + on.skipped_checks,
        off.n_checks,
        "[{tag}] on-arm checks + skips = off-arm checks"
    );
    off_t.sort_by(f64::total_cmp);
    on_t.sort_by(f64::total_cmp);
    println!(
        "[{tag}] n_checks off={} on={} (skipped {}) · kept {}/{} edits · median wall off={:.2}s on={:.2}s (metadata included)",
        off.n_checks,
        on.n_checks,
        on.skipped_checks,
        on.kept_edits,
        fx.sets.iter().map(|s| s.edits.len()).sum::<usize>(),
        off_t[1],
        on_t[1],
    );
    let _ = std::fs::remove_dir_all(&fx.root);
}

/// Bad edit in `a`'s TEST code: `c`'s round-1 check is clean, yet `c` must
/// re-check in round 2 because it depends on the reverted `a`.
#[test]
fn scoped_recheck_goat_test_level_bad_edit() {
    goat_case("testlvl", false);
}

/// Bad edit in `a`'s LIB code: `c` is dirty in round 1 too (the error is
/// reported in `a`'s file through `c`'s check) — still only `b` skips.
#[test]
fn scoped_recheck_goat_lib_level_bad_edit() {
    goat_case("liblvl", true);
}
