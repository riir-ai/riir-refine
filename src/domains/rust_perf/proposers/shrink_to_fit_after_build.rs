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

use super::{line_indent, src, ProposerOpts, SourceEdit};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Block, Expr, File, Ident, Local, Pat, Stmt, UnOp};

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
    let mut binding: Option<(usize, CtorInfo<'_>)> = None;
    for (i, stmt) in stmts.iter().enumerate() {
        let Stmt::Local(local) = stmt else { continue };
        let Some((id, is_mut)) = local_bound_ident(local) else {
            continue;
        };
        if id != *tail_ident {
            continue;
        }
        if binding.is_some() {
            return; // a second binding of the same ident = shadowing ambiguity
        }
        if !is_mut {
            return; // shrink_to_fit needs &mut
        }
        let Some(init) = local.init.as_ref().map(|i| &*i.expr) else {
            return;
        };
        let Some(info) = vec_ctor(init) else {
            return; // bound, but not a Vec ctor this rule proves
        };
        binding = Some((i, info));
    }
    let Some((bind_idx, info)) = binding else {
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
        cap_texts: Vec::new(),
    };
    for stmt in &stmts[bind_idx + 1..] {
        if matches!(stmt, Stmt::Item(_)) {
            continue; // nested fn items: their locals are not this binding
        }
        scan.visit_stmt(stmt);
    }
    if scan.cap_texts.is_empty() {
        return; // no push loop — straight-line builds are the loss regime
    }

    // Len-convention delegation: a push loop whose iterable is a
    // len-convention shape (`for &x in items`, `items.iter()`, …) makes the
    // binding exact-capacity-coverable by the vec-with-capacity RULE —
    // `.len()` is unconditionally usize (the std convention, no type gate
    // needed, unlike range bounds) — so the with_capacity fix applies
    // conceptually and shrink stays silent for a `Vec::new()` binding there
    // too. Range-bound loops keep ONLY the vwc-overlap gate: when vwc
    // declines the range (a u8 bound fails its usize gate), shrink is the
    // correct fallback.
    if info.cap_arg.is_none() && scan.cap_texts.iter().any(|t| matches!(t, Some((_, true)))) {
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
        if scan
            .cap_texts
            .iter()
            .any(|t| matches!(t, Some((s, _)) if *s == arg_text))
        {
            return;
        }
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
/// case where vec-with-capacity declines and shrink must fire).
fn derivable_capacity_text(source: &str, iterable: &Expr) -> Option<(String, bool)> {
    match iterable {
        Expr::Range(r) => r
            .end
            .as_ref()
            .and_then(|e| src(source, e))
            .map(|s| (s, false)),
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

/// Collects the derivable-capacity text of every loop (for/while/loop) whose
/// body pushes onto `ident`. `None` entries = while/loop (no derivable bound
/// — exactly the regime the shrink fix is for).
struct LoopPushScan<'a> {
    source: &'a str,
    ident: &'a Ident,
    cap_texts: Vec<Option<(String, bool)>>,
}

impl<'a> Visit<'a> for LoopPushScan<'a> {
    fn visit_expr(&mut self, e: &'a Expr) {
        let (body, iterable): (Option<&Block>, Option<&Expr>) = match e {
            Expr::ForLoop(f) => (Some(&f.body), Some(&f.expr)),
            Expr::While(w) => (Some(&w.body), None),
            Expr::Loop(l) => (Some(&l.body), None),
            _ => (None, None),
        };
        if let Some(body) = body {
            let mut probe = PushProbe {
                ident: self.ident,
                shadowed: false,
                found: false,
            };
            probe.visit_block(body);
            if probe.found {
                self.cap_texts
                    .push(iterable.and_then(|it| derivable_capacity_text(self.source, it)));
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

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    fn parses(out: &str) {
        assert!(syn::parse_file(out).is_ok(), "output does not parse: {out}");
    }

    /// The eval-span shape (PERF_SPANS): a GUESSED with_capacity hint trimmed
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
    /// better fix wins. (The corpus buggy_code shape delegates to it.)
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
    /// credit pushes to the outer binding (the Stmt::Item exclusion).
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
