//! Operator-aware BM25 tokenizer — the Plan 005 T3.5 tiny-token-crowding fix.
//!
//! ## The defect this exists to fix
//!
//! The substrate default, `riir_neuron_db::bm25::CodeTokenizer`, splits on
//! `!c.is_ascii_alphanumeric() && c != '_'` — it keeps identifier tokens and
//! **discards every punctuation character**. For identifier-rich corpora
//! (`rust_perf`: `with_capacity`, `HashMap`, `Arc`; kernel: `CubeDim`, `launch`)
//! that is fine, and both domains retrieve well (87.5% / 100% top-1).
//!
//! The clippy corpus is the opposite shape. Its 40 rules are minimal generic
//! functions whose *entire* discriminative signal is punctuation:
//!
//! | lint | pattern | signal | after `CodeTokenizer` |
//! |---|---|---|---|
//! | `identity_op` | `fn f(x: u32) -> u32 { x * 1 }` | `* 1` | `fn f x u32 u32 x 1` |
//! | `unused_unit` | `fn f() -> () {}` | `-> ()` | `fn f` |
//! | `deref_addrof` | `&*x` | `&*` | `x` |
//! | `double_parens` | `((x))` | `((` | `x` |
//!
//! Stripped of punctuation every rule collapses toward the same generic
//! `fn f x` bag, and the rule with the most *common* residual tokens —
//! `identity_op` — becomes the nearest neighbour of nearly everything. It
//! topped 7 of the 22 clippy misses in the pre-fix baseline. That is the
//! "tiny-token crowding" Plan 005 T3.5 names: the tiny tokens are the
//! punctuation, and they were being thrown away before scoring.
//!
//! ## The fix
//!
//! Emit identifier tokens exactly as `CodeTokenizer` does (so identifier-rich
//! domains are unaffected by construction), and additionally emit **each
//! maximal run of punctuation as one token**, prefixed with [`OP_PREFIX`] so
//! it can never collide with an identifier.
//!
//! Punctuation *runs*, not a hand-picked operator table, is the deliberate
//! choice: it is fully generic (no Rust-grammar knowledge, nothing to keep in
//! sync with the language), and it lets **BM25's own IDF decide what is
//! discriminative**. Ubiquitous runs (`(`, `)`, `;`, `{`) earn near-zero IDF
//! and contribute nothing; rare ones (`()`, `&*`, `((`, `->`) earn high IDF
//! and do the separating. No hand-tuned weights.
//!
//! ```text
//! "fn f() -> () {}"  →  fn · f · op:() · op:-> · op:() · op:{}
//! "&*x"              →  op:&* · x
//! "((x))"            →  op:(( · x · op:))
//! ```
//!
//! Modelless: pure deterministic string scanning, no weights, no training.

use riir_neuron_db::bm25::Tokenizer;

/// Prefix marking a punctuation-run token.
///
/// Contains `:`, which the identifier scanner can never emit, so an operator
/// token and an identifier token can never collide.
pub const OP_PREFIX: &str = "op:";

/// Identifier tokens (as [`CodeTokenizer`](riir_neuron_db::bm25::CodeTokenizer))
/// **plus** one token per maximal punctuation run.
///
/// See the module docs for why this exists and why runs beat an operator table.
#[derive(Debug, Default, Clone, Copy)]
pub struct RustOpTokenizer;

/// [`RustOpTokenizer`] plus **adjacency bigrams**: for every adjacent token
/// pair where at least one side is an operator token, an extra combined term.
///
/// Unigrams alone cannot represent the signal several clippy lints are
/// *defined* by, because the discriminating feature is a short operator
/// SEQUENCE whose parts are individually ubiquitous:
///
/// | lint | signal | unigrams | why they fail |
/// |---|---|---|---|
/// | `unused_unit` | `-> ()` | `op:->`, `op:()` | both appear in most patterns |
/// | `identity_op` | `* 1` | `op:*`, `1` | both common |
/// | `deref_addrof` | `&*` | `op:&*` | already a single run — fine |
///
/// `op:()` is not rare (every `fn f()` has one), so unigram IDF cannot lift
/// `unused_unit`. The bigram `op:->|op:()` IS rare, and IDF finds it without
/// any hand-tuned weight.
///
/// Bigrams are emitted in addition to (not instead of) unigrams, so this is a
/// strict superset of [`RustOpTokenizer`]'s evidence.
#[derive(Debug, Default, Clone, Copy)]
pub struct RustOpBigramTokenizer;

/// Separator between the two halves of an adjacency bigram. Contains `|`,
/// which neither an identifier nor a lone punctuation run can produce at this
/// position, so bigram terms occupy their own namespace.
pub const BIGRAM_SEP: &str = "|";

impl Tokenizer for RustOpBigramTokenizer {
    fn tokenize_into(&self, text: &str, out: &mut Vec<String>) {
        let start = out.len();
        RustOpTokenizer.tokenize_into(text, out);
        // Adjacency pass over the unigrams just emitted. At least one side
        // must be an operator token — ident/ident bigrams are ordinary
        // n-gram noise and add nothing this fix is about.
        let mut bigrams = Vec::new();
        for w in out[start..].windows(2) {
            let (a, b) = (&w[0], &w[1]);
            if a.starts_with(OP_PREFIX) || b.starts_with(OP_PREFIX) {
                let mut t = String::with_capacity(a.len() + BIGRAM_SEP.len() + b.len());
                t.push_str(a);
                t.push_str(BIGRAM_SEP);
                t.push_str(b);
                bigrams.push(t);
            }
        }
        out.append(&mut bigrams);
    }
}

/// Is `c` part of an identifier token?
#[inline]
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

impl Tokenizer for RustOpTokenizer {
    fn tokenize_into(&self, text: &str, out: &mut Vec<String>) {
        // `char_indices` (not byte indexing) — corpus patterns are ASCII in
        // practice, but a stray non-ASCII char in a doc comment must not
        // panic on a UTF-8 boundary slice.
        let mut chars = text.char_indices().peekable();
        while let Some(&(start, c)) = chars.peek() {
            if is_ident_char(c) {
                let mut end = start;
                while let Some(&(i, c)) = chars.peek() {
                    if !is_ident_char(c) {
                        break;
                    }
                    end = i + c.len_utf8();
                    chars.next();
                }
                out.push(text[start..end].to_lowercase());
            } else if c.is_whitespace() {
                chars.next();
            } else {
                let mut end = start;
                while let Some(&(i, c)) = chars.peek() {
                    if is_ident_char(c) || c.is_whitespace() {
                        break;
                    }
                    end = i + c.len_utf8();
                    chars.next();
                }
                let run = &text[start..end];
                let mut tok = String::with_capacity(OP_PREFIX.len() + run.len());
                tok.push_str(OP_PREFIX);
                tok.push_str(run);
                out.push(tok);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(s: &str) -> Vec<String> {
        RustOpTokenizer.tokenize(s)
    }

    #[test]
    fn identifier_tokens_match_the_substrate_default() {
        // The identifier half must be byte-identical to CodeTokenizer's
        // output, so identifier-rich domains (perf/kernel) cannot regress
        // from the operator additions alone.
        let text = "fn find_user(users: &Vec<User>, id: u32) -> Option<&User>";
        let idents: Vec<String> = tok(text)
            .into_iter()
            .filter(|t| !t.starts_with(OP_PREFIX))
            .collect();
        let expected = riir_neuron_db::bm25::CodeTokenizer.tokenize(text);
        assert_eq!(idents, expected);
    }

    #[test]
    fn punctuation_runs_become_distinct_tokens() {
        assert_eq!(tok("fn f() -> () {}"), vec![
            "fn", "f", "op:()", "op:->", "op:()", "op:{}"
        ]);
    }

    #[test]
    fn the_four_crowding_signals_survive() {
        // Each of these was erased by CodeTokenizer, collapsing the rule into
        // a generic `fn f x` bag. Each must now be present and distinct.
        assert!(tok("fn f(x: u32) -> u32 { x * 1 }").contains(&"op:*".to_string()));
        assert!(tok("fn f() -> () {}").contains(&"op:()".to_string()));
        assert!(tok("&*x").contains(&"op:&*".to_string()));
        assert!(tok("((x))").contains(&"op:((".to_string()));
    }

    #[test]
    fn operator_tokens_cannot_collide_with_identifiers() {
        // No identifier token may ever begin with the operator prefix.
        for t in tok("op fn x_1 -> () op:weird") {
            if let Some(rest) = t.strip_prefix(OP_PREFIX) {
                assert!(
                    !rest.chars().next().is_some_and(is_ident_char),
                    "operator token {t} starts with an identifier char"
                );
            }
        }
    }

    #[test]
    fn is_deterministic_and_total() {
        // Same input → same output, and every non-whitespace char is covered
        // by exactly one token (no silent drops, the CodeTokenizer defect).
        let text = "let y = &*(v[i]) + 1; // note";
        assert_eq!(tok(text), tok(text));
        let covered: usize = tok(text)
            .iter()
            .map(|t| t.strip_prefix(OP_PREFIX).unwrap_or(t).chars().count())
            .sum();
        let non_ws = text.chars().filter(|c| !c.is_whitespace()).count();
        assert_eq!(covered, non_ws, "tokens must cover every non-whitespace char");
    }

    #[test]
    fn non_ascii_does_not_panic() {
        let _ = tok("let s = \"héllo→wörld\"; // ünicode ✓");
    }
}
