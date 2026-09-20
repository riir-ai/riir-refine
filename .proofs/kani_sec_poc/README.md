# Kani sec-property POC (riir-clippy Issue 127)

Machine-checked property evidence for sec corpus rules, via Kani
(model-checking over CBMC). **This crate is deliberately standalone and
detached** (`[workspace]` marker above): it builds ONLY under `cargo kani`
and its pinned nightly toolchain — never under the workspace's stable
rustc, never in CI, never as a per-push gate (workstation/WSL dev-time
oracle only).

## How to run (the 4090 box's WSL2 Ubuntu 22.04 lane)

Windows-native Kani is unsupported; WSL is the sanctioned lane. Copy the
crate to WSL-native storage (the /mnt/e 9p mount is too slow for
compilation I/O), then:

```bash
# one-time (measured 2026-09-20):
cargo install --locked kani-verifier   # 8.6 s (warm cargo cache; cold is minutes)
cargo kani setup                       # 51.2 s — installs pinned nightly-2026-08-21

# per run, from the crate root:
cargo kani            # all 5 harnesses, ~2 s wall total
cargo kani --harness g1a_buggy_cmp_violates_transitivity_of_equal
```

Toolchain pin (the premise Issue 127's falsifier named): **Kani 0.68.0 /
CBMC 6.11.0 / rustc 1.100.0-nightly (2026-08-21)** — Kani's own snapshot,
independent of the workspace's `rust-toolchain.toml` stable pins. The
fixtures carry zero dependencies, so the pin never touches the workspace
graph.

## Measured results (2026-09-20, WSL2 Ubuntu 22.04, 24-core i7-13700K)

| harness | expectation | result | solver | wall |
|---|---|---|---|---|
| `g1a_buggy_cmp_violates_transitivity_of_equal` | FAILURE + witness | **FAILED** — `"comparator not total: Equal∘Equal must imply Equal"` | 0.046 s | 0.15 s |
| `g1b_total_cmp_equal_is_transitive` | SUCCESS | **SUCCESSFUL** | 0.060 s | 0.45 s |
| `g1c_total_cmp_is_total_antisymmetric_nan_self_equal` | SUCCESS | **SUCCESSFUL** | 0.074 s | 0.21 s |
| `g2a_buggy_wire_sum_reaches_overflow` | FAILURE + witness | **FAILED** — `"attempt to add with overflow"` inside the buggy callee | 0.050 s | 0.19 s |
| `g2b_checked_wire_sum_never_silently_wraps` | SUCCESS | **SUCCESSFUL** | 0.050 s | 0.44 s |

Kani-exhibited witnesses (`-Z concrete-playback --concrete-playback print`):

- **G1a violating triple** (f64 bit patterns): x = `-inf`
  (`0xFFF0000000000000`), y = `+NaN` (`0x7FFFFFFFFFFFFFFF`), z = `+inf`
  (`0x7FF0000000000000`) — cmp(x,y)=Equal, cmp(y,z)=Equal, cmp(x,z)=Less.
  Hand-checkable in one line; the exact "NaN in the middle, ordered ends"
  shape the rule predicts.
- **G2a overflow input**: a = b = `0xFFFFFFFF` (u32::MAX from all-0xFF
  bytes) — 2 × u32::MAX overflows the bare `+`.

The two expected-FAILURE harnesses are the point: a FAILED verification
with a concrete witness IS the property-violation evidence (existence);
the SUCCESS harnesses are the impossibility/totality side. Both directions
of both rules decided by the same oracle.

## Honest scope (what this POC does NOT say)

- **Floor, not ceiling**: these are loop-free, dependency-free, ≤6 symbolic
  variables. Solver times of 0.03–0.07 s say nothing about harnesses with
  loops (unwind bounds), heap structures, or workspace-real code — the
  "minutes per fixture" ceiling was never approached, so it remains
  unmeasured. No unwinding config was needed here.
- **No corpus edits** (Issue 127 non-goal): this validates the ORACLE
  mechanism and pre-validates the `unchecked-wire-arithmetic` lead; it does
  not add rules, doc columns, or gates.
- **WSL-only on this box**: the systemd user-session warning in WSL is
  cosmetic (bash -lc runs fine). Windows-native remains unsupported.
