# Bench 055 — CI gate coverage: the allowlist was omission-silent

**Status: LANDED 2026-08-28.** `scripts/ci_feature_guard.sh` Layer 5c
de-allowlisted + Layer 5d (zero-test detector) added.
**Follow-up 2026-08-29:** Layer 5c had still never completed a run — a single
un-annotated binary (`bonsai_retrieval_probe`: 6.7 GB GGUF, ~50 min in
release, run in DEBUG with both tests loading the model concurrently at
15.5 GB RSS) was consuming the entire sweep. 4 binaries finished in the 30 min
before it; **26 finished in the 3 min after it was killed.** Both tests now
`#[ignore]` with the cost in the reason — the policy this bench already wrote
down, applied to the file that was missing it. See §Follow-up.

## The finding

Layer 5c named its test binaries explicitly:

```
cargo test --all-features --test kat_redact_g4 --test kat_sync_payload_g4 ... --bin kat_drain
```

Measured at HEAD (`82c43cc`):

| | count |
|---|---|
| `tests/*.rs` targets in the repo | **51** |
| named by the Layer 5c allowlist | **12** |
| **outside CI entirely** | **39** |

39 of 51 test binaries never ran in the guard. Sixteen of them are
gate-class by filename alone:

`docker_refusal_gate`, `docker_t5_control`, `fix_compile_gates`,
`floor_corpus_integrity`, `g4_l4_composition`, `goat_gate_004`,
`latent_suggest_goat`, `phases_11_14_preflight`, `scope_gate_adoption`,
`sec_boundary_gate`, `stage2_goat`, `t33_tether_g4_alloc`,
`t40_fixer_regate_harness`, `t94_clr_vote_preflight`, `uncontaminated_g1`,
`wgsl_t5_control`

— plus `kat_redact_t32_external`, the Proposal 005 Phase 3 **T3.2
leak-posture suite**, and `sec_boundary_gate`, Proposal 004's
**do-not-reimplement non-negotiable**. Both are standing gates by their own
prose. Both were outside CI.

## The drift is live, not historical

Mid-audit, a sibling agent landed `tests/docker_g4_alloc.rs` — taking the
count from 51 to **52**. It is a **G4 alloc gate** by name and by content.
Under the allowlist it would have been outside CI from birth, and nothing
in the repo would have said so. That is the mechanism failing in real time
while being measured, not a historical artifact being cleaned up.

## Why it rotted, and why it was always going to

The allowlist is an **allowlist with no exhaustiveness check**: adding
`tests/foo.rs` simply never joined CI, and *nothing said so*. There is no
negative signal for omission — a suite that falls out looks exactly like a
suite that was never meant to be there. Every landing that added a test
without also editing a shell script widened the gap silently, which is why
the gap reached 39 without anyone deciding it should.

The allowlist was also **buying nothing**. There is no `[[test]]` block in
`Cargo.toml`: every test target is auto-discovered and self-gates via an
inner `#![cfg(feature = "...")]`. Layer 5a (`cargo clippy --all-targets
--all-features`) already type-checks all 51. So naming a subset saved the
*run* of 39 binaries, not their compile — and hid the other 39 to do it.

## Follow-up 2026-08-29 — Layer 5c had still never completed a run

De-allowlisting Layer 5c was correct, and the design statement below is
correct: what stays out belongs in `#[ignore]`, in the test file, next to the
reason ("network, model weights, hours-long"). **One binary was never given
that annotation, and it was the one that needed it most** — so from the day
Layer 5c landed, it had not once run to completion. Four prior attempts were
each diagnosed as an environment problem (killed process, `nohup` vs
SIGKILL, moved to `screen`). The environment was never the cause.

`tests/bonsai_retrieval_probe.rs` is gated on `bonsai_retrieval`, which
**`--all-features` turns on**. Its own module docs price it at **~50 min in
RELEASE** for one test and mandate `--test-threads=1`. Layer 5c runs it in
**debug**, with default threading, so **both** tests load the 6.7 GB
Ternary-Bonsai GGUF **concurrently**:

| | measured |
|---|---|
| RSS | **15.5 GB** (23.6% of machine memory) |
| CPU | 199% |
| elapsed when killed | 15 min, no output, still going |
| binaries completed in the ~30 min before it | 4 |
| binaries completed in the 3 min after it was killed | **26** |

That last row is the whole finding. One test binary was consuming the entire
run; everything downstream of it was simply never reached. The sweep is not
slow — it was blocked.

**Fix:** both tests marked `#[ignore]` with the cost in the reason string, and
the module docs now carry the `--ignored` invocation plus *why* the annotation
exists. This is not a new policy — it is the policy this bench already wrote
down, applied to the file that was missing it.

Pin verified to FIRE rather than assumed (a `--list` check proves nothing;
it lists ignored tests too). The same binary, same features:

    test dense_rerank_precision ... ignored, loads a 6.7 GB GGUF ...
    test probe_cost            ... ignored, loads a 6.7 GB GGUF ...
    test result: ok. 0 passed; 0 failed; 2 ignored; finished in 0.00s

**15+ min and 15.5 GB -> 0.00 s.**

Layer 5d is unaffected: it fails a `tests/*.rs` target that ran **zero**
tests, and an all-ignored target reports `0 passed; 2 ignored`, not a silent
empty binary. Worth stating explicitly, because "ignore everything" would
otherwise be a way to launder a dead target past 5d — it is not, and 5d
should keep it that way.

**Layer 5c bounded, and the bound verified to fire.** `#[ignore]` fixes any
individual offender, but nothing ENFORCES the annotation — so the layer now
runs under `timeout` and, on expiry, prints the last `Running` target it
started. That turns "hangs forever, cause unknown" (which cost four sessions)
into a named diagnosis. Budget `LAYER_5C_TIMEOUT_SECS`, default 3600 s against
a measured healthy run of ~30 min: it is a WEDGE detector, not a perf gate,
and the failure text says so ("mark it `#[ignore]` ... do not raise this budget
to accommodate it").

Verified against a stubbed `cargo` on `PATH`, exercising the SHIPPED block
(extracted by `sed` from the real script, not a retyped copy) — all four arms,
because a guard that only works on the happy path is the bug this bench is about:

| arm | expected | result |
|---|---|---|
| stub wedges past the budget | FAIL, naming the wedged target | ✓ named `tests/bonsai_retrieval_probe.rs` |
| stub passes | pass through, exit 0 | ✓ |
| stub fails (exit 101) | **propagate 101**, not masked by `tee` | ✓ 101 |
| no `timeout(1)`/`gtimeout(1)` | WARN + run unbounded, exit 0 | ✓ |

The third arm matters most: `| tee` would otherwise return tee's status. It
does not here — the script sets `pipefail` and the new code reads
`PIPESTATUS[0]` — but that was CHECKED rather than assumed, since a guard
that swallows real test failures is strictly worse than no guard.

**First complete Layer 5c run (2026-08-29), the payoff.** With the probe
annotated, the sweep finished for the first time: **57 targets, 1924 passed,
24 ignored, 2 failed.** One failure is the SIGTERM from killing the probe
mid-run. The other is real and had never been observable:
`t33_tether_g4_alloc` — `t33_g4_fit_never_allocates` (**332 allocs, expects
0**) and `t33_g4_tether_mode_adds_zero_steady_state_allocs` (**default 69958
vs tether 83785, +13827**). Filed separately; note that this binary's own docs
specify `--release` and this run was DEBUG, so the debug/release question is
resolved in that issue rather than asserted here.

**Standing hazard, not a one-off.** `--all-features` is a *superset* switch:
any future feature that gates a weights-loading or hours-long test enters
Layer 5c automatically and silently. The annotation is the only defense, and
nothing enforces it. Siblings (`riir-ai`, `riir-train`) carry many
GGUF-loading test files; whether each is annotated was NOT audited here and
is left to those repos' owners rather than swept blind across concurrent
agents' work.

## The fix

**Layer 5c — no allowlist.** `cargo test --all-features --tests`. Cargo's
`--tests` selects every target with `test = true` (lib unittests, bin
unittests, and all integration tests), so the layer cannot rot: a new
`tests/*.rs` file is in CI the moment it exists. What stays *out* is now
decided by `#[ignore]`, in the test file, next to the reason (network,
model weights, hours-long) — which is where that decision belongs. A shell
script is not the right home for it.

**Layer 5d — the zero-test detector.** The one failure `--tests` cannot
catch alone: a test file gated on a feature `--all-features` does not turn
on compiles to an **empty binary** and reports `running 0 tests` — a green
PASS that proves nothing (the cfg-gated-silent-pass class). The old
allowlist could never have caught this either; it names targets, not
outcomes. Layer 5d parses the actual run and fails on any `tests/*.rs`
target that ran zero tests. No list to maintain.

**Layer 5d verifies its own instrument first.** The parser reads cargo's
human-readable output, which is not a stable API. If that format drifts the
awk matches nothing and the layer prints a cheerful green — a check that
silently stopped checking, which is the exact bug class the layer exists to
catch. So before trusting the verdict, 5d asserts

```
count('Running tests/' lines in the run) == count(top-level tests/*.rs on disk)
```

and hard-fails on disagreement. `-maxdepth 1` is load-bearing: cargo
auto-discovers `tests/*.rs` and `tests/*/main.rs` only, so `tests/fixtures/`
must not count toward the expectation.

Validated on a synthetic tree (3 targets + 1 nested fixture) against two
logs: a healthy run (instrument 3/3, ghost `tests/b.rs` correctly flagged,
fixture correctly ignored) and a format-drifted run (`INSTRUMENT FAIL
disk=3 observed=0` — a red, not a false green).

Scoped to `tests/*.rs` by the match itself, not by an exemption list: a bin
with no `#[cfg(test)]` module legitimately reports zero. Ignored tests
count toward `running N`, so an entirely-`#[ignore]`d suite is not flagged.

Parser validated against a synthetic log covering all four cases (populated
lib, legitimately-empty bin, populated integration test, ghost integration
test, all-ignored suite) — flags exactly the ghost.

## The perf finding the measurement handed over

Running the full surface exposed that the guard was already paying its
dominant cost **twice**. Measured (`CARGO_TARGET_DIR=/tmp/ci_cov`, warm,
M3 Max, 2026-08-28):

| target | tests | wall |
|---|---|---|
| `unittests src/lib.rs` (all-features) | 1505 | **894.78 s** |
| `unittests src/bin/heal.rs` | 7 | 0.00 s |
| `unittests src/bin/fixer_regate.rs` | **0** | 0.00 s |
| `unittests src/bin/kat_drain.rs` | 14 | 0.00 s |

`--tests` selects every target with `test = true` — lib unittests and bin
unittests included, not just `tests/*.rs`. So Layer 5b's
`cargo test --lib --all-features` runs the *identical* 894.78 s binary that
Layer 5c already runs. **Removed from 5b** (~15 min saved per guard run);
`cargo test --lib` (default features) stays, since 5c is all-features only.
The fail-fast property 5b provided is preserved because cargo runs lib
unittests first within 5c anyway. The `--bins` flag was dropped from 5c for
the same reason — `--tests` already covers them.

Net: the guard now runs **51 test binaries instead of 12**, and is *cheaper*
than before by one 15-minute lib run.

`fixer_regate` running **0 tests** is the measured justification for Layer
5d's scoping: it is a bin with no `#[cfg(test)]` module, legitimately empty.
Had 5d matched bins, it would have false-FAILed on the first real run.

## What the allowlist did and did not hide

Precision matters here: Layer 5a (`cargo clippy --all-targets
--all-features`) selects `--lib --bins --tests --benches --examples`, so
all 51 targets were already **type-checked**. The allowlist hid **execution**,
not compilation. A suite outside it could not rot into non-compiling — but it
could rot into *failing*, or into asserting nothing, and no layer would say
so. That is the gap Layer 5c closes and Layer 5d backstops.

## A hand-built list makes hand-built-list mistakes

While assembling the validation run I built the target list with
`git ls-files 'tests/*.rs'` and got **51**, not 48 — git's pathspec `*`
matches `/`, so it swept in `tests/fixtures/{dot_per_row_prefix_698,
dot_per_row_postfix_698,kat_meter_sample}.rs`. Those are **fixtures, not
targets**: cargo auto-discovers integration tests as top-level `tests/*.rs`
(plus `tests/*/main.rs`), so it never built them, and passing their names to
`--test` is a hard cargo error.

`--tests` gets this right for free. The bug is only reachable by
maintaining a list — which is the argument for Layer 5c in miniature, made
accidentally, against the person making it.

## Validation status — what is proven, and what is not

**Proven:**
- The audit counts (12 / 51 / 39) are exact, computed from
  `git show HEAD:scripts/ci_feature_guard.sh` against `ls tests/*.rs`.
- Layer 5d's parser and its instrument check are validated on synthetic
  logs covering every case: populated lib, legitimately-empty bin,
  populated integration test, ghost integration test, all-`#[ignore]`d
  suite, nested fixture, and a drifted output format.
- The perf claim is measured, not inferred: the all-features lib binary is
  1505 tests / 894.78 s, and it was being run twice.
- The bin-exclusion decision in 5d is backed by real data (`fixer_regate`
  measured at 0 tests).
- `bash -n` clean.

**NOT proven: that all 48 committed integration suites pass under
`--all-features`.** 36 of them have not executed in the guard for a long
time, so some may fail. The run was attempted three times and abandoned:

1. Orphaned and killed at session teardown (5 binaries in).
2. `nohup setsid` — `setsid` does not exist on macOS; died into `/dev/null`.
3. Relaunched successfully, then **stopped deliberately**: the box was at
   load average 54/99/88 from concurrent sibling agents. Under that load,
   timing-sensitive suites (`probe_cost`, `dense_rerank_precision`, which
   were the ones hanging) fail for load reasons rather than code reasons,
   so neither a green nor a red would have meant anything — and continuing
   would have degraded the sibling's in-flight `g2_scan_latency` gate.

**The first full guard run should happen on an idle box, and it may
legitimately red.** If it does, that is the change working: a suite that
has not executed in months failing is a finding, not a regression
introduced here. Layer 5a already type-checks all targets, so no
compile-level surprise is possible — only behavioural ones.

A separate blocker for a full run today: `tests/docker_t5_control.rs`
(untracked sibling WIP) does not compile — `st.keyword` is used as a field
where it is a method. Not this change's concern and not this change's to
fix; the 48-target validation deliberately scoped to committed files only.

## Honest scope

- **The guard is a local pre-land gate, not GitHub Actions, and structurally
  has to be.** `Cargo.toml` carries `[patch.crates-io]` pointing at
  `../riir-ai/vendor/*` plus path deps on `katgpt-core`; the only workflow in
  `.github/workflows/` is `ops_dashboard.yml`. Running the guard in hosted CI
  would need three private sibling checkouts. This change makes the gate
  *complete*; it does not make it *automatic*.
- **`docker_class_a_agreement` SKIPs when `hadolint` is absent** and prints
  "NOT a pass". That is the right behaviour for an incumbent-oracle gate, but
  a skip is still not a pass — CI hosts that lack `hadolint` get no signal
  from it. Unchanged by this work, noted so it is not mistaken for coverage.
- Doc-tests remain outside the guard (unchanged; `src/lib.rs` has none).
