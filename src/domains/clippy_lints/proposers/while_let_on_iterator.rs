//! `clippy::while_let_on_iterator` — `while let Some(x) = it.next() { .. }` →
//! `for x in it { .. }`.
//!
//! VERIFY-tier: the receiver could be a custom non-Iterator type with a
//! `next()` method — rustc backs the revert under `--verify`.
//!
//! Detection is deliberately conservative (Issue 259): the `for` rewrite
//! MOVES the iterator, so it is emitted ONLY when nothing else can touch it:
//! - the iterator must be a BARE ident (`it.next()`), the binding a real
//!   ident (not `_`);
//! - the loop BODY must not mention the iterator ident — direct paths AND
//!   macro expansion: a locally-defined `macro_rules!` whose definition
//!   tokens mention the ident (the measured E0382 class — riir-train
//!   `aad54860`'s `next_str!`/`next_usize!` rewrote into use-after-move in
//!   three examples), or any body invocation whose argument tokens mention
//!   it (`println!("{}", argv)` — the same hole, both fixed here);
//! - nothing AFTER the loop in the enclosing function may mention the ident
//!   — a post-loop use (`let rest = it.next();`, a second while-let over the
//!   same iterator) breaks under the move with the same E0382. The `.by_ref()`
//!   form that would serve there is NOT emitted: it reborrows the binding and
//!   breaks the non-`mut` `&mut I` shape (E0596, probed), so the rule declines
//!   instead of guessing — decline is a correct answer;
//! - no enclosing function (const/static initializer contexts) → decline.
//!
//! Binding modifiers (`mut`, `ref`) are preserved by splicing the binding's
//! ORIGINAL source text.
//! Span-preserving (T6.4): the whole `while` expr's byte range is replaced
//! by `for <binding> in <IT> <body>` (body = the block's own source text).

use std::collections::HashMap;

use super::{SourceEdit, edit, src};
use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{
    Block, Expr, ExprLet, ExprMacro, ExprMethodCall, ExprWhile, File, Ident, ItemFn, ItemMacro,
    Pat, PatIdent, StmtMacro,
};

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut defs = MacroDefs::default();
    defs.visit_file(file);
    let mut fns = FnSurfaces::default();
    fns.visit_file(file);
    let mut v = Collector {
        source,
        edits: Vec::new(),
        defs: defs.by_name,
        fns: fns.list,
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    /// every `macro_rules!` definition in the file, by macro name
    defs: HashMap<String, Vec<&'a TokenStream>>,
    fns: Vec<FnSurface<'a>>,
}

struct FnSurface<'a> {
    start: usize,
    end: usize,
    invocations: Vec<MacroInvo<'a>>,
}

struct MacroInvo<'a> {
    start: usize,
    args: &'a TokenStream,
    name: String,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_while(&mut self, w: &'ast ExprWhile) {
        syn::visit::visit_expr_while(self, w);
        if let Some(replacement) = self.try_for_loop(w) {
            self.edits.push(edit(w, replacement));
        }
    }
}

/// All `macro_rules!` definitions in the file (top-level and fn-local).
#[derive(Default)]
struct MacroDefs<'a> {
    by_name: HashMap<String, Vec<&'a TokenStream>>,
}

impl<'ast> Visit<'ast> for MacroDefs<'ast> {
    fn visit_item_macro(&mut self, m: &'ast ItemMacro) {
        if m.mac.path.is_ident("macro_rules") {
            if let Some(ident) = &m.ident {
                self.by_name
                    .entry(ident.to_string())
                    .or_default()
                    .push(&m.mac.tokens);
            }
        }
        syn::visit::visit_item_macro(self, m);
    }
}

/// Per-function surface: byte range + every macro invocation in the fn.
#[derive(Default)]
struct FnSurfaces<'a> {
    list: Vec<FnSurface<'a>>,
}

impl<'ast> Visit<'ast> for FnSurfaces<'ast> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        let r = f.span().byte_range();
        let mut invo: Vec<MacroInvo<'ast>> = Vec::new();
        {
            let mut scan = MacroInvoScan { out: &mut invo };
            scan.visit_item_fn(f);
        }
        self.list.push(FnSurface {
            start: r.start,
            end: r.end,
            invocations: invo,
        });
        syn::visit::visit_item_fn(self, f);
    }
}

/// Collects macro invocations (`expr!`/`stmt!` positions; item-position
/// invocations too, but NOT `macro_rules!` definitions themselves — those are
/// the defs, not uses). Two lifetimes: the `&mut` borrow of the output vec
/// and the AST lifetime the collected borrows point into.
struct MacroInvoScan<'a, 'ast> {
    out: &'a mut Vec<MacroInvo<'ast>>,
}

impl<'ast> Visit<'ast> for MacroInvoScan<'_, 'ast> {
    fn visit_expr_macro(&mut self, m: &'ast ExprMacro) {
        self.push(&m.mac);
        syn::visit::visit_expr_macro(self, m);
    }

    fn visit_stmt_macro(&mut self, m: &'ast StmtMacro) {
        self.push(&m.mac);
        syn::visit::visit_stmt_macro(self, m);
    }

    fn visit_item_macro(&mut self, m: &'ast ItemMacro) {
        let is_def = m.mac.path.is_ident("macro_rules");
        if !is_def && m.ident.is_none() {
            // item-position invocation (`thread_local! { .. }` and friends)
            self.push(&m.mac);
        }
        syn::visit::visit_item_macro(self, m);
    }
}

impl<'ast> MacroInvoScan<'_, 'ast> {
    fn push(&mut self, mac: &'ast syn::Macro) {
        let name = mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if name.is_empty() {
            return;
        }
        let r = mac.span().byte_range();
        self.out.push(MacroInvo {
            start: r.start,
            args: &mac.tokens,
            name,
        });
    }
}

impl Collector<'_> {
    fn try_for_loop(&self, w: &ExprWhile) -> Option<String> {
        if !w.attrs.is_empty() || w.label.is_some() {
            return None; // attrs/label would be dropped by the splice
        }
        let Expr::Let(le) = &*w.cond else {
            return None;
        };
        let (binding, next) = some_binding_over_next(le)?;
        // Iterator must be a bare ident (`it`), not a chain (`xs.iter()`).
        let Expr::Path(it_path) = &*next.receiver else {
            return None;
        };
        let it_ident = it_path.path.get_ident()?;
        // Guard: the body must not mention the iterator ident (mutation,
        // reuse, or a same-named binding would break under `for`).
        if body_mentions(&w.body, it_ident) {
            return None;
        }
        // Issue 259: the body can also reach the iterator THROUGH macros —
        // a locally-defined `macro_rules!` whose definition tokens contain
        // the ident expands to a body use (the measured E0382 class), and so
        // does any invocation whose argument tokens contain it.
        let name = it_ident.to_string();
        if self.macro_mention_in(&w.body, &name) {
            return None;
        }
        // Issue 259 (post-loop half): anything after the loop in the same
        // function that mentions the ident breaks under the move (a leftover
        // `it.next()`, a second while-let draining the remainder). Decline.
        let wr = w.span().byte_range();
        let Some(fs) = self.enclosing_fn(wr.start, wr.end) else {
            return None; // const/static context — unscannable, decline
        };
        let fn_text = self.source.get(fs.start..fs.end)?;
        let after = fn_text.get(wr.end - fs.start..)?;
        if text_mentions(after, &name) {
            return None;
        }
        let reaches = fs
            .invocations
            .iter()
            .filter(|inv| inv.start >= wr.end)
            .any(|inv| def_or_args_mention(&inv.name, inv.args, &self.defs, &name));
        if reaches {
            return None;
        }
        let binding = src(self.source, binding)?;
        let iter = src(self.source, &next.receiver)?;
        let body = src(self.source, &w.body)?;
        Some(format!("for {binding} in {iter} {body}"))
    }

    fn enclosing_fn(&self, start: usize, end: usize) -> Option<&FnSurface<'_>> {
        self.fns
            .iter()
            .filter(|f| f.start <= start && f.end >= end)
            .max_by_key(|f| f.start)
    }

    /// Does any macro invocation inside `block` reach `name` — through a
    /// locally-defined macro's definition tokens or an invocation's argument
    /// tokens?
    fn macro_mention_in<'b>(&self, block: &'b Block, name: &str) -> bool {
        let mut invo: Vec<MacroInvo<'b>> = Vec::new();
        {
            let mut scan = MacroInvoScan { out: &mut invo };
            scan.visit_block(block);
        }
        invo.iter()
            .any(|inv| def_or_args_mention(&inv.name, inv.args, &self.defs, name))
    }
}

/// A macro invocation reaches `name` when its locally-defined body mentions
/// the ident OR its argument tokens mention it (arguments flow into the
/// expansion).
fn def_or_args_mention(
    macro_name: &str,
    args: &TokenStream,
    defs: &HashMap<String, Vec<&TokenStream>>,
    target: &str,
) -> bool {
    if let Some(list) = defs.get(macro_name) {
        for d in list {
            if tokens_mention(d, target, defs, true, 0) {
                return true;
            }
        }
    }
    tokens_mention(args, target, defs, false, 0)
}

/// Does `ts` contain the target ident — directly (same token-nesting level,
/// `$`-metavariable names skipped in definition mode) or through a nested
/// invocation of a locally-defined macro?
fn tokens_mention(
    ts: &TokenStream,
    target: &str,
    defs: &HashMap<String, Vec<&TokenStream>>,
    skip_dollar: bool,
    depth: usize,
) -> bool {
    if depth > 12 {
        return false; // pathological nesting — decline-by-silence stays safe
    }
    let toks: Vec<TokenTree> = ts.clone().into_iter().collect();
    for (i, tt) in toks.iter().enumerate() {
        match tt {
            TokenTree::Group(g) => {
                if tokens_mention(&g.stream(), target, defs, skip_dollar, depth + 1) {
                    return true;
                }
            }
            TokenTree::Ident(id) => {
                if skip_dollar
                    && i > 0
                    && matches!(&toks[i - 1], TokenTree::Punct(p) if p.as_char() == '$')
                {
                    continue; // `$name` metavariable placeholder, not a reference
                }
                let s = id.to_string();
                if s == target {
                    return true;
                }
                // `other_mac!(..)` inside these tokens: its definition and
                // its arguments both flow into the expansion.
                if defs.contains_key(&s)
                    && matches!(toks.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!')
                {
                    if let Some(TokenTree::Group(g)) = toks.get(i + 2) {
                        if let Some(list) = defs.get(&s) {
                            for d in list {
                                if tokens_mention(d, target, defs, true, depth + 1) {
                                    return true;
                                }
                            }
                        }
                        if tokens_mention(&g.stream(), target, defs, false, depth + 1) {
                            return true;
                        }
                    }
                }
            }
            _ => {}
        }
    }
    false
}

/// Word-boundary search for `needle` in `hay` (post-loop text scan; a hit
/// inside a string/comment is a false positive in the SAFE direction — it
/// only makes the rule decline).
fn text_mentions(hay: &str, needle: &str) -> bool {
    let bytes = hay.as_bytes();
    let nb = needle.as_bytes();
    if nb.is_empty() {
        return false;
    }
    let mut i = 0;
    while let Some(pos) = hay[i..].find(needle) {
        let a = i + pos;
        let b = a + nb.len();
        let before_ok = a == 0 || !is_word_byte(bytes[a - 1]);
        let after_ok = b == bytes.len() || !is_word_byte(bytes[b]);
        if before_ok && after_ok {
            return true;
        }
        i = b;
    }
    false
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Match `let Some(<binding>) = <recv>.next()` — returns the binding pattern
/// and the (already validated) `next` method call.
fn some_binding_over_next(le: &ExprLet) -> Option<(&PatIdent, &ExprMethodCall)> {
    if !le.attrs.is_empty() {
        return None;
    }
    let Pat::TupleStruct(ts) = &*le.pat else {
        return None;
    };
    if !ts.attrs.is_empty() || !ts.path.is_ident("Some") || ts.elems.len() != 1 {
        return None;
    }
    let Pat::Ident(binding) = &ts.elems[0] else {
        return None;
    };
    if !binding.attrs.is_empty() || binding.subpat.is_some() || binding.ident == "_" {
        return None;
    }
    let Expr::MethodCall(next) = &*le.expr else {
        return None;
    };
    if !next.attrs.is_empty()
        || next.turbofish.is_some()
        || next.method != "next"
        || !next.args.is_empty()
    {
        return None;
    }
    Some((binding, next))
}

/// Does any path in the block mention `name`?
fn body_mentions(body: &Block, name: &Ident) -> bool {
    let mut scan = PathMention { name, found: false };
    scan.visit_block(body);
    scan.found
}

struct PathMention<'a> {
    name: &'a Ident,
    found: bool,
}

impl<'ast> Visit<'ast> for PathMention<'_> {
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path.get_ident() == Some(self.name) {
            self.found = true;
        }
        syn::visit::visit_expr_path(self, p);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_while_let_to_for() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) -> u32 { let mut s = 0; while let Some(x) = it.next() { s += x as u32; } s }";
        let out = propose_fix("while_let_on_iterator", input).expect("matched");
        assert!(
            out.contains("for x in it { s += x as u32; }"),
            "for not produced: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn mut_binding_modifier_preserved() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(mut x) = it.next() { x += 1; } }";
        let out = propose_fix("while_let_on_iterator", input).expect("matched");
        assert!(
            out.contains("for mut x in it"),
            "binding modifier lost: {out}"
        );
    }

    #[test]
    fn body_mentioning_iterator_unchanged() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(x) = it.next() { it.reset(); } }";
        let out = propose_fix("while_let_on_iterator", input);
        assert!(
            out.is_none(),
            "body reuses the iterator — must not fire: {out:?}"
        );
    }

    #[test]
    fn wildcard_binding_unchanged() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(_) = it.next() {} }";
        let out = propose_fix("while_let_on_iterator", input);
        assert!(out.is_none(), "Some(_) must not fire: {out:?}");
    }

    #[test]
    fn method_chain_receiver_unchanged() {
        let input = "fn f(xs: &Vec<u8>) { while let Some(x) = xs.iter().next() {} }";
        let out = propose_fix("while_let_on_iterator", input);
        assert!(out.is_none(), "non-ident receiver must not fire: {out:?}");
    }

    // ── Issue 259 — the macro-expansion class (the measured E0382) ──

    /// The exact riir-train `aad54860` shape: a local `macro_rules!` whose
    /// definition calls `argv.next()` — the expansion is a body use the
    /// `for` move invalidates (E0382 in three examples). Must not fire.
    #[test]
    fn local_macro_reading_iterator_refused() {
        let input = concat!(
            "fn f(mut argv: std::vec::IntoIter<u8>) -> u8 { ",
            "macro_rules! next_str { ($name:expr) => { { match argv.next() { Some(v) => v, None => panic!(\"missing {}\", $name) } } } }; ",
            "while let Some(arg) = argv.next() { ",
            "match arg { 1 => { let _ = next_str!(\"--gguf\"); } _ => {} } ",
            "} 0 }"
        );
        let out = propose_fix("while_let_on_iterator", input);
        assert!(
            out.is_none(),
            "macro expansion reads the iterator — must not fire: {out:?}"
        );
    }

    /// The same hole through invocation ARGUMENTS — `println!("{}", argv)`
    /// is a body use too (pre-existing bug class, same fix).
    #[test]
    fn macro_argument_mention_refused() {
        let input = "fn f(mut argv: std::vec::IntoIter<u8>) { while let Some(x) = argv.next() { println!(\"{}\", argv.len()); } }";
        let out = propose_fix("while_let_on_iterator", input);
        assert!(out.is_none(), "macro arg mentions the iterator: {out:?}");
    }

    /// Post-loop use breaks under the move (E0382) — decline.
    #[test]
    fn post_loop_use_refused() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; while let Some(x) = it.next() { s += x; } let _ = it.next(); s }";
        let out = propose_fix("while_let_on_iterator", input);
        assert!(out.is_none(), "post-loop use must not fire: {out:?}");
    }

    /// A second while-let over the same iterator after the loop is a
    /// post-loop mention for the FIRST loop — it must refuse (its move would
    /// break the second loop's `it.next()`). The SECOND loop may convert:
    /// the first stayed `while let` (consuming to exhaustion, the original
    /// semantics), so its `for` move compiles — rustc-verified (i259 probe:
    /// the mixed while+for pair is clean). Asymmetric by design.
    #[test]
    fn sequential_while_loops_first_refuses_second_converts() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(x) = it.next() { let _ = x; } while let Some(y) = it.next() { let _ = y; } }";
        let out = propose_fix("while_let_on_iterator", input).expect("second loop converts");
        assert!(
            out.contains("while let Some(x) = it.next()") && out.contains("for y in it"),
            "first must refuse, second must convert: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    /// A macro invoked AFTER the loop whose local definition reads the
    /// iterator — the hidden post-loop use (text scan cannot see it).
    #[test]
    fn post_loop_macro_hidden_use_refused() {
        let input = concat!(
            "fn f(mut argv: std::vec::IntoIter<u8>) { ",
            "macro_rules! next_str { () => { { argv.next() } } }; ",
            "while let Some(arg) = argv.next() { let _ = arg; } ",
            "let _ = next_str!(); }"
        );
        let out = propose_fix("while_let_on_iterator", input);
        assert!(out.is_none(), "post-loop hidden macro use: {out:?}");
    }

    /// Guard against over-refusal: a std macro with clean arguments still
    /// converts (the common `println!("{x}")` shape).
    #[test]
    fn clean_std_macro_body_still_converts() {
        let input = "fn f(mut it: std::vec::IntoIter<u8>) { while let Some(x) = it.next() { println!(\"{x}\"); } }";
        let out = propose_fix("while_let_on_iterator", input).expect("matched");
        assert!(
            out.contains("for x in it { println!(\"{x}\"); }"),
            "clean std-macro body must convert: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }
}
