# Bench 143 — Issue 213 G2 (`shader_perf` behind the DomainMask path gate) + Issue 214 G5 (the `@compute` fold fix) — bmo cost runs

**Status:** COMPLETE 2026-11-07 — **both PASS all 3 interleaved rounds**. 213 → `shader_perf` PROMOTED (default + shipped release set); 214 → `@compute` KEPT in `SIGNAL_NEEDLES`. Session: katgpt-rs-f5@m3.

## Where it ran, and why not the queued amo jobs

The pre-registered rig jobs (amo `37f5c2b5` for 213, `b1a77a88`/`01a1b3da` for 214) sat queued Exclusive behind a 4h+ Shared tenant on amo; tailscale to bmo was down. Owner directive 2026-11-07 ("use jumphost amo->bmo"): bmo reached over the amo↔bmo thunderbolt bridge (`ssh -J amo katopz@192.168.10.2`). The amo jobs were CANCELLED by full UUID before either produced a number — **this document is the only G2/G5 record for both issues** (Issue 213's "cite `37f5c2b5` only" rule existed to exclude a stale duplicate; the superseding run is named here instead, no number from any other run is quoted).

bmo has no GitHub egress: the run commits were pushed from m3 through the jump (`i213-g2` = `206a9ebf1`, `i214-head` ⊇ `23e48374`); `riir-ledger` (a path dep, absent on bmo) was seeded from m3 at `f4f5bfa`. Detached worktrees `~/git/wt-i213` @ `206a9ebf1` and `~/git/wt-i214` @ `23e48374` (siblings must be `../X`); both worktrees `git status` clean after the run (Cargo.lock untouched).

## PROVENANCE

| | 213 G2 | 214 G5 |
|---|---|---|
| start (UTC) | 09:36:34 | 09:46:35 |
| end (UTC) | 09:46:35 | 09:56:37 |
| load at start / end | 2.51 / 1.68 | 1.68 / 1.84 |
| power | AC, lowpowermode 0 | AC, lowpowermode 0 |
| RAM | 96 GiB (M5 Ultra) | same |
| concurrent heavy jobs | none (top CPU: WindowServer/fseventsd < 35% one core) | none (macmon 2%) |
| toolchain | cargo/rustc 1.99.0 (rust-toolchain pin) | same |
| siblings | katgpt-rs `d3c090f7f`, riir-ai `3e41a13bd`, riir-infer `78c2047`, riir-kat `9752fa9`, riir-ledger `f4f5bfa`, riir-llm `0e61f04`, riir-neuron-db `02f7be2`, riir-reflexer `8cad872` | same |

Both arms of each A/B share every sibling — the sibling set moves both arms together, never the delta.

## 213 G2 — `scripts/g5_domain_cost.sh --feature shader_perf` (verbatim, 3 interleaved rounds)

| round | added p50 ms/file (support) | added p95 ms/file (support) | files > 1 ms | walk OFF / ON ms | verdict |
|---|---|---|---|---|---|
| 1 | 0.086 (306) — 11.6× headroom | 0.934 (32) — 5.4× | 27/611 | 16253.2 / 16406.8 | PASS |
| 2 | 0.109 (306) — 9.2× | 0.798 (32) — 6.3× | 17/611 | 16296.4 / 16448.3 | PASS |
| 3 | 0.170 (306) — 5.9× | 0.973 (32) — 5.1× | 31/611 | 16384.3 / 16570.2 | PASS |

`cost leg [shader_perf]: PASS over 3 interleaved round(s)` — ON-only build-stamp feature `shader-perf` every round. Against Bench 140 (pre-gate, LOADED amo): p50 1.10–1.28 → 0.09–0.17, p95 5.30–5.92 → 0.80–0.97. ⚠ The rig differs (quiet bmo vs loaded amo) — the before/after ratio mixes the gate with the box; the bar verdict does not (it is absolute, per run). G3 (retention diff, Issue 213 T3 `c56c11e6`) PASSED earlier — both legs green ⇒ T4 promotion fires (verdict round 1: no owner gate when both pass).

## 214 G5 — OLD fold (`8e781fce0^` kernel_pregate.rs) vs NEW fold (HEAD `23e48374`), 3 interleaved rounds

Binaries: OLD sha256 `b49bfd65…8a73e5`, NEW `9d36c75d…5acfc`; identical compiled feature stamp (`latent_retrieval` + defaults).

| round | added p50 ms/file | added p95 ms/file | files > 1 ms | walk OFF / ON ms | verdict |
|---|---|---|---|---|---|
| 1 | 0.025 — 40× | 0.351 — 14.2× | 7/611 | 16445.0 / 16472.5 | PASS |
| 2 | 0.001 — 1000× | 0.403 — 12.4× | 15/611 | 16410.8 / 16388.6 | PASS |
| 3 | −0.055 — ∞ | 0.117 — 42.7× | 0/611 | 16562.1 / 16474.6 | PASS |

`cost leg [kernel_compute_needle_fix]: PASS over 3 interleaved round(s)` — the fold fix is cost-neutral within noise (round 3 is NEGATIVE added cost). Utility: total spans 4422, kernel spans 680, 15/15 rank-1 experts both arms.

### Instrument amendment (disclosed, one commit with this record)

The queued command passed `--feature kernel_compute_needle_fix` to `g5_cost_math.py`, which REFUSES unless the ON stamp carries that feature — an A/B whose axis is a SOURCE change can never satisfy it (the run reached the math step and refused: `round 1 binaries do not differ by kernel_compute_needle_fix`). Fix: `--code-axis LABEL` — the guard INVERTS instead of being skipped: the two stamps must be IDENTICAL (a feature flip cannot masquerade as a code A/B), and the binary digests are disclosed beside the verdict (above). Negative arm proven: `--code-axis` over the 213 OFF/ON pair (stamps differ by `shader-perf`) REFUSES. The arithmetic and ceilings are untouched.
