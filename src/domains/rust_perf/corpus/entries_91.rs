// Batch 377 rust_perf rules — the colorless write-once controlflow family
//! seed (Plan 394 / Research 428): veecore/colorless-rs @
//! `b5db6b95cf2f18efbb12fabe5709ac003fcde622` (MIT OR Apache-2.0 (+ZLIB);
//! clone re-created at the batch T0 — HEAD == pin verified, the tip had NOT
//! moved; lib.rs/colored.rs/SAFETY.md/README.md/CHANGELOG.md read at the
//! pin). Both rules suggestion-only, honestly NotBenchable-by-domain (the
//! win metrics are a maintenance-drift property and a per-call allocation
//! census on a hot poll path — no wall/alloc validator hosts them;
//! `bench_spec: None`).

use crate::PerfRule;

pub fn r1_sync_async_twin_duplication() -> PerfRule {
    PerfRule {
        rule_id: "sync-async-twin-duplication".to_string(),
        name: "A _sync/_async twin pair whose bodies duplicate control flow → ONE color-parameterized body carried by both spellings (maybe_async / async-generic / a Colored<C> trait / the keyword-generics watch); every duplicated statement is a drift surface the pair must be kept agreeing by hand".to_string(),
        category: "concurrency".to_string(),
        buggy_code: r#"// TWIN PAIR: the retry ladder + validation gate exist TWICE —
// byte-identical control flow, once per spelling. When the retry
// ceiling changes, one twin is edited and the other goes stale:
// the pair silently diverges on the same input.
const RETRY_CEILING: u32 = 3;

fn fetch_sync(io: &mut Io) -> Result<Payload, Error> {
    let mut attempt = 0;
    loop {
        match io.step() {
            Ok(p) => {
                validate(&p)?;
                return Ok(p);
            }
            Err(e) => {
                attempt += 1;
                if attempt >= RETRY_CEILING {
                    return Err(e);
                }
            }
        }
    }
}

async fn fetch_async(io: &mut Io) -> Result<Payload, Error> {
    let mut attempt = 0;
    loop {
        match io.step() {
            Ok(p) => {
                validate(&p)?;
                return Ok(p);
            }
            Err(e) => {
                attempt += 1;
                if attempt >= RETRY_CEILING {
                    return Err(e);
                }
            }
        }
    }
}"#.to_string(),
        fixed_code: r#"// ONE body, both colors: the control flow exists exactly once,
// carried by the generic color — the twins are DELETED, not kept
// agreeing by hand. (Attribute forms: #[maybe_async]; async-generic;
// or a hand-rolled Colored<C> trait. The keyword-generics initiative
// is the language-level shape of the same remedy — WATCH, not yet stable.)
async fn fetch_body(io: &mut Io) -> Result<Payload, Error> {
    let mut attempt = 0;
    loop {
        match io.step() {
            Ok(p) => {
                validate(&p)?;
                return Ok(p);
            }
            Err(e) => {
                attempt += 1;
                if attempt >= RETRY_CEILING {
                    return Err(e);
                }
            }
        }
    }
}

fn fetch_sync(io: &mut Io) -> Result<Payload, Error> {
    block_on(fetch_body(io)) // the sync spelling drives the ONE body
}

async fn fetch_async(io: &mut Io) -> Result<Payload, Error> {
    fetch_body(io).await // the async spelling drives the SAME body
}"#.to_string(),
        context_notes: Some(r##"Grade B− family seed — suggestion-only, honestly NotBenchable-by-domain (the win metric is a MAINTENANCE-DRIFT property — twins that were in agreement silently diverging after one side's edit — no wall/alloc validator hosts it; `bench_spec: None`; the dial9/B136 annotation class). SOURCE (veecore/colorless-rs @ b5db6b95cf2f18efbb12fabe5709ac003fcde622, MIT OR Apache-2.0 (+ZLIB); the pin clone re-created at the batch T0, HEAD == pin verified — the tip had NOT moved; lib.rs/colored.rs/SAFETY.md/README.md read at the pin). THE MECHANISM THIS SOURCE NAMES (lib.rs:67-71 at the pin, verbatim): 'In [`Identity`], combinators inline directly. Step transitions compile away into ordinary sequential instructions, and loops become native CPU jumps. No generator state is allocated, and no poll functions are registered.' / 'In [`Async`], combinators lower into compact, unboxed enum state machines driven by standard Rust executors (Tokio, smol, etc.). Memory layout grows linearly rather than exponentially.' — the WRITE-ONCE axis: one body generic over a Color, compiled per color. THE RULE IS THE PROBLEM PATTERN THE MECHANISM SOLVES, never the mechanism: the corpus rule cites the twins and their remedies (the `maybe_async` / `async-generic` crates, a hand-rolled `Colored<C>` trait, the keyword-generics initiative as the language-level watch), NOT `#[colorless]` — the source is pre-crates.io alpha with a RELAXED IMPLICIT DESTRUCTION model ('A local binding that crosses a `.resolve` point may be dropped earlier or later than the lexical end of its enclosing scope. Sibling locals may be dropped in an order different from standard Rust reverse-lexical drop order.' README:155-156 at the pin; CHANGELOG: 'Exact lexical destruction timing and sibling destruction order are not preserved by macro-generated continuations.'), strict lexical RAII is its own roadmap item — DO NOT ADOPT the library until RAII lands; the rule never recommends it. TRIGGER (the reviewer-conditioned body test): the name-suffix pair (`_sync`/`_async`, or blocking/nonblocking trait-impl pairs) PLUS ≥2 duplicated control-flow statements in both bodies (a ladder, a gate, a loop — statements the pair must be kept agreeing by hand). ANTI-REGIME: the single-statement DELEGATION wrapper — a `_sync` whose body is ONE call (`block_on(fetch_async(..))`) is the GOOD pattern (entries_06's own fix shape names it 'the async twin'), carries no duplicated control flow and no drift surface; never flagged. THE GATE (distinct-from, both directions): vs `blocking-sleep-in-async` (entries_06) — THAT owns the CALL-SITE runtime defect (a blocking wait parking the worker INSIDE one async fn; the fix replaces the call, one site, no duplication); THIS owns the REPO-SHAPE defect (two whole bodies duplicated across spellings; the fix deletes one body). A twin pair may contain a blocking-sleep defect in its async member (compose: the sleep rule fixes the call, THIS deletes the twin); a blocking-sleep site with no twin pair is out of THIS rule's scope entirely. vs `runtime-bool-duplicates-inlinable-body` — THAT owns the bool-parameterized monomorphization trade (one body, two specializations compiled); THIS owns two HAND-WRITTEN bodies (no shared body exists to specialize). REGIME: dual-API crates (every protocol codec, embedded driver, and blocking/async library face of the tokio era); trait-impl pairs where both impls duplicate the body; test doubles mirroring prod twins. POPULATION (not firing evidence): maybe-async-0.2.11 sits in the corpus's mined population (094_t03/manifest.json:3340, 094_t04:3450, verdict ok, 32 warnings from rules extant at that scan — R1 did not exist at it; the crate's own README opens 'Why bother writing similar code twice for blocking and async code?' — the problem statement IS the family). Fixture: b394_fixtures.rs F1 (the in-sync agreement pin, the drift arm — one edited ceiling diverges the pair, the hazard the rule names; the delegation-wrapper anti-regime refuses at the body test; the color-parameterized remedy answers exactly what the in-sync pair answered — one body, zero twins). Plan 394 / Research 428."##.to_string()),
        // Not benchable: the win axis is maintenance-drift; no
        // validator hosts a twin-pair divergence census (the
        // dial9/136 class).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn r2_box_dyn_future_in_hot_path_colorless() -> PerfRule {
    PerfRule {
        rule_id: "box-dyn-future-in-hot-path-colorless".to_string(),
        name: "A per-call Box::pin(..) as Pin<Box<dyn Future>> on a hot poll path pays an allocation per call where the inline generic (monomorphized) future pays zero — erase on a per-call path is a measured cost, never a default; keep the box only where it is load-bearing (recursion / dyn-trait boundaries / deliberate compile-time size control)".to_string(),
        category: "concurrency".to_string(),
        buggy_code: r#"// The hot loop erases PER CALL: one heap alloc per step.
fn boxed_hot_loop(steps: u64) -> u64 {
    let mut acc = 0u64;
    for _ in 0..steps {
        let mut f: Pin<Box<dyn Future<Output = u64>>> = Box::pin(Ready(1));
        acc += poll_dyn_to_ready(f.as_mut());
    }
    acc
}"#.to_string(),
        fixed_code: r#"// The same loop over the inline generic future: the type is
// monomorphized into the caller, the alloc count drops to zero.
fn inline_hot_loop(steps: u64) -> u64 {
    let mut acc = 0u64;
    for _ in 0..steps {
        let mut f = Ready(1);
        acc += poll_to_ready(&mut f);
    }
    acc
}"#.to_string(),
        context_notes: Some("Grade C+/B− — suggestion-only, honestly NotBenchable-by-domain (the win metric is a per-call ALLOCATION CENSUS on a hot poll path — measurable with a counting allocator, deterministic, not a timing — but 'hot path' itself is not statically decidable, so no validator hosts the pair; `bench_spec: None`). SOURCE (veecore/colorless-rs @ b5db6b95cf2f18efbb12fabe5709ac003fcde622, MIT OR Apache-2.0 (+ZLIB); the pin clone re-created at the batch T0, HEAD == pin verified; lib.rs/SAFETY.md read at the pin — the source's OWN Identity-vs-boxed-dyn-Future figures 2.06-2.5x / 1.34-1.53x are facade doc-queries on a FOREIGN benchmark, PROVENANCE never bars). THE DEFECT: `Box::pin(..)` coerced to `Pin<Box<dyn Future>>` at a per-call position (loop bodies, per-request handlers) heap-allocates per call; the inline generic future monomorphizes into the caller and allocates ZERO (the fixture census: 256+ allocs vs 0 at 256 steps). THE REMEDY: inline the future — make the consumer generic over the concrete future type (impl Trait in return position, a struct field generic over F: Future). KEEP-THE-BOX cases (the anti-regimes, each load-bearing — never flagged): (a) RECURSIVE futures — a future whose own type appears in its definition cannot be spelled inline; the box breaks the infinite type (the fixture drives a boxed recursive spine and answers correctly); (b) DYN-TRAIT object boundaries — a consumer storing `Box<dyn Future>` owns the erase as its INTERFACE (heterogeneous task collections, plugin/task registries); one erase at the boundary amortized over many polls is not a per-call tax; (c) DELIBERATE compile-time size control — choosing the box to cap monomorphization bloat (compile-time/codegen budgets) is a decision; disclose it at the site, never silently inline. THE GATE (distinct-from, the reviewer-widened set, both directions): vs `async-future-size` (entries_02) — THAT owns the STATE-MACHINE FOOTPRINT (large locals across .await inflating size_of(future) per spawned task; the fix scopes locals INSIDE one async body); THIS owns the ERASE COST (a heap allocation per call at the type-erasure boundary; the fix deletes the erasure). A big future that is also boxed per call has both defects (compose: scope the locals, then inline the box away). vs `async-wrapper-future-size-exponential-nesting` (entries_79) — THAT owns per-layer DUPLICATION of the wrapped future (2^N composed size; the fix stores the inner once); THIS owns per-call ERASURE of one future (alloc-per-call; the fix monomorphizes). The B363 preflight's own harness shape (Pin<Box<dyn Future>> driven through a poll loop, .plans/b363_goat_preflight.rs:127-150) is the MEASUREMENT posture for THAT rule's bars — this rule is the PER-CALL tax decision at the same seam; they compose (measure the composed chain unboxed where the harness allows, box only at the boundary). vs `inline-actor-futures-pool-not-spawn-per-actor` (entries_86) — THAT owns the TASK pool shape (keep-alive tasks vs respawn-on-input); THIS owns the per-call erase INSIDE whichever pool shape — compose. REGIME: per-item poll/step loops (parsers, tick drivers, streaming pipelines), per-request handlers that box a fresh future each call, fan-out paths boxing N fresh futures per batch. ANTI-REGIMES: the three keep-the-box cases above; ONE box amortized over many polls (the erase happens once, the polls are free); boxes the CONSUMER's API demands (a `Box<dyn Future>` parameter is the caller's boundary, not yours to inline). Fixture: b394_fixtures.rs F2 (the census arm — boxed per-call >= 1 alloc/step vs inline exactly 0; the recursive walk keeps its box and answers; the dyn boundary keeps its box; the tamper arm — an erase smuggled into the inline shape is caught by the census). Plan 394 / Research 428.".to_string()),
        // Not benchable: the win axis is a per-call alloc census; r##""##hot
        // path" is not statically decidable, so no validator hosts the
        // pair (the dial9/136 class).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

pub fn b394_rules() -> Vec<PerfRule> {
    vec![
        r1_sync_async_twin_duplication(),
        r2_box_dyn_future_in_hot_path_colorless(),
    ]
}
