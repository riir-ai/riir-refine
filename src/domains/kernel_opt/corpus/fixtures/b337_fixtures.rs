//! Batch 337 fixtures — the jitengine MLX/Metal distill (Plan 346 /
//! Research 376).
//!
//! Standalone: `rustc --edition 2021 --test b337_fixtures.rs && ./b337_fixtures`
//! Every law's test below was compiled AND executed green BEFORE the corpus
//! entries landed (the B320+ convention). Original syntheses — never source
//! verbatim (the source is Apache-2.0, but house style keeps fixtures
//! original). The rules' ref/opt arms in `entries_gpu_152.rs` embed this
//! executed code; the sec law (F6) mirrors into `sec/corpus.rs`.
//!
//! F1 verify-parity-position-pure-grid — the chunk grid is a pure function
//!     of the row's own position, never of request capacity, so a verify
//!     row equals the M = 1 pass at its position across every grid change.
//! F2 compiled-vs-eager-per-op-exactness-admission — per-op-class admission
//!     with a per-pair check: the MAC class admits (bit-exact on every
//!     pair), the transcendental class refuses (an ulp-scale drift flips a
//!     near-tie argmax).
//! F3 gdn-state-snapshot-per-position-verify-early-exit — per-position
//!     recurrent-state snapshots are the speculation early-exit substrate:
//!     the verify walk stops at the first rejected guess, bit-identical to
//!     the chained single-position calls.
//! F4 command-buffer-declared-input-pacing — the commit watermark fires on
//!     DECLARED input bytes; a call declaring the whole stack commits on
//!     its own, a call declaring its own slice keeps the buffer open.
//!     Outputs are bit-identical either way. (CONSTRUCTIBLE-NOT-RUN: this
//!     fixture models the watermark economics; the measured M3 Metal cell
//!     is the lane-owned probe, the B335 disclosure class.)
//! F5 compute-experts-before-router-lookup-fixup — expert compute is
//!     submitted against a device-side snapshot of the slot map before the
//!     host lookup finishes; all-resident keeps the work, misses rebuild
//!     only the missing rows. Outputs bit-identical either way.
//! F6 conditional-reuseaddr-after-liveness-probe (sec) — bind WITHOUT
//!     SO_REUSEADDR first; on EADDRINUSE, rebind with it ONLY when no live
//!     listener answers the address (a wildcard holder answers the loopback
//!     probe, so the hijack posture refuses).

// ══════════════════════════════════════════════════════════════════════
// F1: verify-parity-position-pure-grid
// ══════════════════════════════════════════════════════════════════════

const SPAN_COARSE: usize = 512;
const SPAN_FINE: usize = 1024;
const HORIZON: usize = 32_768;

/// THE LAW: the span at key position `q` reads ONLY the position — a pure
/// function, no request-shape parameter exists in the signature.
fn span_at(q: usize) -> usize {
    if q + 1 <= HORIZON {
        SPAN_COARSE
    } else {
        SPAN_FINE
    }
}

/// Position-pure boundaries for a row of `q_len + 1` keys. `_capacity` is
/// the request-shape parameter the law FORBIDS reading — it is accepted so
/// both grids share one call shape and the purity is visible at the
/// signature, never read.
fn boundaries_pure(q_len: usize, _capacity: usize) -> Vec<usize> {
    let mut out = vec![0usize];
    let mut q = 0usize;
    loop {
        let s = span_at(q);
        let next = (q + s).min(q_len + 1);
        out.push(next);
        if next > q_len {
            break;
        }
        q = next;
    }
    out
}

/// THE BUG CLASS: the same grid keyed on the request's capacity — the
/// boundaries MOVE when the capacity moves, at the SAME positions.
fn boundaries_keyed(q_len: usize, capacity: usize) -> Vec<usize> {
    let span = if capacity <= 32_768 {
        SPAN_COARSE
    } else {
        SPAN_FINE
    };
    let mut out = vec![0usize];
    let mut q = 0usize;
    loop {
        let next = (q + span).min(q_len + 1);
        out.push(next);
        if next > q_len {
            break;
        }
        q = next;
    }
    out
}

/// Sequential per-chunk partial sums — the row arithmetic a consumer runs
/// per chunk. Same partition + same order => bit-identical sums.
fn chunk_sums(boundaries: &[usize], keys: &[f32]) -> Vec<f32> {
    boundaries
        .windows(2)
        .map(|w| {
            let mut acc = 0.0f32;
            for k in &keys[w[0]..w[1]] {
                acc += *k;
            }
            acc
        })
        .collect()
}

#[test]
fn f1_position_pure_grid_keeps_verify_parity() {
    let q_len = 40_000usize;
    // Seeded key weights (deterministic, non-degenerate).
    let keys: Vec<f32> = (0..=q_len)
        .map(|i| (((i as u64).wrapping_mul(2654435761) % 1000) as f32) * 0.001 + 0.1)
        .collect();

    // (a) THE BUG CLASS IS REAL: the capacity-keyed grid moves boundaries
    //     at the same positions when the request's capacity moves.
    let b_small = boundaries_keyed(q_len, 30_000);
    let b_big = boundaries_keyed(q_len, 40_000);
    assert_ne!(
        b_small, b_big,
        "capacity-keyed grid must move under capacity (the bug class exists)"
    );

    // (b) THE LAW: the position-pure grid is INVARIANT across every request
    //     shape — the same positions, the same boundaries, whatever the
    //     request's capacity says (an M = 1 pass and a verify pass see one
    //     partition).
    let m1_bounds = boundaries_pure(q_len, 30_000);
    let verify_bounds = boundaries_pure(q_len, 40_000);
    assert_eq!(
        m1_bounds, verify_bounds,
        "position-pure grid must be invariant across request capacity"
    );

    // (c) VERIFY == M1, BIT-IDENTICAL: the same partition runs the same
    //     sequential sums, so the verify row's per-chunk arithmetic is the
    //     M = 1 pass's bit for bit — and the keyed grid BREAKS that parity.
    let m1_sums = chunk_sums(&m1_bounds, &keys);
    let verify_sums = chunk_sums(&verify_bounds, &keys);
    assert_eq!(m1_sums.len(), verify_sums.len());
    for (i, (a, b)) in m1_sums.iter().zip(verify_sums.iter()).enumerate() {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "chunk {i}: verify row must equal the M = 1 pass bit-identically"
        );
    }
    let keyed_m1 = chunk_sums(&b_small, &keys);
    let keyed_verify = chunk_sums(&b_big, &keys);
    assert_ne!(
        keyed_m1, keyed_verify,
        "the capacity-keyed grid must break the parity (different partitions, different sums)"
    );
}

// ══════════════════════════════════════════════════════════════════════
// F2: compiled-vs-eager-per-op-exactness-admission
// ══════════════════════════════════════════════════════════════════════

/// bf16 round-to-nearest-even on the f32 bit pattern (the per-pair check's
/// operand class: values exactly representable in bf16).
fn bf16_round(x: f32) -> f32 {
    let bits = x.to_bits();
    let lsb = (bits >> 16) & 1;
    let rounded = ((bits + 0x7fff + lsb) >> 16) << 16;
    f32::from_bits(rounded)
}

/// The MAC class, contracted and strict — on bf16-representable operands
/// both are EXACT (integer-valued products and sums round nowhere).
fn mac_contracted(a: f32, b: f32, c: f32) -> f32 {
    a.mul_add(b, c)
}
fn mac_strict(a: f32, b: f32, c: f32) -> f32 {
    a * b + c
}

/// The transcendental class: strict and a drifted fast form. Near the
/// origin the fast form's drift is ulp-scale — small enough to hide inside
/// one rounding step, exactly the source's compiled-sigmoid class.
fn sigmoid_strict(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}
fn sigmoid_fast(x: f32) -> f32 {
    0.5 * (x / (1.0 + x.abs()) + 1.0)
}

/// Argmax over two candidate scores with the index tie-break: equal bits
/// keep the LOWER index. Returns the winning index.
fn pick(a: f32, b: f32, idx_a: usize, idx_b: usize) -> usize {
    if a.to_bits() > b.to_bits() {
        idx_a
    } else if b.to_bits() > a.to_bits() {
        idx_b
    } else {
        idx_a.min(idx_b) // the tie-break
    }
}

#[test]
fn f2_per_op_class_admission_with_per_pair_check() {
    // (a) THE MAC CLASS ADMITS: over a sweep of bf16-representable
    //     (quarter-integer) triples, contracted == strict on EVERY pair —
    //     the per-pair check finds zero differing pairs.
    let mut differing = 0usize;
    for i in -64i32..=64 {
        for j in -64i32..=64 {
            let a = bf16_round(i as f32 * 0.5);
            let b = bf16_round(j as f32 * 0.5);
            let c = bf16_round((i * 3 + j) as f32 * 0.25);
            if mac_contracted(a, b, c).to_bits() != mac_strict(a, b, c).to_bits() {
                differing += 1;
            }
        }
    }
    assert_eq!(
        differing, 0,
        "the MAC class on bf16-representable operands must be exact on every pair"
    );

    // (b) THE TRANSCENDENTAL CLASS DRIFTS AT ULP SCALE NEAR THE ORIGIN:
    //     two inputs one rounding step apart. strict(6e-8) rounds to the
    //     same bits as strict(0.0) — a TIE — while the fast form's drift
    //     pushes b one ulp ABOVE a. The tie the strict form broke by index
    //     becomes a different winner under the fast form: the raw pick
    //     CHANGED, the exact failure the source recorded at its step 637.
    let a = 0.0f32;
    let b = 6.0e-8f32;
    let sa = sigmoid_strict(a);
    let sb = sigmoid_strict(b);
    let fa = sigmoid_fast(a);
    let fb = sigmoid_fast(b);
    assert_eq!(
        sa.to_bits(),
        sb.to_bits(),
        "strict form must tie the near-tie pair (same bits: {sa:e} vs {sb:e})"
    );
    assert!(
        fb.to_bits() > fa.to_bits(),
        "the fast form's drift must distinguish the pair in b's favor ({fa:e} vs {fb:e})"
    );
    assert_eq!(pick(sa, sb, 0, 1), 0, "strict tie-break keeps index 0");
    assert_eq!(
        pick(fa, fb, 0, 1),
        1,
        "the fast form flips the winner to index 1 — the raw pick changed"
    );

    // (c) THE ADMISSION: keyed per op class, decided by the per-pair check.
    //     MAC: zero differing pairs => admit. Transcendental: a
    //     demonstrated pick flip => refuse, never trusted on a raw path.
    let admit = |differing_pairs: usize, flip_demonstrated: bool| {
        differing_pairs == 0 && !flip_demonstrated
    };
    assert!(admit(0, false), "the MAC class admits");
    assert!(
        !admit(1, true),
        "the transcendental class refuses on the demonstrated flip"
    );
}

// ══════════════════════════════════════════════════════════════════════
// F3: gdn-state-snapshot-per-position-verify-early-exit
// ══════════════════════════════════════════════════════════════════════

const DK: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
struct GdnState([f32; DK]);

fn gdn_step(s: GdnState, k: [f32; DK], v: [f32; DK], decay: f32) -> GdnState {
    let mut out = [0.0f32; DK];
    for i in 0..DK {
        out[i] = s.0[i].mul_add(decay, k[i] * v[i]);
    }
    GdnState(out)
}

/// The snapshot walk: state after EVERY position, each stored as it is
/// produced. Bit-identical to the chained single-position calls.
fn gdn_steps(init: GdnState, ks: &[[f32; DK]], vs: &[[f32; DK]], decay: f32) -> Vec<GdnState> {
    let mut s = init;
    let mut states = Vec::with_capacity(ks.len());
    for t in 0..ks.len() {
        s = gdn_step(s, ks[t], vs[t], decay);
        states.push(s);
    }
    states
}

/// Verify with early exit over a proposed window: accept while the target
/// state agrees with the guess, stop at the FIRST rejected guess. Returns
/// (accepted_prefix_len, positions_evaluated). The per-position snapshots
/// are the substrate — each position's check is an O(1) state compare, and
/// the walk never touches a position past the rejection.
fn verify_early_exit(snapshots: &[GdnState], guesses: &[GdnState]) -> (usize, usize) {
    for t in 0..guesses.len() {
        if snapshots[t] != guesses[t] {
            return (t, t + 1);
        }
    }
    (guesses.len(), guesses.len())
}

/// Deterministic seeded inputs.
fn gdn_inputs(n: usize, seed: u64) -> (Vec<[f32; DK]>, Vec<[f32; DK]>, f32) {
    let gen = |i: u64| -> [f32; DK] {
        let mut s = seed ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let a = ((s >> 40) as f32 / (1u32 << 24) as f32 - 0.5) * 2.0;
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let b = ((s >> 40) as f32 / (1u32 << 24) as f32 - 0.5) * 2.0;
        [a, b, a * 0.5, b * 0.5]
    };
    let ks: Vec<[f32; DK]> = (0..n as u64).map(gen).collect();
    let vs: Vec<[f32; DK]> = ((n as u64)..(2 * n as u64)).map(gen).collect();
    (ks, vs, 0.9)
}

#[test]
fn f3_snapshots_bit_identical_and_verify_exits_early() {
    let n = 64usize;
    let (ks, vs, decay) = gdn_inputs(n, 0xB337_F3);
    let init = GdnState([1.0, 0.0, 0.5, -0.5]);

    // (a) BIT-IDENTITY: the snapshot walk equals the chained
    //     single-position calls, bit for bit.
    let snapshots = gdn_steps(init, &ks, &vs, decay);
    let mut s = init;
    for t in 0..n {
        s = gdn_step(s, ks[t], vs[t], decay);
        assert_eq!(
            s, snapshots[t],
            "position {t}: the stored state must equal the chained call's bits"
        );
    }

    // (b) FULL ACCEPTANCE: a window whose guesses ARE the target states
    //     verifies at full width — every position evaluated, all accepted.
    let (accepted, evaluated) = verify_early_exit(&snapshots, &snapshots);
    assert_eq!(accepted, n);
    assert_eq!(evaluated, n);

    // (c) EARLY EXIT: corrupt the guess at position g — the verify stops
    //     at the FIRST rejected guess and evaluates g+1 positions, never
    //     the whole window. The snapshots make the stop free: positions
    //     past the rejection are never touched.
    let g = 5usize;
    let mut guesses = snapshots.clone();
    guesses[g].0[0] += 0.125; // the rejected guess
    let (accepted, evaluated) = verify_early_exit(&snapshots, &guesses);
    assert_eq!(accepted, g, "the prefix up to the first rejection accepts");
    assert_eq!(
        evaluated,
        g + 1,
        "the walk stops AT the first rejected guess: g+1 positions, not {n}"
    );
    assert!(evaluated < n, "early exit must beat the full window");
}

// ══════════════════════════════════════════════════════════════════════
// F4: command-buffer-declared-input-pacing
// ══════════════════════════════════════════════════════════════════════

const WATERMARK_BYTES: usize = 40;
const EXPERTS: usize = 8;
const SLICE_BYTES: usize = 6;
const STACK_BYTES: usize = SLICE_BYTES * EXPERTS; // 48 — the whole stack crosses 40 on its own

/// The commit model: a buffer holds encoded work until one more call's
/// DECLARED input bytes would cross the watermark — then it commits and a
/// fresh buffer opens.
#[derive(Clone, Copy)]
struct PacingEncoder {
    declared: usize,
    commits: usize,
}

impl PacingEncoder {
    fn new() -> Self {
        Self {
            declared: 0,
            commits: 0,
        }
    }
    fn declare(&mut self, bytes: usize) {
        if self.declared + bytes > WATERMARK_BYTES {
            self.commits += 1; // the buffer commits; a fresh one opens
            self.declared = 0;
        }
        self.declared += bytes;
    }
}

/// The expert call's arithmetic reads its OWN weights, never the declared
/// envelope — so the outputs cannot depend on the declaration shape.
fn expert_out(e: usize) -> f32 {
    1.0f32.mul_add((e as f32) * 0.125, 0.5)
}

#[test]
fn f4_declared_inputs_pace_commits_outputs_identical() {
    // THE BUG CLASS: every call declares the WHOLE stack — 48 bytes over
    // the 40-byte watermark on its own — so every call commits.
    let mut whole = PacingEncoder::new();
    let mut outs_whole = Vec::new();
    for e in 0..EXPERTS {
        whole.declare(STACK_BYTES);
        outs_whole.push(expert_out(e));
    }
    assert_eq!(
        whole.commits, EXPERTS,
        "whole-stack declaration must commit on every call"
    );

    // THE LAW: each call declares only its own slice — the buffer holds a
    // group of calls per commit (6-byte slices: six calls fit under 40,
    // the seventh crosses: one commit for the batch of eight).
    let mut sliced = PacingEncoder::new();
    let mut outs_sliced = Vec::new();
    for e in 0..EXPERTS {
        sliced.declare(SLICE_BYTES);
        outs_sliced.push(expert_out(e));
    }
    assert!(
        sliced.commits < whole.commits,
        "sliced declaration must commit less ({} vs {})",
        sliced.commits,
        whole.commits
    );
    assert_eq!(
        sliced.commits, 1,
        "six slices fit; the seventh crosses: one commit"
    );

    // BIT-IDENTICAL OUTPUTS: the declaration shape changes the commit
    // cadence, never the arithmetic — every output is the same bits.
    for (w, s) in outs_whole.iter().zip(outs_sliced.iter()) {
        assert_eq!(
            w.to_bits(),
            s.to_bits(),
            "declared inputs must not change the outputs"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════
// F5: compute-experts-before-router-lookup-fixup
// ══════════════════════════════════════════════════════════════════════

const LAYER_EXPERTS: usize = 12;

fn expert_row(e: usize, x: f32) -> f32 {
    x * ((e as f32) + 1.0) * 0.5
}

/// The device-side snapshot the optimistic submission reads, and the real
/// map the host lookup resolves.
#[derive(Clone)]
struct SlotMap {
    resident: [bool; LAYER_EXPERTS],
}

/// Optimistic compute over the SNAPSHOT: one output row per
/// snapshot-resident expert, submitted before the host lookup finishes.
fn optimistic_rows(snapshot: &SlotMap, x: f32) -> [Option<f32>; LAYER_EXPERTS] {
    let mut rows = [None; LAYER_EXPERTS];
    for (e, slot) in rows.iter_mut().enumerate() {
        if snapshot.resident[e] {
            *slot = Some(expert_row(e, x));
        }
    }
    rows
}

/// The reference: compute every needed row AFTER the lookup, over the REAL
/// residency — the serial posture the optimistic race must match.
fn serial_rows(real: &SlotMap, x: f32) -> [Option<f32>; LAYER_EXPERTS] {
    optimistic_rows(real, x)
}

/// Reconcile: keep the optimistic rows where the snapshot agreed with the
/// real map; rebuild ONLY the rows the snapshot missed; discard rows the
/// snapshot computed for experts the real map says are absent. Returns
/// (rows, rebuilds, discarded).
fn reconcile(
    optimistic: [Option<f32>; LAYER_EXPERTS],
    real: &SlotMap,
    x: f32,
) -> ([Option<f32>; LAYER_EXPERTS], usize, usize) {
    let mut rebuilds = 0usize;
    let mut discarded = 0usize;
    let mut rows = [None; LAYER_EXPERTS];
    for e in 0..LAYER_EXPERTS {
        if real.resident[e] {
            rows[e] = match optimistic[e] {
                Some(v) => Some(v), // the optimistic work is KEPT
                None => {
                    rebuilds += 1; // the MISS-ONLY rebuild: this row, not the layer
                    Some(expert_row(e, x))
                }
            };
        } else if optimistic[e].is_some() {
            discarded += 1; // the snapshot lied: the row is dropped, never rebuilt
        }
    }
    (rows, rebuilds, discarded)
}

#[test]
fn f5_compute_ahead_keeps_work_rebuilds_misses_only() {
    let x = 1.0f32;

    // Arm 1 — ALL RESIDENT: the snapshot agrees with a fully resident map,
    // the optimistic work is kept wholesale, zero rebuilds, and the rows
    // are bit-identical to the serial posture.
    let real_all = SlotMap {
        resident: [true; LAYER_EXPERTS],
    };
    let snap_all = real_all.clone();
    let opt = optimistic_rows(&snap_all, x);
    let (rows, rebuilds, discarded) = reconcile(opt, &real_all, x);
    assert_eq!(rebuilds, 0, "all-resident: nothing to rebuild");
    assert_eq!(discarded, 0);
    assert_eq!(
        rows,
        serial_rows(&real_all, x),
        "kept work is bit-identical"
    );

    // Arm 2 — PARTIAL RESIDENCE: the snapshot misses three experts; the
    // miss-only rebuild touches exactly those three rows, never the whole
    // layer; one snapshot lie is discarded; outputs stay bit-identical to
    // the serial posture.
    let mut real = SlotMap {
        resident: [true; LAYER_EXPERTS],
    };
    real.resident[2] = false;
    real.resident[7] = false;
    real.resident[11] = false;
    let mut snap = real.clone();
    snap.resident[2] = true; // the lookup has not caught up: the snapshot lies
    snap.resident[4] = false; // the snapshot also misses a resident expert
    snap.resident[9] = false;
    let opt = optimistic_rows(&snap, x);
    let (rows, rebuilds, discarded) = reconcile(opt, &real, x);
    assert_eq!(
        rebuilds, 2,
        "miss-only rebuild: exactly the rows the snapshot missed (4 and 9)"
    );
    assert_eq!(
        discarded, 1,
        "the snapshot lie at expert 2 (computed, real-absent) is discarded"
    );
    assert!(
        rebuilds < LAYER_EXPERTS,
        "never the whole layer: {rebuilds} rows rebuilt of {LAYER_EXPERTS}"
    );
    assert_eq!(
        rows,
        serial_rows(&real, x),
        "bit-identical to the serial posture"
    );
}

// ══════════════════════════════════════════════════════════════════════
// F6: conditional-reuseaddr-after-liveness-probe (sec)
// ══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Debug)]
enum Host {
    Loopback,
    Wildcard,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Holder {
    Live,
    TimeWait,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct Sock {
    host: Host,
    port: u16,
}

#[derive(Default)]
struct BindTable {
    holders: Vec<(Sock, Holder)>,
}

#[derive(Debug)]
enum BindErr {
    InUse,
}

enum Probe {
    /// A live listener ANSWERS the address (a wildcard holder answers a
    /// loopback probe).
    Answered,
    /// Nothing answers — any holders are TIME_WAIT ends of a closed server.
    Refused,
}

impl BindTable {
    /// The bind. With reuse OFF, ANY holder at the same host:port refuses —
    /// Live or TimeWait — and a Live WILDCARD holder refuses a loopback
    /// bind too (that is the refusal the rule exists to keep). With reuse
    /// ON, a Live holder at the SAME host still refuses; a TimeWait holder
    /// is overridden (the quick-restart path); and the loopback-with-reuse
    /// bind beside a live wildcard is ALLOWED — the shadow hazard.
    fn bind(&mut self, sock: Sock, reuse: bool) -> Result<(), BindErr> {
        let blocker = self.holders.iter().any(|(h, state)| {
            if h.port != sock.port {
                return false;
            }
            match (*state, h.host, sock.host) {
                (Holder::TimeWait, _, _) => !reuse, // TimeWait: refuses only a strict bind
                (Holder::Live, Host::Loopback, Host::Loopback) => true,
                (Holder::Live, Host::Wildcard, Host::Wildcard) => true,
                // the hijack class: with reuse ON the loopback bind sits
                // beside the live wildcard; without reuse it is refused
                (Holder::Live, Host::Wildcard, Host::Loopback) => !reuse,
                (Holder::Live, Host::Loopback, Host::Wildcard) => true,
            }
        });
        if blocker {
            return Err(BindErr::InUse);
        }
        self.holders.retain(|(h, _)| !(h.port == sock.port));
        self.holders.push((sock, Holder::Live));
        Ok(())
    }

    /// The liveness probe: dial the address. A live wildcard holder
    /// answers a loopback probe; only TimeWait holders stay silent.
    fn probe(&self, sock: Sock) -> Probe {
        let answered = self.holders.iter().any(|(h, state)| {
            h.port == sock.port
                && *state == Holder::Live
                && match (h.host, sock.host) {
                    (Host::Loopback, Host::Loopback) => true,
                    (Host::Wildcard, _) => true,
                    (Host::Loopback, Host::Wildcard) => false,
                }
        });
        if answered {
            Probe::Answered
        } else {
            Probe::Refused
        }
    }

    /// Close: every live holder at the port becomes a TimeWait end.
    fn close(&mut self, port: u16) {
        for (h, state) in self.holders.iter_mut() {
            if h.port == port {
                *state = Holder::TimeWait;
            }
        }
    }
}

/// THE POLICY: bind WITHOUT reuse first. On EADDRINUSE, probe: rebind WITH
/// reuse only when NO live listener answers (the holders are TimeWait);
/// refuse loudly when one does.
fn serve(table: &mut BindTable, sock: Sock) -> Result<&'static str, &'static str> {
    match table.bind(sock, false) {
        Ok(()) => Ok("bound-strict"),
        Err(BindErr::InUse) => match table.probe(sock) {
            Probe::Refused => {
                table
                    .bind(sock, true)
                    .expect("TimeWait-only rebind must succeed");
                Ok("rebound-over-timewait")
            }
            Probe::Answered => Err("refused-live-listener"),
        },
    }
}

#[test]
fn f6_reuseaddr_only_after_the_probe_refuses_live_holders() {
    let lb = |port| Sock {
        host: Host::Loopback,
        port,
    };

    // (a) THE CLEAN PATH: a free port binds strict, no reuse anywhere.
    let mut t = BindTable::default();
    assert_eq!(serve(&mut t, lb(8080)), Ok("bound-strict"));
    assert_eq!(t.holders.len(), 1, "one live holder stands");

    // (b) BESIDE A LIVE LOOPBACK SERVER: the strict bind refuses, the probe
    //     is ANSWERED, the policy refuses — the port keeps its owner.
    let mut t = BindTable::default();
    t.bind(lb(8081), false).expect("the first server binds");
    assert_eq!(serve(&mut t, lb(8081)), Err("refused-live-listener"));

    // (c) BESIDE A LIVE WILDCARD SERVER: the holder also refuses the
    //     loopback strict bind, and the wildcard ANSWERS the loopback
    //     probe — refused, "also when the other server listens at
    //     0.0.0.0".
    let mut t = BindTable::default();
    t.bind(
        Sock {
            host: Host::Wildcard,
            port: 8082,
        },
        false,
    )
    .expect("the wildcard server binds");
    assert_eq!(serve(&mut t, lb(8082)), Err("refused-live-listener"));

    // (d) THE ANTI-REGIME: an UNCONDITIONAL reuse bind would SUCCEED beside
    //     that live wildcard — the shadow hazard the policy exists to
    //     refuse. The model demonstrates the hazard is real.
    let mut t = BindTable::default();
    t.bind(
        Sock {
            host: Host::Wildcard,
            port: 8083,
        },
        false,
    )
    .expect("the wildcard server binds");
    assert!(
        t.bind(lb(8083), true).is_ok(),
        "unconditional reuse must demonstrate the shadow hazard"
    );

    // (e) THE QUICK RESTART: the previous server closed (TimeWait ends
    //     only). The strict bind refuses, the probe is REFUSED (nothing
    //     answers), the rebind WITH reuse succeeds — the restart works
    //     without waiting the TimeWait out.
    let mut t = BindTable::default();
    t.bind(lb(8084), false).expect("the previous server binds");
    t.close(8084);
    assert_eq!(serve(&mut t, lb(8084)), Ok("rebound-over-timewait"));
}
