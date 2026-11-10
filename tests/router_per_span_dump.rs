//! Issue 170 T1 — the per-span ROUTER-annotation dump (the corpus-batch
//! re-pin instrument).
//!
//! `retrieval_precision_regression_gate` asserts the router-annotation floor
//! but prints only aggregate counts; a re-pin (the Issue-020 contract:
//! re-measure + re-pin with the delta recorded, never silently) needs the
//! PER-SPAN routing verdicts so a batch's decay can be attributed — which
//! spans newly misroute, and to which domain's centroid. This test prints
//! exactly that (sorted, tab-separated, diff-friendly) plus the full
//! `run_eval` pin basis, and asserts only population/walk parity — it is an
//! instrument, not a gate (the gate next door owns the floor assertions).
//!
//! Typical use — attribute a floor delta between two corpus states:
//!
//! ```sh
//! # at HEAD:
//! cargo test --release --features latent_retrieval --test router_per_span_dump \
//!   -- --nocapture > /tmp/dump_head.txt
//! # at the last pinned state (detached worktree, same target dir):
//! git worktree add ../riir-refine-wPIN <pin-commit>
//! cp tests/router_per_span_dump.rs ../riir-refine-wPIN/tests/
//! cd ../riir-refine-wPIN && CARGO_TARGET_DIR=<shared> cargo test --release \
//!   --features latent_retrieval --test router_per_span_dump -- --nocapture \
//!   > /tmp/dump_pin.txt
//! diff /tmp/dump_pin.txt /tmp/dump_head.txt   # the newly-misrouting rows
//! ```
//!
//! Posture: the four default-enabled domains (clippy_lints, rust_perf,
//! docker, dist) + latent_retrieval — the state `router_annotation_floor`'s
//! `(false, false, true)` arm is pinned at.
#![cfg(feature = "latent_retrieval")]

use riir_refine::draft::{LatentMatcher, retrieval_eval};

#[test]
fn router_per_span_dump() {
    let mut matcher = LatentMatcher::from_enabled_domains().expect("build matcher");
    let spans = retrieval_eval::labeled_eval_spans();

    let mut rows: Vec<String> = Vec::new();
    let mut checked = 0usize;
    let mut correct = 0usize;
    for span in &spans {
        if !matcher.has_domain(span.domain) {
            continue;
        }
        // The router verdict is computed BEFORE retrieval and does not depend
        // on the pool depth; 8 mirrors the eval's private PROBE_TOP_K (the
        // multivector.rs duplicate precedent — one line, deliberate).
        let (routed, _) = matcher.suggest_span(&span.code, 8);
        checked += 1;
        let ok = routed == span.domain;
        correct += usize::from(ok);
        rows.push(format!(
            "{verdict}\t{expected:?}\t{label}\t{routed:?}",
            verdict = if ok { "ok" } else { "MISS" },
            expected = span.domain,
            label = span.label,
            routed = routed
        ));
    }
    rows.sort_unstable();
    for row in &rows {
        println!("{row}");
    }
    #[allow(clippy::cast_precision_loss)]
    {
        println!(
            "ROUTER_SUMMARY {correct}/{checked} ({:.1}%)",
            correct as f32 / checked.max(1) as f32 * 100.0
        );
    }

    // The full pin basis at this state — the same report the regression gate
    // asserts against (blended top-1/3/5, per-domain, the router line, the
    // miss taxonomy with each miss's outranking top-1).
    let report = retrieval_eval::run_eval();
    retrieval_eval::print_report(&report);
    println!(
        "EVAL_SUMMARY blended_top1 {}/{} blended_top3 {}/{} blended_top5 {}/{} router {}/{}",
        report.blended.hits_at_1,
        report.blended.labeled,
        report.blended.hits_at_3,
        report.blended.labeled,
        report.blended.hits_at_5,
        report.blended.labeled,
        report.router_correct,
        report.router_checked,
    );

    // Population floor: the default posture carries 141 labeled spans at the
    // pinned corpus states; a walk or fixture regression reds here instead of
    // printing a confident table over a shrunken population.
    assert!(
        checked >= 100,
        "router dump population collapsed: {checked}"
    );
    // The dump walk and the gate's own eval must traverse the SAME span
    // population — a walk drift would make the dump attribute a population
    // the gate never measured.
    assert_eq!(
        report.router_checked, checked,
        "dump walk and run_eval disagree on the span population"
    );
}

/// Issue 170 T1 — WHY the router decayed: the shared 128-slot vocab
/// (`SPECTRAL_VOCAB_N`, `SpectralOrder::GlobalFreq` — top-128 tokens by
/// GLOBAL frequency across every enabled domain's corpus texts) is the only
/// corpus-dependent component shared by all domains. Corpus growth in ANY
/// domain re-ranks the global frequency table, so tokens enter/leave the
/// top-128 and every domain's centroid moves with them. This probe prints
/// the vocab membership at the current corpus state (run it in a worktree at
/// a pinned commit and diff — same protocol as [`router_per_span_dump`]):
/// per-domain in-vocab token coverage + which domain-distinctive tokens
/// survive. Membership is read through the public `tf_embed` (a single
/// token's embedding is 1.0 in its slot when in-vocab, all-zero when not —
/// `count_into` is private, deliberately unreached).
///
/// Measurement only — asserts nothing beyond the tokenizer self-check.
#[test]
fn vocab_membership_probe() {
    use riir_refine::HealEntry;
    use riir_refine::domain::HealDomain;
    use riir_refine::domains::clippy_lints::ClippyLintDomain;
    use riir_refine::domains::dist::DistDomain;
    use riir_refine::domains::docker::DockerDomain;
    use riir_refine::domains::rust_perf::RustPerfDomain;
    use riir_refine::draft::{DomainId, SpectralOrder, SpectralVocab};

    // Same tokenizer contract as `for_each_token` (domain_router.rs):
    // runs of [A-Za-z0-9_], length >= 2, case-sensitive.
    fn tokens_of(text: &str) -> Vec<&str> {
        let mut out = Vec::new();
        let bytes = text.as_bytes();
        let mut start = None;
        for (i, b) in bytes.iter().enumerate() {
            let tok_char = b.is_ascii_alphanumeric() || *b == b'_';
            if tok_char && start.is_none() {
                start = Some(i);
            } else if !tok_char {
                if let Some(s) = start {
                    if i - s >= 2 {
                        out.push(&text[s..i]);
                    }
                    start = None;
                }
            }
        }
        if let Some(s) = start {
            if bytes.len() - s >= 2 {
                out.push(&text[s..]);
            }
        }
        out
    }

    let domains: Vec<(DomainId, Vec<String>)> = vec![
        (
            DomainId::Clippy,
            ClippyLintDomain::build_corpus()
                .into_iter()
                .map(|e| e.buggy_code().to_string())
                .collect(),
        ),
        (
            DomainId::Perf,
            RustPerfDomain::build_corpus()
                .into_iter()
                .map(|e| e.buggy_code().to_string())
                .collect(),
        ),
        (
            DomainId::Docker,
            DockerDomain::build_corpus()
                .into_iter()
                .map(|e| e.buggy_code().to_string())
                .collect(),
        ),
        (
            DomainId::Dist,
            DistDomain::build_corpus()
                .into_iter()
                .map(|e| e.buggy_code().to_string())
                .collect(),
        ),
    ];
    let vocab = SpectralVocab::build(&domains, SpectralOrder::GlobalFreq);
    let members: std::collections::HashSet<&str> =
        vocab.tokens().iter().map(|t| t.as_ref()).collect();
    println!("VOCAB_SIZE {}", vocab.tokens().len());
    println!("VOCAB_TOKENS {}", vocab.tokens().join(","));

    // Per-domain coverage: what fraction of each domain's corpus token
    // occurrences survive the top-128 cut — the mass a domain's centroid can
    // actually carry (the tf_embed's non-vocab dilution law, count_into's
    // doc).
    for (id, patterns) in &domains {
        let mut total = 0usize;
        let mut in_vocab = 0usize;
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for p in patterns {
            for tok in tokens_of(p) {
                total += 1;
                if members.contains(tok) {
                    in_vocab += 1;
                    seen.insert(tok);
                }
            }
        }
        #[allow(clippy::cast_precision_loss)]
        {
            println!(
                "COVERAGE {:?} tokens {total} in_vocab {in_vocab} ({:.1}%) distinct_in_vocab {}",
                id,
                if total == 0 {
                    0.0
                } else {
                    in_vocab as f32 / total as f32 * 100.0
                },
                seen.len(),
            );
            println!("   {:?} in-vocab set: {}", id, {
                let mut v: Vec<&str> = seen.iter().copied().collect();
                v.sort_unstable();
                v.join(",")
            });
        }
    }

    // Tokenizer self-check: the contract's edge cases tokenize as documented.
    assert_eq!(tokens_of("let x = 1;"), vec!["let"]);
    assert_eq!(tokens_of("a b cc dd"), vec!["cc", "dd"]);
    assert!(tokens_of("FROM RUN").contains(&"FROM"));
}
