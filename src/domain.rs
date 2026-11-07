//! The `HealDomain` trait — one pluggable code-healing domain.
//!
//! This is the **one new abstraction** introduced by Plan 001 (Proposal 001).
//! Everything else in the Phase 1 refactor is rename + reorganization. The
//! domain-agnostic core (drafter + pruner + corpus) operates on the corpus
//! entries this trait produces. The domain owns: (1) what counts as a fixable
//! rule, (2) how to validate a proposed fix against ground truth.
//!
//! ## Caveat (Proposal 001 caveat 1)
//!
//! The signature is **provisional**. The three domains (clippy, perf, kernel)
//! fit this shape NOW, but a fourth domain might not. Ship exactly 3 impls;
//! do not speculatively add methods for hypothetical domains.

use serde::Serialize;
use serde::de::DeserializeOwned;

/// A corpus entry that the domain-agnostic core can read through.
///
/// Each domain's `HealDomain::Entry` type implements this. The drafter reads
/// `fixed_code()`, the pipeline's `seed_from_corpus` reads all three, and the
/// ruliology search reads `fixed_code()` + `buggy_code()`. This is a thin
/// accessor trait — it carries no domain-specific semantics.
///
/// The `context_notes()` method (Plan 001 post-promotion, 2026-08-13) has a
/// default `None` impl so it is **non-breaking** — domains whose rules are
/// unconditional wins (clippy, `kernel_opt`) need no changes. The `rust_perf`
/// domain overrides it for the 3 annotated rules (2 `ParetoDominated` + 1 `Mixed`
/// per Bench 013) so the drafter can propagate the annotation through
/// [`DraftedFix`](crate::DraftedFix).
pub trait HealEntry {
    /// The rule identifier (clippy: lint name, perf: rule id, kernel: concept id).
    fn rule_id(&self) -> &str;

    /// The buggy code that triggers the rule.
    fn buggy_code(&self) -> &str;

    /// The fixed code that resolves the rule.
    fn fixed_code(&self) -> &str;

    /// Annotation for rules that are NOT default-configuration wins (Bench 013).
    ///
    /// `Some(notes)` means the consumer should treat this rule as
    /// **context-dependent** — the fix is a genuine pattern, but the bench
    /// validator found it is not unconditionally better than the buggy code in
    /// the default configuration (e.g. the compiler already optimizes the buggy
    /// form, or the benefit only appears under a specific runtime regime).
    ///
    /// Default `None` — most rules are unconditional wins. The drafter does NOT
    /// skip or downgrade annotated rules; it propagates the annotation through
    /// [`DraftedFix::context_notes`] so the consumer decides whether their
    /// context matches.
    fn context_notes(&self) -> Option<&str> {
        None
    }
}

/// One pluggable code-healing domain (clippy lints, rust perf, kernel opt, ...).
///
/// The domain-agnostic core (drafter + pruner + ruliology + self-evolve)
/// operates on the corpus entries this trait produces + the validator verdict
/// it returns. The domain owns: (1) what counts as a fixable rule, (2) how to
/// validate a proposed fix against ground truth.
///
/// **Signature is provisional** (Proposal 001 caveat 1) — ship exactly 3 impls,
/// document that it may change, revisit if a fourth domain forces it.
pub trait HealDomain: Sized {
    /// Corpus entry shape: (`rule_id`, `buggy_code`, `fixed_code`, metadata).
    /// Metadata carries domain-specific signal (`lint_group`, `perf_metric`,
    /// `kernel_shape_dtype_hw`, ...).
    type Entry: Clone + Send + Sync + Serialize + DeserializeOwned + HealEntry + 'static;

    /// Validator verdict: did the proposed fix satisfy the domain's ground truth?
    /// Stronger than "parses" — clippy re-runs clippy, perf runs tests+bench,
    /// kernel runs numeric compare vs reference.
    type Verdict: Clone + Send + Sync;

    /// Build the seed corpus for this domain (BLAKE3-committed, freeze/thaw-ready).
    /// Mirrors `build_seed_corpus()` for clippy; each domain ships its own.
    fn build_corpus() -> Vec<Self::Entry>;

    /// Validate a proposed fix against ground truth.
    ///
    /// This is the **Stage 2 tuning-phase validator** (NOT the hot-path guard).
    /// The tuning loop calls this OFF the hot path to build the Stage 2
    /// adaptive drafter pool. clippy re-runs clippy (~130 ms), perf runs
    /// tests+bench (seconds), kernel runs numeric compare vs reference.
    fn validate(rule_id: &str, proposed_fix: &str) -> Self::Verdict;
}

/// The span-shape vocabulary for shape-keyed rule dispatch (Issue 216, the
/// codegram POC — the categorical half; issue 217 T0 owns the ORDERED band
/// axis beside it).
///
/// A [`SpanShapeSet`] classifies ONE retrieval chunk (an [`AstChunker`]
/// fn/impl/struct span) so an annotated rule can leave the fan-out on spans
/// whose shape cannot match. The set is a hand-rolled u16 bitflags — no dep,
/// zero-alloc, `Copy`.
///
/// Shape bits live here (the neutral trait home) so the domain-side
/// annotation table (`rust_perf::corpus::span_shape_annotations`) and the
/// draft-side classifier (`draft::latent_matcher::span_shape`) can both
/// name it without a domains→draft or draft→draft layer inversion. The
/// draft→domains import direction is the established one
/// (`instantiation_rerank`, `headroom_probe`).
///
/// [`AstChunker`]: https://docs.rs/riir-rag/latest/riir_rag/ast_chunker/struct.AstChunker.html
#[cfg(feature = "span_shape_dispatch")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanShapeSet(u16);

#[cfg(feature = "span_shape_dispatch")]
impl SpanShapeSet {
    /// The fail-open verdict: no usable parse (`RerankMode::Off`) or an
    /// unparseable/unrecognized chunk — every rule admits (byte-identical to
    /// the ungated fan-out).
    pub const ALL: Self = Self(u16::MAX);

    /// The empty set (no shape bit set — never a verdict on its own; the
    /// classifier folds `ALL` over an empty fold so no chunk classifies
    /// empty).
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Fold one shape into the set (the classifier's accumulator step).
    #[must_use]
    pub const fn with(mut self, shape: SpanShape) -> Self {
        self.0 |= shape.bit();
        self
    }

    /// True when any shape is shared. An empty annotated set never happens
    /// (table rows carry >= 1 bit), so `ALL.intersects(anything non-empty)`
    /// is true — the fail-open contract falls out of the algebra.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

/// One categorical span shape (Issue 216). The SYN-derived bits read the
/// chunk's own ROOT items (an `AstChunker` chunk IS an item span, so the
/// Issue-063 shared parse already holds them); the needle-derived bits read
/// body-level signals the root items do not surface without a deep walk.
/// Widening this enum = a new registration (the kernel pregate needle-set
/// law): every widening re-pins the golden classifier test + re-runs the
/// G2 cost leg before promotion.
#[cfg(feature = "span_shape_dispatch")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpanShape {
    /// A root `fn` item.
    FnDecl = 0,
    /// A root `fn` carrying `#[test]`.
    TestFn,
    /// A root `async fn`.
    AsyncFn,
    /// A root `impl` block (inherent or trait — merged v1).
    ImplBlock,
    /// A root `trait` declaration.
    TraitDecl,
    /// A root `struct`/`enum`/`union` declaration (layout-rules' shape).
    TypeDecl,
    /// A root `mod` block.
    ModBlock,
    /// A root item-level macro (`macro_rules!`, derive-adjacent items).
    MacroItem,
    /// Body-level needle hit: `for ` / `while ` / `loop {` (over-admits —
    /// a hit inside a string/comment still says `HotLoop`; the mask is
    /// recall-biased by design).
    HotLoop,
    /// Body-level needle hit: `match ` (same over-admit discipline).
    MatchBlock,
}

#[cfg(feature = "span_shape_dispatch")]
impl SpanShape {
    const fn bit(self) -> u16 {
        1 << (self as u16)
    }
}
