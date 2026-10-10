# Plan 221 — Batch 221: the rider-tier conversion set (Leads 9/10/11 + k=8-MMA)

**Status:** COMPLETE — landed (see the snapshot row + the commit for the gate numbers; plan tasks all [x]).

ONE session ONE verify cycle. Two count-neutral kernel_opt RIDERS + ONE rust_perf rule (195→196) + ONE dist rule (6→7).

## Sources (all fetched + read IN FULL at the pins this session, /tmp/b221/)

- **Lead 9** — ggml-org/llama.cpp @ `1f8fa52318b594b373e7447a3c2bb36cdba9515f` (#30209, MIT, merged 2026-10-09): Metal FA 128/96 kernels + the explicit (dk,dv) admission allowlist. 265-line .patch read; every staging-row quote verified verbatim + the NEW consent-triple datum (admission + ops sparse table + tuning row move together).
- **Lead 10** — bevyengine/bevy #25864 (patches `a4d1789badfe64fbd582a3b58525a93988fd90c0` + `8328184452b07def3ae670af95e2b8735f8c4b15`, merged 2026-09-28T07:07:36Z @ `74884ffd`): the runtime-bool exclusive-system fix. 360-line .patch read; chescock's review quote verified via the PR comments API. Landed shape is RICHER than the staging row: patch 1 used the branch-around-closure (the anti-shape itself), patch 2 LANDED the conditional-guard tuple + LastTickGuard drop impl + one `run` body.
- **Lead 11** — bevyengine/bevy #25321 (3 patches, merged 2026-10-05 @ `430c448d`): bench feature split. 366-line .patch read; PR body verified via the API. Resolution: 10 bench features ALL default-on, bench deps optional-normal, per-bench required-features.
- **k=8-MMA** — tracel-ai/cubecl @ `73cecbfc241e48fa174754ba299b790fb84e71e3` (#1802, Apache-2.0): 78-line .patch read (Research 250's re-verify precondition satisfied). Commit body + in-code comment + test doc verbatim.

## Tasks

- [x] Dedup re-greps at authoring (all four; Lead 13/15 collision lesson — verify UNLANDED at write time) — clean (the one grep hit was `off**set-mirror**`, a false positive)
- [x] Lead 9 rider → `entries_gpu_78.rs` B208 `fa-fast-path-validated-exact-shape-admission-v-as-strided-view` (gpu_concept append; count-neutral; the set-mirror + consent-triple atoms)
- [x] k=8 rider → `entries_gpu_09.rs` `mma-fragment-layout-m16n8k16` (gpu_concept append; count-neutral; the accumulator-as-A-fragment identity + the availability widening)
- [x] Lead 10 rule → `rust_perf/corpus/entries_23.rs` NEW (`runtime-bool-duplicates-inlinable-body`, dispatch, NotBenchable annotated; the conditional-guard landed form + the patch-1 anti-shape disclosure) + mod.rs wiring
- [x] Lead 11 rule → `dist/corpus.rs` 7th rule (`bench-targets-shared-dependency-cluster-hint`, BuildTime) + `dist/rules.rs` DIST_RULES + ManifestScan.benches + detector (fires ≥3 [[bench]] with NO required-features anywhere — the fixed form is gated so the fixture contract holds) + the boundary test
- [x] batch221 gate (kernel_opt corpus/tests.rs: rider needles on both hosts; rust_perf corpus/tests.rs: `batch221_rule_present_and_parse`) + dist fixture tests green by the existing contract
- [x] Count pins: rust_perf tests.rs 195→196 ×1 + annotated 141→142 ×3 sites (the two extra pin sites surfaced by the runs — the B211 pin-set-wider class) + NotBenchable 96→97 ×1 + categories 13 UNCHANGED; kernel_opt 784 UNCHANGED (riders); dist pins: corpus.rs doc + table, rules.rs DIST_RULES
- [x] Gates: default lib 2491/0; kernel_opt+latent_retrieval lib 3114/0 (1266 s, ONE full run); rust_perf_bench lib 2522/0 (4 ignored documented); latent_retrieval lib 2676/0 — retrieval floors HELD, no re-pin; clippy -D ×3 postures CLEAN (incl. 3 pre-existing doc_markdown mechanicals in the sibling's landed entries_gpu_84 healed — the B215 precedent)
- [x] freeze/score-bench adjudicated: NOT owed (zero clippy-corpus delta, P30)
- [x] Docs: AGENTS.md ×3 bullets + ×2 feature lines, README ×3 rows (rust_perf 196, kernel_opt rider note, dist 5→7 repairing the STALE-5 drift with honest label), .docs/11_domains/rust_perf.md (count + B221 paragraph), .docs/11_domains/dist.md (seventh-rule section)
- [x] `.plans/.highwater` 220→221; snapshot landing row (Batch 221) replacing the claim row; commit + push (re-fetch before push — the yield discipline)

## Anti-collision

Claim marker pushed BEFORE authoring (`80327d3f`). Re-fetch before every push; if a sibling lands any of the four, that item yields, the rest proceed.
