//! `untrusted-keyed-state-unbounded` — SUGGEST-ONLY detector (Batch 327,
//! the firezone/sanstun distill; Research 364 / Plan 332).
//!
//! A `HashMap`/`HashSet` keyed by PEER-DERIVED distinct keys (peer
//! vocabulary as word segments of the map variable/field, the key type,
//! or the enclosing fn: addr/ip/peer/session/device/tenant/origin/
//! client/conn/source) that grows via `insert` with NO cap guard admits
//! the many-keys memory `DoS` — one adversarial peer family minting
//! distinct keys exhausts memory. The CARDINALITY face of resource
//! exhaustion.
//!
//! ## The source anchor + the fix shape
//!
//! ratelimiter.rs carries the house answer: `const MAX_SOURCES: usize =
//! 65536;` and, before the insert, `if self.buckets.len() >= MAX_SOURCES
//! { return false; }` — a BOUNDED table + FAIL-CLOSED refusal of NEW
//! keys when full. Eviction would REWARD the attacker by admitting them
//! over honest entries; the source's own test name says it:
//! `full_table_refuses_new_sources_until_idle_ones_are_dropped`, and its
//! doc: `Packets from new sources are refused while the table is full.`
//!
//! ## Why suggest-only (never a fix)
//!
//! Choosing the cap and the full-table policy is a protocol decision:
//! refuse-new vs evict-idle vs shed-oldest changes what an attacker can
//! DO to honest tenants. NO `FIXABLE_SEC` membership, NO
//! `collect_edits` arm — the suggestion surfaces through retrieval and
//! review.
//!
//! ## Strict gates (refuse more than fires)
//!
//! - **Map proof**: the insert receiver resolves to a HashMap/HashSet
//!   declared IN THE SAME FILE — a local (`let mut sessions =
//!   HashMap::new()` or a typed annotation) or a struct field. A map
//!   typed in another module is an honest NO.
//! - **Peer-key vocabulary**: the map name, its key type path, or the
//!   enclosing fn name carries an EXACT word segment from
//!   addr/ip/peer/session/device/tenant/origin/client/conn/source
//!   (CamelCase splits — `ClientId` -> client + id; plural forms
//!   miss — `sessions` is not `session` — unless another source hits).
//! - **Suppression, FN-WIDE (the one-guard-hides-the-fn house rule)**:
//!   any `.len()` compared with `>` / `>=` against a cap-like side (an
//!   integer literal, or an ident carrying max/cap/limit), any
//!   `contains_key` call, or any `remove`/`retain`/`extract_if`/
//!   `drain`/`clear` in the fn silences EVERY insert in it.
//! - **Comment guard, house discipline**: `#[cfg(test)]` mods are
//!   skipped; macro statements are never descended into.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - `entry(..).or_insert(..)` growth is undetected — the `insert`
//!   method is the only shape.
//! - `with_capacity` is a capacity HINT, never a bound, and is
//!   deliberately NOT a suppression.
//! - The vocabulary proves a NAME CLASS, not a peer: a session-named
//!   map over trusted internal ids fires too, and review decides
//!   (suggest-only is the blast-radius control).
//! - Cross-fn growth (insert in a helper, map owned by a caller) is
//!   unconnected.
//! - Distinct-from `length-prefix-validate-before-alloc` and
//!   `unbounded-read-into-vec` IN-RULE: those own the VOLUME faces (one
//!   length field -> one alloc; one stream -> one growing buffer); this
//!   owns the CARDINALITY face (many distinct keys -> unbounded entry
//!   count).
//! - Batch 327 / Plan 332 / Research 364 (firezone/sanstun @
//!   `35cb1932948bd0776`, Apache-2.0 — cited as class provenance; the
//!   fixture is an original synthesis).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{BinOp, Expr, File, Item, Pat, Stmt, Type};

/// One detected unbounded peer-keyed insert site.
#[derive(Debug, Clone)]
pub struct KeyedStateFinding {
    /// Byte range of the `insert(..)` call.
    pub site_span: Range<usize>,
    /// The insert call's source text.
    pub site_text: String,
    /// The advice line (the bounded form + the fail-closed law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "peer-keyed state grows unbounded: one adversary \
minting distinct keys exhausts memory (the many-keys DoS). Bound the table \
and FAIL CLOSED when full — `if map.len() >= MAX { return false; }` before \
the insert; eviction would reward the attacker over honest entries \
(Batch 327, suggest-only)";

/// The peer-key vocabulary (exact word segments of the map name, the key
/// type, or the enclosing fn name).
const PEER_VOCAB: &[&str] = &[
    "addr", "ip", "peer", "session", "device", "tenant", "origin", "client", "conn", "source",
];

/// The drain-family methods: any one proves the fn already sheds entries.
const DRAIN_METHODS: &[&str] = &["remove", "retain", "extract_if", "drain", "clear"];

/// `(field-or-var name, key-type word segments)` for every same-file
/// HashMap/HashSet declaration.
type MapDecl = (String, Vec<String>);

/// Everything one fn body tells the adjudicator, collected in one walk.
#[derive(Default)]
struct FnSignals {
    local_maps: Vec<MapDecl>,
    len_cap_guard: bool,
    contains_key_guard: bool,
    drain_guard: bool,
    /// `(insert-call span, receiver's last segment)`.
    inserts: Vec<(Range<usize>, String)>,
}

/// Detect unbounded inserts into peer-keyed tables (suggest-only; no
/// edits are produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<KeyedStateFinding> {
    let mut field_maps = Vec::new();
    collect_field_maps(&file.items, &mut field_maps);
    let mut findings = Vec::new();
    scan_fns(source, &file.items, &field_maps, &mut findings);
    findings
}

fn collect_field_maps(items: &[Item], out: &mut Vec<MapDecl>) {
    for item in items {
        match item {
            Item::Struct(s) => {
                for field in &s.fields {
                    if let Some(key_segs) = map_key_segments(&field.ty) {
                        if let Some(ident) = &field.ident {
                            out.push((ident.to_string(), key_segs));
                        }
                    }
                }
            }
            Item::Mod(m) => {
                // Comment guard (house discipline): #[cfg(test)] mods
                // are fixture habitat.
                if is_test_mod(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_field_maps(nested, out);
                }
            }
            _ => {}
        }
    }
}

fn scan_fns(
    source: &str,
    items: &[Item],
    field_maps: &[MapDecl],
    findings: &mut Vec<KeyedStateFinding>,
) {
    for item in items {
        match item {
            Item::Fn(f) => scan_fn(
                source,
                &f.sig.ident.to_string(),
                &f.block,
                field_maps,
                findings,
            ),
            Item::Impl(imp) => {
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(m) = ii {
                        scan_fn(
                            source,
                            &m.sig.ident.to_string(),
                            &m.block,
                            field_maps,
                            findings,
                        );
                    }
                }
            }
            Item::Mod(m) => {
                if is_test_mod(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    scan_fns(source, nested, field_maps, findings);
                }
            }
            _ => {}
        }
    }
}

fn scan_fn(
    source: &str,
    fn_name: &str,
    block: &syn::Block,
    field_maps: &[MapDecl],
    findings: &mut Vec<KeyedStateFinding>,
) {
    let mut sig = FnSignals::default();
    collect_signals(&block.stmts, &mut sig);
    if sig.inserts.is_empty() {
        return;
    }
    // One guard hides the fn — the house conservative rule.
    if sig.len_cap_guard || sig.contains_key_guard || sig.drain_guard {
        return;
    }
    let fn_vocab = name_segments(fn_name)
        .iter()
        .any(|s| PEER_VOCAB.contains(&s.as_str()));
    for (span, recv_seg) in &sig.inserts {
        // The receiver must be a SAME-FILE map: a local decl or a
        // struct-field decl.
        let key_segs = if let Some((_, k)) = sig.local_maps.iter().find(|(n, _)| n == recv_seg) {
            k.clone()
        } else if let Some((_, k)) = field_maps.iter().find(|(n, _)| n == recv_seg) {
            k.clone()
        } else {
            continue;
        };
        let vocab_hit = fn_vocab
            || name_segments(recv_seg)
                .iter()
                .any(|s| PEER_VOCAB.contains(&s.as_str()))
            || key_segs.iter().any(|s| PEER_VOCAB.contains(&s.as_str()));
        if !vocab_hit {
            continue;
        }
        if span.end > source.len() {
            continue;
        }
        findings.push(KeyedStateFinding {
            site_text: source[span.clone()].to_string(),
            site_span: span.clone(),
            advice: ADVICE,
        });
    }
}

fn collect_signals(stmts: &[Stmt], sig: &mut FnSignals) {
    for stmt in stmts {
        match stmt {
            Stmt::Local(l) => {
                // The decl itself may be the map: a typed annotation
                // (`let x: HashMap<K, V> = ..` — syn 2 keeps the type in
                // the PatType pattern) or a `HashMap::new()` ctor init.
                let (pat_ty, pat_ident) = match &l.pat {
                    Pat::Type(pt) => (Some(&pt.ty), ident_of(&pt.pat)),
                    other => (None, ident_of(other)),
                };
                let mut key_segs: Option<Vec<String>> = pat_ty.and_then(|t| map_key_segments(t));
                if key_segs.is_none() {
                    if let Some(init) = &l.init {
                        if ctor_is_map(&init.expr) {
                            key_segs = Some(Vec::new());
                        }
                    }
                }
                if let (Some(ks), Some(name)) = (key_segs, pat_ident) {
                    sig.local_maps.push((name, ks));
                }
                if let Some(init) = &l.init {
                    collect_signals_expr(&init.expr, sig);
                }
            }
            Stmt::Item(item) => collect_signals_item(item, sig),
            Stmt::Expr(e, _) => collect_signals_expr(e, sig),
            Stmt::Macro(_) => {}
        }
    }
}

fn collect_signals_item(item: &Item, sig: &mut FnSignals) {
    match item {
        Item::Fn(f) => collect_signals(&f.block.stmts, sig),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    collect_signals(&m.block.stmts, sig);
                }
            }
        }
        Item::Mod(m) if !is_test_mod(m) => {
            if let Some((_, nested)) = &m.content {
                for nested_item in nested {
                    collect_signals_item(nested_item, sig);
                }
            }
        }
        _ => {}
    }
}

fn collect_signals_expr(e: &Expr, sig: &mut FnSignals) {
    if let Expr::If(i) = e {
        collect_signals_expr(&i.cond, sig);
        collect_signals(&i.then_branch.stmts, sig);
        if let Some((_, else_e)) = &i.else_branch {
            collect_signals_expr(else_e, sig);
        }
        return;
    }
    match e {
        Expr::Binary(b) => {
            if matches!(b.op, BinOp::Gt(_) | BinOp::Ge(_)) {
                let len_left = contains_len(&b.left);
                let len_right = contains_len(&b.right);
                if (len_left && cap_like(&b.right)) || (len_right && cap_like(&b.left)) {
                    sig.len_cap_guard = true;
                }
            }
            collect_signals_expr(&b.left, sig);
            collect_signals_expr(&b.right, sig);
        }
        Expr::MethodCall(m) => {
            if m.method == "insert" && m.args.len() == 2 {
                if let Some(recv) = last_segment(&m.receiver) {
                    sig.inserts.push((e.span().byte_range(), recv));
                }
            }
            if m.method == "contains_key" {
                sig.contains_key_guard = true;
            }
            if DRAIN_METHODS.contains(&m.method.to_string().as_str()) {
                sig.drain_guard = true;
            }
            collect_signals_expr(&m.receiver, sig);
            for a in &m.args {
                collect_signals_expr(a, sig);
            }
        }
        Expr::Call(c) => {
            collect_signals_expr(&c.func, sig);
            for a in &c.args {
                collect_signals_expr(a, sig);
            }
        }
        Expr::Match(m) => {
            collect_signals_expr(&m.expr, sig);
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    collect_signals_expr(guard, sig);
                }
                collect_signals_expr(&arm.body, sig);
            }
        }
        Expr::Block(b) => collect_signals(&b.block.stmts, sig),
        Expr::Unsafe(u) => collect_signals(&u.block.stmts, sig),
        Expr::Loop(l) => collect_signals(&l.body.stmts, sig),
        Expr::While(w) => {
            collect_signals_expr(&w.cond, sig);
            collect_signals(&w.body.stmts, sig);
        }
        Expr::ForLoop(fl) => {
            collect_signals_expr(&fl.expr, sig);
            collect_signals(&fl.body.stmts, sig);
        }
        Expr::Closure(cl) => collect_signals_expr(&cl.body, sig),
        Expr::Let(lt) => collect_signals_expr(&lt.expr, sig),
        Expr::Paren(p) => collect_signals_expr(&p.expr, sig),
        _ => {}
    }
}

/// `HashMap::new()` / `HashMap::with_capacity(..)` / `HashSet::new()` —
/// any HashMap/HashSet segment in the ctor path.
fn ctor_is_map(e: &Expr) -> bool {
    if let Expr::Call(c) = e {
        if let Expr::Path(p) = &*c.func {
            return p
                .path
                .segments
                .iter()
                .any(|s| s.ident == "HashMap" || s.ident == "HashSet");
        }
    }
    false
}

/// `HashMap<K, _>` / `HashSet<K>` -> Some(word segments of K).
fn map_key_segments(ty: &Type) -> Option<Vec<String>> {
    if let Type::Path(tp) = ty {
        if tp.qself.is_none() {
            if let Some(seg) = tp.path.segments.last() {
                if seg.ident == "HashMap" || seg.ident == "HashSet" {
                    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                        for a in &ab.args {
                            if let syn::GenericArgument::Type(inner) = a {
                                return Some(type_segments(inner));
                            }
                        }
                    }
                    return Some(Vec::new());
                }
            }
        }
    }
    None
}

/// All word segments of a type path (`std::net::IpAddr` -> std / net /
/// ip / addr).
fn type_segments(ty: &Type) -> Vec<String> {
    match ty {
        Type::Path(tp) if tp.qself.is_none() => tp
            .path
            .segments
            .iter()
            .flat_map(|s| name_segments(&s.ident.to_string()))
            .collect(),
        Type::Slice(sl) => type_segments(&sl.elem),
        Type::Array(ar) => type_segments(&ar.elem),
        Type::Reference(r) => type_segments(&r.elem),
        Type::Group(g) => type_segments(&g.elem),
        Type::Paren(p) => type_segments(&p.elem),
        _ => Vec::new(),
    }
}

/// A cap-like side of the comparison: an integer literal, or a path
/// whose last segment carries max/cap/limit (`MAX_SOURCES`, `cap`).
fn cap_like(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => matches!(l.lit, syn::Lit::Int(_)),
        Expr::Path(p) => p.path.segments.last().is_some_and(|s| {
            let name = s.ident.to_string().to_ascii_lowercase();
            name.contains("max") || name.contains("cap") || name.contains("limit")
        }),
        Expr::Binary(b) => cap_like(&b.left) || cap_like(&b.right),
        Expr::Paren(p) => cap_like(&p.expr),
        Expr::Cast(c) => cap_like(&c.expr),
        Expr::Reference(r) => cap_like(&r.expr),
        _ => false,
    }
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

/// The bound identifier of a pattern (`x`, the `x` inside `x: T`).
fn ident_of(pat: &Pat) -> Option<String> {
    if let Pat::Ident(pi) = pat {
        return Some(pi.ident.to_string());
    }
    None
}

/// The last segment of an insert receiver: a plain path
/// (`buckets.insert`) or a field access (`self.buckets.insert`).
fn last_segment(e: &Expr) -> Option<String> {
    match e {
        Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        Expr::Field(f) => match &f.member {
            syn::Member::Named(ident) => Some(ident.to_string()),
            syn::Member::Unnamed(_) => None,
        },
        _ => None,
    }
}

fn is_test_mod(m: &syn::ItemMod) -> bool {
    m.attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
}

/// The name's word segments: `snake_case` splits on `_`, `CamelCase`
/// splits at case boundaries (`ClientId` -> client + id). The vocabulary
/// gates match EXACT segments, never substrings.
fn name_segments(name: &str) -> Vec<String> {
    let chars: Vec<char> = name.chars().collect();
    let mut segs = Vec::new();
    let mut cur = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let prev = if i > 0 { chars[i - 1] } else { c };
        let next = chars.get(i + 1).copied();
        if c == '_' {
            if !cur.is_empty() {
                segs.push(std::mem::take(&mut cur));
            }
            continue;
        }
        let boundary = i > 0
            && c.is_ascii_uppercase()
            && (prev.is_ascii_lowercase()
                || (prev.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase())));
        if boundary && !cur.is_empty() {
            segs.push(std::mem::take(&mut cur));
        }
        cur.push(c.to_ascii_lowercase());
    }
    if !cur.is_empty() {
        segs.push(cur);
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::detect;

    fn findings(src: &str) -> Vec<super::KeyedStateFinding> {
        let file = syn::parse_file(src).expect("fixture parses");
        detect(src, &file)
    }

    #[test]
    fn peer_field_map_insert_fires() {
        // the sanstun buggy posture: a session-named map, no cap.
        let src = "use std::collections::HashMap;\n\n\
            pub struct Tracker {\n\
            \x20   sessions: HashMap<u64, u32>,\n\
            }\n\n\
            impl Tracker {\n\
            \x20   pub fn track_session(&mut self, peer_id: u64, seen: u32) {\n\
            \x20       self.sessions.insert(peer_id, seen);\n\
            \x20   }\n\
            }\n";
        let f = findings(src);
        assert_eq!(f.len(), 1, "the unbounded peer-keyed insert fires");
        assert!(f[0].site_text.contains("insert"));
    }

    #[test]
    fn key_type_vocabulary_fires() {
        // the map name carries no peer segment — the ClientId KEY TYPE
        // does.
        let src = "use std::collections::HashMap;\n\n\
            pub struct Room {\n\
            \x20   members: HashMap<ClientId, Member>,\n\
            }\n\n\
            impl Room {\n\
            \x20   pub fn join(&mut self, who: ClientId, m: Member) {\n\
            \x20       self.members.insert(who, m);\n\
            \x20   }\n\
            }\n";
        assert_eq!(
            findings(src).len(),
            1,
            "the key type carries the vocabulary"
        );
    }

    #[test]
    fn capped_twin_is_silent() {
        // the fixed shape: len() >= MAX before the insert.
        let src = "use std::collections::HashMap;\n\n\
            const MAX_CLIENTS: usize = 1024;\n\n\
            pub struct Room {\n\
            \x20   members: HashMap<ClientId, Member>,\n\
            }\n\n\
            impl Room {\n\
            \x20   pub fn join(&mut self, who: ClientId, m: Member) -> bool {\n\
            \x20       if self.members.len() >= MAX_CLIENTS {\n\
            \x20           return false;\n\
            \x20       }\n\
            \x20       self.members.insert(who, m);\n\
            \x20       true\n\
            \x20   }\n\
            }\n";
        assert!(findings(src).is_empty(), "the cap guard hides the fn");
    }

    #[test]
    fn non_peer_map_is_silent() {
        let src = "use std::collections::HashMap;\n\n\
            pub fn prime_config(rows: &[(String, u32)]) {\n\
            \x20   let mut cache: HashMap<String, u32> = HashMap::new();\n\
            \x20   for (k, v) in rows {\n\
            \x20       cache.insert(k.clone(), *v);\n\
            \x20   }\n\
            \x20   let _ = cache.len();\n\
            }\n";
        assert!(findings(src).is_empty(), "no peer vocabulary, no fire");
    }

    #[test]
    fn receive_handler_without_insert_is_silent() {
        let src = "use std::net::SocketAddr;\n\n\
            pub struct Relay { total: u64 }\n\n\
            impl Relay {\n\
            \x20   pub fn receive(&mut self, addr: SocketAddr, n: u64) {\n\
            \x20       self.total += n;\n\
            \x20   }\n\
            }\n";
        assert!(findings(src).is_empty(), "no insert site, no fire");
    }

    #[test]
    fn drained_map_is_silent() {
        // the fn already sheds entries — the growth has a counterweight.
        let src = "use std::collections::HashMap;\n\n\
            pub struct Relay {\n\
            \x20   peers: HashMap<u64, u32>,\n\
            }\n\n\
            impl Relay {\n\
            \x20   pub fn note(&mut self, who: u64, seen: u32) {\n\
            \x20       self.peers.retain(|_, v| *v > 0);\n\
            \x20       self.peers.insert(who, seen);\n\
            \x20   }\n\
            }\n";
        assert!(findings(src).is_empty(), "the drain hides the fn");
    }

    #[test]
    fn vec_insert_is_silent() {
        // Vec::insert(index, value) is not keyed state.
        let src = "pub fn push_at(buf: &mut Vec<u8>) {\n\
            \x20   buf.insert(0, 1);\n\
            }\n";
        assert!(findings(src).is_empty(), "a Vec is not a keyed table");
    }

    #[test]
    fn local_map_with_peer_name_fires() {
        // the local-map face: fn name carries the vocabulary (`devices`
        // plural misses the exact segment — `track_device` hits).
        let src = "use std::collections::HashMap;\n\n\
            pub fn track_device(id: u64) {\n\
            \x20   let mut devices: HashMap<u64, ()> = HashMap::new();\n\
            \x20   devices.insert(id, ());\n\
            \x20   let _ = devices;\n\
            }\n";
        assert_eq!(
            findings(src).len(),
            1,
            "a local map fires via the fn-name vocabulary"
        );
    }

    #[test]
    fn test_mod_is_skipped() {
        let src = "#[cfg(test)]\nmod tests {\n\
            \x20   use std::collections::HashMap;\n\
            \x20   #[test]\n\
            \x20   fn t() {\n\
            \x20       let mut sessions = HashMap::new();\n\
            \x20       sessions.insert(1u64, 1u32);\n\
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
            .find(|r| r.rule_id == "untrusted-keyed-state-unbounded")
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
