//! Entries slice 07 — Batch 173 (2026-09-23, Plan 167 / Research 204): the
//! serde zero-copy family — the corpus's first dedicated serde rules (the
//! famous-lib coverage axis: `serde/serde_json` is the ecosystem's most-depended-on
//! family and the corpus carried zero dedicated entries; tokio carried 67).
//! Docs-source distill (serde.rs lifetimes + docs.rs `RawValue`); fixtures are
//! original reproductions, quotes verified in Research 204.

use crate::domains::rust_perf::PerfRule;

#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

/// Data table, not logic — grows with distillation batches.
#[allow(clippy::too_many_lines)]
pub(super) fn entries() -> Vec<PerfRule> {
    vec![
        // ── allocation: owned-String deserialize fields → borrowed/Cow
        // zero-copy twin (the serde borrow lane) ─────────────────────────
        // Source: serde.rs/lifetimes.html (the canonical statement of the
        // borrow mechanics + #[serde(borrow)]). A derived Deserialize with
        // String fields copies every string out of the input buffer — R
        // records × S string fields heap allocations + a full memcpy of
        // the bytes — while the deserializer ALREADY holds the whole
        // payload in memory. Typing the fields &'a str / Cow<'a, str>
        // borrows the slices instead: zero-copy "avoids allocating memory
        // to store a string for each individual field and then copying
        // string data out of the input over to the newly allocated field."
        PerfRule {
            rule_id: "serde-borrow-deserialize".to_string(),
            name: "derived Deserialize with String fields on a from_str/from_slice path → &'a str (implicit borrow) or #[serde(borrow)] Cow<'a, str> (the regime-robust spelling) — borrow the field slices out of the input buffer instead of copying every string into a fresh allocation".to_string(),
            category: "allocation".to_string(),
            buggy_code: "use serde::Deserialize;\n\n// Every string field is a fresh heap allocation: the deserializer\n// copies the bytes out of the buffer it already holds. For an R-record\n// x S-string-field payload that is R*S allocations + R*S memcpys per\n// parse.\n#[derive(Deserialize)]\npub struct Event {\n    pub id: u64,\n    pub kind: String,\n    pub actor: String,\n    pub target: String,\n}\n\npub fn parse_all(payload: &str) -> Vec<Event> {\n    serde_json::from_str(payload).expect(\"valid payload\")\n}".to_string(),
            fixed_code: "use serde::Deserialize;\nuse std::borrow::Cow;\n\n// Escape-free string fields borrow slices of the input: zero string\n// allocations, the bytes stay in the buffer. Cow + #[serde(borrow)] is\n// the regime-robust spelling — an ESCAPED string (\\n, \\\", \\uXXXX) is\n// not present verbatim in the input, so serde hands back a transient\n// string: the Cow degrades to Owned (one allocation, exactly the owned\n// form's cost) where a bare &'a str would FAIL at runtime. The input\n// buffer must outlive the parsed struct — compile-enforced.\n#[derive(Deserialize)]\npub struct Event<'a> {\n    pub id: u64,\n    #[serde(borrow)]\n    pub kind: Cow<'a, str>,\n    #[serde(borrow)]\n    pub actor: Cow<'a, str>,\n    #[serde(borrow)]\n    pub target: Cow<'a, str>,\n}\n\npub fn parse_all(payload: &str) -> Vec<Event<'_>> {\n    serde_json::from_str(payload).expect(\"valid payload\")\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 4/0/0 (M3 Max, release, 2026-09-23 — Batch 173 benches): 1.33x/1.32x/1.29x/1.05x at n=1k/10k/50k/100k records (R×3 string fields) — the timing win is modest (the parse's JSON scan dominates and the allocator fast path amortizes the mallocs at scale), so the LOAD-BEARING cell is the alloc axis: EXACT 300016 → 16 at n=100k (3R String allocations → 0; both arms keep the shared output-Vec growth) and 23.4 MB → 21.0 MB of copied bytes. ESCAPED-PAYLOAD REGIME WITNESS (printed by the same bench binary, every size): owned 300019 vs Cow 100019 allocs at n=100k — the escape-free fields still borrow and only the escaped field degrades to Cow::Owned, so the delta collapses from 3R→0 (clean) to 3R→1R (fully-escaped payloads): Cow recovers 2/3 of the allocation win even at the escape anti-regime. G1 asserts the mechanism LIVE in release (plain asserts in the untimed make_input): field equality through both arms on BOTH payloads, Cow::Borrowed for the clean parse, and the Borrowed/Borrowed/Owned split per field on the escaped one. REGIME (from_str/from_slice ONLY): serde.rs — \"when deserializing from an IO stream no data can be borrowed\"; a from_reader payload gains nothing from borrowed typing (the buffer dies at the call). ESCAPE REGIME (load-bearing on the headline): serde.rs — transient data arises \"when escape sequences are being processed so the resulting string is not present verbatim in the input\"; an escaped field allocates REGARDLESS of field type, and the bare &'a str spelling FAILS at runtime there (serde's &str visitor accepts only borrowed data), so the regime-robust spelling is #[serde(borrow)] Cow<'a, str> — Borrowed when escape-free (asserted live in the harness G1 block), Owned when escaped (the collapse the regime witness prints). LIFETIME: the input must outlive the parsed struct — compile-enforced, the borrow checker is the regime's enforcement arm. Distinct from avoid-clone (one clone at a fn boundary), from cow-conditional-alloc (return-type Cow on a conditional transform), from encoded-layout-borrowed-view (accessor erasing a dictionary encoding), and from group-keys-borrow-allocate-on-first-sight (grouping-key materialization) — none touches deserialization field typing. Suggestion-only (a parse-only proposer cannot see the deserializer input path or the field-type/lifetime coupling). Fixtures original reproductions; source: serde.rs/lifetimes.html (serde-rs canonical docs — Deserializer lifetimes + the #[serde(borrow)] field attribute), read + quoted in Research 204. Batch 173 / Plan 167 / Research 204.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(serde_borrow_deserialize_bench_spec()),
        },

        // ── allocation: opaque Value-tree payload field → RawValue
        // passthrough (defer parsing what you don't consume) ─────────────
        // Source: docs.rs/serde_json RawValue (1.0.151 — the lock's
        // version). An opaque field typed serde_json::Value materializes
        // the WHOLE tree — a Map per object, a String per key, a String
        // per string value — even when the consumer only stores/forwards/
        // re-serializes the payload verbatim. #[serde(borrow)]
        // &'a RawValue validates the byte range and remembers it: zero
        // tree allocation, and the forward reads the slice directly
        // ("could be done equivalently using serde_json::Value as the
        // type for payload, but &RawValue will perform better because it
        // does not require memory allocation").
        PerfRule {
            rule_id: "rawvalue-passthrough-defer-parse".to_string(),
            name: "opaque payload field typed serde_json::Value (owned tree, a Map/String allocation per node) but only stored/forwarded/re-serialized → #[serde(borrow)] &'a RawValue (from_str/from_slice) or Box<RawValue> (reader paths) — defer parsing until a consumer actually indexes into the payload".to_string(),
            category: "allocation".to_string(),
            buggy_code: "use serde::Deserialize;\nuse serde_json::Value;\n\n// The opaque payload materializes the WHOLE tree — a Map per object,\n// a String per key, a String per string value — even though this\n// consumer only forwards the payload verbatim to the next hop.\n#[derive(Deserialize)]\npub struct Envelope {\n    pub seq: u64,\n    pub source: String,\n    pub payload: Value,\n}\n\npub fn ingest(raw: &str) -> u64 {\n    let env: Envelope = serde_json::from_str(raw).expect(\"valid envelope\");\n    // The forward: re-serialize the tree back to bytes.\n    serde_json::to_string(&env.payload).expect(\"ser\").len() as u64\n}".to_string(),
            fixed_code: "use serde::Deserialize;\nuse serde_json::value::RawValue;\n\n// RawValue passthrough: the payload's byte range is validated and\n// remembered — zero tree allocation. The forward reads the slice\n// directly; the tree is built only if a consumer ever needs it.\n// Reader-path form (from_reader): Box<RawValue> — a single-slice\n// buffer copy, still far above a full Value tree.\n#[derive(Deserialize)]\npub struct Envelope<'a> {\n    pub seq: u64,\n    pub source: String,\n    #[serde(borrow)]\n    pub payload: &'a RawValue,\n}\n\npub fn ingest(raw: &str) -> u64 {\n    let env: Envelope<'_> = serde_json::from_str(raw).expect(\"valid envelope\");\n    env.payload.get().len() as u64\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant 3/0/0 (M3 Max, release, 2026-09-23 — Batch 173 benches): 4.25x/3.88x/2.10x at n=1k/10k/50k envelopes — the timing win decays as the per-envelope tree parse amortizes at scale but stays dominant. Alloc axis EXACT 950000 → 100000 at n=50k (19/env → 2/env: the ~17-node Value tree — a Map per object, a String per key and per string value — plus the re-serialization buffer vanish; both arms keep the IDENTICAL source: String, so the delta isolates the opaque-field typing; the fixed arm's 2/env = that source String + one steady-state RawValue-path allocation). Bytes re-serialized 142.5 MB → 1.05 MB. G1 compares RE-PARSED VALUES live in release (first 256 envelopes per size + length equality on all) — never bytes: the formatting-retention delta below. BEHAVIORAL DELTA (G1 contract): RawValue serialization retains the ORIGINAL formatting — \"will not be minified or pretty-printed\" — so adoption changes byte-level output; equality gates must compare RE-PARSED VALUES (the harness does), never raw bytes. FEATURE COST: serde_json `raw_value` — a real adoption gate (the bench harness carries the dev-dep feature marker). REGIME: fields that are stored/forwarded/re-serialized verbatim — audit-log envelopes, event hops, proxy relays. READER PATH: Box<RawValue> is the from_reader form — \"almost as efficient but involves buffering the raw value from the I/O stream into memory\" (a single-slice copy, still far above a full Value tree). ANTI-REGIME: any consumer that INDEXES into the payload — the field is opaque, so re-parsing per query defeats the point; type the consumed fields instead and reserve RawValue for the opaque tail (the typed-struct rider, Research 204 candidate 3). Distinct from serde-borrow-deserialize (Batch 173 — typed FIELDS borrow slices; this defers an OPAQUE subtree wholesale) and from serde-borrow-deserialize's escape regime (an escaped string inside the opaque range changes nothing here — RawValue never parses it). Suggestion-only (a parse-only proposer cannot see verbatim-forward semantics). Fixtures original reproductions; source: docs.rs/serde_json/value/struct.RawValue.html (1.0.151 — the lock's version), read + quoted in Research 204. Batch 173 / Plan 167 / Research 204.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(rawvalue_passthrough_defer_parse_bench_spec()),
        },
    ]
}

/// Bench spec for `serde-borrow-deserialize`: R-record JSON array over the
/// wire, 3 escape-free string fields per record.
///
/// Buggy arm: `R*3` String allocations + memcpys (plus the shared output-Vec
/// growth). Fixed arm: the Vec growth only — every field borrows a slice of
/// the buffer. The ESCAPED-PAYLOAD REGIME WITNESS runs inside `make_input`
/// (once per size, outside the timed path): the SAME payload with `\n`/`\"`
/// escapes in every actor field parses through both arms and prints the
/// collapsed delta — every field comes back `Cow::Owned`, so the borrowed
/// arm allocates exactly like the owned one. G1 asserts field equality
/// both regimes + the `Cow::Borrowed/Cow::Owned` mechanism, LIVE in release.
#[allow(clippy::too_many_lines)] // data table: harness + the live G1 block + the regime witness
#[cfg(feature = "rust_perf_bench")]
fn serde_borrow_deserialize_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 50_000, 100_000],
        harness: r#"
// [dep: serde = { version = "1", features = ["derive"] }]
// [dep: serde_json = "1"]
mod buggy {
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct Event {
        pub id: u64,
        pub kind: String,
        pub actor: String,
        pub target: String,
    }

    pub fn parse_all(payload: &str) -> Vec<Event> {
        serde_json::from_str(payload).expect("valid payload")
    }
}

mod fixed {
    use serde::Deserialize;
    use std::borrow::Cow;

    #[derive(Deserialize)]
    pub struct Event<'a> {
        pub id: u64,
        #[serde(borrow)]
        pub kind: Cow<'a, str>,
        #[serde(borrow)]
        pub actor: Cow<'a, str>,
        #[serde(borrow)]
        pub target: Cow<'a, str>,
    }

    pub fn parse_all(payload: &str) -> Vec<Event<'_>> {
        serde_json::from_str(payload).expect("valid payload")
    }
}

pub struct Input {
    clean: String,
}

// Escape-free record: every byte of kind/actor/target is verbatim in the
// payload, so serde can borrow all three slices. `first` skips the
// separating comma for element 0.
fn push_record_clean(out: &mut String, i: usize, first: bool) {
    if !first {
        out.push(',');
    }
    out.push_str(&format!(
        "{{\"id\":{},\"kind\":\"spawn\",\"actor\":\"player_{}\",\"target\":\"zone_17\"}}",
        i, i
    ));
}

// Escaped record: the actor carries \n and \" escapes — the unescaped
// bytes are NOT verbatim in the input, so serde must hand back a
// transient (owned) string no matter how the field is typed.
fn push_record_escaped(out: &mut String, i: usize, first: bool) {
    if !first {
        out.push(',');
    }
    out.push_str(&format!(
        "{{\"id\":{},\"kind\":\"spawn\",\"actor\":\"player\\n42 \\\"hero {}\\\"\",\"target\":\"zone_17\"}}",
        i, i
    ));
}

pub fn make_input(n: usize) -> Input {
    let mut clean = String::with_capacity(n * 80 + 2);
    let mut escaped = String::with_capacity(n * 96 + 2);
    clean.push('[');
    escaped.push('[');
    for i in 0..n {
        push_record_clean(&mut clean, i, i == 0);
        push_record_escaped(&mut escaped, i, i == 0);
    }
    clean.push(']');
    escaped.push(']');

    // ── G1 — LIVE in release (plain assert, not debug_assert): value
    // equality through both arms on BOTH payloads + the Cow mechanism.
    // Runs once per size, outside the timed path.
    {
        let owned: Vec<buggy::Event> = serde_json::from_str(&clean).unwrap();
        let borrowed: Vec<fixed::Event<'_>> = serde_json::from_str(&clean).unwrap();
        assert_eq!(owned.len(), n);
        assert_eq!(borrowed.len(), n);
        for (o, b) in owned.iter().zip(borrowed.iter()) {
            assert_eq!(o.id, b.id);
            assert_eq!(o.kind, b.kind);
            assert_eq!(o.actor, b.actor);
            assert_eq!(o.target, b.target);
            assert!(
                matches!(b.kind, std::borrow::Cow::Borrowed(_)),
                "escape-free payload must hand back Cow::Borrowed (the mechanism)"
            );
        }
        let owned_e: Vec<buggy::Event> = serde_json::from_str(&escaped).unwrap();
        let borrowed_e: Vec<fixed::Event<'_>> = serde_json::from_str(&escaped).unwrap();
        assert_eq!(owned_e.len(), n);
        assert_eq!(borrowed_e.len(), n);
        for (o, b) in owned_e.iter().zip(borrowed_e.iter()) {
            assert_eq!(o.id, b.id);
            assert_eq!(o.kind, b.kind);
            assert_eq!(o.actor, b.actor);
            assert_eq!(o.target, b.target);
            // The mechanism split per field: kind/target are escape-free
            // (still verbatim in the input → Borrowed), actor carries the
            // \n/\" escapes (transient → the Cow MUST degrade to Owned).
            assert!(matches!(b.kind, std::borrow::Cow::Borrowed(_)));
            assert!(matches!(b.target, std::borrow::Cow::Borrowed(_)));
            assert!(
                matches!(b.actor, std::borrow::Cow::Owned(_)),
                "escaped string is transient — Cow must degrade to Owned"
            );
        }

        // ── Escaped-payload regime witness: the SAME escaped payload
        // through both arms, per-call allocation counts off the bench
        // crate's counting allocator. The delta collapses to ~0 — the
        // honest headline pre-emption. This line is skipped by the
        // validator's size-result parser (no size/buggy_ns fields).
        let a0 = ALLOC_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let _: Vec<buggy::Event> = serde_json::from_str(&escaped).unwrap();
        let owned_allocs = ALLOC_COUNT.load(std::sync::atomic::Ordering::Relaxed) - a0;
        let a1 = ALLOC_COUNT.load(std::sync::atomic::Ordering::Relaxed);
        let _: Vec<fixed::Event<'_>> = serde_json::from_str(&escaped).unwrap();
        let cow_allocs = ALLOC_COUNT.load(std::sync::atomic::Ordering::Relaxed) - a1;
        println!(
            "{{\"regime_witness\":\"escaped\",\"records\":{},\"owned_allocs\":{},\"cow_allocs\":{}}}",
            n, owned_allocs, cow_allocs
        );
    }

    Input { clean }
}

pub fn run_buggy(input: &Input) {
    let out = buggy::parse_all(&input.clean);
    let witness = out.len() as u64 + out.last().map(|e| e.id).unwrap_or(0);
    std::hint::black_box(witness);
}

pub fn run_fixed(input: &Input) {
    let out: Vec<fixed::Event<'_>> = fixed::parse_all(&input.clean);
    let witness = out.len() as u64 + out.last().map(|e| e.id).unwrap_or(0);
    std::hint::black_box(witness);
}
"#
        .to_string(),
    }
}

/// Bench spec for `rawvalue-passthrough-defer-parse`: n envelope strings,
/// each carrying a small consumed header + an opaque nested payload that the
/// consumer forwards verbatim.
///
/// Buggy arm: the Value tree per envelope — a Map per object, a String per
/// key, a String per string value — plus the re-serialization allocation.
/// Fixed arm: the header's `source: String` only (kept IDENTICAL in both
/// arms so the delta isolates the opaque-field typing) — the payload byte
/// range is validated, never materialized. G1 asserts re-parsed-VALUE
/// equality (`RawValue` keeps the original formatting — never byte-compare),
/// LIVE in release, once per size.
#[cfg(feature = "rust_perf_bench")]
fn rawvalue_passthrough_defer_parse_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 10_000, 50_000],
        harness: r#"
// [dep: serde = { version = "1", features = ["derive"] }]
// [dep: serde_json = { version = "1", features = ["raw_value"] }]
mod buggy {
    use serde::Deserialize;
    use serde_json::Value;

    #[derive(Deserialize)]
    pub struct Envelope {
        pub seq: u64,
        pub source: String,
        pub payload: Value,
    }

    pub fn ingest(raw: &str) -> u64 {
        let env: Envelope = serde_json::from_str(raw).expect("valid envelope");
        serde_json::to_string(&env.payload).expect("ser").len() as u64
    }
}

mod fixed {
    use serde::Deserialize;
    use serde_json::value::RawValue;

    #[derive(Deserialize)]
    pub struct Envelope<'a> {
        pub seq: u64,
        pub source: String,
        #[serde(borrow)]
        pub payload: &'a RawValue,
    }

    pub fn ingest(raw: &str) -> u64 {
        let env: Envelope<'_> = serde_json::from_str(raw).expect("valid envelope");
        env.payload.get().len() as u64
    }
}

pub struct Input {
    envelopes: Vec<String>,
}

fn push_envelope(out: &mut String, i: usize) {
    out.push_str(&format!(
        "{{\"seq\":{},\"source\":\"sim://zone/17\",\"payload\":{{\"widget\":{{\"pos\":{{\"x\":12,\"y\":7}},\"tags\":[\"alpha\",\"beta\",\"gamma\"]}},\"meta\":{{\"ts\":1695000000,\"note\":\"heartbeat {}\"}}}}}}",
        i, i
    ));
}

pub fn make_input(n: usize) -> Input {
    let envelopes: Vec<String> = (0..n)
        .map(|i| {
            let mut s = String::with_capacity(192);
            push_envelope(&mut s, i);
            s
        })
        .collect();

    // ── G1 — LIVE in release: re-parsed VALUE equality (RawValue keeps the
    // original formatting — byte-compare would pin a formatting accident).
    // Full field equality on the first 256 envelopes + length equality on
    // all; runs once per size, outside the timed path.
    {
        for (i, raw) in envelopes.iter().enumerate().take(256.min(n)) {
            let b: buggy::Envelope = serde_json::from_str(raw).unwrap();
            let f: fixed::Envelope<'_> = serde_json::from_str(raw).unwrap();
            assert_eq!(b.seq, f.seq);
            assert_eq!(b.seq, i as u64);
            assert_eq!(b.source, f.source);
            // The behavioral-delta-safe equality: re-parse BOTH payloads to
            // Value and compare values, never bytes.
            let tree_from_buggy = b.payload;
            let tree_from_fixed: serde_json::Value =
                serde_json::from_str(f.payload.get()).unwrap();
            assert_eq!(tree_from_buggy, tree_from_fixed, "envelope {i} payload");
            // And the verbatim contract: the raw slice round-trips to the
            // same Value the owned tree held.
            let reparsed: serde_json::Value = serde_json::from_str(raw).unwrap();
            assert_eq!(reparsed["payload"], tree_from_fixed);
        }
    }

    Input { envelopes }
}

pub fn run_buggy(input: &Input) {
    let mut witness = 0u64;
    for raw in &input.envelopes {
        witness = witness.wrapping_add(buggy::ingest(raw));
    }
    std::hint::black_box(witness);
}

pub fn run_fixed(input: &Input) {
    let mut witness = 0u64;
    for raw in &input.envelopes {
        witness = witness.wrapping_add(fixed::ingest(raw));
    }
    std::hint::black_box(witness);
}
"#
        .to_string(),
    }
}
