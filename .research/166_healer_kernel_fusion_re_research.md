# Research 166 — Healer Kernel-Fusion Re-Research: Nonergodic 592 · Saddle 593 · GW 594 onto riir-clippy

**Status:** RECORD — three healer-surface verdicts filed (Issue 092 PoC-first; Issue 093 execution ticket; Issue 91 kernel-landed update); no code changed.

> **Origin:** owner challenge — riir-ai Research 371/373/374 were distilled with the healer surface recorded thin or deferred; fusion priority #2 demands the consumer read (the canonical-failure-#5 class: the fusion hunt that never reads the healer). All three kernels landed 2026-09-11: katgpt-rs Plan 592 `NonergodicFilter<K,D>` (`crates/katgpt-micro-belief/src/nonergodic.rs`, feature `nonergodic_belief`, Bench 706, G1–G4 PASS, opt-in), Plan 593 `SaddleEscapeGate` (`crates/katgpt-core/src/saddle_escape.rs`, feature `saddle_escape`, Bench 707, G1–G5 PASS, opt-in), Plan 594 `gw_alignment` (Gromov–Wasserstein quotient alignment, `crates/katgpt-core/src/gw_alignment/`, Bench 709, 11 gates green, opt-in).
> **Method:** three parallel subagents — codebase grep (selection math, driver loops, corpus/dedup, reference sweep) · published prior art (14 searches, claims pinned BEFORE searching) · kernel/note status sweep. Evidence cited file:line. Healer-state ground truth first, then per-claim verdicts.

## 0. Healer ground truth the fusions land on

- **Selection state is per-rule-global, cross-run.** `select_best_candidate` (`src/self_evolve.rs:503-507`): TETHER `(1-ρ)·evo + ρ·reliability` or `W_EVO·evo + W_RATE·reliability` (0.6/0.4); reliability = `best_belief_score` Beta counts. No per-run per-target belief anywhere in selection. The ONLY within-run state ships as **negative memory**: the `--fix-compile` `tried: HashSet<TriedKey>` (`domains/rustc_errors/driver.rs:337`) and the `--fix` P12 re-proposal suppression (`bin/heal.rs:6352-6359`) — suppression never revives.
- **The `--fix-compile` driver is the one true multi-attempt loop**: check → parse+dedupe → propose → order → apply best → recheck → keep|revert → repeat ≤ `max_iters` (default 5, `driver.rs:232-240`); keep iff coded-error count strictly decreased. Per-attempt order lives only in in-memory `FixCompileReport` Vecs — no persisted per-attempt ring (the Issue 91 T1 verdict).
- **Corpus dedup is name/convention-level.** No canonical-form, rule-equivalence, or cross-domain mapping machinery exists: dedup = index-level `zone_hash` (identical text only, `draft/corpus_index.rs:201-211`) + mining-time "dropped as duplicates" + in-rule "Distinct from …" cross-refs. Doc-stated corpora: clippy_lints 49 · rust_perf 126 · kernel_opt 607 · sec 15 · docker 31 · dist 5.
- **Cross-domain routing is scoped OUT at heal time** (`.docs/11_domains/cross_domain_routing.md`: "caller picks domain via feature flag … auto-detection would add complexity for a non-problem"; the spectral router is KILLED, Bench 025). That scoping covers the **hot-path router**, not **offline corpus curation** — the distinction §A turns on.

## A. 371/594 — cross-domain rule-corpus alignment (GW on rule-similarity graphs)

**Pinned claim:** align two healer domains' rule-similarity spaces (embed each rule's buggy/fixed code pair → per-domain similarity matrix → `gw_alignment`) → a cross-domain rule-correspondence map used for validated dedup + fix-space transfer + ingest-time duplicate screening — distinguished from name/keyword dedup by operating on structural trigger-shape/fix-transform similarity.

**Prior art (published):** NO PRIOR ART found for the full technique. Vendor-confirmed gap: SonarQube states "we have no 'similar rule' mapping"; eslint↔sonar mapping requests go unanswered; `eslint-plugin-sonarjs` is manual per-rule porting. Class cousins (narrowing only): heterogeneous CPDP transfer (project feature-space, not rule corpora), SemDeDup (generic embedding dedup). **In-stack:** zero hits for canonical/equivalence/orbit/gw_alignment machinery (grep sweep Q3/Q4).

**Signal-diff vs the recorded thin reframe:** riir-ai 371 §3 recorded the healer lens as "fix-trajectory memory — invariant fix-shape identity as the quotient" and parked it. The grounded version shifts the object: not trajectories but the **rule corpora themselves** (rule = section, domain = gauge, invariant fix-transform = the quotient) — and it now has an instrument (Plan 594 landed) the original reframe lacked.

**Verdict:** **Gain — fusion idea, novelty TBD on the capability axis** (Q1 ✓, Q3 ✓, Q4 ✓; Q2 — "the map is measurably truer than name-dedup" — is unproven and PoC-gated). Filed **Issue 092** (defend-wrong: GW map vs name/keyword baseline vs manual-only, hand-labeled validation subset, report-first). This is the 371-precedent shape (Issue 912 → design pass → PoC → decide) applied to the healer surface. riir-ai Issue 912 closed "consumer PoC deferred-until-need" for the NPC consumer — the healer corpus-curation need is a second, distinct consumer articulation, still PoC-gated.

## B. 373/592 — per-run strategy posterior (the 545 §4 healer half, now unblocked)

**Not a novelty claim.** katgpt-rs Research 545 §4 already records it verbatim: "spans within one file come from ONE domain 'generator' … yet `KernelExpertRouter::pick_domain` is stateless per-span argmax. A file-level domain posterior + per-domain inner state is the nonergodic upgrade of the healer's router. Deferred pending the kernel's GOAT." **That blocker cleared 2026-09-11** (592 G1–G4 PASS).

**Grounded deltas this re-research adds (the reason an execution ticket is now worth filing):**

1. **Domain-level → strategy-level reframe.** The domain-level reading partially collides with the routing doc's "non-problem" verdict (the caller picks the domain by feature flag). The promising surface is **within-run, within-domain**: the `--fix-compile` driver is inherently multi-attempt (max_iters=5) with per-attempt strategy/error-class observables — a `NonergodicFilter` over strategy/error-class generators sharpening across the driver's iterations, with 592's `revive_margin`/`revive_gate` turning today's never-reviving suppression sets into principled revive-on-contradiction.
2. **R135 interplay — design constraint, not objection.** Bench 047's starvation law (98.5% of candidates unseen) punishes selection sophistication at pool sparsity — but the filter learns **within the run budget** from THIS run's outcomes, which is exactly R135 lesson 2 ("any future rating-based selection must update online"). Report-first regardless (R135 doctrine), and per 373's own validation protocol: PoC before any wiring.
3. **Shared instrumentation.** The per-attempt error-code ring (Issue 91 T1.5) is the substrate for BOTH this posterior and Issue 91's trap detector — one instrumentation, two kernel consumers.

**Verdict:** **Gain (delta-recording + execution ticket).** Filed **Issue 093** (PoC-first, three competitors, report-first; wiring only behind an opt-in flag if the PoC wins AND the T1.5 ring lands). No new novelty asserted over 545.

## C. 374/593 — fix-loop trap detection (already tracked; kernel-landed update only)

**Already fully wired into tracking:** Issue 91 (`91_fix_loop_trap_detector.md`, T1 MINED 2026-09-11: unmeasurable with shipped stores — the trajectory kv holds per-family aggregates, fix-compile has zero recorded trajectories; T1.5 ring = the prerequisite) + katgpt-rs Research 546 §7 names it ("Filed PoC-first: riir-clippy Issue 91"). **No new filing.**

**Update applied to Issue 91:** Plan 593 landed (G1–G5, 5.3 ns/loop, 0 alloc) — T2's detector can consume `SaddleEscapeGate`'s observables (oscillation streak + flip-rate EMA) instead of a bespoke two-way counter; T3's kick-does-not-transfer verdict stands for discrete edits (the gate's Kick would need candidate-rotation semantics, only if T2 ever justifies); T1.5 doubles as Issue 093's substrate.

**Prior art (published, for the record):** NARROWS, survives the specifics — existing work detects identical-action repetition (tool-call fingerprints, agentic-loop surveys arXiv:2607.01641) or repairs reintroduced regressions post-hoc (Relifix); none detects a two-error flip-flop cycle within one run and escalates early.

## Verdict summary

| Surface | Kernel (all landed 2026-09-11) | Verdict | Filed |
|---|---|---|---|
| GW rule-corpus alignment (371/594) | `gw_alignment` (594, Bench 709) | Gain — fusion idea, novelty TBD on capability; PoC gates | Issue 092 |
| Per-run strategy posterior (373/592) | `NonergodicFilter` (592, Bench 706) | Gain — 545 §4 execution ticket (unblocked), NOT new novelty | Issue 093 |
| Fix-loop trap detection (374/593) | `SaddleEscapeGate` (593, Bench 707) | Already tracked — kernel-landed update | Issue 91 (edited) |

**The convergence finding is the session's main structural result:** one instrumentation prerequisite (Issue 91 T1.5 per-attempt ring) unblocks two independent kernel consumptions (593 trap gate, 592 per-run posterior). Sequence T1.5 first if either line is pursued.

## Track hygiene

Pure modelless across all three (Bayes filter, observable gate, quotient geometry) — no training content, no riir-train deferral, adversarial panel n/a (classification never touched training). Selection-adjacent claims respect the R135/Bench-047 report-first doctrine and the 525/Bench-834 opt-in-until-evidence precedent. Next free bench number: 083 (reserved by the PoC protocols in Issues 092/093).
