# Plan 247 — Batch 244: the text/unicode family follow-up (regex byte-class law + bstr/memchr internals)

**Status:** IN_FLIGHT — claimed 2026-10-16 02:37+07 (`40ef1075`, zed-m3-idle); chartered by Research 275's follow-up pool (items 2 + 5; simdutf + unicode-normalization stay STAGED unread, not folded by the accumulation clause). Plan-first per Protocol A step 0.

## Source pins (T0 verifies at clone)

| Source | Pin (verify at T0) | License | Domain routing |
|---|---|---|---|
| `rust-lang/regex` | latest release tag on `regex-automata` 0.4.x line | Apache-2.0 OR MIT | rust_perf (simd / dispatch / caching) |
| `BurntSushi/memchr` | latest release tag (2.7.x line) | Unlicense OR MIT | rust_perf (simd) |
| `BurntSushi/bstr` | latest release tag (1.x line) | Apache-2.0 OR MIT | rust_perf (allocation / dispatch) |

## Why this source

Research 275's pool names the regex byte-class law "the ecosystem's canonical upstream of the same mechanism class" as Batch 243's ASCII-run skip: regex-automata translates character classes to BYTE classes precisely so SIMD/memchr fast paths fire. memchr's internals are the mature industrial form of the byte-scan family (`byte-find-position-to-memchr` mined ONE face already; the iterator/avx2/sse2 modules hold more shapes). bstr carries the substring-search + byte-string iteration forms. This is the family's follow-up batch per the landing row of Batch 243 (`5da01990`).

## Tasks

- [ ] T0 — clone `.raw/regex`, `.raw/memchr`, `.raw/bstr` at release pins; verify each pin is still the tag/branch tip (the B193 freshness law); record the pins in Research 276.
- [ ] T0b — dedup grep BEFORE proposing: each candidate concept against `src/domains/rust_perf/` (+ kernel_opt where a GPU-twin face could exist), both vocabularies (English + operator names). Adjudicate duplicates/adjacents/folds per the Batch 30/242 precedents.
- [ ] T1 — read subagents over disjoint file sets (regex-automata class machinery + memchr internals + bstr search) proposing rules with pattern, bounded fix-space, category, rationale, and VERBATIM source quotes; re-verify every quoted line against the real file at the pin (the Batch 31 law); apply the ~50% conversion falsification factor when projecting yield.
- [ ] T2 — author surviving entries in `src/domains/rust_perf/entries_NN.rs` (+ extend the standing syn-parse gate with a `batch244_pseudocode_parses` arm; + count-neutral riders on existing hosts where a face belongs in-rule rather than as a new rule).
- [ ] T3 — in-rule G1 tests for every new rule (each rule carries its own compile-proven fixture; benchable rules get locality-shaped bench fixtures in the Pareto harness shape).
- [ ] T4 — standing gates: full `cargo test --lib` green; `cargo clippy -D warnings` at the ×3 postures (default / `--no-default-features` / `rust_perf,latent_retrieval`); heal any mechanical findings via `cargo refine` first.
- [ ] T5 — freeze-table regeneration (plan 137 T7.5 — rust_perf corpus delta moves the stride windows) + score-bench append in the SAME batch.
- [ ] T6 — retrieval-precision re-measure: re-pin BLENDED_TOP1/TOP3 floors WITH attribution if they move; per-domain floors must ALL HOLD (a per-domain decay FAILS the batch).
- [ ] T7 — GOAT via `rust_perf_bench` for every benchable new rule: G1 correctness, G2 Pareto timing (--release, box state disclosed), G3 no-regression, G4 alloc-count; demote/annotate losers; never promote a perf gain on a wrong result. Box state beside every latency figure (bench_preflight + load class).
- [ ] T8 — docs: Research 276 (born-complete), AGENTS.md ×2 count pins (the feature-list row + the domain entry), `.docs/11_domains/rust_perf.md` ledger row, `.distill/001_mining_queue_snapshot.md` landing row; commit `feat: batch 244: ...` + push; remove `.raw` clones at close (ephemeral-clone law).

## Drop clause (the Batch 243 T3 precedent)

A 2-rule batch is a fine outcome — the P46/P47 precedent. Any candidate that measures ParetoDominated or cannot carry a mostly-clean regime in a synthetic is dropped honestly: rule body + bench fixture removed before landing, the negative recorded in Research 276 + this plan's completion status. Never land a dominated rule to hit a count.

## Conversion yield guidance

Pool read → expect 8–15 candidates → after dedup + falsification → 3–8 real rules + riders. Batch 242 landed 31 (source richer than predicted, disclosed); Batch 243 landed 2 + 1 honest drop (source thinner). No minimum; the corpus is the authority.
