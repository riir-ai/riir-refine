//! Batch 373 sec fixture preflight — the cargo-atlas transcription pair
//! (Plan 389 / Research 423): `flowerornament/atlas`
//! @ `49d9ee41cca1cb8359fe28e2be13b430bba96569` (MIT OR Apache-2.0).
//! ORIGINAL plain-Rust stand-ins — zero repo source pasted (the laws are
//! re-authored; the source quotes stay in the corpus). Compiled AND
//! executed BEFORE entry:
//! `rustc --edition 2021 --test b389_fixtures.rs && ./b389_fixtures`
//!
//! Arms:
//! - F1 `external-helper-config-pinned-invocation`: a helper spawned
//!   with an INHERITED environment — the invoking shell's GIT_DIR steals
//!   the repository selection and GIT_CONFIG_* injects config the helper
//!   never asked for; the pin strips the whole GIT_* prefix family and
//!   sets the locks knob before spawn; the tampered arm proves a partial
//!   strip (GIT_DIR alone, the famous name) still leaks the config
//!   family; the clean-env arm pins the agreement (pinning is a no-op
//!   where no user state exists).
//! - F2 `subprocess-argument-injection-pinned-flags`: a caller-controlled
//!   revision string reaching a parser argv unpinned is consumed as
//!   FLAGS; the pin (--verify + --end-of-options before the operand +
//!   the `^{commit}` wrap) parses it as a rev or refuses; the tampered
//!   arm proves pin-once-at-one-site is not the law (the second call
//!   site re-injects); the jj half format-gates third-party ids (exactly
//!   one, 40-or-64 hex — "JJ did not return one complete Git commit id").

// ═══════════════════════════════════════════════════════════════════════
// F1 — external-helper-config-pinned-invocation
// ═══════════════════════════════════════════════════════════════════════

/// The child environment a helper would be spawned under, captured as a
/// plain map — the stand-in for `std::process::Command`'s env surface.
#[derive(Default)]
struct ChildEnv {
    inherited: Vec<(String, String)>, // what the parent shell carried
    removed: Vec<String>,             // names stripped before spawn
    set: Vec<(String, String)>,       // names the spawner pins itself
}

impl ChildEnv {
    /// What the child actually SEES: parent env minus removals plus sets.
    fn effective(&self) -> Vec<(String, String)> {
        self.inherited
            .iter()
            .filter(|(k, _)| !self.removed.contains(k))
            .cloned()
            .chain(self.set.iter().cloned())
            .collect()
    }

    fn get(&self, key: &str) -> Option<String> {
        self.effective()
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }
}

/// BUGGY: the helper inherits the parent environment untouched — every
/// Git invocation reads whatever repository the shell's GIT_* state
/// selects, not the root the caller asked about.
fn spawn_helper_buggy(parent_env: &[(String, String)]) -> ChildEnv {
    ChildEnv {
        inherited: parent_env.to_vec(),
        ..ChildEnv::default()
    }
}

/// FIXED (the atlas law): repository selection belongs to the requested
/// root, not inherited Git state — the whole `GIT_` prefix family is
/// removed and the locks knob is set, per-helper, before spawn.
fn spawn_helper_pinned(parent_env: &[(String, String)]) -> ChildEnv {
    let mut env = ChildEnv {
        inherited: parent_env.to_vec(),
        ..ChildEnv::default()
    };
    for (name, _) in parent_env {
        if name.starts_with("GIT_") {
            env.removed.push(name.clone());
        }
    }
    env.set
        .push(("GIT_OPTIONAL_LOCKS".to_string(), "0".to_string()));
    env
}

/// TAMPERED (the partial pin): strips the famous name only — GIT_DIR —
/// and leaves the config-injection family (GIT_CONFIG_*) inherited.
/// Looks pinned on the one variable everyone knows; the injection lives.
fn spawn_helper_partial(parent_env: &[(String, String)]) -> ChildEnv {
    let mut env = ChildEnv {
        inherited: parent_env.to_vec(),
        ..ChildEnv::default()
    };
    for (name, _) in parent_env {
        if name == "GIT_DIR" {
            env.removed.push(name.clone());
        }
    }
    env
}

/// The helper's repository-selection read: whatever GIT_DIR survives in
/// its environment WINS over the root the caller passed — the selection
/// theft the pin exists to prevent.
fn resolve_repository(env: &ChildEnv, requested_root: &str) -> String {
    match env.get("GIT_DIR") {
        Some(stolen) => stolen,
        None => requested_root.to_string(),
    }
}

/// The config-injection surface: GIT_CONFIG_COUNT/KEY_n/VALUE_n build an
/// extra config the helper did not ask for (the stand-in reads the
/// count exactly as git's own mechanism does).
fn injected_config(env: &ChildEnv) -> Vec<(String, String)> {
    let count = env
        .get("GIT_CONFIG_COUNT")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    (0..count)
        .filter_map(|i| {
            let key = env.get(&format!("GIT_CONFIG_KEY_{i}"))?;
            let val = env.get(&format!("GIT_CONFIG_VALUE_{i}"))?;
            Some((key, val))
        })
        .collect()
}

#[test]
fn f1_external_helper_config_pinned_invocation() {
    // the hostile shell: the user's ambient Git state steers both the
    // repository selection AND the helper's config.
    let hostile: Vec<(String, String)> = vec![
        ("GIT_DIR".to_string(), "/attacker/repo".to_string()),
        ("GIT_CONFIG_COUNT".to_string(), "1".to_string()),
        ("GIT_CONFIG_KEY_0".to_string(), "core.hooksPath".to_string()),
        (
            "GIT_CONFIG_VALUE_0".to_string(),
            "/attacker/hook".to_string(),
        ),
        ("HOME".to_string(), "/home/dev".to_string()), // not GIT-scoped
    ];
    let requested = "/clean/repo";

    // BUGGY: the inherited state wins on BOTH surfaces — the helper
    // reads the attacker's repository and runs under the injected hook
    // config while believing it served /clean/repo.
    let buggy = spawn_helper_buggy(&hostile);
    assert_eq!(
        resolve_repository(&buggy, requested),
        "/attacker/repo",
        "the unpinned helper's selection is stolen by inherited GIT_DIR"
    );
    assert_eq!(
        injected_config(&buggy),
        vec![("core.hooksPath".to_string(), "/attacker/hook".to_string())],
        "the unpinned helper inherits the injected config"
    );

    // FIXED: the requested root wins, the injected config is gone, and
    // the locks knob is pinned — the caller's arguments decide.
    let pinned = spawn_helper_pinned(&hostile);
    assert_eq!(
        resolve_repository(&pinned, requested),
        requested,
        "the pinned helper selects the requested root"
    );
    assert!(
        injected_config(&pinned).is_empty(),
        "the pinned helper carries no injected config"
    );
    assert_eq!(
        pinned.get("GIT_OPTIONAL_LOCKS").as_deref(),
        Some("0"),
        "the locks knob is pinned before spawn"
    );
    // non-GIT user state survives — the strip is scoped, not a wipe
    assert_eq!(
        pinned.get("HOME").as_deref(),
        Some("/home/dev"),
        "the pin strips the steering family only, not the environment"
    );

    // TAMPERED: the partial pin strips the famous name and STILL leaks
    // — the config family rides into every Git call the helper makes.
    let partial = spawn_helper_partial(&hostile);
    assert_eq!(
        resolve_repository(&partial, requested),
        requested,
        "the partial pin LOOKS correct on the famous name"
    );
    assert_eq!(
        injected_config(&partial),
        vec![("core.hooksPath".to_string(), "/attacker/hook".to_string())],
        "partial stripping is not pinning: the config family survives"
    );

    // ANTI-REGIME (all-safe): a clean shell — no user state, no steering
    // — buggy and pinned agree on every observable.
    let clean: Vec<(String, String)> = vec![("HOME".to_string(), "/home/dev".to_string())];
    let a = spawn_helper_buggy(&clean);
    let b = spawn_helper_pinned(&clean);
    assert_eq!(
        resolve_repository(&a, requested),
        resolve_repository(&b, requested),
        "with no user state both postures select the requested root"
    );
    assert!(injected_config(&a).is_empty() && injected_config(&b).is_empty());
    // the only divergence is the deliberate lock knob — a posture, not
    // a steering surface
    assert_eq!(b.get("GIT_OPTIONAL_LOCKS").as_deref(), Some("0"));
    assert_eq!(a.get("GIT_OPTIONAL_LOCKS"), None);
}

// ═══════════════════════════════════════════════════════════════════════
// F2 — subprocess-argument-injection-pinned-flags
// ═══════════════════════════════════════════════════════════════════════

/// The invocation under construction — the stand-in for an argv array
/// (the args AFTER the program name).
#[derive(Default)]
struct Invocation {
    args: Vec<String>,
}

impl Invocation {
    fn push(&mut self, arg: &str) {
        self.args.push(arg.to_string());
    }
}

/// A toy parser with rev-parse's grammar: tokens before an
/// `--end-of-options` marker that begin with `--` are OPTIONS (the
/// parser acts on them — `executed` is that side channel); after the
/// marker, every token is a REV. `--verify` demands the operand resolve
/// to a known object; the `^{commit}` suffix forces the object type.
fn parse_rev(
    inv: &Invocation,
    objects: &[&str],
    executed: &mut Vec<String>,
) -> Result<String, String> {
    let mut options_ended = false;
    let mut verify = false;
    let mut revs: Vec<String> = Vec::new();
    for arg in &inv.args {
        if !options_ended && arg == "--end-of-options" {
            options_ended = true;
        } else if !options_ended && arg.starts_with("--") {
            if arg == "--verify" {
                verify = true;
            } else {
                // INJECTION: a user rev shaped like a flag reaches the
                // parser's option branch and is ACTED ON, not resolved.
                executed.push(arg.clone());
            }
        } else {
            revs.push(arg.clone());
        }
    }
    let rev = revs.last().ok_or("no revision given")?;
    let obj = rev.strip_suffix("^{commit}").unwrap_or(rev);
    if verify && !objects.contains(&obj) {
        return Err(format!(
            "fatal: ambiguous argument '{rev}': unknown revision"
        ));
    }
    Ok(obj.to_string())
}

/// BUGGY resolve: the caller's rev goes into the argv RAW — no marker,
/// no wrap, no verify.
fn resolve_buggy(inv: &mut Invocation, rev: &str) {
    inv.push("rev-parse");
    inv.push(rev);
}

/// FIXED resolve (the atlas triple, one site): --verify +
/// --end-of-options BEFORE the operand + the `^{commit}` wrap — the
/// user string is parsed as a rev or refused, never as a flag.
fn resolve_pinned(inv: &mut Invocation, rev: &str) {
    inv.push("rev-parse");
    inv.push("--verify");
    inv.push("--end-of-options");
    inv.push(&format!("{rev}^{{commit}}"));
}

/// TAMPERED (the pin-once pattern): the SECOND call site — the tree
/// resolve — re-takes the user's rev with no pin of its own. The law is
/// pin-at-each-call-site, not once at setup.
fn resolve_tree_tampered(user_rev: &str, executed: &mut Vec<String>) -> Result<String, String> {
    let mut inv = Invocation::default();
    inv.push("rev-parse");
    inv.push(user_rev); // site 2: the pin was left at site 1
    parse_rev(&inv, &[], executed)
}

/// The jj half: one tool's ids re-consumed by another. BUGGY forwards
/// the first line unvalidated.
fn jj_ids_buggy(output: &str) -> Result<String, String> {
    let Some(first) = output.lines().next() else {
        return Err("empty".to_string());
    };
    Ok(first.to_owned())
}

/// FIXED: exactly one line, 40-or-64 hex — anything else is refused,
/// named (the atlas refusal verbatim).
fn jj_ids_pinned(output: &str) -> Result<String, String> {
    let ids: Vec<&str> = output.lines().collect();
    if ids.len() != 1 {
        return Err(format!(
            "revision must resolve to exactly one commit, found {}",
            ids.len()
        ));
    }
    let id = ids[0];
    if !matches!(id.len(), 40 | 64) || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("JJ did not return one complete Git commit id".to_string());
    }
    Ok(id.to_owned())
}

#[test]
fn f2_subprocess_argument_injection_pinned_flags() {
    let objects = ["abc123", "def456"];
    let hostile_rev = "--upload-pack=evil"; // a rev shaped like a flag

    // BUGGY: the flag-shaped rev reaches the parser's option branch and
    // is CONSUMED — the injection observable is the side channel the
    // real git process would carry as its own semantics.
    let mut inv = Invocation::default();
    let mut executed: Vec<String> = Vec::new();
    resolve_buggy(&mut inv, hostile_rev);
    let _ = parse_rev(&inv, &objects, &mut executed);
    assert_eq!(
        executed,
        vec![hostile_rev.to_string()],
        "the unpinned rev is parsed as a FLAG, not data"
    );

    // FIXED: the same string crosses the pin — end-of-options makes it a
    // rev, the wrap forces the type, verify refuses it as unknown DATA.
    let mut inv2 = Invocation::default();
    let mut executed2: Vec<String> = Vec::new();
    resolve_pinned(&mut inv2, hostile_rev);
    let pinned = parse_rev(&inv2, &objects, &mut executed2);
    assert!(
        executed2.is_empty(),
        "nothing behind the marker is consumed as an option"
    );
    let err = pinned.expect_err("the flag-shaped string must refuse as a rev");
    assert!(
        err.contains("unknown revision") && err.contains("^{commit}"),
        "the refusal names the rev, never a flag consumption: {err}"
    );

    // TAMPERED (pin-once): site 1 pins fine; the SECOND site re-takes
    // the user's rev raw and the injection fires again — the law is
    // pin-at-each-call-site.
    let mut setup = Invocation::default();
    let mut setup_exec: Vec<String> = Vec::new();
    resolve_pinned(&mut setup, hostile_rev);
    assert!(parse_rev(&setup, &objects, &mut setup_exec).is_err());
    assert!(setup_exec.is_empty(), "site 1 (pinned) consumed nothing");
    let mut second_exec: Vec<String> = Vec::new();
    let _ = resolve_tree_tampered(hostile_rev, &mut second_exec);
    assert_eq!(
        second_exec,
        vec![hostile_rev.to_string()],
        "the unpinned second site re-injects — pin-once is not the law"
    );

    // ANTI-REGIME (all-safe): a plain hex rev — both postures resolve it
    // to the same object (the pin is inert on well-formed input).
    let mut plain_buggy = Invocation::default();
    resolve_buggy(&mut plain_buggy, "abc123");
    let mut plain_exec: Vec<String> = Vec::new();
    let a = parse_rev(&plain_buggy, &objects, &mut plain_exec).expect("plain resolves buggy");
    let mut plain_pinned = Invocation::default();
    resolve_pinned(&mut plain_pinned, "abc123");
    let b = parse_rev(&plain_pinned, &objects, &mut plain_exec).expect("plain resolves pinned");
    assert_eq!(a, b, "both postures agree on a plain rev");
    assert!(plain_exec.is_empty(), "no option consumption on either arm");

    // the jj half — one tool's output is another tool's input only after
    // the format gate.
    let good64 = "a".repeat(64);
    let good40 = "b".repeat(40);
    assert_eq!(jj_ids_pinned(&good64).unwrap(), good64);
    assert_eq!(jj_ids_pinned(&good40).unwrap(), good40);
    let two_lines = format!("{good64}\n{good64}");
    let err2 = jj_ids_pinned(&two_lines).expect_err("two lines refuse");
    assert!(
        err2.contains("exactly one commit") && err2.contains("found 2"),
        "the multi-line refusal names the count: {err2}"
    );
    let malformed = "not-a-commit-id";
    let err3 = jj_ids_pinned(malformed).expect_err("non-hex refuses");
    assert_eq!(
        err3, "JJ did not return one complete Git commit id",
        "the format refusal is named, not guessed"
    );
    let short = "abc123";
    assert!(
        jj_ids_pinned(short).is_err(),
        "an abbreviated id refuses too"
    );
    // BUGGY: the raw bridge forwards the first line whatever it is.
    assert_eq!(
        jj_ids_buggy(&two_lines).unwrap(),
        good64,
        "the unpinned bridge takes the first line of TWO — one commit's context lost"
    );
    assert_eq!(
        jj_ids_buggy(malformed).unwrap(),
        malformed,
        "the unpinned bridge forwards non-hex without a gate"
    );
}
