# Plan 224 — fsearch outbound-PR distill: bench-refine-pr lane (gated on mainnet launch)

**Status:** IN PROGRESS — T1–T4 LANDED this session (verdict, frozen bench fixture, quiet-box before bench + SVG, this plan + record 112 + queue line); T5–T8 `- [-]` GATED on mainnet launch (owner gate — the fix hunt, the after bench, the upstream PR, the marketing tie-in).

Source: https://github.com/noahdunnagan/fsearch — whole-disk file search for macOS (fuzzy names, typo tolerance, indexed content grep, ~1 ms over 8M files). 1.6k stars, MIT, 136 commits, Rust, benchmarked upstream (pgo.sh, README before/now tables, `examples/rig.rs` + `demo/abba.py`).

Owner directive (2026-10-11): this is a **bench-refine-pr** candidate — the outbound fix-PR campaign (Proposal 022) with a marketing tie-in: a before/after bench SVG that mentions refine.riir.ai, to show the fix and the bench tool to the repo's devs. The "before" bench is THIS session's work; the fix and the "after" refine wait for mainnet.

## The distill verdict (T1 — recorded here; MARGINAL sources take no research note per the distill skill)

**Verdict: MARGINAL D+ corpus / no league-lane mapping / OUTBOUND-PR TARGET HIGH.** The plan exists by owner directive (the PR lane), not by corpus grade.

- **Domain routing:** `rust_perf` would be the target (CPU search engine), `clippy_lints` the mechanical lane — **both measured EMPTY on this source**: `cargo clippy --workspace --all-targets --message-format=json` over the pin returns **ZERO findings** (the tree is clippy-clean), so there is nothing mechanical to fix and no positive mining surface for new clippy rules.
- **Corpus candidates — all positive instances of existing corpus law, none mineable** (dedup gate: these ARE the corpus): `vec-with-capacity` (pre-sized Vecs throughout build), branchless case fold (`fold()` = the b|bit trick our LUT-vs-branchless family teaches), NEON-masked fuzzy matching (aarch64 64-byte name masks — beyond our simd family's depth), galloping search + partition_point (binary-search family), bloom filters + varint + trigram posting lists, Fx hashing, binning/branch-and-bound ranked search, PGO in CI (pgo.sh). Grading a rule from these would be mining the corpus's own laws from a better teacher.
- **GOAT test: no candidate** — nothing new passes the four tests; recorded per the two-class split (nothing unbenchable-but-novel either: the source invents past our corpus, it does not lag it).
- **Lane-intel axis:** no mapping — file search does not touch the league lanes (llama.cpp/Metal/CUDA decode-prefill). No league line owed.
- **Product-coverage axis:** not ecosystem infrastructure (not tokio/rayon/serde class); the coverage value here is the OUTBOUND axis — a real-world, perf-literate, 1.6k-star repo as the campaign's first measured PR.
- **One tool-fired finding (evidence, not a rule):** our `dist` domain's `cli-panic-unwind-tables` fired on fsearch's `[profile.release]` (no `panic = "abort"`). Suggest-only; behavior-changing for a threaded daemon (abort kills the process instead of the panicking thread) — weak PR material alone; recorded as proof the tool fires on real external code.
- **Inspection honesty:** read = query.rs hot core (fold/is_subseq/class/bonus/fuzzy_* + name_hit + Searcher.search/ranked), index.rs (build/interning/prefault), content.rs (search/candidates/gallop/or_bits), live.rs (Flat overlay), walk.rs (getattrlistbulk header + scan), engine.rs (search/grep driver), examples/rig.rs + e2e.rs headers, main.rs args, pgo.sh. Unread at verdict time (re-read at fix time): server.rs, fsevents.rs, query.rs tail (2478+), most of content.rs's 3162 lines. Pin: `b88f4e9a90703eb7338085b325c2852fcaf131a9` (MIT) — quotes re-verified at mine time against this sha.

## The bench fixture (T2 — frozen, both arms MUST use these bytes)

- Snapshot: `/Volumes/SDXC1TB/tmp/fsearch-bench/snap/index.bin` — blake3 `50eeff298ccc348f76c35f07882c1988b3649a9c43055713c26676c9a41f99de`; **4,019,254 entries / 2,531,147 unique names / 291 MB** over `/Users/katopz/git` (real dev workspace, scanned in 24.9 s at the pin).
- Query corpus: `/Volumes/SDXC1TB/tmp/fsearch-bench/corpus.json` — blake3 `db232868d6d3b4ef0ee22782ec83e50dc96b62779031f0ba90162dd2cf5f0fb1`; 200 name queries + 10 typing runs, sampled by the **stock rig method** (same xorshift seed `0x9E37_79B9_7F4A_7C15`, same draw order — names/typing are drawn before the content section, so the queries are byte-identical to what `rig gen` produces; `grep: []` because the scoped snapshot carries no content index).
- Harness sidecars (preserved in `.benchmarks/112_fsearch_outbound_pr_bench/`): `bench_scope.rs` (scoped index build — the stock daemon always crawls `/` via `walk::scan(b"/", ..)` and stalled >12 min on this box without Full Disk Access; the harness scans the bounded root instead) + `gen_names.rs` (the name-side corpus generator). Untracked additions to the clone; NO upstream byte modified. Re-clone at the pin and restore both post-mainnet.
- Kinds benched: `name,idle,typing` (no grep — content index absent).

## Before bench (T3 — landed, `.benchmarks/112_fsearch_outbound_pr_bench/`)

- Protocol: quiet-box window only (1-min load < 8, disclosed beside every run); 3 back-to-back `rig run` invocations; publish the clean-window run; after arm must prove **identical result digests** against before (the rig fingerprints every answer).
- Published before (run q1, load 6.37→6.18 stable, M3 Max Mac15,8, AC power): name exact **206 µs** · typo **192 µs** · word **262 µs** · multi **695 µs** · filter **469 µs** · typing (whole filename, all keystrokes) **7.49 ms** — geomean of per-query medians, 3 rounds each.
- Load-sensitivity measured (why the protocol exists): at load 32–40 the same rig reads 0.86–0.94 ms exact (2.8–4.5× degraded); two more runs at load 7.9–9.0 read 2.2–2.3×. The after arm re-runs under the SAME window rule.

## Tasks

- [x] T1 — distill verdict + dedup greps + GOAT two-class test + lane/product axes (this file's verdict section; queue-snapshot line landed the same commit).
- [x] T2 — bench fixture: scoped index (bench_scope.rs), frozen APFS snapshot on SDXC1TB, corpus via the rig's own sampling (gen_names.rs), blake3 both, harness sidecars into the record dir.
- [x] T3 — quiet-box before bench (before_q1.json) + `scripts/fsearch_pr_bench_svg.py` (before/after renderer, refine.riir.ai footer) + `bench_before.svg` (after pending, marked).
- [x] T4 — `.benchmarks/112` record + this plan + the queue line; `.raw/fsearch` clone removed at the commit per the ephemeral-clone law (pin + sidecars carried).
- [-] T5 — **GATED: mainnet launch** — the fix hunt at the pin: re-clone, restore harness, profile the pinned build under the rig (sample/instruments + their pgo.sh flow), pick ONE real, honest win (candidate leads from the verdict read: whole-`/` no-FDA crawl stall — reproduces here, may be our box, verify on theirs first; typing-run batch path (7.49 ms / 10 runs = ~750 µs per keystroke is the fattest row); multi/filter query paths; mmap prefault strategy on cold start). Implement upstream-shaped (their style: dense, commented, no new deps); `rig run` digests byte-identical.
- [-] T6 — **GATED: T5 landed** — after bench: same snapshot (verify blake3), same window rule (load < 8, 3 runs, publish clean run), after.json + digests identical + the after SVG (before/after bars + per-row speedup). Honest bar: only chart rows that move ≥ measurement noise; rows that don't move keep their before value.
- [-] T7 — **GATED: mainnet + T6 ≥ noise on ≥1 row** — the upstream PR: fork, branch, minimal diff, the SVG in the PR body, one attribution line per Proposal 022's recorded posture (measured perf deltas allowed for bench-measured rules; logo + tool-name + site attribution allowed; owner-approved sends → owner account). The PR text leads with THEIR harness and the identical-digests proof, refine.riir.ai mentioned once as the bench/fix tool, no numbers for anything NotBenchable.
- [-] T8 — **GATED: T7 opened** — marketing tie-in close-out: refine.riir.ai bench page row (the site renders the same SVG), queue-snapshot close-out line with the PR URL, AGENTS.md line if the campaign repeats, HISTORY.md record.

## Gates (bind T6/T7)

- G1 correctness: rig result digests byte-identical before vs after on every query (the rig's own fingerprint).
- G2 perf: a row counts as moved only beyond the run-to-run spread measured at the same window class (before spread: quiet runs within ~5% at load ≤ 6.4; anything inside that is "no move" — chart it honestly).
- G3 no-regression: no other row may degrade beyond the same spread (check ALL rows, not just the fixed one).
- G4 provenance: box state beside every number (model, power, load before/after), the same as this record.

## Cautions

1. The whole-disk crawl stall (no-FDA, `walk::scan(b"/")`) is OUR box's observation until reproduced upstream — do NOT put it in the PR without an upstream reproduction; it may be a TCC/consent-path interaction specific to this machine.
2. fsearch's author is perf-literate (NEON, PGO, branch-and-bound) — a shallow "fix" would burn the campaign's credibility. T5's bar is a mechanism-level win their own README table would print, or no PR.
3. The snapshot is APFS-cloned from ~/git content: if the SDXC fixture is ever lost, a rebuilt index is NOT the same fixture (different blake3) — both arms would need re-running, not mixing.
4. The `dist` finding (`panic = "abort"`) is behavior-changing for a daemon — never auto-apply; upstream-decline likely; keep it out of the PR unless the owner wants it argued.
