//! T3.2 leak-posture measurement — the adversarial suite over
//! **consumer-grade** code (Proposal 005 Phase 3 T3.2).
//!
//! `tests/kat_redact_g4.rs` is the G4 suite: it proves the gate blocks or
//! byte-exactly redacts planted secrets in spans we are allowed to sync
//! (`own-repo` / `authored-fixture` / `mined-corpus`). This file asks the
//! DIFFERENT question T3.2 actually gates on:
//!
//! > If we admitted [`SpanOrigin::External`] — user-submitted code, the
//! > fixer-node product — would the CONTENT SCANNER be a sufficient
//! > confidentiality boundary on its own?
//!
//! That question matters because today's safety for consumer code is
//! **origin denial: one policy line.** `external` is absent from the
//! shipped allowlist, and [`RedactionGate::check`] short-circuits on origin
//! BEFORE scanning content — so the scanner has never been the load-bearing
//! defense for this class, and no G4 fixture measures it as one. Serving
//! consumer code means promoting the scanner to load-bearing, so the
//! scanner must be measured against consumer-grade inputs first.
//!
//! # What this suite is for
//!
//! It is a MEASUREMENT, not a pass/fail of the gate. It pins:
//!
//! 1. that origin denial holds and short-circuits (the real defense);
//! 2. the measured EVASION SET of the content scanner under an admitted
//!    `external` origin — so closing an evasion is a [`REDACT_VERSION`]
//!    bump + changelog entry, never a silent detection change (the G9b
//!    discipline the gate already declares for its pattern set);
//! 3. the structural result — see below.
//!
//! # The structural result (why this branch of T3.2 cannot close)
//!
//! The scanner is a **secret detector**. For `external` origin the
//! confidential asset is *the span itself*: a customer's proprietary source,
//! their PII, their business logic. A span can carry zero secrets, pass
//! every rule, and still be exactly the thing that must not egress. That is
//! not a tunable gap — no pattern set closes it, because there is no
//! pattern to match. `t32_confidential_but_secretless_code_is_allowed`
//! demonstrates it mechanically.
//!
//! So "vessel sandbox adversarial suite over consumer-grade code" cannot
//! discharge T3.2 by *content scanning*. It can only be discharged by (a)
//! the vendor-side data-usage sign-off (owner act, one vetted vendor, the
//! CF no-training half already verified in writing), or (b) an actual
//! sandbox that never egresses consumer bytes at all. Recorded in
//! `.issues/052`, with the decision in Proposal 005 T3.2.
//!
//! ```bash
//! cargo test --features kat_service --test kat_redact_t32_external -- --nocapture
//! ```
//!
//! NOTE: string literals below are DELIBERATELY secret-shaped — fake values
//! matching published token formats (the gitleaks-fixture convention), and
//! fictional PII on `.example` domains (RFC 6761 reserved).

#![cfg(feature = "kat_service")]

use riir_clippy::kat_redact::{
    scan_secrets, BlockReason, Finding, FindingKind, GateDecision, RedactionGate, SpanOrigin,
    SyncPolicy, REDACT_VERSION,
};

/// The SHIPPED policy shape (same as G4's — `external` absent by design).
fn shipped_policy() -> SyncPolicy {
    SyncPolicy {
        version: REDACT_VERSION,
        sync_enabled: true,
        allow_origins: vec![
            "own-repo".into(),
            "authored-fixture".into(),
            "mined-corpus".into(),
        ],
        allow_paths: vec!["src/**/*.rs".into(), "tests/**/*.rs".into()],
        max_span_bytes: 65_536,
        allow_redacted: false,
    }
}

/// The COUNTERFACTUAL policy: what the fixer-node product would need —
/// `external` admitted, and no path allowlist (consumer code has no path
/// inside our repo, so a path glob cannot gate it). `allow_redacted: true`
/// is the *generous* reading: it lets a span through with findings stripped
/// rather than blocking it, which is the posture a product would want. Every
/// evasion measured below is therefore measured against the gate at its most
/// permissive sanctioned setting.
fn external_admitted_policy() -> SyncPolicy {
    SyncPolicy {
        allow_origins: vec![
            "own-repo".into(),
            "authored-fixture".into(),
            "mined-corpus".into(),
            "external".into(),
        ],
        allow_paths: Vec::new(),
        allow_redacted: true,
        ..shipped_policy()
    }
}

/// The AWS key every encoding fixture below wraps — the canonical AWS
/// documentation example value, caught bare by `scan_prefixed_tokens`.
const AWS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";

/// A second AWS-shaped key whose HEX encoding does clear the hexish entropy
/// threshold (3.404 ≥ 3.0), where [`AWS_KEY`]'s does not (2.976). Same
/// evasion technique, opposite verdict — the pair is the evidence that the
/// hex class is value-dependent, not a stable gap.
const HEX_CAUGHT_KEY: &str = "AKIAA6OZPVNX5M6RLZ4W";

/// Shannon entropy over bytes, in bits/char — the same quantity
/// `kat_redact`'s private `shannon_entropy` computes, reimplemented here so
/// the measurement below can report the margin instead of only the verdict.
fn shannon_bits(s: &str) -> f64 {
    let mut counts = [0u32; 256];
    for b in s.bytes() {
        counts[b as usize] += 1;
    }
    let n = s.len() as f64;
    -counts
        .iter()
        .filter(|c| **c > 0)
        .map(|c| {
            let p = f64::from(*c) / n;
            p * p.log2()
        })
        .sum::<f64>()
}

fn hex_encode(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

/// Minimal base64 (standard alphabet, padded) — avoids a dep for a fixture.
fn b64_encode(s: &str) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let b = s.as_bytes();
    let mut out = String::new();
    for c in b.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (c.get(1).map_or(0, |x| u32::from(*x)) << 8)
            | c.get(2).map_or(0, |x| u32::from(*x));
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { A[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { A[n as usize & 63] as char } else { '=' });
    }
    out
}

/// One adversarial fixture: a named class of consumer-grade evasion attempt.
struct Attack {
    /// Stable id — pinned in the verdict test.
    id: &'static str,
    /// What the class exploits, for the printed report.
    exploits: &'static str,
    span: String,
}

/// The attack corpus. Every span contains, recoverably, the same AWS key
/// (or a live-shaped credential), so "did the scanner catch it" is a fair
/// question for each: a reader of the payload could reconstruct the secret.
fn attack_corpus() -> Vec<Attack> {
    let k = AWS_KEY;
    vec![
        Attack {
            id: "bare-control",
            exploits: "nothing — the positive control; if this evades, the gate is broken",
            span: format!("let key = \"{k}\";\n"),
        },
        Attack {
            id: "base64-wrapped",
            exploits: "documented gap: no layer-stripping; the encoded blob must itself trip entropy",
            span: format!(
                "// decoded at startup\nconst K: &str = \"{}\";\nlet key = decode(K);\n",
                b64_encode(k)
            ),
        },
        Attack {
            id: "hex-wrapped-marginal",
            exploits: "v1 EVADED here (hex entropy 2.976 vs the 3.0 cut — by 0.024 bits); v2 catches it unconditionally",
            span: format!("const K: &str = \"{}\";\n", hex_encode(k)),
        },
        Attack {
            id: "hex-wrapped-highentropy",
            exploits: "SAME class, different key (hex entropy 3.404) — caught in BOTH versions; the pair is why v1's rule was a coin flip",
            span: format!("const K: &str = \"{}\";\n", hex_encode(HEX_CAUGHT_KEY)),
        },
        Attack {
            id: "byte-array",
            exploits: "no long alnum run exists to scan — the value is decimal byte literals",
            span: format!(
                "const K: [u8; 20] = [{}];\n",
                k.bytes().map(|b| b.to_string()).collect::<Vec<_>>().join(", ")
            ),
        },
        Attack {
            id: "concat-split",
            exploits: "documented gap: token-based scan; the prefix never abuts its body",
            span: format!(
                "let key = concat!(\"{}\", \"{}\");\n",
                &k[..4],
                &k[4..]
            ),
        },
        Attack {
            id: "line-split",
            exploits: "documented gap: a secret split across lines evades the env/literal rules",
            span: format!("let key = \"{}\\\n{}\";\n", &k[..10], &k[10..]),
        },
        Attack {
            id: "xor-obfuscated",
            exploits: "arbitrary reversible transform — unbounded class, no pattern can enumerate it",
            span: format!(
                "const K: [u8; 20] = [{}]; // xor 0x5a\n",
                k.bytes().map(|b| (b ^ 0x5a).to_string()).collect::<Vec<_>>().join(", ")
            ),
        },
        Attack {
            id: "reversed",
            exploits: "the prefix rule anchors on a literal prefix; reversal moves it to the tail",
            span: format!("let key: String = \"{}\".chars().rev().collect();\n",
                k.chars().rev().collect::<String>()),
        },
    ]
}

/// Consumer-grade spans that are CONFIDENTIAL but contain no secret — the
/// structural class. Zero findings is the CORRECT behavior of a secret
/// scanner here, which is exactly why a secret scanner is not a
/// confidentiality boundary for this origin.
fn confidential_secretless_corpus() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "customer-pii",
            "// northwind-clinic patient export (internal)\n\
             fn seed() -> Vec<Patient> {\n\
             \x20   vec![\n\
             \x20       Patient { name: \"Maria Alvarez\", dob: \"1978-03-14\", mrn: \"4471902\",\n\
             \x20                 email: \"maria.alvarez@northwind-clinic.example\" },\n\
             \x20       Patient { name: \"Tomas Rendon\", dob: \"1961-11-02\", mrn: \"4471903\",\n\
             \x20                 email: \"t.rendon@northwind-clinic.example\" },\n\
             \x20   ]\n\
             }\n",
        ),
        (
            "trade-secret-logic",
            "// CONFIDENTIAL — pricing model, do not distribute.\n\
             fn quote(seats: u32, tier: Tier, churn_risk: f32) -> Cents {\n\
             \x20   let base = match tier { Tier::A => 4_900, Tier::B => 2_400, Tier::C => 900 };\n\
             \x20   // undercut competitor floor by 3% when churn risk is high\n\
             \x20   let adj = if churn_risk > 0.62 { 0.97 } else { 1.0 };\n\
             \x20   ((base * seats) as f32 * adj) as Cents\n\
             }\n",
        ),
        (
            "internal-topology",
            "// service map — internal hostnames\n\
             const PRIMARY: &str = \"pg-01.core.acme-internal.example:5432\";\n\
             const REPLICA: &str = \"pg-02.core.acme-internal.example:5432\";\n\
             const VAULT: &str = \"vault.core.acme-internal.example:8200\";\n",
        ),
    ]
}

// ── 1. The real defense: origin denial, and it short-circuits ─────────────

/// The load-bearing fact. Under the SHIPPED policy an `external` span is
/// blocked on ORIGIN with **zero findings** — proof that the content scanner
/// did not run at all. Consumer-code safety today is a policy line, not a
/// scan result, so the scanner carries no evidence for this class.
#[test]
fn t32_origin_denial_short_circuits_before_any_content_scan() {
    let gate = RedactionGate::from_policy(shipped_policy());
    // A span that the scanner WOULD flag, so "zero findings" can only mean
    // the scan was skipped — not that there was nothing to find.
    let span = format!("let key = \"{AWS_KEY}\";\n");
    assert!(
        !scan_secrets(&span).is_empty(),
        "fixture must be scanner-detectable for this test to prove anything"
    );

    let d = gate.check(SpanOrigin::External, Some("consumer/src/main.rs"), &span);
    match &d {
        GateDecision::Blocked { reason, findings } => {
            assert_eq!(
                *reason,
                BlockReason::OriginNotAllowed(SpanOrigin::External),
                "external must be denied on ORIGIN, not on content"
            );
            assert!(
                findings.is_empty(),
                "origin denial must short-circuit BEFORE the content scan — \
                 findings present would mean the scan ran: {findings:?}"
            );
        }
        other => panic!("external origin must be blocked, got {other:?}"),
    }

    // And it holds for a span with nothing to find, for the same reason.
    let clean = "fn add(a: u32, b: u32) -> u32 { a + b }\n";
    assert!(
        !gate.check(SpanOrigin::External, None, clean).is_syncable(),
        "origin denial is content-independent"
    );
}

// ── 2. The measurement: the scanner alone, external admitted ─────────────

/// **The T3.2 measurement.** Admit `external`, at the gate's most permissive
/// sanctioned setting (`allow_redacted: true`, no path gate), and record
/// which consumer-grade evasions the content scanner catches.
///
/// The evaded set is PINNED. Closing any of these is a [`REDACT_VERSION`]
/// bump + changelog entry (G9b) — this test is what makes a silent detection
/// change fail loudly, in either direction.
#[test]
fn t32_measured_evasion_set_is_pinned() {
    let gate = RedactionGate::from_policy(external_admitted_policy());
    let mut caught: Vec<&str> = Vec::new();
    let mut evaded: Vec<&str> = Vec::new();

    println!("\n=== T3.2 content-scanner measurement (external ADMITTED) ===");
    println!("{:<18} {:<8} exploits", "attack", "verdict");
    for a in attack_corpus() {
        let d = gate.check(SpanOrigin::External, Some("consumer/src/main.rs"), &a.span);
        let findings: &[Finding] = d.findings();
        // "Caught" = the gate did something about it: blocked, or redacted
        // with at least one finding. "Evaded" = Allow with zero findings,
        // i.e. the span egresses verbatim.
        let hit = !findings.is_empty();
        match hit {
            true => caught.push(a.id),
            false => evaded.push(a.id),
        }
        println!(
            "{:<18} {:<8} {}",
            a.id,
            if hit { "caught" } else { "EVADED" },
            a.exploits
        );
    }
    println!(
        "--- caught {}/{}, evaded {}/{} ---\n",
        caught.len(),
        caught.len() + evaded.len(),
        evaded.len(),
        caught.len() + evaded.len()
    );

    assert!(
        caught.contains(&"bare-control"),
        "the positive control MUST be caught — if not, the gate is broken, \
         not merely evadable"
    );

    // THE PIN. Measured at REDACT_VERSION 1. A change here is a gate
    // definition change: bump REDACT_VERSION + add a REDACT_CHANGELOG row.
    assert_eq!(
        REDACT_VERSION, 2,
        "the evasion set below was measured at REDACT_VERSION 2 — re-measure \
         and update this pin as part of the next version bump"
    );
    assert_eq!(
        evaded,
        vec![
            "base64-wrapped",
            "byte-array",
            "concat-split",
            "line-split",
            "xor-obfuscated",
            "reversed",
        ],
        "the measured evasion set moved. If an evasion was CLOSED that is \
         good news: bump REDACT_VERSION, add the changelog row, update this \
         pin. If a new one OPENED, that is a regression."
    );
}

/// **The hex layer was a coin flip in v1; v2 makes it deterministic — this
/// test is the before/after evidence and the regression guard.**
///
/// v1's module doc said layer-stripping was absent and "a secret encoded
/// then embedded is only caught if the encoded blob itself trips the entropy
/// rule". True, and that conditional did far more work than it looked:
/// whether a hex-wrapped credential tripped the rule depended on THAT
/// PARTICULAR VALUE's hex entropy, which for real AWS-key shapes sits right
/// on the 3.0 cut. Measured over a deterministic sweep (`AKIA` + 16 chars
/// from the base32 alphabet real keys use, BLAKE3-keyed so it reproduces
/// without a rand dep): **1 481/4 000 = 37.0% caught, entropy range
/// 2.349..3.412 against a 3.000 threshold.** A security boundary whose
/// verdict depends on a value the ATTACKER chooses is not a threshold.
///
/// [`REDACT_VERSION`] 2 drops the entropy test on the hexish arm, so the
/// sweep must now be unanimous. Both halves are asserted: the entropy
/// spread still straddles 3.0 (so the v1 finding stays reproducible and the
/// fixture cannot silently stop being adversarial), AND every key is caught
/// regardless.
#[test]
fn t32_hex_layer_is_deterministic_in_v2() {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    const N: usize = 4_000;
    let mut caught = 0usize;
    let mut would_catch_v1 = 0usize;
    let mut min_e = f64::MAX;
    let mut max_e = f64::MIN;

    for i in 0..N {
        // Deterministic AWS-shaped key from blake3(i).
        let h = blake3::hash(&(i as u64).to_le_bytes());
        let body: String = h.as_bytes()[..16]
            .iter()
            .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
            .collect();
        let key = format!("AKIA{body}");
        // Sanity: the BARE key must be detectable, else the sweep measures
        // malformed fixtures rather than the hex layer.
        assert!(
            scan_secrets(&format!("k=\"{key}\"")).iter().any(|f| f.kind == FindingKind::AwsAccessKey),
            "generated key {key} is not AWS-shaped"
        );

        let hex = hex_encode(&key);
        let e = shannon_bits(&hex);
        min_e = min_e.min(e);
        max_e = max_e.max(e);
        if e >= 3.0 {
            would_catch_v1 += 1;
        }
        if scan_secrets(&hex).iter().any(|f| f.kind == FindingKind::HighEntropyHex) {
            caught += 1;
        }
    }

    let v1_pct = 100.0 * would_catch_v1 as f64 / N as f64;
    println!(
        "\n=== T3.2 hex-layer sweep @ REDACT_VERSION {REDACT_VERSION}: \
         v2 caught {caught}/{N}; v1 rule would have caught {would_catch_v1}/{N} \
         ({v1_pct:.1}%); entropy range {min_e:.3}..{max_e:.3} vs the v1 cut 3.000 ===\n"
    );

    assert_eq!(
        caught, N,
        "v2 must catch EVERY hex-wrapped credential — the whole point of \
         dropping the entropy test on the hexish arm"
    );
    assert!(
        min_e < 3.0 && max_e >= 3.0,
        "the fixture must still straddle the v1 threshold, else it has \
         stopped being the adversarial corpus that justified v2 \
         (range {min_e:.3}..{max_e:.3})"
    );
    assert!(
        (0.15..0.85).contains(&(would_catch_v1 as f64 / N as f64)),
        "and the v1 rule must still measure as a coin flip on this corpus \
         — that is the recorded justification for the v2 bump. Got {v1_pct:.1}%"
    );
}

/// The per-span scan is defeated by chunking, independently of any encoding:
/// the AST chunker splits consumer code into sub-cap spans, and a credential
/// straddling a chunk boundary leaves each chunk individually clean. The
/// gate is per-span by construction and has no cross-span state, so this is
/// structural rather than a pattern gap.
#[test]
fn t32_chunk_boundary_defeats_per_span_scanning() {
    let gate = RedactionGate::from_policy(external_admitted_policy());
    let (head, tail) = AWS_KEY.split_at(8);
    let chunk_a = format!("const K_HEAD: &str = \"{head}\";\n");
    let chunk_b = format!("const K_TAIL: &str = \"{tail}\";\n");

    for (name, chunk) in [("head", &chunk_a), ("tail", &chunk_b)] {
        let d = gate.check(SpanOrigin::External, None, chunk);
        assert!(
            d.findings().is_empty(),
            "chunk {name} should be individually clean (that is the point): {d:?}"
        );
        assert!(matches!(d, GateDecision::Allow), "chunk {name}: {d:?}");
    }
    // Reassembled by any consumer of the sync batch, the secret is intact.
    assert!(
        scan_secrets(&format!("{head}{tail}")).iter().any(|f| f.kind == FindingKind::AwsAccessKey),
        "the reassembled value IS the credential the chunks hid"
    );
}

// ── 3. The structural result ──────────────────────────────────────────────

/// **The decisive test for T3.2.** Consumer-grade spans that are plainly
/// confidential — patient PII, a priced trade secret, internal topology —
/// carry ZERO findings and are ALLOWED verbatim.
///
/// This is not a bug in the scanner: a secret detector correctly finds no
/// secret here. It is the reason the "adversarial suite over consumer-grade
/// code" branch of T3.2 cannot discharge the leak posture. For `external`
/// origin the confidential asset is the span itself, and there is no pattern
/// for "this source belongs to someone else". No [`REDACT_VERSION`] bump
/// closes this class, because closing it is not a detection problem.
#[test]
fn t32_confidential_but_secretless_code_is_allowed() {
    let gate = RedactionGate::from_policy(external_admitted_policy());
    for (name, span) in confidential_secretless_corpus() {
        let d = gate.check(SpanOrigin::External, Some("consumer/src/lib.rs"), span);
        assert!(
            d.findings().is_empty(),
            "{name}: a secret scanner is expected to find no SECRET here — \
             if this starts failing, the scanner gained a PII/IP rule and \
             T3.2's analysis needs revisiting: {d:?}"
        );
        assert!(
            matches!(d, GateDecision::Allow),
            "{name}: egresses verbatim under an admitted external origin — \
             this is the structural gap, not a tunable one: {d:?}"
        );
    }
}

/// The corollary that keeps the conclusion actionable: with `external` NOT
/// admitted (the shipped policy), every one of those same confidential spans
/// is denied. The posture that protects consumer code is the origin
/// allowlist, so the operative control to defend in review is that policy
/// line — not the pattern set.
#[test]
fn t32_shipped_policy_denies_every_confidential_span() {
    let gate = RedactionGate::from_policy(shipped_policy());
    for (name, span) in confidential_secretless_corpus() {
        let d = gate.check(SpanOrigin::External, Some("consumer/src/lib.rs"), span);
        assert!(!d.is_syncable(), "{name} must be denied under the shipped policy: {d:?}");
        assert!(
            matches!(
                d,
                GateDecision::Blocked { reason: BlockReason::OriginNotAllowed(_), .. }
            ),
            "{name}: and denied on ORIGIN specifically: {d:?}"
        );
    }
}
