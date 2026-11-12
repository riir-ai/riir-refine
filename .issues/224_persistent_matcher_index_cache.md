# Issue 224 — the matcher index rebuilds on EVERY run: ~1.3 s of a 1.37 s warm run (95%)
**Status:** open — T1–T4 DONE: `index_cache` PROMOTED to the release set 2026-11-11 via issue 226 (G3 restated PASS +1.4%); T5 pending the release that ships it. Was: T1–T3 LANDED; T4 verdict (b): promotion BLOCKED on issue 226 (the G3 redesign) — delegated-owner verdict 2026-11-11, session 98784e41; T5 pending the release that ships the promotion. — 2026-11-11: the P2b blocker was DISCHARGED (the vendored ragplane seam landed, the modelless matcher core is the default build's path) — T1–T4 unblocked and executed.
**Claimed:** zed-amo-b352-lone, 1794516560, expires 2026-11-15T20:56:00+00:00
Takeover of the expired zed-amo-i224 lease (exp 2026-11-13T04:33Z, no Lease-closed row; T5 stays release-gated). Scope THIS lease: the maintained-binary rebuild at current HEAD (the T5-adjacent bin-age debt — the installed binary's build hash is 4 commits behind develop; the feature-set staleness the 2026-11-12 finding named is ALREADY repaired, release set: complete).

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
- [x] T1 — content-addressed on-disk index cache — **LANDED 2026-11-11** (opt-in `index_cache`): key = blake3(resolved per-domain corpus entries + retrieval config + embedder model id + cache/ranking/bm25 format versions — the leased epoch NUMBER deliberately excluded: identical content under a fresh lease hits); value = per-domain snapshots (rows + zone map + rule ids + the ndb Bm25Index snapshot + anchor mean); `$XDG_CACHE_HOME/riir-refine/index/<key>.idx` (env `RIIR_REFINE_INDEX_CACHE_DIR`); atomic tmp+rename; LRU keeps 8 by mtime with touch-on-hit. Upstream primitive FIRST: ndb `Bm25Index::to_snapshot`/`restore_snapshot` (`7e98f4a` + the clippy-style fixes `bcd2c79`/`658b3f5`, digest-before-parse + full structural revalidation; re-vendored through the sanctioned lane, `--check` green @`658b3f5`). Disarms under `kernel_route_narrow` + the Bonsai dense posture.
- [x] T2 — corrupt/partial entry refuses LOUD and rebuilds — **LANDED**: `LoadOutcome::Corrupt(msg)` prints `[index-cache] corrupt entry <path>: <reason> — rebuilding`; the digest verifies BEFORE any parse, then counts plausibility-capped + structurally revalidated (versions, config drift, domain-set match, duplicate tags, trailing bytes — 6 gate tests incl. six corruption arms).
- [x] T3 — per-user epoch store for the auto-corpus lease — **LANDED** (rides `index_cache`): the 12 h attempt stamp lives at `<cache>/riir-refine/corpus_auto.json`; crate `.refine/` keeps only crate-local state; ungated fallback byte-identical.
- [-] T4 — GOAT gate — **executed, verdict (b): keep opt-in**; promotion rides issue 226. G1 PASS (gate test: restored-vs-built suggest_span Debug-exact across cold/hit/wiped 6-domain runs; binary-level output diff identical after filtering the ON binary's own disclosure lines). G2 PASS (warm "leased rules→scan root" gap **118–120 ms** vs OFF 273–296, bar ≤150; warm total 171–174 vs 326–355 ms — ~2×). G4 PASS (18.8 MiB entry disclosed on store/hit; +85 KiB binary). G3 FAIL AS WRITTEN (cold 573–578 ms vs OFF ~340 = **+69%** — the one-time 18.8 MiB store costs ~233 ms on the exit path; ~2 misses/day at the 12 h window vs −170 ms every warm run). The delegated-owner verdict restated G3 (BEFORE re-measurement) as "time until results printed ≤ +5%" and directed the store-after-print + cheaper-write + size-cap redesign — filed as issue 226, the promotion blocker. Env law ready: `RIIR_REFINE_INDEX_CACHE=0` the exact-literal demote (DEMOTE-ONLY). Gates: default lib 3848/0 + clippy −D clean (byte-identical posture — the cache code compiles to nothing), ON lib 3854/0 (6 new tests), clippy −D ×2 postures (lib + all-targets), ragplane `--check` green. Numbers at HEAD `e8650c18`+tree, reproduced post-rebase at `c13513b5`+tree (amo QUIET AC, load 3.7–6.1/30, release binaries).
- [ ] T5 — re-capture the riir-dapps hero timeline (`gen_refine_timeline.py`) after the release that ships the promotion.

## Sequencing
Issue 221 P2b (claimed, `zed-352d06e6-m3-221`) replaces `NeuronDbRetriever`/KNN
with the vendored ragplane exact-cosine + BM25 retriever. Cache THAT index; starting
T1 before P2b lands would serialize a structure that is about to retire.

**T5-adjacent finding 2026-11-12 (zed-m3 housekeeping lane, READ-ONLY observation — no lease touched):** the maintained release binary at HEAD `3c7090378` (rebuilt 22:48 at the Batch-356 landing, `build: 3c709037881b`) was built WITHOUT `index_cache` in its `--features` list — `--version` prints `release set: STALE — missing index_cache` and the binary's feature stamp carries no index_cache env surface. The tree at that build commit DID carry index_cache in both `RELEASE_FEATURES` and `release.yml` (the 226 promotion `09ee1a61` is an ancestor), so this is a **feature-list omission at rebuild time, not a tree gap** — the rebuild used the pre-226 feature list. Class: the Issue-153/B248 bin-age law's feature-axis sibling (a hash-fresh binary can still serve a stale FEATURE SET). Fix: rebuild with the stamped line (`cargo build --release --features latent_retrieval,points,net-mainnet,fix_verify,update_check,kernel_pregate,shader_perf,span_shape_dispatch,index_cache --bin cargo-refine`) — owed by the i224 lane (this issue's holder) or the next maintained-binary lane; NOT executed here (the 224 lease is live, zed-amo-i224 exp 2026-11-13T04:33Z). T5 (the post-release hero-timeline re-capture) remains blocked on the release that ships the promotion — note the current binary does not serve the promoted lane, so the re-capture before a correct rebuild would measure the OLD posture.

**RESOLUTION 2026-11-13 (zed-amo-b352-lone, the lease takeover):** the feature-set half of the finding was already repaired by the 2026-11-12 23:16 rebuild (`build: 9ab640dc5dda`, `release set: complete` — verified before this lane's work). This lane rebuilt the remaining AXIS — the build hash was 4 commits behind develop (b358 + the B352 landing + fix(230) since `9ab640dc`): rebuilt at HEAD `55373f95` with the stamped set, installed via the `~/.cargo/bin/cargo-refine` symlink, `--version` verifies `build: 55373f95513d` (== HEAD, no -dirty) + `release set: complete` + no Homebrew shadow (`which -a` resolves only `~/.cargo/bin`). T5 stays release-gated (untouched, as the finding scoped).
