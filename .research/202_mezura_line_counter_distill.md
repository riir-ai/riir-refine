# Mezura — fast line counter (rust_perf distill source verdict)

**Status:** RECORD — Batch 172 LANDED 2026-09-23 (4 rules + 2 riders + 1 drop; the landing record: HISTORY.md 2026-09-23 §Batch 172; T1+T2 pre-verified 2026-09-22, addendum below; `.raw/mezura` removed at landing)

- **Source:** https://github.com/subamanis/mezura @ `2dbacb756d2169d2239bcdfbe9391fcd2ed9389b` (post-3.2.0 HEAD, cloned shallow into `.raw/`, deleted after this commit; the 3.2.0 release sha per crates.io trustpub = `6577a40fd2b29e4bb9d1cfb02a316a1932fa1708`)
- **License:** MIT OR Apache-2.0 (fixtures must be ORIGINAL minimal reproductions — the cmpeq/movemask idioms are textbook; write our own AVX2/NEON arms)
- **Inspected:** 2026-09-22 (full-tree clone; `mezura-core/src/engine/{masks,consumer,file_parser}.rs` read; greps over the rest; see caveats)
- **What it is:** the fastest cloc-class line counter (measured 2.41× over scc, 4.80× over tokei on the Linux tree, Linux + Windows, linebench methodology — source-reported). Workspace: `mezura` CLI + `mezura-core` lib (5.8k LOC) + `mezura-mcp`. Edition 2024, thin LTO + CGU 1, deps: memchr, memmap2 (unix), crossbeam-deque, ignore, num_cpus.

## Verdict

**YES, B+ corpus / no lane mapping / no product-coverage fire.** Mine as **Batch 172** (rust_perf), lead with the dual-mask fused block scan.

**Domain routing:** `rust_perf` (simd + io + concurrency categories). NOT `kernel_opt` — zero GPU code; the CPU-SIMD-is-not-kernel_opt trap (this is host byte-scanning, both SIMD arms are `std::arch`).

**Lane-intel axis:** NO mapping — mezura is a file-walking line counter; the workspace's open perf lanes (prefill/decode vs llama.cpp, ANE, GPU kernel classes) are untouched by any mechanism here. Checked against the league scoreboard rows; nothing to file.

**Product-coverage axis:** no famous-lib family seeded (memchr/memmap2/crossbeam-deque/ignore are common deps but no tokio-class gap). The simd byte-scan family grows to its second rule; the io mmap family gains its admission rule (the family had prefault/mincore/pread/coalesce — not map-vs-read).

## Ranked candidates

| # | pattern | source | grade | notes |
|---|---|---|---|---|
| 1 | **dual-mask fused block scan** — one 64B SIMD pass yields BOTH the newline bitmask AND the any-of-needle-set candidate bitmask (AVX2 2×32 cmpeq+movemask; NEON 4×16 with the AND-powers-of-2 + vpaddq bit-gather; scalar fallback shares the contract; feature detect cached at finder build). Line-splitting + candidate scan read off the masks — no search per line. | `mezura-core/src/engine/masks.rs` (257 L) | **A−** | **GOAT lead.** Benchable hermetic (in-memory buffer, both arches — the transposed-weight/unaligned-word cross-arch precedents); bit-identical positions (integer mask math); magnitude: per-line shape touches bytes twice (split + search) + per-line call overhead vs one cmpeq-per-needle-byte + one newline cmpeq per block; suggest-only fix class (hand-SIMD, not a bounded autofix). |
| 2 | **anchor-byte greedy set-cover for multi-needle scans** — choose the FEWEST finding bytes that reach every delimiter symbol (greedy max-cover; letters disfavored — "visited on every word"), anchor multi-byte symbols at an offset BEHIND the found byte (`r#"` found by its quote), pack into memchr2/memchr3 single passes. | `file_parser.rs` L304–444 (`ScanPlan`, `anchors_of`) | B | Dedup-adjacent to `byte-find-position-to-memchr` (B67 — its notes already name the "2/3-needle variants for `\|\|` chains" ladder). Distinct buggy shape: code already scanning per-needle (or scanning first-bytes naively); the cover-SELECTION is the novel half. Risk: coordinator may fold as a B67 rider — argue the selection algorithm, not the primitive. |
| 3 | **mmap-settled-size-gated whole-file read** — unix: map when size ≥ 256 KiB AND mtime age ≥ 5 s (the settle guard avoids the concurrent-truncation SIGBUS; "ripgrep's memory maps carry the same exposure"), else read into the reusable buffer; read protocol uses the expected+1 sentinel (asking one byte past the listed size distinguishes exact-size from grown; sized-on-fill only when the first 64 KiB window fills; stat-elision priced honestly — a partial short read on a failing disk is counted, unreported). | `file_parser.rs` L178–254 (`HeldFile`, `read_file_into`, `is_settled`) | B− | io family grows. Honestly NotBenchable (page-cache state dominates any microbench — the corpus's own io family says so three times: prefault/mincore/pread). The settle guard + the expected+1 protocol are the novel load-bearing halves; the map-vs-read threshold itself is folklore both ways. |
| 4 | **steal-batch-not-per-item** — crossbeam `Injector` per-item steals contend (`Steal::Retry` → `yield_now` = a scheduling round per file) → `steal_batch_and_pop` (half of what's left, so the tail still spreads). | `consumer.rs` L64–73 | C+ | Zero corpus hits for steal_batch/Injector (channel rules exist — different mechanism: deque contention vs park/unpark). Niche dep; NotBenchable (contention timing). |
| 5 | **io-bound consumer oversubscription** — consumers deliberately ≫ cores ("a consumer spends most of its life waiting for a file to open; speed comes from how many reads are in flight"); idle backoff ladder yield×10 → 2 ms sleep. | README §Threads + `consumer.rs` | C+ | Decision-content only; NotBenchable (same platform-nondeterministic class as `batch-blocking-pool-segment`, which it is adjacent to — sizing vs batching). |
| 6 | **scratch-capacity clamp** — reuse the file buffer across files but drop it when retained capacity exceeds 4 MiB (one huge file must not pin 4 MiB+ per thread forever); initial 150 B. | `consumer.rs` L160–162 | C | Third policy beside `reuse-allocation-clear` (Batch base) and kernel_opt's grow-only staging (opposite policy). Metric mismatch (retained memory is not a bench axis) — likely an annotation/rider, not standalone. |
| D | runtime-detect-once · tz-mask consumption · unaligned word loads · per-thread local stats + single merge · ascii→`from_utf8_unchecked` · bounded-head skip heuristics (512 B markers / 8 KiB identification) · phase telemetry · themes/render | various | DROP | Positive instances of B74 (`runtime-feature-detect-per-chunk`), B66 (`sparse-set-bit-iteration-tz-blsr`), B69 (`unaligned-word-load-from-bytes`), B135-family (`partitioned-mut-slices-zero-sync`); unsafe-laden with thin win; ambient structure. |

## Dedup-grep evidence (both vocabularies, `riir-clippy/src/`)

- `movemask|bit.?gather|cmpeq_epi8` → 0 rule hits (matches only inside B67's memchr explanation text) — candidate 1 PASS.
- mmap family (`prefault-admission-keyed-to-ram-headroom` B139, `mincore-probe-routes-gather-arm` B145, `parallel-disjoint-pread-hides-cold-page-latency` B118, `batch-pool-read-makes-serial-readahead-redundant` B123) — none is map-vs-read ADMISSION; candidate 3 PASS (distinct axis, distinct-from notes owed to all four).
- `steal_batch|crossbeam|Injector|work.?steal` → 0 hits (channel rules only) — candidate 4 PASS.
- `oversubscri|num_cpus|io.?bound` → 0 rule hits (1 kernel_opt spin-barrier TEST, different subject) — candidate 5 PASS.
- `capacity.?clamp|retained|scratch` → `reuse-allocation-clear` exists (ADJACENT — read: clear-reuse is alloc-per-call→reuse; the clamp bounds RETENTION on that reuse — same family, distinct refinement; fold as rider unless the coordinator wants it standalone).
- memchr → `byte-find-position-to-memchr` B67 exists; its notes name the 2/3-needle arity ladder — candidate 2 ADJACENT/DUP-RISK (the anchor-cover selection is the argued-distinct half).

## GOAT check (candidate 1)

1. Benchable: YES — Pareto validator, hermetic in-memory bench, sizes 4 KiB–1 MiB source-like synthetic buffer, dual-arch (M3 aarch64 NEON arm + 4090 x86_64 AVX2 arm).
2. Exact semantics: YES — same line count + same candidate positions, bit-identical (integer/mask math only).
3. Magnitude: mechanism-level — bytes touched once vs twice + per-line call overhead eliminated; multi-x expected on the scan phase. (Mezura's 2.41×-over-scc is e2e and corroborates the TECHNIQUE FAMILY, not this rule alone — honest.)
4. Deterministic: YES.

All four pass. Suggest-only fix class (hand-SIMD is not a bounded autofix); the buggy shape (`for line in text.split('\n') { line.find(..) }`) is extremely common in user code — every line-oriented parser.

## Caveats

- Young crate: first release 2026-09-02, 4 versions in 3 weeks, 109 downloads, 51★. Maintenance is irrelevant to pattern grade but recorded for provenance.
- **Unread files — re-verify at mine time:** `producer.rs` (653 L — the walk side, only README + grep level), `identity.rs` (833 L), `language_file.rs` (1345 L), `domain.rs` (934 L), ~85% of `file_parser.rs` (4465 L — read front matter + ScanPlan + greps), render/explain/tests. The ranked grades above cover only what was read.
- The candidate-2 dedup risk is the most likely coordinator kill; candidate 1 has no known duplicate.
- Freeze-table law: if the batch's corpus delta touches `build_seed_corpus()` stride windows, the gen2/gen3 regeneration rides the SAME batch (the Batch-151 lesson) — rust_perf additions may or may not move it; the plan gates on measurement.

## T2 pre-verification addendum (2026-09-22 ~22:4x +07, Plan 166 T1+T2 executed early — read-only; the batch itself stays cadence-gated, `.raw/mezura` re-pinned LIVE at `2dbacb7` until T13)

**T1 — citation re-verification: ALL PASS, zero drift.** Re-cloned at the pinned `2dbacb7`; every recorded citation re-checked against the tree: file line counts (masks 257 / producer 653 / identity 833 / file_parser 4465), constants (`SMALLEST_MAPPED_FILE_BYTES = 262_144`, `MAX_RETAINED_FILE_BUFFER_BYTES = 4_194_304`, `INITIAL_FILE_BUFFER_BYTES = 150`, `SHEBANG_READ_LIMIT = 256`, `MAX_IDENTITY_LEN = 32`), mechanisms (AVX2 2×32 cmpeq+movemask masks.rs L105–127; NEON vpaddq bit-gather L169–170; expected+1 sentinel + 64 KiB sizing window file_parser.rs L221–258; steal_batch consumer.rs L64–73; clamp L160–162; ScanPlan L309 + anchors_of L429), manifest facts (edition 2024, thin LTO, CGU 1, memmap2 under `[target.'cfg(unix)'.dependencies]`), and the README §Threads quote (L676–677, verbatim).

**T2 — the unread-file sweep (producer.rs L1–249 production, identity.rs L1–480 production, file_parser.rs L445–1000): ONE new candidate-grade find, two strengthening cites, five annotations.**

| # | pattern | source | grade | notes |
|---|---|---|---|---|
| 7 | **O_NONBLOCK probe open** — head-probe reads of arbitrary paths open with `OpenOptions::read(true).custom_flags(libc::O_NONBLOCK)` on unix: a kernel notification file (`/sys/kernel/security/apparmor/revision`) "gives its value on the first read and makes the next one wait for that value to change, which never comes" — a plain `File::open`+read head-probe HANGS forever. The read loop treats `WouldBlock` as end-of-head. | `identity.rs` L406–440 (`read_first_bytes`, `open_for_probe`) | **B−** | Dedup PASS (`O_NONBLOCK\|nonblock\|NonBlock` → 0 corpus hits; single mezura site). Distinct from candidate 3 (open-mode hang vs map-admission SIGBUS). NotBenchable-by-construction (the failure is a hang) — suggest-only, same admission class as candidates 3/5. Buggy shape: any tool that head-probes arbitrary files (counters, detectors, previewers) without O_NONBLOCK. |

**Strengthening cites (grades unchanged):**

- Candidate 4 (steal-batch): `producer.rs` L113–133 uses `steal_batch_and_pop` on BOTH the global injector AND peer stealers (skipping self) — the mechanism is engine-wide, not consumer-only.
- Candidate 5 (backoff ladder): third site — producer idle poll `thread::sleep(50µs)` (`producer.rs` L106) beside the consumer's yield→2ms ladder.

**Annotations (family notes; NOT standalone — positive instances / adjacent classes):**

- Stack-buffer case-fold with rare-alloc fallback (`identity.rs` L354–378: `[u8; 32]` + in-place `make_ascii_lowercase`; allocating `to_ascii_lowercase` only past `MAX_IDENTITY_LEN`) — alloc-reuse family.
- Prebuilt `memmem::Finder<'static>` per needle set + per-language lazy matcher caches (`file_parser.rs` L753–835, L913–971) — B67-adjacent (the ScanPlan is the generalized form).
- `code_spans` batching — keywords searched once over gathered code spans "instead of once per alias per line" (`file_parser.rs` L560–567) — batching family.
- `is_minified` average-line early-exit — `memchr_iter('\n').take(len/AVG).count() < len/AVG` settles the average from a bounded prefix (`file_parser.rs` L989–999) — the dropped bounded-head-skip family, second instance.
- `cfg!(windows)` DirEntry-metadata size freebie (`producer.rs` L215–217) — stat-elision, folds into candidate 3's notes.

**ScanPlan consumer machinery (file_parser.rs L445–751) confirms candidates 1–2 end-to-end**: the `Candidates::At(offsets, lead)` path consumes the dual-mask positions directly (no per-line search; offsets rebased per line via `checked_sub(lead)`), the `Unsearched` fallback walks packed memchr/memchr2/memchr3 chunks, and sorts fire only for kinds split across passes — one pass yields line order for free.

**Still unread by design (out of T2's plan-scoped subset):** `language_file.rs` (1345 L), `domain.rs` (934 L), file_parser.rs beyond L1000, render/explain/tests — the remainder is language-definition data and output formatting; no simd/io/concurrency surface expected, and the plan's T2 subset covered the engine files where such candidates live.

## Batch 172 landing close-out (2026-09-23, past-midnight cadence slot)

**Adjudications executed (the coordinator falsification):**

- Candidate 1 `dual-mask-fused-block-scan` — **LANDED** (simd, benchable). MEASURED ParetoDominant 4/0/0 verdict-stable ×2 (4090 AVX2): 43.11×/43.95×/23.10×/18.29× at 4 KiB/64 KiB/256 KiB/1 MiB; alloc 0/0; G-det stable. G1 exhaustive in-module gate (16,384-combo byte×position sweep + dense + short + end-to-end; it caught the `\n`-in-needle-set contract subtlety pre-landing). NEON arm aarch64 cross-compile-verified; **runtime re-gate DONE on M3 (2026-09-23 00:1x +07, the transposed-weight precedent): G1 exhaustive gate PASS on the dispatched NEON arm (corpus::tests 124/0 at release, rust_perf_bench) + perf_bench_demo ParetoDominant 4/0/0 verdict-stable ×2, alloc 0/0, G-det stable ×3; speedups 21.4×/21.5×/20.7×/14.2× at 4 KiB/64 KiB/256 KiB/1 MiB (run 2: 23.3×/21.3×/19.1×/14.2×) — M3 LOADED box (loadavg 15–20, sibling cargo builds throughout), so magnitudes are lower bounds; the verdict quantity (Pareto 4/0/0 + alloc 0/0 + det-stable) is the cross-arch confirmation and it holds.**
- Candidate 2 anchor-cover — **RIDER on `byte-find-position-to-memchr`** (B67's notes name the 2/3-needle ladder; the selection algorithm — greedy min-cover, letters disfavoured, anchor-behind-found-byte — is the rider's distinct content; B67 flipped None→Some).
- Candidate 3 `mmap-settled-size-gated-read` — **LANDED** (io, NotBenchable, the B139/B145/B118 standing; distinct-from paid to all four mmap-adjacent rules).
- Candidate 4 `steal-batch-not-per-item` — **LANDED** (concurrency, NotBenchable; the T2 strengthening cite carried).
- Candidate 5 oversubscription — **DROPPED**: decision-content-only, adjacent to `batch-blocking-pool-segment`'s sizing axis; no distinct mechanism survives the adjacency read.
- Candidate 6 scratch-capacity clamp — **RIDER on `reuse-allocation-clear`** (retention is not a bench axis; bounded reuse as the third retention policy).
- Candidate 7 `nonblocking-open-for-head-probes` — **LANDED** (io, NotBenchable-by-construction; the hang class, distinct from cand 3's SIGBUS axis).

**Net: 4 rules + 2 riders + 1 drop.** rust_perf 145 → 149 (simd 9, io 26, concurrency 21). Lib 2175/0; score-bench #160 heal_rate 96.36% FLAT; freeze tables unmoved by derivation (proposer-less rules never enter the atom vocab). T3 mine-time dedup re-grep PASS on all candidates. `.raw/mezura` removed at landing.
