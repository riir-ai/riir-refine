//! Issue 067 T3 — plant the scratch same-rule exemplar store for the arm-G2
//! A/B.
//!
//! The A/B's discriminating subset needs ≥K=3 Certified SAME-RULE rows per
//! queried lint. The reachable-miss set is off-corpus by construction, so no
//! dev store carries that history yet (the T1 trigger watch is the
//! production-density path). This tool plants rows for exactly the lints
//! that have REAL ground-truth fixes — the `planted_fixes()` positive
//! controls' lints (ptr_arg, or_fun_call, redundant_clone,
//! box_collection) — with hand-varied representative shapes. Every row is
//! `success && verified` ⇒ Certified, so the T1 condition (K=3 Certified
//! same-rule) holds per planted lint.
//!
//! Honesty note for the bench record: the exemplar content is CONSTRUCTED
//! (real fix semantics, hand-varied shapes), not production store history.
//! The four uncovered reachable lints (large_enum_variant,
//! too_many_arguments, vec_init_then_push, + any others) intentionally get
//! no rows — arm G2 ≡ arm B byte-identically there, recorded per-row as
//! `growing_prime_channels_empty: true`.
//!
//! Run:
//!
//! ```sh
//! cargo run --features latent_retrieval --example plant_067_store -- /tmp/gprime_store
//! # then point the harness at it:
//! RIIR_REGATE_STORE_DIR=/tmp/gprime_store cargo run --features cf_workers_ai \
//!     --bin fixer_regate -- --arm DRY   # channels2 must read "live" for the 4 lints
//! ```

use riir_clippy::traj_store::TrajStore;

/// (`lint`, [(`shape`, `fix`); 3]) — the covered lints, each with 3 distinct
/// representative shapes (distinct shapes are load-bearing: the row key is
/// `traj:{lint}:{blake3_12(shape)}`, so duplicate shapes collapse to ONE
/// row and the K=3 condition would not hold).
const PLANTS: &[(&str, &[(&str, &str); 3])] = &[
    (
        "ptr_arg",
        &[
            (
                "pub fn total(v: &Vec<u8>) -> usize { v.len() }",
                "pub fn total(v: &[u8]) -> usize { v.len() }",
            ),
            (
                "pub fn sum(xs: &Vec<i64>) -> i64 { xs.iter().sum() }",
                "pub fn sum(xs: &[i64]) -> i64 { xs.iter().sum() }",
            ),
            (
                "pub fn first(s: &String) -> char { s.chars().next().unwrap() }",
                "pub fn first(s: &str) -> char { s.chars().next().unwrap() }",
            ),
        ],
    ),
    (
        "or_fun_call",
        &[
            (
                "pub fn cfg_or(k: &str) -> String { std::env::var(k).unwrap_or(default_name()) }\nfn default_name() -> String { \"x\".into() }",
                "pub fn cfg_or(k: &str) -> String { std::env::var(k).unwrap_or_else(default_name) }\nfn default_name() -> String { \"x\".into() }",
            ),
            (
                "pub fn n_or(n: Option<u32>) -> u32 { n.unwrap_or(fallback()) }\nfn fallback() -> u32 { 7 }",
                "pub fn n_or(n: Option<u32>) -> u32 { n.unwrap_or_else(fallback) }\nfn fallback() -> u32 { 7 }",
            ),
            (
                "pub fn port_or(cfg: Option<u16>) -> u16 { cfg.unwrap_or(parse_port()) }\nfn parse_port() -> u16 { 8080 }",
                "pub fn port_or(cfg: Option<u16>) -> u16 { cfg.unwrap_or_else(parse_port) }\nfn parse_port() -> u16 { 8080 }",
            ),
        ],
    ),
    (
        "redundant_clone",
        &[
            (
                "pub fn f(s: &str) -> String { s.to_string().clone() }",
                "pub fn f(s: &str) -> String { s.to_string() }",
            ),
            (
                "pub fn g(v: Vec<u8>) -> Vec<u8> { v.clone() }",
                "pub fn g(v: Vec<u8>) -> Vec<u8> { v }",
            ),
            (
                "pub fn h(t: (u32, u32)) -> u32 { let c = t.0.clone(); c }",
                "pub fn h(t: (u32, u32)) -> u32 { t.0 }",
            ),
        ],
    ),
    (
        "box_collection",
        &[
            (
                "pub fn f() -> Box<Vec<u8>> { Box::new(vec![1, 2]) }",
                "pub fn f() -> Vec<u8> { vec![1, 2] }",
            ),
            (
                "pub fn g() -> Box<String> { Box::new(String::from(\"x\")) }",
                "pub fn g() -> String { String::from(\"x\") }",
            ),
            (
                "pub struct H { items: Box<Vec<u32>> }\npub fn mk() -> H { H { items: Box::new(Vec::new()) } }",
                "pub struct H { items: Vec<u32> }\npub fn mk() -> H { H { items: Vec::new() } }",
            ),
        ],
    ),
];

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: plant_067_store <target-dir>  (the dir gets .refine/trajectories.kv)");
        std::process::exit(2);
    });
    let root = std::path::PathBuf::from(arg);
    std::fs::create_dir_all(&root).expect("create target dir");
    let (store, bad) = TrajStore::open(&root).expect("open store");
    assert_eq!(bad, 0, "fresh scratch store must have no corrupt rows");
    for (lint, pairs) in PLANTS {
        for (i, (shape, fix)) in pairs.iter().enumerate() {
            store
                .record(lint, shape, fix, true, true, &format!("067-t3-plant-{i}"))
                .expect("record plant row");
        }
    }
    println!(
        "planted {} lints x 3 Certified rows into {}",
        PLANTS.len(),
        root.display()
    );
    // Self-check: the T1 condition must hold per planted lint.
    let density = store.certified_density();
    for (lint, _) in PLANTS {
        let n = density.get(*lint).copied().unwrap_or(0);
        println!("  {lint}: {n} Certified (need >= 3)");
        assert!(n >= 3, "K=3 condition failed for {lint}");
    }
    println!("OK — K=3 same-rule Certified condition holds for every planted lint");
}
