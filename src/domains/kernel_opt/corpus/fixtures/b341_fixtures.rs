//! Batch 341 fixtures — the metal-graph laws as ONE standalone file.
//!
//! Compiled AND executed BEFORE entry: `rustc --edition 2021 --test
//! b341_fixtures.rs` (the B336/B338/B339 discipline). Each rule's ref/opt
//! arms in `entries_gpu_153.rs` (kernel_opt) and `entries_68.rs` (rust_perf)
//! embed the same executed code. ORIGINAL Rust throughout (the source is
//! Apache-2.0; the fixtures are still original syntheses, never pasted
//! source).
//!
//! F1 = replay-group-eligibility-excludes-mutable-surface (kernel_opt)
//! F2 = patch-validity-against-declared-resource-contract (kernel_opt)
//! F3 = amortized-reuse-claim-break-even-artifact (rust_perf)

// ─────────────────────────────────────────────────────────────────────────────
// F1 — the replay-eligibility classifier: replay is the opt-in fast path;
// any dispatch carrying mutable surface refuses WITH A TYPED REASON; one bad
// dispatch falls the whole group back; the diagnostics count every refusal.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Reason {
    #[default]
    None,
    PatchableField,
    UnknownResourceAccess,
    IneligibleNode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Access {
    ReadOnly,
    Write,
    Unknown,
}

#[derive(Clone, Copy)]
struct Dispatch<'a> {
    patch_flags: u32,
    scalar_count: u32,
    buffers: &'a [(u32, Access)],
}

fn dispatch_eligible(d: &Dispatch) -> Reason {
    if d.patch_flags != 0 || d.scalar_count != 0 {
        return Reason::PatchableField;
    }
    for &(_, access) in d.buffers {
        if access == Access::Unknown {
            return Reason::UnknownResourceAccess;
        }
    }
    Reason::None
}

#[derive(Default, PartialEq, Eq, Debug)]
struct Diag {
    planned: u32,
    used: u32,
    fallback: u32,
    last: Reason,
}

fn plan_group(group: &[Dispatch], diag: &mut Diag) {
    diag.planned += 1;
    if group.len() != 1 {
        // multi-node groups fall back until eligibility proves per-node
        diag.fallback += 1;
        diag.last = Reason::IneligibleNode;
        return;
    }
    let reason = dispatch_eligible(&group[0]);
    if reason != Reason::None {
        diag.fallback += 1;
        diag.last = reason;
        return;
    }
    diag.used += 1;
}

#[test]
fn f1_typed_refusals_and_group_fallback() {
    let ro = [(0u32, Access::ReadOnly)];
    assert_eq!(
        dispatch_eligible(&Dispatch { patch_flags: 0, scalar_count: 0, buffers: &ro }),
        Reason::None
    );
    assert_eq!(
        dispatch_eligible(&Dispatch { patch_flags: 1, scalar_count: 0, buffers: &ro }),
        Reason::PatchableField
    );
    let unk = [(1u32, Access::Unknown)];
    assert_eq!(
        dispatch_eligible(&Dispatch { patch_flags: 0, scalar_count: 0, buffers: &unk }),
        Reason::UnknownResourceAccess
    );

    // multi-node groups fall back FIRST (the source checks node_count != 1
    // before per-node classification, metal_backend.mm:524-528) -- the
    // reason is IneligibleNode even when a member would also fail.
    let good = Dispatch { patch_flags: 0, scalar_count: 0, buffers: &ro };
    let bad = Dispatch { patch_flags: 2, scalar_count: 0, buffers: &ro };
    let mut diag = Diag::default();
    plan_group(&[good, bad], &mut diag);
    assert_eq!(diag.used, 0, "no partial replay");
    assert_eq!(diag.fallback, 1);
    assert_eq!(diag.last, Reason::IneligibleNode);

    // single-node groups classify per-dispatch:
    let mut diag_bad = Diag::default();
    plan_group(&[bad], &mut diag_bad);
    assert_eq!(diag_bad.last, Reason::PatchableField);
    assert_eq!(diag_bad.used, 0);
    assert_eq!(diag_bad.fallback, 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// F2 — the patch validator: binding ≠ contract; every patch is proven against
// the DECLARED contract per aspect, the violated clause NAMED.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Aspect {
    Buffer,
    Range,
    Grid,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PatchErr {
    NotDeclaredPatchable(Aspect),
    NoDeclaredRange,
    RangeIncompatible,
    GridExceedsDeclaredMaximum,
}

struct Contract {
    patchable: [bool; 3],
    range: Option<(usize, usize)>, // declared [base, base+len)
    align: usize,
    max_grid: u64,
}

fn validate_patch(
    c: &Contract,
    aspect: Aspect,
    off: usize,
    len: usize,
    grid: u64,
) -> Result<(), PatchErr> {
    let slot = match aspect {
        Aspect::Buffer => 0,
        Aspect::Range => 1,
        Aspect::Grid => 2,
    };
    if !c.patchable[slot] {
        return Err(PatchErr::NotDeclaredPatchable(aspect));
    }
    if aspect == Aspect::Grid {
        return if grid > c.max_grid {
            Err(PatchErr::GridExceedsDeclaredMaximum)
        } else {
            Ok(())
        };
    }
    let (base, dlen) = c.range.ok_or(PatchErr::NoDeclaredRange)?;
    if off % c.align != 0 || off + len > base + dlen {
        return Err(PatchErr::RangeIncompatible);
    }
    Ok(())
}

#[test]
fn f2_the_four_refusals_named_by_their_clause() {
    let c = Contract {
        patchable: [true, true, true],
        range: Some((0, 256)),
        align: 16,
        max_grid: 1024,
    };
    let no_range = Contract { range: None, ..c };
    assert_eq!(
        validate_patch(&no_range, Aspect::Range, 0, 8, 0),
        Err(PatchErr::NoDeclaredRange)
    );
    assert_eq!(
        validate_patch(&c, Aspect::Grid, 0, 0, 2048),
        Err(PatchErr::GridExceedsDeclaredMaximum)
    );
    assert_eq!(
        validate_patch(&c, Aspect::Range, 8, 16, 0),
        Err(PatchErr::RangeIncompatible)
    );
    assert_eq!(
        validate_patch(&c, Aspect::Range, 240, 32, 0),
        Err(PatchErr::RangeIncompatible)
    );
    assert_eq!(validate_patch(&c, Aspect::Range, 16, 32, 0), Ok(()));
    assert_eq!(validate_patch(&c, Aspect::Grid, 0, 0, 1024), Ok(()));

    // per-aspect patchability: enabling buffer+grid never enables range
    let partial = Contract {
        patchable: [true, false, true],
        ..c
    };
    assert_eq!(
        validate_patch(&partial, Aspect::Range, 16, 8, 0),
        Err(PatchErr::NotDeclaredPatchable(Aspect::Range))
    );
    assert_eq!(validate_patch(&partial, Aspect::Buffer, 0, 8, 0), Ok(()));
}

// ─────────────────────────────────────────────────────────────────────────────
// F3 — the break-even artifact: components separated, break-even carried,
// savings <= 0 a LOUD named verdict (None), never folded into an aggregate.
// ─────────────────────────────────────────────────────────────────────────────

struct BreakEvenReport {
    baseline_us: f64,
    bake_us: f64,
    patch_us: f64,
    launch_us: f64,
    per_launch_savings: f64,
    break_even_launches: Option<f64>,
}

fn measure(baseline_us: f64, bake_us: f64, patch_us: f64, launch_us: f64) -> BreakEvenReport {
    let savings = baseline_us - launch_us;
    BreakEvenReport {
        baseline_us,
        bake_us,
        patch_us,
        launch_us,
        per_launch_savings: savings,
        break_even_launches: if savings > 0.0 {
            Some(bake_us / savings)
        } else {
            None
        },
    }
}

// the FAIR-BASELINE pin: a baseline must not secretly pre-bake — the audit
// walks the baseline's declared behaviors and refuses a private graph-
// equivalent runtime (precomputed schedules / patch tables / replay
// structures) unless the report LABELS the arm bespoke.
#[derive(Default)]
struct BaselineAudit {
    caches_pipelines: bool,
    per_iteration_buffers: bool,
    precomputed_schedules: bool,
    labeled_bespoke: bool,
}

fn baseline_is_fair(a: &BaselineAudit) -> bool {
    if !a.caches_pipelines || !a.per_iteration_buffers {
        return false; // weaker than ordinary competent direct code
    }
    a.labeled_bespoke || !a.precomputed_schedules
}

#[test]
fn f3_break_even_carried_and_loud_on_non_positive() {
    let good = measure(20.0, 9000.0, 5.0, 12.0);
    assert_eq!(good.per_launch_savings, 8.0);
    assert_eq!(good.break_even_launches, Some(1125.0));

    let slower = measure(20.0, 9000.0, 5.0, 20.5);
    assert_eq!(slower.break_even_launches, None, "savings < 0: loud None");
    let flat = measure(20.0, 9000.0, 5.0, 20.0);
    assert_eq!(flat.break_even_launches, None, "savings == 0: not a positive claim");
}

#[test]
fn f3_the_fair_baseline_law() {
    let fair = BaselineAudit {
        caches_pipelines: true,
        per_iteration_buffers: true,
        precomputed_schedules: false,
        labeled_bespoke: false,
    };
    assert!(baseline_is_fair(&fair));

    let strawman = BaselineAudit {
        caches_pipelines: true,
        per_iteration_buffers: true,
        precomputed_schedules: true,
        labeled_bespoke: false,
    };
    assert!(!baseline_is_fair(&strawman), "secret pre-bake refused");

    let labeled = BaselineAudit {
        caches_pipelines: true,
        per_iteration_buffers: true,
        precomputed_schedules: true,
        labeled_bespoke: true,
    };
    assert!(baseline_is_fair(&labeled), "labeled bespoke-executor arm allowed");

    let weak = BaselineAudit {
        caches_pipelines: false,
        per_iteration_buffers: true,
        precomputed_schedules: false,
        labeled_bespoke: false,
    };
    assert!(!baseline_is_fair(&weak), "below competent-direct is not a baseline");
}
