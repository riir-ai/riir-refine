//! The `ane` domain — Apple Neural Engine host-orchestration healing
//! (Plan 377; the owner's 2026-11-14 call).
//!
//! Seventh retrieval `HealDomain`. Scope: **ANE host-placement and
//! conversion decisions whose fix is a measured/verifiable graph or
//! placement property, not a CPU code pattern** — the decision content the
//! CoreML/ANE runtime judges (stateless fixed-window streaming, fp16
//! numerics guards, compute-unit placement, graph lowering shape,
//! precision-critical DSP edges, palette-depth retention gates).
//!
//! ## Provenance
//!
//! Seed corpus distilled from `shershah1024/liquidcodec-ane` @ `177f902`
//! (LFM Open License v1.0 — §5 commercial-use gate satisfied, disclosed) +
//! the corroborating second source `beshkenadze/
//! nemotron-3.5-asr-streaming-0.6b-coreml-ane-stream` README @ `3dabd79`.
//! Full verdict: `.research/409_LiquidCodec_ANE_Distill_Verdict.md` (the
//! six candidates R1–R6 with every source quote re-verified at the pin).
//!
//! ## Why a domain of its own (Plan 377)
//!
//! Research 409's first routing call put these rules in `kernel_opt` (the
//! Batch-337/357/358 ANE/Metal-graph precedent). The owner call converts
//! the routing: the ANE retrieval surface gets its own corpus, its own
//! demonstrability column, and — the substantive half — its own FIRST-PASS
//! code scan (`ane_pregate`, the `kernel_pregate` two-pass shape) so the
//! domain never fans out over non-ANE files. Existing `kernel_opt` ANE-family
//! rules do NOT move (corpus/Elo churn for zero retrieval gain); future ANE
//! distills route here.
//!
//! ## Suggest-only seed, honestly disclosed
//!
//! Every seed rule is **suggestion-only, `NotBenchable-by-domain`** (the win
//! metric lives in the CoreML/ANE runtime — placement residency, `RTFx`,
//! transcript retention — which no validator on this box hosts; the
//! domain-unbenchable annotation class, Batch 136/337/357/358 precedent).
//! The citation names the rule and its decision content; the `--fix` lane
//! declines it (no proposers ship in the seed). Each rule's `notes` carry
//! the source provenance + the regime condition; the measured lane cells
//! that will harden the regime rows live in riir-infer Plan 633 (the ANE
//! audio lane bench) — the same P4.2 dependency Plan 376 records.

use serde::{Deserialize, Serialize};

use crate::domain::{HealDomain, HealEntry};

pub mod corpus;

/// The rule family (parallel to `DistFamily` / `SecFamily`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AneFamily {
    /// Fixed-shape stateless windowing replacing a stateful streaming path.
    Streaming,
    /// fp16 numerics guards (prescale/overflow/subnormal) on ANE statistics.
    Numerics,
    /// Host compute-unit / placement decisions (CPU+ANE residency).
    Placement,
    /// Graph restructure that decides whether ops lower onto the ANE.
    GraphShape,
    /// Precision-critical DSP at graph edges (CPU fp32 tails).
    PrecisionDsp,
    /// Weight palette/quantization depth + retention gates.
    PaletteQuant,
}

impl AneFamily {
    /// Stable label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Streaming => "streaming",
            Self::Numerics => "numerics",
            Self::Placement => "placement",
            Self::GraphShape => "graph-shape",
            Self::PrecisionDsp => "precision-dsp",
            Self::PaletteQuant => "palette-quant",
        }
    }
}

/// One ANE rule: buggy/fixed Rust-shaped pseudocode pair + family +
/// decision-content notes (the source quotes + regime condition).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AneRule {
    /// The rule id (kebab-case, the house convention).
    pub rule_id: String,
    /// The concept in one line (the retrieval surface reads this).
    pub ane_concept: String,
    /// What textual shape triggers the rule, in prose.
    pub trigger: String,
    /// The buggy shape (the stateful / mis-placed / overflow-prone pattern).
    pub buggy_code: String,
    /// The fixed shape (the stateless / placed / guarded pattern).
    pub fixed_code: String,
    /// The family.
    pub family: AneFamily,
    /// Context annotation (the `context_notes` channel) — source provenance,
    /// regime condition, kinship adjudications, the `NotBenchable` disclosure.
    pub notes: Option<String>,
}

impl HealEntry for AneRule {
    #[inline]
    fn rule_id(&self) -> &str {
        &self.rule_id
    }

    #[inline]
    fn buggy_code(&self) -> &str {
        &self.buggy_code
    }

    #[inline]
    fn fixed_code(&self) -> &str {
        &self.fixed_code
    }

    #[inline]
    fn context_notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}

/// The parse-only verdict — the ANE patterns are Rust-shaped pseudocode of
/// the HOST side; there is no `CoreML` compiler on this box, so the verdict
/// is deliberately the L2 parse check and nothing stronger.
#[derive(Debug, Clone)]
pub struct AneVerdict {
    /// Whether the proposed pattern parses as valid Rust.
    pub parses: bool,
}

/// The Apple Neural Engine healing domain — `HealDomain` impl. Validator
/// verdict is parse-only (the domain-unbenchable class: the real oracle is
/// the CoreML/ANE runtime, out of reach here — disclosed in every rule's
/// notes, never simulated).
#[derive(Debug, Clone, Copy, Default)]
pub struct AneDomain;

impl HealDomain for AneDomain {
    type Entry = AneRule;
    type Verdict = AneVerdict;

    fn build_corpus() -> Vec<Self::Entry> {
        corpus::build_ane_seed_corpus().entries
    }

    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict {
        // The L2 pruner is domain-agnostic syntax validation (the kernel
        // scaffold's validate shape; the rule_id rides the unused slot).
        let pruner = crate::pruner::RustAnalyzerPruner::new();
        let result = pruner.validate(proposed_fix, rule_id);
        AneVerdict {
            parses: result.parses,
        }
    }
}
