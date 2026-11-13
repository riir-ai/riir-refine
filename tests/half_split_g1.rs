//! Issue 232 T2 G1 — the half-split agreement readout's pre-registered
//! fixture (`required-features = ["half_split_agreement"]`; the Cargo.toml
//! row protects the reader, the pinned asserts below protect the count —
//! a feature-less run skips the target SILENTLY, which is why every pin
//! here is an exact equality, never a `>=`).
//!
//! Pins: the digest for the fixture corpus + probe set (the G1
//! determinism pin — re-pin ONLY with the reason recorded in the landing
//! note), the view-mapping self-match law, the agree/flip axes, the
//! honest tie row, corpus-input-order independence, the refusal-vocabulary
//! absence in the rendered report, and the CLI lane end-to-end (probe
//! windows over a temp tree + its blindness refusals).

use riir_refine::domain::HealEntry;
use riir_refine::half_split::{ProbeSpan, agreement_readout, cli_report, render};

struct FixtureEntry {
    id: &'static str,
    buggy: &'static str,
    fixed: &'static str,
}

impl HealEntry for FixtureEntry {
    fn rule_id(&self) -> &str {
        self.id
    }
    fn buggy_code(&self) -> &str {
        self.buggy
    }
    fn fixed_code(&self) -> &str {
        self.fixed
    }
}

/// Three rules with deliberately distant vocabularies so the engineered
/// probes below name exactly one entry per view.
fn corpus() -> Vec<Box<dyn HealEntry>> {
    vec![
        Box::new(FixtureEntry {
            id: "charge_refund",
            buggy: "fn charge_refund(amount_cents: u32) -> u32 { let total = amount_cents; return total; }",
            fixed: "fn charge_refund(amount_cents: u32) -> u32 { amount_cents }",
        }),
        Box::new(FixtureEntry {
            id: "iterator_push",
            buggy: "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { let mut out = Vec::new(); for page in pages { out.push(page); } out }",
            fixed: "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { pages.into_iter().collect() }",
        }),
        Box::new(FixtureEntry {
            id: "read_exact",
            buggy: "fn slurp_config(mut handle: File, buf: &mut [u8; 64]) -> usize { handle.read(buf).unwrap() }",
            fixed: "fn slurp_config(mut handle: File, buf: &mut [u8; 64]) -> usize { handle.read_exact(buf).unwrap() }",
        }),
    ]
}

fn refs(c: &[Box<dyn HealEntry>]) -> Vec<&dyn HealEntry> {
    c.iter().map(|b| b.as_ref() as &dyn HealEntry).collect()
}

fn fixture_probes() -> Vec<ProbeSpan> {
    vec![
        // Self-matches: each side's own surface names its rule at exactly
        // 1.0 ON ITS OWN VIEW (engagement proof — the view mapping pin).
        ProbeSpan {
            id: "p_buggy_self".into(),
            text: "fn charge_refund(amount_cents: u32) -> u32 { let total = amount_cents; return total; }".into(),
        },
        ProbeSpan {
            id: "p_fixed_self".into(),
            text: "fn charge_refund(amount_cents: u32) -> u32 { amount_cents }".into(),
        },
        // Agreement: BOTH surfaces of iterator_push, concatenated — both
        // views name iterator_push.
        ProbeSpan {
            id: "p_agree".into(),
            text: "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { let mut out = Vec::new(); for page in pages { out.push(page); } out }\nfn collect_pages(pages: Vec<u16>) -> Vec<u16> { pages.into_iter().collect() }".into(),
        },
        // Flip: iterator_push's trigger surface + read_exact's resolution
        // surface — both substantial, so the probe needs no amplification:
        // each view's exact-half match (cos ≈ 1/√2) must beat the
        // cross-view cousin (read_exact's buggy/fixed differ by ONE token,
        // so the resolution view's cousin score is the close call this
        // probe prices). The pinned outcome below is the oracle.
        ProbeSpan {
            id: "p_flip".into(),
            text: "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { let mut out = Vec::new(); for page in pages { out.push(page); } out }\nfn slurp_config(mut handle: File, buf: &mut [u8; 64]) -> usize { handle.read_exact(buf).unwrap() }".into(),
        },
        // The honest tie row: the zero vector scores 0.0 everywhere; both
        // views take the same lexicographic tie winner — reported, never
        // declined.
        ProbeSpan { id: "p_empty".into(), text: String::new() },
    ]
}

#[test]
fn g1_digest_is_pinned() {
    let c = corpus();
    let all = refs(&c);
    let probes = fixture_probes();
    let rep = agreement_readout(&all, &probes);
    // Determinism first (cheap, always): two runs, byte-identical.
    let again = agreement_readout(&all, &probes);
    assert_eq!(rep, again, "same corpus + probes → byte-identical report");
    // Corpus-input order must not matter (canonical rule-id walk).
    let mut rev = corpus();
    rev.reverse();
    let rall = refs(&rev);
    assert_eq!(
        rep.digest,
        agreement_readout(&rall, &probes).digest,
        "canonical order makes input order irrelevant"
    );
    // THE PIN: the exact digest for THIS fixture corpus + probe set at
    // readout version 1. Any embedder/scorer/tie/digest law change moves
    // this loud — re-pin only with the reason recorded in the landing
    // note (the issue's proof-path ledger carries the re-pin history).
    assert_eq!(
        rep.digest, "df6ea53e82c87ccdb63aa02db18e7ea73f72b96c79de8865fec026037835c004",
        "digest pin — re-pin ONLY with the reason recorded in the landing note"
    );
}

#[test]
fn g1_view_mapping_and_axes_hold() {
    let c = corpus();
    let all = refs(&c);
    let probes = fixture_probes();
    let rep = agreement_readout(&all, &probes);
    assert_eq!(rep.entries, 3);
    assert_eq!(rep.probes, 5);
    let rows: Vec<_> = rep.rows.iter().map(|r| (r.probe.as_str(), r)).collect();
    let by_id = |name: &str| rows.iter().find(|(p, _)| *p == name).map(|(_, r)| r);

    // View mapping (engagement proof): buggy text → 1.0 on the A view;
    // fixed text → 1.0 on the Q view.
    let b = by_id("p_buggy_self").expect("row present");
    assert_eq!(b.pick_a.as_deref(), Some("charge_refund"));
    assert_eq!(b.score_a, 1.0);
    let f = by_id("p_fixed_self").expect("row present");
    assert_eq!(f.pick_q.as_deref(), Some("charge_refund"));
    assert_eq!(f.score_q, 1.0);

    // Agreement axis: both views name iterator_push on the concat probe.
    let a = by_id("p_agree").expect("row present");
    assert_eq!(a.pick_a.as_deref(), Some("iterator_push"));
    assert_eq!(a.pick_q.as_deref(), Some("iterator_push"));
    assert!(a.agree);

    // Flip axis: the crossed concat probe splits the views.
    let fl = by_id("p_flip").expect("row present");
    assert_eq!(fl.pick_a.as_deref(), Some("iterator_push"));
    assert_eq!(fl.pick_q.as_deref(), Some("read_exact"));
    assert!(!fl.agree);

    // The tie row: honest 0.0 scores, the same winner on both views.
    let e = by_id("p_empty").expect("row present");
    assert_eq!(e.score_a, 0.0);
    assert_eq!(e.score_q, 0.0);
    assert_eq!(e.pick_a, e.pick_q);
    assert!(e.agree);

    // Aggregates: exactly one engineered flip; the tie row agrees; the
    // self-match probes are NOT pinned to agree (their other view is
    // whatever the embedder says — only their own view is the pin).
    assert_eq!(rep.flips, 1);
}

#[test]
fn g1_report_prints_no_refusal_vocabulary() {
    let c = corpus();
    let all = refs(&c);
    let rep = agreement_readout(&all, &fixture_probes());
    let text = render(&rep).to_ascii_lowercase();
    for w in ["refuse", "reject", "deny", "refusal"] {
        assert!(!text.contains(w), "the report must never say {w}");
    }
    assert!(text.contains("digest: blake3 "));
}

#[test]
fn g1_cli_lane_reports_and_refuses_blind_reads() {
    // The shared_temp_path law: a per-process temp root — a concurrent
    // process running this same test must never share the directory.
    let root = std::env::temp_dir().join(format!("half_split_g1_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("temp root created");

    // One .rs file whose whole content (exactly 12 lines — ONE window) is
    // iterator_push's full vocabulary repeated, so the single window
    // carries pure signal on BOTH views (the agreement axis shows on the
    // real lane deterministically). A second file crosses the window
    // boundary (13 lines = 2 windows) with crossed vocabulary.
    let buggy_line = "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { let mut out = Vec::new(); for page in pages { out.push(page); } out }";
    let fixed_line =
        "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { pages.into_iter().collect() }";
    let mut file_a = String::new();
    for i in 0..6 {
        file_a.push_str(if i % 2 == 0 { buggy_line } else { fixed_line });
        file_a.push('\n');
    }
    std::fs::write(root.join("agree.rs"), &file_a).expect("fixture written");

    let mut file_b = String::new();
    file_b.push_str("fn slurp_config(mut handle: File, buf: &mut [u8; 64]) -> usize { handle.read(buf).unwrap() }\n");
    file_b.push_str("fn charge_refund(amount_cents: u32) -> u32 { amount_cents }\n");
    while file_b.lines().count() < 13 {
        file_b.push_str("// filler line\n");
    }
    std::fs::write(root.join("flip.rs"), &file_b).expect("fixture written");

    // A non-.rs file: interior non-Rust files are the walk's ordinary
    // skip (a repo tree has thousands — counting each would be noise);
    // the loud `skipped` count is for top-level NAMED targets that are
    // not readable Rust. The report succeeds regardless of the decoy.
    std::fs::write(root.join("notes.txt"), "not rust").expect("decoy written");

    let paths = vec![root.clone()];
    let r1 = cli_report(&paths).expect("a real read succeeds");
    assert!(r1.contains("half-split agreement readout"));
    assert!(r1.contains("pick-agreement:"));
    assert!(r1.contains("digest: blake3 "));
    // The decoy is silently interior-skipped; nothing in the report names it.
    assert!(!r1.contains("notes.txt"));
    // Engagement evidence on the real lane (the compiled corpus here is
    // the FULL product corpus — rule identity is the fixture lane's pin,
    // never the CLI lane's): every expected window row is present and the
    // readout is content-sensitive — new bytes in the tree move the
    // digest.
    assert!(
        r1.contains("agree.rs:1-6"),
        "the agree window is probed: {r1}"
    );
    assert!(r1.contains("flip.rs:1-12"), "window 1 of the 13-line file");
    assert!(
        r1.contains("flip.rs:13-13"),
        "window 2 crosses the boundary"
    );
    // Determinism: same tree, same digest (paths are stable within one
    // run — the id is path-based).
    let r2 = cli_report(&paths).expect("second read succeeds");
    let d1 = r1
        .lines()
        .find(|l| l.starts_with("digest: "))
        .expect("digest line");
    let d2 = r2
        .lines()
        .find(|l| l.starts_with("digest: "))
        .expect("digest line");
    assert_eq!(d1, d2, "same tree → same digest");
    // Content sensitivity: new bytes in the tree move the digest.
    std::fs::write(
        root.join("agree.rs"),
        "fn totally_different_signature(x: u128) -> bool { x == 0 }",
    )
    .expect("content changed");
    let r3 = cli_report(&paths).expect("third read succeeds");
    let d3 = r3
        .lines()
        .find(|l| l.starts_with("digest: "))
        .expect("digest line");
    assert_ne!(d1, d3, "content sensitivity: new bytes move the digest");

    // Blindness refusal: an empty directory is an empty read.
    let empty = root.join("empty_dir");
    std::fs::create_dir_all(&empty).expect("empty dir");
    let err = cli_report(&[empty]).expect_err("no .rs files → blind");
    assert!(err.contains("no probe spans"), "got: {err}");

    // Cleanup (best effort — the temp root is ours by name).
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn g1_cli_lane_refuses_a_pathless_read() {
    // Nothing under a nonexistent path → the probe-blind refusal, never
    // a green zero.
    let nowhere = std::env::temp_dir().join(format!("half_split_g1_none_{}", std::process::id()));
    let err = cli_report(&[nowhere]).expect_err("nothing under a nonexistent path → blind");
    assert!(err.contains("no probe spans"), "got: {err}");
}
