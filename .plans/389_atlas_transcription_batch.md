# Plan 389 — cargo-atlas transcription batch (nominal Batch 373: R1–R3 + faces; R4/R5 gate at authoring)

**Status:** OPEN — staged plan-first 2026-11-19 (Research 423, delegated-owner Claude verdict AGREE round 1, session `d133eb3f-4d93-49f0-9083-b152add054c2`). Source: flowerornament/atlas @ `49d9ee41cca1cb8359fe28e2be13b430bba96569` (MIT OR Apache-2.0). Verdict: YES B− corpus, all candidates suggestion-only NotBenchable (Batch-136 annotation class); zero benchable GOAT — this is a decision-content batch, sec + rust_perf io. The cargo half awaits a free window; the claim row lands when the lane starts; the push is the arbiter.

## Phase 0 — re-pin + re-verify

- [ ] T0.1 Re-clone `.raw/atlas` at the pin `49d9ee41cca1cb8359fe28e2be13b430bba96569` (`--depth 1`); if the tip moved, re-pin and diff the five mechanism files (`crates/atlas-lib/src/revision/{store,mod}.rs`, `src/process.rs`, `src/render/pele/process.rs`, `crates/atlas-lib/src/checks/policy/observations.rs`) — quote re-verification per row, and re-derive every anchor by `grep -n` (the Research-423 lesson: anchors were stale in the first verdict draft; "byte-verified" covers quoted TEXT only, never locations).
- [ ] T0.2 Re-run the dedup greps at 3+ vocabularies over `src/domains/{sec,rust_perf}/corpus.rs` + the landed-batches delta since 2026-11-19 (relate B372 landed between verdict and batch): `GIT_DIR|GIT_OPTIONAL_LOCKS|config /dev/null|TAR_OPTIONS`, `end-of-options|argument injection|CWE-88`, `deadlock|stdin|take(|stream exceeds`, `serialize.*cap|refuses before`, `materialize|intermediate symlink`. Every hit read and ruled in-row; dedup-at-mining binds.
- [ ] T0.3 Read the UNREAD residue named in Research 423 (queries/navigation/*, analysis/representations/*, diagram/projection.rs, source/syntax/walk.rs body, checks/delta, checks/stages bodies, adapter/snapshot.rs, views/*) — re-grade any new candidate into the batch or record as residue.

## Phase 1 — authoring

- [ ] T1.1 R1 `external-helper-config-pinned-invocation` (sec InputValidation): fixture = original four-arm (buggy: helper spawned inheriting user-steerable env/config; fixed: env/config pinned before spawn; all-safe; tampered verdict arm). In-rule adjudication vs the owned env-strip instances (`clippy_oracle.rs` CARGO_TARGET_DIR, `rustc_errors/floor.rs` RUSTFLAGS/RUSTC_WRAPPER — build-config axis) and vs R2 (config surface vs arg surface). Regime conditions carry the TAR_OPTIONS anti-example (upstream-absent at the pin, recorded never PR'd) + the POSIX-shape caveat for `--config /dev/null`.
- [ ] T1.2 R2 `subprocess-argument-injection-pinned-flags` (sec InputValidation, CWE-88): fixture = original four-arm (a user-controlled string reaching a parser argv unpinned vs `--`/`--end-of-options`/format-validated). MANDATORY in-rule adjudication vs `shell_string_command` BOTH directions (its own doc names this class as a different, narrower one — quote it in-rule).
- [ ] T1.3 R3 `child-exchange-concurrent-bounded-drain` (rust_perf io): fixture = original four-arm over a scripted child (serial write-then-read vs concurrent bounded drain with kill-on-first-failure; the buggy arm must DEMONSTRATE the hazard shape without actually hanging — bounded child script, not a real deadlock). In-rule adjudication vs kat_migrate_normal's `Stdio::piped()` instance (positive instance at home, recorded not flagged).
- [ ] T1.4 R4 `serialize-under-cap-refuses-before-spawn` — gate at authoring: land as a rule ONLY if distinct from B274/entries_58 under one read of both; else fold as a face onto B274's notes.
- [ ] T1.5 R5 `measured-tree-verifies-against-pinned-revision` — gate at authoring per the verdict round: land as a rule ONLY with a fixture that fails then passes; else record as a design annotation in Research 423 (already carries it as C+ annotation-or-rule).
- [ ] T1.6 F1 rider face onto B342 `read-output-trust-inherits-argument-taint` (count-neutral, in-rule); F2 recorded in Research 423 only.
- [ ] T1.7 All fixtures standalone-compiled AND executed BEFORE entry (the standing gate); `bench_spec: None` + `NotBenchable` on every landed rule.

## Phase 2 — gates + landing

- [ ] T2.1 Standing gates at an isolated `CARGO_TARGET_DIR`: default lib + full-feature lib (rust_perf_bench posture) + sec-corpus compile, clippy `-D warnings` at the four postures, fmt gate, batch needles pass by name.
- [ ] T2.2 Retrieval floors HELD (no re-pin expected — sec + io additions); pins_manifest `--write` in the landing commit; count pins one-landing (AGENTS §Role sec/rust_perf heads, README, health_views, the kernel-side witness untouched, the narrative heads' with/without/annotated chains).
- [ ] T2.3 Score-bench/freeze NOT owed unless the clippy_lints corpus moved (P30 assert); dual_allocation gate from the riir-refine cwd; `rm -rf .raw/atlas` rides the landing commit's completion act.
- [ ] T2.4 Update Research 423 Status line (verdict → landed, batch number), append the BATCH LANDED row to `.distill/001_mining_queue_snapshot.md`, commit + push on develop.

## Out of scope (named, not filed here)

- The sge/seal-remake atlas-policy adoption lane — a separate sprint filing in `../seal-sprints`, decided by a Claude verdict when a guard lane picks it up (Research 423 §TOOL-LANE MAPPED). Nothing in THIS repo's graph consumes atlas.
