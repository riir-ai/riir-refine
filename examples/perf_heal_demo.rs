//! Example: heal a perf rule using the `rust_perf` domain.
//!
//! Demonstrates the Phase 2 scaffold end-to-end:
//! 1. Build the seed corpus (39 rules from the rust-optimize skill checklist
//!    + nnethercote/perf-book + rustc-hash/bevy patterns).
//! 2. Validate that each fixed pattern parses via the L2 pruner.
//! 3. Show the corpus commitment + tamper-evidence check.
//! 4. Optionally run `cargo test` verification if `rust_perf_verify` is on.
//!
//! Run with:
//! ```text
//! cargo run --example perf_heal_demo --features rust_perf
//! cargo run --example perf_heal_demo --features rust_perf_verify  # with cargo test
//! ```

use riir_refine::corpus::Corpus;
use riir_refine::domain::HealDomain;
use riir_refine::domains::rust_perf::RustPerfDomain;

fn main() {
    println!("=== rust_perf domain — Phase 2 scaffold demo ===\n");

    // 1. Build the seed corpus.
    let corpus: Corpus<RustPerfDomain> = riir_refine::build_perf_seed_corpus();
    println!("Seed corpus: {} entries", corpus.entries.len());
    println!(
        "Commitment (BLAKE3): {}",
        hex_commitment(&corpus.commitment)
    );
    println!();

    // 2. List the rules by category.
    println!("Rules by category:");
    let mut entries = corpus.entries.clone();
    entries.sort_by(|a, b| a.category.cmp(&b.category).then(a.rule_id.cmp(&b.rule_id)));
    for entry in &entries {
        println!("  [{}] {} — {}", entry.category, entry.rule_id, entry.name);
    }
    println!();

    // 3. Validate each fixed pattern via the domain's validate() (L2 fast path
    //    when rust_perf_verify is off; cargo test when it's on).
    println!("Validating fixed patterns via domain validate():");
    let mut all_ok = true;
    for entry in &corpus.entries {
        #[cfg(not(feature = "rust_perf_verify"))]
        {
            let verdict = RustPerfDomain::validate(&entry.rule_id, &entry.fixed_code);
            let status = if verdict.parses { "OK" } else { "FAIL" };
            println!("  [{status}] {}", entry.rule_id);
            if !verdict.parses {
                all_ok = false;
            }
        }
        #[cfg(feature = "rust_perf_verify")]
        {
            let verdict = RustPerfDomain::validate(&entry.rule_id, &entry.fixed_code);
            let status = if verdict.test_passes { "OK" } else { "FAIL" };
            println!(
                "  [{status}] {} (passed={}, failed={})",
                entry.rule_id, verdict.passed, verdict.failed
            );
            if !verdict.test_passes {
                all_ok = false;
            }
        }
    }
    println!();

    if all_ok {
        println!("\u{2713} All fixed patterns validated");
    } else {
        println!("\u{2717} Some fixed patterns failed validation");
    }
    println!();

    // 4. Tamper-evidence check.
    println!("Tamper-evidence check:");
    println!("  verify() on pristine corpus: {}", corpus.verify());
    let mut tampered = corpus.clone();
    tampered.entries[0].fixed_code = "// tampered".to_string();
    println!(
        "  verify() on tampered corpus: {} (expected: false)",
        tampered.verify()
    );
    println!();

    // 5. Trait impl exercise.
    println!("HealDomain trait impl:");
    let via_trait = RustPerfDomain::build_corpus();
    println!(
        "  build_corpus() returned {} entries (matches seed: {})",
        via_trait.len(),
        via_trait.len() == corpus.entries.len()
    );
    println!();

    // 6. Optional: cargo test verification (if rust_perf_verify is on).
    #[cfg(feature = "rust_perf_verify")]
    {
        println!("=== rust_perf_verify: running cargo test on first rule ===");
        use riir_refine::PerfVerifier;
        let verifier = PerfVerifier::new();
        if !verifier.is_cargo_available() {
            println!("  (cargo not on PATH — skipping cargo test verification)");
        } else {
            let first = &corpus.entries[0];
            println!("  Verifying rule: {}", first.rule_id);
            let result = verifier.verify(&first.fixed_code, &first.rule_id);
            println!(
                "  ran={}, test_passes={}, passed={}, failed={}",
                result.ran, result.test_passes, result.passed, result.failed
            );
            if let Some(e) = &result.error {
                println!("  error: {e}");
            }
        }
    }

    println!("\n=== Phase 2 scaffold demo complete ===");
}

/// Format a 32-byte BLAKE3 hash as a hex string.
fn hex_commitment(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
