// Batch 412 — the mlxfast staged pool's rust_perf half (Plan 411 /
// Research 445): the Research 437 staged candidates, mined at the pin,
// Layr-Labs/mlxfast-bonsai2-27b-engine @ `0c7c3923` (MIT), the SAME pin
// Batch 402 mined — the staged names were stage labels, each re-surfaced
// and re-verified in-source this session (the Batch-409 lesson: a staged
// lead's claim expires against the corpus, re-grep at claim time).
//
// rust_perf 396 → 400: four rules —
// - `cache-restore-adopt-or-drop-whole-entry` (caching) — a cache hit
//   adopts the ENTIRE pre-built artifact set in one transaction: pass
//   one verifies EVERY member against the manifest the save wrote, the
//   tree is touched only in pass two; any mismatch is a miss, never a
//   partial restore (a mixed-generation tree that looks built measures
//   a composite engine no stage authored).
// - `acceptance-band-derived-from-own-axis-measured-cv`
//   (measurement-honesty) — each scored axis carries its own acceptance
//   band DERIVED from that axis's own measured session CV (tight side =
//   ceil-to-half-percent of 10x CV, clamped to [4x spread, prior-band)),
//   never one shared tolerance carried over unexamined.
// - `readiness-wait-liveness-first-deadline-bounded` (concurrency) — a
//   boot/readiness wait checks LIVENESS FIRST each iteration (a dead
//   child is named and exits in one poll, never at the deadline), then
//   probes readiness, and the whole wait is bounded by an explicit
//   deadline with the failure named and the log attached.
// - `calibration-band-two-sided-straddles-one-zero-cv-frozen-clock`
//   (measurement-honesty) — a calibration health band is two-sided and
//   must straddle 1 (low < 1 < high); every measured value and band
//   edge is positive (a zero CV is a frozen clock, not a stable box);
//   a CV above the ceiling is a refusal, not a warning; and the
//   calibration file can only STOP a score, never move one.
//
// DROPPED at T1 dedup (the prior-coverage kill, recorded in Research
// 445): `per-step-shared-scalars-host-advanced-one-array` — ALREADY
// OWNED by `host-mirror-shared-scalar-advance-one-time-verdict`
// (entries_10, Batch 182) mined from the SAME repo lineage (the LayerCacheV2
// host-mirror/one-shared-array/one-time-verdict law is the identical
// mechanism; the staged label was its restatement).
//
// All four suggestion-only NotBenchable (the win metrics live in the
// arena's calibration/restore/boot infrastructure — no validator hosts
// them; the dial9/136 annotation posture). Fixtures:
// `corpus/fixtures/b412_perf_fixtures.rs` — standalone-compiled AND
// executed BEFORE entry (debug AND -O, 18/18); the gate caught TWO
// fixture defects pre-entry (the boot-probe closure was FnMut not Fn —
// the wait signatures now take the general bound; the corrupt cache
// member sorted FIRST alphabetically, defeating the "late member" arm —
// renamed so the corrupt member sorts last).
// NOTHING INVENTED — every quote read at the clone at the pin this
// session.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

/// Batch 412 — the mlxfast staged pool `rust_perf` rules.
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
pub fn b412_rules() -> Vec<PerfRule> {
    vec![
        r1_cache_restore_adopt_or_drop_whole_entry(),
        r2_acceptance_band_derived_from_own_axis_measured_cv(),
        r3_readiness_wait_liveness_first_deadline_bounded(),
        r4_calibration_band_two_sided_straddles_one_zero_cv_frozen_clock(),
    ]
}

// ---------------------------------------------------------------------------
// Rule 1 — cache-restore-adopt-or-drop-whole-entry
// ---------------------------------------------------------------------------
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
fn r1_cache_restore_adopt_or_drop_whole_entry() -> PerfRule {
    PerfRule {
        rule_id: "cache-restore-adopt-or-drop-whole-entry".to_string(),
        name: "a cache hit restores its artifact set ALL-OR-NOTHING — verify every \
member against the manifest the save wrote, touch the tree only after every \
member verifies; any mismatch is a miss (rebuild), never a partial restore"
            .to_string(),
        category: "caching".to_string(),
        buggy_code: r#"// restore that copies each artifact as it verifies: a corrupt LATE
// member is detected only AFTER the early members were installed —
// the tree mixes cache bytes and stale bytes and every existence
// check passes. The bench then measures a composite engine that no
// build stage ever authored.
fn restore(cache: &Cache, arts: &[Art]) -> Result<(), Miss> {
    for a in arts {
        let want = cache.manifest_of(a)?;
        if sha(file(a)) != want {
            return Err(Miss::Mismatch(a.name()));
        }
        install(a); // tree mutated BEFORE the whole set is known good
    }
    Ok(())
}
// caller: match restore(..) { Err(_) => rebuild() } — too late: the
// tree already carries half the cache generation."#
            .to_string(),
        fixed_code: r#"// the transaction: pass one VERIFIES (tree untouched), pass two
// INSTALLS. Any absent/mismatched member voids the whole entry —
// the refusal message names the member, so the rebuild is
// attributable.
fn restore(cache: &Cache, arts: &[Art]) -> Result<(), Miss> {
    let mut verified = Vec::with_capacity(arts.len());
    for a in arts {
        let want = cache.manifest_of(a)?; // absent member -> miss
        if sha(file(a)) != want {
            return Err(Miss::Mismatch(a.name())); // tree untouched
        }
        verified.push(a);
    }
    for a in verified {
        install(a); // only after every member verifies
    }
    Ok(())
}"#
        .to_string(),
        context_notes: Some(
            "MECHANISM (cache-admission integrity; caching): a build-artifact \
cache entry is a SET of members produced by ONE build generation (binaries + a \
library + its fingerprint sidecar + a CLI) — a hit that installs SOME members has \
manufactured a tree whose parts come from different generations: the missing member \
is discoverable, the partial one measures a composite engine no stage authored (the \
mixed-generation bench is measurement poison, not just waste). THE SOURCE'S LAW, \
verbatim at the pin (tools/build-cache.sh:72-77): the restore loop verifies each \
artefact's sha256 against the MANIFEST ('[[ \"$(sha256_of_file ...)\" == \"${want}\" \
]] || { log \"miss: ${a} in the cache does not match its manifest; refusing a partial \
restore\"; exit 1; }') and only the SECOND loop copies ('for a in \"${ARTEFACTS[@]}\"; \
do mkdir -p \"$(dirname \"${a}\")\"; cp -p ...; done') — the stated law (:21-22): 'The \
artefacts are verified by sha256 on restore against the MANIFEST the save wrote; any \
mismatch is a miss, never a partial restore.' THE KEY COMPLETENESS RIDER (:59-64, a \
measured incident): a worker restored from a cache keyed without the BUILD ROOT died \
loading its library ('Failed to load the default metallib', run 34030992573) — the \
content key must cover sources (dirty files re-hashed), toolchain, the library \
fingerprint, host, AND the build root; a key missing any axis silently serves bytes \
from a different generation. THE PRE-STAGED TWIN (tools/stage-baseline-workspace.sh:\
177-199): the pre-built library pair is adopted ONLY after its fingerprint equals THIS \
checkout's vendored-source fingerprint — 'A mismatch is fatal, never a fallback: the \
alternative is a metallib built from other Metal sources measuring the control leg' — \
and the fingerprint sidecar rides as a sibling so the binding stays verifiable at every \
later stage. ⛔ KINSHIP (adjudicated both directions): untrusted-download-verify-then-\
atomic-install (entries_69/B345) owns ONE artifact's verify-before-promote + atomic \
rename — THIS owns the SET transaction across a multi-member entry (the all-or-nothing \
semantics over a set is the load-bearing half; a single artifact cannot be partially \
installed, an artifact set is exactly where partial install happens), compose; \
raw-bytes-with-hit-backfill-once (entries_69) owns TIERED-HIT read economics (read \
once in a shared form, detached backfill) — different axis (how a hit is READ vs what \
a hit may INSTALL); vanished-entry-tolerant-eviction (entries_69) owns eviction-time \
tolerance of peer-caused NotFound — THIS the restore-time refusal of a partial entry \
(opposite phases of one cache); bit-parity-geometry-adoption-no-boot-timing \
(entries_gpu_122, the SIBLING repo's 9b engine) is the nearest SHAPE (verify-before-\
adopt with a permanent fallback lane) but its object is kernel GEOMETRY proven by bits \
vs THIS's artifact BYTES proven by digests — compose; reserve-then-commit-\
capacity-accounting (entries_58) owns BUDGET reservation, not integrity; \
frozen-artifact-plus-delta-tail-overlay (entries_81) owns cache COMPOSITION (base + \
tail), not entry atomicity. REGIME: any cache whose entries are member-sets (build \
caches, artifact stores, model bundles with sidecars, dataset packs with manifests); \
restore paths that write into a tree a consumer will build/measure from. ANTI-REGIME: \
single-artifact entries (B345's law already covers them); caches whose consumers \
tolerate mixed generations by design (content-addressed stores where every read \
re-verifies). NotBenchable-by-domain (the win is never MEASURING a mixed-generation \
tree — an integrity property with a measurement-honesty consequence; the second-read \
price per member is the only cost, and the cache-hit-skips-rebuild economics are the \
cache's value, not this law's). P113: upstream's own landed decision — decision \
content only. Source: Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923 (MIT) — \
tools/build-cache.sh:69-79 (the restore arm) + :21-22 (the stated law) + :59-64 (the \
root-in-key incident) + tools/stage-baseline-workspace.sh:177-199 (the pre-staged \
twin). Quotes byte-verified at the clone at the pin this session. MIT. Batch 412 / \
Plan 411 / Research 445."
                .to_string(),
        ),
        // Not benchable: the win axis is an infrastructure-integrity /
        // gate-honesty / boot-diagnosis / calibration verdict; no
        // validator hosts it (the dial9/136 annotation class).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 2 — acceptance-band-derived-from-own-axis-measured-cv
// ---------------------------------------------------------------------------
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
fn r2_acceptance_band_derived_from_own_axis_measured_cv() -> PerfRule {
    PerfRule {
        rule_id: "acceptance-band-derived-from-own-axis-measured-cv".to_string(),
        name: "each scored axis of a composite benchmark carries its own acceptance \
band DERIVED from that axis's own measured session CV — tight side = \
ceil-to-half-percent of 10x the per-axis CV, clamped to [4x the session spread, \
the prior era's band) — never one shared tolerance copied across axes or carried \
over unexamined from a prior era"
            .to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// one shared tolerance for both axes, carried over unexamined:
// the decode axis (CV 0.09%) accepts readings the prefill axis
// (CV 0.65%) would call noise — the shared band is tuned to the
// LOUDER axis and blind to the quieter one.
const BAND: f64 = 0.03;

fn decode_healthy(delta: f64) -> bool {
    delta.abs() <= BAND // a 1.5% decode shift passes: ordinary for
                        // prefill, 16x decode's own CV — and it rides
                        // into the scored composite unchecked
}

fn passes(speedup: f64) -> bool {
    speedup >= 1.02 // the decode floor; the prefill floor is not
                    // consulted — one verb gated, one rides free
}"#
        .to_string(),
        fixed_code: r#"// per-axis furniture, derived from each axis's own noise:
struct AxisCalibration { cv: f64, spread: f64, prior_band: f64 }

fn derive_band(c: &AxisCalibration) -> f64 {
    let tight = (10.0 * c.cv * 200.0).ceil() / 200.0; // ceil to 0.5%
    tight.max(4.0 * c.spread).min(c.prior_band)       // scatter floor,
}                                                     // never loosened

fn passes_floors(decode: f64, prefill: f64, d_floor: f64, p_floor: f64) -> bool {
    decode >= d_floor && prefill >= p_floor // BOTH verbs over BOTH floors
}

fn composite(d: Option<f64>, p: Option<f64>) -> Option<f64> {
    match (d, p) { // any degenerate sibling refuses the composite
        (Some(a), Some(b)) if a.is_finite() && b.is_finite() && a > 0.0 && b > 0.0 =>
            Some(a * 0.75 + b * 0.25),
        _ => None,
    }
}"#
        .to_string(),
        context_notes: Some(
            "MECHANISM (gate-furniture honesty; measurement-honesty): a \
composite benchmark's sibling verbs (prefill + decode, both seconds-per-token but \
different noise populations) each carry their OWN gate furniture — own acceptance \
band with own up/down tolerances (a symmetric health gate on the unscored axis, an \
asymmetric scored-axis gate with the tight side on regressions), own speedup floor \
(BOTH checked), own baseline, own weight — and the band magnitudes are DERIVED, not \
copied: tight side = ceil-to-half-percent of 10x the per-axis session CV at a PINNED \
calibration session (artifacts sha256-listed, the evidence directory named), bounded \
below by 4x the session spread ('so ordinary run-to-run scatter cannot trip it') and \
above by the prior era's band ('never loosened' — a monotone-tightening ratchet), the \
rejected alternative recorded for review. THE SOURCE, verbatim at the pin \
(Sources/MLXFastCore/Constants.swift:205-212): 'the band SHAPE is preserved from the \
qwen-era precedent (symmetric prefill health gate; asymmetric decode with the tight \
side on regressions), and the magnitudes are re-derived from this session's measured \
variability. The qwen precedent pinned its tight sides at ~7.7x the session CV \
(prefill 5% over CV 0.65%; decode +2% over CV 0.26%) and its decode gain cap at ~19x; \
i.e. bands sat ~8-25x over measured CV. Rule applied here: tight-side band = \
ceil-to-half-percent of 10x the per-axis session CV (10x sits at the conservative end \
of that precedent envelope):' THE CONSUMER ARITY (the label's 'sibling-verb' half): \
the composite refuses (return .nan) when EITHER sibling's input is non-finite or \
non-positive (Score.swift:34-43), the floors check BOTH verbs, and the derived-lines \
jq declines the row unless BOTH sibling metrics are positive (benchmark.sh:242/275/312 \
'select($p > 0 and $d > 0)' — the '// 0' defaults are guarded, not silent zeros). THE \
CROSS-ARTIFACT FACE (tools/lint-benchmark-manifest.py:310-324): the scoring constants \
are stated in TWO artifacts (manifest scoring + contract scoring_semantics) and a \
linter holds every shared key equal ('Only keys BOTH files state; a disagreement \
between them is the drift this catches'); the pair-support floor is additionally \
enforced at the measuring pin itself — 'the ruled floor rode entirely on the \
wrapper's argv' and a min 2/target 4 run once 'published a median over half the ruled \
support' (:213-221). ⛔ KINSHIP (adjudicated both directions): score-is-lower-median-\
pair-never-mean (entries_96, the SAME repo @ this pin, landed B402) owns score \
SELECTION ACROSS PROMPTS — THIS owns the PER-AXIS gate furniture that adjudicates \
each leg BEFORE aggregation: adjacent stages of one pipeline, compose never merge \
(entries_96 already quotes this repo's pairing law; THIS cites it as the sibling, \
never re-owns aggregation); token-count-from-authoritative-counter-not-stream-chunks \
(entries_84) owns the measurement UNIT's authority — THIS owns the NOISE unit (CV) a \
band is derived from: same family theme (honest units), different mechanism (counter \
authority vs band derivation); calibration-probe-swept-band-single-point-outlier \
(entries_94) owns the swept INPUT band for calibration probes — THIS the derived \
ACCEPTANCE band the calibrated data then feeds: the input side vs the gate side of \
one methodology, compose; benchmark-spread-above-threshold-reports-unstable \
(entries_70) owns the stability VERDICT on measured spread — THIS derives the band \
FROM the measured spread (the verdict's data is THIS rule's input), compose. REGIME: \
any composite benchmark whose axes have different noise populations; any gate whose \
tolerance was inherited from a prior era/model rather than derived from the data the \
gate now judges. ANTI-REGIME: single-axis benches (one band, derived or not, is the \
whole story); axes with no calibration session (derive from the first measured \
session and pin it — the absence of history is not license for a carried-over \
constant). NotBenchable-by-domain (the win is gate honesty — a band that neither \
trips on ordinary scatter nor admits lucky readings; consistent with the \
measurement-honesty suggestion-only family posture). P113: upstream's own landed \
decision — decision content only. Source: Layr-Labs/mlxfast-bonsai2-27b-engine @ \
0c7c3923 (MIT) — Sources/MLXFastCore/Constants.swift:189-221 (the band-derivation \
block) + Sources/MLXFastCore/AcceptanceBand.swift:36-67 (per-axis check) + \
Sources/MLXFastCore/Score.swift:34-64 (the composite refusal + both floors). Quotes \
byte-verified at the clone at the pin this session. MIT. Batch 412 / Plan 411 / \
Research 445."
                .to_string(),
        ),
        // Not benchable: the win axis is an infrastructure-integrity /
        // gate-honesty / boot-diagnosis / calibration verdict; no
        // validator hosts it (the dial9/136 annotation class).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 3 — readiness-wait-liveness-first-deadline-bounded
// ---------------------------------------------------------------------------
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
fn r3_readiness_wait_liveness_first_deadline_bounded() -> PerfRule {
    PerfRule {
        rule_id: "readiness-wait-liveness-first-deadline-bounded".to_string(),
        name: "a boot/readiness wait checks LIVENESS FIRST on every iteration — a \
child that exited before it was healthy is named and exits in ONE poll with its \
log attached, never discovered at the deadline — then probes readiness, and the \
whole wait is bounded by an explicit deadline whose expiry names the timeout and \
attaches the log"
            .to_string(),
        category: "concurrency".to_string(),
        buggy_code: r#"// probe-only wait: a child that CRASHED during boot is never
// ready, so the loop burns the ENTIRE timeout before failing — and
// the exit is the same boolean as a slow box, so the caller cannot
// tell 'died' from 'still booting' without re-running everything.
loop {
    if probe_socket() { break; }
    if start.elapsed() > DEADLINE { return Err("not healthy"); }
    sleep(POLL);
}
// a crashed resident costs the full 5400 s here — per lane, per boot."#
            .to_string(),
        fixed_code: r#"// liveness-first: poll 1 names a dead child; the deadline bounds
// only the SLOW class; both failures carry the log tail.
loop {
    if !alive(&child) { // liveness BEFORE readiness
        return Err(BootFault::ExitedBeforeHealthy(log_tail()));
    }
    if probe_socket() { break; }
    if start.elapsed() > DEADLINE {
        return Err(BootFault::NotHealthyWithin(DEADLINE, log_tail()));
    }
    sleep(POLL);
}"#
        .to_string(),
        context_notes: Some(
            "MECHANISM (wait-loop ordering; concurrency): a readiness wait \
has TWO exit classes — the child DIED (permanent, diagnosable NOW from its log) and \
the child is SLOW (needs the deadline) — and a probe-only loop collapses both into \
one timeout-shaped failure that costs the full budget per boot. THE SOURCE'S ORDER, \
verbatim at the pin (tools/resident-up.sh:582-600, the wait loop): liveness fires \
first ('if ! kill -0 \"${RESIDENT_PID}\" 2>/dev/null' -> 'the resident exited before \
it was healthy; its log:' + tail -40 + teardown + exit 1), then the readiness probe \
('if [[ -S \"${SOCKET_PATH}\" ]] && probe_resident ...' -> break), then the deadline \
('if (( now - start >= HEALTH_TIMEOUT_S ))' -> 'the resident was not healthy within \
${HEALTH_TIMEOUT_S}s; its log:' + tail + exit 1), on a 2 s poll — the deadline is an \
env-tuned VALIDATED ceiling ('HEALTH_TIMEOUT_S must be a positive integer', :363-364; \
default 5400 for the load + first hello). The ordering is load-bearing at the failure \
mode that matters: a crashed booting child fails in ONE poll with its log, so the \
diagnosis ('its log:') rides the failure itself — nobody re-runs a 90-minute boot to \
discover the exit code that was already known. ⛔ KINSHIP (adjudicated both \
directions): deadline-bounded-poll-of-submitted-completion-event (entries_gpu_60) \
owns the HOST-side deadline poll of a SUBMITTED WORK ITEM — the deadline axis is \
shared kinship, but the object differs (an in-flight compute event vs a child \
process's boot readiness) and gpu_60 carries no liveness-first ordering (its deadline \
IS the only exit class it needs); compose on the deadline discipline; \
relaxed-spin-acquire-confirm-metadata-flag-wait (entries_gpu_118) owns the DEVICE-\
side two-tier wait ladder + the three-way fault taxonomy — the fault-classification \
SPIRIT is shared (name the disease, never one boolean) but the layer differs (memory \
words vs processes); fail-stop-deadline-trap-bounded-cross-rank-wait (entries_gpu_153) \
owns waits that CANNOT fail gracefully (in-kernel, co-resident — the only sound action \
is a deliberate fault) — THIS is the graceful host-side twin (both exits return); \
spin-keep-awake-checked-capability-degrade-loud (entries_74) owns a clock-HOLDING \
spin (anti-sleep) with a checked capability — different mechanism (keeping the box \
awake vs waiting for a child); conditional-reuseaddr-after-liveness-probe (entries_91\
/B337) is the liveness-PROBE kin by shape — its probe decides WHETHER to rebind once; \
THIS probes liveness INSIDE the loop every iteration to classify the exit. REGIME: \
any boot/wait loop over a child process or service socket (residents, daemons, \
test-harness servers, CI workers); any place a crashed child currently costs the full \
timeout. ANTI-REGIME: in-process futures (the runtime's own cancellation already \
distinguishes dropped tasks); device-side waits (gpu_118/153 own those layers); waits \
where liveness is UNKNOWABLE (a peer on another host — the deadline + a heartbeat is \
that lane's shape). NotBenchable-by-domain (the win is boot-failure diagnosis cost — \
one poll vs the whole deadline — and the named-exit contract; no perf validator hosts \
it). P113: upstream's own landed decision — decision content only. Source: \
Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923 (MIT) — tools/resident-up.sh:582-600 \
(the wait loop) + :131/:363-364 (the validated deadline env). Quotes byte-verified at \
the clone at the pin this session. MIT. Batch 412 / Plan 411 / Research 445."
                .to_string(),
        ),
        // Not benchable: the win axis is an infrastructure-integrity /
        // gate-honesty / boot-diagnosis / calibration verdict; no
        // validator hosts it (the dial9/136 annotation class).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}

// ---------------------------------------------------------------------------
// Rule 4 — calibration-band-two-sided-straddles-one-zero-cv-frozen-clock
// ---------------------------------------------------------------------------
#[must_use]
#[allow(clippy::too_many_lines)] // data table, not logic (the corpus convention)
fn r4_calibration_band_two_sided_straddles_one_zero_cv_frozen_clock() -> PerfRule {
    PerfRule {
        rule_id: "calibration-band-two-sided-straddles-one-zero-cv-frozen-clock".to_string(),
        name: "a calibration health band is TWO-SIDED and must straddle the neutral \
point (low < 1 < high — a one-sided band can never call the judged leg healthy in \
one direction); every measured value and band edge must be POSITIVE (a zero CV \
across repeated passes is a frozen clock, not a stable box — refused, not \
celebrated); a CV above the ceiling is equally a refusal, not a warning; and the \
calibration can only STOP a score, never move one (the live ratio is the \
denominator, never the calibration)"
            .to_string(),
        category: "measurement-honesty".to_string(),
        buggy_code: r#"// the calibration file IS the denominator and the band is
// whatever the last run said: a sub-1.0 baseline silently shrinks
// every candidate's ratio (score inflation), and a suspiciously
// perfect repetition (CV = 0) is recorded as a GREAT box.
fn score(candidate: f64, calibration: f64) -> f64 {
    candidate / calibration // the stored baseline moves every score
}

fn accept(entry: &Cal) -> bool {
    entry.high >= 1.0 // one-sided: improvements can never be judged
}"#
        .to_string(),
        fixed_code: r#"// the band is a HEALTH GATE on the control leg, never the
// denominator; it straddles 1; its edges and the CV are positive;
// zero CV refuses; high CV refuses; refusals write NO file.
fn validate(entry: &Cal, cv_ceiling: f64) -> Result<(), String> {
    for v in [entry.mean, entry.cv, entry.low, entry.high] {
        if !v.is_finite() || v <= 0.0 {
            return Err(format!(
                "{v}; every measured value and every band edge must be positive \
                 (a zero CV across repeated passes is a frozen clock, not a stable box)"));
        }
    }
    if !(entry.low < 1.0 && 1.0 < entry.high) {
        return Err(format!(
            "band [{}, {}]; a band must straddle 1 (low < 1 < high), or the leg it \
             judges can never be healthy", entry.low, entry.high));
    }
    if entry.cv > cv_ceiling {
        return Err("CV above the ceiling is a refusal, not a warning — \
                    the box did not repeat itself".into());
    }
    Ok(())
}

fn gate(leg_ratio: f64, band: &Cal) -> Option<bool> {
    // inside the band: NO opinion — the score stands on its own legs.
    (leg_ratio < band.low || leg_ratio > band.high).then_some(false)
}"#
        .to_string(),
        context_notes: Some(
            "MECHANISM (calibration honesty; measurement-honesty): a \
ranked run is PAIRED (a serial-control leg on the organizer's reference tree + a \
candidate leg, same job, same box, the LIVE ratio is the score) and the calibration \
file is that pairing's HEALTH BAND, recorded per axis per golden with the mean, the \
CV and the band — three laws make it trustworthy. (1) THE ROLE LAW, verbatim at the \
pin (tools/calibrate-box.sh:7-21): 'The calibration file this script writes is NEVER \
that denominator. It is a HEALTH BAND: it records what this box's serial-control leg \
costs when the box is itself, so benchd can kill a run by name when leg 1 lands \
outside the band instead of sealing a score measured on a machine that was thermally \
throttled, shared, or otherwise not the machine that was calibrated. So a stale \
calibration file cannot move a score. It can only stop one.' (2) THE ZERO-CV LAW \
(tools/ranked-box-preflight.sh:540-544, the calibration validator): every field must \
be a positive finite number — 'a zero CV across repeated passes is a frozen clock, \
not a stable box' — perfectly identical readings are a fabrication/frozen-clock \
signal, never a stability credential; and the TWO-SIDED law rides the same validator \
(:546-553): 'a band must straddle 1 (low < 1 < high), or the leg it judges can never \
be healthy' — a one-sided band is structurally unable to judge one direction of \
leg behavior. (3) THE HIGH-CV LAW (tools/calibrate-box.sh:22-25): the calibrator \
'REFUSES to write a file when the CV exceeds 1% on either axis, because a box that \
cannot repeat itself has no band worth recording. That refusal is the point of the \
four passes, and this script does not soften it' — and the refusal line (:252-253) \
names the absence ('benchd wrote no calibration file ... a CV above 1% on either axis \
is a refusal, not a warning -- the box did not repeat itself'): the missing artifact \
IS the alarm. THE SCOPE RIDER (:32-42): the calibration runs under the SAME fleet GPU \
lock as the ranked measurement ('A calibration measured beside another GPU owner is a \
calibration of the wrong machine'), inside the REFERENCE tree only ('the control \
leg's engine, its Metal library and its weights all come from the pinned commit, so \
nothing this submission changed can move the denominator or the band'), and boots NO \
resident of its own (a candidate-tree resident would measure the wrong engine — the \
defect a public run hit). ⛔ KINSHIP (adjudicated both directions): \
benchmark-spread-above-threshold-reports-unstable (entries_70) owns the HIGH-spread \
stability VERDICT (report UNSTABLE, never a flaky headline) — THIS owns the full \
calibration contract: the LOW/zero pole (frozen clock), the straddle law, the \
role law (stop-never-move), and the refusal-writes-nothing shape; the high-CV leg \
COMPOSES with entries_70 (the two poles of one stability axis), the rest is new \
decision content; aa-control-prices-the-fidelity-instrument (entries_08) owns the \
A/A control that prices an instrument's fidelity — the zero-CV refusal is the \
CALIBRATION-side twin of the A/A control's too-perfect signal (both refuse readings \
too good to be measured), cite; calibration-probe-swept-band-single-point-outlier \
(entries_94) owns the swept INPUT band for probes — THIS the health band the \
calibration WRITES (input band vs output band), compose; \
measured-round-cost-spec-controller (entries_gpu_161) owns contamination guards on a \
measured table — different artifact. REGIME: any paired/measured workflow whose \
control leg carries a recorded health band (scoreboards, league cells, hardware \
calibration files, canary baselines); any consumer tempted to divide by a stored \
baseline. ANTI-REGIME: absolute (unpaired) timings — there is no control leg to band; \
bands already consumed as denominators by an audited protocol (the law is the \
auditor). NotBenchable-by-domain (the win is calibration honesty — a score sealed on \
a throttled/shared box is worse than no score; no perf validator hosts it). P113: \
upstream's own landed decision — decision content only. Source: \
Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923 (MIT) — tools/calibrate-box.sh:7-42 \
(the role law + the CV refusal + the scope rider) + :245-253 (the refusal line) + \
tools/ranked-box-preflight.sh:525-553 (the calibration validator: positivity, the \
zero-CV clause, the straddle law). Quotes byte-verified at the clone at the pin this \
session. MIT. Batch 412 / Plan 411 / Research 445."
                .to_string(),
        ),
        // Not benchable: the win axis is an infrastructure-integrity /
        // gate-honesty / boot-diagnosis / calibration verdict; no
        // validator hosts it (the dial9/136 annotation class).
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: None,
    }
}
