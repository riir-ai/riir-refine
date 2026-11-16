//! Batch 367 rust_perf fixtures — the opal harness measurement-integrity
//! half (Plan 381 / Research 413): `xingjian-bai/opal` @
//! `97762859ba5e2527e5c6f63d9890dfbab860d51f` (MIT). ORIGINAL plain-Rust
//! stand-ins — zero opal source pasted (the detector ALGEBRA is re-authored
//! with our own constants; the upstream's measured figures stay in the
//! research note, never in code). Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b367_fixtures.rs && ./b367_fixtures`
//! (the b363/b365/b366 pre-entry discipline).
//!
//! Indexing convention (one, everywhere): rows[k] is the sampled vector
//! AFTER update k; m[k] (k in 1..L) is the sampled change of update k;
//! big[k] says update k's change is a jump. Opal's own analyze() carries
//! the same law shifted by one (their D[j-1] is the change of update j) —
//! the first draft of this fixture mixed the two conventions and the
//! pre-entry gate caught it (three reds), which is what the gate is for.
//!
//! Arms per rule:
//! - F1 `eval-window-state-restore-roundtrip-detection`: a scored subject
//!   ring-sampled every update by a strided fixed sample (seeded offset,
//!   stride coprime with every dimension so the sample does not fall into
//!   one row or one column); the detector algebra (jump = m >
//!   THRESHOLD(5.0) x typical(median of non-zero m) AND m >
//!   JUMP_FLOOR(1e-3) x sampled norm; round trip = SEEN(0.5) away /
//!   BACK(0.25) remaining / isolation both sides) must (a) FLAG the
//!   cheater scale-and-restore around a measurement window, (b) record the
//!   persistent weight-average swap as INFORMATIONAL (not a flag), (c) NOT
//!   flag a legitimate high-lr stretch (a contiguous run of big updates),
//!   (d) NOT flag a sustained oscillation (also a contiguous stretch), (e)
//!   document the panel falsifiers as honest not-detected (an out-and-back
//!   older than the ring window; a move smeared across sub-floor updates),
//!   (f) NOT false-flag a warm-restart legit jump, and (g) the tampered
//!   arm (a detector without the floor, judging m against typical alone)
//!   must RED on the frozen-tensor zero-typical noise case;
//! - F4 `capacity-gated-exact-undo-loud-degrade`: a swap-with-rollback
//!   harness whose snapshot is admitted by a byte budget — under budget
//!   the restore is byte-identical and the swap undone; over budget the
//!   snapshot is skipped with a LOUD named event (never silent) and the
//!   harness degrades to report-not-undo; the regime condition carries
//!   (a first-step failure touching state OUTSIDE the snapshot — an RNG
//!   stream — is not undone, documented); the tampered arm (a silent skip
//!   that reports "undo available") reds;
//! - F5 `self-caused-wait-excluded-from-rate-estimate`: a rate estimated
//!   from wall intervals must exclude the consult pauses the mechanism
//!   itself introduces, and take the median over a small window so an
//!   interval carrying an evaluation does not shrink the estimate; the
//!   naive arm (raw intervals, mean) overstates the time-per-update; the
//!   panel falsifiers (bimodal latency hides the tail under a median —
//!   demonstrated; each accepted rewrite stales the estimate — the
//!   bounded window is the disclosed mitigation) ride as regime notes.

// ─────────────────────────────────────────────────────────────────────────────
// Shared subject model: a "guarded state" is a 12x16 f64 table the scored
// run mutates every update; the measurement window re-scores it at a
// NON-announced step, which is what the detector watches.
// ─────────────────────────────────────────────────────────────────────────────

const RING_BEFORE: usize = 32;
const RING_AFTER: usize = 4;
const RING_LEN: usize = RING_BEFORE + RING_AFTER + 2;

/// The strided fixed sample of a subject: (offset, stride, count) in its
/// flat row-major storage, stride coprime with every dimension > 1 — the
/// sample must not fall into one row or one column.
#[derive(Clone, Copy, Debug)]
struct SampleSpec {
    offset: usize,
    stride: usize,
    count: usize,
}

fn gcd(mut a: usize, mut b: usize) -> usize {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

fn stride_for(numel: usize, count: usize, rows: usize, cols: usize) -> usize {
    let mut s = (numel / count).max(1);
    // coprime with every dimension > 1 (opal's _stride law)
    while s > 1 && ((rows > 1 && gcd(s, rows) != 1) || (cols > 1 && gcd(s, cols) != 1)) {
        s -= 1;
    }
    s
}

/// The ring: the sampled entries after each of the last RING_LEN updates.
struct SampleRing {
    spec: SampleSpec,
    samples: Vec<Vec<f64>>, // samples[slot] = the sampled values after that update
    slot_update: Vec<Option<usize>>,
}

impl SampleRing {
    fn new(rows: usize, cols: usize, seed: u64) -> Self {
        let numel = rows * cols;
        let count = 256.min(numel);
        let stride = stride_for(numel, count, rows, cols);
        let count = count.min((numel - 1) / stride + 1);
        // seeded offset (a tiny LCG stands in for the seeded RNG)
        let mut st = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        st ^= st >> 33;
        let offset = (st as usize) % (numel - (count - 1) * stride);
        Self {
            spec: SampleSpec {
                offset,
                stride,
                count,
            },
            samples: vec![Vec::new(); RING_LEN],
            slot_update: vec![None; RING_LEN],
        }
    }

    fn sample_of(&self, table: &[f64]) -> Vec<f64> {
        let s = self.spec;
        (0..s.count)
            .map(|k| table[s.offset + k * s.stride])
            .collect()
    }

    /// record the sampled entries after update u
    fn record(&mut self, u: usize, table: &[f64]) {
        let slot = u % RING_LEN;
        self.samples[slot] = self.sample_of(table);
        self.slot_update[slot] = Some(u);
    }

    fn have(&self, j: i64) -> bool {
        j >= 0 && self.slot_update[(j as usize) % RING_LEN] == Some(j as usize)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// F1 — the detector algebra (re-authored; constants pinned in the rule text)
// ─────────────────────────────────────────────────────────────────────────────

const THRESHOLD: f64 = 5.0;
const JUMP_FLOOR: f64 = 1e-3;
const SEEN: f64 = 0.5;
const BACK: f64 = 0.25;
const MIN_MOVING: usize = 4;

#[derive(Debug, PartialEq)]
enum Verdict {
    /// a round trip: the subject was moved away for the measurement and
    /// moved back after it — FLAG the run
    RoundTrip {
        changed_at: usize,
        restored_at: usize,
    },
    /// a lone jump not undone: informational (a training intervention,
    /// never a flag)
    InfoJump { changed_at: usize },
    /// nothing to report
    Quiet,
}

fn dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// The window detector: walk the ring around the evaluation at step `t`,
/// classify the subject's sampled trajectory. `typical` = the median of the
/// NON-zero per-update sampled changes in the window (a frozen subject has
/// none — typical 0); a jump is an update whose sampled change exceeds
/// THRESHOLD x typical AND JUMP_FLOOR x the sampled norm (the absolute
/// floor is what keeps a zero-typical window quiet). Isolation both sides:
/// a contiguous stretch of jumps (a high-lr regime, a sustained
/// oscillation) is not a round trip.
fn detect(ring: &SampleRing, t: usize) -> Verdict {
    let t_i = t as i64;
    let mut lo = t_i;
    while lo - 1 >= t_i - (RING_BEFORE as i64) - 1 && ring.have(lo - 1) {
        lo -= 1;
    }
    let mut hi = t_i;
    while hi + 1 <= (t + RING_AFTER) as i64 && ring.have(hi + 1) {
        hi += 1;
    }
    if lo >= hi {
        return Verdict::Quiet;
    }
    let updates: Vec<usize> = ((lo as usize)..=(hi as usize)).collect();
    let rows: Vec<&Vec<f64>> = updates
        .iter()
        .map(|&u| &ring.samples[u % RING_LEN])
        .collect();
    let l = rows.len();
    let it = updates.iter().position(|&u| u == t).expect("t in window");

    // m[k-1] (k in 1..l): the sampled change of update updates[k]
    let m: Vec<f64> = (1..l).map(|k| dist(&rows[k], &rows[k - 1])).collect();
    // wn[k]: the sampled norm of rows[k] (the floor compares against the
    // pre-change norm wn[k-1], opal's wn[:-1])
    let wn: Vec<f64> = (0..l)
        .map(|k| rows[k].iter().map(|v| v * v).sum::<f64>().sqrt())
        .collect();

    // typical: the median of the NON-zero changes (frozen subject -> 0)
    let mut nz: Vec<f64> = m.iter().cloned().filter(|&x| x > 0.0).collect();
    nz.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let typical = if nz.len() >= MIN_MOVING {
        nz[nz.len() / 2]
    } else {
        0.0
    };

    // big[k] for k in 1..l (index 0 unused): update k's change is a jump
    let mut big = vec![false; l];
    for k in 1..l {
        big[k] =
            m[k - 1] > THRESHOLD * typical && m[k - 1] > JUMP_FLOOR * wn[k - 1] && m[k - 1] > 0.0;
    }

    // a = the last jump at or before the evaluation (a row index)
    let before: Vec<usize> = (1..=it).filter(|&k| big[k]).collect();
    let a = match before.last() {
        Some(&a) => a,
        None => return Verdict::Quiet,
    };
    // the evaluation saw the moved point: |s_t - s_(a-1)| >= SEEN * m_a
    let away = dist(rows[it], rows[a - 1]);
    if away < SEEN * m[a - 1] {
        return Verdict::Quiet; // moved back before the evaluation: not what it saw
    }
    // not a lone change: part of a stretch of large updates (a high
    // learning rate, a sustained oscillation)
    if a < 2 || big[a - 1] {
        return Verdict::InfoJump {
            changed_at: updates[a],
        };
    }
    // b = the first jump after the evaluation
    let after: Vec<usize> = ((it + 1)..l).filter(|&k| big[k]).collect();
    if let Some(&b) = after.first() {
        let back = dist(rows[b], rows[a - 1]);
        // the update just after b is not itself a jump (past the ring the
        // scenario is complete — the fixture's final posture)
        let isolated_after = (b + 1 < l && !big[b + 1]) || (b + 1 >= l);
        if back <= BACK * away && isolated_after {
            return Verdict::RoundTrip {
                changed_at: updates[a],
                restored_at: updates[b],
            };
        }
    }
    if a + 1 < l && big[a + 1] {
        return Verdict::InfoJump {
            changed_at: updates[a],
        }; // followed by more large updates
    }
    Verdict::InfoJump {
        changed_at: updates[a],
    }
}

/// A scripted subject: base drift every update, plus per-update overlays
/// the scenario arms apply.
struct ScriptedSubject {
    table: Vec<f64>,
}

impl ScriptedSubject {
    fn new(n: usize) -> Self {
        let table: Vec<f64> = (0..n).map(|i| ((i % 17) as f64) * 0.25 + 0.5).collect();
        Self { table }
    }

    fn drift(&mut self) {
        for v in self.table.iter_mut() {
            *v += 1e-5 * (0.5 + (*v * 7.0).fract());
        }
    }

    fn scale_all(&mut self, factor: f64) {
        for v in self.table.iter_mut() {
            *v *= factor;
        }
    }

    fn set_all(&mut self, value: f64) {
        for v in self.table.iter_mut() {
            *v = value;
        }
    }

    fn spike_subset(&mut self, factor: f64) {
        // a big change on a slice of the subject (a legitimate high-lr
        // class: several consecutive big updates)
        for (i, v) in self.table.iter_mut().enumerate() {
            if i % 3 == 0 {
                *v *= factor;
            }
        }
    }

    fn ramp_all(&mut self, delta: f64) {
        for v in self.table.iter_mut() {
            *v += delta;
        }
    }
}

/// Run a scenario: `steps` updates, each applying `script` after the base
/// drift; the measurement window re-scores at step `t`; the detector
/// resolves against the ring. Returns (ring, verdict).
fn run_scenario(
    steps: usize,
    t: usize,
    script: impl Fn(usize, &mut ScriptedSubject),
) -> (SampleRing, Verdict) {
    let mut subject = ScriptedSubject::new(12 * 16);
    let mut ring = SampleRing::new(12, 16, 20261018);
    ring.record(0, &subject.table);
    for u in 1..=steps {
        subject.drift();
        script(u, &mut subject);
        ring.record(u, &subject.table);
    }
    let verdict = detect(&ring, t);
    (ring, verdict)
}

#[test]
fn f1_cheater_scale_and_restore_around_the_window_flags() {
    // The cheat: multiply the subject by a probe factor just before the
    // evaluation (update t-1), hold it through the window, restore it at
    // t+2. The detector must see: away jump before t, the evaluation
    // seeing the moved point, a back jump after t, isolated both sides.
    let t = 20usize;
    let (_ring, v) = run_scenario(40, t, |u, s| {
        if u == t - 1 {
            s.scale_all(8.0); // move the subject to the probe point
        }
        if u == t + 2 {
            s.scale_all(1.0 / 8.0); // move it back after
        }
    });
    assert!(
        matches!(v, Verdict::RoundTrip { .. }),
        "the cheater scale-and-restore must FLAG as a round trip, got {v:?}"
    );
}

#[test]
fn f1_persistent_swap_is_informational_never_a_flag() {
    // A weight average swapped in for good: one big jump, never undone.
    // The detector records an informational eval_jump; the run is NOT
    // flagged (a training intervention, not a cheat).
    let t = 20usize;
    let (_ring, v) = run_scenario(40, t, |u, s| {
        if u == 12 {
            s.set_all(9.0); // swap in an average for good
        }
    });
    assert_eq!(
        v,
        Verdict::InfoJump { changed_at: 12 },
        "a persistent swap is an informational jump, got {v:?}"
    );
}

#[test]
fn f1_legitimate_high_lr_stretch_is_not_flagged() {
    // Several consecutive big updates (a high learning rate): a contiguous
    // stretch of jumps, not a lone move-and-restore. Not flagged.
    let t = 20usize;
    let (_ring, v) = run_scenario(40, t, |u, s| {
        if (16..=19).contains(&u) {
            s.spike_subset(2.5); // a burst of big updates BEFORE the window
        }
    });
    assert!(
        !matches!(v, Verdict::RoundTrip { .. }),
        "a high-lr stretch must NOT flag as a round trip, got {v:?}"
    );
}

#[test]
fn f1_sustained_oscillation_is_not_flagged() {
    // A sustained oscillation as the isolation law means it: a CONTIGUOUS
    // stretch of large updates (every update big — a bad-lr oscillation).
    // The stretch check (the update before the last jump is itself a
    // jump) keeps it off the round-trip template. (A gapped square wave
    // with drift between the jumps is a DIFFERENT thing: that is the
    // A2 cheat shape — move-and-restore tied to the evaluation — and the
    // detector legitimately flags it; see f1_cheater.)
    let t = 20usize;
    let (_ring, v) = run_scenario(40, t, |u, s| {
        if (8..=32).contains(&u) {
            if u % 2 == 0 {
                s.scale_all(2.0);
            } else {
                s.scale_all(0.5);
            }
        }
    });
    assert!(
        !matches!(v, Verdict::RoundTrip { .. }),
        "a sustained oscillation (contiguous big stretch) must NOT flag, got {v:?}"
    );
}

#[test]
fn f1_falsifier_out_and_back_slower_than_the_ring_window_is_not_detected() {
    // The panel falsifier: the excursion starts before the ring window
    // (the before-jump scrolled out), so the template never completes.
    // DOCUMENTED not-detected — the ring cannot see past its own length.
    // t=36, steps=44: the ring holds updates 7..44; the move at u=4 is
    // outside every window the detector can open at t.
    let t = 36usize;
    let (_ring, v) = run_scenario(44, t, |u, s| {
        if u == 4 {
            s.scale_all(8.0); // outside the ring window by the time t is checked
        }
        if u == t + 2 {
            s.scale_all(1.0 / 8.0); // the back jump IS in the window — alone
        }
    });
    assert_eq!(
        v,
        Verdict::Quiet,
        "an excursion older than the ring window must be NOT DETECTED (the documented blind spot), got {v:?}"
    );
}

#[test]
fn f1_falsifier_smeared_move_across_subfloor_updates_is_not_detected() {
    // The panel falsifier, honestly modeled for THIS algebra: the move is
    // spread across updates each under the ABSOLUTE floor (JUMP_FLOOR x
    // sampled norm) AND under THRESHOLD x typical — no single update arms
    // the template, so a ramp that moves the subject meaningfully in
    // aggregate is invisible. White-box: assert every in-window per-update
    // delta is under both bounds; black-box: detect == Quiet; and the
    // witness: the subject's mean at the ramp peak is far past what the
    // base drift alone would have moved it.
    let t = 76usize; // the ring at steps=80 holds updates 43..80
    let ramp = 1.2e-4;
    let mut subject = ScriptedSubject::new(12 * 16);
    let mut ring = SampleRing::new(12, 16, 20261018);
    ring.record(0, &subject.table);
    for u in 1..=80usize {
        subject.drift();
        if (41..=56).contains(&u) {
            subject.ramp_all(ramp); // 16 in-window ramp updates, each sub-floor
        }
        ring.record(u, &subject.table);
    }
    // white-box (opal's exact jump condition): NO single update in the
    // window is a jump — a delta is a jump only when it clears BOTH the
    // relative bound (THRESHOLD x typical) AND the absolute floor
    // (JUMP_FLOOR x the sampled norm); the smear's deltas stay under the
    // floor, which is the clause that keeps the detector quiet here.
    let rows: Vec<&Vec<f64>> = (43..=80).map(|u| &ring.samples[u % RING_LEN]).collect();
    let mut nz: Vec<f64> = Vec::new();
    for k in 1..rows.len() {
        let d = dist(rows[k], rows[k - 1]);
        nz.push(d);
    }
    let mut sorted = nz.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let typical = sorted[sorted.len() / 2];
    let wn = rows[0].iter().map(|v| v * v).sum::<f64>().sqrt();
    for &d in &nz {
        let is_jump = d > THRESHOLD * typical && d > JUMP_FLOOR * wn && d > 0.0;
        assert!(d > 0.0, "every smeared delta is positive");
        assert!(
            d < JUMP_FLOOR * wn,
            "every smeared delta is under the ABSOLUTE floor ({d} vs {})",
            JUMP_FLOOR * wn
        );
        assert!(
            !is_jump,
            "no single smeared update may arm the template (jump at d={d})"
        );
    }
    let v = detect(&ring, t);
    assert_eq!(
        v,
        Verdict::Quiet,
        "a smeared sub-floor move must be NOT DETECTED (the documented blindness), got {v:?}"
    );
    // the witness: the ramp moved the mean 12x past the drift's own move
    let peak_mean =
        ring.samples[56 % RING_LEN].iter().sum::<f64>() / ring.samples[56 % RING_LEN].len() as f64;
    let start_mean =
        ring.samples[43 % RING_LEN].iter().sum::<f64>() / ring.samples[43 % RING_LEN].len() as f64;
    let drift_only = 15.0 * 1e-5; // what drift alone would add per element
    assert!(
        peak_mean - start_mean > 8.0 * drift_only,
        "the ramp's aggregate move must dwarf the drift's ({})",
        peak_mean - start_mean
    );
}

#[test]
fn f1_warm_restart_legit_jump_does_not_false_flag() {
    // A warm restart: one legit big jump (an average restored at a
    // checkpoint boundary), then ordinary drift, never undone. The
    // informational arm holds; no flag.
    let t = 24usize;
    let (_ring, v) = run_scenario(40, t, |u, s| {
        if u == 22 {
            s.set_all(4.0); // the checkpoint's average, kept
        }
    });
    assert!(
        !matches!(v, Verdict::RoundTrip { .. }),
        "a warm-restart legit jump must NOT false-flag, got {v:?}"
    );
}

#[test]
fn f1_tampered_detector_without_the_floor_reds_on_frozen_tensors() {
    // The gate: a detector that judges m against `typical` ALONE (no
    // JUMP_FLOOR absolute floor) fires on a FROZEN subject — fewer than
    // MIN_MOVING non-zero changes in the window means typical = 0 (opal's
    // own frozen-tensor clause), so THRESHOLD x typical = 0 and ANY
    // epsilon perturbation exceeds it. The floor is load-bearing; without
    // it the detector is a coin flip on noise. The tampered detector must
    // RED here; the real one (with the floor) stays quiet.
    let mut subject = ScriptedSubject::new(8 * 8);
    let mut ring = SampleRing::new(8, 8, 7);
    ring.record(0, &subject.table);
    for u in 1..=16 {
        // FROZEN: no drift at all — the subject never moves on its own
        if u == 10 {
            // one tiny nudge on half the subject: noise, not a jump
            for (i, v) in subject.table.iter_mut().enumerate() {
                if i % 2 == 0 {
                    *v += 1e-12;
                }
            }
        }
        ring.record(u, &subject.table);
    }
    let rows: Vec<&Vec<f64>> = (0..=16).map(|u| &ring.samples[u % RING_LEN]).collect();
    let mut nz: Vec<f64> = Vec::new();
    for k in 1..rows.len() {
        let d = dist(rows[k], rows[k - 1]);
        if d > 0.0 {
            nz.push(d);
        }
    }
    // opal's frozen clause: fewer than MIN_MOVING non-zero changes -> typical = 0
    let typical = if nz.len() >= MIN_MOVING {
        let mut s = nz.clone();
        s.sort_by(|a, b| a.partial_cmp(b).unwrap());
        s[s.len() / 2]
    } else {
        0.0
    };
    assert!(
        nz.len() < MIN_MOVING && typical == 0.0,
        "the frozen window must collapse typical to 0"
    );
    // tampered: typical alone — any nonzero change exceeds THRESHOLD x 0
    let mut tampered_fired = false;
    for k in 1..rows.len() {
        let d = dist(rows[k], rows[k - 1]);
        if d > 0.0 && d > THRESHOLD * typical {
            tampered_fired = true;
        }
    }
    assert!(
        tampered_fired,
        "the tampered no-floor detector must RED on the frozen-tensor noise (typical {typical})"
    );
    // the real detector's floor holds the same window quiet
    let v = detect(&ring, 12);
    assert!(
        !matches!(v, Verdict::RoundTrip { .. }),
        "the real detector (with JUMP_FLOOR) must stay quiet on the noise nudge, got {v:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F4 — capacity-gated-exact-undo-loud-degrade
// ─────────────────────────────────────────────────────────────────────────────

const SNAPSHOT_BUDGET_BYTES: usize = 64; // the fixture's snapshot_limit

struct SwapHarness {
    state: Vec<u64>,
    rng_stream: u64, // OUTSIDE the snapshot: the regime-condition witness
    events: Vec<String>,
    last_undo: Option<Vec<u64>>,
}

impl SwapHarness {
    fn new(n: usize) -> Self {
        Self {
            state: (0..n as u64).collect(),
            rng_stream: 0xC0FFEE,
            events: Vec::new(),
            last_undo: None,
        }
    }

    /// byte estimate = the state's bytes (the fixture's `nbytes` stand-in)
    fn snapshot(&mut self) -> bool {
        let size = self.state.len() * 8;
        if size > SNAPSHOT_BUDGET_BYTES {
            // LOUD: the named event, never silent
            self.events.push(format!(
                "snapshot-size: the copy that undoes a failing new program would take {:.2} bytes, above \
                 snapshot_limit_bytes={SNAPSHOT_BUDGET_BYTES}; it is skipped (raise the limit to keep it), \
                 so a failure is not undone",
                size as f64
            ));
            return false;
        }
        self.last_undo = Some(self.state.clone());
        true
    }

    /// the candidate program's first step: mutate the state (and the
    /// OUT-OF-SNAPSHOT RNG stream)
    fn candidate_first_step(&mut self) {
        for v in self.state.iter_mut() {
            *v = v.wrapping_mul(3).wrapping_add(1);
        }
        self.rng_stream = self
            .rng_stream
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
    }

    /// exact undo: restore the state IN PLACE (the same objects — the
    /// fixture models aliasing by copying back into the same Vec)
    fn undo(&mut self) -> bool {
        match self.last_undo.take() {
            Some(copy) => {
                for (dst, src) in self.state.iter_mut().zip(copy) {
                    *dst = src;
                }
                true
            }
            None => false,
        }
    }

    /// the degraded posture: report-not-undo
    fn failure_report(&self, undone: bool) -> String {
        if undone {
            "reverted: the update was undone exactly".to_string()
        } else {
            "reverted: this update was not undone (no snapshot); it may be partly applied"
                .to_string()
        }
    }
}

#[test]
fn f4_under_budget_the_undo_is_exact() {
    let mut h = SwapHarness::new(4); // 32 bytes <= budget
    let before_state = h.state.clone();
    assert!(h.snapshot(), "under budget: the snapshot is taken");
    h.candidate_first_step();
    assert_ne!(
        h.state, before_state,
        "the candidate must have moved the state"
    );
    assert!(h.undo(), "the undo restores");
    assert_eq!(h.state, before_state, "the restore is byte-identical");
    assert!(h.events.is_empty(), "no events on the happy path");
}

#[test]
fn f4_over_budget_skips_loud_and_degrades_to_report_not_undo() {
    let mut h = SwapHarness::new(16); // 128 bytes > budget
    assert!(!h.snapshot(), "over budget: skipped");
    assert_eq!(h.events.len(), 1, "exactly one LOUD named event");
    assert!(
        h.events[0].contains("snapshot-size") && h.events[0].contains("snapshot_limit_bytes"),
        "the event names itself and the limit (the loud-degrade contract)"
    );
    h.candidate_first_step();
    let after = h.state.clone();
    assert!(!h.undo(), "no undo available");
    let report = h.failure_report(false);
    assert!(
        report.contains("not undone"),
        "the degraded posture reports report-not-undo: {report}"
    );
    assert_eq!(
        h.state, after,
        "the state keeps whatever the failed step did"
    );
}

#[test]
fn f4_regime_condition_rng_stream_outside_the_snapshot_is_not_restored() {
    // The regime condition: a snapshot that fully captures the MUTABLE
    // STATE UNDER TEST is the guarantee's boundary — a first-step failure
    // touching state OUTSIDE the snapshot (an RNG stream) is not undone.
    let mut h = SwapHarness::new(4);
    let rng_before = h.rng_stream;
    assert!(h.snapshot());
    h.candidate_first_step(); // advances BOTH the state and the stream
    assert!(h.undo());
    assert_eq!(h.state.len(), 4);
    assert_ne!(
        h.rng_stream, rng_before,
        "the RNG stream is outside the snapshot: NOT undone (the documented regime condition)"
    );
}

#[test]
fn f4_tampered_silent_skip_reports_undo_available_reds() {
    // The gate: a harness that skips the snapshot but STILL reports undo
    // available is the silent-lie class the loud-degrade law forbids.
    let mut h = SwapHarness::new(16);
    let took = h.snapshot();
    // tampered: pretend the undo is armed anyway
    let tampered_undo_armed = true; // the lie
    assert!(!took, "the skip happened");
    let report = if tampered_undo_armed {
        h.failure_report(true) // claims "undone exactly"
    } else {
        h.failure_report(false)
    };
    assert!(
        report.contains("undone exactly"),
        "the tampered harness lies: {report}"
    );
    // the real harness never claims an exact undo without a snapshot:
    // its report path is driven by the snapshot's own verdict
    let real_report = h.failure_report(took);
    assert!(
        real_report.contains("not undone"),
        "the honest path reports the truth: {real_report}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F5 — self-caused-wait-excluded-from-rate-estimate
// ─────────────────────────────────────────────────────────────────────────────

const RATE_WINDOW: usize = 5;

/// The rate estimator: ms-per-update from wall intervals with the consult
/// pauses EXCLUDED, median over a small window.
struct RateEstimator {
    intervals: std::collections::VecDeque<f64>,
    last_t: Option<f64>,
    last_u: Option<usize>,
    paused_ms: f64,
    median_ms_per_update: Option<f64>,
}

impl RateEstimator {
    fn new() -> Self {
        Self {
            intervals: std::collections::VecDeque::with_capacity(RATE_WINDOW),
            last_t: None,
            last_u: None,
            paused_ms: 0.0,
            median_ms_per_update: None,
        }
    }

    fn paused(&mut self, seconds: f64) {
        self.paused_ms += seconds * 1000.0;
    }

    /// tick at a check step: time per update since the previous check,
    /// without the pauses in between.
    fn tick(&mut self, u: usize, now_ms: f64) {
        if let (Some(t), Some(last_u)) = (self.last_t, self.last_u) {
            if u > last_u {
                let raw = (now_ms - t - self.paused_ms).max(0.0) / (u - last_u) as f64;
                self.intervals.push_back(raw);
                if self.intervals.len() > RATE_WINDOW {
                    self.intervals.pop_front();
                }
                let mut xs: Vec<f64> = self.intervals.iter().cloned().collect();
                xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
                self.median_ms_per_update = Some(xs[xs.len() / 2]);
            }
        }
        self.last_t = Some(now_ms);
        self.last_u = Some(u);
        self.paused_ms = 0.0;
    }
}

#[test]
fn f5_consult_pauses_are_excluded_from_the_rate() {
    // 5 checks, 10 updates each, 20 ms/update of true work — but checks
    // 2 and 3 carry a 4-second agent consult in between. The naive
    // interval (mean, pauses included) reads far past the true rate; the
    // estimator (pauses excluded) recovers ~20 ms/update.
    let mut est = RateEstimator::new();
    let true_ms_per_update = 20.0;
    let mut now = 0.0;
    let mut u = 0usize;
    est.tick(u, now);
    let mut naive_intervals: Vec<f64> = Vec::new();
    let mut last_naive_t = 0.0;
    for check in 1..=5 {
        let consult = check == 2 || check == 3;
        if consult {
            est.paused(4.0); // the harness's own wait
            now += 4000.0; // the wall time the consult took
        }
        u += 10;
        now += true_ms_per_update * 10.0;
        est.tick(u, now);
        naive_intervals.push(now - last_naive_t);
        last_naive_t = now;
    }
    let got = est.median_ms_per_update.expect("the median is measured");
    assert!(
        (got - true_ms_per_update).abs() < 1.0,
        "the pause-excluded median must recover ~{true_ms_per_update} ms/update, got {got}"
    );
    let naive_mean: f64 = naive_intervals.iter().sum::<f64>() / naive_intervals.len() as f64 / 10.0;
    assert!(
        naive_mean > true_ms_per_update * 1.5,
        "the naive mean (pauses included) must overstate the per-update time by >1.5x, got {naive_mean}"
    );
}

#[test]
fn f5_median_window_absorbs_an_evaluation_carrying_interval() {
    // One interval carries an evaluation (2 s of scoring, no pause
    // accounting — the interval is just big); the median over the window
    // must not shrink the estimate toward it.
    let mut est = RateEstimator::new();
    let mut now = 0.0;
    let mut u = 0usize;
    est.tick(u, now);
    for check in 1..=5 {
        u += 10;
        now += 20.0 * 10.0;
        if check == 3 {
            now += 2000.0; // an evaluation inside this interval
        }
        est.tick(u, now);
    }
    let got = est.median_ms_per_update.expect("measured");
    assert!(
        (got - 20.0).abs() < 1.0,
        "the median must hold ~20 despite the evaluation interval, got {got}"
    );
}

#[test]
fn f5_falsifier_bimodal_tail_documented_and_accepted_rewrite_stales_estimate() {
    // The panel falsifiers, carried as the rule's honest regime notes:
    // (a) a bimodal latency distribution hides the slow mode under a
    // median-of-5 — the estimate reads the FAST mode while the tail is
    // real (demonstrated); (b) every accepted rewrite changes the
    // per-update cost, so the estimate is stale for the next window BY
    // CONSTRUCTION — the bounded window (WINDOW=5) is the disclosed
    // mitigation: it re-fits fast, never claims a long-run truth.
    let mut est = RateEstimator::new();
    let mut now = 0.0;
    est.tick(0usize, now);
    let mut u = 0usize;
    for check in 1..=5 {
        u += 10;
        now += if check == 4 {
            200.0 * 10.0
        } else {
            20.0 * 10.0
        };
        est.tick(u, now);
    }
    let got = est.median_ms_per_update.expect("measured");
    assert!(
        (got - 20.0).abs() < 1.0,
        "the median hides the 10x tail (bimodal blindness — the documented falsifier), got {got}"
    );
    // (b): the window is bounded, so a rewrite that halves the per-update
    // cost re-fits within WINDOW checks — assert the bound.
    assert_eq!(
        est.intervals.len(),
        RATE_WINDOW,
        "the window is bounded at WINDOW"
    );
}
