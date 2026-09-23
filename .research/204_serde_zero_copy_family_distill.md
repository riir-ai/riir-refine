# Research 204 — serde zero-copy family distill (borrow-deserialize + RawValue passthrough)

**Status:** RECORD — Batch 173 landed 2026-09-23 (2 rules: serde-borrow-deserialize + rawvalue-passthrough-defer-parse; candidate 3 landed as candidate 2's context-notes rider per T6's adjudication; measured verdicts + the escape-regime witness in the batch's HISTORY entry) · quotes pre-verified 2026-09-23 ~03:0x (Plan 167 T2 early, all verbatim vs serde_json 1.0.151 = the lock's version); dedup re-run T3 early — PASS, zero covering rules, corpus still at Batch-172 state

> **ADDENDUM 2026-09-23 ~03:0x +07 (Plan 167 T2+T3 executed EARLY — read-only pre-verification, the Plan-166 T1+T2 precedent; the batch itself stays cadence-held at the past-midnight slot ~09-24):** T2 re-fetched both pages — ALL quotes verbatim, zero drift, and a provenance STRENGTHENING surfaced: docs.rs now serves serde_json **1.0.151** and our Cargo.lock pins **1.0.151**, so every RawValue quote is anchored to the exact version this repo builds against (not just "versionless canonical"). serde.rs lifetimes: the three regime quotes + the `Deserialize<'static>` DROP premise + the implicit-vs-explicit borrow mechanics all match; the page's own `#[serde(borrow)] comment: Cow<'b, str>` example is the planned candidate-1 fixture shape. RawValue: the five quoted sentences match; the docs example `#[serde(borrow)] payload: &'a RawValue` is candidate 2's fixture shape verbatim; from_string's two no-realloc conditions (no surrounding whitespace, capacity == len) and from_string_unchecked's debug-asserted re-parse contract confirmed at 1.0.151. T3 fresh dedup greps: zero covering rules on both vocabularies (id-level incidental hits only; `RawValue|visit_borrowed` → 0 corpus hits; content `deserial`/`serde_json` → the two known unrelated files), and the tests.rs `annotated == 94` pin reads the corpus at exactly the Batch-172 state — no batch has landed since the 02:5x first-pass dedup. Bonus read-only confirmation for T5: serde_json is a plain main dep (Cargo.toml L26, no features) — declaring `raw_value` under dev-deps unifies features only when dev targets build, so the shipped release binary path never compiles the feature in. Batch 173 remains QUEUED at the cadence gate (first idle pass past midnight +07 → 09-24) or owner GO; T4 authoring is now unblocked to a verified source baseline.

## Verdict line

**YES B− corpus / no lane mapping / seeds the serde family** (product-coverage YES, the dial9 shape). Queue slot: **Batch 173** — plan-first per the 2026-08-18 owner call: [`.plans/167_batch173_serde_family_distill.md`](../.plans/167_batch173_serde_family_distill.md). This verdict closes the "Batch 173 has no queued source" gap recorded by the 09-23 00:4x/02:4x passes.

## Source

- serde official documentation — [Deserializer lifetimes](https://serde.rs/lifetimes.html) and the `#[serde(borrow)]` field attribute section on the same page. Fetched 2026-09-23 ~02:5x +07.
- serde_json API documentation — [`serde_json::value::RawValue`](https://docs.rs/serde_json/latest/serde_json/value/struct.RawValue.html) (crate feature `raw_value`). Fetched 2026-09-23 ~02:5x +07.

Provenance: serde-rs canonical docs (dtolnay et al.) — the authoritative statement of the borrow mechanics; the derive macro's behavior is defined by these pages. **Docs-source, not a code tree**: no `.raw/` clone, no sha pin (nothing cloned; nothing to delete). Quotes below are verbatim from the fetched pages and are re-verified at mine time (Plan 167 T2).

## Domain routing

`rust_perf`, category `allocation` (serde-borrow rule) / `allocation` or `io` (RawValue rule — batch decides exact strings). The tempting WRONG domain does not fire here: both patterns are pure CPU-side allocation mechanics — nothing kernel-shaped (the CPU-SIMD-is-not-kernel_opt trap is not in play).

## The product-coverage axis — why this verdict exists

Corpus family census (2026-09-23, mention counts over `src/domains/rust_perf/corpus/`): tokio 67 · rayon 45 · memchr 27 · regex 24 · **serde 7** · hashbrown 1 · itoa/ryu/ahash 0. The 7 serde mentions are all incidental — `avoid-clone`/`cow-conditional-alloc` fixtures and an unrelated build-manifest example inside an io rule (`entries_05.rs` L507). **Zero dedicated serde rules** in a corpus that carries 67 tokio entries — while serde/serde_json is the most-depended-on family in the ecosystem (the corpus carries bevy/wasm/rayon families for exactly this reason). This is the dial9 tokio gap one lib over: a famous-lib family at seeding size.

**Follow-up source pool** (accumulation clause — pool sources read before the batch window fold into the same batch, the Batch-145/146 precedent):

1. This verdict's two pages (canonical borrow + RawValue mechanics).
2. serde.rs field-attribute docs (the `borrow` entry) — same site, deeper attr detail.
3. Manishearth's zero-copy deserialization series ("Not a Yoking Matter", "Zero-Copy All the Things!") — conceptual backing, regime vocabulary.
4. d3lm, "Rust: Beware of Escape Sequences" — regime evidence for candidate 1 (escaped strings defeat borrowing; see the annotation below).
5. simd-json — alternative-impl lane only (already mined Batch 74 SWAR); a crate-swap, not a bounded pattern.

## Ranked candidates

| # | Pattern | Grade | Source | Notes |
|---|---|---|---|---|
| 1 | `serde-borrow-deserialize` — derived `Deserialize` with `String` fields on a `from_str`/`from_slice` path → `&'a str`/`&'a [u8]` (implicit) or `#[serde(borrow)]` `Cow<'a, str>` (explicit) | **A− (GOAT lead)** | serde.rs lifetimes | R·S heap allocs + full-payload string memcpy → 0 |
| 2 | `rawvalue-passthrough-defer-parse` — opaque field typed `serde_json::Value` (owned tree, every node allocated) but only stored/forwarded/re-serialized → `#[serde(borrow)] &'a RawValue` (or `Box<RawValue>` on reader paths) | **B** | RawValue docs | Tree parse O(n) allocs → byte-range validation 0 allocs |
| 3 | `deserialize-typed-not-value` — deserialize into `Value` then extract a few typed fields → deserialize directly into the typed struct | **B−** | composition of the above | Likely RIDER on candidate 2 (typed struct + RawValue for the opaque rest composes); adjudicate at mine time |
| — | DROP: `Deserialize<'static>` guidance ("never what you want") | KILL | serde.rs | API-shape/ambient structure, not perf decision content |
| — | DROP: escape-sequences as its own rule | KILL | serde.rs | Regime annotation on candidate 1, not a bounded fix-space of its own |

## Candidate 1 — the GOAT lead, all four criteria

1. **Benchable** — `serde_json` is ALREADY a main dep (Cargo.toml L26, zero new tree cost) and `serde` carries `derive`. Pareto validator + the exact alloc-count axis: owned arm R records × S string fields = R·S `String` allocations; borrowed arm = 0. No fixture plumbing beyond a static payload string.
2. **Exact semantics** — parsed values identical; G1 = string-equality over every field, trivially bit-exact. Integer/float fields unaffected.
3. **Magnitude story** — mechanism-level, not "probably faster": every borrowed string field skips one heap allocation AND the memcpy of its bytes out of the input. serde.rs: zero-copy "avoids allocating memory to store a string for each individual field and then copying string data out of the input over to the newly allocated field."
4. **Deterministic** — no timing, no environment dependence.

**Regime conditions (the rule's other half, all quotable from the source):**

- **Input path**: borrowing works through `serde_json::from_str` / `from_slice`. From an IO stream it is impossible — serde.rs: "when deserializing from an IO stream no data can be borrowed". A `from_reader` payload gains nothing from `&'a str` typing.
- **Escape sequences**: serde.rs — transient data arises "when escape sequences are being processed so the resulting string is not present verbatim in the input." An escaped string (`\n`, `\"`, `\uXXXX`) is handed over transient and allocated REGARDLESS of the field type — the win exists only for escape-free payloads. The bench MUST carry an escaped-payload arm as the regime witness (the owned-vs-borrowed delta collapses there; saying so pre-empts a misleading headline number).
- **Lifetime**: the input buffer must outlive the deserialized struct — compile-enforced, the borrow checker is the regime's enforcement arm.

**Dedup evidence (both vocabularies, zero corpus hits):** id-level grep `serde|borrow|zero|cow|deserial` over rule ids → only unrelated hits (`vec-macro-zero-fill`, `avoid-clone`, `cow-conditional-alloc`, `encoded-layout-borrowed-view`, `group-keys-borrow-allocate-on-first-sight`); content-level `deserial` → `entries_05.rs` L507 (a resume-manifest fixture inside an unrelated io rule) + `tests.rs`; `RawValue|visit_borrowed|serde_json` → `from_slice`/`copy_from_slice` false positives only. Distinct-from the four nearest rules: `avoid-clone` (one clone at a fn boundary), `cow-conditional-alloc` (return-type Cow on transform), `encoded-layout-borrowed-view` (accessor erasing a dictionary encoding), `group-keys-borrow-allocate-on-first-sight` (grouping-key materialization) — none touches deserialization field typing.

## Candidate 2 — RawValue passthrough

RawValue docs, verbatim: "A `RawValue` can be used to defer parsing parts of a payload until later, or to avoid parsing it at all in the case that part of the payload just needs to be transferred verbatim into a different output object." And the decisive comparison: "This could be done equivalently using serde_json::Value as the type for payload, but &RawValue will perform better because it does not require memory allocation. The correct range of bytes is borrowed from the input data and pasted verbatim into the output."

Regimes and annotations:

- Borrowed form (`#[serde(borrow)]` + `&'a RawValue`) for `from_str`/`from_slice`; boxed form (`Box<RawValue>`) for `from_reader` — "almost as efficient but involves buffering the raw value from the I/O stream into memory" (a single-slice copy, still >> a full `Value` tree).
- **Feature gate**: serde_json `raw_value` — a real adoption cost; for the bench arm this rides the DEV-dependency feature declaration only (serde_json is already a dep; feature unification note goes in the plan).
- **Behavioral delta**: serializing a `RawValue` retains its original formatting — "will not be minified or pretty-printed". Byte-level output changes when the fix is adopted; G1 must compare re-parsed VALUES (or document the formatting delta), not raw bytes.
- Enrichment notes for `from_string`/`from_string_unchecked` (no-realloc conditions: no surrounding whitespace + capacity == len; the unchecked variant skips re-parse under a documented safety contract, debug-asserted): rider material at mine time.

GOAT: passes all four with a narrower fix surface than candidate 1 (feature-gated; the field is opaque afterwards) — B, not A−.

## Lane-intel axis

Checked before recording: no mapping. serde field typing is not a mechanism in any open perf lane (league scoreboard rows are prefill/decode/GPU-lane questions; workspace issue grep for serde zero-copy yields nothing). Nothing owed in riir-ai — the lane-intel artifacts are explicitly none, mirroring Plan 166's defers line.

## Yield estimate

2–3 rules after ~50% coordinator falsification (candidates 1+2 landing; 3 folds into 2 as a rider or lands as a small third entry). Matches the recent-batch norm (172: 4+2+1; 171: pack of 2; the corpus's batch median).

## Honest caveats

- **Docs-source grading**: the patterns are canonical and the mechanics quotable, but the GRADER saw documentation, not consumer code density. The corpus law (fixtures are ORIGINAL minimal reproductions) applies doubly — never paste the docs examples verbatim into `buggy_code`/`fixed_code`.
- **The escape-sequence regime is load-bearing on the headline number**: benches over escape-free payloads measure the full win; real-world payloads with escaped strings see a proportional collapse. Both arms ship, both numbers print.
- **`from_reader` anti-regime**: the largest-serving-path input mode (streams) is the one where candidate 1 gains nothing and candidate 2 degrades to the boxed form. The rules' context_notes must carry this so a consumer on a reader path is not misadvised.
- **yaml/toml/other formats**: the mechanics are serde-generic but the borrowed-data support is deserializer-specific (`serde_yaml`/`toml` differ). Rules are scoped to serde_json unless the batch finds generic phrasing that stays true.
- Docs pages are versionless canonical (no drift risk like a fast-moving repo), but Plan 167 T2 still re-verifies quotes at mine time per the standing discipline.

## Bottom line

Mine as **Batch 173** (next batch), lead with `serde-borrow-deserialize` (A−, all four GOAT criteria pass, alloc axis is its native validator), `rawvalue-passthrough-defer-parse` second, candidate 3 adjudicated at mine time. Plan 167 is open; batch executes at the standing cadence gate or owner GO.
