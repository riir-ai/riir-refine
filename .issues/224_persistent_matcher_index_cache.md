# Issue 224 — the matcher index rebuilds on EVERY run: ~1.3 s of a 1.37 s warm run (95%)
**Status:** open — measured, unclaimed. Sequenced AFTER Issue 221 P2b (the retriever this caches is being replaced; cache the post-P2b index, never the retiring KNN). — 2026-11-11: the P2b blocker is DISCHARGED (the vendored ragplane seam landed, the modelless matcher core is the default build's path, lib.rs P2b rows verified) — T1–T4 unblocked.
**Claimed:** zed-amo-i224, 1794371604, expires 2026-11-13T04:33:00+00:00

## Measurement (release binary, quiet rig)

**— 2026-11-11 UPDATE (zed-amo-i224, the attribution + root-cause fix):** the
phase instrument (`RIIR_REFINE_MATCHER_TIMING=1`, landed `7227dd56`) split the
build: **kernel bm25+insert 862–1008 ms of the ~1.3 s build (75%)**, perf
201–223 ms, embed ~13 ms TOTAL (1%), router ~21 ms. The cost was never the
embedding — it was `Bm25Index::index`'s per-document postings update, which
CLONES each affected term's full posting list (papaya values are replaced, not
mutated in place, on the 0.2 line) — quadratic for common terms (`fn`, `self`
in thousands of the 17,552 kernel rows = gigabytes of Posting memcpys).
ROOT CAUSE FIXED upstream: riir-neuron-db `90ab1ba` adds `index_batch` (local
std prebuild with O(1) pushes + one vacant insert per term; within-batch dups
replay remove semantics; non-fresh indexes fall back to sequential; postings
stay doc-ordered — bit-identity test `g1_index_batch_is_bit_identical_to_sequential_index`);
refine `f325d1ec` re-vendors @90ab1ba + switches the seam to ONE batch call.
Measured after: kernel 123–142 ms, perf 59–75 ms, warm e2e **1.37 s → 0.34 s
(~4×)**. Gates: ragplane pins 7/7, vendored tests 31/31, retrieval floors
29/0, latent_retrieval 3982/0, default lib 3842/0, clippy −D ×3 postures,
ndb lib 476/0. The matcher build now stands at ~250 ms of the 340 ms warm run
— T1's cache targets the REMAINING rebuild: a snapshot/restore surface on the
batch primitive (deserialize + vacant-install ≈ tens of ms) can take warm
setup under the 150 ms G2 bar; the bar gets re-adjudicated at the T4 verdict
with these numbers as the new floor.

Original measurement (pre-fix, kept for the record):
`cargo-refine 0.5.0` (release tarball, sha256 `c3ebca55…` verified), sample crate =
riir-dapps `cloudflare/kat-service/scripts/sample_crate` (2 source files, 5 clippy
shapes), amo (Apple M5 Ultra, AC, load 1.5–1.8/30, exclusive rig-queue lane),
2026-11-10. Line arrival times (ms from spawn), dry run, same crate + same HOME:

| run | auto-corpus fetch | `corpus: leased rules` → `scan root` | lint | total |
|---|---|---|---|---|
| 1 (first) | 0 → 1265 | 1266 → 2617 (**1351**) | 176 | 2822 |
| 2 | skipped (12 h window) | 4 → 1304 (**1300**) | 82 | 1415 |
| 3 | skipped | 4 → 1308 (**1304**) | 32 (oracle cache HIT) | 1371 |

The gap is `LatentMatcher::from_enabled_domains_with_overlay` (`src/bin/heal.rs`
~15383): the whole multi-domain retrieval index is rebuilt from the corpus every
process. Bench 132 already saw it ("Matcher build (once per process) … boot-time,
never per scan. Disclosed.") — but a CLI boots per invocation, so boot = per run.
Every other phase is already cached (the clippy oracle hit drops lint to 32 ms).
On the refine.riir.ai hero timeline (riir-dapps Issue 164) this is the "load rules"
phase: 56% of a first run, 95% of a warm one.

Second, smaller finding: the 12 h auto-corpus window + lease store live in the
CRATE's `.refine/` (`maybe_auto_corpus`, `stamp_path = root/.refine/corpus_auto.json`),
so every NEW crate re-fetches the corpus (~0.6–1.3 s network) even when the machine
already holds the current epoch. Per-user (XDG cache) epoch storage would make
that a stat.

## Proposal
- [ ] T1 — content-addressed on-disk index cache: key = blake3(corpus overlay bytes
      + leased epoch + embedder model id + compiled-domain set + index format
      version); value = the built index (fixed-size Pod rows, mmap/zero-copy read);
      location per-user (`$XDG_CACHE_HOME/riir-refine/index/<key>`), atomic
      write (tmp + rename), stale keys pruned LRU. Miss = today's build + write.
- [ ] T2 — corrupt/partial entry refuses LOUD and rebuilds (never a wrong index).
- [ ] T3 — per-user epoch store for the auto-corpus lease (crate `.refine/` keeps
      only the crate-local state); a new crate on a machine at the current epoch
      skips the fetch.
- [ ] T4 — GOAT gate behind `index_cache` (opt-in): G1 identical suggestions +
      fixes vs the rebuild path on the score-bench corpus (bit-identical rankings);
      G2 warm `cargo refine` on the sample crate ≤ 150 ms setup (from ~1300), measured
      `--release` on a QUIET rig via `agent-mail rig run --exclusive`; G3 cold run
      no slower than +5 %; G4 cache size reported. Promote to default if green.
- [ ] T5 — re-capture the riir-dapps hero timeline (`gen_refine_timeline.py`) after
      the release that ships it.

## Sequencing
Issue 221 P2b (claimed, `zed-352d06e6-m3-221`) replaces `NeuronDbRetriever`/KNN
with the vendored ragplane exact-cosine + BM25 retriever. Cache THAT index; starting
T1 before P2b lands would serialize a structure that is about to retire.
