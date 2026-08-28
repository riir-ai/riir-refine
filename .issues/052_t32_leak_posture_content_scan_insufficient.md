# Issue 052 — T3.2's content-scan branch cannot discharge the leak posture (measured)

Status: **RESOLVED-BY-MEASUREMENT 2026-08-28 — the branch is REFUTED, not
blocked.** `external` origin stays DENIED; Proposal 005 Phase 3 remains
gated, and its only remaining close paths are the vendor sign-off (owner)
or a true no-egress sandbox. Suite: `tests/kat_redact_t32_external.rs`
(6 tests). Record: [Bench 052](../.benchmarks/052_t32_external_leak_posture.md).

Parent: [Proposal 005](../.proposals/005_kat_heal_service_network.md) T3.2 ·
gate substrate `src/kat_redact.rs` (`RedactionGate`, `REDACT_VERSION = 1`) ·
sibling suite `tests/kat_redact_g4.rs` (G4, 21 tests)

## The question T3.2 actually asks

T3.2 reads: *"Leak-posture sign-off (CF data-usage policy verification, **or
vessel sandbox adversarial suite over consumer-grade code**)"*. The second
branch looks like engineering work that could close an owner-gated phase
without an owner act, so it was taken up first.

Reframed precisely: **if we admitted `SpanOrigin::External`, would the
content scanner be a sufficient confidentiality boundary on its own?**

That is the real question because of a fact that the existing G4 suite does
not cover and that is easy to miss: **consumer-code safety today is origin
denial — one policy line.** `external` is absent from the shipped
`allow_origins`, and `RedactionGate::check` short-circuits on origin
*before* scanning content. So the scanner has never been the load-bearing
defense for this class, and no existing test measures it as one. Serving
consumer code promotes the scanner to load-bearing. Hence: measure it first.

## What was measured

A counterfactual policy admits `external`, drops the path allowlist
(consumer code has no in-repo path, so a glob cannot gate it) and sets
`allow_redacted: true` — the gate at its **most permissive sanctioned
setting**, so every evasion below is measured against the friendliest
configuration a product would ship.

### Result 1 — the scanner catches 2 of 9 consumer-grade evasions

| attack | verdict | exploits |
|---|---|---|
| `bare-control` | caught | positive control |
| `base64-wrapped` | **EVADED** | no layer-stripping; blob must itself trip entropy |
| `hex-wrapped-marginal` | **EVADED** | hex entropy 2.976 vs the 3.0 threshold — by **0.024 bits** |
| `hex-wrapped-highentropy` | caught | SAME class, different key (entropy 3.404) |
| `byte-array` | **EVADED** | no alnum run exists — decimal byte literals |
| `concat-split` | **EVADED** | token scan; prefix never abuts its body |
| `line-split` | **EVADED** | secret split across lines |
| `xor-obfuscated` | **EVADED** | arbitrary reversible transform |
| `reversed` | **EVADED** | prefix rule anchors on a literal prefix |

Most of these are the module doc's own declared gaps, so finding them is
confirmation rather than news. The evaded set is **pinned** at
`REDACT_VERSION = 1`, so closing one is a version bump + changelog row
(the G9b discipline) and never a silent detection change.

### Result 2 — the hex layer is a coin flip, not a gap (new)

The doc's hedge — *"only caught if the encoded blob itself trips the entropy
rule"* — carries far more weight than it appears to. Whether a hex-wrapped
credential trips the rule depends on **that specific value's** hex entropy,
and real AWS-key shapes sit right on the threshold. Measured over a
deterministic 4,000-key sweep (`AKIA` + 16 base32 chars, BLAKE3-keyed):

**1,481/4,000 = 37.0% caught; entropy range 2.349 .. 3.412 against a
threshold of 3.000.**

The two hex fixtures above are the same technique with opposite verdicts.
So the hex class cannot be reasoned about: it is not an accepted gap with a
known residual risk, and not a defense that can be relied on — the
technique succeeds or fails on the value, **and an attacker picks the
value.** Pinned as a RANGE (0.15..0.85), deliberately: the claim is
"neither reliably caught nor reliably evaded", and a point pin would invite
tuning the fixture instead of the gate.

### Result 3 — per-span scanning is defeated by chunking, structurally

The gate is per-span with no cross-span state. A credential straddling an
AST-chunk boundary leaves each chunk individually clean (`Allow`, zero
findings), and any consumer of the sync batch can reassemble it. No pattern
set fixes this, because each chunk is genuinely clean.

### Result 4 — THE STRUCTURAL ONE: confidential ≠ secret

Consumer spans that are plainly confidential — patient PII on `.example`
domains, a commented trade-secret pricing rule, internal host topology —
produce **zero findings and are ALLOWED verbatim**.

This is not a scanner bug. A secret detector correctly finds no secret
there. It is the reason the branch fails: **for `external` origin the
confidential asset is the span itself.** There is no pattern for "this
source belongs to someone else". No `REDACT_VERSION` bump closes this class
because closing it is not a detection problem.

## Decision

**The content-scan branch of T3.2 is REFUTED. `external` stays denied.**

1. **`external` remains absent from `allow_origins`.** The operative
   control protecting consumer code is that policy line, not the pattern
   set — so that is the line to defend in review. Pinned by
   `t32_shipped_policy_denies_every_confidential_span`.
2. **No `REDACT_VERSION` bump is proposed here.** Closing the 7 evasions
   would be real work with real value for *own-repo* hygiene, but it would
   not change this verdict — Result 4 is untouched by any pattern
   improvement. Doing the work and then admitting `external` would be the
   dangerous outcome: a much stronger scanner still allows the PII span.
3. **Trainer-nodes-only stands** (the owner's 2026-08-25 call), now on a
   measured basis rather than a precautionary one.
4. **T3.2's remaining close paths**, both outside this branch:
   - **(a) vendor-side sign-off** — one vetted vendor, per-model
     data-usage policy, opt-in + redaction. The CF no-training half is
     already verified in writing (Bench 049 §T5); the model-provider
     license is the open half and is an owner/legal act.
   - **(b) a true no-egress sandbox** — consumer bytes never leave the
     customer's boundary; inference runs where the code already is. This
     is architecture, not a test suite, and would be a new plan.

## Why this is filed as resolved rather than open

The task asked for an adversarial suite over consumer-grade code. The suite
exists, runs in CI, and returned a verdict: **this branch cannot close the
gate.** A refutation with a mechanical demonstration is a result, not a
blocker — the follow-on work (paths a/b) belongs to Proposal 005 T3.2 and
an owner decision, not to an open issue here.

Residual worth restating plainly: the 7 pinned evasions are live for
`own-repo` spans too. They are lower-risk there (we control what is in our
own source, and G4's planted-secret classes ARE caught bare), but they are
not zero-risk, and the hex coin flip in particular means the gate's own
documented reassurance is weaker than it reads. That is hygiene work, not a
leak-posture blocker, and is not claimed as closed by this issue.
