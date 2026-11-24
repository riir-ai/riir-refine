//! Batch 412 kernel fixture preflight — the mlxfast staged pool's
//! kernel_opt member (Plan 411 / Research 445, the Research 437 staged
//! candidates, Layr-Labs/mlxfast-bonsai2-27b-engine @ 0c7c3923, MIT —
//! the route law lives in the vendored model-finalizer surface) as ONE
//! standalone file (1 rule, the four-arm discipline).
//!
//! ORIGINAL plain-Rust reproduction — zero repo source pasted.
//! Compiled AND executed BEFORE entry:
//! `rustc --edition 2021 --test b412_fixtures.rs`
//! at debug AND -O.
//!
//! Arms:
//! - F1 (route-bound-at-construction): the re-probe arm (the hot path
//!   re-checks eligibility every forward — per-call work is a re-probe,
//!   and a lazy flag can flip lanes mid-run); the construction-bound arm
//!   (the gate answers ONCE, the hot closure carries only the shape
//!   predicate); the no-recovery arm (ineligible binds Stock and STAYS
//!   Stock — no mid-run upgrade, no fallback-then-retry); the
//!   one-load-one-route arm (two routers bound from different profiles
//!   share no mutable route state — no mixing).
#![allow(dead_code)]

// ══════════════ F1 — route-bound-at-construction-hot-closure-shape-predicate-only ══════════════

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Lane {
    /// the installed optimized lane (rows 1..=16 only)
    RowOwnedMetal,
    /// the generic route
    Stock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Profile {
    Decode,
    Full,
    /// an ineligible serving profile
    Translate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Geometry {
    hidden: usize,
    experts: usize,
    top_k: usize,
}

/// the geometry contract the optimized lane is compiled for
const CONTRACT: Geometry = Geometry {
    hidden: 2_048,
    experts: 256,
    top_k: 8,
};
/// the row window the installed Metal lane owns
const MAX_ROWS: usize = 16;

/// The CONSTRUCTION GATE: eligibility is answered ONCE, at bind time —
/// the returned router is immutable and its hot path carries ONLY the
/// data-dependent rows predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Router {
    /// permanently the stock lane
    StockOnly,
    /// the installed Metal lane for rows 1..=MAX_ROWS, stock beyond
    RowOwnedLane,
}

fn bind_route(profile: Profile, g: Geometry) -> Router {
    // the gate's `else { return stock }` is permanent: the hot path
    // never parses configuration, reads the environment, or recovers
    // from an ineligible dispatch.
    match profile {
        Profile::Decode | Profile::Full if g == CONTRACT => Router::RowOwnedLane,
        _ => Router::StockOnly,
    }
}

impl Router {
    /// The hot path: exactly ONE data-dependent predicate — the rows
    /// compare. No eligibility re-probe, no profile read, no env read.
    fn route(&self, rows: usize) -> Lane {
        match self {
            Router::StockOnly => Lane::Stock,
            Router::RowOwnedLane => {
                if rows >= 1 && rows <= MAX_ROWS {
                    Lane::RowOwnedMetal
                } else {
                    Lane::Stock
                }
            }
        }
    }
}

/// BUGGY SHAPE: the hot path re-checks eligibility every forward —
/// profile, geometry, availability — and a lazily-initialized flag can
/// FLIP lanes mid-run.
#[derive(Debug)]
struct ReProbingRouter {
    profile: Profile,
    g: Geometry,
    /// lazy: None until the first forward "probes" it
    installed: Option<bool>,
}

impl ReProbingRouter {
    fn route(&mut self, rows: usize) -> Lane {
        // a lazy probe: the lane can CHANGE between call 1 and call N
        // if anything the probe reads moved.
        if self.installed.is_none() {
            self.installed =
                Some(matches!(self.profile, Profile::Decode | Profile::Full) && self.g == CONTRACT);
        }
        // and every call re-reads the environment-shaped input anyway:
        let env_now_matches = self.g == CONTRACT; // a "live" re-probe
        if self.installed == Some(true) && env_now_matches && rows <= MAX_ROWS && rows >= 1 {
            Lane::RowOwnedMetal
        } else {
            Lane::Stock
        }
    }
}

#[test]
fn f1_hot_path_is_shape_predicate_only() {
    let r = bind_route(Profile::Decode, CONTRACT);
    // every call: the rows compare decides; nothing else is consulted.
    assert_eq!(r.route(1), Lane::RowOwnedMetal);
    assert_eq!(r.route(16), Lane::RowOwnedMetal);
    assert_eq!(r.route(17), Lane::Stock);
    assert_eq!(r.route(0), Lane::Stock);
}

#[test]
fn f1_buggy_lazy_flag_flips_midrun() {
    // the defect shape: a lazy/mutable lane decision can differ between
    // calls when the probe input moves — a mid-run lane flip the
    // construction-bound router cannot express.
    let mut r = ReProbingRouter {
        profile: Profile::Decode,
        g: CONTRACT,
        installed: None,
    };
    assert_eq!(r.route(4), Lane::RowOwnedMetal);
    // "the environment changed" (the geometry field is the stand-in):
    r.g = Geometry {
        hidden: 1,
        experts: 1,
        top_k: 1,
    };
    // the re-probing router DEMOTES the lane mid-run:
    assert_eq!(
        r.route(4),
        Lane::Stock,
        "the lazy re-probe flipped the lane mid-run"
    );
    // the construction-bound router CANNOT do this: its decision is
    // frozen in the enum, the hot path reads only rows.
    let frozen = bind_route(Profile::Decode, CONTRACT);
    assert_eq!(frozen.route(4), Lane::RowOwnedMetal);
}

#[test]
fn f1_ineligible_binds_stock_and_stays_stock() {
    // an ineligible profile binds Stock permanently — even at rows the
    // optimized lane owns. No mid-run upgrade, no recovery retry.
    let r = bind_route(Profile::Translate, CONTRACT);
    assert_eq!(r, Router::StockOnly);
    for rows in 1..=16 {
        assert_eq!(r.route(rows), Lane::Stock, "rows {rows}");
    }
    // the geometry miss binds Stock too.
    let r2 = bind_route(
        Profile::Decode,
        Geometry {
            hidden: 4_096,
            experts: 256,
            top_k: 8,
        },
    );
    assert_eq!(r2, Router::StockOnly);
}

#[test]
fn f1_one_load_one_route_no_mixing() {
    // two routers bound from different profiles are independent values —
    // there is no shared mutable route state to mix (the source's
    // Task-locality law, modeled structurally: Router is Copy, carries
    // no shared cell).
    let a = bind_route(Profile::Decode, CONTRACT);
    let b = bind_route(
        Profile::Translate,
        Geometry {
            hidden: 2_048,
            experts: 256,
            top_k: 8,
        },
    );
    assert_eq!(a.route(4), Lane::RowOwnedMetal);
    assert_eq!(b.route(4), Lane::Stock);
    // binding b again changes nothing about a.
    let b2 = bind_route(Profile::Translate, CONTRACT);
    assert_eq!(b2, Router::StockOnly);
    assert_eq!(
        a.route(4),
        Lane::RowOwnedMetal,
        "router a unaffected by b's binding"
    );
}
