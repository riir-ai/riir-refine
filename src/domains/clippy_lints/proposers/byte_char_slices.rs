//! `clippy::byte_char_slices` — `[b'S', b'H', b'R', b'D']` → `*b"SHRD"`.
//!
//! Style lint (new in clippy 1.98): an array literal whose every element is a
//! byte literal is a byte-string literal in disguise. The rewrite is
//! bit-identical by construction — the produced `[u8; N]` holds exactly the
//! same bytes, and `*b".."` (deref of the `&'static [u8; N]`) is legal in
//! every position the array literal was (const, static, let, argument). The
//! ndb wire-magic batch (`FREEZE_MAGIC: [u8; 4] = *b"FRZE"`, commit `a9699c2`
//! class) landed this exact form with the consensus-committed device-vector
//! pins still passing.
//!
//! Guards: every element must be a `Lit::Byte` (any other element shape →
//! silent); non-empty; no comments inside the array span (a replacement would
//! displace them). Escape rendering: printable ASCII passes through, `"` and
//! `\` are escaped, `\n`/`\r`/`\t`/`\0` use named escapes, everything else
//! uses `\xHH`.

use super::helpers::{byte_range, drops_interior_comment};
use super::{edit, SourceEdit};
use syn::visit::Visit;
use syn::{Expr, ExprArray, ExprLit, File, Lit};

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

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_expr_array(&mut self, a: &'ast ExprArray) {
        syn::visit::visit_expr_array(self, a);
        if let Some(replacement) = self.try_byte_string(a) {
            self.edits.push(edit(a, replacement));
        }
    }
}

impl Collector<'_> {
    fn try_byte_string(&self, a: &ExprArray) -> Option<String> {
        if a.elems.is_empty() {
            return None;
        }
        let mut text = String::with_capacity(a.elems.len() + 4);
        for elem in &a.elems {
            let Expr::Lit(ExprLit {
                lit: Lit::Byte(b), ..
            }) = elem
            else {
                return None;
            };
            text.push_str(&byte_token(b.value()));
        }
        // Whole-span replacement — any interior comment would be displaced.
        if drops_interior_comment(self.source, byte_range(a), &[]) {
            return None;
        }
        let mut replacement = format!("*b\"{text}\"");
        // Postfix-continuation guard: unary `*` binds LOOSER than `.`/`[`/`?`,
        // so an array in method-receiver/index position needs parens —
        // `[b'-', b'.'].into_iter()` must become `(*b"- .").into_iter()`,
        // never `*b"- .".into_iter()` (that derefs the Iterator). Found live
        // by the katgpt-pruners compile gate (2 auto-reverted edits,
        // Batch 91 consumer proof).
        if self.source[byte_range(a).end..]
            .trim_start()
            .starts_with(['.', '[', '?'])
        {
            replacement = format!("({replacement})");
        }
        Some(replacement)
    }
}

/// Escape one byte for a `b"..."` literal.
fn byte_token(v: u8) -> String {
    match v {
        0x20..=0x7e if v != b'"' && v != b'\\' => (v as char).to_string(),
        b'"' => "\\\"".to_string(),
        b'\\' => "\\\\".to_string(),
        b'\n' => "\\n".to_string(),
        b'\r' => "\\r".to_string(),
        b'\t' => "\\t".to_string(),
        0 => "\\0".to_string(),
        _ => format!("\\x{v:02x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::super::propose_fix;

    #[test]
    fn converts_wire_magic_const() {
        let input = "const M: [u8; 4] = [b'F', b'R', b'Z', b'E'];";
        let out = propose_fix("byte_char_slices", input).expect("matched");
        assert_eq!(out, "const M: [u8; 4] = *b\"FRZE\";");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn escapes_quotes_and_control_bytes() {
        let input = "const M: [u8; 3] = [b'\"', b'\\n', b'\\xFF'];";
        let out = propose_fix("byte_char_slices", input).expect("matched");
        assert_eq!(out, "const M: [u8; 3] = *b\"\\\"\\n\\xff\";");
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn single_element_fires() {
        let input = "static S: [u8; 1] = [b'X'];";
        let out = propose_fix("byte_char_slices", input).expect("matched");
        assert_eq!(out, "static S: [u8; 1] = *b\"X\";");
    }

    #[test]
    fn non_byte_element_silent() {
        let input = "const M: [u8; 2] = [b'A', 66];";
        let out = propose_fix("byte_char_slices", input);
        assert!(out.is_none(), "mixed elements must not fire: {out:?}");
    }

    #[test]
    fn empty_array_silent() {
        let input = "const M: [u8; 0] = [];";
        let out = propose_fix("byte_char_slices", input);
        assert!(out.is_none(), "empty array must not fire: {out:?}");
    }

    #[test]
    fn interior_comment_refused() {
        let input = "const M: [u8; 2] = [b'A', /* x */ b'B'];";
        let out = propose_fix("byte_char_slices", input);
        assert!(out.is_none(), "interior comment must refuse: {out:?}");
    }

    #[test]
    fn method_receiver_position_parenthesizes() {
        // The katgpt-pruners revert class: unary * binds looser than `.` —
        // the array in receiver position needs parens.
        let input = "fn f() -> u8 { [b'-', b'.'].into_iter().next().unwrap() }";
        let out = propose_fix("byte_char_slices", input).expect("matched");
        assert!(
            out.contains("(*b\"-.\").into_iter()") || out.contains("(*b\"- .\").into_iter()"),
            "{out}"
        );
        syn::parse_file(&out).expect("re-parses");
    }

    #[test]
    fn index_position_parenthesizes() {
        let input = "fn f() -> u8 { [b'a', b'b'][1] }";
        let out = propose_fix("byte_char_slices", input).expect("matched");
        assert!(out.contains("(*b\"ab\")[1]"), "{out}");
        syn::parse_file(&out).expect("re-parses");
    }
}
