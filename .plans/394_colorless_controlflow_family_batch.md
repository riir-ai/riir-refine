# Plan 394 — the colorless-rs family-seed batch (R1 sync-async twins + R2 drop-or-keep; nominal Batch 378)

**Status:** STAGED — unclaimed (claim-before-edit; 48 h lease from the claim push; the push is the arbiter)
**Claimed:** zed-m3@m3, 1795283407, expires 2026-11-23T17:50:07Z
**Source:** veecore/colorless-rs @ `b5db6b95cf2f18efbb12fabe5709ac003fcde622` (verdict pin; re-clone + re-pin at T0 — fast-moving, zero publish history). Research 428 is the controlling verdict.
**Batch number:** nominal **378** (376 = Plan 387 anemll, LANDED 2026-11-21; 377 = Plan 393 humanfia, staged) — RE-DERIVE at landing HEAD from the score_history tail per convention.

## Scope

Transcription batch, rust_perf domain, suggestion-only NotBenchable-by-domain throughout (the dial9/B136 annotation class). Expected landing: `entries_90` (re-derive the slot number at authoring by `ls`). Expected corpus delta: rust_perf +1 or +2, with 0–1 drop.

## Tasks

- [ ] T0 Re-clone `.raw/colorless-rs` at the verdict pin + quote re-verification (Research 428 §The mechanism + §Dedup evidence quotes). If the tip moved: re-pin with disclosure, re-read every cited site, note drift in-rule.
- [ ] T0b Dedup re-verification at landing HEAD, both vocabularies, per the verdict's greps PLUS the reviewer-widened set for R2: entries_02 (`async-future-size`) + entries_79 (`async-wrapper-future-size-exponential-nesting`) + their preflight (`.plans/b363_goat_preflight.rs` — the `Pin<Box<dyn Future>>` harness shape) + the named-async-rule census. Population evidence row re-derived: `maybe-async` in the 094 manifests (t03:3340 / t04:3450) — R1's first firing target.
- [ ] T1 Author R1 `sync-async-twin-duplication` (the family seed, B−). TRIGGER: name-suffix pair (`_sync`/`_async`, or blocking/nonblocking trait-impl pairs) **PLUS a body test** — ≥2 duplicated control-flow statements. ANTI-REGIME: the single-expression DELEGATION wrapper. Cited remedies: `maybe_async` / `async-generic` / hand-rolled `Colored<C>` trait / keyword-generics watch. IN-RULE: "Distinct from blocking-sleep-in-async" (call-site runtime defect vs repo-shape). Suggestion-only, `bench_spec: None`, NotBenchable annotation.
- [ ] T2 Author R2 `box-dyn-future-in-hot-path-colorless` under the DROP-OR-KEEP gate: keep ONLY if the anti-regime bounds cleanly (legal-erase cases: recursive futures, dyn-trait object boundaries, deliberate monomorphization/compile-time-bloat control). If the bounded shape fails at authoring → drop honestly, the drop is the record.
- [ ] T3 ORIGINAL fixtures (`corpus/fixtures/b378_fixtures.rs` or the re-derived slot): the twin pair (duplicated ≥2 statements), the delegation-wrapper anti-regime arm, the erased-future arm + its legal-erase arms. Standalone-compiled AND executed BEFORE any corpus edit.
- [ ] T4 Gates at an isolated `CARGO_TARGET_DIR` on a QUIET rig. ⛔ BOX-STATE (verdict-lane note, 2026-11-21): m3 was BUSY with a sibling release-test run — docs land on m3, but fixture EXECUTION and all cargo gates route to amo/bmo via `agent-mail rig run --shared` (the zero-cargo docs lane precedent; quote the rig in the landing receipt).
- [ ] T5 pins_manifest --write in-landing + the count web one-landing (AGENTS rust_perf head + README + `.docs/11_domains/rust_perf.md` + health_views total/perf + the kernel-side rust_perf witness) — all re-derived AT LANDING HEAD (rust_perf 380 / sec 80 / ane 13 / kernel_opt 1093 as of the B376 landing, 2026-11-21).
- [ ] T6 score-bench/freeze NOT owed unless the clippy surface moved (assert zero clippy_lints delta); release-set binary rebuild NOT owed for a suggestion-only rust_perf corpus delta per the B368/B369 disclosure — owed at the next maintained-binary lane.
- [ ] T7 Research 428 Status → `DISTILLED — batch-378-landed` (or the honest NEGATIVE row per the ungoat law if R1 dies at falsification); snapshot LANDING row; dual_allocation gate at the landing commit; `feat: batch 378: ...` push; `rm -rf .raw/colorless-rs` at close.

## Proof path (pre-registered per the ungoat law)

R1's fixture IS the proof: the twin arm demonstrates the duplicated-body shape the detector keys on; the delegation-wrapper arm proves the anti-regime refuses; the tampered arm (one twin edited, the other stale) demonstrates the drift the rule exists to prevent. If the batch cannot show failing-then-passing behavior on the real detector vocabulary, R1 lands as annotation-class decision content only — never a graded rule without its fixture.

## Yield rules

A pushed claim row naming Plan 394 / Batch 378 / the colorless source lane from another session supersedes this on fetch. Expiry 48 h from the claim push.

NOTHING INVENTED — every quote at the pin. Session: zed-m3-colorless@m3, epoch 1795227779.
