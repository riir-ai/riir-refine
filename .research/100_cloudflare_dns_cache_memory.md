# Research 100 — Distill verdict: Cloudflare 1.1.1.1 DNS cache memory optimization (Big Pineapple)

**Status:** RECORD — LANDED as Batch 86 / Plan 047 (2026-08-29, idle loop; corpus 91 → 94, allocation 20→22 + layout 8→9). GOAT: serialize-once-byte-buffer-store ParetoDominant 4.26/3.52/2.90× with alloc axis EXACT 200,001→2 (2n+1→2, the malloc storm IS the mechanism); context-equal-field-elide ParetoDominant 3.12/3.01/2.00× at the ~80% mix with allocs 100,001→20,056 (the elision regime made visible); bitflag-pack NotBenchable-speed with the 12→8 size axis pinned by gate. Verdict-stable ×2 both benchable; G-det stable; retrieval floors 13/13 HELD zero re-pins; 809/0 default + 827/0 bench-state lib; clippy 0 both states. Both riders landed (box-str-immutable → Box<[T]> generalization; box-large-enum-variant honest costs). All 3 candidates survived authoring falsification — context-equal did NOT collapse into the dedup/intern family (the distinct decision: the value is ABSENT because the context re-derives it — no sharing, no canonicalization). Count-pin walk found the corpus header line STALE at 89/79/10 (B85 never bumped it) — now true at 94/82/12.

## Source

- **URL:** https://blog.cloudflare.com/dns-cache-memory-optimization-1111/ — "How we saved 100 terabytes of memory by optimizing 1.1.1.1's DNS cache" (Sebastiaan Neuteboom, 2026-08-27, 12-min engineering blog).
- **Inspection date:** 2026-08-28 (full text read; no code repo exists — blog only).
- **Verdict:** YES, **B+** — route **`rust_perf`** (categories: allocation + layout). NOT `kernel_opt` — zero GPU content; no routing trap present (stated per format). Not a league item.
- **Grade rationale:** Meilisearch/Datadog-class production-verified engineering blog (counting-allocator benchmark + fleet rollout graphs with a before/after table), the lane the idle-loop online-search path re-validated on B73/B75/B76/B78. The discount from A− is **corpus overlap**: roughly 60% of its territory is already covered by `box-str-immutable` / `box-large-enum-variant` / `shrink-to-fit-after-build`, so the marginal yield is 2–3 rules + 2 riders, not 5.

## Source shape (what it is)

Cloudflare's Big Pineapple DNS cache: >250B entries fleet-wide; five successive storage changes cut the **per-entry footprint 953 → 420 B (−56%)**, per-entry allocations **1.1 KB → 461 B (−58%)**, insert throughput **625K → 893K entries/s (+43%)**, lookup latency **828 → 670 ns (−19%)**; ~100 TB fleet working-set freed (production p99 resident 9.3 → 5.3 GB). Methodology is exactly our perf-bench validator's shape: a **custom counting allocator wrapping `System`** recording per-entry alloc count + bytes (our alloc axis), a production-like traffic mix (56% A / 25% AAAA / 19% TXT stand-in, 1–4 records/entry), plus production rollout measurement to confirm resident-memory translation. jemalloc underneath.

## The five changes (mechanism inventory)

1. **`Vec<T>` → `Box<[T]>`, `String` → `Box<str>`** for frozen (write-once) fields — drops the 8-byte capacity word per field AND the growth slack on the heap. 8 such fields/entry → 64 B/entry, >15 TB fleet.
2. **Answer/authority/additional as ONE list + `u16` offsets** (DNS record counts fit u16) instead of two more slice headers — −28 B/entry; plus **several bools packed into a single bitflag**, whose padding collapse shrank the struct by MORE than the bool bytes.
3. **Owner elision** — `owner: Option<Box<Name>>`; `None` = identical to the queried domain (the cache key, already in hand at every lookup), `Some` = full heap name (the CNAME-chain case). Most records elide.
4. **Box the large enum variants** — `RecordData` was 144 B because NAPTR (136 B) dominated, while A (4 B) + AAAA (16 B) are >80% of traffic. Boxing the large variants shrinks the common case by ~120 B/record. Honest costs recorded: allocator **size-class rounding** (TXT 32 B fits its bin; MX 40 B rounds to 48), **locality scatter** (one contiguous allocation → many), and the rare large variant (NAPTR) now pays slightly MORE.
5. **Records as length-prefixed raw wire bytes in ONE `Box<[u8]>`** (+ the u16 section offsets from #2) built via a **reused scratch buffer** that persists across insertions; `A`/`AAAA`/`TXT`/DNSSEC copy straight out of the buffer into the outgoing message (no re-serialize); only name-carrying records (CNAME/NS/MX/SOA) still parse for RFC 1035 name compression. Insert +13%, lookup −5% — the closest-to-isolated attributions in the piece.
- **The REJECTED alternative (the anti-shape):** full-wire-message storage — DNSSEC (DO flag) would force caching two variants or filtering post-build, plus per-lookup parse cost. Rule 1's doc must carry WHY the middle ground (record bytes only, structured metadata kept) was chosen.

## Provenance chain

Blog-only; numbers author-reported and production-verified by rollout (May 18 → Jul 6, 2026). The +43%/−19% and the 953→420 B table are **END-TO-END cache-flow numbers — never cite isolated** (the mnt.io precedent). Wire-format/name-compression upstream = RFC 1035.

## Dedup-gate evidence (grep `src/domains/rust_perf/`, both vocabularies, 2026-08-28)

| Candidate | Grep terms (source + operator) | Result |
|---|---|---|
| Vec→Box<[T]> / String→Box<str> | `box_str_immutable`, `Box<[`, `capacity` | **`box-str-immutable` EXISTS** ("Vec<String> (3-word handle) → Vec<Box<str>> (2-word, no capacity)", benched) → the str half is a positive instance; the `Box<[T]>` generalization = **RIDER**. Slack half covered by **`shrink-to-fit-after-build`** + `vec-with-capacity`. |
| Box large enum variants | `box_enum`, `box-enum` | **`box-large-enum-variant` EXISTS** (BigData([u8;1024]) → Box, NotBenchable size axis) → positive instance; the honest costs + 80%-skew regime numbers = **RIDER**. |
| u16 section offsets / one-buffer | `offset`, `arena`, `length-prefix` | **CLEAN** — no corpus rule. (`usize-index-to-u32` narrows a field type; it does not replace sibling slice headers with offsets.) |
| Context-equal field elision | `elide`, `intern`, `Option<Box`, `dedup` | **CLEAN** — only `dedupe-hashset-vs-vec` (value dedup) + elision comments in clone rules; nothing stores `None = same-as-context`. |
| Bitflag bool packing | `bitflag`, `bool-fields`, `bitset` | **CLEAN in rust_perf** — only the tantivy TinySet xor rider on `tz-blsr`. Note: the REGIME BOUNDARY twin exists in **kernel_opt** (Batch 32a `bool-fields-over-bitset-hot-reads`: per-tick-READ flags stay bools; pack when bytes streamed) — the CPU stored-at-scale side is its complement and must cross-cite it. |
| Serialize-once byte-buffer store | `wire`, `serialize-once` | **CLEAN** — no rule stores frozen records as length-prefixed bytes for copy-out rebuild. |

## Ranked candidate table

| # | Pattern | Source | Grade | Domain/category | Notes |
|---|---|---|---|---|---|
| 1 | `serialize-once-byte-buffer-store` — frozen records as length-prefixed raw bytes in ONE `Box<[u8]>` + `u16` section offsets + reused scratch; copy-out fast path for copyable payloads | change 5 (+2) | **A− (GOAT lead)** | rust_perf/allocation | Absorbs the offsets form as the composed shape. Regime: sequential-rebuild consumers (cache → rebuild message) win; random-index consumers lose (round-robin rotation complexity, the source's own caveat). G1 = byte-identical reconstruction. |
| 2 | `context-equal-field-elide` — `Option<Box<T>>` with `None` = same-as-enclosing-context (the cache key), inferred at read | change 3 | B | rust_perf/allocation | Two-regime: %-equal high (most DNS owners = qname) vs low (then it's pure overhead: tag + branch at read). Composes with 1. |
| 3 | `bitflag-pack-when-footprint-dominates` — bools → bitflag for stored-at-scale structs; padding collapse can exceed the field bytes | change 2 (padding para) | B− | rust_perf/layout | Deterministic size axis (the `box-large-enum-variant` NotBenchable-speed precedent). Cross-cite the kernel_opt `bool-fields-over-bitset-hot-reads` inverse regime. Mine-time check: default-repr field reorder interaction. |
| 4 | **RIDER** `box-str-immutable` ← generalize to `Box<[T]>` (any frozen T, not just str); cite 64 B/entry + 100 TB | change 1 | rider | rust_perf/allocation | The B66 rider precedent: positive instance → doc rider, not a new rule. |
| 5 | **RIDER** `box-large-enum-variant` ← honest costs (size-class rounding, locality scatter, rare variant pays more) + the skewed-frequency regime numbers | change 4 | rider | rust_perf/layout | The article's costs section is exactly the caveat the existing rule lacks. |
| 6 | Evidence only: counting-allocator methodology (= our alloc axis, already shipped), scratch-buffer persist (= `reuse-allocation-clear` positive), full-wire-message rejection (= anti-shape note inside rule 1) | methodology + rejected alt | — | — | No entries. |

**Drops:** full-wire-message cache (rejected by the source itself; encoded as rule 1's anti-shape), jemalloc bin taxonomy (allocator-specific, ambient), ECS multi-version caching (architecture).

## GOAT check (lead: rule 1)

1. **Benchable** — yes: insert-path timing + the exact **alloc-count axis** (per-entry allocs drop from ~1-per-record to 1; bytes drop by the enum/padding overhead). Matches the source's own isolated +13% insert attribution.
2. **Exact semantics** — wire bytes are canonical; G1 = byte-identity of reconstructed records/payloads.
3. **Magnitude story** — structural: −(8+8) B per replaced slice-header pair, −(largest-variant padding) per record, −(N−1) allocations per entry; source-attributed +13% insert / −5% lookup.
4. **Deterministic** — yes: fixed traffic mix + fixed seed.

## Yield estimate

**2–3 rules + 2 riders** after ~50% coordinator falsification. Candidates 2 and 3 are the most falsifiable: elide could collapse into a dedup/intern family shape at authoring; bitflag must survive the `#[repr]`/field-reorder interaction and the hot-read counter-regime.

## Caveats (honest)

- **Blog only, no repo** — every number is author-reported; fleet figures are context, never isolated mechanism proof. Quote only the two near-isolated attributions (+13% insert, −5% lookup) with that qualifier.
- **DNS specifics shape rules 1–2** (name compression, DNSSEC DO flag, ECS). The generic forms must abstract to "a value already available in the enclosing context/key" and "the consumer rebuilds the serialized form anyway, with small patches".
- **Default-repr interaction:** Rust field reordering means bitflag packing's padding win is layout-dependent; the rule must say "measure `size_of` before/after", not promise the collapse.
- **rust-optimize checklist cross-check (owner's second lens):** the candidates map onto checklist items 1 (allocations: `Box<str>`, scratch reuse), 2 (layout: bitflag, u16 offsets), 3 (elision = compute/store once), 8 (bounded domain → fixed-size `u16`). No `.rs` scope was given, so the optimize-loop itself had no target — the checklist was used as the verdict lens only; convergence with it is itself evidence the source is corpus-shaped.
- Corpus fixtures must be **original minimal reproductions** (never paste the article's structs verbatim — license hygiene + the binary-only distribution rule).

## Queue slot

**Batch 86 / Plan 047** (this note). Batch 85 = the sibling's queued cflow distill (Research 098 / Plan 046) — disjoint surfaces, never sweep their WIP. Mine-time re-verify: re-read the article (it is immutable, but re-confirm no Part 2 landed), re-run the dedup greps (the corpus moves between verdict and mine), author entries plan-first. After landing: update this note's Status → RECORD with the batch pointer, add the queue-snapshot line, walk the count pins.
