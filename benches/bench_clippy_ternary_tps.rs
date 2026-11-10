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
//! # Diagnostic env vars (Issue 670 instruments, all default-inert)
//!
//! - `TRUNCATE_P=<n>` — truncate every prompt to n tokens (P-regime probe;
//!   both arms see the identical truncated input, so cross-arm bit-identity
//!   stays the valid G1 criterion even though the completion is odd).
//! - `LOGITS_PROBE=1` — print post-prefill top-2 logits + gap per case.
//! - `ZC_WARMUP=<n>` — throwaway prefill of n tokens before the case loop
//!   (zerocopy first-prefill hazard probe).
//!
//! # Run
//!
//! ```sh
//! BONSAI_GGUF=/Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
//! CLIPPY_CASES=benches/fixtures/e2e_cases_seed.json \
//! DECODE_TOKENS=64 \
//!   cargo run --release --features ternary_inference \
//!   --bench bench_clippy_ternary_tps -- --nocapture
//! ```
//!
//! `DECODE_TOKENS` is capped low by default because this path is slow — the
//! point is a *rate*, not a complete fix. The rate is what the comparison needs.
//!
//! # Speculative decode (Issue 665)
//!
//! `SPECULATIVE=1` switches the decode loop from sequential (one token per
//! forward) to speculative (n-gram drafter predicts K=2, GPU verifies in one
//! batched read). Reports acceptance rate (α) + spec rounds alongside tok/s.
//! Default OFF. `SPEC_CHECKPOINT=cpu` selects the CPU-side checkpoint path
//! (default: `gpu`, the CopyCubeCL zero-sync production path).
//!
//! **Measured on clippy prompts (2026-08-14, M3 Max, needless_return case):**
//! G1 PASS (bit-identical emitted text vs sequential) but **α=0.19-0.22** —
//! far below the α≥0.58 breakeven (Issue 665 T1) — decode 3.3-4.4× SLOWER
//! than sequential. The n-gram drafter cannot predict natural-language
//! `<think>` reasoning. Issue 665's 1.21× was measured on repetitive text;
//! it does NOT reproduce on this workload. NET-NEGATIVE here.
//!
//! **Re-validated 2026-08-14 post-Issue-669-landing (3 cases, landed HEAD):**
//! G1 PASS — speculative emits text identical to the sequential arms on all
//! 3 cases. Decode 6.66 tok/s vs sequential 15.2-17.0 (~2.3-2.6× slower,
//! Unity-load session — same net-negative direction). The corrected
//! bench_665 repetitive-text re-gate also measured net-negative (0.812×
//! GPU-sync) — see Issue 665's T6 CORRECTED section.
//!
//! **bench_665 off-by-one bug (found while wiring this):** the upstream
//! `bench_665_speculative_decode.rs` verify loop compares `argmax(all_logits[i])`
//! against `draft_tokens[i]`, but `all_logits[i]` predicts the position AFTER
//! `draft_tokens[i]` — the correct comparison is `all_logits[i-1]` (or the
//! pre-speculation logits for `d_0`). The buggy wiring produced G1 FAIL
//! (garbage tokens) + inflated α=0.13 on this workload; the corrected wiring
//! produces bit-identical output. bench_665's α=0.88 was measured with the
//! buggy verify on repetitive text where the off-by-one coincidentally matches.

#![allow(clippy::pedantic)]

use std::path::Path;
use std::time::Instant;

use riir_infer_core::deltanet::effective_rotary_dim;
use riir_infer_core::gguf_loader::{GgufFile, load_qwen_deltanet_ternary_weights_gguf};
use riir_infer_core::rope::RopeFreqTable;
use riir_infer_core::tokenizer::BpeTokenizer;

/// Speculative decode lookahead (K=2 is the sweet spot per Issue 665 T1).
const DRAFT_K: usize = 2;

/// Checkpoint implementation: `gpu` (CopyCubeCL zero-sync, production path)
/// or `cpu` (read_to_cpu + write_back, the pre-fix path). Default `gpu`.
fn gpu_checkpoint() -> bool {
    std::env::var("SPEC_CHECKPOINT")
        .map(|v| v != "cpu")
        .unwrap_or(true)
}

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
    let use_speculative: bool = std::env::var("SPECULATIVE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
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
    let ctx = riir_infer_gpu::CubeCLContext::new().expect("CubeCL Metal context");
    let mut fwd = riir_infer_gpu::TernaryDeltanetGpuForward::new(&ctx, &config, &weights);
    let gpu_init_s = t_gpu.elapsed().as_secs_f64();
    eprintln!("[bench010] GPU-resident forward ready in {gpu_init_s:.1}s");

    println!("\n=== Bench 010 — katgpt ternary, clippy prompts, M3 Metal ===");
    println!("model load: {load_s:.1}s   gpu init: {gpu_init_s:.1}s   (both excluded from tok/s)");
    println!(
        "decode path: {}",
        if use_speculative {
            format!("speculative (K={DRAFT_K}, Issue 665)")
        } else {
            "sequential".to_string()
        }
    );
    println!();
    if use_speculative {
        println!(
            "{:<24} {:>7} {:>7} {:>10} {:>10} {:>10} {:>10} {:>8} {:>6}",
            "lint",
            "prompt",
            "decode",
            "prefill s",
            "decode s",
            "dec tok/s",
            "tot tok/s",
            "alpha",
            "rounds"
        );
    } else {
        println!(
            "{:<24} {:>7} {:>7} {:>10} {:>10} {:>10} {:>10}",
            "lint", "prompt", "decode", "prefill s", "decode s", "dec tok/s", "tot tok/s"
        );
    }

    let mut rows = Vec::new();

    // Issue 670 probe: throwaway warmup prefill. The surviving ZEROCOPY
    // corruption hits ONLY the first zerocopy prefill after context init
    // (case-order A/B: whichever case runs first gets wrong logits; later
    // ones are bit-identical to simdgroup). If this warmup absorbs the
    // corruption, all real cases run clean — the workaround shape for the
    // opt-in feature.
    if std::env::var("ZC_WARMUP").is_ok() {
        let warmup_p: usize = std::env::var("ZC_WARMUP")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|v| *v > 0)
            .unwrap_or(16);
        fwd.reset_state();
        let t = Instant::now();
        let _ = fwd.prefill(&vec![1usize; warmup_p]);
        eprintln!(
            "[bench010] zerocopy warmup prefill ({warmup_p} tok) in {:.2}s",
            t.elapsed().as_secs_f64()
        );
    }

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

        // Issue 670 candidate B probe: optionally truncate the prompt to P
        // tokens (after the block clamp) so the A/B can exercise the P<64
        // regime the zerocopy gate now confines itself to. Both arms see the
        // IDENTICAL truncated input — the pass criterion is cross-arm
        // bit-identity of the emitted text, not output coherence (a prompt
        // cut mid-template completes oddly by construction).
        let prompt_tokens: Vec<usize> = std::env::var("TRUNCATE_P")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .map(|p| prompt_tokens[..p.min(prompt_tokens.len())].to_vec())
            .unwrap_or_else(|| prompt_tokens.to_vec());

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
        // compiled in but DEFAULT OFF — the T4 flush fix
        // (`MetalTensorZeroCopyGemm::dispatch` flush-first, landed 2026-08-14)
        // recovered the throughput half (decode 0.37 → ~13.6 tok/s, within
        // ~10-20% of default; prefill ~parity). The G1 half is CLOSED as a
        // negative result (Issue 670 T5, 2026-08-14): the surviving corruption
        // hits ONLY the FIRST zerocopy prefill after context init — case-order
        // A/B proved it follows POSITION, not input content, P, or FP regime
        // (whichever case runs first gets wrong logits; later prefills are
        // BIT-IDENTICAL to simdgroup, so the kernel numerics are exact). Not
        // cured by flush (T4), first-dispatch `client.sync()`, P-gating
        // (candidate B, refuted — corrupts at P=32 too), or a same-P warmup
        // at mixed P (downgrades, doesn't eliminate). Issue 663's +1.21× was
        // an ISOLATED prefill bench — no end-to-end gain exists at production
        // scale (prefill parity in every arm). ZEROCOPY=1 forces it on for
        // A/B re-investigation; default is OFF.
        //
        // Diagnostic env vars (Issue 670 instruments, all default-inert):
        //   TRUNCATE_P=<n>  truncate every prompt to n tokens (P-regime probe)
        //   LOGITS_PROBE=1  print post-prefill top-2 logits + gap per case
        //   ZC_WARMUP=<n>   throwaway prefill of n tokens before the case loop
        riir_infer_gpu::set_prefill_use_simdgroup(true);
        #[cfg(target_os = "macos")]
        if std::env::var("ZEROCOPY").unwrap_or_else(|_| "0".to_string()) == "1" {
            riir_infer_gpu::set_prefill_use_metal_tensor_zerocopy(true);
        }

        let t_prefill = Instant::now();
        let mut last_logits = fwd.prefill(&prompt_tokens);
        let prefill_s = t_prefill.elapsed().as_secs_f64();

        // Issue 670 root-cause probe: dump the post-prefill top-2 logits + gap.
        // The surviving ZEROCOPY corruption is deterministic PER INPUT and
        // P-INDEPENDENT (needless_return garbages at both P=93 and P=32; the
        // other two cases clean at both) — the signature of a fixed numerics
        // difference (e.g. the raw kernel's half-precision weight tile)
        // flipping a near-tie argmax, not FP non-determinism. A collapsed
        // top-2 gap + flipped top-1 vs the default arm confirms it.
        if std::env::var("LOGITS_PROBE").is_ok() {
            let mut top: Vec<(usize, f32)> = last_logits[..vocab]
                .iter()
                .enumerate()
                .map(|(i, &v)| (i, v))
                .collect();
            top.sort_by(|a, b| b.1.total_cmp(&a.1));
            let (t1i, t1v) = top[0];
            let (t2i, t2v) = top[1];
            eprintln!(
                "[probe] {} top1=({t1i}, {t1v:.4}) top2=({t2i}, {t2v:.4}) gap={:.4}",
                case.lint,
                t1v - t2v
            );
        }

        // ── Decode ────────────────────────────────────────────────────────
        // Two paths, selected by the SPECULATIVE env var:
        //
        // - Sequential (default): one token per forward. The G1 baseline.
        //   Matches llama.cpp's `--temp 0.0` greedy decode.
        //
        // - Speculative (SPECULATIVE=1): n-gram drafter predicts K=2 tokens,
        //   GPU verifies both in one batched logits read. Acceptance prefix +
        //   rollback on reject. Mirrors bench_665_speculative_decode.rs. Issue 665
        //   T6 measured 1.21× at α=0.88 on an isolated decode bench; this harness
        //   re-validates on the production-shaped clippy-prompt path.
        //
        // Both paths produce identical tokens on a given prompt (greedy argmax
        // both sides) — only the throughput differs.
        let t_decode = Instant::now();
        let (generated, acceptance, spec_rounds) = if use_speculative {
            decode_speculative(
                &mut fwd,
                &weights,
                &prompt_tokens,
                decode_tokens,
                tokenizer.eos_id(),
            )
        } else {
            let mut generated: Vec<usize> = Vec::with_capacity(decode_tokens);
            let first = argmax(&last_logits[..vocab]).unwrap_or(0);
            generated.push(first);
            for _ in 1..decode_tokens {
                let cur = *generated.last().unwrap();
                fwd.set_input_token(&weights, cur);
                last_logits = fwd.forward_token();
                let next = argmax(&last_logits[..vocab]).unwrap_or(0);
                if next == tokenizer.eos_id() {
                    break;
                }
                generated.push(next);
            }
            (generated, 0.0_f64, 0_usize)
        };
        let decode_s = t_decode.elapsed().as_secs_f64();

        let dec_tps = generated.len() as f64 / decode_s;
        let tot_tps = (prompt_tokens.len() + generated.len()) as f64 / (prefill_s + decode_s);

        if use_speculative {
            println!(
                "{:<24} {:>7} {:>7} {:>10.2} {:>10.2} {:>10.3} {:>10.3} {:>8.2} {:>6}",
                case.lint,
                prompt_tokens.len(),
                generated.len(),
                prefill_s,
                decode_s,
                dec_tps,
                tot_tps,
                acceptance,
                spec_rounds
            );
        } else {
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
        }

        let text = tokenizer.decode(&generated);
        eprintln!("[bench010] {} emitted: {:?}", case.lint, text);
        rows.push((
            case.lint.clone(),
            prompt_tokens.len(),
            generated.len(),
            prefill_s,
            decode_s,
            dec_tps,
            tot_tps,
            text,
        ));
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
        println!(
            "  prefill : {pt} tok / {ps:.2} s = {:.3} tok/s",
            pt as f64 / ps
        );
        println!(
            "  decode  : {dt} tok / {ds:.2} s = {:.3} tok/s",
            dt as f64 / ds
        );
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

/// Speculative decode loop (Issue 665).
///
/// Corrected implementation — the bench_665 verify logic was off-by-one
/// (`all_logits[i]` used to mean logits AFTER `draft_tokens[i]`, so verifying
/// `d_i` needed `all_logits[i-1]`, or a separate `read_logits()` for `d_0`).
/// bench_665 got α=0.88 on repetitive counting text where the off-by-one
/// coincidentally matches; on natural-language clippy prompts it collapses to
/// α=0.13 + G1 FAIL.
///
/// The substrate now returns K+1 position-aligned logits (Issue 669 follow-up):
/// `all_logits[i]` predicts `draft_tokens[i]`'s position for ALL i (including
/// i=0 — it re-reads the same GPU buffer a threaded `current_logits` would
/// have cloned), and `all_logits[K]` predicts the position after the last
/// draft. No per-round `read_logits()` sync, no threading.
///
/// Algorithm (per round, starting from GPU position P):
///
/// 1. Draft predicts `[d_0, ..., d_{K-1}]` for positions P, P+1, ..., P+K-1.
/// 2. `forward_speculative_verify` processes each draft token at its position,
///    returning K+1 position-aligned logits.
/// 3. Verify: `d_i` against `all_logits[i]` (predicts P+i). Accept matching
///    prefix.
/// 4. If all accepted: no rollback. P += K. `self.logits` already holds the
///    last forward's output → next round's `all_logits[0]` is the seed.
/// 5. If rejected at j: rollback to P. Re-apply accepted + corrected. P += j+1.
///    The re-apply's last forward writes `self.logits` → next round's
///    `all_logits[0]` is the seed.
///
/// No bonus token — committing `argmax(all_logits[K])` without processing it
/// misaligns GPU pos vs token positions (Issue 669). Accept at most K tokens
/// per round.
///
/// Returns `(generated_tokens, acceptance_rate, spec_rounds)`.
#[cfg(feature = "ternary_inference")]
fn decode_speculative(
    fwd: &mut riir_infer_gpu::TernaryDeltanetGpuForward,
    weights: &riir_infer_core::deltanet::ternary_weights::QwenDeltaNetTernaryWeights,
    prompt_tokens: &[usize],
    decode_tokens: usize,
    eos_id: usize,
) -> (Vec<usize>, f64, usize) {
    use riir_infer_gpu::speculative_decode::NgramDrafter;

    if decode_tokens == 0 {
        return (Vec::new(), 0.0, 0);
    }

    // Seed the drafter from the prompt n-grams.
    let mut drafter = NgramDrafter::default_trigram();
    let prompt_u32: Vec<u32> = prompt_tokens.iter().map(|&t| t as u32).collect();
    drafter.add_context(&prompt_u32);

    let mut generated: Vec<u32> = Vec::with_capacity(decode_tokens);

    let mut spec_rounds = 0usize;
    let mut total_accepted = 0usize;
    let mut total_draft = 0usize;

    while generated.len() < decode_tokens {
        spec_rounds += 1;

        // 1. Draft predicts K tokens for the next K positions.
        //    Context = prompt + generated so far (the drafter needs the suffix).
        let mut context: Vec<u32> = prompt_u32.clone();
        context.extend_from_slice(&generated);
        let draft: Vec<u32> = drafter.predict(&context, DRAFT_K);
        let draft_tokens: Vec<usize> = draft.iter().map(|&t| t as usize).collect();

        // 2. Checkpoint + speculative verify (K forwards, 1 batched logits read).
        //    GPU-side checkpoint (default): CopyCubeCL dispatches, zero CPU sync
        //    (~0.4ms for 160MB, overlaps with subsequent dispatches). CPU-side
        //    (SPEC_CHECKPOINT=cpu): batched read to CPU + write back on rollback.
        let use_gpu_ckpt = gpu_checkpoint();
        let checkpoint_pos = fwd.pos();
        let checkpoint = if use_gpu_ckpt {
            fwd.checkpoint_speculative_gpu();
            None
        } else {
            Some(fwd.checkpoint_speculative())
        };
        let all_logits = fwd.forward_speculative_verify(weights, &draft_tokens);
        // all_logits is K+1 position-aligned (Issue 669 follow-up):
        //   all_logits[i] predicts draft_tokens[i]'s position (i < K)
        //   all_logits[K] predicts the position after the last draft
        // all_logits[0] re-reads the GPU buffer that a threaded
        // `current_logits` would have cloned — bit-identical, no sync.

        // 3. Verify: accept matching prefix, reject at first divergence.
        let mut first_reject = DRAFT_K;
        for i in 0..DRAFT_K {
            total_draft += 1;
            let main_token = argmax(&all_logits[i]).unwrap_or(0);
            if main_token == draft_tokens[i] {
                total_accepted += 1;
                generated.push(draft_tokens[i] as u32);
                if generated.len() >= decode_tokens || draft_tokens[i] == eos_id {
                    first_reject = i + 1;
                    break;
                }
            } else {
                // Draft wrong — accept the main model's token instead.
                generated.push(main_token as u32);
                first_reject = i;
                break;
            }
        }

        // 4. Nothing to thread: both round-exit paths leave `self.logits`
        //    holding the current-position logits (full accept → the last
        //    verify forward's output; reject → the last re-apply forward's
        //    output), so the next round's all_logits[0] is the seed.

        // 5. Rollback + re-apply if any tokens were rejected.
        //    The GPU processed d_0..d_{K-1} during verify, but d_{first_reject}
        //    was wrong. Rollback restores pre-speculation state; re-apply the
        //    accepted prefix + corrected token.
        if first_reject < DRAFT_K {
            match (use_gpu_ckpt, checkpoint) {
                (true, None) => fwd.rollback_speculative_gpu(checkpoint_pos),
                (false, Some(cp)) => fwd.rollback_speculative(&cp),
                _ => unreachable!("checkpoint variant mismatch"),
            }
            let n_to_reapply = first_reject + 1; // accepted drafts + 1 corrected
            let reapply_tokens = &generated[generated.len() - n_to_reapply..];
            for &tok in reapply_tokens {
                fwd.set_input_token(weights, tok as usize);
                let _ = fwd.forward_token();
            }
        }

        // 6. Update the drafter with newly committed tokens.
        let update_len = generated.len().min(drafter.order());
        drafter.add_context(&generated[generated.len() - update_len..]);

        // 7. EOS check.
        if (*generated.last().unwrap() as usize) == eos_id {
            break;
        }
    }

    let gen_usize: Vec<usize> = generated.iter().map(|&t| t as usize).collect();
    let acceptance = if total_draft > 0 {
        total_accepted as f64 / total_draft as f64
    } else {
        0.0
    };
    (gen_usize, acceptance, spec_rounds)
}
