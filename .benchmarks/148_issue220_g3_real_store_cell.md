# Bench 148 — Issue 220 (c) G3: the adoption-gate REAL-store cell (katgpt-rs plan 633 T3)

**Status:** EXECUTED 2026-11-10 (m3; instrument `examples/adoption_g3_cell.rs`, release build; two full runs byte-identical). **Verdict: honest split — G3-b and G3-c FAIL as pre-registered, G3-d PASS, G3-a PASS as NOT-VACUOUS (not a discrimination measurement). `adoption_gate` STAYS OPT-IN — no promotion.** Delegated-owner Claude verdict: REVISE, all four revisions applied (recorded below, session `3e68878e-6d79-42e1-88ad-71167e81eccf`).

## Cell identity + pre-registration

Bars were written into `.issues/220_adoption_gate_consumer_intake.md` (issue removed at close; the file's full life is `git log --follow`) BEFORE any run — the pre-registration evidence is the claim commit **`ad0711680`** (2026-11-10T18:33Z, lease `zed-m3-i220-g3cell` epoch 1794335612). The instrument commit is **`4038536c6`** (report-only example; seeded xorshift64 per (p, arm, seed), no global RNG; never writes; `from_rows` replay determinism self-check). The five real windows: riir-dao 2 rows, riir-auth 3, riir-grams 1, riir-instinct 21, riir-deployer 6 = **33 rows / 30 accepted / 3 rejected / 17 tags / 0 seat rows**, accumulated by pre-work (b) (per-repo fix runs with `adoption_gate` armed; kept edits under non-clean verify record nothing — the P104 posture — so instinct's 30-fix run contributed its 21 revert rows only).

Scorer facts (katgpt-core `adoption_gate`, producer landed `cf65c58a7`): every 5th NEW accepted outcome → audit ring (out of T); all rejected → T-rejected; `score = (1−α)·retained + α·(1−repeated)`; incumbent `retained ≡ 1`, `repeated = rej/(acc+rej)`; ties → challenger; NoEvidence → keep incumbent. Consult mechanics (T1 wiring): demote Refuse = challenger < incumbent; readmit Refuse = the rule's own ring-filed rows include a rejected digest. Gate-off baseline = the pre-gate posture (Issue 102 machinery): every consult proceeds.

**Box state:** m3, AC power (battery 99% finishing charge), system memory free 95%, load 3.43/6.53/9.36 (1/5/15 — decaying sibling-lane builds; this cell is a decision-correctness read, not a latency bar; the scorer is exact integer/f64 arithmetic over 33 rows), 2 cargo/rustc processes (sibling lanes, isolated target dirs), pinned toolchain 1.99.0.

## Results at the producer default α = 0.5

Verdict census (34 consults over 5 windows): **14 demote REFUSE / 2 readmit REFUSE / 1 ABSTAIN / 17 PROCEED** (of which 1 exact-tie). Decisive rows:

| consult | verdict | retained | repeated | score | note |
|---|---|---|---|---|---|
| instinct demote `vec-with-capacity` | REFUSE | 0.3750 | 0.1429 | **0.6161** | incumbent 0.9706; the rule carries 12 accepted rows — the off posture would have demoted the window's dominant accepted carrier |
| instinct demote `collapsible_match` | PROCEED | 1.0000 | 0.0000 | 1.0000 | its lone row IS the rejected one; demoting raises the score |
| instinct demote `hashmap-with-capacity` | PROCEED | 1.0000 | 0.0588 | 0.97059 | **EXACT TIE vs incumbent 0.97059 → tie→challenger** (its row is audit-filed only) |
| instinct readmit `collapsible_match` | REFUSE | 0.0000 | 1.0000 | 0.0000 | repeated = 1 — the verify-revert class readmit would repeat |
| instinct readmit `hashmap-with-capacity` | ABSTAIN | — | — | — | empty rule slices (audit-filed only) — the abstain path on real data |
| deployer demote `explicit_auto_deref` | PROCEED | 1.0000 | 0.0000 | 1.0000 | both its rows were reverts — demoting the reverted rule raises the score (the gate's second reading) |
| deployer readmit `explicit_auto_deref` | REFUSE | 0.0000 | 1.0000 | 0.0000 | repeated = 1 |
| grams demote `manual_is_multiple_of` | REFUSE | 0.0000 | 0.0000 | 0.5000 | empty challenger on a non-empty window — NoEvidence is decided on the INCUMBENT rings, not challenger emptiness |
| dao/auth demotes (4) | REFUSE | 0.3333–0.6667 | 0.0000 | 0.6667–0.8333 | incumbent 1.0 (zero-rejection windows) |

## The bars, as measured

### G3-a — PASS as NOT-VACUOUS (reworded per the delegated-owner verdict; NOT a discrimination measurement)

The gate is not vacuous against the always-proceed posture: it refuses 16 of 17 destructive-ish consults that the off posture would have allowed (the 17th is the exact-tie PROCEED — the bar's "refuses every destructive consult" wording is corrected: ties are excluded, so the honest count is 16/17 with the tie named). ⛔ **This is NOT evidence of discrimination:** the only baseline measured is "always proceed"; against the law's own default (keep incumbent = refuse destructive consults by default), the gate's ONLY divergence on real data is that single tie-PROCEED — which is the most questionable verdict, not the strongest (it demotes a rule carrying 1 accepted row on a tie at the DEFAULT α). Real discrimination (refusing exactly the harmful demotes) needs labelled per-consult harm outcomes — unmeasured here. The gate's refusal explanations all carry scorer numbers matching hand-recomputation.

### G3-b — FAIL as pre-registered (bar: zero α flips; measured: 5)

All 5 flips are ONE derived class: on zero-rejection windows (dao, auth, grams) `inc_rep = 0`, so `d(α) = (1−α)(ret−1) + α(inc_rep − rep)` vanishes at α=1 → exact tie → the pinned tie→challenger rule ADOPTS → demoting the only productive rule in a clean window PROCEEDS at the α=1 endpoint. Empirically confirmed (`--alpha 1` rerun: dao demotes PROCEED, score 1.0000 both sides). d(α) is linear in α so each consult crosses at most once — the class is fully derived, not just observed. **Zero flips at α=0.5 across all 34 consults.** The wider reading (delegated owner): the problem is NOT endpoint-only — tie→challenger on a destructive consult conflicts with decline-first at ANY α, because an exact tie is a no-evidence state and no-evidence already means keep the incumbent. Routed to katgpt-rs **issue 994** (tie rule on destructive consults: change everywhere vs endpoint guard — to be PRE-REGISTERED for the next cell, never patched into this one).

### G3-c — FAIL as pre-registered (bar: dominant refusals ≥ 0.90 stable at p ≤ 0.2)

Dominant-carrier demote-refusal stability per window (32 seeds × p ∈ {0.1, 0.2} × both arms; instabilities concentrate in the acc→rej arm — flipping the rule's own accepted row legitimately moves the challenger's retained numerator and files the digest into the rejected ring):

| window | dominant refusal (accepted rows carried) | p=0.1 | p=0.2 |
|---|---|---|---|
| instinct | demote `vec-with-capacity` (12) | **1.000** | **1.000** |
| auth | demote `unnecessary_get_then_check` (2) | **1.000** | **0.906** |
| grams | demote `manual_is_multiple_of` (1) | 0.969 | 0.781 |
| dao | demote `doc_lazy_continuation` (1) | 0.844 | 0.688 |
| deployer | demote `collapsible_match` (1) | 0.844 | 0.688 |

The flips are semantically correct (the verdict follows the evidence — a flipped judge verdict changes the challenger composition); the fragility is **EVIDENTIARY, not mechanical**. Leave-one-out: the instinct vwc refusal survived ALL 21 single-row flips; the 1-row refusals flip whenever their own row flips.

**Quotability — by the MEASURED quantity, not a row-count proxy** (delegated-owner revision; the run refutes a row-count line: auth's 2-row window meets the bar): a consult verdict is quotable as gate evidence iff its own noise stability ≥ 0.90 at p ≤ 0.2. Today: **2 of 34 consults are quotable** (instinct demote `vec-with-capacity`, auth demote `unnecessary_get_then_check` — min stabilities 1.000 and 0.906); the other 32 are not. Re-run the noise leg as the windows grow; per-consult, never pooled.

### G3-d — PASS

5 windows (≥5), 17 tags (≥11), both refusal directions on real rows, 1 abstain on real data, 1 exact-tie proceed. No green zero.

### Determinism

Two full runs byte-identical (`cmp` clean). Seat-swap leg: **0 seat rows in every window — UNMEASURED on real data** (fixture gates cover the mechanics); it runs when armed seat traffic exists.

## Pin corrections (disclosed, post-run, with reasons — never silent)

1. dao/auth demote pins quoted RETAINED values as scores (0.5/0.1667/0.3333; actual scores 0.75/0.6667/0.8333). The REFUSE verdicts themselves were pinned correctly. **Instrument matched the scorer exactly; the error was hand arithmetic.**
2. **VERDICT-SHAPE correction (the only one): grams demote pinned ABSTAIN, measured REFUSE.** Mechanics misread: `NoEvidence` is decided on the incumbent window's rings (both empty), not on challenger emptiness — demoting the only rule in a 1-row window leaves a challenger that retains nothing → maximal regression → refuse. The abstain path IS exercised on real data elsewhere (instinct readmit `hashmap`).
3. instinct demote `shrink-to-fit-after-build` pinned ≈ 0.8785, measured 0.8705 (ring-filed output count miscounted: 14, not 18).
4. α-stability pin (zero flips) wrong as described in G3-b — the derivation missed the zero-rejection tie class; the count (5) and the class were derived exactly once measured.

12 of 13 pinned verdict shapes matched exactly pre-correction.

## Findings (the durable content)

1. **The gate discriminates correctly at the default posture on evidence-rich rules — and its headline refusal is noise-immune.** The dominant-carrier refusal (12-row evidence) is 32/32 stable under every noise arm and survived every single-row flip. The 2-row auth refusal meets the 0.90 bar. Refusals on 1-row evidence are not quotable at the bar.
2. **Tie→challenger on destructive consults is the real defect surface** (delegated owner's widening): an exact tie is uninformative; the default answer must be keep-incumbent. Measured at the DEFAULT α (the hashmap tie-PROCEED demotes a rule carrying an accepted fix on zero evidence of harm). Producer-side: katgpt-rs issue 994 — decide tie→incumbent-everywhere vs endpoint-guard, pre-register the chosen rule for the NEXT cell, never patch into this one.
3. **Decontamination holdout churn under noise** (leave-one-out disclosure): flipping any accepted verdict shifts the accepted-count sequence, so every-5th-audit membership moves for downstream rows (measured: instinct's audit-filed `hashmap` readmit flips ABSTAIN→PROCEED when ANY upstream accepted row flips). Never-double-file still holds per replay. Optional hardening in issue 994: key the holdout by `digest mod 5` instead of position — membership stops shifting; does not block this verdict.
4. **Seat leg unmeasured** (0 seat rows); fixture gates cover the mechanics.
5. **The outcome windows are the constraint.** 33 rows across 5 repos with 3 rejections is a thin store; the cell's honest consequence is a growth loop — accumulate real fix verdicts per repo, re-run the noise leg per consult.

## Delegated-owner verdict (recorded)

**REVISE** (Claude, 2026-11-10, session `3e68878e-6d79-42e1-88ad-71167e81eccf`) — all four revisions applied: (1) G3-a reworded to not-vacuous + tie excluded + the discrimination caveat; (2) issue 994 widened to the tie rule generally + optional digest-keyed holdout; (3) quotability by measured per-consult stability, not row count; (4) **T4 (evidence-sealing sidecar) DEFERRED by the delegated owner now** — no gain and no GOAT result to protect; the deterministic instrument + this record already reproduce the evidence. The reviewer noted it reviewed only the summary, did not re-run the cell or read the instrument.

## Consequences

- `adoption_gate` stays **opt-in**; no promotion (the ⛔ law holds — two bars failed).
- Consult quotability is per-consult measured stability ≥ 0.90 at p ≤ 0.2; today instinct-vwc + auth-unngc only.
- katgpt-rs plan 633: T1 + T3 checked with this record; **T4 `- [-]` deferred** with reopen trigger (a later G3 re-run that could support promotion).
- Re-run triggers: the noise leg when dao/deployer/grams windows grow; the seat leg on armed seat traffic; the full cell re-run after the issue-994 tie-rule decision lands (pre-registered then).
- Issue 220 is CLOSED (removed per the noise-reduction rule; this record + git history carry it).
