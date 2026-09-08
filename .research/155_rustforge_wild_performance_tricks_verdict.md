# Research 155 — Distill-source verdict: the Wild-linker "RustForge wild performance tricks" article

**Status:** RECORD — the owner directed mining as the e2e contribution test; the batch LANDED as Batch 135 (Plan 114, rust_perf 114 → 117). Born-complete; pending work lives in Plan 114's ledger, not here.

## Source

- URL: https://davidlattimore.github.io/posts/2025/09/02/rustforge-wild-performance-tricks.html
- Author: David Lattimore — the Wild linker author; the six techniques are distilled from Wild's production code.
- Date: 2025-09-02. Inspected: 2026-09-09 (full article text supplied with the verdict request; quotes below cite the article's technique descriptions, re-verified against that text at write time).
- Provenance chain: article ← Wild linker (production, shipped). No repo clone needed — the article carries its own snippets + x86 asm; there is no `.raw/` entry and no sha to pin. License: a blog post with no explicit OSS license — **cite-shape only; every corpus fixture is an original reproduction** (the Girder BSL discipline, Batch 131).

## The six candidates, the dedup gate, and the grades

Dedup greps ran over `riir-clippy/src/` under BOTH vocabularies (the source's terms AND the corpus's operator-name variants). Zero hits under both = pass; a single hit = a LEAD that was read before any dup call.

| # | Candidate | Grade | Verdict | Dedup evidence |
|---|---|---|---|---|
| 1 | Mutable slicing for sharing between threads (contiguous per-object id ranges → per-thread non-overlapping `&mut [SymbolId]`; article uses nightly-shaped `split_off_mut`, the portable core is `split_at_mut`/`chunks_mut` partition) | **B−** | **LAND** as `partitioned-mut-slices-zero-sync` | `split_off_mut|split_at_mut|par_bridge` → 0 hits. Leads read: `cache-line-block-partitioned-lock` (B78) partitions LOCK GRANULARITY — this eliminates locks entirely via disjoint ownership; `parallel-disjoint-pread-hides-cold-page-latency` (B118) is io positional reads, not shared-memory mutation. Distinct axes, both compose. |
| 2 | Parallel initialisation of the Vec (`sharded-vec-writer`, the author's own crate: per-object shards, `return_shards` completeness check) | **C** | **DROP** (lead recorded) | `sharded-vec|sharded` → 0 hits in rust_perf. Killed on two grounds: the fix's core is a third-party crate suggestion (`// [dep: ...]` harness convention exists, but the pattern without the crate collapses into candidate 1's partition + `vec-with-capacity` — composition, not a new decision content); and "add NO dependencies" holds for this batch. The completeness-check idea (verify every byte was written exactly once) is recorded here as a lead for a future correctness-domain intake. |
| 3 | Atomic ↔ non-atomic in-place conversion, zero cost (same size+align ⇒ std's Vec allocation-reuse makes `into_iter().map(Atomic::new).collect()` compile to nothing — the article shows the movups/no-loop asm; back-conversion consumes via `into_inner`, never loads; `mem::take` at the phase boundaries) | **B+** | **LAND** as `atomic-vec-zero-cost-convert` — the GOAT lead | `AtomicU32|into_inner|atomic.*convert` → only `mutex-to-atomic` (B-era) which is the OPPOSITE direction (Mutex → atomic as the permanent storage); `mem::take` → the `mem-take` rule (clone+clear vs move) — adjacent idiom, different mechanism, cited as the entry/exit boundary arm. Zero coverage of representation-SWITCHING. |
| 4 | Buffer reuse across a lifetime change (`reuse_vec<T,U>`: const size/align assert makes incompatible T/U a COMPILE error at the call site; `clear()` makes the map body unreachable so the loop compiles out; converts `Vec<&'a T>` → `Vec<&'static T>` reusing the heap allocation) | **B** | **LAND** as `vec-reuse-across-lifetime-change` | `reuse` → `reuse-allocation-clear` (LEAD, read: caller-passed `&mut Vec<T>` scratch, clear+push — NO lifetime/type retargeting; the obstacle it removes is "the compiler can't hoist", not "the borrow checker can't type the hoisted buffer"). `hoist` → `debug-assert-length-hoist`, `regex-hoist-lazylock`, `dispatch-handler-pair-hoist` — none about buffer lifetimes. `mem::take` → adjacent. The novel core is exactly the lifetime-stripping dimension the dedup gate was told to check hard; it survives. |
| 5 | Deallocation on a separate thread (`rayon::spawn(move || drop(buffer))` for LARGE buffers — the article itself gates on size + notes spawn allocates) | **C−** | **DROP** (lead recorded) | `rayon` → 0 hits; `thread::spawn` → harness internals only. Killed: the win is allocator- and size-dependent (macOS libmalloc vs mimalloc diverge hugely on large-free cost), the trigger needs a size threshold the healer cannot measure, and no deterministic bench verdict exists in the harness. The DECISION CONTENT ("the free of a huge buffer runs on the hot thread") is real — recorded as a lead for a measured-GOAT intake if we ever instrument our own large-buffer frees. |
| 6 | MaybeUninit shadow struct for non-trivial-Drop types (`Foo<'a> {owned, borrowed}` → shadow with `MaybeUninit<&'static str>` so the Vec crosses threads) | **C+** | **DROP** (recorded as rule 4's extension arm) | `MaybeUninit` → 0 hits. Killed: the fix-space has an unsound-under-observation edge (the article's own caveat: on panic/early-return the shadowed field is left empty — acceptable only when the value is dead), the pattern is niche (only types that mix owned + borrowed fields), and the bounded fix is rule 4's mechanism plus one field-wise map. Folding it into rule 4's `context_notes` as the named extension keeps the knowledge without a weak standalone rule. |

## GOAT candidate: `atomic-vec-zero-cost-convert`

1. **Benchable** on the rust_perf Pareto validator + exact alloc axis: buggy arm converts from a borrowed `Vec<u32>` (fresh allocation + element copy loop — REQUIRED, the source still lives); fixed arm converts an owned vec via `into_iter().map(AtomicU32::new).collect()` (allocation-reuse, the loop compiles out per the article's asm). A shared `clone()` in both arms keeps the timing comparison honest while making the alloc axis EXACT: 2 → 1 allocations steady-state.
2. **Exact semantics**: the conversion is a representation change over identical bytes — outputs bit-identical by construction; G1 parity gate `batch135_fixtures_g1_parity` pins it.
3. **Magnitude story**: one malloc + one n-element copy eliminated per conversion; at the phase-boundary usage the article targets (switch representations per phase instead of paying atomics everywhere or transmuting), the conversion cost goes from O(n) + alloc to ~zero.
4. **Deterministic**: no threads, no locks, no timing-dependent verdict path; alloc integers exact.

`vec-reuse-across-lifetime-change` and `partitioned-mut-slices-zero-sync` ride the same gate shape (alloc elision 1→0; contended-mutex vs zero-sync partition, alloc axis EXEMPT per the Batch-78 probe class).

## Lane-intel axis: does NOT fire

The axis asks whether the source moves an open league lane (M3 decode/prefill, quant recipes, kernel families). This is host-side build/run-time Rust (a linker's symbol-resolution phases) — no GPU kernel, no serving recipe, no quant surface, and the league's build tooling lane does not exist as an open issue. No league/lane line is produced; the corpus verdict stands alone. (Checked explicitly per the 2026-09-05 qwen38-recipe-miss rule, not skipped.)

## Domain routing

`rust_perf` — categories: concurrency (rules 3, 5-family) + allocation (rule 4). The tempting WRONG domain: **`clippy_lints`** — the transforms look like local structural rewrites (a `.collect()` reshuffle, a helper fn), and a parse-only proposer can match their shapes; but the rules are REGIME rules (when the conversion is free, when the partition is legal, when the retarget is sound) — exactly the content clippy_lints' "parse-only provable lint shapes" tier rejects and rust_perf's regime-annotated entries exist for. NOT `kernel_opt` (no GPU surface — the CPU-SIMD-vs-kernel_opt trap's mirror).

## Yield estimate

Six candidates → 3 rules landed (50% — exactly the coordinator-falsification band the skill warns about: candidate 2 was killed on the dependency ground, candidate 5 on measurement-determinism, candidate 6 on fix-space soundness). No rider surface (the article's techniques are self-contained; no existing rule needed a new face — checked against the nearest neighbours above).

## Honest caveats

- **Provenance is a blog post, not a repo**: no pin sha exists; the article text was supplied with the request and quotes are cite-shape only. The techniques originate in Wild (production) so the mechanism claims are load-tested, but the two asm-backed claims (conversion loop vanishes; reuse_vec loop vanishes) are re-verified for OUR fixtures by the Pareto benches + alloc axis, not taken from the article.
- **`split_off_mut` is nightly-shaped** in the article; the landed rule's fix is the portable `chunks_mut`/`split_at_mut` partition. The rule text says so.
- **The reuse_vec 'static element type is a caller contract**, not compiler-enforced: the buffer must be cleared (or dropped) before any referent dies, and a panic between `take` and restore loses the buffer (frees it — never UB). The fixture makes this sound mechanically (`Box::leak` rows) so the harness needs no contract; the rule text carries the real-world contract. The article states the same panic caveat for its MaybeUninit variant.
- **Rule 5-family (defer-drop) was not measured** — dropped, not annotated-measured. If a future batch instruments large-buffer frees (riir-ai prefill buffers are the natural site), the lead is this note + §candidate 5.
- The corpus's own benchable/not-benchable accounting had drifted (header said 25 NotBenchable, the gated test said 14, reality was 27 — the B118–B129 io annotations were never counted anywhere). Batch 135 repairs all three sites to 27. Recorded here so the next audit reads the repair as a repair, not a regression.

## Queue slot

Batch 135. Plan: `.plans/114_batch135_rustforge_wild_tricks.md`. Queue line: `.distill/001_mining_queue_snapshot.md` (same commit).
