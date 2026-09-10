# Issue 087 — `kat_burn_stack` e2e: the burn auto-report returns `Skipped` at clean HEAD (3 tests red) — suspected account-plane regression from the auto-login/dir-move era

**Status:** RESOLVED — filed 2026-09-10 by the Issue 080/081/085 fix session (found during Issue 080 validation; NOT caused by that diff). FIXED same day: commit `fbedf6d2` (harness-side; root cause below).
**Progress 2026-09-10 (root cause + fix landed, commit `fbedf6d2`):** ROOT CAUSE IS **NOT** the account plane. It is **stale e2e harness units vs the deliberate whole-KAT wire contract** introduced by `e9eca71d` (2026-09-09 11:21, "burn wire carries WHOLE KAT — convert the micro-KAT charge book at the sync boundary", issue 081) — same day, ~11 h AFTER the harness landed (`f3a7cf28`, 2026-09-09 00:10). The harness (written against the pre-conversion skip predicate `charged_kat <= ack` + raw-micro push) bills only 11 micro-KAT/run; `wire_kat_total(11) = 0` whole KAT ≤ ack 0 → legitimate `Skipped` BEFORE any dial — exactly the observed shape (bills, exit 0, no `burn sync:` line, zero requests). The conversion is correct and cross-repo-pinned (the 10⁶× over-report incident consumed a 100 KAT grant for a true 2.32 KAT spend; unit goldens client+dapps pin floor semantics; sub-KAT remainders ride the next crossing), so the fix is HARNESS-side: fixture now bills exactly 1_000_000 micro-KAT (1 whole KAT) per run — a comment-only `a.rs` (valid Rust, zero AST items → AstChunker's documented whole-file single-chunk fallback; billing is raw-byte parse-independent per the T0.5 contract) with an in-test `count_word_tokens == 1_000_000` self-assert; the three stale assertions updated to the wire unit (`sent == wire_kat_total(y)`, `ack.burn_kat_total == wire_kat_total(charged)`, the stacked line asserted against the exact whole-KAT line format). NO assertions relaxed — the mock-receives-a-real-report coverage is STRICTER now (exact wire totals 1→2 KAT, signature over the wire number). Evidence the account plane is healthy: all 13 kat_burn_client lib/transport tests green at HEAD (incl. `run_burn_sync_reports_and_advances_the_ack_over_a_real_socket`). Validated: `kat_burn_stack` 3/3 at the required-features set AND at `--all-features` (the ci_feature_guard Layer 5c configuration — this target IS in that sweep; it red between guard runs since Sep 9); `kat_soft_gate_cli` 13/13; clippy clean on the touched file (both feature sets). Files changed: `tests/kat_burn_stack.rs` only. Coverage-hole follow-up (coordinator call): the ~1-day red window existed because the heavy guard is run manually between windows — no new lane needed (Layer 5c/5d already cover it), but a cheap dedicated `cargo test --test kat_burn_stack` row in a fast local gate would shorten the next one.

## The finding

`cargo test --test kat_burn_stack` fails **3 tests (0 passed)** at clean HEAD
`fcc478c`. Failure shape: the run bills and exits 0, but the burn
auto-report never fires — no `burn sync:` line, the mock server receives
zero requests (`run_burn_sync` returns `Skipped`, i.e. the account/key load
fails or a skip leg trips before any dial).

Reproduction evidence (Issue 080 validation, 2026-09-10): identical 3
failures reproduced at clean HEAD `fcc478c` in an isolated worktree
(sibling repos intact; worktree + `/tmp` target dir removed after), so the
regression is in COMMITTED state, not in any working-tree WIP. Suspected
root cause sits in the recently committed account-plane changes (Issue-081
account-dir move / the auto-login era, `--mine` auto-login + auto-sync
contract from the 2026-09-08/09/10 owner mandates).

## Why it matters

`kat_burn_stack` is the only e2e that exercises the burn auto-report stack
(warn-and-pay → cumulative book → `wire_kat_total` conversion → signed
push). Issue 081's incident class (symmetric mocks, asymmetric units) is
exactly what this target exists to catch — with it red, the burn wire has
no executable cross-boundary check until repaired. Note the sibling target
`kat_soft_gate_cli` (same binary+env harness) is fully green (13/13), which
localizes the break to the burn-report leg, not the harness.

## Suggested next step

Reproduce, then bisect the account-plane commits (the account-dir move and
`ensure_logged_in` inline chain are the prime suspects) for the load
failure that makes `run_burn_sync` skip; fix at the root, not by relaxing
the test.
