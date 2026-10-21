//! The span-shape embedding (Issue 107 T2) — the fixseq ring's embedding
//! column producer.
//!
//! A deterministic, modelless, zero-dep feature hashing of a fix-compile
//! span's ORIGINAL trigger text (`FixCandidate::shape` — the bytes the
//! strategy acts on): tokenize, hash unigrams + adjacent bigrams into a
//! fixed bucket count with signed accumulation (the feature-hashing trick,
//! Weinberger et al. 2009 — signs make bucket collisions cancel in
//! expectation instead of co-adding), L2-normalize, and quantize to i8.
//!
//! Why write-time capture (the `latent-encode-at-capture` rule, not
//! post-hoc): the site's bytes DRIFT as fixes land — the ring exists to
//! record a mutating loop — and [`span_hash`](crate::domains::rustc_errors::driver)
//! is a SITE id (blake3 of `file:line`), not a content hash. Offline
//! re-embedding by `span_hash` would read post-run bytes for a pre-fix
//! state; the column must land at attempt time or it is wrong forever.
//!
//! Why i8-quantized (×127, L2-normalized first): the ring's
//! `AttemptRecord` keeps `Eq` (the Issue-105 integer-stamping precedent —
//! `journey_residual_sum` is u32 for exactly this reason), the jsonl row
//! stays compact (~32 integers), and sub-1% rounding loss is noise for a
//! linear probe. Pre-T2 rings decode the column as `None`
//! (`#[serde(default)]` on the record).
//!
//! Ungated by design (the `horizon_report` law): the producer is consumed
//! by the default-on `rustc_errors` driver at ring-write time AND readable
//! from every feature build; `blake3` is already an unconditional dep.
//! Determinism: blake3 output, explicit tokenization over bytes, explicit
//! rounding — same input, same vector, across machines and versions.

/// The embedding dimensionality. 32 buckets over unigram+bigram features:
/// family vocabularies at one span are tens of distinct tokens, so 32
/// signed buckets carry the contrast without making the LORO ridge
/// (41-dim with the 9 ring-native features) sample-starved.
pub const SPAN_EMBED_DIMS: usize = 32;

/// The quantization scale — decoded dims are `i8 / 127.0` in `[-1, 1]`.
pub const SPAN_EMBED_SCALE: f32 = 127.0;

/// Is this byte part of an identifier token? ASCII alnum + `_` + every
/// non-ASCII byte (UTF-8 continuation/lead bytes fold into the surrounding
/// token — the exact codepoint never matters, only the bucket hash).
const fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

/// Tokenize `text` into identifier runs and single-char sigils, yielding
/// byte slices (borrowed, zero-alloc). Whitespace separates tokens and is
/// dropped — indentation never enters the embedding.
fn tokens(text: &str) -> impl Iterator<Item = &[u8]> {
    let bytes = text.as_bytes();
    let mut i = 0;
    std::iter::from_fn(move || {
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_whitespace() {
                i += 1;
            } else if is_ident_byte(b) {
                let start = i;
                while i < bytes.len() && is_ident_byte(bytes[i]) {
                    i += 1;
                }
                return Some(&bytes[start..i]);
            } else {
                i += 1;
                return Some(&bytes[i - 1..i]);
            }
        }
        None
    })
}

/// Bucket + sign of one feature: blake3 → (bucket % dims, ±1). The sign
/// comes from a byte the bucket does not read, so bucket and sign are
/// independent.
///
/// `pub(crate)` — the trained two-tower posture (Issue 126) reuses the exact
/// tokenization + bucket law at its own bucket count; a second copy would be
/// a second thing to get wrong.
pub(crate) fn bucket_sign(feature: &[u8], dims: usize) -> (usize, i8) {
    let h = blake3::hash(feature);
    let b = h.as_bytes();
    let bucket = (u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize) % dims;
    let sign = if b[4] & 1 == 0 { 1 } else { -1 };
    (bucket, sign)
}

/// Walk `text`'s tokens into a DENSE signed unigram+bigram bag over `dims`
/// buckets, L2-normalized in place (zero vector for empty/whitespace-only
/// text). `pub(crate)` — shared with the Issue-126 feature hashing.
pub(crate) fn fill_hashbag(text: &str, dims: usize, acc: &mut [f32]) {
    for v in acc.iter_mut() {
        *v = 0.0;
    }
    let mut prev: Option<&[u8]> = None;
    for tok in tokens(text) {
        let (b, s) = bucket_sign(tok, dims);
        acc[b] += f32::from(s);
        if let Some(p) = prev {
            // Bigram input = p ++ ' ' ++ tok — no token contains a space
            // (whitespace separates), so the join is unambiguous.
            let mut joined = Vec::with_capacity(p.len() + 1 + tok.len());
            joined.extend_from_slice(p);
            joined.push(b' ');
            joined.extend_from_slice(tok);
            let (b2, s2) = bucket_sign(&joined, dims);
            acc[b2] += f32::from(s2);
        }
        prev = Some(tok);
    }
    let norm = acc.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in acc.iter_mut() {
            *v /= norm;
        }
    }
}

/// Embed the span text: hashed unigram + adjacent-bigram bag, signed,
/// L2-normalized, ×127 i8-quantized. Empty/whitespace-only text → the zero
/// vector (an honest "no bytes captured", never a panic).
#[must_use]
pub fn span_embed(text: &str) -> [i8; SPAN_EMBED_DIMS] {
    let mut acc = [0.0_f32; SPAN_EMBED_DIMS];
    fill_hashbag(text, SPAN_EMBED_DIMS, &mut acc);
    let mut out = [0_i8; SPAN_EMBED_DIMS];
    for (o, v) in out.iter_mut().zip(acc.iter()) {
        // fill_hashbag L2-normalized, so |v| ≤ 1; round-then-clamp keeps the
        // cast exact (|rounded| ≤ 127 < i8::MAX + 1).
        #[allow(clippy::cast_possible_truncation)] // post-clamp: exact
        #[allow(clippy::cast_sign_loss)] // i8 carries the sign, not u32
        {
            *o = (v * SPAN_EMBED_SCALE)
                .round()
                .clamp(-SPAN_EMBED_SCALE, SPAN_EMBED_SCALE) as i8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_whitespace_only_is_zero() {
        assert_eq!(span_embed(""), [0; SPAN_EMBED_DIMS]);
        assert_eq!(span_embed("  \n\t "), [0; SPAN_EMBED_DIMS]);
    }

    #[test]
    fn deterministic() {
        let text = "let mut v = Vec::<u8>::new(); v.push(1);";
        assert_eq!(span_embed(text), span_embed(text));
    }

    #[test]
    fn whitespace_insensitive() {
        // Same token stream, different spacing/indentation.
        let a = span_embed("let mut v = Vec::new();");
        let b = span_embed("let\tmut\nv  =   Vec :: new ( ) ;");
        assert_eq!(a, b, "whitespace separates tokens and nothing else");
    }

    #[test]
    fn distinct_texts_differ() {
        let a = span_embed("let mut v = Vec::new();");
        let b = span_embed("let mut w = Vec::new();");
        assert_ne!(a, b, "one token difference must move a bucket");
    }

    #[test]
    fn order_sensitive_beyond_unigrams() {
        // "a b" and "b a" share unigrams, differ in bigrams.
        let a = span_embed("alpha beta");
        let b = span_embed("beta alpha");
        assert_ne!(a, b);
    }

    #[test]
    fn quantized_norm_bound_holds() {
        let e = span_embed("pub fn probe() -> usize { let v = Vec::new(); v.push(1); v.len() }");
        let l2: f32 = e
            .iter()
            .map(|&v| {
                #[allow(clippy::cast_precision_loss)] // i8 → f32 is exact
                {
                    let f = f32::from(v) / SPAN_EMBED_SCALE;
                    f * f
                }
            })
            .sum();
        // L2-normalized to 1 before quantization; rounding costs < 1 per
        // dim, so the decoded norm stays within ~sqrt(32)/127 ≈ 4.5% of 1.
        #[allow(clippy::cast_precision_loss)]
        let slack = (SPAN_EMBED_DIMS as f32).sqrt() / SPAN_EMBED_SCALE;
        assert!(
            (l2.sqrt() - 1.0).abs() <= slack,
            "decoded L2 norm {l2:.4} outside quantization slack"
        );
        assert!(
            e.iter().all(|&v| v >= -127),
            "i8 lower bound holds by construction"
        );
    }

    #[test]
    fn non_ascii_folds_into_tokens_without_panic() {
        // Unicode string content is valid Rust source; bytes ≥ 0x80 join
        // the token stream deterministically.
        let a = span_embed("let s = \"日本語テキスト\";");
        let b = span_embed("let s = \"日本語テキスト\";");
        assert_eq!(a, b);
        assert_ne!(a, [0; SPAN_EMBED_DIMS]);
    }
}
