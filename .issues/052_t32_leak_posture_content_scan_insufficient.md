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
2. **No pattern work is proposed *for the purpose of admitting `external`*.**
   Closing the 7 evasions has real value for *own-repo* hygiene, but it
   cannot change this verdict — Result 4 is untouched by any pattern
   improvement. Doing the work and then admitting `external` would be the
   dangerous outcome: a much stronger scanner still allows the PII span.

   **One exception was taken, and only one — `REDACT_VERSION` 1 → 2, see
   §The v2 bump below.** It is justified on own-repo determinism grounds,
   NOT as progress toward `external`, and it does not move the verdict
   above. The distinction matters enough to state twice because conflating
   them is exactly the trap this issue names.
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

## The v2 bump (`REDACT_VERSION` 1 → 2) — the one change taken

Result 2 is not merely a gap; it is a **miscalibrated single parameter in a
security boundary**, and that is worth fixing on its own merits regardless
of `external`.

**The argument.** Whether a hex-wrapped credential blocked depended on the
entropy of that specific value, measured at 37.0% over 4,000 AWS-shaped
keys with the range (2.349..3.412) straddling the 3.000 cut. A boundary
whose verdict is chosen by the attacker is not a threshold. The rest of the
gate is explicitly fail-closed — the module doc already accepts that
committed digests block "by design" — so an all-hex run of 32+ chars should
block unconditionally: it is a digest or an encoded blob, and both are
things this gate exists to stop.

**v2:** the hexish arm of `scan_entropy` drops the entropy test entirely.
The non-hex arm KEEPS its 4.5 cut — deliberately, because `_` is not an
ASCII hex digit, so every long `snake_case` run lands on that arm and the
4.5 threshold is what stops identifiers from false-positiving. Measured
after: **4,000/4,000** hex-wrapped keys caught (from 1,481).

**Cost, measured before committing to it** — over this repo's own `src/` +
`tests/`: 17 hexish runs ≥ 32 chars, **15 already blocked**, so exactly
**2 newly block**, and both are the same synthetic `4242…` fixture. The
change is close to free.

**Blast radius checked, because a version bump is fail-closed by design**
(`policy.version != REDACT_VERSION` ⇒ deny-all):

| surface | finding |
|---|---|
| `riir-clippy/.heal-sync-policy` | migrated to `version: 2` in the same commit — REQUIRED, or all sync deny-alls |
| `riir-clippy-glm/.heal-sync-policy` | **unaffected** — independent clone, vendors its own `kat_redact.rs` with its own `REDACT_VERSION = 1`, so it stays self-consistent. Not touched. |
| `kat_sync_payload` golden | `redact_version` is a WIRE HEADER field, so the golden moved 1 → 2. `batch_commitment` is over the ROW lines only and is unchanged — one line moved, which is the evidence the commitment domain is correct. |
| `riir-dapps` mirror | **no coordination needed.** dapps parses `redact_version` as DATA (`vu32`) and validates only `payload_version`; it has no cargo dep on riir-clippy (the Issue 041 one-way rule). Its hardcoded `redact_version: 1` fixtures keep passing and keep exercising the parser against an older header, which is coverage worth keeping. Left untouched. |

**An observation worth recording rather than acting on:** `redact_version`
is an **unverifiable claim**. Scanning happens on the sender, so a receiver
cannot confirm any gate ran, let alone which version — a sender could
assert v1 (weaker) or v2 (stronger) freely. That makes the field
provenance metadata for audit, not a control, and it is why the receiver
declining to validate it is the *right* call: rejecting rows because the
sender upgraded its scanner would be strictly harmful, and rejecting on a
claimed-old version would be security theatre. Noted so nobody later
"hardens" it into a check.
