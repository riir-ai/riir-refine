//! Behavioural equivalence for candidate clippy fixes — the instrument Plan 336
//! T5.1 and T5.3 both name as their blocker.
//!
//! # Why this exists
//!
//! riir-refine's L4-vs-decline gate ([`tests/t51_l4_vs_decline.rs`], Bench 039)
//! scored a candidate fix as a **WIN** when `cargo clippy` stopped firing AND the
//! *contract identifiers* were preserved. At n=9 that produced 5 wins — and 2 of
//! the 5 were **silently wrong**:
//!
//! ```text
//! option_if_let_else  None   : orig=0     l4=1          WRONG
//! manual_strip        "xyz"  : orig=None  l4=Some("z")  WRONG
//! ```
//!
//! `o.unwrap_or(0) + 1` adds 1 to the `None` branch; `s.get(2..)` drops the
//! `starts_with("ab")` guard. Both keep every public identifier, so no name-based
//! check can see them. Plan 336 T5.3 states the consequence: *"Any future L4
//! promotion criterion must require behavioural equivalence, not just contract
//! preservation — otherwise `p_win` is an upper bound at any `n`."*
//!
//! This module is that requirement, made executable.
//!
//! # How
//!
//! Differential property testing on the *pair*. Given two sources that each
//! define the target fn, we parse the signature with `syn`, derive a bounded
//! input domain per argument type, emit a single dependency-free driver, compile
//! it with plain `rustc` (not cargo — no dep graph, no shared target dir, cheap
//! enough to run on a loaded box), and compare `{:?}` of the two returns over
//! every input. First divergence wins.
//!
//! Deterministic by construction: the corpora are fixed literals plus a
//! fixed-seed LCG, the same discipline as Plan 336 T3.4a's advantage-floor
//! harness. Re-running cannot change a verdict.
//!
//! # What it deliberately does NOT claim
//!
//! This is a **refuter, not a prover**. [`Verdict::Equivalent`] means "no
//! divergence found over this domain" — it is not a proof of equivalence, and it
//! is only as good as the domain. Types outside the supported set return
//! [`Verdict::Unsupported`], which callers MUST NOT read as a pass; an
//! unsupported case is unscored, exactly as Bench 039 excludes rather than
//! flatters an unscoreable fixture.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The outcome of a differential comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// No divergence over `cases` inputs. NOT a proof — see module docs.
    Equivalent { cases: usize },
    /// A concrete counterexample. This is the class Bench 039 could not see.
    Divergent {
        input: String,
        original: String,
        candidate: String,
    },
    /// Could not be scored. Callers must treat this as "no verdict", never a pass.
    Unsupported { reason: String },
}

impl Verdict {
    /// True only for a positive equivalence result. `Unsupported` is false — the
    /// whole point is that an unscoreable case must not read as a pass.
    #[must_use]
    pub fn is_equivalent(&self) -> bool {
        matches!(self, Verdict::Equivalent { .. })
    }

    /// True when a counterexample was found. A promotion gate should reject on this.
    #[must_use]
    pub fn is_divergent(&self) -> bool {
        matches!(self, Verdict::Divergent { .. })
    }
}

/// A supported argument domain. Bounded on purpose: every entry maps to a fixed
/// corpus, so the generated driver has no runtime dependency and no randomness
/// beyond the seeded LCG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dom {
    Int {
        rust: &'static str,
    },
    Bool,
    Char,
    /// `&str` / `String` — owned as `String`, borrowed per side.
    Str,
    /// `Vec<u8>` / `&Vec<u8>` / `&[u8]` — owned as `Vec<u8>`, borrowed per side.
    Bytes,
    /// `Option<Int>`; carries the inner rust type name.
    OptInt {
        rust: &'static str,
    },
    /// `f32` / `f64` (Bench 114's pilot extension — the real-module
    /// population's #1 blocker was 26 float-signature fns reading
    /// Unsupported). The corpus carries the adversarial IEEE values:
    /// signed zero, NaN, infinities — Debug prints all of them distinctly.
    Float {
        rust: &'static str,
    },
    /// `Vec<int>` / `&[int]` for any integer width (the #2 blocker — 29
    /// non-u8 int-slice fns). `u8` keeps the older [`Dom::Bytes`] shape so
    /// every existing fixture's generated rows stay byte-identical.
    Ints {
        rust: &'static str,
    },
    /// `Option<f32>` / `Option<f64>`.
    OptFloat {
        rust: &'static str,
    },
}

/// How one side of the pair receives an owned value. `ptr_arg`'s whole fix is a
/// signature change (`&Vec<u8>` → `&[u8]`), so the two sides legitimately want
/// different borrows of the *same* value; without this, the fix we most want to
/// pass would be unscoreable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Borrow {
    ByValue,
    Ref,
    Slice,
    StrRef,
}

impl Dom {
    /// Fixed corpus. Ordered so the known-adversarial values come FIRST: `None`
    /// exposes `option_if_let_else`, `"xyz"` exposes `manual_strip`. A fast fail
    /// on the documented cases is the design goal, not a coincidence.
    fn corpus(self) -> Vec<String> {
        match self {
            Self::Int { rust } => ["0", "1", "2", "42", "7"]
                .iter()
                .map(|v| format!("{v}{rust}"))
                .chain(std::iter::once(format!("{rust}::MAX")))
                .chain(std::iter::once(format!("{rust}::MIN")))
                .collect(),
            Self::Bool => vec!["false".into(), "true".into()],
            Self::Char => vec!["'a'".into(), "'A'".into(), "'0'".into(), "' '".into()],
            Self::Str => [
                "xyz", "", "a", "ab", "abc", "ba", "abab", "z", "hello", "ab",
            ]
            .iter()
            .map(|s| format!("String::from({s:?})"))
            .collect(),
            Self::Bytes => vec![
                "vec![]".into(),
                "vec![0u8]".into(),
                "vec![1u8,2,3]".into(),
                "vec![7u8; 17]".into(),
            ],
            Self::OptInt { rust } => vec![
                "None".into(),
                format!("Some(0{rust})"),
                format!("Some(1{rust})"),
                format!("Some(42{rust})"),
                format!("Some({rust}::MAX)"),
            ],
            Self::Float { rust } => vec![
                format!("0.0{rust}"),
                format!("-0.0{rust}"),
                format!("1.5{rust}"),
                format!("-2.25{rust}"),
                format!("{rust}::NAN"),
                format!("{rust}::INFINITY"),
                format!("{rust}::MIN_POSITIVE"),
            ],
            Self::Ints { rust } => vec![
                "vec![]".into(),
                format!("vec![0{rust}]"),
                format!("vec![1{rust},2,3]"),
                format!("vec![7{rust};17]"),
            ],
            Self::OptFloat { rust } => vec![
                "None".into(),
                format!("Some(0.0{rust})"),
                format!("Some(-0.0{rust})"),
                format!("Some(1.5{rust})"),
                format!("Some({rust}::NAN)"),
            ],
        }
    }

    /// The owned local's Rust type, so the driver can declare a typed `Vec`.
    fn owned_ty(self) -> String {
        match self {
            Self::Int { rust } | Self::Float { rust } => rust.to_string(),
            Self::Bool => "bool".into(),
            Self::Char => "char".into(),
            Self::Str => "String".into(),
            Self::Bytes => "Vec<u8>".into(),
            Self::OptInt { rust } | Self::OptFloat { rust } => format!("Option<{rust}>"),
            Self::Ints { rust } => format!("Vec<{rust}>"),
        }
    }
}

/// Parse a `syn` type into a domain plus the borrow that type wants.
fn classify(ty: &syn::Type) -> Option<(Dom, Borrow)> {
    match ty {
        syn::Type::Reference(r) => {
            let (d, _) = classify(&r.elem)?;
            let b = if matches!(d, Dom::Str) {
                Borrow::StrRef
            } else {
                Borrow::Ref
            };
            Some((d, b))
        }
        syn::Type::Slice(s) => {
            let (d, _) = classify(&s.elem)?;
            // `[T]` behind a reference — the ptr_arg destination for u8; any
            // other integer width is the Bench-114 Ints domain.
            match d {
                Dom::Int { rust: "u8" } => Some((Dom::Bytes, Borrow::Slice)),
                Dom::Int { rust } => Some((Dom::Ints { rust }, Borrow::Slice)),
                Dom::Float { rust } => Some((Dom::Float { rust }, Borrow::Slice)),
                _ => None,
            }
        }
        syn::Type::Path(p) => {
            let seg = p.path.segments.last()?;
            let name = seg.ident.to_string();
            let ints = [
                "u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize",
            ];
            if let Some(rust) = ints.iter().find(|i| **i == name) {
                return Some((Dom::Int { rust }, Borrow::ByValue));
            }
            if let Some(rust) = ["f32", "f64"].iter().find(|i| **i == name) {
                return Some((Dom::Float { rust }, Borrow::ByValue));
            }
            match name.as_str() {
                "bool" => Some((Dom::Bool, Borrow::ByValue)),
                "char" => Some((Dom::Char, Borrow::ByValue)),
                "str" | "String" => Some((Dom::Str, Borrow::ByValue)),
                "Option" => {
                    let syn::PathArguments::AngleBracketed(a) = &seg.arguments else {
                        return None;
                    };
                    let syn::GenericArgument::Type(inner) = a.args.first()? else {
                        return None;
                    };
                    match classify(inner)? {
                        (Dom::Int { rust }, _) => Some((Dom::OptInt { rust }, Borrow::ByValue)),
                        (Dom::Float { rust }, _) => Some((Dom::OptFloat { rust }, Borrow::ByValue)),
                        _ => None,
                    }
                }
                "Vec" => {
                    let syn::PathArguments::AngleBracketed(a) = &seg.arguments else {
                        return None;
                    };
                    let syn::GenericArgument::Type(inner) = a.args.first()? else {
                        return None;
                    };
                    match classify(inner)? {
                        (Dom::Int { rust: "u8" }, _) => Some((Dom::Bytes, Borrow::ByValue)),
                        (Dom::Int { rust }, _) => Some((Dom::Ints { rust }, Borrow::ByValue)),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Publicize every TOP-LEVEL fn so the driver can call it cross-module.
/// Real mined modules (the Bench 114 population) declare private `fn`s —
/// every driver call read E0603 and the pilot settled nothing. The rewrite
/// is column-0 + name-anchored (nested fns are indented; `pub` is illegal
/// there), applied IDENTICALLY to both sides. Anything it mangles fails the
/// driver's compile → `Unsupported` — the failure mode is an honest unscored
/// case, never a false Equivalent.
fn publicize_top_level_fns(src: &str) -> String {
    // The target names come from a real parse: only top-level Item::Fn
    // idents, so a col-0 `fn name` inside a doc comment or raw string can
    // only match if the module genuinely declares that same fn — and the
    // compile gate still backstops the pathological residue.
    let names: BTreeSet<String> = syn::parse_file(src)
        .map(|f| {
            f.items
                .iter()
                .filter_map(|i| match i {
                    syn::Item::Fn(x) => Some(x.sig.ident.to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    if names.is_empty() {
        return src.to_string();
    }
    let mut out = String::with_capacity(src.len() + names.len() * 4);
    for line in src.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if !line.starts_with(' ') && !line.starts_with('\t') {
            // Skip an existing visibility prefix; publicize only when the
            // item declares one of our target fns.
            let after_vis = match trimmed.find(' ') {
                Some(i) => {
                    let first = &trimmed[..i];
                    if first == "pub" || first.starts_with("pub(") {
                        &trimmed[i + 1..]
                    } else {
                        trimmed
                    }
                }
                None => trimmed,
            };
            let declares_target = |head: &str| -> bool {
                // Walk the fn qualifiers (async/const/unsafe/extern "C") then
                // match `fn NAME` against the target set.
                let mut head = head.trim_start();
                while let Some(sp) = head.find(' ') {
                    let first = &head[..sp];
                    let rest = &head[sp + 1..];
                    match first {
                        "fn" => {
                            let id = rest.trim_start();
                            let name: String = id
                                .chars()
                                .take_while(|c| c.is_alphanumeric() || *c == '_')
                                .collect();
                            return names.contains(&name);
                        }
                        "async" | "const" | "unsafe" => head = rest,
                        "extern" => {
                            // Skip the optional ABI string: `extern "C" fn`.
                            if let Some(abi) = rest.trim_start().strip_prefix('"') {
                                let Some(close) = abi.find('"') else { break };
                                head = &abi[close + 1..];
                            } else {
                                head = rest;
                            }
                        }
                        _ => break,
                    }
                }
                false
            };
            let already_pub = trimmed.starts_with("pub ") || trimmed.starts_with("pub(");
            if !already_pub && declares_target(after_vis) {
                out.push_str("pub ");
            }
        }
        out.push_str(line);
    }
    out
}

/// Pick the fn to compare: `f` when present (every t51 fixture uses it),
/// otherwise the single `pub fn`. Ambiguity is Unsupported, never a guess.
fn target_fn(src: &str, name_hint: &str) -> Result<syn::Signature, String> {
    let file = syn::parse_file(src).map_err(|e| format!("parse error: {e}"))?;
    let fns: Vec<&syn::ItemFn> = file
        .items
        .iter()
        .filter_map(|i| match i {
            syn::Item::Fn(f) => Some(f),
            _ => None,
        })
        .collect();
    if let Some(f) = fns.iter().find(|f| f.sig.ident == name_hint) {
        return Ok(f.sig.clone());
    }
    match fns.as_slice() {
        [only] => Ok(only.sig.clone()),
        [] => Err("no free fn to compare".into()),
        _ => Err(format!(
            "ambiguous: {} fns and none named `{name_hint}`",
            fns.len()
        )),
    }
}

fn call_expr(var: &str, b: Borrow) -> String {
    match b {
        Borrow::ByValue => format!("{var}.clone()"),
        Borrow::Ref => format!("&{var}"),
        Borrow::Slice => format!("&{var}[..]"),
        Borrow::StrRef => format!("{var}.as_str()"),
    }
}

/// Deterministic LCG (Numerical Recipes constants) — same discipline as the
/// Plan 336 T3.4a floor harness. No `rand` dep, no wall-clock seed.
fn lcg(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state
}

/// Compare `original` and `candidate` behaviourally.
///
/// `name_hint` is the fn to compare (`"f"` for every t51 fixture). `max_cases`
/// caps the generated inputs. `workdir` receives the generated driver + binary;
/// pass something under `/tmp` so nothing lands in a shared target dir.
///
/// # Errors
/// Never — an unusable pair is reported as [`Verdict::Unsupported`] so a caller
/// cannot accidentally treat a harness failure as a pass.
#[must_use]
pub fn check(
    original: &str,
    candidate: &str,
    name_hint: &str,
    max_cases: usize,
    workdir: &Path,
) -> Verdict {
    let unsup = |r: String| Verdict::Unsupported { reason: r };

    // Bench 114: publicize both sides' top-level fns before emitting — the
    // driver calls them cross-module and real mined modules declare them
    // private (E0603 on every call). The rewrite is name-anchored and
    // identical on both sides; a mangled driver fails to compile and reads
    // Unsupported, never a false Equivalent.
    let original = publicize_top_level_fns(original);
    let candidate = publicize_top_level_fns(candidate);
    let original = original.as_str();
    let candidate = candidate.as_str();

    let doms = match plan_domains(original, candidate, name_hint) {
        Ok(d) => d,
        Err(e) => return unsup(e),
    };
    let rows = build_rows(&doms, max_cases);
    let driver = emit_driver(original, candidate, name_hint, &doms, &rows);
    compile_and_run(&driver, workdir, rows.len())
}

/// Classify each argument position from BOTH signatures. The domain must agree;
/// the borrow may differ — that difference IS the `ptr_arg` fix
/// (`&Vec<u8>` → `&[u8]`), so rejecting it would make the fix we most want to
/// pass unscoreable.
fn plan_domains(
    original: &str,
    candidate: &str,
    name_hint: &str,
) -> Result<Vec<(Dom, Borrow, Borrow)>, String> {
    let sig_o = target_fn(original, name_hint).map_err(|e| format!("original: {e}"))?;
    let sig_c = target_fn(candidate, name_hint).map_err(|e| format!("candidate: {e}"))?;
    if sig_o.inputs.len() != sig_c.inputs.len() {
        return Err(format!(
            "arity differs: {} vs {}",
            sig_o.inputs.len(),
            sig_c.inputs.len()
        ));
    }
    if sig_o.inputs.len() > 8 {
        return Err("arity > 8".to_string());
    }
    let mut doms: Vec<(Dom, Borrow, Borrow)> = Vec::with_capacity(sig_o.inputs.len());
    for (a, b) in sig_o.inputs.iter().zip(sig_c.inputs.iter()) {
        let (syn::FnArg::Typed(ta), syn::FnArg::Typed(tb)) = (a, b) else {
            return Err("self receiver".to_string());
        };
        let Some((da, ba)) = classify(&ta.ty) else {
            return Err("unsupported arg type in original".to_string());
        };
        let Some((db, bb)) = classify(&tb.ty) else {
            return Err("unsupported arg type in candidate".to_string());
        };
        if da != db {
            return Err(format!("arg domain changed: {da:?} vs {db:?}"));
        }
        doms.push((da, ba, bb));
    }
    Ok(doms)
}

/// Build the input table. Single-arg cases walk the FULL corpus — that is what
/// guarantees `None` and `"xyz"` are reached, i.e. what makes the two documented
/// silent breaks detectable rather than luck. Multi-arg cases take an all-first
/// and an all-last row, then LCG-sample the per-position corpora.
fn build_rows(doms: &[(Dom, Borrow, Borrow)], max_cases: usize) -> Vec<Vec<String>> {
    if doms.is_empty() {
        return vec![vec![]]; // zero-arg fn: one comparison of the return value
    }
    let corpora: Vec<Vec<String>> = doms.iter().map(|(d, _, _)| d.corpus()).collect();
    let mut rows: Vec<Vec<String>> = Vec::new();
    if doms.len() == 1 {
        for v in &corpora[0] {
            rows.push(vec![v.clone()]);
        }
    } else {
        rows.push(corpora.iter().map(|c| c[0].clone()).collect());
        rows.push(corpora.iter().map(|c| c[c.len() - 1].clone()).collect());
        let mut st = 0x5EED_u64;
        while rows.len() < max_cases {
            rows.push(
                corpora
                    .iter()
                    .map(|c| {
                        let n = u64::try_from(c.len()).unwrap_or(1).max(1);
                        let i = usize::try_from(lcg(&mut st) % n).unwrap_or(0);
                        c[i].clone()
                    })
                    .collect(),
            );
        }
    }
    rows.truncate(max_cases.max(1));
    rows.shrink_to_fit();
    rows
}

/// Emit the dependency-free differential driver.
fn emit_driver(
    original: &str,
    candidate: &str,
    name_hint: &str,
    doms: &[(Dom, Borrow, Borrow)],
    rows: &[Vec<String>],
) -> String {
    let tuple_ty = doms
        .iter()
        .map(|(d, _, _)| d.owned_ty())
        .collect::<Vec<_>>()
        .join(", ");
    let binds = (0..doms.len())
        .map(|i| format!("let a{i} = row.{i}.clone();"))
        .collect::<Vec<_>>()
        .join(" ");
    let args_o = doms
        .iter()
        .enumerate()
        .map(|(i, (_, b, _))| call_expr(&format!("a{i}"), *b))
        .collect::<Vec<_>>()
        .join(", ");
    let args_c = doms
        .iter()
        .enumerate()
        .map(|(i, (_, _, b))| call_expr(&format!("a{i}"), *b))
        .collect::<Vec<_>>()
        .join(", ");
    let table = rows
        .iter()
        .map(|r| format!("({})", r.join(", ")))
        .collect::<Vec<_>>()
        .join(", ");
    let (tuple_ty, table) = if doms.len() == 1 {
        // A 1-tuple needs the trailing comma to stay a tuple.
        (
            format!("({tuple_ty},)"),
            rows.iter()
                .map(|r| format!("({},)", r[0]))
                .collect::<Vec<_>>()
                .join(", "),
        )
    } else if doms.is_empty() {
        ("()".to_string(), "()".to_string())
    } else {
        (format!("({tuple_ty})"), table)
    };

    format!(
        r#"#![allow(warnings)]
mod orig {{ {original} }}
mod cand {{ {candidate} }}
fn main() {{
    // Silence panic spew: a panic on one side and not the other IS a divergence,
    // and we report it ourselves rather than letting the runtime print it.
    std::panic::set_hook(Box::new(|_| {{}}));
    let table: Vec<{tuple_ty}> = vec![{table}];
    for (i, row) in table.iter().enumerate() {{
        {binds}
        let o = std::panic::catch_unwind(|| format!("{{:?}}", orig::{name_hint}({args_o})));
        let c = std::panic::catch_unwind(|| format!("{{:?}}", cand::{name_hint}({args_c})));
        let (os, cs) = (
            o.unwrap_or_else(|_| "<panic>".to_string()),
            c.unwrap_or_else(|_| "<panic>".to_string()),
        );
        if os != cs {{
            println!("DIVERGE\t{{:?}}\t{{}}\t{{}}", row, os, cs);
            std::process::exit(7);
        }}
    }}
    println!("EQUIVALENT\t{{}}", table.len());
}}
"#
    )
}

/// Compile with plain `rustc` — no cargo, no dep graph, no shared target dir —
/// and read the verdict off stdout.
fn compile_and_run(driver: &str, workdir: &Path, n_rows: usize) -> Verdict {
    let unsup = |r: String| Verdict::Unsupported { reason: r };
    if std::fs::create_dir_all(workdir).is_err() {
        return unsup("could not create workdir".into());
    }
    let src = workdir.join("equiv_driver.rs");
    let bin: PathBuf = workdir.join("equiv_driver");
    if std::fs::write(&src, driver).is_err() {
        return unsup("could not write driver".into());
    }
    let build = Command::new("rustc")
        .args(["--edition", "2021", "-O", "-o"])
        .arg(&bin)
        .arg(&src)
        .output();
    let build = match build {
        Ok(o) => o,
        Err(e) => return unsup(format!("rustc not runnable: {e}")),
    };
    if !build.status.success() {
        // A candidate that does not compile is a REGRESSION, not an equivalence
        // question — the caller's clippy step already classifies it. Reporting
        // Unsupported keeps this instrument from double-counting that verdict.
        let err = String::from_utf8_lossy(&build.stderr);
        let first = err
            .lines()
            .find(|l| l.contains("error"))
            .unwrap_or("compile failed");
        return unsup(format!("driver did not compile: {first}"));
    }
    let run = match Command::new(&bin).output() {
        Ok(o) => o,
        Err(e) => return unsup(format!("driver not runnable: {e}")),
    };
    let out = String::from_utf8_lossy(&run.stdout);
    if let Some(line) = out.lines().find(|l| l.starts_with("DIVERGE")) {
        let mut p = line.split('\t').skip(1);
        return Verdict::Divergent {
            input: p.next().unwrap_or("?").to_string(),
            original: p.next().unwrap_or("?").to_string(),
            candidate: p.next().unwrap_or("?").to_string(),
        };
    }
    if let Some(line) = out.lines().find(|l| l.starts_with("EQUIVALENT")) {
        let n = line
            .split('\t')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(n_rows);
        return Verdict::Equivalent { cases: n };
    }
    unsup(format!(
        "driver produced no verdict (exit {:?})",
        run.status.code()
    ))
}

/// Convenience: the identifier set a caller may want alongside the verdict, so a
/// gate can report "contract preserved BUT behaviour diverged" — the exact
/// combination Bench 039 scored as a WIN.
#[must_use]
pub fn divergence_is_invisible_to_names(original: &str, candidate: &str) -> bool {
    fn idents(src: &str) -> BTreeSet<String> {
        syn::parse_file(src)
            .map(|f| {
                f.items
                    .iter()
                    .filter_map(|i| match i {
                        syn::Item::Fn(f) => Some(f.sig.ident.to_string()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    idents(original) == idents(candidate)
}
