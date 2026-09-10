//! Issue 025 T2 — native-regex scanner over the **non-syn WGSL surface**.
//!
//! The healer and the mining program are blind to every surface `syn` cannot
//! parse. This module is the prototype scanner for the first of those
//! surfaces: hand-written `.wgsl` kernel files (123 in riir-gpu / 10,830
//! lines, T1 inventory). It exists to open `--suggest` coverage on real
//! kernel code and to seed WGSL buggy→fixed mining from riir-gpu git history
//! (the fix commits behind Issues 697/698/701 carry literal before/after
//! `.wgsl` pairs).
//!
//! ## Scope ruling (from the issue — binding)
//!
//! - **Suggest-only.** No proposers, no `--fix` surface. Findings are
//!   citations of corpus `rule_id`s, never edits.
//! - **Native `regex` crate** — no WASM boundary for first-party rules
//!   (serialization tax + a `rubrc`-class external-runtime risk for zero
//!   benefit while rules are ours).
//! - **AST-exact Rust detection is NOT regressed** — this module never
//!   touches `.rs` files. The Plan 005 T6.4 lesson (token heuristics on Rust
//!   false-positived on `v[v[i]]`) is why.
//!
//! ## Precision-first stance (the T5 gate is `zero` false positives on the
//! ## clean-file control)
//!
//! Each pattern fronts a corpus rule by citing its `rule_id`, and each is
//! restricted to shapes that are **textually unambiguous**. Where a rule's
//! real discriminator needs dataflow the text does not carry, the pattern is
//! either conservative (biased to miss, never to fire on clean code) or the
//! rule is recorded as **not regex-expressible** in v1:
//!
//! | corpus rule (fronted) | textual tell | v1 status |
//! |---|---|---|
//! | `row-dot-split-inner-axis-block-reduce` (B44) | `@workgroup_size(1…)` — a compute shader mapped to single-thread workgroups doing serial loops (real carriers: `m2p_attention.wgsl` ×3, `spectralquant_attention.wgsl`) | ✅ pattern |
//! | `declared-max-array-taxed-per-thread-under-runtime-bound` (B46; the m2p `array<f32, 256>` scores instance behind Issue 701 H-B) | private/function-scope `var x: array<T, N>;` with `N ≥ 128` (threshold = floor of observed corpus instances: 128, 256); `var<workgroup>`/storage/uniform excluded — those are legitimately shared | ✅ pattern |
//! | `share-cube-size-const-between-kernel-and-launcher` (B44) | `@workgroup_size(N)` with `N > 1` AND the same `N` restated as a `let` binding in the body — the silent cross-site stride contract (the arithmetic-only form is deliberately NOT flagged: a bare `N` may be a tensor dim) | ✅ pattern (let-form only) |
//! | `multi-head-out-proj-consumes-head-concat` (B47; Issue 701 H-C) | an accumulation `acc (+)= scalar * w[…loop var…]` inside a nested `for` — the multiplicative operand invariant in the summation index (the scalar-times-rowsum that is not a matvec) | ⚠️ **experimental, OFF by default** — the first live calibration materialized the documented FP class: its ONLY firing on the current tree is `layer_fuse.wgsl` L61, a legitimate `alpha`-lerp (`val = val + alpha * target_hiddens[…]`), textually identical to the bug. Enabled only via [`WgslScanner::with_experimental`] for git-history mining where the shape IS the bug (the Issue 701 pre-fix `m2p`) |
//! | `smem-staging-gated-by-cross-thread-reuse` (B35) | **single-slot staging** (mined from the Issue 686 H7 fix pair, T6): a `var<workgroup>` array whose write is a BARE pass-through load (`A[i] = src[…]` — no top-level arithmetic) and whose every read uses ONE non-varying index expression — no thread reads two distinct slots, so the staging buys nothing a direct global read doesn't (same-address reads broadcast) while costing barriers. The computed-write form (`A[i] = x * w[…]`, the live `role_transport.wgsl`) is deliberately NOT flagged — staging a value computed once and reused by 16 threads IS cross-thread reuse > 1 | ✅ pattern (T6; upgraded from "not regex-expressible" — the 686 fix pair supplied the textual tell: bare-load write + single-slot reads) |
//! | `row-dot-split-inner-axis-block-reduce` (B44) — **body form** | a serial-stride dual-index dot (`acc = acc + x[base + j] * y[base2 + j]` with `j += 1` evidence nearby) — one thread serially looping the whole row length (the Issue 698 pre-fix `dot_per_row_f32` cube body, B44's own mining source) | ⚠️ **experimental, OFF by default** — 11 live instances sit under parallel grids (serial-K GEMV shapes: `spectralquant_maxsim`, `muon_fused`, `eigenbasis_rotation`, …) where the fix regime (output count too small to fill the machine) is a workload fact the text does not carry. The attribute form above stays default |
//!
//! Conservative misses are acceptable (the mining loop can widen a pattern
//! when a real buggy file proves it too narrow); a false positive on a clean
//! file poisons the mining loop's garbage-in guard, so patterns stay narrow.

use regex::Regex;
use std::sync::OnceLock;

/// A finding: one rule citation at one source line. Suggest-only output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgslFinding {
    /// The corpus rule this finding fronts (the citation contract).
    pub rule_id: &'static str,
    /// 1-based line number in the scanned source.
    pub line: usize,
    /// The matched line, trimmed (the human-review excerpt).
    pub excerpt: String,
}

/// The DEFAULT pattern set (see the module table) — patterns with zero
/// observed false positives on the current riir-gpu tree (T2 calibration
/// smoke, 123 files). Order = documentation order; `scan` output is sorted
/// by `(line, rule_id)` for determinism.
pub const WGSL_PATTERN_RULES: &[&str] = &[
    "row-dot-split-inner-axis-block-reduce",
    "declared-max-array-taxed-per-thread-under-runtime-bound",
    "share-cube-size-const-between-kernel-and-launcher",
    "smem-staging-gated-by-cross-thread-reuse",
];

/// The experimental pattern set — implemented and tested, but OFF by
/// default because the T2 calibration smoke caught its only live firing on
/// a legitimate `alpha`-lerp (`layer_fuse.wgsl` L61). Enabled via
/// [`WgslScanner::with_experimental`] for git-history mining where the
/// loop-invariant-operand shape IS the bug (the Issue 701 pre-fix m2p).
pub const WGSL_EXPERIMENTAL_RULES: &[&str] = &[
    "multi-head-out-proj-consumes-head-concat",
    // The body-form twin of the default attribute pattern (`@workgroup_size(1…)`):
    // a serial-stride dual-index dot. OFF by default because a serial inner
    // dot under a parallel grid (the serial-K GEMV shape, 11 live instances)
    // is textually unadjudicable — whether the output count is too small to
    // fill the machine is a workload fact. Mining-only (the diff adjudicates).
    "row-dot-split-inner-axis-block-reduce",
];

/// Private-array declared-size threshold: the floor of observed corpus
/// instances (`array<f32, 128>`, `array<f32, 256>` in `m2p_attention.wgsl`).
/// Smaller private arrays are conventional scratch, not the declared-max tax.
const PRIVATE_ARRAY_FLOOR: u64 = 128;

fn re_workgroup_size() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"@workgroup_size\(\s*(\d+)\s*(?:,\s*\d+\s*){0,2}\)").expect("valid regex")
    })
}

/// P2 — single-thread workgroup mapping (`row-dot-split` regime): a compute
/// shader whose workgroup is `(1)`, `(1,1)` or `(1,1,1)` pins one thread per
/// workgroup and serializes the inner loops — the B44 regime where the output
/// count is too small to fill the machine and the reduction axis should be
/// split instead (one workgroup per row, lanes striding coalesced).
fn scan_single_thread_workgroup(source: &str) -> Vec<WgslFinding> {
    let re = re_workgroup_size();
    let mut out = Vec::new();
    for (i, raw) in source.lines().enumerate() {
        // Scan the comment-stripped line: a `// …@workgroup_size(1,1,1)` doc
        // mention is not a finding (the T2 smoke caught exactly that in
        // `attention_batched.wgsl` L12).
        let line = strip_line_comment(raw);
        if let Some(m) = re.captures(&line) {
            let x: u64 = m.get(1).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
            if x == 1 {
                out.push(WgslFinding {
                    rule_id: "row-dot-split-inner-axis-block-reduce",
                    line: i + 1,
                    excerpt: line.trim().to_string(),
                });
            }
        }
    }
    out.shrink_to_fit();
    out
}

/// P6 — single-slot staging (`smem-staging-gated-by-cross-thread-reuse`, the
/// Issue 686 H7 fix shape, mined in T6): a `var<workgroup>` array whose
/// staging buys nothing a direct global read doesn't. The 686 fix removed
/// exactly this from `covariance.wgsl` with the justification "each thread
/// reads exactly one row element per position — same-address reads broadcast
/// in one transaction — so the staging saved no global traffic while costing
/// 2 barriers/position". The textual tell it left behind:
/// - every write is a BARE pass-through load (`A[i] = src[…]…]` — a single
///   array access, no top-level arithmetic: staging a COMPUTED value
///   (`A[i] = x * w[…]`, the live `role_transport.wgsl`) amortizes work across
///   threads and is deliberate reuse > 1 — not flagged);
/// - every read uses ONE identical index expression (no thread reads two
///   distinct slots — tree reductions read `[t]` AND `[t + stride]`, GEMM
///   tiles read compound `[r * 16 + k]` indexes, both not flagged);
/// - that read index is a single bare identifier that never self-increments
///   (a per-thread-constant slot; compound indexes like the transpose's
///   `[local_col * N + local_row]` give each thread a UNIQUE slot — the
///   staging there exists precisely to coalesce, not flagged).
///
/// Under those three guards the staged value is either the thread's own
/// register (write index == read index) or a same-address broadcast (threads
/// sharing the coordinate) — the staging is pure barrier cost. Finding at the
/// DECL line (the actionable site: delete the array, read global directly).
fn scan_single_slot_staging(source: &str) -> Vec<WgslFinding> {
    static DECL: OnceLock<Regex> = OnceLock::new();
    static BARE_LOAD_WRITE: OnceLock<Regex> = OnceLock::new();
    static INC_ADD: OnceLock<Regex> = OnceLock::new();
    static INC_SELF: OnceLock<Regex> = OnceLock::new();
    let decl = DECL.get_or_init(|| Regex::new(r"var<workgroup>\s+(\w+)\s*:").expect("valid regex"));
    let bare_load_write = BARE_LOAD_WRITE.get_or_init(|| {
        // `A[...] = src[...];` and NOTHING else on the line (an optional `as`
        // cast on the RHS index is Rust-cube text; WGSL has none).
        Regex::new(r"^\s*\w+\s*\[[^]]*\]\s*=\s*\w+\s*\[[^]]*\]\s*(?:as\s+\w+\s*)?;\s*$")
            .expect("valid regex")
    });
    let inc_add = INC_ADD.get_or_init(|| Regex::new(r"(\w+)\s*\+=").expect("valid regex"));
    // `X = X + …` — the regex crate has no backreferences, so the two
    // identifiers are captured separately and compared in code.
    let inc_self =
        INC_SELF.get_or_init(|| Regex::new(r"(\w+)\s*=\s*(\w+)\s*\+").expect("valid regex"));

    let lines: Vec<String> = source.lines().map(strip_line_comment).collect();

    // Self-incremented identifiers anywhere (for-loop headers via `k = k + 1u`,
    // while-loops via `j += 1u32`) — a read index drawn from this set is a
    // loop-varying slot, i.e. the thread reads MANY slots (legit staging).
    let mut incremented: Vec<String> = Vec::new();
    for l in &lines {
        for m in inc_add.captures_iter(l) {
            if let Some(v) = m.get(1) {
                incremented.push(v.as_str().to_string());
            }
        }
        for m in inc_self.captures_iter(l) {
            if let (Some(a), Some(b)) = (m.get(1), m.get(2)) {
                if a.as_str() == b.as_str() {
                    incremented.push(a.as_str().to_string());
                }
            }
        }
    }

    let mut out = Vec::new();
    for (li, raw) in lines.iter().enumerate() {
        let Some(dm) = decl.captures(raw) else {
            continue;
        };
        let Some(name) = dm.get(1).map(|g| g.as_str()) else {
            continue;
        };

        // Occurrence scan: `name[…]` on every line; classify write (followed
        // by `=` not `==`) vs read; collect the read-index texts.
        let open = format!("{name}[");
        let mut has_write = false;
        let mut all_writes_bare_load = true;
        let mut read_idx: Vec<String> = Vec::new();
        for l in &lines {
            let mut from = 0usize;
            while let Some(p) = l[from..].find(&open) {
                let start = from + p + open.len();
                let Some(rel) = l[start..].find(']') else {
                    break;
                };
                let idx = l[start..start + rel].trim().to_string();
                let after = l[start + rel + 1..].trim_start();
                if let Some(rest) = after.strip_prefix('=') {
                    if rest.starts_with('=') {
                        read_idx.push(idx); // `A[i] == …` comparison reads A
                    } else {
                        has_write = true;
                        if !bare_load_write.is_match(l.trim()) {
                            all_writes_bare_load = false;
                        }
                    }
                } else {
                    read_idx.push(idx);
                }
                from = start + rel + 1;
            }
        }
        if !has_write || !all_writes_bare_load || read_idx.is_empty() {
            continue;
        }
        // ONE identical read-index text across every read.
        let first = read_idx[0].clone();
        if !read_idx.iter().all(|r| r == &first) {
            continue;
        }
        // The index is a single bare identifier (no compound transpose/tile
        // expressions) that never self-increments (per-thread-constant slot).
        let is_bare_ident = first.chars().all(|c| c.is_alphanumeric() || c == '_')
            && first
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_');
        if !is_bare_ident || incremented.iter().any(|v| v == &first) {
            continue;
        }
        out.push(WgslFinding {
            rule_id: "smem-staging-gated-by-cross-thread-reuse",
            line: li + 1,
            excerpt: raw.trim().to_string(),
        });
    }
    out.shrink_to_fit();
    out
}

/// P3 — declared-max private array (`declared-max-array-taxed`): a
/// function/private-scope array sized to a hard maximum pays the DECLARED max
/// in indexable thread-local memory per thread regardless of the runtime
/// bound (the `array<f32, 256>` scores instance, Issue 701 H-B). Fix shape:
/// pipeline override constants per bound class. `var<workgroup>` /
/// `var<storage>` / `var<uniform>` are excluded — those are shared by design.
fn scan_declared_max_private_array(source: &str) -> Vec<WgslFinding> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"^\s*var\s*(?:<\s*private\s*>)?\s+\w+\s*:\s*array\s*<[^<>]+,\s*(\d+)\s*>")
            .expect("valid regex")
    });
    let mut out = Vec::new();
    for (i, raw) in source.lines().enumerate() {
        let line = strip_line_comment(raw);
        if let Some(m) = re.captures(&line) {
            let n: u64 = m.get(1).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
            if n >= PRIVATE_ARRAY_FLOOR {
                out.push(WgslFinding {
                    rule_id: "declared-max-array-taxed-per-thread-under-runtime-bound",
                    line: i + 1,
                    excerpt: line.trim().to_string(),
                });
            }
        }
    }
    out.shrink_to_fit();
    out
}

/// P1 — workgroup-size literal restated in the body
/// (`share-cube-size-const`): `@workgroup_size(N)` with `N > 1` plus a body
/// `let BIND = N u?;` — the kernel uses that binding for stride
/// reconstruction, so the attribute and the literal form a silent contract
/// that drifts without an error when either side changes. The fix is one
/// shared `const`/`override` feeding both. Only the let-binding form fires:
/// bare arithmetic uses of `N` are too often tensor dims (precision-first).
fn scan_cube_size_restatement(source: &str) -> Vec<WgslFinding> {
    let attr = re_workgroup_size();
    // Collect the distinct x-dims > 1 declared in this file.
    let dims: Vec<u64> = {
        let mut v = Vec::new();
        for m in attr.captures_iter(source) {
            if let Some(x) = m.get(1).and_then(|g| g.as_str().parse::<u64>().ok()) {
                if x > 1 && !v.contains(&x) {
                    v.push(x);
                }
            }
        }
        v
    };
    if dims.is_empty() {
        return Vec::new();
    }
    let mut restated = Vec::with_capacity(dims.len());
    for n in &dims {
        let pat = format!(r"let\s+\w+\s*=\s*{n}u?\s*;");
        let re = Regex::new(&pat).expect("valid literal regex");
        for (i, raw) in source.lines().enumerate() {
            let line = strip_line_comment(raw);
            if re.is_match(&line) {
                restated.push(WgslFinding {
                    rule_id: "share-cube-size-const-between-kernel-and-launcher",
                    line: i + 1,
                    excerpt: line.trim().to_string(),
                });
            }
        }
    }
    restated.shrink_to_fit();
    restated
}

/// P4 — loop-invariant multiplicative operand in a dot-shaped accumulation
/// (`multi-head-out-proj-consumes-head-concat`): inside a `for` loop, an
/// accumulation `acc += LHS * RHS;` (or `acc = acc + LHS * RHS;`) where `LHS`
/// is a bare identifier (no subscript — the scalar that should have varied
/// over the summation index) and `RHS` indexes the loop's induction
/// variable. That is the `attn_val * rowsum(wo)` shape: a scalar times a
/// summed weight row is not the matvec, and identity-weight fixtures mask it.
///
/// Guards against the known legit look-alike (`acc += scale * x[k]`):
/// - `LHS` must not be a numeric literal (constant scaling is idiomatic);
/// - the summation loop must be NESTED inside another loop (the projection
///   shape is `d_out` × `kk` — an outer output dim and an inner summation).
///
/// A legit per-thread scale/mask variable is still textually identical —
/// the module table records this as the pattern's adjudicated imprecision
/// (materialized live by the `layer_fuse.wgsl` lerp, hence experimental).
fn scan_invariant_operand_accumulation(source: &str) -> Vec<WgslFinding> {
    static FOR_HEAD: OnceLock<Regex> = OnceLock::new();
    static ACC: OnceLock<Regex> = OnceLock::new();
    static PRODUCT: OnceLock<Regex> = OnceLock::new();
    let for_head =
        FOR_HEAD.get_or_init(|| Regex::new(r"for\s*\(\s*var\s+(\w+)\s*=").expect("valid regex"));
    let acc = ACC.get_or_init(|| {
        Regex::new(r"^\s*(?:var\s+)?(\w+)\s*(\+=|=)\s*(.*?);\s*(?://.*)?$").expect("valid regex")
    });
    let product = PRODUCT.get_or_init(|| Regex::new(r"^(\w+)\s*\*\s*(.+)$").expect("valid regex"));

    // Loop-var stack with brace depth bookkeeping (line-approximate: pushes
    // on a for-header line, pops when the closing brace returns to the push
    // depth). `if x { return; }` same-line pairs are net-zero, as intended.
    let mut stack: Vec<(u64, String)> = Vec::new(); // (depth AFTER the header line, loop var)
    let mut depth: u64 = 0;
    let mut out = Vec::new();
    for (i, raw) in source.lines().enumerate() {
        let line_no = i + 1;
        let code = strip_line_comment(raw);
        let before = depth;
        let opens = code.matches('{').count() as u64;
        let closes = code.matches('}').count() as u64;

        // Evaluate accumulation against the loops enclosing THIS line, using
        // the depth as of line entry (a for-header line is not inside itself).
        // The `stack.len() >= 2` arm is the NESTING guard: the projection
        // shape is an outer output dim × inner summation loop.
        if let (Some(inner), true) = (stack.last(), stack.len() >= 2) {
            if let Some(m) = acc.captures(&code) {
                let acc_var = m.get(1).map_or(String::new(), |g| g.as_str().to_string());
                let op = m.get(2).map_or("", |g| g.as_str());
                let mut rhs = m
                    .get(3)
                    .map_or(String::new(), |g| g.as_str().trim().to_string());
                if op == "=" {
                    // Normalize `acc = acc + <expr>` to `<expr>`; anything else
                    // (plain assignment) is not an accumulation.
                    let prefix = format!("{acc_var} + ");
                    if let Some(stripped) = rhs.strip_prefix(&prefix) {
                        rhs = stripped.trim().to_string();
                    } else {
                        rhs.clear();
                    }
                }
                if let Some(p) = product.captures(&rhs) {
                    let lhs = p.get(1).map_or("", |g| g.as_str());
                    let indexed = p.get(2).map_or("", |g| g.as_str());
                    let is_literal = lhs.parse::<f64>().is_ok();
                    let rhs_uses_inner = indexed.contains(&inner.1);
                    let rhs_indexed = indexed.contains('[');
                    // `lhs` is a bare identifier by the product regex (no
                    // subscript) — the loop-invariant operand.
                    if !lhs.is_empty() && !is_literal && rhs_indexed && rhs_uses_inner {
                        out.push(WgslFinding {
                            rule_id: "multi-head-out-proj-consumes-head-concat",
                            line: line_no,
                            excerpt: code.trim().to_string(),
                        });
                    }
                }
            }
        }

        // Then advance the loop stack with this line's braces.
        if let Some(m) = for_head.captures(&code) {
            let var = m.get(1).map_or(String::new(), |g| g.as_str().to_string());
            let after_header = before + opens - closes;
            stack.push((after_header, var));
        }
        depth += opens;
        depth = depth.saturating_sub(closes);
        while let Some(&(push_depth, _)) = stack.last() {
            if depth <= push_depth.saturating_sub(1) && depth < push_depth {
                stack.pop();
            } else {
                break;
            }
        }
    }
    out.shrink_to_fit();
    out
}

/// P7 — serial-stride dual-index dot (`row-dot-split-inner-axis-block-reduce`,
/// BODY form; the Issue 698 pre-fix `dot_per_row_f32` cube body — B44's own
/// mining source): an accumulation `acc = acc + x[base + j] * y[base2 + j]`
/// where BOTH operands are indexed and share the loop variable `j`, with
/// serial-stride evidence (`j += 1u` / `j = j + 1u`) within a ±8-line window
/// — one thread serially looping the whole row length. The fixed form strides
/// by the lane/plane width (`j += PLANE_DIM`, `j += cube_size`) and is silent.
///
/// EXPERIMENTAL (mining-only): 11 live instances sit under PARALLEL grids
/// (serial-K GEMV shapes — `spectralquant_maxsim`, `muon_fused`,
/// `eigenbasis_rotation`, …) where the fix regime (output count too small to
/// fill the machine) is a workload fact the text does not carry. In git-history
/// mining the DIFF adjudicates: the pattern firing pre-fix and silent post-fix
/// on a commit that removed the line is a validated pair.
fn scan_serial_row_dot(source: &str) -> Vec<WgslFinding> {
    static ACC: OnceLock<Regex> = OnceLock::new();
    static DUAL: OnceLock<Regex> = OnceLock::new();
    let acc = ACC.get_or_init(|| {
        Regex::new(r"^\s*(?:var\s+)?(\w+)\s*(\+=|=)\s*(.*?)\s*;\s*$").expect("valid regex")
    });
    let dual = DUAL.get_or_init(|| {
        Regex::new(
            r"^(\w+)\s*\[([^]]*)\]\s*(?:as\s+\w+\s*)?\*\s*(\w+)\s*\[([^]]*)\]\s*(?:as\s+\w+\s*)?$",
        )
        .expect("valid regex")
    });

    let lines: Vec<String> = source.lines().map(strip_line_comment).collect();
    let idents = |idx: &str| -> Vec<String> {
        idx.split(|c: char| !(c.is_alphanumeric() || c == '_'))
            .filter(|t| {
                t.chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
            })
            .map(str::to_string)
            .collect()
    };

    let mut out = Vec::new();
    for (i, code) in lines.iter().enumerate() {
        let Some(m) = acc.captures(code) else {
            continue;
        };
        let acc_var = m.get(1).map_or(String::new(), |g| g.as_str().to_string());
        let op = m.get(2).map_or("", |g| g.as_str());
        let mut rhs = m
            .get(3)
            .map_or(String::new(), |g| g.as_str().trim().to_string());
        if op == "=" {
            let prefix = format!("{acc_var} + ");
            if let Some(stripped) = rhs.strip_prefix(&prefix) {
                rhs = stripped.trim().to_string();
            } else {
                continue; // plain assignment, not an accumulation
            }
        }
        let Some(d) = dual.captures(&rhs) else {
            continue;
        };
        let (idx_a, idx_b) = (
            d.get(2).map_or("", |g| g.as_str()),
            d.get(4).map_or("", |g| g.as_str()),
        );
        let common: Vec<String> = idents(idx_a)
            .into_iter()
            .filter(|t| idents(idx_b).contains(t))
            .collect();
        if common.is_empty() {
            continue;
        }
        // Serial-stride evidence within ±8 lines: `j += 1u;` / `j += 1u32;`
        // (cube Rust) / `j = j + 1u)` (WGSL for-header).
        let lo = i.saturating_sub(8);
        let hi = (i + 9).min(lines.len());
        let window: String = lines[lo..hi].join("\n");
        for j in &common {
            let esc = regex::escape(j);
            let stride = Regex::new(&format!(
                r"{esc}\s*(?:\+=\s*1[a-z0-9]*|\s*=\s*{esc}\s*\+\s*1[a-z0-9]*)\s*[;)]"
            ))
            .expect("valid stride regex");
            if stride.is_match(&window) {
                out.push(WgslFinding {
                    rule_id: "row-dot-split-inner-axis-block-reduce",
                    line: i + 1,
                    excerpt: code.trim().to_string(),
                });
                break; // one finding per accumulation line
            }
        }
    }
    out.shrink_to_fit();
    out
}

/// Strip a trailing `//` line comment (WGSL block comments are rare on code
/// lines in these kernels; a `//` inside a string would be pathological in a
/// kernel file and is accepted as a limitation).
fn strip_line_comment(line: &str) -> String {
    match line.find("//") {
        Some(idx) => line[..idx].to_string(),
        None => line.to_string(),
    }
}

/// The WGSL surface scanner. Construct once, scan many files (regexes are
/// `OnceLock`-cached across instances). Default = the precision-vetted
/// pattern set only; [`Self::with_experimental`] adds the experimental set.
#[derive(Debug, Clone, Copy, Default)]
pub struct WgslScanner {
    include_experimental: bool,
}

impl WgslScanner {
    /// Scan one `.wgsl` source with the default (precision-vetted) pattern
    /// set. Output sorted by `(line, rule_id)` — deterministic.
    #[must_use]
    pub fn scan(&self, source: &str) -> Vec<WgslFinding> {
        let mut out = Vec::new();
        out.extend(scan_single_thread_workgroup(source));
        out.extend(scan_declared_max_private_array(source));
        out.extend(scan_cube_size_restatement(source));
        out.extend(scan_single_slot_staging(source));
        if self.include_experimental {
            out.extend(scan_invariant_operand_accumulation(source));
            out.extend(scan_serial_row_dot(source));
        }
        out.sort_by(|a, b| (a.line, a.rule_id).cmp(&(b.line, b.rule_id)));
        out
    }

    /// Enable the experimental pattern set (see [`WGSL_EXPERIMENTAL_RULES`])
    /// — for git-history mining where the loop-invariant-operand shape IS the
    /// bug, not for the live-tree suggest path.
    #[must_use]
    pub fn with_experimental(mut self) -> Self {
        self.include_experimental = true;
        self
    }

    /// Scan the `#[cube]` kernel bodies inside a Rust source (Issue 025 T3).
    ///
    /// Items are located by the [`crate::domains::kernel_opt::cube_extract`]
    /// token-tree walk (never parsed as Rust — `syn` refuses the `CubeCL`
    /// DSL), each item's text is scanned with the same pattern set as
    /// [`Self::scan`], and finding lines are mapped back to ABSOLUTE file
    /// lines. Same precision stance as the `.wgsl` surface: the current
    /// patterns are WGSL-syntax-shaped, so live cube bodies (Rust-shaped)
    /// fire nothing — the mapping is exercised by synthetic corpus text and
    /// the pipeline is the T6 mining substrate (cube-shaped patterns land
    /// with the mined buggy→fixed pairs).
    ///
    /// Limitation (same class as [`Self::scan`]): the pass is over raw span
    /// TEXT, so a WGSL-shaped snippet inside a cube-body string literal
    /// would fire — accepted, same as the `.wgsl` scanner's `//`-in-string
    /// limitation.
    #[must_use]
    pub fn scan_cube_rust(&self, rs_source: &str) -> Vec<WgslFinding> {
        let mut out = Vec::new();
        for item in super::cube_extract::extract_cube_items(rs_source) {
            for mut f in self.scan(&item.text) {
                f.line = item.start_line + f.line - 1;
                out.push(f);
            }
        }
        out.sort_by(|a, b| (a.line, a.rule_id).cmp(&(b.line, b.rule_id)));
        out.shrink_to_fit();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(found: &[WgslFinding]) -> Vec<&'static str> {
        found.iter().map(|f| f.rule_id).collect()
    }

    /// P2 — the real m2p header fires; healthy geometries do not.
    #[test]
    fn single_thread_workgroup_detected() {
        let buggy = "@compute @workgroup_size(1, 1, 1)\nfn main() { }";
        let found = WgslScanner::default().scan(buggy);
        assert!(rules(&found).contains(&"row-dot-split-inner-axis-block-reduce"));

        let healthy = "@compute @workgroup_size(16, 16, 1)\nfn main() { }";
        assert!(!rules(&WgslScanner::default().scan(healthy))
            .contains(&"row-dot-split-inner-axis-block-reduce"));

        let one_d = "@compute @workgroup_size(64)\nfn main() { }";
        assert!(!rules(&WgslScanner::default().scan(one_d))
            .contains(&"row-dot-split-inner-axis-block-reduce"));
    }

    /// P3 — the real m2p private arrays fire (128 + 256 are the corpus
    /// instances); workgroup arrays and small scratch do not.
    #[test]
    fn declared_max_private_array_detected() {
        let buggy = "fn k() {\n    var scores: array<f32, 256>;\n    var q: array<f32, 128>;\n}";
        let found = WgslScanner::default().scan(buggy);
        assert_eq!(
            rules(&found)
                .iter()
                .filter(|&&r| r == "declared-max-array-taxed-per-thread-under-runtime-bound")
                .count(),
            2
        );

        let shared = "var<workgroup> tile: array<f32, 256>;\nfn k() { }";
        assert!(!rules(&WgslScanner::default().scan(shared))
            .contains(&"declared-max-array-taxed-per-thread-under-runtime-bound"));

        let small = "fn k() { var buf: array<f32, 16>; }";
        assert!(!rules(&WgslScanner::default().scan(small))
            .contains(&"declared-max-array-taxed-per-thread-under-runtime-bound"));
    }

    /// P1 — the corpus let-binding restatement fires; a shared const and an
    /// unrelated literal do not.
    #[test]
    fn cube_size_restatement_detected() {
        let buggy = "@compute @workgroup_size(64)\nfn k() {\n    let cube_size = 64u;\n}";
        let found = WgslScanner::default().scan(buggy);
        assert!(rules(&found).contains(&"share-cube-size-const-between-kernel-and-launcher"));

        let fixed = "@compute @workgroup_size(64)\nfn k() {\n    let cube_size = CUBE_SIZE;\n}";
        assert!(!rules(&WgslScanner::default().scan(fixed))
            .contains(&"share-cube-size-const-between-kernel-and-launcher"));

        let unrelated = "@compute @workgroup_size(64)\nfn k() {\n    let dim = 128u;\n}";
        assert!(!rules(&WgslScanner::default().scan(unrelated))
            .contains(&"share-cube-size-const-between-kernel-and-launcher"));
    }

    /// P4 — the naive fused out-projection (the Issue 701 H-C shape) fires
    /// ONLY under `with_experimental`; the correct two-phase projection,
    /// both-sides-varying dots, and idiomatic constant scaling never fire;
    /// and the REAL false positive that demoted the pattern to experimental
    /// (the `layer_fuse.wgsl` alpha-lerp) stays silent in the DEFAULT scan.
    #[test]
    fn invariant_operand_out_projection_experimental() {
        // Naive: attn_val is invariant in kk — scalar times summed row.
        let naive = "fn proj(attn_val: f32, wo: array<f32>, h: u32, d: u32) -> f32 {\n    var acc: f32 = 0.0;\n    for (var d_out = 0u; d_out < h; d_out = d_out + 1u) {\n        for (var kk = 0u; kk < h; kk = kk + 1u) {\n            acc += attn_val * wo[d_out * h + kk];\n        }\n    }\n    return acc;\n}";
        let experimental = WgslScanner::default().with_experimental();
        let found = experimental.scan(naive);
        assert!(
            rules(&found).contains(&"multi-head-out-proj-consumes-head-concat"),
            "naive fused out-proj must fire under experimental, got {found:?}"
        );
        // Default scan: OFF (the pattern is not precision-vetted).
        assert!(!rules(&WgslScanner::default().scan(naive))
            .contains(&"multi-head-out-proj-consumes-head-concat"));

        // Correct: both operands indexed by the loop var (a real dot).
        let correct_dot = "fn dot(q: array<f32>, k: array<f32>, hd: u32) -> f32 {\n    var acc: f32 = 0.0;\n    for (var d = 0u; d < hd; d = d + 1u) {\n        for (var kk = 0u; kk < hd; kk = kk + 1u) {\n            acc += q[d] * k[d * hd + kk];\n        }\n    }\n    return acc;\n}";
        assert!(!rules(&experimental.scan(correct_dot))
            .contains(&"multi-head-out-proj-consumes-head-concat"));

        // Idiomatic constant scaling must NOT fire (numeric literal LHS).
        let const_scale = "fn s(x: array<f32>, n: u32) -> f32 {\n    var acc: f32 = 0.0;\n    for (var d = 0u; d < n; d = d + 1u) {\n        for (var k = 0u; k < n; k = k + 1u) {\n            acc += 2.0 * x[d * n + k];\n        }\n    }\n    return acc;\n}";
        assert!(!rules(&experimental.scan(const_scale))
            .contains(&"multi-head-out-proj-consumes-head-concat"));

        // The correct two-phase out-projection: full-row matvec, both vary.
        let two_phase = "fn out_proj(attn: array<f32>, wo: array<f32>, h: u32) -> f32 {\n    var sum: f32 = 0.0;\n    for (var d = 0u; d < h; d = d + 1u) {\n        for (var k = 0u; k < h; k = k + 1u) {\n            sum = sum + attn[k] * wo[d * h + k];\n        }\n    }\n    return sum;\n}";
        assert!(!rules(&experimental.scan(two_phase))
            .contains(&"multi-head-out-proj-consumes-head-concat"));

        // THE demotion evidence: the real layer_fuse lerp line — a legitimate
        // alpha-interpolation, textually identical to the bug — must stay
        // silent in the DEFAULT scan (it fires under experimental, which is
        // exactly why the pattern is not precision-vetted).
        let lerp = "fn fuse(alpha: f32, target: array<f32>, p: u32, j: u32, dim: u32) -> f32 {\n    var val: f32 = 0.0;\n    for (var i = 0u; i < p; i = i + 1u) {\n        for (var jj = 0u; jj < j; jj = jj + 1u) {\n            val = val + alpha * target[p * j + jj + dim];\n        }\n    }\n    return val;\n}";
        assert!(!rules(&WgslScanner::default().scan(lerp))
            .contains(&"multi-head-out-proj-consumes-head-concat"));
    }

    /// P6 — single-slot staging (the Issue 686 H7 fix shape): the bare-load
    /// write + single-bare-ident read form fires; the computed-write staging
    /// Computed-write staging (the live `role_transport.wgsl`), tree
    /// reductions (multi-form reads), compound
    /// transpose indexes, loop-varying indexes, and non-bare-load writes are
    /// all silent.
    #[test]
    fn single_slot_staging_detected() {
        // The 686-pre / gram shape: bare pass-through load + single-slot reads.
        let buggy = "var<workgroup> s_row: array<f32, 16>;
fn covariance() {
    s_row[local_row] = input_kv[pos_base + global_row];
    accum += s_row[local_row] * s_col[local_col];
}";
        let found = WgslScanner::default().scan(buggy);
        assert!(
            rules(&found).contains(&"smem-staging-gated-by-cross-thread-reuse"),
            "the 686-pre shape must fire, got {found:?}"
        );

        // The 686-post form: direct global reads (no workgroup staging).
        let fixed = "fn covariance() {
    accum += input_kv[pos_base + global_row] * input_kv[pos_base + global_col];
}";
        assert!(!rules(&WgslScanner::default().scan(fixed))
            .contains(&"smem-staging-gated-by-cross-thread-reuse"));

        // Computed-write staging (the LIVE role_transport.wgsl): staging a
        // value computed once + reused by 16 threads IS reuse > 1 — silent.
        let computed = "var<workgroup> s_row: array<f32, 16>;
fn k() {
    s_row[local_row] = val * role_emb[role_offset + global_row];
    accum += s_row[local_row] * s_col[local_col];
}";
        assert!(!rules(&WgslScanner::default().scan(computed))
            .contains(&"smem-staging-gated-by-cross-thread-reuse"));

        // Tree reduction: reads at [t] AND [t + stride] — multi-form — silent.
        let tree = "var<workgroup> row_sq: array<f32, 16>;
fn k() {
    row_sq[local_id] = partial_sq;
    row_sq[local_id] = row_sq[local_id] + row_sq[local_id + stride];
}";
        assert!(!rules(&WgslScanner::default().scan(tree))
            .contains(&"smem-staging-gated-by-cross-thread-reuse"));

        // Compound transpose index: unique slot per thread — the staging
        // exists to coalesce — silent.
        let transpose = "var<workgroup> tile: array<f32, 256>;
fn k() {
    tile[local_row * 16u + local_col] = input[global_row * 16u + local_col];
    output[global_col * 16u + global_row] = tile[local_col * 16u + local_row];
}";
        assert!(!rules(&WgslScanner::default().scan(transpose))
            .contains(&"smem-staging-gated-by-cross-thread-reuse"));

        // Loop-varying read index (the thread reads MANY slots) — silent.
        let loop_var = "var<workgroup> buf: array<f32, 64>;
fn k() {
    buf[lane] = src[lane];
    for (var i = 0u; i < 64u; i = i + 1u) {
        sum += buf[i];
    }
}";
        assert!(!rules(&WgslScanner::default().scan(loop_var))
            .contains(&"smem-staging-gated-by-cross-thread-reuse"));

        // Non-bare-load write (computed RHS via function call) — silent.
        let fn_write = "var<workgroup> tile: array<f32, 16>;
fn k() {
    tile[col] = f32(row);
    out[row * 16u + col] = tile[col] * 2.0;
}";
        assert!(!rules(&WgslScanner::default().scan(fn_write))
            .contains(&"smem-staging-gated-by-cross-thread-reuse"));
    }

    /// P7 — serial-stride dual-index dot (the Issue 698 pre-fix cube body,
    /// B44 body form): fires ONLY under `with_experimental`; the strided
    /// (plane-parallel) fixed form and single-index operands stay silent.
    #[test]
    fn serial_row_dot_experimental() {
        // The 698-pre shape: serial stride over the full row length.
        let buggy = "fn dot_per_row(u: &[f32], g: &[f32], h: u32) {
    let mut acc = f32::new(0.0f32);
    let mut j = 0u32;
    while j < h {
        acc = acc + u[(base + j) as usize] * g[(g_base + j) as usize];
        j += 1u32;
    }
}";
        let experimental = WgslScanner::default().with_experimental();
        let found = experimental.scan(buggy);
        assert!(
            rules(&found).contains(&"row-dot-split-inner-axis-block-reduce"),
            "the 698-pre serial dot must fire under experimental, got {found:?}"
        );
        // Default scan: OFF (serial-K GEMV ambiguity — mining-only).
        assert!(!rules(&WgslScanner::default().scan(buggy))
            .contains(&"row-dot-split-inner-axis-block-reduce"));

        // The 698-post form: lane-strided (PLANE_DIM) — silent.
        let fixed = "fn dot_per_row(u: &[f32], g: &[f32], h: u32) {
    let mut acc = f32::new(0.0f32);
    let mut j = lane;
    while j < h {
        acc = acc + u[(base + j) as usize] * g[(g_base + j) as usize];
        j += PLANE_DIM;
    }
}";
        assert!(
            !rules(&experimental.scan(fixed)).contains(&"row-dot-split-inner-axis-block-reduce")
        );

        // WGSL for-header serial stride — fires (the m2p phase-1 shape).
        let wgsl_buggy = "fn qkv(input_seq: array<f32>, w: array<f32>, h: u32) -> f32 {
    var sum: f32 = 0.0;
    for (var k = 0u; k < h; k = k + 1u) {
        sum = sum + input_seq[pos * h + k] * w[off * h + k];
    }
    return sum;
}";
        assert!(rules(&experimental.scan(wgsl_buggy))
            .contains(&"row-dot-split-inner-axis-block-reduce"));
    }

    /// Determinism + a clean control: a healthy tiled kernel fires nothing.
    #[test]
    fn clean_kernel_fires_nothing_and_scan_is_deterministic() {
        let clean = "@compute @workgroup_size(16, 16, 1)\nfn transpose(@builtin(global_invocation_id) gid: vec3<u32>) {\n    let row = gid.y;\n    let col = gid.x;\n    if (row >= 16u || col >= 16u) { return; }\n    var tile: array<f32, 16>;\n    tile[col] = f32(row);\n    out[row * 16u + col] = tile[col] * 2.0;\n}";
        let s = WgslScanner::default();
        assert!(s.scan(clean).is_empty(), "clean kernel must fire nothing");
        assert_eq!(s.scan(clean), s.scan(clean));
    }

    /// The pattern sets cite only corpus rule ids (the citation contract).
    #[test]
    fn every_pattern_cites_a_corpus_rule() {
        let corpus_ids: Vec<String> =
            crate::domains::kernel_opt::corpus::build_kernel_seed_corpus()
                .entries
                .iter()
                .map(|e| e.rule_id.clone())
                .collect();
        for id in WGSL_PATTERN_RULES.iter().chain(WGSL_EXPERIMENTAL_RULES) {
            assert!(
                corpus_ids.iter().any(|c| c == id),
                "pattern cites non-corpus rule {id}"
            );
        }
    }

    /// T3 — cube-body findings map back to ABSOLUTE file lines, and
    /// comment/attr noise around the item never fires.
    #[test]
    fn cube_rust_scan_maps_lines_to_absolute_file_lines() {
        // A cube body carrying a WGSL-shaped declared-max line (synthetic:
        // real cube bodies are Rust-shaped and fire nothing — this pins the
        // mapping, not the live tree).
        let src = "// `#[cube]` mention in a comment\n#[cfg(feature = \"cubecl_runtime\")]\n#[cube(launch_unchecked)]\nfn k(input: &[f32]) {\n    var buf: array<f32, 256>;\n}\n";
        let found = WgslScanner::default().scan_cube_rust(src);
        assert_eq!(found.len(), 1, "got {found:?}");
        assert_eq!(
            found[0].rule_id,
            "declared-max-array-taxed-per-thread-under-runtime-bound"
        );
        // The body line is file line 5 (attr at 3, body decl at 5).
        assert_eq!(found[0].line, 5);
        assert!(found[0].excerpt.contains("array<f32, 256>"));

        // Determinism.
        assert_eq!(found, WgslScanner::default().scan_cube_rust(src));
    }

    /// G2 (Bench 033) — the per-file scan budget: < 1 ms/file on the largest
    /// committed fixture (201 lines, the post-fix m2p control), amortized
    /// over 200 scans. Release-only per the repo's G2 convention (debug
    /// regex timing is meaningless for the budget).
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "release-only: debug regex timing is meaningless for the budget gate"
    )]
    #[allow(clippy::cast_precision_loss)]
    fn g2_scan_latency_under_1ms_per_file() {
        const FIXTURE: &str =
            include_str!("../../../tests/fixtures/m2p_attention_postfix_701.wgsl");
        let s = WgslScanner::default();
        let n = 200usize;
        let t = std::time::Instant::now();
        for _ in 0..n {
            let _ = s.scan(FIXTURE);
        }
        let per_file = t.elapsed().as_secs_f64() * 1_000.0 / n as f64;
        assert!(
            per_file < 1000.0,
            "scan budget exceeded: {per_file:.1} µs/file (target < 1000)"
        );
    }
}
