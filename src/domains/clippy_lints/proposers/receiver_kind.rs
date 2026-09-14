//! Shared Option/Result receiver classification for the bool-terminal-form
//! proposers (`map_unwrap_or`, `unnecessary_map_or`).
//!
//! Extracted from `map_unwrap_or.rs` (2026-09-14, P15) when the born-lint
//! closure gained its own consumer: the healer's own `map_or(false, f)`
//! fallback output is re-flagged by real clippy as `unnecessary_map_or`, and
//! BOTH rules need the same parse-only receiver-kind analysis to pick the
//! type-correct terminal form (`is_some_and`/`is_none_or` are Option-only,
//! `is_ok_and` is `Result`-only — the wrong pick is a hard E0599, the
//! `bench_440` `env::var` catch).
//!
//! Three classification sources, in precedence order:
//!
//! 1. **Fn-parameter types** — `fn f(x: Option<u8>)` makes the plain-path
//!    receiver `x` `OptionLike` by the parse tree's own annotation (exact).
//! 2. **Safe-let initializers** (the 2026-09-14 extension, this module's
//!    addition): `let best = v.iter().find(..); best.map_or(false, f)` —
//!    the born-lint production shape (seal-game-editor `e2112bce`, the
//!    receiver was type-UNKNOWN under the v1 pass). SOUND by construction:
//!    a binding qualifies only when the name is bound EXACTLY ONCE in the
//!    whole fn (no shadowing `let`/match/for/closure binding anywhere), is
//!    never assigned (`x = ..`, compound assigns), and is not a param name —
//!    so every bare use of the name in that fn refers to the one binding,
//!    whose type is by definition its initializer's inferred type. A Rust
//!    binding has ONE static type; assignments are guarded out; mutation
//!    through `&mut`/`mem::take` preserves the declared type.
//! 3. **Receiver text shapes** — last-match-wins allowlists (`.get(` …
//!    `OptionLike`; `env::var(`, `.parse(`, `.lock(` … `ResultLike`) plus the
//!    same-file BARE-CALL map (`git_out(..)` → the fn's `-> Option<T>`).
//!
//! Known limitation (documented, unchanged from the v1 substrate): a name
//! that is BOTH a param and a shadowing `let` keeps the PARAM kind
//! (position-blind); the safe-let guard refuses the collision so the new
//! source never makes this worse. Macro-token assignments are invisible to
//! syn (the standing macro-argument divergence class) — the `--verify`
//! compile gate and the oracle feed back the rare miss.

use std::collections::HashMap;

use syn::visit::Visit;
use syn::{BinOp, Block, Expr, File, Local, Pat, Signature};

/// Parse-only receiver classification for the bool-literal terminal forms.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ReceiverKind {
    OptionLike,
    ResultLike,
    Unknown,
}

/// One enclosing fn's name→kind bindings (params + safe-lets). Collectors
/// keep a stack; `classify_receiver` consults the innermost.
#[derive(Debug, Default, Clone)]
pub(crate) struct ScopeKinds {
    pub(crate) params: HashMap<String, ReceiverKind>,
    pub(crate) let_kinds: HashMap<String, ReceiverKind>,
}

impl ScopeKinds {
    /// Build a fn's scope: exact param classifications + the sound safe-let
    /// classifications. `fn_rets` is the file-wide pre-pass map (later fns
    /// included) for bare-call initializer classification.
    pub(crate) fn from_fn(
        sig: &Signature,
        block: &Block,
        source: &str,
        fn_rets: &HashMap<String, ReceiverKind>,
    ) -> Self {
        let params = fn_param_kinds(sig);
        let let_kinds = collect_safe_lets(block, source, &params, fn_rets);
        Self { params, let_kinds }
    }
}

/// File-wide `fn name → return-type kind` map, built BEFORE the main visit
/// (a forward call to a later-defined fn classifies — the v1 during-visit
/// collection missed every forward reference).
#[must_use]
pub(crate) fn collect_fn_rets(file: &File) -> HashMap<String, ReceiverKind> {
    struct RetScan {
        map: HashMap<String, ReceiverKind>,
    }
    impl<'ast> Visit<'ast> for RetScan {
        fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
            if let Some(kind) = fn_ret_kind(&f.sig) {
                self.map.insert(f.sig.ident.to_string(), kind);
            }
            syn::visit::visit_item_fn(self, f);
        }
        fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
            if let Some(kind) = fn_ret_kind(&f.sig) {
                self.map.insert(f.sig.ident.to_string(), kind);
            }
            syn::visit::visit_impl_item_fn(self, f);
        }
    }
    let mut v = RetScan {
        map: HashMap::new(),
    };
    v.visit_file(file);
    v.map
}

fn fn_ret_kind(sig: &Signature) -> Option<ReceiverKind> {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return None;
    };
    param_type_kind(ty)
}

/// Extract Option/Result classification from a fn signature's parameter
/// types. Only simple `ident: Type` patterns (incl. `&`-wrapped) are
/// recorded — complex destructuring patterns are skipped (Unknown).
fn fn_param_kinds(sig: &Signature) -> HashMap<String, ReceiverKind> {
    let mut out = HashMap::with_capacity(sig.inputs.len());
    for arg in &sig.inputs {
        let syn::FnArg::Typed(pt) = arg else { continue };
        let syn::Pat::Ident(id) = &*pt.pat else {
            continue;
        };
        let kind = param_type_kind(&pt.ty);
        if let Some(kind) = kind {
            out.insert(id.ident.to_string(), kind);
        }
    }
    out
}

fn param_type_kind(ty: &syn::Type) -> Option<ReceiverKind> {
    let ty = match ty {
        syn::Type::Reference(r) => &*r.elem,
        _ => ty,
    };
    let syn::Type::Path(tp) = ty else { return None };
    // Last path segment: `Option` / `Result` bare or `std::option::Option`.
    // Generic args are ignored — `Option<T>` is OptionLike for every T.
    match tp
        .path
        .segments
        .last()
        .map(|s| s.ident.to_string())
        .as_deref()
    {
        Some("Option") => Some(ReceiverKind::OptionLike),
        Some("Result") => Some(ReceiverKind::ResultLike),
        _ => None,
    }
}

/// Classify a receiver of the shape `callee(..)` / `path::callee(..)`: extract
/// the callee's final ident (scanning back from the first `(` over the
/// `::`-qualified segments), then look it up in the same-file fn-return map.
/// None unless the text actually IS a call (an ident chain directly followed
/// by `(`) — method-call receivers and struct literals stay out of scope.
fn bare_call_receiver_kind(
    recv: &str,
    fn_rets: Option<&HashMap<String, ReceiverKind>>,
) -> Option<ReceiverKind> {
    let map = fn_rets?;
    let open = recv.find('(')?;
    let head = &recv[..open];
    let head = head.trim_end();
    if head.is_empty() || head.contains('.') || head.contains('(') {
        return None;
    }
    let callee = head.rsplit("::").next()?.trim();
    if callee.is_empty() || !callee.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    map.get(callee).copied()
}

/// Classify a receiver for the terminal forms. Last-match-wins over the
/// receiver text: a chain like `env::var(k).ok()` is Option-producing
/// (`.ok()` converts to Option), so the OUTERMOST (last) known-shape call
/// decides. Evidence-based allowlists: Option shapes are the dominant
/// `.get(`/`.find(` accessors; Result shapes are the std APIs seen in
/// production heals (`env::var`, `.parse`).
pub(crate) fn classify_receiver(
    recv: &str,
    scope: Option<&ScopeKinds>,
    fn_rets: Option<&HashMap<String, ReceiverKind>>,
) -> ReceiverKind {
    const OPTION_SHAPES: [&str; 7] = [
        ".get(", ".find(", ".first(", ".last(", ".pop()", ".next()", ".ok()",
    ];
    // `.lock(`: std Mutex/RwLock `lock()` → `LockResult` (a Result alias) —
    // evidence: 6 production sites in one heal sweep (riir-mmorpg-examples
    // karma/party mirrors, 2026-08-28: `.lock().map(bool-f).unwrap_or(false)`
    // left `map_or` residue that clippy's type info flagged as `is_ok_and`).
    const RESULT_SHAPES: [&str; 4] = ["env::var(", ".parse(", ".parse::<", ".lock("];
    // A plain-path receiver typed by an enclosing fn scope: the parse tree's
    // own annotation is authoritative (`fn f(x: Option<u8>)`; then a safe
    // `let` binding — see the module doc's soundness argument).
    if !recv.contains('.') && !recv.contains('(') {
        if let Some(scope) = scope {
            if let Some(kind) = scope.params.get(recv.trim()) {
                return *kind;
            }
            if let Some(kind) = scope.let_kinds.get(recv.trim()) {
                return *kind;
            }
        }
        return ReceiverKind::Unknown;
    }
    // A BARE-CALL receiver typed by a same-file fn signature
    // (`git_out(..).map(f).unwrap_or(false)` — the build-script shape:
    // build.rs:33 recurred 2026-09-04 after the riir-examples 08-19 miss).
    // The callee ident immediately before the FIRST `(` is looked up in the
    // same-file fn-return map; method-call receivers (leading `.`) and
    // foreign callees stay Unknown. Params still outrank this (a same-named
    // local param is the tighter scope); a shadowing `let` binding would
    // too in real rustc — out of scope for a parse-only pass, and the
    // --verify compile gate reverts any E0599 that slips through.
    if let Some(kind) = bare_call_receiver_kind(recv, fn_rets) {
        return kind;
    }
    let mut best: Option<(usize, ReceiverKind)> = None;
    for shape in OPTION_SHAPES {
        if let Some(pos) = rfind_shape(recv, shape) {
            if best.is_none_or(|(p, _)| pos > p) {
                best = Some((pos, ReceiverKind::OptionLike));
            }
        }
    }
    for shape in RESULT_SHAPES {
        if let Some(pos) = rfind_shape(recv, shape) {
            if best.is_none_or(|(p, _)| pos > p) {
                best = Some((pos, ReceiverKind::ResultLike));
            }
        }
    }
    best.map_or(ReceiverKind::Unknown, |(_, kind)| kind)
}

/// `rfind` for a shape allowlist entry, ALSO matching the turbofish call
/// form: `.get(` additionally matches `.get::<T>(` — the paren-suffixed
/// shape with an explicit generic argument (`store.get::<ThinkTimer>(id)`,
/// live residue in riir-mmorpg-examples projector.rs:287, 2026-08-28: the
/// plain `.get(` text never matches `.get::<`, so the receiver fell to
/// Unknown and the terminal `is_some_and` was withheld). The generic text
/// itself is unmatchable from a fixed allowlist; the prefix + `::<` is the
/// sound anchor. Positions stay comparable (the prefix starts where the
/// plain shape starts), so last-match-wins ordering is unaffected.
fn rfind_shape(recv: &str, shape: &str) -> Option<usize> {
    if let Some(pos) = recv.rfind(shape) {
        return Some(pos);
    }
    if let Some(prefix) = shape.strip_suffix('(') {
        let turbofish = format!("{prefix}::<");
        return recv.rfind(&turbofish);
    }
    None
}

/// The safe-let collection (this module's extension): one pass counting
/// every binding occurrence + assignment target, recording candidate
/// single-ident non-`mut` lets, then filtering to names with exactly ONE
/// binding, ZERO assignments, no param collision, and a classifiable
/// initializer (shapes + bare-call + PARAM ALIAS only — no transitive
/// let-chaining, keeping the pass order-independent).
fn collect_safe_lets(
    block: &Block,
    source: &str,
    params: &HashMap<String, ReceiverKind>,
    fn_rets: &HashMap<String, ReceiverKind>,
) -> HashMap<String, ReceiverKind> {
    struct Scan<'a> {
        source: &'a str,
        binding_counts: HashMap<String, usize>,
        assigned: std::collections::HashSet<String>,
        candidates: Vec<(String, &'a Expr)>,
    }
    impl<'ast> Visit<'ast> for Scan<'ast> {
        fn visit_pat(&mut self, p: &'ast Pat) {
            if let Pat::Ident(id) = p {
                *self
                    .binding_counts
                    .entry(id.ident.to_string())
                    .or_insert(0) += 1;
            }
            syn::visit::visit_pat(self, p);
        }
        fn visit_local(&mut self, l: &'ast Local) {
            if let (Pat::Ident(id), Some(init)) = (&l.pat, l.init.as_ref()) {
                if id.mutability.is_none() {
                    self.candidates
                        .push((id.ident.to_string(), &*init.expr));
                }
            }
            syn::visit::visit_local(self, l);
        }
        fn visit_expr(&mut self, e: &'ast Expr) {
            match e {
                Expr::Assign(a) => {
                    if let Some(name) = assign_target_ident(&a.left) {
                        self.assigned.insert(name);
                    }
                }
                // Compound assignment (`+=`, `|=`, ..) — every BinOp variant
                // ending in `Assign`; enumerated so a plain `x + y` never
                // counts against candidate `x`.
                Expr::Binary(b) => {
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
                    ) {
                        if let Some(name) = assign_target_ident(&b.left) {
                            self.assigned.insert(name);
                        }
                    }
                }
                _ => {}
            }
            syn::visit::visit_expr(self, e);
        }
    }
    let mut v = Scan {
        source,
        binding_counts: HashMap::new(),
        assigned: std::collections::HashSet::new(),
        candidates: Vec::new(),
    };
    v.visit_block(block);

    let mut out = HashMap::with_capacity(v.candidates.len());
    for (name, init) in v.candidates {
        if v.binding_counts.get(&name).copied().unwrap_or(0) != 1 {
            continue; // shadowed/re-bound anywhere in the fn — not sound
        }
        if v.assigned.contains(&name) {
            continue; // re-assigned — type provenance could rotate
        }
        if params.contains_key(&name) {
            continue; // param collision — the param kind stands (position-blind)
        }
        let Some(text) = crate::edit_emitter::src(v.source, init) else {
            continue;
        };
        let alias_scope = ScopeKinds {
            params: params.clone(),
            let_kinds: HashMap::new(),
        };
        let kind = classify_receiver(&text, Some(&alias_scope), Some(fn_rets));
        if kind != ReceiverKind::Unknown {
            out.insert(name, kind);
        }
    }
    out
}

/// The assigned name of an assignment LHS: a bare ident (paren-unwrapped).
fn assign_target_ident(e: &Expr) -> Option<String> {
    match e {
        Expr::Path(p) => p.path.get_ident().map(ToString::to_string),
        Expr::Paren(p) => assign_target_ident(&p.expr),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds_of(src: &str) -> ScopeKinds {
        let file = syn::parse_file(src).expect("fixture parses");
        let rets = collect_fn_rets(&file);
        let mut first: Option<ScopeKinds> = None;
        let mut scan = FnScan {
            scope: &mut first,
            rets: &rets,
            src,
        };
        scan.visit_file(&file);
        first.unwrap_or_default()
    }

    struct FnScan<'a> {
        scope: &'a mut Option<ScopeKinds>,
        rets: &'a HashMap<String, ReceiverKind>,
        src: &'a str,
    }
    impl<'ast> Visit<'ast> for FnScan<'ast> {
        fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
            // Capture the FIRST fn's scope (multi-fn fixtures classify the
            // fn under test, which is written first).
            if self.scope.is_none() {
                *self.scope =
                    Some(ScopeKinds::from_fn(&f.sig, &f.block, self.src, self.rets));
            }
        }
    }

    #[test]
    fn safe_let_from_find_classifies_option() {
        let k = kinds_of("fn f(v: &[u8]) -> bool { let best = v.iter().find(|x| **x > 1); best.map_or(false, |x| *x > 2) }");
        assert_eq!(k.let_kinds.get("best"), Some(&ReceiverKind::OptionLike));
    }

    #[test]
    fn safe_let_from_param_alias_classifies() {
        let k = kinds_of("fn f(x: Option<u8>) -> bool { let best = x; best.map_or(false, |v| v > 1) }");
        assert_eq!(k.let_kinds.get("best"), Some(&ReceiverKind::OptionLike));
    }

    #[test]
    fn safe_let_from_forward_fn_call_classifies() {
        // The fn is defined AFTER the use site — the file-wide pre-pass sees it.
        let k = kinds_of(concat!(
            "fn use_it() -> bool { let best = pick(); best.map_or(false, |v| v > 1) }\n",
            "fn pick() -> Option<u8> { None }"
        ));
        assert_eq!(k.let_kinds.get("best"), Some(&ReceiverKind::OptionLike));
    }

    #[test]
    fn mutated_let_refused() {
        let k = kinds_of("fn f(v: &[u8]) -> bool { let mut best = v.iter().find(|x| **x > 1); best.map_or(false, |x| *x > 2) }");
        assert_eq!(k.let_kinds.get("best"), None);
    }

    #[test]
    fn reassigned_let_refused() {
        let k = kinds_of("fn f(a: &[u8], b: &[u8]) -> bool { let best = a.iter().find(|x| **x > 1); let other = b.iter().find(|x| **x > 1); best = other; best.map_or(false, |x| *x > 2) }");
        assert_eq!(k.let_kinds.get("best"), None);
        // `other` is fine (never reassigned) — the guard is per-name.
        assert_eq!(
            k.let_kinds.get("other"),
            Some(&ReceiverKind::OptionLike)
        );
    }

    #[test]
    fn shadowed_let_refused() {
        let k = kinds_of("fn f(v: &[u8]) -> bool { let best = v.iter().find(|x| **x > 1); let best = v.iter().find(|x| **x > 2); best.map_or(false, |x| *x > 3) }");
        assert_eq!(k.let_kinds.get("best"), None);
    }

    #[test]
    fn let_shadowing_a_match_binding_refused() {
        // A match arm binding of the same name counts — soundness guard.
        let k = kinds_of("fn f(v: &[u8]) -> bool { let best = v.iter().find(|x| **x > 1); match v.first() { Some(best) => best > &1, None => false } }");
        assert_eq!(k.let_kinds.get("best"), None);
    }

    #[test]
    fn param_collision_let_refused() {
        let k = kinds_of("fn f(x: Result<u8, ()>) -> bool { let x = std::env::var(\"K\"); x.map_or(false, |v| v.len() > 1) }");
        assert_eq!(k.let_kinds.get("x"), None);
        // The param kind still stands.
        assert_eq!(k.params.get("x"), Some(&ReceiverKind::ResultLike));
    }

    #[test]
    fn unclassifiable_init_refused() {
        // `max_by_key` is not in the shape allowlist (evidence-driven set) —
        // Unknown init → no entry (conservative, same as a foreign call).
        let k = kinds_of("fn f(v: &[u8]) -> bool { let best = v.iter().max_by_key(|x| *x); best.map_or(false, |x| *x > 2) }");
        assert_eq!(k.let_kinds.get("best"), None);
    }

    #[test]
    fn compound_assign_refused() {
        let k = kinds_of("fn f(v: &[u8], n: &mut u32) -> bool { let best = v.iter().find(|x| **x > 1); *n += 1; best.map_or(false, |x| *x > 2) }");
        assert_eq!(k.let_kinds.get("best"), Some(&ReceiverKind::OptionLike));
        assert!(!k.let_kinds.contains_key("n"));
    }

    #[test]
    fn classify_uses_let_kinds_after_params() {
        let src = "fn f(v: &[u8]) -> bool { let best = v.iter().find(|x| **x > 1); best.map_or(false, |x| *x > 2) }";
        let scope = kinds_of(src);
        assert_eq!(
            classify_receiver("best", Some(&scope), None),
            ReceiverKind::OptionLike
        );
        assert_eq!(
            classify_receiver("nope", Some(&scope), None),
            ReceiverKind::Unknown
        );
    }
}
