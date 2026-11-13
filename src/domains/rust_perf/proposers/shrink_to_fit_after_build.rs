//! `shrink-to-fit-after-build` — release a Vec's over-allocated capacity once
//! the build is done (insert `v.shrink_to_fit();` before the tail).
//!
//! Pattern: a `let mut v = Vec::new()` / `Vec::with_capacity(hint)` binding
//! whose ident is the fn block's tail expression, with at least one loop
//! pushing onto `v` in between. Growth doubling over-allocates up to ~2x for
//! runtime-driven push loops, and a guessed `with_capacity` hint wastes
//! whatever the real count undershoots. One inserted call releases the excess
//! for long-lived vectors. The rule's win is PEAK MEMORY, not speed (the
//! corpus note: the realloc can even cost a little time on a hot build path)
//! — Plan 049 T0.0c ships the proposer because the headroom instrument
//! (Bench 043 decomposition) showed the rule's eval span ranked in-pool with
//! no edit to apply.
//!
//! ## The exact-capacity complement (mutual exclusion with `vec-with-capacity`)
//!
//! When the capacity is DERIVABLE at the binding, the better fix is
//! `Vec::with_capacity(n)` — no growth, no waste, no extra call — and that is
//! [`super::vec_with_capacity`]'s rewrite. This proposer therefore runs the
//! vec-with-capacity probe over the same file (same `ProposerOpts`, so
//! verify-mode widens both together) and stays SILENT on any binding it would
//! rewrite. One further exact-hint guard covers the shape vec-with-capacity
//! cannot touch (its rewrite replaces the `new()` init, not a
//! `with_capacity` one):
//!
//! - `with_capacity(ARG)` + a push loop whose derivable bound TEXT equals
//!   `ARG` is exact — skip. (`with_capacity(count)` + `for i in 0..count`;
//!   `with_capacity(items.len())` + `for x in &items`.)
//! - `with_capacity(IDENT)` + a pushing loop whose HEADER names that ident
//!   whole-word (`with_capacity(steps)` + `while i < steps`) is exact by the
//!   same reading — skip (the P121 intake class: while bounds carry no
//!   derivable-capacity text, so the text-equality guard above cannot see
//!   them; the word gate does). Superstring idents (`steps_total`) are a
//!   different bound and keep the fire.
//! - `with_capacity(m * n)` + a PROVEN exact-fill push nest (`for 0..m {`
//!   `for 0..n { push } }`) is EXACT-FILL by construction — the factor
//!   multiset equals the chain's bound multiset, so `len == capacity` on
//!   every path and the inserted call is a dead no-op (the I236 wrong-heal
//!   class: six dead heals on riir-infer's `int_ref` lane, all reverted by
//!   hand). The proof bar is strict: one pushing chain, every non-leaf
//!   level pushes only through its unique UNCONDITIONAL child (top-level
//!   stmt), the leaf carries exactly one unconditional top-level push, no
//!   early exit (`break`/`continue`/`return`/`?`) anywhere in the chain,
//!   no shadow re-let, no other len-mutator on the ident in or beside the
//!   chain, and every level's bound derivable (an inclusive range
//!   `0..=n` or a non-zero start `2..n` is unresolvable — the count is
//!   not the bound text). Anything unprovable keeps the fire: a
//!   mismatched product (`n * 2` + one loop) is the shipped
//!   over-provisioned win, and the miss costs nothing while the wrong
//!   heal costs a revert.
//! - `with_capacity(count * 2)` + `0..count` (over-provisioned) or
//!   `with_capacity(4096)` + a data-driven loop (a guess) DO fire — the
//!   excess is real.
//!
//! `while`/`loop` push bodies and unknown-iterator for loops (`.enumerate()`,
//! custom iterators) are exactly the regime vec-with-capacity declines — this
//! proposer is the fallback there.
//!
//! ## Scope + safety gates (parse-only)
//!
//! - Fires only on fn-body blocks (v1); inner blocks / macro bodies are out.
//! - The tail must be the BARE binding ident (no `.clone()`, no method tail).
//! - Exactly one `let mut` binding of the ident in the block (a second is
//!   shadowing ambiguity — skip); the initializer must be a `Vec::new` /
//!   `Vec::with_capacity` call (receiver type proven by the constructor, not
//!   by inference).
//! - At least one push onto the ident from inside a loop body whose scan did
//!   not re-bind the ident (straight-line pushes on a compile-known count are
//!   the LOSS regime — the realloc buys nothing).
//! - `shrink_to_fit` already present in the block → skip (idempotence).
//! - Nested `fn` items are excluded from the push scan (their locals are not
//!   this block's binding — the `for_each_fn` nested-fn lesson).
//! - The edit is a zero-width insertion at the tail's start: everything
//!   outside the inserted `v.shrink_to_fit();` + newline + indent stays
//!   byte-identical. No signature change, no type ripple — the L2 parse
//!   gate, the `--verify` compile gate, and `rust_perf_verify` validate
//!   semantics.

use super::{ProposerOpts, SourceEdit, line_indent, src};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{BinOp, Block, Expr, File, Ident, Lit, Local, Pat, RangeLimits, Stmt, UnOp};

/// Byte range of any node's span.
fn byte_range<T: Spanned>(t: &T) -> std::ops::Range<usize> {
    t.span().byte_range()
}

pub(crate) fn collect_edits_opts(source: &str, file: &File, opts: ProposerOpts) -> Vec<SourceEdit> {
    // Bindings the exact-capacity rewrite already covers: shrink stays
    // silent there (`Vec::with_capacity(n)` is the strictly better fix, and
    // applying both would compose into a redundant no-op call).
    let covered: Vec<std::ops::Range<usize>> =
        super::vec_with_capacity::collect_edits_opts(source, file, opts)
            .into_iter()
            .map(|e| e.start..e.end)
            .collect();
    let mut edits = Vec::new();
    super::for_each_fn(file, |_sig, block| {
        collect_block_edits(source, block, &covered, &mut edits);
    });
    edits
}

/// The Vec ctor proven by the binding's initializer expression.
struct CtorInfo<'a> {
    init_range: std::ops::Range<usize>,
    /// `Some` for `Vec::with_capacity(arg)` — the hint argument.
    cap_arg: Option<&'a Expr>,
}

fn vec_ctor(init: &Expr) -> Option<CtorInfo<'_>> {
    let Expr::Call(call) = init else { return None };
    let Expr::Path(p) = call.func.as_ref() else {
        return None;
    };
    let segs: Vec<_> = p.path.segments.iter().collect();
    if segs.len() < 2 || segs[segs.len() - 2].ident != "Vec" {
        return None;
    }
    match segs[segs.len() - 1].ident.to_string().as_str() {
        "new" => Some(CtorInfo {
            init_range: byte_range(init),
            cap_arg: None,
        }),
        "with_capacity" => {
            if call.args.len() != 1 {
                return None;
            }
            Some(CtorInfo {
                init_range: byte_range(init),
                cap_arg: Some(call.args.first().unwrap()),
            })
        }
        _ => None,
    }
}

fn local_bound_ident(local: &Local) -> Option<(Ident, bool)> {
    let pat = match &local.pat {
        Pat::Ident(pi) => pi,
        Pat::Type(pt) => match &*pt.pat {
            Pat::Ident(pi) => pi,
            _ => return None,
        },
        _ => return None,
    };
    Some((pat.ident.clone(), pat.mutability.is_some()))
}

/// G2 — the single `let mut` Vec-ctor binding of `tail_ident` in the block:
/// `Some((stmt index, ctor info))`. `None` — the refusal cases included —
/// when the ident is bound twice (shadowing ambiguity), bound immutably
/// (`shrink_to_fit` needs `&mut`), or bound to something other than a
/// `Vec::new` / `Vec::with_capacity` call this rule proves.
fn find_vec_binding<'a>(stmts: &'a [Stmt], tail_ident: &Ident) -> Option<(usize, CtorInfo<'a>)> {
    let mut binding: Option<(usize, CtorInfo<'a>)> = None;
    for (i, stmt) in stmts.iter().enumerate() {
        let Stmt::Local(local) = stmt else { continue };
        let Some((id, is_mut)) = local_bound_ident(local) else {
            continue;
        };
        if id != *tail_ident {
            continue;
        }
        if binding.is_some() {
            return None; // a second binding of the same ident = shadowing ambiguity
        }
        if !is_mut {
            return None; // shrink_to_fit needs &mut
        }
        let init = local.init.as_ref().map(|i| &*i.expr)?;
        let info = vec_ctor(init)?; // bound, but not a Vec ctor this rule proves
        binding = Some((i, info));
    }
    binding
}

fn collect_block_edits(
    source: &str,
    block: &Block,
    covered: &[std::ops::Range<usize>],
    edits: &mut Vec<SourceEdit>,
) {
    let stmts = &block.stmts;
    // G1 — the tail is the bare binding ident.
    let Some(Stmt::Expr(tail, None)) = stmts.last() else {
        return;
    };
    let Expr::Path(tail_path) = tail else { return };
    let Some(tail_ident) = tail_path.path.get_ident() else {
        return;
    };

    // G2 — exactly one `let mut` Vec-ctor binding of that ident in the block.
    let Some((bind_idx, info)) = find_vec_binding(stmts, tail_ident) else {
        return;
    };

    // Exact-capacity probe: vec-with-capacity rewrites this binding → skip.
    // Independently, a len-convention push loop (`for &x in items`,
    // `items.iter()`, …) makes the binding exact-capacity-coverable by the
    // SAME rule — `.len()` is unconditionally usize (the std convention, no
    // type gate needed, unlike range bounds) — so the with_capacity fix
    // applies conceptually and shrink stays silent there too. Range-bound
    // loops keep ONLY the vwc-overlap gate: when vwc declines the range (a
    // u8 bound fails its usize gate), shrink is the correct fallback.
    if covered
        .iter()
        .any(|r| info.init_range.start < r.end && r.start < info.init_range.end)
    {
        return;
    }

    // G3 — at least one loop pushing onto the ident (runtime-driven growth).
    let mut scan = LoopPushScan {
        source,
        ident: tail_ident,
        loops: Vec::new(),
    };
    for stmt in &stmts[bind_idx + 1..] {
        if matches!(stmt, Stmt::Item(_)) {
            continue; // nested fn items: their locals are not this binding
        }
        scan.visit_stmt(stmt);
    }
    if scan.loops.is_empty() {
        return; // no push loop — straight-line builds are the loss regime
    }
    let cap_texts: Vec<Option<(String, bool)>> =
        scan.loops.iter().map(|l| l.cap_text.clone()).collect();
    let header_texts: Vec<String> = scan.loops.iter().map(|l| l.header.clone()).collect();

    // Len-convention delegation: a push loop whose iterable is a
    // len-convention shape (`for &x in items`, `items.iter()`, …) makes the
    // binding exact-capacity-coverable by the vec-with-capacity RULE —
    // `.len()` is unconditionally usize (the std convention, no type gate
    // needed, unlike range bounds) — so the with_capacity fix applies
    // conceptually and shrink stays silent for a `Vec::new()` binding there
    // too. Range-bound loops keep ONLY the vwc-overlap gate: when vwc
    // declines the range (a u8 bound fails its usize gate), shrink is the
    // correct fallback.
    if info.cap_arg.is_none() && cap_texts.iter().any(|t| matches!(t, Some((_, true)))) {
        return;
    }

    // G4 — idempotence: the block already carries the call.
    let br = byte_range(block);
    let Some(block_text) = source.get(br.start..br.end) else {
        return;
    };
    if block_text.contains("shrink_to_fit") {
        return;
    }

    // G5 — an exact with_capacity hint (text-equal to a push loop's derivable
    // bound) leaves nothing to release.
    if let Some(arg) = info.cap_arg {
        let Some(arg_text) = src(source, arg) else {
            return;
        };
        if cap_texts
            .iter()
            .any(|t| matches!(t, Some((s, _)) if *s == arg_text))
        {
            return;
        }
    }

    // G5b — a BARE-IDENT hint whose name bounds a pushing loop's HEADER is
    // exact by the same reading G5 makes textual: `with_capacity(steps)` +
    // `while i < steps` receives exactly `steps` pushes (the P121 misfire
    // class — a while bound has no derivable-capacity text, so G5's
    // text-equality cannot see it, and the inserted call is a provable
    // no-op). Whole-word match only: `steps_total` is a different bound and
    // keeps the fire. Expression hints (`n * 2`) are untouched — the
    // over-provisioned arm still fires.
    if let Some(Expr::Path(hint_path)) = info.cap_arg {
        if let Some(hint) = hint_path.path.get_ident() {
            let hint = hint.to_string();
            let bounded = header_texts.iter().any(|h| {
                h.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|w| w == hint)
            });
            if bounded {
                return;
            }
        }
    }

    // G5c — a pure-product hint (`m * n`) over a PROVEN exact-fill push
    // chain (`for 0..m { for 0..n { push } }`) leaves nothing to release:
    // the chain provably receives exactly the product of its bounds, so
    // `len == capacity` on every path and the call would be a dead no-op
    // (the I236 wrong-heal class). The proof bar is strict — see the
    // module doc — and anything unprovable keeps the incumbent fire:
    // `n * 2` + one loop is the shipped over-provisioned arm.
    if product_hint_is_exact_fill(
        &scan,
        info.cap_arg,
        source,
        tail_ident,
        &stmts[bind_idx + 1..],
    ) {
        return;
    }

    // Zero-width insertion at the tail start; the tail keeps its own line.
    let tail_start = byte_range(tail).start;
    let indent = line_indent(source, tail_start);
    edits.push(SourceEdit {
        start: tail_start,
        end: tail_start,
        replacement: format!("{tail_ident}.shrink_to_fit();\n{indent}"),
    });
}

/// The derivable-capacity TEXT of a for-loop iterable, plus whether the
/// shape is a len-convention form (`.len()` unconditionally usize — safe to
/// delegate without a type gate). `None` for while/loop bodies and unknown
/// iterators (exactly the shrink regime). Range bounds are NOT
/// len-convention: their type is unknown to a parse-only scan (the u8-bound
/// case where vec-with-capacity declines and shrink must fire). An INCLUSIVE
/// range end (`0..=n` iterates n+1 times) and a non-zero literal start
/// (`2..n` iterates end-start times) are also `None` — the iteration count
/// is not the bound text, and the exact-fill guards must match provable
/// counts only (I236; the fire is the safe default for unresolvable bounds).
fn derivable_capacity_text(source: &str, iterable: &Expr) -> Option<(String, bool)> {
    match iterable {
        Expr::Range(r) => {
            if matches!(r.limits, RangeLimits::Closed(_)) {
                return None;
            }
            let non_zero_start = match r.start.as_deref() {
                None => false,
                Some(Expr::Lit(l)) => !matches!(&l.lit, Lit::Int(i) if i.to_string() == "0"),
                Some(_) => true,
            };
            if non_zero_start {
                return None;
            }
            r.end
                .as_ref()
                .and_then(|e| src(source, e))
                .map(|s| (s, false))
        }
        Expr::Unary(u) if matches!(u.op, UnOp::Deref(_)) => match &*u.expr {
            Expr::Path(p) => src(source, p).map(|s| (format!("{s}.len()"), true)),
            _ => None,
        },
        Expr::Path(_) => src(source, iterable).map(|s| (format!("{s}.len()"), true)),
        Expr::MethodCall(m) => {
            if matches!(
                m.method.to_string().as_str(),
                "iter" | "iter_mut" | "into_iter"
            ) && matches!(&*m.receiver, Expr::Path(_))
            {
                return src(source, &m.receiver).map(|s| (format!("{s}.len()"), true));
            }
            None
        }
        _ => None,
    }
}

/// One pushing loop discovered in the block: its derivable bound (if any),
/// header text, spans, and its body — the raw material for the G5/G5b text
/// matches and the G5c exact-fill chain proof.
struct PushingLoop<'a> {
    cap_text: Option<(String, bool)>,
    header: String,
    expr_span: std::ops::Range<usize>,
    body_span: std::ops::Range<usize>,
    body: &'a Block,
}

/// Collects every loop (for/while/loop) whose body pushes onto `ident`,
/// nested or not — nested pushing loops are recorded too (the outer loop
/// pushes through them). `None` `cap_text` entries = while/loop (no derivable
/// bound — exactly the regime the shrink fix is for).
struct LoopPushScan<'a, 'b> {
    source: &'b str,
    ident: &'b Ident,
    loops: Vec<PushingLoop<'a>>,
}

impl<'a> Visit<'a> for LoopPushScan<'a, '_> {
    fn visit_expr(&mut self, e: &'a Expr) {
        let (body, iterable, cond): (Option<&Block>, Option<&Expr>, Option<&Expr>) = match e {
            Expr::ForLoop(f) => (Some(&f.body), Some(&f.expr), None),
            Expr::While(w) => (Some(&w.body), None, Some(&w.cond)),
            Expr::Loop(l) => (Some(&l.body), None, None),
            _ => (None, None, None),
        };
        if let Some(body) = body {
            let mut probe = PushProbe {
                ident: self.ident,
                shadowed: false,
                found: false,
            };
            probe.visit_block(body);
            if probe.found {
                self.loops.push(PushingLoop {
                    cap_text: iterable.and_then(|it| derivable_capacity_text(self.source, it)),
                    header: iterable
                        .and_then(|it| src(self.source, it))
                        .or_else(|| cond.and_then(|c| src(self.source, c)))
                        .unwrap_or_default(),
                    expr_span: byte_range(e),
                    body_span: byte_range(body),
                    body,
                });
            }
        }
        visit::visit_expr(self, e);
    }
}

/// Does this loop body push onto `ident`? A re-`let` of the ident inside the
/// body shadows the binding — pushes after it belong to the shadow, so the
/// probe stops crediting pushes once the shadow appears.
struct PushProbe<'a> {
    ident: &'a Ident,
    shadowed: bool,
    found: bool,
}

impl<'a> Visit<'a> for PushProbe<'a> {
    fn visit_local(&mut self, i: &'a Local) {
        if let Some((id, _)) = local_bound_ident(i) {
            if id == *self.ident {
                self.shadowed = true;
            }
        }
        visit::visit_local(self, i);
    }

    fn visit_expr(&mut self, e: &'a Expr) {
        if let Expr::MethodCall(m) = e {
            if !self.shadowed && m.method == "push" {
                if let Expr::Path(p) = &*m.receiver {
                    if p.path.get_ident().is_some_and(|i| i == self.ident) {
                        self.found = true;
                    }
                }
            }
        }
        visit::visit_expr(self, e);
    }
}

// ── G5c — the exact-fill proof helpers (I236) ────────────────────────────

/// Len-mutating methods on the binding ident that break an exact-fill
/// count: any of these in or beside the push chain means the count is not
/// the product of the loop bounds (`extend`/`insert` add, `clear`/
/// `truncate` remove — either way the proof is gone).
const LEN_MUTATORS: &[&str] = &[
    "push",
    "insert",
    "append",
    "extend",
    "extend_from_slice",
    "resize",
    "resize_with",
    "splice",
    "clear",
    "truncate",
    "drain",
];

/// Whitespace-insensitive text form used for factor/bound multiset
/// comparison (`m * n` vs `m*n` spellings must match).
fn normalize_text(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Factor texts of a pure multiplication chain (paren/group unwrapped),
/// normalized. `None` when the hint is anything else — a bare ident, a
/// literal, a call, an addition — which is the incumbent arms' regime
/// (G5/G5b text matches, the over-provisioned fire).
fn product_factors(source: &str, arg: Option<&Expr>) -> Option<Vec<String>> {
    let mut out = Vec::new();
    if !mul_walk(source, arg?, &mut out) {
        return None;
    }
    Some(out)
}

fn mul_walk(source: &str, e: &Expr, out: &mut Vec<String>) -> bool {
    let e = match e {
        Expr::Paren(p) => &p.expr,
        Expr::Group(g) => &g.expr,
        other => other,
    };
    match e {
        Expr::Binary(b) if matches!(b.op, BinOp::Mul(_)) => {
            mul_walk(source, &b.left, out) && mul_walk(source, &b.right, out)
        }
        other => match src(source, other) {
            Some(t) => {
                out.push(normalize_text(&t));
                true
            }
            None => false,
        },
    }
}

fn multiset_eq(a: &[String], b: &[String]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut x: Vec<_> = a.iter().collect();
    let mut y: Vec<_> = b.iter().collect();
    x.sort_unstable();
    y.sort_unstable();
    x == y
}

/// `true` when `l` is not nested inside another pushing loop's body.
fn is_root_loop(l: &PushingLoop, loops: &[PushingLoop]) -> bool {
    !loops
        .iter()
        .any(|o| o.body_span.start <= l.expr_span.start && l.expr_span.end <= o.body_span.end)
}

/// Mutation probe over a body that skips whole sub-trees by exact span
/// (the nested pushing loops are the chain's children, not this level's
/// own pushes). Records push spans, other len-mutator calls, early exits,
/// and shadow re-lets of the ident.
struct MutationScan<'i> {
    ident: &'i Ident,
    skips: Vec<std::ops::Range<usize>>,
    pushes: Vec<std::ops::Range<usize>>,
    other_mutations: bool,
    early_exits: bool,
    shadowed: bool,
}

impl<'i> MutationScan<'i> {
    fn new(ident: &'i Ident, skips: Vec<std::ops::Range<usize>>) -> Self {
        Self {
            ident,
            skips,
            pushes: Vec::new(),
            other_mutations: false,
            early_exits: false,
            shadowed: false,
        }
    }
}

impl<'a> Visit<'a> for MutationScan<'_> {
    fn visit_local(&mut self, i: &'a Local) {
        if let Some((id, _)) = local_bound_ident(i) {
            if id == *self.ident {
                self.shadowed = true;
            }
        }
        visit::visit_local(self, i);
    }

    fn visit_expr(&mut self, e: &'a Expr) {
        let r = byte_range(e);
        if self.skips.contains(&r) {
            return; // a chain child — its internals are the child level's
        }
        match e {
            Expr::Break(_) | Expr::Continue(_) | Expr::Return(_) | Expr::Try(_) => {
                self.early_exits = true;
            }
            Expr::MethodCall(m) if !self.shadowed => {
                if let Expr::Path(p) = &*m.receiver {
                    if p.path.get_ident().is_some_and(|i| i == self.ident) {
                        let method = m.method.to_string();
                        if method == "push" {
                            self.pushes.push(r);
                        } else if LEN_MUTATORS.contains(&method.as_str()) {
                            self.other_mutations = true;
                        }
                    }
                }
            }
            _ => {}
        }
        visit::visit_expr(self, e);
    }
}

/// `true` when `span` is a top-level statement expression of `body` —
/// unconditional reachability within the body (no `if`/`match` wrapper).
fn span_is_top_level_stmt(body: &Block, span: &std::ops::Range<usize>) -> bool {
    body.stmts
        .iter()
        .any(|s| matches!(s, Stmt::Expr(e, _) if byte_range(e) == *span))
}

/// The exact-fill proof for a push chain rooted at a loop's BODY: `Some
/// (bounds)` iff the body provably receives exactly `product(bounds)`
/// pushes onto the ident. Each level must carry exactly one child loop as
/// an unconditional top-level statement (a child wrapped in `if` runs
/// zero-or-more times — the count is unprovable), push only through it,
/// and end at a leaf of exactly one unconditional top-level push. Early
/// exits, shadows, other len-mutators, ambiguous nests, and unresolvable
/// bounds all refuse the proof (`None` → the incumbent fire).
fn chain_bounds_from_body(body: &Block, ident: &Ident, source: &str) -> Option<Vec<String>> {
    // Direct children only: pushing loops of this body that are not nested
    // inside ANOTHER pushing loop of the same body (a doubly-nested loop is
    // the child's child, found by the recursion — counting it here would
    // refuse every three-deep chain).
    let all = pushing_loops_in_body(body, ident, source);
    let children: Vec<&PushingLoop> = all.iter().filter(|l| is_root_loop(l, &all)).collect();
    let skips: Vec<_> = children.iter().map(|c| c.expr_span.clone()).collect();
    let mut scan = MutationScan::new(ident, skips);
    scan.visit_block(body);
    if scan.shadowed || scan.early_exits || scan.other_mutations {
        return None;
    }
    match children.len() {
        0 => {
            // Leaf: exactly one push, unconditional (top-level stmt).
            if scan.pushes.len() == 1 && span_is_top_level_stmt(body, &scan.pushes[0]) {
                Some(Vec::new())
            } else {
                None
            }
        }
        1 => {
            // A non-leaf level must push ONLY through its child: any direct
            // push here adds pushes the product does not count.
            if !scan.pushes.is_empty() {
                return None;
            }
            let child = &children[0];
            if !span_is_top_level_stmt(body, &child.expr_span) {
                return None; // conditional child: zero-or-more executions
            }
            let bound = child.cap_text.as_ref().map(|(s, _)| normalize_text(s))?;
            let mut rest = chain_bounds_from_body(child.body, ident, source)?;
            let mut out = Vec::with_capacity(rest.len() + 1);
            out.push(bound);
            out.append(&mut rest);
            Some(out)
        }
        _ => None, // ambiguous nest (sibling pushing loops at one level)
    }
}

/// The G5c verdict: `true` when the hint is a pure product AND the block
/// provably fills the Vec to exactly that product (the module doc's strict
/// bar — single unconditional chain, no early exits, no shadows, no other
/// len-mutators, every bound derivable). Anything unprovable returns
/// `false` and the rule keeps its incumbent fire.
fn product_hint_is_exact_fill(
    scan: &LoopPushScan<'_, '_>,
    cap_arg: Option<&Expr>,
    source: &str,
    ident: &Ident,
    region: &[Stmt],
) -> bool {
    let Some(factors) = product_factors(source, cap_arg) else {
        return false;
    };
    let roots: Vec<&PushingLoop> = scan
        .loops
        .iter()
        .filter(|l| is_root_loop(l, &scan.loops))
        .collect();
    let [root] = roots.as_slice() else {
        return false;
    };
    let root_is_top = region
        .iter()
        .any(|s| matches!(s, Stmt::Expr(e, _) if byte_range(e) == root.expr_span));
    if !root_is_top {
        return false;
    }
    let mut outside = MutationScan::new(ident, vec![root.expr_span.clone()]);
    for stmt in region {
        if matches!(stmt, Stmt::Item(_)) {
            continue;
        }
        outside.visit_stmt(stmt);
    }
    let outside_clean = outside.pushes.is_empty()
        && !outside.other_mutations
        && !outside.early_exits
        && !outside.shadowed;
    if !outside_clean {
        return false;
    }
    let Some(mut bounds) = chain_bounds_from_body(root.body, ident, source) else {
        return false;
    };
    let Some((rb, _)) = root.cap_text.as_ref() else {
        return false;
    };
    bounds.insert(0, normalize_text(rb));
    multiset_eq(&factors, &bounds)
}

/// Every loop whose body pushes onto `ident`, found anywhere inside
/// `body` (nested loops included).
fn pushing_loops_in_body<'a, 'b>(
    body: &'a Block,
    ident: &'b Ident,
    source: &'b str,
) -> Vec<PushingLoop<'a>> {
    let mut scan = LoopPushScan {
        source,
        ident,
        loops: Vec::new(),
    };
    scan.visit_block(body);
    scan.loops
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    fn parses(out: &str) {
        assert!(syn::parse_file(out).is_ok(), "output does not parse: {out}");
    }

    /// The eval-span shape (`PERF_SPANS)`: a GUESSED `with_capacity` hint trimmed
    /// by the real count. This is the Plan 049 T0.0c headroom span.
    #[test]
    fn eval_span_guessed_hint_fires() {
        let code = "pub fn live_avatar_ids(roster: &[u64]) -> Vec<u64> {\n    \
             let mut ids = Vec::with_capacity(4096);\n    \
             for &id in roster {\n        \
             if id != 0 {\n            ids.push(id);\n        }\n    }\n    \
             ids\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("ids.shrink_to_fit();"), "{out}");
        // The tail keeps its own line, after the inserted call.
        assert!(out.contains("ids.shrink_to_fit();\n    ids\n}"), "{out}");
        parses(&out);
    }

    /// `while` push loops are the core regime: vec-with-capacity declines
    /// them (no derivable bound), so shrink is the fallback.
    #[test]
    fn while_loop_vec_new_fires() {
        let code = "fn drain(it: &mut std::vec::IntoIter<u32>) -> Vec<u32> {\n    \
             let mut v = Vec::new();\n    \
             while let Some(x) = it.next() {\n        v.push(x);\n    }\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();\n    v\n}"), "{out}");
        parses(&out);
    }

    /// The P121 misfire shape (riir-infer `Rig::greedy`): a bare-ident hint
    /// bounded by a WHILE condition naming the same quantity — exactly
    /// `steps` pushes, nothing to release. G5's text-equality cannot see it
    /// (a while bound carries no derivable-capacity text); the word gate
    /// declines.
    #[test]
    fn exact_hint_while_bounded_stays_silent() {
        let code = "fn greedy(steps: usize) -> Vec<u32> {\n    \
             let mut out = Vec::with_capacity(steps);\n    \
             let mut i = 0;\n    \
             while i < steps {\n        out.push(i as u32);\n        i += 1;\n    }\n    \
             out\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// A superstring ident (`steps_total`) is a DIFFERENT bound — the word
    /// gate must not decline, the fire stands.
    #[test]
    fn superstring_bound_still_fires() {
        let code = "fn f(steps_total: usize, steps: usize) -> Vec<u32> {\n    \
             let mut out = Vec::with_capacity(steps);\n    \
             for i in 0..steps_total {\n        out.push(i as u32);\n    }\n    \
             out\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("out.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// An unrelated `while` (its condition never names the hint) stays the
    /// shrink regime — unknown bound, the fire is correct.
    #[test]
    fn unrelated_while_still_fires() {
        let code = "fn f(steps: usize, it: &mut std::vec::IntoIter<u32>) -> Vec<u32> {\n    \
             let mut out = Vec::with_capacity(steps);\n    \
             while let Some(x) = it.next() {\n        out.push(x);\n    }\n    \
             out\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("out.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// Unknown-iterator for loops (`.enumerate()` is not in the
    /// vec-with-capacity table) — shrink fires.
    #[test]
    fn unknown_iterator_for_fires() {
        let code = "fn collect_hits(profile: &[f32]) -> Vec<usize> {\n    \
             let mut hits = Vec::new();\n    \
             for (i, &rel) in profile.iter().enumerate() {\n        \
             if rel > 0.5 {\n            hits.push(i);\n        }\n    }\n    \
             hits\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("hits.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// `Vec::new()` + a derivable `0..n` range: the exact-capacity rewrite
    /// (vec-with-capacity) covers this binding — shrink stays silent, the
    /// better fix wins. (The corpus `buggy_code` shape delegates to it.)
    #[test]
    fn derivable_range_delegates_to_vec_with_capacity() {
        let code = "fn build(n: usize) -> Vec<u32> {\n    \
             let mut v = Vec::new();\n    \
             for i in 0..n {\n        v.push(i as u32);\n    }\n    \
             v\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// `Vec::new()` + a slice-iter loop: same delegation to vec-with-capacity
    /// (`items.len()`).
    #[test]
    fn slice_iter_delegates_to_vec_with_capacity() {
        let code = "fn unique_spawn_indices(indices: &[u16]) -> Vec<u16> {\n    \
             let mut seen = Vec::new();\n    \
             for &i in indices {\n        \
             if !seen.contains(&i) {\n            seen.push(i);\n        }\n    }\n    \
             seen\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// `with_capacity(count)` + `0..count` is an EXACT hint — nothing to
    /// release (the box-avoid fixture shape; the cross-fire gate).
    #[test]
    fn exact_hint_stays_silent() {
        let code = "pub fn build_scores(count: usize) -> Vec<Box<i64>> {\n    \
             let mut scores = Vec::with_capacity(count);\n    \
             for i in 0..count {\n        scores.push(Box::new(i as i64));\n    }\n    \
             scores\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// `with_capacity(items.len())` + a slice-iter loop is exact too.
    #[test]
    fn exact_hint_len_form_stays_silent() {
        let code = "fn copy_all(items: &[u32]) -> Vec<u32> {\n    \
             let mut out = Vec::with_capacity(items.len());\n    \
             for &x in items {\n        out.push(x);\n    }\n    \
             out\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// An OVER-provisioned hint (`count * 2` for `count` pushes) leaves real
    /// excess — fires.
    #[test]
    fn overprovisioned_hint_fires() {
        let code = "fn build(n: usize) -> Vec<u32> {\n    \
             let mut v = Vec::with_capacity(n * 2);\n    \
             for i in 0..n {\n        v.push(i as u32);\n    }\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    // ── G5c — the exact-fill proof (I236 wrong-heal class) ──────────────

    /// THE I236 class: a pure-product hint over a nested push chain is
    /// exact-fill by construction (`m * n` capacity, `m*n` unconditional
    /// pushes) — the inserted call would be a dead no-op. Stays silent.
    #[test]
    fn product_hint_nested_exact_stays_silent() {
        let code = "fn gemm(m: usize, n: usize) -> Vec<i32> {\n    \
             let mut y = Vec::with_capacity(m * n);\n    \
             for i in 0..m {\n        \
             for j in 0..n {\n            y.push(1);\n        }\n    }\n    \
             y\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// Paren and whitespace spellings normalize to the same factors.
    #[test]
    fn product_hint_paren_whitespace_stays_silent() {
        let code = "fn gemm(m: usize, n: usize) -> Vec<i32> {\n    \
             let mut y = Vec::with_capacity((m)*n);\n    \
             for i in 0..m {\n        \
             for j in 0..n {\n            y.push(1);\n        }\n    }\n    \
             y\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// Three-deep nest, three-factor product — the same proof, deeper.
    #[test]
    fn product_hint_three_deep_stays_silent() {
        let code = "fn f(m: usize, n: usize, k: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(m * n * k);\n    \
             for i in 0..m {\n        \
             for j in 0..n {\n            \
             for q in 0..k {\n                v.push(1);\n            }\n        }\n    }\n    \
             v\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// A len-convention outer bound joins the product
    /// (`items.len() * n` over `for x in items { for j in 0..n }`).
    #[test]
    fn product_hint_len_convention_outer_stays_silent() {
        let code = "fn f(items: &[u8], n: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(items.len() * n);\n    \
             for x in items {\n        \
             for j in 0..n {\n            v.push(*x);\n        }\n    }\n    \
             v\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// SIBLING loops sum their pushes (`m + n`, not `m * n`) — the proof
    /// refuses and the rule fires (the capacity product is a real guess).
    #[test]
    fn product_hint_sibling_loops_fire() {
        let code = "fn f(m: usize, n: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(m * n);\n    \
             for i in 0..m {\n        v.push(1);\n    }\n    \
             for j in 0..n {\n        v.push(2);\n    }\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// A conditional push at the leaf (`if` inside the inner loop) breaks
    /// the exact count — fires.
    #[test]
    fn product_hint_conditional_leaf_fires() {
        let code = "fn f(m: usize, n: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(m * n);\n    \
             for i in 0..m {\n        \
             for j in 0..n {\n            \
             if j > 0 {\n                v.push(1);\n            }\n        }\n    }\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// A conditionally-reached child loop (nested inside an `if`) runs
    /// zero-or-more times — the count is unprovable — fires.
    #[test]
    fn product_hint_conditional_child_fires() {
        let code = "fn f(m: usize, n: usize, flag: bool) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(m * n);\n    \
             for i in 0..m {\n        \
             if flag {\n            \
             for j in 0..n {\n                v.push(1);\n            }\n        }\n    }\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// A direct push beside the nested chain at one level adds pushes the
    /// product does not count — fires.
    #[test]
    fn product_hint_outer_direct_push_fires() {
        let code = "fn f(m: usize, n: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(m * n);\n    \
             for i in 0..m {\n        v.push(0);\n        \
             for j in 0..n {\n            v.push(1);\n        }\n    }\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// A straight-line push after the chain adds one more — fires.
    #[test]
    fn product_hint_straight_line_push_after_chain_fires() {
        let code = "fn f(m: usize, n: usize, last: u8) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(m * n);\n    \
             for i in 0..m {\n        \
             for j in 0..n {\n            v.push(1);\n        }\n    }\n    \
             v.push(last);\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// An early exit inside the chain (`break`) shortens the count — fires.
    #[test]
    fn product_hint_break_in_chain_fires() {
        let code = "fn f(m: usize, n: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(m * n);\n    \
             for i in 0..m {\n        \
             if i > 2 {\n            break;\n        }\n        \
             for j in 0..n {\n            v.push(1);\n        }\n    }\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// An inclusive inner bound (`0..=n` iterates n+1 times) is not the
    /// bound text — the count exceeds the product and the excess is real —
    /// fires.
    #[test]
    fn product_hint_inclusive_inner_fires() {
        let code = "fn f(m: usize, n: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(m * n);\n    \
             for i in 0..m {\n        \
             for j in 0..=n {\n            v.push(1);\n        }\n    }\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// A shadow re-let inside the leaf (after the credited push) makes the
    /// remaining pushes ambiguous — the chain proof refuses — fires.
    #[test]
    fn product_hint_shadow_in_leaf_fires() {
        let code = "fn f(m: usize, n: usize) -> Vec<u8> {\n    \
             let mut v = Vec::with_capacity(m * n);\n    \
             for i in 0..m {\n        \
             for j in 0..n {\n            v.push(1);\n            \
             let mut v = Vec::new();\n        }\n    }\n    \
             v\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// A mismatched product (`m * k` for an `m*n` nest) is a real guess —
    /// fires (the over-provisioned arm at product arity).
    #[test]
    fn product_hint_mismatched_factors_fire() {
        let code = "fn gemm(m: usize, n: usize, k: usize) -> Vec<i32> {\n    \
             let mut y = Vec::with_capacity(m * k);\n    \
             for i in 0..m {\n        \
             for j in 0..n {\n            y.push(1);\n        }\n    }\n    \
             y\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("y.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// Straight-line pushes on a compile-known count: the realloc buys
    /// nothing — silent.
    #[test]
    fn straight_line_pushes_stay_silent() {
        let code = "fn pair() -> Vec<u32> {\n    \
             let mut v = Vec::new();\n    \
             v.push(1);\n    v.push(2);\n    \
             v\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    #[test]
    fn non_mut_binding_stays_silent() {
        let code = "fn drain(it: &mut std::vec::IntoIter<u32>) -> Vec<u32> {\n    \
             let v = Vec::new();\n    \
             while let Some(x) = it.next() {\n        v.push(x);\n    }\n    \
             v\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    #[test]
    fn non_bare_tail_stays_silent() {
        let code = "fn drain(it: &mut std::vec::IntoIter<u32>) -> usize {\n    \
             let mut v = Vec::new();\n    \
             while let Some(x) = it.next() {\n        v.push(x);\n    }\n    \
             v.len()\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    #[test]
    fn shadowed_binding_stays_silent() {
        let code = "fn f(n: usize) -> Vec<u32> {\n    \
             let mut v = Vec::new();\n    \
             for i in 0..n {\n        \
             let v = vec![i as u32];\n        \
             drop(v);\n    }\n    \
             let mut v = Vec::new();\n    \
             v\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    #[test]
    fn loop_body_shadow_stays_silent() {
        let code = "fn f(it: &mut std::vec::IntoIter<u32>) -> Vec<u32> {\n    \
             let mut v = Vec::new();\n    \
             while let Some(x) = it.next() {\n        \
             let v = vec![x];\n        v.push(x);\n    }\n    \
             v\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    #[test]
    fn already_has_shrink_stays_silent() {
        let code = "fn f(it: &mut std::vec::IntoIter<u32>) -> Vec<u32> {\n    \
             let mut v = Vec::new();\n    \
             while let Some(x) = it.next() {\n        v.push(x);\n    }\n    \
             v.shrink_to_fit();\n    \
             v\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// Non-Vec ctors (`HashSet`) never fire — the ctor path proves the type.
    #[test]
    fn non_vec_ctor_stays_silent() {
        let code = "use std::collections::HashSet;\n\
             fn f(indices: &[u16]) -> HashSet<u16> {\n    \
             let mut seen = HashSet::new();\n    \
             for &i in indices {\n        seen.insert(i);\n    }\n    \
             seen\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// A nested fn whose PARAMETER shares the binding's name must not
    /// credit pushes to the outer binding (the `Stmt::Item` exclusion).
    #[test]
    fn nested_fn_same_name_stays_silent() {
        let code = "fn outer() -> Vec<u32> {\n    \
             fn helper(v: &mut Vec<u32>) {\n        \
             for i in 0..4 {\n            v.push(i);\n        }\n    }\n    \
             let mut v = Vec::new();\n    \
             v\n}\n";
        assert!(propose_fix("shrink-to-fit-after-build", code).is_none());
    }

    /// A trailing comment after the tail survives verbatim.
    #[test]
    fn trailing_comment_survives() {
        let code = "fn f(it: &mut std::vec::IntoIter<u32>) -> Vec<u32> {\n    \
             let mut v = Vec::new();\n    \
             while let Some(x) = it.next() {\n        v.push(x);\n    }\n    \
             v // the drained set\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(
            out.contains("v.shrink_to_fit();\n    v // the drained set"),
            "{out}"
        );
        parses(&out);
    }

    /// Two matching fns in one file both get their insertion.
    #[test]
    fn two_fns_both_edited() {
        let code = "fn a(it: &mut std::vec::IntoIter<u32>) -> Vec<u32> {\n    \
             let mut v = Vec::new();\n    \
             while let Some(x) = it.next() {\n        v.push(x);\n    }\n    \
             v\n}\n\n\
             fn b(it: &mut std::vec::IntoIter<u32>) -> Vec<u32> {\n    \
             let mut w = Vec::new();\n    \
             while let Some(x) = it.next() {\n        w.push(x);\n    }\n    \
             w\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("v.shrink_to_fit();"), "{out}");
        assert!(out.contains("w.shrink_to_fit();"), "{out}");
        parses(&out);
    }

    /// The annotated binding form (`let mut near: Vec<u64> = Vec::new()`)
    /// behaves identically.
    #[test]
    fn annotated_binding_fires() {
        let code = "fn f(it: &mut std::vec::IntoIter<u64>) -> Vec<u64> {\n    \
             let mut near: Vec<u64> = Vec::new();\n    \
             while let Some(x) = it.next() {\n        near.push(x);\n    }\n    \
             near\n}\n";
        let out = propose_fix("shrink-to-fit-after-build", code).expect("pattern matched");
        assert!(out.contains("near.shrink_to_fit();"), "{out}");
        parses(&out);
    }
}
