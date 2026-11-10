//! Input-scope model for the heal pipeline — the Bench 669 consumer adoption
//! of katgpt-core's `contrastive_scope` primitive (Issue 674 / Research 493,
//! arXiv:2608.13545).
//!
//! The Issue 030 relevance gate on [`HealPipeline::heal`] is **rule-coverage
//! only**: `rule_in_corpus || source != Memory`. It cannot catch the
//! complementary failure — an out-of-scope INPUT (non-Rust code, garbage, a
//! mis-routed span) paired with an in-corpus rule id. In that cell the
//! drafter serves the corpus template, it parses, the rule is covered → the
//! pipeline **serves a Rust fix for a Python input with a straight face**, and
//! on the L4 path the same garbage burns a ~48 s GPU call.
//!
//! This module closes that axis with the 2D gate:
//!
//! | rule coverage | input scope | behavior |
//! |---|---|---|
//! | any | out (`D > θ`) | decline immediately, L4 never consulted |
//! | any | in | today's path; `evolution_score × sigmoid(−κ·D)` |
//!
//! ## Honest scope claim (the Issue 020 boundary)
//!
//! This gates **Rust-vs-not-Rust + garbage**, NOT cross-domain Rust. Lexical
//! domain routing between the healer's own Rust domains measured a ~70%
//! accuracy ceiling and was killed for gating (Issue 020 / Bench 025) — a perf
//! `HashMap` rule and a clippy `HashMap` rule share a token spectrum. The
//! out-corpus here is deliberately NON-Rust, so the discrimination is
//! near-trivial (Rust identifiers/punctuation are absent from the out side
//! and vice versa) and the claim is kept small. A caller who wants
//! cross-domain gating should re-read Bench 025 before building one.
//!
//! ## κ / θ pinning (consumer-specific — the Bench 669 caveat)
//!
//! The katgpt-core defaults (`κ = 0.05`, `θ = 8.0`) are POC-scale. This
//! consumer pins both from measurement over the default healer corpora:
//!
//! - **θ** sits inside the measured gap between the worst (least-negative)
//!   in-distribution D and the best (least-positive) held-out OOS D, with
//!   margin both sides — see [`THETA`] and the separation-pinning test.
//! - **κ** is steep enough that `sigmoid(−κ·|D|)` saturates to **exactly
//!   `1.0f32`** for every in-distribution corpus fixture (f32 `sigmoid`
//!   saturates for arguments ≥ ~16.6), which makes the in-distribution
//!   heal path **bit-identical** with the model installed vs not — the
//!   load-bearing G3 no-regression property.
//!
//! D scales with document length (a sum over tokens), so long real files are
//! robustly in-scope and the honest gray zone is short mixed snippets — a
//! documented property of the Naive-Bayes document score, not a defect.
//!
//! ## Tokenizer
//!
//! Zero-alloc lexical scan: maximal identifier runs (`[A-Za-z0-9_]`), maximal
//! punctuation runs, digits. Case-sensitive (Rust's `fn`/`Fn` distinction is
//! signal). Tokens absent from the vocab are skipped — for a token unseen in
//! BOTH corpora the smoothed log-odds is `log2((N_in+αV)/(N_out+αV)) ≈ 0`
//! when the corpora are comparable size, so skipping is the same answer as
//! scoring it at ~0 without growing the table.

use std::collections::HashMap;

use katgpt_core::contrastive_scope::{
    ContrastiveScoreBuilder, ContrastiveScoreTable, DEFAULT_ALPHA, ScopeGate, ScopeVerdict,
};

use crate::domain::HealEntry;

/// Decline threshold `θ`, pinned from the measured in/OOS gap over the
/// default healer corpora (the separation-pinning test in
/// `tests/scope_gate_adoption.rs` fails loudly if the gap closes).
///
/// Measured landscape (2026-08-20, 40-fixture clippy seed corpus in /
/// held-out non-Rust probes out): corpus D ∈ [−177, −3.9] (the −3.9 outlier
/// is the one-line `redundant_static_lifetimes` fixture); held-out OOS D
/// ≥ +20 for every probe whose language family the out-corpus covers.
/// `θ = 0` declines any input that leans out at ALL, passes neutral
/// (`D = 0`, all-unseen tokens — absence of evidence is not decline), and
/// keeps a 3.9-bit in-side margin.
///
/// **Honest limitation:** garbage in a language family the out-corpus does
/// NOT cover scores neutral (unseen tokens are symmetric-smoothed to ≈ 0)
/// and passes. The gate discriminates over the token space it was built on.
pub const THETA: f32 = 0.0;

/// Haircut steepness `κ`, pinned so the worst (least-negative) in-distribution
/// corpus fixture — `redundant_static_lifetimes`, the one-liner at D = −2.08
/// (re-measured 2026-10-26, issue 172; it was −7.6 at Bench 040 and drifted
/// as the vocab grew) — saturates f32 `sigmoid` to EXACTLY `1.0` (argument
/// 8.5·2.08 = 17.7 ≥ 16.6), making the in-distribution
/// heal path bit-identical with the model installed vs not. With `θ = 0` the
/// haircut band is `D ≤ 0` (factor ∈ [0.5, 1.0] — weak-evidence inputs get
/// discounted toward the neutral point, strong in-scope inputs untouched).
/// If the corpus grows a shorter fixture, the separation-pinning test fails
/// loudly and this constant moves.
pub const KAPPA: f32 = 8.5;

/// The canonical out-of-scope corpus (table-building half).
///
/// Representative non-Rust documents an L4 daemon client or API caller could
/// realistically send: other languages, config formats, prose. Held-out probes
/// (test-only, never used to build the table) are a DIFFERENT set — the
/// Report-the-Floor discipline from Bench 669's probe battery.
pub const OUT_OF_SCOPE_DOCS: &[&str] = &[
    // Python
    r"import os
def process_items(items, limit):
    total = 0
    for item in items:
        if item is None:
            continue
        total += item.get('value', 0)
    return total if total < limit else None
",
    // JavaScript
    r#"export async function fetchUserData(userId) {
  const response = await fetch(`/api/users/${userId}`);
  if (!response.ok) throw new Error("fetch failed");
  const data = await response.json();
  return data.map(u => ({ id: u.id, name: u.name ?? "anon" }));
}
"#,
    // WGSL
    r"@group(0) @binding(0) var<storage, read_write> output: array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
  let i = gid.x;
  if (i >= arrayLength(&output)) { return; }
  output[i] = output[i] * 2.0 + 1.0;
}
",
    // GLSL
    r"#version 450
layout(location = 0) in vec3 position;
layout(location = 0) out vec4 fragColor;
void main() {
  vec3 normalized = normalize(position);
  fragColor = vec4(normalized, 1.0);
}
",
    // JSON
    r#"{"name": "heal-config", "version": 2, "rules": [
  {"id": "needless_return", "severity": "warn", "fix": true},
  {"id": "match_bool", "severity": "allow", "fix": false}
], "thresholds": {"theta": 4.0, "kappa": 0.4}}
"#,
    // YAML
    r"version: 2
rules:
  - id: needless_return
    severity: warn
    fix: true
thresholds:
  theta: 4.0
  kappa: 0.4
",
    // TOML
    r#"[thresholds]
theta = 4.0
kappa = 0.4

[[rules]]
id = "needless_return"
severity = "warn"
fix = true
"#,
    // SQL
    r"SELECT u.id, u.name, COUNT(o.id) AS order_count
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
WHERE u.created_at >= '2026-01-01'
GROUP BY u.id, u.name
HAVING COUNT(o.id) > 5
ORDER BY order_count DESC;
",
    // Go
    r#"package main

import "fmt"

func processItems(items []string, limit int) int {
	total := 0
	for _, item := range items {
		if item == "" {
			continue
		}
		total++
	}
	return total
}
"#,
    // HTML
    r#"<!DOCTYPE html>
<html lang="en">
<head><meta charset="utf-8"><title>Report</title></head>
<body>
  <div class="summary" id="main">42 fixes applied</div>
  <script src="/static/app.js" defer></script>
</body>
</html>
"#,
    // Markdown prose
    r"# Healing Report

The pipeline applied **twelve** fixes across three files today.
See the changelog for details on which rules fired. Note that the
`needless_return` rule accounts for most of the diff.
",
    // prose
    "The quick brown fox jumps over the lazy dog. This sentence exists to \
     provide ordinary English prose tokens with no code structure at all, \
     which is exactly the kind of garbage a misconfigured client sends.",
    // CSS (the layout-token family — without it a CSS probe is all-unseen,
    // which is NEUTRAL, not out — the NB table discriminates only over the
    // token space it was built on)
    r".summary {
  color: #333;
  font-size: 14px;
  margin: 0 auto;
  padding: 8px 16px;
}
.error { color: red; font-weight: bold; }
",
    // C++
    r"#include <vector>
#include <algorithm>
std::vector<int> filter(const std::vector<int>& input, int threshold) {
    std::vector<int> output;
    for (const auto& element : input) {
        if (element > threshold) output.push_back(element);
    }
    return output;
}
",
    // Java
    r"import java.util.List;
import java.util.ArrayList;
public class Handler {
    private final int limit;
    public Handler(int limit) { this.limit = limit; }
    public List<String> handle(List<String> input) {
        List<String> out = new ArrayList<>();
        for (String s : input) {
            if (s.length() < limit) out.add(s);
        }
        return out;
    }
}
",
    // Ruby
    r"def process_items(items, limit)
  total = 0
  items.each do |item|
    next if item.nil?
    total += item[:value] unless item[:value] > limit
  end
  total
end
",
    // shell
    r#"#!/usr/bin/env bash
set -euo pipefail
for file in "$@"; do
  if [[ -f "$file" ]]; then
    grep -c warning "$file" || true
  fi
done
"#,
    // XML
    r#"<?xml version="1.0" encoding="UTF-8"?>
<report version="2">
  <summary fixes="12" warnings="3"/>
  <rules>
    <rule id="needless_return" fix="true"/>
  </rules>
</report>
"#,
];

/// A frozen input-scope model: string vocab + the contrastive table + gate.
///
/// Built once (cold path) from the caller's corpus; read per-heal (hot path,
/// zero-alloc via scratch reuse). `Arc`-shareable — the table is immutable
/// after `finish()`.
pub struct ScopeModel {
    vocab: HashMap<String, u32>,
    table: ContrastiveScoreTable,
    gate: ScopeGate,
}

impl ScopeModel {
    /// Build from raw document strings with explicit gate parameters.
    ///
    /// Vocab assignment is deterministic (sorted unique tokens), so two
    /// builds over the same corpora produce bit-identical tables — the
    /// BLAKE3 commitment in the table stays reproducible.
    ///
    /// # Panics
    ///
    /// Panics if the corpus vocabulary exceeds `u32::MAX` entries (a corpus
    /// of megabytes at most — unreachable in practice).
    #[must_use]
    pub fn build_with(in_docs: &[&str], out_docs: &[&str], kappa: f32, theta: f32) -> Self {
        // Deterministic vocab: collect unique tokens, sort, assign ids.
        let mut tokens: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for doc in in_docs.iter().chain(out_docs) {
            for tok in tokenize(doc) {
                if seen.insert(tok.to_string()) {
                    tokens.push(tok.to_string());
                }
            }
        }
        tokens.sort_unstable();
        let vocab: HashMap<String, u32> = tokens
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let id = u32::try_from(i)
                    .expect("corpus vocabulary fits in u32 (it is megabytes at most)");
                (t.clone(), id)
            })
            .collect();

        let mut builder = ContrastiveScoreBuilder::new(vocab.len(), DEFAULT_ALPHA);
        for doc in in_docs {
            builder.observe_in(&map_tokens(doc, &vocab));
        }
        for doc in out_docs {
            builder.observe_out(&map_tokens(doc, &vocab));
        }
        Self {
            vocab,
            table: builder.finish(),
            gate: ScopeGate { kappa, theta },
        }
    }

    /// Build with this consumer's pinned κ/θ ([`KAPPA`] / [`THETA`]).
    #[must_use]
    pub fn build(in_docs: &[&str], out_docs: &[&str]) -> Self {
        Self::build_with(in_docs, out_docs, KAPPA, THETA)
    }

    /// Build for a domain corpus: the in-side is every entry's `buggy_code`
    /// (the input shape the pipeline is FOR), the out-side is the canonical
    /// [`OUT_OF_SCOPE_DOCS`].
    #[must_use]
    pub fn build_for_corpus<E: HealEntry>(corpus: &[E]) -> Self {
        let in_docs: Vec<&str> = corpus.iter().map(HealEntry::buggy_code).collect();
        Self::build(&in_docs, OUT_OF_SCOPE_DOCS)
    }

    /// The gate configuration (for inspection / re-pinning tests).
    #[must_use]
    pub fn gate(&self) -> &ScopeGate {
        &self.gate
    }

    /// The underlying table (for the probe battery + commitment checks).
    #[must_use]
    pub fn table(&self) -> &ContrastiveScoreTable {
        &self.table
    }

    /// `D(input)` for a raw source string. Zero-alloc (scratch reuse).
    #[must_use]
    pub fn score_str(&self, input: &str, scratch: &mut Vec<u32>) -> f32 {
        scratch.clear();
        scratch.extend(tokenize(input).filter_map(|tok| self.vocab.get(tok).copied()));
        katgpt_core::contrastive_scope::scope_score(&self.table, scratch)
    }

    /// Full verdict over a raw source string: haircut + decline decision.
    ///
    /// `confidence` is the nominal confidence to haircut (the pipeline passes
    /// `1.0` and multiplies the served `evolution_score` by the resulting
    /// haircut — exactly `1.0` for strongly in-scope inputs, by saturation).
    #[must_use]
    pub fn verdict_str(
        &self,
        input: &str,
        confidence: f32,
        scratch: &mut Vec<u32>,
    ) -> ScopeVerdict {
        let d = self.score_str(input, scratch);
        self.gate.apply(confidence, d)
    }
}

/// Lexical tokenizer: maximal identifier runs, maximal punctuation runs.
///
/// Yields `&str` slices of the input (zero-alloc). Whitespace separates.
fn tokenize(input: &str) -> impl Iterator<Item = &str> {
    fn is_word(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }
    fn is_punct(c: char) -> bool {
        !c.is_whitespace() && !is_word(c)
    }

    let mut rest = input;
    std::iter::from_fn(move || {
        // Skip whitespace between tokens — it never forms a token.
        let non_ws = rest.trim_start();
        rest = non_ws;
        let mut char_indices = rest.char_indices();
        let (_, first) = char_indices.next()?;
        let word = is_word(first);
        let end = char_indices
            .find(|(_, c)| if word { !is_word(*c) } else { !is_punct(*c) })
            .map_or(rest.len(), |(i, _)| i);
        let (tok, next) = rest.split_at(end);
        rest = next;
        Some(tok)
    })
}

/// Map a document's tokens to vocab ids (unseen tokens dropped — see the
/// module doc for the symmetric-smoothing argument).
fn map_tokens(doc: &str, vocab: &HashMap<String, u32>) -> Vec<u32> {
    tokenize(doc)
        .filter_map(|tok| vocab.get(tok).copied())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> ScopeModel {
        let in_docs: &[&str] = &[
            "fn f(x: i32) -> i32 { return x + 1; }",
            "let v: Vec<u8> = Vec::with_capacity(10);",
            "match opt { Some(v) => v, None => 0 }",
            "pub struct Config { pub(crate) threshold: f32 }",
            "impl Default for Config { fn default() -> Self { Self { threshold: 0.5 } } }",
            "use std::collections::HashMap;",
            "for (i, item) in items.iter().enumerate() { let _ = i; }",
            "let s: &str = \"literal\"; let owned = s.to_string();",
        ];
        ScopeModel::build(in_docs, OUT_OF_SCOPE_DOCS)
    }

    /// A realistic held-out Python probe (NOT one of the table-building docs):
    /// multi-line, Python-characteristic tokens dominate the shared punctuation.
    const PY_PROBE: &str = "import json\ndef load_config(path):\n    with open(path, 'r') as f:\n        data = json.load(f)\n    if not isinstance(data, dict):\n        raise ValueError('expected object')\n    return data.get('rules', [])\n";

    #[test]
    fn tokenizer_splits_identifiers_and_punctuation_runs() {
        let toks: Vec<&str> = tokenize("fn f(x: i32) -> i32 { x+1 }").collect();
        assert_eq!(
            toks,
            vec![
                "fn", "f", "(", "x", ":", "i32", ")", "->", "i32", "{", "x", "+", "1", "}"
            ]
        );
    }

    #[test]
    fn rust_inputs_score_negative_and_python_positive() {
        let m = model();
        let mut scratch = Vec::new();
        let d_rust = m.score_str("fn process(x: u64) -> u64 { x * 2 }", &mut scratch);
        let d_python = m.score_str(PY_PROBE, &mut scratch);
        assert!(d_rust < 0.0, "Rust input must lean in-scope, got {d_rust}");
        assert!(
            d_python > 0.0,
            "Python input must lean out-scope, got {d_python}"
        );
    }

    #[test]
    fn verdict_declines_python_and_passes_rust() {
        let m = model();
        let mut scratch = Vec::new();
        let v_rust = m.verdict_str("fn process(x: u64) -> u64 { x * 2 }", 1.0, &mut scratch);
        let v_py = m.verdict_str(PY_PROBE, 1.0, &mut scratch);
        assert!(!v_rust.declined);
        assert!(v_py.declined, "Python must exceed theta (d={})", v_py.d);
    }

    #[test]
    fn short_ambiguous_snippets_are_the_honest_gray_zone() {
        // A 9-token Python snippet whose tokens are mostly Rust-shared
        // (return, x, *, parens) accumulates sub-threshold evidence and is
        // NOT declined — absence of evidence is not decline. Documented
        // property of the NB document sum; see the module doc.
        let m = model();
        let mut scratch = Vec::new();
        let v = m.verdict_str("def process(x):\n    return x * 2\n", 1.0, &mut scratch);
        assert!(!v.declined, "short ambiguous snippet must pass (d={})", v.d);
    }

    #[test]
    fn build_is_deterministic() {
        let in_docs: &[&str] = &["fn a() {}", "fn b(x: u8) -> u8 { x }"];
        let a = ScopeModel::build(in_docs, OUT_OF_SCOPE_DOCS);
        let b = ScopeModel::build(in_docs, OUT_OF_SCOPE_DOCS);
        assert_eq!(a.table().commitment(), b.table().commitment());
    }

    #[test]
    fn empty_input_is_neutral_not_declined() {
        let m = model();
        let mut scratch = Vec::new();
        let v = m.verdict_str("", 1.0, &mut scratch);
        // Bit comparison: the empty sum is exactly +0.0 by construction.
        assert_eq!(v.d.to_bits(), 0.0f32.to_bits());
        assert!(!v.declined);
    }
}
