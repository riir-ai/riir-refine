//! Batch 412 — the mlxfast staged pool's `kernel_opt` member (Plan 411 /
//! Research 445): the Research 437 staged candidates, mined at the pin,
//! Layr-Labs/mlxfast-bonsai2-27b-engine @ `0c7c3923` (MIT — the SAME pin
//! Batch 402 mined). The staged name `route-selection-frozen-at-
//! construction` lives in the vendored model-finalizer surface, NOT the
//! staged start file (Transform.swift carries only the thinner
//! precedence-ordered family detection) — the label was a stage pointer,
//! the mechanism lives where the construction gate is.
//!
//! `kernel_opt` 1125 → 1126: one rule —
//! - `route-bound-at-construction-hot-closure-shape-predicate-only`
//!   (Dispatch) — the optimized-vs-stock route question is answered ONCE
//!   per model load at a construction gate; the bound router is an
//!   immutable value whose hot path carries ONLY the data-dependent
//!   shape predicate (the rows compare); ineligible binds are permanent
//!   (no mid-run upgrade, no fallback-then-retry), and construction
//!   state is task-local so concurrent loads cannot mix routes.
//!
//! DROPPED at T1 dedup (recorded in Research 445):
//! `per-step-shared-scalars-host-advanced-one-array` — ALREADY OWNED by
//! `host-mirror-shared-scalar-advance-one-time-verdict` (`rust_perf`
//! `entries_10`, Batch 182) mined from the SAME repo lineage; the staged
//! label was its restatement (the Batch-409 prior-coverage lesson:
//! re-grep at claim time, never mine from the staging text).
//!
//! Fixtures: `corpus/fixtures/b412_fixtures.rs` — standalone-compiled
//! AND executed BEFORE entry (debug AND -O, 4/4). NOTHING INVENTED —
//! every quote read at the clone at the pin this session.
#![allow(dead_code)]

use crate::domains::kernel_opt::KernelRule;

/// Batch 412 — the mlxfast staged pool kernel rule.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn entries() -> Vec<KernelRule> {
    vec![
        // ── R1 — the construction-time route bound: eligibility is
        // answered once at bind; the hot closure carries only the shape
        // predicate. ──
        KernelRule {
            rule_id: "route-bound-at-construction-hot-closure-shape-predicate-only".to_string(),
            gpu_concept: "MECHANISM (construction-time route binding; category D — dispatch, \
the route-installation family): the optimized-vs-stock lane question is answered ONCE per \
model load at a construction gate — profile eligibility (a task-local installation state) \
plus the geometry contract (hidden/experts/topK/normalize equality) — and returns either \
the stock closure or the optimized closure as an IMMUTABLE value; the gate's else branch \
is permanent stock (the file contract: 'Model forwards never parse configuration, read \
the environment, or recover from an ineligible optimized dispatch'), and what remains on \
the hot path is exactly ONE data-dependent predicate — the logical-M rows compare (rows \
1..=16 the installed Metal lane, wider values the explicit prefill route). THE SOURCE, \
verbatim at the pin (Vendor/mlx-swift-lm/Libraries/MLXLLM/Models/Qwen35A3BOptimization.swift:\
589-592): '/// Bind either the unchanged stock finalizer or the exact row-owned decode \
/// route. The only execution-time decision in the optimized closure is logical /// M: \
rows 1...16 are the installed Metal lane; all wider values are the /// explicit prefill \
route selected by construction.' TWO SUPPORTING LAWS: the route table 'never mutates once \
published' (:427-429), and construction state is Task-local 'so simultaneous model loads \
cannot mix routes' (:316-318) — one-load-one-route isolation. THE ECONOMY: per-forward \
work is one rows compare, never an eligibility re-probe — and lane STABILITY is the \
real product: no lazy-init flag can flip the lane mid-run, no recovery path can retry \
the optimized dispatch behind the runner's back, and two concurrently-bound routers are \
independent values with no shared mutable route state. ⛔ KINSHIP (adjudicated both \
directions): attention-route-retirement-by-measurement (gpu_103) owns route-table \
RETIREMENT by measurement (a dead crossover collapses the predicate and deletes the \
special cases) — THIS owns route INSTALLATION frozen at construction: a different \
lifecycle phase and evidence standard (measurement vs the construction gate), and they \
compose (the frozen table is what the retirement law later simplifies); \
a8-activation-route-stability-guard (gpu_18) owns route ADOPTION re-asked per \
transformed shape (re-run the full dispatch on the downgraded shape, adopt only if it \
still lands the tensor tier) — THIS is the deliberate complement: when the model is \
fixed, never re-ask; distinct by decision cadence (per-transformed-shape vs \
once-per-model-load) and by question ('does the transformed shape still win' vs 'which \
lane does this model get, immutably'); launch-heuristic-pinned-to-calibration-arch \
(gpu_141) owns launch HEURISTICS pinned to a calibrated arch — the pin-once spirit at \
the launch-config layer, cite; compiled-vs-eager-per-op-exactness-admission (gpu_152) \
owns per-op-CLASS admission (integer/bf16 compile, transcendentals eager) — a per-op \
admission decision vs THIS's per-model lane binding, compose; \
whole-step-gdn-decode-window-one-launch (gpu_126) and the io-gpu-preemption-dispatch-\
budget (gpu_161) own dispatch BUDGET geometry — different axis. REGIME: any hot path \
whose optimized lane is bound to a load-time contract (profile + geometry) — model \
finalizers, backend selectors, kernel-variant tables, protocol handlers bound at \
connection setup. ANTI-REGIME: routes that MUST re-ask per call (data-dependent \
dispatch where the shape genuinely varies per item — that is the predicate's job, keep \
it); eligibility that legitimately changes mid-run (a capability probe that can flip — \
then the design must own the flip explicitly, not hide it in a lazy flag). \
NotBenchable-by-domain (the win is lane stability across a run — no re-probing, no \
mid-run flips, no mixed routes under concurrent loads; the per-call delta is one \
branch and would measure the box, matching gpu_103's posture). P113: upstream's own \
landed decision — decision content only. Source: Layr-Labs/mlxfast-bonsai2-27b-engine \
@ 0c7c3923 (MIT) — Vendor/mlx-swift-lm/Libraries/MLXLLM/Models/Qwen35A3BOptimization.\
swift:589-612 (the router finalizer) + :2-4 (the file contract) + :316-318 (task-local \
construction state) + :427-429 (the published table never mutates) + :600-603 (the \
construction gate). Quotes byte-verified at the clone at the pin this session. MIT. \
Batch 412 / Plan 411 / Research 445."
                .to_string(),
            ref_shape: r"// NAIVE: eligibility re-probed every forward, and a lazy
// flag that can flip the lane mid-run.
struct Router {
    profile: Profile,
    g: Geometry,
    installed: Option<bool>, // lazy: decided on first call
}

impl Router {
    fn route(&mut self, rows: usize) -> Lane {
        if self.installed.is_none() {
            // probe eligibility NOW (and every call re-reads it)
            self.installed = Some(matches!(self.profile, Profile::Decode) && self.g == CONTRACT);
        }
        let env_still_matches = self.g == CONTRACT; // live re-probe
        if self.installed == Some(true) && env_still_matches && rows <= 16 {
            Lane::RowOwnedMetal
        } else {
            Lane::Stock
        }
    }
}
"
            .to_string(),
            opt_pattern: r"// BOUND AT CONSTRUCTION: the gate answers once; the returned
// router is an immutable value; the hot path is the rows compare.
#[derive(Clone, Copy)]
enum Router {
    StockOnly, // permanent: the gate's else branch
    RowOwnedLane,
}

fn bind_route(profile: Profile, g: Geometry) -> Router {
    match profile {
        Profile::Decode | Profile::Full if g == CONTRACT => Router::RowOwnedLane,
        _ => Router::StockOnly, // never recovers, never upgrades
    }
}

impl Router {
    fn route(&self, rows: usize) -> Lane {
        match self {
            Router::StockOnly => Lane::Stock,
            Router::RowOwnedLane => {
                if (1..=16).contains(&rows) { Lane::RowOwnedMetal } else { Lane::Stock }
            }
        }
    }
}
"
            .to_string(),
        },
    ]
}
