# Plan 248 — Batch 245: the Batch-244 defer list conversion (byte-scan/search leftovers)

**Status:** COMPLETE 2026-10-16 — claimed `d5e097fb` (zed-m3-idle, the queue-snapshot claim row; push is the arbiter).

Source: the Batch-244 defer list (Plan 247 / Research 276) — eight staged leads from the three Batch-244 sources (`rust-lang/regex` @ tag `regex-automata-0.4.18` commit `e6ad938198e68339a593f85d228f2d44cb54718f`, Apache-2.0/MIT; `BurntSushi/memchr` @ tag `2.8.3` commit `5fdb40c054e1fff359a2f7bdf7f87a13b34b465d`, Unlicense/MIT; `BurntSushi/bstr` @ tag `1.13.1` commit `134195be38c7c9a4887980fd0f6f8bae042dd91e`, Apache-2.0/MIT), each deferred WITH a reason. simdutf + unicode-normalization stay STAGED unread (Batch 246 candidates — never a silent scope grow). Research note: `.research/277_batch245_defer_list_conversion.md`.

The eight leads (from Research 276's defer table):
1. `self-loop-state-memchr-skip` (regex-automata `dfa/accel.rs`) — high value; bench needs long homogeneous runs + the disclosed negative arm (common exit bytes).
2. `pair-probe-confirm-fused` (memchr generic packedpair — two-offset AND mask) — dedup vs `dual-mask-fused-block-scan` needs its own pass.
3. `align-probe-overlap-tail` (the aligned-loop skeleton) — alignment half washes on modern unaligned-load µarchs (incl. this box); tail conversion real but offset-dependent.
4. `match-branch-cold-confirm` (`#[cold] #[inline(never)]` the rare confirm arm) — attribute-only; the source records its own counter-case (packedpair measured no consistent difference).
5. `char-search-encode-once-stack` (bstr `find_char` — encode the needle once, scan bytes) — thin sibling of `byte-find-position-to-memchr`.
6. `canonical-set-guarded-rebuild` (regex-syntax `interval.rs` — the is_canonical guard before sort) — adjacent to `sortedness-check-branchless-accumulate`.
7. `nonmax-sentinel-option-niche` (regex-automata `NonMaxUsize`) — timing-only-at-scale; composes with the u32-width rule.
8. The memmem Finder-hoist pair (memchr + bstr) — adjacent to `regex-hoist-lazylock`; that rule's body read is the adjudication input.

## Tasks

- [x] T0 — re-clone + pin (ephemeral-clone law): `.raw/regex` + `.raw/memchr` + `.raw/bstr` at the Batch-244 release pins (tags `regex-automata-0.4.18` / `2.8.3` / `1.13.1`; verify HEADs == the pinned commits; licenses confirmed in the Batch-244 record). Removed at close.
- [x] T0b — the Finder-hoist adjudication input FIRST: read the `regex-hoist-lazylock` corpus entry body (entries_02.rs:385 + its bench spec at entries_02.rs:1353) and decide merge/rider/own-rule for the memmem Finder-hoist pair BEFORE the subagents read (their briefs need the verdict).
- [x] T0c — dedup grep BEFORE proposing (the load-bearing filter): each of the eight candidates grepped against the 217-rule corpus (`src/domains/rust_perf/corpus/*.rs` + `src/domains/kernel_opt/corpus/*.rs`), 3+ vocabulary variants each; nearest neighbors named per candidate; ~50% conversion falsification applied per the lithos precedent.
- [x] T1 — two disjoint read subagents (A: regex-automata leads 1/6/7 + the `#[cold]` source lines for lead 4; B: memchr+bstr leads 2/3/4/5 + the Finder-hoist source lines for lead 8) — propose pattern/fix-space/category/regime/anti-regime with pin-verified verbatim quotes. Coordinator re-verifies EVERY proposed quote byte-for-byte at the pins (the Batch 31 law).
- [x] T2 — coordinator adjudication per lead: CONVERT (benchable or NotBenchable-annotated) / DEFER (reason) / DROP (reason). Expected shape per the falsification precedent: 3–5 conversions + riders; match-branch-cold-confirm DROPS unless the counter-case is refutable at the pin; align-probe-overlap-tail likely lands as a tail-conversion-only rider or drops the alignment half honestly.
- [x] T3 — author converted entries: `entries_31.rs` (new slice, header carries the batch provenance + the honest drops/defers), in-rule G1 tests per entry (regime-boundary + counter-case where the source carries one), `context_notes` with box-state provenance + the distinct-from set + anti-regimes; NotBenchable entries annotated per the standing format.
- [x] T4 — wiring: `mod.rs` registration + doc-header update; batch245 parse+needle gates in `corpus/tests.rs` (incl. NEGATIVE asserts for dropped/deferred rule ids); count pins re-pinned at the post-batch numbers across ALL assert sites (tests.rs totals + mod.rs doc + AGENTS.md feature row + README table + `.docs/11_domains/rust_perf.md` ledger).
- [x] T5 — GOAT (benchable conversions): G2 on the rust_perf_bench Pareto validator, `--release`, box-state provenance line (M3 Max/Mac15,8, AC power, load disclosed); G4 alloc-count axis; self-loop-state-memchr-skip's fixture carries the disclosed negative arm (common exit bytes) per Research 276's own note. Losers demoted/dropped honestly (the Batch-243 precedent).
- [x] T6 — standing gates: full lib tests; clippy `-D warnings` at default / no-default / all-features (+ the release-shaped feature set if the healer is rebuilt); retrieval floors re-measured — HELD no re-pin expected (rust_perf-only delta, blended pool shape unchanged), re-pin with attribution only if they move; gen2+gen3 freeze-tables regenerated (rust_perf corpus delta expected — plan 137 T7.5); score-bench appended.
- [x] T7 — docs: AGENTS.md rust_perf count + batch line; README table row; `.docs/11_domains/rust_perf.md` headline; Research 277 born-complete (status line per the research-note shape); Research 276 status → the defer list CONVERTED; plan 247 stays COMPLETE (no edit); queue snapshot landing row (same commit).
- [x] T8 — commit + push (one unit; `feat:` prefix; `Session: zed-m3-idle, <epoch>` in the body); `.raw/{regex,memchr,bstr}` removed; `/tmp` scratch targets cleaned; plan ticked in the same commit.

## Deferred

- [-] simdutf + unicode-normalization (staged unread since Research 275's follow-up pool) — Batch 246 candidates; the accumulation clause cuts both ways.
- [-] The memchr 2.7→2.8 pin-bump observation — a WATCH leg, not a batch input: Batch 244/245 mine at 2.8.3; a corpus rule citing 2.7-era line numbers would drift. Recorded in the queue snapshot's watch list; no action this batch.
- [-] memchr 3.x material (byteshift-unpack, Twice searchers) — verified NOT PRESENT at 2.8.3; joins the memchr-3.x watch leg.
