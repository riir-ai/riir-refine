//! Plan 225 T5.2 — the owner-condition gate (the 2026-10-30 verdict): the
//! armed seat may enter the REAL fix path only behind a serve boundary
//! where a WRONG SEAT PICK IS REJECTED and the modelless answer stands in.
//!
//! The wrong-pick legs exercise the REAL `HealPipeline::heal_with_pool`
//! seam (the R6 pool contract — the miss path whose pool carries the
//! memory candidate + the proposer's own output) against a real loopback
//! seat speaking the v1 rerank contract; the compile check is the
//! boundary's injected seam (the production caller passes the
//! `fix_verify`-backed gate — no parallel checker).
//!
//! `#![cfg]` protects the COUNT; the Cargo.toml `required-features` row
//! protects the READER (the T1.1e repo-birth law — landed same commit).
#![cfg(all(feature = "escalation_manifest", feature = "seat_rerank"))]

use riir_refine::code_egress::EgressConsent;
use riir_refine::drafter::TernaryDrafter;
use riir_refine::escalation_dispatch::{EscalatedFix, EscalationDispatch, ReadinessPolicy};
use riir_refine::escalation_manifest::EscalationManifest;
use riir_refine::escalation_serve::{ServeParams, serve_seat_rerank_checked_with};
use riir_refine::pruner::RustAnalyzerPruner;
use riir_refine::{ClippyFixEntry, HealPipeline};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

const RULE: &str = "needless_return";
const BUGGY: &str = "fn foo() -> i32 { return 42; }";

/// A memory fix deliberately DISTINCT from the proposer's own output, so
/// the real miss-pool carries two members (the manifest's frozen
/// min-pool floor is 2 — a 1-member pool never becomes seat traffic).
const MEMORY_FIX: &str = "fn foo() -> i32 { 7 }";

fn manifest_for(endpoint: &str) -> EscalationManifest {
    let json = format!(
        "{{\"domains\":{{\"clippy\":{{\"modelless_cap\":0.5,\"max_rate\":0.9,\
         \"target\":{{\"kind\":\"seat_rerank\",\"endpoint\":\"{endpoint}\"}}}}}}}}"
    );
    let m: EscalationManifest = serde_json::from_str(&json).expect("manifest json");
    m.validate().expect("manifest valid");
    m
}

/// A real loopback seat that answers up to `n` connections with a
/// contract-valid pick of `pool[pick_index]` (the caller's list by index —
/// the R7 echo is the trust anchor).
fn seat_pick_server(pool: &[String], pick_index: usize, n: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("loopback bind");
    let addr = listener.local_addr().expect("addr");
    let body = format!(
        "{{\"suite\":\"heal\",\"options\":{},\"pick\":{},\"pick_index\":{},\
         \"abstained\":false,\"confidence\":0.9}}",
        serde_json::to_string(pool).expect("pool json"),
        serde_json::to_string(&pool[pick_index]).expect("pick json"),
        pick_index
    );
    std::thread::spawn(move || {
        for _ in 0..n {
            let Ok((mut sock, _)) = listener.accept() else {
                break;
            };
            let mut buf = Vec::with_capacity(16_384);
            let mut chunk = [0u8; 4096];
            loop {
                let read = sock.read(&mut chunk).unwrap_or(0);
                if read == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..read]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4) {
                    let head = String::from_utf8_lossy(&buf[..pos]).to_ascii_lowercase();
                    let clen: usize = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse().ok())
                        .unwrap_or(0);
                    if buf.len() >= pos + clen {
                        break;
                    }
                }
            }
            let http = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(http.as_bytes());
        }
    });
    format!("http://{addr}")
}

fn miss_pool_pipeline() -> (HealPipeline, Vec<String>) {
    let mut p = HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new());
    // One memory trajectory with a fix DISTINCT from the proposer's own
    // output: on the empty-corpus miss the relevance gate declines SERVING
    // the memory candidate, but the pool was already collected — and birth
    // point 3b appends the proposer's own output beside it (the Issue-161
    // pool contract this plan's R6 law rides).
    p.report_outcome(RULE, BUGGY, MEMORY_FIX, true, false);
    let empty: [ClippyFixEntry; 0] = [];
    let outcome = p.heal_with_pool(RULE, BUGGY, &empty);
    assert!(
        outcome.result.is_none(),
        "memory-only on an empty corpus is the miss path, got {:?}",
        outcome.result.map(|r| r.fixed_code)
    );
    assert!(
        outcome.pool.len() >= 2,
        "the real miss-pool must clear the frozen min-pool floor of 2, got {:?}",
        outcome.pool
    );
    (p, outcome.pool)
}

/// THE owner-condition leg: through the real heal_with_pool seam, a seat
/// pick that FAILS the serve-boundary compile check is rejected
/// (StoodByModelless naming the boundary) and the modelless answer (the
/// miss — nothing served) stands.
#[test]
fn wrong_seat_pick_is_rejected_and_the_modelless_answer_stands() {
    let (_p, pool) = miss_pool_pipeline();
    let endpoint = seat_pick_server(&pool, 0, 1);
    let manifest = manifest_for(&endpoint);
    let mut dispatch = EscalationDispatch::arm_with(&manifest, "clippy", None)
        .expect("arm ok")
        .expect("armed");

    let deadline = Instant::now() + Duration::from_millis(2_000);
    let consent = EgressConsent::parse(Some("snippet"));
    let mut never_compiles = |_: &str| false;
    let decision = serve_seat_rerank_checked_with(
        &mut dispatch,
        false, // the modelless path MISSED (the seat's one case)
        BUGGY,
        &pool,
        ServeParams {
            deadline,
            policy: ReadinessPolicy::default(),
            consent: &consent,
            compile_check: &mut never_compiles,
        },
    );
    match decision {
        EscalatedFix::StoodByModelless {
            reason: riir_refine::escalation_dispatch::StandReason::Declined(msg),
        } => {
            assert!(
                msg.contains("compile check"),
                "the stand must name the boundary, got: {msg}"
            );
        }
        EscalatedFix::Answer { fix, .. } => {
            panic!("a compile-failing pick was SERVED: {fix}");
        }
        other => panic!("expected the boundary Decline, got {other:?}"),
    }
}

/// The positive half of the same boundary: a pick that PASSES the check is
/// served verbatim (the boundary is the compile gate, not a blanket veto).
#[test]
fn passing_seat_pick_is_served_through_the_boundary() {
    let (_p, pool) = miss_pool_pipeline();
    let endpoint = seat_pick_server(&pool, 0, 1);
    let manifest = manifest_for(&endpoint);
    let mut dispatch = EscalationDispatch::arm_with(&manifest, "clippy", None)
        .expect("arm ok")
        .expect("armed");

    let deadline = Instant::now() + Duration::from_millis(2_000);
    let consent = EgressConsent::parse(Some("snippet"));
    let mut compiles = |fix: &str| fix.starts_with("fn foo");
    let decision = serve_seat_rerank_checked_with(
        &mut dispatch,
        false,
        BUGGY,
        &pool,
        ServeParams {
            deadline,
            policy: ReadinessPolicy::default(),
            consent: &consent,
            compile_check: &mut compiles,
        },
    );
    match decision {
        EscalatedFix::Answer { fix, source, .. } => {
            assert_eq!(fix, pool[0], "the served pick is the caller's candidate");
            assert_eq!(source, "seat_pick");
        }
        other => panic!("expected the checked pick to be served, got {other:?}"),
    }
}

/// The worthiness law at the boundary: a SERVED modelless fix maps to the
/// gate's Worthy stand — no socket is touched (the endpoint is a dead
/// port; any seat traffic would surface as a Decline, never a Worthy) and
/// the modelless fix the caller already holds is the answer.
#[test]
fn served_modelless_fix_stands_without_seat_traffic() {
    let manifest = manifest_for("http://127.0.0.1:9");
    let mut dispatch = EscalationDispatch::arm_with(&manifest, "clippy", None)
        .expect("arm ok")
        .expect("armed");

    // The REAL served case: a corpus entry whose buggy shape matches →
    // heal_with_pool SERVES it; the boundary must stand it untouched.
    let corpus = [ClippyFixEntry {
        lint_name: RULE.to_string(),
        lint_group: "style".to_string(),
        buggy_code: BUGGY.to_string(),
        fixed_code: "fn foo() -> i32 { 42 }".to_string(),
    }];
    let mut p = HealPipeline::new(TernaryDrafter::fallback(), RustAnalyzerPruner::new());
    let outcome = p.heal_with_pool(RULE, BUGGY, &corpus);
    let served = outcome.result.as_ref().expect("corpus-backed heal serves");
    assert_eq!(served.fixed_code, "fn foo() -> i32 { 42 }");

    let deadline = Instant::now() + Duration::from_millis(2_000);
    let consent = EgressConsent::parse(Some("snippet"));
    let mut unused = |_: &str| panic!("the seat must not be consulted on a worthy case");
    let decision = serve_seat_rerank_checked_with(
        &mut dispatch,
        outcome.result.is_some(),
        BUGGY,
        &outcome.pool,
        ServeParams {
            deadline,
            policy: ReadinessPolicy::default(),
            consent: &consent,
            compile_check: &mut unused,
        },
    );
    match decision {
        EscalatedFix::StoodByModelless {
            reason: riir_refine::escalation_dispatch::StandReason::Worthy,
        } => {}
        other => panic!("expected the Worthy stand, got {other:?}"),
    }
}

/// A row that sets `modelless_cap: 0.0` never escalates — 0.0 is not a
/// miss under 0.0 (the gate's own grammar is the operator's off dial).
#[test]
fn zero_cap_row_never_escalates_a_miss() {
    let json = "{\"domains\":{\"clippy\":{\"modelless_cap\":0.0,\"max_rate\":0.9,\
                \"target\":{\"kind\":\"seat_rerank\",\"endpoint\":\"http://127.0.0.1:9\"}}}}";
    let m: EscalationManifest = serde_json::from_str(json).expect("manifest json");
    m.validate().expect("manifest valid");
    let mut dispatch = EscalationDispatch::arm_with(&m, "clippy", None)
        .expect("arm ok")
        .expect("armed");

    let deadline = Instant::now() + Duration::from_millis(2_000);
    let consent = EgressConsent::parse(Some("snippet"));
    let mut unused = |_: &str| panic!("a zero-cap row must never reach the seat");
    let decision = serve_seat_rerank_checked_with(
        &mut dispatch,
        false,
        BUGGY,
        &["a".to_string(), "b".to_string()],
        ServeParams {
            deadline,
            policy: ReadinessPolicy::default(),
            consent: &consent,
            compile_check: &mut unused,
        },
    );
    match decision {
        EscalatedFix::StoodByModelless {
            reason: riir_refine::escalation_dispatch::StandReason::Worthy,
        } => {}
        other => panic!("expected the Worthy stand, got {other:?}"),
    }
}
