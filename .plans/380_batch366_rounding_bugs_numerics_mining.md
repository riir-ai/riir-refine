# Plan 380 — Batch 366: the rounding-bug numerics transcription batch (TorchLean/FloatLib atlas → sec/Numerics)

**Status:** LANDED 2026-11-16 (Batch 366, sec 69 → 74 + 1 face; the queue snapshot carries the LANDED row). **Lease closed:** zed-m3-b366-rounding@m3, claimed 1794796375, expired 2026-11-18T02:32:55Z (folded at landing, same session).

Proof path (pre-registered before any corpus edit): each rule lands with (a) our own Rust fixture reproducing the collapse (buggy arm must produce the exact wrong value the blog's table records — e.g. R1 −0.0·up at x=−6 f32, R2 0.0 at d=2^−12), (b) the fixed arm surviving it, (c) a tampered-verdict arm proving the fixture gate reds; all suggest-only `NotBenchable-by-domain` (Issue-185/B352 posture) — no Pareto claim is made or needed.

## Phase 0 — T0 re-derivation (gate for everything below)

- [x] T0.1 — re-read both live sources at mine time; re-verify each candidate's numbers against the blog tables (R1 −8.4e−11@x=−6; R2 3.0e−8@d=2^−12; R3 −2 vs 8/9; R4 1/64×5000 + decay→1.0; R5 +∞; R6 +∞ batch sums). Any number that no longer reproduces in OUR Rust fixture kills its candidate (ungoat law — demote in the same commit). **DONE: all six reproduced** (R3's −2 needed the exact PyG scatter-mean op order — the tie at 2^25 — derived and bit-reproduced; R6's deterministic arm is the precision stall at 2048, the libraries' ±∞ is tree-order, both carried on the face).
- [x] T0.2 — re-run the dedup greps at the landing head (R1 vs `fused-activation-approximation-expansion` + `sigmoid-minus-one`; R4 vs `fp16-subnormal-product-power-of-two-rescale`; R6 vs `wide-reduction-precision-split` — both directions, in-rule adjudication paragraphs written BEFORE the entries). **DONE: all four hosts verified present; the R1 vs sigmoid-minus-one distinctness (operator identity: no sigmoid-minus subtraction in the gelu identity) carried in the R1 entry.**

## Phase 1 — entry authoring (sec/Numerics, all suggest-only)

- [x] T1.1 — R1 `gelu-tanh-cancellation-sigmoid-form` (lead): fixture = tanh-GELU activation fn + deep-negative inputs; fix = `x·σ(2z)` identity spelling; regime note carries the band law (2e^{−2z} vs half-ulp-below-1); context_notes: NOT the erf switch, composes with `fused-activation-approximation-expansion` (different axis).
- [x] T1.2 — R2 `kl-penalty-exp-minus-d-cancellation`: fixture = GRPO-style k3 KL penalty at small d; fix = `expm1(d) − d`; upstream-NAIVE note (transcription only, never a PR); regime |d| ≲ 1e−3. (Entry also carries the in-stack pointer: riir-train issue 691 FIXED 29ba94d0.)
- [x] T1.3 — R3 `variance-cancellation-two-pass`: fixture = variance over a large-mean slice; fix = centered two-pass; regime note names JAX's documented `algorithm="stable"` as decision content.
- [x] T1.4 — R4 `decay-gate-precision-collapse-low-precision`: fixture = EMA/decay update in a low-precision lane; fix = log-space increment / wider accumulator; the Issue-940 carve-out written into the rule (f32 sync-crossing reductions stay order-pinned; this rule is low-precision local lanes only).
- [x] T1.5 — R5 `logit-clamp-epsilon-absorption`: fixture = logit clamp with eps below half-ulp; fix = output-domain clamp; regime eps ≤ half-ulp near the bound.
- [x] T1.6 — R6 `sum-single-rounding-low-precision` — face-or-rule call at authoring: if the fixture can't clear R6's regime independently of R3/R4's, land as a count-neutral FACE on `wide-reduction-precision-split` (the B351 rider-face precedent) with the Issue-940 carve-out as the face text. **LANDED AS THE FACE** (the stall arm is deterministic and independent of R3/R4's regimes, but the batch's pre-registered shape was 5 rules + 1 face and the B− grade's conservative landing is the rider; the F6 fixture pins the stall at 2048 and the face text carries the tree-order ±∞ half).

## Phase 2 — fixtures + gates

- [x] T2.1 — fixtures standalone-compiled AND executed BEFORE entry (`fixtures/b366_fixtures.rs`, the 4-arm discipline: buggy/fixed/all-safe/tampered); the buggy arms must FAIL exactly as the blog tables record. **DONE: 6/6 tests, zero warnings, `rustc --edition 2021 --test b366_fixtures.rs`.**
- [x] T2.2 — standing gates at the shared postures: clippy `-D` ×3 postures (default / no-default / sec — ALL ZERO), sec test battery (the `batch366_rules_present_families_and_provenance` + `batch366_detectors_fire_on_buggy_stay_silent_on_fixed` pair mirroring the B352 tests — both green; lib 4398/0 sec posture, 3916/0 default), lib count floor re-pinned (pins_manifest sec.rules 69 → 74; health_views line pin + total 1684 → 1689).
- [x] T2.3 — `pins_manifest.py --write` (landed WITH the corpus change; manifest + prose AGREE); score-bench NOT owed (zero clippy_lints surface, B363 precedent) — disclosed in the queue snapshot row.
- [x] T2.4 — provenance headers: live-fetch URLs + date; NO sha pins (no clone — say so in each entry's notes, the blog URL is the pin). (Enforced by the pin test's no-`@ \`` assert.)

## Phase 3 — ledger + docs

- [x] T3.1 — count web in one landing: AGENTS.md §Role sec counts (69/62 → 74/67), `.docs/` corpus tables (the health_views rows carry the compiled tree; README table row updated), queue snapshot Batch-366 LANDED row replacing this plan's claim.
- [x] T3.2 — cross-repo residue: katgpt-rs Issue 1220 (gegelu_tanh) is R1's LIVE stack counterpart and riir-train Issue 691 (k3 collapse, nf_grpo.rs:252 + svr_driver.rs:1151) is R2's — add one-line pointers from R1/R2's entries to those issues (fix-path precedent, not a corpus dependency); both issues carry the fixture-first proof paths. (R2's pointer landed in-entry; issue 691 was closed by the fix landing 29ba94d0 — the entry cites the commit. R1's katgpt-rs 1220 pointer rides the entry prose.)
- [x] T3.3 — commit `feat:` on `develop`, push; close Research 412's queue claim; Research 673 (katgpt-rs) §5 already carries the pointer — verify it resolves. (Resolves via the Research 412 source row; the R1 ↔ katgpt-rs 1220 cross-link is the landing note's residue pointer.)
