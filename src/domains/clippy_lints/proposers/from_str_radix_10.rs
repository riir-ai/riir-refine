//! `clippy::from_str_radix_10` — `u32::from_str_radix(s, 10)` →
//! `s.parse::<u32>()` (Batch 160 / Plan 151 — the frontier-lead mining pack
//! VI; family fixture verified to fire exactly this lint under
//! all+pedantic+nursery AND the default-groups arm, 2026-09-18).
//!
//! ## Why the rewrite is identity (the FIXABLE bar)
//!
//! std defines `impl FromStr for {every int primitive}` as literally
//! `from_str_radix(src, 10)` — `parse::<T>()` calls THAT function through
//! `FromStr::from_str` with the SAME `&str` input and returns the SAME
//! `Result<T, ParseIntError>`. The turbofish re-spells the type the original
//! call already named, so the rewrite is RESOLUTION-IDENTICAL: no NEW name
//! resolution is introduced (unlike `legacy_numeric_constants`, which
//! introduced the bare primitive path and needed the shadow guard — here
//! both sides spell `TY` in type position, so a shadow would resolve the
//! same way in both and the identity holds by construction).
//!
//! The receiver is arg0's text VERBATIM (paren-wrapped when not
//! postfix-safe): `from_str_radix`'s `&str` param accepted the receiver via
//! deref coercion, and method resolution reaches `str::parse` through the
//! same autoderef chain — every receiver shape the original compiled on,
//! the rewrite compiles on.
//!
//! Shape: `TY::from_str_radix(arg0, 10)` — a bare 2-segment path call
//! (`TY` ∈ the 12 int primitives, exactly where `from_str_radix` and the
//! radix-10 `FromStr` impl both exist), radix an int LITERAL whose base-10
//! value is 10 (suffix rides — a compiling original's literal already
//! unifies with the `u32` param). Guards: exactly 2 args; no qself /
//! leading colon / path arguments on either segment; attrs on the call
//! refuse; comments inside the deleted glue refuse (keep-list = arg0 —
//! the radix literal and both path segments are re-spelled).

use syn::visit::Visit;
use syn::{ExprCall, File};

use super::helpers::{byte_range, drops_interior_comment, int_lit, is_postfix_safe};
use super::{edit, SourceEdit};

/// The int primitives — exactly the types with BOTH a `from_str_radix`
/// inherent assoc fn and a `FromStr` impl defined through it.
const RADIX_TYPES: &[&str] = &[
    "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
];

/// Collect the lint's edits against the original source. Empty = no match.
pub(crate) fn collect_edits(source: &str, file: &File) -> Vec<SourceEdit> {
    let mut v = Collector {
        source,
        edits: Vec::new(),
    };
    v.visit_file(file);
    v.edits
}

struct Collector<'a> {
    source: &'a str,
    edits: Vec<SourceEdit>,
}

impl<'ast> Visit<'ast> for Collector<'ast> {
    fn visit_expr_call(&mut self, c: &'ast ExprCall) {
        syn::visit::visit_expr_call(self, c);
        if let Some(replacement) = self.try_rewrite(c) {
            self.edits.push(edit(c, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_rewrite(&self, c: &ExprCall) -> Option<String> {
        // Bare 2-segment path: `TY::from_str_radix` (no qself, no leading
        // colon, no path arguments on either segment).
        let syn::Expr::Path(p) = &*c.func else {
            return None;
        };
        if p.qself.is_some() || p.path.leading_colon.is_some() || p.path.segments.len() != 2 {
            return None;
        }
        let segs = p.path.segments.iter().collect::<Vec<_>>();
        if !segs[0].arguments.is_none() || !segs[1].arguments.is_none() {
            return None;
        }
        let ty = segs[0].ident.to_string();
        if !RADIX_TYPES.contains(&ty.as_str()) || segs[1].ident != "from_str_radix" {
            return None;
        }
        if c.args.len() != 2 || !c.attrs.is_empty() {
            return None;
        }
        // The radix must be an int literal whose VALUE is 10 (suffix rides —
        // a compiling original's literal already unifies with `u32`).
        let radix = c.args.last()?;
        let (digits, _suffix) = int_lit(radix)?;
        if digits.parse::<u32>().ok()? != 10 {
            return None;
        }
        // The receiver rides VERBATIM; paren-wrap when not postfix-safe.
        let recv = c.args.first()?;
        let recv_text = self.source.get(byte_range(recv))?;
        let recv_out = if is_postfix_safe(recv) {
            recv_text.to_string()
        } else {
            format!("({recv_text})")
        };
        // Comments inside the deleted glue refuse; arg0's span is the
        // keep-list (the path segments + radix literal are re-spelled).
        if drops_interior_comment(self.source, byte_range(c), &[byte_range(recv)]) {
            return None;
        }
        Some(format!("{recv_out}.parse::<{ty}>()"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn u32_call_fires() {
        let out = propose_fix(
            "from_str_radix_10",
            "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); u32::from_str_radix(s, 10) }",
        )
        .expect("matched");
        assert!(
            out.contains("s.parse::<u32>()"),
            "not rewritten to parse: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn signed_and_usize_types_fire() {
        for ty in ["i8", "i64", "usize"] {
            let out = propose_fix(
                "from_str_radix_10",
                &format!(
                    "fn num(s: &str, hits: &mut Vec<{ty}>) -> Result<{ty}, std::num::ParseIntError> {{ hits.push(0); {ty}::from_str_radix(s, 10) }}",
                ),
            )
            .expect("matched");
            assert!(
                out.contains(&format!("s.parse::<{ty}>()")),
                "{ty} not rewritten: {out}"
            );
            syn::parse_file(&out).expect("re-parses");
        }
    }

    #[test]
    fn suffixed_radix_fires() {
        let out = propose_fix(
            "from_str_radix_10",
            "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); u32::from_str_radix(s, 10u32) }",
        )
        .expect("matched");
        assert!(
            out.contains("s.parse::<u32>()"),
            "suffixed radix not rewritten: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn non_postfix_safe_receiver_paren_wraps() {
        let out = propose_fix(
            "from_str_radix_10",
            "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); u32::from_str_radix(&s.to_uppercase(), 10) }",
        )
        .expect("matched");
        assert!(
            out.contains("(&s.to_uppercase()).parse::<u32>()"),
            "reference receiver not paren-wrapped: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn nested_in_larger_expr_fires() {
        let out = propose_fix(
            "from_str_radix_10",
            "fn num(s: &str, hits: &mut Vec<u32>) -> u32 { hits.push(0); u32::from_str_radix(s, 10).unwrap_or(1) }",
        )
        .expect("matched");
        assert!(
            out.contains("s.parse::<u32>().unwrap_or(1)"),
            "sub-expression not rewritten in place: {out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn other_radix_refuses() {
        // Radix 2 is a genuinely different parser — must not fire.
        assert!(propose_fix(
            "from_str_radix_10",
            "fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); u32::from_str_radix(s, 2) }",
        )
        .is_none());
    }

    #[test]
    fn variable_radix_refuses() {
        assert!(propose_fix(
            "from_str_radix_10",
            "fn num(s: &str, r: u32, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); u32::from_str_radix(s, r) }",
        )
        .is_none());
    }

    #[test]
    fn non_int_type_and_path_shapes_refuse() {
        // A non-primitive type segment — not an int FromStr impl.
        assert!(propose_fix(
            "from_str_radix_10",
            "fn num(s: &str, hits: &mut Vec<u32>) -> u32 { hits.push(0); Foo::from_str_radix(s, 10).unwrap() }",
        )
        .is_none());
        // Leading colon — conservative refusal (documented divergence).
        assert!(propose_fix(
            "from_str_radix_10",
            "fn num(s: &str, hits: &mut Vec<u32>) -> u32 { hits.push(0); ::u32::from_str_radix(s, 10).unwrap() }",
        )
        .is_none());
        // 1-arg / 3-arg calls.
        assert!(propose_fix(
            "from_str_radix_10",
            "fn num(hits: &mut Vec<u32>) { hits.push(0); let _ = u32::from_str_radix(\"1\"); }",
        )
        .is_none());
    }

    #[test]
    fn comment_in_glue_refuses() {
        assert!(propose_fix(
            "from_str_radix_10",
            "fn num(s: &str, hits: &mut Vec<u32>) -> u32 { hits.push(0); u32::from_str_radix(s, /* radix */ 10).unwrap() }",
        )
        .is_none());
    }
}
