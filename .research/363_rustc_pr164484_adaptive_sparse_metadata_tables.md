# Research 363 — rust-lang/rust PR #164484: adaptive sparse encoding for metadata tables

**Status:** DISTILLED — pending owner decision. **Delegated-owner verdict round: AGREE (round 1, session aa025329, 2026-11-03)** — grade/domain/dedup upheld; the reviewer's independent dedup pass (`rank.?select|succinct|elias.?fano|roaring`) found no collision (`byte-rank-select-rarest-probe` is frequency-rank for search, not popcount-rank); one wording correction adopted: the nnethercote ~11-01 queue lead is RELATED EVIDENCE and stays OPEN (the lead is author-scoped; this PR is TaKO8Ki-authored / nnethercote-reviewed), never "discharged".
**Verdict:** YES B− corpus (1 rust_perf rule + 1 face; category `layout`; suggestion-tier; NotBenchable-by-metric) · league-lane N/A STATED · the standing "nnethercote ~11-01" queue dated lead: RELATED EVIDENCE, lead stays OPEN (author-scoped; see the verdict-recording line above)
**Batch slot:** nominal Batch 326 — `.plans/330_rustc_sparse_metadata_tables_batch326.md` (STAGED, plan-first)
**Inspected:** 2026-11-03 (Zed m3, GitHub API reads: PR metadata + files/patch + issue-comments thread; no clone needed — the entire mechanism is 4 files, +101/−24, quoted from the files API at the pins)

## Source

- <https://github.com/rust-lang/rust/pull/164484> — "Use adaptive sparse encoding for metadata tables".
- Author TaKO8Ki (rust-lang member), assigned and approved r=nnethercote; merged by bors 2026-11-01T01:40:35Z; milestone 1.101.0.
- Pins: head `de21c98c951e91adb4abf353494ba7739f425ab8` · merge `026faeb1afef7581b2e49b2512d361d09819cb69` · METADATA_VERSION 11 → 12 (wire break).
- Files: `compiler/rustc_metadata/src/rmeta/{table.rs (+84/−14), mod.rs, decoder.rs, encoder.rs}`.
- **Provenance chain (cite both ends):** the PR deletes the standing FIXME it implements — verbatim from the diff: `// FIXME(eddyb) investigate more compact encodings for sparse tables.` / `// On the PR @michaelwoerister mentioned:` / `// > Space requirements could perhaps be optimized by using the HAMT `popcnt`` / `// > trick (i.e. divide things into buckets of 32 or 64 items and then` / `// > store bit-masks of which item in each bucket is actually serialized).` So: PR ← FIXME ← michaelwoerister's PR discussion ← HAMT popcnt indexing.

## Mechanism (verbatim at the pin)

Per-def-index metadata tables where most entries hold default/absent values. Three decision contents:

1. **The adaptive smaller-of gate** (table.rs `write_to`):
   ```rust
   const SPARSE_BLOCK_LEN: usize = 64;
   const SPARSE_INDEX_BYTES: usize = 12;
   // census pre-gate: count non-defaults ONLY when the index could pay for itself
   let non_defaults = if index_bytes < len * width {
       self.blocks.iter().filter(|block| **block != [0; N]).count()
   } else { len };
   // Use dense encoding when at least 75% of the entries are non-default.
   // This avoids sparse lookup overhead for tables with few default entries.
   let sparse = len - non_defaults > len / 4
       && u32::try_from(non_defaults).is_ok()
       && index_bytes + non_defaults * width < len * width;
   ```
   Sparse is chosen **only when it encodes smaller** (`index_bytes + non_defaults*width < len*width`) AND the table is sparse enough (>25% defaults) AND the packed-position count fits u32. The 75% occupancy knob is measured, not guessed — the review thread: "I've restored the occupancy threshold to 75% for now, since lowering it to 50% didn't show a clear benefit."
2. **The blocked-bitmap layout** (the HAMT popcnt trick): per 64-entry block one `(bitmap: u64, base: u32)` index record — `base` = "counts the non-default entries in all preceding blocks, so lookups do not need to scan their bitmaps"; the bitmap "bits mark non-default entries, starting with the least significant bit"; values follow packed in table order. Cost: 12 B per 64 entries.
3. **The O(1) rank decode**: `index = base as usize + (bitmap & (bit - 1)).count_ones() as usize;` — unset bit → `Default::default()`; set bit → the popcount-rank positions into the packed region. The mode flag rides the existing header word: encode `(width << 1) | usize::from(sparse)`, decode `width_and_sparse >> 1` / `& 1`.

## Measured (thread-verified; provenance, never bars)

rustc-perf try runs (10-17, 10-23 ×2, and the 11-01 rollup triage), consistently across runs:

- **Crate metadata size: −20.15% average** (author comment 6246665077: "the instruction-count regressions, averaging 0.66% … are an acceptable trade-off for the 20.15% average reduction in crate metadata size"). ⚠ The prompt that commissioned this distill said 20.18% — the thread-verified figure is **20.15%**; recorded here, corrected.
- **Max-RSS: −3.7% to −3.8% primary** ([−7.1%, −0.4%], 213–249 benchmarks per run) — a memory-footprint win beside the disk win.
- **Instruction count: +0.3% to +0.4% mean primary** ([0.1%, 2.6%] range) — ACCEPTED, `perf-regression-triaged`. nnethercote's closing triage (comment 6570030431): "This increased instructions counts a bit but greatly decreased metadata size and also reduced max-rss. Cycles and wall-time were neutral. We decided before merging that this trade-off was reasonable."
- Binary size −0.4% to −0.6%; cycles/wall neutral.

The accepted instruction bump is not a footnote — it is the rule's **anti-regime evidence**: the sparse decode path adds a 12 B index load + bit test + popcount on every access; surfaces where random-access read latency dominates and bytes don't matter stay dense.

## Domain routing

**`rust_perf`, category `layout`** — the tempting WRONG domain is `kernel_opt`: the corpus's entire sparse/bitmap/popcount vocabulary currently lives in kernel_opt rules, and the source is compiler code. The rule's Rust shape is a host-side serialized-table encoding decision (encode-time format selection over index-keyed tables), the same host-shape class as the layout-category size rules.

## Dedup evidence (3+ vocabularies, live corpus 2026-11-03: rust_perf 292 / kernel_opt 1051)

| Neighbor | Read verdict |
|---|---|
| `csr-index-via-prefix-sum` (kernel_opt, popcorn) | DISTINCT — CSR **construction** mechanics (prefix sums + atomic slot counters building index lists in-kernel); no wire format, no presence bitmaps, no rank decode, no adaptive gate. |
| `tile-coo-sparse-tile-aligned-format` (kernel_opt, FlashQuant) | DISTINCT — a sparse **format** but tile-local, designed backward from fused-GEMM kernel constraints (u8 tile-local coords, bucket metadata, uint4 records); consumer is a GPU kernel, not a serialized artifact decoder. Cross-domain kinship to name in-rule. |
| `distinct-index-bitmap-ascending-compaction` (kernel_opt, MLX) | DISTINCT — bitmap + popcount prefix scan as a **one-pass EMIT** of set bits ascending; this rule's decode is a **persistent random-access rank** structure. |
| `attention-row-mask-bitmap-per-row-not-per-block` (kernel_opt, uzu) | DISTINCT — packed row masks inside a kernel's score test; not a table wire format. |
| rust_perf corpus (`sparse|dense|bitmap|popcount|serial|compact|fill ratio` sweeps) | **NO owner** for serialized index-keyed-table encoding. Closest are the size-axis family precedents, not collisions: `box-large-enum-variant` ("Not benchable: the win is size_of", `bench_spec: None`) and `narrow-counter-overflow-promotion` (measured ParetoDominated on speed, kept for the memory axis with honest notes). |

## GOAT test

Candidate: `adaptive-sparse-blocked-table-encoding` (final name at authoring).

1. **Benchable: NO** — the win metric is artifact bytes + max-RSS; the rust_perf validator measures latency Pareto + alloc count. This failure is the corpus's established size-axis class (`bench_spec: None` precedent above) → **per-rule NotBenchable ANNOTATION, never a demotion** (B136/dial9 split; B202 landed a compiler-internals rule exactly this way: rustc #160193 → rust_perf, NotBenchable, grade B).
2. **Exact semantics: YES** — decode-identical values (default entries → `Default::default()`, others bit-identical bytes); the fixture proves round-trip equality against dense encoding plus both gate arms.
3. **Magnitude story: YES** — measured on the largest build matrix in existence (rustc-perf): −20.15% metadata, −3.8% max-RSS, mechanism arithmetic (12 B per 64 entries vs `width` bytes per every default slot).
4. **Deterministic: YES** — pure function of table contents; no timing-dependent verdicts.

→ Criteria 2–4 pass; criterion 1 fails metric-domain → annotation class. Suggestion-tier decision-content entry (a parse-only proposer cannot see a table's fill distribution — same type-blindness posture as `narrow-counter-overflow-promotion`).

## Ranked candidates

| # | Candidate | Grade | Notes |
|---|---|---|---|
| 1 | `adaptive-sparse-blocked-table-encoding` (bitmap blocks + prefix base + packed values + smaller-of gate + header flag) | **B−** | THE rule. All four decision contents above; anti-regime carries the accepted instruction cost. |
| 2 | Census-gated measurement face (count non-defaults only when `index_bytes < len*width` could win) | face | Folded into #1's admission — "don't run the census when the census can't pay". Never standalone. |
| — | Header flag bit-packing (`(width<<1)|sparse`) | DROP | Too thin; ambient layout packing. |
| — | METADATA_VERSION 11→12 | DROP | Wire hygiene, not a perf pattern; recorded as a caveat (adopting the shape = version bump). |

**Yield estimate:** 1 rule + 1 face after ~50% coordinator falsification. If the rule dies at T2/T5, the plan closes as a clean negative with this note standing.

## Lane-intel axis

**No league/lane mapping** — the league lanes are Metal/CUDA Bonsai prefill/decode; compiler metadata does not move them. STATED per the skill.

**Queue-lead relation (the strict reading, verdict round 1):** the mining queue carries a standing dated lead "nnethercote ~11-01" (the 10-17 watch pass: `author:nnethercote` merged ≥10-08 = 5 PRs, "ALL rustc-internals class … none converts to app-level rust_perf/kernel_opt at fidelity; the ~11-01 dated lead STANDS"; cited again at B323's landing). This PR merged exactly 2026-11-01T01:40Z in nnethercote's review domain (metadata/rmeta, his active area) and converts at fidelity — but the lead is AUTHOR-scoped and this PR is TaKO8Ki-authored, so it is RELATED EVIDENCE for the watch, not a discharge: the strict `author:nnethercote` re-sweep stays with the next watch pass, lead OPEN.

## Product-coverage axis

No famous-lib family in play (this is rustc itself, the ecosystem root). The rule seeds a compact-index-table encoding shape whose realistic consumer surface is hand-rolled binary formats — storage/serialization crates, cache files, telemetry sinks, mmapped indexes: moderate, honestly niche, and NOT a tokio-class gap. Follow-up pool if the family grows: this PR (the canonical modern instance), the FIXME/HAMT-popcnt lineage, the rustc-dev-guide rmeta chapter, and Roaring-bitmap-class literature for the run/container variants.

## Honest caveats

- **20.15%, not 20.18%** — thread-verified correction of the commissioning prompt's figure.
- `perf-regression-triaged` = a deliberate, reviewer-ratified trade — never quote the label as a defect; it IS the anti-regime evidence.
- The 75% occupancy knob is upstream-measured (50% tried, reverted); our rule records it as a measured knob, never re-tunes it blind.
- `u32::try_from(non_defaults)` cap: the packed-position index is u32; tables above 2³² non-defaults stay dense. Fixture must include the boundary.
- METADATA_VERSION 12: any adoption of the shape in a persistent format owes the version-bump discipline.
- Delegated-owner verdict round for this distill was rate-limited at record time (resets 15:10 ICT 2026-11-03; a single post-reset re-attempt is disclosed in the landing row). The recording stands on the skill's own gates (dedup 3+ vocabularies, GOAT two-class split, B202/size-axis precedents).
- Nothing invented: every quote above was read at the API pin this session; quotes re-verify at T1 of plan 330 before any corpus edit.

## Queue slot

Nominal **Batch 326**, plan `.plans/330` (STAGED). At record time the queue head is open: uniffi-rs/plan 327 landed as Batch 325, sccache/plan 328 landed as Batch 324; the two `.plans/329_*` mozilla-lane files on disk were a live dual-allocation — **resolved owner-directed 2026-11-03**: first-allocation keeps 329 for the methodology port, the wetrun renumbered to `.plans/331_mozilla_outbound_pr_wetrun.md` with every citing site moved, `.plans/.highwater` → 331. Corpus edit waits for the slot claim per the mining queue.
