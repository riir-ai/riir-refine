//! Bench 010 — katgpt ternary generation on the clippy task, M3 Max Metal.
//!
//! This is the arm Bench 009 was missing. Bench 009 compared llama.cpp against
//! the **retriever** (a corpus lookup), and Bench 585 compared a **raw GEMV
//! kernel**. Neither measured *our own inference stack generating a clippy fix*,
//! which is the only apples-to-apples counterpart to llama.cpp on this task.
//!
//! Both arms are given the **same prompt**, from the same fixture, in the same
//! session, and both report tok/s over the same token accounting:
//!
//! ```text
//! prefill tok/s = prompt_tokens / prefill_wall
//! decode  tok/s = generated_tokens / decode_wall
//! total   tok/s = (prompt + generated) / total_wall
//! ```
//!
//! Greedy (argmax) decode, matching llama.cpp's `--temp 0.0`.
//!
//! # Run
//!
//! ```sh
//! BONSAI_GGUF=/Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
//! CLIPPY_CASES=benches/fixtures/e2e_cases_seed.json \
//! DECODE_TOKENS=64 \
//!   cargo run --release --features ternary_inference \
//!   --example bench_clippy_ternary_tps
//! ```
//!
//! `DECODE_TOKENS` is capped low by default because this path is slow — the
//! point is a *rate*, not a complete fix. The rate is what the comparison needs.

#![allow(clippy::pedantic)]

use std::path::Path;
use std::time::Instant;

use riir_engine::deltanet::effective_rotary_dim;
use riir_engine::gguf_loader::{load_qwen_deltanet_ternary_weights_gguf, GgufFile};
use riir_engine::rope::RopeFreqTable;
use riir_engine::tokenizer::BpeTokenizer;

/// Clamp the native 262K context — it would allocate ~128 GiB of KV cache.
/// Clippy prompts are ~100 tokens, so 1024 is generous. Mirrors the value the
/// riir-ai Go driver uses (`GPU_FWD_BLOCK_SIZE`), so the two are comparable.
const GPU_FWD_BLOCK_SIZE: usize = 1024;

/// Same system framing the llama.cpp arm receives in
/// `scripts/bench_bonsai_vs_clippy_tokps.py`, so neither arm gets an easier task.
const SYSTEM: &str = "You are a Rust clippy auto-fixer. Given a clippy lint name and the offending code, output ONLY the corrected Rust code in a single ```rust code block. No explanation, no commentary.";

#[derive(serde::Deserialize)]
struct Case {
    lint: String,
    buggy: String,
    #[allow(dead_code)]
    want: String,
}

#[derive(serde::Deserialize)]
struct CaseFile {
    name: String,
    cases: Vec<Case>,
}

/// Greedy pick. `partial_cmp` (not `>`) because logits are `f32` and a NaN must
/// not silently win — `Ordering::Greater` is the only case that replaces.
fn argmax(v: &[f32]) -> Option<usize> {
    v.iter()
        .enumerate()
        .fold(None::<(usize, f32)>, |best, (i, &x)| match best {
            Some((_, b)) if x.partial_cmp(&b) != Some(std::cmp::Ordering::Greater) => best,
            _ => Some((i, x)),
        })
        .map(|(i, _)| i)
}

fn main() {
    let gguf_path = std::env::var("BONSAI_GGUF")
        .unwrap_or_else(|_| "../riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf".to_string());
    let cases_path = std::env::var("CLIPPY_CASES")
        .unwrap_or_else(|_| "benches/fixtures/e2e_cases_seed.json".to_string());
    let decode_tokens: usize = std::env::var("DECODE_TOKENS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(64);
    let n_cases: usize = std::env::var("N_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let case_file: CaseFile =
        serde_json::from_str(&std::fs::read_to_string(&cases_path).expect("read cases fixture"))
            .expect("parse cases fixture");
    eprintln!(
        "[bench010] fixture={} cases={} decode_tokens={decode_tokens}",
        case_file.name,
        case_file.cases.len()
    );

    // ── Load model (cost reported, never folded into the rate) ────────────
    eprintln!("[bench010] loading {gguf_path} ...");
    let t_load = Instant::now();
    let path = Path::new(&gguf_path);
    let (mut config, weights) =
        load_qwen_deltanet_ternary_weights_gguf(path).expect("load ternary weights");
    config.block_size = GPU_FWD_BLOCK_SIZE;
    let load_s = t_load.elapsed().as_secs_f64();
    assert!(
        weights.invariants_hold(),
        "ternary weight invariants violated — GGUF may be corrupt"
    );
    let vocab = config.vocab_size;
    eprintln!(
        "[bench010] weights loaded in {load_s:.1}s (n_layer={}, n_embd={}, vocab={vocab})",
        config.n_layer, config.n_embd
    );

    let gguf_header = GgufFile::open(path).expect("open gguf header");
    let tokenizer = BpeTokenizer::from_gguf(&gguf_header).expect("tokenizer from gguf");
    let _rope = RopeFreqTable::new(config.rope_theta, effective_rotary_dim(&config));

    // ── GPU-resident forward: the M3 Metal path (all activations stay on GPU) ──
    eprintln!("[bench010] initializing GPU-resident forward (CubeCL wgpu<msl>) ...");
    let t_gpu = Instant::now();
    let ctx = riir_gpu::CubeCLContext::new().expect("CubeCL Metal context");
    let mut fwd = riir_gpu::TernaryDeltanetGpuForward::new(&ctx, &config, &weights);
    let gpu_init_s = t_gpu.elapsed().as_secs_f64();
    eprintln!("[bench010] GPU-resident forward ready in {gpu_init_s:.1}s");

    println!("\n=== Bench 010 — katgpt ternary, clippy prompts, M3 Metal ===");
    println!("model load: {load_s:.1}s   gpu init: {gpu_init_s:.1}s   (both excluded from tok/s)");
    println!();
    println!(
        "{:<24} {:>7} {:>7} {:>10} {:>10} {:>10} {:>10}",
        "lint", "prompt", "decode", "prefill s", "decode s", "dec tok/s", "tot tok/s"
    );

    let mut rows = Vec::new();

    for case in case_file.cases.iter().take(n_cases) {
        // Same chat template the server applies via --jinja, and the same
        // system+user split the Python arm sends.
        let chat = format!(
            "<|im_start|>system\n{SYSTEM}<|im_end|>\n<|im_start|>user\nclippy lint: `{}`\n\nOffending code:\n```rust\n{}\n```\n\nCorrected code:<|im_end|>\n<|im_start|>assistant\n",
            case.lint, case.buggy
        );
        let tokens = tokenizer.encode(&chat);
        assert!(!tokens.is_empty(), "tokenizer produced no tokens");

        let block = config.block_size.max(1);
        let prompt_tokens: &[usize] = if tokens.len() >= block {
            &tokens[..block - 1]
        } else {
            &tokens[..]
        };

        fwd.reset_state();

        // ── Prefill: BATCHED simdgroup cmma path (the production DEFAULT-ON) ──
        // The old loop `for tok in prompt_tokens { forward_token(tok) }` was the
        // one-token-at-a-time GEMV path — the slow path Bench 010 measured at
        // 17.89 tok/s prefill (8.38x behind llama.cpp). The batched `prefill()` API
        // processes all prompt tokens in one call via `prefill_project` (GEMM,
        // not GEMV) using the simdgroup cmma kernel (Issue 628/637, DEFAULT-ON in
        // riir-gpu). Measured +42.5% (1.42×) prefill gain over the GEMV loop.
        //
        // The Issue 663 zero-copy RAW-Metal overlay (`metal_tensor_gemm`) is
        // compiled in but DEFAULT OFF — it REGRESSES in this harness config:
        //   - decode 17.2 → 0.37 tok/s (47× slower) — likely GPU memory pressure
        //     from the zerocopy weight cache at 27B model scale
        //   - prefill 25.0 → 13.4 tok/s (regression, not the +1.21× Issue 663 claims)
        //   - G1 FAILS on some cases (`needless_return` emits garbage tokens)
        // Issue 663's +1.21× was measured on an ISOLATED prefill bench, not the
        // full forward — does not reproduce at scale. ZEROCOPY=1 env var forces it
        // on for A/B re-investigation; default is OFF.
        let _ = riir_gpu::set_prefill_use_simdgroup(true);
        #[cfg(target_os = "macos")]
        if std::env::var("ZEROCOPY").unwrap_or_else(|_| "0".to_string()) == "1" {
            let _ = riir_gpu::set_prefill_use_metal_tensor_zerocopy(true);
        }

        let t_prefill = Instant::now();
        let mut last_logits = fwd.prefill(prompt_tokens);
        let prefill_s = t_prefill.elapsed().as_secs_f64();

        // ── Decode: greedy argmax, matching --temp 0.0 ────────────────────
        let t_decode = Instant::now();
        let mut generated: Vec<usize> = Vec::with_capacity(decode_tokens);
        let first = argmax(&last_logits[..vocab]).unwrap_or(0);
        generated.push(first);
        for _ in 1..decode_tokens {
            let cur = *generated.last().unwrap();
            fwd.set_input_token(&weights, cur);
            last_logits = fwd.forward_token(cur);
            let next = argmax(&last_logits[..vocab]).unwrap_or(0);
            if next == tokenizer.eos_id() {
                break;
            }
            generated.push(next);
        }
        let decode_s = t_decode.elapsed().as_secs_f64();

        let dec_tps = generated.len() as f64 / decode_s;
        let tot_tps = (prompt_tokens.len() + generated.len()) as f64 / (prefill_s + decode_s);

        println!(
            "{:<24} {:>7} {:>7} {:>10.2} {:>10.2} {:>10.3} {:>10.3}",
            case.lint,
            prompt_tokens.len(),
            generated.len(),
            prefill_s,
            decode_s,
            dec_tps,
            tot_tps
        );

        let text = tokenizer.decode(&generated);
        eprintln!("[bench010] {} emitted: {:?}", case.lint, text);
        rows.push((case.lint.clone(), prompt_tokens.len(), generated.len(), prefill_s, decode_s, dec_tps, tot_tps, text));
    }

    // ── Aggregate ─────────────────────────────────────────────────────────
    let n = rows.len() as f64;
    if n > 0.0 {
        let pt: usize = rows.iter().map(|r| r.1).sum();
        let dt: usize = rows.iter().map(|r| r.2).sum();
        let ps: f64 = rows.iter().map(|r| r.3).sum();
        let ds: f64 = rows.iter().map(|r| r.4).sum();
        println!();
        println!("aggregate over {} case(s):", rows.len());
        println!("  prefill : {pt} tok / {ps:.2} s = {:.3} tok/s", pt as f64 / ps);
        println!("  decode  : {dt} tok / {ds:.2} s = {:.3} tok/s", dt as f64 / ds);
        println!(
            "  total   : {} tok / {:.2} s = {:.3} tok/s",
            pt + dt,
            ps + ds,
            (pt + dt) as f64 / (ps + ds)
        );
    }

    // JSONL for the comparison script.
    let out = std::env::var("OUT_JSONL").unwrap_or_else(|_| "/tmp/clippy_ternary_tps.jsonl".into());
    let mut s = String::new();
    for (lint, pt, dt, ps, ds, dtps, ttps, text) in &rows {
        s.push_str(&format!(
            "{{\"lint\":\"{lint}\",\"prompt_tokens\":{pt},\"decode_tokens\":{dt},\"prefill_s\":{ps:.4},\"decode_s\":{ds:.4},\"decode_tok_per_sec\":{dtps:.4},\"total_tok_per_sec\":{ttps:.4},\"chars\":{}}}\n",
            text.len()
        ));
    }
    std::fs::write(&out, s).expect("write jsonl");
    println!("\nwrote {out}");
}
