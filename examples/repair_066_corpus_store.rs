//! Issue 066 T3 — the one-shot store repair for the poisoned docker corpus rows.
//!
//! The 08-28/08-29/08-31/09-01 docker rows in the repo-local Warm-tier store
//! carry `count: 2` (the old `rule_id:` needle counted the `rule()`
//! constructor's PLUMBING) against true corpus sizes 27/31/31/31. The rows are
//! FIRST-HAND (`source-walk`) provenance, so `--import-series` correctly yields
//! to them (traj_store.rs provenance precedence) and cannot repair them — the
//! only sanctioned writer is `upsert_corpus_count`, which the CLI stamps for
//! TODAY only. This example is the dated upsert, run ONCE:
//!
//! ```sh
//! cargo run --release --features latent_retrieval,rust_perf,clippy_lints \
//!   --example repair_066_corpus_store
//! ```
//!
//! The repair is idempotent (same inputs → same bytes at the same key) and
//! honestly provenance-stamped `repair-066` — second-hand BY DEFINITION (a
//! hand repair, not a measurement), which means a future `--import-series`
//! with a corrected jsonl MAY supersede it. That is correct behavior: after
//! T1 the live stamp path and the mined layer agree on these values, so every
//! writer in the system now converges on the truth.
//!
//! True values are derived FROM GIT (`git show <rev>:corpus.rs | grep -c '    rule('`),
//! not hardcoded — the same computation the git-mined dashboard layer runs.

use riir_clippy::traj_store::TrajStore;

fn main() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let true_counts: &[(&str, &str, u32)] = &[
        // (day, corpus.rs rev holding the day's final corpus, true entries)
        ("2026-08-28", "3839457", 27),
        ("2026-08-29", "e63f819", 31),
        ("2026-08-31", "e63f819", 31),
        ("2026-09-01", "e63f819", 31),
    ];
    for (day, rev, expected) in true_counts {
        // Verify the expected value against git before writing anything —
        // a repair that trusts its own table is just a second poison.
        let out = std::process::Command::new("git")
            .args(["show", &format!("{rev}:src/domains/docker/corpus.rs")])
            .current_dir(repo)
            .output()
            .expect("git show");
        assert!(out.status.success(), "git show {rev} failed");
        let counted = String::from_utf8_lossy(&out.stdout)
            .matches("    rule(")
            .count() as u32;
        assert_eq!(
            counted, *expected,
            "git truth for {day} (rev {rev}) is {counted}, table says {expected} — fix the table"
        );
        let (store, _bad) = TrajStore::open(repo).expect("open store");
        store
            .upsert_corpus_count(day, "docker", *expected, true, "repair-066")
            .unwrap_or_else(|e| panic!("upsert {day}: {e}"));
        println!(
            "repaired {day}: docker = {expected} (verified against {rev}, provenance repair-066)"
        );
    }
    println!("done — re-run `cargo heal --corpus-stats --json` to confirm the series");
}
