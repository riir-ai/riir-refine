# 399 — SGLang Unified Memory distill (LMSYS blog, the one-arena hybrid-state design)

**Status:** DISTILLED — pending owner decision

## Source

- Blog: LMSYS "Unified Memory in SGLang: One Arena for Every Token Lifetime" — https://www.lmsys.org/blog/2026-11-11-unified-memory (2026-11-11; fetched in full 2026-11-12, live over the wire).
- Implementation: `sgl-project/sglang` behind `--enable-unified-memory` (experimental); the results section pins SGLang `daa171846f`, one H200, Qwen3.5-4B + GPT-OSS-20B. **NOT cloned this session** — a zero-cargo verdict lane; every candidate below is graded from blog prose (the dial9/penthera precedent). Plan 365's T0 optionally clones sglang at that pin to verify the admission/ledger code shapes before authoring; fixtures stay original Rust regardless.
- Provenance: SGLang is Apache-2.0; blog quotes are verbatim-attributed decision content. All candidates are **UPSTREAM: present at pin** (the feature ships behind the flag — decision-content transcription only, never outbound-PR leads; the P113 check resolves trivially).
- Disjointness within the LMSYS surface: Research 398 (same day) distilled a DIFFERENT post — Muse Spark ultrafast + `facebookresearch/sgl-muse-mega` (icache/L2/prefetch device rules). Zero candidate overlap; the two verdicts are complementary halves of the SGLang serving stack. The "unified memory" hits in Research 207/227 are the Apple/MLX **hardware** term (host page cache vs device alloc; bandwidth-cheap regime) — different axis entirely, cited in-rule where adjacent, never a collision.

## Verdict

**YES B− corpus [2–3 rules + 1 swing after ~50% falsification] / LANE INTEL MEDIUM-HIGH (riir-infer Issues 1201 + 1203 + 742 — the flashstates multi-session hybrid-state serving lane) / PRODUCT COVERAGE YES (seeds the deferred-reuse/epoch-lifecycle concurrency family — 0 owners; a serving capacity-integrity lane beside the B139 admission family).**

## Domain routing

`rust_perf` — categories: **layout** (R1), **concurrency** (R2), **serving/measurement** (R3, R4). Tempting WRONG domain: `kernel_opt` (the subject is GPU-serving memory) — rejected: every candidate's fix-space is HOST-side Rust (allocation geometry, free-list lifecycle, admission accounting); no device code anywhere in the blog.

## Ranked candidates

| # | rule_id | grade | notes |
|---|---|---|---|
| R1 | `co-lifecycle-rows-pack-one-entry-strided-views` | **B+ LEAD, the GOAT candidate** | Layout. Blog: "Each token's KV rows across all layers are packed into a single entry, allowing them to be allocated, freed, and moved together. Rows of different shapes and dtypes can share an entry, so speculative draft KV can use the same allocation and lifecycle as target KV even when their dtypes differ" + "a layer's view strides over the entries". Rust shape: N row-groups with IDENTICAL lifetimes but heterogeneous shapes/dtypes as parallel buffers → ONE allocation, per-group strided views preserving consumer shapes; one alloc/free/move/copy covers all. Regime: identical lifetimes AND bulk lifecycle ops (or fragmentation) matter. ANTI-REGIME: independent lifetimes/growth; packing that forces a readback or breaks a consumer layout (the split-decay pole — see adjudication). **Benchable**: alloc axis 2N→1 exact, time ~neutral → ParetoDominant on allocs (the `group_keys_borrow_allocate_on_first_sight` shape). MANDATORY in-rule adjudication BOTH directions vs kernel_opt `split-decay-outputs-eliminate-readback` (that rule: packing is a PESSIMIZATION when consumers read different shapes and forces a device→host→device readback; this rule REQUIRES views that preserve consumer shapes and wins on LIFECYCLE ops, never read avoidance — when packing would force a readback or break a consumer shape, split-decay wins = this rule's anti-regime) + vs `attention_combined_kv` (two same-shape halves, one binding — descriptor-pressure axis; this is N heterogeneous groups, lifecycle-amortization axis) + `carry-dual-layout-dead-slot` (one producer parameterized to consumer layout) + `partial-reset-watermark-pool` (pool reset-cost axis, not packing). Instances recorded, not separate rules: DSA indexer + MLA KV fused entry; draft-KV target-entry lanes. UPSTREAM: present at pin. |
| R2 | `logical-free-immediate-physical-reuse-waits-settle` | B− | Concurrency/lifecycle; suggestion-only, NotBenchable-by-domain (capacity-under-overlap metric — the dial9/Batch-136 class, annotation not demotion). Blog: "**logical free is immediate; physical reuse waits**" + the three-case table (free-with-old-reader; move-with-old-reader = copy + remap, no wait, only source REUSE waits; move-with-old-writer = ordered BEFORE the copy) + the pending/vacated/fenced states. Rust shape: arena/slot freelist carrying a pending set drained by epoch/event/settle watermark; the capacity fn counts logical free; the alloc gate refuses pending slots. Regime: any overlapped-execution runtime (schedule stream + compute stream, CUDA-graph replay, async pipeline). ANTI-REGIME: single-threaded immediate-reuse arenas (`partial-reset-watermark-pool`'s world — the pending machinery is pure overhead there). Family seed: deferred-reuse/epoch-lifecycle, 0 owners (grep below); the famous-lib pool is crossbeam-epoch — our own house map (papaya) is epoch-based, so the law conceptually fires at home. Distinct from `vanished-entry-tolerant-eviction` (peer-eviction tolerance), `slow-consumer-latest-value-single-slot` (coalescing), `deferred-command-flush-hoist-out-of-batch-loop` (barrier batching). Lane feed: 742 T4's CUDA-graph-safe cache ops are the graph-replay instance of this law — a page move under replay is exactly "move with an old reader". UPSTREAM: present at pin. |
| R3 | `joint-demand-admission-single-pool-free-bytes-logs-only` | B− | Serving/measurement integrity; suggestion-only. Blog: "A free-byte count for a single sub-pool exists only for logs and metrics, and a check mode flags any decision that reads it" + the 512-full-KV-pages + 64-window-pages worked example + eviction walks that "count what that victim releases in the other sub-pools". Rust shape: `admits(&self, demand: JointDemand) -> bool` over coupled demands (page rounding couples pools) vs a per-pool `free_bytes() >= need` read on a decision path. Regime: multi-pool/arena runtimes where demands are coupled. ANTI-REGIME: genuinely independent resources (single-resource admission is correct). Distinct from `prefault-admission-keyed-to-ram-headroom` (B139 — ONE resource vs TOTAL host RAM), from B274 `batch-load-admission-sized-from-files-refuses-past-the-hard-budget` (one hard budget), from `reserve-then-commit-capacity-accounting` (B324 sccache — reserve/commit WITHIN one pool). New axis: CROSS-pool coupling on the decision path. |
| R4 | `fixed-ratio-pool-partition-strands-demand` | C+ swing — face-or-drop gate at authoring | Blog evidence: wrong ratios cost 12%/14% throughput, cap concurrency 24 vs 51, strand bytes behind a startup guess ("unified memory ran about 1,000 requests at once where separate pools at the default ratio ran 618"); the ratio flags "remain only as boot-sizing targets, not walls at run time". Bounded shape: const-ratio partition over coupled pools → demand-driven fungible capacity (one arena). Honestly borderline architecture-tier; declared at authoring. Adjacent: B207's `unified-memory-page-cache-is-device-allocatable` (host page cache vs device alloc across MEMORIES, drop-deliberately — different axis from within-device pool partitioning), B139 (headroom admission). |

## Drops (the gate worked)

- **Band/compaction arena machinery** (end bands, floating bands, frontier/holes snapshot, the three-level no-move/compact/relayout ladder) — allocator architecture; no bounded per-site fix-space. Lane intel for 1201, not a rule.
- **Feasibility-vs-realization separation; virtual-id → physical-page remapping; PP stage capacity simulation** — serving-runtime design taxonomy, no Rust-shaped rule.
- **Page-use ledger + HiCache/Mooncake/PD ordering** — feature architecture of their stack (the ordering RULES are R2's law; the ledger plumbing is not).
- **Results-section numbers** — measurement provenance only (H200, their models, two-run averages): never bars; and their own headline honesty ("No regression at defaults") says the win lives under wrong/no ratios, not at tuned defaults — cite with the disclosure, per the source-shelf law.

## GOAT gate

- R1 passes all four: benchable on the alloc axis; exact semantics (pure layout, bit-identical); mechanism-level magnitude (N+1 allocations → 1; one bulk move vs N); deterministic.
- R2/R3 fail criterion 1 for a DOMAIN reason (capacity/admission-under-overlap metrics live in serving runtimes no validator hosts) → first-class suggestion-only per the Batch-136 precedent, NOT a demotion.
- R4 fails criterion 1 pattern-side (architecture) → face-or-drop, declared.

## Dedup evidence (greps this session, both vocabularies, hits read)

- `arena|sub.?pool|partition|sliding.window|memory.ratio` over `src/domains/rust_perf/` → hits ruled: `cache-line-block-partitioned-lock` (TRUE-sharing lock granularity — partition means something else), `partial-reset-watermark-pool` (reset cost), `align-64-false-sharing` (padding). Zero owners of pool-partition/fungible-capacity.
- `epoch|logical free|pending|deferred|quiescent|in.?flight reader` → hits are papaya **cost annotations** (entries_01/03 — "papaya epoch GC overhead dominates without concurrent access"), `deferred-command-flush-hoist` (barrier batching), `streaming-prefix-holdback` (pattern-sized holdback). **Zero owners of deferred-reuse/epoch-lifecycle as a law.**
- `admission|joint demand|free.?bytes|free.?list|reserve.?then.?commit|capacity check` → B139, B145 (`mincore-probe-routes-gather-arm`), B274, B324 — all single-resource/single-pool axes. Zero owners of cross-pool joint admission.
- kernel_opt `arena|band|compact|strided view|per.token entry` → `split-decay-outputs-eliminate-readback`, `carry-dual-layout-dead-slot`, `attention_combined_kv`, `repeat-axis-aliased-index-arena` + `ascending-tile-arena-address-is-loop-index` (GPU tile arenas — host-side distinct), `launch-cost-scales-with-allocation-count-pack-one-arena` (dispatch-count axis, B-relax-adjacent — already the relax verdict's kin). R1 adjudicates against the first three in-rule.
- Queue snapshot: the one prior SGLang-adjacent staging (OpenRSI's `request-invariant-encoding-memoized-across-requests`) is a serving-cache axis — distinct. No sibling has distilled THIS blog (the muse-mega verdict, Research 398, names zero memory-lane candidates).

## Lane mapping (mandatory axis)

- **Mapped lane: riir-infer Issue 1201 (+ its open remainder 1203) — FlashStates-class multi-session hybrid state serving, over 742 T4's `qwen38_prefix_cache` (whole-prefix KV+GDN checkpoints, shipped single-stream).** The issue's own Why names "vLLM + SGLang — both perf-rematch opponents — measurably collapse on concurrent agentic hybrid workloads". This blog is SGLang's answer to exactly that problem for hybrid models (full KV + window KV + recurrent state under concurrency): one arena (no partition knob), joint admission, demand-driven capacity, overlap/graph-safe page lifecycle.
- Transferable design feeds (named, not executed — the lane's own order): (a) **joint-demand admission pricing** (window pages + state slots together, page rounding included) — the multi-session admission shape 1201 needs; (b) **logical-free/physical-reuse-waits + move-with-old-reader copy-remap** — the CUDA-graph-safe page-move law 742 T4 requires (graph replay = the old reader); (c) **demand-driven fungible capacity vs a fixed retention budget** — 1201 T1 pre-registered a pinned 25%-of-demand retention budget; the blog's stranding datum is the evidence the budget must stay a target, never a wall, if the cell shows stranding; (d) **per-token entry packing** — the page layout for RadixPrefixTree chunk pages carrying heterogeneous per-layer rows.
- NOT transferable (closes): every H200/their-model number = provenance, never headroom (absolute-numbers caveat; the league owns our-box comparisons, and memory partitioning is not a league axis — league rows are single-stream prefill/decode tok/s); the overlap-scheduler stream mechanics and Mooncake/PD/HiCache integrations are their stack; the compaction/band allocator is a full-allocator rewrite our single-stream lanes do not need today; the partition problem only binds at 1201's multi-session posture.
- Follow-up: **mail the pointer to the 1201/1203 lane holder** (lane live: T2 code landed `8e64a036` 2026-11-11; cells pending per `.issues/1203` — "m3 stalls cache-free; amo queued"), offering the blog as the admission/lifecycle reference design with the four feeds above. No new issue filed — 1201 owns the beat (the splash/OpenRSI verdict discipline).

## Product coverage (mandatory axis)

**YES** — seeds the **deferred-reuse/epoch-lifecycle concurrency family** (0 owners; the famous-lib pool: crossbeam-epoch source + guide, papaya internals — our own house map is epoch-based, so the rule fires at home conceptually) and opens a **serving capacity-integrity lane** beside the B139 admission family. Follow-up source pool (accumulation clause per the dial9 precedent — pool sources read before the batch window fold into the same batch): vLLM PagedAttention (the pool-partition ancestor), SGLang design docs (the usage/design guides the blog names), crossbeam-epoch's guide.

## Yield estimate

2–3 rules after ~50% falsification (R1 near-certain; R2/R3 likely; R4 the swing). Nominal Batch 355.

## Honest caveats

- Graded from BLOG PROSE; the sglang Python implementation unread (no clone, zero-cargo verdict lane). Plan 365 T0 optionally clones at the results pin for code-level admission shapes; fixtures remain original Rust regardless.
- The feature is EXPERIMENTAL (their own limitations: priority preemption disabled, window-rows-without-checkpoint overhang, startup refusals, `--max-running-requests` still fixed) — transcribe the laws, never the maturity.
- Sibling queue-row gap disclosed, not repaired: Research 395/397/398 + plans 362/363/364 (Batches 352–354) landed without queue-snapshot rows — this file's previous last section was the relax amendment. Recorded here for the doc-sync pass; their lanes own the repair.
- Sibling WIP on this checkout (untracked `.benchmarks/128_mozilla_rustperf_day2/` run dirs) untouched; this commit stages only its own three files.

## Recording

Plan 365 (staged plan-first per the law) carries the batch tasks; the cargo half awaits a free window — the claim row lands when it starts, the push is the arbiter.

Session: zed-amo-unified-memory-distill, epoch 1794487400
