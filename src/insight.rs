//! Issue 140 T1+T2 — the modelless insight distiller + the abstraction-level
//! admission predicate (RLTL;DR, arXiv:2609.37633, `.research/228`).
//!
//! The paper's measured laws this module encodes, zero gradients:
//!
//! - **L1 (abstraction level):** one-sentence GENERIC insights transfer
//!   cross-task; episode-bound literals (hashes, IDs, line numbers, paths,
//!   long verbatim quotes) are label noise. [`literal_density`] +
//!   [`admit`] are that law as a predicate.
//! - **The generator-identity ablation:** what matters is VERIFIER ACCESS,
//!   not who writes the sentence (21.4% vs 3.4% without verifier output).
//!   A deterministic extractive template over the structured diagnostic is
//!   therefore licensed — [`distill`].
//! - **The ~17-token budget:** the paper's insights average ≈17 tokens. Ours
//!   is char-based and looser ([`INSIGHT_S_MAX_CHARS`]) because rustc
//!   messages carry structure an LLM would paraphrase away; the constants
//!   are the knobs, pinned by tests.
//!
//! **Determinism pin (T1):** [`distill`] is a pure function — the same
//! verifier output yields byte-identical insight text, pinned by test. This
//! is what makes the T3 control arm meaningful: a deterministic template
//! over (error-code, shape-class) features is ORDERING under a new name
//! unless the insight text carries signal beyond those features, and the
//! paired arm is what separates conditioning from ordering.
//!
//! **Ungated by design** (the `horizon_report` law): plain `&str` inputs, not
//! `RustcErrorDiag` (that type lives in the `rustc_errors`-gated domain).
//! The intended caller-side extraction, documented once here:
//! `code` = `diag.code`; `message` = `diag.message`; `guard` = the first
//! `MachineApplicable` suggestion's message, else the first suggestion's
//! message, else `None`. The riir-train tuple miner (Plan 432 T0.4) applies
//! the same predicate at write time over teacher outputs — a boundary-forced
//! local copy (~50 lines), cross-referenced both ways.

use std::fmt::Write as _;

/// T2 — the insight length cap. The paper's ≈17-token budget; ours is
/// char-based and looser because rustc messages carry structure (the code
/// token + a shape headline + a guard clause). ~160 chars ≈ 30-40 tokens;
/// tightening it is a re-pin of this constant + the pinned tests, never an
/// edit scattered through callers.
pub const INSIGHT_S_MAX_CHARS: usize = 160;

/// T2 — the literal-density refusal threshold. `density < θ` admits (strict:
/// exactly θ refuses — the boundary is noise, not signal). 0.20 allows one
/// short span reference in a five-word insight while refusing any
/// path/line-number/hash-laden trace.
pub const INSIGHT_LITERAL_DENSITY_THETA: f32 = 0.20;

/// Cap on the message's shape headline, in words (the first clause of the
/// rustc message, e.g. "cannot borrow `x` as mutable").
const SHAPE_HEADLINE_MAX_WORDS: usize = 12;

/// Cap on the guard clause, in words (e.g. "consider cloning the value").
const GUARD_MAX_WORDS: usize = 10;

/// T1 — the distilled verifier output a retry conditions on.
///
/// Plain `&str` fields keep this module ungated; see the module doc for the
/// `RustcErrorDiag` extraction recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsightInput<'a> {
    /// The rustc/clippy error code, verbatim (`E0507`).
    pub code: &'a str,
    /// The diagnostic's top-level message (the shape headline source).
    pub message: &'a str,
    /// The best suggestion's human-readable hint, when the verifier shipped
    /// one ("consider cloning the value …"). `None` is legal — the insight
    /// degrades to code + shape headline.
    pub guard: Option<&'a str>,
}

/// T1 — deterministic extractive insight: `"{code} {shape} — {guard}"`.
///
/// The shape headline is the FIRST CLAUSE of the message (words up to the
/// first clause boundary — `,` `;` `:` — or [`SHAPE_HEADLINE_MAX_WORDS`]),
/// whitespace-collapsed. The guard is the first clause of the hint, capped
/// at [`GUARD_MAX_WORDS`]. No truncation mid-word: a clause that exceeds its
/// cap is dropped whole (an over-budget insight must FAIL admission
/// honestly, not be chopped into fake genericity).
///
/// Not guaranteed to admit — long messages or literal-laden guards produce
/// text [`admit`] refuses. That is the design: distill is extraction,
/// admission is judgment, and the two are separately testable.
#[must_use]
pub fn distill(input: &InsightInput) -> String {
    let mut out = String::with_capacity(INSIGHT_S_MAX_CHARS + 8);
    let code = input.code.trim();
    let _ = write!(out, "{code}");
    if let Some(shape) = first_clause(input.message, SHAPE_HEADLINE_MAX_WORDS) {
        out.push(' ');
        out.push_str(&shape);
    }
    if let Some(guard) = input.guard.and_then(|g| first_clause(g, GUARD_MAX_WORDS)) {
        out.push_str(" — ");
        out.push_str(&guard);
    }
    out
}

/// First clause of `text`: whitespace-collapsed words up to the first
/// clause boundary (`,`, `;`, `:` — boundary punctuation stripped from the
/// final word) or the end of the text. `None` when the text has no words or
/// the clause EXCEEDS `max_words` — an over-budget clause is dropped whole
/// (see [`distill`]): word-chopping it would fake genericity out of a
/// literal-laden trace the admission predicate must see honestly.
fn first_clause(text: &str, max_words: usize) -> Option<String> {
    let mut words: Vec<&str> = Vec::new();
    for w in text.split_whitespace() {
        let trimmed = w.trim_end_matches([',', ';', ':']);
        if trimmed.is_empty() {
            continue; // a bare boundary token carries no words
        }
        if trimmed.len() != w.len() {
            words.push(trimmed);
            break; // boundary punctuation ends the clause
        }
        words.push(w);
    }
    if words.is_empty() || words.len() > max_words {
        None
    } else {
        Some(words.join(" "))
    }
}

/// T2 — fraction of whitespace tokens that are episode-bound literals.
///
/// An episode literal is a token that pins the insight to ONE site/run
/// rather than a rule (the paper's label-noise class):
/// - hex hashes: ≥8 chars, all hex digits (`a1b2c3d4e5f60718`);
/// - line numbers: ≥3 chars, all decimal digits (the code token `E0507`
///   is not all-digit and never matches);
/// - paths: contains a separator (`/` or `\`) AND a dot;
/// - mangled/verbatim blobs: any single token ≥24 chars.
///
/// Leading/trailing punctuation (backticks, quotes, parens) is ignored for
/// the classification — `` `x` `` is a normal token, `` `mangled_…` `` is
/// caught by the length rule. Empty text is density 0.0 (admission's
/// emptiness check is separate).
#[allow(clippy::cast_precision_loss)] // token counts -> f32 — a ratio, not a knob
#[must_use]
pub fn literal_density(text: &str) -> f32 {
    let mut total = 0usize;
    let mut literals = 0usize;
    for tok in text.split_whitespace() {
        total += 1;
        if is_episode_literal(tok) {
            literals += 1;
        }
    }
    if total == 0 {
        return 0.0;
    }
    literals as f32 / total as f32
}

/// Token-level episode-literal test — see [`literal_density`] for the
/// classes and their rationale.
#[must_use]
fn is_episode_literal(tok: &str) -> bool {
    let t: &str = tok.trim_matches(|c: char| {
        !(c.is_ascii_alphanumeric() || c == '/' || c == '\\' || c == '_' || c == '.')
    });
    if t.len() >= 24 {
        return true; // mangled names / verbatim blob fragments
    }
    if t.len() >= 8
        && t.chars().all(|c| c.is_ascii_hexdigit())
        && t.chars().any(|c| c.is_ascii_alphabetic())
    {
        return true; // hex hash (alphabetic-present excludes plain line numbers)
    }
    if t.len() >= 3 && t.chars().all(|c| c.is_ascii_digit()) {
        return true; // line/column numbers
    }
    if (t.contains('/') || t.contains('\\')) && t.contains('.') {
        return true; // path-shaped
    }
    false
}

/// T2 — the abstraction-level admission predicate:
/// `admit(f) ⇔ non-empty ∧ len(f) ≤ S_max ∧ literal_density(f) < θ`.
///
/// Both directions are load-bearing: an empty string and a 200-char essay
/// refuse on the length leg; a path/line/hash-laden trace refuses on the
/// density leg even when short. The strict `<` means exactly θ refuses.
#[must_use]
pub fn admit(insight: &str) -> bool {
    let trimmed = insight.trim();
    !trimmed.is_empty()
        && trimmed.len() <= INSIGHT_S_MAX_CHARS
        && literal_density(trimmed) < INSIGHT_LITERAL_DENSITY_THETA
}

/// Issue 140 T3 — the retry-family key's shape half: the message's
/// leading clause with backticked identifier spans blanked.
///
/// `"cannot borrow `x` as mutable, as it is behind…"` →
/// `"cannot borrow _ as mutable"`. Two sites with different variable
/// names but the same error SHAPE collapse to one family — the paper's
/// L1 abstraction (generic insights transfer) applied at the KEY level:
/// the family is what a distilled insight is ABOUT, so episode
/// identifiers must not split it.
///
/// Differences from [`distill`]'s clause extraction, both deliberate:
/// - backticked spans blank to `_` FIRST (the key must not name episode
///   variables);
/// - an over-long clause TRUNCATES at [`SHAPE_HEADLINE_MAX_WORDS`]
///   words rather than dropping whole — a key needs collision
///   CONSISTENCY, not admission honesty; dropping would fold every
///   long-message shape into one `""` family and merge unrelated
///   errors.
///
/// Deterministic: same message → byte-identical class.
#[must_use]
pub fn shape_class(message: &str) -> String {
    // Pass 1: blank backticked spans (an unclosed backtick blanks to the
    // end — conservative, same class as any other blanked tail).
    let mut blanked = String::with_capacity(message.len());
    let mut in_ticks = false;
    for ch in message.chars() {
        if ch == '`' {
            if in_ticks {
                blanked.push('_');
            }
            in_ticks = !in_ticks;
            continue;
        }
        if !in_ticks {
            blanked.push(ch);
        }
    }
    if in_ticks {
        blanked.push('_');
    }
    // Pass 2: first clause, whitespace-collapsed, capped.
    let mut words: Vec<&str> = Vec::new();
    for w in blanked.split_whitespace() {
        let trimmed = w.trim_end_matches([',', ';', ':']);
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.len() != w.len() {
            words.push(trimmed);
            break; // clause boundary ends the headline
        }
        words.push(w);
        if words.len() == SHAPE_HEADLINE_MAX_WORDS {
            break; // cap — truncate, never drop (see the module-level doc)
        }
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e0507_input() -> InsightInput<'static> {
        InsightInput {
            code: "E0507",
            message: "cannot borrow `x` as mutable, as it is behind a `&` reference",
            guard: Some("consider cloning the value, which reduces the borrows"),
        }
    }

    /// T1 determinism pin — the paper's loop requires the same verifier
    /// output to condition retries identically.
    #[test]
    fn distill_is_deterministic() {
        let a = distill(&e0507_input());
        let b = distill(&e0507_input());
        assert_eq!(a, b);
    }

    /// T1 shape — code + FIRST CLAUSE of the message + first clause of the
    /// guard. The trailing clauses ("as it is behind a `&` reference",
    /// "to reduce the number of borrows") are dropped at their commas.
    #[test]
    fn distill_takes_first_clauses() {
        assert_eq!(
            distill(&e0507_input()),
            "E0507 cannot borrow `x` as mutable — consider cloning the value"
        );
    }

    /// Guard-less degradation: code + shape headline only. The colon in
    /// "use of moved value: `a`" ENDS the clause — the episode identifier
    /// `` `a` `` is dropped by extraction, which is exactly the L1 behavior
    /// we want from a deterministic distiller.
    #[test]
    fn distill_without_guard_degrades_to_shape() {
        let out = distill(&InsightInput {
            code: "E0382",
            message: "use of moved value: `a`",
            guard: None,
        });
        assert_eq!(out, "E0382 use of moved value");
    }

    /// An over-budget message clause is DROPPED WHOLE, not truncated
    /// mid-word — chopping would fake genericity out of a literal-laden
    /// trace (the admission predicate must see the honest text).
    #[test]
    fn distill_drops_overlong_clauses_whole() {
        let long_msg = "word ".repeat(20).trim().to_string() + ", second clause";
        let out = distill(&InsightInput {
            code: "E0507",
            message: &long_msg,
            guard: None,
        });
        assert_eq!(out, "E0507");
    }

    /// Whitespace collapse — multi-space/newline inputs normalize.
    #[test]
    fn distill_collapses_whitespace() {
        let out = distill(&InsightInput {
            code: "E0507",
            message: "cannot   borrow\n `x`   as mutable, as it is behind",
            guard: None,
        });
        assert_eq!(out, "E0507 cannot borrow `x` as mutable");
    }

    /// The canonical generic insight ADMITS (the paper's transfer class).
    #[test]
    fn admit_passes_generic_insight() {
        let insight = distill(&e0507_input());
        assert!(
            admit(&insight),
            "distilled E0507 insight must admit: {insight}"
        );
        assert!(literal_density(&insight) < INSIGHT_LITERAL_DENSITY_THETA);
    }

    /// The paper's label-noise class REFUSES: a trace pinned to one
    /// episode by path + line numbers + hash.
    #[test]
    fn admit_refuses_literal_laden_trace() {
        let trace =
            "fix failed at /Users/x/git/repo/src/lib.rs:483 hash a1b2c3d4e5f60718 line 1204";
        assert!(!admit(trace));
        assert!(literal_density(trace) >= INSIGHT_LITERAL_DENSITY_THETA);
    }

    /// Each literal class fires in isolation (the predicate's per-class
    /// arms — one broken class would silently widen admission).
    #[test]
    fn literal_classes_fire_in_isolation() {
        // hex hash (≥8 hex chars, alphabetic present)
        assert!(is_episode_literal("a1b2c3d4e5f6"));
        // line number (≥3 digits)
        assert!(is_episode_literal("483"));
        // path (separator + dot)
        assert!(is_episode_literal("src/lib.rs"));
        // mangled blob (≥24 chars)
        assert!(is_episode_literal("__internal_mangled_symbol_x09"));
        // and the non-literals:
        assert!(!is_episode_literal("E0507"), "code token is not a literal");
        assert!(!is_episode_literal("`x`"), "short ident is not a literal");
        assert!(!is_episode_literal("mutable"));
        assert!(!is_episode_literal("48"), "2-digit number is not a literal");
        assert!(
            is_episode_literal("12345678"),
            "all-digit 8 chars hits the line-number class (hex needs an alphabetic)"
        );
    }

    /// The length leg refuses independently of density.
    #[test]
    fn admit_refuses_overlong() {
        let essay = "E0507 ".to_string() + &"generic words ".repeat(30);
        assert!(literal_density(&essay) < INSIGHT_LITERAL_DENSITY_THETA);
        assert!(essay.trim().len() > INSIGHT_S_MAX_CHARS);
        assert!(!admit(&essay));
    }

    /// Empty refuses (the 0/0 density corner must not admit vacuously).
    #[test]
    fn admit_refuses_empty() {
        assert!(!admit(""));
        assert!(!admit("   "));
        assert!((literal_density("") - 0.0).abs() < f32::EPSILON);
    }

    /// Constants pinned — they are the paper-law knobs (17-token budget,
    /// θ threshold); a change is a re-pin, visible in the diff.
    #[test]
    fn constants_are_pinned() {
        assert_eq!(INSIGHT_S_MAX_CHARS, 160);
        let theta: f32 = INSIGHT_LITERAL_DENSITY_THETA;
        assert!((theta - 0.20_f32).abs() < f32::EPSILON);
    }

    /// Boundary arithmetic: a density EXACTLY θ refuses (strict <).
    #[test]
    fn boundary_theta_refuses() {
        // 5 tokens, 1 literal -> exactly 0.2 == theta -> refuse
        let text = "one two three four 483";
        let d = literal_density(text);
        assert!(
            (d - 0.2_f32).abs() < 1e-6,
            "density should be exactly 0.2, got {d}"
        );
        assert!(!admit(text));
        // 6 tokens, 1 literal -> 1/6 < 0.2 -> admit
        let text6 = "one two three four five 483";
        assert!(literal_density(text6) < INSIGHT_LITERAL_DENSITY_THETA);
        assert!(admit(text6));
    }

    /// A realistic rustc guard hint with backticked code spans still
    /// admits when the spans are short (structural, not episode-bound).
    #[test]
    fn backticked_short_spans_admit() {
        let insight = "E0507 cannot borrow `x` as mutable — replace `&x` with `&mut x`";
        assert!(admit(insight));
    }

    // ── Issue 140 T3: shape_class ──────────────────────────────────────

    /// Backticked identifiers blank — different variables, same error
    /// shape, ONE family (the cross-task transfer the paper measures).
    #[test]
    fn shape_class_blanks_backticked_identifiers() {
        let a = shape_class("cannot borrow `x` as mutable, as it is behind a `&` reference");
        let b =
            shape_class("cannot borrow `owned_value` as mutable, as it is behind a `&` reference");
        assert_eq!(a, "cannot borrow _ as mutable");
        assert_eq!(a, b, "episode identifiers must not split the family");
    }

    /// Clause split + whitespace collapse; the `:` in the moved-value
    /// message ends the clause exactly as in `distill`.
    #[test]
    fn shape_class_splits_at_clause_boundaries() {
        assert_eq!(shape_class("use of moved value: `a`"), "use of moved value");
        assert_eq!(
            shape_class("  cannot   borrow\n `v`  as mutable, more"),
            "cannot borrow _ as mutable"
        );
        assert_eq!(shape_class(""), "");
    }

    /// An over-long clause TRUNCATES at the word cap instead of dropping
    /// whole — the key needs collision consistency (a drop would merge
    /// every long-message shape into one empty family).
    #[test]
    fn shape_class_truncates_long_clauses() {
        let long = "word ".repeat(30).trim().to_string() + ", tail";
        let cls = shape_class(&long);
        assert_eq!(cls.split(' ').count(), SHAPE_HEADLINE_MAX_WORDS);
        assert!(cls.starts_with("word word"));
        // The capped key is STABLE — the same long message maps to the
        // same class every time (determinism pin).
        assert_eq!(cls, shape_class(&long));
    }

    /// An unclosed backtick blanks to the end (conservative — the same
    /// class as any other blanked tail, never a distinct literal-bearing
    /// key).
    #[test]
    fn shape_class_blanks_unclosed_backtick_to_end() {
        assert_eq!(shape_class("no method named `deref"), "no method named _");
    }
}
