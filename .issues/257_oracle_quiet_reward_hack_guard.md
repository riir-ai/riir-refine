# Issue 257 — oracle-quiet reward-hack guard for the clippy fix lane

**Status:** Open — filed 2026-11-24 from `../riir-refine/.research/447_Noeri_Self_Improving_AIs_Landscape.md` (Noeri's self-improvement-loop results post + correction-formula methodology). Class-level prior art is PUBLISHED (cited below) — this issue claims ONLY the in-stack delta: classify applied clippy/rustc_errors-domain fixes that silence the oracle without fixing the finding, and report a hacked-adjusted heal rate.
**Route:** riir-refine fix-lane batch lane — mailed to `*@amo` 2026-11-24 (epoch 1795522368); UNCLAIMED — the recipient claims with a lease (claim-lease law) before implementing; fleet-boss hourly re-fire is the backstop.

## The class, in our lane

A fix "succeeds" when the oracle goes quiet on the finding. That can happen WITHOUT the underlying issue being fixed:

- `#[allow(clippy::lint)]` / `#![allow(...)]` / `#[expect(...)]` inserted at (or above) the flagged span;
- flagged code replaced by `todo!()` / `unimplemented!()` / `unreachable!()` / `panic!()`-class stubs;
- the flagged item/span deleted outright (the warning disappears because the code is gone);
- edits to files the oracle or its pins read (tests, benches — the bench-target guard already declines bench files statically; this is the classification, not the declension).

Today nothing in the fix path reads the diff SHAPE against the flagged span: `fix_verify` compile-gates, `clippy_verify` (opt-in) re-runs the oracle — either way an allow-insertion scores as a fix. The mined corpus currently carries no allow-insertion fix rules (checked: `src/domains/clippy_lints/` proposers carry `#[allow]` only as their own source annotations), but the L4/hosted fallback lane and the steer-retry loop generate arbitrary edits, and any future mined rule or self-evolve proposal can drift into this class.

## Tasks

- [ ] T0 (classifier, report-only): a parse-light diff-shape predicate over APPLIED fixes in the oracle-fed lanes — reads the flagged span from the clippy JSON feed (`--message-format=json`; the auto-oracle already carries spans) and the post-fix diff, and classifies `oracle_quiet` with a typed reason per shape (attribute-insertion / stub-replacement / flagged-span-deletion / oracle-file-edit). Never an LLM call; classify on every applied fix regardless of verify posture (posture-independent — reads the diff, not the oracle).
- [ ] T1 (outcome vocabulary): record the classification on the outcome row (adoption_gate window + `EvolveRecorder` trajectory). Prefer an outcome-kind FLAG over a new EvidenceTier — tier mutation is gated by the Lemma-4.2 tier-guard contract; the flag composes with existing tiers without moving the store's certification semantics.
- [ ] T2 (corrected reporting): the fix-run end roll-up and the score-bench strata carry hacked counts when non-zero (silent-when-zero, the Issue-165 discipline). Two figures, never one silently standing in for the other: PRIMARY = the exact subtraction `credited_nonhack = credited − hacked_credited` with the denominator (judged rows) stated beside it — our classifier labels each fix row individually, so the true count is known and Noeri's proportional form would subtract a guess where we have the number; CROSS-CHECK = Noeri's ratio form `credited × (1 − hacked/judged)` recorded for comparability with their published figures (Research 447). Deliberate suppressions stay visible, not punished: the classification is REPORTED with its reason — refusing a developer's deliberate `#[allow]` is out of scope. Retroactive demotion of previously-credited outcomes is decided by a Claude verdict once the T3 fixture shows precision on the planted shapes, then recorded in this issue (per the Gating law: not an owner gate).
- [ ] T3 (tests + canary): fixture with one fix per shape + two controls — a legit semantic fix with the same lint, and a file whose `#[allow]` PREDATES the fix (the classifier must read the PRE-fix diff as its baseline, not merely find an attribute near the span, or it false-fires on an existing suppression); the canary proves the classifier fires; G4 unchanged (classify is report-only on the outcome path); `--verify-args` compatible; never in the release set until its own gates pass (opt-in or always-on-report-only per what T0 costs — it is parse-light, so report-only default-on is the expected posture).

## Proof path

The fixture IS the gate for T0/T3: each planted shape must classify, the control must not, and the corrected figure must move on the planted row and stay byte-identical on the control. No synthetic training data, no echo gate needed (classification is deterministic diff-shape logic, not learned).

## Related

Research 447 (the filing landscape note; Noeri correction formula) · Research 377 (SkySynth five-shape hack taxonomy — the classification vocabulary to reuse for reasons) · shipped guards this extends: bench-target guard (bench files), adoption_gate readmit refusal (output history), blueprint-report (verdict audit) · prior art (class-level, why the claim is narrow): EvilGenie arXiv:2511.21654 (test-file-edit detection for reward-hack measurement), APR patch-overfitting line (Smith 2015 FSE; ODS 2024, FSE; arXiv:2603.11262), TRACE 2026 (human-verified exploit taxonomy).
