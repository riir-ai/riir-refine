//! `clippy::let_unit_value` — `let _ = <unit-expr>;` → `<unit-expr>;`
//! (the binding is dropped, the expression kept verbatim as a statement).
//!
//! Probe-mapped trigger surface (clippy 1.99.0, 2026-11-10 — the P120
//! intake, riir-grams wire-v3 lane): the lint fires on `let _ = <unit
//! method-chain>` (the intake's `sealed.verify().expect(..)` bench shape),
//! on unit-returning std macro calls (`let _ = println!(..)`,
//! `let _ = assert!(..)`), and on underscore-NAMED patterns
//! (`let _y = <unit-expr>` — referenceable, unlike `_`). It stays SILENT
//! on pure unit literals (`let _ = ();`, `let x = ();`) — those bindings
//! are vacuous and are not this proposer's business. Real clippy's
//! diagnostics are HELP-ONLY (no machine-applicable suggestion — probed:
//! `sugg: null`, the "omit the let binding" child carries no replacement),
//! so the P21 oracle lane cannot heal the class; this corpus proposer is
//! the only fix path.
//!
//! The rewrite is semantics-preserving ONLY when the initializer is
//! unit-typed: for any non-unit type the bare statement would raise
//! `unused_must_use` (a lint-behavior regression the compile gate cannot
//! see — warnings are not errors). Parse-only therefore fires on the
//! PROVABLY-unit shapes and stays silent everywhere else:
//!
//! - a unit-returning std macro call (fixed expansion types:
//!   `println!`/`print!`/`eprintln!`/`eprint!`, `assert!`/`assert_eq!`/
//!   `assert_ne!`, `panic!`/`unreachable!`/`todo!`/`unimplemented!`).
//!   NOT `writeln!`/`write!` (`io::Result` — the `must_use` class above);
//! - a block-shaped expression whose tail is unit (no tail expr after
//!   the last statement — `ExprBlock`/`ExprUnsafe`);
//! - an `if` WITHOUT an else branch (the branch type is `()` by law);
//! - a loop expression (`loop`/`while`/`for` — always `!` or `()`, never
//!   `must_use`);
//! - an unqualified call to a file-local free fn that provably returns
//!   unit: the name is UNIQUE across the file, defined in the SAME scope
//!   as the call site (a scope-stack walk — nested-mod / nested-fn defs
//!   resolve only in their own scope), carries no return arrow or `-> ()`,
//!   and is shadowed by NO binding or `macro_rules!` anywhere in the file
//!   (same conservative pre-pass shape as `chunks_exact_to_as_chunks`'s
//!   `NonConstNames`). Foreign fns, method calls, qualified paths,
//!   `use`-imported names, and cfg-alternative duplicate defs all refuse.
//!
//! Everything else (method chains — the intake's own bench shape —,
//! `?`/`.await` initializers, `match` heads, unit consts, tuple patterns)
//! is the documented divergence: real clippy is type-aware, this lane is
//! deliberately narrower. The manual fix / `cargo clippy --fix` is the
//! sanctioned path for those.
//!
//! Pattern gate: the wildcard `_` is unreferenceable by language law and
//! fires free; any NAMED pattern (including underscore-prefixed `_y`,
//! which IS referenceable) requires zero ident occurrences of the name
//! after the initializer (any occurrence — conservative, the
//! `vec_init_then_push` style: reads like missed fires, never wrong
//! fires). Non-ident patterns (tuple/struct destructures) refuse: a unit
//! value cannot destructure, so clippy would not have fired. Attributed
//! let-statements (the cfg class) refuse — the rewrite would displace
//! the attribute's subject.

use super::SourceEdit;
use super::helpers::drops_interior_comment;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, File, ItemFn, Local, Pat, ReturnType, Stmt};

/// The unit-returning std macro allowlist. Each expansion's type is fixed
/// by std — no receiver, no generics, no `must_use`.
const UNIT_MACROS: &[&str] = &[
    "println",
    "print",
    "eprintln",
    "eprint",
    "assert",
    "assert_eq",
    "assert_ne",
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
];

/// One file-local free fn definition: unit-returning or not, in its scope.
struct FnDef {
    /// The scope path the fn was defined in (`""` = file top level,
    /// otherwise the ancestor item chain joined with `::`).
    scope: String,
    /// Provably unit: no return arrow, or `-> ()`.
    unit: bool,
}

/// File-wide pre-pass: fn definitions (name → defs), every runtime
/// binding name (shadow candidates), every `macro_rules!` name, every
/// `use`-imported leaf name (a same-name import anywhere in the call's
/// scope chain shadows the local def — conservatively: any import
/// refuses), and the byte position + text of EVERY ident (the
/// later-reference gate's positions — one walk, not one per candidate
/// local).
#[derive(Default)]
struct LocalFns {
    defs: Vec<(String, FnDef)>,
    bindings: std::collections::HashSet<String>,
    macros: std::collections::HashSet<String>,
    use_names: std::collections::HashSet<String>,
    ident_positions: Vec<(usize, String)>,
    /// The item-scope stack the walk is currently inside (`""` root).
    scope_path: Vec<String>,
}

impl<'ast> Visit<'ast> for LocalFns {
    fn visit_ident(&mut self, i: &'ast syn::Ident) {
        self.ident_positions
            .push((i.span().byte_range().start, i.to_string()));
        syn::visit::visit_ident(self, i);
    }

    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        self.defs.push((
            i.sig.ident.to_string(),
            FnDef {
                scope: self.scope_path.join("::"),
                unit: is_unit_return(&i.sig.output),
            },
        ));
        self.scope_path.push(i.sig.ident.to_string());
        syn::visit::visit_item_fn(self, i);
        self.scope_path.pop();
    }

    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        self.scope_path.push(i.ident.to_string());
        syn::visit::visit_item_mod(self, i);
        self.scope_path.pop();
    }

    fn visit_pat_ident(&mut self, p: &'ast syn::PatIdent) {
        if p.ident != "_" {
            self.bindings.insert(p.ident.to_string());
        }
        syn::visit::visit_pat_ident(self, p);
    }

    fn visit_item_macro(&mut self, i: &'ast syn::ItemMacro) {
        if let Some(ident) = i.ident.as_ref() {
            self.macros.insert(ident.to_string());
        }
        syn::visit::visit_item_macro(self, i);
    }

    fn visit_item_use(&mut self, i: &'ast syn::ItemUse) {
        collect_use_leaf_names(&i.tree, &mut self.use_names);
        syn::visit::visit_item_use(self, i);
    }
}

impl LocalFns {
    /// The ONE unique file-wide unit-returning def for `name`, unshadowed
    /// (bindings, macros, use imports), visible from `call_scope` — the
    /// def's scope must be an ancestor-or-equal of the call site's scope
    /// path on module boundaries (`""` is every scope's root; `"m"`
    /// resolves for `"m"` and `"m::f"`, never for `"mm::f"`). Uniqueness
    /// is file-wide (two defs in any scopes refuse — cfg alternatives
    /// included).
    fn visible_unit_def(&self, name: &str, call_scope: &str) -> bool {
        if self.bindings.contains(name)
            || self.macros.contains(name)
            || self.use_names.contains(name)
        {
            return false;
        }
        let mut matches = 0;
        let mut visible = false;
        for (def_name, def) in &self.defs {
            if def_name == name {
                matches += 1;
                if !def.unit {
                    return false;
                }
                if scope_resolves_from(&def.scope, call_scope) {
                    visible = true;
                }
            }
        }
        matches == 1 && visible
    }

    /// Any ident occurrence of `name` at/after `byte` — the named
    /// pattern's later-reference gate. Every occurrence class counts
    /// (paths, types, attributes, macro-internal idents) — conservative
    /// by construction: reads like missed fires, never wrong fires.
    fn referenced_after(&self, name: &str, byte: usize) -> bool {
        self.ident_positions
            .iter()
            .any(|(pos, text)| *pos >= byte && text == name)
    }
}

/// `def` is visible from `call` when `def` names an ancestor-or-equal
/// scope of `call` (item-chain prefix on `::` boundaries). The file root
/// (`""`) is every scope's ancestor.
fn scope_resolves_from(def: &str, call: &str) -> bool {
    if def.is_empty() || def == call {
        return true;
    }
    call.starts_with(def)
        && call.as_bytes().get(def.len()) == Some(&b':')
        && call.as_bytes().get(def.len() + 1) == Some(&b':')
}

/// Every leaf identifier a use tree binds (globs contribute their name
/// too — `use m::*` binds the module name `m`, its leaves unknown → the
/// glob arm records nothing but the conservatively-refusing count lives
/// in the caller: a glob import anywhere refuses ALL unqualified fn
/// arms via `glob_import`, collected separately).
fn collect_use_leaf_names(tree: &syn::UseTree, out: &mut std::collections::HashSet<String>) {
    match tree {
        syn::UseTree::Name(n) => {
            out.insert(n.ident.to_string());
        }
        syn::UseTree::Rename(n) => {
            out.insert(n.rename.to_string());
        }
        syn::UseTree::Glob(_) => {
            // Unknown leaf set — the glob marker refuses below.
            out.insert("\0glob".to_string());
        }
        syn::UseTree::Path(p) => collect_use_leaf_names(&p.tree, out),
        syn::UseTree::Group(g) => {
            for t in &g.items {
                collect_use_leaf_names(t, out);
            }
        }
    }
}

fn is_unit_return(output: &ReturnType) -> bool {
    match output {
        ReturnType::Default => true,
        ReturnType::Type(_, ty) => matches!(**ty, syn::Type::Tuple(ref t) if t.elems.is_empty()),
    }
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut pre = LocalFns::default();
    pre.visit_file(file);
    let mut v = Collector {
        source,
        edits: Vec::new(),
        fns: pre,
        scope_path: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
    fns: LocalFns,
    scope_path: Vec<String>,
}

impl Collector<'_> {
    fn try_unit_let(&mut self, local: &Local) {
        // Attributed lets: the rewrite would displace the attribute's
        // subject (the cfg class — vec_init_then_push precedent).
        if !local.attrs.is_empty() {
            return;
        }
        // Ident-pattern shapes only. The wildcard `_` is Pat::Wild in syn
        // (verified: `let _ = …` does NOT parse as Pat::Ident) and is
        // unreferenceable by language law — it fires free; every NAMED
        // pattern (including underscore-prefixed `_y`, which IS
        // referenceable) is checked against later occurrences. Non-ident
        // patterns cannot bind a unit value — clippy would not have fired.
        // `let pat: Ty = …` parses as Pat::Type wrapping the real pattern —
        // unwrap it (the annotation is inside the deleted span either way).
        let inner_pat: &Pat = match &local.pat {
            Pat::Type(pt) => &pt.pat,
            other => other,
        };
        let wildcard = matches!(inner_pat, Pat::Wild(_))
            || matches!(inner_pat, Pat::Ident(p) if p.ident == "_");
        if !wildcard && !matches!(inner_pat, Pat::Ident(_)) {
            return;
        }
        // Let-else cannot coexist with an irrefutable pattern (`_` never
        // diverges) — but guard anyway: the diverge arm changes the
        // statement's grammar and the rewrite would splice into it.
        let Some(init) = local.init.as_ref().map(|i| i.expr.as_ref()) else {
            return;
        };
        if local.init.as_ref().is_some_and(|i| i.diverge.is_some()) {
            return;
        }
        // The removed span is `let <pat> (: ty)? =` — a comment inside it
        // would be displaced.
        let span = local.span().byte_range().start..init.span().byte_range().start;
        if drops_interior_comment(self.source, span.clone(), &[]) {
            return;
        }
        if !wildcard {
            let name = match inner_pat {
                Pat::Ident(p) => p.ident.to_string(),
                _ => return,
            };
            let init_end = init.span().byte_range().end;
            let refs = self.fns.referenced_after(&name, init_end);
            if refs {
                return;
            }
        }
        if !self.is_provably_unit(init) {
            return;
        }
        // The rewrite DELETES `let <pat> (: ty)? =` and keeps the
        // initializer verbatim — the original trailing `;` survives in
        // place, so the statement becomes `<expr>;` with zero resampling
        // of the expression's bytes.
        self.edits.push(SourceEdit {
            start: span.start,
            end: span.end,
            replacement: String::new(),
        });
    }

    /// The syntactic unit proof. Every true arm's expansion/evaluation
    /// type is fixed by std or by the file-local definition.
    fn is_provably_unit(&self, expr: &Expr) -> bool {
        match expr {
            // `loop`/`while`/`for` evaluate to `!` or `()`, never must_use.
            Expr::Loop(_) | Expr::While(_) | Expr::ForLoop(_) => true,
            // An `if` without else IS `()` by law.
            Expr::If(i) => i.else_branch.is_none(),
            // A block whose tail is a statement (or empty) is `()`.
            Expr::Block(b) => block_tail_is_unit(&b.block),
            Expr::Unsafe(u) => block_tail_is_unit(&u.block),
            Expr::Macro(m) => m
                .mac
                .path
                .get_ident()
                .is_some_and(|id| UNIT_MACROS.contains(&id.to_string().as_str())),
            Expr::Call(c) => {
                let syn::Expr::Path(p) = &*c.func else {
                    return false;
                };
                if p.qself.is_some() || p.path.segments.len() != 1 {
                    return false; // method call / qualified path — unresolvable
                }
                let Some(segment) = p.path.segments.last() else {
                    return false;
                };
                if !segment.arguments.is_none() {
                    return false;
                }
                let name = segment.ident.to_string();
                self.fns
                    .visible_unit_def(&name, &self.scope_path.join("::"))
            }
            _ => false,
        }
    }
}

fn block_tail_is_unit(block: &syn::Block) -> bool {
    block
        .stmts
        .last()
        .is_none_or(|stmt| !matches!(stmt, Stmt::Expr(_, None)))
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        self.scope_path.push(i.sig.ident.to_string());
        syn::visit::visit_item_fn(self, i);
        self.scope_path.pop();
    }

    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        self.scope_path.push(i.ident.to_string());
        syn::visit::visit_item_mod(self, i);
        self.scope_path.pop();
    }

    fn visit_local(&mut self, l: &'ast Local) {
        self.try_unit_let(l);
        syn::visit::visit_local(self, l);
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    // ── the fire arms ──

    #[test]
    fn unit_macro_wildcard_rewrites() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let _ = println!(\"hello\"); }",
        )
        .expect("matched");
        assert!(out.contains("println!(\"hello\");"), "{out}");
        assert!(!out.contains("let"), "binding remains: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn unit_macro_named_unreferenced_rewrites() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let _unused = println!(\"hello\"); }",
        )
        .expect("matched");
        assert!(out.contains("println!(\"hello\");"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn assert_macros_fire() {
        for (mac, call) in [
            ("assert", "assert!(a > 0)"),
            ("assert_eq", "assert_eq!(a, b)"),
            ("assert_ne", "assert_ne!(a, b)"),
            ("panic", "panic!(\"x\")"),
            ("unreachable", "unreachable!()"),
            ("todo", "todo!()"),
            ("unimplemented", "unimplemented!()"),
            ("print", "print!(\"{}\", a)"),
            ("eprintln", "eprintln!(\"x\")"),
            ("eprint", "eprint!(\"x\")"),
        ] {
            let src = format!("fn probe(a: u8, b: u8) {{ let _ = {call}; }}");
            let out =
                propose_fix("let_unit_value", &src).unwrap_or_else(|| panic!("{mac} did not fire"));
            assert!(!out.contains("let _ ="), "{mac}: {out}");
            syn::parse_file(&out).unwrap_or_else(|e| panic!("{mac} does not re-parse: {e} {out}"));
        }
    }

    #[test]
    fn unit_tail_block_rewrites() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let _ = { log_a(); log_b(); }; }\nfn log_a() {}\nfn log_b() {}",
        )
        .expect("matched");
        assert!(out.contains("{ log_a(); log_b(); };"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn empty_block_rewrites() {
        let out = propose_fix("let_unit_value", "fn probe() { let _ = {}; }").expect("matched");
        assert!(out.contains("{};"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn if_without_else_rewrites() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe(c: bool) { let _ = if c { log_a(); }; }\nfn log_a() {}",
        )
        .expect("matched");
        assert!(out.contains("if c { log_a(); };"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn loop_shapes_rewrite() {
        for shape in [
            "loop { log_a(); }",
            "while c { log_a(); }",
            "for _ in 0..3 { log_a(); }",
        ] {
            let src = format!("fn probe(c: bool) {{ let _ = {shape}; }}\nfn log_a() {{}}");
            let out =
                propose_fix("let_unit_value", &src).unwrap_or_else(|| panic!("{shape} no fire"));
            assert!(!out.contains("let _ ="), "{shape}: {out}");
            syn::parse_file(&out).unwrap_or_else(|e| panic!("{shape}: {e} {out}"));
        }
    }

    #[test]
    fn file_local_unit_fn_same_scope_fires() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let _ = unit_fn(); }\nfn unit_fn() {}",
        )
        .expect("matched");
        assert!(out.contains("unit_fn();"), "{out}");
        assert!(!out.contains("let _"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_fn_scope_resolves() {
        let out = propose_fix(
            "let_unit_value",
            "fn outer() {\n    fn helper() {}\n    let _ = helper();\n}",
        )
        .expect("matched");
        assert!(out.contains("helper();"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn typed_annotation_rewrites() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let _unused: () = println!(\"x\"); }",
        )
        .expect("matched");
        assert!(out.contains("println!(\"x\");"), "{out}");
        assert!(!out.contains(": ()"), "annotation remains: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    // ── the refusal arms ──

    #[test]
    fn io_result_macro_refuses() {
        // writeln! returns io::Result<()> — the bare statement would raise
        // unused_must_use, a lint regression the compile gate cannot see.
        let out = propose_fix(
            "let_unit_value",
            "fn probe(w: &mut String) { let _ = writeln!(w, \"x\"); }",
        );
        assert!(out.is_none(), "io::Result class fired: {out:?}");
    }

    #[test]
    fn format_units_call_refuses() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() -> Result<(), std::io::Error> { let _ = format_units()?; Ok(()) }\nfn format_units() -> Result<usize, std::io::Error> { Ok(4) }",
        );
        assert!(out.is_none(), "`?` initializer fired: {out:?}");
    }

    #[test]
    fn method_chain_refuses() {
        // The intake's own bench shape: a typeck-only unit chain.
        let out = propose_fix(
            "let_unit_value",
            "struct Seal;\nimpl Seal { fn verify(&self) -> Result<(), String> { Ok(()) } }\nfn probe(s: Seal) { let _ = s.verify().expect(\"x\"); }",
        );
        assert!(out.is_none(), "method chain fired: {out:?}");
    }

    #[test]
    fn named_referenced_later_refuses() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let used = println!(\"x\"); if used == () { } }",
        );
        assert!(out.is_none(), "referenced binding fired: {out:?}");
    }

    #[test]
    fn attributed_local_refuses() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { #[cfg(unix)] let _ = println!(\"x\"); }",
        );
        assert!(out.is_none(), "attributed local fired: {out:?}");
    }

    #[test]
    fn foreign_fn_name_refuses() {
        // `helper` is called but never defined in this file — unresolvable.
        let out = propose_fix("let_unit_value", "fn probe() { let _ = helper(); }");
        assert!(out.is_none(), "foreign fn fired: {out:?}");
    }

    #[test]
    fn non_unit_local_fn_refuses() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let _ = make(); }\nfn make() -> u32 { 7 }",
        );
        assert!(out.is_none(), "non-unit local fn fired: {out:?}");
    }

    #[test]
    fn cross_scope_fn_refuses() {
        // Defined in a nested module — a same-name call at file scope does
        // not resolve to it.
        let out = propose_fix(
            "let_unit_value",
            "mod m { pub fn helper() {} }\nfn probe() { let _ = helper(); }",
        );
        assert!(out.is_none(), "cross-scope fn fired: {out:?}");
    }

    #[test]
    fn binding_shadow_refuses() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let helper = 7; let _ = helper(); }\nfn helper() {}",
        );
        assert!(out.is_none(), "shadowed fn fired: {out:?}");
    }

    #[test]
    fn duplicate_def_refuses() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let _ = helper(); }\nfn helper() {}\nmod m { fn helper() {} }",
        );
        assert!(out.is_none(), "ambiguous def fired: {out:?}");
    }

    #[test]
    fn qualified_path_refuses() {
        let out = propose_fix(
            "let_unit_value",
            "mod m { pub fn helper() {} }\nfn probe() { let _ = m::helper(); }",
        );
        assert!(out.is_none(), "qualified path fired: {out:?}");
    }

    #[test]
    fn if_with_else_refuses() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe(c: bool) -> u8 { let _ = if c { 1 } else { 2 }; 0 }",
        );
        assert!(out.is_none(), "if-with-else fired: {out:?}");
    }

    #[test]
    fn unit_literal_stays_silent() {
        // clippy 1.99 is silent on pure unit literals — no corpus value.
        let out = propose_fix("let_unit_value", "fn probe() { let _ = (); }");
        assert!(out.is_none(), "unit literal fired: {out:?}");
    }

    #[test]
    fn comment_in_removed_span_refuses() {
        let out = propose_fix(
            "let_unit_value",
            "fn probe() { let _ /* note */ = println!(\"x\"); }",
        );
        assert!(out.is_none(), "interior comment fired: {out:?}");
    }

    #[test]
    fn interior_comments_in_kept_expr_survive() {
        let src = "fn probe() { let _ = println!(\"x\" /* why */); }";
        let out = propose_fix("let_unit_value", src).expect("matched");
        assert!(out.contains("/* why */"), "comment displaced: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn untouched_bytes_survive() {
        let src = "// header\nfn probe() {\n    // keep me\n    let _ = println!(\"x\");\n    other();\n}\nfn other() {}";
        let out = propose_fix("let_unit_value", src).expect("matched");
        assert!(out.contains("// header\n"), "{out}");
        assert!(out.contains("    // keep me\n"), "{out}");
        assert!(out.contains("other();"), "{out}");
        assert_eq!(src.lines().count(), out.lines().count(), "{out}");
    }
}
