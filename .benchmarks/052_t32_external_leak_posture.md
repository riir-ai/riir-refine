# Bench 052 — T3.2 leak posture: the content scanner under an admitted `external` origin

**Status:** RECORD (2026-08-28) — **REFUTATION, plus one shipped fix.** The
content-scan branch of Proposal 005 T3.2 cannot discharge the leak posture
for consumer-grade code; `external` origin stays denied. The measurement
also surfaced a miscalibrated parameter that was worth fixing on its own
merits, shipped as **`REDACT_VERSION` 1 → 2** (§The v2 bump) — justified on
own-repo determinism, explicitly NOT as progress toward `external`.
Decision + reasoning: **§Verdict** below. Issue 052 carried the same
decision and was CLOSED + REMOVED 2026-08-28 per the noise-reduction rule —
this bench and [Proposal 005](../.proposals/005_kat_heal_service_network.md)
T3.2 are the surviving record. Originating commits: `2581fa6` (the suite),
`a0d03b3` (the v2 bump). Re-verified green at HEAD on removal: 6/6.

Parent: [Proposal 005](../.proposals/005_kat_heal_service_network.md) T3.2 ·
[Bench 049](049_fixer_regate_poc.md) §T5 (the CF no-training half) ·
substrate `src/kat_redact.rs` · sibling suite `tests/kat_redact_g4.rs`

Files: `tests/kat_redact_t32_external.rs` (6 tests, `--features kat_service`).

```bash
cargo test --features kat_service --test kat_redact_t32_external -- --nocapture
```

## Why a second suite next to G4

G4 (21 tests) proves the gate blocks or byte-exactly redacts planted secrets
in spans we are *allowed* to sync. It never measures the class T3.2 gates
on, and cannot, because of a short-circuit: `RedactionGate::check` denies on
ORIGIN before scanning content, and `external` is absent from the shipped
allowlist. **So consumer-code safety today is one policy line, and the
scanner carries no evidence about this class at all** — pinned by
`t32_origin_denial_short_circuits_before_any_content_scan`, which asserts a
scanner-detectable span comes back with *zero findings*, provable only if
the scan never ran.

Serving consumer code promotes the scanner to load-bearing. This bench
measures whether it could bear that load.

## Method

Counterfactual policy: shipped shape **+ `external` admitted**, path
allowlist dropped (consumer code has no in-repo path, so a glob cannot gate
it), `allow_redacted: true`. That is the gate at its **most permissive
sanctioned setting** — every evasion below is measured against the
friendliest configuration a product would ship, so the result is not an
artifact of a strict test policy.

Every attack span contains the same credential *recoverably* (a reader of
the payload can reconstruct it), so "did the scanner catch it" is a fair
question for each.

## Result 1 — 2 of 9 caught

| attack | verdict | exploits |
|---|---|---|
| `bare-control` | caught | positive control — if this evaded, the gate would be broken, not evadable |
| `base64-wrapped` | **EVADED** | no layer-stripping; run is 27 chars (< 32 min) at 3.81 bits |
| `hex-wrapped-marginal` | **EVADED** | hex entropy **2.976** vs threshold **3.000** — by 0.024 bits |
| `hex-wrapped-highentropy` | caught | same class, different key — entropy 3.404 |
| `byte-array` | **EVADED** | decimal byte literals; no alnum run exists to scan |
| `concat-split` | **EVADED** | `concat!("AKIA", …)` — prefix never abuts its body |
| `line-split` | **EVADED** | backslash-continued literal across lines |
| `xor-obfuscated` | **EVADED** | arbitrary reversible transform |
| `reversed` | **EVADED** | prefix rule anchors on a literal prefix; reversal moves it to the tail |

Most are the module doc's own declared gaps — confirmation, not news. The
evaded set is **pinned** at `REDACT_VERSION = 1`, so closing one becomes a
version bump + changelog row (G9b) rather than a silent detection change.
The pin fails in BOTH directions by design: a closed evasion is good news
that must still be recorded.

## Result 2 — the hex layer is a coin flip, quantified (new)

The module doc hedges that an encoded secret is *"only caught if the encoded
blob itself trips the entropy rule."* That conditional does far more work
than it reads. For real AWS-key shapes the hex expansion sits **on** the
threshold.

Deterministic sweep, 4,000 AWS-shaped keys (`AKIA` + 16 base32 chars,
BLAKE3-keyed so it reproduces without a rand dep):

| metric | value |
|---|---|
| caught | **1,481 / 4,000 = 37.0%** |
| entropy range | **2.349 .. 3.412** |
| threshold (hexish) | 3.000 |

The two hex rows in Result 1 are the same technique with opposite verdicts.
So the hex class is **not an accepted gap with a known residual risk, and
not a defense** — the technique succeeds or fails on the value, and **an
attacker picks the value.**

Pinned as a RANGE (0.15..0.85) plus a straddle assertion (`min < 3.0 ≤
max`), deliberately: the claim is *"neither reliably caught nor reliably
evaded"*, and a point pin would invite tuning the fixture instead of the
gate.

**FIXED in v2 — measured 4,000/4,000 caught** (see §The v2 bump). The test
now asserts BOTH halves: v2 unanimity, AND that the v1 rule still measures
as a coin flip on the same corpus — so the justification for the bump stays
reproducible and the fixture cannot silently stop being adversarial.

## Result 3 — chunking defeats per-span scanning, structurally

The gate is per-span with no cross-span state. A credential straddling an
AST-chunk boundary leaves each chunk individually clean (`Allow`, zero
findings) while the reassembled value is the credential. No pattern set
fixes this: each chunk is *genuinely* clean.

## Result 4 — the decisive one: confidential ≠ secret

Consumer spans that are plainly confidential — patient PII (`.example`
domains, RFC 6761), a commented trade-secret pricing rule, internal host
topology — return **zero findings and are ALLOWED verbatim**.

Not a scanner bug: a secret detector correctly finds no secret there. It is
why the branch fails. **For `external` origin the confidential asset is the
span itself**, and there is no pattern for *"this source belongs to someone
else."* No `REDACT_VERSION` bump closes this class, because closing it is
not a detection problem.

## Verdict

**REFUTED — the content-scan branch cannot close T3.2.** `external` stays
denied. Results 1–3 say the scanner is weak against deliberate evasion;
Result 4 says that even a perfect scanner would not suffice, which is what
makes this a refutation rather than a to-do list. **Improving the pattern
set and then admitting `external` is the dangerous outcome** — a much
stronger scanner still allows the PII span.

Remaining close paths, both outside this branch: **(a)** vendor-side
data-usage sign-off (owner/legal; CF no-training half already verified in
writing, Bench 049 §T5, model-provider license open), or **(b)** a true
no-egress sandbox where consumer bytes never leave the customer boundary —
architecture, a new plan, not a test suite.

Trainer-nodes-only stands, now on a measured basis rather than a
precautionary one.

Two operative consequences, stated so they survive the issue's removal:

1. **`external` remains absent from `allow_origins`, and that policy line —
   not the pattern set — is what to defend in review.** Pinned by
   `t32_shipped_policy_denies_every_confidential_span`.
2. **No pattern work is proposed *for the purpose of admitting `external`*.**
   Closing the remaining evasions has real value for own-repo hygiene, but
   it cannot move this verdict: Result 4 is untouched by any pattern
   improvement. The v2 bump below is justified on own-repo determinism
   grounds ONLY — conflating the two is the exact trap this bench names.

## Honest residual

The 7 pinned evasions are live for `own-repo` spans too. Lower risk there —
we control our own source, and G4 shows the planted-secret classes ARE
caught bare — but not zero, and the hex coin flip means the gate's own
documented reassurance is weaker than it reads. That is **hygiene work, not
a leak-posture blocker**, and this bench does not claim it as closed.

## The v2 bump — the one thing this bench changed in the gate

Result 2 describes a **miscalibrated parameter in a security boundary**, not
just a gap: the verdict was chosen by whoever picked the credential. The
rest of the gate is explicitly fail-closed (the module doc already accepts
that committed digests block "by design"), so the consistent rule is that an
all-hex run of 32+ chars blocks unconditionally — it is a digest or an
encoded blob, and both are what this gate exists to stop.

`REDACT_VERSION` 1 → 2: the **hexish** arm of `scan_entropy` drops the
entropy test. The **non-hex** arm keeps its 4.5 cut deliberately — `_` is
not an ASCII hex digit, so every long `snake_case` run lands there, and 4.5
is what stops identifiers from false-positiving.

| | v1 | v2 |
|---|---|---|
| hex-wrapped AWS-shaped keys caught (4 000-key sweep) | 1 481 (**37.0%**) | **4 000 (100%)** |
| behavior | value-dependent | deterministic |
| newly blocked spans in this repo's `src/` + `tests/` | — | **2** (both the same synthetic `4242…` fixture; 15 of 17 hexish runs already blocked) |

### Blast radius (a version bump is fail-closed: mismatch ⇒ deny-all)

| surface | finding |
|---|---|
| `riir-clippy/.heal-sync-policy` | migrated to `version: 2` in the same commit — REQUIRED, else all sync deny-alls |
| `riir-clippy-glm/.heal-sync-policy` | **unaffected** — independent clone vendoring its own `kat_redact.rs` at `REDACT_VERSION = 1`, self-consistent. Not touched. |
| `kat_sync_payload::golden_payload_bytes_are_pinned` | `redact_version` is a wire HEADER field, so the golden moved 1 → 2. `batch_commitment` is over the ROW lines only and did **not** move — which is the evidence that the commitment domain is scoped correctly. |
| `riir-dapps` mirror | **no coordination needed.** dapps parses `redact_version` as DATA (`vu32`) and validates only `payload_version`; no cargo dep (the Issue 041 one-way rule). Its hardcoded `redact_version: 1` fixtures keep passing and keep exercising the parser against an older header — coverage worth keeping. Left untouched. |

### `redact_version` is an unverifiable claim (recorded, not acted on)

Scanning happens on the SENDER, so a receiver cannot confirm that any gate
ran, let alone which version — a sender may assert v1 or v2 freely. The
field is therefore **provenance metadata for audit, not a control**, and the
receiver declining to validate it is correct: rejecting rows because a
sender upgraded its scanner would be strictly harmful, and rejecting on a
claimed-old version would be security theatre. Recorded so nobody later
"hardens" it into a check.
