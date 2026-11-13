# Plan 371 — Batch 360: polars revisit #2 delta (rust_perf)

> Status: CLOSED — BATCH 360 LANDED 2026-11-13 (zed-amo-b360-polars; all tasks [x]; the landing commit is the arbiter). Executes Research 405 (`.research/405_polars_2x_streaming_delta_distill.md`). Verdict YES B− corpus-delta / lane-intel NONE / product-coverage YES. Source `pola-rs/polars` re-pinned at the batch pin `437948302e11e15d47fc8b2882f9edb0848e5ebc` (2026-11-13T15:59:46Z; MIT). The cargo half awaits a free window; the claim row lands when the lane starts, the push is the arbiter.
**Lease closed:** zed-amo-b360-polars, 1794587562, was expires 2026-11-15T16:32:42Z — Batch 360 LANDED under it the same session (the landing commit is the arbiter)

## Scope

- [x] T0 — DONE 2026-11-13 (zed-amo-b360-polars): re-cloned at master tip `437948302e11e15d47fc8b2882f9edb0848e5ebc` (2026-11-13T15:59:46Z — the tip moved ~2.5h past `61fdb30f`); ALL NINE quote sites verified byte-exact at the new pin (matched_rows.rs MARKED L4-5 + the mark_rows transition L36-40; single_key.rs nulls_marked L29 + the block-probe comment L94; string.rs string_len_chars L53-54 + INLINE_CONTINUATION_BITS + the tail shift; arity.rs prim_unary_values L46-47; iterator.rs the SHRD comment pair L431-432 + the combine body L433-435; into_groups.rs create_groups_from_sorted L67-69; zstd.rs MAX_CACHED_CONTEXT_BYTES L16 + CCTX/DCTX L18-20; hash_keys.rs BLOCK_SIZE/MIN_PREFETCH_BUCKETS L29-32; sink.rs the two hot-table grow comments L42/L46 + hot_groups/ all six files). Upstream-state rows: all PRESENT, unchanged. New pin of record: `437948302`.
- [x] T1 — DONE 2026-11-13: dedup re-verification at landing HEAD (349 rule names extracted + vocab greps) — ZERO owners across all 8 candidate mechanisms; the four shared-mechanism adjudications stand as planned (R1↔bitflag-pack footprint axis + B334; R2↔B244 (→ T2 face); R7↔software-prefetch-storage-bound (named positive regime, unowned); R8↔B334 N=1 streak vs N-slot skew; R4↔B69 byte-assembly vs word-shift, adjacent-distinct; funnel/shrd vocab = 0 hits).
- [x] T2 — DECIDED: R2 = FACE, not a rule (see the staged-authoring block below for the face text contract).
- [x] T3 — LANDED: the two rules' bodies + quote banks + the face text drafted IN THIS PLAN (the section below); the G1 identities preflight-verified 3/3 standalone (`.plans/b360_g1_preflight.rs`, rustc --test: funnel exhaustive s 1..=63 exact + s=0-returns-lo + s=64 periodic wrap; flag-guard aux bit-identical under a 200-hit dup-heavy stream; char-count identity on mixed/multibyte/emoji UTF-8). ⚠ LAW NOTE the preflight surfaced: the funnel's `% 64` makes it PERIODIC (s=64 wraps to s=0's answer) — carry in the rule text as the pinned domain shape. The corpus entries + b360_fixtures.rs finalize at the T4 quiet window (context_notes need the MEASURED verdicts — never invented).

### T3 staged authoring (this lane, 2026-11-13 — drafts only; context_notes finalize AFTER T4's measured verdicts, NOTHING INVENTED)

**T2 DECIDED — R2 is a FACE, not a rule** (the B66 float-sum precedent): the count-per-block popcount over a byte view IS B244 `iter-count-bulk-specialization`'s law; the new content rides as a count-neutral positive-instance FACE on entries_30 carrying (a) the char-count IDENTITY (`view.length − continuation_count = char_count` for valid UTF-8), (b) the byte-CLASS mask (`b & 0xC0 == 0x80`) generalizing B244's single-byte equality predicate, (c) the shifted-self u128 inline-view mask (`data & !(data << 1) & INLINE_CONTINUATION_BITS & used` — a byte is continuation iff its PREDECESSOR was a leading byte; no per-byte loop), (d) both-directions in-rule adjudication (equality-broadcast vs class-mask; count-per-block vs identity arithmetic).

**R1 `first-hit-flag-guards-aux-structure`** — category: caching (the flag memoizes the aux-write; the bit-steal is the layout atom). Buggy: per-event `bitset.fetch_or(bit)` RMW on the shared word for EVERY hit. Fixed: a MARKED bit stolen into the row-index word (`1 << (BITS-1)`); the bitset is touched only on the flag TRANSITION.
```rust
// buggy (per-event side-structure write — the shared word is RMW'd on every repeat hit)
pub struct JoinState { pub seen: Vec<u64>, bits: usize }
impl JoinState {
    pub fn mark_hit(&mut self, first_row: usize, rows: &[u32]) {
        let (w, b) = (first_row / 64, first_row % 64);
        self.seen[w] |= 1u64 << b;              // RMW every hit, dup-heavy streams pay per repeat
        for &r in rows { let (w2, b2) = (r as usize / 64, r as usize % 64); self.seen[w2] |= 1u64 << b2; }
    }
}
// fixed (flag-in-entry guards the aux write; the index word's top bit is the flag)
pub const MARKED: u32 = 1u32 << 31;
pub struct JoinState { pub rows: Vec<u32>, pub seen: Vec<u64> } // rows[i] carries idx | MARKED once marked
impl JoinState {
    pub fn mark_hit(&mut self, first_slot: usize, rows: &[u32]) {
        if self.rows[first_slot] & MARKED == 0 {
            self.rows[first_slot] |= MARKED;    // the transition fires ONCE per key
            for &r in rows { let r = (r & !MARKED) as usize; self.seen[r / 64] |= 1u64 << (r % 64); }
        }
    }
}
```
G1: the marked/unmarked set bit-identical at every density. Bench (T4): seeded duplicate-density ladder [0/50/90/99%] × [1K,16K,256K]. QUOTE BANK @ `437948302` (all verified byte-exact): matched_rows.rs L4 `/// Set on the first row index of a key once a probe key matched it.` · L5 `pub const MARKED: IdxSize = 1 << (IdxSize::BITS - 1);` · L36-40 `if first_idx & MARKED == 0 { first.store(first_idx \| MARKED); for row in rows { self.mark(row.load() & !MARKED); } }` · second in-source instance single_key.rs L29 `nulls_marked: RelaxedCell<bool>`.

**R4 `u128-funnel-shift-combine`** — category: arithmetic. Buggy: `(lo >> s) \| (hi << (64 - s))` — UB/wrong at s=0 (`hi << 64` panics debug, wraps wrong release). Fixed: the u128 funnel `(((hi as u128) << 64) \| lo as u128) >> (s % 64)`; the `% 64` IS the s=0 correctness. G1: exhaustive shifts 0..=64 × boundary word pairs vs the reference. QUOTE BANK @ `437948302`: iterator.rs L431-432 `// Compiles to 128-bit SHRD instruction on x86-64.` + `// Yes, the % 64 is important for the compiler to generate optimal code.` · L433-435 `let wide = ((hi as u128) << 64) \| lo as u128; (wide >> (self.shift % 64)) as u64`. Adjudication in-rule: B69 `unaligned-word-load-from-bytes` assembles from BYTES (try_into + from_le_bytes) — this shifts two loaded WORDS; adjacent-distinct.
```rust
// buggy (the two-shift form — s=0 is the trap)
pub fn combine_two_shift(lo: u64, hi: u64, s: u32) -> u64 { (lo >> s) \| (hi << (64 - s)) }
// fixed (the u128 funnel — the % 64 makes s=0 exact)
pub fn combine_funnel(lo: u64, hi: u64, s: u32) -> u64 {
    let wide = ((hi as u128) << 64) \| lo as u128;
    (wide >> (s % 64)) as u64
}
```

R3/R5/R6/R7/R8: DEFER to the snapshot as staged leads (each keeps its 405 grade + reason) — this batch lands 2 rules + 1 face, at the plan's 2-4 target; the survivors re-grade at their own batch (the authoring-time kill risk rows R5/R3 were the disclosed thin ones).
- [x] T4 — RUN 2026-11-13 ~23:52 +07, load 1.25 quiet (the Zed rustc had just drained): R1 ParetoDominant 3/0/0 TWO-RUN STABLE (13.62x/52.32x/43.03x then 12.77x/50.47x/63.83x, allocs 0->0 every size) — GOAT PASS; R4 two-run Mixed (2/0/1: 1.15x/1.08x/0.98x then 1/0/2: 1.08x/1.03x/tie) — landed correctness-first with the timing disclosed. Bars read: R1 bench = bitset-mark-per-event vs flag-guarded mark over a seeded duplicate-density ladder [0%, 50%, 90%, 99%], sizes [1K, 16K, 256K]; G1 bit-identical marked/unmarked sets at every density; bar = ParetoDominant at ≥90% density AND no regression at 0% (the flag's load+branch tax). R4 = exhaustive shifts 0..=64 × boundary word pairs vs the two-shift reference (the shift=0 trap pinned in the tampered arm). R2 (if standalone) = chars().count() vs byte-class popcount over ASCII-heavy/mixed/multibyte-heavy ladders, G1 exact on valid UTF-8.
- [x] T5 — 6/6 standalone-compiled AND executed pre-entry (rustc --edition 2021 --test; four-arm per law: the inverted-guard tampered arm diverges and is caught, the clamp-heal tampered arm returns the wrong window at s=0 silently; two fixture bugs of the lane's own caught and fixed pre-entry).
- [x] T6 — ALL GREEN: default lib 3877/0, no-default 1070/0, rust_perf_bench lib 3908/0, batch360 needles + kernel witness re-pinned, clippy -D warnings x3 postures (5 doc_markdown mechanicals in the new files fixed inline), fmt on touched files, retrieval floors HELD (regression gate green, no re-pin), pins_manifest --write + bare AGREE (334/136/198/15). Gates read: batch360 needles, default lib, rust_perf_bench lib (if benchable entries landed), clippy `-D warnings` ×3 postures, fmt gate, retrieval floors (re-pin only on a REAL regression), pins_manifest --write same commit as the corpus change.
- [x] T7 — DONE: 5 corpus pins + 2 annotated-census pins + kernel witness + health total 1649->1651 + the perf-334 line pin + mod.rs narrative + fn doc + AGENTS/README/.docs prose heads; freeze/score-bench NOT owed (zero clippy_lints surface — asserted); release-set binary rebuild disclosed to the i224 lane. Pins read: AGENTS.md + README + `.docs/11_domains/rust_perf.md` head + health_views + mod.rs narrative + the rust_perf witness wherever the B355/B359 batch landed it; freeze/score-bench NOT owed if the clippy_lints surface is untouched (assert it).
- [x] T8 — DONE: this plan CLOSED; Research 405 -> RECORD; snapshot landing row + discharged claim; `.raw/polars` removed at close; ONE feat commit + push. Docs read: AGENTS.md batch entry; queue snapshot Batch 360 pool → DONE; Research 405 Status → RECORD (batch landed); `.raw/polars` removed at close (cleanup is part of the batch — a landed batch with a live .raw entry is an unfinished batch).

## Non-goals

- No polars outbound-PR lane (their AI policy forbids agent interaction; every row is PRESENT-state decision content).
- No polars dependency anywhere — fixtures are original std-only reproductions.
- The 2.0 streaming/OOC engine surface: engine-ambient, out of scope (recorded as drops in Research 405).
