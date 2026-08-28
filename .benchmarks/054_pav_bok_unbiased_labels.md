# Bench 054 — Plan 356 A2: the BoK label is an estimator, and the plan's sketch was the biased one

**Status:** MEASURED 2026-08-28 — **defect found and fixed.** The Bo-K
relabel shipped as a *plug-in* of the MC point estimate, which is
systematically biased LOW by up to **14.6 percentage points** in this plan's
own low-budget regime. Replaced with the unbiased `pass@k` U-statistic over
the raw Bernoulli counts: worst-case |bias| **0.1461 → 0.0057** (a 26×
reduction, and the residual is pure Monte-Carlo noise).

Parent: riir-train [Plan 356](../../riir-train/.plans/356_pav_process_advantage_edit_scorer.md) A2 ·
substrate `src/pav_data/mod.rs` (`bok`, `bok4`, `bok_unbiased`, `BOK_K`) ·
gate `bok_unbiased_is_unbiased_where_the_plugin_is_not` (lib tests)

```bash
cargo test --features pav_data --lib bok -- --nocapture
```

## The defect

Plan 356 A2 specifies the relabel as `Q^{Bo4} = 1 − (1 − Q̂)^4`, and the A1
harness implemented exactly that (`bok4(q_hat)`).

That formula is the correct **definition** of Bo-K in terms of the *true*
per-rollout success probability `Q`. It is the wrong **estimator** when what
you have is a noisy `Q̂ = s/n`, because

```
f(q) = 1 − (1 − q)^k     ⇒     f''(q) = −k(k−1)(1−q)^(k−2) < 0   for k ≥ 2
```

is strictly concave, so by Jensen `E[f(Q̂)] < f(E[Q̂]) = f(Q)`. The plug-in
can only read low, and the error grows as `n` shrinks.

**This is not a corner case for this plan — it is the operating point.** A1's
low-budget regime is `n_mc` 3–8 with `n_cov > n_mc`, and the bias peaks in
the mid/low-`Q` band, which is exactly where PAV's advantage signal
(`Q̂(after) − Q̂(before)`) lives. A label that is compressed downward
unevenly across the `Q` range distorts the very differences A3 trains on.

## The fix

With `s` successes in `n` iid samples, the probability that a uniformly
random `k`-subset of those `n` contains no success is `C(n−s, k)/C(n, k)`. A
uniform `k`-subset of `n` iid Bernoulli(Q) draws is itself `k` iid
Bernoulli(Q) draws, so that expectation is exactly `(1−Q)^k`. Hence

```
BoK̂ = 1 − C(n−s, k)/C(n, k) = 1 − prod_{i=0}^{k−1} (n−s−i)/(n−i)
```

is **exactly unbiased** for `1 − (1−Q)^k`. (The standard `pass@k` estimator;
the product form is used so no factorial is materialised.) Sanity anchors,
all pinned as tests: `k=1` returns exactly `Q̂`; `n=k` reduces to the
indicator `1{s ≥ 1}`; `s=0` is exactly 0; fewer than `k` failures is
exactly 1.

## Measured (K=4, 40 000 trials/cell, seeded splitmix64)

| n | q | truth | unbiased | bias | plug-in | bias |
|---:|---:|---:|---:|---:|---:|---:|
| 4 | 0.05 | 0.1855 | 0.1878 | +0.0023 | 0.1318 | **−0.0537** |
| 4 | 0.10 | 0.3439 | 0.3432 | −0.0007 | 0.2480 | **−0.0959** |
| 4 | 0.20 | 0.5904 | 0.5906 | +0.0002 | 0.4510 | **−0.1394** |
| 4 | 0.30 | 0.7599 | 0.7608 | +0.0009 | 0.6138 | **−0.1461** |
| 4 | 0.50 | 0.9375 | 0.9374 | −0.0001 | 0.8332 | **−0.1043** |
| 4 | 0.70 | 0.9919 | 0.9920 | +0.0001 | 0.9490 | −0.0429 |
| 5 | 0.20 | 0.5904 | 0.5961 | +0.0057 | 0.4813 | **−0.1091** |
| 5 | 0.30 | 0.7599 | 0.7601 | +0.0002 | 0.6411 | **−0.1188** |
| 8 | 0.20 | 0.5904 | 0.5935 | +0.0031 | 0.5201 | −0.0703 |
| 8 | 0.30 | 0.7599 | 0.7603 | +0.0004 | 0.6849 | −0.0750 |
| 16 | 0.20 | 0.5904 | 0.5907 | +0.0003 | 0.5532 | −0.0372 |
| 16 | 0.30 | 0.7599 | 0.7599 | −0.0000 | 0.7217 | −0.0382 |

(Full 24-cell sweep — `n ∈ {4,5,8,16} × q ∈ {0.05,0.1,0.2,0.3,0.5,0.7}` —
prints under `--nocapture`; the rows above are the extremes plus the
`n`-decay column.)

**Worst |bias|: plug-in 0.1461, unbiased 0.0057.**

Three things the sweep settles:

1. **The bias is large where it matters and does not vanish at realistic n.**
   Even at `n=16` — twice the plan's `n_mc` ceiling — the plug-in still reads
   3.7–3.8 pp low across the mid band.
2. **It is one-signed.** Every one of the 24 cells reads low, as Jensen
   requires. That makes it a *systematic distortion* of the label scale, not
   noise that averages out over a dataset.
3. **The unbiased arm's residual is MC noise, not bias.** Its errors change
   sign across cells and shrink with trial count; the largest (+0.0057) is
   within 3σ of zero for a [0,1]-bounded estimator at 40 k trials.

## What changed in the code

- `bok(q, k)` — the closed form, generalised in `k` (A4 beams at `K=4`; the
  label must be the value of the same `K` the server uses). `BOK_K = 4`.
- `bok_unbiased(n_success, n_samples, k)` — the U-statistic. Documented
  contract for the three degenerate inputs: `n < k` falls back to the
  plug-in *and says so* (the U-statistic does not exist below `k` samples),
  `n == 0` is no-evidence, `k == 0` is vacuous.
- `PavPair.q_bo4` is now **the unbiased label**; `PavPair.q_bo4_plugin`
  carries the old closed form as a **diagnostic**, so the bias stays
  auditable in the dataset instead of being an invisible modelling choice.
- `PavManifest.schema_version = 2` + `bok_k`. The bump is the point: a
  consumer that reads a v1 row as a v2 label trains on systematically low
  targets, and that must be loud rather than silent. v1 was never released
  with real data.
- The clean-root anchor (`q = 1`, zero rollouts) keeps the closed form
  deliberately — it is a definitional row, not an estimate, and
  `bok_unbiased` would correctly read `n = 0` as "no evidence".

## Verdict

**A2's transform is DONE and is now a measured improvement over the plan's
sketch**, not a transcription of it. The gate lives in the **lib** test
module on purpose, so `ci_feature_guard.sh` Layer 5b (`cargo test --lib
--all-features`) actually runs it.

Validation: `pav_data` lib suite 19/19; full lib suite **895 passed / 0
failed**; `cargo clippy --features pav_data --all-targets` clean over
`src/pav_data/` (the one narrowing cast is isolated in `narrow_prob` with a
stated reason).

## Honest residual — what this bench does NOT claim

It does not claim A2 is closed. A2's completion criterion in Plan 356 is
"real A1 estimates flow through it", and **no real estimates exist yet**:
A1's rollout campaign is the ~30–40 h daemon budget and both GPUs were
occupied at the time of writing (4090 at 100 % on the 760 campaign; the M3
at 99 % device utilisation running a sibling's *sustained-throughput A/B*,
which concurrent load would have invalidated). The BCL4 adapter A1 needs
does exist (`riir-train/models/plan341/v2_best.BCL4.bin`), so the campaign
is compute-blocked only.

What this bench does guarantee is that when those estimates do flow, they
land on a correct label — which is strictly better than discovering the
Jensen bias after spending the GPU budget and training A3 on it.
