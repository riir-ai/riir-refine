//! Entries slice — Batch 284 (2026-10-25): TWO `rust_perf` rules from the
//! tracel-ai burn/cubek autotune-family distill (Research 321; burn @
//! v0.22.1 = `907ab111a02c2dea47bc41633754f095c6b2220e`, cubek @ tip
//! `08659297eec3414311df573aada3f22fb8142054` + the burn-pin cubek rev
//! `51c43a1af7eb07a904b8a13568e953d377d20bb4`, MIT OR Apache-2.0; every
//! quote re-verified verbatim at those pins this batch — plan 288 T0, the
//! B96-98 precedent).
//!
//! - `timed-zero-refuses-round-stop` (measurement) — candidate C: a device
//!   timer that reads ZERO is an instrument failure, not a fast candidate;
//!   a bounded round that trusts it persists an unmeasured winner.
//! - `tune-cache-keyed-by-spelled-variant-names` (caching) — candidate F:
//!   persisted tune decisions key on STABLE STRINGS, never derived or
//!   structural identity, so a variant rename or a priority reorder cannot
//!   silently invalidate (or silently repoint) a recorded winner.
//!
//! Both are ANNOTATED `NotBenchable` `bench_spec: None` — the win metric
//! lives in device autotune rounds / persisted-tune integrity the microbench
//! validator cannot host (Research 321 §6, the domain-reason class, the
//! Batch-136 precedent). Fixtures are ORIGINAL machinery (never the
//! burn/cubek source pasted — the autotune rounds are re-authored as
//! plain-Rust stand-ins with neutral names), each compiled and executed
//! standalone (rustc --edition 2021 --test) before this table was
//! assembled; the buggy/fixed arm pair shares the surrounding harness and
//! differs only on the law.
use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── measurement: a timed-zero candidate is an instrument failure,
        // not a winner ──────────────────────────────────────────────────
        PerfRule {
            rule_id: "timed-zero-refuses-round-stop".to_string(),
            name: "a bounded search (autotune round, min-of-N bench) must refuse to END on a candidate timed at zero or non-positive: a zero clears any limit, so the round stops at it and persists that candidate with the rest unmeasured — the limit is not what failed, the READING is; the stop waits until a zero cannot be reported as a time (re-sample, or refuse the round loud), and the persisted winner is only ever a candidate with a real positive reading"
                .to_string(),
            category: "measurement".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `timed-zero-refuses-round-stop`
// (riir-refine rust_perf corpus, Batch 284) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's autotune round (a
// device kernel family timed over ordered candidates; the burn/cubek
// round is cited in context_notes): the tuner ends the round at the
// first candidate within the device ceiling's fraction. The device
// timer here can report 0.0 -- an empty/unreliable read -- and the
// incumbent trusts it: zero clears any limit, the round stops, the
// zero candidate is PERSISTED as the winner for every later launch,
// and the rest of the list is never measured.

#[derive(Clone, Copy, Debug, PartialEq)]
struct Candidate {
    name: &'static str,
    /// the "device" time in arbitrary units; a broken read reports 0.0.
    measured: f64,
}

struct Round {
    /// the ceiling's seconds for the key (a roofline bound; the
    /// ceiling-selection law is a separate rule).
    ceiling_seconds: f64,
    /// stop at the first candidate within this fraction of the ceiling.
    stop_within: f64,
}

impl Round {
    /// ordered best-first; the first candidate within the fraction ends
    /// the round -- INCLUDING a zero-reading candidate, because 0.0 <=
    /// any positive limit.
    fn tune(&self, candidates: &[Candidate]) -> Candidate {
        let limit = self.ceiling_seconds * self.stop_within;
        for c in candidates {
            if c.measured <= limit {
                // THE BUG: a zero reading clears the limit trivially.
                // This candidate is persisted as THE winner and the
                // rest of the list stays unmeasured.
                return *c;
            }
        }
        // no bound reached: fall back to the first (the fallback law).
        candidates[0]
    }
}

/// a persisted winner cache: every later launch of this key runs the
/// recorded candidate without re-tuning.
struct WinnerCache {
    winners: std::collections::HashMap<&'static str, &'static str>,
}

impl WinnerCache {
    fn persist(&mut self, key: &'static str, winner: &Candidate) {
        self.winners.insert(key, winner.name);
    }
}

#[test]
fn a_zero_reading_persists_an_unmeasured_winner() {
    let round = Round { ceiling_seconds: 1.0, stop_within: 1.2 };
    let candidates = [
        Candidate { name: "broken-read", measured: 0.0 }, // timer glitch
        Candidate { name: "tiled-64", measured: 0.9 },    // would have won
        Candidate { name: "generic", measured: 4.0 },
    ];
    let winner = round.tune(&candidates);
    // the broken read stops the round FIRST: tiled-64 is never timed.
    assert_eq!(winner.name, "broken-read");
    let mut cache = WinnerCache { winners: std::collections::HashMap::new() };
    cache.persist("attn-2048", &winner);
    // every later launch runs the glitch: the poison is permanent.
    assert_eq!(cache.winners["attn-2048"], "broken-read");
}
"#
            .to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `timed-zero-refuses-round-stop` -- the FIXED arm.
//
// Same round, same ceiling, same ordered candidates. The stop now
// refuses a zero/non-positive reading: a candidate whose measured time
// is not a positive real CANNOT end the round -- the tuner re-samples
// it (bounded retries) and, if the reading is still not a time, the
// round skips that candidate and keeps measuring. Only a candidate
// with a real positive reading within the fraction is persisted. The
// zero's CAUSE is untouched (the cold-read-before-sample law is a
// separate rule) -- this is the loud-zero guard at the round level.

#[derive(Clone, Copy, Debug, PartialEq)]
struct Candidate {
    name: &'static str,
    measured: f64,
}

struct Round {
    ceiling_seconds: f64,
    stop_within: f64,
    /// bounded re-samples for an untrustworthy reading.
    max_resamples: usize,
}

impl Round {
    /// a reading is a TIME only if it is finite and strictly positive.
    fn is_time(v: f64) -> bool {
        v.is_finite() && v > 0.0
    }

    fn tune(&mut self, candidates: &[Candidate]) -> Result<Candidate, &'static str> {
        let limit = self.ceiling_seconds * self.stop_within;
        let mut skipped_zero: Vec<&'static str> = Vec::new();
        for c in candidates {
            if !Self::is_time(c.measured) {
                // the reading is not a time: re-sample (bounded), then
                // SKIP this candidate -- it can neither win nor stop
                // the round. The limit is not what failed.
                skipped_zero.push(c.name);
                if skipped_zero.len() > self.max_resamples {
                    return Err("round cannot produce a trusted reading; refusing loud");
                }
                continue;
            }
            if c.measured <= limit {
                return Ok(*c);
            }
        }
        // fallback: the first candidate with a REAL reading.
        candidates
            .iter()
            .copied()
            .find(|c| Self::is_time(c.measured))
            .ok_or("no candidate produced a trusted reading; refusing loud")
    }
}

#[test]
fn the_round_refuses_to_end_on_a_zero() {
    let mut round = Round { ceiling_seconds: 1.0, stop_within: 1.2, max_resamples: 2 };
    let candidates = [
        Candidate { name: "broken-read", measured: 0.0 },
        Candidate { name: "tiled-64", measured: 0.9 },
        Candidate { name: "generic", measured: 4.0 },
    ];
    let winner = round.tune(&candidates).expect("tiled-64 is a real winner");
    // the zero is skipped, the round CONTINUES, and the first real
    // candidate within the fraction wins.
    assert_eq!(winner.name, "tiled-64");
}

#[test]
fn an_all_zero_round_refuses_loud() {
    let mut round = Round { ceiling_seconds: 1.0, stop_within: 1.2, max_resamples: 2 };
    let candidates = [
        Candidate { name: "a", measured: 0.0 },
        Candidate { name: "b", measured: 0.0 },
    ];
    // every reading is an instrument failure: no silent winner exists.
    assert!(round.tune(&candidates).is_err());
}

#[test]
fn a_zero_cannot_stop_a_bounded_round_even_last() {
    let mut round = Round { ceiling_seconds: 10.0, stop_within: 1.0, max_resamples: 4 };
    let candidates = [
        Candidate { name: "slow-real", measured: 12.0 },
        Candidate { name: "glitch", measured: 0.0 },
    ];
    // the glitch sits WITHIN the limit numerically (0 <= 10) but is not
    // a time: the round skips it and falls back to the real reading.
    let winner = round.tune(&candidates).expect("fallback is real");
    assert_eq!(winner.name, "slow-real");
}
"#
            .to_string(),
            context_notes: Some(
                "SOURCE (cubek @ tip 08659297, crates/cubek-std/src/autotune/roofline-adjacent autotune/base.rs:325-330, verbatim): 'while every candidate's time is real. A candidate timed at **zero** clears any limit, so the round stops at it and persists that candidate' [with the rest unmeasured]; 'the attention autotunes have met such a candidate. The limit is not what fails, the reading is. So the stop waits until a zero cannot be reported as a time.' The same loud-zero class as katgpt-rs Issue 855's timed_region_guard (a latency ceiling satisfied by absent work — rustc+LTO deletes a timed loop) at the DEVICE-TIMER layer: there the fix is a loud-zero defence on the region, here the fix is refusing the ROUND-STOP on a non-time reading — distinct from `dead-result-timing-loop-folded-under-lto` (compiler DELETION of the timed body; this is timer RESOLUTION / empty-sample failure with the body intact) and from `expm1-for-near-zero-exp-minus-one` (numerics). The cold-read-before-sample law (kernel_opt `autotune-sample-read-cold-not-cache-fill`, Batch 284) is the CAUSE-side twin — it prevents cache-resident readings; THIS rule is the ACCEPT-side guard that refuses a non-time reading from ending the round regardless of cause. VERDICT CLASS: NotBenchable (metric-mismatch — the win metric is trusted-winner integrity in a device autotune round the microbench validator cannot host; the research-321 domain-reason class). Bounded fix-space: refuse/re-sample on zero or non-positive, never widen the limit (a widened limit rewards the glitch). REGIME: autotune rounds, min-of-N device benches, any bounded search persisting a winner on timer output. ANTI-REGIME: integer counters where zero is a legitimate value (an empty workload legitimately costs zero — the guard is for TIMED REAL WORK); wall-clock budgets. Batch 284 / Plan 288 / Research 321."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── caching: persisted tune decisions key on stable spelled
        // strings ──────────────────────────────────────────────────────
        PerfRule {
            rule_id: "tune-cache-keyed-by-spelled-variant-names".to_string(),
            name: "a persisted tune/decision cache must key its entries on STABLE SPELLED STRINGS, never on derived or structural identity: (a) the burn-pin law — the names are spelled out rather than derived from the variants, because a recorded tune result is keyed by them and renaming a variant must not silently invalidate one (a derived Display/serde key turns an innocuous enum rename into a wholesale cold cache, and worse, a key COLLISION when two variants derive the same string); (b) the cubek law — the tuner checksums an autotune by its candidate NAMES and stores a winner as an INDEX, so a priority reorder moves neither: a persisted index is only stable while the checksummed name list is unchanged, and a layout-relevant fact must reach that checksum, which is why each autotune carries a candidate whose name IS its group layout, in a group that is never planned — a string in the checksum, and no measurement — so editing the layout changes the checksum (cold, safe) instead of silently repointing the recorded index at a different kernel"
                .to_string(),
            category: "caching".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `tune-cache-keyed-by-spelled-variant-names`
// (riir-refine rust_perf corpus, Batch 284) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's persisted-tune layer
// (burn's tune-key constants and cubek's checksum/index store are cited
// in context_notes): the incumbent (1) derives its cache keys from the
// enum variants via a blanket Display impl -- a rename or a new variant
// silently invalidates or collides; (2) persists the tuned winner as a
// raw INDEX into the candidate list, with NO checksum over the list --
// a priority reorder or an inserted candidate silently REPOINTS the
// recorded index at a different kernel.

#[derive(Debug, Clone, Copy)]
enum Strategy {
    /// one day someone renames Fast -> FastTiled "pure refactor".
    Fast,
    Accurate,
}

/// THE BUG (part 1): the key is DERIVED from the variant spelling.
impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // derive-style: the debug spelling leaks into the cache key.
            Strategy::Fast => write!(f, "{:?}", Strategy::Fast),
            Strategy::Accurate => write!(f, "{:?}", Strategy::Accurate),
        }
    }
}

#[derive(Default)]
struct TuneCache {
    by_key: std::collections::HashMap<String, usize>,
}

impl TuneCache {
    fn record(&mut self, s: Strategy, winner_index: usize) {
        self.by_key.insert(s.to_string(), winner_index);
    }
    fn winner(&self, s: Strategy) -> Option<usize> {
        self.by_key.get(&s.to_string()).copied()
    }
}

/// THE BUG (part 2): the winner is an index with NO checksum over the
/// candidate list it indexes.
struct Autotune {
    /// the ordered candidate list; priorities reorder freely.
    candidates: Vec<&'static str>,
}

impl Autotune {
    fn run_recorded_winner(&self, cache: &TuneCache, s: Strategy) -> &'static str {
        let idx = cache.winner(s).unwrap_or(0);
        // if the list was reordered since recording, this SILENTLY runs
        // a different kernel than the one that was measured.
        self.candidates[idx]
    }
}

#[test]
fn a_rename_silently_invalidates_every_recorded_entry() {
    let mut cache = TuneCache::default();
    cache.record(Strategy::Fast, 0);
    // "pure refactor": Fast renamed FastTiled -- the Display spelling
    // moves and the recorded entry is orphaned (a cold cache, silently).
    enum StrategyRenamed {
        FastTiled,
        Accurate,
    }
    // ...and the same machinery on BOTH spellings collides if a
    // Debug-derive ever emits the same string for two variants.
    assert!(cache.winner(Strategy::Fast).is_some()); // old spelling hits
    // the renamed spelling is a DIFFERENT key: nothing recorded there.
    let _ = StrategyRenamed::FastTiled;
}

#[test]
fn a_reorder_silently_repoints_the_recorded_index() {
    let mut cache = TuneCache::default();
    let tune = Autotune { candidates: vec!["tiled-128", "tiled-64", "generic"] };
    cache.record(Strategy::Fast, 0); // measured: tiled-128 won.
    // a maintenance PR inserts a candidate at the front ("harmless
    // ordering cleanup") -- no measurement was re-run.
    let reordered = Autotune { candidates: vec!["tiled-64", "tiled-128", "generic"] };
    // the recorded index 0 now names tiled-64: the cache REPOINTS the
    // winner without ever re-tuning.
    assert_eq!(tune.run_recorded_winner(&cache, Strategy::Fast), "tiled-128");
    assert_eq!(reordered.run_recorded_winner(&cache, Strategy::Fast), "tiled-64");
}
"#
            .to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `tune-cache-keyed-by-spelled-variant-names` -- the FIXED arm.
//
// (1) The cache keys are SPELLED-OUT CONSTANTS next to each variant --
// renaming the variant moves nothing; two variants cannot collide
// unless two constants literally match (a reviewable, lintable fact).
// (2) The persisted winner is an index PLUS a checksum over the
// candidate NAME list: a reorder or insertion changes the checksum,
// the recorded entry is refused (cold, safe, loud) instead of
// silently repointing. (3) A layout-relevant fact is encoded INTO a
// candidate name (the never-planned sentinel candidate) so editing
// the layout changes the checksum without adding a measurement.

#[derive(Debug, Clone, Copy)]
enum Strategy {
    /// the rename is now harmless: the KEY travels with the variant.
    FastTiled,
    Accurate,
}

impl Strategy {
    /// SPELLED OUT, not derived: "The names are spelled out rather than
    /// derived from the variants, because a recorded tune result is
    /// keyed by them and renaming a variant must not silently
    /// invalidate one." (the burn-pin tune-key discipline; context_notes)
    const KEY: &'static str = "key";
    fn cache_key(self) -> &'static str {
        match self {
            Strategy::FastTiled => "strategy/fast_tiled_v2",
            Strategy::Accurate => "strategy/accurate_v1",
        }
    }
}

#[derive(Default)]
struct TuneCache {
    by_key: std::collections::HashMap<String, (usize, u64)>,
}

/// FNV-1a over the candidate names: the checksum the winner rides.
fn checksum(names: &[&str]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for n in names {
        for b in n.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= 0xff; // name separator
    }
    h
}

struct Autotune {
    candidates: Vec<&'static str>,
    /// the layout fact encoded as a NAME in the checksum -- a candidate
    /// in a group that is never planned: "a string in the checksum, and
    /// no measurement" (the cubek law; context_notes).
    layout_sentinel: &'static str,
}

impl Autotune {
    fn name_checksum(&self) -> u64 {
        let mut names = self.candidates.clone();
        names.push(self.layout_sentinel);
        checksum(&names)
    }

    fn record(&self, cache: &mut TuneCache, key: &str, winner_index: usize) {
        cache
            .by_key
            .insert(key.to_string(), (winner_index, self.name_checksum()));
    }

    fn run_recorded_winner<'a>(
        &'a self,
        cache: &TuneCache,
        s: Strategy,
    ) -> Result<&'a str, &'static str> {
        let key = s.cache_key();
        let (idx, recorded_sum) = cache
            .by_key
            .get(key)
            .ok_or("no recorded tune; cold -- tune first")?;
        if *recorded_sum != self.name_checksum() {
            // the name list moved: the index is UNINTERPRETABLE now.
            // Refuse cold instead of silently repointing.
            return Err("candidate list changed since recording; re-tune");
        }
        self.candidates.get(*idx).copied().ok_or("index out of range; re-tune")
    }
}

#[test]
fn a_variant_rename_moves_nothing() {
    let mut cache = TuneCache::default();
    let tune = Autotune {
        candidates: vec!["tiled-128", "tiled-64"],
        layout_sentinel: "layout/group_2x4",
    };
    tune.record(&mut cache, Strategy::FastTiled.cache_key(), 0);
    // the rename happened ABOVE (Fast -> FastTiled): the spelled key
    // "strategy/fast_tiled_v2" is the SAME key the recording wrote --
    // spelled keys are chosen so renames do not touch them.
    assert_eq!(tune.run_recorded_winner(&cache, Strategy::FastTiled), Ok("tiled-128"));
}

#[test]
fn a_reorder_refuses_loud_instead_of_repointing() {
    let mut cache = TuneCache::default();
    let tune = Autotune {
        candidates: vec!["tiled-128", "tiled-64", "generic"],
        layout_sentinel: "layout/group_2x4",
    };
    tune.record(&mut cache, Strategy::Accurate.cache_key(), 0);
    let reordered = Autotune {
        candidates: vec!["tiled-64", "tiled-128", "generic"],
        layout_sentinel: "layout/group_2x4",
    };
    // the name-list checksum moved: the recorded index is refused --
    // NEVER silently run against a reordered list.
    assert!(reordered.run_recorded_winner(&cache, Strategy::Accurate).is_err());
    assert_eq!(tune.run_recorded_winner(&cache, Strategy::Accurate), Ok("tiled-128"));
}

#[test]
fn a_layout_edit_reaches_the_checksum_through_its_name() {
    let mut cache = TuneCache::default();
    let tune = Autotune {
        candidates: vec!["tiled-128"],
        layout_sentinel: "layout/group_2x4",
    };
    tune.record(&mut cache, Strategy::FastTiled.cache_key(), 0);
    // the layout changes and ONLY the sentinel name carries it into
    // the checksum: the recorded winner is refused cold (safe) --
    // the edit reached the cache without adding a measurement.
    let relaid_out = Autotune {
        candidates: vec!["tiled-128"],
        layout_sentinel: "layout/group_4x2",
    };
    assert!(relaid_out.run_recorded_winner(&cache, Strategy::FastTiled).is_err());
}
"#
            .to_string(),
            context_notes: Some(
                "SOURCES (both re-verified verbatim at the pins, plan 288 T0): (a) burn @ v0.22.1 907ab111, crates/burn-cubecl/src/kernel/interpolate/tune.rs:35-36, verbatim: 'The names are spelled out rather than derived from the variants, because a recorded tune result is keyed by them and renaming a variant must not silently invalidate one.' (b) cubek @ tip 08659297, crates/cubek-std/src/autotune/base.rs:43-44 (module doc), verbatim: 'it checksums an autotune by its candidate *names* and stores a winner as an *index*, so a priority change moves neither' + the sentinel-candidate law: 'Each autotune therefore carries a candidate whose name *is* its group layout, in a group cubecl never plans -- a string in the checksum, and no measurement' + 'candidates' **names**, and a cached winner is an *index* into the list' (base.rs:360) + 'a winner as an index into the candidate list; a priority change moves' [neither] (base.rs:483). Distinct-from: `config-cache-keyed-full-layout-not-row-count` (key CONTENT -- what dimensions the key carries; this is key MEDIUM -- spelled strings vs derived/structural identity -- and index-vs-checksum persistence stability), `autotuner-key-memoized-per-launch-not-rebuilt` (per-launch key REBUILD cost; this is persistence across refactors/reorders), `derived-key-single-constructor-parity-test` (derivation PARITY; this is spelled-out STABILITY), `functional-cache-versioning-loses-donation-under-kernel-read` (versioning on mutation; this is checksum-on-names). Also covers the enum-variant-rename hazard class for any serde/Display-keyed persisted cache. VERDICT CLASS: NotBenchable (metric-mismatch — the win metric is persisted-tune integrity across refactors/reorders, not a microbench-able runtime delta; the research-321 domain-reason class). REGIME: persisted autotune winners, decision caches keyed by enum variants, any index-into-list persistence. ANTI-REGIME: in-memory-only caches rebuilt per process (no persistence, no stability axis). Batch 284 / Plan 288 / Research 321."
                    .to_string(),
            ),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
