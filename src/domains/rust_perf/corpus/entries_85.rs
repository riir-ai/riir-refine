// Batch 371 — the codegraph kernel transcription distill
// (colbymchenry/codegraph @ cc17b69a66adeb41eab9b4bf6d2f61e8cd311b81, MIT;
// re-pinned at T0 2026-11-18 — the tip moved 62 commits past the verdict
// pin 44098dec, all quotes re-verified at the NEW pin).
//
// rust_perf 364 → 365: R1 `hex-encode-lut-two-chars-not-format-per-byte`
// (allocation, BENCHABLE — the batch's GOAT lead).
//
// Quotes byte-verified at the NEW pin cc17b69a (the ids.rs hex loop is
// byte-identical across the re-pin window; T0 diff verified). NOTHING
// INVENTED.
#![allow(dead_code)]

use crate::domains::rust_perf::PerfRule;

// ── R1: the LUT encodes two chars per byte; format! builds a String per byte ──
pub fn r1_hex_encode_lut_two_chars_not_format_per_byte() -> PerfRule {
    PerfRule {
        rule_id: "hex-encode-lut-two-chars-not-format-per-byte".to_string(),
        name: "Hex-encoding bytes through `format!(\"{b:02x}\")` inside the loop allocates a String and runs the formatting machinery PER BYTE → a const `HEX_LUT: [u8; 16]` lookup writes two ASCII bytes per byte into a pre-sized buffer — byte-identical output (std's LowerHex for u8 IS the two-nibble LUT), zero allocations in the loop".to_string(),
        category: "allocation".to_string(),
        buggy_code: r#"// NAIVE: per-byte format! — a String allocation + formatting
// machinery for every byte, in a hot id-generation path.
fn node_id(kind: &str, digest: &[u8; 16]) -> String {
    let mut hex = String::with_capacity(kind.len() + 1 + 32);
    hex.push_str(kind);
    hex.push(':');
    for b in &digest[..16] {
        hex.push_str(&format!("{b:02x}")); // alloc + fmt machinery per byte
    }
    hex
}"#.to_string(),
        fixed_code: r#"// FIXED: the const LUT — two table reads per byte into the
// pre-sized buffer. Byte-identical: a u8's two nibbles ARE the
// lowercase hex spelling.
const HEX_LUT: &[u8; 16] = b"0123456789abcdef";

fn node_id(kind: &str, digest: &[u8; 16]) -> String {
    let mut hex = String::with_capacity(kind.len() + 1 + 32);
    hex.push_str(kind);
    hex.push(':');
    let mut raw = Vec::with_capacity(32);
    for b in &digest[..16] {
        raw.push(HEX_LUT[(b >> 4) as usize]);
        raw.push(HEX_LUT[(b & 0x0f) as usize]);
    }
    // LUT output is pure ASCII by construction.
    hex.push_str(std::str::from_utf8(&raw).expect("LUT output is ASCII"));
    hex
}"#.to_string(),
        context_notes: Some("Grade A− — the batch's GOAT LEAD, BENCHABLE (byte-identity by construction + the alloc axis). SOURCE (colbymchenry/codegraph @ cc17b69a66adeb41eab9b4bf6d2f61e8cd311b81, MIT — RE-PINNED at T0 2026-11-18: the tip moved 62 commits past the Research-418 verdict pin 44098dec; the cited codegraph-kernel/src/ids.rs hex loop is BYTE-IDENTICAL across the re-pin window, diff-verified; quotes re-verified at the NEW pin). THE DEFECT, ids.rs:56-61 at the pin verbatim: `for b in &digest[..16] {` / `hex.push_str(&format!(\\\"{b:02x}\\\"));` / `}` — one String allocation + the format machinery PER BYTE, 16 per node id, in the hottest id-generation path of the product (every node id of the knowledge graph carries a 32-hex tail). THE UPSTREAM CONTEXT IS THE PARITY CONTRACT: the kernel's oddities (SHA-256 not BLAKE3, `format!` hex) are forced by byte-for-byte graph parity with the TS extractor (scripts/kernel-parity.mjs) — cite, never fix — BUT the LUT rewrite is byte-identical BY DEFINITION (a u8's LowerHex IS two nibble lookups), so the parity contract is exactly what makes the fix safe: upstream-absent fix space (open; outbound-PR-eligible if the gated lane ever opens — G-PR0..G-PR4 apply then, the defer recorded on Plan 385). G1: the fixture's exhaustive pin over all 256 byte values + the 16-byte digest shape (LUT == format! oracle, bit-exact). PRE-REGISTERED BARS (the validator cell runs at the cargo window): fixed ParetoDominant 3/0/0 at n = 1K/16K/262K ids — the ≥1.05x floor at every size, allocs 17-class → 1 per id (buggy: the format! String per byte + the head; fixed: the head + the scratch); a Tie or any Slower demotes in-landing (the ungoat law). REGIME: hot-path byte→hex (digest ids, trace ids, content hashes rendered to strings, log/serialization shims); anywhere format!-per-byte sits inside a loop over >16 items. ANTI-REGIMES: one-shot display paths (a format! call per MESSAGE, not per byte — the machinery amortizes); wide-char encodings needing case/padding decisions per byte (the LUT assumes lowercase fixed-width — extend the LUT or refuse). DISTINCT-FROM: the LUT family (minimal-perfect-hash-static-lookup, dense-subdomain-arithmetic-before-table-lookup, f16-to-f32-accumulator-lut) owns lookup STRUCTURES for classification — THIS is digest-to-ASCII encoding, the LUT is the spelling, not the mechanism; `fold-key-len-into-one-word-compare` composes (the id's tail IS the encoded digest). Fixture: b371_fixtures.rs F1 (the exhaustive byte-identity oracle pin + the counting-allocator alloc axis + the tampered-LUT loud failure). Plan 385 / Research 418.".to_string()),
        #[cfg(feature = "rust_perf_bench")]
        bench_spec: Some(hex_encode_lut_bench_spec()),
    }
}

#[cfg(feature = "rust_perf_bench")]
fn hex_encode_lut_bench_spec() -> crate::domains::rust_perf::bench::BenchSpec {
    crate::domains::rust_perf::bench::BenchSpec {
        sizes: vec![1_024, 16_384, 262_144],
        harness: r#"
pub struct Input {
    digests: Vec<[u8; 16]>,
    sink: u64,
}

fn make_input(n: usize) -> Input {
    // Deterministic digests; every byte value appears across the set.
    let mut digests = Vec::with_capacity(n);
    let mut state = 0x243f_6a88_85a3_08d3u64;
    for _ in 0..n {
        let mut d = [0u8; 16];
        for slot in d.iter_mut() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *slot = (state >> 33) as u8;
        }
        digests.push(d);
    }
    Input { digests, sink: 0 }
}

const HEX_LUT: &[u8; 16] = b"0123456789abcdef";

fn run_buggy(inp: &mut Input) -> u64 {
    let mut acc = 0u64;
    for d in &inp.digests {
        let mut hex = String::with_capacity(32);
        for b in &d[..16] {
            hex.push_str(&format!("{b:02x}"));
        }
        acc = acc.wrapping_add(hex.len() as u64).wrapping_add(hex.as_bytes()[31] as u64);
    }
    inp.sink = acc;
    acc
}

fn run_fixed(inp: &mut Input) -> u64 {
    let mut acc = 0u64;
    let mut raw = Vec::with_capacity(32);
    for d in &inp.digests {
        raw.clear();
        for b in &d[..16] {
            raw.push(HEX_LUT[(b >> 4) as usize]);
            raw.push(HEX_LUT[(b & 0x0f) as usize]);
        }
        let hex = std::str::from_utf8(&raw).unwrap();
        acc = acc.wrapping_add(hex.len() as u64).wrapping_add(hex.as_bytes()[31] as u64);
    }
    inp.sink = acc;
    acc
}

fn fold(inp: &Input) -> u64 { inp.sink }
"#
        .to_string(),
    }
}

pub fn b371_rules() -> Vec<PerfRule> {
    vec![r1_hex_encode_lut_two_chars_not_format_per_byte()]
}
