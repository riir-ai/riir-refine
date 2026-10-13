**Status:** OPEN — owner-gated posture call (workspace-level, pre-existing since 492381b2 / riir-kat dda8a39; measured again 2026-10-13 while landing the topup-QR change)

## 163: --all-features hermetic-CLI tier red since the net pins — the empty-set refusal preempts allow-local's loopback allowance

## The finding

At `--all-features` (Layer 5c's posture), the whole hermetic loopback-CLI
tier has been RED on develop since the net pins landed (492381b2,
2026-10-09; riir-kat dda8a39, 2026-10-07). Measured on stashed clean
HEAD before any of today's edits, and again after:

- `tests/topup_cli.rs` — 4/6 phases fail (happy, unarmed, nopack, history)
- `tests/kat_adopt_state_cli.rs` — 5/6 fail
- `tests/kat_burn_stack.rs` — 7/7 fail

Every failure is the same shape: the suite spawns the real
`cargo-refine` binary against a loopback mock
(`http://127.0.0.1:PORT`), and `command_service_url()` →
`kat_net_pin::ensure_net_allowed` refuses before any dial:

```
service url refused: this build enables BOTH net-mainnet and net-devnet —
the allowed host set is EMPTY by overlap semantics; rebuild with at most
one pin (net: conflict: …)
```

## The root cause — an ORDER question in `riir-kat/src/kat_net_pin.rs`

`ensure_net_allowed` checks `allowed.is_empty()` (the both-pins overlap
= empty set) FIRST and refuses unconditionally — BEFORE the loopback
branch that would honor `allow-local`. `--all-features` enables both
pins AND `allow-local`, so the wrangler-dev/loopback allowance is
unreachable exactly in the posture the guard's own Layer 5c runs.

- `kat_net_pin.rs` module table says "BOTH pins → every host refuses,
  loud" — the code matches the table's BOTH row, but the per-pin rows
  promise loopback `+ allow-local`, and the burn-stack suite's own
  advisory prints "every dial still refuses" as if designed. The intent
  is ambiguous; the tests disagree with the code, and the guard's own
  recorded "Layer 5c 3920/0" verdicts (batch 228, 2026-10-13) are
  inconsistent with a run of this suite — the failures are real and
  deterministic today.

## The owner-gated call

Two defensible repairs — they are OPPOSITE laws:

1. **Reorder the seam** (riir-kat): in `ensure_net_allowed`, check the
   `allow-local` loopback allowance BEFORE the empty-set refusal. Both
   pins then still refuse every HOSTED net (the anti-spoof law holds);
   only loopback opens — the hermetic-test + wrangler-dev posture.
   Layer 5c goes green; the shipped multi-net binary changes nothing.
2. **Pin the suites**: gate the three loopback-CLI suites OFF
   `--all-features` (or make their harnesses pass `--net`-style env),
   accepting that the all-features posture never exercises the money
   CLI lanes.

Deciding factor: is `--all-features` a posture CI owns (then it must be
GREEN there — option 1) or an accidental combination the suites should
avoid (option 2)? The repo's own Layer-4 rationale ("catches combo-only
bugs") argues for option 1.

## Evidence commands

```sh
CARGO_TARGET_DIR=/tmp/rf-check cargo test --all-features --test topup_cli
CARGO_TARGET_DIR=/tmp/rf-check cargo test --all-features --test kat_adopt_state_cli
CARGO_TARGET_DIR=/tmp/rf-check cargo test --all-features --test kat_burn_stack
# vs the suite's own required-features posture (pins OFF — green):
CARGO_TARGET_DIR=/tmp/rf-check cargo test --test topup_cli --features kat_transport,latent_retrieval,rust_perf,clippy_lints   # 8/8
```

Note: `topup_cli`'s happy-path dial census was ALSO stale at EVERY
posture (the T10 `GET /` pre-flight from riir-kat 23d551b postdates the
suite) — repaired in 2682d7a0 alongside the QR feature, which is what
surfaced this.
