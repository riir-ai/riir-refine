# Research 134 — online perf search: nnethercote July-2026 post (fresh source walk #8-analog)

**Status:** RECORD — one rule candidate SHORTLISTED for Batch 112 (rust_perf, layout family),
two external-confirmations queued as riders, one lane record, one ops note. Zero corpus delta
this unit (landing = Batch 112, next build window).

**LANDED 2026-09-05 as Batch 112 / Plan 083** (corpus 100 → 101, layout 9 → 10; riders
landed on `usize-index-to-u32` + `sort-key-cache-precompute-out-of-lock`). Mine-time
measured addendum: the threshold is ARCH-SHAPED — aarch64 INLINES a 136-byte by-value
move (ldp/stp pairs + an 8 B tail, asm-verified) while a 320-byte move emits `bl _memcpy`
(the ARM ceiling sits in (136, 320]); x86-64's 128 B ceiling is where the source's step
lives. M3 bench: Mixed 1/0/1 (the 1M tie is the honest mechanism cell at 136 B on ARM);
x86-64 re-gate DISCHARGED same-day on the 4090 — the asm shows the step directly
(48 B copy inlines; the 136-byte loop's hot loop carries `movl $136; callq memcpy`). The partitioned-visitor lane record (#17124) and the
XProtect ops note stand unchanged.

## Method

- Queue item 11 (online rust/kernel perf search); MCP `web_search_prime` quota dead until
  09-15 → `webReader` + `fetch` (r.jina.ai) route. Source: nnethercote.github.io (the
  perf-book author's blog — the queue's named source family, but the BLOG itself was never
  walked; Batches 48/53 mined the checklist/perf-book, not the posts).
- The feed returned the full recent archive. **The 2026-07-31 post ("How to speed up the
  Rust compiler in July 2026") post-dates every prior walk and is entirely undistilled.**
  The 2025-12-05 post's non-compiler sections (XProtect, -Zmacro-stats) were also unread.
  Everything older was checked against the corpus and dropped as already-covered doctrine.

## The July 2026 post — graded rows

| # | PR | Mechanism | Grade | Disposition |
|---|---|---|---|---|
| 1 | rust-lang/rust **#160005** | **LLVM lowers any by-value move of a >128-byte type to a `memcpy` CALL** (not inline stores). A hot 136-byte type paid ~half its compile time in memcpy; fix = (a) avoid 2/3 of the moves, (b) shrink the type to 104 bytes so the remaining moves inline → **40% wall-time** on the two worst crates. DHAT copy-profiling was the instrument that found it. | **A− source → Batch 112 seed** (rust_perf, layout) — the 128-byte threshold IS the decision content; benchable on both arches. See shortlist. |
| 2 | #158720 | AST expression nodes shrunk 72→64 bytes — "small enough to fit in a cache line"; >10% wall on AST-heavy benches, cache-miss −29%. (From 104 bytes years ago — "chip, chip, chipping".) | **RIDER (external confirmation)** on `usize-index-to-u32` (B61): the cache-line-boundary doctrine — struct shrink past a 64B boundary is measurably a step function, not a smooth curve. |
| 3 | #157179 | rustdoc sorted impls by LONG GENERATED HTML STRINGS; switched to a short text repr as the sort key → >6% icount. | **RIDER (external confirmation)** on `sort-key-cache-precompute-out-of-lock` (B73): never sort by expensive payloads; derive a cheap key. Third external instance (B73 already carries the polars + tantivy lineage). |
| 4 | clippy **#17124/#17132** | Clippy called every lint's `check_foo` visitor method per node; most are no-op default impls → empty-call elimination via pass combination → **10-30% runtime**, branch mispredictions −20-80%. nnethercote independently found + fixed the same thing days later (#157762, unmerged). | **Lane record (architectural in user code)** — the fix restructures visitor dispatch infra, no bounded site-level rewrite. In-corpus adjacency: `dyn-to-enum` (2.5-2.9x) covers the dynamic-dispatch half; the DISTINCT mechanism here is "partition a handler registry by which methods each handler actually implements" — a plausible future rust_perf rule if a real in-tree instance appears. Recorded for the post-mining watch. |
| 5 | #159091 | rustdoc added to the PGO training set → −2.85% mean rustdoc wall. "PGO is fiddly but the training set matters." | Lane record — env/build doctrine, no rule. |
| 6 | #158942, #159266 | AST node field removal / attribute repr overhaul — sub-1% icounts. | Too thin. |
| 7 | #153122, #153521, #154304, #157781, #158794, #159115 | Incremental-comp internal tweaks (dep-graph read dedup etc.) — rustc-internal. | No user fix-space. |
| 8 | #155678 | HIR query merge — internal. | No fix-space. |
| 9 | #142642 (Dec post) | rustdoc-JSON perf format draft. | Off-lane. |

## The Dec 2025 post — non-compiler rows

| # | Mechanism | Grade | Disposition |
|---|---|---|---|
| A | **macOS XProtect scans every freshly-built executable on first launch; each build script/test binary pays it; the scan daemon is single-threaded so parallel launches queue (one 3.88s build-script bar was mostly daemon wait). Fix: add the terminal app as a Developer Tool (System Settings) — rustc's own `tests/ui` suite went 9m42s → 3m33s.** | **OPS NOTE (owner act — a security trade-off; never execute unilaterally)** — our workspace runs MANY small test binaries per gate run (`test_gate.sh`, integration `[[test]]` targets, doctests); the effect compounds exactly like the cargo-nextest/Zed docs case. Record: if our gate wall-times look mysteriously inflated on the M3 with fresh binaries, this is the named suspect. |
| B | `-Zmacro-stats` (nightly) measures per-macro generated code; Bevy's `#[derive(Reflect)]` generated 1.5MB of redundant code; trimming the macro's output → −39% generated code, −16% cargo-check for bevy_ui. | Lane record + **candidate**: `neuron-db-sdk`'s derive macro (shard encode/decode) is our one first-party proc macro. A nightly `-Zmacro-stats` pass over ndb-sdk would price its codegen. Requires a nightly build (keep-quiet-box discipline defers it to the next build window; never a default-toolchain task). |
| C | `format_args!`/`fmt::Arguments` repr (Mara Bos) — rustc-internal. `VecCache` first-segment fast path — internal. `-Zhint-mostly-unused` — nightly flag, large-API-crate doctrine. | No user-code rules. |

## Shortlist → Batch 112

**Rule candidate: `hot-byval-move-over-memcpy-threshold`** (name TBD at authoring).

- **Trigger:** a type > 128 bytes moved by value in a hot path (fn args, returns, loop-carried
  assignment, `Vec<T>` element moves during sort/retain/growth) — LLVM lowers each move to a
  `memcpy` call instead of inline load/store pairs.
- **Fix space (two bounded forms + one architectural):**
  1. pass by reference / iterate by ref where the consumer only reads (mechanical, borrow-shaped —
     suggestion tier for a proposer; bench-first);
  2. shrink the type under 128 bytes (repack / `usize-index-to-u32` composition / box the cold
     tail — the `box-large-enum-variant` mechanics generalized to struct cold fields);
  3. avoid the move entirely (in-place construction) — architectural.
- **Bench:** 136B vs 104B struct move-loop + a by-ref arm; expect the source's 40% class on x86;
  **aarch64 unmeasured — the threshold is an LLVM backend constant (`MaxStoresPerMemcpy*`
  family) and may differ per target — cross-arch re-gate mandatory (the `powi-vs-powf`
  precedent; the corpus bench runs both arches anyway).**
- **Dedup:** grep-verified 2026-09-05 — no rust_perf rule covers by-value move size / the memcpy
  threshold (`box-large-enum-variant` is variant-packing; `narrow-counter-overflow-promotion` is
  u64→u32 counters; `smallvec-stack` is heap→stack for small N; none mention the move-cost axis).
- **Riders at landing:** row 2 (cache-line boundary) onto `usize-index-to-u32`; row 3 (short
  sort key) onto `sort-key-cache-precompute-out-of-lock`; row 4 recorded as the partitioned-
  visitor lane record (this file is the citation).

## Standing lessons

- The nnethercote BLOG (not just the perf-book/checklist) is a fresh, high-yield source —
  the July 2026 post alone seeded a rule. Next walk should re-check the feed after his next
  post (he announced he's seeking paid rustc work again; the series continues).
- webReader worked for the index page; the jina route (`r.jina.ai`) worked for the full
  archive in one fetch (webReader 500'd on the post URL). Both recorded for quota-dead windows.
