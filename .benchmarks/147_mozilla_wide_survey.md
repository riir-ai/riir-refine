# Bench 147 — Mozilla org wide-survey: seven fresh repos at `cargo refine`'s grown corpus (Batch 340), zero sendable quick wins; the quartet stays the campaign surface

**Date:** 2026-11-08 · zed-m3-idle-mozilla-survey · owner ask: "check other rust repo on mozilla that we can run cargo refine and has potential to fix for perf gain... find quick win not too huge and too complex first."
**Binary:** cargo-refine 0.4.8, build `b65715d0b68b` (release set, corpus through Batch 340 — 304 rust_perf rules + kernel_pregate + shader_perf default). Scans parse-only (`RIIR_REFINE_AUTO_ORACLE=0`), oracle-anchored clippy where the posture allowed.

## Repos surveyed (fresh pins, 2026-11-08)

| repo | pin | files | surface verdict |
|---|---|---|---|
| mozilla/neqo | `4b6d13566f56` | 247 (211 excluding fuzz/fixture) | **no sendable finding.** Pattern citations concentrate on `build.rs`/`benches/` (retrieval noise: bevy/linker/avx2 rules on build scripts). Uncapped per-crate scans of `neqo-transport`/`neqo-http3`/`neqo-qpack`/`neqo-common`/`neqo-udp`/`mtu` surfaced high-score `reserve-then-commit`/`batch-reserve` hits on whole-file spans — verified NOT matching (those spans are enums/handlers, not the reserved-structure shapes). Oracle-anchored clippy at their MSRV 1.95.0: **clean except 21× E0602 unknown-lint `clippy::assert_is_empty`** (their CI runs a newer clippy where the lint exists — a TOOLCHAIN-POSTURE artifact, not a finding). The one mechanical shape (`Vec::new()` in `neqo-http3/src/control_stream_remote.rs:70` `receive()`) allocates a per-call frame vec of trivially small length — gain ≈ 0, fails the PR-value bar. |
| mozilla/grcov | `86d5001522d6` | 33 | **no sendable finding.** The parser (`src/parser.rs`, 2973 lines) is already byte-keyed dispatch + `FxHashMap` + checked-fold key parsing — the mechanical quick-win shapes are already taken. Citation surface = use-lines + `#![allow]` spans (noise). |
| mozilla/dump_syms | `0a6ddad8fee3` | 21 | **no sendable finding.** Oracle-anchored clippy at their exact CI line (`--all-features --tests --all --examples -D clippy::all`, MSRV absent → stable posture): **verified-real ZERO warnings** (build-finished marker + empty capture both checked — the Issue-153 green-zero law). The one cited shape (`regex-hoist-lazylock`, `PathMappingGenerator::new` @ `mapping.rs:88`) constructs per mapping ENTRY (≤ a handful per run from CLI flags), not per line — not a hot path. Their multi-file path already hoists `PathMappings` once (`dumper.rs:283`). |
| mozilla/cargo-vet | `cef4dd43a0b9` | 34 | clippy-only citations (tests). Nothing perf-shaped. |
| mozilla/webrtc-sdp | `a3f731fce84b` | 14 (parse core cited clean) | **no sendable finding.** The 3444-line `attribute_type.rs` parse core carries no mechanical perf shape on the pattern surface; only use-line noise. |
| mozilla/authenticator-rs | `57545a634747` | 105 | clippy-only citations (redundant_static_lifetimes etc. on build.rs). Nothing perf-shaped. |
| mozilla/glean (`glean-core`) | `a61f1158699a` | 87 | **no sendable finding.** The high-score serde citations (`serde-borrow-deserialize`/`rawvalue-passthrough`, scores 80–106 at `internal_metrics.rs` spans 12-23/227-239/501-515) land on structs that DON'T derive Deserialize — retrieval noise on struct-field spans. `keyword-map-to-match` @ `core_metrics.rs:118-259` is a static-metric initializer block, not a keyword map. |

**Roll-up: 0 actionable perf findings across 7 fresh Mozilla repos at the grown corpus.** The quartet verdict (bench 128 Part 1) replicates on every new repo: mechanically clean at the pattern surface, and the oracle-anchored checks that could run were clean too.

## What this means for the campaign (Proposal 022/025)

- The Mozilla org's Rust surface offers **no quick mechanical win** for the healer's evidence lane. The #2968 issue-driven lane (bench 128, waiting on the upstream claimant answer) remains the only live Mozilla candidate, and it is owner-gated + upstream-gated.
- The evidence-bearing PR lane should therefore source from OUTSIDE mozilla-org (the campaign's repo selection is not contract-frozen to Mozilla — 022 says "upstream PRs", Mozilla was the grant-aligned choice). Any next send needs a fresh target hunt with the same gates, or the sccache #2968 lane unblocking.
- The fixer/trainer feed (the owner's "unfixed → fixer/trainer node" ask): the scan outputs committed beside this record (`/tmp/neqo_*.md` ephemeral — re-derive with the commands below; the pins + rule-name histograms above are the durable record). The noise classes observed (use-line routing, `#![allow]`-span routing, struct-field-span serde citations on non-derive structs) are pre-existing P115-class findings, corroborated on three more repos.

## Method notes

- Preview-cap disclosure (P115): whole-tree runs cap at 25 spans × top-3 per domain; every repo above was additionally scanned uncapped per crate/file for the triage.
- Oracle-anchored lanes that RAN: neqo (MSRV 1.95.0, workspace clippy, clean-modulo-unknown-lint), dump_syms (CI posture, verified-real zero). Lanes that CANNOT run locally: neqo's `cargo hack --feature-powerset` CI matrix (SKIP-class per G-PR1b).
- Local clone hygiene: dump_syms needed an empty `[workspace]` table for the standalone clippy run (added, then REVERTED — upstream ships none; the diff is not persisted). Clones live under `.raw/mozilla-heal/` per the campaign convention, pins above; remove at close-out.
- Session-tree hygiene: `src/domains/rust_perf/corpus/{entries_57,entries_61,tests}.rs` carry a SIBLING session's uncommitted fmt WIP (mtimes 2026-11-07 21:10, pre-dating this session) — untouched, not committed by this record.
