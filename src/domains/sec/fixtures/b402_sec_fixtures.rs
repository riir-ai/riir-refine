//! Batch 402 sec fixture preflight — the deny-list-completeness law
//! (Plan 402 / Research 437, Layr-Labs/mlxfast-bonsai2-27b-engine @
//! 0c7c3923, MIT — the arena reorg's confinement-profile lane) as ONE
//! standalone file (1 rule, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproductions — zero repo source pasted (the
//! source is Python/Seatbelt). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b402_sec_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 deny-last arm: in a later-rules-wins engine, an allow covering
//!   an evaluator path is OVERRIDDEN by the protective deny emitted
//!   after it; the naive order (deny then allow) reads the golden.
//! - F2 incomplete-official arm: a missing required deny input refuses
//!   WITH THE NAME (exit 2 semantics); the naive builder proceeds and
//!   the fence is silently narrower.
//! - F3 divergent-provenance arm: per-caller ARGUMENTS let one caller
//!   omit an evaluator path and read the golden; env-sourced inputs
//!   deny the same set at every call site.
//! - F4 resolution arm: a symlinked temp dir + a not-yet-existing
//!   output tree resolve at the deepest EXISTING ancestor with the
//!   tail kept — the naive realpath of the full path matches nothing.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ══════════════════ the model ══════════════════

/// a later-rules-wins deny/allow engine (the Seatbelt class):
/// the LAST rule covering a request wins.
#[derive(Default)]
struct Engine {
    rules: Vec<Rule>,
}

struct Rule {
    allow: bool,
    path_prefix: String,
}

impl Engine {
    /// advisory-only check: the last matching rule wins
    fn decides(&self, path: &str) -> Option<bool> {
        self.rules
            .iter()
            .rev()
            .find(|r| path.starts_with(&r.path_prefix))
            .map(|r| r.allow)
    }
}

#[derive(Default)]
struct ProfileBuilder {
    engine: Engine,
    /// the env-sourced evaluator paths (ONE authoritative channel)
    env_evaluator_paths: Vec<String>,
}

impl ProfileBuilder {
    /// (2) the ONE authoritative channel: deny inputs read from the env,
    /// never taken as caller arguments
    fn from_env_channel() -> Self {
        ENV.with(|e| ProfileBuilder {
            engine: Engine::default(),
            env_evaluator_paths: e.borrow().values().cloned().collect(),
        })
    }

    /// allows first...
    fn allow_write_prefix(mut self, prefix: &str) -> Self {
        self.engine.rules.push(Rule {
            allow: true,
            path_prefix: prefix.to_string(),
        });
        self
    }

    /// (1) ...protective denies LAST so they win over every allow
    fn finish_with_deny_last(mut self) -> Self {
        for p in &self.env_evaluator_paths {
            let resolved = resolve_deepest_existing(p);
            self.engine.rules.push(Rule {
                allow: false,
                path_prefix: resolved,
            });
        }
        self
    }

    /// the defect: denies in the middle, allows appended after —
    /// a covering allow OUTRANKS the deny in a later-wins engine
    fn deny_then_allow(mut self, deny_paths: &[String], allow_prefix: &str) -> Self {
        for p in deny_paths {
            let resolved = resolve_deepest_existing(p);
            self.engine.rules.push(Rule {
                allow: false,
                path_prefix: resolved,
            });
        }
        self.engine.rules.push(Rule {
            allow: true,
            path_prefix: allow_prefix.to_string(),
        });
        self
    }

    /// (3) the official posture refuses incomplete, naming every miss
    fn official_from(
        env: &BTreeMap<String, String>,
        required: &[&str],
    ) -> Result<Self, Vec<String>> {
        let missing: Vec<String> = required
            .iter()
            .filter(|k| env.get(**k).is_none())
            .map(|k| (*k).to_string())
            .collect();
        if !missing.is_empty() {
            return Err(missing); // exit 2 with the FULL list, not the first
        }
        let mut b = ProfileBuilder::default();
        for k in required {
            if let Some(v) = env.get(*k) {
                b.env_evaluator_paths.push(v.clone());
            }
        }
        Ok(b)
    }
}

/// (4) the resolution rider: resolve at the deepest EXISTING ancestor,
/// keep the tail — realpath alone matches nothing for a not-yet-existing
/// output tree, and a symlinked ancestor must resolve like the engine's.
fn resolve_deepest_existing(path: &str) -> String {
    let p = Path::new(path);
    let mut anc: PathBuf = p.to_path_buf();
    loop {
        if anc.exists() {
            let resolved = anc.canonicalize().unwrap_or_else(|_| anc.clone());
            let tail = p.strip_prefix(&anc).unwrap_or_else(|_| Path::new(""));
            return resolved.join(tail).to_string_lossy().into_owned();
        }
        match anc.parent() {
            Some(parent) if parent != anc => anc = parent.to_path_buf(),
            _ => return path.to_string(),
        }
    }
}

thread_local! {
    /// the stand-in env channel
    static ENV: RefCell<BTreeMap<String, String>> = RefCell::new(BTreeMap::new());
}

// ══════════════════ arms ══════════════════

const GOLDEN: &str = "/srv/evaluator/goldens/q38";

#[test]
fn f1_deny_last_overrides_a_covering_allow() {
    ENV.with(|e| {
        e.borrow_mut().insert("GOLDEN_DIR".into(), GOLDEN.into());
    });
    let engine = ProfileBuilder::from_env_channel()
        .allow_write_prefix("/srv/evaluator") // the sloppy write-prefix
        .allow_write_prefix("/tmp/out") // the intended write tree
        .finish_with_deny_last()
        .engine;
    // the deny is LAST: the covering allow does NOT rescue the golden
    assert_eq!(
        engine.decides(GOLDEN),
        Some(false),
        "deny-last wins over every allow"
    );
    // an allow outside the evaluator tree stays allowed
    assert_eq!(engine.decides("/tmp/out/step.bin"), Some(true));

    // the naive order: deny first, the SAME covering allow appended
    // after — in a later-wins engine the allow OUTRANKS the deny and
    // the golden reads
    let naive = ProfileBuilder::default()
        .deny_then_allow(&[GOLDEN.to_string()], "/srv/evaluator")
        .engine;
    assert_eq!(
        naive.decides(GOLDEN),
        Some(true),
        "deny-then-allow leaks the golden"
    );
    ENV.with(|e| e.borrow_mut().clear());
}

#[test]
fn f2_incomplete_official_refuses_naming_every_miss() {
    let env: BTreeMap<String, String> = BTreeMap::from([
        ("GOLDEN_DIR".to_string(), GOLDEN.to_string()),
        (
            "BASELINE_WORKSPACE".to_string(),
            "/srv/evaluator/baseline".to_string(),
        ),
        // BASELINE_CALIBRATION and BENCHD_BIN_DIR are UNSET
    ]);
    let required = [
        "GOLDEN_DIR",
        "BASELINE_WORKSPACE",
        "BASELINE_CALIBRATION",
        "BENCHD_BIN_DIR",
    ];
    let err = match ProfileBuilder::official_from(&env, &required) {
        Err(e) => e,
        Ok(_) => panic!("the official posture must refuse an incomplete env"),
    };
    assert_eq!(
        err.len(),
        2,
        "every missing input is named, not just the first"
    );
    assert!(err.contains(&"BASELINE_CALIBRATION".to_string()));
    assert!(err.contains(&"BENCHD_BIN_DIR".to_string()));
    // the naive builder: proceeds with whatever it got — a silently
    // narrower fence (modeled: no refusal, the profile builds)
    let naive_builds = env.contains_key("GOLDEN_DIR"); // the subset check never fires
    assert!(
        naive_builds,
        "the naive path has no completeness gate at all"
    );
}

#[test]
fn f3_env_sourced_deny_set_is_caller_invariant() {
    ENV.with(|e| {
        e.borrow_mut().insert("GOLDEN_DIR".into(), GOLDEN.into());
    });
    // two callers, zero deny-path arguments: both read the SAME channel,
    // both deny the golden
    let a = ProfileBuilder::from_env_channel()
        .allow_write_prefix("/tmp/a")
        .finish_with_deny_last()
        .engine;
    let b = ProfileBuilder::from_env_channel()
        .allow_write_prefix("/tmp/b")
        .finish_with_deny_last()
        .engine;
    assert_eq!(a.decides(GOLDEN), Some(false));
    assert_eq!(
        b.decides(GOLDEN),
        Some(false),
        "caller B denies the same set"
    );

    // the defect: caller args — a caller that forgets one path leaks it
    let leaky = ProfileBuilder::default()
        .allow_write_prefix("/tmp/c")
        .deny_then_allow(&[], "/tmp/c") // no denies passed at all
        .engine;
    assert_eq!(
        leaky.decides(GOLDEN),
        None,
        "the argument-less caller denies NOTHING"
    );
    ENV.with(|e| e.borrow_mut().clear());
}

#[test]
fn f4_resolve_at_deepest_existing_ancestor_keeps_tail() {
    let base = std::env::temp_dir().join(format!("b402_sec_{}", std::process::id()));
    let real = base.join("real_goldens");
    std::fs::create_dir_all(&real).expect("mkdir");
    // the output tree does NOT exist yet — only its parent does
    let out_missing = base.join("out/step.bin");
    // 1) realpath of the not-yet-existing path matches nothing
    let naive = std::fs::canonicalize(&out_missing);
    assert!(
        naive.is_err(),
        "canonicalize of a missing tree fails — the naive rule is dead"
    );
    // 2) resolve at the deepest EXISTING ancestor, keep the tail
    let resolved = resolve_deepest_existing(out_missing.to_str().unwrap());
    let expected_tail = std::fs::canonicalize(&base).unwrap();
    assert!(
        resolved.starts_with(expected_tail.to_str().unwrap()),
        "resolved = {resolved}; must root at the resolved ancestor {expected_tail:?}"
    );
    assert!(
        resolved.ends_with("out/step.bin"),
        "the tail is kept verbatim"
    );
    // 3) the symlink ancestor resolves like the engine's view
    let link = base.join("link_goldens");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&real, &link).expect("symlink");
    let resolved_link = resolve_deepest_existing(link.to_str().unwrap());
    assert_eq!(
        Path::new(&resolved_link),
        real.canonicalize().unwrap(),
        "the existing symlinked leaf resolves to its target"
    );
    std::fs::remove_dir_all(&base).ok();
}
