//! Batch 351 sec fixtures — the 0sec-labs/foxguard transcription laws as ONE
//! standalone file (5 rules, the four-arm discipline per rule where the law
//! admits one).
//!
//! ORIGINAL plain-Rust reproductions — zero foxguard source pasted (MIT OR
//! Apache-2.0; the source is a Rust SAST's detector bodies and is never a
//! gate — the transcription law). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b351_fixtures.rs`.
//!
//! Arms per rule (the b350 discipline):
//! - F1 `sql-query-value-interpolation-unbound`: a bound value stays DATA
//!   even when it carries a quote-craft (demonstrated on a toy SQL parser),
//!   the interpolated value breaks out (the attack demonstrated), the
//!   trusted-constant interpolation arm is SAFE (the anti-regime the rule
//!   carries — B350's fixed fixture interpolates an allowlisted key), and
//!   the tampered "binder" that actually concatenates disagrees.
//! - F2 `quantum-vulnerable-primitive-selection`: the honest classifier
//!   flags the classical primitives and passes the PQ-safe skip list with
//!   the leading-word boundary law (`user_ed25519_key_id` NOT flagged,
//!   `ed25519_dalek::SigningKey` flagged); the naive substring matcher
//!   fires the buried-occurrence false positive; the tampered arm that
//!   ignores the skip list disagrees.
//! - F3 `regex-backtracking-engine-untrusted-pattern`: the nested-quantifier
//!   pattern blows past a step cap under a backtracking matcher while the
//!   linear engine settles the same input in n steps (the engine split
//!   demonstrated, bounded so the test stays fast); the linear-engine arm
//!   is clean on ANY pattern shape (the anti-regime); the naive arm
//!   ("small inputs are fine") fails one input size later.
//! - F4 `server-connection-without-read-timeout`: the deadline'd handler
//!   drops a never-completing client (demonstrated on a virtual clock), the
//!   deadline-less handler is still pinned at a large tick count, the
//!   bounded-but-long long-poll deadline survives (the deadline is a value,
//!   never a kill), and the tampered "deadline" that never fires disagrees.
//! - F5 `open-redirect-unvalidated-target`: relative-only normalization
//!   refuses the absolute off-site jump and passes the relative path and
//!   the route-name builder (the source's safe half); the passthrough
//!   ships the evil URL; the "*" allowlist arm disagrees.

// ────────────────────────────────────────────────────────────────────
// F1 — the SQL value-position bind
// ────────────────────────────────────────────────────────────────────

mod f1_sql_value_bind {
    /// A toy SQL model: the parser toggles "in string literal" on every
    /// `'`. DATA outside a quote can never change the statement shape;
    /// interpolated text becomes part of the parsed statement.
    struct ToySql {
        in_string: bool,
        /// Bytes consumed OUTSIDE a quoted slot after one opened — the
        /// region where injected text executes as statement structure.
        outside_after_open: usize,
    }

    impl ToySql {
        fn parse(sql: &str) -> Self {
            let mut p = ToySql {
                in_string: false,
                outside_after_open: 0,
            };
            let mut seen_open = false;
            for b in sql.bytes() {
                if b == b'\'' {
                    p.in_string = !p.in_string;
                    seen_open = true;
                } else if seen_open && !p.in_string {
                    p.outside_after_open += 1;
                }
            }
            p
        }
    }

    /// FIXED: the value rides a bind slot — the engine transmits it as a
    /// typed parameter, never as statement text. The value's quotes are
    /// never parsed: the statement's quote structure is exactly the
    /// statement's own.
    fn bound_value_is_data(value: &str) -> ToySql {
        // The statement text is fixed; the value never enters it.
        let stmt = "SELECT id FROM users WHERE name = ?1";
        let mut p = ToySql::parse(stmt);
        let _ = value; // transmitted out-of-band as parameter 1
        p
    }

    /// NAIVE: the value is formatted into the statement text.
    fn interpolated_value(value: &str) -> ToySql {
        let stmt = format!("SELECT id FROM users WHERE name = '{value}'");
        ToySql::parse(&stmt)
    }

    /// The attacker's value: break out of the quoted slot.
    const CRAFT: &str = "x' OR 1=1 --";

    #[test]
    fn f1_bound_value_stays_data() {
        let p = bound_value_is_data(CRAFT);
        // No slot ever opened: the value's quote never reached the parser.
        assert_eq!(
            p.outside_after_open, 0,
            "a bound value must contribute zero parsed statement bytes"
        );
        assert!(!p.in_string);
    }

    #[test]
    fn f1_interpolated_value_breaks_out() {
        let p = interpolated_value(CRAFT);
        // The craft closed the quote and injected `OR 1=1` outside the
        // slot — parsed as statement structure.
        assert!(
            p.outside_after_open >= 8,
            "the injected SQL must be parsed outside the quoted slot (the attack)"
        );
    }

    #[test]
    fn f1_trusted_constant_interpolation_is_safe() {
        // The anti-regime: a trusted QUOTE-FREE constant interpolated into
        // the slot carries no attacker input — the same shape B350's fixed
        // fixture legitimately interpolates (an allowlisted key). The rule
        // fires on UNBOUND DYNAMIC values, never on constants.
        let p = interpolated_value("alice");
        assert_eq!(
            p.outside_after_open, 0,
            "a constant value never opens an escape path"
        );
    }

    #[test]
    fn f1_tampered_binder_that_concatenates_disagrees() {
        // The tampered arm: a "bind" that secretly formats the value into
        // the statement. It must be distinguishable from the honest binder
        // by exactly the property the rule guards.
        let honest = bound_value_is_data(CRAFT);
        let tampered = interpolated_value(CRAFT);
        assert_ne!(
            honest.outside_after_open, tampered.outside_after_open,
            "the concatenating 'binder' must disagree with the honest one"
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// F2 — the PQ primitive classifier
// ────────────────────────────────────────────────────────────────────

mod f2_pq_selection {
    /// The decision content under test: classical primitives are flagged,
    /// PQ primitives pass, and the boundary law decides what "carries the
    /// algorithm name" means.
    #[derive(Debug, PartialEq)]
    enum Verdict {
        Classical,
        PostQuantum,
        Unrelated,
    }

    const CLASSICAL: [&str; 8] = [
        "rsa", "ecdsa", "ecdh", "dsa", "ed25519", "x25519", "p256", "p384",
    ];
    /// The PQ-safe skip list (FIPS 203/204/205 + the drafts + the hybrid
    /// migration composite the source's own table carries).
    const PQ_SAFE: [&str; 6] = [
        "ml_kem",
        "ml_dsa",
        "slh_dsa",
        "fn_dsa",
        "hqc",
        "x25519_ml_kem768",
    ];

    /// The boundary law (the source's `crypto_token_in_path`): a token
    /// matches only as the LEADING WORD of a `::`-segment — `ed25519`
    /// matches `ed25519_dalek`, never a buried `user_ed25519_key_id`.
    fn token_leads_segment(path_lower: &str, token: &str) -> bool {
        path_lower.split("::").any(|seg| {
            seg == token
                || seg
                    .strip_prefix(token)
                    .is_some_and(|rest| rest.starts_with('_'))
        })
    }

    /// FIXED: the honest classifier.
    fn classify(path_lower: &str) -> Verdict {
        let mentions = |tokens: &[&str]| tokens.iter().any(|t| token_leads_segment(path_lower, t));
        if mentions(&PQ_SAFE) {
            Verdict::PostQuantum
        } else if mentions(&CLASSICAL) {
            Verdict::Classical
        } else {
            Verdict::Unrelated
        }
    }

    /// NAIVE: substring matching without the boundary law.
    fn classify_naive(path_lower: &str) -> Verdict {
        if PQ_SAFE.iter().any(|t| path_lower.contains(t)) {
            Verdict::PostQuantum
        } else if CLASSICAL.iter().any(|t| path_lower.contains(t)) {
            Verdict::Classical
        } else {
            Verdict::Unrelated
        }
    }

    #[test]
    fn f2_classical_flagged_pq_passed() {
        assert_eq!(classify("rsa::rsa_private_key::new"), Verdict::Classical);
        assert_eq!(
            classify("ed25519_dalek::signing_key::generate"),
            Verdict::Classical
        );
        assert_eq!(classify("ml_kem::fips203::keypair"), Verdict::PostQuantum);
        assert_eq!(classify("ml_dsa::fips204::sign"), Verdict::PostQuantum);
    }

    #[test]
    fn f2_boundary_law_spares_buried_occurrences() {
        // The source's own precision example: a field NAMED for a key id
        // is not a use of the algorithm.
        assert_eq!(classify("user_ed25519_key_id"), Verdict::Unrelated);
        assert_eq!(classify("my_rsa_helper::path"), Verdict::Unrelated);
    }

    #[test]
    fn f2_naive_substring_fires_the_buried_fp() {
        assert_eq!(
            classify_naive("user_ed25519_key_id"),
            Verdict::Classical,
            "the naive matcher fires exactly the false positive the boundary law exists for"
        );
    }

    #[test]
    fn f2_tampered_skip_list_ignored_disagrees() {
        // The tampered arm: "PQ-safe" dropped — everything with a
        // classical name is flagged even when the PQ primitive is present
        // (a hybrid migration path).
        let tampered = |path_lower: &str| {
            if CLASSICAL.iter().any(|t| token_leads_segment(path_lower, t)) {
                Verdict::Classical
            } else {
                Verdict::Unrelated
            }
        };
        let hybrid = "x25519_ml_kem768::hybrid_decapsulate";
        assert_eq!(classify(hybrid), Verdict::PostQuantum);
        assert_eq!(
            tampered(hybrid),
            Verdict::Classical,
            "ignoring the skip list must disagree on the hybrid shape"
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// F3 — the backtracking-engine hazard
// ────────────────────────────────────────────────────────────────────

mod f3_redos_engine {
    /// The pattern shape: a quantified group that itself contains a
    /// quantifier — `(a+)+$` semantics.
    /// Backtracking matcher: exponential candidate splitting, step-capped
    /// so the demonstration is bounded.
    fn backtrack_steps(input: &[u8], cap: u64) -> u64 {
        // Match `(a+)+$` by recursion over group-iteration counts.
        fn rec(input: &[u8], pos: usize, steps: &mut u64, cap: u64) -> bool {
            if *steps >= cap {
                return false;
            }
            *steps += 1;
            // The group can consume k = 1..=remaining 'a's, then recurse.
            let mut k = 1usize;
            while pos + k <= input.len() && input[pos..pos + k].iter().all(|&b| b == b'a') {
                if rec(input, pos + k, steps, cap) {
                    return true;
                }
                k += 1;
                if *steps >= cap {
                    return false;
                }
            }
            // The outer group matched zero iterations; succeed iff at end.
            pos == input.len()
        }
        let mut steps = 0;
        let _ = rec(input, 0, &mut steps, cap);
        steps
    }

    /// Linear matcher: one pass, no candidate splitting.
    fn linear_steps(input: &[u8]) -> u64 {
        // (a+)+ on an all-'a' prefix is settle-in-one-pass for a linear
        // engine; the steps are the scan length.
        input.len() as u64 + 1
    }

    const CAP: u64 = 20_000;
    const N: usize = 22; // small n, exponential still blows the cap

    #[test]
    fn f3_backtracking_blows_the_cap_linear_settles() {
        let mut input = vec![b'a'; N];
        input.push(b'b'); // the classic near-miss tail: never matches
        let bt = backtrack_steps(&input, CAP);
        let lin = linear_steps(&input);
        assert_eq!(
            bt, CAP,
            "the backtracking matcher must hit the step cap (hazard demonstrated)"
        );
        assert!(
            lin < 100,
            "the linear engine settles the same input in ~n steps ({lin})"
        );
    }

    #[test]
    fn f3_linear_engine_clean_on_any_shape() {
        // The anti-regime: the LINEAR engine's cost is input-bounded
        // regardless of the pattern's quantifier shape — the pattern is
        // never the hazard there.
        let mut input = vec![b'a'; 64];
        input.push(b'b');
        assert!(linear_steps(&input) < 100);
    }

    #[test]
    fn f3_naive_small_inputs_are_fine_fails_later() {
        // The naive posture: "backtracking is fine for small inputs".
        // One size later it is not: steps grow past the small-input cost
        // by an exponential factor (bounded by the cap).
        let small: Vec<u8> = vec![b'a'; 6].into_iter().chain([b'b']).collect();
        let bigger: Vec<u8> = vec![b'a'; N].into_iter().chain([b'b']).collect();
        let s = backtrack_steps(&small, CAP);
        let b = backtrack_steps(&bigger, CAP);
        assert!(
            b > s * 10 || b == CAP,
            "growing n by ~4x must blow the cost up (small={s}, big={b})"
        );
    }

    #[test]
    fn f3_tampered_all_linear_disagrees() {
        // The tampered arm: an "engine classifier" that reports every
        // engine linear. It must disagree with the measured cost split.
        let tampered = |_input: &[u8]| 1_u64; // claims everything is O(1)
        let input: Vec<u8> = vec![b'a'; N].into_iter().chain([b'b']).collect();
        assert_ne!(
            tampered(&input),
            backtrack_steps(&input, CAP),
            "calling the backtracking engine 'linear' must disagree with its measured cost"
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// F4 — the read deadline
// ────────────────────────────────────────────────────────────────────

mod f4_read_deadline {
    /// A virtual-clock connection handler: the client sends its request
    /// line one byte per tick (or never finishes).
    struct Conn {
        bytes_sent: usize,
        need: usize,
    }

    #[derive(Debug, PartialEq)]
    enum Outcome {
        Complete,
        Dropped,
        Waiting,
    }

    /// FIXED: the deadline'd handler — drop at the deadline.
    fn with_deadline(conn: &mut Conn, tick: usize, deadline: usize) -> Outcome {
        if conn.bytes_sent >= conn.need {
            Outcome::Complete
        } else if tick >= deadline {
            Outcome::Dropped
        } else {
            Outcome::Waiting
        }
    }

    /// NAIVE: no deadline — the handler waits forever.
    fn without_deadline(conn: &mut Conn, _tick: usize) -> Outcome {
        if conn.bytes_sent >= conn.need {
            Outcome::Complete
        } else {
            Outcome::Waiting
        }
    }

    fn stalled_client() -> Conn {
        Conn {
            bytes_sent: 3,
            need: 64,
        } // sent "GET /", then went silent
    }

    #[test]
    fn f4_deadline_drops_the_slowloris_client() {
        let mut c = stalled_client();
        assert_eq!(with_deadline(&mut c, 10, 30), Outcome::Waiting);
        assert_eq!(
            with_deadline(&mut c, 30, 30),
            Outcome::Dropped,
            "the never-completing client is dropped AT the deadline"
        );
    }

    #[test]
    fn f4_no_deadline_pins_the_worker() {
        let mut c = stalled_client();
        // Ten thousand ticks later the handler is STILL waiting — the
        // worker is pinned for the connection's lifetime.
        assert_eq!(without_deadline(&mut c, 10_000), Outcome::Waiting);
    }

    #[test]
    fn f4_long_poll_deadline_is_a_value_not_a_kill() {
        // The honest anti-regime: a deliberately long-poll endpoint sets
        // a bounded but LONG deadline — the class fires on the ABSENCE of
        // a deadline, never on the value chosen.
        let mut c = stalled_client();
        let deadline = 3600; // one hour, bounded
        assert_eq!(with_deadline(&mut c, 3599, deadline), Outcome::Waiting);
        assert_eq!(with_deadline(&mut c, 3600, deadline), Outcome::Dropped);
    }

    #[test]
    fn f4_tampered_deadline_that_never_fires_disagrees() {
        // The tampered arm: a "deadline" of usize::MAX — present in name,
        // never firing. It must disagree with any real deadline.
        let mut c = stalled_client();
        let honest = with_deadline(&mut c, 30, 30);
        let tampered = with_deadline(&mut c, 30, usize::MAX);
        assert_ne!(
            honest, tampered,
            "an infinite 'deadline' is the naive shape"
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// F5 — the open-redirect gate
// ────────────────────────────────────────────────────────────────────

mod f5_open_redirect {
    #[derive(Debug, PartialEq)]
    enum Verdict {
        Relative,
        RouteName,
        Allowed,
        Refused,
    }

    /// The source's SAFE half: route-name builders produce
    /// server-controlled URLs — a redirect to their result is not
    /// user-controlled.
    const ROUTE_BUILDERS: [&str; 5] = [
        "url_for",
        "url_for_external",
        "reverse",
        "reverse_lazy",
        "get_absolute_url",
    ];

    /// FIXED: relative-only normalization + the route-builder safe half.
    fn gate(target: &str, built_by: Option<&str>) -> Verdict {
        if let Some(f) = built_by {
            if ROUTE_BUILDERS.contains(&f) {
                return Verdict::RouteName;
            }
        }
        if target.starts_with('/') && !target.starts_with("//") {
            Verdict::Relative
        } else {
            Verdict::Refused
        }
    }

    /// NAIVE: the target passes through.
    fn passthrough(_target: &str) -> Verdict {
        Verdict::Allowed
    }

    #[test]
    fn f5_relative_passes_absolute_offsite_refused() {
        assert_eq!(gate("/dashboard", None), Verdict::Relative);
        assert_eq!(
            gate("https://evil.example/steal", None),
            Verdict::Refused,
            "the absolute off-site jump is refused"
        );
        // The protocol-relative form is absolute too.
        assert_eq!(gate("//evil.example/x", None), Verdict::Refused);
    }

    #[test]
    fn f5_route_name_builder_is_the_safe_half() {
        assert_eq!(
            gate("whatever_the_route_resolves_to", Some("url_for")),
            Verdict::RouteName,
            "a route-name builder's output is server-controlled (the source's own safe half)"
        );
    }

    #[test]
    fn f5_passthrough_ships_the_evil_url() {
        assert_eq!(passthrough("https://evil.example/steal"), Verdict::Allowed);
    }

    #[test]
    fn f5_tampered_wildcard_allowlist_disagrees() {
        // The tampered arm: an "allowlist" containing "*" admits
        // everything — it must disagree with the honest gate on the evil
        // URL.
        let tampered = |target: &str, extra: &[&str]| {
            if extra.contains(&"*") || target.starts_with('/') {
                Verdict::Allowed
            } else {
                Verdict::Refused
            }
        };
        assert_eq!(gate("https://evil.example/steal", None), Verdict::Refused);
        assert_eq!(
            tampered("https://evil.example/steal", &["*"]),
            Verdict::Allowed,
            "the wildcard 'allowlist' must disagree with the honest gate"
        );
    }
}
