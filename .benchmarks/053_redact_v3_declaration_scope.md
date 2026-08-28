# Bench 053 — REDACT_VERSION 3: the identifier-policy rule was decided by line width

Status: **RECORD — measured 2026-08-28, shipped in the same change.** Verdict:
**the gate's verdict on a secret-shaped declaration was a function of where
`rustfmt` broke the line, not of the span.** Rescoped the rule from the
physical line to the declaration. Cost measured at **0 newly-blocked spans**
across 230 `.rs` files in this repo.

Repo: riir-clippy · Surface: `src/kat_redact.rs` `scan_rust_secret_literals`
Related: [Bench 052](052_t32_external_leak_posture.md) (the v2 hexish bump — same defect class),
[Proposal 005](../.proposals/005_kat_metered_self_improving_service_network.md) T0.2

---

## What was measured

The identifier-policy rule (`let/const/static NAME … = "LITERAL";` where NAME
is secret-shaped) required the name, the `=` and the literal **on one physical
line**. Probed with a low-entropy, non-format value (`correcthorsebatterystaple`,
25 bytes) so that only this rule can reach it — the entropy floor (32-char runs)
and the known-format patterns are both out of range, which means the probe
measures this rule and nothing else.

| # | Spelling | v2 verdict | v3 verdict |
|---|---|---|---|
| A | `const DATABASE_PASSWORD: &str = "…";` (one line) | **BLOCKED** | BLOCKED |
| B | `const DATABASE_PASSWORD: &str =` ⏎ `    "…";` | **ALLOW** | BLOCKED |
| C | `let api_secret =` ⏎ `    "…";` | **ALLOW** | BLOCKED |
| D | `export DB_PASSWORD=\` ⏎ `  …` (shell continuation) | ALLOW | ALLOW *(out of scope — see below)* |
| E | `export DB_PASSWORD=…` (one line) | **BLOCKED** | BLOCKED |
| F | `static SERVICE_ACCOUNT_TOKEN: &'static str =` ⏎ `    "…";` | **ALLOW** | BLOCKED |

**A vs B is the finding.** Same secret, same declaration, same file — the only
difference is a line break, and the verdict flips.

## Why this is a defect and not an accepted gap

The module doc listed "a secret split across two lines evades the env/literal
rules" under *honest scope*. That framing implies an author had to do something
unusual. They do not: **`rustfmt` generates spelling B automatically** whenever
the declaration exceeds `max_width`. Verified directly —

```rust
// input (one line, 101 cols)
const DEFAULT_SERVICE_ACCOUNT_DATABASE_PASSWORD: &str = "correcthorsebatterystaplelongvalue";
// rustfmt --edition 2021 output
const DEFAULT_SERVICE_ACCOUNT_DATABASE_PASSWORD: &str =
    "correcthorsebatterystaplelongvalue";
```

So the trigger is *name length + indentation depth + value length crossing 100
columns* — three things that have nothing to do with whether the value is a
secret. This is the **same defect class as v1's hexish entropy cut** (Bench
052): a verdict decided by a quantity orthogonal to the security question. The
v2 rule there was *"an attacker picks the value"*; here it is *"a formatter we
run by default picks the verdict"*. For the own-repo threat model — accidental
commit of a bare secret — the formatter case is strictly worse, because it
fires without anyone intending it.

## The fix

The rule's **unit of analysis** moves from the physical line to the
declaration: when a secret-shaped declaration's `=` ends the line, the value is
read from the next line. One line of lookahead — exactly the shape `rustfmt`
emits (it never inserts a blank line there). Single pass, no extra allocation,
**no new pattern**. The same-line and continuation paths share one
value-matching helper (`push_rust_secret_literal`) so they cannot drift.

## Cost

Measured with the real scanner over the whole repo (not a regex approximation):

```
=== repo-wide scan: 230 .rs files ===
  aws-access-key: 23      high-entropy-b64: 19    openai-style-key: 11
  bearer-token: 2         high-entropy-hex: 17    private-key-block: 5
  conn-string-creds: 5    github-token: 4         slack-token: 2
  gitlab-token: 2         rust-secret-literal: 12
RustSecretLiteral findings: 12
of which WRAPPED (would have been MISSED before v3): 0
```

**0 newly-blocked spans.** A regex sweep across 8 sibling repos found 31
wrapped secret-shaped declarations, of which exactly **1** carries a string
literal (`riir-chain … KEY_HEX -> "aaa…"`, 64 hex chars) — and that one is
already caught span-wide by the v2 unconditional hexish rule, so it is not a
v3 delta either. Zero migration cost, latent hole closed.

## What v3 deliberately does NOT close

- **Shell `\`-continuation of an env assignment** (row D). No formatter emits
  it, and the shipped `allow_paths` carry no shell path — closing it would be
  speculative pattern growth.
- **A secret split inside the literal** (`"AKIA…\` ⏎ `…EXAMPLE"`) — the
  `line-split` case in `tests/kat_redact_t32_external.rs`. This is
  author-chosen obfuscation, not a formatting artifact, and it belongs to the
  unbounded transform class Bench 052 refuted chasing.

**The external evasion set did not move: v1 2/9 → v2 3/9 → v3 3/9.** That is
the expected result and it is pinned as such. v3's gain is entirely on the
**accidental own-repo** class, which the T3.2 suite does not measure. Stating
it explicitly guards against the trap Bench 052 names — reading a scanner
improvement as progress toward admitting `external`. It is not: `external`
stays denied by policy, and the operative control remains the `allow_origins`
line.

## Gates

| Gate | Result |
|---|---|
| G1 correctness | PASS — `v3_verdict_is_invariant_to_rustfmt_wrapping` (4 spellings, finding range asserted to cover both quotes) |
| G1b no-shadow | PASS — `v3_carried_declaration_does_not_shadow_the_next_line` (a lapsed carry still finds the next declaration; the 8-byte floor still applies) |
| G3 no-regression | PASS — lib 842, `kat_redact_g4` 22, `kat_redact_t32_external` 6, `kat_sync_payload_g4` 11, `kat_pending_estimator_g4` 21 |
| G9b versioning | PASS — `REDACT_VERSION` 3 + changelog row + `.heal-sync-policy` migrated + golden header moved + t32 version pin re-measured |
| Pin fires | **VERIFIED** — reverting `carried = true` → `false` FAILS `v3_verdict_is_invariant_to_rustfmt_wrapping` (the invariance test is written as a property over spellings, so it cannot be satisfied by adding a pattern) |

## Open item this surfaced (different repo)

`riir-dapps` `src/kat/mining.rs:277` enforces `payload_version` but **never
checks `redact_version`** — the receiver accepts a payload produced by any
gate version, including one weaker than the current floor. The client fails
closed on a stale *policy*; the service does not fail closed on a stale
*gate*. Filed as [riir-dapps Issue 007](../../riir-dapps/.issues/007_ingest_accepts_any_redact_version.md)
(commit `6d65f5d`) rather than fixed here — boundary (the service plane is
riir-dapps per Proposal 005 Issue 041), and the receive-side floor is a design
decision for the Worker/DO shell in flight there. The compatibility question is
settled in the issue so it need not be re-derived: `MIN_REDACT_VERSION = 3` is
free today (no deployed endpoint, no persisted payload on disk in either repo),
and the honest limit is recorded too — the payload is UNSIGNED, so a version
floor is a staleness guard against an honest-but-stale client, not an
authenticity control.
