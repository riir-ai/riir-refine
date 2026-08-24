//! Issue 038 G8 e2e: the full CLI store-backend flow, end-to-end (spawns
//! the real binary + real cargo/clippy inside it — run explicitly:
//!
//! ```text
//! cargo test --features fix_verify,clippy_verify,latent_retrieval \
//!   --test store_backend_e2e -- --ignored --nocapture
//! ```
//!
//! Proves, in ONE run on a scratch git repo with a planted fixable lint:
//!
//! 1. **Store-only persistence (038-T2/T3)** — a `--fix --verify` run
//!    persists verdicts to the repo-local `.heal/trajectories.kv` and
//!    NEVER (re)writes the legacy envelope.
//! 2. **The one-time migration** — a pre-existing envelope is imported
//!    (rows verbatim) and renamed `memory.json.migrated`.
//! 3. **The closed loop (037-E2 on the new backend)** — the oracle-confirmed
//!    edit lands as a `Certified` row with `success_count >= 1`.
//! 4. **`--memory-stats` reads the new backend state** (the debug query).

#![cfg(all(
    feature = "fix_verify",
    feature = "clippy_verify",
    feature = "latent_retrieval"
))]

use std::path::Path;
use std::process::Command;

/// One binary invocation with the env a real heal session carries.
fn heal(envs: &[(&str, &str)], args: &[&str]) -> (bool, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cargo-heal"));
    cmd.env_remove("RIIR_HEAL_TRAJ_ROOT");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.args(args);
    let out = cmd.output().expect("spawn cargo-heal");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

#[test]
#[ignore = "spawns cargo + clippy (the verify oracle); run explicitly for the 038 G8 proof"]
#[allow(clippy::too_many_lines)] // linear fixture script: setup → run → assert
fn store_backend_replaces_envelope_end_to_end() {
    let tmp = std::env::temp_dir().join(format!("heal038_e2e_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("src")).unwrap();

    // A scratch GIT repo (the .git-walk store resolver keys on it) holding
    // a real cargo crate with two planted fixable lints: a pedantic-group
    // format-arg site (oracle-confirmed via -W) + a needless_return site.
    std::fs::write(
        tmp.join("Cargo.toml"),
        "[package]\nname = \"heal038_fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
         [lib]\npath = \"src/lib.rs\"\n",
    )
    .unwrap();
    std::fs::write(
        tmp.join("src/lib.rs"),
        concat!(
            "pub fn greet(name: &str) -> String {\n",
            "    format!(\"{}\", name)\n",
            "}\n",
            "\n",
            "pub fn answer() -> u32 {\n",
            "    return 42;\n",
            "}\n",
        ),
    )
    .unwrap();
    assert!(
        Command::new("git")
            .arg("init")
            .arg(&tmp)
            .output()
            .is_ok_and(|o| o.status.success()),
        "git init (the store's repo-root resolver needs .git)"
    );

    // A pre-existing legacy envelope with two buckets — the migration input.
    let envelope = tmp.join("memory.json");
    std::fs::write(
        &envelope,
        concat!(
            r#"{"version":1,"trajectories":["#,
            r#"{"lint_name":"migrated_probe","lint_name_hash":1,"buggy_code":"b1","fixed_code":"f1","direction":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],"success_count":2,"fail_count":0},"#,
            r#"{"lint_name":"migrated_probe","lint_name_hash":1,"buggy_code":"b2","fixed_code":"f2","direction":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],"success_count":0,"fail_count":0}"#,
            r#"]}"#,
        ),
    )
    .unwrap();

    // ── The run: --fix --verify on the fixture crate. Persistence must go
    // to the store; the envelope must migrate (import + rename), never
    // save. Envelope location is env-pinned to the fixture copy.
    let envs: &[(&str, &str)] = &[("RIIR_HEAL_MEMORY", envelope.to_str().unwrap())];
    let lib = tmp.join("src/lib.rs");
    let (ok, output) = heal(envs, &["--fix", "--verify", lib.to_str().unwrap()]);
    assert!(ok, "heal run failed:\n{output}");
    assert!(
        output.contains("migrated 2/2 envelope bucket(s)"),
        "migration summary missing:\n{output}"
    );

    // 1. Store-only persistence: the store exists, the envelope was
    //    renamed, and NO fresh envelope was written back.
    assert!(
        tmp.join(".heal/trajectories.kv").exists(),
        "store not created"
    );
    assert!(
        tmp.join("memory.json.migrated").exists(),
        "envelope not renamed"
    );
    assert!(
        !envelope.exists(),
        "store-backend run must not rewrite the envelope"
    );

    // 2. The heal actually healed (the fixture lints are gone).
    let healed = std::fs::read_to_string(&lib).unwrap();
    assert!(
        healed.contains("{name}"),
        "format-arg not inlined:\n{healed}"
    );
    assert!(
        !healed.contains("return 42;"),
        "needless_return survived:\n{healed}"
    );

    // 3. The durable rows: dump via the binary (backend-agnostic export)
    //    and assert the migration rows + a CERTIFIED verdict row for the
    //    oracle-confirmed edit (037-E2 on the new backend).
    let (ok, dump) = heal(
        &[("RIIR_HEAL_TRAJ_ROOT", tmp.to_str().unwrap())],
        &["--dump-trajectories"],
    );
    assert!(ok, "dump failed:\n{dump}");
    let mut migrated_rows = 0;
    let mut certified = 0;
    for line in dump.lines().filter(|l| l.starts_with('{')) {
        let v: serde_json::Value = serde_json::from_str(line).expect("jsonl row");
        if v["last_run_id"] == "migrated" {
            migrated_rows += 1;
            // Stats verbatim: the b1 bucket carried 2 successes, b2 zero.
            let expect = if v["shape"] == "b1" { 2 } else { 0 };
            assert_eq!(
                v["success_count"], expect,
                "migrated stats must be verbatim"
            );
        }
        if v["tier"] == 0 && v["success_count"].as_u64().unwrap_or(0) >= 1 {
            let lint = v["lint_key"].as_str().unwrap_or_default();
            assert!(
                lint == "uninlined_format_args" || lint == "needless_return",
                "unexpected certified lint: {lint}"
            );
            certified += 1;
        }
    }
    assert_eq!(
        migrated_rows, 2,
        "both envelope buckets must import:\n{dump}"
    );
    assert!(certified >= 1, "no oracle-confirmed Certified row:\n{dump}");

    // 4. The debug query reads the new state.
    let (ok, stats) = heal(envs, &["--memory-stats"]);
    assert!(ok, "memory-stats failed:\n{stats}");
    assert!(
        stats.contains("MIGRATED — the active backend is the repo-local .heal/trajectories.kv"),
        "stats must name the migrated backend:\n{stats}"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// 038-T3's migration fence: the `.migrated` rename makes the import
/// fire EXACTLY once — a second persisting run finds no envelope, does not
/// re-migrate, and still opens + records on the store.
#[test]
#[ignore = "spawns cargo + clippy; run with the e2e above"]
fn migration_is_once_only() {
    let tmp = std::env::temp_dir().join(format!("heal038_once_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("src")).unwrap();
    std::fs::write(
        tmp.join("Cargo.toml"),
        "[package]\nname = \"heal038_once\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[lib]\npath = \"src/lib.rs\"\n",
    )
    .unwrap();
    std::fs::write(
        tmp.join("src/lib.rs"),
        "pub fn answer() -> u32 {\n    return 42;\n}\n",
    )
    .unwrap();
    let _ = Command::new("git").arg("init").arg(&tmp).output();
    let envelope = tmp.join("memory.json");
    std::fs::write(
        &envelope,
        r#"{"version":1,"trajectories":[{"lint_name":"m","lint_name_hash":1,"buggy_code":"b","fixed_code":"f","direction":[0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0],"success_count":0,"fail_count":0}]}"#,
    )
    .unwrap();
    let envs: &[(&str, &str)] = &[("RIIR_HEAL_MEMORY", envelope.to_str().unwrap())];
    let lib = tmp.join("src/lib.rs");
    let (ok, out1) = heal(envs, &["--fix", "--verify", lib.to_str().unwrap()]);
    assert!(ok, "run 1 failed:\n{out1}");
    assert!(out1.contains("migrated 1/1"), "run 1 must migrate:\n{out1}");

    // Re-plant the lint (the file was healed) + re-run: NO second migration
    // line (no envelope left), and the store still opens + records.
    std::fs::write(&lib, "pub fn answer() -> u32 {\n    return 42;\n}\n").unwrap();
    let (ok, out2) = heal(envs, &["--fix", "--verify", lib.to_str().unwrap()]);
    assert!(ok, "run 2 failed:\n{out2}");
    assert!(
        !out2.contains("migrated"),
        "migration must fire once:\n{out2}"
    );
    assert!(
        !Path::new(&envelope).exists(),
        "no fresh envelope may appear on store-backend runs"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}
