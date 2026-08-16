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
//! | `smem-staging-gated-by-cross-thread-reuse` (B35) | — needs within-workgroup **reuse counts** (dataflow), not text | ❌ not regex-expressible in v1 |
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
];

/// The experimental pattern set — implemented and tested, but OFF by
/// default because the T2 calibration smoke caught its only live firing on
/// a legitimate `alpha`-lerp (`layer_fuse.wgsl` L61). Enabled via
/// [`WgslScanner::with_experimental`] for git-history mining where the
/// loop-invariant-operand shape IS the bug (the Issue 701 pre-fix m2p).
pub const WGSL_EXPERIMENTAL_RULES: &[&str] = &["multi-head-out-proj-consumes-head-concat"];

/// Private-array declared-size threshold: the floor of observed corpus
/// instances (`array<f32, 128>`, `array<f32, 256>` in `m2p_attention.wgsl`).
/// Smaller private arrays are conventional scratch, not the declared-max tax.
const PRIVATE_ARRAY_FLOOR: u64 = 128;

fn re_workgroup_size() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"@workgroup_size\(\s*(\d+)\s*(?:,\s*\d+\s*){0,2}\)").expect("valid regex"))
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
    let mut restated = Vec::new();
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
    let acc = ACC.get_or_init(|| Regex::new(r"^\s*(?:var\s+)?(\w+)\s*(\+=|=)\s*(.*?);\s*(?://.*)?$").expect("valid regex"));
    let product =
        PRODUCT.get_or_init(|| Regex::new(r"^(\w+)\s*\*\s*(.+)$").expect("valid regex"));

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
                let mut rhs = m.get(3).map_or(String::new(), |g| g.as_str().trim().to_string());
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
        if self.include_experimental {
            out.extend(scan_invariant_operand_accumulation(source));
        }
        out.sort_by(|a, b| (a.line, a.rule_id).cmp(&(b.line, b.rule_id)));
        out
    }

    /// Enable the experimental pattern set (see [`WGSL_EXPERIMENTAL_RULES`])
    /// — for git-history mining where the loop-invariant-operand shape IS
    /// the bug, not for the live-tree suggest path.
    #[must_use]
    pub fn with_experimental(mut self) -> Self {
        self.include_experimental = true;
        self
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
        let corpus_ids: Vec<String> = crate::domains::kernel_opt::corpus::build_kernel_seed_corpus()
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
}
