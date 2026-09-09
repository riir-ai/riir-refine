# Mining queue snapshot — the distill-program resume state

> **Live queue state (2026-09-05, post-Batch-111 + the R134 perf-search unit)**

> **FOLDED 2026-08-31:** the closed-batch history (every idle-loop cycle entry through
> Batch 93 + the FULL post-mining intake records + the sweep/batch-landing sections)
> lives VERBATIM at `.research/108_mining_queue_snapshot_history.md`. This file keeps
> only the live queue — keep it under ~400 lines; fold again when history re-accumulates.
> This file IS the resume state: read at session start, update it in the same commit
> as each batch. Batch DETAIL lives in `.research/NNN_*.md` — this file holds only
> the rolling queue (next targets, sibling-blocked files, corpus count, standing
> lessons). Canonical batch ledger = AGENTS.md batch entries; per-batch detail =
> `.research/NNN_*.md` + `.plans/NNN_*.md`.
>
> Mining via a live external clone — the `.raw` discipline applies: clone into
> `<repo>/.raw/<name>` (gitignored), pin `@ <sha>` + license in the note BEFORE
> the verdict, `rm -rf` when the batch lands. Full contract = `distill` skill
> §"Full-source access" (Batch 96-98 vendor precedent) + katgpt-rs `research` §0.5.

> **Live queue state (2026-09-09, post-Batch-141 — the ds4 Part-B rust_perf batch LANDED (Plan 121, corpus rust_perf 121 → 124, see Last landed); queued (execution HELD — `kernel_opt/corpus/entries_gpu_27..35.rs` carry sibling WIP; never two sessions into one corpus): the CloudRift "Outperforming vLLM and Llama.cpp on Gemma4-12B" article (Aug 1 2026) + the emmy tip past B138's pin (tip now `321288d8`, serving rev `d3df1f2a`; blog = quote authority) — SCOUTED 2026-09-09, Research 163: YES B+, kernel_opt 2–3 rules + ~4 riders (lead `fast-accumulate-periodic-promote-bounded-error` — FP16-acc HMMA at 2× rate + f32-shadow fold every 64 K, flat 3.3e-4; + smem-budget kill-point-staggered single-buffer refill; + roofline-gates-transport-upgrade rule-or-rider vs B138's limiter; + split-KV occupancy rider on B105 — LSE-merge correctness already COVERED by B137's atomic-arm-legality), LEAGUE INTEL NONE — triple-crossed (Gemma4-12B FP16 ≠ Bonsai/qwen3.8 ternary, 5090 half not-our-silicon, Emmy ships no quant support); the Issue-085 dist lane LANDED 2026-09-09 (`79dcf9b`, the owner's reversal — the 6th healing domain shipped); arxiv walk #9 opens Mon 09-14 (sibling lane); M3 batch records follow)**
>
> **BATCH 141 LANDED 2026-09-09 (Plan 121 / Research 162 — the antirez/ds4 Part-B walk, the batch R162 staged; opened when the Issue-085 dist lane landed `79dcf9b` and the rust_perf write set was verified disjoint from the sibling's kernel_opt WIP): 3 rust_perf rules, corpus 121 → 124 (io 19 → 21, allocation 23 → 24; NotBenchable 27 → 30; annotated 64 → 67), all suggest-only decision-content.** Source antirez/ds4 @ `6289c516` (MIT; `.raw/ds4b` fresh clone, HEAD == pin, LICENSE verified, removed at landing; quotes re-verified at the pin). Rules: `compaction-reserve-before-prompt-build` (allocation — the reserve for a destructive context rewrite is MEASURED from the real prompt (build it, tokenize it, + slack + output budget) and EVERY free-threshold consumer reads that one figure — ds4_agent.c :9354-9364 + 4 call sites), `compaction-tail-boundary-prefers-complete-turn` (io — truncation boundaries are SEMANTIC: bounded walk-back (2× budget, floor sys_len) to the latest COMPLETE user turn, the source's own verbatim rule "A slightly longer complete turn is preferable to losing the user's constraints and keeping only a fragment of our answer", + atomic-span snap (strictly-inside → span START) — :9286-9300 + :9367-9377), `streaming-prefix-holdback-bounded-buffer` (io — a stream recognizer holds back at most the marker's own length in a TYPE-SIZED buffer, matches incrementally behind the prose-line-boundary gate, flushes verbatim on first mismatch — no line-length pending window — :225-226 + :3294-3325). Riders adjudicated: image-boundary snap folded INTO rule 2 (not standalone); visible-prompt-key normalization NOT landed (B106's cache-identity face carries it); R162's "~3 riders" resolved to those two dispositions. Gates: batch141 2/2 FIRST-SHOT (presence+parse+needles; the G1 semantics test — reserve max-lift, complete-turn walk + span snap incl. the boundary/end negatives, holdback bounded + verbatim mismatch flush + full-match-at-last-byte) + rust_perf lib 1193/0 + default 1193/0 + kernel_heal 1530/0/1i + kernel_heal+latent 1652/0/2i (retrieval floors + router floor HELD, no re-pin) + clippy 0 ×2 + score-bench #90 FLAT 96.2/1.9 (10th consecutive) + corpus-stats 124. Count pins walked: header totals + category table (io/allocation) + NotBenchable ledger + batch-list +3 + "Not benchable" line + build-fn doc + entries.len + without_spec (+3 membership) + annotated ×3 sites (two were NOT in the B139 walk's inventory — caught by the run, not by the plan: context_notes serde + drafter-propagation) + AGENTS ×2 + README ×2 + .docs rust_perf.md (count + Batch-141 paragraph + 67-of-124) + .docs index (a PRE-EXISTING 120 stale from B139's walk, repaired). LESSON re-proven (B125, 15th strike): a line-initial `+ ` in a /// doc paragraph parses as a list marker — the io-ledger reflow fired 3 doc_lazy_continuation errors until reworded to flowing prose. Records: Plan 121 / Research 162 (Status → RECORD).

> **VERDICT 2026-09-09 (Research 161 / Plan 120) — `rustsec/advisory-db` @ `b50980aa` (CC0): YES B, route sec.** Data-source shape (taxonomy oracle, not code): 1,223 advisories, category oracle computed (DoS 126 / memory-corruption 271 / memory-exposure 88); lead rule `unbounded-read-into-vec` (h2/CVE-2019-9514 class, `take(limit)` bounded fix, suggest-only) — dedup: composes with `length-prefix-validate-before-alloc` (stream-growth ≠ alloc-from-length), 5 classes COVERED by existing rules (dropped), zeroization C+ deferred, panic-on-parse + traversal dropped (fix-space unbounded). Yield 1-2. Lane-intel axis: does NOT fire (no perf-league surface — security data). `.raw/advisory-db` removed at the verdict commit; re-clone at pin for Batch 140.

> **BATCH 139 LANDED 2026-09-09 (Plan 119 / Research 154; armed as Plan 113/B134, renumbered at execution `956ae50`):** the ds4-metal PR #5 distill (polymorf's Qwen3.8 M5 rounds + the maintainer's integration commits; pin `18ca8ec`, delta past Batch 129's `b85a6174` = 19 commits) — 4 kernel_opt rules (expert-major tile order H, one-row-per-simdgroup narrow matvec E, tile-max prefiltered exact top-k I, sustained-load-drift interleave protocol D) + 1 rust_perf io rule (prefault-admission-keyed-to-ram-headroom, fail-closed + the mincore instrument refusal) + 8 riders (swap-restore on B7/B58, stale-source log-once on B129, MPP double-buffer on B130, contraction rejection, kv-dequant chunk-width facet, GDN swept-table row, scorer staging on B108, merge prefetch on register-rotation); corpus 605/121/49/14/31. Gates: batch139 3/3 + rust_perf gate w/ fail-closed G1 + default 1188/0 + kernel_heal 1525/0/1i + kernel_heal+latent 1647/0/2i + retrieval floors byte-identical (8th) + router 95/131 FLAT (floor held) + clippy 0 ×3 + score-bench #86 FLAT 96.2/1.9 (9th). Corrigenda vs Research 154: stale-sidecar defense = dispatch-granularity coupling, not in-kernel; NR0 was 4→1 not 2→1 for half the shapes; prefetch is `44fb985`; GDN_PREFILL_NSG predates `bc61011`. `.raw/ds4-metal` removed at landing. Sibling-worktree hygiene: Batch 129 rider files `entries_gpu_34/35` committed as HEAD+rider blobs (hash-object) so the sibling's rustfmt WIP stayed uncommitted.

> **BATCH 138 LANDED 2026-09-09 (Plan 118; `a68005e` → the landing-tail commit):** the CloudRift "Learning FlashAttention the Hard Way" Parts 1+2 distill — 10 kernel_opt rules (A +2, B +2, E +2, F +1, K +1, L +1, M +1; corpus 591 → 601; entries_gpu_38.rs) + 4 riders + 5 drops-as-covered, both adjudications recorded (two-chains below-threshold → twisted-monoid provenance; warp-scoped-barrier wash → absorbed by the SFU limiter rule), lane-intel YES scoped to consumer silicon (FA-2 parity is the honest frontier on the 4090/M3; the transferable set: Q-hoist, C→A repack, split-copy issue/deposit — the #758 form is EXACTLY the M3 no-cp.async regime). Sources: emmy @ `bdd26c53` (the tip carrying #758 — post-article content mined in-batch) + both blog articles fetched full-text (the bare URL 404s; real paths carry `-part-1`/`-part-2`). Gates: batch138 3/3 FIRST-SHOT + default 1187/0 + kernel_heal 1521/0/1i + kernel_heal+latent 1643/0/2i + retrieval floors byte-identical (kernel 100/100/100, blended 94.7/96.9/97.7 — 7th consecutive) + router measured 95/131 = 72.5% (UP from 91, floor 86 held, no re-pin) + clippy 0 ×3 + corpus-stats 601/120/49/14/31 + score-bench #85 FLAT 96.2/1.9 (8th consecutive; OOD axis live). Session notes: HEAD moved under the batch (sibling 077/081 lanes, `adaebc4` → `206a052`) — a stale batch2 concept pin repaired to a prefix assert + the sibling's Issue-077 T2 ungated `crate::score_bench` test reference fixed as a cited drive-by (E0433 on every score_bench-off build). Research 158 (157 was taken mid-session by the VeloxQuant sibling — highwater re-read at write time). `.raw/emmy138` removed at landing. ~~Batch 138 = FA parts 1+2 stays queued; do not fold~~ CONSUMED.

> **BATCH 137 LANDED 2026-09-09 (Plan 116; `288f984` → `4c2e94a`/`b638e71` → `efe724c` → `818dd8a` → `b724437` → `235c8d4` → the landing commit):** the CloudRift matmul-article distill — 11 kernel_opt rules (A +5, E +4, L +1, N +1; corpus 580 → 591, incl. 2 recorded WASHES: ptxas already coalesces/reschedules), both adjudications resolved (split-K legality + contention = two cross-referenced rules; ldmatrix x2→x4 = full rule), lane-intel YES (splitk-width two-floor gate strongest for the 4090 lane). Gates: batch137 3/3 by name + default 1187/0 + kernel_heal 1518/0/1i + kernel_heal+latent 1635/0/2i + retrieval floors byte-identical (kernel 100/100/100, blended 94.7/96.9/97.7) + router re-pinned 88→86/131 (measured 91/131, the B131 sec-centroid mechanism in kernel flavor) + clippy 0 ×3 all-targets + corpus-stats 591/120/49/14/31 + score-bench #83 FLAT 96.2/1.9 (7th consecutive). Drive-bys: 1 stale router pin (580→591) + 4 pre-existing test-target clippy rot + the kat_refill cfg asymmetry (E0425, broke kat_service-without-transport builds — the score_bench build caught what the author's transport-on test builds could not). `.raw/emmy` removed at landing. Batch 138 = FA parts 1+2 stays queued; do not fold.

- **Last landed: Batch 136 (2026-09-09, Plan 115 — the Lattimore "Graph Algorithms in Rayon" blog distill; M3 box, CPU-only, blog cite-shape NO clone): rust_perf +3 (117 → 120: concurrency 10→13), all three NotBenchable-annotated decision-content — the source is a QUALITATIVE approach comparison with NO measured numbers and the author's own closing verdict IS the ladder's content.** Rules: `dynamic-work-parallelism-ladder` (the lead — work-unknown-in-advance STRUCTURE decision: scoped spawning is the composition-safe default despite the per-task heap boxing; spawn_broadcast + private job control STARVES the pool; channel+par_bridge fails composition — owned-value-cannot-split-into-borrowed-work-items; pick by composition first, allocation second; the rayon doc's 'more expensive' warning is the trap) + `sender-per-work-item-terminates-channel` (the dynamic-queue shutdown idiom — sender clone per work item, drop the ORIGINAL after seeding, the channel closes when the last item drops; the drop(tx) is load-bearing) + `nested-par-inside-channel-bridge-deadlocks` (the correctness lead — a nested par_iter inside bridge-fed work steals, blocks on the SAME channel, the interrupted item's sender clone keeps it open, thread-tied tasks hang; fixes: serialize inner / pre-materialize / scoped spawning). Rider folded in-rule: rayon try_for_each_init init-per-WORK-ITEM (the 0.6.0 note). DEDUP: zero corpus hits under par_bridge|spawn_broadcast|scoped.?spawn|work.?item|crossbeam; nearest = B65 channel-send-batching-unpark (throughput batching — orthogonal) + B135 partitioned-mut-slices-zero-sync (KNOWN work — complementary); bounded-channel-backpressure (async backpressure — different axis). Lane-intel axis: does NOT fire (host-side concurrency; no league surface). GATES: rust_perf filter 253/0 (2 new batch136 gates: presence+parse+needles incl. NotBenchable membership; termination-parity G1 on the ownership-close idiom) + lib default 1187/0 + kernel_heal 1515/0/1i + kernel_heal+latent 1632/0/2i + retrieval kernel 100/100/100 blended 94.7/96.9/97.7 (byte-identical to B118-B135) + router FLAT 93/131 (71.0%, floor 88 held — the context_notes annotations did not move any centroid, no re-pin) + clippy 0 ×3 + score-bench #82 FLAT 96.2/1.9 + --corpus-stats 120. Count pins walked: corpus.rs header totals ×2 + concurrency prose ×2 + benchable/NotBenchable lists + entries.len 117→120 + without_spec 27→30 (+3 membership asserts) + annotated 60→63 ×3 + AGENTS ×2 + README ×2 (rule row + tree) + .docs ledger (count + Batch-136 paragraph + category prose + index row). LESSON re-proven: a fuzzy edit_file anchored on a name line hit the NEIGHBORING rule (duplicate category field) — struct literal edits need rule_id-anchored context, and the E0063 duplicate-field error is the safety net. Source fetched full-text at mine time (16.8 KB, 20 rayon hits; the /posts/2025/11/27/ URL shape — the scout's 27-graph-algorithms path 404s). Records: Plan 115 / queue-verdict 2026-09-09 (item-11 scout).

- **Last landed: Batch 135 (2026-09-09, Research 155 / Plan 114 — the Wild-linker "RustForge wild performance tricks" article distill; M3 box, CPU-only): rust_perf +3 (114 → 117: allocation 22→23, concurrency 8→10), all three MEASURED benchable — the first owner-directed e2e contribution test.** Source: David Lattimore's Wild-linker article (davidlattimore.github.io, 2025-09-02; blog post, no repo clone — cite-shape only, ALL fixtures original). Rules: `atomic-vec-zero-cost-convert` (concurrency, the GOAT lead — Vec<T> ↔ Vec<AtomicT> representation switching is FREE when size+align match: into_iter().map on the OWNED vec reuses the buffer and the loop compiles out; MEASURED ParetoDominant 4/0/0 verdict-stable ×2, 1.83–1.33× at 100–100K, alloc axis EXACT 2→1; switch representation per phase instead of permanent atomics or transmute; back-conversion consumes via into_inner) + `vec-reuse-across-lifetime-change` (allocation — a hot loop allocating Vec<&'iter T> per iteration because the hoisted buffer cannot be TYPED → own the buffer + retarget in place: const size/align assert (inline const) + clear() + empty collect; MEASURED Mixed 2/0/2 (1.49×@100/1.07×@1K, Tie@10K/100K) + alloc axis EXACT 1→0 everywhere — the integers are the load-bearing cell; the 'static element type is a caller contract, panic = buffer-loss never UB; MaybeUninit shadow-struct extension recorded in-rule, not landed) + `partitioned-mut-slices-zero-sync` (concurrency — shared Vec under one contended Mutex although each worker writes a known DISJOINT contiguous range → chunks_mut partition, zero locks; MEASURED ParetoDominant 4/0/0, 6.90/9.12/11.81/6.13× at 1/2/4/8 threads; alloc axis EXEMPT per the Batch-78 contended-mutex probe class; std-only thread::scope — the article's split_off_mut is nightly-shaped). DEDUP GATE: `reuse-allocation-clear` covers the caller-passed scratch (this batch's rule is the lifetime-TYPING obstacle it cannot express); `cache-line-block-partitioned-lock` partitions LOCK GRANULARITY (this eliminates synchronization via ownership partition — they compose); zero hits for split_off_mut/sharded/MaybeUninit/rayon-spawn. DROPS: sharded-vec parallel init (third-party dep + composition), defer-drop-on-thread (allocator-dependent, no deterministic verdict — lead recorded), MaybeUninit shadow struct (folded into the lifetime rule). Lane-intel axis: does NOT fire (host-side linker-phase Rust; no league surface — recorded explicitly). GATES: default lib 1179/0 + bench-feature lib 1197/0/1i (the REAL-compile gate caught 3 harness bugs — Ordering import inside mods, mut closure, super-above-crate-root — the B131 lesson re-proven) + batch135 gates 3/3 (presence+parse incl. the inline-const syn acceptance, G1 parity via batch135_fixtures_g1_parity, ignored batch135_pareto_benches runner) + clippy green both states + count pins walked: corpus.rs header (117/23/10/90/27 — REPAIRED the pre-existing 25-vs-14 NotBenchable drift), benchable/not lists (87/111 → 90/117, 15/102 → 27/117), build-fn doc 105→117, entries.len 114→117, annotated 57→60 ×3 sites, fourteen_rules_are_not_benchable 14→27 (pre-existing rot — the B118–B129 io annotations were never counted; nobody runs the rust_perf_bench-gated test), AGENTS ×2, README rust_perf row (REPAIRED stale "112 through Batch 125") + tree count 101→117, .docs/11_domains/rust_perf.md (count + Batch-135 paragraph + category prose + 3 table rows), .docs/11_domains/README.md index 110→117 + score-bench #81 FLAT 96.2/1.9 (100/100). Disk incident: / hit 100% mid-batch (sibling /tmp build dirs ~130 GB — untouched per the never-touch-siblings rule); cleaned ONLY my own plan114_target + retried — the validator's real-compile gate re-caught the super-above-crate-root bug on the retry, so the ENOSPC intermission cost nothing. Record: Plan 114 / Research 155.

- **Last landed: Batch 131 (2026-09-08, Research 153 / Plan 112 — the Girder sec distill; M3 box, CPU-only): sec +2 (12 → 14, the NEW `InputValidation` family; both SUGGEST-ONLY — FIXABLE_SEC + collect_edits untouched) + 2 parse-only detectors with zero-FP negative suites.** Source `dhishwasher/Girder` @ `3b29453f` (BSL 1.1 — ORIGINAL fixtures only, shapes cited at the pin; fresh clone, HEAD == pin, LICENSE verified, removed at landing; all four evidence sites re-read: aether-dap transport.rs UNBOUNDED `vec![0u8; len]` from a peer `Content-Length` vs collaboration_transport.rs BOUNDED twin `0 < n ≤ limit` pre-alloc + the limit+1 negative test, + both 0600 sites). Rules: `length-prefix-validate-before-alloc` (the lead — same-fn parse→`vec![..; len]` with no intervening bound comparison = allocation-size DoS; the both-ways evidence: an unbounded transport beside its own bounded twin, same author; detector tracks idents through casts/?/ok_or/Some/arithmetic, any comparison/match/assert validates incl. derived-length→source, vec!-repeat form only, cross-fn + field-origin recall limits pinned AS TESTS) + `secret-file-create-without-0600` (T5 admission PASSED — narrow credential vocabulary word-boundary incl. plural stems `secrets.json`, create-class writes only, ANY hardening marker suppresses the whole fn, ambiguous words `key`/`identity`/`private`/`pem` EXCLUDED so api_key/private_key are honest misses; suggest-only because a portable rewrite is not parse-only-provable — Windows from_mode maps coarsely). SecFamily +`InputValidation` (additive, appended after PanicSafety). Gates: sec lib 1261/0 (count pins: corpus 14, rule_ids_unique_and_kebab, FIXABLE_SEC unchanged 6) + boundary gate 2/2 (real clippy over the grown probe crate — CAUGHT an E0425 in my first fixtures: `std::io::TcpStream` vs `std::net::TcpStream`, the compile-vs-parse axis doing its job) + lib default 1177/0 (sibling kat-work count, sec is cfg-gated out) + retrieval kernel_heal+latent BYTE-IDENTICAL (kernel 100/100/100 blended 94.7/96.9/97.7, router 93/131) + sec self-variant top-1 holds for both new rules + ROUTER ANNOTATION RE-PINNED per the Issue 020 contract: sec arms re-measured 41/101 → 35/101 (34.7%, floor 36→30/101) and 82/131 → 79/131 (60.3%, floor 77→74/131) — the new wire/fs vocabulary ATTRACTS generic Rust spans to the sec centroid, first measured sec-arm DECAY, recorded not weakened; the no-docker sec arms are no-longer-buildable states (default-feature hard deps) noted in the floor fn + clippy 0 default+sec + score-bench #80 FLAT 96.2/1.9. Count pins walked: AGENTS ×2 + README ×2 + .docs index + sec.md §Batch 131 + corpus doc table + proposers mod.rs. LESSON re-proven: the boundary gate's real-compile pass catches what syn parse gates cannot — run it BEFORE claiming fixture readiness, not after. Records: Plan 112 / Research 153. Pin now current: `dhishwasher/Girder` @ `3b29453f` (clone removed at landing).
- **Last landed: Batch 129 (2026-09-08, Research 151 / Plan 110 — the ds4 Qwen Q2 decode-fusion + prefill-structural distill; M3 box, CPU-only): kernel_opt +5 (570 → 575: C 45→46, H 27→28, L 14→15, M 18→19, N 41→42) + rust_perf io +2 (112 → 114, io 16→18) + 5 riders + a full negative-results harvest.** Dual trigger verified by API (B120 zero-build): PR #991 head `ffd85d42` → `b85a6174` (LINEAR — the B125 rebase trap did NOT re-fire; the compare IS the delta, 12 commits / 42 files) AND upstream main `c0a6119f` → `f62ca29a` (whose content commit `b6af0ad` rode into the PR branch via the upstream merge — one mine covers both triggers). Source `ivanfioravanti/ds4-metal` @ `b85a6174` branch `qwen3.8-flash-next` + `antirez/ds4` @ `f62ca29a` (both MIT; fresh clones, HEAD == ls-remote, LICENSE verified, removed at landing; 2 read lanes + coordinator, quotes spot-re-verified at the pins; Lane 2 honestly reclassified the coordinator's b6af0ad draft rule → rider on the landed B123 static-seed rule). Rules: pipeline-cache-key-covers-every-function-constant (M; subset cache keys serve one variant's compiled binary for another's launch args = silent wrong math; the key extension is the step forgotten when the SECOND function constant lands — the source's own 38d3e45→b4c3550 history is the defect-first lesson) + block-quant-weight-pad-never-extends-activations (N; Q2_K 256-block padding is weight-stride-only, loops gate on the LOGICAL bound, partial activation groups fail-closed, tests poison the padding nonzero) + expert-remainder-routed-to-smaller-tile-twins (H; per-expert tail rows run the smallest covering tile twin under a compile-time tail_base; the runtime-branch alternative measured −10.98% and the disabled branch alone slowed the control in-source) + window-recurrence-halo-snapshot-block-parallel (L; K≤4 causal conv parallelizes behind a HALO-only snapshot — the full-input snapshot measured slower and was superseded in-source; snapshot scope is the load-bearing choice) + fused-residual-update-writes-separate-next-buffer (C; fused residual+norm reads old read-only, writes distinct next, alias-rejects fail-closed, gates on the measured session shape — MTP measured 0.04-0.42% slower and was restricted out) + instrumented-attribution-not-production-times (rust_perf io; instrumentation slowed the pipeline ~38% in-source — ranking-only evidence; the ranked-#1 family measured NEGATIVE while the ranking's LAST item produced the retained win) + unset-control-arm-silent-default-parity (rust_perf io; unsetting the env on a DEFAULT-ON candidate runs the same code in both arms — explicit control value + 0-vs-1 validation, the --control-value fix shipped in-source). Riders: argmax CPU-parity facets (B25; BOTH lanes converged independently), fusion-payback second-stage-fan-in instance (124,160 partials vs 61 — a ~2000x reduce-input multiplication that ate the dispatch saving) + fail-fast + negative-recording contract, static-seed dynamic-producer twin (b6af0ad: recency-inflated selection score passed as eviction hotness pinned seeded experts; 0.48 → 4.38 t/s at 100K ctx; seam feeds TWO fields — hotness AND LFU count; the non-Apple carve-out premise recorded UNVERIFIED), kv-dequant expansion-ratio axis (Q8 ~1.9x expansion won, IQ2 ~6-8x lost at the SAME 8K batch — the threshold is FORMAT-dependent too; round-identity facet), full-vector-bit-identity two-weak-gates instance (local FP64 + greedy-text identity passed while full-logit caught 2.63 max-abs; MoE routing amplifies rounding). Gates: batch129 4/4 FIRST-SHOT + lib default 1168/0 + kernel_heal 1493/0/1i + kernel_heal+latent 1610/0/2i + retrieval kernel 100/100/100 blended 94.7/96.9/97.7 (byte-identical to B118-B128) + ROUTER ANNOTATION RE-PINNED per the Issue 020 contract (re-measured 88/131 = 67.2%, was 94/131; floor 89→83 per measured−5; the 7 adds shifted TF-cosine centroids — one annotation flipped) + clippy 0 ×3 + score-bench #78 FLAT 96.2/1.9. Count pins walked: totals ×5 + category table C/H/L/M/N + mod.rs lists/history + rust_perf header/io/annotated ×4 + AGENTS ×3 + README ×2 (B128's README gap re-fixed) + .docs both + the batch25 argmax FULL-STRING pin regenerated (prefix+rider form, the B125 lesson). LESSON (new): a shell python write to mod.rs was silently reverted by a later edit_file on the same file (stale cached read) — only the batch-gate run caught it; keep a file on ONE write tool per session; and a count-pin regex that silently mis-anchors (no-trailing-comma case) needs a post-write membership verify, not a printed OK. prismml master re-checked identical to `5ea87dda` — the P14 watch's "15 commits past the league pin" are tag ancestry inside B128's mine, no trigger. Records: Plan 110 / Research 151. Pins now current: PR `b85a6174`, upstream `f62ca29a`.
- **Last landed: Batch 130 (2026-09-08, Research 152 / Plan 111 -- the CloudRift cuBLAS-5090 TMA distill; M3 box, CPU-only, 4090 idle-checked 18%/565 MiB): kernel_opt +5 (575 -> 580: A 121->122, B 60->61, D 130->132, E 46->47; entries_gpu_36.rs) + 7 riders.** Source `cloudrift-ai/emmy` @ `2aaa0328` (Apache-2.0; the blog landing commit #89 -- NOT the lead's `DeploDock/Emmy` path which 404s: the org is `cloudrift-ai`, found via code search for the `deplodock bench` CLI string; `8764cc2e` restructured the repo so the recipe dir + `deplodock/compiler/cuda/lower.py` exist only at the pin; fresh clone, HEAD == pin, LICENSE verified, removed at landing; 2 read lanes + coordinator, quotes re-verified). Rules: vendor-dispatch-verify-before-outsource (D, the lead -- cuBLAS 5090 batched FP32 dispatches ONE `simt_sgemm_128x32_8x5` across a 32x range at 33-42% FMA pipe while the same binary escalates to 73/82% on Pro 6000/H200; "no escalation at any threshold" = the ladder is MISSING, not mistuned; a suspiciously-good win is a dispatcher-bug SIGNAL; protocol = ncu kernel name + pipe util + cross-SKU same-binary + the vendor's own batched API form as baseline) + sub-ms-bench-boost-clock-never-engages (D; below ~1 ms the boost governor never engages -- interleaved pairs + median + ratio-not-absolute + clock sampling, clocks deliberately unlocked) + lds-cluster-cross-warp-phase-not-static-distance (B; once static hiding suffices, LDS temporal distribution governs cross-warp phase: clustered spacing-0 halves dispatch_stall 44->22% vs the generator's spread spacing-5; generator-side cause = emission order at lower.py:192-198, no scheduling knob) + tma-single-issuer-bulk-tile (A; one thread issues the whole 2D tile, mbarrier byte-count completion + parity double-buffer, honest win = instruction count + simplicity NOT FMA pipe; ragged-K silent-truncation caveat found at mine time) + multiplexed-grid-axis-partition-modes-exclusive (E; split-K x batch both on blockIdx.z refused at config time -- the runner's precedence silently corrupts). Riders: l2-grouped-tile-schedule (SWIZ=8 formula + padded launcher + DRAM-parity boundary), unroll-fixed-inner (partial-unroll monotonicWORSE / no-unroll 57% boundary), autotune-warps-stages (launch_bounds 20x-spill negative + tensor-pipe family caveat + triple-buffer same-IPC ceiling), cpasync-multistage-tile-pipeline (82.6% TMA crossover + try_wait first-read), hardcoded-fast-path-jit-specialization (codegen #define/#if twin, honest 98->101%), launch-heuristic-pinned-to-calibration-arch (CC insufficient SKU key + DMA-generation tile optimum + same-CC library-side divergence), fast-math-contraction-behind-feature-flag (fmad = the EXACT half, split from use_fast_math). Blog-vs-repo corrections recorded (Research 152): the negative-results table is BLOG-ONLY (riders carry the caveat), the 260->460 MHz figure is blog-only (repo: ~260->~2400 + run samples 562->1890 / 682->810), Pro 6000 has its OWN magma-legacy-path bug. Gates: batch130 3/3 FIRST-SHOT + lib default 1168/0 + kernel_heal 1496/0/1i + kernel_heal+latent 1613/0/2i + retrieval kernel 100/100/100 blended 94.7/96.9/97.7 (byte-identical) + router annotation RE-PINNED (re-measured 93/131 = 71.0%, was 88 at B129 -- first measured IMPROVEMENT since B128; floor 83->88 per measured-5) + clippy 0 x3 + score-bench #79 FLAT 96.2/1.9. Count pins walked: totals x5 + category table A/B/D/E + 5 STALE category comments repaired (C/H/L/M/N -- the B129 walk missed its own comment pins) + batch23/batch29 FULL-STRING pins regenerated + 3 terse exact pins converted to prefix+rider (the B125 lesson). LESSON (new): the regex rider-applier with non-greedy concept capture DOUBLED concept text in 3 of 7 targets (caught by pins + junction-verify before landing); the standing protocol is line-anchored splice + junction check. Records: Plan 111 / Research 152. Pin now current: `cloudrift-ai/emmy` @ `2aaa0328`.
- **~~QUEUED → PLAN-FIRST~~ CONSUMED — Batch 131 LANDED 2026-09-08 (see Last landed above; Research 153 / Plan 112): dhishwasher/Girder → YES B− corpus, route `sec`.** THE CORRECTION: the first pass graded NO on the distill skill's stale routing table ("sec DOES NOT EXIST") — the owner's challenge measured the truth: `sec` is LIVE (12 rules / 5 families, Bench 046 Phase 0+1, `sec_boundary_gate`); the pre-flight corpus grep is the defense, the table is a snapshot. Source `dhishwasher/Girder` @ `3b29453f` (**BSL 1.1** — cite-shape-only, ORIGINAL fixtures mandatory; single-author 10-star 60.7k-LOC application repo = rule CONSUMER code, so the value is its security discipline, not techniques). Lead: `length-prefix-validate-before-alloc` (suggest-only per the shipped ffi-export-panicking-body precedent; BOTH-WAYS evidence in-source — `aether-dap/src/transport.rs` L24-55 `read_frame` does `vec![0u8; len]` from a peer-controlled `Content-Length` UNBOUNDED while `collaboration_transport.rs` L1209 read_frame validates `0 < n ≤ limit` pre-alloc with a limit+1 negative test: same author, two transports, one missed the gate — the class recurs even in careful code). Needs an ADDITIVE `SecFamily::InputValidation` variant (no existing family fits). Secondary `secret-file-create-without-0600` (C+, admission-gated at mine time; the 0600 write-half exists in our own kat_account.rs — precedent, not coverage). DROP: fail-closed-config + digest-bound-approval (no parse-visible trigger), route-context BFS (domain algorithm). Dedup: zero frame/alloc-bound hits in the sec corpus; nearest = kernel_opt `length-prefix-shape-faithful-hash` (the hashing class, different mechanism). Lane axis: does not fire (no serving/kernel/quant surface). TOOLING FOOTNOTE (owner side-question, recorded to prevent re-evaluation): Girder's semantic-code-graph product is NOT a workspace adoption candidate — single-repo scope (root fixed at server start; substrate-first crosses 17 repos), feature-gate-blind (tree-sitter indexes `#[cfg]`-gated dead code as live), dynamic-dispatch recall 0.000 by its own measurement (our trait-heavy HealDomain/ECS shape); grep + cargo-tree + riir-rag/BM25/AnyRAG + KG-triples already cover the need cross-repo. Unread at verdict: aether-debugger/*, marketplace.rs (re-clone at pin if a third candidate emerges). .raw/girder live at pin for mine time, removed at landing.
- **QUEUED LEAD-PAIR (filed 2026-09-08, post-Batch-130; the two newer same-author CloudRift articles, the natural Batch 132/133 candidates — renumbered 2026-09-08 when the Girder sec batch took Batch 131 plan-first): (a) "Modern GPU Matmul Optimization" (Dmitry Trifonov, 2026-05-29, cloudrift.ai/blog -- "register tiling, vectorized loads, shared-memory staging, cp.async, TMA, warp specialization, split-K, and tensor cores. Each is introduced, then demonstrated by toggling it on a real kernel and watching the generated code diff and the latency change on an RTX 5090") and (b) "Learning FlashAttention the Hard Way" Parts 1+2 (2026-07-06/08 -- "a generic compiler moveset walks one attention kernel from 25959us to 206us on an RTX 5090 -- on par with FlashAttention-2 -- with a measured number for every rung, the optimizations measurement refused, and the reasons FA-3/4 cannot land on consumer silicon"; Part 1 covers safe softmax as associative reduction + Welford + "a test for when any loop is secretly associative").** Code = the repo TIP surface (the restructured `emmy/compiler/`: pipeline passes incl. 095_interleave_loads, 096_pair_ldmatrix_loads, 110_drop_redundant_syncs, backend `_tma.py`, decomposition 070_matmul.py). Dedup REQUIRED at mine time: B130's five rules + the FA family (B128 Metal/FA, B113 fa-vec, B100/B115), the softmax family (B79-era + online-softmax rules), and the compiler-pass-shaped rust_perf corpus. Re-clone at the then-current tip (the pin discipline: blog + tip may have diverged again). Route: kernel_opt primary, rust_perf io secondary.
- **QUEUED (verdict 2026-09-08; .raw read at the pin, removed at verdict commit): ivanfioravanti/ds4-metal PR #5 (Qwen3.8 M5 Max rounds 1+2 + integration) → Batch 134 — YES B corpus / LEAGUE INTEL MODERATE; the ds4 trigger FIRED both axes: PR branch `b85a6174` (B129 pin) → `18ca8ec` (19 commits incl. merge — the PR page's "12 commits" undercounts: 6 post-merge maintainer commits ride along: MPP double-buffer, Q4_K decode MXFP4 specialization, PLE demand-page + prefault-headroom, stale-source diagnostics, tool-call fix) AND upstream main `f62ca29a` → `6289c516` (SEPARATE axis, NOT in the branch — Batch-135 candidate, dedup `ssd|glm|state.space` first).** Source ivanfioravanti/ds4-metal @ `18ca8ecdb5732a5c77c053e554e57daadf5ca32e` branch qwen3.8-flash-next (MIT verified; 25 files +9105/−93; polymorf's bit-exact M5 work + maintainer M3-Ultra integration/validation — 384 full-vocab vectors + 36,006,400 floats byte-identical). Rule candidates: expert-major-tile-order (D/H GOAT lead; grid (row×tiles, experts) so expert rows stay cache-resident across tiles; +1.8–2.6%; bit-exact by enumeration — NEW vs l2-grouped-tile-schedule's swizzle-within-grid) · narrow-matvec-one-row-per-simdgroup (A; ≤1024-row single-token F16/F32 matvecs 1 row/simdgroup — the INVERSE of row-tile-amortize's widening, an occupancy axis; −0.55% rollback pin) · tile-max-prefiltered-exact-topk (D; k-th tile max as lower bound, exactness proof + in-dispatch fallback, targets 128–262K, honest −0.0%/−0.8% at 16K) · prefault-admission-keyed-to-ram-headroom (io; WILLNEED vs demand-page two-regime on hw.memsize ≥ model+sidecar+16 GiB; + DONTNEED residency sweep 717→129 MiB; + the macOS mincore cache-wide-scope instrument caveat) · sustained-load-drift-abba + first-use-context-penalty-untimed-warm-pass (io; 10–20% drift/minutes → four fresh processes ABBA per frontier; ~25% first-session-at-new-ctx penalty → untimed pass first; the +16% artifact failure class; no-op-candidate noise floor) · prefilter-producer-gate-coupling (io C+; selector consumes the scorer's tile-maxima sidecar, scalar scorers don't write it; stale-maxima negative fixture). Riders: MTP swap-restore → B58/B7; stale-source log-once → B129 pipeline-cache detection half; MPP double-buffered k-step → B130 cpasync Metal instance; chunk-width-per-format (Q2@8192 +35%, Q4@4096) → B129 kv-dequant axis; GDN front 1024 threads → B104; scorer staging + half4 keys + 16-split prefetch → B113/B108; staged-loop contraction rejection → B130 fast-math; full-vocab byte-identity discipline → B123 confirmation. LANE: M3-decode leads (narrow-matvec + GDN widening = the B831 idle-core regime; Bonsai is a GDN hybrid) + protocol affirmation of the league's interleaved/verified-idle discipline; MoE/MTP/MPP content NOT transferable (dense Bonsai, 721/717 closed, M5 tensor cores); Flash-Next re-arm stays unlit (still not ≤24-GB-feasible). Execute from `.plans/113_batch134_ds4_pr5_m5_round.md`; fresh clone at the pin, re-verify HEAD == ls-remote (branch is HOT).
- **~~QUEUED: the Lattimore rayon-graph-parallelism blog walk~~ CONSUMED — Batch 136 LANDED 2026-09-09 (Plan 115; 3 NotBenchable-annotated concurrency rules 117→120, the try_for_each_init note folded in-rule; full record in the header above).** ~~Original verdict:~~ the walk's candidates all landed as authored: `dynamic-work-parallelism-ladder` (the lead) + `sender-per-work-item-terminates-channel` + `nested-par-inside-channel-bridge-deadlocks`; the async/await speculation stayed a recorded future-work note (NOT a rule) and the unboxed-rayon-spawn patch idea stayed out (upstream API change). Workspace coupling stands: the nested-parallelism deadlock is a LIVE hazard class for workspace rayon code (katgpt-rs + riir-ai).

- **Last landed: Batch 128 (2026-09-07, Research 150 / Plan 109 — the PrismML llama.cpp Metal/FA distill; M3 box, CPU-only): kernel_opt +5 (565 → 570: A 120→121, K 9→11, N 39→41).** Source `PrismML-Eng/llama.cpp` master @ `5ea87dda` (MIT; fresh blob-filtered clone, quotes re-verified at the tip, removed at landing) — the unmined Metal/FA surface of the fork B127 mined for its CUDA slice. Rules: kv-dequant-materialization-batch-threshold-gate (N; commit 63b64a50 — amortization-gated KV dequant `ne[1] < 32` + the always-reserve-the-temp-buffer companion) + offline-device-tuned-table-dispatch (K; f280b269 — generated per-SKU (Q,NE) tables, bucketed context dims, exact → domain-default → family-representative → baseline ladder) + tuned-config-resource-limit-fallback (K; f280b269 — SMEM re-validation at dispatch, exceed → baseline Q=1, never half-clamp) + quantize-scale-math-wide-precision-denorm-safe (N; 78ec4c37 — fp32 quantize transform in the fp16 FA kernel, denorm 1/qd overflow = inf) + extent-clamped-operand-views-partial-k-tile (A; a3027337 — dynamic_extent + view-extent clamp `min(tile, K−loop_k)`, 'Assisted-by: pi:llama.cpp/Qwen3.8-27B'). Skip ledger: 3af988fa (epilogue family), 369e1cd6 (wide-row-copy), b615f5b4 (build-infra). Gates: batch128 3/3 FIRST-SHOT + lib default 1167/0 + kernel_heal 1489/0/1i + kernel_heal+latent 1606/0/2i + retrieval kernel 100/100/100 blended 94.7/96.9/97.7 (byte-identical) + clippy 0 ×3 (doc_markdown ×3 + a line-start `+` list-marker misparse — the B125 lesson re-proven — healed) + score-bench #77 FLAT 96.2/1.9. Pin note: the league pin `e311ed38f` is NOT master ancestry (league tag `prism-b10660-e311ed3`) — future quotes on this fork pin a master SHA. 4090 checked pre-mine (Tailscale; LAN refused = documented IP drift): 18% util, no collision. Records: Plan 109 / Research 150. Pins now current: master `5ea87dda`.

- **Last landed: Batch 125 (2026-09-07, Research 148 / Plan 106 — the ds4 PR rebase-surge distill; M3 box, CPU-only): kernel_opt +4 (555 → 559: A 117→118, D 128→129, E 45→46, N 37→38) + rust_perf io +1 (111 → 112, io 16), all NotBenchable-annotated where honest.** Source ivanfioravanti/ds4-metal @ ffd85d42 (MIT; fresh clone staged by Plan 106 T0, HEAD == ls-remote, quotes verified at the pin, clone removed at landing). THE TRIGGER WAS SMALL — the PR was REWRITTEN onto upstream main, so the compare double-counted two banked populations (B123's upstream window + the B118/121/122 Qwen stack); the true delta was ~5 tip commits. Rules: kv-reuse-prefix-ends-at-last-evaluated-token (A; tool-decode stops BEFORE the assistant end token — trim the terminator from the reuse prefix or the next turn consumes a KV hole; gate on the decode finish reason) + new-syntax-family-enumerates-every-keyed-surface (D; a family added to the renderer must reach EVERY keyed surface — silent KV-checkpoint loss, silent vision-flag loss, loud CLI exit 2; POSITIVE engagement tests; 4010cc6 = the config-surface rider) + interleave-independent-accumulators-only (E; order axis at fixed precision — interleave independent rows only, consumer-visible sums keep textual order) + fixed-point-parity-needs-reference-rounding (N; JFIF <<16 + round-half + clamp — the corpus's first CPU-side instance, domain caveat in-rule) + control-reproduces-on-baseline-trims-overclaim (rust_perf io; failing parity control → reproduce on the baseline FIRST, trim the oracle to consumed outputs under matching compute shape, never widen tolerance) + RIDER on B57 (per-iteration multi-buffer slots delete the back-edge barrier, bit-identical). Gates: batch125 4/4 FIRST-SHOT + lib default 1167/0 + kernel_heal 1483/0/1i + kernel_heal+latent 1600/0/2i + retrieval kernel 100/100/100 blended 94.7/96.9/97.7 (byte-identical) + clippy 0 ×3 + score-bench #75 FLAT 96.2/1.9. LESSON: the doc-lint cascade — appending a `- ` doc bullet re-walks the whole module doc; the B118-B124 `+`-chain prose parsed its line-start `+` as a list marker (30 lazy-continuation findings on untouched lines); fixed structurally (`plus` + the missing blank line), rule recorded in Research 148. Count pins walked: totals 555→559 ×3 (tests ×2 + router) + per-category A/D/E/N + rust_perf 111→112 + annotated 54→55 ×2 + mod.rs lists + corpus.rs doc counts. Record: Plan 106 / Research 148. Pins now current: PR ffd85d42, upstream c0a6119f.**
- **NEXT-UP: Batch 125 (Plan 106) — the ds4 PR rebase-surge re-mine; trigger FIRED 2026-09-07 09:15 +07 (Plan 102 T7 re-scan): PR head `69c38d23` → `ffd85d42` (re-based onto upstream main `c0a6119f`, ~47 new commits in the 09-05 21:48Z..09-06 23:11Z window); upstream main UNCHANGED at the B123 pin.** Lanes: Part A / Batch 125 = Metal SSD-expert-cache + streaming axis (kernel_opt; dedup `ssd|expert|stream|readahead|resident|preload` FIRST — the B123 family; expect riders where content cross-pollinated) + the NEW speed-bench protos (gqa_mma_proto.metal 1640 ln, qsa_prefill_bench.c, hc_chain_proto); Part B / Batch 126 = tool-text preservation family (ds4_tool_text.h + 9 commits) + QA-score/frontier-logit validators (rust_perf); Part C = league intel only (DSpark EOS-early-stop → 721/717 read-only; ROCm gfx1151 foreign-hardware rules only if hardware-independent). Execute from `.plans/106_batch125_ds4_rebase_surge.md`; fresh clone at the pin (force-push history ×2 — never reuse `.raw`).**
- **QUEUED LEAD (filed 2026-09-07, 4090 box — the riir-ai B877 opponent autopsy): ~~PrismML/llama.cpp CUDA ternary MMQ mma family @ e311ed38f (the league opponent pin; MIT; prior fork contact B96/B113/B116 was Metal/format lanes — the CUDA side is UNMINED): `ggml/src/ggml-cuda/{mmq.cu Q1_0/Q2_0/PQ2_0 cases, mma.cuh, mmq-config-{ampere,blackwell,cdna}.cuh per-arch tile tables}` + dispatch fast-path `ggml-cuda.cu:1867` (Q1_0/PQ2_0 @ ne11 ≥ 128) + opt-in Hopper wgmma path.** Route: kernel_opt (CUDA mma GEMM). Lead content: ternary-native (~2.1 bpw feed) m16n8k16-class MMQ pipelining implied e2e at 2.2–2.4× riir-gpu's `gemm_ternary_i8_mma_cuda_raw` (ours 23% of Ada int8 peak vs the fork's implied ~50–54% — B877 arithmetic) — dual-use: the per-arch-config / multi-stage-cp.async / ldmatrix-swizzle class is 3–6 kernel_opt rules AND the structure reference for riir-ai Issue 884's adopt port (T6 there; don't block one on the other). Dedup before any batch: B96/B98 (Metal extraction/telescope), B113 (fa-vec), B116 (PTQ1_0 format), Research 107 (ktransformers W4A16 — the fp16-acc anchor class B877 corrects). Re-verify quotes at a fresh clone per the .raw discipline; the B877 arithmetic lives in riir-ai `.benchmarks/877_b803_closure_falsified_opponent_autopsy.md`.** **MINED 2026-09-07 — Batch 127 (Research 149 / Plan 108): 6 rules, corpus 559 → 565 (A +2, B +1, D +1, H +1, N +1): lut-free-bit-spread-vadd4-unpack (B; the ~1.7x anti-LUT constant-cache serialization + the synthetic-uniform-data GOAT trap in-rule), per-block-int32-chunk-rescale-fp32-accum (N; exact i32-wgmma per 128-block + fp32 outer — the complement of R107's fp16-acc anchor), one-time-dense-repack-keyed-sync-publish (D; tuple-key + publish-after-sync, silent-wrong-answer guards), sm90a-window-shape-gated-wgmma-dispatch (A; exact cc window — cc>=900 alone breaks on Blackwell/tcgen05), streamk-starved-tile-grid-dispatch (A; ntiles<8*nsm cutover + owned-tile direct-store + the atomicAdd non-determinism caveat for replay stacks), moe-broadcast-quantize-once-scatter (F/H; dedup the quantize transform, scatter the cheap artifact) + riders (B98 stock-Q8_0-dot external confirmation, B113 ternary-rides-Q8_0-geometry, B20 two-level byte_perm crumb decode, cp.async ~1.8x measured instance, CDNA4 xf32 removal held pending second instance). League intel: the wgmma path is DOUBLY opt-in (build + env) — default league runs never execute it; the B877 gap attribution carries that caveat. Port notes for riir-ai 884 T6 recorded in Research 149 (J_max sizing trap, dispatch gate checklist, determinism pin). Gates: batch127 3/3 + lib 1167/0 + kernel_heal 1486/0/1i + latent 1603/0/2i + retrieval byte-identical + clippy 0 x3 + score-bench #76 FLAT 96.2/1.9. Org-name trap: `PrismML-Eng`, not `PrismML` (the shorthand 404s); the vendor checkout is NOT at the pin (fresh .raw clone only). Count pins: tests.rs totals x4 (the stale-mutable 559 hiding in batch116_rules_present) + category table A/B/D/H/N + router + mod.rs comments (two pre-existing stale headers healed: E 45->46, A/B B125-era drift) + AGENTS 565 + .docs count row. score_bench combo en-route fix: heal.rs v5_is_removal_rule signature drift (&str vs fn(&String)) — the B125 landing broke the score_bench build without noticing; closure-fixed, re-verified.**
- **Last landed: Batch 122 (2026-09-07, Research 145 / Plan 102 -- the ds4 PR vision-delta distill, Part A of the double trigger; M3 box, CPU-only, zero compute consumers): 3 rust_perf io rules, corpus 105 → 108 (io 9 → 12), all NotBenchable-annotated.** Source ivanfioravanti/ds4-metal branch qwen3.8-flash-next @ 69c38d23 (MIT; .raw clone staged by Plan 102 T0, HEAD re-verified == ls-remote, quotes re-verified at pin, removed at landing). Rules: model-aware-format-over-hand-assembled-tokens (the lead -- hand-pushed user/vision/newline/text token sequences drifted from the chat template; ONE model-aware multimodal formatter owns the turn + atomic rollback) + implemented-surface-requires-provisioning-target (vision fully implemented but the downloader never fetched the encoder -- dead out of the box; named per-artifact fetch target + load-time error NAMES the target) + diagnostics-not-just-exit-code (a REPL exits zero after a failed turn if a later turn succeeds -- scan captured stderr for per-turn failure patterns, save full transcripts, do not grade content). Gates: batch122 gates 3/3 (the parse gate caught bare-statement snippets -- syn reads whole files, snippets wrapped in fn; the gate doing its job) + lib default 1162/0 (count pins walked: total 105→108, annotated 48→51 ×2, 87 benchable UNCHANGED, header ledger io 12 / 21 NotBenchable + 3 ledger lines) + retrieval re-run kernel 100/100/100 blended 94.7/96.9/97.7 (byte-identical to B118/119/121) + score-bench FLAT 96.2/1.9 + clippy 0 default/rust_perf. Record: Research 145 / Plan 102 (Status → RECORD, Part B queued).**
- **QUEUED (Batch 124, post-B123, small): arXiv:2609.04040 (walk #8, confidence-gated admission for prefetching) — YES rust_perf evaluation-design, 1–2 NotBenchable rules.** The confound-disentangle shape: learned predictors are benchmarked against always-issues baselines, conflating PREDICTOR with ADMISSION POLICY; matched controls (the SAME confidence gate on a 257-param MLP and on classical stride) make the neural advantage vanish. Three decision contents: (1) matched-control evaluation design — one variable at a time, the gate is ITS OWN variable (sibling of B121's ab-isolate-one-component-by-baseline-byte-copy, which isolates the ARTIFACT under test; this isolates the POLICY vs the MODEL at evaluation-design level); (2) proxy-vs-endpoint metric split — gate improves accuracy 11%→15% and removes 35% of prefetches, but DRAM reads change ±0.07%: better proxies do not imply better endpoints; (3) the null-arm identity — gate-closed execution reproduces the no-prefetch baseline EXACTLY (a provable no-op arm, the admission-gate twin of our feature-flag bit-identity discipline). External confirmation of the gate-first law (880's kill-switch, evpi_gate, cadence exits). arXiv:2609.02027 (LRU hit-ratio mean-field) = NOTE only, no batch — recorded as the analytic comparator in riir-ai Issue 882's Related. arXiv:2608.30226 (LaMoC) = SKIP — training-time Fisher-blended compression, riir-train-adjacent (routing-restricted), no code-shape rule.**
- **Last landed: Batch 123 (2026-09-07, Research 146 / Plan 103 -- the ds4 upstream-surge distill, Part B of the double trigger; M3 box, CPU-only): kernel_opt +3 (552 → 555; A 116→117, H 24→26) + rust_perf io +2 (108 → 110), all NotBenchable-annotated.** Source antirez/ds4 @ c0a6119f (MIT; .raw clone staged, pin ls-remote-verified, quotes re-verified at pin by 2 subagents + coordinator, clone removed at landing). kernel_opt: batch-pool-read-makes-serial-readahead-redundant (the lane GOAT -- an advisory read-ahead hint for ranges whose next consumer is a queued parallel read pool is duplication, not prefetch; M5 Max prefill 73/77→93/97 tok/s) + cold-large-prefill-whole-layer-lookahead-over-partial-cache (read-pattern dispatch keyed position-OR-layout AND chunk-size ≥256 AND cache temperature, ONE layer lookahead; the named complement of row-addressable-operand-index-not-full-tensor-map; 63→91 tok/s at fixed 35.36 GiB) + static-seed-priority-bounded-to-observed-scale (static preload seeds enter a shared eviction namespace at counter-comparable magnitude 1..=31 or nothing ever evicts; SSD re-reads 94.65→78.18 GiB). rust_perf: full-vector-bit-identity-over-argmax (MEASURED failure class: full-vector drift at 4/4 frontiers where argmax matched; full float32 identity + artifact pre-validation + identical/drift/invalid three-status) + renderer-error-is-not-capacity-overflow (tri-state fit validator; render errors must not fire the destructive compaction path). Gates: batch123 rust_perf gates 3/3 (parse gate caught top-level caller snippets AGAIN -- the B122 lesson, second strike) + kernel category pins walked (A/H counts, 555 ×5 sites, exhaustive membership) + lib 1165/0 + kernel_heal 1478/0/1i + kernel_heal+latent 1595/0/2i + retrieval kernel 100/100/100 blended 94.7/96.9/97.7 byte-identical + score-bench FLAT 96.2/1.9 + clippy 0 ×3. Not landed (ledger in Research 146): vendor-blas-tuned-algo-latched-by-version, k-loop-tail-mask-seed-once, memory-stream-terminator-needs-own-byte, small-append-cache-resident-expert-dispatch. Record: Plan 103 / Research 146 (Status → RECORD).**
- **Last landed: Batch 124 (2026-09-07, Research 147 / Plan 104 -- the arXiv:2609.04040 matched-control evaluation-design distill; walk-#8 headline consumed): 1 rust_perf io rule, corpus 110 → 111 (io 15), NotBenchable-annotated.** matched-control-disentangles-policy-from-model: matched controls separate the ADMISSION GATE from the LEARNED MODEL (the paper's control made a 257-param MLP indistinguishable from gated stride); proxy-vs-endpoint (accuracy 11→15% but DRAM ±0.07% -- better proxies do not imply better endpoints); null-arm identity (gate-closed reproduces the no-prefetch baseline exactly). Workspace hook: external confirmation of the gate-first law (880 kill-switch, evpi_gate, cadence exits). Gates: batch124 1/1 (caught a case-sensitive needle + an ENOSPC-corrupted write) + lib 1166/0 + clippy 0 ×2 + retrieval floors byte-identical + score-bench FLAT. INCIDENT: Data volume hit 100% mid-batch (/tmp/plan047_* sibling dirs ~9 GB untouched; my target cleaned 66 GiB); the ENOSPC write corrupted the rule and the parse gate caught it -- re-read the touched region after any ENOSPC. Record: Plan 104 / Research 147.**
- **QUEUED (Batch 124, small): arXiv:2609.04040 matched-control evaluation-design rule (see the walk-#8 line below) -- can pair with a cuBLASLt/CubeCL vendor-algo analogue if one surfaces.**
[Batch 124 DISCHARGED -- see Last landed above.]
- **Last landed: Batch 121 (2026-09-07, Research 144 / Plan 101 -- the ds4 PR #991 IQ2 delta re-mine; the queue's ds4 trigger FIRED: PR head 236cb2a5 → c5ae2eb5, one LINEAR commit, parent == the B118 pin; upstream main unchanged 9ab7053): 3 rust_perf io rules, corpus 102 → 105 (io 6 → 9), all NotBenchable-annotated.** Source ivanfioravanti/ds4-metal commit c5ae2eb5 (MIT; fresh clone .raw/ds4-metal, HEAD == pin, LICENSE verified; removed at landing). The delta is BUILD TOOLING + MEASUREMENT METHODOLOGY, no kernel code: qwen4_iq2.py (NEW 181 ln, two-stage quantize/assemble) + qwen38-iq2-quality.md (NEW 62 ln) + 6.4K ln results JSON + 2 doc patches. Rules: resume-revalidates-provenance-before-reuse (identity tuple = source + calibration + TOOL hash checked against the manifest BEFORE cache reuse + per-part digest re-verify; failure class = presence-keyed resume mixes outputs across tool versions) + write-verify-readback-before-publish (hidden .incomplete write, template guards BEFORE byte one, INDEPENDENT-reader hash verify per payload -- verifying through the write path confirms the cache, not the disk -- atomic-rename publish + sidecar manifest, refuse-existing) + ab-isolate-one-component-by-baseline-byte-copy (measurement honesty: byte-copy every non-under-test tensor from the baseline artifact so the artifact diff IS the variable [96 trunk gate/up swapped, all else verbatim from the Q4_K build]; PAIRED per-case diffs over raw counts; confounded recipes [Q8 = down weights too, Q4_0 = earlier recipe] named and out of the headline; un-instrumented memory counters [peak RSS vs mmap-backed GPU weights] = PARTIAL budgets, labelled; RIDER: calibration provenance -- same pinned imatrix as baseline, zero-count entries take the deterministic fallback). LEAGUE INTEL (named-not-edited): quantization economics -- hard-suite passes FLAT [IQ2 36/50 vs Q4_K 35/50, same evaluator/seed on M3 Ultra] while BF16-referenced drift is 3-6x worse [NLL 0.27828 vs 0.21287, top-1 91.32% vs 96.57%, MAE 0.13992 vs 0.04307] -- headline metrics hide quant drift, paired probability comparisons expose it; 64 GB feasibility UNPROVEN by the source's own accounting [50.34 GB + 2.92 GiB tracked = 53.48 GB partial budget on a 512 GB box, 'does not validate Metal allocation limits or OS headroom on a physical 64 GB Mac'] -- if a Flash-Next league row ever arms, IQ2 is the candidate recipe pending real-64GB validation; MTP short-prompt greedy parity byte-for-byte [88.9% draft acceptance] consistent with our closed MTP economics, no reopen. Gates: batch121 gates 3/3 FIRST-SHOT + lib default 1159/0 (count pins walked: total 102→105, annotated 45→48 ×2, serde 45→48, io 6→9, member-list +3, 87/102→87/105, doc line) + kernel_heal+latent lib 1589/0/2i + retrieval re-run kernel 100/100/100 blended 94.7/96.9/97.7 (byte-identical to B118/119) + score-bench FLAT 96.2/1.9/100 + clippy 0 default/kernel_heal lib. Record: Plan 101 / Research 144 (Status → RECORD).

- **Next ds4 trigger: FIRED 2026-09-07 05:52 +07 (both axes) — Plan 102 / Batch 122 staged.** PR head `c5ae2eb5` → `69c38d23` (4 commits, 361 ln: Qwen vision download target + model-aware multimodal formatting fix + vision quality results) AND upstream main `9ab7053` → `c0a6119f` (41 files, +6118/−1100: GLM attention/rope + NEW Metal SSD-experts + session-state + SSD-cache tests; fork main synced to upstream). `.raw/ds4-metal` staged at `69c38d2` (gitignored) + upstream ref fetched. Part A (PR vision delta, rust_perf io +3–4 rules: model-aware-format-over-hand-assembled-tokens / implemented-surface-requires-provisioning-target / diagnostics-not-just-exit-code / stdin-path-line-break-guard) = Batch 122 from Plan 102; Part B (upstream surge — kernel_opt, dedup `ssd|state.space|expert` first) = Batch 123, own session. Execute from `.plans/102_batch122_ds4_double_delta_remine.md`.**

- **Last landed: Batch 119 (2026-09-06, Research 143 / Plan 098 -- the ss16118/low-latency-nccl near-SoL collectives distill; M3 box, CPU-only, 4090 idle-checked at 20%/659 MiB; consumed ahead of the time-gated arxiv walk #8 per the Batch-113/117/118 precedent): 2 kernel_opt rules + 1 rider, corpus kernel_opt 550 -> 552 (E +2; entries_gpu_30.rs).** Source ss16118/low-latency-nccl @ 5357eff325eddf978137de7140195a5568fa8a11 (BSD-3 NVIDIA; shallow clone in .raw/low-latency-nccl removed at landing) + the paper surface arXiv:2607.16100 (ETH+NVIDIA). PROVENANCE corrections at the pin: the repo names the sentinel POISON (ncclLLPoisonBuffer/Async fills, ncclLLBufferIsPoisoned polls, the recv Reset arm re-poisons -- no ncclLLBufferInitSentinel); the sizing helper is ncclCalcMinReductionBufferSize (LL doubles the buffer -- 2x scratch verified in code at ll_buffer.cu:74); LL128 ENUMERATED 'NOT YET IMPLEMENTED' + // TODO in the recv path (the LL128-atomic caveat cites the PAPER). Honest finds in-rule: the poison table maps by dtype SIZE; the 'FP overrides' header carries NO specializations; stale comments claim 0x7FC00000/0xDEADBEEF vs the table's 0xFFFAFFFA-class NaN patterns; the 4-byte poison IS a representable i32 (-327686). Rules: sentinel-readiness-over-separate-flag (E, lead -- dtype-poison init + poll-non-sentinel; constraints = never-equal-sentinel [NaN payload for FP / int collision documented], RESET-BEFORE-REUSE [without it the reader consumes stale-as-fresh: the ABA], PEER-STORE-VISIBILITY, SINGLE-WRITER SLOTS => determinism preserved) + ll-flag-in-data-packed-poll (E, the {flag,data} one-vector-store regime twin; epoch band 2..=255 with 0/1 reserved, wrap-aliasing => LL REQUIRES multi-buffering, both-flags-match ~0u retry; the two-regime split by message size IS the decision); both carry the LL128-atomic determinism caveat (atomic-ADD accumulation non-deterministic FP, gamma ~ 3.8e-6 @N=64 FP32, addition-only, NVLink cache-line atomicity -- forbidden for bit-identity/replay-gated consumers). Rider: B58 += the bidirectional peer-exchange credit face ('each receive is implicit permission for the next send'; phrase-needle pins intact). Gates: batch119_sentinel_never_equal_and_reset_before_reuse (NaN deliverability + int-collision pin + the ABA/reset arm + all-slots readiness) + batch119_epoch_band_wraps_and_split_slot_matches (1000-advance band sweep never touches 0/1 + wrap-aliasing + rotation offsets + split-slot match/retry + reset-flag-0). GOAT: batch119 gates 5/5 FIRST-SHOT + kernel_heal lib 1455/0/1i debug / 1456/0 release + default 1142/0 + combined 1574/0 release / 1572/0/2i debug floors HELD zero re-pins + retrieval re-run kernel 100/100/100 blended 94.7/96.9/97.7 of 131 (byte-identical to Batch 118) + clippy 0 in default/kernel_heal/kernel_heal+latent lib + all-targets + score-bench #66 FLAT 96.2/1.9/100/100 (dedicated CARGO_TARGET_DIR=/tmp/batch119_target, removed at landing). Count pins walked incl. the batch116 test's own count pin + ops regen artifacts left dirty as theirs. Record: Plan 098 / Research 143 (Status -> RECORD).

- **Batch 120 (ds4 PR queue re-check) DISCHARGED 2026-09-06 ~00:30 (M3, GitHub API, zero-build): NO DELTA — PR #991 head still exactly `236cb2a549d05ea1941a19bd4f154e0abc858661` (the Batch-118 pin; state open, 6 commits, updated_at 10:33:12Z = BEFORE the Batch-118 mine read) and upstream antirez/ds4 main still `9ab705347c1775e7599ede7eb81a6255ec7dccb5` (the Batch-117 pin).** No re-mine, no re-clone; Batch 118's pins remain current. Next ds4 trigger: a force-push past `236cb2a5` or upstream main movement past `9ab70534`.

- **QUEUED (verdict 2026-09-06, corrected same day from an initial conversation-MARGINAL after owner challenge — the B110 precedent governs: corpus rules serve USER code on hardware we do not own, and 1–3 rules IS the recent-batch norm): ~~arXiv:2607.16100 low-latency-nccl → Batch 119~~ CONSUMED (landed 2026-09-06, see Last landed).**

- **Last landed: Batch 118 (2026-09-06, Research 142 / Plan 097 -- the antirez/ds4 PR #991 (Qwen3.8-Flash-Next) distill; M3 box, CPU-only, 4090 idle-checked at 21%/659 MiB desktop-class + zero compute processes; consumed AHEAD of the time-gated arxiv walk #8 per the Batch-113/117 precedent): 6 kernel_opt rules + 1 rust_perf io rule + 7 riders, corpus kernel_opt 544 -> 550 (A +2, H +1, N +1, D +1, I +1; entries_gpu_29.rs) / rust_perf 101 -> 102 (io; annotated NotBenchable per the B94 precedent).** Source ivanfioravanti/ds4-metal branch qwen3.8-flash-next @ 236cb2a5 (MIT; force-pushed once, HEAD re-verified == pin at mine time; upstream main unchanged at 9ab7053 = the B117 pin so the PR branch IS the delta); two read lanes (kernel surfaces / handoff+io) + coordinator synthesis. Rules: role-scoped-store-must-match-compute-guard (A, the correctness lead -- unguarded store on a role-colliding row_half -> EXACTLY-UNIFORM softmax, invisible four ways, SIX sessions [corrects the verdict's 7]; gate batch118_unguarded_store_yields_uniform_softmax = the production-magnitude exactly-uniform detector + the tiny-magnitude-invisibility arm), tile-amplification-precedes-staging-pass (H, CONVERGED by both lanes -- 16x re-read + 3.2x padding -> one full-chunk tile 125.2 -> 42.2 ms/layer; staging NET-NEGATIVE post-fix; knob-must-reach-the-loop honesty rider), operand-precision-gated-by-consumer-invariance (N, GOAT-grade pass/fail arms +0.0000082 vs MAE 0.0581 + the Q-split null ablation; closure is MODEL-SCOPED -- re-armed default for Flash-Next; generalizes B110), compute-role-split-by-accumulator-pressure (A, NEW axis vs B79 -- 2 score + 4 PV simdgroups, 1.50x standalone / 0%+-1% e2e until the MoE wall moved then +4.3-9%), speculation-scheduler-break-even-flip (D, +3.8% plain decode -> 202 bypass vs 62; honest baseline + 110% margin + window + monotone bypass; acceptance is a PROMPT-FAMILY property; 110% default doc-only, DSpark knobs default 0), running-threshold-prune-compaction-aliasing-guard (I, 253 -> ~60 ms/layer, memcmp == 0 test survives; the in-place-compaction ALIASING GUARD mandatory), parallel-disjoint-pread-hides-cold-page-latency (rust_perf io, ~20x cold QD1 -> QD32, warm neutral, floor 512 + cap 64 + degrade-to-inline; NEAR-adjacent = B94 coalesce's opposite regime). Riders (7): B24 xor-butterfly 8-lane sub-simdgroup twin; B95 GDN second family + NSG cooperation-radius invariant; B77 hc-inject derived-sum + low-rank activation reuse; B57 hc_chain canonical + red[16] slot-partition complement; stripe-semaphore monotone-counter all-arrive GRID barrier + residency deadlock-freedom + recompute-to-avoid + the 9-49% fusion negative; block-reduction-scratch shuffle-lane unmutated-partial face (gate batch118_indexed_shuffle_ladders_must_read_unmutated_partials: indexed ladders double-count v0+4v1+2v2+v3, xor butterflies safe, max survives by idempotence); smem-envelope short-K occupancy-first face (1.7x on threadgroup bytes alone). Corrections vs the verdict applied: '7 sessions' -> SIX; 'B98 NSG ladder' -> mis-mapping (B98 = tiny-rows-scalar; the NSG content rides B95); ds4_qwen4.c SUPERSEDED -- items (a)(c)(d)(f)(g) handoff-verified + corroborated, (e) code-live. GOAT: batch118 gates 5/5 + kernel_heal lib 1450/0 + default 1142/0 + combined 1567/0/2i floors HELD zero re-pins (retrieval re-run kernel 100/100/100, blended 94.7/96.9/97.7) + clippy 0 three states lib + all-targets + score-bench #65 FLAT 96.2/1.9/100/100. Count pins walked: the B22 FULL-STRING pin regenerated (1600 -> 2437 chars) + the batch116 test's OWN count pin (the 20th site) + rust_perf annotated 44 -> 45 / unannotated 57 held. DRIVE-BY guard repairs (sibling fresh code, the B59/batch98 precedent, cited in the landing commit): d4aad87's frontier_pairs left ungated on its rustc_errors import -> cfg(any(rustc_errors, latent_retrieval)) [Layer 5a --no-default-features E0433]; 0b1e9c9's 5-wide edit tuple broke the fix_verify bin map -> 5-pattern + _note; 0c167b2's manual_checked_division in the G8 print -> checked_div. Record: Plan 097 / Research 142 (Status -> RECORD; .raw/ds4-metal removed at landing).

- **QUEUED → CONSUMED (verdict 2026-09-06; LANDED as Batch 119, see Last landed above): arXiv:2607.16100 low-latency-nccl (near-SoL GPU collectives) → Batch 119 — YES B corpus / NO lane mapping (league + workspace are single-GPU; zero collective surface).** Source arXiv:2607.16100 (ETH+NVIDIA, Shen/Hoefler et al.); code repo `ss16118/low-latency-nccl` **cloned at mine time per this line's own instruction — @ 5357eff325eddf978137de7140195a5568fa8a11, BSD-3 NVIDIA**; the mine-time provenance corrections vs this queue line: repo names the sentinel POISON (ncclLLPoisonBuffer/Async, no ncclLLBufferInitSentinel), sizing helper = ncclCalcMinReductionBufferSize (2x scratch verified in code), LL128 ENUMERATED NOT-YET-IMPLEMENTED (the caveat cites the paper). Lead: **sentinel-readiness-over-separate-flag** (B: dtype `-NaN` sentinel init + poll `val != sentinel` replaces the flag array + the data→flag ordering fence; full bandwidth preserved; constraints = never-equal-sentinel (FP32 sign-bit NaN payload; int-dtype collision documented: 0xFFFAFFFA = i32 -327686), reset-before-reuse, peer-store visibility, single-writer slots ⇒ determinism PRESERVED), then **ll-flag-in-data-packed-poll** (B−: 16B atomic `{flag,data}` poll flag==epoch — the tiny-message regime twin; halves bandwidth, kills the signaling RTT); both carry the LL128-atomic determinism caveat as decision text (flag-via-atomic-ADD = non-deterministic FP, γ_{N−1} ≈ 3.8e−6 @N=64 FP32, addition-only, NVLink cache-line atomicity — forbidden for bit-identity/replay-gated consumers). Rider → B58 (bidirectional double-buffer credit: "each receive is implicit permission for the next send"). DROP: LL128-atomic as a rule (hardware-bound + non-deterministic); SoL floor methodology (`2·L_L2RTT + L_remote_store`, threadfence probe) stays note-level — no collective lane exists to consume it. Measured basis is the PAPER's own (barrier ≥1 µs each ≈ 40% of small-message AR; 11.0 → 2.37 µs @4×GB200; SoL 1.404 µs, one-shot within 7%) — absolute numbers, not our cells. Dedup: zero flag-in-data/sentinel coverage; all `semaphore|spin|acquire|poll` hits are barrier-class (B92 = the replaced baseline). Record: Plan 098 / Research 143.

- **QUEUED (verdict 2026-09-06, Research 142 / Plan 097): antirez/ds4 PR #991 (Qwen3.8-Flash-Next) distill → Batch 118 — YES B+ corpus / LEAGUE INTEL HIGH.** Source ivanfioravanti/ds4-metal branch `qwen3.8-flash-next` @ `236cb2a549d05ea1941a19bd4f154e0abc858661` (MIT; upstream main unchanged at the B117 pin `9ab7053` — the PR branch is the entire delta, 6 commits: native Metal path for Qwen3.8-Flash-Next, 48 layers / 36 GDN + 12 GQA w/ QSA / 512-expert MoE / built-in MTP / external PLE n-gram sidecar; `metal/qwen4.metal` 2,648 ln + `ds4_metal.m` +1,233 + `QWEN38_PERF_HANDOFF.md` 2,103 ln / 21 measured sessions). Lead GOAT: **role-scoped-store-must-match-compute-guard** (M; handoff 26c — an unguarded fragment store contributed wrong-role-scope rows and produced EXACTLY-uniform attention for 7 sessions while every gate passed small-magnitude fixtures; the rule carries a falsifiable uniform-softmax detector at production magnitude — a correctness rule, not a perf rule). Then tile-amplification-precedes-staging-pass (D; 512-row MoE tiles = 16× weight re-read + 3.2× padding → full-chunk tile, 125→42 ms/layer, staging-pass negative as boundary content), operand-precision-gated-by-consumer-invariance (D/rider vs B110, adjudicate at mine time — scorer MMA +0.000008 vs attention MMA MAE 0.0581, Q-split proved the P-tile binding), speculation-scheduler-break-even-flip (D; +3.8% plain decode flipped the MTP scheduler, fix = 110% cumulative-ratio margin), parallel-disjoint-pread-hides-cold-page-latency (rust_perf io; cold PLE gather 6.9 s → 338 ms @ 32 threads, byte-identical) + 2 rule-or-rider stragglers (running-threshold prune + aliasing hazard; pipeline phase-role split — third role-split axis vs B79/B102). Riders 8–12 mapped (B24/B103/B110/B95/B57/B108/B98/B77/B106/B92/B29). **LEAGUE INTEL (named-not-edited — sibling acts):** the league Flash-Next watch row (`../riir-ai/.docs/09_performance/perf_rematch_league.md`) fires its content condition — this pack is ~100–113 GB, NOT ≤24-GB-feasible, so the re-arm trigger stays unlit but the row should note the second Flash-Next candidate + the PLE sidecar economics; riir-ai Issue 769 (ANE) gets a THIRD external confirmation (ds4's own ANE offload is a measured negative in the handoff); Issues 717/721 (spec decode) CONFIRM-only (MTP economics match our closed negatives, no reopen). Absolute numbers M3-Ultra-only — not headroom evidence. **Slot: behind arxiv walk #8 (time-gated Mon 09-07 ~07:00). PR is OPEN + force-pushed once — re-clone at pin + re-verify quotes at mine time.**

- **2026-09-06 unit (4090, this box) — the Plan-095 score-bench difficulty axis: generation 6 = the real-repo shape ladder, candidate #2's premise FALSIFIED, the salvage ADMITTED (95.2% < v1, Bench 073).** Three extraction granularities measured (1,247 lone fns → 0 multi; adjacency runs → 1 same-class pair; file-header carves over 8 sweeps → 4 organic multi) — the bare-stub filter is the binding constraint (v1 scope limit; the compilable crust is lint-free). The salvage mechanized v1's own provenance note as the FILE-HEADER CARVE: 17 real spans (13 single + 4 organic multi, 6 classes, 4 repos) mined at the parent of each repo's first heal sweep; freeze 17/17 at the BENCH instrument, 0 exclusions; territory single 13/13 = 100% (real shapes solve at parity — the "single < v1" prediction REFUTED) vs multi 7/8 = 87.5% (the one miss = the Issue-028 comment-guard class re-found organically). New tool `examples/score_bench_mine_v6.rs`; CLI `--generation 6`; rows #53-#56 + `score_history_v6.svg`. The multi rung stays anecdote-sized until per-fixture stub manifests (v1 Phase 3) — owner-gated. Record: Plan 095 (§1a/§1b) / Bench 073.

- **Last landed: Batch 117 (2026-09-06, Research 141 / Plan 090 -- the antirez/ds4 Metal tensor-prefill delta distill, the intake lead consumed ahead of arxiv walk #8 per the Batch-113 precedent; M3 box, CPU-only -- 4090 at 86% GPU under a foreign workload): 5 rules + 8 riders, corpus 539 -> 544 (L +1, A +1, C +1, M +1, F +1; entries_gpu_28.rs).** Source antirez/ds4 @ 9ab7053 (MIT) re-cloned at the pin -- ~14 commits past the Batch-97/98 pin 110afdd; the intake's mechanism guesses PARTIALLY CONFIRMED: routed-activation reuse is NOT the same-input family (a tensor-pipeline operand MATERIALIZATION -- dense layout for the mma tensor op -- distinct from B77's GEMV memo and the B58/B63/B97/B98 launch-merge/smem shapes), MXFP4 prefill-activation reuse is enablement-only NOT-LANDED (source refuses attribution; non-uniform 2K +9%/4K +2.8%/8K -2.3%; the control-path 8K/16K nondeterminism flag recorded in-rule), and the 0e9cc2d masking fix yielded a SURPRISE rule (dense-fast-path-bounded-by-selector-coverage F: dense substitution legal exactly through top_k + anchor_pool - 1). The CUDA scratch commit yielded device-affine-scratch-admission-with-pool-fallback (A: the admission tuple's device-affinity check is the multi-GPU illegal-access guard; capture-safety motivation = the remove-the-alloc complement of B58). Two gates + the B57/B35/B23/B103 riders as planned. GOAT: batch117 gates 3/3 (the parse gate forced Rust-shaped pseudocode rewrites on 3 entries) + kernel_heal 1389/0 + default 1086/0 unchanged + combined 1504/0/2i floors HELD (retrieval gate re-run explicitly 1 passed) + clippy 0 in three states + score-bench #45 FLAT. Count pins walked incl. TWO surprises: the batch116 test's OWN count pin (the 19th site) + the batch23 FULL-STRING concept needle regenerated with the B23 rider (+996 chars -- the B22 lesson, second application). Record: Plan 090 / Research 141 (.raw/ds4 removed at landing).

- **2026-09-06 unit 7 (M3) — the last unclaimed sweep edit cleared; queue now empty until arxiv walk #8.** The mmorpg deferred edit (unit-5: skipped, sibling-active) was re-claimed after the 092/097 lanes landed: dry-run found **2** `semicolon_if_nothing_returned` sites — `src/headless.rs:1033` (`HeadlessNode::tick`, a NEW V2-wave straggler) + `src/scenarios/runner.rs:658` (the unit-5 site; its sibling else-if branch already carried the semicolon) — landed mmorpg `35b9867` (compile green `--features scenario_runner`, the gate that compiles runner.rs; real clippy 0 in the class; tests/ + crates/riir-bevy dry-run 0 edits → mmorpg territory COMPLETE). Workspace open-issue survey (item 1): katgpt-rs / riir-clippy / ndb / game-sdk / dapps / dao / deployer / seal-remake / riir-auth have ZERO open issue files; riir-train open issues are the consolidation agent's lane; riir-ai = 851/876 (active agent) + 769/844-T6 (owner-gated) + holding records; chain 110 owner-gated (ESP32); seal-game-editor read-only. **4090 check before mining: 3 cargo + 4 rustc processes active on the box → Batch 117 NOT claimed per the standing rule** (and no named 117 target exists — the next mining input is arxiv walk #8, time-gated Mon 09-07 ~07:00). Convenience corpus counts below refreshed to post-Batch-116-addendum (539).

- **CORRECTION (2026-09-05 19:48, owner observation "docker is fine"): the Docker Desktop cred-helper wedge was TRANSIENT and has self-recovered** — re-tested live: `docker-credential-desktop get/list` respond in 0.09-0.16s (were 15-25s timeouts), `docker pull alpine:3.19` succeeds in 7.5s (was "error getting credentials"), `docker build --check` with a registry base finds its warnings (was DeadlineExceeded at 61s), tier2 gate 4/4 in 7.0s. The "restart does NOT fix" claim is RETRACTED — the app restart (15:30) plus elapsed time (or the owner's own interaction) recovered it; attribution honestly unknown. The ports-3128/5555 inference was WRONG (still closed while healthy — those are VM-internal, not host-loopback dependencies). **What stands**: the hang WAS real for ~75 min (14:37-15:45, sampled `open()` blocking) and any future recurrence is now bounded by buildcheck's CALL_TIMEOUT; the calls `(a)`/`(b)` timeline above is unchanged.

- **2026-09-05 unit (M3, later) — the FULL CI GUARD ROT-CHECK discharged; 3 REAL breaks + 1 oracle-liveness gap found and fixed.** The honest-skip debt (B93→B112, ~20 batches of "each affected state validated individually") was paid down in one ~25-min run. Found, all fixed in one commit: (1) **3 clippy `cast_lossless` errors in the B111 ue8m0 gate** (`corpus/tests.rs`, landed `d710353f`) — `byte as i32`/`b as u32` under 1.98 `-D warnings` in the lib-test state the per-batch GOAT clippy states didn't cover; fixed to `i32::from`/`u32::from`. (2) **`single_match` in the kat sync grant block** (`bin/heal.rs`, Plan-079 code) → `if let Ok(key)`. (3) **kat_wetrun_gwet fixture was stale TWO ways** — the mock's `post_sync` healthy arm answered the OLD `{"recorded": n}` wire contract (f5cce1e's tightened gate needs `accepted:true` + per-row counts; the REAL service's contract test is the authority), and the Issue-010 diagnostic assertions pinned the RETIRED phrases (STRUCTURALLY UNREACHABLE / NOT retryable / OWNER act) instead of the T4.4a two-cause vocabulary — 5/11 G-WET tests red at HEAD, invisible since 09-05 03:28 because NO pinned suite ran the target. (4) **t48 GP verdict constant stale** (pre-existing at HEAD `7e74abc`, verified via stash): corpus growth since 08-31 (Batch 91 +4 clippy entries, B110–112 perf entries) moved GP @3 to 2 points below random (0.220 vs 0.240) — no longer within-1 "noise-equivalent"; verdict re-pinned `NEGATIVE-noise-equivalent` → `NEGATIVE` with the re-measure recorded in the doc block (McNemar b=76 c=1 z=+8.55 AGAINST GP, conclusion unchanged). (5) **docker_buildcheck oracle hang → bounded**: `docker-credential-desktop get` wedged box-side for ~75 min (14:37-15:45, sampled `open()` blocking), so every Tier-2 `docker build --check` on a non-scratch base hung and burned the layer's 3600s budget; `run_raw` now enforces a 120s CALL_TIMEOUT (~50x the slowest healthy call) via try_wait-poll + pipe-drain workers, degrading to `Unavailable` (a hang is 'not checked', never a pass); the guard's 5c gained a slow-test awk probe naming any test the libtest watchdog flags ≥60s. ~~"restart does NOT fix"~~ — RETRACTED same evening (owner observation + re-test 19:48: helper responds in 0.09-0.16s, pull 7.5s, tier2 4/4 in 7.0s; the wedge was transient and self-recovered after the 15:30 restart + elapsed time; the ports-3128/5555 inference was wrong, they are VM-internal and irrelevant). The CALL_TIMEOUT stays: the hang class is real, only its duration was not permanent. (6) Drive-bys: the Issue-070 T1 test's doc-markdown + too_many_lines + cast_precision_loss findings (sibling's fresh code, invisible to their default-state clippy) mechanically allowed per the B59/batch98 precedents; TWO latent awk bugs in the guard's own 5d layer (`tests[\/]` nonterminated class on macOS BWK awk — both patterns rewritten class-free and verified against run9's real log). **Guard record: layers 1–5c ALL PASS on run9 (instrument 64/64 targets observed, zero-test check clean); run10 = the first full END-TO-END green incl. the fixed 5d tail.** clippy 0 all-features lib; all-features lib 1940/0/16i; t48 4/4; kat_wetrun_gwet 11/11; tier2 buildcheck 4/4 (3.7s — the CALL_TIMEOUT works: same test took 397s under the wedge).

- **QUEUED (verdict 2026-09-05, Research 137 / Plan 086): llama.cpp v0.4.0 DELTA re-mine → Batch 113 — YES B+.** Source ggml-org/llama.cpp v0.4.0 @ 5266f24da75d (MIT; shallow clone read + removed). B104's fa-vec pins sit INSIDE the release window — delta re-mine, fa-vec infra NOT re-mined (post-pin movement = more SKU table rows, B104's prediction held). GOAT lead PAIR from `kernel_ssm_scan_ssd_mma_f32` (ssm.metal:300-473, read at pin): **chunk-transition-consumed-by-mma** (G — per-chunk C·Bᵀ Gram built once + reused across channel tiles, intra+inter outputs on simdgroup MMA, five-clause use_mma gate, mma_tokens/sequential-tail dispatch + snap_reserve rollback interop) + **carried-state-reduction-stays-scalar** (D — the source's own contract at ssm.metal:454: "Reassociating this particular product with MMA compounds rounding differences at every chunk boundary" — outputs may ride the matrix unit, the carried-state reduction may NOT). Plus **shader-fastdiv-mulhi** (utils.glsl:1-37 — umulExtended mulhi + round-up >>L + u32 L-packing + quotient-zero branch-skip; B59's GPU twin) and **sparse-attention-kv-budget** (fa.metal — index-list + -1 sentinel pad + hint-clamp; B-, rule-or-rider at T1). Metal top-k radix = B107 confirmation + fixed-cost shape gate. ~12-15 rider confirmations mapped in Plan 086 T7 (B19/22/23/38/44/46/57/58/65/71/76/77/92/96/98/104/105/107/110). Kronecker FWHT non-pow2 was REVERTED upstream (#28184) — regime evidence only. **LEAGUE FLAG:** upstream fa-vec table COMPLETE for M3 Max at v0.4.0 (#28373) — fork pin e311ed38f lacks fa-vec; future opponent re-pin moves M3 prefill cells. Slot: after arxiv walk #8.
- **Last landed: Batch 113 (2026-09-05, Research 137 / Plan 086 -- the llama.cpp v0.4.0 delta re-mine; M3 box, CPU-only; executed ahead of its soft arxiv-walk-#8 slot since the walk is time-blocked): 4 rules + 18 riders, corpus 518 -> 522 (G +1, D +1, N +1, I +1; entries_gpu_25.rs).** Source ggml-org/llama.cpp @ 5266f24da (v0.4.0, MIT) re-cloned at the pin; quote-verification lane re-read all five surfaces, THREE attribution corrections (XOR-swizzle + MoE-fusion CUDA-side; --lazy-mode app-layer; Metal-4 gate = has_tensor + name-gate incl. M6/A20 with measured reasons). Rules: chunk-transition-consumed-by-mma (G, lead half 1: per-chunk Gram built once, reused across channel tiles; five-clause gate; kernel-FAMILY tail) + carried-state-reduction-stays-scalar (D, the sharper half: the source's own contract -- non-compounding products may ride the matrix unit, the carried state stays scalar in token order) + shader-fastdiv-mulhi (N: mulhi round-up + byte-spaced L-pack + quotient-zero skip) + sparse-kv-index-list-sentinel-padded (I: ascending index list, -1 sentinel to the tile multiple, mask = single source of truth, trust-but-clamp). Riders: 16 kernel_opt + 2 rust_perf name-fields (full map in the AGENTS.md entry). League flag: v0.4.0 completes the fa-vec M3 Max table (#28373) -- recorded in riir-ai perf_rematch_league.md. GOAT: batch113 gates 3/3 FIRST-SHOT + kernel_heal 1371/0/1i + default 1086/0 + combined 1486/0/2i floors HELD zero re-pins + clippy 0 four states + --all-targets clean. Record: Plan 086 / Research 137.
- **Last landed: Batch 115 ADDENDUM (2026-09-06, Research 139 landing addendum 2 / Plan 088 -- the collision salvage; 4090 box, CPU-only): a parallel session's duplicate Batch 115 yielded per push-wins (the Issue-665 / B103-addendum class; the duplicate's own quote pass had already caught the lane-C radix misattribution the landed batch codes correctly); the salvage the landed batch lacks = 4 rules + 6 riders, corpus 528 -> 532 (D 121 -> 124, E 41 -> 42; entries_gpu_26.rs addendum section).** Rules: dispatch-cliff-bitexact-shape-confinement (D -- the width-wall lead: the sdpa `qL * gqa <= 32` family-switch changes accumulation order -> top-2 VALUE drift; fix = shape confinement + forward/replay SHARED boundary constant), readout-trim-needs-committed-id-coverage (D -- the dedicated guard for the 49,152-trim negative: committed-id histogram + escape range + promotion-gate verification), dead-output-rows-state-append-only-path (D -- KV-only row path + packed KV-only weights + fail-closed capability), distinct-index-bitmap-ascending-compaction (E -- counting sort by SET IDENTITY: distinctness deletes the tie machinery; the ordered sibling of ordinal-map). Riders: B96 bit-plane += the in-consumer offset-sum placement arm (Qwen35.swift:3914 -- CORRECTS the landed batch's 'already covered' claim: the covered arm is the HOST-side once-per-slice sumy, this is the in-kernel per-block recompute); B97 += the verbatim-body text-identity construction + weak-identity sidecar (Qwen35.swift:2184); B104 x2 (DIRECT_NIBBLES spelling axis + register-cliff re-sweep, quantized.h:1958; worst-domain 2.3e-4/13x on semantic pruning); B100 += the host-side grid-trim witness (67--80% no-op groups); exhaustive-domain-input-patch += the per-OUTPUT fidelity-exclusion half (beta, e3b4531d9:27). GOAT: batch115 + addendum gates 8/8; kernel_heal 1379/0; default 1086/0; combined 1494/0/2i floors HELD; all-features 1934/0/16i; clippy clean; score-bench flat 96.2/1.9/100/100. Count pins walked (corpus 532, D 124, E 42, router 532). Record: Research 139 landing addendum 2.
- **Last landed: Batch 115 (2026-09-06, Research 139 / Plan 088 -- the qwen-3.8-mtp-challenge submission-diff-history walk; M3 box, CPU-only; the RE-GRADED YES-B+ verdict consumed -- B100 read the tip TREE, this batch read the ~50-diff HISTORY, complementary): 6 rules + 8 riders, corpus 522 -> 528 (D +3, M +2, N +1; entries_gpu_26.rs).** Source Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06ac (Apache-2.0) re-cloned with FULL history at the pin; two read lanes (A prefill / B decode+draft-head+apparatus) + coordinator synthesis; both seed rider verdicts resolved (nibble digit-fold = the B96 rider's two-posture delta; in-kernel input-side offset sum = ALREADY COVERED by B96's bit-plane rider). Rules: `ordinal-map-exact-tail-selection` (D -- a reference sort's tail-K replaced EXACTLY by order-encoding its TOTAL order into a monotone u32 key; stable ties toward the HIGHER index, NaN above all, signed zero folded; same-set + tie-forcing + positive-control gates; 14-dispatch full-argsort stub -> 2), `complete-coverage-scatter-collapses-operand-to-dense` (D -- island row-COVERAGE decides dead compute: a complete-permutation scatter means every packed row is overwritten unread -> one dense matmul + an install-time argSort-inverse reorder; partial keeps the scatter; lazy eligibility when install ordering makes the replaced gate unknowable; the measured 49,152 head-trim REGRESSION (accept 1.00 -> 0.877) folded into the decision content as the retention direction of the same coverage lens), `exhaustive-domain-input-patch-bit-identity` (N -- a 16-bit input domain is exhaustively sweepable (65,280 finite); patch ONLY the divergent inputs via direct output-word mapping before the lossless expansion; exhaustive zero-mismatch gate + boundary negative control; f32 unsweepable -> tolerance stays), `derived-index-from-pinned-checkpoint-is-a-cache` (M -- a warm-time index from pinned tensors alone is CACHE-class: legality boundary + canonical order (runtime-derived == offline-screened) + FAIL-LOUD over silent fallback (a plausible time for the wrong mechanism) + one attempt forever), `warm-reachable-dispatch-signatures-before-timed-window` (M -- JIT backends compile per signature (shape/dtype/input-count/host-device-mix) and an ADAPTIVE schedule reaches a SET: warm the reachable set with token-neutral throwaway state, the scored EXPRESSION not a call prefix, extend with receipt when the schedule changes), `env-gate-reaches-and-arms-the-scored-process` (D, measurement-honesty #2 -- sanitizer-allowlist spelling (a gate outside the allowlist runs both arms on the same code) + FORCE-SET injected defaults into first-use-cached knobs (ambient stock defeats no-overwrite) + durable arm witness not stderr + bare-leg no-env receipt). Riders (8): B63 boundary-fused residual pair (external confirmation + eval-ladder-BOTH-HALVES scheduling delta); B96 activation-side digit-fold + two-posture exactness (fold-side vs shift-side by downstream-exactness); B97 single-rounding-boundary fusion member; B38 unconsumed-variant factory skip (the JIT cost at the factory gate, flag-consistent); B107 workgroup-radix scope + ordinal-bitmap tie-cut + bf16 16-bit EXACT key (halves the passes, kills the refine pass); B51 gathered-operand dequant-at-use in a fused norm; B82 shapeless-trace Slice-fatal (shape-generic compile impossible for split-boundary ops -- replay-time fatal); B77 derived-sum-table member (output-independent + on-device same-arithmetic + cross-consumer amortization). Mine-time finds (record-only): a STALE case-8 dispatch comment live at the tip (argues IPG-3 while the shipped instantiation is IPG-4; stale across >=4 graders / 5 days -- the M8 cell flip-flopped 3->4->3->4 and M5 3->5->3->5 across Accept/Validate commits, the B104 no-harm discipline is the standing fix); commit 5068eb8 is an EMPTY commit (arena accepts unchanged trees as score re-rolls); prefill asyncEval nearly inert at this shape (118.7 ms enqueue vs 4043 ms GPU block). GOAT: batch115 gates 5/5 FIRST-SHOT (incl. the ordinal-map stable-sort-semantics proof over 4000+ corner-heavy values + the coverage dense/partial non-vacuity gate) + kernel_heal 1376/0/1i + default 1086/0 + combined 1491/0/2i floors HELD zero re-pins + clippy 0 (default / kernel_heal / kernel_heal+latent / --all-targets) + score-bench flat. Count pins walked (tests.rs totals/expected-array D121/M17/N34/sums/grouped + router.rs 528 + mod.rs doc/member lists/category comments). Record: Plan 088 / Research 139 (Status -> RECORD).
- **Last landed: Batch 114 (2026-09-05, Research 138 / Plan 087 -- the oMLX ANE-prefill + GDN blocked-sequential distill; M3 box, CPU-only; the shelf row-5 named-unmined lead + the PR3298/2853 follow-up consumed): 2 rules + 6 riders, corpus 516 -> 518 (G +1, D +1; entries_gpu_24.rs).** Source jundot/omlx @ e467261 (Apache-2.0) re-cloned at the pin; a quote-verification lane re-read all ten surfaces with exact line numbers. `recurrence-prefill-exact-restructure-before-reformulation` (G, the A- GOAT lead -- restructure the EXACT sequential recurrence's memory layout BEFORE an algebraic reformulation: stock per-slice k/q re-read = 32x redundant traffic, restructure = 8x fewer threadgroups per k/q row + register-resident state; ~2x @16K on M3 ULTRA source-reported; the WY/FLA chunked form a LOSER x3 independent + our Bench 734/769 negatives) + `ane-bank-split-ladder` (D -- near-half-first split ladder at program-create mapping failure (0x20004; our 0x50004 the same class); async-release gc+settle BEFORE the headroom re-measure; 70% phys-footprint jetsam gate (4 bounded attempts hit 48.3 GB on 48 GB, source-measured); bank-count DETERMINISM axis; fail-open snapshot; feeds riir-ai 876/769). Riders: B84 token-local recurrent-safe fraction cap; B84 pad-tail profitability + single-prompt guard; B22 sub-simdgroup cooperation radius (FULL-STRING pin regenerated); B95 whole-sequence at prefill grain; B107 program-shape vs delivered-chunk-width; B79 scalar-staging pad boundary. Honest notes: the 'carried-state-reduction-stays-scalar' comment is Batch 113's llama.cpp content, NOT this source; Kernel S has no channel-tile loop (Kernel B's). GOAT: batch114 gates 3/3 FIRST-SHOT + kernel_heal 1368/0/1i + default 1086/0 + combined 1483/0/2i floors HELD zero re-pins (kernel 30 labeled 100/100/100, blended 94.7/96.9/97.7) + clippy 0 four states. Count pins walked; .plans/.highwater repaired 87->88, .research 138->139 (missed-bump class from the sibling's 088/139 filings). Record: Plan 087 / Research 138.
- **Last landed: Batch 116 (2026-09-06, Research 140 / Plan 089 — the PrismML fork ternary + verify-batch distill; 4090 box, CPU-only; the 09-06 watch-triage row below consumed): 6 rules, corpus 532 -> 538 (A +1, D +3, E +1, N +1; entries_gpu_27.rs) + 3 riders.** Source PrismML-Eng/llama.cpp origin/prism @ 7113f6462 (Apache-2.0, 12 commits past the league pin e311ed38f; both lanes quote-verified via git show, vendor checkout untouched). Rules: activation-bitplane-popcount-word-dot (N; lossy int8 activation bit-planes + AND+popcount consume, default-off upstream, behavior-axis gated — honest unmeasured-timing status recorded), scratch-sizing-shares-dispatch-eligibility-predicate (D; tail-carved scratch, one predicate both allocator + encoder consume), byte-owning-threads-non-positional-digit-codes (A; threads own BYTES for non-positional packings, 29.8->136.4 source-reported, upstream's own negative-results ledger in-rule), fwht-wide-block-row-per-threadgroup-exchange-ladder (E; fills the register-resident butterfly's named beyond-budget arm — boundary-pointer-is-not-ownership), shader-shape-cutover-keys-on-resource-block-ratio (D; two-term disjunction width OR n/subgroup_size, no-op self-check, recorded specialization geometry), same-input-folded-transform-memo-per-build (D; graph-build CSE keyed (input, rotation), key-completeness invariant source-asserted-by-comment). Riders: telescoped-digit += mantissa-bound exactness (the load-bearing delta — the base-3 form is bit-exact WITHOUT a power-of-two argument), bridge-rung += register-budget sizing + exact-fit nr1=3 + per-type crossover, dispatcher-eligibility += 4 inverse-direction instances. Kills: multi-column Q1_0 standalone (COVERAGE of bridge-rung), PTQ1_0-extraction standalone (radix sibling), per-expert-instantiation standalone (dispatcher class). GOAT: batch116 gates 3/3 (needle self-verify caught 3 mismatches pre-landing; the parse gate caught one WGSL-isms snippet; the compile gate caught the generator's missing closing quote — the B37/B111 lesson classes live) + kernel_heal lib 1382/0 + default 1086/0 + kernel_heal+latent 1497/0/2i floors HELD zero re-pins + all-features 1936/0/16i + clippy 0 both states (5 doc_markdown backtick fixes in the new //! header — the B37 lesson, 14th strike) + score-bench #43 FLAT 96.2/1.9/100/100. Count pins walked: tests.rs (total x2 + comment + expected-array A/D/E/N + sum + grouped + batch-list line) + router.rs 538 + mod.rs (doc line + module decl + extend + 4 category comments + 4 list tails). Record: Research 140 / Plan 089.
- **Last landed: Batch 116 ADDENDUM (2026-09-06, Research 140 landing addendum 2 / Plan 089 collision note -- the Issue-665 collision salvage; M3 box, CPU-only): 1 rule + 2 riders + 2 standing numeric gates, corpus 538 -> 539 (N 35 -> 36; entries_gpu_27.rs addendum section).** A parallel M3 session authored the same batch concurrently from the same 09-06 watch entry; the landed batch (above) owns per push-wins, the duplicate yielded before commit. THE SALVAGE = what the landed batch genuinely lacks: swar-lane-widened-base3-dp4a-dot (N -- the int8 dp4a CONSUME family of the same base-3 codec; the landed batch owns the float-pipe extraction family via byte-owning + the B98 telescope rider; 16-bit lane widening so the radix multiply cannot carry between bytes; answers the packing rule's 'dp4a can't consume base-3' limitation; the landed batch's own kills respected -- no extraction standalone, no multi-column standalone, no per-expert standalone), riders: digit-LUT-vs-arithmetic per-path split [142.1-vs-143.5 dependent-load negative] + coarse-scale format retention with the FOUR-surface registration completeness [traits/quantize-dispatch/ftype-map/row-validation, four different failure modes] on the new entry + the guard-side mirror face on dispatcher-eligibility [an assert widened to admit F16 lost its buffer-layout conjunct, ~3.8x wdata overrun if reached -- the face the landed 4 instances do not cover]. Gates: batch116_addendum_base3_mantissa_bound_exhaustive [turns the landed rider's prose criterion into a pinned gate: exhaustive radix-3 256x5 + the radix-7 within-bound control + the radix-11 falsifiable negative that MUST diverge] + batch116_addendum_swar_lanes_exhaustive [per-lane 256x4x3-neighbor sweep + the exhaustive adjacent-pair carry-channel sweep]. Salvage NOT needed: fold-CSE / ratio-cutover / multi-column / popcount / scratch-predicate / byte-owning were all covered by the landed batch, several more completely than the duplicate's versions. GOAT: batch116 7/7 (their 3 + addendum 4) + kernel_heal lib green + default green + combined green floors HELD + clippy 0 all states. The yielded duplicate's corpus text preserved at target/batch116/mine/ (gitignored). Record: Research 140 addendum 2 / Plan 089 collision note.

- **Last landed: Batch 112 (2026-09-05, Research 134 / Plan 083 -- the nnethercote July-2026 byval-move memcpy-threshold distill, rust_perf layout; M3 box, CPU-only; the R134 shortlist consumed same-day): 1 rule, corpus 100 -> 101 (layout 9 -> 10) + 2 name-field confirmation riders (usize-index-to-u32 += #158720 cache-line boundary; sort-key-cache += #157179 short sort key). THE ARCH FINDING: the memcpy threshold is a backend constant and arch-shaped -- aarch64 INLINES a 136-byte by-value move (ldp/stp pairs + 8 B tail, asm-verified) while a 320-byte move emits `bl _memcpy` (ARM ceiling in (136, 320]); x86-64's ceiling is 128 B (#160005, 40% wall on rustc). M3 bench Mixed 1/0/1 (1.29x/0.99x, stable x2, allocs 0/0) -- the 1M tie is the honest mechanism cell; x86 re-gate DISCHARGED same-day on the 4090 (asm: 48 B inlines, 136 B hot loop carries `movl $136; callq memcpy`; wall 1.32-1.35x vs the M3's 1.00-1.13x). GOAT: batch112 gates 4/4 first-shot + default 1086/0 + bench 1104/0 + latent 1193/0/1i floors HELD + clippy 0 three states + score-bench #39 FLAT. Pins: annotated 43->44, unannotated held at 57 (first-pass arithmetic said 56, the runtime count refused). .plans/.highwater found stale at 81 (plan 082 existed) -- repaired to 83. Record: Plan 083 / Research 134.
- **Last landed: Batch 111 (2026-09-05, Research 133 / Plan 082 -- the vLLM libtorch_stable REMAINDER tier + the rider sweep, the B110-deferred queue consumed; M3 box, CPU-only; source re-cloned + re-pinned at 8277c42e at mine time -- origin had moved to 16328c7a minutes before the clone): 11 rules, corpus 505 -> 516 (A +3, D +2, H +2, K +1, M +1, N +2; entries_gpu_23.rs) + 20 riders across 13 files (19 kernel_opt + rust_perf weight-tile-consumption-order) + 2 KILLs (#21 noinline, #23 clamp-before-collective -- both COVERAGE by B110 rules) + 1 demotion (#24 capture-stable -> RIDER on B92 device-m). Dispositions + full rider map in AGENTS.md Batch 111 entry. GOAT: batch111 gates 4/4 + kernel_heal 1361/0 + combined 1469/0/2i floors HELD + default 1082/0 + clippy 0 three states; the 3 FULL-STRING pins of ridden rules regenerated (batch22/23/26 -- older gates pin the whole concept string). Generator discipline (target/batch111/); the ue8m0 gate's vacuity guard caught the sweep fixture's own bug twice. Record: Plan 082 / Research 133.
- **Last landed: Batch 110 (2026-09-05, Research 133 / Plan 081 — the vLLM `libtorch_stable` distill; 4090 box, CPU-only): 11 rules, corpus 494 → 505 (E +4, D +2, B +2, C +1, K +1, N +1; entries_gpu_22.rs).** Source vllm-project/vllm `csrc/libtorch_stable/` @ `8277c42e4c74c5dd604f29b6d938a3456658f247` (Apache-2.0, never mined). Rules: spin-barrier-needs-co-residency-capped-launch (the B58 correctness mirror, gate-pinned), captured-graph-init-unconditional-and-stream-ordered (the B58 complement), ballot-matrix-tie-rank, m-group-shared-weight-scan-register-relief (our router-GEMM shape), softmax-denominator-cancels-through-scale-invariant-norm, batch-invariance-locks-reduction-geometry (Issue-845 kin), rope-pair-partner-via-cross-lane-shuffle, row-parallelism-ladder-split-vs-pack, persistent-kernel-length-bucketed-mechanism-ladder, router-math-stays-fp32, degenerate-selection-falls-back-to-identity-uniform. GOAT: batch110 gates 4/4 FIRST-SHOT + kernel_heal 1357/0/1i + default 1082/0 + floors HELD zero re-pins + clippy 0. Riders DEFERRED to the Batch 111 sweep (inventory in Research 133). .raw/vllm removed at landing. Record: Plan 081 / Research 133.
- **2026-09-05 verdict (M3): humanrouter/qwen38-flash-6bit-m3-ultra-recipe — MARGINAL C+ as corpus source, LEAGUE INTEL HIGH (grade emphasis revised after owner challenge; .raw read in full @ eca338086f16f497278787ab2ab029d4269316d5, Apache-2.0, clone removed).** Corpus axis stands: ~250 LOC of mx.fast.metal_kernel helpers + a 92-line C++ MLX primitive, nearly all instances of B92–B106 rules (weighted10 exact = B95/B98 order-pin instance; hcfuse = B63 concat + group-size pad; dense-select/gdn-block/ane-cpu tables = B104; hcmix sigmoid = B32a; **flash_repack DORMANT — referenced by no patch hunk/env/script**). League axis was UNDER-WEIGHTED in the first verdict: the recipe's QUALIFIED profile runs oMLX PR3298 ANE GDN prefill offload in production — **36 GDN layers only, dual_ANE, CPU fraction 0.125 @ 8 threads, fixed-seq 3264 + tail_padding_min 2048, load-thread working-copy materialization before engine-thread handoff** — the exact design shape Bench 775/779 named as the sign-flip path for the OPEN riir-ai Issue 769 lane (program collapse + layer-level pipelining; our negatives were measured under f=1.0 serial + 96-program round-robin). Bonsai is a DeltaNet hybrid = the offloaded module class matches. NOT transferable: MTP (721/717 structural negative, MoE-verify economics), 8192 chunk (our 0.926x negative), HC fusions (not our arch); absolute numbers are 60-core-Ultra/6B-active — no headroom inference. Recipe prefill +14.6–18.5% is un-attributed across components (their own caveat). FOLLOW-UPS OFFERED: (a) targeted distill of oMLX PR3298 + PR2853 (the reference implementation — likely real kernel_opt D rules: dual-ANE bank prep, CPU/ANE split scheduling, working-copy handoff, fixed-seq qualification); (b) append the reference-config addendum to riir-ai .issues/769 as the re-arm probe spec (GDN-only + dual-ANE + f=0.125/8t + fixed-seq, Bench-775 S2 harness reuse).

- **2026-09-05 verdict (M3): jundot/omlx ANE-prefill + GDN blocked-sequential — YES B+ → Batch 114 (Research 138 / Plan 087, plan-first; .raw @ e467261edc786efd33b1e9023d5c4a827f8aa1c1 Apache-2.0, re-clone at pin).** This IS the shelf row-5 named-unmined lead (ane_heterogeneous_sources.md) + the PR3298/2853 follow-up the qwen38-recipe verdict offered. Candidates: `recurrence-prefill-exact-restructure-before-reformulation` (A− GOAT lead, G — Kernel S smem-staged TB-blocks kill the 32× k/q re-read, ~2× @16K ⚠M3-Ultra-reported; the WY/FLA reformulation loser ×3 as the boundary content — the corpus has the chunked FAMILY but not this boundary), `ane-bank-split-ladder` (B, D — ~4 GiB device-address window at program-create, near-halves retry, async-release gc+settle before headroom re-measure, 70% jetsam gate, bank-count/greedy-tie determinism; zero dedup hits; feeds our R1/R7/T2), `recurrent-safe-approximate-slice-cap` (B− — cap-at-z + alignment floor + loud cap log; B84 extension), `sub-simdgroup-cooperation-radius` (B− — 8-lane shuffle_down + leader simd_shuffle broadcast, zero hot-loop barriers; B22/B95 rider), `program-shape-vs-delivered-chunk-coupling` (C+ — compile-but-never-execute), tail-padding-profitability + recurrence-boundary tiling (C riders on B84), tri-engine CPU-fraction split + warm-first-dispatch (C). Drops: smem +8 pad (B79 boundary instance — vectorized-vs-scalar staging), host numpy gather (coalesce instance). Yield: 2–4 rules + 4–6 riders. Lane coupling: riir-ai Issue 876 (the new prefill-first M3 queue) carries the consumer probes; 769 T2/R1/R4a–R7 adoption rows now have located implementations.

- **2026-09-05 verdict (M3): Layr-Labs/qwen-3.8-mtp-challenge @ 0863b06 + the Yukon mlxfast qwen38 scoreboard — CORPUS DEDUP-CLOSED / LEAGUE INTEL MODERATE.** Owner ask ("273.6% boost / 97.8 median decode / 973.0 prefill — distill to fix our prefill"; feeds riir-ai 851 + 876). CORPUS axis: the linked commit IS the repo TIP and IS Batch 100's byte-identical mined pin (GitHub API: zero commits after 0863b06, authored 08-23) — **the watch lane for this repo is CLEAN, no new opponent deltas since B100**. Residual mining surface = the ONE file family B100's lanes never read: `Qwen35.swift`'s QMV/xsums machinery, exposed by the tip's own submission ("Validate submission ec24d591", the xsums fill fusion): the fused residual+RMSNorm kernel gains an `emitSums` variant whose epilogue runs the standalone fill kernel's body VERBATIM over the bytes it just wrote (device barrier, same vec<bfloat16_t,4> loads, same three BF16 adds per group, same (k_block*32+lane)*stride+row address — the table is the fill's bit pattern BY CONSTRUCTION, not by an algebraic reassociation argument), handed to the wide-QMV consumer through a weak-identity 8-slot sidecar (exact `MLXArray` object identity so a released tensor can never alias a later one + (K,M)+table-size revalidation + allocation-free miss scan with fallback to the shipped fill), deleting 127/257 standalone fill dispatches per verify round (mlp.gate_up 64 / gdn.in_proj 47 / fa.qkv 16). DEDUP GREP: zero corpus hits under xsum/chunk-sum both vocabularies; nearest = B77 shared-activation-quantize-memo (an op-RESULT memo, not a producer epilogue), B30 fuse-count-into-mutation-pass (scalar, no handoff), B96 bit-plane-plane-sums-input-side-offset-dot (the consumer MATH — the shared activation sum; the NEW content is that sum's PLACEMENT: producer-epilogue-table vs standalone-fill vs in-kernel recompute). OFFERED (queued behind Batch 114): 1-rule mini-batch `producer-emits-consumer-table-epilogue-identity-sidecar` (D — publish gated by the CONSUMER's routability predicate, necessary-not-sufficient so an unrouted table is harmless; width-gated M=4..9 so the M=1 serial leg + prefill run byte-identical) + riders on B96 (activation-sum placement axis) and B77 (the weak-identity shape-revalidated handoff half). LEAGUE axis: the cited record (273.6% boost, Dec 97.8 median / Pre 973.0 — owner-provided; the scoreboard renders the gemma4 tab to non-JS fetchers, so the qwen38 numbers are UNVERIFIED by this session) is a TRIPLE-CROSSED non-scoring reference (paired B=8 Apple-Silicon COHORT ≠ our same-box M3 cells; qwen38-flash 6B-active-class MoE ≠ Bonsai-27B dense ternary; 6-bit MLX ≠ PQ2_0) — consistent with 851 A4(c)'s same-box MLX-class cells (1.25–1.46k on 35B-A3B), so NO new bar. The linked submission is DECODE-VERIFY-SIDE ONLY (its own manifest: "M=1 (the serial leg) and prefill run the shipped kernel byte for byte" — cite as a prefill lever NOWHERE). Champion rows across the arena show MTP = 1.0 tok/round (gemma4 tab verified; qwen38 presumed same shape) = the arena's cumulative wins are KERNEL-level — external confirmation of 721/717; the site's protocol (same-box baseline/candidate pairing + thermal cool gate + "local scores are estimates") externally confirms the league fairness-manifest discipline.

- **CORRECTION (2026-09-05 later, same session — supersedes the grading of the line above; the correction itself was owner-forced: "we fall behind -50%, they +273% and you decide to skip?").** Three errors retracted: (1) one decode-only diff stood in for the arena — the +273% is the CUMULATIVE of ~50 scored submission diffs (08-17..08-23), which NOBODY has walked (B100 read a file subset of the tip tree only — a tip-tree mine and a diff-history mine are different objects); (2) the fairness caveat (not same-box) was allowed to become the verdict — it governs NUMBERS, never techniques; (3) the "qwen38-flash 6B-active-class MoE" in the line above was a MISREAD of the humanrouter recipe's "6bit" quant marker — the challenge backbone is **Qwen 3.8 27B: 64 layers, 4-layer hybrid repeat, hidden 5120, vocab 248320 (README-verbatim) = Bonsai-EXACT shape class**, dense hybrid, not MoE. Scoring facts now pinned from the repo README: the track score is **decode-only serial-relative** (median over 8 hidden prompts, serial = 1.0, ceiling 3.0 — "no separate prefill component; the seed prefill is charged inside the decode window"), so 273.6% = 2.736× decode on an M5 Max — never a prefill number, never same-box — but the seed prefill IS score-bearing and prefill-touching diffs exist and are UNREAD. RE-GRADED **YES B+ as a diff-history mine**; full verdict + 3-diff candidate seed (two-level centroid index over the 248320 readout — our exact lm_head width, nibble digit-scale fold, in-kernel offset sum, one-dispatch gather+reduce, dead-JIT factory skip, env-sanitizer fake-A/B hazard (A−, B109 family), head-trim coverage guard): `.research/139_qwen38_mtp_diff_history_verdict.md`; walk opens as **Plan 088 / Batch 115**, Lane A = seed-prefill diffs FIRST, queued behind Batch 114. The 851/876 addenda of the earlier line get matching dated corrections in the sibling commit.

- **SECOND CORRECTION (2026-09-06, the recipe-source delta-check — riir-ai `a0b260469`, operative record in 769 T10-P4 + 876-Q1; supersedes ONLY the "dense" clause of the correction above): the arena backbone is NOT dense — it IS a MoE.** The recipe's qualified env + provenance carry LIVE MoE machinery (`FLASH_WEIGHTED10` top-10 expert combination, `FLASH_DENSE_SELECT`, "expert reduction") and Qwen's own model docs call Qwen3.8-Flash-Next a MoE with ~6B params activated per token — so the ORIGINAL line's "6B-active-class MoE" was substantively RIGHT on MoE-ness (its "6bit"→"6B-active" provenance remains a misread), and the 09-05 "dense hybrid, not MoE" clause is RETRACTED. What survives unchanged: the decode-only serial-relative scoring (ceiling 3.0), the diff-history-mine re-grade (Research 139 / Batch 115, LANDED), and the geometry match — now SCOPED to the attention/GDN/readout class only (64L / 5120 / 248320 / 24h / 4KV / hd-256, verified verbatim). Practical consequence: Batch 115's six rules are mechanism rules and stand, but the walk's EXPERT-side diffs map to MoE FFNs we do not have — only its attention/GDN-side diffs are family-matched for Bonsai; the 721/717 MoE-verify-economics read is STRENGTHENED.

- **NEXT-UP: nothing armed — walk #8 was CONSUMED by Batch 124 (arXiv:2609.04040, Research 147/Plan 104); the nnethercote blog re-walk stays blocked (re-checked 2026-09-07: still Jul 31 latest); the next arxiv walk (#9) lane is ACTIVE-SIBLING territory (the 2608.27763 fast-weight-attention distillation session owns the arxiv lane — do not collide). Next batch needs a NEW input trigger.** ~~PRISM FORK BATCH-116 LEADS QUEUED~~ **CONSUMED: Batch 116 LANDED 2026-09-06** (Research 140 / Plan 089; 6 rules + 3 riders, corpus 532 -> 538; GOAT all green — batch116 gates 3/3, kernel_heal lib 1382/0, default 1086/0, combined 1497/0/2i floors HELD zero re-pins, all-features 1936/0/16i, clippy 0 both states, score-bench #43 FLAT). The Batch 112 x86 re-gate is DISCHARGED. Protocol C re-run 2026-09-05: clean (intake done=12 open=0); score-bench run #39 FLAT at 96.2/1.9/100/100 post-Batch-112.

- **2026-09-06 opponent-watch triage (4090 box, zero-build; riir-ai `.benchmarks/rematch/2026-09-06_watch.md`, since 09-04) — PRISM FORK IS 12 COMMITS PAST THE LEAGUE PIN `e311ed38f`, four kernel_rule families + one LEAGUE-STRUCTURAL format change. VERDICTS YES B+ AS A BATCH-116 MINE (kernel_opt N/D) + LEAGUE INTEL HIGH.** Source PrismML-Eng/llama.cpp `origin/prism` (Apache-2.0, the league's Bonsai opponent lineage; prior contact B96 @ e311ed38f + B98/B113 riders). **THE STRUCTURAL HALF — `01fd9521c` PTQ1_0, ternary at group 128, 1.75 bpw vs PQ2_0's 2.125 = 17.6% fewer bytes with BIT-IDENTICAL reconstruction** (round-trip of real folded ternary weights bit-identical to PQ2_0; test-quantize-fns abs 0.008678 / dot 0.141111 vs tq1_0's 0.008681/0.141345 — the only error either format carries is the shared fp16 scale rounding; upstream TQ1_0's single fp16 scale per 256 DISCARDS one of two group scales at 128-group checkpoints = 6.43% multiplicative weight error, the reason for the new type id 143). A fork re-pin to a PTQ1_0 checkpoint moves BOTH their byte traffic (17.6% down) and decode (see `7113f6462`) — fork policy stands (read-only vendor binary at the pin, never maintained), so this is owner-repin intel + the distill material, not a live scoreboard change. **THE KERNEL LEADS:** (1) `7113f6462` exact-float trit extraction (+29.5–37.8% decode M5 Pro, instruction-bound ISA) — byte = base-3 fraction of 256, trit n = `g_{n+1} − 3·g_n` for `g_k = floor(3^k·u)`, all `3^k·b < 2^24` so the floors are EXACT; summing `t_n·y_n` telescopes to `Σ_k g_k·(y_{k−1} − 3·y_k) + g_5·y_4` — activation-side COEFFICIENTS staged once per src1 and reused across EVERY weight row, inner loop one floor + one fma per trit, zero integer ops; tolerance-gated (reassociates the fp32 sum) — **the base-3 sibling of B98's telescoped-byte-FMA with the NEW decision content being the activation-side coefficient STAGING PLACEMENT**; second half (per-expert pipeline claimed by supports_op but never instantiated → SEGFAULT, silent-truncated test run) = an external B103-addendum `dispatcher-eligibility-predicate-lags-kernel-capability` instance. (2) `d0ce941ec` multi-column Q1_0 mat-vec for 2–8-row VERIFY batches — spec-decode verify rows were falling to generic mul_mv_ext at flat ~2.5 TFLOPS (a 4-row 27B forward cost 3.5 decode steps); kernel reads each q1_0 block once, dots against nr1 ∈ {2,3,4} src1 columns, mv-to-mm crossover at 16 rows; pp4 44.7→69.2 tok/s — **the weight-stationary multi-TOKEN twin of the B58/B63 same-input family (N columns × one weight read, vs one input × N weights); directly our qwen38 p=16 verify-chunk + Issue-721 verify-economics lane**; en-route the honest instrument: removing ONLY the per-weight bit test runs pp4 69.2→118.9 = the bit-extraction is the remaining cost, not the weight read (an instruction-bound diagnosis probe); half2 (+3.5–4.5%) and popcount arms measured and deliberately kept OFF. (3) `633168fb6` fold-transform sharing — Hadamard fold memoized on (input, rotation); 144/401 folded weights (36%) were duplicate FWHT transforms (ffn_gate+ffn_up, qkv+gate, q+k+v groups); +3.5% pp512 / +3.1% tg128, greedy byte-identical — graph-build-time CSE of fold transforms; nearest = B77's runtime quantize-memo (op-result, different axis) — likely NEW (our Rust graph builder can CSE trivially; the rule content is the (input, rotation) KEY requiring both to match). (4) `ca02f9b83`+`6f1c6908d` FWHT wide-block — Metal threadgroup-staged variant at 4096/8192 (register wall: N/32 values/thread stops fitting); **Vulkan cutover keyed on the REGISTER-BLOCK RATIO n/subgroup_size, not width alone** (subgroup 16 × 2048 width = 128 values/invocation = the same wall at an admitted width) — a width cutoff that ignores subgroup width admits the wall it exists to prevent; extends B96's fwht rule (N 64..2048) upward + the shared-memory shader with rows-per-workgroup as a recorded specialization constant. Upstream llama.cpp delta in the same window is already-banked content (fa-vec M3 = B113; the Kronecker FWHT restore `4d91760` = the #28184 restore B113 predicted, now merged). vLLM new: `bc96d76` persistent top-k smem fallback (B111 family complement), `9509fc8` MLA decode epilogue (decode lane), `16328c7` Kimi nvfp4 alignment (not our lane). MLX `2d44ab0` M5 tunings (M3-lane note only). Dedup grep before any batch: B96/B98/B92 (extraction/telescope), B103-addendum (dispatcher), B58/B63/B77/B110 (multi-column/same-input family), B96-fwht + B79 (staging). Batch-116 candidate: 3–4 rules + riders, CPU-only, mine at the fork pin `7113f6462`.

- **2026-09-05 sweep unit (4090 box) — NO LAND, territory contested; re-arm condition recorded.** The queue
  fell to item 3 (clippy sweep): riir-dapps + riir-viewbridge dry-run **0 edits** (riir-dapps clean under real
  clippy --all-targets — developed with the healer in the loop; viewbridge's 08-26 sweep still holds).
  riir-game-sdk dry-ran **19 edits / 6 files** (riir-e2e: map_unwrap_or x5, uninlined x8, shrink-to-fit x2,
  vec-with-capacity, use_self, +1 the fixpoint pass) — applied, then **REVERTED unvalidated**: the nft_e2e
  surface cannot compile on this box. Three environment findings, all cross-repo:
  (1) **riir-auth + riir-mmorpg-examples were missing from E:/git** — cloned from gist-rs (auth @ 3ecdd1e,
    mmorpg @ d1640cd), restoring the both-boxes sync rule for those two.
  (2) **game-sdk origin/develop is UNBUILDABLE from a fresh clone** — mmorpg's `0e4403a`/`d1640cd` (the
    scenelab migration, 030 P1) consumes `../riir-game-sdk/crates/riir-scenelab` + the game-sdk `panel`
    feature, NEITHER of which exists at game-sdk origin (the crate half of the 092 move-map was never
    pushed). Every cargo invocation in the graph dies at manifest load even with the deps inactive. The
    scenelab session (riir-ai Issues 871/872) is LIVE — their lane, not filed here.
  (3) **riir-chain `src/wallet/tx_log.rs:2079` E0282** (inferred `Option<T>` where T is unconstrained —
    `set_permissions(mode.clone())`) breaks the lib **under specific feature selections only** (compiles
    clean as a riir-dapps dep; breaks as an riir-e2e `nft_e2e` transitive dep). The file is the documented
    excluded-sibling (031/032) territory and the NFT-retirement migration (125) landed hours ago — the
    owning session should see it; not filed in the read-only repo.
  **Re-arm:** the game-sdk e2e sweep after the chain-125/scenelab migration lands and origin resolves from
  a fresh clone. Housekeeping done this unit: ticked Plan 075's stale GOAT checklist (batch verifiably
  landed 09-04), 068's stale open-list rows updated (D7 done, D8 named M3-lane), leftover wrangler-dev
  tree from the wet-run session killed (port 8813 clear).
  **RE-ARM BLOCKERS BOTH CLEARED (2026-09-05 later session, M3):** (1) game-sdk origin/develop now carries
  `crates/riir-scenelab` + the workspace-members entry + the panel feature (verified `git ls-tree origin/develop`);
  (2) the chain tx_log E0282 filed + fixed chain-side (Issue 126, `e6a0e221` — `None::<std::fs::Permissions>`
  type-pins the cfg(not(unix)) arm; M3 `cargo check -p riir-chain --lib` green; Windows re-verify owed on the
  4090; `f23da0da`/`e6a0e221`/`56c078b8` pushed). Remaining re-arm condition is only a build window (the 851
  ranking lane is live + wait-quiet on this box).

- **2026-09-05 re-arm unit EXECUTED + LANDED (4090 box; the game-sdk e2e sweep + the closure tail; M3 records
  the box coordination):** window opened (GPU 5%/660 MiB, desktop-class only, zero cargo). All 9 closure repos
  synced on the box first — found + cleared a **stale 82-min index.lock in riir-ai** (crashed process; the ~40
  git.exe on the box are per-repo `fsmonitor--daemon` instances, innocent — verify lock AGE before deleting);
  cleared the box's leftover Batch-111 plan draft (`082_vllm_libtorch_stable_batch111.md`, hash-DIFFERED from
  origin's landed `d710353` version — landed record wins) + dirty `.plans/.highwater` (82 vs origin 83).
  Healer REBUILT on the box (5m06s — the 10-commit pull touched src/; `cargo build --release --bin cargo-heal
  --features fix_verify,clippy_verify`; NOTE: `-p cargo-heal` fails — the package is `riir-clippy`, the BIN is
  cargo-heal). **Issue 126 Windows re-verify DISCHARGED:** `cargo check -p riir-chain --lib` on Windows
  1.98.1-msvc, clean in 10.39s (the AGENTS.md caveat is closed). **Landed game-sdk `9e3bef8`: 26 edits / 12
  files** (the blocked 19-edit e2e set had GROWN — the T5.4/T5.5/T8.7 test files landed since — plus 7
  scenelab edits from the new crate: map_unwrap_or x5, shrink-to-fit x6, uninlined x5, use_self,
  vec-with-capacity, items_after_statements, range_plus_one). Validated count-pinned: e2e default **29/29**,
  item_e2e **194/194**, nft_e2e **236/236** (nft implies item_e2e so it subsumes the item row's surface),
  scenelab-trace **32/32** + trace_index+stat_cards compile+run green. One `prod_l3_sigkill_drills`
  0xc0000005 access-violation = the documented SIGKILL-subprocess flake (passed 6/6 on re-run x3, differing
  crash line each time — the `.issues/023` class). 2 pre-existing `last_log_size` never-read warnings in
  prod_t54_soak: the heal did NOT create them (diff-verified untouched lines) — owner note. Chain tail:
  dry-run 1 edit (`items_after_statements` in tests/asset_lifecycle_persistence.rs) — the box CANNOT validate
  it (chain_cold_write pulls libsql-ffi whose bundled build panics PermissionDenied on exFAT — environmental),
  so **box reverted, M3 applied + validated 10/0 under the exact feature gate, landed chain `4b7b78e3`**; box
  synced after. mmorpg dry-run 1 edit (`semicolon_if_nothing_returned` in src/scenarios/runner.rs) —
  **SKIPPED, sibling-active**: the renderer-extraction agent's M3 lane is restructuring exactly that surface;
  left for the owning lane. riir-dapps + riir-viewbridge re-confirm 0 edits (prior unit's verdict holds).
  Swept-closure state: katgpt-rs/ndb/auth/clippy/dapps/viewbridge clean; game-sdk + chain LANDED this unit;
  mmorpg 1 edit deferred to its active agent; riir-ai hands-off (sibling lanes).

- **Corpus counts (convenience copy, synced 2026-09-06 post-Batch-119; AGENTS.md is the authoritative ledger — bump there per batch, this copy is a read aid):** kernel_opt 552 (E 45 · A 116 · D 128 · N 37 + the other categories per the tests.rs expected-array) · rust_perf 102 · clippy_lints 45 · sec 12 · docker 31
  (AGENTS.md is the authoritative ledger — bump there per batch, snapshot counts are a convenience copy).
- **Last landed:** **Batch 108 (2026-09-05, Plan 078 — the riir-ai M3 campaign measured-GOAT distill, prefill
  GEMM + decode attention; M3 box, CPU-only; the FIRST in-source batch since Batch 63): 4 rules, corpus 490 → 494
  (A +1, E +2, G +1; entries_gpu_21.rs).** Source riir-ai @ `59a172623` committed state only (no .raw — the sources
  live in the sibling repo at the pin). Rules: smem-envelope-gate-kernel-variant (G — the Issue-779 derived
  capacity predicate 32*(hd+4) <= hd*33 at ALL 8 rows-family launchers), multi-simdgroup-shared-staged-tile (A —
  the Issue-771-T1 smem 128x64 GEMM, per-sg PRIVATE result regions, bit-identical),
  independent-accumulator-chains-hide-load-latency-at-capped-occupancy (E — the Issue-831 4-chain unroll, ~1.8x
  @4096 / flat @2048), byte-cut-loses-when-plateau-is-issue-bound (E, refutation-type — the packed-f16-KV arm
  1.44-1.60x SLOWER at 114-vs-400 GB/s issue-bound plateau). RIDERS (5): B01 cmma staged-GEMM in-stack instance,
  B06 capped-occupancy regime complement, B57 staged-epilogue exploitation half, + the walk-#6
  land-with-next-adjacent-batch pair DISCHARGED (B79 += 2608.17641 rl-triton breadth; B82 += 2608.01651 Bole
  confirmation). RECORDED-NOT-LANDED: axis-swap refutation (S2:40-42), 4-arm ladder (B104 ethos), empty-partial
  identity (B35 grain). GOAT: batch108 gates 3/3 FIRST-SHOT + kernel_heal 1349/0/1i + default 1078/0 +
  kernel_heal+latent 1457/0/2i (floors HELD zero re-pins) + clippy 0 three states + all-targets. Gate-catch: the
  B18 full-concept pin moved with the rider (B22 precedent). Count pins walked incl. the stale 464 cover-all
  comment repaired. Record: Plan 078 / Research 131.
- **Last landed:** **Batch 107 (2026-09-05, Research 130 / Plan 077 — the uzu + lily defers batch; the B105
  deferred queue CONSUMED; 4090 box, CPU-only): 8 rules, corpus 482 → 490 (A +2, C +1, D +1, E +2, H +1, N +1).**
  Sources re-cloned at the pins (uzu @ 115609b MIT, lily @ 1ed972ed Apache-2.0); 12 defers re-verified by two lanes,
  3 dropped (flat-topology = B23-family batch-trie; RIF-sentinel = born-dead arms; weight-hygiene = B63/B31 instances),
  1 collapsed to the B63 rider (+ the B95 double rider: gdn_prefill_regscan + simdgroup-uniform-guards). Rules:
  sparse-demand-paged-kv-cache (A), gumbel-argmax-filtered-sampling-descent (C), packed-total-order-multi-field-argmax (E),
  integer-mma-span-epilogue-dequant (N), single-threadgroup-fused-count-scan (H), radix-select-ordered-key-topk (E),
  prefill-chunk-command-buffer-capacity-coupled (D), tensor-op-strided-device-operand-binding (A). UPSTREAM HAZARDS:
  uzu counts_offsets_fused E>128 CONFIRMED + EXTENDED (chunk-read missing base — offsets wrong for E > 128; parity max
  E = 128 while wiring admits 512) + weaver_top_children unclamped Gumbel (silent −inf always-lose bias) +
  unified_sampling bounded-fallback semantics note + lily descending_blocks shipped-but-unarmed. GOAT: batch107 gates 3/3
  + kernel_heal 1346/0/1i + default 1078/0 + kernel_heal+latent 1454/0/2i (floors HELD zero re-pins) + clippy 0 both
  states. Count pins walked (incl. the stale N comment repaired 27→29). Record: Research 130 / Plan 077.
- **Last landed:** **Batch 106 (2026-09-05, Research 129 / Plan 076 — the MTPLX upstream
  residue distill; 4090 box, CPU-only; the queued NEXT-UP row consumed): 5 rules, corpus
  477 → 482 (D 106 → 111).** Source youssofal/MTPLX upstream main @ `dd4031a8f9ed9d0a532b6e16bcc147dc40399c1d`
  (v2.11.1, Apache-2.0) — the three surfaces Batch 103 never read: the upstream `mistakes/`
  post-mortem corpus (16 files, filename IS the rule), docs/plans+specs+concurrency, and
  the targeted kernel diff since the mined base `8bc4d88`. `prefix-reuse-key-must-mirror-
  the-restore-predicate` (D, the B+ lead CONFIRMED with BOTH instances in-source: the
  sentinel bank-fingerprint post-mortem + the dd4031a8 fix commit itself — the fingerprint
  carries only what changes KV/MTP history without changing tokens, and every reuse
  estimator must ask the question the restore path answers; the 2.10.2 exact-containment
  shed re-prefilled 41,901 tokens cold / 54 s with a 41,391-token block restore available),
  `request-arrival-work-asks-reuse-state-first` (D — arrival-time work gated on the
  bank's shares_ram_prefix answer + never await unconsumed work; the PLE first-gather
  650k-row warm-turn waste), `serving-raises-only-behind-strict-flag` (D — the verdict-
  delivery policy composed with B103's measurement half: measurement = raise, serving =
  count + one warning per class), `derived-key-single-constructor-parity-test` (D — one
  constructor for every reader+writer of a hashed contract key + a drift-failing parity
  test + provenance markers for values equal to defaults), `vendor-fused-op-instantiation-
  check` (D, axis-A find — read the shipped metallib's symbol table before assuming a
  vendor fused op covers your shape; MLX sdpa is instantiated only for head dims
  64/96/128/256, the MLA latent is 512 wide, so the 'fast path' measured the vendor's own
  fallback). Riders: p99-inter-emit-gap census + delivered-cadence doctrine (league doc
  citations), stale-binary provenance (Bench-749/Issue-866 confirmation), git-stash-
  shared-by-worktrees (our AGENTS.md rule confirmed), census-vs-E2E 4th instance. 11
  mistake files dispositioned out-of-scope in Research 129; `docs/perf/` subdirs named
  unread-remainder. EN-ROUTE: fresh FULL clone needed (the queue's depth-1 note) +
  `core.longpaths` (mistake filenames exceed Windows MAX_PATH); the mod.rs "464-rule"
  doc line was 2 batches stale (post-B104/B105) — repaired to 482. GOAT: batch106 gates
  3/3 FIRST-SHOT + kernel_heal lib 1343/0/1i + default lib 1078/0 + kernel_heal+latent
  1451/0/2i (floors HELD zero re-pins) + clippy 0 --lib both states (the exFAT hard-link
  cargo noise excepted). Count pins walked: tests.rs 75 + 5368-sum + 5382-grouped +
  expected-array D + router.rs 461 + mod.rs D header/list + the 464→482 doc-line repair.
  Prior: **Batch 104 (2026-09-05, Research 126 / Plan 074 — the ggml-org/llama.cpp
  fa-vec offline swept launch-table distill; the 09-02-queued league opponent-watch lead
  consumed; M3 box, CPU-only, read-only source lane): 2 rules, corpus 464 → 466 (K 5 → 7).**
  Infra `efeda76b9` (08-24: ggml-metal-tuning.{h,cpp} the runtime table + tools/tuning/ the
  offline sweep harness), table pin `8963a9bdc` (08-28, #27863: the M3 Max rows — 91 rows
  F16+Q8_0, the league-relevant SKU; our pinned opponent fork e311ed38f does NOT carry fa-vec,
  upstream-only), harness `d230ddd76` (b10793; tools/tuning/ BYTE-IDENTICAL pin→HEAD verified
  by empty diff; post-pin movement is table ROWS only: 1,293 → 3,420 rows across 20 SKUs in
  two weeks, the community sweeping through discussion #27668). `swept-shape-bucket-launch-tables`
  (K — the A− GOAT lead: when a kernel's optimal launch config is non-monotonic across
  device × dtype × shape × axis buckets, sweep OFFLINE and ship a constexpr data table keyed
  by an 8-byte packed struct ("tightly packed for memcmp") with SIX decision contents:
  the fallback ladder (exact bucket → collapsed domain default → family-representative SKU →
  template baseline), the short-axis skip ("short KV: attention is a small slice of the step,
  left to baseline"), consumer-side device-limit validation ("fall back to baseline (Q=1) if
  the tuned config exceeds threadgroup memory"), the shared legality constraint
  ("single source shared by the offline tuner and test-backend-ops"), exception-row sparsity,
  and the swept-vs-derived axis split (Q/NE swept, nsg stays the runtime formula); grid
  derived from runtime specialization boundaries ("Bucket 0 is not sampled at all: the
  runtime leaves short KV at baseline, so no measurement there can reach the table");
  harness "never a pass/fail test" / "stdout carries nothing but table rows"; in-stack
  contrast: our tiled-flash m16/m32 single-axis single-SKU length gate),
  `no-harm-bucket-representative` (K — a bucket representative is a WORST-CASE-REGRET
  contract, never best-average: "no slower than the baseline config at every point that
  bucket covers" + the padding witness ("one of a bucket's first Q values carries the worst
  padding it can ever see, and that value has to be sampled ... a floor on the evidence, not
  a proof") + two gates not one (TUNE_THETA 1.05 aggregate / TUNE_TAU 0.05 pointwise) +
  unmeasured-is-baseline + anchor/cooldown drift gating with dirty-window re-measure +
  refusal reporting + fixture realism is timing-load-bearing (the -INF mask blocks)).
  Dedup: anchor-shape-bucketing (L) = cache-KEY bucketing, composed not duplicated;
  K-family runtime-autotune rules = RUNTIME mechanics; B77 twins = compile-time tile
  instances; B103 shape-class = capability verdicts. Drop ledger: sample-where-consumable
  folded into rule 1; harness-as-shipped-binary folded as process half; mask-timing note
  cited in rule 2 (B97 owns the mechanism). GOAT: batch104 gates 3/3 + kernel_heal lib
  1294/0/1i + default 1035/0 + kernel_heal+latent floors HELD (kernel self-variants
  100/100/100 — zero re-pins, blended 94.7/96.9/98.5) + clippy 0 --lib three states (one
  doc_markdown fix: F16+Q8_0 backticks — the B37 lesson). Count pins walked incl.
  router.rs expert total (the 18th-count-pin-site lesson — the FIRST failed run caught it).
  EN-ROUTE: the stale .plans/.highwater (read 72, Plan 073 existed — the 848 class)
  repaired to 74 at allocation; en-route gate-collision REPAIRED IN-PASS (the batch104
  insertion edit fuzzy-clobbered the batch97 oracle test's header + batch103's `for`
  header — both restored byte-exact before any commit, caught by the compile). Record:
  Research 126 / Plan 074 (.raw/llama-upstream removed at landing).**
  Prior: **Batch 103 (2026-09-04, Research 124 / Plan 072 — the MTPLX PR #391 + MLX NAX
  combined host-orchestration batch; T7 default COMBINE executed; M3 box, CPU-only): 4 rules,
  corpus 459 → 463 (D 98 → 102) + the B100 external-instances rider.** `shape-class-keyed-capability`
  (D — the A− GOAT lead: a capability verdict is valid ONLY for the shape tuple the native gate reads;
  the MTPLX wiring defect — the ONE call site that could reach the verify width asked about a width the
  flag never armed — measured the control on both seeds silently; the owner's rule 'a flag either works
  on every request path the server accepts, or it fails loudly at install'),
  `wavefront-overlap-pair-ceiling` (D — bound the overlap-pair count BEFORE building any multi-stream
  seam: exactly S-1 pairs, each bounded by min(producer tail, consumer head); their 2×2 wavefront:
  '0.4 % against a 15 % gate; tile ceiling 19.2 %' — 'the ceiling is too low to fund'; + 'cross-stream
  fences cost as much as the overlap gains'), `request-shape-decline-mirror-completeness` (D — THE
  MIRROR: every request-dependent decline branch of the native contract paired with a stable key;
  everything else is CONFIGURATION and raises; two production outages ('HTTP 500 on every HumanEval
  prompt' + the 1,024-token-in-a-2,048-bank crossover miss); decline sites named not totaled + reasons
  counted + the armed-lane-must-engage raise), `backing-provenance-causal-shadowed-extent-widen`
  (D — from the MLX NAX extraction @ f99e916: widen K to the tile multiple only after the slices PROVE
  the padded rows are backed (stride sanity + row-pitch containment + buffer end) and causally
  unreachable (qL_off from the WIDENED extent keeps the true diagonal); ragged-tail branch deleted).
  Rider: B100 register-budget-head-split += EXTERNAL INSTANCES (the NAX hd-256 dsplit — WN==2
  static_assert, the ONE-peer exchange, Q-half register residency; the #4380 GQA-12/16 table — hpt
  falls as GQA rises, KV ≥ 8192 threshold unchanged). T0 re-pin: MTPLX verdict pin ef96313 →
  **a5e38bb** (branch rewrite deleted the fable_* scaffolding + rewrote the body 09-03 18:24Z; TWO
  verdict anchors did NOT survive, carried honestly in-rule: the W87 attribution NOT re-confirmable +
  the hard-coded-draft-width note re-anchored to the wiring test). T5 graph-build overlap (W67)
  evaluated then DROPPED with reason (their lane is default-on at the new head, but our Bench-761/771
  caches consumed the same seam — no measured exposed host-enqueue window remains). T6 riders
  research-noted: Bench 805 mask-fuse external confirmation (+3.5% prefill but divergence 8/8 at
  median position 7.5 — ours bit-identical, the posture is the point); Issue 734 GDN regime note
  (theirs −0.35 s on unblocked-eager stock vs ours 0.97-0.98× on already-batched); Bench 641/B77
  micro-vs-e2e third external instance (micro 1.94×, e2e −3.6%). Non-goals held: NO prefill-throughput
  claim; no league action. GOAT: batch103 gates 3/3 + kernel_heal lib 1285/0/1i + default 1029/0 +
  kernel_heal+latent 1392/0/2i (floors HELD zero re-pins) + clippy 0 --lib all three states. Count
  pins walked + EN-ROUTE rot fixed: the 15 mod.rs category header comments were stale by up to 2
  batches (A 103 vs 105, C 39 vs 41, H 16 vs 17, N 27 vs 28 — pinned by NO test) — all 15 now match
  the pinned table. Generator lesson re-proven (B77 escape-strip class): the in-place B100 rider edit
  landed bare quotes (22 compile errors), repaired byte-precisely; the Python-heredoc GENERATOR path
  was clean first-shot.**
  Prior: **Batch 102** (2026-09-04, Research 123 / Plan 071 — the FlashQuant
  sparse-dense fusion distill, arXiv 2608.15531; taken over from a dead sibling session +
  renumbered 070 → 071 per the reserved-071 flag): 6 rules, corpus 453 → 459 (A +2, C +2,
  H +1, N +1). `heterogeneous-unit-paths-shared-tile-fusion` (C — the additive-heterogeneous-
  paths member of the same-input family: TC dense + CUDA-core sparse over one tile space,
  offline tile correspondence, EW extra-warps rider folded), `lpt-min-heap-disjoint-column-
  ownership` (H — assignment-time LPT balance, one owner per column, zero atomics — the work-
  map family's balance complement), `tile-coo-sparse-tile-aligned-format` (N — u8 indices
  from staged-tile extent + padding-slot repurpose), `bucketed-sparse-reader-column-partition`
  (A — access-granularity mismatch, consumer-side partition vs B92's address-transform arm),
  `bank-group-circular-element-reorder` (A — offline bank-group staggering keyed to the
  consumer walk), `stream-k-persistent-cta-mac-segments` (C — MAC-space decomposition with
  B58 uncapped-default as mandatory control arm; Lane B's decomposition-vs-reduction kill-check
  beat Lane A's rider deferral — coordinator adjudicated LAND). Headline honestly carried:
  batch ≥ 8 regime, NOT our batch-1 decode; INT4+BF16 evidence-only. GOAT: batch102 gates 3/3
  first-shot + kernel_heal lib 1282/0/1i + default 1029/0 + kernel_heal+latent 1389/0/2i with
  retrieval floors HELD (kernel 100/100/100, 13th consecutive) + clippy 0 both states. Full CI
  guard NOT run (the B95 honest-skip shape — the 504 T3 NoRA matrix holds the 4090 until
  ~09:00–10:30 09-04).** Prior: Batch 101 (2026-09-02, Plan 069 — `dispatch-handler-pair-hoist`,
  corpus 98 → 99, x86 re-gate ARMED) + Batch 100 (2026-09-02, Research 120 / Plan 068 —
  the Layr-Labs/qwen-3.8-mtp-challenge distill: 8 rules, corpus 445 → 453, A +3, D +3, E +2).**
- **Last landed:** **Batch 100 (2026-09-02, Research 120 / Plan 068 — the Layr-Labs/qwen-3.8-mtp-challenge
  distill, the M5-Max Swift/Metal inference arena, same team as ds4 B97/98): 8 rules + 1 B22 rider,
  corpus 445 → 453 (A +3, D +3, E +2). Lead `functional-cache-versioning-loses-donation-under-
  kernel-read` (the MLX slice-update donation-loss trap; ONE numeric correction from the verdict:
  23 layers × {K,V} × 8 GiB, not 24×; + the throttle-serialization rider ~13× + the ≥2-layers
  fixture corollary) + the in-place-contract family (fused single-writer-reads-newest-from-input;
  fence-token-consumer + the MLX <8-element CONSTANT-address-space ABI rider) + Metal
  residency-set wire-once + the register-budget HPT head split (test file confirmed:
  CBv2PagedEligibilityTests) + 3 NOVEL (batch-composition-invariant-grid-early-return;
  never-gqa-multiply-full-d-merge-smem — the source doc's own extracted rule; container-rebind-
  over-retrace-for-churn-survival — the FOURTH graph-churn mechanism beside B58/B81/B92/B97).
  Rider: B22 warp-shuffle += the Metal measured-cost note (simd_sum ~14.5 cyc vs ~2 shuffle).
  Pin byte-identical to the verdict (tree static since 08-23; leaderboard bumps are auth-gated).
  Two verification lanes; DFlashBatchedEngine = honest zero. R6 SKIPPED per plan (config-knob);
  7 defers ledgered in Research 120 (span-snapping, hidden-footprint ledger, chained-decode
  overlap, dtype priming, scratch-lane recycle, wrap-seam warmup, dispatch-batching note).
  GOAT: kernel_heal 1262/0/1i + default 1012/0 + clippy 0 both states + floors 14/14 HELD
  (bands identical); ALL count pins walked incl. the batch22 full-concept pin (moved WITH the
  rider — the gate caught the deliberate change as designed).** Prior: Batch 99 (2026-09-02, Research 119 / Plan 067 — the wasmi v2.0 blog
  distill, the first rust_perf batch since Batch 94): `mir-destprop-branch-site-merge`
  MEASURED ParetoDominant 2/0/1 (1.04/1.24/1.25×, corpus validator; probe-stable ×2),
  asm-verified mechanism (merged = cmovne + ONE indirect tail-jump; split = two
  DIRECT tail-jumps, no re-merge), negative control measured (runtime-opaque
  targets = 0.95-1.00× — rule applies ONLY to compile-time-known code pointers),
  toolchain regime rustc ≥1.92 DestProp; corpus 97 → 98, dispatch 3 → 4; the C+
  secondary `hot-dispatch-gpr-arg-budget` probed-Tie on msvc-x64 (1.009/0.999×) →
  queued-secondary kill-free (sysv64 window). GOAT: default 1012/0 + bench 1030/0 +
  floors 13/13 + score-bench FLAT + clippy 0 both states; SIX count pins walked
  (three caught by the full-suite run). Prior: Batch 98 (2026-09-02, Research 118 / Plan 065 — the ds4 dense.metal/moe.metal
  leads Batch 97 queued: 10 rules, corpus 435 → 445, +4 riders on B97×2/B35/B32a; GOAT all green,
  floors HELD, score-bench #29 flat; 4 mine-time corrections incl. the INVERTED M5 sigmoid
  hazard and the args-aliasing-by-value correction). Prior: Batch 93 (`501fdb2` — ktransformers
  residue disposition, ZERO corpus delta, the last mined-source lane CLOSED) + the t48 substrate
  tie-break fix (`944741f`).
  **2026-09-01 late: P8 adoption wave 2** — the nan-unsafe sweep's benches/examples/
  tests residue + unclaimed repos discharged across riir-clippy (self + loop-habitat
  proposer fix `5122bbe`), riir-neuron-db (`4524854`), riir-train (`20b53943`),
  riir-ai (`1e6e4b06a`, 188 sites / 13 crates); katgpt-rs residue RE-SCANNED
  AND DISCHARGED 2026-09-02 (`a9a72a3e`, wave 3: the ~100 estimate was 17
  live sites / 13 files after the sibling sweeps consumed the rest; 3 gate
  configs count-identical; shared-checkout discipline held — exactly 13
  files committed while the sibling's 16 uncommitted files stayed out via
  explicit-path `git add`). Full record: P8 ADOPTION WAVE 2 below.
- **2026-09-03 (4090 box, while B102 FlashQuant runs in a sibling lane): Protocol B
  heal sweep of riir-dapps/src — the repo was NEVER swept (created 08-20, postdates
  the Aug campaign; P8 wave recorded only "zero hits" there).** 16 edit sites / 23
  placeholder edits in src/kat/{crank,payreq}.rs (15 `use_self` — the Batch-89 rule's
  first live workspace yield — + 1 `match_bool` guard-free 2-arm). riir-dapps ff'd
  3eacd6c→5a55048 first (issue 010/011 hygiene + kat/ compose work landed upstream).
  Validation: dapps `cargo clippy --all-targets` 0 warnings + lib 25/25 + gated
  integration 25/25 (`kat_payments,kat_payreq,kat_staking` — the 4 kat test binaries
  are inner-`#![cfg(feature)]`-gated; default run shows "0 passed", the empty-binary
  class). Landed `b7f1fbe`, pushed. Zero residue post-heal; no proposer divergence
  surfaced (use_self + match_bool both first-shot correct). The `.plans/070` sibling
  plan (untracked, B102 IN FLIGHT) respected — kernel_opt corpus, AGENTS.md kernel
  lines, and the NAX/LDG/riders queue items all left untouched this cycle.
- **Next executable (cadence-gated):**
  - **~~Batch 99 candidate (queued 2026-09-02, Plan 067 / Research 119): wasmi v2.0 blog~~
    LANDED 2026-09-02** (see Last landed). Dual verdict consumed: the same source TRIGGERED
    the workspace wasmi 1.0 → 2.0 upgrade (riir-ai Plan 563 — 13 pins / 7 repos lockstep +
    fuel recalibration; that lane is riir-ai's, not this repo's) — **upgrade LANDED same day
    2026-09-02**: katgpt-rs `133664b9`, riir-ai `0b7cad41e`, neuron-db `22e8ff4`, game-sdk
    `c08c02b`, chain `2c3e0b9d`, mmorpg `b3aaac6` (closeout `146d50c0a`); unified on one
    `wasmi 2.0.0`, zero API breakage, gains 2.0–3.3×, Bench 851.
  - **~~Queued-secondary (kill-free): `hot-dispatch-gpr-arg-budget`~~ — probed Tie on
    windows-msvc x64 (1.009/0.999×, two runs; the blog's evidence is sysv64 tail-call
    dispatch). Re-open ONLY with a sysv64 measurement window; ABI-specific,
    interpreter/VM-loop-shaped pattern.** Related: B101's x86 re-gate DONE 2026-09-04
    (ParetoDominant 3/0/0, expected-Tie REFUTED — the runtime-table fixture loads
    in-loop on both arches; see Plan 069 + the corpus annotation).
  - **~~Batch 100 candidate (queued 2026-09-02, Plan 068 / Research 120): Layr-Labs/qwen-3.8-mtp-challenge~~
    LANDED 2026-09-02** (see Last landed — 8 rules, not the verdict's 3–4 estimate: the two verification
    lanes surfaced 3 novel rules beyond the planned R1-R5). **Deferred-with-reasons ledger carried in
    Research 120's appendix — the strongest reopen candidates:** hidden fast-path footprint charged
    to the admission ledger + refuse-on-half-budget (CompiledDecodeV2; reopen on the next
    qwen-3.8-mtp-challenge-adjacent batch or a paged-KV admission consumer) and indivisible-attention-
    span chunk snapping + one-shot starvation guard (SchedulerV2; reopen on a multimodal or span-atomic
    chunked-prefill consumer).
  - **~~Unnumbered candidate (probe-only, 2026-09-02 — Research 119 §Parallel-session addendum):
    `dispatch-handler-pair-hoist` (rust_perf/dispatch)~~ LANDED 2026-09-02 as Batch 101
    (Plan 069, corpus 98 → 99, dispatch 4 → 5; validator ParetoDominant 3/0/0 — 2.89×/1.21×/1.23×;
    x86 re-gate ARMED in Plan 069 — 4090 contention at batch time).** Original candidate row: The aarch64 complement of Batch 99's landed
    rule: the merged conditional-dispatch form keeps an IN-LOOP indexed handler load on aarch64 (asm-pinned,
    `csel` offset + `ldr` + one `blr`; the classic `HANDLERS[op](..)` per-dispatch idiom), and hoisting the
    handler pair once (register-resident select) measured ParetoDominant 2/0/0 — 1.19-1.23x at 10K/1M
    dispatches on UNPREDICTABLE streams (structured/periodic = Tie band; verdict-stable x3, 0/0 allocs).
    NOT landed with Batch 99 (collision yielded push-wins) — needs its own dedup cycle vs the landed
    `mir-destprop-branch-site-merge` (distinct mechanism/fixtures: value-returning + runtime table vs
    tail-call + known items; the landed rule's negative control marks the runtime-opaque regime
    out-of-scope) + a cross-arch re-gate (x86_64's merged form uses `lea rip+h_*` — no in-loop load — so
    the hoist win may be aarch64-shaped). Batch 99's own split-vs-merged negative control (0.95-1.00x on
    runtime-opaque targets) does NOT cover this transform: splitting re-merges structurally on aarch64,
    but the hoist still wins by eliminating the load.
  - **~~NEXT UP (UNBLOCKED by B102's landing): Plan 072 (Research 122) — youssofal/MTPLX
    PR #391 (davidtai Qwen3.8-Flash-Next restack) — W87-led kernel_opt batch, 2–3 rules
    + riders, MAY COMBINE (default COMBINE) with the queued NAX leads~~ — LANDED 2026-09-04
    as Batch 103 (Research 124: 4 rules incl. the NAX backing-provenance lead + the B100 rider;
    T5 graph-build overlap dropped with reason; T7 COMBINE executed; ~~the dispatcher-eligibility
    predicate lead is DISCHARGED as fix-space guidance inside the request-shape rule's engagement-
    canary complement — no separate corpus rule~~ — **OVERRIDDEN same day by the Issue-665
    collision addendum (07fa0df): the lead LANDED as the 5th rule** (`dispatcher-eligibility-
    predicate-lags-kernel-capability`, D, corpus 463 -> 464) after push-wins adjudication —
    the "discharged" call was an oversight (the lead sat in no drop ledger; the landed decline-
    mirror rule cited its fix-space without the rule existing), see Research 124 §addendum.** Original queue row: (`backing-provenance-
    causal-shadowed-extent-widen` — the MLX 846 rule-1; `dispatcher-eligibility-predicate-
    lags-kernel-capability` — the MLX #4416 rule-2) into ONE post-B102 kernel batch (one
    corpus cycle, one GOAT run, one count-pin walk). Verdict: YES grade B as a mining source,
    MARGINAL for prefill throughput — every retained prefill lever is HAVE / refuted / no-analog
    in-stack (Bench 805 mask-fuse twin, Bench 710 wider-chunk refuted, Issue 734 GDN-chunked
    negative, Issue 742 prefix cache). Candidates: `shape-class-keyed-capability` (A− lead — a
    one-shape capability refusal disarmed the flag process-wide; key by the full shape tuple,
    re-ask per call), `wavefront-overlap-pair-ceiling` (B — 2×2 wavefront ⇒ 1 pair ⇒ 19.2%
    ceiling vs the 15% gate), `request-shape-decline-mirror-completeness` (B — the fast path
    mirrors every request-dependent decline branch; two HTTP-500 outages); W67 graph-build
    overlap evaluate-then-likely-drop. Pin `ef96313…` (fork davidtai/MTPLX-1, Apache-2.0,
    unmerged; re-pin at T0). Also queued for the next B100 touch: the NAX hd-256 dsplit
    rider + the #4380 GQA-12/16 table (B100 rider enrichment).
  - **MTPLX RESIDUE (queued 2026-09-04, cadence-gated behind the B105 defers) — LANDED 2026-09-05 as Batch 106 (Research 129 / Plan 076): 5 rules, corpus 477 → 482 (D 106 → 111); the B+ lead CONFIRMED with both instances in-source; see the Last-landed entry above.** Batch 103's population was the PR #391 diff + body;
    upstream `youssofal/MTPLX` main has NEVER been mined (**196 commits / 300 files** ahead of the
    mined base `8bc4d88`, `behind_by` 0; v2.10.2 09-01 + v2.11.1 09-04; head
    `dd4031a8` 2026-09-04T10:25Z, Apache-2.0), the `mistakes/` post-mortem corpus is **16 files
    upstream vs 9 in the vendor clone** (7 upstream-only), and `docs/plans` 1217 / `docs/specs` 757
    / `docs/concurrency` 259 lines are unread. **Lead (B+):
    `prefix-reuse-key-must-mirror-the-restore-predicate`** — TWO independent instances in one
    source (HEAD `dd4031a8`: the 2.10.2 admission shed asked `SessionBank.longest_prefix` exact
    containment while restore serves by block prefix ⇒ 41,901 tokens re-prefilled cold, 54 s, with
    a 41,391-token restore available; + the trailing-sentinel bank-fingerprint mistake, the
    mirror-image direction). Sibling of the landed `shape-class-keyed-capability` one layer up, and
    **the first MTPLX row with an in-stack analog** (Issue 742 GDN+KV prefix cache, 441× TTFT) —
    every prefill lever in R122's table was HAVE / refuted / no-analog. Axis B is the best
    confidence-per-token (filenames ARE the rules): `git-stash-is-shared-by-every-worktree` is an
    external confirmation of our own stash rule + the `hash-object`/`update-index` fix pattern;
    `mean-tps-...-p99-inter-emit-gap` censuses delivered-cadence silence, an axis we do not measure
    at all. Blockers: the vendor clone is **depth-1 shallow** (`a5e38bb` + `ef96313`) so it cannot
    serve the upstream axis — fresh pinned clone required; `.research/.highwater` reads 127 while
    128 exists (stale-highwater class, repair at allocation). Index + anchors:
    `.research/122` §Residue (SCOPE-CORRECTED 2026-09-04). Execution = Research 129 / Plan 076,
    axes A+B in one corpus cycle (B103/B105 COMBINE precedent), axis C riders only.
  - **The arxiv pool re-walk #4: DONE 2026-09-02 ~07:15 +07** (`.research/113` — covered
    the Tuesday-09-01 announcement window; zero corpus delta; the spec-dec lane woke up:
    DBloom's ceiling-bin acceptance histogram (zero-cost telemetry adoption candidate for
    the verify-loop benches) + compressed draft-side KV (riir-train route) + a rigor-matched
    audit paper (measurement doctrine); two doctrine citations (G4/alloc budgets,
    modelless-first). **Next window 09-03+** (daily differential cadence; MCP quota 09-15).
  - **The arxiv pool re-walk #6: DONE 2026-09-03 (4090 box, read-only lane while the
    Plan-371 GPU window runs)** (`.research/121` — Wed-09-02 batch differential;
    strict window NOT FIRED: only AceSpec 2609.02514, C). **Walk yield = the NEW
    GPU-kernel query surface** (`abs:"GPU kernel"` × DC/PF/AR), which out-recalled
    every category-listing surface. **BOTH queued source-verifications DISCHARGED
    same day (`.research/121` §discharges):** **2608.15764 (A−) CONFIRMED** — H100
    plain-load bandwidth peaks at K≈2 outstanding loads/thread, −35% by K=8
    (fixed-work + order + 2-die + allocation-sweep + ncu-controlled; evidence
    quality unusually high); Ada transfer OPEN (paper H100-only; Blackwell ≠ Ada);
    mechanism explicitly UNIDENTIFIED by the authors (only the operational curve
    distills); cp.async axis COMBINES with B92 (equality at 1 group, 2.1–2.9× at
    16, shrinking with occupancy — the occupancy×depth WHEN-to-use axis B92 lacks).
    **BATCH RESOLVED ANNOTATION-ONLY 2026-09-04 (T0a ran in the post-matrix window on this box): the H100 knee does NOT transfer to Ada** — sparse-grid bandwidth saturates at K=2 (951.6 GB/s median, 94.4% of nominal) and stays FLAT through K=16 (−0.3%); no −35%-by-K=8 decline in either grid; K=1 only −1.7% below plateau; verdict-stable ×3 (pooled medians n=9/cell). Shape (b): NO rule landed (a depth-cap STOP rule would be WRONG on Ada); B92 gained the Ada-neutral measured rider (corpus prose only, kernel_opt stays 466); T2 MOOT by precondition. Plan 073 all tasks closed; instrument + raw CSVs at `scripts/ldg_probe/`; Research 121 §2608.15764 has the RESULT addendum. T0b stands: decode GEMV at natural K≈2 = optimal either way. Original armed-batch record: plan filed 2026-09-04: `.plans/073_offered_load_knee_ldg_probe.md` — T0b GEMV K-audit DONE in-plan, findings recorded there; headline: the production dp4a decode GEMV already sits at natural K≈2 = the paper's H100 optimum, so the decode surface is a likely NEGATIVE case for depth-adding levers and the T2 rider's home narrows to the prefill staged-tile kernels — the T0b audit pre-read that guided the probe design; superseded by the RESOLVED header above). **2608.15531 FlashQuant: headline REFUTED for
    direct adoption** (INT4+BF16 weight outliers unreachable from ternary/Q4_K;
    batch≥8 regime — pads to 8, advantage "diminishes for very small batches such
    as 1 and 2" — not our batch-1 decode); **mechanism KEPT QUEUED** as a
    paper-distill batch candidate: heterogeneous-EXECUTION-UNIT warp specialization
    (TC dense ∥ CUDA-core sparse over one tile space, Stream-K, disjoint
    output-column ownership via LPT heap) = the additive-heterogeneous-paths member
    of the B58/B98 same-input family; dedup surface large (B79 plane-split, B98
    moe-sum, B15) → needs its own plan-first session. **2608.12700 (A−) = doctrine
    citation, no batch** — contract-grade kernel verifier, 1,487/14
    standard-test-vs-verifier disagreement over 2,638 kernels = the external measured
    confirmation of the Bench-802/Research-116 fault-class doctrine; cite in the
    lossy-surface + tolerance-budget surfaces at the next doc-sync touch. Riders
    queued (land WITH the next adjacent batch, never standalone): ~~Bole 2608.01651 →
    B82 (tree-verify state-free external confirmation, memory-side win measured),
    rl-triton 2608.17641 → B79 (associative-scan breadth: 7 RL estimators as ONE
    first-order recurrence)~~ — BOTH DISCHARGED 2026-09-05 with Batch 108 (the next adjacent batch; see the
    B108 Last-landed entry). Lane records: DAMP 2608.27513 (B− GDN state-quant, the
    DASC group; the recurrent-state surface consolidating), H-Scale 2608.28113
    (the quant scale-AXIS triplet completed: 845-PoT / OCGQuant-grouping /
    H-Scale-value), sigmoid-eviction 2608.23296 (external sigmoid-not-softmax
    confirmation in the learned-eviction regime). Walk-#5 miss corrected honestly:
    SMELT 2609.01343 + mzCache 2609.01338 sat inside walk #5's Tue-batch window
    (09-01T14:4xZ) unrecorded — both C, cheap miss; walk #5's query set doesn't
    reach training-scaling/mobile-memory papers. Standing lanes for walk #7:
    spec-dec (unrestricted), KV+eviction, quant+inference, GPU-kernel (permanent),
    DC/PF listings. **Next window Fri 09-04 ~07:00 +07** (MCP quota 09-15).
  - **The arxiv pool re-walk #5: DONE 2026-09-02 (later session, subagent read-only lane)**
    (`.research/115` — Sept-ID (2609.x) differential wave + two NEW query surfaces
    [KV+eviction, quant+inference]; zero corpus delta). **Queued from the walk — both
    DISCHARGED same day (`.research/116`):** 2609.00363 (A−) CONFIRMED on all three pinned
    questions — PoT requantization is deterministic weights-only w/ a byte-exact
    reconstruction gate (mirrorable; caveat: pin our own scale inventory first), the
    ≤1-spacing blindness transfers to f16-KV AS DOCTRINE (not the INT32 alibi — our fp32
    accumulation is order-dependent and multi-ULP, hence tolerance-visible), and Bench 802's
    flat-budget verdict is upgraded to the fault-class classification (tolerance gates
    certify boundedness + preconditions + operand provenance ONLY; Bench 728's div.full is
    the live invisible-class instance). **Lever FILED: riir-ai Issue 845** (PoT-scale
    determinism for ternary group scales + q8kv block scales — kills the div.full class by
    construction; requantize-not-rewrite + byte-exact gate mandatory; entry via GOAT with
    bit-identity promoted from measured to required). 2609.00066 (B+) — runtime-cost concern
    REFUTED in the B+ direction: decision = offline deterministic greedy (RMS-ranked,
    ~5 min/8B model), runtime = static permutation absorbed into the quantize kernel
    (contiguous-segment fast path; −1.5..−2.9% prefill vs fused-RTN, decode memory
    identical). **Grouping lever QUEUED** with a falsifiable entry condition (measured
    collateral-error case the B55 sidecar does not fix at lower cost); block-invariance
    boundary recorded: OCG applies ONLY to block-shared-scale surfaces (q8kv, Q4_K, ternary
    groups, KVarN), never per-channel-scaled ones (Bench 556's insight).
    DASC 2608.30386 = GDN lane record only. Standing fix for walk #6: drop `cat:cs.LG`
    from the spec-dec query (walk-#4 recall hole).
  - **Issue 845 (the PoT lever) CPU HALF LANDED 2026-09-02** — katgpt-rs `91a9cdaf`
    (Plan 586, `pot_scales` opt-in in katgpt-types): `quantize_from_f32_pot` (snap-before-
    quantize = requantize-from-parents), exact `snap_f32_to_pow2`, `all_scales_are_pow2`,
    scale-covariance property test, SNR harness — PoT rel-err ratio 0.988–1.012 vs native
    (the carry loop absorbs the ≤√2 snap factor). T0 scale inventory in Plan 586.
    **GPU G1/G3 LANDED + PASSED same day 2026-09-02** (4090-exclusive window, Vulkan
    wgpu<spirv>; gate `crates/riir-gpu/tests/issue845_pot_scale_gpu_g1.rs` — fold invariance
    0 diffs on 3 Bonsai shapes, PoT-shift covariance 0 violations; structural scoping:
    f32-accumulator fold-invariance out of reach → the integer-dot slice landed as riir-ai
    Issue 850 / Bench 850, all four gates PASS). Lever RESOLVED-SCOPED; `pot_scales` opt-in,
    no consumer; q8kv slice + trit-tier mirror stay deferred (850 T4 scoped the q8kv
    discriminator to the f16 STORAGE site). Plan 586 + this line reconciled 2026-09-03.
  - **The arxiv pool re-walk #7: DONE 2026-09-05 ~04:15 +07** (`.research/125` —
    zero corpus delta). **Strict differential EMPTY — the frontier has NOT advanced**:
    measured 09-04T21:15Z, the whole API tops out at submissions 09-02T17:59:46Z
    (cs.LG probe) and the listing's newest announcement group is still "Thu 3 Sep";
    walk #6's predicted Fri-09-04 ~07:00 +07 batch did not post (next observable
    ≈ Sun 09-06 20:00 ET → **walk #8 window Mon 09-07 ~07:00 +07**, the
    09-02T18:00Z→09-03T18:00Z submissions rolling into it). **The walk's yield is
    the BACKFILL triage**: the broader standing queries surface 12 rows inside
    walk #6's own strict window of which walk #6 recorded ONE — root cause:
    walk #6's per-lane queries were narrower than the standing set (KV+eviction
    vs plain KV; quant+inference misses pretraining/capability rows). Graded:
    **2609.01962 Post-Training Ternarization of Qwen3-4B (B− doctrine — per-task
    retention spread BoolQ 84.6% vs ARC-C 43.8% under a −9.8pp aggregate = external
    lossy-surface confirmation, CITED into the katgpt-rs rule this walk;
    effective-bit accounting 1.641 vs nominal 1.58 = the honest-accounting analog
    for our ternary GGUF claims; lossless packing 8.29→3.96 GiB unchanged-PPL vs a
    LOSSY third-party packing excluded — the BLAKE3-committed-packing echo)**;
    2609.02029 HeadWiseKV (B− lane record — per-head STATIC residency for hybrid
    global layers, materialize-residency-not-mask; **out-of-stack: Issue-746
    append-only KV prohibition**); 2609.02846 UE5M3 FP4 (C+/B− doctrine — drops RHT
    + BF16 final-layer exemption, +21.2% throughput at lower loss = recipe-level
    ROTATION-TAX echo beside our Bench-716 refutation + LightRot/GyRot; the
    scale-FORMAT axis again: Issue-845 PoT narrow-deterministic vs UE5M3 wide);
    2609.02027 LRU mean-field (C+, serving-capacity theory); 2609.02052 SchedBlame
    (C+ — culprit-attributed CPU contention via eBPF = the principled version of
    Bench-649/`watch_midrun` victim-side gates; cite in the league doc at next
    touch). **Walk #6's queued 2608.12700 citation (contract-grade verifier →
    lossy-surface/tolerance-budget) DISCHARGED in the same docs commit.**
    Tooling note: the `fetch` MCP tool strips Atom `<entry>` bodies and its
    submittedDate range filter returns 0 through both encodings — walks go via
    `curl` + unbounded lane fetch + local `<published>` window filtering.
  - **09-15 MCP search quota reset** → online perf-search pool walk (queue item 11).
  - **2026-09-04 (M3, idle unit): Protocol B sweep of seal-remake — the repo was NEVER
    swept (created 09-01).** Zero own-crate findings: `cargo clippy --workspace
    --all-targets` clean (isolated /tmp target; the only warnings are the sibling
    katgpt-rs lib's known `logit_scratch` + riir-viewbridge-core build-script info
    lines — the landing sessions' gate discipline held). Nothing to heal; no commit.
  - **2026-09-04 (M3, idle unit): riir-ai Issue 866 FIXED** — the perf-rematch
    contention detector's two inline basename allowlists missed released cargo
    examples (digit_ood_eval held the M3 GPU 29 min while the gates read quiet).
    Fixed riir-ai `acd443b9a`: committed vocabulary file (contender_names.txt, 19
    patterns, empty=hard-refusal, floor pinned) + resource arm (≥80% CPU AND ≥2 GiB
    RSS, RIIR_CONTAM_* knobs, Darwin/Linux) + legible negative ([scan-stats],
    enforce_midrun tag contract pinned) + 32-assertion fixture selftest
    (perf_rematch_detector_selftest.sh) + en-route wrapper/VAR= interleave normalizer
    fix. Validated LIVE first-run: caught digit_ood_eval in the wild (589 scanned, 21
    matched, gate refused, both arms). Issue file removed at close; durable note in
    the league doc rule 4.
  - **2026-09-04 (M3, idle cycle 7, unit 1 — hygiene): katgpt-rs Issue 717 file
    REMOVED** (`0187b384`) — status CLOSED 09-03, record `.benchmarks/699_lt2_deep_loop_
    stability.md` carries T7-defer + Plan-373 contingency; zero external refs to the
    issue path. katgpt-rs issues re-triaged: 718 (T3 owner cost call), 719 (T2-T4
    trigger-gated), 721 (T3 owner sequencing) all gated; chain 095 (multi-repo
    extraction, unclaimed), 101 (constraint note), 110 (needs ESP32 silicon) not
    idle-loop units.
  - **2026-09-04 (M3, idle cycle 7, unit 2 — mmorpg re-sweep with the post-Batch-90
    fixable set, mmorpg `bd4a4f2`): 54+3 edits / 17 files** — first re-sweep of the
    08-28-swept repo under the new rules (use_self/manual_let_else/items_after_
    statements); yield = items x6 + manual_let_else x4 + match_bool x2 + map_unwrap_or
    x3 + uninlined bulk; chain/dao/auth/viewbridge dry-runs all ZERO (their fixpoint
    holds). **Drive-by: container-tick-test was UNBUILDABLE** — no [workspace] table +
    not a parent member → every cargo command refused ("believes it's in a workspace");
    fixed with the warm-tier-do self-rooting precedent. **The healer's verify gate
    cannot handle standalone-workspace-inside-repo layouts** (it ran the parent-context
    cargo check over the excluded crate and reverted the whole first batch) — heal the
    standalone crates in a SECOND scoped pass after their roots resolve; noted as
    tooling follow-up **— ROOT-CAUSE DIAGNOSTIC LANDED next cycle (this repo, lib test
    1035/0): `compile_oracle::workspace_belief_cause` + `enrich_cargo_run_error`** —
    when cargo's "believes it's in a workspace" signature line fails a baseline/loop
    check, the error now names the memberless nested manifest + the enclosing workspace
    + the self-rooting remedy (wired into BOTH CheckRunner::run_check and ::run_clippy;
    membership itself deliberately NOT parsed — ancestor members lists accept globs, a
    false "you are excluded" would be worse than none). 2 unit fixtures + e2e
    `fix_verify_e2e_workspace_belief_refusal_names_the_remedy` (real cargo temp layout:
    fail-closed BEFORE any write + enriched message). **build.rs map_or terminal-fix recurred** (the Unknown-receiver
    call-expr conservative emission — `git_out(..).map_or(false, |s| ..)` →
    is_some_and by hand) — **ROOT CAUSE FIXED same cycle, riir-clippy `609d7267`:
    map_unwrap_or same-file fn-signature resolution** — Collector fn_rets map
    (fn name → Option/Result from its RETURN type), classify_receiver consults it
    for bare-call receivers (qualified path OK; foreign calls/method receivers stay
    Unknown); 4 fixtures (45 total); e2e proof on the exact build.rs shape heals to
    is_some_and first-pass; score-bench #35 flat (96.2/1.9/100); lib 1033/0.
    Standing residual: shadowing-let misclassification accepted (parse-only;
    --verify reverts E0599). Validation: default lib 441/0, e2e_topology 12/0,
    warm_tier_e2e 3/0, bench_remote_smoothing 8/0, bench_game_sync 2/0/2i, taming 2/0
    + hero_fsm 1/0 (bevy_backend), out_of_box 1/0 (scenario_runner,social_signals,
    pet_pvp), duo 18/0 release (+scenario_runner), warm-tier-do contract 4/0,
    clippy 0 own-crate both states.
  - **2026-09-04 (M3, idle cycle 8): two units.** **Unit 1 — hygiene follow-through,
    mmorpg `97d60eb`:** container-tick-test's generated Cargo.lock TRACKED + a nested
    .gitignore (/target, /node_modules, /.wrangler — the warm-tier-do precedent; the
    repo-root .gitignore is root-anchored and never covered nested crate targets) —
    closes cycle 7's noted-not-fixed leftover; staged-set audit clean (2 files, both
    mine). **Unit 2 — the cycle-7 verify-gate tooling gap CLOSED, riir-clippy
    `97f4ca32`:** see the inline closure on the cycle-7 entry above
    (workspace_belief_cause + enrich_cargo_run_error; lib 1033→1035/0; fix_verify e2e
    2/2 incl. the new workspace-belief fixture; clippy 0; no-dep check clean; GLOBAL
    BINARY REBUILT release fix_verify,clippy_verify per the AGENTS.md rule). Gates
    re-checked at cycle start: T0a STILL GATED (no t504_confirm/status.txt — the 504
    T3 harvest lane is actively running; 4090 quiet at 5%/617 MiB but the gate needs
    ALL_DONE, not just a quiet box); walk #8 → Mon 09-07; MCP pool → 09-15; re-sweep
    surface still closed (riir-train 8 / riir-neuron-db 46 / riir-game-sdk 7 dirty —
    sibling lanes: 509/digit_ood, a src-wide sweep, riir-e2e NFT; riir-ai heavily
    dirty). The nearest_rank-copy housekeeping lane is ANOTHER AGENT's — not touched.
  - **Batch 94 (LANDED 2026-09-01, Plan 058 / Research 112): oMLX PR #3260
    expert-streaming distill CONSUMED** — 8 kernel_opt D + 1 rust_perf io +
    3 riders (B63 4th member, B71 producer complement, B84 routing member);
    corpus 414 → 422 / 96 → 97; head never force-pushed, all anchors
    re-verified at mine time (4 line-range corrections + 2 paraphrases
    folded). GOAT all green: 1238/0 kernel_heal + 1001/0 default + 1728/0
    all-features + clippy 0 both states + floors 14/14 + score-bench #25
    flat. The oMLX external-distill pool is now CLOSED.
  - League opponent-watch: **AWAKE as of walk #4 (09-02)** — 2608.30427 (DBloom:
    block-widening erosion + histogram telemetry; third echo of our DFlash verdict class)
    + 2608.30252 (compressed draft-side KV; riir-train route). Re-arm condition unchanged
    (a ≤24-GB-feasible Flash-Next-class variant with native MTP); the zero-cost adoption
    (ceiling-bin histogram in the verify-loop benches) rides the next bench touch.
    History note (the pinned Rapid-MLX reference row, riir-ai Bench 831, clean window
    `11895e9e5`): prefill 179/189 pp512/pp2048 — decode ≈ llama.cpp; format-crossed non-scoring, scoreboard unmoved;
    corpus delta ZERO. The contaminated first window is the case study: numbers wrong
    by 20–94% and one conclusion inverted — armed-autofire sequencing rule recorded).
    **09-01 same-window CORRECTION (riir-ai `f098133a7` league update): llama.cpp reads
    pp2048 192.95 / pp512 221.49 in the same window ≈ Rapid-MLX's 189.59 — the
    "1.45× llama.cpp" was CROSS-WINDOW; the ~190 prefill frontier is ENGINE-GENERIC.**
    The same session found + FIXED our long-context decode deficit (0.74× → 0.93× @2K,
    +25% e2e, greedy-token-identity EXACT): split-K Metal flash decode
    (`QwenAttentionDecodeGatedSplit{,Combine}CubeCL`, default-ON >512 pos).
    **Batch 95 (LANDED 2026-09-01, Plan 055 / Research 111): mlx-lm
    `gated_delta.py` GDN prefill — 3 rules, corpus 411 → 414 (D 85, E 26,
    G 26).** `simdgroup-row-packing-reduction-amortization` (E),
    `whole-sequence-zero-barrier-single-dispatch-scan` (G — the FOURTH
    recurrence dispatch strategy beside scan/chunk),
    `reduction-restructure-needs-source-owned-comparator` (D — the
    comparator-ORACLE discipline; the snapshot's rider estimate was
    PROMOTED to a full rule: both subagent lanes independently converged
    on it as decision content). Riders: B38 env-flag, B23
    hardcoded-fast-path eligibility, B23 gqa; B79 sequential-loop note
    confirmed for GDN. Snapshot corrections folded at mine time: the
    reduction split is 3-level in-lane + 2-level cross-lane (not 2+2);
    the "(3) RIDER" comparator discipline carried full-rule content.
    GOAT: batch95 gate + kernel_heal 1235/0 + default 1001/0 + clippy 0
    both states + retrieval floors 14/14 HELD; full CI guard not run (the
    3 affected states validated; recorded in Research 111). **Streaming axis added 08-31 (Research 110 §8):**
    oMLX PR #3260 streams Flash-Next-class checkpoints (99 GiB) on 18–36 GB Macs at
    15–17 t/s, and DwarfStar/ds4 (antirez's DeepSeek-V4 engine) publishes M3 Max 128 GB
    q2 at 26.68 t/s decode / 250.1 t/s long prefill — the ≤24-GB re-arm condition should
    read "RAM-feasible or streaming-feasible"; owner call whether the league admits a
    streaming opponent.
  - **NEW 2026-09-02 (riir-ai Issue 843 T5 / league opponent-watch intake): the 1.4×
    prefill jump is REAL, root-caused from source — PrismML fork `9ca265a → e311ed38f`
    landed native Metal PQ2_0 kernels (`73f0fed04`, 08-25, `ggml-metal/kernels/mul_mv.metal`
    `kernel_mul_mv_pq2_0_f32` + quantize.h codec — verified ZERO PQ2_0 mentions in
    `ggml/src/ggml-metal/` at 9ca265a, so the old pin ran type-142 through the generic
    dequant fallback). Secondary in-range fork commits: GDN ring decode (`bed6e9a6b` +
    `08d8c9035`, the known +3.2% decode move, already distilled as the mlx-lm GDN
    batch-95 lane's external confirmation), `e8b6452b1` im2col fix + wide-row set_rows,
    `e0c828b90` Hadamard weight-fold runtime. ~~ARMED DISTILL SOURCE:
    `../vendor/prismml-llama.cpp @ e311ed38f` — `ggml/src/ggml-metal/kernels/mul_mv.metal`
    (PQ2_0/Q2_0 group-128 native Metal mul_mv: 8 sub-blocks × 16 weights, SIMDWIDTH/8
    lane packing — same ternary-class codec as our Q2_0 GEMV family; verdict B, same
    model/quant as the league opponent) + `set_rows.metal` wide-row KV write. Not yet
    mined; next idle-loop batch candidate after the arxiv re-walk.~~ **MINED 2026-09-02
    — Batch 96 (Research 114 / Plan 060): 4 rules, corpus 422 → 426 (N +2, D +1, B +1):
    telescoped-digit-coefficients-byte-fma-dot (N), bit-plane-plane-sums-input-side-
    offset-dot (N), wide-row-copy-tile-rows-across-workgroups (D),
    fwht-shuffle-xor-butterfly-register-resident (B) + riders (B20/B92 byte-perm
    external measured confirmation on HIP v_perm_b32, MI300X 1.9–2.9× decode rows;
    CDNA-vs-RDNA MMVQ geometry regime). Source verdict upgraded to A- at mine time
    (measured HIP rows + upstream PR 26980 lineage).**
  - **MINED 2026-09-05 — Batch 105 LANDED (Research 127 + 128 COMBINED / Plan 075): 11 rules, corpus 466 → 477 (A +1, C +1, D +3, E +1, F +1, H +3, M +1; entries_gpu_18.rs).** The combine mandate executed: both sources mined in one corpus cycle with the cross-source pairs cross-referenced IN-rule (indirect-tile-map vs capacity-bound-sentinel = opposite postures, both landed; lane-sequenced vs atomic scatter = determinism contrast, both carried). Mine-time falsifications: uzu "RELAXED everywhere" REFUTED (strict-layout override arm on tree_verify); the "Hadamard of a constant column" comment DOES NOT EXIST in-tree (quote killed, the bias-demotion code is the evidence); lily tile policy is CHUNK-SIZE-routed (32/64 rows, both 4 simdgroups) NOT expert-load-routed — the B77 rider framing above is corrected; family-gate refusal text says "family 10, M5 and later". The lily GOAT lead (concurrent-dispatch) landed corpus-side with its M3 A/B fixture DEFERRED (Plan 075 T7 — GPU lane claimed by 851 Phase A; re-arms when quiet). Batch 106 slot MERGED into 105 (the next batch takes 106). ~~Deferred candidates (recorded for a future batch): uzu placement-sparse-demand-paged-kv (B+), gumbel-argmax-unified-sampling-descent (B+), multi-field-argmax-tuple-reduction (B−), flat-topology-fast-path (weak), integer-mma-epilogue (B−), fused-count-scan (C+), radix-select (B−); lily chunk-command-buffer-prefill, cross-level-fused-residual-norm, RIF-sentinel, tensor-ops-device-q, load-time-concat-rows (= B63 external instance), weight-hygiene cluster.~~ **MINED 2026-09-05 — Batch 107 (Research 130 / Plan 077): ALL defers consumed — 8 rules landed (corpus 490), 3 dropped with reasons, 1 collapsed to the B63 rider; load-time-concat-rows = B63 external instance (rider note in Research 130, not a rule). Upstream hazard found: uzu counts_offsets_fused.metal counts_shared[128] OOB writes for 128 < E ≤ 512 — CONFIRMED + EXTENDED in B107 (chunk-read missing base).** Gates: batch105 3/3, kernel_heal 1338/0/1i, default 1076/0, retrieval floors HELD, count pins walked (tests.rs totals/expected-array/sums/grouped + router.rs + mod.rs member lists + 7 category comments).
  - **(verdict record, now mined — Research 128 — perplexityai/pplx-garden `lily/` @ 1ed972ed3f0bd5616c997c9507c25616c63394fc, Apache-2.0 — Perplexity's Rust+Metal Qwen3.6-35B-A3B engine, blog carries MEASURED matched ablations):** YES grade **A−**, route kernel_opt (ds4/B97-98 posture — M5+/family-10 runtime gate, read-only on the pinned M3). Two A− leads with ZERO corpus coverage: `gpu-resident-moe-routing-no-host-sync` (histogram→scan-offsets→scatter→tile-map in ONE command buffer, CPU never inspects routing intermediates, blog +89% prefill @512; distinct from B97 map-building — residency/sync-boundary is the decision) and `concurrent-dispatch-dependency-level-barriers` (MTLDispatchType::Concurrent decode pass, level_barrier at every true data edge, 795 kernels → 555 stages; distinct from B26 in-order merge, B70 counters, B100 fence-values; **the GOAT lead — benchable on M3, Concurrent dispatch is Metal 3.1-era not M5-gated**, riir-gpu's raw-Metal lane is the fixture vehicle). B+ pair: `pipelined-token-slot-ping-pong-submit-ahead` (U32[2] next_token slots, argmax writes step N+1's INPUT, submit-before-wait across two command buffers; distinct from B25 argmax-download + B58 pool rotation) + `split-k-two-pass-attention-decode-long-context` (FixedBlock route ≥32768 with exact_model_shape gate, MLX sdpa_vector_2pass_1 adapted; blog +7.7/27.4/40.2% @32K/64K/128K; zero corpus coverage). Riders: B16 dequant-in-tensor-op-GEMM (+77.4% external measured), B77 tile-by-expert-load axis (+13.2%), B100 GQA-fold table row hit a THIRD time (d256→hpt4 @KV≥8192; MLX+MTPLX+Lily), B95 GDN register-scan (same mlx-lm upstream), B62 register-A skinny router. Negative (league): spec-decode −18% batch-1 (THIRD independent confirmation of Issue 717/721), decode weight-read-bound ±0.2% (Bench 763/771), orchestration-not-kernels is where the headroom was (roofline 90-97% on M5). pplx-garden's RDMA surface (fabric-lib/p2p-all-to-all/gdrapi): **NO** — cluster domain, zero benchability; mine `lily/` only. **Mine-time mandate: cross-reference Batch 105/uzu in the dedup gate** (same model family, both MLX-derived — uzu lane-sequenced-scatter vs Lily atomic scatter, uzu indirect-tile-map vs Lily tile-map are likely one family with different determinism postures); combine 105+106 or sequence strictly. Repo-shape trap recorded: the zread index was STALE pre-#19 (showed RDMA only, no lily/) — verify repo shape against the commit graph, never a cached structure fetch. Plan-first at batch open. Research 128. YES grade **A−**, route kernel_opt; 2 inspection lanes DONE (A: matmul/GEMM/MXU/split-K/RHT/qmv; B: attention/MoE/GDN/SSD/radix-topk) + 16 quotes coordinator-verified + dual-vocab dedup DONE. **Leads: `indirect-device-dispatch-tile-map-moe` (A− — device-computed grid via MTLDispatchThreadgroupsIndirectArguments, ragged MoE M-axis never round-trips to host; zero corpus coverage), `cooperative-tensor-mma-capability-route` (B+ — Apple MPP cooperative-vector matmul2d, supportsMXU fail-closed probe, M5+/Gen18+ next-silicon, B103-NAX posture), `rht-epilogue-fusion-contract` (B+ — output-RHT fused only when split_k==1, bias demoted post-RHT ("Hadamard of a constant column is not that vector"), requires quantized W; composes with B98), `target-tile-split-k-deterministic-two-dispatch` (B — quant-class target-tile table + quant-group alignment ladder + TWO-kernel fixed-order reduce; two-regime vs B92 semaphore), `a8-activation-route-stability-guard` (B — re-run dispatch with a_full_precision=false, adopt Int8 only if still MXU-tier), `lane-sequenced-deterministic-bucket-scatter` (B — atomic-free deterministic MoE permute, 32-step lane lockstep; distinct from B97/B102/B94), `packed-nibble-tensor-operand-alignment-gated` (B — W4 consumed natively by the tensor unit when aligned).** Riders: B49 ×2 (SSD runtime-bounded lane_states + the state-64 specialization; METAL_PRAGMA_UNROLL ignored under metal4.0 -O2), B97 frontier-clamp instance, B103/B100 GQA-M-packing + route-ladder instances, B104 tg-per-core occupancy-target table + qmv 300-row unanimous-family-consensus table (1176-decision xxh3 fingerprint), B98 GEMM-over-GEMV carve-outs, LinearAttn state-resident-chunked instance (MXU_MIN_T 256 / CHUNK 64; chunked MXU-gated so M1–M4 run plain recurrent — corroborates our Issue 734 negative), Morton counter-regime note on l2-grouped-tile-schedule (4× padding-budget gate). Drops: Morton-as-rule (covered), attention ladder standalone (B104), chunked-GDN standalone (B79/B95/Issue-734), weaver/speculators/sparse (unread, decode-side or flag-at-mine). Caveats: NO committed perf numbers in-tree; MXU = M5+ only (zero pinned-box effect); quotes re-verify at mine time. Mined as Batch 105 (combined with Research 128; see the LANDED note above). Research 127.
  - **NEW 2026-09-02 late (league opponent watch #5, M3 lane — zero perf movement on the
    fork, upstream M3 Max fa-vec tuning LANDED):** fork `origin/prism` moved only 3
    commits past the pin `e311ed38f`, all docs (CONTRIBUTING/AGENTS ×2) — **the pinned
    opponent binary stays valid, no re-pin**. Upstream ggml-org since 08-25: ~26 Metal
    commits, ONE load-bearing for this box — `8963a9bdc` (08-28, #27863) adds
    `fa_vec_tuned_table` rows for **M3 Max** (F16 + Q8_0; 64 GB MacBook Pro, low-power
    mode) to `ggml-metal-tuning.cpp` — AFTER the fork pin, fork does not carry it.
    League reading: (a) all scored cells remain FAIR (both sides untuned at measure
    time); (b) the opponent bar is now POTENTIALLY STALE — any future re-pin at
    upstream absorbs per-(device, dtype, head_dim, ne11/ne01-bucket) fa-vec launch
    tuning and the attributable-delta rule applies to every flash-attention cell;
    (c) the fork policy (read-only vendor binary, owner call Issue 782) is unchanged
    by this — record only. **Distill lead (kernel_opt, mechanism class):**
    ~~`fa-vec` tuning-table dispatch — per-device/per-dtype/per-(dk,dv)/per-context-bucket
    swept launch-parameter tables for the flash-attention vector kernels + the
    `ggml-metal-tuning fa-vec` autotune sweep harness that generates them — vs our
    two-point length gate (m16 <8192 ≤ / m32 ≥8192) on the tiled flash; the swept
    multi-bucket axis is the richer shape.~~ **MINED 2026-09-05 — Batch 104 (Research 126 /
    Plan 074): 2 rules, corpus 464 → 466 (K 5 → 7); see Last landed above.** The rest of
    the upstream Metal movement is
    the same mechanism for OTHER chips (M1/M1 Max/M1 Ultra/M2/M2 Pro/M2 Max/M3 Pro/
    M3 Ultra/M4/M4 Pro/A18 Pro — ~15 commits) + `d5d993a09` Metal 4.0 tensor API on
    M5+/A19+ (next-silicon external confirmation of the matrix-core-attention family
    note in Issue 831's disposition) + `2a74817f9` top-k radix (sampling path, not our
    hot loop).
  - **VERDICT 2026-09-03 (this session — the 846 deferred intake, kernel-reading
    extraction from `../vendor/mlx` @ f99e916; the riir-ai 844 reading extract's
    two candidate rules verified against the ACTUAL kernel bodies at
    `steel/attn/kernels/steel_attention_nax.h` 907 lines, full read):**
    - **Candidate rule 1 — causal-shadowed padding read for tile alignment
      (`has_backing_rows`): CONFIRMED NOVEL, kernel-grade, corpus-worthy.** The
      orchestrator-level reading held: kL widens to the next bk multiple IFF the
      K/V slices provably back the padded rows (negative-stride reject + row-pitch
      containment `row0 + rows <= pitch` + buffer-bounds end check); padded rows
      sit beyond every query's causal horizon (`qL_off = kL − qL` preserves the
      true diagonal), garbage reads are shadowed by the causal mask, and the
      ragged-tail masked-store branch is DELETED for the common chunked-KV-cache
      shape. Dedup verified: `has_backing_rows`-style backing-provenance is
      ABSENT from the corpus (B84 pad-tail PADS THE DATA via a scratch copy +
      strip-before-stateful-consumer — a different mechanism entirely: data-side
      padding with a crossover tuner vs extent-side widening with read-provenance
      proof and NO data movement; B17 clamp+discard keeps the guard; B100
      early-return derives extent from row metadata — none carry "prove the rows
      exist behind the slice, then read past the logical extent under causal
      shadow"). The provenance check itself (stride sanity + pitch containment +
      buffer end) is portable to ANY engine whose KV slices view a ring/cache
      allocation. Proposed id: `backing-provenance-causal-shadowed-extent-widen`.
    - **Candidate rule 2 — hd-256 head-dim split (`attention_nax_dsplit`):
      CONFIRMED as the B100 external-confirmation instance** (`register-budget-
      head-split-gqa-reread-trade`), exactly as the 844 extract predicted —
      split the head dim across WN=2 simdgroups of the second warp dimension,
      halving the accumulator set, K/V re-read per half, redundant-per-half
      softmax with ONE smem exchange of the partial Q@K.T fragment pairs
      (`s_xchg[WM][WN][2*kEPF*32]`, both halves share the frag-to-lane mapping
      so the exchange needs no coordinate math). Fold as an instance/rider at
      the next B100 touch, not a new rule. Two EXTRA specifics worth pinning in
      the fold: the Q-half register-residency for the whole KV loop
      (`NAXTile<T,1,1> Qtiles[TDh]` hoisted OUT of the loop — the TDh fragments
      are "cheap next to the accumulators"), and the exchange reducing EXACTLY
      ONE peer so WN is structurally fixed at 2 (static_assert).
    - **NAX-gating boundary re-verified live:** `use_fallback`'s widening branch
      AND both NAX kernel routes require `metal::is_nax_available()` (M5-class
      per-core neural accelerator) — absent on M3/M4, so the widening cannot
      move our pinned M3 opponents at any version (the 0.32.2 reference stays
      structurally unaffected; version-pinning applies to M5-class cells only).
      ALSO unfused on M3: hd-256 falls to the decomposed path at seq > 8
      (`return query_head_dim == 192 || query_head_dim == 256`), and the
      non-NAX steel kernel's shape table tops out at bd=256 with bq32/bk16 —
      never compiled for the Bonsai shape's fused use.
    - **Disposition:** rule 1 QUEUED as a next-batch kernel_opt lead (batch
      ~B102; needs fixtures + GOAT per standing discipline — the pseudocode is
      host-side extent-widening + a provenance predicate, straightforward to
      fixture); rule 2 = B100 rider (deferred to the next B100 touch); nothing
      routes to the flash-vs-GEMM prize as a lever (NAX is hardware-unavailable
      on M3 — the prize stays KV-traffic amortization per T1). The 846 handoff
      item DISCHARGED at the reading level; fixtures own the rest.
  - **NEW 2026-09-02 late (MLX watch, `../vendor/mlx` 37c26e5 → 90846ad):** three
    Metal-kernel leads adjacent to our lanes: (1) `f99e916` #4416 — head-dim-256
    prefill with array mask now runs the FUSED kernel path (Bonsai IS hd=256 — read
    for what gated the fallback); (2) `685dba9` #4431 — `sdpa_vector_2pass_1_gqa`
    batch-offset fix for K/V in GQA decode — cross-check target for OUR split-K
    decode pair (Bench 831 `QwenAttentionDecodeGatedSplit{,Combine}`, G1 0/512 flips;
    their bug class = per-batch K/V offset in the 2-pass split — **VERIFIED CLEAN
    2026-09-03 (4090 box, read-only inspection unit, zero-contention while the
    Plan-371 window runs)**: our pair has NO batch axis (single-token decode —
    query `n_head*head_dim`, KV `n_positions*n_kv_head*head_dim`; sequences never
    share a buffer with an implicit batch stride), so MLX's bug dimension is
    structurally absent. The analogous axes all check out by inspection of
    `qwen_attention_cubecl.rs` (f32 split L948-1151, f16 split L1459-1688,
    combine L1160-1218, geometry L922-927) + the caller
    (`ternary_deltanet_gpu_forward.rs` L2387-2393 alloc, L4265-4300 launches):
    (a) GQA per-head K/V offset `kv_group = head_idx*n_kv_head/n_head` at
    `pos*kv_stride + kv_off + d` correct in BOTH twins (f16 word twins
    `kv_stride_words`/`kv_off_words` exact under the even-head_dim contract);
    (b) per-split position ranges disjoint + complete — hd-aligned split_len
    (`split_decode_geometry`), split i owns `[i*split_len, min(+split_len, n_pos))`,
    first_tile division exact, over-dispatched splits write EMPTY partials
    (m=-1e30/l=0/out=0 → w=0 in the merge; the stale-byte×NaN hazard guarded by
    design comment); (c) split↔combine partials layout identical
    `(head*n_splits+split)*(hd+2)`, BOTH launches take n_splits from the SAME
    geometry call, `new()` allocation uses the same 64-split cap (monotone-superior
    to any per-call n_splits; every combine read is written by the same call's
    split launch — no stale cross-call reads); (d) f16 K-dot q↔word pairing + V
    dim-pair ownership verified element-by-element, per-dim summation order
    matches f32. Boundary notes (NOT bugs): hd ≤ 256 is the family-wide smem
    ceiling (all 6 attention kernels allocate 256-float smem; tree/prefill kernels
    document it in comments, decode kernels imply it) — a future hd>256 model would
    OOB-write smem, pin with a debug_assert if such a model ever lands;
    `split_decode_geometry(0, ..)` would div-by-zero (unreachable — decode always
    has ≥1 position).; (3) `bbf8a7a` #4380 — widens the 2-pass
    GQA decode kernel to GQA sizes 12/16 (threshold data for when split-K pays —
    ours fires >512 pos regardless of GQA). Leads (1)/(3) were marked not-yet-mined
    low-priority — **both now DISCHARGED 2026-09-03, see the extraction unit below**
    (decode lane stays league-defended; prefill is the open prize — neither changes
    that); lead (2) DISCHARGED as the verification above — verdict CLEAN, no corpus
    delta, no issue filed.
    **Leads (1)+(3) DISCHARGED 2026-09-03 (4090 box, read-only kernel-extraction
    unit #2 — `git show` on the vendor clone, zero contention while the Plan-371
    window + B102 run):**
    - **(1) `f99e916` #4416 — CONFIRMED the dispatcher-drift rule shape, QUEUED
      as the second next-batch kernel_opt lead** (candidate id
      `dispatcher-eligibility-predicate-lags-kernel-capability`, name final at
      batch time). Mechanism, fully verified at source: #3842 (`714a7ef`) landed
      the hd-256 NAX fused path with the gate `do_causal && !has_arr_mask`, and
      the KERNEL carried full array-mask support from the SAME commit —
      `has_mask` function_constant(300), mask buffer bindings (5/6), in-kernel
      mask reads in both the main + dsplit kernels (verified in
      `steel_attention_nax.h` at 714a7ef). The `!has_arr_mask` conjunct was dead
      conservatism for the kernel's ENTIRE life: every array-mask hd-256
      seq≥1024 prefill silently ran the decomposed fallback — correct output,
      slower, NO test fails, profiler shows work happening. #4416 deleted the
      stale conjunct 13 days later (`(do_causal || has_arr_mask)`), touching
      ONLY the 2-line dispatcher predicate — zero kernel changes. The
      soundness evidence for any such widening lives in the kernel's
      SPECIALIZATION SURFACE (func consts / mask-typed kernel name), not in
      the predicate. Fix-space: align the predicate with capability + a
      POSITIVE ENGAGEMENT test (assert the fused path ENGAGES at every shape
      the kernel supports — output-parity tests cannot see this failure
      class). NAX-scoped evidence (M5-class), portable mechanism — our own
      capability-conditional gates (split-decode >512, tiled-flash/SMEM tiers,
      m16/cmma) carry engagement canaries from their GOATs, which IS the
      standing defense the rule prescribes. Dedup verified: B23
      hardcoded-fast-path (specializes KERNELS for known configs — not
      predicate drift), vendor-batched-gemm-contiguity-staging (external
      library's internal path chosen by LAYOUT — host fix is contiguization;
      here OUR predicate vs OUR kernel, fix is predicate re-alignment),
      aliased-slot-allocate-replace-fallback (a semantically WRONG fallback
      vs a merely-slower one), gate-prep-fold capability gate (loud REJECT on
      incapable backends — the inverse direction). Landing gated behind B102
      (kernel_opt corpus contention) with the NAX rule-1 lead.
    - **(3) `bbf8a7a` #4380 — B100 rider ENRICHMENT recorded** (folds at the
      next B100 touch beside the NAX rule-2 instance): the 2-pass GQA decode
      kernel adds gqa 12/16 instantiations (table: 64/g8/hpt8, 128/g8/hpt4,
      128/g12/hpt4, 128/g16/hpt2) — hpt FALLS as the GQA factor rises, the
      register-budget trade in the exact B100 direction, a SECOND external
      instance beside the NAX hd-256 dsplit. Dispatcher predicate generalized
      to `gqa_dims` (g8×{64,128} ∪ {g12,g16}×128); the 2-pass ELIGIBILITY
      length threshold is unchanged at KV ≥ 8192 — boundary note: MLX's 2-pass
      vector kernel pays only at 8K while our split-K flash decode fires >512
      (Bench 831) — DIFFERENT kernel families (their single-pass vector baseline
      vs our gated flash split), not a contradiction, do not cite as one. Our
      GQA factor is fixed so the g-axis does not bind for Bonsai — recorded as
      threshold data only.
    - **Watch pin + tooling note (same unit):** MLX origin/main re-verified ==
      `90846ad`, zero new movement since the recorded watch tip; the box's
      clone checkout sits behind-14 stale (local HEAD `c899c5a`) — refreshed at
      mine time per the `.raw` discipline, not by this read-only unit.
      Extraction-tooling: `git grep <rev> -- <wrong-path-prefix>` returns EMPTY
      exit-1 — indistinguishable from `absent at this rev` (this session first
      mis-read the NAX kernel as mask-less via a `metal/steel/...` vs
      `metal/kernels/steel/...` path slip; `git show <rev> --stat=200` prints
      full paths and is the disambiguator — the same silent-empty class the
      multi-root workspace grep warns about).
  - **LANDED 2026-09-02 (Batch 97, Research 117 addendum / Plan 063): antirez/ds4
    (DwarfStar) distill — 9 rules, corpus 426 → 435 (N+1 E+2 C+2 H+1 M+1 A+1 F+1).**
    Pin `110afdd8`; quotes re-verified byte-accurate; provenance vs upstream
    ggml-org/llama.cpp @ 9ca265a (the check FALSIFIED the mask-block classification
    as llama.cpp-lineage — landed with corrected origin, the popcorn precedent).
    GOAT lead `half-domain-exact-band-dequant-with-raw-bit-oracle` (N) landed with
    the mandatory T6 exhaustive CPU twin — which caught its own first-implementation
    bug (implicit mantissa bit overlapping the exponent field in the normal path).
    Lane B delivered `bit-exact-dispatch-fusion-boundary-emulation` (C) +
    `virtual-thread-folding-preserves-reduction-tree` (E) + the mask rule (F).
    Dropped: candidate 7 row_blocks (LICM/B44). GOAT all green (kernel_heal 1252/0,
    default 1008/0, floors 14/14 HELD, clippy 0, score-bench FLAT).**
  - **~~NEXT BATCH LEADS (from the same source, dense.metal — DS4-native matvec
    family, NOT llama.cpp-lineage):~~ MINED 2026-09-02 — Batch 98 (Research 118 /
    Plan 065): all 5 leads + both P8-intake candidates consumed (10 rules + 4
    riders; the dense.metal lead file is now FULLY MINED).** Original leads for
    the record: (1) quad-composition via pair-impl
    composition + args aliasing — ONE kernel hosts FOUR same-input projections
    by splitting the grid at tgs0 and calling the existing pair impl twice with
    a per-range aliased args struct (dense.metal L1100-1153, device-scope barrier
    before the state append); (2) `kernel_mul_mv_id_mxfp4_sum6_*` route-fused
    down-projection SUM — six routes fused into one accumulation pass with a
    single final simd_sum (moe.metal family, same-input member); (3) the
    shared-read-not-merging decision boundary — two same-input Q8_0 matrices
    that share the ACTIVATION read but deliberately keep per-projection
    reduction shapes (dense.metal L325-330, the family's boundary case); (4)
    tiny-dense-matvec scalar fallback — parallelism DOWN-shift for short rows,
    trading lanes for dispatch + reduction overhead (dense.metal L1440-1441);
    (5) small-batch matvec-ext bridging GEMV and GEMM for 2..8 prompt tokens
    (dense.metal L1596+L1697). P8-intake candidates: host ds4_metal.m
    single-entry pipeline memo (L3077-3081, "decode calls this once per layer
    with identical arguments"); M5 stable-sigmoid-vs-exp annotation (tanh form
    produced non-finite logits at fused vector sites on M5 Max while finite in
    the standalone path — dsv4_hc lane).**
- **Sibling lanes (do not touch):** **riir-train — OWNER-RESTRICTED (2026-09-04): do not
  route NEW work to riir-train for any task; only riir-clippy-related units are sanctioned
  for this session lane.** Existing riir-train in-flight lanes stay with their owning
  siblings (e.g. `/tmp/riir-train-499b`); read-only citations of riir-train records are
  fine — never initiate fixes/gates/benches there. The 4090 perf lanes — sibling PV work
  mid-flight as of 08-29 (re-verify lane state at session start before any 4090 unit).
- **2026-09-03 (4090 box, second unit — plan-board truth sweep, zero-contention while the
  Plan-371 window + sibling B102 run):** katgpt-rs Plan 586 reconciled to RESOLVED-SCOPED
  (the GPU gates had PASSED 09-02 while the plan + the 845 line above still read
  "armed/GPU-window-gated" — a future session could have re-run a passed gate);
  riir-ai Issue 847 status line reconciled (T7/T8 were already `- [-]` defer-marked; the
  "Remaining" clause now names only the owner promotion call). Plans 582 (CLOSED-NEGATIVE)
  Plans 582 (CLOSED-NEGATIVE)
  and 584 (OPEN-DEFERRED with triggers) verified already-honest — untouched. Plan 530
  flagged for a future doc-sync pass (status "IN PROGRESS — pivoted to inference prefill"
  predates the Bench-641/653/730 prefill arc; absorption not verified, not marked blind).
  katgpt-rs ff'd 345e301f→f84f5c7e first (the 713/716 release-gate session's landings).
- **2026-09-03 (4090 box, third unit — doc-sync on the Plan-530 flag above, zero-contention
  while the resident riir-train GPU run holds the box): riir-ai Plan 530 reconciled to
  RESOLVED** (riir-ai `f5e8c430e`) — pivot arc CLOSED (Goal 2's ≥3× kernel gate superseded
  by the prefill arc: Benches 774/790/805 default-on, M3 prefill @2048 44.94→~105.78 tok/s;
  Goal 3 landed — batched prefill DEFAULT-ON via Issues 653/730); backward remainder VERIFIED
  LANDED cudarc-side in riir-train-gpu (`ternary_deltanet_backward_cudarc.rs` Phases 1–4,
  cites "Issue 641 T7.5", gate `test_gpu_backward_matches_cpu_reference`, consumer riir-train
  arm_c under `lora-gpu-backward` "T7.6") — tasks T6–T11 marked `[x]` DONE-AS-CUDA, T2–T5/T12
  `- [-]` superseded/void with reasons (Metal/CubeCL backward never built + now
  boundary-forbidden in riir-ai per Issue 741 / `ci_modelless_residue.sh` C6). The
  GPU-training-forward lane's live state = riir-train Issue 729 (open-loop, option-1
  deferred with reopen trigger). Flag consumed — no open doc-sync leads remain from the
  09-03 plan-board sweep.
- **Post-mining intake: 3 open classes** (A + B filed 2026-09-07, C filed 2026-09-08 —
  all from riir-ai Issue 888; the three entries below). P8 nan-unsafe float sort
  comparator CLOSED
  2026-09-01 (landed as sec `nan-unsafe-sort-comparator`, Plan 057; record below); all
  prior classes closed (records below; full text in the history note).
- **2026-09-07 (M3, riir-ai Issue 888 consumer session) — OPEN class A:
  `config-default-is-a-named-constant-and-the-consumer-reads-the-constant`.**
  A static, cheap, high-precision lint candidate, measured **3x in one repo**:

  | field | default expression | what the consumer actually reads |
  |---|---|---|
  | `LoraConfig.target_modules` | `LoraTarget::all().to_vec()` | `LoraTarget::COUNT` — **~35 sites / 9 files** |
  | `AuditConfig.ring_capacity` | `AUDIT_RING_K` | `AUDIT_RING_K`, including as an **array length** (`[AuditEntry; AUDIT_RING_K]`), so the field is structurally un-honorable |
  | `CrowdMcgsConfig.belief_decay_rate` | `DEFAULT_BELIEF_DECAY_RATE` | `DEFAULT_BELIEF_DECAY_RATE` |

  **The shape:** a `pub` config field whose `Default` value is a *named constant or
  associated const*, where the consuming code reads **the constant** rather than
  `self.field`. Result is silent-wrong configuration — the caller sets the knob, gets the
  constant, and any A/B over that dimension measures nothing. `rustc`'s `dead_code` cannot
  see it (a `pub` field is assumed used-by-API), so it is invisible to
  `cargo clippy --all-features --all-targets` at every feature state and on every platform.

  **Why a lint beats a grep here:** the trigger is fully local and syntactic — (1) the field
  has a `Default`/constructor init whose RHS is a path expression to a `const`, (2) there is
  no `.field` read of that field in the defining crate, (3) that same const path IS read
  elsewhere. Strictly more precise than the whole-repo heuristic the consumer used
  (`riir-ai/scripts/inert_config_knob_report.py`: 57 candidates, 36 verified by hand).
  **Two guards learned the hard way in that consumer:** the field may be read by
  **destructuring** (`let Cfg { .., field } = cfg`) *in its own file* — that blind spot put
  three LIVE fields in the report's most-confident bucket — and the defining crate may be
  **upstream**, so 7 of 22 rows turned out live in a sibling repo. A lint runs per-crate
  with type info and dodges both, which is exactly why it belongs here and not in a sweep.
- **2026-09-07 (same session) — OPEN class B: `assert-on-default-config-value`.**
  A test that asserts a config field equals its own default —
  `assert!(cfg.sc_inference)`, `assert!(!cfg.track_variance)`,
  `assert!((c.learning_rate - 0.0002).abs() < 1e-6)` — is often the **only** read of that
  field anywhere, so an inert knob reads as **covered**. Measured: 2 rows in riir-ai
  (`SelfConditionConfig.sc_inference`, `ElboAuxConfig.track_variance` — both asserted from a
  *sibling repo's* test, which is also how they escaped the consumer's first pass), and the
  same shape recurs across riir-games / riir-games-quest `learning_rate` assertions. It is a
  **generic** smell, not a riir-ai one: asserting `Default` reproduces `Default` and proves
  nothing about the field being *used*.

  Proposer angle: flag a test whose only interaction with a `*Config` field is an
  equality/boolean assertion against the literal appearing in that type's `Default` impl,
  and suggest a **behavioral** assertion instead — the catching test is always behavioral
  ("at f=0.5 `bytes_total()` is ~half the f=1.0 value"; "`target_modules=[Q,V]` yields 2
  adapters, not 6"). Same family as the Plan-014 fake-coverage concern this file tracks.
- **2026-09-08 (M3, riir-ai Issue 888 T15/T16/T18 follow-up) — OPEN class C:
  `config-path-orphaned-by-refactor-or-never-plumbed` (two sub-shapes, one detector
  family).**

  **C1 — orphaned-by-optimization** (the T17 watch, filed per its mandate): a perf
  refactor replaces a config-fed path with a compile-time table/match and copies the
  values faithfully — zero test fails, the knob stays pub and looks live. Measured once
  (riir-games-civ `BehaviorConfig.alarm_radius`: a 20-arm `match` duplicated all 22
  per-species values after the const-table optimization; fixed by wiring to the
  SpeciesRegistry, riir-ai `88c51824b`). Detection needs HISTORY, not just syntax —
  the field had a reader that vanished — so this is a **review-time + sweep-time**
  heuristic, not a per-compile lint: on any commit touching a `Default`/constructor
  init of a `pub` config field, check the field still has a non-test reader at the new
  HEAD (the class-A lint's condition 2 reused as a post-hoc check).

  **C2 — self-referential-config-struct** (the T18 escalation): a whole `pub` config
  struct whose only readers are its own `validate()`, its own tests, and a GOAT-proof
  file's config-only tests — plus advertised TOML/YAML sections nothing loads (the
  section name colliding with a DIFFERENT live struct elsewhere makes it worse:
  `PlasmaConfig`'s `[domain.inference]` doc pointed at the router's `InferenceBudget`).
  Measured once (riir-games `PlasmaConfig`, 10 fields, presets advertising per-game
  tuning values that tuned nothing; deleted, riir-ai `dbb939b26`). Detector shape:
  per-struct — collect all `.field` reads of the type's fields crate-externally; a
  struct with zero external reads is class C2 regardless of internal reads. Same
  type-aware advantage as class A: `dead_code` cannot see it (pub = assumed-used).

  Both sub-shapes share class A's two guards: destructuring reads in the defining file,
  and upstream-crate liveness (a struct can be live in a sibling).
- **2026-09-03 (M3, idle loop — E4 decay-gate triage, the flat 35-run curve resolved):**
  the score bench has read 96.2/1.9 since 08-25 and the audit's E4 gate referenced the
  pre-P9 peak (98.1), printing a false decay context every run. Full miss-map triage
  (`per_fixture_diagnostics`): ALL THREE entries are documented, probe-verified
  boundaries — (1) `map_unwrap_or` persist on `riir-ai-ruliology-player-001` = the P9
  trait-object-cast mapper guard declining CORRECTLY (probe RE-VERIFIED 2026-09-03 on
  the current toolchain: the map_or rewrite still E0308s — expected `&fn(..)`, found
  `&dyn Fn(..)`; original compiles — the guard is sound, we-stay-silent is SAFER than
  clippy's autofix); (2) `useless_format` persist = the Run-#3 String-typed frontier
  (bench-symmetric trade, `created` is the trust axis); (3) `single_option_map`
  created = clippy's own cascade on `manual_map`'s output. **The 98.1→96.2 step was
  DENOMINATOR GROWTH (the P9 fixture deliberately frozen as a declined lint), not
  healing decay.** E4 re-pinned in `scripts/self_evolve_audit.sh` to the post-P9
  stable floor (96.2/<=1.9, tie slack) and now EVALUATES (was a static print; hr/cr
  were computed but never compared). Fixtures untouched — reclassifying correct
  declines as resolved would be the fake-coverage gaming Plan 014 exists to prevent.
- **2026-09-03 (4090 box, fourth unit — plan-number collision NOTE, discovered at the
  post-unit rebase):** `.plans/070` is DUAL-ALLOCATED between two sibling lanes —
  `070_kat062_t6a_counterfactual_surgery_gate.md` (COMMITTED `eae2e4e`, pushed —
  Status COMPLETE, Bench 066) vs `070_flashquant_sparse_dense_fusion_distill.md`
  (untracked WIP, Status IN FLIGHT, created 09-03). **Tie-breaker (Bench-697/848
  push-wins + baked-refs): kat062 KEEPS 070.** The flashquant plan MUST renumber →
  **071** at its next write time (read highwater 70, take 71, write back 71 — the
  current `.highwater`=70 is already the correct next-free state; do NOT take a
  number AT highwater, the 848 lesson) + grep-fix any live refs to "Plan 070"/"070_"
  inside the file. The number is not taken by another session here — this note is
  the flag; do not commit through the duplicate name. **UPDATE 2026-09-03 (verdict-filing
  session): `.plans/.highwater` now reads 72 — Plan 072 allocated to the MTPLX PR391
  batch (Plan 072 / Research 122, committed). 071 stays RESERVED for the flashquant
  renumber per the flag above; a renumber write-back must be max(current-highwater, 71)
  so the bump never regresses (the Bench-675 class).** **RESOLVED 2026-09-04 (B102
  takeover session): the flashquant plan renumbered to 071 and LANDED (Research 123
  / Plan 071, all tasks `[x]`); highwater stays 72; the duplicate-070 state is
closed.**
- **Owner-gated tails:** `.issues/044` (paging secrets `ALERT_WEBHOOK_URL` +
  `MONTHLY_BUDGET_USD`), `.issues/045` (T7 launchctl activation) — verified 08-31.
- **Queue items 6/7** (boundary-guard + substrate-first drift audit): last full run 08-29,
  quarterly — item 6 re-run OFF-CADENCE 2026-09-04 early (see the idle-cycle-4 entry: the
  19→16 owner move needed a fresh contract measurement + the SP5 phase was found unable to
  complete on a loaded box); item 7 stays at 08-29, due ~11-29.
- **2026-09-03 late (idle cycle 2, M3):** P18 verification sweep DONE (see the P18 row —
  all rust.yml full-guard on default branch; token missing on train/mmorpg/seal-remake,
  owner act); Issue 062 closed-negative + removed (`3e12635`, AGENTS.md record); plan 069
  landing-cleanup box verified done and closed; Protocol B sweep of **riir-dao** = ZERO
  residue (healer 0 proposable edits over 21 files / 118 spans; real `cargo clippy
  --workspace --all-targets` 0 warnings — the repo is 6 days old and written under its own
  `-D warnings` gate; the flashquant `.plans/070` dual-allocation self-resolved — file gone,
  highwater 72). Active siblings probed before the unit: riir-neuron-db UNCOMMITTED WIP
  (avoid), riir-chain last touched 21:52 same evening (defer its re-sweep to a quieter
  cycle — 3 weeks of post-campaign code incl. the closed 031/032 territory is still owed).
- **2026-09-03 night (idle cycle 3, M3 — four units, all pushed):**
  (1) **riir-train Issue 507 LANDED `e0716476`** — the repo's rust.yml compiled 15,147
  `#[test]` sites and executed none; new parallel `tests` job runs
  `scripts/test_gate.sh` (engine `--lib` default features, per-target FLOORS firing
  downward only — floor 1487 = the committed-HEAD measurement, strict single-result-line
  parse, `--test-threads=2` for the bench_transfer_overhead load-flake, `--canary`
  proven both directions: normal PASS 1498, canary FAIL via the floor branch). NOT
  covered, named: riir-train-gpu (1,669 GPU-bound), integration targets (277/487
  required-features), examples/benches. Guard stays compile-only by design. Issue file
  removed (noise-reduction); record in riir-train AGENTS.md + the workflow/script
  headers. NOTE: the tests job inherits the guard's SIBLING_REPOS_TOKEN loud-fail until
  the owner provisions it (P18).
  (2) **riir-deployer first-ever heal sweep `7a091db`** — 24 edits / 6 files (use_self
  x12, match_bool x5, + uninlined/vec-capacity/map_unwrap_or/single_match_else/
  shrink-to-fit); clippy 0 + 178/0 tests. (3) **riir-game-sdk heal sweep `fff904f`** —
  18 edits / 5 files; 3 targets inner-`#![cfg]`-gated → feature-aware validation (prod_e2e
  + game_e2e compile+run); sibling WIP arrived MID-UNIT (Cargo.lock + orchard demo dirty
  at commit time) → named-path staging held, theirs excluded. (4) **riir-chain re-sweep:
  the "still owed" note above is STALE — measured 0 proposable edits / 459 files** (a
  sibling discharged it in-window; dockerfile lines are the ledger-pinned declines).
  riir-auth scanned clean (0/17); riir-viewbridge PARKED (Proposal 001 D1/D3 — skip
  sweeps of parked repos, the CI gate is the rot check). Contended and skipped: katgpt-rs
  (717-719 agent), riir-ai (854-858 agent), mmorpg (092/094 agents), neuron-db (dirty),
  seal-remake (fps agent), 4090 (NoRA + Plan-371 + B102). kernel_opt mining stays gated
  behind sibling B102; B101 x86 re-gate armed on the 4090 window.
- **2026-09-04 early (idle cycle 4, M3 — boundary-contract unit + burner top-up):**
  (1) **Queue item 6 OFF-CADENCE re-run, and the gate could not previously complete on this
  box:** the contract script's SP5 phase walks 8 repos' manifests with
  `find -not -path '*/target/*'` — which FILTERS but still DESCENDS into every target tree
  (measured standalone: katgpt-rs alone 3m42s under build load, riir-train 43s) — two runs
  exceeded a 10-min timeout while healthy and read as hangs (the 862 collision note's
  line-621 death was a sibling session hitting this file mid-edit; crossfire documented both
  sides). Fixed in riir-ai: prune form + a **zero-scan hard guard** (a scan returning zero
  Cargo.toml for a Rust repo = hard error, never the vacuous green — the over-escaped
  predicate intermediate demonstrated that shape LIVE: 8 find syntax errors, zero rows, both
  SP5 gates ok). Canary: zero-manifest contract repo fires hard exit 2; full workspace run
  green in 35 s. **Edge count RE-MEASURED: 224 edges / 16 repos, contract clean exit 0** —
  the 862 issue's "currently unmeasured" record closed in its addendum. Landed riir-ai
  `3ae41bc83` via the detached-worktree push dance (main checkout carried sibling WIP + was
  diverged; patch-id dedup will collapse the local twin `18180d322` on the next rebase).
  (2) **riir-burner top-up `8c2ab9a`** — 2 semicolon_if_nothing_returned in pipeline.rs
  (missed by the 23:43 sweep; compile-gated, 2 Certified, clippy 0, 19 tests) — landed
  minutes before the owner's 19→16 move moved the repo to `obsolete/` (work preserved; the
  AGENTS.md lineage note now names it). (3) viewbridge re-dry-run post-`143aceb` = 0 edits
  (fully clean); riir-dapps confirmed already discharged by its finder (`d91881f` manual +
  `2b9cce9` post-fix re-heal). Contended/skipped: game-sdk (sibling WIP dirty), all other
  active-lane repos per the sibling-lanes note.
- **Batch 103 landed this cycle (2026-09-04):** see Last landed above; Plan 072 all tasks `[x]`;
  Plan 073 stays ARMED on the 4090 window (T0a probe blocked by the NoRA matrix until ~10:00-12:30).
- **2026-09-04 (this session, 4090 box — Plan 073 T0a EXECUTED, batch RESOLVED annotation-only):** the window opened (NoRA Issue-504 matrix ALL_DONE + closed; GPU compute-quiet — session-0 services + desktop GUI only per the warn-don't-block policy; the sibling had already yielded the lane). Pre-flight 3/3 → ran the staged `probe_873.exe` 3× (verdict-stable ×3) → **the H100 knee is ABSENT on Ada**: sparse grid saturates at K=2 (951.6 GB/s median, 94.4%) and stays flat through K=16 (−0.3%); saturating grid flat/noise (±8% within-cell, non-discriminating). Pooled medians + full record in Research 121 §2608.15764 RESULT; plan 073 all tasks closed; B92 Ada rider landed (corpus prose only, count 466 unchanged); T2 MOOT. Instrument + CSVs committed at `scripts/ldg_probe/` (the 848 lesson: compiled AND run before committing). One run-recipe lesson: git-bash needs `./probe_873.exe` (CWD is not in PATH); idle SM clocks park at ~270 MHz and boost to 2805 under the probe. Next on the 2608.15764 lane: nothing — the batch is closed ( reopen only on an H100-class part entering the fleet).
- **2026-09-04 ~04:30 (idle cycle 5, M3 — T0a harness PRE-AUTHORED, zero-contention unit):** the 4090
  was observed 0% util / 8.1 GB resident at 04:17 — the NoRA matrix is sequenced BEHIND the M3 509-T4
  run and had NOT started — but the window was NOT taken (the sibling's post-T4 pull/build/launch on
  that box is minutes-after-T4 and is itself probe-poisoning contention; the prior cycle already
  yielded the 504/509 lane). Instead the T0a authoring moved OUT of the window: `probe.cu` (nvcc sm_89
  standalone CUDA C; K∈{1,2,4,8,16} template arms; sparse 64×256 + saturating 512×256 grid regimes
  isolating depth from concurrency; fixed 4096 loads/thread; allocation-sweep; 3 interleaved reps; K
  independent 2-fadd consume chains so the 16B load cannot narrow) + run recipe + 3-condition pre-flight
  live at **`.raw/ldg_probe/`** (gitignored scratch; nvcc v13.3 confirmed on the box; design self-review
  caught the warmup nullptr + serial-consume-chain bugs BEFORE any run). Instrument commits only AFTER
  compile+run (the 848 lesson). Full details in `.plans/073` T0a. Everything else re-verified still gated
  at 04:14 (T4 running PID 48700; walk #7 not due; MCP quota; 066-T4 LANDED by idle cycle / P18 owner; mmorpg-092 siblings).
- **2026-09-04 ~04:50 (idle cycle 6, M3 — T0a harness COMPILE-VERIFIED on the box, zero-GPU-touch de-risk unit):** the window was still closed at cycle start (NO `t504_confirm/status.txt`; 4090 GPU 31% / 8.1 GB resident with `bonsai_lora_accuracy_parity_arm_c.exe` — the sibling 504 lane is LIVE on the box), so the compile gate moved out of the window too: scp → nvcc → GREEN, zero GPU state touched. **Three pre-window defects caught + fixed** (the de-risk paying for itself): (1) the warmup nullptr guard was INCOMPLETE (deref guarded, the cudaEventElapsedTime CALL not — InvalidValue → exit(1) during warmup, before any measurement); (2) CUDA 13 REMOVED `cudaDeviceProp::clockRate`/`memoryClockRate` (compile error on the box) — replaced with `cudaDeviceGetAttribute(cudaDevAttrClockRate/MemoryClockRate)`; (3) box recipe: non-interactive ssh PATH lacks cl.exe — prepend the MSVC 14.41 Hostx64/x64 bin. **ptxas: all 5 arms 40–44 regs, 0 spills — K=16 at 41 regs REFUTES the anticipated "spill at high K invalidates the arm" caveat; the depth axis is clean through K=16.** Compiled+verified `probe_873.exe` STAGED at `C:\Users\katop\ldg_probe_873\` — the window-open recipe is now: pre-flight → run → scp CSV back. `.raw/ldg_probe/README.md` + `.plans/073` T0a updated in-plan. Instrument still commits only AFTER compile+run (the 848 lesson). **Walk #7 correction: it ALREADY LANDED while this cycle ran (`4c9ba72`, sibling session) — strict window EMPTY, next announce Sun 09-06 20ET — so the inherited summary's "walk #7 due ~07:00 today" is superseded; no walk is pending until Sunday.** Everything else re-verified still gated (MCP quota; 066-T4/P18 owner; mmorpg-092 siblings).
- **Hygiene:** current after this entry. `.issues/` = 059/060/061 (owner/ops/legal-gated)
  + 065 (T3/T7 latent; T6 CLOSED on the 09-03 nightly PASS); **066 fully resolved + file
  REMOVED 09-04 (`1a1db877` — T4 had landed `4239dbdd`; durable record = the AGENTS.md
  paragraph beside Issue 050)**; plans tree 0 unchecked (069 closed 2026-09-03).
- **065 T6 verdict (2026-09-04 cycle):** the fix `319319a` passed the scheduled 18:23 UTC
  run end-to-end — regen OK 10.5 min, per-repo duration lines, the budget-exhaustion branch
  fired live (riir-clippy kept cached) with coverage=10 through the merge path, deploy
  verified, `=== done ===`. Checkbox flipped in `.issues/065`; first fresh dashboard
  artifact since 09-01 03:44. The T6 pre-run also surfaced 066 (docker corpus needle).
- **066 resolved same cycle:** T1+T2 (`c9863d6` — needle `    rule(` in both walkers +
  `corpus_stats_needle_tests` pinning walk==runtime for every compiled domain) and T3
  (`d731cac` — the four first-hand poison rows repaired via the one-shot dated upsert
  `examples/repair_066_corpus_store.rs`, git-verified values, provenance `repair-066`).
  Store = projection = mined now agree (27/31/31/31/31/31); the nightly regeneration diff
  that motivated `a7c473c`'s "do not commit" note is structurally gone. ~~Remaining: T4
  (log projection-vs-mined divergence instead of silently losing — owner call).~~
  **T4 LANDED same day** by the idle cycle (log-not-fail divergence lines + the mined-layer
  CWD anchor it exposed — `_count_in_rev` inherited the caller CWD and read zeros off-root;
  both subprocess calls now `cwd=REPO_ROOT`); the 3 residual drift lines root-caused benign
  (commit-time vs stamp-time definitional boundary), recorded in `.issues/066` T4.
  Corpus-count note: kernel_opt moved 453→459 in the sibling's Batch 102 (landed mid-cycle,
  `17f00e7`) — today's stamp carries it; no action needed here.
- **2026-09-04 ~07:10 (idle cycle 9, M3 — hygiene unit, zero-contention during the sibling
MLX 32K cell):** Issue 066 fully resolved + file REMOVED per noise-reduction (`1a1db877`;
T1–T4 were all already landed `c9863d6`/`d731cac`/`4239dbdd` — the closeout is the AGENTS.md
record + `git rm`). Plans 002/051 re-verified: 0 REAL unchecked boxes (the plan-002 grep
hits are prose; 051's last box was verified done 08-31). Everything else re-verified still
gated: T0a (NO `t504_confirm/` status file; 4090 busy 21% / 8.1 GB — the riir-chain verify
sibling holds the box), walk #8 → Sun 09-06, MCP pool → 09-15, Batch 104 = sibling session,
owner gates unchanged (P18, 059/060/061/065, katgpt-rs 718/719/721, riir-ai lanes).
Sweep surfaces all confirmed closed per the 09-03/09-04 entries (dao ZERO, deployer
`7a091db`, auth 0/17, chain 0/459, game-sdk `fff904f` — now sibling-dirty again, dapps
discharged, seal-remake zero-findings 09-04, viewbridge PARKED, mmorpg sibling lane).
Queues empty after this entry → idle report.
- **2026-09-05 (M3, idle cycle 11 — Batch 104 LANDED):** the 09-02-queued fa-vec league
    opponent-watch lead CONSUMED — see Last landed above (Research 126 / Plan 074, corpus
    464 → 466, K 5 → 7; `ecade10` + the plan-close `6f26898`). Everything else re-verified
    still gated: T0a (NO `t504_confirm/status.txt`; 4090 at 10% / 8.1 GB — near-quiet but the
    gate needs ALL_DONE, and the sibling 504-harvest lane is still running its analysis per the
    session context), walk #8 → Mon 09-07, MCP pool → 09-15, owner gates unchanged (P18,
    059/060/061 — 065 RESOLVED + removed by the sibling `0ba69cf` with T7 landed `1117f0a`).
    Upstream moved +3 mid-unit (the dispatcher mask-fuse rider `8bbd59e` + Plan-072 status
    `577b30d` + Issue-067 docs `0fb2a97`); rebased clean, counts re-verified post-rebase
    (the rider edits an existing rule's concept text — no corpus delta). Queues: the mining
    queue is again leadless for kernel_opt external sources (the oMLX pool closed at B94;
    ds4/PrismML/MTPLX/NAX/llama-fa-vec all mined; the arxiv walk is the remaining intake
    channel until 09-15) — next units are T0a when unblocked, walk #8 Monday, and the
    Protocol-B re-sweep when the dirty siblings settle.

## Standing rules (pointers — full text in the skills + AGENTS.md)

- Plan-first per batch (Protocol A step 0): `.plans/NNN` via `.plans/.highwater`,
  re-scan the folder at WRITE time (dual-allocation discipline).
- Per batch, ONE commit: research note (with a `**Status:**` line) + AGENTS.md batch
  entry + corpus-count bump + this snapshot's one-line queue update.
- Generator lessons (edit_file escape-stripping, `ensure_ascii=False`, line-initial
  `+ ` doc-markdown, CRLF check before fmt, count-pin walks) are recorded per-batch in
  AGENTS.md entries — read the last 2–3 batch entries before writing corpus content.
- kernel_opt file-mining corpus is COMPLETE (B51 declared); open intake channels:
  paper distills (the B70+ arxiv channel), measured-GOAT distillations (the B49
  reopen path (c)), online search (quota-gated), arxiv listing walks.
- Instrument note: `scripts/self_evolve_audit.sh` §3/E5 parses `- [x] P\d` items in
  the intake section below — keep that exact item format when updating.

## Post-mining intake (compact — full records in `.research/108_mining_queue_snapshot_history.md`)

- [x] **2026-09-09 DOMAIN IDEA (owner call) — SUPERSEDED SAME DAY: the owner's pushback ("we deliver best software by heal/fix the perf/sec/mem/storage — you not notice 2x until i ask") corrected the verdict; the domain SHIPPED as `dist` (Issue 085, this session).** The parked NO-GO's oracle blocker was wrong by the repo's own precedent (`rust_perf` fixes verify by `cargo test` re-run, never by benchmarking each fix — the gain is proven ONCE at class level, and the -61% measurement IS that proof). Phase 1 landed suggest-only with 4 rules; see `.issues/085_dist_healing_domain.md` + `.docs/11_domains/dist.md`. Name: `dist` (as recommended here; `optimize` would have collided with rust_perf). The guard-vs-domain division of labor: `binary_size_gate.sh` = THIS repo's release lane at package time; the domain = every scanned manifest at dev time.
- [ ] **2026-09-09 DISTILL VERDICT — NVIDIA "CUDA Rust: two tracks" (blog 2026-09-08) + NVlabs/cuda-oxide + NVlabs/cutile-rs: NO for corpus mining (C-), lane intel recorded.** Sources are kernel TOOLCHAINS, not kernel-pattern libraries: cuda-oxide = a rustc codegen backend (Rust MIR → Pliron → LLVM → PTX, SIMT track, early alpha, Linux + pinned nightly + LLVM 21+); cutile-rs = Tile-DSL over CUDA Tile IR JIT (stable Rust, published, real consumers: HF Grout + mistral.rs; paper claims safe-Rust GEMM within 0.3% of low-level Tile IR on B200). Dedup gate KILLED every candidate under both vocabularies: cutile lazy-record + single-sync ≈ `chain-dispatches-single-sync-point`/`kernelize-host-chain-after-sync-batching-floor`/`batched-downloads-single-sync-cycle`; cuda-oxide checked-launch-contract/prepare = the naive shape of `launch-unchecked-caller-owned-size-contract`; compile-time size-specialized kernel policies ≈ `hardcoded-fast-path-jit-specialization`/`entrypoint-variants-share-layer-body`/`share-cube-size-const-between-kernel-and-launcher`; Tile graph replay ≈ the 6-rule captured-graph family (`captured-graph-bakes-buffer-addresses` etc.); DisjointSlice/partition exclusivity = safety model, zero perf decision content (KILL); `gemm_sol_final` Blackwell classes (CLC, tcgen05, cta_group::2) = not-our-silicon (sm_89 4090, sm_90+/sm_100-gated) AND unbenchable here — both toolchains are Linux-only, so no GOAT candidate passes the benchable axis on the M3 (no NVIDIA) or the Windows 4090. **LANE INTEL (the value): (a) Grout (HF's Qwen3 inference engine on cutile) is a NEW Rust opponent FAMILY — recommend a watch-only line in riir-ai `.issues/846` (opponent watch): 171 tok/s Qwen3-4B on a 5090 / 82 tok/s Qwen3-32B on B200 are DIFFERENT silicon + different model — never scoring cells, no re-pin trigger, absolute numbers are not headroom evidence per the standing caveat; it becomes a league opponent only if someone pins same-box same-model cells. (b) NVIDIA officially backing Rust→PTX + Tile IR is a long-term riir-gpu substrate SIGNAL (CubeCL + cudarc twins unaffected today; no action). (c) Not cloned — surface verdict from blog + READMEs (2026-09-09); re-clone at pin if ever promoted.** Ecosystem side-finds during the 4090 onboarding this verdict rode: gist-rs/cargo-heal `install.ps1` probes the stale `x86_64-pc-windows-msvc` asset name (v0.1.2+ ships gnu → fresh installs 404; manual gnu-zip install used), and release binaries v0.1.2/v0.1.3 default `RIIR_HEAL_KAT_SERVICE_URL` to `heal.gist.rs` which has NO public DNS record (source HEAD defaults `https://ai.gist.rs` at `kat_lease_client.rs:1061`; user-env override unblocked the 4090) — owner call: DNS record or cut v0.1.4 from HEAD + installer asset-name fix.

- [x] **2026-09-09 PURITY-RECALL GAP in the shared `is_pure_expr` helper — `unnecessary_lazy_evaluations` silently declined a span real clippy flagged (FOUND LIVE: the riir-dapps issue-035 heal — `cargo heal --fix --write --verify src/kat/ledger.rs` scanned 10 spans, applied 0 edits, while `cargo clippy` flagged `ok_or_else(|| DappError::LedgerRefused { .. })` at the payout leg; the struct-literal closure body fell through `is_pure_expr`'s `Path|Lit|Paren|Unary|Binary` arm set to `_ => false`).** Probed real clippy 2026-09-09 (temp crate, 8 shapes): FLAGGED = struct literal with pure fields, multi-line struct literal (the incident shape), field access `e.a`, `Err(1u8)`, `Some(1)`; NOT flagged = struct literal with an impure field, index `a[0]`, arbitrary call. Extension bounded to exactly that set: `Expr::Struct` (all fields pure + struct-update base pure), `Expr::Field` (base pure), `Expr::Call` (callee path's last segment `Some|Ok|Err` ONLY + args pure) — indexing/macros/arbitrary calls stay impure, matching clippy. Shared-helper blast radius checked: 3 consumers (`unnecessary_lazy_evaluations` closure bodies, `manual_unwrap_or` match-arm bodies, `manual_slice_fill` fill values) share the no-side-effects contract; all recall up, no contract change. 4 regression tests incl. the exact multi-line dapps shape (splice carries body text verbatim, re-parses) + impure-field/arbitrary-call negatives; lib 1191/0 at default AND clippy_lints; clippy 0; e2e `--fix` dry-run on a repro file carrying the original span now proposes `[unnecessary_lazy_evaluations]` where the pre-fix binary proposed nothing; score-bench #89 FLAT (heal 96.2 / created 1.9 held, 10th consecutive). Commit: this one.

Zero open classes (P12 CLOSED 2026-09-07 below); the standing divergence (P4 residual) has no action planned.

- [ ] **2026-09-09 CLIPPY-NURSERY LINT GAP — `needless_pass_by_ref_mut` (~80 findings in ONE riir-ai sweep) is absent from the clippy_lints corpus.** Surfaced by the riir-ai Windows-lane repair sweep (0e80cf26a, `--workspace --all-targets`): ~80 findings in one pass (test-helper launchers `&mut self -> &self` + caller-side `unused_mut` cascade), and the span-retrieval healer surfaced NOTHING for it — the sweep fell through to `cargo clippy --fix`. Corpus-verified absent 2026-09-09 (grep: zero rule entries in `src/domains/clippy_lints/`; the only repo hit is this codebase's own `#[allow]` at `self_evolve.rs:691` — we hit the lint ourselves). Intake shape: a bounded fix class (`&mut self` → `&self` where the body never mutates, PLUS the caller-side `let mut x` → `let x` cascade the change induces — the cascade is the part `cargo clippy --fix` needed 4 passes for, so the rule must emit BOTH halves or the fix does not verify). **Classification CORRECTED same day (was: "rustc-native, warn-by-default in 1.98" — measured FALSE, see riir-ai HISTORY.md 97f32e6ec): the lint is `clippy::needless-pass-by-ref-mut`, NURSERY group, default `allow` on clippy 0.1.98 (both boxes; probe: fires only under explicit `-W`, zero hits flagless; absent from rustc 1.98.1's lint registry). Domain routing therefore RESOLVES to clippy_lints (never rustc_errors). Honest priority caveat: default-allow means the lint NEVER appears in a default-flags clippy stream — a corpus rule for it only fires for users who enable nursery or the lint explicitly, so weigh its corpus slot accordingly (the 49-lint corpus already carries nursery members, so it is in-policy, just low-stream).** Verification oracle: `clippy_verify` with the lint explicitly `-W`-enabled (default-flags verification would compile the rule to a no-op — the healer's own cfg-gate lesson, lint-flavored).

- [ ] **2026-09-08 OPPONENT WATCH (Issue 846 loop, 4090 pass) — the PTQ1_0 re-pin precondition LANDED + three pattern leads.** PrismML `9294043b2` "native PTQ1_0 MMQ tile loader and packed dequantize" (#160, 09-08, self-reported CUDA-unverified by the committer; gates passed on H100 at the pre-#152 base): PTQ1_0 large-batch was falling back to dequant→cuBLAS at **2.26× slower than PQ2_0 @ n=512** while "its decode already led" — now it takes the same quantized-GEMM path as the other low-bit types (`mmq-load-tiles.cuh` +100 lines, MMQ type dispatch, DECL_MMQ_CASE, packed coalesced dequantize for the conversion sites). **League effect: NO scoring delta** (the scored cells pin the PQ2_0-class artifact @ `e311ed38f`; quant-class rule) — but the 09-07 row's recorded re-pin trigger now has its PATH half in place; it fires when the fork ships a PTQ1_0-class league artifact (1.75 bpw feed vs PQ2_0's ~2.1 — and our writer/loader then needs a PTQ1_0 arm or the bout is format-crossed non-scoring). **Distill lead (kernel_opt, the mechanism): `ggml_cuda_mmq_decode_ptq1_0_qs4` expands packed 2-bit ternary codes via ITERATED `v*3` + `& 0x00FF00FF` byte-mask (5 rounds/word-pair, `__byte_perm(0x4140/0x4342)` gather, `__vsub4` −1 bias) — a MULTIPLY-based digit expansion vs our B882 9-op PRMT table decode (IMAD-pipe vs PRMT/LOP3 mix); honest cap: B897 measured the whole reachable decode saving at ~1–1.5% kernel on the v10t lane — corpus class, NEVER a lane reopen.** Full-source reference: the fork vendor checkout at sha `9294043b2` (`mmq-load-tiles.cuh` is 100 lines, read in-place). Pattern leads (low): vLLM `9c297e3` PDL (programmatic dependent launch) on fusedQKNormRope — launch-gap overlap WITHOUT smem staging; honest note: our 449-launch GEMV family already co-runs across kernel boundaries (B848's measured mechanism), so PDL's win there is the already-banked overlap — mine as a general launch-gap rule class, never a C-lane reopen. llama.cpp `73ab759` branchless Q4_K/Q5_K unpack for mmvq (decode-GEMV branch-elimination class; dedup grep vs existing unpack rules at batch time). MLX `5778a97` NAX long-unmasked D72/D80 → the Issue 844 M3 watch (recorded there, no intake). vLLM SM12x-only items (FlashInfer GDN prefill `f6326f5`, FP8 CTA raster swizzle `4df8018`, H20 MoE low-batch `12579c7`) = not-our-silicon pattern notes. No scoring delta on any pinned row; pins stand.

- [ ] **2026-09-07 P14 ADDENDUM (Bench 885 / Issue 884 T5, riir-ai 512b74939) — the league gate FLIPPED TO PASS (0.845×, first ever for bonsai-4090-prefill-pp2048) and two TOOLING classes landed en route:** **(j) ATTESTATION-WRITER KILLED AFTER MEASUREMENT (shell class):** in `perf_rematch.sh`, `names="$(grep … | head | sed | paste -sd ';' -)"` — with the grep matching NOTHING, grep exits 1, and under `set -o pipefail` the ASSIGNMENT fails → `set -e` killed the script AFTER both bench arms had run and BEFORE the result JSON was written: five measured cells lost in a row with `runs:` printed and no `result ->`. Distinct from the R296/trap-sentinel family (that class launders FAILURES; this one DISCARDS SUCCESSFUL MEASUREMENTS at the write step). Rule candidate: any `var="$(pipeline)"` whose pipeline can end in a legitimately-empty grep needs `|| true` on the assignment when the script is under `set -euo pipefail`; audit class = result-writers downstream of measured work. **(k) TOOLCHAIN-OVERRIDE DECAY (build class):** a manifest/case-hardcoded `RUSTUP_TOOLCHAIN=1.95.0` survived a workspace pin bump (1.98.1, 09-04) and compiled a sibling repo (katgpt-rs, which carries NO rust-toolchain.toml) at the box default 1.93.0 — E0658 on fresh `isolate_lowest_one` code; the bout dies pre-measurement. Rule candidates: (i) env overrides recorded in manifests must be re-validated against the repo pin at emit time (`rust-toolchain.toml` is the authority; a pinned repo + an override env = assert they agree); (ii) a repo with version-sensitive code and NO pin file is itself a drift hazard (upstream note for the katgpt-rs owner). Context: the T5 bout (ours 2805 vs 3317.89) rode the q8+v10t route — see the rung-6 entry above for the kernel classes; league record riir-ai `.benchmarks/885_issue884_t5_league_repin.md`.**
- [ ] **2026-09-07 OPPONENT WATCH (Issue 846 loop, post-T5 cycle) — prismml fork is 15 commits past our pinned opponent commit e311ed38f; CUDA-side movement is MINOR for our row (the pin stands): `6f1c6908d` cuda FWHT kernels for block widths >2048 (decode-side Hadamard, not prefill GEMM) + `01fd9521c` NEW FORMAT PTQ1_0 (ternary group-128, 1.75 bpw, "lossless vs PQ2_0") — a QUANT-CLASS change: when the fork lands a PTQ1_0 prefill path and the league model migrates, that is a recorded re-pin trigger (never compare across quant classes). The other 13 are METAL-side decode (+4–38% claims) — M3-lane re-pin triggers, not 4090 prefill. vLLM 17 perf commits (incl. DSV3 GEMM strided-tensor class `a56654d` — 12–81% claims; distill lead for the MMQ strided-B question, low priority). No opponent jump invalidates today's cells: our bout ran against the pinned e311ed38f build, and the 09-07 window read (3317.89) already includes zero post-pin CUDA prefill movement. Next watch cycle re-triages.**

- [ ] **2026-09-07 P14 UPDATE — rung 6 MEASURED: the L2-traffic repairs are a GOAT — T4's 1.8× gate CLEARED (riir-ai Bench 884 / Plan 574, 4090 prefill mma, commit 3a2167c7e).** First ncu data for the family (Nsight Compute 2026.2.0, elevated counter access — ERR_NVGPUCTRPERM without elevation, Windows): the slab was NEVER mma-pipe-bound — it was **L2-BANDWIDTH-bound on two kernel-side amplifications**: (1) the SHARED epilogue (v6→v9 all inherit v8's schedule) mapped consecutive lanes to consecutive TOKENS (`tk = idx % TOKS; r = idx / TOKS` then `out[tok*m + row0 + r]`) so every warp store hit 32 different out rows 69,632 B apart = **8.0× write amplification (35.65M vs 4.46M minimal write sectors, byte-identical across v6tb/v9/v9t — a shared-epilogue bug, not a variant cost)**; (2) scattered per-warp scale loads (`group_scale[row*groups + grp]`, 160-B row stride, quad-broadcast, ~128 distinct sectors/group/block). FIX (v10/v10t = v9 body + exactly two deltas): double-buffered 128-float scale stages via a new 4-B `cp.async.ca` helper (once per group per BLOCK, not per warp) + a transposed `[TOKS][33]` epilogue stage (33 ≡ 1 mod 32 keeps writers conflict-free; readers `r = idx & 31; tk = idx >> 5` → 128-B contiguous stores) — same ops, same order, bit-identical by construction. MEASURED (interleaved ×9, ffn_gate m=17408 n=5120 p=2048, bitdiffs 0, all prior arms reproduce B883 within ~1%): **v10 1.835× / v10t 1.881× vs v4-q8 — 1.533× over the v9t route winner** (1.603→1.056 ms, 34.1%→52.3% of int8 peak); e2e 30,721-row NLL/argmax IDENTICAL, wall 74.95→63.17 s (−15.7%); cumulative vs the 2× hi/lo anchor ≈ **2.530×** (was 1.667×) — **T4's ≥1.8× kernel gate CLEARED, T5 league re-pin UNBLOCKED**. ncu witness: write sectors 35.65M→4,456,461 (exactly minimal), `lts__t_bytes` 3.84→2.84 GB, L2 SOL 74.7%→59.0% (no longer top), tensor 34.2%→55.1% + ALU 29.6%→48.5% now the walls, IPC 1.13→1.82. NEW kernel_opt corpus classes: **(g) POSITIVE — strided-epilogue repair: an out-store loop whose lane→column map is transposed relative to the output layout costs 8× L2 write sectors (32 sectors per 128 B) on a [T][N]-major tensor; the transposed-smem-stage + `r = idx & 31` reader map is the general fix (bank-shift the stage rows by +1 mod 32 for writer conflict-freedom)** — check EVERY mma-family epilogue, the bug was inherited silently across 6 kernel generations; **(h) POSITIVE — per-block scalar staging: a scalar slab (scales) read per-warp from gmem at ≥4-B stride should be cp.async'd once per block per step into a double-buffered smem stage (4-B `.ca` chunks; `.cg` is 16-B-only); honest caveat: the L2-traffic share was SMALL (L1 absorbed the cross-warp reuse — reads unchanged 84.4M sectors) — the measured win is issue-pressure relief (IPC 1.13→1.82), so mine this as an ISSUE-COUNT/IPC rule, not a bandwidth rule**; **(i) METHODOLOGY — profile-before-pipeline: the family spent 6 rungs (B879→B883) optimizing instruction mix while the top SOL was L2 at 76–90% with a fixable 8× write amplification; a one-section ncu pass (SOL + write sectors) would have routed straight to Plan 574. Gate rule candidate: before proposing a compute-side kernel rung, record the SOL table + top-stall pair.** OPEN (revised by data): decode-ALU 48.5% (the 9-op PRMT chain — ALU-lean decode variants; ldmatrix still cannot serve 2-bit codes, class (f)), tensor 55.1%, occupancy 33% (reg-capped, the v5 lesson — do not re-open). CLOSED-by-data: 3-stage cp.async depth (v10's gain came from traffic, not depth — latency was issue pressure) + warp-tile growth. Full data: riir-ai `.benchmarks/884_issue884_t4_l2_traffic_rung.md`; code: `gemm_ternary_i8_mma_v6_src.rs` (GEMM_BODY_V10_Q8 + v6_cp_async4).**

- [ ] **2026-09-07 P14 UPDATE — rung 5 MEASURED: the staged-packed cp.async pipeline is a GOAT (riir-ai Bench 883 / Plan 573, 4090 prefill mma).** B882's open lead (d) landed as `GEMM_BODY_V9_Q8` (v9/v9t): the v6tb staged structure over the packed-2-bit Q2_0 codes — A staged ONCE per block via cp.async into a double-buffered 128×12-word code stage (48-B rows: 16-B-aligned cp.async dsts + conflict-free quad-broadcast LDS at `12·g mod 32`), per-use `v8_decode_pair` fragment decode (ldmatrix CANNOT serve this granularity — the 4 lanes of a mma quad need different BYTES of the same code word, ldmatrix distributes whole words to lane columns). ONE barrier per group (v6tb: two — its SWAR A store waited out the reads). MEASURED (interleaved ×9, ffn_gate m=17408 n=5120 p=2048, bitdiffs 0, v4/v6tb reproduce B882 within 1%): **v9 1.222× / v9t 1.224× vs v4-q8 — 1.064× over the v6tb route winner** (1.706→1.603 ms, 32.4%→34.5% of int8 peak); e2e 30,721-row NLL/argmax lists IDENTICAL, chunk wall −15% (89.1→75.8 s, corroborating, sequential runs). cumulative vs the 2× hi/lo anchor ≈ **1.667×** (was 1.566×) — T4's 1.8× gate still open. NEW kernel_opt corpus classes: (e) POSITIVE — packed-code cp.async stage + per-use PRMT decode beats amortized SWAR staging when the packed format makes the operand LDGSTS-servable (the stage-store ALU + second barrier were the cost v9 removed); (f) ldmatrix granularity limit — byte-sub-word fragment consumption forces LDS-broadcast decode paths, a structural rule for 2-bit-code mma operands. OPEN: 3-stage cp.async depth (A 6,144 B/stage → 73.7 KB total, fits sm_89 100 KB opt-in) + warp-tile growth on the staged-packed route (B880's B-side 4× read amplification is now the residual wall). Full data: riir-ai `.benchmarks/883_issue884_t2b_staged_packed_rung.md`; code: `gemm_ternary_i8_mma_v6_src.rs` (GEMM_BODY_V9_Q8).**
- [ ] **2026-09-07 P14 NEW — kernel_opt mining leads from riir-ai Issue 884 T2b / Benches 879+880+881+882 (4090 prefill mma, measured).** The fork-config scheduling class (XOR-perm smizzle for BOTH operands, warp-tile growth to 2× mma/fragment-load, `cp.async.cg` B stage, 128-token tiles) was ported and gate-verified bit-identical vs the shipping v4-q8 kernel — measured only **1.112×** (28.0% → 31.1% of int8 dense peak at ffn_gate m=17408 n=5120 p=2048), NOT the opponent's 47–53% class. **B880 measured the ldmatrix rung the same session-day:** `ldmatrix.m8n8.x4` A + `.x2` B fragment loads over the SAME swizzle = **v6tb 1.156×** (the arm route; 32.7% of peak at the B882 re-run), **v6tl 1.135×** — POSITIVE class for sm_89 s8-mma pipelines; but still not the fork class. **B881 closed the fork-structure question on our bitplane format (v7/v7t NEGATIVE, 0.785–1.106×). B882 closed the FORMAT half (Plan 572): the packed-2-bit Q2_0-code rung — the fork's own `block_q2_0` code table via the riir-infer-core q2_0 bridge's inverse, ingest transform `pack_bitplanes_to_q2`, and a 9-op PRMT fragment decode (`prmt` table + masks + constant-selector merges, bit-identical incl. the (1,1) fold, PRMT-model test caught the a2 selector transpose pre-device) — e2e 30,721-row NLL/argmax IDENTICAL, kernel NEGATIVE (v8 0.989× / v8t 0.912×): v7→v8 = 1.26× from the format alone CONFIRMS the decode-ALU model, and the residual ~18% gap to staged v6tb isolates the GLOBAL-A LOAD STRUCTURE (8 LDG/lane/k-step + per-token-warp re-reads) as the binding wall — TWO formats at the same structure lose the same way, so the fork's A-path advantage = format (matched) + a load/schedule class not yet ported. MINING LEADS: (a) PRMT-table packed-code decode (the table IS a 4-entry byte LUT in one register — constant-selector merges as byte shuffles; new corpus class, zero dedup hits vs the B20 two-level byte_perm crumb); (b) NEGATIVE ×2 for global-A fragment loads on sm_89 s8-mma at 16 warps (both formats) — do not propose per-fragment global-A staging rules for this tile class; (c) NEGATIVE: 256-entry smem staging LUT (B880), 2-blocks/SM, double-A, plain v6; (d) OPEN lead: staged-packed cp.async pipeline — packed codes shrink the A stage 4× (4 KB vs 16 KB per 128×128-k group) enabling deep multi-stage cp.async of BOTH operands within the default smem budget, trading ldmatrix-A for per-use register decode (B882 §why — the next T2b unit). Full data: riir-ai `.benchmarks/879_…` + `880_…` + `881_…` + `882_issue884_t2b_format_rung.md`; code: `gemm_ternary_i8_mma_v6_src.rs` (v7/v8 bodies) + `prefill_cuda_mma.rs` (the transform).**

- [ ] **2026-09-07 P13 NEW — cfg-arm `mut` dropped without compiling the consumer's feature combo (substrate incident, fix `riir-ai a2d28a386`).** A lint-heal sweep on `riir-games-mmorpg/authority_sim/mod.rs` (commit `bea9464bc`, "drop unused mut on the claim_system builder") removed a `mut` that is load-bearing under `#[cfg(feature = "pvp_karma")]` (the block re-assigns the binding). The validating combo ("seal's sextet") did not include pvp_karma, so the E0384 surfaced only in consumer combos that forward the substrate's pvp_karma (seal party/pet_pvp/target_coordination lanes) — and would also break riir-ai's own --all-features lane. Fix pattern: `let mut` + `#[cfg_attr(not(feature = "pvp_karma"), allow(unused_mut))]` (mirrors the file's own lineage_surface pattern). **Intake question for the healer:** should `--verify` refuse (or warn-loud) when the edit touches a binding re-assigned inside ANY `#[cfg]` block whose feature is not in the verify-args feature set? This is the compile-gate blind spot the AGENTS already documents for default-off code ("a default-features check compiles gated files empty — a green check proves nothing about them"), now with a concrete consumer-breaking instance. Fix is landed; this row is the improvement-intake record.

- [x] **2026-09-06 P11 NEW — gen-7 bench yields, three classes (Bench 074; all
  pre-existing, none gen-7-caused): LANDED 2026-09-06 (Plan 099 — authored 097,
  renumbered per the 848-class collision with the sibling's Batch-118 plan — /
  Bench 075;
  implementation `ef2129a`).** (1) the `needless_range_loop` rewrite
  produces NON-COMPILING output when a substituted slot sits in a comparison
  context (`if __elem > 2` E0308 — the fresh `&u32` binding vs an integer
  literal needs a deref; `n += __elem` compiles via `AddAssign<&u32>`) —
  FIXED by the universal `*__elem` deref splice (desugar-identical to
  `v[i]` in every rvalue context; parenthesized only at postfix bases;
  probe-measured: the bare binding fails THREE classes, not one —
  comparisons, typed lets, AND value-arg method receivers where
  `&u32: Ord` resolves on the reference). (2) the `__elem` collision guard
  makes NESTED needless_range_loop a hard persist (level 2 refuses after
  level 1 introduces the binding) — FIXED by fresh-binding uniquing
  (taken-set over the `__elem<digits>` family + template-capture safety;
  level 2 heals with `__elem2`). (3) the capacity-hint nested-block blind
  spot: `vec-with-capacity` and `hashmap-with-capacity` are SILENT when the
  collection sits inside an `if let`/match-arm block — FIXED by nested-block
  descent in both (shape A: block recursion; shape B: the later-scan/
  first-mention walk descends block-bearing stmts with per-level gates;
  probe correction: hashmap shape A was ALREADY firing — the intake's
  "both silent" was shape B; and vec's silence had a second stacked cause:
  bare-path iterables were never a vec capacity source). (4) context, not a
  fix: the fix cascade EXPOSES deeper non-vocabulary lints (`x.map(|v| v)`
  → map_identity + single_option_map; distance-sq's `r * r` →
  suboptimal_flops) — unchanged, still context. **Series movement: gen-7
  78.26% → 87.5% (deterministic ×2), chain 86.7% → 100%, parse_safety 100%,
  created unmoved 12.5%; v1 E4 holds 96.2/1.9; freeze re-landed 16/16
  verified 0 relabels** (roster moves: `gen7-ref-nrl-nrl` → `gen7-d2-nrl-nrl`
  chain; `gen7-d2-nrl-format` → `gen7-flat-nrl-format`, the created-chain
  subtype dissolved and retained as regression pin; NEW `gen7-ref-nrl-write`
  keeps the refused territory honest). Record: Bench 075.

- [x] **2026-09-06 P12 CLOSED 2026-09-07 (Plan 100 / Bench 077 — T0 FALSIFIED the "parse-only undecidable" premise for every class):** (1) `&mut`-method receivers — FIXED by the mutator header (`MUTATOR_METHODS` trigger + `param_iter_form`; owned-`mut` param emits `&mut v`; consuming methods excluded on purpose — E0507 in every header; custom mutators stay `--verify`-backstopped, the irreducible residual); (2) `assert_eq!` slots — FIXED by template-less family-macro descent (`all_expr_args` when the template position holds no literal; `assert_eq!(*__elem, 2)` compiles — the expansion `&(*__elem)` = `&u32`; message-less `assert!` conds ride the same path); (3) `let x = v[i]` transitive typed uses — ALREADY FIXED by the P11 universal splice (Copy-only input universe: non-Copy `let x = v[i]` is E0507 in the ORIGINAL, never a clippy input) — pinned, not changed. **NEW C4 class found by the T0 probe + fixed:** `&mut` params were folded into "shared" by the old resolver, so the bare iter MOVED the borrow and any non-slot body use (`v.len()`) emitted E0382 — now refused (exclusive-borrow guard: `collection_uses − exact.len()`). Bonus heal: the tracing named-target form now heals (formerly conservative-refused); a named capture hidden in any string literal of a template-less family macro refuses (new load-bearing guard — the old walk never looked inside literals). Series stability EXACTLY as pre-registered: gen-7 ×3 87.5%/12.5%/parse 100% deterministic (unchanged), v1 E4 96.2/1.9/100 holds; lib 1156/0 (floor held), lib+score_bench 1357/0/8i, integration trio 13/0/2i, clippy both states clean (incl. the pre-existing `generations.rs:823` doc-markdown straggler fixed en route). Record: Plan 100 / Bench 077.

- [x] **2026-09-04 P10 NEW — LTO fold-elimination kills timing gates whose work
  **LANDED 2026-09-05 (Batch 109, Plan 080, 4090 box): the rule + TWO gate-refuted draft fix-patterns** (XOR-sink GF(2) cancellation over 0..2^k; wrapping_add balanced-batch collapse to 2^64-2048) — the shipped pattern is CHAINED state + consumer black_box + the loud nonzero-baseline assert; both anti-shapes pinned as assertions in batch109_sink_consumes_per_iteration_data. Corpus 99 → 100, complexity 19, NotBenchable 14, annotated 43. Record: Research 132.
      has a dead result** (found by the katgpt-rs Issue 723 Class A2 fixes,
      commit `b4cef008`): rustc 1.98.1 + fat LTO eliminates inlined-callee
      work whose outer result is dead, EVEN THROUGH `black_box` INSIDE the
      callee — measured by a direct-call probe in one binary: used result
      16.6 µs vs `let _ = f()` ≈ 0. The rv gate measured 0 ns → NaN% for its
      entire release life. Fix patterns proven in-tree: (1) xorshift
      data-dependent work + caller-consumed sinks (`black_box` at the CONSUMER);
      (2) a bare CONSTANT work fn still folds through a sink (XOR of a
      constant 2M times = 0) — the sink must mix per-iteration data; (3)
      equal non-pruner work in both arms (one shared timed fn) so the ratio
      isolates the delta; (4) interleaved (baseline, candidate) chunks with a
      median-of-ratios for bar-adjacent gates — two sequential 2M arms read
      +5.2% and +21.7% thirty seconds apart, the paired median read −0.0%/
      +0.6% twice. Route: rust_perf measurement-honesty rule candidate
      (`dead-result-timing-loop-eliminated-under-lto` class) — the detector
      is the loud zero-baseline assert; the proposer would need CFG-level
      dataflow (suggest-only first). Evidence: katgpt-rs
      `tests/bench_gdsd_modelless.rs` header + `tests/rv_gated_routing.rs`
      sink docs.

- [x] **2026-09-06 NEW — ds4 Metal tensor-prefill pair (external kernel_opt leads;
      watch-coverage gap closed same day):** the opponent watch tracked only
      llama.cpp/PrismML/vLLM/MLX — ds4 (antirez; owner of the 250.1 t/s @11709 M3 Max
      prefill reference row, Batch 97's source repo) had ZERO coverage and landed two
      Metal tensor-prefill PERF commits on main: `2ea3839` "Reuse routed activations in
      Metal tensor prefill" (09-04) + `5806d09` "Batch Metal TP DSpark experts and reuse
      MXFP4 prefill activations" (09-05). Route: kernel_opt external distill, same
      repo as Batch 97 — local clone still at pin `110afdd`, ~10+ commits behind
      upstream main, so re-verify every quote at the NEW pin. Mechanism guesses to
      falsify at mine time: routed-activation reuse is plausibly the same-input family
      (B58 launch-merge / B63 weight-concat / B77 quantize-memo / B97 staged-tile
      sibling); MXFP4 prefill-activation reuse pairs with B97's half-domain dequant
      band + the B56 live-fields-only class. Also in-window: `b42682f` aligned Q8 CUDA
      decode scratch reuse (A-family candidate, CUDA-side), `0e9cc2d` GLM attention
      masking fix (quality class, likely no rule). Watch harness now covers ds4 + oMLX
      (riir-ai `perf_rematch.sh`, both auto-clone + path-filtered), so future movement
      arrives via the watch. oMLX checked at the same pass: origin/HEAD == `e467261` ==
      the Batch-114 mining pin — zero movement, pin current.


  **LANDED (Batch 117, 2026-09-06, Research 141 / Plan 090):** 5 rules + 8 riders, corpus 539 -> 544 (L/A/C/M/F +1 each; routed-activation-pack-once-shared-operand, device-affine-scratch-admission-with-pool-fallback, batch-verify-rows-single-dispatch, row-aware-indexing-reuses-static-single-token-kernel, dense-fast-path-bounded-by-selector-coverage). Falsification results: routed-activation reuse IS a new rule (tensor-pipeline operand materialization, distinct from the B77 memo + the launch-merge family); MXFP4 packed-prefill arm NOT-LANDED (enablement-only, source refuses attribution, non-uniform +9/+2.8/-2.3%); the GLM masking fix yielded 2 riders (B57 single-pass twins, B35 selector-padding) + 1 surprise rule (dense-fast-path-bounded-by-selector-coverage). Pin 9ab7053; .raw/ds4 removed at landing.

- [x] **2026-09-05 P9 NEW — clippy 1.98 new-lint corpus candidates** (found by the
      1.98.1 toolchain re-gate sweep across the workspace; measured populations in
      brackets): `chunks_exact_to_as_chunks` [katgpt-rs 12 + 4 chunks_exact_mut,
      ndb 2 — `as_chunks::<N>().0`, needs const-generic N; MachineApplicable],
      `manual_checked_ops` [riir-dao 17, katgpt-rs 7 — if-guard + else-arm rewrite,
      MaybeIncorrect for clippy (the 17-site dao run applied 0 via `clippy --fix`),
      our proposer CAN see the else arm and emit `checked_div().unwrap_or(v)`,
      guarded against sample-accounting guards where the check gates more than the
      division — the riir-dao `sim/mod.rs:843/852` allow class], `collapsible_match`
      [katgpt-rs 6], `sort_by_key` [katgpt-rs 4], `byte_char_slices` [ndb 6,
      katgpt-rs 2 — `*b"FRZE"`, trivially fixable], `manual_slice_fill`
      [ndb 2], `explicit_counter_loop` [katgpt-rs 1], `manual_isolate_lowest_one`
      [1], `needless_pattern_in_..`-class [1], `redundant_reference_format_args`
      [1]. P0 candidates by population + bounded fix-space: `manual_checked_ops`
      + `byte_char_slices` + `manual_slice_fill`; `chunks_exact_to_as_chunks`
      needs an as-chunks MSRV check (std 1.88). Populations measured under load,
      re-count at batch time.
      **LANDED same day (Batch 91, M3): all four P0 candidates +
      `chunks_exact_to_as_chunks` as corpus rules + proposers — corpus 45 → 49
      (2 FIXABLE: `byte_char_slices`, `manual_slice_fill`; 2 VERIFY:
      `manual_checked_ops` [signed-divisor refusal gate; the MIN/-1 corner +
      custom-Div E0599 class rides the compile gate], `chunks_exact_to_as_chunks`
      [`&[u8]`→`&[u8; N]` body class + non-const Path size; MSRV floor std 1.88
      documented in the module doc — real clippy is MSRV-aware so the suggest
      path is inherently safe, the AST `--fix` path has no manifest access
      (fxhashmap precedent)]). Probe first (rustc 1.98.1 probe crate):
      `manual_checked_ops` fires ONLY on unsigned (i32/i64/isize silent), on
      `==`/zero-on-left/`!=`/`> 0` forms, NOT on `%`, span = the CONDITION;
      `manual_slice_fill` fires ONLY on the `&mut place` form (`iter_mut()`
      silent), span = the whole for-loop; all four lints are suggestion-only
      (no structured fix) — the proposers are the value. Gates: default lib
      1076/0, latent 1182/0/1i, clippy_verify 1090/0/6i, all-features 1883/0/16i,
      clippy `--all-targets` 0, retrieval floors HELD, suggest-retrievability
      PASS, score-bench #37 FLAT (96.2/1.9/100/100), real-clippy truthfulness
      4/4 (each corpus snippet fires its named lint). POST-MINING FINDING (same
      day, the loop working as designed): the `byte_char_slices` proposer
      shipped with a LATENT postfix-continuation bug -- `[b'-', b'.'].into_iter()`
      rewrote to `*b"- .".into_iter()`, which parses as `*(b"- .".into_iter())`
      (unary `*` binds looser than method-call `.`), so the emitted byte-string
      derefs the Iterator. Found live in the katgpt-rs consumer proof: the
      `--verify` compile gate AUTO-REVERTED both katgpt-pruners edits
      byte-exactly -- zero damage shipped, the safety net's first real catch on
      this class. Fix riir-clippy `012c68b`: when the next non-ws char after
      the rewrite is `.` or `[`, parenthesize the byte-string literal
      (`(*b"{[").iter()`); 2 new fixtures (method-receiver position, index
      position). Re-heal with the fixed binary landed the 2 sites (compiler.rs
      + dfa.rs, oracle 2/2 Certified) -- katgpt-rs `0ffe0e15`, 34 verified
      edits total across the four new classes. REMAINING P9 tail (not
      landed, populations small/judgement): `collapsible_match` [6],
      `sort_by_key` [4], `explicit_counter_loop` [1], `manual_isolate_lowest_one`
      [1], `needless_pattern_in_..`-class [1], `redundant_reference_format_args`
      [1] — re-file from the next fired histogram.

- [x] **2026-09-03 LIVE REPRO → CLOSED same night (items_after_statements try_hoist, fix
      `143aceb`):** a `--fix --write --verify` sweep of riir-viewbridge reproduced the
      column-0 defect on real code — `crates/riir-viewbridge-abi/tests/net_ffi_roundtrip.rs`
      `ws_burst_over_the_inbound_cap_drops_oldest_and_keeps_order`: the hoisted `const N`
      landed correctly BUT the displaced `let url = spawn_ws_relay();` lost its
      indentation (column 0). **Both standing gates passed it**: the compile gate is
      indentation-blind (column-0 `let` is valid Rust → verify kept the edit), and the
      fmt-clean-at-HEAD gate correctly skipped the file (pre-existing drift) — so the
      mangling SURVIVED TO DISK. Edit reverted by the sweeping session; surface yielded.
      The fixer landed `143aceb` (indent-aware splicing + all-or-nothing hoist +
      macro_rules-definition refusal, 4 regression fixtures) and rebuilt the binary the
      same hour; the re-heal produced a CLEAN diff (hoist at correct indent, displaced
      statement untouched), clippy 0 + 10/10 under `--features net` (the file-level
      `#![cfg(feature = "net")]` green-zero caught by count-assertion before trusting
      the default run), landed viewbridge `ea0337e`. **Standing criterion recorded:** the
      hoist preserves indentation BY CONSTRUCTION (not via rustfmt) — rustfmt-skip files
      are exactly where a malformed hoist ships.

- [x] P1 **macro-nesting family** — CONSUMED 2026-08-28 (the descent unit): match_bool +
      uninlined macro-arg descent with the oracle boundary (uninlined direct-child skip —
      format_in_format_args owns that position); 9 fixtures (5 match_bool + 4 uninlined);
      score-bench run #9 steady (96.2/1.9/100/100); live sites healed e2e in ndb /
      riir-train / riir-ai (compile-gated per target).
- [x] P2 **array-literal guard family** — CLOSED 2026-08-29: documented permanent
      divergence, verdict evidence-backed (the `2bf2a44` E0308 proof; population measured
      36 raw hits → 6 map-form → 3 must-not-fire); fails the P7 value/rate bar (machinery
      ~60–80 lines for 3 cosmetic sites); joins the we-stay-silent table.
- [x] P3 **named-arg-with-rename** — CONSUMED 2026-08-29: probe-first oracle table
      (rename + width-$ re-point + identical-delete compositing; field-access values
      SILENT; unreferenced named arg = hard rustc error), the `RenameArg` emitter with
      17 fixtures, module 61/61, e2e byte-for-byte vs oracle; adopted cross-repo
      (katgpt-rs `2e445664` 42 edits, ndb `ffb3234`); + the stale-binary mtime
      discipline (check `~/.cargo/bin/cargo-heal` mtime vs the last proposer commit
      before adoption runs); incident record 2026-08-31: the installed binary sat STALE at the
            08-30 08:48 build across five 08-31 src/ commits (P15 narrowing, P12 suppression,
            Batch 92 corpus +11, guard repair, t48 tie-break) despite the P12 commit message
            claiming "installed binary rebuilt" — cause suspected: the session built under a
            /tmp CARGO_TARGET_DIR and the claim referred to that build. Rebuilt from HEAD
            23:28 +07 (`kernel_opt 411` probe PASS + suggest smoke) — the mtime-vs-commit
            check is load-bearing, trust the mtime not the message. **Recurred 09-02** (the
            Sep-1 18:01 binary sat through the 062 OOD harness + Batch 98's corpus +10) and
            **MECHANIZED same day**: `self_evolve_audit.sh` §0 `bin-age:` check — binary
            mtime (symlink FOLLOWED; macOS stat does not deref by default, caught live)
            vs the last commit touching src/ + Cargo.toml; sibling path-dep state
            deliberately out of scope; gates grep `bin-age:.*STALE`; both branches
            probe-verified. Binary rebuilt from HEAD 17:26 +07 (`kernel_opt 445` probe
            PASS + suggest smoke).
- [~] P4 **created-warning pair** — PARTIALLY CONSUMED 2026-08-28 (`1941712`): the
      plain-expression closure gap was already closed in expr_is_bool; the live residue
      was RECEIVER classification (turbofish `get::<` prefix match, `.lock(` joined
      RESULT_SHAPES); 8 sites terminal-fixed manually (opaque domain-method closure
      bodies — parse-only undetectable). Remaining open: crate-local Option-producing
      helpers (`env_var(` etc.) stay Unknown BY DESIGN (allowlist overfitting) —
      **no action planned** (documented divergence).
- [x] P5 **outcome-verdict wiring** — feed `--verify`-run outcomes + post-heal clippy
      results back into memory tiers. Tracked in [Issue 037] T2 (root-caused — CLI
      records outcomes as `false` only). **LANDED 2026-08-24 via 037-T2 (`667562a`):**
      kept-edit verdict matrix + the clippy lint-resolution oracle + the durable
      `.heal/trajectories.kv` landing; legacy rows stay Heuristic (history is not
      re-judged). Self-evidence-excluded from the E5 gate by design.
- [x] P6 **match_bool guard-on-first-arm + single_match_else literal arms** — CLOSED
      2026-08-29: 20+ shapes probed vs clippy 1.97 across 5 probe crates; flat-conjunction
      guards (`true if g` → `if flag && g`), arm-order-preserving emission, BOOL literals
      confirmed single_match_else-SILENT, and the single-stmt-else OVERSHOOT gate (real
      clippy silent there — never outrun the oracle); match_bool 24 tests + sm_else 28
      tests; e2e byte-for-byte on all 3 new shapes.
- [x] P7 **no-proposer default lints** — CONSUMED 2026-08-29: `manual_is_multiple_of` →
      VERIFY_LINTS tier (corpus 40 → 41; the receiver-allowlist lane rejected — covers
      ~none of the real sites), signed-site auto-revert proven e2e; retrieval floor
      re-pinned per the floor contract (blended top-3 97/101 → 95/101, fix-before-repin,
      baseline isolated via a HEAD worktree); `if_same_then_else` stays owner-gated.
- [x] P8 **unnecessary_sort_by deref-binding shape** — CLOSED 2026-08-29: 12-shape probe
      (deref → MA `sort_by_key(|&a| a)`, plain sort_unstable_by → MA sort_unstable — was
      also proposer-skipped); `Binding::{Plain,Deref}` classification + same-form-only
      rule; 13 module tests; e2e reproduces the oracle byte-for-byte.
- [x] P8b **unnecessary_sort_by field-key family** — CLOSED 2026-08-29: the key family is
      GENERAL (fields/methods/postfix/tuples/consts/external idents); THE FALSE-ORACLE-FACT
      correction (`custom_comparator_unchanged` asserted silence where the probe shows MA —
      test now fires); Paren-grouped keys deliberately skipped (oracle de-parens — parity);
      30 module tests; e2e parity on all 4 fired shapes.
- [x] P9 **compile-breaking proposal cluster in ruliology/player.rs** — CLOSED 2026-08-25,
      commit `64820c1`: root cause = the map_unwrap_or → map_or rewrite needs an unsize
      coercion (`&fn(..)` → `&dyn Fn(..)`) that map_or's free inference variable
      structurally cannot perform; fix = skip mappers whose TAIL casts to a trait-object
      type; 5 new fixture tests pin the guard; **score-bench run #5 delta** (heal
      98.1% → 96.2% — the honest we-stay-silent pin; created 1 unchanged, parse_safety
      100% held); the two innocent single_match_else edits landed on the real file
      afterward (riir-ai `6ca4f00e3`, 70/70 tests green).
- [x] P10 **needless_borrows_for_generic_args + needless_pass_by_value** — CLOSED
      2026-08-29, architecturally out-of-scope: `buffer.extend(&items)` vs
      `buffer.extend(items)` BOTH compile with DIFFERENT semantics — the compile gate
      cannot see it, so even VERIFY-tier is unsound for these lints (semantic-and-silent
      breaks); the correct consumer is `cargo clippy --fix` or the rustc_errors domain.
- [x] P11 **`is_postfix_safe` includes `Expr::Cast`** — CLOSED 2026-08-29 (same-day):
      `x as u64.method()` groups the method into the TYPE position (broken); cast
      receivers now parenthesize; the P7 local exclusion subsumed + the two identical
      LOCAL copies in manual_map/unnecessary_unwrap deduped to the shared helper (the
      copy drift that CAUSED P11); latent `!x as u8` wrong-math closed as
      defense-in-depth; 828/0 lib, score-bench #14 steady.
- [x] **P12 — verify-revert CHURN: the verify gate does not suppress re-proposal.**
      CONSUMED 2026-08-31: the re-proposal suppression filter — the fixpoint pass drops
      any edit whose exact (rule, restore, replacement) pair has 2+ consecutive
      compile-gate fails in the verdict backend (two, not one: whole-file reverts
      attribute fails to every edit in the file — run 2 is the retry window); text
      change auto-unlocks; kill-switch `RIIR_HEAL_SUPPRESS_REVERTED=0`; e2e proven
      (`tests/p12_suppress_reverted_e2e.rs`); score-bench #23 flat (96.2/1.9/100/100).
- [x] **P14 (mis-diagnosed) — MOOTED:** the "unformatted bare-block shape" was the
      manual_let_else mangle × fmt-skip interaction, not an items_after_statements
      emission defect. Closed; no code change.
- [x] **P15 — useless_format emitter can produce invalid cast-then-method text.**
      CONSUMED 2026-08-31 (probe-first, 4 rounds vs clippy 1.97): the single-arg branch
      is TYPE-GATED upstream (peeled type str/String) so cast/binary/unary/paren args
      are ALL oracle-silent — the single-positional path narrowed to the one
      syntax-provable firing shape (a `.to_string()` method-call arg, which also kills
      the double-call emission); every other non-ident arg refuses; refusal tests + 4
      new fixtures; score-bench run #22 UNCHANGED; retrieval floors 13/13.
- [x] **manual_let_else binding-name mangling** — ROOT-CAUSED + FIXED 2026-08-31
      (CRITICAL, 15 real gated breaks): the emitted pattern kept the ARM's inner binding
      instead of the OUTER let name; fix = `renamed_single_binding` byte-precise
      span-splice + the SHORTHAND-FIELD refusal (renaming rewrites the FIELD — refuse);
      4 fixtures.
- [x] **unnecessary_sort_by field keys (E0507)** — REFUSED 2026-08-31 (parse-only
      type-blind): `sort_by_key(|a| a.k)` moves the field out of `&K`, compiles only for
      Copy; the whole field subfamily now refuses (12 fixtures flipped to refusal tests
      + 5 new); whole-binding method keys keep firing.
- [x] **use_self `Self<A>` (E0109)** — FIXED 2026-08-31: `Self` cannot carry explicit
      generic args; whole-segment replacement when the matched path's args match the
      impl head; 1 new fixture.
- [x] **`neg_cmp_op_on_partial_ord`** — REFUSED-FOREVER class, no corpus rule:
      `!(a > b)` → `a <= b` is NaN-UNSAFE on floats; an autofix would be a wrong-code
      generator. Recorded in the cargo-heal skill divergence table as the canonical
      never-heal example.
- [x] **`type_complexity`** — REFUSED class, no corpus rule: the fix invents a type name
      + declaration site — no bounded parse-only rewrite exists; clippy itself does not
      suggest one.
- [x] **`unusual_byte_groupings`** — CLOSED-REFUSED 2026-08-31: below the frequency bar
      by 20–360× (2 sites vs the 39-site smallest admitted candidate); style-only with a
      fiddly fix-space; the frequency bar protects retrieval floors. Re-open only on a
      future fired-histogram population.
- [x] **`unused_assignments` + rustc-side lints** — OUT OF SCOPE by construction (not
      clippy — the clippy_verify oracle never sees them).
- [x] **P-a (guard-landing residue protocol)** — CLOSED 2026-08-31:
      `scripts/consumer_residue_sweep.sh` + `scripts/swept_repos.txt` + the AGENTS.md
      "Guard-landing protocol" section (a guard is not landed until the residue is
      counted); the script was caught lying twice (stale-build silence + the JSON shape)
      and now prints a liveness witness.
- [x] **P-b (the all-features screen)** — CLOSED 2026-08-31: the sweep screen is
      `cargo clippy --workspace --all-features --all-targets --keep-going` (all 32
      residue sites sat behind non-default features; check is structurally blind to
      both guarded classes); landed consumer-side as riir-ai `ci_feature_guard.sh`
      Layer 2.

- [x] P8 **nan-unsafe float sort comparator** (`partial_cmp(..).unwrap_or(Equal)`
      inside a sort closure) — **DONE 2026-09-01 (Plan 057, sec domain): landed as
      `nan-unsafe-sort-comparator`** — 12th sec rule, new `PanicSafety` family, the 6th
      FIXABLE proposer. **The intake's type-blindness concern DISSOLVED by std
      `total_cmp`:** the intake assumed the replacement needed operand DIRECTION +
      float TYPE (the `float_order::{desc,asc,desc_f64,asc_f64}` twin problem), but
      `a.partial_cmp(b)` → `a.total_cmp(b)` preserves the recv/arg order mechanically
      (direction rides the same positions), `total_cmp` is name-identical on f32/f64,
      AND the arg carries over VERBATIM — `PartialOrd::partial_cmp` pins the trait
      signature `(&self, other: &Self)`, exactly the shape `total_cmp` takes, so a
      pre-compiling call site rewrites signature-compatibly and only non-`total_cmp`
      receivers (`Option<f32>`, other Ord primitives) E0599 → verify-reverted. Gates:
      sort context (sort_by/sort_unstable_by/select_nth_unstable_by in the enclosing
      fn — the intake's precision guard) + exact chain shape + `Equal`-only default +
      comment guard + macro args refuse; helper-fn comparators passed to
      `sort_by(helper)` by name are the honest recall limit. Probe-first (Protocol C):
      zero clippy lints fire on the shape (1.97 pedantic+nursery+perf+suspicious+
      correctness+style) — the suggest-only tier the intake recommended was for a
      type-ambiguous rewrite that no longer exists; the strict-gated FIXABLE proposer
      (the `constant-time-secret-compare` precedent) is the more valuable form.
      Validation: sec lib 1051/0 (11 proposer tests incl. the exact corpus round-trip),
      default lib 1001/0 unchanged, boundary gate 2/2 with the new fixture
      auto-covered, retrieval floors HELD (13/13 + 14/14 combined, sec self-variant
      top-1), score-bench run #24 flat (96.2/1.9/100/100), clippy 0 touched files.
      ≈300 sibling library sites are now addressable via heal sweeps with
      `--features sec` (a later per-repo unit).
      **ADOPTED 2026-09-01 (same day):** katgpt-rs `e854958e` (64 files / 113 sites,
      src only) + unused-import cleanup `26d055c6` (7 files); riir-train `d37553e5`
      (41 files / 59 sites); riir-neuron-db `4af0c63` (7 files / 19); riir-mmorpg-
      examples `77d1319` (1).
      **ADOPTION WAVE 2 — 2026-09-01 late session (4090 box): the benches/examples/
      tests residue + the unclaimed repos DISCHARGED.** riir-clippy self `02a5580`
      (11 sites incl. the loop-habitat catch) + **proposer root-cause fix `5122bbe`**:
      the nan_safe_cmp walker's visit_expr had NO ForLoop/While/Loop/Let/Assign/
      Return/Try/Yield/Await/Range/Reference/Group/Repeat arms — the whole-block
      sort SCAN (syn Visit) detected loop-habitat sorts but the edit COLLECTOR
      never descended into loop bodies (found live: sort inside the per-case `for`
      at bench_clippy_ternary_tps.rs:310; 3 fixtures). riir-neuron-db `4524854`
      (16 files / 19 sites post-collision-dedup vs sibling 4af0c63); riir-train
      `20b53943` (6 sites post-dedup vs sibling 24bc7780's 53 in the same surface —
      both sessions swept tests/benches/examples within the hour, rebase
      auto-dropped identical hunks cleanly BOTH times, zero manual conflict
      resolution); riir-ai `1e6e4b06a` (**188 sites / 138 files across 13 crates** —
      the lib was NEVER swept: engine 47, gpu 71, poc 20, examples 15, civ 12,
      games 8, agents 4, rag 4, + 9 singles; per-crate lib suites all green,
      ci all-features screen's 4 error classes proven PRE-EXISTING not-mine).
      Post-wave live residue: ~9 sites in riir-train (2 tuple-receiver suggest-only
      + min_by + helper-fn comparator shapes = the strict-gate recall limits),
      ~2 max_by sites ndb, riir-clippy raw-string fixtures (correctly untouched).
      **WAVE 3 — katgpt-rs DISCHARGED 2026-09-02 (`a9a72a3e`):** the yielded
      tests/benches/examples surface re-scanned to **17 sites / 13 files**
      (the ~100 estimate was pre-sibling-sweep; their waves consumed the
      rest — re-scan-before-heal vindicated). All
      `partial_cmp(..).unwrap()/unwrap_or(Equal)` sort comparators →
      `total_cmp` (the latent NaN-panic class). Validation: 3 gate configs
      count-identical pre/post — default (spec_reconciliation 1 +
      latent_steering 2 + smooth_min_poc 0; latent_field_steering and
      micro_belief are default-on, the gate map said so), one 10-feature
      union (8 targets / 30 tests), and the 2 harness=false binaries
      (bench + example run as exes, structure-identical normalized outputs).
      12/13 rustfmt'd; 1 CRLF healed unfmt'd. Shared-checkout discipline:
      the sibling had 16 uncommitted files mid-flight; explicit-path
      `git add` + staged-set check (13, 0 outside tests/benches/examples)
      kept the commit clean — and the `git diff HEAD~1` 29-file scare was
      the working-tree artifact (single-ref diff reads the WORKING TREE,
      not HEAD; `git show HEAD` is the commit-truth command).
      Fixture-literal sites (score-bench raw strings) correctly never touched.
      Residue: tests/benches/examples slices (katgpt-rs ~47, riir-train ~53),
      riir-wallet-worker 1 FFI site + moka-wasm 10 FFI sites
      (ffi-extern-fn-catch-unwind unit, needs wasm32 validation), seal-game-editor
      17 raw unexamined. **New boundary found by the sweep (P-intake class):**
      TUPLE receivers — `(f32, f32)` sort keys suggest the rewrite but verify
      auto-reverts (E0599: tuples have no `total_cmp`; observed
      riir-mmorpg-examples `src/pet_visual.rs:550`, a cfg(test) site — the compile
      gate caught it correctly, manual fix = sort-key decomposition, a semantic
      choice). Tuple-receiver decomposition = candidate proposer extension
      (rewrite `|a, b| a.partial_cmp(b).unwrap()` over tuple keys to
      lexicographic `.0`-then-`.1` total_cmp — needs a decision on which key
      dominates; leave suggest-only until a real NaN risk shows up).
- [x] **P16 — manual_let_else binding-mangle residue: class CLOSED at 17 gated
      breaks in katgpt-rs (2026-09-02).** The 6038e59 root-cause fix repaired 15
      same-day; a 4090-side audit of every macOS-gated file the heal touched
      found the last 2 (`katgpt-backend/src/gpu.rs` test sites — invisible to a
      Windows compile gate), repaired by the sibling `c284dbb2` (M3-validated:
      both repaired tests EXECUTE on Metal) while a 4090 session independently
      derived the same fix and YIELDED per the dual-allocation discipline
      (duplicate commit reset, never pushed). Audit method: enumerate the 5
      macOS-gated source files, hand-diff each against the heal commit — the
      rest of the gated surface got only shrink_to_fit/const-hoist additions.
- [x] **P17 — standalone `rustc` is a VACUOUS checker for E0425-class mangles
      in dependency-heavy gated files (2026-09-02; the anti-vacuity control
      caught it).** Compiling a single gated file (`rustc --emit=metadata
      gpu.rs`) aborts at IMPORT resolution (E0432/E0433 unlinked crates) BEFORE
      late resolution can fire the E0425 — mangled and fixed versions both
      report 0 "cannot find value", so the check cannot discriminate. The
      discriminating check: extract the rewritten lines VERBATIM into a stub
      harness (fake receiver type) and rustc THAT — type-checks the real
      content without the dependency graph, ~2 min, and it is the only
      Windows-reachable gate for macOS-gated repairs.
- [-] **P18 (VERIFICATION SWEEP DONE 2026-09-03 — remainder is owner-only. Measured
      via `gh api` across all 13 gist-rs repos + katopz/katgpt-rs: (1) every repo's
      rust.yml runs the repo's own `scripts/ci_feature_guard.sh` (the full guard, not
      a subset) and EXISTS ON THE DEFAULT BRANCH (schedule/dispatch liveness, the
      `.issues/704` rot class) with schedule+dispatch+push triggers — including
      riir-train (rust.yml on `main`, its then-working branch — NOTE 2026-09-05:
      riir-train flipped to `develop` as default + working branch on 2026-09-04; the
      ops_dashboard.yml checkout was re-pointed `main` → `develop` the same day as
      this note, and its rust.yml triggers were stripped to dispatch-only) and riir-game-sdk (default
      flipped main→develop mid-day 2026-09-03 by the parallel CI session);
      (2) riir-unity has ZERO `.rs` files and no Cargo.toml — the boundary-only
      `no_rust_boundary.yml` is the correct shape, closed; (3) riir-dao's "no PAT
      needed" AGENTS.md claim VERIFIED — its only escaping dep is katgpt-core
      (public katopz/katgpt-rs, default GITHUB_TOKEN checkout), no riir-dapps path
      dep in the manifest yet; (4) `SIBLING_REPOS_TOKEN` configured on riir-chain /
      riir-dapps / riir-viewbridge / riir-auth / riir-clippy; **still MISSING on
      riir-train (Sun 04:17 cron), riir-mmorpg-examples (Mon 04:47), seal-remake
      (Wed 04:37)** — all three guards hard-fail LOUDLY at the first private
      checkout (the designed Issue-028 expiry-alarm behavior), but the owner should
      provision the secret before those first runs; provisioning is an owner act
      (secret value — not derivable, and minting one from a session's OAuth token
      is a security decision). riir-neuron-db (public-only dep) and riir-deployer
      (zero escaping deps by hard rule) legitimately need none. Original record —
      self-applied 2026-09-02, riir-clippy `2044722`; riir-deployer
      landed 2026-09-01, deployer `2b7d58d`; riir-ai landed 2026-09-02,
      riir-ai `4b085ebeb` — the workspace's WIDEST surface: 79 escaping deps
      across FIVE siblings pinned in its new scripts/standalone_dep_gate.sh
      (20 katgpt-* from public katgpt-rs; riir-neuron-db+neuron-db-sdk;
      riir-chain+riir-chaind; riir-train-engine; riir-dapps) + rust.yml with
      weekly Sat 03:58 UTC cron (default branch = develop, so the rot check
      audits it), Layer-4 Lean deliberately NOT duplicated (elan absent →
      loud skip; lean_proofs.yml strictly owns proofs); riir-train landed
      2026-09-02 same session, riir-train `66ffcd10` — 22 escaping deps
      across THREE siblings (12 katgpt-*; 9 riir-ai crates + the vendored
      cubecl-runtime; riir-neuron-db) + rust.yml with weekly Sun 04:17 UTC
      cron (default branch = main, this repo's working branch); its guard
      is COMPILE-LEVEL by design (five cargo check layers) so the ubuntu
      run is runner-friendly — the guard header's macOS exclusions
      (Issue 498) behave as on the Windows 4090 box on Linux; remaining:
      riir-train + riir-mmorpg-examples (ZERO workflows), riir-game-sdk
      (nightly+wasm32 only — verify test_gate.sh coverage), riir-unity
      (no_rust_boundary.yml only — no Rust, boundary-only shape may be
      correct), riir-dao + riir-chain + riir-dapps + riir-viewbridge (have
      rust.yml — verify it runs the FULL guard, per the 2026-09-02 re-scan:
      the original remaining-list predates several workflow landings); also
      verify the SIBLING_REPOS_TOKEN secret is configured per-repo (the
      hard-error makes an absent/expired one LOUD on the first scheduled
      run) — one
      per cycle) — full-gate
      workflows missing in the sibling repos (katgpt-rs Issue 701 R2 table,
      2026-09-01).** Self-apply landed: riir-clippy's own guard (which was
      already a SUPERSET of the katgpt-rs full gate — layer 5a is clippy
      `-D warnings` both feature extremes) is now CI-enforced via
      `.github/workflows/rust.yml` (weekly Fri 03:23 UTC on develop + main
      pushes + dispatch, mirroring the ndb/riir-chain spending-limit
      convention) + `scripts/standalone_dep_gate.sh` (9 escaping path deps
      pinned across 3 siblings — katgpt-rs public; riir-ai + riir-neuron-db
      private via `SIBLING_REPOS_TOKEN` with the Issue 028 expiry-alarm
      hard-error). Measured error count at landing: the full 5-layer guard
      ALL LAYERS PASS after (a) syncing all 3 floating siblings (riir-ai was
      2 behind — the stale checkout showed a dead `note_tiled_flash_cmma_pv_launch`
      that upstream had already wired) and (b) one own-crate fix: the denied
      `redundant_closure_for_method_calls` at `src/draft/jury_disagreement.rs:426`
      (→ `sort_by(f64::total_cmp)`). Two load-bearing facts learned: path-dep
      warnings are UNCAPPED but NOT denied — they surface in the log yet can
      never fail the gate (riir-gpu's tiled-flash warning leak, from the
      sibling's 09-01/02 work, is visible here but is their lint debt to
      close; do NOT fix mid-flight files in a hot sibling repo) — so the
      `-D warnings` gate fails ONLY on own-crate warnings, no sibling-hostage
      problem; and Cargo.lock is GITIGNORED here (floating siblings churn it),
      so the CI cache key hashes Cargo.toml + rust-toolchain.toml instead.
      CI runner notes live in the workflow preamble: docker Tier-2
      deterministically degraded via dead `DOCKER_HOST` (daemon+registry from
      shared runner IPs = flake factory; local runs own live-oracle
      coverage), layer-5c wedge budget 7200 s for the 4-core runner,
      ~300-min job budget (30-feature --each-feature layer + the GPU
      all-features tree), first scheduled runs are the cost measurement.
      **riir-deployer landed 2026-09-01 (`2b7d58d`)** — the cheapest unit in
      the set and the only INVERTED shape in the workspace: its dep-gate pin
      is EMPTY (zero escaping path deps IS the repo's hard rule — "a deployer
      must build when everything it deploys is broken"), so its
      `standalone_dep_gate.sh` asserts none rather than pinning some; the
      lexical `escapes()` kept verbatim so the four gates stay
      audit-equivalent. Verified by injection at landing (an injected
      `../../../riir-chain` dev-dep fires exit-1; `cloudflare/control-do`'s
      contained `../../crates/riir-control` edge does not). Full guard = 4
      layers (dep gate + clippy `-D warnings` at both feature extremes +
      `cargo test --workspace` incl. the loopback e2e), ALL LAYERS PASS ~40 s
      warm locally; workflow = standalone (no toolchain) + feature-guard
      jobs, weekly Fri 04:07 UTC (distinct slot — the default branch there IS
      develop, so the rot check audits it), main pushes + dispatch.
      Note for the remaining repos: the katgpt-rs 701 R2 table predates some
      workflow landings — re-scan `ls .github/workflows/` per repo at unit
      time instead of trusting the list (riir-dao already carries rust.yml;
      whether it runs the FULL guard is the check that matters).

- [x] **P19 — needless_return on the cfg-switched dual-return fn (2026-09-04, live
      misfire → CONSUMED same day, `2dc31c0`):** a `--fix --write --verify` sweep of
      seal-remake fired needless_return on `fps_bench.rs::present_mode_from_env` —
      the parse-view last stmt was `#[cfg(target_arch = "wasm32")] return
      PresentMode::default();` and the rewrite (whose edit span COVERS the stmt's
      attributes) promoted the wasm32 arm to an UNGUARDED tail, dropping the attr:
      `unreachable_code` on native (the consumer's `-D warnings` clippy caught it;
      the healer's own `cargo check` verify passed — warnings don't fail check, the
      P-a screen class). Probe-first (clippy 1.93): real clippy is SILENT on the
      whole switch — it runs post-expansion, so its verdict is per-view, while the
      proposer's "last statement" is parse-view only and is NOT last on the arches
      whose cfg drops the stmt. Fix: refuse when the trailing return carries ANY
      attribute (cfg = view-dependent last; anything else = silent attr loss on the
      rewrite) — 3 fixtures incl. the positive control that a cfg'd NON-return stmt
      earlier in the body keeps firing (the guard is scoped to the target stmt, not
      cfg-anywhere). Lib 1038/0 (+3), score-bench #35 steady (96.2/1.9/100/100),
      binary rebuilt + live-smoked (1 edit proposed on the repro: the plain return
      only). Consumer-side disposition for the record: the heal's other 13 edits
      were sound; the misfired edit was reverted to the dual-#[cfg] form in the
      same session (seal-remake `ce5f7ee6`).

**Standing never-heal divergence table:** the cargo-heal skill owns it (comment-guarded
matches, array-literal defaults, named-arg renames, nested macro args, NaN-unsound
negated comparisons, cfg-attributed trailing returns).


## Sweep findings log (2026-09-05, 4090 idle session)

- **2026-09-06 unit — riir-clippy Issue 071 CLOSED (`9ebac59`):** the tier-2 buildcheck gate red was a REGISTRY-BLOCKED BOX, not a check-set drift — the three check-only fixtures rode `FROM alpine:3.20`, so each oracle call paid a registry resolve; on this box each DeadlineExceeded (~60 s × 6 calls ≈ the recorded 399 s) → `Unavailable`, which the gate honestly refuses. The `OracleSilentOnBuggy` verdict was not reproducible (the Issue-723 repair-the-instrument-first lesson — all three proposed dispositions would have been wrong). Fix: fixtures → `FROM scratch` (registry-free, tier-2 399 s red → 2.74 s 4/4), `buildcheck::classify()` completion-marker backstop (no `Check complete` on stdout → `Errored`, never a fake `Ran(empty)`), gate failure output names `docker version`. Records: `.docs/11_domains/docker.md` header + degradation §, AGENTS.md issue-071 row; issue file removed per noise-reduction. docker.md's tier-2 row now reads "was ~9.6 s with `alpine` round-trips" — healthy-box cost, superseding this entry's wedge arithmetic.

- **2026-09-05 (later session) — mmorpg 092 item (3) CLOSED + chain E0282 ROOT-CAUSED:**
  (a) the substrate's 3 pre-existing clippy warnings healed at root cause (riir-ai
  `fcb880986`; unused_mut + unused_assignments + doc_lazy_continuation; lib 430/0/4i
  count-identical; crate clippy now ZERO) — mmorpg `e18a5ab` records the closure.
  (b) **riir-chain tx_log.rs E0282 root cause + the 1-line fix, for the owning session**
  (repo is READ-ONLY to the idle loop per the standing owner instruction — not fixed
  here): on non-unix the `#[cfg(unix)]` arm compiles out, so `old_mode` (tx_log.rs:2079)
  is `Option<?T>` constrained by NOTHING; the sole use is `mode.clone()` at :2104 — the
  ambiguous deref-clone on an unknown pointee (`<&?T as Clone>` vs `?T::clone`) → E0282
  "type must be known at this point". Unix builds are fine (`Permissions::from_mode`
  constrains it) and dapps dep builds are fine because the enclosing module is
  `chain_cold_write`-gated (dapps doesn't enable it; game-sdk `nft_e2e` does) — hence
  the feature-selection dependence. Fix: annotate the binding — `let old_mode:
  Option<std::fs::Permissions> = { ... }` (identical on unix; typed None elsewhere).
  PROBED LIVE at chain tip `3ffd900`: `cargo check -p riir-e2e --test prod_nft_mint
  --features nft_e2e` (game-sdk) fails on exactly this; game-sdk e2e sweep re-arm stays
  blocked on it.

- **riir-chain Issue-125 blocker CLEARED**: the NftProgram retirement landed
  (`4044471`) + closed (`5f52d4c`); the consumer half (`5d2d1bcbe`) landed in
  riir-ai. mmorpg 092 item (2) unblocked and RUN: guard Layers 1-3 pass,
  needs_ruliology 45/45.
- **mmorpg Windows platform break fixed** (`9f11c06`): `port_evict.rs` test const
  `EADDRINUSE` existed only under macOS/linux cfg - Windows E0425 at Layer 1.
  Added the `#[cfg(windows)] 10048` (WSAEADDRINUSE) arm, probed live (48/98/100
  decode Uncategorized on Windows; only 10048 maps to AddrInUse).
- **NEW ISSUE mmorpg 096**: scn_t11 Timeout (credited plateaus 5 < 8 at the
  hero-sleep boundary t~2200, never resumes) + scn_t6 fire drift 1451 -> 1811
  (+25%). Deterministic; pre-existing at tip; first full guard run since 09-02.
  Owner/scenelab-lane (871/872 session owns the scenario engine).
- **riir-dapps 012 closed** (`db98807`): live-verified by Plan 079 G3
  (claim -> gate Open, steps 1-4 PASS); file removed per noise-reduction.
- **game-sdk 026 closed** (`ea01194`); fresh-clone buildability re-probed CLEAN
  (cargo metadata exit 0) - the scenelab + panel commits pushed 09-05 resolve it.

- **2026-09-06 unit (M3 idle housekeeping) — boundary-guard 30th run + doc-sync CLEAN; queues empty.**
  Full-workspace `ci_boundary_contract.sh` (script mtime Sep 5 11:20, not mid-edit): exit 0,
  16 repos / 236 cross-repo dep edges unchanged from the 28th run, 2 known-drift = the ledgered
  seal-online-remaster pair (unchanged), C6 modelless-residue green (0 LOC tracked residue),
  C7 4 repos no-lock (the documented shape), C8 advisory worsened 498 → 507 commits behind on
  the `6f392727` pin (5 security fixes in the gap — standing owner call). katgpt-rs delta since
  the 29th run = the 513 lane only (`d434e3d5` cfg-scanner fix + `9865feae` docs pushed;
  `96cc0d7d` max_empty-WALL commit unpushed in the shared checkout — the sibling's in-flight
  work, hands off per Issue-665 discipline); scripts/docs surface, no dep-graph delta. doc-sync
  riir-clippy CLEAN: `9ebac59`/`55c3b3d`/`ad35e28` each carried their docs same-commit
  (docker.md degradation § + AGENTS.md issue-071 row verified). Issues scan across all 16
  contract repos: every remaining open issue is active-lane (riir-ai 629/769/844/846/851/852/
  875/876 league+ANE trackers, mmorpg 092/096/097 — 097 is the live agent's own tracker,
  riir-train 513 live lane + the 4090-gated set), read-only-ratcheted (seal-game-editor), or
  hardware-gated (chain 110; chain itself read-only to the idle loop) — NOTHING unclaimed this
  cycle. Mining: arxiv walk #8 time-blocked until Mon 09-07 ~07:00 +07; Protocol C intake clean
  (done=12 open=0); Batch 115 landed, nothing queued behind the walk. Benchmarks skipped (tree
  clean; the 09-05 run-10 full-guard green + score-bench #39 flat stand as current baselines).
  No work invented; stopping cleanly. Run-log note: the 30th-run row is recorded HERE because
  the boundary-guard SKILL.md run log lives in the katgpt-rs shared checkout, which carries
  the sibling's unpushed commit this cycle (a push from here would carry theirs).
  **Disk-pressure incident (this commit's window):** the first commit attempt failed
  `No space left on device` — Data volume at 100% (1.2 GiB avail of 1.8 TiB). Freed 10 GB the
  documented-safe way (this repo's own `target/debug/incremental`, no build running; the
  `target/release/cargo-heal` binary the global symlink points at untouched). The remaining
  big consumers are ACTIVE agents' surfaces and NOT reclaimable from here: /tmp A/B target
  dirs (`v1_*`/`v2*` ≈ 40+ GB — the game-sdk 097-V2 lane; `513_*_probe` ≈ 11 GB — the 513
  sibling; `e6_*` ≈ 19 GB — the E6 goldens lane; `skill_graph_t5` 4.5 GB), plus
  katgpt-rs `target/` at 93 GB (its owner's sweep to compact, the 728 precedent). Post-free
  avail 16 GiB — workable but the next heavy lane should budget its own cleanup.

- **2026-09-06 unit 3 (M3 idle housekeeping) — binary staleness repaired + Protocol C re-run clean; Batch 116 hands OFF to its owning session.** The audit's §0 caught the installed `cargo-heal` binary predating the last src commit (`63a6a34`, the Batch-115 addendum) — rebuilt at HEAD into an isolated `/tmp` target (1m25s) and copied over `target/release/cargo-heal` (the symlink target); adoption runs serve the current corpus again. **Protocol C re-run (post-binary-refresh): CLEAN** — intake done=12 open=0; E4 flat (heal=96.2% created=1.9% safety=100%); A1 13 distinct days; A2 10 repos; §1c self-collapse monitors deterministic across all 10 stores (game-sdk H=0.441/34.6% same-rule = the largest-store wide-retrieval shape, unremarkable); §1d attributed misses tiny (ai manual_is_multiple_of ×5, katgpt-rs chunks_exact_to_as_chunks ×1, chain manual_let_else ×2 + catch-unwind ×1, clippy manual_is_multiple_of ×2 — all verify-tier reverting classes already in the corpus, no new proposer class). **Batch 116 (prism fork): the untracked Plan 089 + Research 140 pair found on disk — the allocating session's in-flight work, NOT adopted** (mtime minutes old when found; the owner session's own unit-2 run-log does not mention them; dual-allocation salvage applies only to PUSHED artifacts). Queues unchanged: arxiv walk #8 opens Mon 09-07 ~07:00; nnethercote still blocked; 096 closed NOT-REPRO by its owner mid-session. Stopping cleanly — no work invented.
  active sibling lanes.** Full-workspace `ci_boundary_contract.sh`: exit 0, 16 repos / 236
  cross-repo dep edges — IDENTICAL to the 30th run measured hours earlier, which is the
  finding: two sibling lanes are actively rewiring manifests in the working tree right now
  (riir-ai: the Issue-097-V2 view-core move, `entity_sync.rs` deleted + `lib.rs` modified;
  riir-game-sdk: `Cargo.toml` + `test_gate.sh` modified, `ed25519.rs` deleted) and neither
  has moved the measured graph yet (their edits are tree-only, not manifest-committed).
  Re-run on the same day as the 30th run; the next boundary run AFTER these lanes land is
  the one that can move the count. Same 2 known-drift (ledgered seal-online-remaster pair),
  C6 green, C7 same 4-repo no-lock shape, C8 advisory unchanged at 507 commits behind the
  `6f392727` pin (5 security fixes in the gap — standing owner call; the 4090 has not
  re-pinned since the 30th run). katgpt-rs still `ahead 1` (`96cc0d7d` unpushed) — hands
  off that repo entirely, so this row records HERE again. Doc-sync: no new commits in this
  repo since the 30th-run docs commit (`3075c73` IS HEAD) — CLEAN, no action. Disk re-checked:
  Data volume unchanged at 16 GiB avail; top /tmp consumers are the same active sibling
  A/B arms inventoried last unit — nothing reclaimable from here. Queues otherwise unchanged
  from unit 1: arxiv walk #8 time-blocked until Mon 09-07 ~07:00 +07; Protocol C clean;
  issues/plans scans clean (see unit 1). No work invented; stopping cleanly.

- **2026-09-06 unit 4 (M3 idle housekeeping, next session) — quiet-repo clippy sweep + targeted Mode 2 audit; all clean.** Installed cargo-heal rebuilt WITH fix_verify,clippy_verify,latent_retrieval,rust_perf (the prior unit's rebuild was minimal-feature — `--verify` refused; 1m13s isolated at /tmp/rc-heal-m3, copied over the symlink target). **Sweep (Protocol B dry-runs → targeted --write --verify): seal-remake bin `single_match_else` (fa6801a9, compile-gated under mmorpg_persistence,mmorpg_full_sim, adoption 6/6) + tests `map_unwrap_or` x2 in authority_sim_adoption (2f540f34, same gate) + riir-ai rag/agents 9 edits (match_bool x6 rrf/score_norm/chunker, shrink-to-fit x2 retriever/babel_author; a463ee036 via temp-worktree push — the shared checkout was behind-3 with sibling WIP blocking rebase, pushed a463ee036 then ref-move + restore of the incoming docs-commit files, sibling WIP preserved)**. CLEAN: riir-viewbridge (10+tests), riir-dapps src (34), riir-auth+riir-dao (27), riir-deployer (12+tests), katgpt-web (2), seal-remake rest. **Mode 2 targeted audit (item 7): seal-remake seam/harness/lab_trace + viewbridge — zero substrate re-implementation (no sigmoid/decay/embed/dot vocabulary, golden-proven thin consumer); nothing to file.** nnethercote re-checked (still Jul 31 latest — idempotent with the prior record); benchmarks correctly DEFERRED (sibling riir-e2e item_e2e test at 364% CPU — timing gates would be noise); defer-marking sweep: only chain 110 (owner-gated). Cargo.lock churn in seal-remake = sibling's game-sdk bevy_ecs removal re-resolving, NOT committed twice. Queues unchanged; no work invented.
- **2026-09-06 unit 5 (M3 idle housekeeping, continuation session) — full re-verification idle report; queues empty, all partitions held.** Issue-first gate re-scanned FRESH (a lot landed in the window): chain Issue 127 RESOLVED + pushed (`81a00607`, synced) and chain already at `clippy --workspace --all-targets` 0 findings today (`de95452d`, the clippy-1.98 batch — the 08-18 chain sweep was superseded by its owning session, NOT a gap); ndb HEAD is the 513 lane's (`93c6681`/`fe0f54b` T6 + Issue 612 fix, fresh mtime on its test file) — CLAIMED, skip; mmorpg 096 is a RESOLVED record kept 1 cycle for the 4090 session's cross-check (mmorpg lane's, clean + synced); `maxion-protector` surfaced in the .issues scan but has NO BOUNDARY.md — not enrolled in the contract set, out of scope. Plans re-scanned: freshest are riir-ai 562 (their lane), clippy 089 (Batch 116 pair intact/untouched), kat 585 (513 lane, closeout already doc-synced 09-06), deployer 004 (T4.4a landed by its owner today), game-sdk 005 (097 V2 lane), train 387 (routing-restricted), seal-game-editor 156 (read-only) — all partitioned. **Item 8 (benchmarks) re-dispositioned: the contention cleared mid-session (sibling riir-e2e gate run finished; box 0 cargo processes, load 3.1) BUT the sweep has NO unclaimed target — every bench-gate repo (game-sdk budget rows, katgpt-rs 39 armed GOAT gates, riir-ai benches, mmorpg G2) sits in an active sibling lane, and the game-sdk e2e sweep already ran on the 4090 (09-05 re-arm unit)** — deferred on PARTITION, not just contention; first unit when those lanes clear. doc-sync: ran today twice already (unit record line-1563 window + the katgpt-rs 09-06 row) and this session's only landings are trivial heals (no doc per the anti-pattern). boundary-guard: 30th run already today. Protocol C: intake 0 open (09-05 re-run stands). nnethercote: re-verified via unit 4 (same day) — idempotent. arxiv walk #8: time-gated, opens 09-07 ~07:00 +07. No edits made to any repo this unit (pure verification); queues unchanged.
- **2026-09-06 unit 6 (M3 idle housekeeping, next session) — ITEM 8 EXECUTED: benchmark-regression sweep on the two quiet bench-gate repos; 2 REAL REDS fixed + 2 stale pins re-pinned (chain `8b5d371e`); game-sdk budget gate 7/7 green.** The unit-5 partition lifted (513-lane repos train/ndb/katgpt-rs all clean at origin; only riir-clippy Batch-116 rename + riir-ai 097-V2 WIP remain active — both avoided). **game-sdk `budget_gate.sh` (the M3-calibrated wall-clock rows, this box IS the calibrated class): ALL 7 GREEN** — tick p99 11.8 ms vs 40 ms (3.4x headroom), relay fanout p99 472 ns, quorum 2.75 us, CPI 29,735 tx/s, DeFi composed tick p99 15.3 ms; population floor 7/7, load 3.35 advisory, no drift vs baseline. **riir-chain: first FULL pass over all 85 LIB_SUITES rows since the 81a00607 arming + de95452d clippy batch** (the script's own 40-min wall exceeded one terminal window; rows run in 7 chunks, every row at its own feature set): spec_match_gate 12 suites / 107 tests exact (incl. 81a00607's own armed targets domain_registry_monotonicity 9/9 + signer_identity 7/7 + mnemonic 9/9); rows 1-19/20-28/30/32/35-61/63-71/73-82 all at pins. **Found + fixed, chain `8b5d371e`:** (1) `e2e_replay_recovery::a_node_whose_log_has_a_nonce_gap_refuses_to_start` FAILED — the drill asserted the B1 SEALED-head refusal, but **Plan 044's compaction-base boot probe now fires first** for front truncation (deleting the first of two post-checkpoint entries leaves the survivor's MAC verifying neither at base 0 nor index 0); production refusal is correct + louder; drill cause-asserts follow the current mechanism, doc narrates the 4th guard move, B1's absolute-count form stays pinned in tx_log unit tests + d5_log_retention_gates; 3/3. (2) `wasm_validators::all_program_ids_have_budgets` FAILED — **Issue 125 (4044471) retired NftProgram, the array lost its NFT entry, but the assert still demanded 16**; 15 is the live count, message names Issue 125; 381/381. (3) two stale pins re-pinned FROM MEASUREMENT per the gate's convention: chaind lib +guard 150→164 (Issue 044 daemon-wiring +11 + node guard tier/state +3, landed post-32ded556 without same-commit re-pins), domain-anchor 529→541 (Issue 122's 0616f194 ledger-plane tests). Lesson re-confirmed twice in one sweep: a count-pinned row catches exactly the commit that skips its own gate — 125's validation ran programs_fv 473 + programs 411 but never wasm_validators, and Issue 122/044 landed tests in rows whose pins nothing re-measured. Sweep log: game-sdk needed no edits; chain queue otherwise empty (110 owner-gated). Queues unchanged: arxiv walk #8 opens 09-07 ~07:00 +07; nnethercote blocked; Batch 116 GOAT in flight (sibling); 097 V2 in flight. Stopping cleanly.

- **2026-09-07 unit 12 (M3 idle loop, full-gate lane CLEAR — no cargo processes; issue-first gate re-armed) — three units landed: mmorpg 096 record removed (`fa6c850`), riir-ai 880 T0 kill-switch FIRED no-consumer (`c2f313b5a` + Bench 872), riir-ai 881 T3 cadence exit shipped (`9c1f15555`) + T1/T2 substrate audit (`8ca933685`).** Item 1 scan: in-scope claimables worked oldest-first (mmorpg 096 RESOLVED record — 1-cycle hold elapsed, file removed per its own text, git history holds the 4090 reappearance guidance; riir-ai 880/881 claimed this session; 882 pull-gated stands; 879 T3 heavy NLL bench stands; katgpt-rs 731/732 blocked/deferred stand; 733 filed 05:07 mid-flight by another session — avoided; chain/train/editor/esp32/maxion exclusions stand). **880 T0: warm-tier inline-fetch stall measured via `t0_issue880_warm_inline_fetch_stall_probe` (ignored release instrument, 512p × 23r = 11.8K rows): query_player_state p99 2.9 µs (0.006% of tick budget), empty flush (the per-tick contact) p99 42 ns, loaded flush p50 4.3 ms = write-side fsync, sparse-gated — the tick path has ZERO steady-state warm reads (STAQ re-login only) → prefetch would add work to save ~2 µs; T1–T3 un-armed by design; reopening surface = workerd DO; warm_tier_e2e 3/3 PASS at tip.** **881 T3: `cce_cadence_exit = ["katgpt-core/cadence_gate"]` opt-in — CceCrowdBatch `.with_cadence_exit(tau, d_min)` runs the same `CcePrimalDual::step` sequence but exits on `LoopResidualExit` fed per-step ‖Δρ‖₁; cap-fallback proven BIT-IDENTICAL vs `run()` (to_bits), caller-side d_min floor at the break site (substrate counts states, loop counts iterations — off-by-one by construction, windows stay fed); unit fixture exits 8/100 iters at equal er; G3 2883 default / 2887 gated 0 failed. 881 T1/T2 substrate audit: T1's mapping aspirational (one cycle/tick, no damp→deliberate→restart ladder exists — grep zero), T2 mostly subsumed by Issue 364 `clr_skip_until_tick` + KnpcSelector/Plan 304 (residual: low-tier-only K shrink, touches NpcClrState freeze format — deferred, arm only if T4 shows fire-tick cost matters); T4 re-scoped to calibrate the shipped T3 exit's tau on real zones.** arxiv walk #8 opens ~07:00 (next). Queues otherwise unchanged.**
- **2026-09-07 unit 11 (M3 idle loop, same window as unit 10; zero-contention greps only — the riir-chain full-gate still running) — ITEM 11 web sweep + a REAL find: the e854958e Group-B tail in the never-swept repos; riir-neuron-db 13 production sites fixed (`d22d3b2`+`778f17a`); chain remainder on record.** nnethercote perf-book re-check: ONE new commit since the recorded state (`a05dd0f`, the Iai-callgrind→Gungraun rename — tooling nomenclature, zero rule content; NOT a distill candidate). Broader web sweep (rust-perf guides, rustc 1.90 LLD-by-default, compiler-perf posts): nothing rule-shaped for the runtime-perf corpora. **The code-smell hunt then struck: the 832/729/878 NaN-comparator lineage had NEVER measured the 5 smaller repos.** partial_cmp-unwrap census across 10 repos: zero live sites (2 comment-only mentions). total_cmp census: 90 sites / 5 repos → classified per the 729 Group-B rule (ascending + binary_search = fine; descending/selection = NaN promotes): **riir-neuron-db 13 production sites REAL** (experience_graph ×4 similarity ranking, dense_embed ×3 top-k, bm25 ×1, index:1374 select_nth, funnel ×1, consolidation ×2, shard_compactor CPQR pivot ×1 — all `b.total_cmp(&a)` from the e854958e sec-domain heal, which fixed the abort class but wrote NaN-promotes-in-descending; this crate had the heal WITHOUT the 729/878 float_order fix; katgpt-core is the crate's only cross-repo dep and float_order is unconditional at main's head — no manifest edits) — **fixed via `desc`/`cmp_for_max`, NaN sinks under every direction, bit-identical on NaN-free input. neuron-db-sdk ×4 documented-deliberate** ("deterministic on NaN" is the stated intent; desc NaN-first-vs-last is an owner call, recorded not taken). **Skip classes recorded:** game-sdk ×5 (demo/test NaN-unreachable + facade forbids katgpt-core; asset-converter build-tool dep cost), mmorpg ×2 (scenario harness), seal-remake 0 (all ascending/documented). **riir-chain 14 sites = open remainder** (claimed lane at census time; re-run command + classification rule pinned in the issue). **The 841 double-reversal fired LIVE in my own first pass** — mechanically preserving `desc(b.x, a.x)` positions double-reversed into ASCENDING and 10 tests failed in exactly the touched paths (bm25 ranking, consolidation sleep_transfer, funnel drain, graph traversal); corrected to natural `desc(a.x, b.x)`; the round-trip proves those suites are this class's regression guards. Detection/fix separation honored (issue filed first `d22d3b2`, highwater 612→613); HISTORY row at close; issue removed per noise-reduction. Validation: clippy 0 at both feature extremes, 1346/0/15i all-features lib. Queues unchanged: arxiv walk #8 ~07:00; chain 14 sites wait for its lane; katgpt-rs lockfile dual-pin still the owner call.**
- **2026-09-07 unit 10 (M3 idle loop, context-overflow continuation of the Batch-121 thread; zero-contention units only — the riir-chain full-gate lane is compiling on the box) — items 1/2/4/6/13/14 swept: 3 docs commits + boundary 36th run CLEAN; arxiv walk #8 still time-gated ~07:00.** Item 1 re-scan: all claimable issues verified parked — riir-ai 879 T3 (league-adjacent GPU measurement), 880 T0 (build+run, box busy), 881 (riir-ai code lane, heavyweight), 882 (pull-gated, triggers not fired), mmorpg 096 (RESOLVED record held for the Monday 04:47 UTC cron cross-check — today IS Monday but cron is ~7h out), chain 129 (the active lane's own fresh filing), esp32 110 (needs physical board), train (routing-restricted), seal-game-editor (read-only), maxion-protector (no BOUNDARY.md, standing out-of-scope ruling); riir-clippy `.issues/` empty. Item 2: **plan bookkeeping lag flipped, clippy `1ba12bd` — 096 T2/T3/T4 were done-in-substance (generations.rs gen-7 section + `tests/generation_v7_freeze.rs` + [[test]] row + `print_v7_chain_tables` all verified in tree) while T5–T8 were checked; 101 had a DONE-but-unchecked corpus-grep line + a duplicate RECORD flip.** Item 4: **riir-ai Issue 878 file removed, `f2d2b128f` — HISTORY row 19 already said 'file removed at close' but the file survived the addendum session (all 4 commits were edits, no deletion commit); record + path refs verified before rm, no other doc touched.** Item 6: **boundary-guard 36th run — full-workspace exit 0, 17 repos / 235 edges (35th: 16/234; delta = riir-esp32 enrolled with its satellite-probe edge — katgpt-rs `056fabe8`'s 16→17 enrollment doc CONFIRMED by measurement), S1 mmorpg guard exit 0 zero violations; the C8/C9 escalation recorded: katgpt-rs now DUAL-PINNED across build roots (`6f392727` ×3 lockfiles at 567-behind/5-sec-fixes — worsened 530→567 — vs chain's fresh `a0ef5d68` at 3-behind), owner-facing bump decision; run-log row katgpt-rs `f2dc4cc1`.** Item 13: doc-sync delta since 04:00 = only my 3 docs-only commits (no doc impact; Batch 121's doc-sync already ran through that landing). Item 14: Protocol C re-run — intake done=12 open=0, §1c monitors deterministic across stores (ai same-rule 27.9% = known largest-store shape), §1d misses = the same known verify-tier classes, ELO top = nan-unsafe-sort-comparator 1710.5 W=128 L=0 (the sweep class). Items 3/7/8 deferred on CPU contention (the chain lane's full-gate; game-sdk budget rows ran green yesterday). Queues: arxiv walk #8 opens ~07:00 +07; ds4 re-armed at `c5ae2eb5`; nnethercote blocked. Stopping cleanly.**
- **2026-09-07 unit 13 (parity-lane continuation session) — healer binary OUTAGE repaired + quiet-repo sweep landed; seal/riir-ai verified at tip under the 883 wave-3 landing.** The global `cargo-heal` symlink was DANGLING (target/release/cargo-heal missing — target/ swept, likely the disk-pressure cleanup; a dry-run failed `no such command: heal`) → rebuilt at HEAD with fix_verify,clippy_verify,latent_retrieval,rust_perf (1m23s isolated /tmp/rc-heal-m3), copied over the symlink target, global command verified. **Sweep (riir-clippy src/ only — seal-remake + riir-ai hands-off, both carrying sibling WIP): 42 files / 242 edits compile-gated applied (shrink-to-fit-after-build ×31 incl. 3 in the freshly-landed Batch-126 corpus files + vec_with_capacity capacity propagation ×3; the dry-run's [docker]/[clippy] bracket lines were span metadata, not edits) — spot-checked the vec_with_capacity sites (loop bounds match capacities: 256/256, 4/4), 1167 lib tests green + clippy 0; landed `d08b91d`.** Disposition: chain Issue 110 (the flag key-store funded-ness note) — recorded-not-fixed, its fix IS the issue's design work, not a lint. seal-remake at `961a2b77` (the 883 sibling's wave-3 seal literal patch landed + pushed by them) + riir-ai at `0c837665f`: adoption probe 6/6, workspace green — Issue 008's lanes survive the wave; the residual `drain_and_commit_per_player_state now_tick` warning stands (that lane's own gate; fixing the symptom without the tick-consuming commit would bury the design question). Queues: chain full-gate lane still running (benchmarks stay partitioned); ds4 re-armed per snapshot; arxiv walk #8 consumed by Batch 124.
- **2026-09-06 unit 9 (M3 idle loop, continuation of unit 8) — the 729 residual + the whole riir-ai NaN-comparator tail: ~140 sites across 8 crates incl. PRODUCTION civ `map_tick` (riir-ai `1ee35da79`, `.issues/878`) + the katgpt-rs deref stragglers unit 8 shipped (`649ce5fe`).** The handoff's 3 recorded sites were the visible tip: the same catch-all grep widened to tests/examples/benches/`cfg(test)` mods and the `std::cmp::Ordering::Equal` spelling + multi-line comparator heads, finding PRODUCTION sites 832's production-only pass also missed — civ `guard_ai ×3, predator_fsm ×3, predator ×5, eagle, crime, movement, leo_act` (NaN in a belief confidence or dist² makes min_by/max_by pick arbitrarily) + riir-engine `fourier/tuning.rs`. Executed by 4 parallel agents on disjoint crates (engine+poc / examples / games+civ+shared / gpu), coordinator follow-ups on the completion-grep residue. **Deref-depth lesson made systematic (probe-verified in a scratch crate, rustc 1.98.1): `xs.iter().max_by(|a, b|…)` binds `a: &f32`; `xs.iter().enumerate().max_by(|(_, a), (_, b)|…)` binds `a: &&f32`; the same chain through `.map(|(i,c)| (i, owned))` binds `a: f32` — three depths, one line shape, ~27 post-compile-gate corrections across both repos. A parse-only proposer cannot see it; the compile gate is the only detector.** games-shared + router gained `cmp_for_max` twins in their crate-local float_order modules (katgpt-core optional there). riir-gpu: 8 files not(macos)/CUDA-arm edited mechanically, validated by identical-pattern twins + the release union (15 targets + 2 examples + gemma2 lib-test no-run, 0 errors). **EOL lesson re-strikes via the agent write path**: 13 CRLF gpu test files came back LF (+4591/−4501 diff) — caught by diff-stat before push, CRLF restored byte-exact, compile re-verified, amended (final +274/−184); the slice-13 trap is not rustfmt-specific, ANY non-bytes-aware write converts. Pre-existing find recorded, not fixed: sheaf_personality_bridge 3 tie tests fail at HEAD (fixture premise contradicts current_dominant_tag's sigmoid sign convention; probe proves the comparator swap is tie-identical). f2c305dd's own latent breaks (7 comparator sites broken under feature-gated/all-features compiles + 11 dead imports) fixed in katgpt-rs `649ce5fe` — each found by compiling the gate that includes it. staged_set_audit "3 episodes"/"stale-vs-HEAD" = false positives again (all episodes mine; every would-lose line is a sweep target). Validation: riir-engine lib+tests+benches clean under the 11-feature union, poc lib+benches clean, 10 examples clean under 5 features, games tests+benches clean under leo/imagine/swarm union, civ lib clippy 0 new, spot-runs 6 suites (karc 37/0, sheaf 39/0, attribution 2/0, ict_clr 1/0, quant_moe 1/0, cross_game_prefix 1/0); katgpt-rs side: pruners all-features 0 err/126 pass, core 1974/0, bench_453 2/0, cond_audit 5/0, mcts_eq1 4/0, manifold 12/0, stepcode 6/0, 13 examples 0 err. Queues: arxiv walk #8 opens 09-07 ~07:00 +07 (next mining input); everything else unchanged.
- **2026-09-06 unit 8 (M3 idle housekeeping, continuation session) — katgpt-rs Issue 729 NaN-comparator sweep: the missed repo of the Issue-832 class closed (`f2c305dd`, 132 files, ~160 sites + 13 Group-B fixes).** Code-smell hunt (new_v4 clean, partial_cmp re-grep) found LIVE legacy `partial_cmp(..).unwrap_or(Equal)`/`.unwrap()` comparator sites across katgpt-rs — the repo that OWNS `float_order` was never in riir-ai Issue 832's T4 sibling population. Sweep executed end-to-end: **Group A** (~150 sites) -> `asc/desc/cmp_for_max/cmp_for_min` terminals; **Group B** (13 sites) — total_cmp in descending/selection positions promotes NaN above +inf — the sharpest being `katgpt-ruliology/bandit::best_arm` where a NaN payoff was SELECTABLE as best arm; direction-neutral positions -> total_cmp. No-dep crates (dec/tokenizer/micro-belief/sense) + optional-dep attn-match (local documented `desc_nan_last`) use the honest fallback. **Census lesson: single-line greps undercount multi-line comparator chains ~50%** (`.unwrap_or` on the next line) — the complete census is a partial_cmp + 3-line-forward unwrap window. Match-ergonomics deref table recorded in-issue (destructured-pattern `**a` vs field-access `*a.1` vs range `&a`); `cargo check -p` feature-UNIFIES vs `--workspace` does not — per-crate green is not a workspace claim. Validation: workspace `--all-targets --keep-going` clean + 17 suites green (core 1974/0, root 203/0, pruners 126/0, speculative 305/0, dec 225/0, forward 125/0, +11 more) + clippy clean. Exclusions pinned: float_order oracles, 8 loud `.expect(no NaN)` timing guards, None-arm sign matches. Cross-repo residual for the riir-ai lane (active sibling WIP): sense_bandit_demo x2, karc_bridge test x1. staged_set_audit's stale-vs-HEAD signal = the documented deletion-sweep false positive (every would-lose line is a sweep target, verified before staging). Queues otherwise unchanged: arxiv walk #8 opens 09-07 ~07:00 +07; nnethercote blocked; Protocol C clean; boundary 30th run stands.
- **2026-09-06 unit 9 (M3 idle housekeeping, continuation session after Batch 117) — boundary-guard 35th run + doc-sync + the ffi-dissolution follow-up wave; 6 commits across 4 repos.** The ffi lane LANDED mid-window (riir-ai `23389c801` 12:28 + riir-chain `47098911` 12:29 — the chain dev-dep removal landed the same minute as the 34th run's prediction required) — exactly the landing the 34th run said must be re-measured. **Boundary 35th run: EXIT 1 on entry → the 2 C4 rots the 34th run predicted (the riir-chain CANONICAL row's prose names `riir-ai` + `riir-ffi`; C4 extracts both; the lane updated C5 but missed the C4 case list) → FIXED unowned per the 34th run's stated option: case list + `riir-ai` (repo itself, the existing `riir-chain` exemption class) + `riir-ffi` (DISSOLVED lineage — cannot be a measured dep again) → canaried BOTH directions (planted riir-zzzcanary rots; revert byte-identical → **exit 0, 16 repos / 234 edges**, 235→234 = exactly the chain→ffi dev edge leaving) — riir-ai `0af26c2b5`. SKILL.md 35th-row logged katgpt-rs `3303f467`.** **Doc-sync (the dissolution's book gaps — the lane's in-landing docs covered overview/02_crates/08_wasm/AGENTS/BOUNDARY but missed 4 surfaces):** riir-ai README crate table ('⚠️ Blocked' → Dissolved row + honest count), architecture.md Mermaid FFI node (+ VIZ/GMTOOL EXTERNAL annotations, same staleness class), 03_pillars T3/T4 + risk row, 10_browser/latent_state_framework (incl. TWO PRE-EXISTING broken links to riir-engine memory_layout.rs — canonical moved to riir-wasm, engine re-exports; the retired wasm_compute gate recorded as collapsed-by-570) — ai `d500c4a31`; riir-train sibling-layout line — train `b0fec904`; katgpt-rs chunked_content_store consumer paths (overview + 03_memory, broken link re-pointed to riir-chain/tests, verified on disk) — kat `030a0137`; run-log row — kat `b9d7e07c`. **Dissolution follow-up wave (en-route finds, both lanes now done → unowned → closed):** riir-wasm latent_sidecar.rs 3 in-source comments citing the collapsed `wasm_compute` gate → `latent_sidecar` (Plan 570 L58's own collapse; comments-only, cargo check -p riir-wasm green) — ai `de00a8f32`; chain 3 manifest comments describing pre-dissolution shape (root :1469 'dev-deps riir-ffi instead' vs :913-916's correct story; chaind :185 '+ riir-ffi' vs the actual `riir-wasm/latent_sidecar` dep line; chaind :724 'doesn't pull riir-ffi') — chain `af5bbcef`, cargo metadata green both. Remaining `riir-ffi` mentions workspace-wide re-scanned post-fix: all pattern-lineage citations ('mirrors riir-ffi's convention') or dissolution-aware — consistent. Queues unchanged: arxiv walk #8 opens 09-07 ~07:00 +07 (genuinely time-gated, nothing to unblock); nnethercote blocked (re-checked today by the prior unit); benchmarks partitioned (NaN lane holds ai+kat, 098 lane holds mmorpg+game-sdk); mining queue empty. riir-train ff-pulled `4b390df6` en route (the sync rule). Stopping cleanly.
- **2026-09-07 unit 14 (parity-lane continuation session — riir-ai Issue 885 claimed and LANDED end-to-end; seal T7 combo discharged; doc-sync across 3 repos; box-saturation + disk-critical state recorded).** Item-1 gate re-scanned before the claim: 885's touch surface (riir-games-mmorpg topology/boot/mod/lib) had ZERO overlap with the dirty files (all riir-gpu = Plan 390's lane; 883 wave-3 committed at `0c837665f`) → claim safe. **Substrate `c35eb1154`:** `RsmTopology::parse_value` (pure; arms testable without process-global env mutation) + `from_env_named(env_name)` + `from_env()` delegating via `DEFAULT_TOPOLOGY_ENV` (examples-branded, bit-identical) + `AuthorityBootConfig::topology_env_name: Option<&'static str>` threaded through `spawn_relay_server` (boot eprintln names the env actually read); 9/9 topology lib tests (7 existing + parse arms + unset-name fallback); clippy clean at default AND `authority_sim` features. **seal `7d673e82`:** `SEAL_TOPOLOGY` → `boot.topology_env_name` in `seal_boot()`; adoption probes 6/6; seal `.issues/008` T6 row CLOSED (matrix + status + task line). **Docs `8f1904e7b` (riir-ai) / `1fa8d86f`+`cafe9444` (seal) / `ae9de0a` (game-sdk):** 885 issue file removed at close + HISTORY row; T7 gm_server combo VERIFIED GREEN at post-883-wave-3 HEAD (`--features gm_server --all-targets`, zero warnings — the wave-3 unblock condition discharged); seal AGENTS Issue-092 env-knob list; game-sdk node_roles.md topology row notes the parameterized seam. **Box state for the next session:** disk hit 100% mid-session (2.4 GiB free — my own `git add` failed with index.lock ENOSPC; freed 3.6 GB by rm-ing MY /tmp/issue885 target; /tmp/issue879_kvarn 1.4G + /tmp/issue881_t4b 4.5G are OPEN-issue lanes' caches — NOT reclaimed, their issues stand); 801%-CPU bevy release build in /tmp/w3_ex + chain plan048 gate running → items 3/5/8 stay deferred (Plan 337); 4090 UNREACHABLE (LAN + Tailscale) — no batch-conflict check possible. Issue-landscape after this unit: katgpt-rs 731 active-today (T4 flows through Proposal 045, owner gate), seal 007 = two owner calls only, riir-ai 876/879/881/882 carry dormant /tmp lanes (collision risk), examples 099 = 883 sibling's, chain edits would poison the running gate. Batch 126 (tool-text family) mining input stands, gated on 1–4 + box drain. Stopping cleanly.
- **2026-09-07 unit 15 (M3 idle loop continuation — Batch 126 DISCHARGED as N/A at the trigger re-check; Plan 106 COMPLETE; no re-clone, zero-build unit).** Box drained per the start context (25 GiB free, CPU 36%; the chain gate re-runs in its DEFAULT target dir, the 883 lane wrapped) → item-5 gate re-opened. **Trigger check FIRST (B120 zero-build protocol, GitHub API): PR #991 head still exactly `ffd85d42` — the Batch-125 pin, updated_at 2026-09-06T23:18:03Z BEFORE the B125 mine read; upstream antirez/ds4 main still exactly `c0a6119f` — the Batch-123 pin. NO DELTA, no force-push since; the ds4 queue is DISCHARGED.** Per Plan 106 T5's own verdict ("LIKELY CLOSES AS N/A unless a second instance surfaces"), Batch 126 closes **N/A without a mine**: the tool-text preservation family was B123-banked, the QA-validator files were B121-family-deduped, and the sole deferred remainder (the escape-round-trip observation) stays deferred-single-source — reopen on the next ds4 trigger (a force-push past `ffd85d42` or upstream movement past `c0a6119f`). **T7 verified done-in-substance:** `.raw/ds4-metal` already removed at the B125 landing (the plan box was stale); only the tiny historical batch scripts remain (`batch118/` 52K, `ldg_probe/` 16K — working scripts, not clones, left). **Plan 106 marked COMPLETE** (all tasks [x], Status → COMPLETE). Pins now current: PR `ffd85d42`, upstream `c0a6119f`. Queue state after discharge: the ds4 lane is EMPTY; arxiv walk #8 consumed by Batch 124; nnethercote checked unit-13; mining queue EMPTY — next mining input requires a NEW trigger (ds4 movement, an arxiv walk #9, or the post-mining intake). Commit+push this unit; stopping cleanly.

- **2026-09-07 unit 13 (M3 idle loop, continuation after Batch 127's thread handoff) — housekeeping: boundary 38th scoped run CLEAN + README doc-sync gap fixed (`b718e34`) + stale binary rebuilt + ELO recomputed ×4 stores + Protocol C clean.** (1) **Boundary-guard 38th run** (scoped `--repo riir-clippy` + S7, parse-only; script verified not-mid-edit first — mtime Sep 6 13:13 unchanged, `bash -n` clean): TRUE exit 0, 10 edges all declared (unchanged — B127 was corpus-only, no manifest surface, confirmed by measurement), S7 dep-line grep EMPTY, SP5a green, C7/C8 not-checked = the documented no-lockfile shape; the 37th run's same-day full-workspace CLEAN covers the rest of the workspace. Run-log row NOT appended to the katgpt-rs SKILL.md (that repo's 731 lane is active — same deferral as the 37th row; next katgpt-rs commit can carry both). (2) **Doc-sync: ONE gap found + fixed** — the B125/B127 count-pin sweeps healed AGENTS/tests/router/headers/.docs but MISSED README.md: domain table kernel_opt "Batch 1-117 | 544" → "1-127 | 565" (+ the distill list extended to the .docs header's set) + rust_perf "101 rules (Batch 112)" → "112 (through Batch 125...)" + the corpus-tree line 544→565. Committed `b718e34`, pushed. (3) **§0 setup health RED fixed**: installed cargo-heal binary (11:35) predated the B127 src landing (13:17). REBUILD GOTCHA: the audit's suggested command (`--features latent_retrieval`) FAILS — the bin's required-features are `latent_retrieval,rust_perf,clippy_lints,kat_service`; the unit-4 full set (`fix_verify,clippy_verify,latent_retrieval,rust_perf`) ALSO fails without `kat_service` (the Plan-107 T1 required-feature). Correct command: `cargo build --release --features fix_verify,clippy_verify,latent_retrieval,rust_perf,kat_service --bin cargo-heal` (1m20s). Note: staleness exposure was narrower than §0 implied — kernel_opt/sec are source-walk domains (565 correct even in the stale binary); only the compiled domains + the score_bench combo fix were stale. Audit §0 now all OK. (4) **ELO recompute**: local store (460 matches, 118/176 rows) + katgpt-rs (27/178) + riir-ai (11/2350) + mmorpg (408/419) — pending-recompute counts cleared; stores are gitignored `.heal/` writes only, sibling source trees untouched (verified). (5) **Protocol C: intake done=12 open=0 STANDS; §1d miss-map triaged, NO new actionable class.** The mim cluster (riir-ai 24 W rows, top shapes `i % 2 == 0`/`tick % 2 == 0`) = pre-B59-fix-era rows (B59's local-receiver guard landed TODAY mid-day; post-fix heal passed clean) + broken-tree-era rows (the 830-era all-features audits ran clippy on a 99-error tree — clippy fires on mid-edit trees, the `--verify` compile gate auto-reverts, ELO demotes; that's containment working as designed, not a proposer bug). Probed 1.98.1 live: `n % d == 0` unsigned+variable-divisor fires and its suggestion compiles; signed `% 2 == 0` does NOT fire; generic-T sites cannot compile so the lint cannot reach them. The `map_unwrap_or` single loss (`strip_suffix..map(str::to_string).unwrap_or(text)`) = one sample, no recurrence in any of the 10 stores, no consec — the shape compiles only when the default is OWNED (our own `usize_text` is the working instance); a guard off one unexplained sample is speculation → recorded, not acted (the probe-evidence rule cuts both ways). Score-bench NOT re-run (no proposer change; E4 PASS at 96.2/1.9, run #76 stands). (6) **Mining: queue EMPTY, no trigger** — all QUEUED leads consumed (B113/B118/B119/B124/B127); nnethercote re-checked still Jul-31; arxiv walk #9 = the active sibling session's lane (collision-avoid). 4090 checked before mining (Tailscale; LAN refused): 14% / 659 MiB = idle, but no batch armed anyway. (7) Cross-repo: chain full-gate lane mid-run (rsync'd ENOSPC recovery), katgpt-rs 731 lane + the arxiv distillation session both active, Plan-571 sibling circling seal-remake — all lanes claimed, nothing unclaimed here; items 3 (benchmarks — tree clean post-commit, score-bench current) correctly skipped. Stopping cleanly.
- **2026-09-09 unit 16 (M3 idle loop, post-B136 hygiene session — Protocol C re-run under the v8/parity-arm box lock; zero-build unit).** Box: the parity arm (p335, PID 90715, ~100% CPU) blocks v8's quiet gate (9.6 h waiting at last check, deadline 09-10 10:50) → all cargo work deferred, Plan-337 posture. Units landed this session: clippy hygiene (075/076 removed, `0579682`), dapps 023 defer-mark (`f5b75ba`), boundary 48th S1–S8 sweep (kat `cc0a29e8`, all clean; false-empty lesson re-hit — path anchor + `2>/dev/null`), doc-sync delta (CLEAN — B136 self-documenting via its own .docs ledger walk; chain 133 HISTORY row verified present L451; morning sweep `ff2de9d`/`b5929a6b`/`a59050b` covers the rest), item-11 scout (nnethercote consumed through Jul 31 = B112; no new candidate; queue already holds CloudRift pair + ds4 PR#5 + ds4 Part-B). **Protocol C (this unit): gates all PASS — E4 96.2/1.9 flat (run #82), E5 via P9/P8, A1 17 days / A2 331 heal-commits; §1d miss-map = the SAME classes unit-13 triaged 09-07 (mim = pre-B59-fix-era + broken-tree-era rows; manual_let_else = P16 closed; chunks_exact = Batch-91 corpus rule; map_unwrap_or single-sample) — NO new proposer class, intake stays done=12 open=0. ACTIONABLE FINDS (both cargo-gated, first post-unblock actions): (a) §0 STALE BINARY — installed cargo-heal (03:24) predates B136's corpus commit (2bef7bc 03:29); rebuild with the unit-13 full feature set `fix_verify,clippy_verify,latent_retrieval,rust_perf,kat_service` (1m20s, isolated /tmp target); kernel_opt/sec unaffected (source-walk domains). (b) riir-dapps store: 4 pending-recompute ELO rows — `cargo heal --recompute-elo` against that store after the rebuild.** Score-bench NOT re-run (no proposer change). Queue state: B137 (CloudRift matmul article) is the natural next batch, plan-first, left for the B135/136 session's lane claim (21-min cadence at session start); arxiv walk #9 time-gated Mon 09-14. Stopping cleanly.
