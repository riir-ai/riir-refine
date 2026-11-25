// Batch 416 — the uutils/coreutils 0.13.0 perf transcription batch (Plan 405 /
// Research 440). Pin tag 0.13.0 = 07b76df463390fd56856725201ae52df5df0a063
// (2026-11-19), MIT — outbound-PR lane OPEN; base 0.12.0 = 5ab52189443867d56e8.
// SIX rules: R2 (complexity, BENCHABLE — the materialization-cost cell passed,
// k=1 bar 0.383, crossover k=8 measured), R5 (complexity, BENCHABLE —
// ParetoDominant 71x/4392x/25545x all-adjacent), R6 (layout, BENCHABLE —
// out-of-line 0.838/0.838 reproduced across rustc 1.98.1 AND 1.99.0), R3
// (complexity, decision-only NotBenchable), R4 (allocation, NotBenchable —
// peak-set arithmetic fixture-instrumented exactly: 2000-depth vs 3500-N),
// R7 (io, NotBenchable — metadata-call count fixture-instrumented exactly:
// 8 -> 3). R1 was the A- GOAT lead and DEMOTED at its own G2 cell (the
// pre-registered falsifier fired: the tiered form was SLOWER than the best
// single arm at 18/21 density x size cells, tier/best 1.28-1.80 — the scalar
// word-probe gate pass costs the dispatch, the upstream win is REAL SIMD
// count; demoted to a FACE on `prefilter-skip-density-demotion` in the
// landing commit, the ungoat law, measured negative recorded in that face).
//
// Dedup re-verified at the landing HEAD (3+ vocabularies per row, the
// Batch-409 lesson): `sort_by_cached_key` ZERO owners re-grepped (R2's
// counterpart still unowned); ancestor/exit-marker ZERO owners (entries_34/39
// "ancestor" hits are commit-ancestry prose, entries_51 owns the depth axis
// only); interior-remove dedup_by ZERO owners; out-of-line inline(never)
// entries_03/33 are dispatch-stability arms, not the caller-state regime;
// symlink_metadata/stat-once ZERO owners (the e02/e05/e06/e08/e10 hits are
// Regex-once, log-gating, reserve-then-commit metadata — different axes);
// formatter-per-item-String: the e02/e06 owners are push_str(&format!) and
// log-gating — the od residue lands as a face on log-write-direct.
//
// FACES (count-neutral riders on their owners, same commit):
// - R1-atoms face on entries_30 `prefilter-skip-density-demotion` (the
//   demote target: exact-count tier before the work, zero-hit borrow,
//   carried per-block guess — with OUR measured negative).
// - od per-formatter String face on entries_02 `log-write-direct`.
// - closed-set-enum positive instance on entries_01 `dyn-to-enum` (ls).
// - HashSet->FxHashSet positive instance on entries_01 `fxhashmap` (ls.rs
//   listed_ancestors is FxHashSet at the pin).
// - cold-error-ctor positive instance on entries_31 `match-branch-cold-confirm`.
// - fill_buf face on entries_30 `bufread-scan-in-buffer-scratch-fragment`.
// - coalesce-write face on entries_02 `bufwriter-batched`.
// - plain-run face on entries_65 `byte-flag-table-masked-run-scan`.
//
// T2b rode this batch: the `sort-by-key` rule (entries_02) re-worded to the
// true std mechanism (the old name/comment claimed sort_by_key caches keys —
// false; caching is sort_by_cached_key's job).
//
// Fixture preflight: b416_perf_fixtures.rs 9/9 arms all-ok at debug AND -O
// BEFORE entry (the gate caught FOUR arm defects: the tiered arm's dense
// fallback cleared the accumulator wiping prior blocks; the R2 dear key was
// an FNV HASH — not order-preserving — rebuilt as the padded collation key;
// the R4 peak-vs-N assert needed a fan graph to separate depth from N; the
// R7 symlink double-follow count is 8 not 6). G2 cells measured on m3 macOS
// aarch64, rustc 1.99.0 + 1.98.1, quiet box (load ~2-3), scratch deleted at
// teardown (plan T7).

use crate::domains::rust_perf::PerfRule;
#[cfg(feature = "rust_perf_bench")]
use crate::domains::rust_perf::bench::BenchSpec;

pub fn b416_rules() -> Vec<PerfRule> {
    vec![
        // ── R2: lazy key when comparisons per item are low (complexity) ──
        PerfRule {
            rule_id: "sort-key-lazy-when-comparisons-per-item-low".to_string(),
            name: "precompute a DEAR sort key only where comparisons per element amortize it — a k-way merge is ~n·log k comparisons (k=1: ZERO), so compare by line and never build keys; the full sort's O(n log n) is the amortize-able case (std's own sort_by_cached_key guidance)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "struct Line { raw: Vec<u8>, key: Vec<u8> }\n\nfn merge_two_streams(a: Vec<Line>, b: Vec<Line>, out: &mut Vec<Vec<u8>>) {\n    // materializing a collation key for EVERY element of a merge that\n    // performs ~n comparisons (k=2) — and ZERO when merging one stream —\n    // is pure overhead: the merge compares by line directly\n    let mut keyed: Vec<_> = a.into_iter().chain(b).map(|mut l| { l.key = collation_key(&l.raw); l }).collect();\n    keyed.sort_by(|x, y| x.key.cmp(&y.key));\n    out.extend(keyed.into_iter().map(|l| l.raw));\n}".to_string(),
            fixed_code: "fn merge_two_streams(runs: &[Vec<Vec<u8>>], out: &mut Vec<Vec<u8>>) {\n    // k-way merge over pre-sorted runs: ~n·log k line comparisons, no key\n    // ever built; k = 1 is a pass-through with zero comparisons\n    let k = runs.len();\n    let mut cursors = vec![0usize; k];\n    loop {\n        let mut best: Option<usize> = None;\n        for (ri, &cur) in cursors.iter().enumerate() {\n            if cur < runs[ri].len() {\n                best = match best {\n                    None => Some(ri),\n                    Some(b) => {\n                        if runs[ri][cur] < runs[b][cursors[b]] { Some(ri) } else { Some(b) }\n                    }\n                };\n            }\n        }\n        match best {\n            Some(ri) => { out.push(runs[ri][cursors[ri]].clone()); cursors[ri] += 1; }\n            None => break,\n        }\n    }\n}".to_string(),
            context_notes: Some("MEASURED verdict-stable (m3 macOS aarch64, rustc 1.99.0, -O, quiet box load ~2.5, 2026-11-25; min-of-9 interleaved): k=1 lazy/pre = 0.383 (the pre-registered k=1 bar <= 0.95 PASSED — 8192 key allocations vs 1) with the crossover MEASURED at k=8 = 1.212 (a linear-pick lazy merge loses past the crossover; a heap merge shifts it — the regime line is comparisons-per-element x key cost vs key materialization). Measured verdict class: Mixed — the lazy arm wins its regime (k=1..4 on the fixture) and loses past the measured crossover; the crossover IS the rule's regime content, honestly measured both directions. Primary axis = key MATERIALIZATION cost (allocations + bytes built vs bytes the comparisons inspect); a count-based bar is ARITHMETICALLY FORBIDDEN (a lazy k-way heap merge at k>=2 does ~n·log2 k comparisons x 2 key reads >= n, so a count bar fires for the wrong reason; k=1 does exactly zero). SOURCE (uutils/coreutils tag 0.13.0 @ 07b76df463, MIT, src/uu/sort/src/sort.rs:2773-2775 verbatim): 'Merging only performs O(n log k) comparisons (and none at all when merging a single file), so computing a full sort key for every line — as the regular sort path does to amortize O(n log n) comparisons — is pure overhead here.' Result identity in-source (:2794): 'locale_cmp (ICU compare_utf8) and the sort-key comparison agree on ordering by construction.' The REGULAR path's create (sort.rs:689-702, read at the pin) materializes keys into a shared collation_key_buffer at Line::create — the amortize-able case this rule's regime keeps. THE GENUINE PRO-COMPUTE COUNTERPART is std's sort_by_cached_key — grep-verified ZERO corpus owners at the landing HEAD (re-run at claim, the Batch-409 lesson); adjudicated NOT against sort-key-cache-precompute-out-of-lock (entries_04: that owns lock+clone INSIDE a comparator, not the comparisons-per-element axis) and NOT against the misnamed sort-by-key (entries_02 — T2b re-worded it this same landing: std sort_by_key calls the key fn per comparison; its own Bench-016 notes measured 4/4 Tie). Dedup: 'lazy sort key|precomput|sort_by_cached_key' over entries_*.rs = zero owners. Fixture b416_perf_fixtures.rs F4: order identity k=1..=4 (ties first-index both arms), the k=1 zero-comparison law asserted exactly; the dear key is the PADDED COLLATION KEY (order-preserving by construction — the first draft's FNV hash was NOT order-preserving and the G1 identity caught it). Regime: k-way merges, early-exit scans, any pipeline whose comparisons per element are small relative to key cost. ANTI-REGIMES: the full sort path (comparisons per element ~ log n — precompute wins, sort_by_cached_key's own case); cheap Copy keys (no materialization to avoid); keys with a consumer beyond comparison. UPSTREAM: present at the pin (their landed decision — decision content only, never a PR). Batch 416 / Plan 405 / Research 440.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(sort_key_lazy_bench_spec()),
        },
        // ── R5: in-loop interior remove → single-pass fold (complexity) ──
        PerfRule {
            rule_id: "in-loop-interior-remove-single-pass-fold".to_string(),
            name: "ordered interior coalescing via Vec::remove in the loop (quadratic tail shifts) → one sort + one fold pass where each successor merges into its predecessor (the overlap test IS the decision content)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "fn absorb(rs: &mut Vec<Range>, i: usize) {\n    // each absorption shifts the whole tail: heavy coalescing is O(n^2)\n    while i + 1 < rs.len() {\n        if rs[i].low <= rs[i + 1].high && rs[i].high >= rs[i + 1].low {\n            rs[i].high = rs[i].high.max(rs[i + 1].high);\n            rs.remove(i + 1);\n        } else {\n            break;\n        }\n    }\n}".to_string(),
            fixed_code: "fn coalesce(mut rs: Vec<Range>) -> Vec<Range> {\n    rs.sort_unstable_by_key(|r| r.low);\n    let mut folded: Vec<Range> = Vec::with_capacity(rs.len());\n    let mut it = rs.into_iter();\n    if let Some(first) = it.next() {\n        let mut cur = first;\n        for r in it {\n            if cur.low <= r.high && cur.high >= r.low {\n                cur.high = cur.high.max(r.high);\n            } else {\n                folded.push(cur);\n                cur = r;\n            }\n        }\n        folded.push(cur);\n    }\n    folded\n}".to_string(),
            context_notes: Some("MEASURED ParetoDominant verdict-stable (m3 macOS aarch64, rustc 1.99.0, -O, quiet, 2026-11-25; min-of-9): 71x / 4392x / 25545x at n = 1000 / 16384 / 262144 on all-adjacent input (243.6us vs 3.4us / 201.1ms vs 45.8us / 8.707s vs 340.8us — the quadratic tail shift is 8.7 SECONDS at 262K where the fold is 341us). The full closure (completed at verdict round 1 — the overlap test is the decision content): dedup_by(|a, b| { if a.low <= b.high { b.high = max(b.high, a.high); true } else { false } }) — each successor folds into its predecessor in one pass, unmodified pairs survive. SOURCE (uutils/coreutils tag 0.13.0 @ 07b76df463, MIT, src/uucore/src/lib/features/ranges.rs:240-249 read at the pin: merge() sorts by low then dedup_by with exactly the overlap test above — 'a.low <= b.high' folds, else survives). Fixture b416_perf_fixtures.rs F6: merged-output identity over touching / nested / disjoint shapes; the remove-loop's tail-shift count 1,999,000 at n=2000 all-adjacent (quadratic) vs the fold's zero; the disjoint-heavy ANTI-REGIME measured as the expected tie (zero merges either arm — recorded, never a failure). ANTI-REGIMES: disjoint-heavy streams (nothing coalesces — the tie is the honest outcome); removal that IS unordered (swap-remove owns that axis) or FRONT-biased (VecDeque owns that axis); single-element removal (O(1) either way). Dedup re-run at the landing HEAD: 'Vec::remove|quadratic|dedup_by|single pass' — the front owner (vec-remove-front-to-vecdeque, entries_02) and the unordered owner (swap-remove, entries_01) hold their axes; the ORDERED INTERIOR axis has no other owner. UPSTREAM: present at the pin. Batch 416 / Plan 405 / Research 440.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(interior_fold_bench_spec()),
        },
        // ── R6: opaque call out of line protects caller hot state (layout) ──
        PerfRule {
            rule_id: "opaque-call-out-of-line-protects-caller-hot-state".to_string(),
            name: "an opaque (opaque-to-the-optimizer) call inside a hot per-byte loop spills the loop's live state to memory on every iteration — move the per-byte work into its own #[inline(never)] stage so the caller's state stays in registers (the ALWAYS-taken opposite regime of the rare-branch cold-confirm law)".to_string(),
            category: "layout".to_string(),
            buggy_code: "fn translate(input: &[u8], sink: &mut Vec<u8>) {\n    let mut state = 0u64; // hot: live across every iteration\n    for &b in input {\n        state = state.wrapping_mul(31).wrapping_add(b as u64);\n        opaque_write(sink, b.wrapping_add(state as u8)); // opaque call IN the loop:\n        // the optimizer cannot see through it, so `state` and the loop cursor\n        // go to memory around every call\n    }\n}".to_string(),
            fixed_code: "fn translate(input: &[u8], sink: &mut Vec<u8>) {\n    let mut state = 0u64;\n    for &b in input {\n        translate_stage(&mut state, b, sink); // the stage is #[inline(never)] too:\n        // the loop body is one call with a stable ABI; `state`'s home is the\n        // callee's frame, the caller's loop stays a register walk\n    }\n}\n\n#[inline(never)]\nfn translate_stage(state: &mut u64, b: u8, sink: &mut Vec<u8>) {\n    *state = state.wrapping_mul(31).wrapping_add(b as u64);\n    opaque_write(sink, b.wrapping_add(*state as u8));\n}".to_string(),
            context_notes: Some("MEASURED, TWO-RUSTC REPRODUCTION (the pre-registered fragility bar — a win that does not reproduce across two rustc versions demotes to a face): out-of-line/inlined = 0.838 on rustc 1.99.0 AND 0.838 on rustc 1.98.1 (m3 macOS aarch64, -O, n=262144, min-of-11 interleaved: 502.1us vs 421.0us and 502.5us vs 421.1us) — a reproducible ~16% win on this box/toolchain pair, recorded WITH the versions — the out-of-line arm faster at EVERY measured cell (2 sizes x 2 rustc versions), the ParetoDominant posture at the measured surface; the win is compiler-dependent by nature and the two-version cell is the landing bar (other toolchains re-measure before citing). SOURCE (uutils/coreutils tag 0.13.0 @ 07b76df463, MIT, src/uu/tr/src/simd.rs:147-148 verbatim): 'Kept out of line: inlined into translate_input, the raw write made the translator state go to memory on every byte, doubling the time of tr -s.' (the 'doubling' is the author's unquantified claim — PROVENANCE, never a bar; our cell measured 1.19x the other direction). RIDER (6d6f830dc, same pin): 'Work on a local copy of the line state, so it can stay in registers.' ADJUDICATION: match-branch-cold-confirm (entries_31) owns the RARE heavyweight branch (#[cold] + inline(never) to keep the hot path's rare arm out of line and the predictor cold) — THIS is the ALWAYS-taken opposite regime: the opaque call is on every iteration and the law is the CALLER's live state vs the ABI, never branch frequency; compose on a loop carrying both a hot opaque call and a rare error arm. Fixture b416_perf_fixtures.rs F7: output identity inlined vs out-of-line. Regime: hot per-element loops whose body calls an opaque/uninlinable fn (raw writes, FFI, fmt machinery) while keeping loop-carried state live. ANTI-REGIMES: inlinable callees (LLVM keeps registers anyway); loop-carried state the callee legitimately needs by pointer (the fix moves it, never duplicates the semantics); cold rare calls (the entries_31 regime). UPSTREAM: present at the pin. Batch 416 / Plan 405 / Research 440.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: Some(opaque_ool_bench_spec()),
        },
        // ── R3: chain-follow depth cap, not a visited set (complexity, decision-only) ──
        PerfRule {
            rule_id: "chain-follow-depth-cap-not-visited-set".to_string(),
            name: "a link chain that GROWS (never revisiting state) cannot be stopped by a visited set — cap the hops (a fixed limit treated as a loop, kernel-style) instead of tracking visited targets (the cap also skips a PathBuf clone + hash insert per step)".to_string(),
            category: "complexity".to_string(),
            buggy_code: "use std::path::{Path, PathBuf};\nuse std::collections::HashSet;\n\nfn follow(mut path: PathBuf, parent: &Path) -> std::io::Result<PathBuf> {\n    let mut visited: HashSet<PathBuf> = HashSet::new();\n    loop {\n        let target = std::fs::read_link(&path)?;\n        path = parent.join(target); // the chain GROWS: foo -> foo/bar -> ...\n        if !visited.insert(path.clone()) {\n            return Err(std::io::Error::new(std::io::ErrorKind::Other, \"cycle\"));\n        }\n    }\n}".to_string(),
            fixed_code: "use std::path::{Path, PathBuf};\n\nconst FOLLOW_LIMIT: usize = 40;\n\nfn follow(mut path: PathBuf, parent: &Path) -> std::io::Result<PathBuf> {\n    for _ in 0..=FOLLOW_LIMIT {\n        if !std::fs::symlink_metadata(&path)?.file_type().is_symlink() {\n            return Ok(path);\n        }\n        path = parent.join(std::fs::read_link(&path)?);\n    }\n    // like the kernel: a chain longer than the limit IS a loop\n    Err(std::io::Error::new(std::io::ErrorKind::FilesystemLoop, \"too many links\"))\n}".to_string(),
            context_notes: Some("DECISION-ONLY, NotBenchable (bench_spec: None — the domain-unbenchable annotation class; the win is correctness-on-growing-chains + the dropped per-step PathBuf clone + hash insert, priced on a real filesystem's link topology). SOURCE (uutils/coreutils tag 0.13.0 @ 07b76df463, MIT, src/uucore/src/lib/features/fs.rs:427 verbatim): 'A fixed cap rather than tracking visited links: loops such as foo -> foo/bar never revisit the same state.' And (:791): 'Like the kernel, a chain longer than [SYMLINK_FOLLOW_LIMIT] is treated as a loop.' SYMLINK_FOLLOW_LIMIT = 40 VERIFIED at the pin (fs.rs:45; the pre-change constant was 20 — the first draft wrongly carried it, corrected at verdict round 1). T0 body reads at the pin: Line::create confirms the R2 framing; map_err_context (error.rs:516-537) is a plain lazy-context mapper — no candidate, struck; the od per-formatter String residue CONFIRMED (every formatter fn returns a fresh String per item) and landed as a face on log-write-direct. ANTI-REGIME carried honestly: a cap cannot tell a long ACYCLIC chain from a cycle — it cuts both at LIMIT (fixture b416_perf_fixtures.rs F9 walks a 500-link growing chain: the visited set traverses ALL 500 and never fires; the cap stops at 40). Dedup re-run at the landing HEAD: 'symlink|visited set|follow limit|growing chain' — zero owners (the one grep hit irrelevant). Regime: any self-referential chain walk (symlinks, delegating configs, redirect chains). UPSTREAM: present at the pin. Batch 416 / Plan 405 / Research 440.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R4: explicit-stack DFS ancestor set exit marker (allocation) ──
        PerfRule {
            rule_id: "explicit-stack-dfs-ancestor-set-exit-marker".to_string(),
            name: "a DFS cycle-detection set that is insert-only holds EVERY visited node for the whole run — push an Exit marker so each node's id leaves the set on the way out and the set holds only the CURRENT chain (peak size = depth, not N)".to_string(),
            category: "allocation".to_string(),
            buggy_code: "fn walk(root: usize, nodes: &[(DirId, Vec<usize>)]) {\n    let mut listed = FxHashSet::default(); // insert-only: never shrinks\n    let mut stack = vec![root];\n    while let Some(i) = stack.pop() {\n        let (id, kids) = &nodes[i];\n        if !listed.insert(*id) {\n            continue; // cycle\n        }\n        for &c in kids.iter().rev() {\n            stack.push(c);\n        }\n    }\n    // `listed` now holds EVERY node ever visited — on a deep walk past a\n    // finished subtree, revisits falsely report \"already listed\"\n}".to_string(),
            fixed_code: "enum Frame { Enter(usize), Exit(DirId) }\n\nfn walk(root: usize, nodes: &[(DirId, Vec<usize>)]) {\n    let mut listed = FxHashSet::default();\n    let mut stack = vec![Frame::Enter(root)];\n    while let Some(f) = stack.pop() {\n        match f {\n            Frame::Exit(id) => { listed.remove(&id); } // the ancestor leaves\n            Frame::Enter(i) => {\n                let (id, kids) = &nodes[i];\n                if !listed.insert(*id) { continue; }\n                stack.push(Frame::Exit(*id));\n                for &c in kids.iter().rev() {\n                    stack.push(Frame::Enter(c));\n                }\n            }\n        }\n    }\n    debug_assert!(listed.is_empty()); // only the current chain ever lingers\n}".to_string(),
            context_notes: Some("NotBenchable-by-domain (bench_spec: None — the win metric is PEAK SET SIZE (memory), priced on real directory topologies; the fixture instruments the arithmetic EXACTLY instead: b416_perf_fixtures.rs F5 asserts order identity + cycle-verdict identity + peak 2000 (depth) vs 3500 (N) on a 2000-chain + 1500-leaf fan with a cycle re-entry — the first draft's pure-chain graph could not separate depth from N (both were 2000) and the gate caught it). SOURCE (uutils/coreutils tag 0.13.0 @ 07b76df463, MIT, src/uu/ls/src/ls.rs — read at the pin: the insert-only set was holding every directory listed and reporting false 'already listed' on any revisit; StackItem::Exit(DirId) removes the id on exit; debug_assert!(listed_ancestors.is_empty()) at :1267; the FxHashSet at :1247 is also this batch's positive-instance face on fxhashmap). THE FALSE-POSITIVE BUG IS THE DECISION CONTENT: an insert-only set does not just waste memory — it MIS-CLASSIFIES a revisit of a finished subtree as a cycle. ADJUDICATION: recursive-input-walk-explicit-stack-indices-not-copies (entries_51) owns the EXPLICIT-STACK-indices-vs-path-copies axis (the depth-LIMIT law); THIS owns the ancestor-SET-LIFETIME axis (what the set holds: the current chain, not the run) — compose on the same walk. Dedup re-run at the landing HEAD: 'ancestor|exit marker|visited' over entries_*.rs — the entries_34/39 'ancestor' hits are commit-ancestry prose, zero owners on the set-lifetime axis. Regime: any explicit-stack walk with a visited/ancestor set (trees, graphs, package dep walks). ANTI-REGIMES: walks that WANT the global listed-set (a directory-once listing is exactly ls's own OTHER semantics — the fix is for cycle detection, not dedup-of-output); iterative BFS with a level set (no exit order to exploit). UPSTREAM: present at the pin. Batch 416 / Plan 405 / Research 440.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
        // ── R7: stat-once per path, consumers share the result (io) ──
        PerfRule {
            rule_id: "stat-once-per-path-consumers-share-the-result".to_string(),
            name: "one stat classifies the file AND later feeds its size/mode consumers — classify with symlink_metadata() and share the result; a link-following classifier call (is_dir()) beside a second metadata() is a duplicate syscall per path, and is_dir() DESCENDS through symlink cycles".to_string(),
            category: "io".to_string(),
            buggy_code: "fn scan(path: &Path) -> Option<Entry> {\n    let is_dir = path.is_dir();          // stat #1 (follows symlinks)\n    let meta = fs::metadata(path).ok()?; // stat #2 (follows again)\n    Some(Entry { is_dir, len: meta.len(), mode: meta.permissions() })\n    // a symlink cycle makes BOTH calls descend forever\n}".to_string(),
            fixed_code: "fn scan(path: &Path) -> Option<Entry> {\n    // ONE stat, non-following: classifies AND carries len/mode for every\n    // later consumer; symlinks report AS symlinks (no descent, no cycles)\n    let meta = fs::symlink_metadata(path).ok()?;\n    Some(Entry { is_dir: meta.is_dir(), len: meta.len(), mode: meta.permissions() })\n}".to_string(),
            context_notes: Some("NotBenchable-by-domain (bench_spec: None — the win metric is SYSCALL COUNT on a real filesystem; the fixture instruments the count arithmetic EXACTLY instead: b416_perf_fixtures.rs F8 asserts 8 -> 3 metadata calls on a 3-path walk — the link-following double stat follows the symlink TWICE, so the naive arm pays 2 per plain path and 4 on the symlink). SOURCE (uutils/coreutils tag 0.13.0 @ 07b76df463, MIT): shred.rs:691 verbatim 'One stat classifies the file and later gives its size and mode. A FIFO' (the size/mode consumers share the classify stat); ls.rs:792 'Result<MetaData> got from symlink_metadata() or metadata() based on config' (one stat, the config picks the kind); mv/hardlink.rs:89 + :183 symlink_metadata() as the classify call. THE CORRECTNESS HALF (ls.rs:929 + :1625 at the pin): path.is_dir() FOLLOWS symlinks, so the scanner 'descended through link cycles' — classify with symlink_metadata(), never a link-following call, and the walk terminates. Dedup re-run at the landing HEAD: 'stat.*once|metadata memo|symlink_metadata|lstat' — ZERO owners on the metadata axis (the nearest kin persistent-digest-memo-stat-identity-keyed, entries_36, is disk-keyed DIGESTS; the e58 sccache metadata() hit is the reserve-then-commit re-measure, a different axis); the stat-once family has no owner before this rule. Regime: filesystem walks that classify then read size/mode (scanners, backups, du-class tools, dedup indexes). ANTI-REGIMES: callers who genuinely need the LINK-FOLLOWING view (then ONE fs::metadata() shared is the same law — the axis is the COUNT, not the follow choice); paths already stat-ed by the caller (thread the Meta through, do not re-stat). UPSTREAM: present at the pin. Batch 416 / Plan 405 / Research 440.".to_string()),
            #[cfg(feature = "rust_perf_bench")]
            bench_spec: None,
        },
    ]
}

#[cfg(feature = "rust_perf_bench")]
fn sort_key_lazy_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![8_192, 32_768, 131_072],
        harness: r#"
struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

const KEY_WIDTH: usize = 64;

fn make_sorted_runs(k: usize, per_run: usize, seed: u64) -> Vec<Vec<Vec<u8>>> {
    let mut xs = Xs(seed | 1);
    let mut runs = Vec::with_capacity(k);
    for _r in 0..k {
        let mut run: Vec<Vec<u8>> = Vec::with_capacity(per_run);
        for _ in 0..per_run {
            let len = 8 + (xs.next() % 56) as usize;
            run.push((0..len).map(|_| (xs.next() % 26) as u8 + b'a').collect());
        }
        run.sort();
        runs.push(run);
    }
    runs
}

/// buggy: materialize a padded key per element, then merge by key
fn run_buggy(runs: &[Vec<Vec<u8>>]) -> Vec<Vec<u8>> {
    let mut keyed: Vec<(Vec<u8>, usize, usize)> = Vec::new();
    for (ri, run) in runs.iter().enumerate() {
        for (i, line) in run.iter().enumerate() {
            let mut k = vec![0u8; KEY_WIDTH];
            let w = line.len().min(KEY_WIDTH);
            k[..w].copy_from_slice(line);
            keyed.push((k, ri, i));
        }
    }
    if runs.len() > 1 {
        keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    }
    keyed.into_iter().map(|(_, ri, i)| runs[ri][i].clone()).collect()
}

/// fixed: k-way merge by line, zero key builds
fn run_fixed(runs: &[Vec<Vec<u8>>]) -> Vec<Vec<u8>> {
    let k = runs.len();
    let mut cursors = vec![0usize; k];
    let mut out = Vec::new();
    if k == 1 {
        return runs[0].clone();
    }
    loop {
        let mut best: Option<usize> = None;
        for (ri, &cur) in cursors.iter().enumerate() {
            if cur < runs[ri].len() {
                best = match best {
                    None => Some(ri),
                    Some(b) => {
                        if runs[ri][cur] < runs[b][cursors[b]] { Some(ri) } else { Some(b) }
                    }
                };
            }
        }
        match best {
            Some(ri) => { out.push(runs[ri][cursors[ri]].clone()); cursors[ri] += 1; }
            None => break,
        }
    }
    out
}

pub fn make_input(n: usize) -> Vec<Vec<Vec<u8>>> {
    let k = 1usize.max(n / 8_192); // k=1 at the first size: the k=1 bar cell
    make_sorted_runs(k, 8_192, 42)
}

pub fn run_buggy_entry(input: &[Vec<Vec<u8>>]) {
    let out = run_buggy(input);
    std::hint::black_box(&out);
}

pub fn run_fixed_entry(input: &[Vec<Vec<u8>>]) {
    let out = run_fixed(input);
    std::hint::black_box(&out);
}
"#
        .to_string(),
    }
}

#[cfg(feature = "rust_perf_bench")]
fn interior_fold_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![1_000, 16_384, 262_144],
        harness: r#"
struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

#[derive(Clone, Copy)]
struct Range { low: u64, high: u64 }

fn make_ranges_all_adjacent(n: usize, seed: u64) -> Vec<Range> {
    let mut xs = Xs(seed | 1);
    let mut v = Vec::with_capacity(n);
    let mut low = 0u64;
    for _ in 0..n {
        let len = 1 + xs.next() % 4;
        v.push(Range { low, high: low + len });
        low += len;
    }
    v
}

fn run_buggy(input: &[Range]) -> Vec<Range> {
    let mut rs = input.to_vec();
    let mut i = 0;
    while i + 1 < rs.len() {
        if rs[i].low <= rs[i + 1].high && rs[i].high >= rs[i + 1].low {
            rs[i].high = rs[i].high.max(rs[i + 1].high);
            rs.remove(i + 1);
        } else {
            i += 1;
        }
    }
    rs
}

fn run_fixed(input: &[Range]) -> Vec<Range> {
    let mut rs = input.to_vec();
    rs.sort_unstable_by_key(|r| r.low);
    let mut folded: Vec<Range> = Vec::with_capacity(rs.len());
    let mut it = rs.into_iter();
    if let Some(first) = it.next() {
        let mut cur = first;
        for r in it {
            if cur.low <= r.high && cur.high >= r.low {
                cur.high = cur.high.max(r.high);
            } else {
                folded.push(cur);
                cur = r;
            }
        }
        folded.push(cur);
    }
    folded
}

pub fn make_input(n: usize) -> Vec<Range> {
    make_ranges_all_adjacent(n.max(2), 9)
}

pub fn run_buggy_entry(input: &[Range]) {
    let out = run_buggy(input);
    std::hint::black_box(&out);
}

pub fn run_fixed_entry(input: &[Range]) {
    let out = run_fixed(input);
    std::hint::black_box(&out);
}
"#
        .to_string(),
    }
}

#[cfg(feature = "rust_perf_bench")]
fn opaque_ool_bench_spec() -> BenchSpec {
    BenchSpec {
        sizes: vec![65_536, 262_144],
        harness: r#"
#[inline(never)]
fn opaque_write(sink: &mut Vec<u8>, b: u8) {
    if b == 0xFF {
        sink.push(0);
    }
    sink.push(b);
}

fn run_buggy(input: &[u8], sink: &mut Vec<u8>) {
    let mut state = 0u64;
    for &b in input {
        state = state.wrapping_mul(31).wrapping_add(b as u64);
        opaque_write(sink, b.wrapping_add(state as u8)); // opaque call IN the loop
    }
}

#[inline(never)]
fn translate_stage(state: &mut u64, b: u8, sink: &mut Vec<u8>) {
    *state = state.wrapping_mul(31).wrapping_add(b as u64);
    opaque_write(sink, b.wrapping_add(*state as u8));
}

fn run_fixed(input: &[u8], sink: &mut Vec<u8>) {
    let mut state = 0u64;
    for &b in input {
        translate_stage(&mut state, b, sink);
    }
}

pub fn make_input(n: usize) -> Vec<u8> {
    let mut xs = Xs(7);
    (0..n).map(|_| (xs.next() % 251) as u8 + 1).collect()
}

struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
}

pub fn run_buggy_entry(input: &[u8]) {
    let mut sink = Vec::with_capacity(input.len());
    run_buggy(input, &mut sink);
    std::hint::black_box(&sink);
}

pub fn run_fixed_entry(input: &[u8]) {
    let mut sink = Vec::with_capacity(input.len());
    run_fixed(input, &mut sink);
    std::hint::black_box(&sink);
}
"#
        .to_string(),
    }
}
