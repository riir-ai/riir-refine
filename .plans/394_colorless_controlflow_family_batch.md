# Plan 394 — the colorless-rs family-seed batch (R1 sync-async twins + R2 drop-or-keep; nominal Batch 378)

**Status:** LANDED as Batch 377 2026-11-22 (feat `04b790e6e` + docs `15457ef98`, pushed) — the batch is complete; the STAGED line below was the pre-claim posture and is kept only as history. Plan closed; riir-refine Issue 394's numbered-file work is done (the landing records live in AGENTS.md's per-batch arcs + the snapshot LANDING row).
**Lease closed:** zed-m3@m3, epoch 1795283407, was expires 2026-11-23T17:50:07Z — the batch LANDED 2026-11-22 (feat 04b790e6e + docs 15457ef98, pushed), the work complete
**Source:** veecore/colorless-rs @ `b5db6b95cf2f18efbb12fabe5709ac003fcde622` (verdict pin; re-clone + re-pin at T0 — fast-moving, zero publish history). Research 428 is the controlling verdict.
**Batch number:** LANDED AS **377** — at this batch's landing HEAD the highest LANDED was 373 (376 anemll + 373 atlas); the nominal-378 assumption (377 humanfia in flight) never materialized — the plan-393 lane never started (T0.1-only lease closed). The re-derivation takes the next free number 377; disclosed in the claim row + landing receipt.

## Scope

Transcription batch, rust_perf domain, suggestion-only NotBenchable-by-domain throughout (the dial9/B136 annotation class). Expected landing: `entries_90` (re-derive the slot number at authoring by `ls`). Expected corpus delta: rust_perf +1 or +2, with 0–1 drop.

## Tasks

- [x] T0 Re-clone `.raw/colorless-rs` at the verdict pin + quote re-verification (Research 428 §The mechanism + §Dedup evidence quotes). If the tip moved: re-pin with disclosure, re-read every cited site, note drift in-rule.
- [x] T0b Dedup re-verification at landing HEAD, both vocabularies, per the verdict's greps PLUS the reviewer-widened set for R2: entries_02 (`async-future-size`) + entries_79 (`async-wrapper-future-size-exponential-nesting`) + their preflight (`.plans/b363_goat_preflight.rs` — the `Pin<Box<dyn Future>>` harness shape) + the named-async-rule census. Population evidence row re-derived: `maybe-async` in the 094 manifests (t03:3340 / t04:3450) — R1's first firing target.
- [x] T1 Author R1 `sync-async-twin-duplication` (the family seed, B−). TRIGGER: name-suffix pair (`_sync`/`_async`, or blocking/nonblocking trait-impl pairs) **PLUS a body test** — ≥2 duplicated control-flow statements. ANTI-REGIME: the single-expression DELEGATION wrapper. Cited remedies: `maybe_async` / `async-generic` / hand-rolled `Colored<C>` trait / keyword-generics watch. IN-RULE: "Distinct from blocking-sleep-in-async" (call-site runtime defect vs repo-shape). Suggestion-only, `bench_spec: None`, NotBenchable annotation.
- [x] T2 Author R2 `box-dyn-future-in-hot-path-colorless` under the DROP-OR-KEEP gate: keep ONLY if the anti-regime bounds cleanly (legal-erase cases: recursive futures, dyn-trait object boundaries, deliberate monomorphization/compile-time-bloat control). If the bounded shape fails at authoring → drop honestly, the drop is the record.
- [x] T3 ORIGINAL fixtures (`b394_fixtures.rs` — the slot convention follows the PLAN number per the b389 precedent; 7/7 standalone green) (`corpus/fixtures/b378_fixtures.rs` or the re-derived slot): the twin pair (duplicated ≥2 statements), the delegation-wrapper anti-regime arm, the erased-future arm + its legal-erase arms. Standalone-compiled AND executed BEFORE any corpus edit.
- [x] T4 Gates — RESOLVED LOCAL: bmo unreachable (Tailscale down at gate time, two worktree-setup failures logged), amo's shared queue held behind two Exclusive lanes ~20 min; the sibling's test drained and m3 fell quiet (load 2.15/16), so the full battery ran locally at the isolated /tmp/plan394 target: 3946/0 default + 4442/0 rust_perf + 4591/0 triple-feature lib, clippy -D x3 postures clean, pins_manifest green, fmt gate PASSED after adjudicating the two clean-inverted tripwires (b375 pre-existing + b389 own-landing miss, git-verified entry-clean, the b353/b367/b368 row precedent). Disclosed: local-run deviation from this task's pre-registered routing, box state quoted `CARGO_TARGET_DIR` on a QUIET rig. ⛔ BOX-STATE (verdict-lane note, 2026-11-21): m3 was BUSY with a sibling release-test run — docs land on m3, but fixture EXECUTION and all cargo gates route to amo/bmo via `agent-mail rig run --shared` (the zero-cargo docs lane precedent; quote the rig in the landing receipt).
- [x] T5 pins_manifest --write in-landing + the count web one-landing (AGENTS rust_perf head + README + `.docs/11_domains/rust_perf.md` + health_views total/perf + the kernel-side rust_perf witness) — all re-derived AT LANDING HEAD (rust_perf 380 / sec 80 / ane 13 / kernel_opt 1093 as of the B376 landing, 2026-11-21).
- [x] T6 clippy-surface delta ZERO verified (clippy_lints corpus unmoved — score-bench/freeze not owed); release-set binary rebuild not owed per the B368/B369 disclosure unless the clippy surface moved (assert zero clippy_lints delta); release-set binary rebuild NOT owed for a suggestion-only rust_perf corpus delta per the B368/B369 disclosure — owed at the next maintained-binary lane.
- [x] T7 Research 428 Status → batch-377-landed → `DISTILLED — batch-378-landed` (or the honest NEGATIVE row per the ungoat law if R1 dies at falsification); snapshot LANDING row; dual_allocation gate at the landing commit; `feat: batch 378: ...` push; `rm -rf .raw/colorless-rs` at close.

## Proof path (pre-registered per the ungoat law)

R1's fixture IS the proof: the twin arm demonstrates the duplicated-body shape the detector keys on; the delegation-wrapper arm proves the anti-regime refuses; the tampered arm (one twin edited, the other stale) demonstrates the drift the rule exists to prevent. If the batch cannot show failing-then-passing behavior on the real detector vocabulary, R1 lands as annotation-class decision content only — never a graded rule without its fixture.

## Yield rules

A pushed claim row naming Plan 394 / Batch 378 / the colorless source lane from another session supersedes this on fetch. Expiry 48 h from the claim push.

NOTHING INVENTED — every quote at the pin. Session: zed-m3-colorless@m3, epoch 1795227779.
