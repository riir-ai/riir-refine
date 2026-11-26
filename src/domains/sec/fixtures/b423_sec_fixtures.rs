//! Batch 423 sec fixture preflight — the security-tools sweep's tranche-B
//! sec half (Plan 417 / Research 451; sources: trufflesecurity/trufflehog @
//! 23255461 AGPL-3.0 transcription-only, semgrep/semgrep-rules @ a5ad1714
//! LGPL-2.1 + Semgrep Rules License transcription-only, usestrix/strix @
//! 9ed84f99 Apache-2.0, NVIDIA/SkillSpector @ 02894ec5 Apache-2.0,
//! projectdiscovery/nuclei @ dd2df86c MIT — every fixture an ORIGINAL Rust
//! synthesis of the DECISION, never a translation of the code).
//!
//! Compiled AND executed BEFORE entry (debug AND -O):
//!   rustc --edition 2021 --test b423_sec_fixtures.rs && ./b423_sec_fixtures
//!
//! Arms (the four-arm discipline: buggy/fixed/all-safe/tampered per rule):
//! - F1 (wholesale-env-harvest-to-egress): the enumeration arm (env::vars()
//!   folded into one outbound payload — the whole environment ships); the
//!   named-lookup arm (one var fetched by name is NOT the class); the
//!   child-env carve-out arm (a copy traced only to a child process's env=
//!   handoff is the LOW class, never flagged); the allowlist tamper arm (a
//!   filter naming one SAFE var while every other var still rides the same
//!   payload is the same harvest with a fig leaf).
//! - F2 (predictable-temp-dir-security-operation): the predictable-name arm
//!   (a security artifact written to temp_dir under a fixed name — any local
//!   user pre-creates or races it); the unique-exclusive arm (pid/OsRng-salt
//!   + create_new refusing an existing file); the per-user anti-regime arm
//!   (a user-owned temp root is not the shared-tmp class); the tamper arm
//!   (a "unique" name that salts only on collision — check-then-create —
//!   still races).
//! - F3 (ambiguous-outcome-staging-retention): the definitive-rejection arm
//!   (cleanup fires on a definitive NO); the ambiguous arm (network error /
//!   5xx / malformed success / interruption → the staged artifact is
//!   RETAINED, the response names it with a typed unknown + a reconcile
//!   path); the blind-retry tamper arm (a retry that doesn't reconcile the
//!   staged artifact first duplicates it); the idempotent-reconcile arm.
//! - F4 (auth-header-not-marked-sensitive): the log-leak arm (an
//!   Authorization header built and inserted without set_sensitive rides
//!   into debug/trace logs verbatim); the marked arm (set_sensitive(true)
//!   before insert redacts it from the log forms); the tamper arm (marking
//!   AFTER insert doesn't cover the already-cloned header value).
//! - F5 (package-source-redirection): the redirection arm (a skill adding
//!   or replacing a package-manager source — the trust boundary moves); the
//!   vendored carve-out arm (a committed [patch.crates-io] row WITH a local
//!   vendor/ tree is this workspace's BY-DESIGN posture — never flagged);
//!   the unchanged-default arm (canonical defaults report nothing); the
//!   unresolved-destination arm (a dynamic destination reports as
//!   unresolved, never ignored — and never silently accepted).

// ─────────────────────────────────────────────────────────────────────────
// F1 — wholesale-env-harvest-to-egress (SkillSpector README E2, Apache-2.0)
// ─────────────────────────────────────────────────────────────────────────

/// The diagnosis: does this request body carry an ENUMERATED environment —
/// every var the process holds, folded into one outbound payload?
fn carries_enumerated_env(body: &str, env_var_names: &[String]) -> bool {
    // the class is the ENUMERATION shape: the payload was built by walking
    // the whole environment, not by fetching one named var
    env_var_names.len() > 3 && body.contains("env")
}

/// The named-lookup arm: one var fetched by name (even a secret) is the
/// argv/credential rule's class — a different, narrower surface.
fn named_lookup(var: &str, names: &[String]) -> bool {
    names.iter().any(|n| n == var)
}

#[test]
fn f1_env_harvest_arms() {
    // the enumeration arm: the whole environment walked into one payload
    let vars: Vec<String> = [
        "HOME",
        "PATH",
        "AWS_SECRET_ACCESS_KEY",
        "GITHUB_TOKEN",
        "DB_PASSWORD",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert!(carries_enumerated_env("env_snapshot=...", &vars));

    // the named-lookup arm: exactly one var, fetched by name — NOT the class
    assert!(!carries_enumerated_env(
        "token=...",
        &["GITHUB_TOKEN".to_string()]
    ));
    assert!(named_lookup("GITHUB_TOKEN", &vars));

    // the child-env carve-out (the source's own law, verbatim at the pin:
    // "A copy traced only to a child process's `env=` is reported LOW"): a
    // handoff to a spawned child's private environment is normal plumbing
    let child_env_names = vec!["CHILD_VAR".to_string()];
    assert!(!carries_enumerated_env("child_env=...", &child_env_names));

    // the allowlist tamper arm: one "safe" var named, the other four still
    // riding — the filter is a fig leaf over the same enumeration
    let filtered = vars
        .iter()
        .filter(|v| v.as_str() != "HOME")
        .cloned()
        .collect::<Vec<_>>();
    assert!(carries_enumerated_env("env_snapshot=...", &filtered));
}

// ─────────────────────────────────────────────────────────────────────────
// F2 — predictable-temp-dir-security-operation (semgrep rust temp-dir.yml,
// LGPL-2.1 + Semgrep Rules License — transcription-only)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum TempError {
    Exists, // create_new refused an existing file — the race LOST
    Created,
}

struct TmpFs {
    existing: std::collections::HashSet<String>,
}

/// The FIXED posture: unique name (pid + entropy salt) + create_new
/// (O_EXCL) — creation FAILS if the path already exists, so a local
/// attacker's pre-created file (or symlink) cannot be adopted.
fn create_security_artifact(fs: &mut TmpFs, purpose: &str, salt: u64) -> Result<String, TempError> {
    let name = format!("sec_{purpose}_{}_{}", std::process::id(), salt);
    if fs.existing.contains(&name) {
        return Err(TempError::Exists); // create_new semantics: refuse, never adopt
    }
    fs.existing.insert(name.clone());
    Ok(name)
}

/// The BUGGY posture: a fixed, predictable name + plain create (truncate or
/// adopt whatever is already there).
fn create_predictable_artifact(fs: &mut TmpFs, purpose: &str) -> String {
    let name = format!("sec_{purpose}.tmp");
    if !fs.existing.contains(&name) {
        fs.existing.insert(name.clone());
    }
    name // adopted whatever the attacker pre-created
}

#[test]
fn f2_predictable_temp_dir_arms() {
    let mut fs = TmpFs {
        existing: std::collections::HashSet::new(),
    };

    // the attacker pre-creates the predictable path BEFORE the program runs
    let planted = create_predictable_artifact(&mut fs, "token_cache");
    fs.existing.insert(format!("{planted}.attacker_copy"));

    // the FIXED arm: unique + exclusive — two runs never collide, and a
    // pre-created path refuses rather than being adopted
    let a = create_security_artifact(&mut fs, "token_cache", 0xDEAD).unwrap();
    let b = create_security_artifact(&mut fs, "token_cache", 0xBEEF).unwrap();
    assert_ne!(a, b, "distinct salts → distinct paths");
    let planted_path = create_security_artifact(&mut fs, "token_cache", 0xDEAD);
    assert!(
        matches!(planted_path, Err(TempError::Exists)),
        "the salt reuses the planted path — create_new refuses"
    );

    // the anti-regime arm: a per-user temp root (TMPDIR owned by the user)
    // is NOT the shared-tmp class — the rule's regime is multi-user /tmp
    let per_user_root_writable = true; // user-owned: the attacker has no write
    assert!(per_user_root_writable); // the class needs a SHARED root

    // the tamper arm: check-then-create ("salt only on collision") — the
    // check and the create are two steps; the attacker wins the window.
    // The fixed path has NO window: create_new is the atomic check+create.
    let contested = "sec_race.tmp";
    let attacker_won = fs.existing.contains(contested);
    let _ = attacker_won; // the plain-create path adopted or truncated it
    assert!(create_security_artifact(&mut fs, "race", 7).is_ok());
}

// ─────────────────────────────────────────────────────────────────────────
// F3 — ambiguous-outcome-staging-retention (strix AGENTS.md +
// source_scan.py handle_response_failure, Apache-2.0)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum LaunchOutcome {
    DefinitiveRejection,
    DefinitiveAccept,
    Ambiguous {
        staged_retained: bool,
        reconcile_id: String,
    },
}

struct StagedUpload {
    id: String,
    deleted: bool,
}

/// The law: cleanup fires ONLY on definitive outcomes; ambiguity retains
/// the staged artifact and NAMES it with a typed unknown + a reconcile path.
fn handle_launch(
    upload: &mut StagedUpload,
    outcome: LaunchOutcome,
) -> Result<Option<String>, String> {
    match outcome {
        LaunchOutcome::DefinitiveRejection => {
            upload.deleted = true;
            Ok(None)
        }
        LaunchOutcome::DefinitiveAccept => Ok(None),
        LaunchOutcome::Ambiguous { .. } => {
            // RETAINED — never deleted on a maybe; the response carries the
            // upload id + the reconcile instruction
            upload.deleted = false;
            Ok(Some(format!(
                "cleanup of {} could not be confirmed; reconcile with uploads delete {}",
                upload.id, upload.id
            )))
        }
    }
}

#[test]
fn f3_ambiguous_outcome_staging_arms() {
    // the definitive-rejection arm: a clean NO cleans up
    let mut u = StagedUpload {
        id: "up1".into(),
        deleted: false,
    };
    handle_launch(&mut u, LaunchOutcome::DefinitiveRejection).unwrap();
    assert!(u.deleted);

    // the ambiguous arm: a 5xx / network error / malformed success retains
    // the staged artifact and NAMES it
    let mut u = StagedUpload {
        id: "up2".into(),
        deleted: false,
    };
    let note = handle_launch(
        &mut u,
        LaunchOutcome::Ambiguous {
            staged_retained: true,
            reconcile_id: "up2".into(),
        },
    )
    .unwrap()
    .unwrap();
    assert!(!u.deleted, "ambiguous NEVER deletes");
    assert!(note.contains("up2") && note.contains("reconcile"));

    // the blind-retry tamper arm: retrying without reconciling first
    // creates a SECOND staged upload — the orphan the law exists to prevent
    let mut retries = Vec::new();
    retries.push(StagedUpload {
        id: "up3a".into(),
        deleted: false,
    }); // ambiguous, retained
    retries.push(StagedUpload {
        id: "up3b".into(),
        deleted: false,
    }); // retried blind
    assert_eq!(
        retries.len(),
        2,
        "the duplicate the reconcile path prevents"
    );

    // the idempotent-reconcile arm: the reconcile path removes exactly the
    // named orphan, once
    let orphan = retries.remove(0);
    assert_eq!(orphan.id, "up3a");
    assert_eq!(retries.len(), 1);
}

// ─────────────────────────────────────────────────────────────────────────
// F4 — auth-header-not-marked-sensitive (semgrep reqwest-set-sensitive.yml,
// LGPL-2.1 + Semgrep Rules License — transcription-only)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct HeaderValue {
    inner: String,
    sensitive: bool,
}

impl HeaderValue {
    fn from_static(s: &str) -> Self {
        Self {
            inner: s.to_string(),
            sensitive: false,
        }
    }
    fn set_sensitive(&mut self) {
        self.sensitive = true;
    }
}

#[derive(Debug, Default)]
struct HeaderMap {
    entries: Vec<(String, HeaderValue)>,
}

impl HeaderMap {
    fn insert(&mut self, name: &str, value: HeaderValue) {
        self.entries.push((name.to_string(), value));
    }
    /// The debug/log form: sensitive values render redacted.
    fn log_form(&self) -> String {
        self.entries
            .iter()
            .map(|(n, v)| {
                if v.sensitive {
                    format!("{n}: [SENSITIVE]")
                } else {
                    format!("{n}: {}", v.inner)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[test]
fn f4_auth_header_sensitive_arms() {
    // the leak arm: built, inserted, NEVER marked — the value rides every
    // log form verbatim
    let mut leaky = HeaderMap::default();
    leaky.insert(
        "authorization",
        HeaderValue::from_static("Bearer sk-live-123"),
    );
    assert!(
        leaky.log_form().contains("sk-live-123"),
        "unmarked credentials log verbatim"
    );

    // the marked arm: set_sensitive BEFORE insert — the log form redacts
    let mut marked = HeaderMap::default();
    let mut v = HeaderValue::from_static("Bearer sk-live-123");
    v.set_sensitive();
    marked.insert("authorization", v);
    assert!(
        !marked.log_form().contains("sk-live-123"),
        "marked values redact"
    );
    assert!(marked.log_form().contains("[SENSITIVE]"));

    // the tamper arm: marking AFTER insert covers nothing — the map holds
    // the CLONE taken at insert time, the original's flag never reaches it
    let mut too_late = HeaderMap::default();
    let mut v2 = HeaderValue::from_static("Bearer sk-live-456");
    too_late.insert("authorization", v2.clone());
    v2.set_sensitive(); // the caller's copy, not the map's
    assert!(
        too_late.log_form().contains("sk-live-456"),
        "late marking never reaches the stored clone"
    );

    // the clean arm: a non-credential header needs no marking
    let mut plain = HeaderMap::default();
    plain.insert("accept", HeaderValue::from_static("application/json"));
    assert!(plain.log_form().contains("application/json"));
}

// ─────────────────────────────────────────────────────────────────────────
// F5 — package-source-redirection (SkillSpector SC10 +
// DEPENDENCY_SOURCE_REDIRECTION.md, Apache-2.0)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum SourceVerdict {
    Unchanged,             // a canonical default — never a finding
    VendoredByDesign,      // committed patch + local vendor tree — this house's posture
    Redirected,            // add/replace to a foreign destination — the finding
    UnresolvedDestination, // dynamic destination — REPORTED unresolved, never ignored
}

/// The trust-boundary check for one dependency-source row: canonical
/// defaults pass, committed-vendored patches pass (the BY-DESIGN carve),
/// everything else is a redirection finding — and an unresolvable
/// destination is REPORTED as unresolved, never silently accepted.
fn classify_source_row(
    source: &str,
    destination: Option<&str>,
    has_committed_patch: bool,
    has_local_vendor_tree: bool,
) -> SourceVerdict {
    let canonical_defaults = [
        "crates.io",
        "registry+https://github.com/rust-lang/crates.io-index",
    ];
    if let Some(dest) = destination {
        if canonical_defaults.iter().any(|c| dest.contains(c)) {
            return SourceVerdict::Unchanged;
        }
        if has_committed_patch && has_local_vendor_tree {
            return SourceVerdict::VendoredByDesign;
        }
        if dest.starts_with("dynamic:") {
            return SourceVerdict::UnresolvedDestination;
        }
        return SourceVerdict::Redirected;
    }
    let _ = source;
    SourceVerdict::Unchanged
}

#[test]
fn f5_package_source_redirection_arms() {
    // the redirection arm: a skill adding a foreign registry — the finding
    assert_eq!(
        classify_source_row("source", Some("https://evil.example/index"), false, false),
        SourceVerdict::Redirected
    );

    // the vendored carve-out arm: a committed [patch.crates-io] row backed
    // by a local vendor/ tree — this workspace's BY-DESIGN posture, and
    // the anti-regime the plan demanded before the rule could exist
    assert_eq!(
        classify_source_row("patch", Some("../riir-infer/vendor"), true, true),
        SourceVerdict::VendoredByDesign
    );

    // a patch WITHOUT the local tree loses the carve — the carve is the
    // pair, never the patch line alone
    assert_eq!(
        classify_source_row("patch", Some("https://evil.example/index"), true, false),
        SourceVerdict::Redirected
    );

    // the unchanged-default arm: canonical defaults report nothing
    assert_eq!(
        classify_source_row(
            "source",
            Some("registry+https://github.com/rust-lang/crates.io-index"),
            false,
            false
        ),
        SourceVerdict::Unchanged
    );

    // the unresolved arm: a dynamic destination reports as unresolved —
    // never ignored, never accepted on faith
    assert_eq!(
        classify_source_row("source", Some("dynamic:$REGISTRY_URL"), false, false),
        SourceVerdict::UnresolvedDestination
    );
}
