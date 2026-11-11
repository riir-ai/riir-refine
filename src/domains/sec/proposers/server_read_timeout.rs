//! `server-connection-without-read-timeout` — SUGGEST-ONLY detector
//! (Batch 351, the 0sec-labs/foxguard distill; Research 394 / Plan 361).
//! The slowloris family seed — zero prior corpus owners.
//!
//! A `TcpListener` accept loop handing a stream to a per-connection
//! handler WITHOUT a `set_read_timeout`/`set_write_timeout` call in the
//! handler's scope: a client that opens a connection and trickles (or
//! never sends) the request pins the worker connection indefinitely —
//! a handful of such connections exhausts the worker pool (slowloris,
//! CWE-400 resource exhaustion).
//!
//! ## The source anchor (0sec-labs/foxguard @ `fbeab16`)
//!
//! `src/rules/go.rs` `NetHttpNoTimeout`, verbatim at the pin:
//! "http.ListenAndServe without timeout configuration enables slowloris
//! attacks" — fix advice verbatim: "use http.Server with
//! ReadTimeout/WriteTimeout to prevent slowloris". The Rust shape
//! differs by API, not law: there is no `http.ListenAndServe` — the
//! class shape is the std `TcpListener` accept loop (our own serve
//! edges are exactly this shape), and the timeout vocabulary is
//! `set_read_timeout` / `set_write_timeout` / `read_timeout` /
//! `write_timeout`.
//!
//! ## THE ADJUDICATION IN-RULE vs `untrusted-keyed-state-unbounded`
//!
//! Both are the resource-exhaustion family's faces, ADJACENT-DISTINCT:
//! THAT rule owns peer-KEYED TABLE growth (unbounded map entries per
//! attacker identity); THIS owns per-CONNECTION WAIT (a worker pinned
//! by one stalled stream). Different resource (rows vs worker-seconds),
//! different fix (a cap + eviction policy vs a deadline), different
//! attacker cost (many identities vs many sockets).
//!
//! ## Why suggest-only (never a fix)
//!
//! The deadline VALUE is a protocol decision (an interactive request
//! edge wants seconds; a long-poll/SSE endpoint wants bounded-but-long)
//! and the refusal shape (drop vs 408) is the server's error-path
//! design. NO `FIXABLE_SEC` membership, NO `collect_edits` arm.
//!
//! ## Honest recall limits (stated in-rule, per the plan)
//!
//! - The timeout vocabulary must be in the HANDLER's scope (the accept
//!   loop fn or its callee chain is unconnected parse-only — the same
//!   fn-level law every sec rule carries); a deadline set in a helper
//!   the detector cannot see fires and review decides.
//! - Async stacks (`tokio::net::TcpListener` + `tokio::time::timeout`)
//!   carry the same class under different vocabulary — this detector
//!   scopes to the std sync shape (the source's transcription law: the
//!   Rust shape stated, the std surface); the async face is future.
//! - The deadline is a VALUE, never a kill: the class fires on the
//!   ABSENCE of a deadline, never on a chosen long value.
//! - Detector: `proposers::server_read_timeout`. Fixture:
//!   `fixtures/b351_fixtures.rs` F4. Batch 351 / Plan 361 / Research 394
//!   (0sec-labs/foxguard @ `fbeab16`, MIT OR Apache-2.0 — cited as class
//!   provenance; the fixture is an original synthesis, standalone-compiled
//!   AND executed before entry).

use std::ops::Range;

use quote::ToTokens;
use syn::spanned::Spanned;
use syn::{File, Item};

/// One detected deadline-less connection handler.
#[derive(Debug, Clone)]
pub struct ReadTimeoutFinding {
    /// Byte range of the accept-loop expression.
    pub site_span: Range<usize>,
    /// The site's source text.
    pub site_text: String,
    /// The advice line (the deadline law).
    pub advice: &'static str,
}

/// The fixed advice text (single source, tested).
pub const ADVICE: &str = "a per-connection handler with NO read/write \
deadline pins its worker on a client that never completes the request — \
a handful of stalled sockets exhausts the pool (slowloris, CWE-400). Set \
`stream.set_read_timeout(..)` / `set_write_timeout(..)` (bounded values; \
a long-poll endpoint sets a bounded-but-long deadline — the deadline is \
a value, never a kill) (Batch 351, suggest-only)";

/// The deadline vocabulary (the fix's own suppression set).
const TIMEOUT_TOKENS: &[&str] = &[
    "set_read_timeout",
    "set_write_timeout",
    "read_timeout",
    "write_timeout",
];

/// Detect deadline-less connection handlers (suggest-only; no edits are
/// produced by this module, ever).
#[must_use]
pub fn detect(source: &str, file: &File) -> Vec<ReadTimeoutFinding> {
    let mut findings = Vec::new();
    for item in &file.items {
        collect_in_item(source, item, &mut findings);
    }
    findings
}

fn collect_in_item(source: &str, item: &Item, findings: &mut Vec<ReadTimeoutFinding>) {
    match item {
        Item::Fn(f) => scan_fn(source, &f.block, findings),
        Item::Impl(imp) => {
            for ii in &imp.items {
                if let syn::ImplItem::Fn(m) = ii {
                    scan_fn(source, &m.block, findings);
                }
            }
        }
        Item::Mod(m) => {
            // Comment guard (house discipline): #[cfg(test)] mods are
            // fixture habitat, not production serve edges.
            if m.attrs.iter().any(|a| {
                a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test")
            }) {
                return;
            }
            if let Some((_, items)) = &m.content {
                for nested in items {
                    collect_in_item(source, nested, findings);
                }
            }
        }
        _ => {}
    }
}

fn scan_fn(source: &str, block: &syn::Block, findings: &mut Vec<ReadTimeoutFinding>) {
    let fn_ts = quote::ToTokens::to_token_stream(block).to_string();
    // Token streams render with spaces (`. incoming ()`) — compact before
    // the vocabulary checks.
    let compact: String = fn_ts.chars().filter(|c| !c.is_whitespace()).collect();
    // Class shape: a std TcpListener accept loop (`.accept()` or the
    // `.incoming()` iterator form in the fn).
    if !(compact.contains(".accept()") || compact.contains(".incoming()")) {
        return;
    }
    // Suppression (the fix's own vocabulary): any deadline call in the
    // handler's scope silences it.
    if TIMEOUT_TOKENS.iter().any(|t| compact.contains(t)) {
        return;
    }
    // The finding anchors on the accept-loop site (the for/if arms are
    // the handler's shape; the fn is the scope).
    let byte_range = block.span().byte_range();
    findings.push(ReadTimeoutFinding {
        site_span: byte_range.clone(),
        site_text: source.get(byte_range).unwrap_or("").to_string(),
        advice: ADVICE,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn findings(src_text: &str) -> Vec<ReadTimeoutFinding> {
        let file = syn::parse_file(src_text).expect("parses");
        detect(src_text, &file)
    }

    /// Fires on its own corpus buggy fixture (the self-fire law).
    #[test]
    fn fires_on_own_buggy_fixture() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "server-connection-without-read-timeout")
            .expect("rule present");
        assert!(
            !findings(&rule.buggy_code).is_empty(),
            "the proposer must fire on its own buggy fixture"
        );
    }

    /// Batch 351 gate — the corpus `fixed_code` MUST stay silent (the
    /// fix carries the deadline vocabulary).
    #[test]
    fn corpus_fixed_fixture_is_silent() {
        let corpus = crate::domains::sec::corpus::build_sec_seed_corpus();
        let rule = corpus
            .entries
            .iter()
            .find(|r| r.rule_id == "server-connection-without-read-timeout")
            .expect("rule present");
        assert!(
            findings(&rule.fixed_code).is_empty(),
            "the fixed fixture must stay silent"
        );
    }

    /// The accept loop without a deadline fires.
    #[test]
    fn deadlineless_accept_loop_fires() {
        let src_text = "pub fn serve(listener: TcpListener) {\n    \
             for stream in listener.incoming() {\n        \
             let Ok(stream) = stream else { continue };\n        \
             handle(stream);\n    }\n}\n";
        assert_eq!(findings(src_text).len(), 1, "the deadline-less loop fires");
    }

    /// The deadline suppression (the fix's own vocabulary).
    #[test]
    fn deadline_suppresses() {
        let src_text = "pub fn serve(listener: TcpListener) {\n    \
             for stream in listener.incoming() {\n        \
             let Ok(stream) = stream else { continue };\n        \
             let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));\n        \
             handle(stream);\n    }\n}\n";
        assert!(findings(src_text).is_empty());
    }

    /// A long-poll deadline (bounded-but-long) suppresses too — the
    /// deadline is a value, never a kill.
    #[test]
    fn long_poll_deadline_suppresses() {
        let src_text = "pub fn serve(listener: TcpListener) {\n    \
             for stream in listener.incoming() {\n        \
             let Ok(stream) = stream else { continue };\n        \
             let _ = stream.set_read_timeout(Some(Duration::from_secs(3600)));\n        \
             handle(stream);\n    }\n}\n";
        assert!(findings(src_text).is_empty());
    }

    /// No accept loop in the fn → silent (the class-shape gate).
    #[test]
    fn no_accept_loop_silent() {
        let src_text = "pub fn dial(addr: &str) -> TcpStream {\n    \
             TcpStream::connect(addr).unwrap()\n}\n";
        assert!(findings(src_text).is_empty());
    }

    /// `#[cfg(test)]` mods are skipped (house discipline).
    #[test]
    fn test_mods_skipped() {
        let src_text = "#[cfg(test)]\nmod tests {\n    \
             fn serve(listener: TcpListener) {\n        \
             for stream in listener.incoming() { handle(stream.unwrap()); }\n    }\n}\n";
        assert!(findings(src_text).is_empty());
    }
}
