//! Issue 232 T2 — the half-split agreement readout (Research 495 C2, the
//! gradient-free analog of T³'s `⟨∇L_A, ∇L_Q⟩`; source: dang2026t3 via
//! `riir-train/.research/495`).
//!
//! **The reading (adjudicated at design time):** T³ splits its training
//! corpus into halves A and Q and reads the agreement between the updates
//! each half induces. The gradient-free analog at the corpus-is-the-model
//! surface keeps the corpus's two data views and asks whether they ANSWER
//! THE SAME WAY: the A half = each rule's TRIGGER exemplar
//! ([`HealEntry::buggy_code`]), the Q half = each rule's RESOLUTION
//! exemplar ([`HealEntry::fixed_code`]); one scorer per view over a shared
//! neutral probe set; pick-agreement = both views name the same rule. An
//! entry-level random rule split was REFUSED: two scorers over disjoint
//! rule sets cannot agree on a rule id BY CONSTRUCTION (pick-agreement a
//! constant zero, flip rate a constant one — a dead readout), so the
//! corpus's A/Q split is realized as the corpus's two text surfaces, which
//! keeps every axis live. T³'s L2 lesson is the reason the cross-view
//! signal is the interesting one: a probe confirmed by the same surface
//! that produced it is self-confirming (own-half testing degraded every
//! epoch in the paper); here the probe must be confirmed by BOTH views to
//! count as agreed.
//!
//! **Class cousin (the signal-diff, per the issue):** query-by-committee /
//! ensemble disagreement measures independently-BUILT predictors. Here one
//! predictor class reads two corpus views, so disagreement localizes the
//! PROBE (a span whose nearest knowledge differs between its trigger and
//! resolution neighborhoods — view-ambiguity), never predictor diversity.
//!
//! **The L1 regime gate is NOT here.** This lane reports; T3's
//! multiplicity-gated arming predicate owns any future arming decision,
//! and the correlation cell (this signal vs later measured holdout
//! deltas, on repeated-consultation vs 1-pass surfaces) is the issue's
//! next gate. No T³ constant is imported; no decision threshold exists in
//! this module — every readout value is continuous (rates + a mean gap),
//! and nothing in the readout refuses, demotes, or gates anything.
//!
//! **Calibration by construction:** both views share the embedder and the
//! score scale, so the top-1 confidence gap is same-scale comparable with
//! no transform (and no softmax anywhere — the house law is sigmoid, and
//! only a future ARMING consumer would need one).
//!
//! **Determinism (the Issue-940 family):** the corpus walks in canonical
//! rule-id order; every reduction is a sequential fold; the top-1 tie rule
//! is ONE fixed strict rule (equal score → the lexicographically smaller
//! rule id wins); the readout digest is BLAKE3 over a canonical byte
//! encoding (f32 scores fold as their exact bit patterns). Same corpus +
//! same probes → byte-identical report, every machine — the G1 pin.
//!
//! ## Feature posture
//!
//! `half_split_agreement` (opt-in, default-OFF, no implies — std + blake3
//! only). Feature-off is byte-identical by construction (new gated code
//! only, no shared-path edit). The CLI lane is `--half-split-report
//! [PATHS…]`; a build without the feature refuses exit 2 (the corpus-gap
//! shape). Report-only first per the issue's proof-path ledger; promotion
//! talk waits for the correlation cell.

use crate::domain::HealEntry;
use crate::span_embed::{SPAN_EMBED_DIMS, span_embed};
use std::path::{Path, PathBuf};

/// The readout's schema version — folded into the digest, so a law change
/// moves every pin loud.
pub const READOUT_VERSION: u64 = 1;

/// Probe-span granularity for the CLI lane: one probe = a fixed run of
/// source lines (window = stride, so windows partition the file — no
/// line is scored twice). A granularity constant, never a decision
/// threshold.
pub const PROBE_WINDOW_LINES: usize = 12;

/// How many per-probe rows the CLI report prints before the `+K more`
/// fold (the whole set always rides the digest — the fold is a display
/// cap, never a readout cap).
const ROWS_PRINTED: usize = 20;

/// One probe: a stable id (the CLI uses `path:start-end`) + the text both
/// views score.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeSpan {
    pub id: String,
    pub text: String,
}

/// One probe's two-view verdict. A `None` pick happens only when a view
/// scored no entry (an empty view) — it is reported as such, never a
/// refusal (the T1 verdict law).
#[derive(Debug, Clone, PartialEq)]
pub struct SpanVerdict {
    pub probe: String,
    pub pick_a: Option<String>,
    pub score_a: f32,
    pub pick_q: Option<String>,
    pub score_q: f32,
    pub agree: bool,
}

/// The whole readout. `digest` = BLAKE3 over the canonical encoding
/// ([`digest_bytes`]) — the G1 determinism pin.
#[derive(Debug, Clone, PartialEq)]
pub struct AgreementReport {
    pub readout_version: u64,
    pub entries: usize,
    pub probes: usize,
    pub agreements: usize,
    pub flips: usize,
    /// Mean over probes of |top1(A) − top1(Q)| — the same-scale
    /// confidence-divergence axis (calibration by construction; no
    /// transform). Sequential f32 fold, pinned order.
    pub mean_conf_gap: f32,
    /// In probe order (the caller's), never re-sorted.
    pub rows: Vec<SpanVerdict>,
    pub digest: String,
}

/// Exact cosine over the quantized span embeddings: integer dot/norms (i32
/// — 32 dims × 127² fits trivially), ONE f32 division at the end. A zero
/// vector (empty/whitespace text) scores 0.0 against everything — the
/// honest no-signal score, never a panic. The exact-integer shape is the
/// determinism law, so the lossy casts below are pinned as deliberate:
/// every intermediate is exact in i32, and the final division is the ONLY
/// rounding step (the fixture's exact-1.0 self-match pin sits on it).
#[allow(clippy::cast_precision_loss)]
fn cosine(a: &[i8; SPAN_EMBED_DIMS], b: &[i8; SPAN_EMBED_DIMS]) -> f32 {
    let mut dot: i32 = 0;
    let mut na: i32 = 0;
    let mut nb: i32 = 0;
    for i in 0..SPAN_EMBED_DIMS {
        let (x, y) = (i32::from(a[i]), i32::from(b[i]));
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0 || nb == 0 {
        return 0.0;
    }
    dot as f32 / ((na as f32).sqrt() * (nb as f32).sqrt())
}

/// One view's canonical form: rule id + embedding. The rule-id order is a
/// DETERMINISM input only — the top-1 tie rule makes the picks
/// order-independent.
type View = Vec<(String, [i8; SPAN_EMBED_DIMS])>;

/// The top-1 pick over one view: max cosine; ONE strict tie rule — equal
/// score → the lexicographically smaller rule id (the Issue-940
/// first-index law in its id form). The returned id borrows from
/// `view`. Exact f32 equality is the tie rule (the scores are
/// integer-exact quotients; `float_cmp` deliberate).
#[allow(clippy::float_cmp)]
fn top1<'a>(view: &'a View, probe: &[i8; SPAN_EMBED_DIMS]) -> Option<(&'a str, f32)> {
    let mut best: Option<(&'a str, f32)> = None;
    for (id, vec) in view {
        let s = cosine(probe, vec);
        let take = match best {
            None => true,
            Some((bid, bs)) => s > bs || (s == bs && id.as_str() < bid),
        };
        if take {
            best = Some((id.as_str(), s));
        }
    }
    best
}

/// The readout itself: build both views, score every probe through both,
/// aggregate. Pure over its inputs — same corpus + probes → the same
/// report, byte for byte. The exact-integer scoring stage is the
/// determinism law; the two display-scale casts carry deliberate allows
/// (issue-940 `float_cmp` + `cast_precision_loss`), NOT silences.
#[allow(clippy::cast_precision_loss, clippy::float_cmp)]
#[must_use]
pub fn agreement_readout(entries: &[&dyn HealEntry], probes: &[ProbeSpan]) -> AgreementReport {
    let mut canon: Vec<(String, [i8; SPAN_EMBED_DIMS], [i8; SPAN_EMBED_DIMS])> = entries
        .iter()
        .map(|e| {
            (
                e.rule_id().to_string(),
                span_embed(e.buggy_code()),
                span_embed(e.fixed_code()),
            )
        })
        .collect();
    canon.sort_by(|a, b| a.0.cmp(&b.0));
    let view_a: View = canon
        .iter()
        .map(|(id, buggy, _)| (id.clone(), *buggy))
        .collect();
    let view_q: View = canon
        .iter()
        .map(|(id, _, fixed)| (id.clone(), *fixed))
        .collect();

    let mut rows = Vec::with_capacity(probes.len());
    let mut gap_acc = 0.0_f32;
    for p in probes {
        let pe = span_embed(&p.text);
        let (pick_a, score_a) =
            top1(&view_a, &pe).map_or((None, 0.0), |(id, s)| (Some(id.to_string()), s));
        let (pick_q, score_q) =
            top1(&view_q, &pe).map_or((None, 0.0), |(id, s)| (Some(id.to_string()), s));
        // Both picks are Some exactly when both views hold entries; the
        // None-vs-None arm is a correct false (two empty views cannot
        // agree on a rule — the honest verdict).
        #[allow(clippy::single_match_else)]
        let agree = match (&pick_a, &pick_q) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        };
        gap_acc += (score_a - score_q).abs(); // exact same-scale f32s (Issue-940; float_cmp deliberate)
        rows.push(SpanVerdict {
            probe: p.id.clone(),
            pick_a,
            score_a,
            pick_q,
            score_q,
            agree,
        });
    }

    let agreements = rows.iter().filter(|r| r.agree).count();
    let flips = rows.len() - agreements;
    let mean_conf_gap = if probes.is_empty() {
        0.0
    } else {
        gap_acc / probes.len() as f32
    };
    let mut rep = AgreementReport {
        readout_version: READOUT_VERSION,
        entries: canon.len(),
        probes: rows.len(),
        agreements,
        flips,
        mean_conf_gap,
        rows,
        digest: String::new(),
    };
    rep.digest = blake3::hash(&digest_bytes(&rep)).to_hex().to_string();
    rep
}

fn put_str(b: &mut Vec<u8>, s: &str) {
    b.extend_from_slice(s.as_bytes());
    b.push(0);
}

fn put_opt_str(b: &mut Vec<u8>, s: Option<&str>) {
    match s {
        Some(v) => {
            b.push(1);
            put_str(b, v);
        }
        None => b.push(0),
    }
}

/// The digest's canonical byte encoding: a fixed header + version +
/// counts, then each row (probe id, both picks with a presence byte, both
/// scores as exact f32 bit patterns, the agree flag), then the
/// aggregates. Anything that moves the readout moves the digest.
fn digest_bytes(rep: &AgreementReport) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"i232-t2-half-split\0");
    b.extend_from_slice(&rep.readout_version.to_le_bytes());
    b.extend_from_slice(&(rep.entries as u64).to_le_bytes());
    b.extend_from_slice(&(rep.probes as u64).to_le_bytes());
    for r in &rep.rows {
        put_str(&mut b, &r.probe);
        put_opt_str(&mut b, r.pick_a.as_deref());
        b.extend_from_slice(&r.score_a.to_le_bytes());
        put_opt_str(&mut b, r.pick_q.as_deref());
        b.extend_from_slice(&r.score_q.to_le_bytes());
        b.push(u8::from(r.agree));
    }
    b.extend_from_slice(&(rep.agreements as u64).to_le_bytes());
    b.extend_from_slice(&(rep.flips as u64).to_le_bytes());
    b.extend_from_slice(&rep.mean_conf_gap.to_le_bytes());
    b
}

/// The plain-text report. The digest line is LAST (machine-greppable).
/// Display caps rows at [`ROWS_PRINTED`] — the full set always rides the
/// digest. The verdict law shows here too: the text reports what it saw
/// and never refuses a probe. The `push_str`/`format` shape is deliberate
/// (clippy's `format_push_string` preference would rebuild the same
/// string); the allow rides the function.
#[allow(clippy::format_push_string)]
#[must_use]
pub fn render(rep: &AgreementReport) -> String {
    let mut out = String::new();
    out.push_str("half-split agreement readout (i232 T2 — report-only; T3 owns any arming)\n");
    out.push_str(&format!(
        "corpus: {} rule(s) — views: trigger A = buggy_code, resolution Q = fixed_code\n",
        rep.entries
    ));
    out.push_str(&format!("probes: {}\n", rep.probes));
    let rate = if rep.probes == 0 {
        0.0
    } else {
        #[allow(clippy::cast_precision_loss)] // display rounding only
        {
            100.0 * rep.agreements as f32 / rep.probes as f32
        }
    };
    out.push_str(&format!(
        "pick-agreement: {}/{} ({rate:.1}%)\n",
        rep.agreements, rep.probes
    ));
    out.push_str(&format!("flips: {}/{}\n", rep.flips, rep.probes));
    out.push_str(&format!(
        "mean |top1 gap| (A vs Q): {:.4}\n",
        rep.mean_conf_gap
    ));
    if !rep.rows.is_empty() {
        out.push_str("\nprobe                            A -> Q         verdict\n");
        for r in rep.rows.iter().take(ROWS_PRINTED) {
            let dash = "—";
            let a = r.pick_a.as_deref().unwrap_or(dash);
            let q = r.pick_q.as_deref().unwrap_or(dash);
            let verdict = if r.agree { "agree" } else { "flip" };
            out.push_str(&format!("{:<32} {:<14} {:<14} {verdict}\n", r.probe, a, q));
        }
        let hidden = rep.rows.len().saturating_sub(ROWS_PRINTED);
        if hidden > 0 {
            out.push_str(&format!("  ... +{hidden} more row(s) (in the digest)\n"));
        }
    }
    out.push_str(&format!("digest: blake3 {}\n", rep.digest));
    out
}

// ── the CLI lane ─────────────────────────────────────────────────────────
//
// `--half-split-report [PATHS…]`: probe spans = fixed line windows over the
// given paths' `.rs` files (a directory walks recursively, sorted; symlinked
// directories are not followed). Report-first, gates nothing (the
// frontier-report law); exits 0 on a real read and BLINDNESS refusals only
// — no compiled domain, or zero probe spans, is an empty read and the lane
// declines it rather than print a confident green zero (the empty-walk
// law).

fn collect_rs_files(paths: &[PathBuf], out: &mut Vec<PathBuf>, skipped: &mut usize, depth: usize) {
    if depth > 16 {
        *skipped += 1;
        return;
    }
    for p in paths {
        if p.is_dir() {
            let mut children: Vec<PathBuf> = Vec::new();
            match std::fs::read_dir(p) {
                Ok(rd) => {
                    children = rd
                        .filter_map(std::result::Result::ok)
                        .map(|e| e.path())
                        .collect()
                }
                Err(_) => *skipped += 1,
            }
            children.sort();
            for c in children {
                if c.is_dir() {
                    // Symlinked directories are not followed (a cycle or a
                    // mount point must never turn a report into a walk of
                    // the machine).
                    if !c.is_symlink() {
                        collect_rs_files(std::slice::from_ref(&c), out, skipped, depth + 1);
                    }
                } else if c.extension().is_some_and(|e| e == "rs") {
                    out.push(c);
                }
            }
        } else if p.is_file() && p.extension().is_some_and(|e| e == "rs") {
            out.push(p.clone());
        } else {
            *skipped += 1;
        }
    }
}

/// Probe spans from one file: [`PROBE_WINDOW_LINES`]-line windows (window =
/// stride), whitespace-only windows skipped. The id is `path:start-end`
/// with 1-based inclusive line numbers. An unreadable file yields no
/// probes (the caller's skipped count is the disclosure).
fn probe_windows(path: &Path, text: &str) -> Vec<ProbeSpan> {
    let shown = path.display().to_string();
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut start = 0;
    while start < lines.len() {
        let end = (start + PROBE_WINDOW_LINES).min(lines.len());
        let window = lines[start..end].join("\n");
        if !window.trim().is_empty() {
            out.push(ProbeSpan {
                id: format!("{shown}:{}-{}", start + 1, end),
                text: window,
            });
        }
        start += PROBE_WINDOW_LINES;
    }
    out
}

/// The CLI lane over real paths: enumerate the compiled domains' corpora
/// (one view pair per rule), walk the paths for probe windows, run the
/// readout, render. `Err` = the blindness-refusal message (the caller
/// exits 1 with it); every other outcome prints.
///
/// # Errors
/// The blindness class only: no compiled healing domain, or zero probe
/// spans under the given paths (the empty-walk law — a confident green
/// zero is the failure mode this lane refuses to print).
pub fn cli_report(paths: &[PathBuf]) -> Result<String, String> {
    let mut all: Vec<&dyn HealEntry> = Vec::new();
    // Each corpus binding lives at fn scope so the `&dyn HealEntry` refs
    // in `all` outlive every block (borrowck: a cfg block would drop its
    // binding while `all` still borrows it).
    #[cfg(feature = "clippy_lints")]
    let c_clippy = crate::build_seed_corpus();
    #[cfg(feature = "clippy_lints")]
    for e in &c_clippy.entries {
        all.push(e);
    }
    #[cfg(feature = "rust_perf")]
    let c_perf = crate::build_perf_seed_corpus();
    #[cfg(feature = "rust_perf")]
    for e in &c_perf.entries {
        all.push(e);
    }
    #[cfg(feature = "kernel_opt")]
    let c_kernel = crate::build_kernel_seed_corpus();
    #[cfg(feature = "kernel_opt")]
    for e in &c_kernel.entries {
        all.push(e);
    }
    #[cfg(feature = "sec")]
    let c_sec = crate::build_sec_seed_corpus();
    #[cfg(feature = "sec")]
    for e in &c_sec.entries {
        all.push(e);
    }
    #[cfg(feature = "docker")]
    let c_docker = crate::build_docker_seed_corpus();
    #[cfg(feature = "docker")]
    for e in &c_docker.entries {
        all.push(e);
    }
    #[cfg(feature = "dist")]
    let c_dist = crate::build_dist_seed_corpus();
    #[cfg(feature = "dist")]
    for e in &c_dist.entries {
        all.push(e);
    }
    if all.is_empty() {
        return Err(
            "half-split-report: no healing domain compiled into this build — rebuild with at least one domain feature"
                .to_string(),
        );
    }

    let mut files = Vec::new();
    let mut skipped = 0_usize;
    collect_rs_files(paths, &mut files, &mut skipped, 0);
    files.sort();
    let mut probes = Vec::new();
    for f in &files {
        match std::fs::read_to_string(f) {
            Ok(text) => probes.extend(probe_windows(f, &text)),
            Err(_) => skipped += 1,
        }
    }
    if probes.is_empty() {
        return Err(format!(
            "half-split-report: no probe spans under {} path(s) — nothing to read",
            paths.len()
        ));
    }

    let rep = agreement_readout(&all, &probes);
    let mut text = render(&rep);
    if skipped > 0 {
        use std::fmt::Write as _;
        writeln!(text, "skipped: {skipped} unreadable/non-Rust path(s)").ok();
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct E(&'static str, &'static str, &'static str);
    impl HealEntry for E {
        fn rule_id(&self) -> &str {
            self.0
        }
        fn buggy_code(&self) -> &str {
            self.1
        }
        fn fixed_code(&self) -> &str {
            self.2
        }
    }

    fn corpus() -> Vec<Box<dyn HealEntry>> {
        vec![
            Box::new(E(
                "charge_refund",
                "fn charge_refund(amount_cents: u32) -> u32 { let total = amount_cents; return total; }",
                "fn charge_refund(amount_cents: u32) -> u32 { amount_cents }",
            )),
            Box::new(E(
                "iterator_push",
                "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { let mut out = Vec::new(); for page in pages { out.push(page); } out }",
                "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { pages.into_iter().collect() }",
            )),
            Box::new(E(
                "read_exact",
                "fn slurp_config(mut handle: File, buf: &mut [u8; 64]) -> usize { handle.read(buf).unwrap() }",
                "fn slurp_config(mut handle: File, buf: &mut [u8; 64]) -> usize { handle.read_exact(buf).unwrap() }",
            )),
        ]
    }

    fn refs(c: &[Box<dyn HealEntry>]) -> Vec<&dyn HealEntry> {
        c.iter().map(|b| b.as_ref() as &dyn HealEntry).collect()
    }

    #[test]
    fn self_match_pins_the_view_mapping() {
        // Engagement proof first: the A view IS buggy_code, the Q view IS
        // fixed_code. A probe that IS an entry's buggy text must score
        // exactly 1.0 against that entry ON THE A VIEW (identical i8
        // vectors → integer-exact cosine 1.0, the strict maximum).
        let c = corpus();
        let all = refs(&c);
        let buggy_text = "fn charge_refund(amount_cents: u32) -> u32 { let total = amount_cents; return total; }";
        let fixed_text = "fn charge_refund(amount_cents: u32) -> u32 { amount_cents }";
        let probes = vec![
            ProbeSpan {
                id: "p_buggy".into(),
                text: buggy_text.into(),
            },
            ProbeSpan {
                id: "p_fixed".into(),
                text: fixed_text.into(),
            },
        ];
        let rep = agreement_readout(&all, &probes);
        let rb = &rep.rows[0];
        assert_eq!(rb.pick_a.as_deref(), Some("charge_refund"));
        assert_eq!(rb.score_a, 1.0, "identical text is integer-exact 1.0");
        let rf = &rep.rows[1];
        assert_eq!(rf.pick_q.as_deref(), Some("charge_refund"));
        assert_eq!(rf.score_q, 1.0);
    }

    #[test]
    fn both_views_agree_on_a_full_text_concat_probe() {
        // A probe carrying BOTH of iterator_push's surfaces verbatim must
        // be named by both views — the agreement axis live.
        let c = corpus();
        let all = refs(&c);
        let text = "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { let mut out = Vec::new(); for page in pages { out.push(page); } out }\nfn collect_pages(pages: Vec<u16>) -> Vec<u16> { pages.into_iter().collect() }";
        let probes = vec![ProbeSpan {
            id: "p_agree".into(),
            text: text.into(),
        }];
        let rep = agreement_readout(&all, &probes);
        let r = &rep.rows[0];
        assert_eq!(r.pick_a.as_deref(), Some("iterator_push"));
        assert_eq!(r.pick_q.as_deref(), Some("iterator_push"));
        assert!(r.agree);
    }

    #[test]
    fn crossed_views_flip_on_a_crossed_concat_probe() {
        // A probe carrying iterator_push's trigger surface + read_exact's
        // resolution surface (both substantial — the same engineered probe
        // as the G1 fixture): each view's exact-half match beats the
        // cross-view cousin, and the views disagree — the flip axis live.
        let c = corpus();
        let all = refs(&c);
        let text = "fn collect_pages(pages: Vec<u16>) -> Vec<u16> { let mut out = Vec::new(); for page in pages { out.push(page); } out }\nfn slurp_config(mut handle: File, buf: &mut [u8; 64]) -> usize { handle.read_exact(buf).unwrap() }";
        let probes = vec![ProbeSpan {
            id: "p_flip".into(),
            text: text.into(),
        }];
        let rep = agreement_readout(&all, &probes);
        let r = &rep.rows[0];
        assert_eq!(r.pick_a.as_deref(), Some("iterator_push"));
        assert_eq!(r.pick_q.as_deref(), Some("read_exact"));
        assert!(!r.agree);
        assert_eq!(rep.flips, 1);
        assert_eq!(rep.agreements, 0);
    }

    #[test]
    fn empty_probe_is_an_honest_tie_row_never_a_refusal() {
        // The zero vector scores 0.0 against every entry; both views take
        // the same strict tie winner (the lexicographically first rule id)
        // and the row reports agree — reported, never declined.
        let c = corpus();
        let all = refs(&c);
        let probes = vec![
            ProbeSpan {
                id: "p_empty".into(),
                text: String::new(),
            },
            ProbeSpan {
                id: "p_ws".into(),
                text: "   \n\t  ".into(),
            },
        ];
        let rep = agreement_readout(&all, &probes);
        for r in &rep.rows {
            assert_eq!(r.score_a, 0.0);
            assert_eq!(r.score_q, 0.0);
            assert_eq!(r.pick_a, r.pick_q, "both views take the same tie winner");
            assert!(r.agree);
        }
    }

    #[test]
    fn deterministic_across_corpus_input_order() {
        let c = corpus();
        let all = refs(&c);
        let probes = vec![
            ProbeSpan {
                id: "a".into(),
                text: "fn f() -> u32 { let t = 1; return t; }".into(),
            },
            ProbeSpan {
                id: "b".into(),
                text: "for x in it { out.push(g(x)); }".into(),
            },
            ProbeSpan {
                id: "c".into(),
                text: "let n = r.read(&mut buf)?;".into(),
            },
        ];
        let r1 = agreement_readout(&all, &probes);
        let r2 = agreement_readout(&all, &probes);
        assert_eq!(r1, r2, "same corpus + probes → byte-identical report");
        // The corpus-walk order must not matter: the readout sorts
        // internally (canonical rule-id order).
        let mut rev = corpus();
        rev.reverse();
        let rall = refs(&rev);
        let r3 = agreement_readout(&rall, &probes);
        assert_eq!(r1, r3, "canonical order makes input order irrelevant");
    }

    #[test]
    fn render_carries_no_refusal_vocabulary() {
        let c = corpus();
        let all = refs(&c);
        let probes = vec![ProbeSpan {
            id: "p".into(),
            text: "fn f() {}".into(),
        }];
        let rep = agreement_readout(&all, &probes);
        let text = render(&rep).to_ascii_lowercase();
        for w in ["refuse", "reject", "deny", "refusal"] {
            assert!(!text.contains(w), "the report must never say {w}");
        }
        assert!(text.contains("digest: blake3 "));
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn probe_windows_partition_and_skip_blank_runs() {
        let p = Path::new("f.rs");
        let text = "a\n\n\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn";
        let probes = probe_windows(p, text);
        // 16 lines / 12-line windows = 2 probes; window 2 starts at line 13.
        assert_eq!(probes.len(), 2);
        assert!(probes[0].id.starts_with("f.rs:1-12"));
        assert!(probes[1].id.starts_with("f.rs:13-16"));
        // All-blank files yield no probes at all.
        assert_eq!(probe_windows(p, "\n\n \n\t\n").len(), 0);
    }
}
