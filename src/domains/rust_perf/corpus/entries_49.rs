//! Entries slice - Batch 283 (2026-10-25): one `rust_perf` rule from the
//!  llama.cpp fork's #393 host-time wave (commit `63f16c408f`,
//! "ggml-backend, cuda : cut ~55-65 us of host time per decode step
//! (batched sched input uploads, no node scans on CUDA graph replay;
//! byte-identical)", MIT lineage, read in FULL from the vendor checkout
//! pinned AT the commit): `revalidation-skipped-by-certifying-identity-token`
//! (caching) - the CUDA-side half of that commit, where a per-step
//! compatibility scan over an unchanged captured graph is skipped on a
//! two-sided identity-token check and a derived consumer map is rebuilt
//! only at the update boundary the token marks. The commit's OTHER half
//! (ggml-backend.cpp, the batched sched input uploads) is a copy-batching
//! face - async copies + one wait instead of one blocking copy each - NOT
//! a validation skip; it is disclosed in `context_notes` and not mined.
//!
//! Every quoted figure is SOURCE-reported (the commit message's H200
//! decode/server numbers and the 55-65 us per-decode-step host cut -
//! provenance, never bars) or OUR-BOX-reported (riir-ai Bench 1000, the
//! 2026-10-25 league bout that named #393 as a cause of the opponent's
//! +2.11% decode move - the host-time class is a league-mover at our
//! geometry, which is why the rule exists). Every quote is verbatim at
//! the pin.
//!
//! Fixtures are ORIGINAL machinery (never the ggml source pasted - the
//! capture-and-replay host is re-authored as a plain-Rust stand-in with
//! neutral names), each compiled and executed standalone (rustc
//! --edition 2021 --test) before this table was assembled; the
//! buggy/fixed arm pair shares the surrounding harness and differs only
//! on the law - the buggy arm counts the per-step scans and demonstrates
//! the 0 == 0 absence trap, the fixed arm implements the two-sided
//! token, the success-path write, the fallback re-issue and the
//! boundary-keyed map.
use crate::domains::rust_perf::PerfRule;

/// Data table, not logic - grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── caching: a per-step re-validation scan on an unchanged input
        // is skippable on a certifying identity token -------------------
        // Batch 283 / PrismML llama.cpp fork @ 63f16c408f #393 (the
        // CUDA-side half: the admission token + the boundary-keyed
        // consumer map; the batched-upload half is NOT this law).
        PerfRule {
            rule_id: "revalidation-skipped-by-certifying-identity-token".to_string(),
            name: "a per-step re-validation scan over an input the previous step already validated is skippable when a certifying identity token exists and matches - the token must be written ONLY by the validation's own success path so token equality certifies 'the last scan passed AND on this exact input'; absence of the token (zero/None) must never count as a hit - a bare equality would let 0 == 0 admit a never-validated input, so the full scan runs; a token mismatch falls back to the scan and re-issues the token only after it passes; and the token must be minted by the input's BUILDER at every rebuild (a monotonic identity, never reused) so a rebuilt or edited input cannot keep an old token - with a derived map keyed to the same content rebuilt only at the update boundary the token marks, never on pure replay steps that never read it"
                .to_string(),
            category: "caching".to_string(),
            buggy_code: r#"#![allow(dead_code)]
// Fixture for `revalidation-skipped-by-certifying-identity-token`
// (riir-refine rust_perf corpus, Batch 283) -- the ANTI-PATTERN arm.
//
// ORIGINAL machinery standing in for the source's host-side replay path
// (a captured device graph replayed every decode step; the commit is
// cited in context_notes): a runtime captures a program once, then
// replays it each step. The incumbent re-runs the FULL node
// compatibility scan on EVERY step -- including the replay steps of a
// graph the scan already passed and that has not moved -- and rebuilds
// a derived consumer map every step although replays never read it.
// Also shown: the token shortcut written WITHOUT the absence guard,
// where 0 == 0 admits a graph no scan ever validated.

use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Debug)]
struct Node {
    kind: u8,
    shape: [usize; 2],
}

/// a caller-built graph. `stamp` is minted by the scheduler at every
/// (re)build of the graph, so a rebuilt graph carries a different
/// stamp; 0 means the builder never minted one.
#[derive(Clone)]
struct Graph {
    stamp: u64,
    nodes: Vec<Node>,
}

/// the captured program: the node properties the scan validates
/// against, plus the identity token of the graph they were recorded on.
#[derive(Clone)]
struct Captured {
    cert: u64,
    props: Vec<Node>,
}

struct Host {
    captured: Option<Captured>,
    scans: usize,      // compatibility scans performed
    map_builds: usize, // derived consumer-map builds
    outputs: Vec<u64>,
}

impl Host {
    fn new() -> Self {
        Host { captured: None, scans: 0, map_builds: 0, outputs: Vec::new() }
    }

    /// the expensive part: walk EVERY node and confirm kind + shape
    /// still match the captured program. O(nodes) host work per call.
    fn compatibility_scan(&mut self, g: &Graph) -> bool {
        self.scans += 1;
        match &self.captured {
            None => false,
            Some(cap) => &cap.props == &g.nodes,
        }
    }

    fn capture(&mut self, g: &Graph) {
        self.captured = Some(Captured { cert: g.stamp, props: g.nodes.clone() });
    }

    /// one step, incumbent shape: capture once, then re-run the scan
    /// and rebuild the derived map on EVERY step -- replay steps
    /// included, though a replay never reads the map.
    fn step(&mut self, g: &Graph, replaying: bool) -> bool {
        if self.captured.is_none() {
            self.capture(g);
        }
        if !self.compatibility_scan(g) {
            return false;
        }
        // the derived consumer map, built from the node list: rebuilt
        // unconditionally; nothing on a replay step ever reads it
        let _map: HashMap<usize, usize> = g
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n.kind as usize))
            .collect();
        self.map_builds += 1;
        self.outputs.push(evaluate(g));
        let _ = replaying; // the incumbent never consults it
        true
    }
}

/// a deterministic stand-in for the captured computation: a replay step
/// must produce exactly what an eager evaluation of the same nodes
/// produces.
fn evaluate(g: &Graph) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
    for n in &g.nodes {
        h ^= (n.kind as u64) << 8 | ((n.shape[0] as u64) << 3) | n.shape[1] as u64;
        h = h.rotate_left(7).wrapping_mul(0x2545_F491_4F6C_DD1D);
    }
    h
}

/// the token shortcut written WITHOUT the absence guard: bare equality.
/// A graph the builder never stamped carries 0 and a fresh capture slot
/// starts at 0 -- the pair compares equal and the scan is skipped for a
/// graph NO scan ever passed.
fn unguarded_token_skip(g: &Graph, stored_cert: u64) -> bool {
    g.stamp == stored_cert
}

#[test]
fn the_scan_reruns_on_every_replay_step_of_an_unchanged_graph() {
    let g = Graph {
        stamp: 7, // minted by the scheduler at this graph's build
        nodes: vec![
            Node { kind: 1, shape: [4, 64] },
            Node { kind: 2, shape: [64, 64] },
            Node { kind: 3, shape: [64, 8] },
        ],
    };
    let mut host = Host::new();
    // step 0 captures; steps 1..=99 replay the SAME unchanged graph
    for step in 0..100u32 {
        assert!(host.step(&g, step > 0));
    }
    // the waste: 100 full node scans for ONE graph that never moved --
    // the scan step 0 ran already passed on this exact graph, and its
    // verdict transfers to every later step for free
    assert_eq!(host.scans, 100, "the compatibility scan re-ran per step");
    assert_eq!(host.map_builds, 100, "the derived map rebuilt per replay step");
    // ...while the replayed answers never changed
    assert!(host.outputs.windows(2).all(|w| w[0] == w[1]));
}

#[test]
fn the_unguarded_token_pair_admits_a_never_validated_graph() {
    // a graph that FAILS compatibility against the captured program
    // (its second node changed shape) but carries stamp 0: it never
    // went through the scheduler, so no token was ever minted for it.
    let rogue = Graph {
        stamp: 0,
        nodes: vec![
            Node { kind: 1, shape: [4, 64] },
            Node { kind: 2, shape: [999, 64] }, // incompatible
        ],
    };
    // equality alone: 0 == 0 reads as a "hit" and the scan is skipped
    // for a graph no scan ever validated. The `!= 0` absence guard is
    // what kills exactly this admission in the fixed arm.
    assert!(unguarded_token_skip(&rogue, 0));
}

fn main() {}
"#
            .to_string(),
            fixed_code: r#"#![allow(dead_code)]
// Fixture for `revalidation-skipped-by-certifying-identity-token`
// (riir-refine rust_perf corpus, Batch 283) -- the LAW arm.
//
// The SAME host with the certifying identity token: the compatibility
// scan runs only when the token cannot vouch for the graph -- first
// capture, a scheduler rebuild (the stamp moved), or no stamp at all
// (absence is NEVER a hit: a bare `stamp == cert` compare would let
// 0 == 0 admit a graph no scan ever validated). The token is written
// ONLY inside the scan's own success path, so a stored token certifies
// "the last scan passed AND on this exact graph" -- a rejected graph
// never stores one. The derived consumer map rebuilds only at the
// update boundary the token marks, never on a pure replay step.

use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Debug)]
struct Node {
    kind: u8,
    shape: [usize; 2],
}

/// a caller-built graph. `stamp` is minted by the scheduler at every
/// (re)build -- monotonic, never reused -- so a rebuilt graph carries a
/// different stamp and an edited graph CANNOT keep its old one; 0 means
/// the builder never minted one.
#[derive(Clone)]
struct Graph {
    stamp: u64,
    nodes: Vec<Node>,
}

#[derive(Clone)]
struct Captured {
    cert: u64,
    props: Vec<Node>,
}

struct Host {
    captured: Option<Captured>,
    scans: usize,
    map_builds: usize,
    outputs: Vec<u64>,
}

impl Host {
    fn new() -> Self {
        Host { captured: None, scans: 0, map_builds: 0, outputs: Vec::new() }
    }

    /// the expensive part: walk EVERY node. O(nodes) host work per call
    /// -- the thing replay steps must not pay twice. (The source walks
    /// the nodes twice -- a compatibility scan, then a per-node props
    /// compare -- this fixture folds both into one scan; the token
    /// governs both.)
    fn compatibility_scan(&mut self, g: &Graph) -> bool {
        self.scans += 1;
        match &self.captured {
            None => false,
            Some(cap) => &cap.props == &g.nodes,
        }
    }

    /// the admission: the token short-circuits first; the full scan is
    /// the FALLBACK and re-issues the token only through its own success
    /// path. TWO-SIDED GUARD: `stamp != 0` (absence is never a hit) and
    /// `stamp == cert` (the token names THIS capture).
    fn graph_compatible(&mut self, g: &Graph) -> bool {
        if let Some(cap) = &self.captured {
            if g.stamp != 0 && g.stamp == cap.cert {
                return true; // the scan that issued this token already
                             // passed on this exact graph: skip the walk
            }
        }
        let ok = self.compatibility_scan(g);
        if ok {
            let cap = self.captured.as_mut().expect("scan passed on a live capture");
            cap.cert = g.stamp; // THE WRITE: only the success path stores
        }
        ok
    }

    /// the second half of the validation: must the captured program be
    /// re-recorded? The token answers it for free -- a hit means the
    /// props matched when the token was issued, so nothing needs
    /// re-recording; the scan path conservatively says yes.
    fn update_required(&self, g: &Graph) -> bool {
        match &self.captured {
            Some(cap) if g.stamp != 0 && g.stamp == cap.cert => {
                debug_assert_eq!(cap.props.len(), g.nodes.len());
                false
            }
            _ => true,
        }
    }

    /// one step. `replaying` = this step is served from the captured
    /// program without host re-evaluation; on that path a token hit has
    /// already certified "same content", so the derived consumer map --
    /// which only the evaluate/capture path ever reads -- must NOT
    /// rebuild.
    fn step(&mut self, g: &Graph, replaying: bool) -> bool {
        if self.captured.is_none() {
            // a fresh capture slot starts at cert 0: the two-sided
            // guard cannot hit on it, so the first step ALWAYS scans
            // and the token is issued by that scan's success
            self.captured = Some(Captured { cert: 0, props: g.nodes.clone() });
        }
        if !self.graph_compatible(g) {
            return false;
        }
        // the update boundary, the map's only rebuild sites: first
        // capture, a re-validation, or a non-replay step -- never a
        // pure replay
        if !replaying || self.update_required(g) {
            let _map: HashMap<usize, usize> = g
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| (i, n.kind as usize))
                .collect();
            self.map_builds += 1;
        }
        self.outputs.push(evaluate(g));
        true
    }
}

/// identical to the buggy arm's: a replay must be byte-identical to an
/// eager evaluation.
fn evaluate(g: &Graph) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
    for n in &g.nodes {
        h ^= (n.kind as u64) << 8 | ((n.shape[0] as u64) << 3) | n.shape[1] as u64;
        h = h.rotate_left(7).wrapping_mul(0x2545_F491_4F6C_DD1D);
    }
    h
}

#[test]
fn a_token_hit_skips_the_scan_and_the_map_on_replay_steps() {
    let g = Graph {
        stamp: 7,
        nodes: vec![
            Node { kind: 1, shape: [4, 64] },
            Node { kind: 2, shape: [64, 64] },
            Node { kind: 3, shape: [64, 8] },
        ],
    };
    let mut host = Host::new();
    for step in 0..100u32 {
        assert!(host.step(&g, step > 0));
    }
    // ONE scan (the first capture; the token vouches for the 99
    // replays) and ONE map build (at the capture boundary; replays
    // never read it)
    assert_eq!(host.scans, 1, "the token skipped the scan on every replay step");
    assert_eq!(host.map_builds, 1, "the derived map stayed at its capture boundary");
    assert!(host.outputs.windows(2).all(|w| w[0] == w[1]), "replays are identical");
}

#[test]
fn token_absence_is_never_a_hit() {
    let g = Graph {
        stamp: 0, // the builder never minted a stamp
        nodes: vec![Node { kind: 1, shape: [4, 64] }, Node { kind: 2, shape: [64, 64] }],
    };
    let mut host = Host::new();
    // case 1: a live capture holding a REAL token (cert 9) -- a stamp-0
    // caller graph can never ride it: the guard forces the scan
    host.captured = Some(Captured { cert: 9, props: g.nodes.clone() });
    assert!(host.graph_compatible(&g));
    assert_eq!(host.scans, 1, "a stamp-0 graph cannot ride a stored token");
    // the source stores the incoming stamp after the scan even when it
    // is 0 -- a stored 0 reads as "no certificate" and can never hit
    // (the guard), so the overwrite is safe, merely fast-path-free
    assert_eq!(host.captured.as_ref().unwrap().cert, 0);
    // case 2: the 0 == 0 trap itself -- fresh slot, same content: the
    // unguarded pair would skip; the scan runs
    host.captured = Some(Captured { cert: 0, props: g.nodes.clone() });
    assert!(host.graph_compatible(&g));
    assert_eq!(host.scans, 2, "0 == 0 must not read as a hit: the scan ran");
    // case 3: a stamp-0 graph whose CONTENT does not match is refused,
    // and a refused graph issues no token of its own
    let rogue = Graph {
        stamp: 0,
        nodes: vec![Node { kind: 1, shape: [4, 64] }, Node { kind: 2, shape: [999, 64] }],
    };
    assert!(!host.graph_compatible(&rogue));
    assert_eq!(host.scans, 3);
    assert_eq!(host.captured.as_ref().unwrap().cert, 0, "a rejected graph never issues a token");
}

#[test]
fn a_changed_stamp_rescans_and_reissues_the_token() {
    let nodes = vec![
        Node { kind: 1, shape: [4, 64] },
        Node { kind: 2, shape: [64, 64] },
        Node { kind: 3, shape: [64, 8] },
    ];
    let mut host = Host::new();
    let first = Graph { stamp: 7, nodes: nodes.clone() };
    for step in 0..10u32 {
        assert!(host.step(&first, step > 0));
    }
    assert_eq!(host.scans, 1);
    assert_eq!(host.map_builds, 1);
    // the scheduler REBUILDS the graph: same content, a new monotonic
    // stamp -- the old token no longer names it
    let rebuilt = Graph { stamp: 8, nodes: nodes.clone() };
    assert!(host.step(&rebuilt, false));
    assert_eq!(host.scans, 2, "the moved token fell back to the full scan");
    assert_eq!(
        host.captured.as_ref().unwrap().cert,
        8,
        "the scan's success path re-issued the token for the new stamp"
    );
    assert_eq!(host.map_builds, 2, "the re-validation step is an update boundary");
    // and the replays are cheap again
    for _ in 0..10u32 {
        assert!(host.step(&rebuilt, true));
    }
    assert_eq!(host.scans, 2, "the fresh token vouches for the replays");
}

#[test]
fn the_guard_kills_what_the_unguarded_pair_admits() {
    // the discriminator: the rogue from the buggy arm, against a live
    // host holding a capture whose cert is 0
    let rogue = Graph {
        stamp: 0,
        nodes: vec![
            Node { kind: 1, shape: [4, 64] },
            Node { kind: 2, shape: [999, 64] }, // incompatible
        ],
    };
    let mut host = Host::new();
    host.captured = Some(Captured {
        cert: 0,
        props: vec![Node { kind: 1, shape: [4, 64] }, Node { kind: 2, shape: [64, 64] }],
    });
    // unguarded: 0 == 0 -> "hit" -> the incompatible graph is admitted
    assert!(rogue.stamp == host.captured.as_ref().unwrap().cert);
    // guarded: the scan runs, refuses, and stores nothing
    assert!(!host.graph_compatible(&rogue));
    assert_eq!(host.scans, 1, "the scan ran; it refused");
    assert_eq!(host.captured.as_ref().unwrap().cert, 0, "nothing stored for the refusal");
}

fn main() {}
"#
            .to_string(),
            context_notes: Some("Grade B - NotBenchable here (the source figures are CUDA host-path numbers on their boxes - provenance, never bars; no device-graph validator exists in this corpus's served paths; the decision content is the TWO-SIDED-GUARD CERTIFYING-TOKEN law and it is CPU-testable, which IS the fixtures). THE LAW: a per-step re-validation pass over an UNCHANGED input is skippable when a certifying identity token exists and matches - the token written ONLY by the validation's own success path, so token equality certifies 'the last scan passed AND the input is the same one it passed on'. FOUR load-bearing halves, all present in the source: (1) THE WRITE LIVES IN THE SUCCESS PATH - the token's store site is one helper whose ONLY caller sits behind the compatibility admission, so a stored token certifies a scan that actually passed and a rejected input never stores one; (2) THE ABSENCE GUARD - zero means 'no certificate has ever been issued' (the graph's uid starts at 0 and the counter that mints real ones starts at 1) and absence is NEVER a hit: the admission, verbatim: '(cgraph->uid != 0 && cgraph->uid == graph->uid)' - without the `!= 0` half, a never-stamped input against a fresh capture slot reads 0 == 0 and skips validating an input NO scan ever passed, which is the trap the buggy fixture demonstrates; (3) THE TOKEN IS MINTED BY THE BUILDER, NOT THE VALIDATOR - the scheduler assigns a fresh monotonic uid at every graph (re)build, so a rebuilt or edited graph CANNOT keep its old token: token equality implies the same graph instance, which is why the scan's verdict transfers; (4) THE SCAN STAYS THE AUTHORITY - the token is a short-circuit hint with a sound fallback: a mismatch falls back to the full scan and re-issues the token only after it passes. THE SOURCE (PrismML llama.cpp fork, commit 63f16c408f #393, MIT lineage; the CUDA-side half, read in full at the vendor pin): the incumbent ran the compatibility scan on EVERY graph-compute call - commit message, verbatim: 'On the CUDA side, host only: skip the graph compatibility scan on replay when the graph uid is unchanged, and build the shared-Q8 consumer map only while nodes are evaluated or captured.' The fix's own in-code comment, verbatim: 'graph->uid is only set for a compatible graph (ggml_cuda_graph_update_required), and an unchanged uid means the same nodes: skip the scan of the nodes' - the named helper is where the stored token is issued (its first act after its own token check fails is to store the incoming uid, then walk the nodes; this commit only wired the token into the ADMISSION - the helper and its store predate it). THE SECONDARY FACE (same host-work-per-replay-step theme, same commit): the derived consumer map (gb10_shared_q8_consumer_counts, a per-node scan product keyed to graph CONTENT) is rebuilt only at the update boundary - gated 'if (!use_cuda_graph || cuda_graph_update_required)' - with the in-code comment, verbatim: 'only read while the nodes are evaluated or captured: not built when a captured CUDA graph is replayed' - a derived map keyed to content is rebuilt when the content marker says the content moved, never per replay step; the fixture folds 'update required' into the token (a hit IS the no-update verdict; a scan-path step rebuilds conservatively - it can only rebuild an identical map, never skip a needed one). MEASURED (source-reported, the commit message - provenance, never bars): 'Cut about 55-65 us of host time per decode step (about 130 us on 27B)'; 'Logits and server responses are byte-identical. On H200: decode +2-3.5% (2B-27B), server +2-4.4%, 9B MTP K=2 +2.9%'. HONESTY ON THE NUMBER: the 55-65 us headline covers the WHOLE commit - its OTHER half (ggml-backend.cpp, the batched sched input uploads: async copies + one wait instead of one blocking copy each, falling back for meta backends, backends without async upload, non-contiguous inputs and the events path) is a COPY-BATCHING face, not a validation skip, and is NOT this rule. OUR-BOX (why the rule exists at this batch): riir-ai Bench 1000 (2026-10-25, 4090 sm_89 vs the scored tag prism-b10819-909f0ca @ 909f0cac9c, which carries this commit) measured the opponent +2.11% decode (88.80 -> 90.68, the largest single-window opp decode move since #258), naming #393 as a cause alongside the #391 fusion - the bench's own words call #393 'host-side, fires on every NVIDIA box'; the host-time class is a league-mover at our geometry. REGIME: warm replay loops that re-validate a stable input per step - capture-and-replay graph/plan paths, per-step eligibility re-checks on unchanged sessions, config validators re-run per item of an unchanged batch, rebuilds of derived maps keyed to unchanged content; the win is O(nodes) host work per step cut to one integer compare, paid where it belongs (first capture, real change). ANTI-REGIME: the input genuinely changes per step (the scan IS the semantic; the design already handles it - the moved token falls back and re-issues); the token minted anywhere except the builder (a hand-assigned id that survives content edits certifies nothing - the token must be invalidated by construction on every rebuild); content-hash identity instead of a builder-minted token (re-hashing the payload can cost the scan it saves - a different tradeoff axis, not this law); the token treated as a security boundary (it is a HINT: in the source a hash-collided cache KEY only costs a fallback scan, never a wrong execution - the scan remains the admission authority). DISTINCT-FROM (grep-verified in this corpus): same-input-folded-transform-memo-per-build (kernel_opt gpu_27 / B116) memoizes a BUILT transform NODE keyed (input, rotation) per graph build - construction CSE returning a shared subgraph node; THIS skips a RE-VALIDATION pass keyed by a certifying token - nothing is built or memoized but the verdict 'the scan passed', and the lifetime axis differs (one build vs per-step replay). autotuner-key-memoized-per-launch-not-rebuilt (kernel_opt gpu_115 / B265) is the adjacent per-step host-cost axis: there the memoized thing is a rebuilt cache KEY plus the tuner's DECISION (a decision replayed raw, bypassing the wrapper), here the skipped thing is a RE-VALIDATION of an unchanged premise; there a wrong warm hit answers a launch it does not match, here a wrong hit is structurally impossible when the token is builder-minted + success-path-stored + absence-guarded - and the full scan stays the fallback either way. per-request-dirty-range-clear-graph-capture-sticky-marks (kernel_opt gpu_105 / B251, + its Batch-265 rider on pool-free graph-set invalidation, llama.cpp #26771: 'captured CUDA graphs can still use freed buffers, so they must be captured again') is the capture-reuse family's OTHER faces - what stays dirty across clears under a live capture, and graph-set LIFECYCLE against buffer reclamation; THIS owns the per-replay-step VALIDATION cost the same replay path pays - compose, distinct axes. cache-reuse-assertion-gated-on-eviction-counter (rust_perf entries_40 / B270, same category) gates a TEST assertion about cache reuse on an eviction counter; THIS is a runtime skip of repeated validation. fail-closed-multi-resource-commit-transaction (kernel_opt gpu_08) carries the nearest skip-revalidation kin ('a prevalidated flag skips revalidation') - there the skip is ONE-SHOT inside one validated transaction with the error policy flipping after the first mutation; HERE the skip is PER-STEP across steps, certified by a persistent identity token with the absence guard and the scan fallback. GREP EVIDENCE: 'certif|identity token|uid|revalidat|compatibility scan' over src/domains/*/corpus/ - the certif* hits are certification-of-CORRECTNESS senses (error-bound screens, per-instance coverage sets, capability verdicts, arrival bits, admitted-by-enumeration gates); the revalidat* hits are the commit-transaction prevalidated flag and a resume-provenance rule; 'identity token', standalone 'uid' and 'compatibility scan' - ZERO hits: no owner of the certifying-token validation skip. Fixtures ORIGINAL (the capture-and-replay host re-authored as a plain-Rust stand-in with neutral names - no upstream code or identifiers pasted into the fixtures; the quotes above are the only place the source speaks). CITE: PrismML llama.cpp fork commit 63f16c408fc87a77718eb839f2e3c6a5bfa5cb62 #393 ('ggml-backend, cuda : cut ~55-65 us of host time per decode step (batched sched input uploads, no node scans on CUDA graph replay; byte-identical)', MIT lineage) - ggml/src/ggml-cuda/ggml-cuda.cu (the admission line + the gb10_shared_q8_consumer_counts gate; the uid mint is ggml_graph_next_uid, a monotonic counter starting at 1, and the store site is ggml_cuda_graph_update_required whose sole caller sits behind the compatibility admission) + ggml/src/ggml-backend.cpp (the OTHER half, disclosed not mined); our-box record: riir-ai .benchmarks/1000_prism_b10819_bout.md. Batch 283 / Plan 287 / Research 320.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}
