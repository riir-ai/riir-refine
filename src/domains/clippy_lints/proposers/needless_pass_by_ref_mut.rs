//! `clippy::needless_pass_by_ref_mut` — `&mut self` → `&self` on methods
//! whose bodies never use `self` mutably, plus the same-file `let mut`
//! caller cascade.
//!
//! NURSERY group, default `allow` on clippy 0.1.98 — the lint only appears
//! in a stream that enables it explicitly (`-W clippy::needless-pass-by-ref-mut`),
//! so this rule rides the `--clippy-json` oracle lane / `--verify` tier
//! (the healer's own lint-flavored cfg-gate lesson: a default-flags verify
//! would compile the rule to a no-op).
//!
//! Probe-mapped trigger surface (clippy 0.1.98 / 48a229ceae, 2026-09-14,
//! `.raw/pbrm_probe`):
//! - `pub` methods are SILENT (clippy's exported-API guard); private AND
//!   `pub(crate)` methods FIRE — the proposer mirrors that visibility line;
//! - trait-impl methods are SILENT (an impl cannot relax its trait's
//!   receiver) — the proposer only walks INHERENT impls;
//! - private free fns with `&mut` params fire too (probed) — OUT OF SCOPE
//!   here, the documented divergence (receiver-only v1);
//! - real clippy's suggestion deletes `mut ` at the receiver span and is
//!   applicability `Unspecified` — `cargo clippy --fix` applied 0 edits
//!   (probed); the proposer emits the same `&self` shape span-preserving;
//! - real clippy is TYPE-AWARE (it fires on `self.lookup()` when the callee
//!   is `&self`, and on `self.label.clone()`). syn cannot resolve types:
//!   the callee must be (a) on a small std-`&self` allowlist, or (b)
//!   declared exactly once in this file with a `&self` receiver. Unknown /
//!   ambiguous / `&mut` callees refuse the WHOLE method (a `&mut` callee
//!   this pass fixes becomes visible next fixpoint pass — the 4-pass
//!   composition, not intra-pass cleverness).
//!
//! The cascade: real clippy's own fix is definition-site ONLY — the caller
//! `let mut h` cleanup is rustc's separate `unused_mut` (machine-applicable,
//! one `cargo clippy --fix` pass). To keep a healed file warning-free in
//! `-D warnings` consumer CIs, this proposer ALSO drops the binding `mut`
//! for the provable shape: every mention of the binding inside its
//! enclosing fn is the receiver of a call to a method this pass fixed
//! (locals are fn-scoped, so the fn range is the complete use set; the
//! method name must be declared exactly once in the file). Any other use
//! shape — assignment, `&mut`, field write, an unfixed method call, a
//! by-value pass, a macro, a shadowing re-let — leaves the `mut` alone
//! (the leftover is rustc's machine-applicable `unused_mut`, never a
//! break). Wrong fixes on BOTH halves are check-visible compile errors
//! (E0596/E0384) the `--verify` gate auto-reverts — the tier argument.

use super::{edit, SourceEdit};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, File, Item, Visibility};

/// std methods that take `&self` on every std type that has them (the
/// `needless_borrow` receiver-whitelist precedent). A custom type overriding
/// one with a `&mut self` receiver is the documented divergence — E0596,
/// revertable by the compile gate.
const SHARED_STD_METHODS: &[&str] = &[
    "as_slice", "as_str", "as_deref", "as_ptr", "capacity", "clone", "contains", "first", "get",
    "is_empty", "iter", "keys", "last", "len", "to_string", "to_vec", "values",
];

/// One same-file method declaration (any impl block, inherent or trait).
struct MethodDecl {
    name: String,
    receiver_is_shared: bool,
}

/// A `let mut <ident>` binding candidate for the caller cascade.
struct CascadeCandidate {
    name: String,
    /// The binding's own `Pat::Ident` span (`mut x`) — excluded from the
    /// use scan, the edit target.
    pat_range: std::ops::Range<usize>,
    /// The enclosing fn item's byte range — the complete use set of a
    /// local binding (locals are fn-scoped).
    fn_range: std::ops::Range<usize>,
    /// Receiver-of-fixed-method mentions seen inside `fn_range`.
    receiver_hits: usize,
    /// Set when ANY other mention shape appeared — kills the cascade edit.
    used_otherwise: bool,
}

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(_source: &str, file: &File) -> Vec<SourceEdit> {
    let table = method_table(file);

    // Pass 1 — definition edits (inherent impls only).
    let mut edits: Vec<SourceEdit> = Vec::new();
    let mut fixed_names: Vec<String> = Vec::new();
    for_each_inherent_method(file, &mut |m| {
        let Some(recv) = eligible_receiver(m) else {
            return;
        };
        if !body_is_shared_only(&m.block, &table) {
            return;
        }
        edits.push(edit(recv, "&self".to_string()));
        fixed_names.push(m.sig.ident.to_string());
    });
    if edits.is_empty() {
        return edits;
    }

    // Pass 2 — the caller cascade (only when a definition edit landed).
    let mut candidates = cascade_candidates(file);
    if !candidates.is_empty() {
        let receiver_uses = fixed_receiver_uses(file, &fixed_names);
        let mut scan = CascadeScan {
            candidates: &mut candidates,
            receiver_uses: &receiver_uses,
        };
        scan.visit_file(file);
        edits.extend(cascade_edits(&candidates, &receiver_uses, &table));
    }
    edits
}

/// Walk every method of every INHERENT impl (`trait_` is None), incl. items
/// nested in inline `mod` blocks.
fn for_each_inherent_method(file: &File, f: &mut impl FnMut(&syn::ImplItemFn)) {
    fn walk_items(items: &[Item], f: &mut impl FnMut(&syn::ImplItemFn)) {
        for item in items {
            match item {
                Item::Impl(imp) if imp.trait_.is_none() => {
                    for ii in &imp.items {
                        if let syn::ImplItem::Fn(m) = ii {
                            f(m);
                        }
                    }
                }
                Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        walk_items(inner, f);
                    }
                }
                _ => {}
            }
        }
    }
    walk_items(&file.items, f);
}

/// Walk ALL impl methods (inherent + trait) into the same-file table.
fn method_table(file: &File) -> Vec<MethodDecl> {
    fn walk_items(items: &[Item], table: &mut Vec<MethodDecl>) {
        for item in items {
            match item {
                Item::Impl(imp) => {
                    for ii in &imp.items {
                        if let syn::ImplItem::Fn(m) = ii {
                            let shared = matches!(
                                m.sig.inputs.first(),
                                Some(syn::FnArg::Receiver(r))
                                    if r.reference.is_some() && r.mutability.is_none()
                            );
                            table.push(MethodDecl {
                                name: m.sig.ident.to_string(),
                                receiver_is_shared: shared,
                            });
                        }
                    }
                }
                Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        walk_items(inner, table);
                    }
                }
                _ => {}
            }
        }
    }
    let mut table = Vec::new();
    walk_items(&file.items, &mut table);
    table
}

/// Is `m` eligible for the definition edit, returning its receiver node?
/// Inherent impl + non-`pub` visibility (private or exactly `pub(crate)`,
/// the probed firing line) + plain `&mut self` receiver + a plain fn
/// (attrs / async / const / unsafe / extern / variadic all refuse).
fn eligible_receiver(m: &syn::ImplItemFn) -> Option<&syn::Receiver> {
    if !m.attrs.is_empty() || m.defaultness.is_some() {
        return None;
    }
    let vis_ok = match &m.vis {
        Visibility::Inherited => true,
        Visibility::Restricted(r) => r.in_token.is_none() && r.path.is_ident("crate"),
        Visibility::Public(_) => false,
    };
    if !vis_ok {
        return None;
    }
    let sig = &m.sig;
    if sig.asyncness.is_some()
        || sig.constness.is_some()
        || sig.unsafety.is_some()
        || sig.abi.is_some()
        || sig.variadic.is_some()
    {
        return None;
    }
    match sig.inputs.first() {
        Some(syn::FnArg::Receiver(r)) => {
            // Plain `&mut self` only (syn 2: `reference` + `mutability` —
            // the tuple's inner `mut` slot stays None for receivers): a
            // typed `self: &mut Self` receiver (colon form) refuses v1.
            if r.reference.is_some()
                && r.mutability.is_some()
                && r.colon_token.is_none()
                && r.attrs.is_empty()
            {
                Some(r)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Does the method body use `self` only through shared borrows?
fn body_is_shared_only(block: &syn::Block, table: &[MethodDecl]) -> bool {
    let mut scan = SelfUseScan { table, refused: false };
    scan.visit_block(block);
    !scan.refused
}

struct SelfUseScan<'a> {
    table: &'a [MethodDecl],
    refused: bool,
}

impl<'ast> Visit<'ast> for SelfUseScan<'_> {
    /// Method calls rooted at `self`: allowed iff allowlisted or resolvable
    /// to exactly one same-file `&self` declaration. Allowed receivers are
    /// NOT descended into (their base chain is a pure self-read and the
    /// callee is settled) — only the args are visited.
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if root_is_self(&m.receiver) {
            if self.callee_allowed(&m.method) {
                for a in &m.args {
                    self.visit_expr(a);
                }
                return;
            }
            self.refused = true;
        }
        syn::visit::visit_expr_method_call(self, m);
    }

    /// A `self`-rooted field/index chain in read position is a shared read.
    fn visit_expr_field(&mut self, f: &'ast syn::ExprField) {
        if root_is_self(&f.base) {
            return;
        }
        syn::visit::visit_expr_field(self, f);
    }
    fn visit_expr_index(&mut self, i: &'ast syn::ExprIndex) {
        if root_is_self(&i.expr) {
            return self.visit_expr(&i.index);
        }
        syn::visit::visit_expr_index(self, i);
    }

    /// Assignments / compound assignments whose LHS starts at `self`.
    fn visit_expr_assign(&mut self, a: &'ast syn::ExprAssign) {
        if starts_at_self(&a.left) {
            self.refused = true;
        }
        syn::visit::visit_expr_assign(self, a);
    }
    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        use syn::BinOp;
        if matches!(
            b.op,
            BinOp::AddAssign(_)
                | BinOp::SubAssign(_)
                | BinOp::MulAssign(_)
                | BinOp::DivAssign(_)
                | BinOp::RemAssign(_)
                | BinOp::BitXorAssign(_)
                | BinOp::BitAndAssign(_)
                | BinOp::BitOrAssign(_)
                | BinOp::ShlAssign(_)
                | BinOp::ShrAssign(_)
        ) && starts_at_self(&b.left)
        {
            self.refused = true;
        }
        syn::visit::visit_expr_binary(self, b);
    }

    /// Any `&mut` operand mentioning `self` anywhere (`&mut self.x`,
    /// `&mut *self`) is a mutable reborrow.
    fn visit_expr_reference(&mut self, r: &'ast syn::ExprReference) {
        if r.mutability.is_some() {
            let mut seek = SelfMention { found: false };
            syn::visit::Visit::visit_expr(&mut seek, &r.expr);
            if seek.found {
                self.refused = true;
            }
        }
        syn::visit::visit_expr_reference(self, r);
    }

    /// Macro bodies are invisible — any macro whose tokens mention `self`
    /// could be mutating it (`write!(self.buf, ..)`).
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if mac.tokens.to_string().contains("self") {
            self.refused = true;
        }
        syn::visit::visit_macro(self, mac);
    }

    /// A bare `self` path in any other position (call arg, comparison,
    /// deref read) — conservative refuse; the receiver/field/index skips
    /// above keep the shared reads clean.
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path.is_ident("self") {
            self.refused = true;
        }
        syn::visit::visit_expr_path(self, p);
    }
}

impl SelfUseScan<'_> {
    fn callee_allowed(&self, name: &syn::Ident) -> bool {
        if SHARED_STD_METHODS.iter().any(|m| name == *m) {
            return true;
        }
        let mut hits = self
            .table
            .iter()
            .filter(|d| name == d.name.as_str());
        // Exactly one same-file declaration, and it takes `&self`.
        matches!((hits.next(), hits.next()), (Some(d), None) if d.receiver_is_shared)
    }
}

/// Does the expression chain bottom out at the bare `self` path
/// (`self`, `(self)`, `self.f.g`, `self[i]`, …)?
fn root_is_self(e: &Expr) -> bool {
    match e {
        Expr::Path(p) => p.path.is_ident("self"),
        Expr::Paren(p) => root_is_self(&p.expr),
        Expr::Field(f) => root_is_self(&f.base),
        Expr::Index(i) => root_is_self(&i.expr),
        _ => false,
    }
}

fn starts_at_self(e: &Expr) -> bool {
    root_is_self(e)
}

struct SelfMention {
    found: bool,
}

impl<'ast> Visit<'ast> for SelfMention {
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path.is_ident("self") {
            self.found = true;
        }
        syn::visit::visit_expr_path(self, p);
    }
}

/// Collect `let mut <ident>` cascade candidates with their enclosing fn
/// range (bindings are fn-scoped — that range is the complete use set).
fn cascade_candidates(file: &File) -> Vec<CascadeCandidate> {
    fn walk_items(items: &[Item], out: &mut Vec<CascadeCandidate>) {
        for item in items {
            match item {
                Item::Fn(f) => {
                    collect_locals(&f.block, &f.span().byte_range(), out);
                }
                Item::Impl(imp) => {
                    for ii in &imp.items {
                        if let syn::ImplItem::Fn(m) = ii {
                            collect_locals(&m.block, &m.span().byte_range(), out);
                        }
                    }
                }
                Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        walk_items(inner, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk_items(&file.items, &mut out);
    out
}

fn collect_locals(
    block: &syn::Block,
    fn_range: &std::ops::Range<usize>,
    out: &mut Vec<CascadeCandidate>,
) {
    let mut v = LocalSeek {
        fn_range: fn_range.clone(),
        out,
    };
    v.visit_block(block);
}

struct LocalSeek<'a> {
    fn_range: std::ops::Range<usize>,
    out: &'a mut Vec<CascadeCandidate>,
}

impl<'ast> Visit<'ast> for LocalSeek<'_> {
    fn visit_local(&mut self, l: &'ast syn::Local) {
        if l.attrs.is_empty() && l.init.as_ref().is_some_and(|i| i.diverge.is_none()) {
            if let Some(p) = plain_ident_pat(&l.pat) {
                if p.mutability.is_some() {
                    self.out.push(CascadeCandidate {
                        name: p.ident.to_string(),
                        pat_range: p.span().byte_range(),
                        fn_range: self.fn_range.clone(),
                        receiver_hits: 0,
                        used_otherwise: false,
                    });
                }
            }
        }
        syn::visit::visit_local(self, l);
    }
}

/// The plain (possibly type-wrapped) ident pattern of a local, if any.
fn plain_ident_pat(pat: &syn::Pat) -> Option<&syn::PatIdent> {
    match pat {
        syn::Pat::Ident(p) => Some(p),
        syn::Pat::Type(t) => match &*t.pat {
            syn::Pat::Ident(p) => Some(p),
            _ => None,
        },
        _ => None,
    }
}

/// One `<ident>.<method>()` receiver occurrence: (path span, ident, method).
struct ReceiverUse {
    path_range: std::ops::Range<usize>,
    name: String,
    method: String,
}

/// Record every method call whose receiver is a bare path, filtered to the
/// methods this pass fixed — the cascade's use-classification input.
fn fixed_receiver_uses(file: &File, fixed: &[String]) -> Vec<ReceiverUse> {
    let mut v = ReceiverScan { out: Vec::new() };
    v.visit_file(file);
    v.out
        .into_iter()
        .filter(|r| fixed.contains(&r.method))
        .collect()
}

struct ReceiverScan {
    out: Vec<ReceiverUse>,
}

impl<'ast> Visit<'ast> for ReceiverScan {
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if let Expr::Path(p) = &*m.receiver {
            if let Some(id) = p.path.get_ident() {
                self.out.push(ReceiverUse {
                    path_range: p.span().byte_range(),
                    name: id.to_string(),
                    method: m.method.to_string(),
                });
            }
        }
        syn::visit::visit_expr_method_call(self, m);
    }
}

/// The cascade verdict walk: per candidate, count fixed-method receiver
/// mentions inside its fn range and refuse on ANY other mention shape
/// (including same-name re-lets — shadowing).
struct CascadeScan<'a> {
    candidates: &'a mut [CascadeCandidate],
    receiver_uses: &'a [ReceiverUse],
}

impl<'ast> Visit<'ast> for CascadeScan<'_> {
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if let Some(id) = p.path.get_ident() {
            let range = p.span().byte_range();
            let is_fixed_receiver = self.receiver_uses.iter().any(|r| r.path_range == range);
            for c in &mut *self.candidates {
                if id != c.name.as_str()
                    || !c.fn_range.contains(&range.start)
                    || range == c.pat_range
                {
                    continue;
                }
                if is_fixed_receiver {
                    c.receiver_hits += 1;
                } else {
                    c.used_otherwise = true;
                }
            }
        }
        syn::visit::visit_expr_path(self, p);
    }

    fn visit_local(&mut self, l: &'ast syn::Local) {
        let range = l.pat.span().byte_range();
        if let Some(pat) = plain_ident_pat(&l.pat) {
            let name = pat.ident.to_string();
            for c in &mut *self.candidates {
                if c.name == name && range != c.pat_range && c.fn_range.contains(&range.start) {
                    // A same-name re-let inside the fn — shadowing: refuse.
                    c.used_otherwise = true;
                }
            }
        }
        syn::visit::visit_local(self, l);
    }
}

/// Emit the binding-`mut` strip for every candidate that survived the walk:
/// ≥1 fixed-method receiver use, no other mention, and every called method
/// name declared exactly once in the file (no cross-type ambiguity).
fn cascade_edits(
    candidates: &[CascadeCandidate],
    receiver_uses: &[ReceiverUse],
    table: &[MethodDecl],
) -> Vec<SourceEdit> {
    let mut edits = Vec::new();
    for c in candidates {
        if c.receiver_hits == 0 || c.used_otherwise {
            continue;
        }
        let own = receiver_uses
            .iter()
            .filter(|r| r.name == c.name && c.fn_range.contains(&r.path_range.start));
        let unique = own.clone().all(|r| {
            table.iter().filter(|d| d.name == r.method).count() == 1
        });
        if unique {
            edits.push(SourceEdit {
                start: c.pat_range.start,
                end: c.pat_range.end,
                replacement: c.name.clone(),
            });
        }
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    const IMPL_HEAD: &str = "struct Probe { hits: u32, label: String }\nimpl Probe {\n";

    #[test]
    fn fires_on_private_read_only_method() {
        let input = format!("{IMPL_HEAD}    fn read(&mut self) -> u32 {{ self.hits }}\n}}\n");
        let out = propose_fix("needless_pass_by_ref_mut", &input).expect("matched");
        assert!(out.contains("fn read(&self)"), "no receiver fix: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn cascade_drops_caller_mut() {
        let input = format!(
            "{IMPL_HEAD}    fn read(&mut self) -> u32 {{ self.hits }}\n    fn run() -> u32 {{ let mut p = Probe {{ hits: 1, label: String::new() }}; p.read() }}\n}}\n"
        );
        let out = propose_fix("needless_pass_by_ref_mut", &input).expect("matched");
        assert!(out.contains("fn read(&self)"), "no receiver fix: {out}");
        assert!(out.contains("let p = Probe"), "no cascade: {out}");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn cascade_refused_when_binding_has_other_use() {
        // `p.bump()` is an unfixed &mut method — the binding keeps its mut.
        let input = format!(
            "{IMPL_HEAD}    fn read(&mut self) -> u32 {{ self.hits }}\n    fn bump(&mut self) {{ self.hits += 1; }}\n    fn run() -> u32 {{ let mut p = Probe {{ hits: 1, label: String::new() }}; p.bump(); p.read() }}\n}}\n"
        );
        let out = propose_fix("needless_pass_by_ref_mut", &input).expect("matched");
        assert!(out.contains("fn read(&self)"), "no receiver fix: {out}");
        assert!(out.contains("let mut p"), "cascade must refuse: {out}");
    }

    #[test]
    fn cascade_refused_on_field_read_use() {
        // `p.hits` read — outside the narrow all-receiver-calls shape.
        let input = format!(
            "{IMPL_HEAD}    fn read(&mut self) -> u32 {{ self.hits }}\n    fn run() -> u32 {{ let mut p = Probe {{ hits: 1, label: String::new() }}; let n = p.hits; p.read() + n }}\n}}\n"
        );
        let out = propose_fix("needless_pass_by_ref_mut", &input).expect("matched");
        assert!(out.contains("let mut p"), "cascade must refuse: {out}");
    }

    #[test]
    fn cascade_refused_on_shadowing_relet() {
        let input = format!(
            "{IMPL_HEAD}    fn read(&mut self) -> u32 {{ self.hits }}\n    fn run() -> u32 {{ let mut p = Probe {{ hits: 1, label: String::new() }}; let r = p.read(); let p = r; p }}\n}}\n"
        );
        let out = propose_fix("needless_pass_by_ref_mut", &input).expect("matched");
        assert!(out.contains("let mut p"), "shadowed binding must refuse: {out}");
    }

    #[test]
    fn pub_method_silent() {
        let input = format!("{IMPL_HEAD}    pub fn read(&mut self) -> u32 {{ self.hits }}\n}}\n");
        let out = propose_fix("needless_pass_by_ref_mut", &input);
        assert!(out.is_none(), "pub method must stay silent: {out:?}");
    }

    #[test]
    fn pub_crate_fires() {
        let input = format!(
            "{IMPL_HEAD}    pub(crate) fn read(&mut self) -> u32 {{ self.hits }}\n}}\n"
        );
        let out = propose_fix("needless_pass_by_ref_mut", &input).expect("matched");
        assert!(
            out.contains("pub(crate) fn read(&self)"),
            "pub(crate) must fire: {out}"
        );
    }

    #[test]
    fn trait_impl_silent() {
        let input = "pub trait T { fn work(&mut self) -> u32; }\nstruct P { v: u32 }\nimpl T for P {\n    fn work(&mut self) -> u32 { self.v }\n}\n";
        let out = propose_fix("needless_pass_by_ref_mut", input);
        assert!(out.is_none(), "trait impl must stay silent: {out:?}");
    }

    #[test]
    fn assignment_body_silent() {
        let input = format!("{IMPL_HEAD}    fn set(&mut self, v: u32) {{ self.hits = v; }}\n}}\n");
        let out = propose_fix("needless_pass_by_ref_mut", &input);
        assert!(out.is_none(), "assigning body must stay silent: {out:?}");
    }

    #[test]
    fn mut_reborrow_body_silent() {
        let input = "struct P { v: Vec<u32> }\nimpl P {\n    fn drain(&mut self) { touch(&mut self.v); }\n}\nfn touch(v: &mut Vec<u32>) { v.push(1); }\n";
        let out = propose_fix("needless_pass_by_ref_mut", input);
        assert!(out.is_none(), "&mut reborrow must stay silent: {out:?}");
    }

    #[test]
    fn mut_callee_body_silent() {
        let input = format!(
            "{IMPL_HEAD}    fn bump(&mut self) {{ self.hits += 1; }}\n    fn refresh(&mut self) {{ self.bump(); }}\n}}\n"
        );
        let out = propose_fix("needless_pass_by_ref_mut", &input);
        assert!(out.is_none(), "&mut callee must stay silent: {out:?}");
    }

    #[test]
    fn shared_callee_body_fires() {
        // The probe's f_priv_call shape: self.lookup() where lookup is &self.
        let input = format!(
            "{IMPL_HEAD}    fn lookup(&self) -> u32 {{ self.hits }}\n    fn base_plus(&mut self, x: u32) -> u32 {{ self.lookup() + x }}\n}}\n"
        );
        let out = propose_fix("needless_pass_by_ref_mut", &input).expect("matched");
        assert!(
            out.contains("fn base_plus(&self,"),
            "shared callee must fire: {out}"
        );
    }

    #[test]
    fn allowlisted_field_chain_fires() {
        // The probe's d_priv_label shape: self.label.clone().
        let input = format!(
            "{IMPL_HEAD}    fn label_clone(&mut self) -> String {{ self.label.clone() }}\n}}\n"
        );
        let out = propose_fix("needless_pass_by_ref_mut", &input).expect("matched");
        assert!(
            out.contains("fn label_clone(&self)"),
            "allowlisted chain must fire: {out}"
        );
    }

    #[test]
    fn unknown_callee_body_silent() {
        let input = format!("{IMPL_HEAD}    fn render(&mut self) {{ self.sketch(); }}\n}}\n");
        let out = propose_fix("needless_pass_by_ref_mut", &input);
        assert!(out.is_none(), "unknown callee must stay silent: {out:?}");
    }

    #[test]
    fn ambiguous_callee_silent() {
        // `run` declared twice in-file (two impls) — resolution ambiguous.
        let input = "struct A { v: u32 }\nstruct B { v: u32 }\nimpl A {\n    fn run(&mut self) { self.tick(); }\n    fn tick(&mut self) { self.v += 1; }\n}\nimpl B {\n    fn run(&mut self) { self.v += 1; }\n}\n";
        let out = propose_fix("needless_pass_by_ref_mut", input);
        assert!(out.is_none(), "ambiguous callee must stay silent: {out:?}");
    }

    #[test]
    fn macro_mentioning_self_silent() {
        let input = "macro_rules! bump { ($s:expr) => { $s.hits += 1; } }\nstruct P { hits: u32 }\nimpl P {\n    fn tick(&mut self) { bump!(self); }\n}\n";
        let out = propose_fix("needless_pass_by_ref_mut", input);
        assert!(
            out.is_none(),
            "self-mentioning macro must stay silent: {out:?}"
        );
    }

    #[test]
    fn bare_self_arg_silent() {
        let input = "struct P { hits: u32 }\nimpl P {\n    fn other(p: &mut P) -> u32 { p.hits }\n    fn tick(&mut self) -> u32 { Self::other(self) }\n}\n";
        let out = propose_fix("needless_pass_by_ref_mut", input);
        assert!(out.is_none(), "bare self arg must stay silent: {out:?}");
    }

    #[test]
    fn async_and_attributed_silent() {
        let input = "struct P { hits: u32 }\nimpl P {\n    async fn peek(&mut self) -> u32 { self.hits }\n    #[inline]\n    fn poke(&mut self) -> u32 { self.hits }\n}\n";
        let out = propose_fix("needless_pass_by_ref_mut", input);
        assert!(out.is_none(), "async/attributed must stay silent: {out:?}");
    }

    #[test]
    fn let_else_binding_no_cascade() {
        let input = format!(
            "{IMPL_HEAD}    fn read(&mut self) -> u32 {{ self.hits }}\n    fn run() -> u32 {{ let mut p = Probe {{ hits: 1, label: String::new() }}; p.read() }}\n    fn alt() -> u32 {{ let Some(p) = Some(Probe {{ hits: 2, label: String::new() }}) else {{ return 0 }}; p.read() }}\n}}\n"
        );
        let out = propose_fix("needless_pass_by_ref_mut", &input).expect("matched");
        // The let-else binding carries no `mut` to strip; run() still cascades.
        assert!(out.contains("let p = Probe"), "clean cascade must fire: {out}");
    }
}
