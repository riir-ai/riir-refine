//! Entries slice — Batch 340 (2026-11-06): two `rust_perf` rules
//! from the SPILLWAY distill (Plan 349 / Research 381).
//!
//! Source: saintmachines/spillway @ `0fc9f55f3d5380108be5f2f6680fa07affedfe83`
//! (Apache-2.0; arXiv 2611.07222 "SPILLWAY: Separating Target Admission from
//! Sanitization to Defend Tool-Using Agents against Prompt Injection" —
//! CITATION.cff binds the repo to the paper; push 2026-11-05). The corpus's
//! FIRST agent-egress-guard source. The two rules here are the SCAN-LAW half
//! (the deterministic ingress sanitizer's own linear-time rewrite of the
//! reference gate's quadratic hand-rolled scans); the ADMISSION half (the
//! removed-span veto + trust-inheritance axes) landed in the sec domain the
//! same batch. Every quote re-verified BYTE-EXACT at the pin (T0, the
//! `src/spillway_guard/{sanitizer,floor}.py` read); fixtures are ORIGINAL
//! plain-Rust stand-ins (never pasted source), standalone rustc-compiled AND
//! executed before landing. Batch 340 / Plan 349 / Research 381.

use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── scanning: a closer-existence GATE in front of a two-anchor
        // lazy-wildcard scan ─────────────────────────────────────────────
        // Source: sanitizer.py "Linear-time matching" prose + _paired_matches
        // (the same-Match-objects bit-compatibility contract).
        PerfRule {
            rule_id: "paired-closer-precollect-gates-lazy-wildcard".to_string(),
            name: "A hand-rolled two-anchor scan (OPEN - anything - CLOSE, lazy middle) restarted at every opener pays one end-of-text sweep per UNCLOSED opener - k unclosed openers cost O(k*n) -> precollect the CLOSER positions once (closers are rare), and run the expensive scan only at an opener that has a closer starting at/after its end - the lazy middle then stops at that first closer, so every admitted scan is short and the matches come out identical in order and span".to_string(),
            category: "complexity".to_string(),
            buggy_code: r##"// NAIVE: for a region marker OPEN ..= CLOSE (lazy — stops at the
// FIRST close), the scan restarts at every opener and each opener
// with no closer after it sweeps to the end of the text. An
// adversarial body of k unclosed openers costs O(k * n).

pub struct Region {
    pub start: usize,
    pub end: usize,
}

/// Scan `OPEN<...>CLOSE` regions (lazy middles) by restarting at
/// every opener.
pub fn regions_naive(text: &str, open: &str, close: &str) -> Vec<Region> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(off) = text[from..].find(open) {
        let start = from + off;
        match text[start + open.len()..].find(close) {
            Some(mid) => {
                let end = start + open.len() + mid + close.len();
                out.push(Region { start, end });
                from = end;
            }
            // the unclosed opener: this find swept to END OF TEXT and
            // found nothing — and the next opener restarts the sweep
            None => {
                from = start + open.len();
            }
        }
    }
    out
}"##.to_string(),
            fixed_code: r##"// OPT: the closers are found ONCE (they are rare), the LAST
// closer start is tracked (overlapping closers stepped), and the
// expensive lazy scan runs only at an opener whose end is <= the
// last closer start — A MATCH EXISTS AT AN OPENER EXACTLY WHEN A
// CLOSER STARTS AT OR AFTER THE OPENER'S END. There the lazy scan
// stops at the first closer. Same matches, same order, same spans.

pub struct Region {
    pub start: usize,
    pub end: usize,
}

pub fn regions_gated(text: &str, open: &str, close: &str) -> Vec<Region> {
    // Precollect: every closer start, then the LAST one (a later
    // closer overlapping the last match is stepped by `pos` below,
    // mirroring the naive scan's advance).
    let mut last_close_start: Option<usize> = None;
    {
        let mut from = 0;
        while let Some(off) = text[from..].find(close) {
            let at = from + off;
            last_close_start = Some(at);
            from = at + close.len();
        }
    }
    let Some(last) = last_close_start else {
        return Vec::new(); // no closer anywhere -> no match anywhere
    };
    let mut out = Vec::new();
    let mut pos = 0usize; // matches may not overlap (the naive advance)
    let mut from = 0;
    while let Some(off) = text[from..].find(open) {
        let start = from + off;
        from = start + open.len();
        // THE GATE: a closer starting at/after this opener's end?
        if start >= pos && start + open.len() <= last {
            if let Some(mid) = text[start + open.len()..].find(close) {
                let end = start + open.len() + mid + close.len();
                out.push(Region { start, end });
                pos = end;
            }
        }
    }
    out
}"##.to_string(),
            context_notes: Some(r##"MEASURED at the standalone fixture (m3 / Mac15,8, release, 2026-11-06, AC power, high power mode, nothing else compiling — box state per the law): the honest unclosed-opener adversary is a TAIL shape (all closers first, then k openers no closer follows — a lazy middle finds the NEXT closer wherever it sits, so interleaving closers after unclosed openers fakes the adversary; the fixture's first draft did exactly that and was caught by its own census). Ladder (64 matched regions + k unclosed, median best-of-7 alternating): k=128/n=8.6K naive 131 us vs gated 21 us = 6.3x; k=512/n=28K 1.36 ms vs 29 us = 48x; k=2048/n=106K 18.6 ms vs 154 us = 121x; k=8192/n=420K 529 ms vs 320 us = 1655x — the naive arm scales k*n, the gate arm stays ~n. The fixture also pins the CENSUS (deterministic integers, not wall clock): exactly one FAILED end-of-text sweep per unclosed opener, at most one admitted scan per matched region. DOMAIN PRECISION (in-rule, load-bearing): the `regex` crate is linear-time by construction and its literal prefilters make crate-driven scans fast — this law is for HAND-ROLLED scan loops (find() chains, custom parsers, state machines); wrapping a crate-driven scan in the gate adds the precollect pass for nothing. CORRECTNESS CONTRACT (the source's own bar, carried in-rule): 'the same matches: the Match objects of the same compiled patterns, so the same spans and groups, in the same order' — the gated scan must be proven span-and-order identical on the adversarial AND the benign inputs (the fixture asserts both); the non-overlap advance is part of the contract, stepped identically in both arms; the last-closer tracking must step OVERLAPPING closers (a closer start inside the previous closer's own extent is still a candidate gate bound — the source scans them per-position for exactly this). ANTI-REGIMES: every opener closed (the gate never refuses an opener — pure overhead); closers as dense as openers (the precollect pass is O(n) over a dense needle — no win to buy); the middle is anchored/greedy to a FIXED endpoint (a middle that must reach a known end is not shortened by the first-closer stop — the gate's premise dies). KINSHIPS (cite, never merge): `prefilter-choice-shape-ladder` owns matcher-SHAPE selection at build time (memchr/Finder/refuse shapes) — THIS owns a closer-existence GATE over an existing two-anchor shape; `byte-find-position-to-memchr` + `self-loop-state-memchr-skip` own single-needle scan mechanics (the precollect's inner loop rides those); `alternation-prefix-collapse-at-build` owns literal-set prefix collapsing — a different axis. DEDUP ADJUDICATION (T1 re-run at the batch slot): closer.precollect | closer.exist | open.close.pair | two.anchor.gate | unclosed.opener over the rust_perf + kernel_opt corpora → zero owners. Batch 340 / Plan 349 / Research 381 (saintmachines/spillway @ `0fc9f55f`, Apache-2.0 — sanitizer.py:152-208 quoted byte-exact at the pin; the fixture is an original synthesis, standalone-compiled AND executed — `corpus/fixtures/b340_fixtures.rs` F1 tests)."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None, // NotBenchable: the win is census-shaped (the fixtures carry the probe counts); a temp-crate bench adds no axis the fixtures lack
        },
        // ── scanning: a rare-delimiter ANCHOR in front of a
        // per-start-position token scan ─────────────────────────────────
        // Source: floor.py `_email_matches` prose (the EMAIL_RE local-part
        // run analysis) — the '@'-per-run anchor.
        PerfRule {
            rule_id: "rare-delimiter-anchored-token-scan".to_string(),
            name: "A token scan whose match REQUIRES a rare delimiter (the '@' in an address, a scheme marker, a path separator) restarted per start position re-sweeps a delimiter-free run once per run character — O(run^2) adversarial -> iterate the DELIMITER positions once and try the match only at the run head that ends at each delimiter (every start in the run would meet the same delimiter and the same tail, so ONE probe per delimiter subsumes the run) — O(n + matches*len), same tokens in the same order".to_string(),
            category: "complexity".to_string(),
            buggy_code: r##"// NAIVE: token extraction in finditer shape — try candidate
// starts left to right; after an EMIT, resume after the match;
// after a FAIL, advance ONE character. A delimiter-free run of
// length R costs O(R^2): every start in the run sweeps the rest
// of the run before failing, and no emit ever shortens the walk.

/// Extract `local@domain` tokens (non-overlapping, finditer shape).
pub fn tokens_naive(text: &str) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let is_local =
        |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'%' | b'+' | b'-');
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < bytes.len() {
        if !is_local(bytes[start]) {
            start += 1;
            continue;
        }
        // sweep the local-part run to its end looking for '@'
        let mut i = start;
        while i < bytes.len() && is_local(bytes[i]) {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'@' {
            let dom_start = i + 1;
            let dom_len = text[dom_start..]
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
                .unwrap_or(text.len() - dom_start);
            let end = dom_start + dom_len;
            out.push((start, text[start..end].to_string()));
            start = end; // the engine resumes after the match (no overlap)
        } else {
            // no '@' after this run: the sweep was wasted, and the
            // NEXT start of the same run re-sweeps it — the quadratic
            start += 1;
        }
    }
    out
}"##.to_string(),
            fixed_code: r##"// OPT: the delimiter positions are walked ONCE (a memchr-class
// find over the whole body), and the match is tried only at the
// FIRST character of the run that ends at each delimiter — every
// start in that run would meet the same '@' and the same domain,
// so one probe per delimiter subsumes the run. Non-run characters
// are never probed; delimiter-free text costs ONE find pass.

pub fn tokens_anchored(text: &str) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let is_local =
        |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'%' | b'+' | b'-');
    let mut out = Vec::new();
    let mut pos = 0usize; // first byte not consumed by an emitted token
    let mut from = 0usize;
    while let Some(off) = text[from..].find('@') {
        let at = from + off;
        from = at + 1;
        if at < pos {
            continue; // inside the previous token's extent
        }
        // walk BACK to the run head: the ONLY candidate start whose
        // probe this delimiter answers (the run's earlier starts
        // share the same tail, so the head subsumes them all)
        let run_start = text[..at]
            .bytes()
            .rposition(|b| !is_local(b))
            .map_or(0, |p| p + 1)
            .max(pos);
        if run_start >= at {
            continue; // empty local part — no token ends at this '@'
        }
        let dom_start = at + 1;
        let dom_len = text[dom_start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
            .unwrap_or(text.len() - dom_start);
        let end = dom_start + dom_len;
        out.push((run_start, text[run_start..end].to_string()));
        pos = end;
    }
    out
}"##.to_string(),
            context_notes: Some(r##"MEASURED at the standalone fixture (m3 / Mac15,8, release, 2026-11-06, same provenance posture as the paired rule above): the two honest poles (fixture-measured, m3 release, 2026-11-06, AC/high-power, nothing else compiling): the RUN pole — one delimiter-free local-part run beside real addresses, the baseline walks the run once per candidate start (the R^2/2 census): r=240K naive ~155 s vs anchored ~3 ms ≈ 5e4x (asserted once at r=96K — naive 25.5 s vs anchored 1.4 ms ≈ 1.8e4x — a 7-sample median would cost minutes; the census equivalence is what the fixture pins); the DENSE pole (the ANTI-REGIME, printed not asserted): delimiters as dense as run characters — the anchor DEGENERATES to the baseline shape and the backward run-head walk is extra work, measured 0.75-0.81x — anchor only where the delimiter is RARE. Outputs token-identical by the fixture asserts on all classes (same tokens, same order, same spans — the baseline is the finditer shape, resume-after-match). DOMAIN PRECISION (in-rule): the `regex` crate's literal prefilter makes EMAIL_RE-class patterns fast — this law is for HAND-ROLLED byte scans (validators, redactors, log screeners); the rulewright's own secret-scan lane is the dogfood instance (per-start rescans over ASCII-token candidate spans are this shape — disclosed as context, never a self-fix in the same batch). CORRECTNESS CONTRACT (carried in-rule): same tokens, same order, same spans — the run head is the FIRST start that can produce this delimiter's token (later starts in the run would emit the SAME token and are suppressed by the consumed-extent advance exactly as the finditer baseline's resume-after-match dictates); a run head whose probe FAILS must NOT advance `pos` (the baseline still retries later starts — only EMITTED tokens consume extent). ANTI-REGIMES (MEASURED at the dense pole, the fixture prints it): delimiters as dense as run characters — the anchor degenerates to the baseline and the run-head walk is pure extra work (0.75-0.81x, a small REGRESSION); token classes with NO rare delimiter (nothing to anchor on — use the class-run family); multi-delimiter classes where different delimiters admit different run heads (one anchor pass per delimiter class, never one shared walk). KINSHIPS (cite, never merge): `ascii-run-swar-block-skip` (B243) owns byte-CLASS run skipping — the same runs, a different axis (class purity, not delimiter anchoring; the two COMPOSE: the class skip walks pure runs, the anchor picks the head); `prefilter-skip-density-demotion` owns runtime density latching of a CANDIDATE PROBE (this rule's probe is already density-cheap — one per delimiter); `byte-find-position-to-memchr` owns the delimiter find itself. DEDUP ADJUDICATION (T1 re-run at the batch slot): delimiter.anchor | run.head | per.start.rescan | at.sign | delimiter.free.run over the rust_perf + kernel_opt corpora → zero owners. Batch 340 / Plan 349 / Research 381 (saintmachines/spillway @ `0fc9f55f`, Apache-2.0 — floor.py:66-84 quoted byte-exact at the pin; the fixture is an original synthesis, standalone-compiled AND executed — `corpus/fixtures/b340_fixtures.rs` F2 tests)."##.to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None, // NotBenchable: the win is census-shaped (the fixtures carry the probe counts); a temp-crate bench adds no axis the fixtures lack
        },
    ]
}
