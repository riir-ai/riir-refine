# Research 290 — Batch 255: the llama.cpp #30637 MoE SLRU host-cache law

**Status:** RECORD (2026-10-19, shikuwa/4090 — the batch's distillation record;
quotes verified at the pin in-session)

Source: `ggml-org/llama.cpp` @
`33d6135cbe20e90e67d9db1c874fbb4a9dc64b19` — "moe-cache : replace MoE LRU
cache with a 50/50 SLRU cache" (#30637, MIT, Evan Huus, merged 2026-10-19
07:28:50 -0400; `src/llama-moe-cache.cpp` +60/−26, `src/llama-moe-cache.h`
+2/−2). Found by the standing watch: exactly ONE mechanism commit in the
window past the B254 pin `098ce526` — the B209/B210/B254 thin-watch-day
solo-batch precedent. Read via `git show` at the pin in the F:/llama.cpp
clone (the clone's HEAD has since moved to `d7fa69b7`; the pin is present
in its history and every quote below was read AT the pin).

## The law (one line)

A host-side recency cache holding EXPENSIVE entries (refill costs a
megabyte-scale upload) should be a 50/50 SLRU, not a plain LRU: first touch
lands in PROBATION, the SECOND touch promotes to PROTECTED, protected is
capped at half the cache with overflow demoting the protected-LRU head,
victims scan probation-head-first then protected while skipping every
expert the CURRENT request selected, hits are promoted BEFORE victims are
chosen for misses, and the no-victim case aborts loud.

## The verified quotes (verbatim at the pin; inner double-quotes rendered
as single quotes per house string-literal safety)

1. Header (llama-moe-cache.h): the cache "keeps recently and repeatedly
   used experts of host-resident MoE layers in a device buffer" (was "keeps
   the most recently used experts").
2. The struct comment (llama-moe-cache.cpp): "probation and protected LRU
   lists; hits promote slots to the protected half" over `int32_t head[2] =
   { -1, -1 };` + `int32_t tail[2] = { -1, -1 };` + `std::vector<bool>
   is_protected;` + `int32_t n_protected = 0;` — the single head/tail pair
   becomes TWO segments.
3. The mover: "move slot s to the most recently used end of a segment" —
   one `void move(int32_t s, bool protect)` serving both lists, tracking
   `n_protected += int(protect) - int(from);`.
4. The promotion: `void touch(int32_t s) { move(s, true); if (n_protected
   > n_slots/2) { move(head[1], false); } }` — second touch promotes; the
   50/50 cap is `n_slots/2`; overflow demotes the protected-LRU head.
5. The fill: the miss path ends `move(s, false);` where the old code ended
   `touch(s);` — FIRST TOUCH = PROBATION, never protected.
6. The victim: `int32_t victim(int32_t il) const` scanning `for (int
   segment = 0; segment < 2; ++segment) { for (int32_t s = head[segment];
   s >= 0; s = next[s])` — probation-head-FIRST then protected — with the
   request pin in the loop body: "keep every expert selected by this
   request, including earlier fills" guarding `if (key < 0 || key /
   n_expert != il || seen[key % n_expert] != seen_gen) { return s; }` (the
   pre-existing per-request `seen`/`seen_gen` generation set now consulted
   at victim selection; the old code read the bare `const int32_t s =
   head;`). Exhaustion falls to `GGML_ABORT('no SLRU victim');`.
7. The ordering: the promote-pass comment "promote hits before choosing
   victims for misses" (was "hits go to the tail first, so the head can be
   evicted below"); the plan doc "give a slot to each expert selected by
   ids in layer il, evicting from probation first" (was "the misses evict
   the least recently used experts").
8. The read path keeps: "large batches only read the cache, so the experts
   used in generation stay in it" — the two populations the policy
   separates (generation repeatedly uses a small set → promoted,
   protected; wide prefill sweeps are one-touch → probation).

## The four decision clauses

1. Promote on the SECOND touch, never the first — one use proves nothing
   about reuse.
2. Protected cap `n_slots/2`, overflow demotes the protected-LRU head —
   the cap keeps the protected half from becoming the whole cache.
3. Victims scan probation-head-first then protected, skipping the current
   request's own selections — a miss must not evict a slot this request
   still needs (the multi-layer `plan()` case: an earlier fill of THIS
   request sits in a list the scan reaches).
4. Promote hits BEFORE choosing victims; the impossible case aborts loud
   (`GGML_ABORT`, unreachable while the `uniq <= n_slots` admission holds —
   the seen-pinned slots cannot cover every slot while misses remain;
   traced and asserted in the fixture's structure).

## Why plain LRU loses (mechanism)

Recency is a poor frequency proxy when request shapes are bimodal. One
wide request sweeps n one-touch experts through a single list, and each
eviction removes the oldest repeatedly-used entry — the sweep recycles the
working set instead of its own tail. SLRU makes one-touch entries evict
among themselves. The fixture (both halves carry executable `#[test]`s,
EXECUTED once in a throwaway crate this session — the corpus parse gate
alone never runs them): 4-slot cache, fill [0,1,2,3], hot pair [0,1]
again, two sweeps of two — plain LRU answers the hot pair with TWO cold
misses afterward; SLRU answers with two hits (the sweeps never reached the
protected half).

## NotBenchable (honest)

Upstream ships no numbers. The win metric is host-resident expert-upload
traffic under interleaved request shapes — a synthetic A/B re-measures the
fixture's own eviction order against itself (the B136 decision-content
class; the B241 slot-state pair precedent).

## Routing + dedup

rust_perf, **caching** category (+1 → caching 23 by the corpus grep count;
the per-category prose is admitted stale-prone, the asserts are the
ledger). Dedup zero owners: greps `slru|probation|second.*touch|promote.*
protected|victim.*scan` return nothing across rust_perf + kernel_opt.
Distinct-from set (in-rule): B103 evict-oldest (the ANTI-LRU verdict for
zero-cost-rebuild contexts — the opposite regime), B241
empty-slot-first-victim-order (victim ORDER within one list), B210 #29887
(GPU write-path regime gate), B237 (capacity growth), kernel_opt B189
shape-bucketed-graph-cache-lru (LRU as a memory budget over keyed buckets),
B219 prompt-cache-rewind-states (budget-fitted history retention).

## Staged-not-converted (the record for the next slots)

- **TensorFold `9ccf4727` → `d1018b31` = +988 commits** (the 1.1.0/1.1.1
  waves). A staged CAMPAIGN, not one batch (the B219/B249 multi-slice
  precedent — one session one verify cycle would balloon). Named leads
  needing at-pin re-verification at the next slot: `018e73c6` digest-cache
  (bounded checksummed index keyed device/inode/size/mtime/ctime-ns —
  strongest candidate), `3c0d6d3c` GLM prefix-move-before-evict.
- bevy `85503941e` → `ddd047d56` (+8): mostly null; `c38fa547bf` #25419
  orphaned-entity-ID transform fix = watch-class correctness.
- wgpu `e084b66aa`, cubecl `2243f5a8`: deltas unexamined (open).
- uzu + mlxfast ls-remote timed out ×2 this session (network); mlxfast's
  weekly floor is ~10-24 anyway.

## Gate record

batch255 parse+needle gates 2/2 FIRST-RUN PASS; default lib 2698/0;
rust_perf_bench lib 2729/0 (6 ignored = the documented freeze-regen e2es);
clippy −D ×4 postures clean (default / all-features / no-default /
rust_perf, all --all-targets); retrieval floors HELD no re-pin (blended
125/141 top-1 ≥ 124, 131/141 top-3 ≥ 130, 133/141 top-5 ≥ 132); score-bench
run #190 appended (heal_rate 0.964, FLAT); freeze NOT owed (zero
clippy-corpus delta — the M4/M5 generations key on the clippy seed corpus,
not rust_perf); maintained binary rebuilt at the release set →
`~/.cargo/bin/cargo-refine.exe`, `--version` build `fbcb7bbf104c-dirty` ==
HEAD (dirty = the uncommitted batch content, resolves at the landing
commit).

## En-route (the pins doing their job + the hazards)

1. **STALE-PIN REPAIR (the batch's second yield): the `with_spec` pin read
   119 through Batch 250 slice 1** — its +1 rule (the AVX2 butterfly) is
   BENCHABLE and the pin was never re-pinned because that landing's gates
   ran the default lib and the test only compiles under `rust_perf_bench`
   (the B239/B207 pin-hiding class the message itself documents). PROVEN
   at HEAD pre-landing: the assert failed `got 120`. Re-pinned to the
   measured truth + B255's +0-benchable delta.
2. Two MORE annotated-count sites (1904 `forty_three_annotated_rules`,
   2055 `drafter_propagates`) surfaced on the first full rust_perf-feature
   lib run — both re-pinned 178→179.
3. **The CRLF hazard struck a SEVENTH time** — the claim-row edit lane
   rewrote `.distill/001_mining_queue_snapshot.md` to CRLF (755/753 diff);
   fixed-forward in the same amended commit (`fbcb7bbf`, byte-clean +2).
4. **The B182 write_file escape lesson re-fired**: the `Some(` paren of
   the context_notes was missing its closer (`Some("...".to_string(),`) —
   rustc reports it as an unclosed-delimiter error POINTING AT THE STRING,
   not at the paren; the misleading span cost four probe rounds. The
   lesson: an "unclosed string" rustc error on a scanner-verified string
   means look at the DELIMITERS AROUND it.
5. The fixture `#[test]`s were EXECUTED once in a throwaway crate before
   landing (the corpus never compiles NotBenchable fixtures) — both traces
   proven, not hand-waved.

## Queue after

L8 league-posture A/B stays owner-gated (StartLux resident to
2026-10-20T13:30Z; riir-train issue 635). B256 candidates: the TensorFold
+988 staged campaign (at-pin re-verification first), the wgpu/cubecl
unexamined deltas, uzu/mlxfast at their next poll. Dated leads: walk #15
~10-20 (M3 claim-first), ai-1017 ~10-21, mlxfast ~10-24, nnethercote
~11-01.
