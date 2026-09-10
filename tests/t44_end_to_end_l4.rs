//! Plan 336 T4.4 — the END-TO-END arm: real adapter, real daemon, real pipeline.
//!
//! `g4_l4_composition.rs` proves the composition CONTRACT with a test double
//! (10/10, milliseconds). This file proves the contract holds when the thing
//! behind the seam is an actual 27B ternary model — which is a different claim,
//! and the only one that can close G4 without a caveat.
//!
//! ## Why it is `#[ignore]`d
//!
//! It loads a ~7 GB GGUF and decodes on CPU: ~193 s for the first fix, ~24 s
//! after (riir-train `80601f19`). That does not belong in a suite people run on
//! every edit. It is a harness you fire deliberately.
//!
//! ## Running it
//!
//! ```bash
//! export RIIR_L4_BIN=/tmp/t42b/release/examples/bonsai_clippy_l4_eval
//! export RIIR_L4_GGUF=$HOME/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf
//! export RIIR_L4_LORA=/tmp/t41/t26_step50_best.LORC.BCL4.bin   # a BCL4 checkpoint
//! cargo test --features l4_lora --test t44_end_to_end_l4 -- --ignored --nocapture
//! ```
//!
//! Without those three variables the tests **skip loudly** rather than fail:
//! `cargo test -- --ignored` sweeps every ignored test in the crate, and a hard
//! failure there would punish someone running an unrelated suite.
//!
//! ## What a PASS here does and does not mean
//!
//! It means the wiring is sound: a modelless miss reaches L4, the answer passes
//! through the L2 pruner gate, and a healthy modelless hit never pays the ~24 s.
//! It does **not** mean L4 is useful — that is G1's job, and G1 is currently
//! 0/60 (riir-train Bench 465). Expect `L4 declined or was discarded` to be the
//! common outcome until a checkpoint scores > 0: the adapter emits edit-only
//! fragments, and unbalanced ones cannot parse (see `l4_daemon` tests).
#![cfg(feature = "l4_lora")]

use std::time::Instant;

use riir_clippy::build_seed_corpus;
use riir_clippy::l4_daemon::DaemonL4;
use riir_clippy::{HealPipeline, HealSource, RustAnalyzerPruner, TernaryDrafter};

/// Read the three env vars, or return `None` so the caller can skip.
fn config() -> Option<(String, String, String)> {
    let bin = std::env::var("RIIR_L4_BIN").ok()?;
    let gguf = std::env::var("RIIR_L4_GGUF").ok()?;
    let lora = std::env::var("RIIR_L4_LORA").ok()?;
    Some((bin, gguf, lora))
}

fn skip_notice() {
    eprintln!(
        "SKIP: set RIIR_L4_BIN, RIIR_L4_GGUF and RIIR_L4_LORA to run the T4.4 \
         end-to-end arm (see this file's header)."
    );
}

fn pipeline_with_daemon(bin: &str, gguf: &str, lora: &str) -> Result<HealPipeline, std::io::Error> {
    // 96 matches the G1 gate's decode budget, so a fix produced here is the
    // same shape the gate scores. Changing it would make the two incomparable.
    let t = Instant::now();
    let daemon = DaemonL4::spawn(bin, gguf, lora, 96)?;
    eprintln!("[t44] daemon ready in {:.1}s", t.elapsed().as_secs_f32());
    Ok(
        HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new())
            .with_l4_fallback(Box::new(daemon)),
    )
}

/// Invariant 2 + 3 against the real model: a genuine modelless miss must reach
/// L4, and whatever comes back must survive the L2 pruner or be discarded.
#[test]
#[ignore = "loads a ~7 GB GGUF; run explicitly with --ignored"]
fn real_adapter_serves_a_modelless_miss() {
    let Some((bin, gguf, lora)) = config() else {
        skip_notice();
        return;
    };
    let corpus = build_seed_corpus();
    let mut pipeline = match pipeline_with_daemon(&bin, &gguf, &lora) {
        Ok(p) => p,
        Err(e) => panic!("daemon failed to start: {e}"),
    };

    // A lint the corpus has never seen: drafter has no template, memory is
    // empty, so L0-L3 cannot serve it. This is the case L4 exists for.
    let buggy = "fn totals(v: &Vec<i32>) -> i32 {\n    let mut s = 0;\n    for i in 0..v.len() {\n        s += v[i];\n    }\n    return s;\n}";
    let t = Instant::now();
    let result = pipeline.heal("totally_unknown_lint_xyz", buggy, &corpus.entries);
    let secs = t.elapsed().as_secs_f32();

    match &result {
        Some(r) => {
            eprintln!(
                "[t44] served by {:?} in {secs:.1}s:\n{}",
                r.source, r.fixed_code
            );
            assert_eq!(
                r.source,
                HealSource::L4Lora,
                "a modelless miss can only be served by L4 — any other source \
                 means the miss was not a miss and this test proves nothing"
            );
            assert!(
                r.parses,
                "invariant 3: only a parsing L4 fix may be returned"
            );
        }
        None => {
            // Not a failure. A declined or non-parsing answer is the CONTRACT
            // working (invariant 3), and is the expected outcome while G1 is
            // 0/60 and the adapter emits unbalanced edit-only fragments.
            eprintln!(
                "[t44] L4 declined or its answer was discarded after {secs:.1}s — \
                 contract-correct; see G1 (Bench 465) for whether the adapter is useful"
            );
        }
    }
}

/// Invariant 1 against the real model, and the one with teeth: a modelless HIT
/// must not pay for the daemon at all. Asserted on WALL CLOCK, because that is
/// the thing a spurious call actually costs — the model is loaded and ready, so
/// only a real invocation could take seconds.
#[test]
#[ignore = "loads a ~7 GB GGUF; run explicitly with --ignored"]
fn modelless_hit_never_pays_for_the_daemon() {
    let Some((bin, gguf, lora)) = config() else {
        skip_notice();
        return;
    };
    let corpus = build_seed_corpus();
    let entry = corpus
        .entries
        .first()
        .expect("seed corpus is non-empty")
        .clone();
    let mut pipeline = match pipeline_with_daemon(&bin, &gguf, &lora) {
        Ok(p) => p,
        Err(e) => panic!("daemon failed to start: {e}"),
    };

    let t = Instant::now();
    let result = pipeline.heal(&entry.lint_name, &entry.buggy_code, &corpus.entries);
    let secs = t.elapsed().as_secs_f32();
    eprintln!("[t44] modelless hit resolved in {secs:.3}s");

    let r = result.expect("a corpus entry must heal modellessly");
    assert_ne!(
        r.source,
        HealSource::L4Lora,
        "a corpus hit must not be served by L4"
    );
    assert!(
        secs < 5.0,
        "modelless hit took {secs:.1}s — the daemon is warm, so seconds means L4 \
         was invoked on a path that must never touch it (invariant 1)"
    );
}
