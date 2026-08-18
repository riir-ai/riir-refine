//! `clippy::semicolon_if_nothing_returned` — `fn unit() { helper() }` →
//! `fn unit() { helper(); }` (clippy 1.93 wording: "consider adding a `;` to
//! the last statement for consistent formatting").
//!
//! Probe-verified against real clippy 1.93 (Bench-probe `target/probe_semi`):
//! the lint fires on the semicolon-less tail of ANY block whose value is
//! syntactically discarded — fn bodies of unit fns, if/match arms,
//! unsafe/bare blocks, loop bodies — but ONLY when that block spans
//! multiple lines (the "for consistent formatting" rationale; single-line
//! blocks stay silent, including single-line fn tails — the prior
//! fn-tail-only proposer over-fired there). Valued positions (valued fn
//! tails, `let x = unsafe { … }` bindings) never fire; single-line `while`
//! bodies are a clippy quirk we under-approximate (documented).
//!
//! Parse-only soundness: the discarded flag propagates exclusively through
//! syntactic statement positions — semicol'd statements, block-like
//! statements without semicolons (grammar-guaranteed discarded), tails of
//! blocks already known discarded, and loop bodies (their tail expr never
//! feeds a value). Nothing type-dependent is assumed.
//!
//! Span-preserving (Issue 016): the tail statement's byte range is replaced
//! by the ORIGINAL expression text plus `;`. Impl methods + nested inline
//! mods are covered via `for_each_fn`.

use super::{edit, for_each_fn, src, SourceEdit};
use syn::spanned::Spanned;
use syn::{Expr, File, ReturnType, Stmt};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for_each_fn(file, |sig, block| {
        let unit = matches!(sig.output, ReturnType::Default);
        walk_block_stmts(source, block, unit, &mut edits);
    });
    edits
}

/// Walk a block's statements. `tail_discarded` = this block's tail
/// expression value is syntactically unused.
fn walk_block_stmts(source: &str, block: &syn::Block, tail_discarded: bool, edits: &mut Vec<SourceEdit>) {
    let last = block.stmts.len().saturating_sub(1);
    for (i, stmt) in block.stmts.iter().enumerate() {
        match stmt {
            // Semicol'd statement: value discarded. Block-like bodies carry
            // the discard inward; non-block-like already has its `;`.
            Stmt::Expr(e, Some(_)) => visit_discarded_blockish(source, e, edits),
            Stmt::Expr(e, None) => {
                let blockish = is_block_like(e);
                if blockish {
                    let discarded = if i == last {
                        tail_discarded
                    } else {
                        // Mid-block block-like statements are grammar-discarded
                        // — EXCEPT when cfg-attributed in a valued block:
                        // sibling cfg elision can promote them to the tail
                        // whose value IS used (the temp.rs E0308 class: three
                        // #[cfg(target_arch)] switched tails of a `-> f32` fn).
                        tail_discarded || !carries_cfg(e)
                    };
                    if discarded {
                        visit_discarded_blockish(source, e, edits);
                    }
                } else if i == last
                    && tail_discarded
                    && !is_control_flow(e)
                    && block_is_multiline(source, block)
                {
                    if let Some(text) = src(source, e) {
                        edits.push(edit(stmt, format!("{text};")));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Recurse into a block-like construct known to sit in a discarded-value
/// position. Non-block-like exprs are a no-op.
fn visit_discarded_blockish(source: &str, e: &Expr, edits: &mut Vec<SourceEdit>) {
    match e {
        Expr::Unsafe(u) => walk_block_stmts(source, &u.block, true, edits),
        Expr::Block(b) => walk_block_stmts(source, &b.block, true, edits),
        Expr::Match(m) => {
            for arm in &m.arms {
                match &*arm.body {
                    Expr::Block(b) => walk_block_stmts(source, &b.block, true, edits),
                    Expr::Unsafe(u) => walk_block_stmts(source, &u.block, true, edits),
                    _ => {}
                }
            }
        }
        Expr::If(i) => visit_discarded_if(source, i, edits),
        // Loop/while/for bodies never feed their tail expr as a value.
        Expr::Loop(l) => walk_block_stmts(source, &l.body, true, edits),
        Expr::While(w) => walk_block_stmts(source, &w.body, true, edits),
        Expr::ForLoop(f) => walk_block_stmts(source, &f.body, true, edits),
        _ => {}
    }
}

fn visit_discarded_if(source: &str, i: &syn::ExprIf, edits: &mut Vec<SourceEdit>) {
    walk_block_stmts(source, &i.then_branch, true, edits);
    if let Some((_, e)) = &i.else_branch {
        match &**e {
            Expr::If(nested) => visit_discarded_if(source, nested, edits),
            Expr::Block(b) => walk_block_stmts(source, &b.block, true, edits),
            _ => {}
        }
    }
}

/// The block's source span crosses a line break (clippy's formatting gate).
fn block_is_multiline(source: &str, block: &syn::Block) -> bool {
    source
        .get(block.span().byte_range())
        .is_some_and(|text| text.contains('\n'))
}

/// Tail expressions that read fine (and format better) without `;`.
fn is_block_like(e: &Expr) -> bool {
    matches!(
        e,
        Expr::If(_)
            | Expr::Match(_)
            | Expr::Block(_)
            | Expr::Loop(_)
            | Expr::While(_)
            | Expr::ForLoop(_)
            | Expr::Unsafe(_)
    )
}

/// Control flow that is already terminated in value position.
fn is_control_flow(e: &Expr) -> bool {
    matches!(e, Expr::Return(_) | Expr::Break(_) | Expr::Continue(_))
}

/// A `#[cfg(...)]`-attributed block-like expression (exact `cfg` —
/// `cfg_attr` never gates compilation). Statement-position attrs land on
/// the expression's own `attrs` field.
fn carries_cfg(e: &Expr) -> bool {
    let attrs = match e {
        Expr::Block(b) => &b.attrs,
        Expr::Unsafe(u) => &u.attrs,
        Expr::If(i) => &i.attrs,
        Expr::Match(m) => &m.attrs,
        Expr::Loop(l) => &l.attrs,
        Expr::While(w) => &w.attrs,
        Expr::ForLoop(f) => &f.attrs,
        _ => return false,
    };
    attrs.iter().any(|a| a.path().is_ident("cfg"))
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn adds_semicolon_to_unit_fn_tail_call() {
        let input = "fn helper() {}\nfn main() {\n    helper()\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.contains("helper();"), "semicolon not added: {out}");
        assert!(out.contains("fn helper() {}"), "other fn must be untouched: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn single_line_fn_tail_untouched() {
        // Probe-verified: real clippy stays silent on single-line blocks
        // (the multiline gate) — the pre-extension proposer over-fired here.
        let input = "fn helper() {}\nfn main() { helper() }";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "single-line tail must not fire: {out:?}");
    }

    #[test]
    fn adds_semicolon_to_tail_macro() {
        let input = "fn main() {\n    println!(\"hi\")\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.contains("println!(\"hi\");"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn valued_fn_untouched() {
        let input = "fn f(x: u32) -> u32 { x }";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "valued fn must not fire: {out:?}");
    }

    #[test]
    fn valued_fn_block_arms_untouched() {
        let input = "fn f(x: bool) -> u32 {\n    if x {\n        1\n    } else {\n        2\n    }\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "valued if-arms must not fire: {out:?}");
    }

    #[test]
    fn block_like_tail_untouched() {
        let input = "fn f(x: bool) { if x { helper(); } }";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "if-tail must not fire: {out:?}");
    }

    #[test]
    fn multiline_if_arm_fires() {
        let input = "fn helper() {}\nfn f(x: bool) {\n    if x {\n        helper()\n    }\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.contains("helper();"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn multiline_match_arm_block_fires() {
        let input = "fn helper() {}\nfn f(v: Option<u32>) {\n    match v {\n        Some(_) => {\n            helper()\n        }\n        None => {}\n    }\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.contains("helper();"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn semicoloned_unsafe_inner_fires() {
        let input = "fn helper() {}\nfn f() {\n    unsafe {\n        helper()\n    };\n    let _y = 1;\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.contains("helper();"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn for_body_match_arm_fires() {
        let input = "fn helper() {}\nfn f(v: &[u32]) {\n    for x in v {\n        match x {\n            0 => {\n                helper()\n            }\n            _ => {}\n        }\n    }\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.contains("helper();"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn single_line_if_arm_untouched() {
        let input = "fn helper() {}\nfn f(x: bool) { if x { helper() } }";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "single-line if-arm must not fire: {out:?}");
    }

    #[test]
    fn already_semicoloned_untouched() {
        let input = "fn main() {\n    helper();\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "already-terminated tail must not fire: {out:?}");
    }

    #[test]
    fn bare_return_tail_untouched() {
        let input = "fn f(x: u32) {\n    return\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "control-flow tail must not fire: {out:?}");
    }

    #[test]
    fn cfg_switched_blocks_in_valued_fn_untouched() {
        // The temp.rs E0308 class: cfg-switched sibling blocks are the fn's
        // RETURN VALUE on whichever target compiles — parse-time they look
        // like discarded mid-block statements.
        let input = "fn helper_f32() -> f32 {\n    0.0\n}\nfn f() -> f32 {\n    #[cfg(target_arch = \"aarch64\")]\n    {\n        helper_f32()\n    }\n    #[cfg(not(target_arch = \"aarch64\"))]\n    {\n        helper_f32()\n    }\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input);
        assert!(out.is_none(), "cfg-promotable tails must not fire: {out:?}");
    }

    #[test]
    fn cfg_switched_blocks_in_unit_fn_fire() {
        // In a unit fn the promoted tail stays discarded — firing is sound.
        let input = "fn helper() {}\nfn f() {\n    #[cfg(target_arch = \"aarch64\")]\n    {\n        helper()\n    }\n    #[cfg(not(target_arch = \"aarch64\"))]\n    {\n        helper()\n    }\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.matches("helper();").count() >= 1, "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_fn_body_fires() {
        // The katgpt-attn block_topk.rs miss: the unsafe-wrapper pattern
        // (`fn f() { unsafe fn inner() { … } inner() }`) puts real code inside
        // a fn declared in another fn's body — for_each_fn must descend into
        // Stmt::Item(Item::Fn) at any block depth.
        let input = "fn helper() {}\nfn f() {\n    unsafe fn inner() {\n        for x in 0..8 {\n            unsafe {\n                helper()\n            };\n        }\n    }\n    unsafe { inner() };\n}";
        let out = propose_fix("semicolon_if_nothing_returned", input).expect("matched");
        assert!(out.contains("helper();"), "nested-fn tail must fire: {out}");
        // The OUTER fn's own tail (`unsafe { inner() };`) is semicol'd and its
        // block body `inner()` is single-line — must NOT gain a semicolon
        // (single-line gate).
        assert!(out.contains("unsafe { inner() };"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }
}
