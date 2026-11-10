//! End-to-end demo of the kernel_opt healing pipeline with expert routing.
//!
//! This is the **composition example** that closes the Plan 003/004 gap: the
//! `KernelExpertRouter` (15 category experts, 8-dim activity vector) was fully
//! corpus-wired and 15/15 reachable, but had no runnable code path that actually
//! called it. This example demonstrates the intended composition:
//!
//! 1. Build the seed corpus (68 rules across 15 categories).
//! 2. Build the expert router (15 category centroids).
//! 3. For each representative kernel signature: route to an expert, narrow the
//!    candidate set to that expert's category rules, then draft a fix via the
//!    ternary SIMD matvec drafter (L1) or the corpus template fallback (L0).
//! 4. Validate each drafted fix via the domain's L2 validator.
//! 5. Show the thermal tier for each signature (Plasma → Hot → ... → Freeze).
//!
//! The routed heal narrows the candidate search from 68 rules (flat) to ~4–6
//! rules (one category) — a 10–15× candidate reduction that makes both the L0
//! template path and the L1 matvec path more targeted.
//!
//! Run with:
//! ```sh
//! cargo run --example kernel_opt_demo --features kernel_opt
//! cargo run --example kernel_opt_demo --features kernel_verify  # with CubeCL compile
//! ```
//!
//! ## What this demonstrates
//!
//! - **Router narrowing**: the consumer provides a kernel signature via
//!   [`build_activity`], the router maps it to the relevant category expert,
//!   and the heal flow searches only that category's rules.
//! - **Domain-agnostic drafter**: [`TernaryDrafter`] operates on any
//!   [`HealEntry`] — it doesn't know about kernel categories. The router does
//!   the narrowing; the drafter does the drafting.
//! - **Thermal escalation**: the [`KernelThermalRouter`] reports which tier a
//!   signature would need. The scaffold always returns Plasma (modelless-first
//!   mandate); Warm/Cold/Freeze require 4090 CUDA + LoRA (deferred).

use riir_refine::domain::HealDomain;
use riir_refine::domains::kernel_opt::KernelCategory;
use riir_refine::{
    KernelExpertRouter, KernelOptDomain, KernelRule, KernelThermalRouter, TernaryDrafter,
    build_activity, build_kernel_seed_corpus,
};

/// A representative kernel signature for the demo.
struct Signature {
    /// Human-readable label.
    label: &'static str,
    /// Activity vector axes: M, N, K, dtype, hw (all normalized to [0, 1]).
    shape: (f32, f32, f32, f32, f32),
    /// Operation-class one-hot flags.
    flags: (bool, bool, bool),
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║   riir-clippy — kernel_opt Routed Healing Pipeline Demo     ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // ── L0: Build the corpus ─────────────────────────────────────────────
    let corpus = build_kernel_seed_corpus();
    println!(
        "📦 L0 Corpus: {} rules (BLAKE3 verified: {})",
        corpus.entries.len(),
        if corpus.verify() { "✅" } else { "❌" }
    );
    let categories = riir_refine::build_corpus_categorized();
    println!("   Categories: {}", categories.len());
    for (cat, rules) in &categories {
        println!("     {} — {} rules", cat.as_str(), rules.len());
    }
    println!();

    // ── Router + Thermal ──────────────────────────────────────────────────
    let router = KernelExpertRouter::new();
    let thermal = KernelThermalRouter::new();
    println!(
        "🔀 Router: {} experts, {}-dim activity vector",
        riir_refine::NUM_KERNEL_EXPERTS,
        riir_refine::ACTIVITY_DIM,
    );
    println!("🌡️  Thermal: Plasma-first (Warm/Cold/Freeze deferred to 4090)");
    println!();

    // ── L1: Set up the drafter ────────────────────────────────────────────
    // Small deterministic weights for the demo. Production would load frozen
    // direction vectors from .bits files.
    let weights = demo_weights();
    let mut drafter = TernaryDrafter::with_weights(weights);
    println!("⚡ L1 Drafter: ternary SIMD matvec (weights: 8 rows × 16 cols)");
    println!();

    // ── Representative kernel signatures ──────────────────────────────────
    // Each signature is chosen to route to a DIFFERENT category expert,
    // demonstrating that the router discriminates kernel classes by shape +
    // operation-class flags. The activity vectors are close to the canonical
    // routing vectors from the `per_category_canonical_routing` test.
    let signatures = [
        Signature {
            label: "Naive matvec (strided read → coalescing)",
            // High M, low everything else → category A (memory access)
            shape: (0.9, 0.05, 0.0, 0.0, 0.0),
            flags: (false, false, false),
        },
        Signature {
            label: "Full flash-attention (seq×head_dim×head_dim)",
            // All dims high + is_attention → category F (flash-attn tiling)
            shape: (1.0, 1.0, 1.0, 0.0, 0.0),
            flags: (true, false, false),
        },
        Signature {
            label: "Fused quantized GEMM (quant+matmul)",
            // is_fusion flag → category C (fusion)
            shape: (0.0, 0.0, 0.0, 0.5, 0.5),
            flags: (false, true, false),
        },
        Signature {
            label: "Conv2d 1×1 (layout transform + batch pack)",
            // Moderate M, high N, high dtype → category L (conv-as-GEMM + layout)
            shape: (0.25, 0.9, 0.0, 1.0, 0.0),
            flags: (false, false, false),
        },
        Signature {
            label: "Compile-time mul-by-pow2 (IR strength reduction)",
            // is_ir_transform flag → category O (IR strength reduction)
            shape: (0.0, 0.0, 0.0, 0.0, 0.0),
            flags: (false, false, true),
        },
        Signature {
            label: "Varlen packing (cu_seqlens + chunk-reset cumsum)",
            // High M, high dtype, NO is_attention → category J (varlen + chunked reduction)
            shape: (1.0, 0.0, 0.0, 1.0, 0.0),
            flags: (false, false, false),
        },
        Signature {
            label: "Compute-bound inner-product (K-dominated)",
            // High K only → category B (compute)
            shape: (0.0, 0.0, 1.0, 0.0, 0.0),
            flags: (false, false, false),
        },
        Signature {
            label: "Autotune dispatch (dtype×hw grid search)",
            // High dtype + hw → category K (autotune dispatch)
            shape: (0.0, 0.0, 0.0, 1.0, 1.0),
            flags: (false, false, false),
        },
    ];

    // ── Routed heal pipeline ──────────────────────────────────────────────
    println!("─── Routed Heal Pipeline ──────────────────────────────────────");
    println!();

    let mut total_routed = 0usize;
    let mut total_drafted = 0usize;
    let mut total_validated = 0usize;

    for sig in &signatures {
        total_routed += 1;
        let (m, n, k, dtype, hw) = sig.shape;
        let (is_attn, is_fusion, is_ir) = sig.flags;
        let activity = build_activity(m, n, k, dtype, hw, is_attn, is_fusion, is_ir);

        // Route → expert → category
        let expert = router.route(&activity);
        let expert_name = router.route_named(&activity);
        let category =
            KernelExpertRouter::category_for_expert(expert).unwrap_or(KernelCategory::MemoryAccess);
        let rule_ids = KernelExpertRouter::rules_for_expert(expert);

        // Narrow candidates: only rules in the routed category
        let candidates: Vec<&KernelRule> = corpus
            .entries
            .iter()
            .filter(|e| rule_ids.contains(&e.rule_id.as_str()))
            .collect();

        // Thermal tier (scaffold: always Plasma — the modelless-first tier)
        let tier = thermal.route(true, true, 0.9);

        println!("🔑 Signature: {}", sig.label);
        println!(
            "   Routed to: expert {} ({}) — category: {}",
            expert,
            expert_name,
            category.as_str()
        );
        println!(
            "   Candidates: {}/{} rules (narrowed by router)",
            candidates.len(),
            corpus.entries.len()
        );
        println!("   Thermal:    {:?}", tier);

        if candidates.is_empty() {
            println!("   ⚠️  No candidates in category — skipping");
            println!();
            continue;
        }

        // Draft a fix from the narrowed candidate set.
        // Use the first candidate's ref_shape as the "buggy input" (simulating
        // a real kernel the consumer wants to optimize). In production, this
        // would come from profiling output or a user-submitted kernel.
        let rule_id = &candidates[0].rule_id;
        let buggy_code = &candidates[0].ref_shape;

        // L1: Draft using ternary matvec on the narrowed set
        let candidates_owned: Vec<KernelRule> = candidates.iter().map(|r| (*r).clone()).collect();
        let fix = drafter.draft_fix(rule_id, buggy_code, &candidates_owned);
        match &fix {
            Some(f) => {
                total_drafted += 1;
                // L2: Validate via the domain's validator
                let verdict = KernelOptDomain::validate(rule_id, &f.code);

                // The verdict type differs by feature:
                // - kernel_opt (default): KernelVerdict { parses }
                // - kernel_verify: KernelVerifyResult { compiles, ran, error }
                #[cfg(not(feature = "kernel_verify"))]
                let is_valid = verdict.parses;
                #[cfg(feature = "kernel_verify")]
                let is_valid = verdict.compiles;

                let status = if is_valid { "✅ VALID" } else { "❌ INVALID" };
                total_validated += if is_valid { 1 } else { 0 };

                println!(
                    "   Drafted:   {} [{:?}] conf={:.4}",
                    rule_id, f.source, f.confidence
                );
                println!("   Validate:  {}", status);

                // Show a brief diff preview
                let buggy_first = buggy_code.lines().take(1).collect::<Vec<_>>().join("");
                let fixed_first = f.code.lines().take(1).collect::<Vec<_>>().join("");
                println!("   Buggy:     {}", buggy_first);
                println!("   Fixed:     {}", fixed_first);
            }
            None => {
                println!("   ⚠️  No fix drafted (identity guard or empty candidates)");
            }
        }
        println!();
    }

    // ── Summary ──────────────────────────────────────────────────────────
    println!("─── Summary ────────────────────────────────────────────────────");
    println!("  Signatures routed: {}", total_routed);
    println!("  Fixes drafted:     {} (L1 ternary matvec)", total_drafted);
    println!(
        "  Fixes validated:   {} (L2 domain validator)",
        total_validated
    );
    println!();

    // ── Flat vs routed comparison ────────────────────────────────────────
    println!("─── Flat vs Routed Candidate Count ─────────────────────────────");
    println!(
        "  Flat corpus:     {} rules searched per heal",
        corpus.entries.len()
    );
    let avg_category_size: f64 =
        categories.iter().map(|(_, r)| r.len()).sum::<usize>() as f64 / categories.len() as f64;
    println!(
        "  Routed (avg):    {:.1} rules searched per heal (category avg)",
        avg_category_size
    );
    println!(
        "  Reduction:       {:.1}×",
        corpus.entries.len() as f64 / avg_category_size
    );
    println!();

    // ── Fallback mode (L0 corpus template, no weights) ───────────────────
    println!("─── Fallback Mode (L0 corpus template, no weights) ────────────");
    let mut fallback = TernaryDrafter::fallback();
    // Route a varlen-packing signature (category J)
    let activity = build_activity(1.0, 0.0, 0.0, 1.0, 0.0, false, false, false);
    let expert = router.route(&activity);
    let rule_ids = KernelExpertRouter::rules_for_expert(expert);
    let candidates: Vec<KernelRule> = corpus
        .entries
        .iter()
        .filter(|e| rule_ids.contains(&e.rule_id.as_str()))
        .cloned()
        .collect();
    if !candidates.is_empty() {
        let rule_id = &candidates[0].rule_id;
        let buggy = &candidates[0].ref_shape;
        if let Some(fix) = fallback.draft_fix(rule_id, buggy, &candidates) {
            println!("  Routed to expert {} → first rule: {}", expert, rule_id);
            println!("  Source:   {:?}", fix.source);
            println!("  Conf:     {:.4} (template default)", fix.confidence);
        }
    }
    println!();

    println!("✅ Routed healing pipeline demo complete.");
}

/// Build small deterministic ternary weights for the demo.
///
/// Same pattern as `heal_demo.rs`. Production would load frozen direction
/// vectors from `.bits` files.
fn demo_weights() -> katgpt_core::TernaryWeights {
    let rows = 8;
    let cols = 16;
    let mut weights = vec![0.0f32; rows * cols];
    for i in 0..rows {
        for j in 0..cols {
            let idx = i * cols + j;
            weights[idx] = match (i + j) % 3 {
                0 => 1.0,
                1 => -1.0,
                _ => 0.0,
            };
        }
    }
    katgpt_core::TernaryWeights::quantize_from_f32(&weights, rows, cols)
}
