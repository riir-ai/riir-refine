# rsconstmap / constmap — paired-probe-layout + static-map distill verdict

**Status:** RECORD — Batch 179 LANDED (see HISTORY.md for the landing commit; the three rules + rider measured ParetoDominant on M3; plan-first verdict was YES B+)

- Source: https://github.com/lemire/rsconstmap @ `bf7b33419d37c5d141075b9c1642fb69642a7896` (shallow clone `.raw/rsconstmap`, removed at landing commit; re-clone at the pin at mine time).
- Crate: `constmap` 0.3.0 on crates.io. License: MIT OR Apache-2.0. 10★, 4 commits, last 2026-09-16 — COMPLETE, not WIP; criterion benches; rust-version 1.80.
- Inspection date: 2026-09-24 (4090 box, user-directed `@distill`).
- Provenance chain: rsconstmap (Rust port) ← lemire/constmap (Go, tracks its v1.1.0) + fastconstmap (C/Python sibling) ← **Binary Fuse Filters: Fast and Smaller Than Xor Filters**, Mueller Graf & Lemire, ACM JEA 27, 2022 (DOI 10.1145/3510449) ← Xor Filters, ACM JEA 25, 2020. Cite the paper for the math, the repo for the Rust shape.
- Role: static string→u64 maps via binary fuse construction — `data[h0(k)] ^ data[h1(k)] ^ data[h2(k)] == value`; lookup = one XXH64 + three array reads + two XORs. Three variants: `ConstMap` (absent keys undefined, ~9 B/key), `VerifiedConstMap` (fingerprint word per key → `NOT_FOUND`, ~18 B/key), `PairedVerifiedConstMap` (verified + value collocated with fingerprint in 16 B slots).

## Dedup-grep evidence (riir-clippy/src/, both vocabularies)

- `fuse filter|xor filter|bloom|perfect hash|phf` → 0 rule hits (1 kernel_opt BLOOM kill-switch name, 1 shader fixture string, 1 oxc **negative note** in entries_03: gperf-class perfect hash could NOT beat a plain `match` — that is the compile-time-literal regime, disjoint from this source's runtime-built regime; cite as adjacent contrast, not duplicate).
- `hashmap` (rust_perf) → family exists: `o1-lookup`, `fxhashmap`, `entry-vs-contains-insert`, `hashmap-with-capacity`, `btreemap-sorted-iteration`, `papaya-lockfree`, **`keyword-map-to-match`** (LazyLock<HashMap<&str,V>> literal dispatch → match, Batch 61). NONE covers the runtime-built-immutable-large-keyset regime. Distinctness: keyword-map-to-match requires COMPILE-TIME literals; this rule's keys exist only at construction time.
- `cache.line|paired|interleav|aos|soa|colocat` → `soa-hot-field` (AoS→SoA for one-hot-field sequential scans), `align-64-false-sharing` (coherence), `narrow-counter-overflow-promotion` (density). None is SoA→paired-slots for co-probed arrays. PASS.
- `batch.*(hash|gather|lookup)|map_many|block of 8|memory-level parallelism|in.flight` → 0 rule hits. `software-prefetch-storage-bound` READ (B85): removes SW prefetch HINTS on bandwidth-bound streams — different mechanism entirely (no hints here; loop restructure so dependent gathers issue after independent hashes → OoO/MLP overlap). `dual-mask-fused-block-scan` (B172): fuses two predicates per block (compute fusion); this DEFERS dependent gathers (dependency breaking). PASS.
- `chunked write|64 kib|bufreader|bufwriter` → `buffered-io` + `bufwriter-batched` exist (syscall amortization via wrapper). The source's self-chunk finding is a RIDER on that family, not a rule (see drops).
- Lane-intel greps (`riir-ai/.issues`: vocab lookup / tokenizer hash / perfect hash / fuse / constmap / static string map) → **0 hits**. No open lane maps.

## Ranked candidates

| # | Pattern | Source | Grade | Notes |
|---|---|---|---|---|
| 1 | **`paired-probe-layout-present-vs-absent`** — two parallel arrays probed at the same indices per lookup (fingerprint + value) → interleave into 16 B `[u64;2]` slots; present-key probes touch 3 cache lines instead of 6; **absent keys REGRESS** (the separate fingerprint array is half the map and early-exits before touching values — pairing always drags the value in, miss working set doubles). Regime keyed on hit rate: "Use Paired when most keys you look up are present, Verified when most are absent." | lib.rs:709-795 (`Slot = [u64; 2]`, `xor3_slots`, the doc's own both-sides table); README §Paired layout | **A− (GOAT lead)** | layout category. Same total bytes both arms (alloc axis flat, pure timing). Baseline SSE2/NEON 128-bit slot XOR needs no runtime detection (lib.rs:749-751) — folds into fixed_code + annotation (adjacent to `runtime-feature-detect-per-chunk`: detection UNNECESSARY on baseline ISA, one line). Suggestion-only (struct+ctor+read-site rewrite — the soa-hot-field class). Distinct from `soa-hot-field` BOTH directions (single-hot-field sequential scan vs multi-word random probe; regime key = probe hit-rate, not field-hotness). |
| 2 | **`block-hash-before-gather-mlp`** — loop where each iter = hash(key) then dependent scattered loads → block of 8: hash all first (stack arrays), THEN issue all gathers; "the array accesses of every key in the block be in flight at once… That memory-level parallelism is the only reason a batch beats a loop over `map`." Honest negative cell IN-SOURCE: Xeon Verified cold = none (two-array probe latency the batching cannot hide); block 8-vs-16 both-sides measured ("Eight wins or nearly wins everywhere; 16 wins one case and loses another"). Anti-regime: map fits LLC → "roughly even". | mapmany.rs:11-23 (`BATCH_BLOCK = 8` + adjudication), :52-85 (two-phase block loop) | **A−/B+** | iterators category. Needs working set > LLC (10M keys ≈ 80 MB — reachable). Go-vs-Rust port note folds in: Go hand-hoists batched hashing because `xxhash.Sum64String` reloads primes per call; "in Rust `xxh64` inlines directly into the block loop, so … there is nothing left to hoist" — half the Go gain is already free here. Suggestion-only (restructure). |
| 3 | **`immutable-static-map-filter-selection`** — HashMap<String,u64> built ONCE from runtime data, never mutated, lookup-dominated → construct-once binary-fuse static map: key storage DROPPED (~9 vs 41-52 B/key at 1M), 7.0 ns vs 54.6 ns lookup (M4 Max, 1M keys), margin GROWS with key count. Regime lattice: absent keys = garbage (fingerprint variant +9 B/key for NOT_FOUND); no enumeration; construction slower (amortize). | README §Performance gains / §Scaling; lib.rs ConstMap/VerifiedConstMap | **B+** | complexity category (map-selection family). COMPLETES the family's regime lattice: keyword-map-to-match (compile-time literals, ≤dozen keys) / fxhashmap (mutable hot map) / btreemap (sorted reads) / o1-lookup (scan→map) / **this (runtime-built, 10k+, immutable, lookup-only)**. Suggestion-only (library swap); benchable via `[dep: constmap]` (papaya dep precedent). |
| R1 | `serialize-self-chunk-64kib` — contiguous in-memory array → self-chunk the transfer at 64 KiB instead of per-word writes; measured 1020 ms → 11.1 ms (1M keys); **BufReader does NOT help** ("the chunking already batches the syscalls, and the extra copy costs slightly more than it saves"), BufWriter ≈ same. + fused FNV checksum during transfer (HashWriter — no second pass). | serialize.rs:31-56 + README §Serialization | C+ rider | RIDER on `buffered-io`/`bufwriter-batched` (regime annotation: data already contiguous → self-chunk; wrapper's copy ≈ what it saves at 64 KiB). Not a standalone rule. |
| R2 | Wire header padded to 32 B so both u64 arrays land 8-aligned for mmap/alias-as-`[u64]` (README §Serialization). | — | C fold | Enabling/correctness decision, no perf number — annotation on #3. |
| R3 | Bench-honesty doctrine for map benches: query order RANDOM (construction order lets the prefetcher hide real work), key buffer allocated IN QUERY ORDER (else a scattered dependency-carrying load joins every lookup), query the WHOLE key set (small samples flatter every impl). | README §How these are measured | fold | Annotation rider on #1/#2 bench specs (the B174 bench-honesty precedent). |
| D1 | Binary fuse construction algorithm itself | lib.rs:1-700 | DROP | A library construction, not a bounded fix-space; we do not port it (fixtures-original constraint; the ported-numeric-algorithm exhaustive-gate rule never triggers). |
| D2 | XXH64 / FNV-1a / magic-bytes choices; Go/C interop formats | serialize.rs | DROP | Ambient; fxhashmap already owns the non-adversarial-hasher regime. |
| D3 | Baseline-SIMD-no-detection as standalone | lib.rs:749 | DROP | One annotation line on #1 (adjacent to runtime-feature-detect-per-chunk, which is about detection COST). |

## GOAT check — lead `paired-probe-layout-present-vs-absent`

1. Benchable on the rust_perf Pareto validator (alloc axis: same bytes both arms → flat-by-construction, pure timing isolation) — YES.
2. Exact semantics: u64 XOR machinery, values bit-identical — YES.
3. Magnitude: 6 → 3 cache lines per present-key probe; source-measured 21% (M4 Max) / ~9-20% (Xeon); bench must carry BOTH cells (present-query arm AND absent-query arm — the regression is the rule's regime content, not a failure).
4. Deterministic — YES. **4/4 PASS.** (#2 passes 4/4 with the cold-regime caveat; #3 passes with the absent-key contract stated.)

## Lane intel / product coverage

- **Lane mapping: NONE** (checked-and-empty: 0 hits in `riir-ai/.issues` for vocab/tokenizer-hash/perfect-hash/fuse/constmap/static-string-map; league lanes are GPU prefill/decode, untouched by host static maps).
- **Product coverage: minor-positive** — the std-HashMap-selection family (7 existing rules) gets its missing regime cell (immutable runtime-built large key sets). Follow-up pool for the family: phf crate (compile-time MPHF docs), lemire/constmap (Go) + fastconstmap (C) siblings of this same source, the JEA 2022 paper. Accumulation clause: pool reads before the batch window fold into the same batch.

## Caveats

- Consumer-base narrowness: static string→u64 maps are a niche (vs tokio/serde-class families) — the value is decision density, not ecosystem reach.
- Source numbers are two-machine (M4 Max / Xeon Gold 6548N), Lemire's own hardware; our benches measure our own cells (4090 box + M3 cross-check per the dual-pin precedents).
- Unread at verdict time (verify at mine time): lib.rs:1-700 (construction internals — not ported), serialize.rs:60-1260 (paired format + interop), report.rs, benches/constmap_bench.rs (informs OUR bench specs — the R3 doctrine lives there), tests/.
- README vs lib.rs doc quote a different present-key figure (21% M4 / 9% Xeon in README; "about 20% on Xeon" in the lib.rs doc) — source-internal variance, cite both honestly.

## Yield

3 candidates (A− / A−-B+ / B+) + 3 riders/folds. After coordinator falsification: **2-3 rules land + 2 riders** — the two layout/MLP rules are the load-bearing cells; #3 is the family-completion cell with the weakest proposer story.

Queue: nominal **Batch 179** (behind B176-FWHT-rider / B177-EXL3 / B178-cinference), Plan 172, lead with `paired-probe-layout-present-vs-absent`.
