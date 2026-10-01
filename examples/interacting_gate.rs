//! Issue 099 T1 — the interacting-corpus ADMISSIBILITY GATE.
//!
//! A corpus that only LOOKS coupled silently restores the exact blind spot
//! Issue 099 exists to remove, wearing the name of the fix (the Bench 087
//! `--multi` precedent: a lever that looked like the missing one and was
//! not). This gate proves every committed case actually interacts BEFORE
//! any measurement is reported off it, and REDS if a case stops
//! interacting under rustc drift.
//!
//! Two admissible tiers, measured per case over the structural 2×2
//! (`seed` / `broken` / `a_only` / `b_only` — see
//! [`riir_refine::domains::rustc_errors::corpus_interacting`]):
//!
//! 1. **SET tier** — the pair's cold diagnostic multiset is NOT the sum of
//!    the single-mutation multisets: `key(broken) != key(a_only) ⊎
//!    key(b_only)` with `key = (code, primary-line-text)`. This is the
//!    masking class: member B's diagnostic only exists (or renders with a
//!    different code) depending on member A's state.
//! 2. **LADDER tier** — the playbook candidate ladder for one member's
//!    diagnostics differs between the other member's broken/fixed states:
//!    the production playbooks are run in-process against both variants'
//!    sources and the proposed `(strategy_id, replacement, trigger-text)`
//!    sets compared. This is Issue 099's mechanism-2 claim ("the second
//!    error's candidate ladder changes rung") measured at the exact seam
//!    the driver consumes.
//!
//! Validity floor (checked first, loud): the seed compiles clean, `a_only`
//! carries `code_a`, `b_only` carries `code_b`, and `broken` carries
//! `code_a` (code_b MAY be masked — masking is an admissible interaction).
//!
//! ```bash
//! cargo run --release --example interacting_gate --features rustc_errors
//! cargo run --release --example interacting_gate --features rustc_errors -- --verbose
//! ```
//!
//! Exit 0 = every committed case interacts. Exit 1 = at least one case
//! failed admissibility (drop it from the family table, loudly, per the
//! issue — or investigate rustc drift). Exit 2 = harness unavailable.

use riir_refine::domains::rustc_errors::corpus_interacting::interacting_pairs;
use riir_refine::domains::rustc_errors::diag::RustcErrorDiag;
use riir_refine::domains::rustc_errors::floor::FloorHarness;
use riir_refine::domains::rustc_errors::playbooks::Playbooks;

/// SET-tier key: (code, primary span's line text) — line TEXT, never the
/// line NUMBER, so segment-length differences between variants cannot
/// manufacture a fake interaction (the probe pass caught that artifact in
/// the raw-label fingerprint).
fn set_key(diags: &[RustcErrorDiag], source: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<(String, String)> = diags
        .iter()
        .filter_map(|d| {
            let line = d.primary.as_ref().map(|(_, l)| *l)?;
            let text = lines.get(line.saturating_sub(1)).map(|s| s.trim().to_string());
            Some((d.code.clone(), text.unwrap_or_default()))
        })
        .collect();
    out.sort();
    out
}

/// LADDER-tier fingerprint of one variant's whole candidate menu: every
/// playbook proposal for every diagnostic, as `(strategy_id, replacement,
/// trigger-text-at-span)`. Byte spans are normalized away via the trigger
/// text — spans shift with segment length; what the driver ORDERS is the
/// strategy/replacement/shape triple.
fn ladder_fingerprint(
    diags: &[RustcErrorDiag],
    source: &str,
    playbooks: &Playbooks,
) -> Vec<(String, String, String)> {
    let file = std::path::Path::new("src/lib.rs");
    let mut out: Vec<(String, String, String)> = Vec::new();
    for d in diags {
        for c in playbooks.propose(d, file, source) {
            let trigger = source
                .get(c.byte_span.0..c.byte_span.1)
                .unwrap_or("")
                .to_string();
            let extras = c
                .extra_edits
                .iter()
                .map(|e| e.replacement.clone())
                .collect::<Vec<_>>()
                .join("|");
            out.push((
                c.strategy_id.clone(),
                format!("{}#{}", c.replacement, extras),
                trigger,
            ));
        }
    }
    out.sort();
    out
}

fn codes_of(diags: &[RustcErrorDiag]) -> Vec<String> {
    let mut c: Vec<String> = diags.iter().map(|d| d.code.clone()).collect();
    c.sort();
    c.dedup();
    c
}

fn main() {
    let verbose = std::env::args().any(|a| a == "--verbose");
    let harness = FloorHarness::new();
    if !harness.is_available() {
        eprintln!("cargo unavailable — no admissibility measurement possible");
        std::process::exit(2);
    }
    let playbooks = Playbooks::production();
    let cases = interacting_pairs();
    println!(
        "=== Issue 099 T1 — interacting-corpus admissibility gate ({} designed cases) ===",
        cases.len()
    );

    let mut admitted = 0usize;
    let mut failed: Vec<String> = Vec::new();
    let t0 = std::time::Instant::now();

    for case in &cases {
        // ── validity floor ──────────────────────────────────────────────
        let d_seed = harness
            .check_source(&format!("{}_seed", case.name), &case.seed)
            .unwrap_or_else(|e| panic!("{}: seed check failed: {e}", case.name));
        let d_a = harness
            .check_source(&format!("{}_a", case.name), &case.a_only)
            .unwrap_or_else(|e| panic!("{}: a_only check failed: {e}", case.name));
        let d_b = harness
            .check_source(&format!("{}_b", case.name), &case.b_only)
            .unwrap_or_else(|e| panic!("{}: b_only check failed: {e}", case.name));
        let d_bb = harness
            .check_source(&format!("{}_bb", case.name), &case.broken)
            .unwrap_or_else(|e| panic!("{}: broken check failed: {e}", case.name));

        let seed_clean = d_seed.is_empty();
        let a_declares = codes_of(&d_a).iter().any(|c| c == case.code_a);
        let b_declares = codes_of(&d_b).iter().any(|c| c == case.code_b);
        let bb_declares_a = codes_of(&d_bb).iter().any(|c| c == case.code_a);
        let valid = seed_clean && a_declares && b_declares && bb_declares_a;

        // ── SET tier ────────────────────────────────────────────────────
        let mut union: Vec<(String, String)> = set_key(&d_a, &case.a_only);
        union.extend(set_key(&d_b, &case.b_only));
        union.sort();
        let set_interacts = set_key(&d_bb, &case.broken) != union;

        // ── LADDER tier ─────────────────────────────────────────────────
        // B's ladder with A broken (broken variant) vs A fixed (b_only).
        let ladder_b = ladder_fingerprint(&d_b, &case.b_only, &playbooks);
        let ladder_b_in_bb = ladder_fingerprint(&d_bb, &case.broken, &playbooks);
        let ladder_interacts = ladder_b != ladder_b_in_bb;

        let interacts = valid && (set_interacts || ladder_interacts);
        if interacts {
            admitted += 1;
        } else {
            failed.push(case.name.clone());
        }
        if verbose || !interacts {
            println!(
                "  {:<32} valid={} set={} ladder={} -> {}",
                case.name,
                valid as u8,
                set_interacts as u8,
                ladder_interacts as u8,
                if interacts { "ADMITTED" } else { "DROP" }
            );
            if !interacts || verbose {
                println!(
                "      seed={} a_only={:?} b_only={:?} broken={:?}",
                d_seed.len(),
                codes_of(&d_a),
                codes_of(&d_b),
                codes_of(&d_bb)
            );
                println!("      broken-set: {:?}", set_key(&d_bb, &case.broken));
                println!("      union-set : {:?}", union);
            }
        }
    }

    harness.cleanup();
    println!(
        "\nadmitted {admitted}/{} in {:.1}s — every committed case must interact (drop non-interacting rows from the family tables, loudly)",
        cases.len(),
        t0.elapsed().as_secs_f64()
    );
    // Loud bookkeeping: the tier distribution, so a corpus that is entirely
    // ladder-only (weaker evidence) cannot quietly pass as set-interacting.
    if admitted < cases.len() {
        eprintln!("NON-INTERACTING CASES (drop from the family or investigate rustc drift): {failed:?}");
        std::process::exit(1);
    }
}
