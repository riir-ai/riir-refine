//! `capacity-exceeded-silent-fallback-not-refusal` — SUGGEST-ONLY detector
//! (Batch 294, the wasmtime RUSTSEC-2026-0379 distill; Research 332 /
//! Plan 298 T1.3).
//!
//! A fixed-capacity structure built from variable-length input that
//! DEGRADES SILENTLY when capacity is exceeded — the cap-exceeded branch
//! truncates and falls through, so the encode/build CONTINUES with
//! silently-dropped entries: wrong encoding, wrong dispatch, no refusal.
//! The 0379 lesson is the fix's refusal half: upstream now REFUSES
//! past-65,520 jump-table targets loud — capacity-exceeded input must
//! refuse, never silently mis-dispatch (CVE-2026-115893: the aarch64
//! `br_table` silently pointed tables past 256K entries at the WRONG
//! target).
//!
//! ## Why suggest-only (never a fix)
//!
//! The bounded repair is an early capacity check + a typed error at the
//! encode/build boundary — the error type is the fn's choice (the
//! `ffi-export-panicking-body` class: a protocol decision, never
//! mechanical). NO `FIXABLE_SEC` membership, NO `collect_edits` arm — the
//! suggestion surfaces through retrieval and review.
//!
//! ## The bounded form
//!
//! ```text
//! if targets.len() > MAX_TARGETS {
//!     return Err(EncodeError::TooManyTargets); // refuse loud
//! }
//! ```
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Same-cap shape**: the `if` condition compares a `.len()` against a
//!   cap (`> CAP` / `>= CAP`) and the branch truncates THE SAME receiver
//!   to THE SAME cap text (`owned.truncate(MAX_TARGETS)`) — the silent
//!   degrade signature. Different caps, or an unconditional `truncate`,
//!   stay silent.
//! - **Encode/build vocabulary (the FP control)**: the fn name carries
//!   encode/build/pack/serialize/emit/table/dispatch/jump/frame/section/
//!   vessel segments, OR the body writes fixed-width counts
//!   (`to_{be,le,ne}_bytes`) — "limit the preview to 10 lines" UI caps
//!   outside that vocabulary stay silent.
//! - **Refusal twin suppression**: any `return` / `?` / `panic`-family
//!   (panic!/unreachable!/todo!/unimplemented!/assert!) / `unwrap`/
//!   `expect` ANYWHERE in the branch suppresses — the branch already
//!   exits loud (one loud exit hides the branch, the house conservative
//!   rule).
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are skipped;
//!   macro tokens are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - **Benign truncation fires and review decides**: a warned truncate or
//!   a log-limit drop inside encode-vocabulary fns fires too — the rule
//!   flags the silent-degrade QUESTION, not a definite bug (suggest-only
//!   is the blast-radius control).
//! - Const-vs-literal caps compare by TOKEN TEXT: `> 64` + `truncate(64)`
//!   fires, `> CAP` + `truncate(CAP)` fires, `> CAP` + `truncate(64)`
//!   does not.
//! - The slice form (`&entries[..CAP]`) and `take(n)` clamps are out —
//!   the truncate call is the only shape.
//! - Cross-fn builds (truncate in a helper, encode in the caller)
//!   unconnected.
//! - Distinct-from `length-prefix-validate-before-alloc`: that rule owns
//!   the MISSING-CHECK face (no cap at all → unbounded allocation-DoS
//!   sink); this owns the LYING-CAP face (a cap that silently drops
//!   instead of refusing — the wrong-dispatch sink). Complementary
//!   defects, distinct repairs.
//! - Batch 294 / Plan 298 / Research 332 (RUSTSEC-2026-0379 /
//!   CVE-2026-115893, CC0 — cited as class provenance; the fixture is an
//!   original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{BinOp, Expr, File, Item, Stmt};

/// One detected silent-cap truncate site.
#[derive(Debug, Clone)]
pub struct SilentTruncateFinding {
    /// Byte range of the `truncate(CAP)` call (the degrade site).
    pub site_span: Range<usize>,
    /// The truncate call's source text.
    pub site_text: String,
    /// The advice line (the bounded form + the failure class).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a fixed-capacity encode/build silently TRUNCATES \
when capacity is exceeded and falls through — the output continues with \
silently-dropped entries (wasmtime CVE-2026-115893 class: wrong targets, \
no refusal). Refuse loud at the boundary: \
`if items.len() > CAP { return Err(..) }` — a cap that lies is worse than \
no cap (Batch 294, suggest-only)";

/// The encode/build vocabulary (exact `_`-segments of the fn name).
const ENCODE_VOCAB: &[&str] = &[
    "encode", "build", "pack", "serial", "marshal", "emit", "table", "dispatch", "jump", "frame",
    "section", "vessel",
];

/// Detect silent capacity truncation in encode/build fns (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<SilentTruncateFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<SilentTruncateFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.sig.ident.to_string(), &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.sig.ident.to_string(), &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production encode paths.
            if is_test_mod(m) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, findings);
                }
            }
        }
        _ => {}
    }
}

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

fn scan_fn(
    source: &str,
    name: &str,
    block: &syn::Block,
    findings: &mut Vec<SilentTruncateFinding>,
) {
    // Vocabulary gate: encode/build fn name segments, OR the body writes
    // fixed-width counts (the oddly-named encoder face).
    let name_ok = name
        .to_ascii_lowercase()
        .split('_')
        .any(|seg| ENCODE_VOCAB.contains(&seg));
    let body_ts = block.to_token_stream().to_string();
    let body_ok = body_ts.contains("to_be_bytes")
        || body_ts.contains("to_le_bytes")
        || body_ts.contains("to_ne_bytes");
    if !name_ok && !body_ok {
        return;
    }
    let mut caps = Vec::new();
    collect_caps(&block.stmts, &mut caps);
    for span in caps {
        if span.end > source.len() {
            continue;
        }
        findings.push(SilentTruncateFinding {
            site_text: source[span.clone()].to_string(),
            site_span: span,
            advice: ADVICE,
        });
    }
}

/// One `if` candidate: the truncate-call span.
type CapCandidate = Range<usize>;

fn collect_caps(stmts: &[Stmt], out: &mut Vec<CapCandidate>) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    collect_caps_expr(&init.expr, out);
                }
            }
            Stmt::Item(item) => collect_caps_item(item, out),
            Stmt::Expr(e, _) => collect_caps_expr(e, out),
            Stmt::Macro(_) => {}
        }
    }
}

fn collect_caps_item(item: &Item, out: &mut Vec<CapCandidate>) {
    match item {
        Item::Fn(f) => collect_caps(&f.block.stmts, out),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    collect_caps(&m.block.stmts, out);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_caps_item(nested, out);
                }
            }
        }
        _ => {}
    }
}

fn collect_caps_expr(e: &Expr, out: &mut Vec<CapCandidate>) {
    if let Expr::If(i) = e {
        adjudicate_if(i, out);
        // Keep walking: nested ifs are their own candidates.
        collect_caps_expr(&i.cond, out);
        collect_caps(&i.then_branch.stmts, out);
        if let Some((_, else_e)) = &i.else_branch {
            collect_caps_expr(else_e, out);
        }
        return;
    }
    match e {
        Expr::Binary(b) => {
            collect_caps_expr(&b.left, out);
            collect_caps_expr(&b.right, out);
        }
        Expr::Paren(p) => collect_caps_expr(&p.expr, out),
        Expr::MethodCall(m) => {
            collect_caps_expr(&m.receiver, out);
            for a in &m.args {
                collect_caps_expr(a, out);
            }
        }
        Expr::Call(c) => {
            collect_caps_expr(&c.func, out);
            for a in &c.args {
                collect_caps_expr(a, out);
            }
        }
        Expr::Match(m) => {
            collect_caps_expr(&m.expr, out);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    collect_caps_expr(guard, out);
                }
                collect_caps_expr(&arm.body, out);
            }
        }
        Expr::Block(b) => collect_caps(&b.block.stmts, out),
        Expr::Unsafe(u) => collect_caps(&u.block.stmts, out),
        Expr::Loop(l) => collect_caps(&l.body.stmts, out),
        Expr::While(w) => {
            collect_caps_expr(&w.cond, out);
            collect_caps(&w.body.stmts, out);
        }
        Expr::ForLoop(fl) => {
            collect_caps_expr(&fl.expr, out);
            collect_caps(&fl.body.stmts, out);
        }
        _ => {}
    }
}

/// The signature: `if <recv>.len() > CAP { .. recv.truncate(CAP) .. }`
/// with NO refusal anywhere in the branch.
fn adjudicate_if(i: &syn::ExprIf, out: &mut Vec<CapCandidate>) {
    let Some((len_recv, cap_text)) = cap_comparison(&i.cond) else {
        return;
    };
    let mut truncates = Vec::new();
    collect_truncates(&i.then_branch.stmts, &cap_text, &len_recv, &mut truncates);
    if truncates.is_empty() {
        return;
    }
    if branch_refuses(&i.then_branch.stmts) {
        return;
    }
    out.extend(truncates);
}

/// `(len-receiver last segment, cap token text)` when the condition is a
/// `.len() > CAP` / `>= CAP` comparison.
fn cap_comparison(cond: &Expr) -> Option<(String, String)> {
    let Expr::Binary(b) = cond else {
        return None;
    };
    if !matches!(b.op, BinOp::Gt(_) | BinOp::Ge(_)) {
        return None;
    }
    let (len_side, cap_side) = match (contains_len(&b.left), contains_len(&b.right)) {
        (true, false) => (&b.left, &b.right),
        (false, true) => (&b.right, &b.left),
        _ => return None,
    };
    let len_recv = last_segment_of_len_receiver(len_side)?;
    let cap_text = cap_side.to_token_stream().to_string();
    Some((len_recv, cap_text))
}

fn contains_len(e: &Expr) -> bool {
    let mut found = false;
    contains_len_walk(e, &mut found);
    found
}

fn contains_len_walk(e: &Expr, found: &mut bool) {
    if *found {
        return;
    }
    match e {
        Expr::MethodCall(m) => {
            if m.method == "len" && m.args.is_empty() {
                *found = true;
                return;
            }
            contains_len_walk(&m.receiver, found);
            for a in &m.args {
                contains_len_walk(a, found);
            }
        }
        Expr::Binary(b) => {
            contains_len_walk(&b.left, found);
            contains_len_walk(&b.right, found);
        }
        Expr::Paren(p) => contains_len_walk(&p.expr, found),
        Expr::Cast(c) => contains_len_walk(&c.expr, found),
        Expr::Field(f) => contains_len_walk(&f.base, found),
        Expr::Index(ix) => {
            contains_len_walk(&ix.expr, found);
            contains_len_walk(&ix.index, found);
        }
        Expr::Call(c) => {
            contains_len_walk(&c.func, found);
            for a in &c.args {
                contains_len_walk(a, found);
            }
        }
        _ => {}
    }
}

/// The last path segment of the receiver carrying the `.len()` call
/// (`owned.len()` -> "owned", `self.entries.len()` -> "entries").
fn last_segment_of_len_receiver(e: &Expr) -> Option<String> {
    fn walk(e: &Expr) -> Option<String> {
        if let Expr::MethodCall(m) = e {
            if m.method == "len" && m.args.is_empty() {
                return last_segment(&m.receiver);
            }
            // Keep looking deeper (a chained call around the .len()).
            if let Some(hit) = walk(&m.receiver) {
                return Some(hit);
            }
            for a in &m.args {
                if let Some(hit) = walk(a) {
                    return Some(hit);
                }
            }
            return None;
        }
        match e {
            Expr::Binary(b) => walk(&b.left).or_else(|| walk(&b.right)),
            Expr::Paren(p) => walk(&p.expr),
            Expr::Cast(c) => walk(&c.expr),
            Expr::Reference(r) => walk(&r.expr),
            _ => None,
        }
    }
    walk(e)
}

fn last_segment(e: &Expr) -> Option<String> {
    let Expr::Path(p) = e else {
        return None;
    };
    p.path.segments.last().map(|s| s.ident.to_string())
}

/// Collect `truncate(CAP)` calls in the branch matching the cap text and
/// the `.len()` receiver (deep — a nested conditional truncate is the same
/// silent-degrade shape).
fn collect_truncates(stmts: &[Stmt], cap_text: &str, len_recv: &str, out: &mut Vec<CapCandidate>) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                if let Some(init) = &l.init {
                    truncates_expr(&init.expr, cap_text, len_recv, out);
                }
            }
            Stmt::Item(item) => truncates_item(item, cap_text, len_recv, out),
            Stmt::Expr(e, _) => truncates_expr(e, cap_text, len_recv, out),
            Stmt::Macro(_) => {}
        }
    }
}

fn truncates_item(item: &Item, cap_text: &str, len_recv: &str, out: &mut Vec<CapCandidate>) {
    match item {
        Item::Fn(f) => collect_truncates(&f.block.stmts, cap_text, len_recv, out),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    collect_truncates(&m.block.stmts, cap_text, len_recv, out);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, items)) = &m.content {
                for nested in items {
                    truncates_item(nested, cap_text, len_recv, out);
                }
            }
        }
        _ => {}
    }
}

fn truncates_expr(e: &Expr, cap_text: &str, len_recv: &str, out: &mut Vec<CapCandidate>) {
    match e {
        Expr::MethodCall(m) => {
            if m.method == "truncate"
                && m.args.len() == 1
                && m.args[0].to_token_stream().to_string() == cap_text
                && last_segment(&m.receiver).is_some_and(|r| r == len_recv)
            {
                let span = e.span().byte_range();
                out.push(span);
            }
            truncates_expr(&m.receiver, cap_text, len_recv, out);
            for a in &m.args {
                truncates_expr(a, cap_text, len_recv, out);
            }
        }
        Expr::Call(c) => {
            truncates_expr(&c.func, cap_text, len_recv, out);
            for a in &c.args {
                truncates_expr(a, cap_text, len_recv, out);
            }
        }
        Expr::If(i) => {
            truncates_expr(&i.cond, cap_text, len_recv, out);
            collect_truncates(&i.then_branch.stmts, cap_text, len_recv, out);
            if let Some((_, else_e)) = &i.else_branch {
                truncates_expr(else_e, cap_text, len_recv, out);
            }
        }
        Expr::Match(m) => {
            truncates_expr(&m.expr, cap_text, len_recv, out);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    truncates_expr(guard, cap_text, len_recv, out);
                }
                truncates_expr(&arm.body, cap_text, len_recv, out);
            }
        }
        Expr::Block(b) => collect_truncates(&b.block.stmts, cap_text, len_recv, out),
        Expr::Unsafe(u) => collect_truncates(&u.block.stmts, cap_text, len_recv, out),
        Expr::Loop(l) => collect_truncates(&l.body.stmts, cap_text, len_recv, out),
        Expr::While(w) => {
            truncates_expr(&w.cond, cap_text, len_recv, out);
            collect_truncates(&w.body.stmts, cap_text, len_recv, out);
        }
        Expr::ForLoop(fl) => {
            truncates_expr(&fl.expr, cap_text, len_recv, out);
            collect_truncates(&fl.body.stmts, cap_text, len_recv, out);
        }
        Expr::Binary(b) => {
            truncates_expr(&b.left, cap_text, len_recv, out);
            truncates_expr(&b.right, cap_text, len_recv, out);
        }
        Expr::Paren(p) => truncates_expr(&p.expr, cap_text, len_recv, out),
        _ => {}
    }
}

/// Any refusal in the branch suppresses: `return`, `?`, the panic family,
/// `unwrap`/`expect` (the loud-exit twin; one loud exit hides the branch).
fn branch_refuses(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| match stmt {
        Stmt::Local(l) => l.init.as_ref().is_some_and(|i| expr_refuses(&i.expr)),
        Stmt::Item(item) => item_refuses(item),
        Stmt::Expr(e, _) => expr_refuses(e),
        Stmt::Macro(m) => {
            let last = m.mac.path.segments.last().map(|s| s.ident.to_string());
            last.is_some_and(|p| {
                matches!(
                    p.as_str(),
                    "panic"
                        | "unreachable"
                        | "todo"
                        | "unimplemented"
                        | "assert"
                        | "assert_eq"
                        | "assert_ne"
                )
            })
        }
    })
}

fn item_refuses(item: &Item) -> bool {
    match item {
        Item::Fn(f) => branch_refuses(&f.block.stmts),
        Item::Impl(imp) => imp
            .items
            .iter()
            .any(|ii| matches!(ii, syn::ImplItem::Fn(m) if branch_refuses(&m.block.stmts))),
        _ => false,
    }
}

fn expr_refuses(e: &Expr) -> bool {
    match e {
        Expr::Return(_) | Expr::Try(_) => true,
        Expr::Macro(mac) => {
            let last = mac.mac.path.segments.last().map(|s| s.ident.to_string());
            last.is_some_and(|p| {
                matches!(
                    p.as_str(),
                    "panic"
                        | "unreachable"
                        | "todo"
                        | "unimplemented"
                        | "assert"
                        | "assert_eq"
                        | "assert_ne"
                )
            })
        }
        Expr::MethodCall(m) => {
            if matches!(m.method.to_string().as_str(), "unwrap" | "expect") {
                return true;
            }
            expr_refuses(&m.receiver) || m.args.iter().any(expr_refuses)
        }
        Expr::Binary(b) => expr_refuses(&b.left) || expr_refuses(&b.right),
        Expr::Paren(p) => expr_refuses(&p.expr),
        Expr::Cast(c) => expr_refuses(&c.expr),
        Expr::Call(c) => expr_refuses(&c.func) || c.args.iter().any(expr_refuses),
        Expr::If(i) => {
            expr_refuses(&i.cond)
                || branch_refuses(&i.then_branch.stmts)
                || i.else_branch.as_ref().is_some_and(|(_, e)| expr_refuses(e))
        }
        Expr::Match(m) => {
            expr_refuses(&m.expr)
                || m.arms.iter().any(|arm| {
                    arm.guard.as_ref().is_some_and(|(_, g)| expr_refuses(g))
                        || expr_refuses(&arm.body)
                })
        }
        Expr::Block(b) => branch_refuses(&b.block.stmts),
        Expr::Unsafe(u) => branch_refuses(&u.block.stmts),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::SilentTruncateFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    #[test]
    fn encode_fn_silent_truncate_fires() {
        let src = "const MAX: usize = 64;\n\
            pub fn encode_table(targets: &[u16], out: &mut Vec<u8>) {\n\
            \x20   let mut owned = targets.to_vec();\n\
            \x20   if owned.len() > MAX {\n\
            \x20       owned.truncate(MAX);\n\
            \x20   }\n\
            \x20   out.extend_from_slice(&(owned.len() as u16).to_be_bytes());\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the truncate-and-fall-through shape fires");
        assert!(f[0].site_text.contains("truncate(MAX)"));
    }

    #[test]
    fn refusal_twin_is_silent() {
        let src = "const MAX: usize = 64;\n\
            pub fn encode_table(targets: &[u16]) -> Result<Vec<u8>, String> {\n\
            \x20   let mut owned = targets.to_vec();\n\
            \x20   if owned.len() > MAX {\n\
            \x20       return Err(format!(\"too many: {}\", owned.len()));\n\
            \x20   }\n\
            \x20   Ok(owned.iter().flat_map(|t| t.to_be_bytes()).collect())\n\
            }\n";
        assert!(findings(src).is_empty(), "the loud-refuse twin never fires");
    }

    #[test]
    fn non_vocab_fn_stays_silent() {
        let src = "pub fn limit_suggestions(items: &mut Vec<String>) {\n\
            \x20   if items.len() > 10 {\n\
            \x20       items.truncate(10);\n\
            \x20   }\n\
            }\n";
        assert!(findings(src).is_empty(), "no encode vocabulary, no fire");
    }

    #[test]
    fn mismatched_cap_is_silent() {
        let src = "const MAX: usize = 64;\n\
            pub fn pack_rows(rows: &mut Vec<u32>) {\n\
            \x20   if rows.len() > MAX {\n\
            \x20       rows.truncate(8);\n\
            \x20   }\n\
            }\n";
        assert!(
            findings(src).is_empty(),
            "different caps do not prove the shape"
        );
    }

    #[test]
    fn mismatched_receiver_is_silent() {
        let src = "const MAX: usize = 64;\n\
            pub fn pack_rows(rows: &mut Vec<u32>, other: &mut Vec<u32>) {\n\
            \x20   if rows.len() > MAX {\n\
            \x20       other.truncate(MAX);\n\
            \x20   }\n\
            }\n";
        assert!(
            findings(src).is_empty(),
            "the truncated vec is not the compared vec"
        );
    }

    #[test]
    fn unconditional_truncate_is_silent() {
        let src = "pub fn pack_rows(rows: &mut Vec<u32>) {\n\
            \x20   rows.truncate(8);\n\
            }\n";
        assert!(findings(src).is_empty(), "no exceeded-branch shape");
    }

    #[test]
    fn body_vocab_faces_the_oddly_named_fn() {
        let src = "const CAP: usize = 16;\n\
            pub fn shrink_and_write(items: &mut Vec<u8>, out: &mut Vec<u8>) {\n\
            \x20   if items.len() > CAP {\n\
            \x20       items.truncate(CAP);\n\
            \x20   }\n\
            \x20   out.extend_from_slice(&(items.len() as u16).to_le_bytes());\n\
            }\n";
        assert_eq!(findings(src).len(), 1, "the to_le_bytes writer fires");
    }

    #[test]
    fn test_mod_is_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n\
            \x20   const MAX: usize = 4;\n\
            \x20   #[test]\n\
            \x20   fn t() {\n\
            \x20       let mut v = vec![1u8, 2, 3, 4, 5];\n\
            \x20       if v.len() > MAX { v.truncate(MAX); }\n\
            \x20       assert_eq!(v.len(), 4);\n\
            \x20   }\n\
            }\n";
        assert!(findings(src).is_empty(), "fixture habitat stays silent");
    }

    #[test]
    fn corpus_buggy_fixture_fires() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "capacity-exceeded-silent-fallback-not-refusal")
            .expect("rule present");
        let f = findings(&rule.buggy_code);
        assert!(!f.is_empty(), "the corpus buggy fixture must fire");
        let fixed = syn::parse_file(&rule.fixed_code).expect("fixed parses");
        assert!(
            detect(&rule.fixed_code, &fixed).is_empty(),
            "the fixed fixture must stay silent"
        );
    }
}
