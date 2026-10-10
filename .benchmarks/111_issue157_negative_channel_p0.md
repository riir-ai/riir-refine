# Bench 111 — Issue 157 P0: the retrieval-negative channel's volume + near-tie census

**Date:** 2026-10-10
**Issue:** `.issues/157_span_conditional_contrastive_negatives_ruleindex.md` (the owner-gated triage's P0 probe)
**Verdict chain:** Claude verdict session `2b8a1316` rounds 1–3 — REVISE → REVISE (small) → **AGREE**: GO on P0 ONLY, with the amendments folded in (acceptable-vs-wrong split, span disjointness, UNPOWERED vocabulary, per-family retention gates for any future P1, pre-registered hyperparameters). P1 depends on P0 coming back neither DECLINE nor UNPOWERED.
**Posture:** M3 Max, macOS 26.6.2, rustc 1.98.1 (pinned), release profile. Census example `examples/p157_near_tie_census.rs` (print-only, writes nothing) at this repo's HEAD, features `latent_retrieval,clippy_lints,rust_perf,kernel_opt` + the crate's default-on `docker`/`dist` — a 5-domain posture (171 eval spans), disclosed because it is WIDER than the 3-domain floors gate. Store dump via the installed `cargo-refine 0.2.1` @ build `1e447f53` (some commits behind HEAD `d0eb681a` at dump time — the disclosure issue 139's re-measure used the same way; a store dump reads bytes, it executes no healer logic). Box state: battery/charging, sibling builds active (non-measurement unit — counts and store rows only, no latency claims).

## Pre-registrations (stated before the run, in the example's module doc)

- Margin X = a **scale-free fraction of the span's own raw fused top-1 pool score** (the pool is an unnormalized latent+BM25 similarity — Bench 018's raw [3.19..34.71] class — so an absolute threshold does not transfer). Fractions {5%, 10%, 25%}; **10% primary**; measured on the PRE-rerank pool (the surface a latent-dot demotion modifies), hit/top-1 identity from the shipped `canonical_rank` (instantiation-first).
- Kill criteria (the verdict chain, final form): (1) raw volume ~0 → DECLINE (no data); (2) sampled acceptable share > 50% (Wilson bounds three-way) → DECLINE; (3) negatives overlap no miss span's confusion set → DECLINE (vacuous); (4) near-tie count ~0 → DECLINE (demotion can't flip); (5) miss-set n too small → UNPOWERED, never a measured negative.

## P0.1 — volume census (the local trajectory store, `--dump-trajectories`)

| class | rows | notes |
|---|---:|---|
| total | 309 | `.refine/trajectories.kv`, this repo |
| **non-empty `rejected`** | **1** | 12 (row, rejected-rule) pairs, ONE span: `semicolon_if_nothing_returned` @ `src/kat_adopt.rs` (run `run-18dc9e9a68a9b398-*`) |
| known-empty (real run, no candidate set) | 308 | |
| unknown (`last_run_id == "migrated"`) | 0 | |

The `rejected` field landed 2026-10-08 (Issue 148 T-b) — **three days of accumulation**. Volume ~0 here is an artifact of field AGE, not of mechanism; the honest reading of criterion (1) is "the channel is **vacuous on current data**", not "the label source is dead".

The 12 rejected ids of the single row: `cache-mount-id-scope`, `chef-stage-toolchain-skew`, `diagnostics-not-just-exit-code`, `healthcheck-tool-absent-in-runtime`, `keyword-map-to-match`, `large-api-dependency-mostly-unused-hint`, `member-profile-ignored`, `pgo-training-set-covers-real-workload-hint`, `single_char_pattern`, `unset-control-arm-silent-default-parity`, `while_let_on_iterator`, `write_with_newline`.

## P0.2 — miss-set + near-tie census (171 spans, 5 domains)

**hits_at_1 = 91.2% (156/171). Miss set n = 15.**

| readout | value |
|---|---|
| near-tie at X=10% (primary) | **2** of 15 |
| near-tie at X=5% / X=25% | 2 / 5 |
| gold at rank 1 (adjacent — one demotion from top-1) | **4** of 15 |
| gold absent from the pool entirely (R3) | 4 |
| wrong top-1 instantiates (`inst=true`) | 0 of 15 |

Per-miss anatomy (domain, label ← wrong top-1, gold rank, margin as % of top-1's fused score):

```
Clippy  useless_conversion                ← identity_op                             rank 1   15.1%
Clippy  needless_borrow                   ← unnecessary_mut_passed                  rank 12  86.5%
Clippy  single_char_pattern               ← unnecessary_mut_passed                  rank 7   46.0%
Clippy  comparison_to_empty               ← manual_is_multiple_of                   rank 7   37.9%
Clippy  redundant_closure                 ← chunks_exact_to_as_chunks               ABSENT
Clippy  deref_addrof                      ← identity_op                             ABSENT
Perf    dyn-to-enum                       ← soa-hot-field                           rank 5   39.2%
Perf    box-str-immutable                 ← immutable-static-map-filter-selection   rank 2   26.0%
Docker  apt-no-install-recommends         ← apt-lists-not-removed                   rank 1   22.6%
Docker  apt-not-apt-get                   ← apt-lists-not-removed                   rank 3   32.4%
Docker  instruction-casing                ← no-empty-continuation                   rank 1    0.6%
Docker  cargo-locked-flag                 ← cache-mount-id-scope                    rank 1    0.7%
Docker  cache-mount-missing-on-cargo-build← manifest-first-layer-split              ABSENT
Dist    shipped-binary-missing-size-profile ← shipped-binary-unstripped             rank 2   21.8%
Dist    cli-panic-unwind-tables           ← shipped-binary-missing-size-profile     ABSENT
```

**Overlap join (P0.1 ∩ P0.2):** stored-negative rule ids ∩ wrong-top-1 rule ids = **1 rule** (`cache-mount-id-scope` — and its negative came from an `assert!`-macro span on this repo's own `src/kat_adopt.rs`, embedding-far from the docker fixture where the rule steals top-1, so the pair would likely not fire there). `single_char_pattern` appears as a rejected id AND as a miss *label* — but the channel demotes wrong top-1s, not missed golds, so that is not channel overlap.

## The certification leg — BLOCKED on the data model, not skipped

The acceptable-vs-truly-wrong split (criterion 2) cannot run today, on EITHER side:

- The store does **not persist pre-fix span text** (`TrajRowDoc` keeps `fix`, `shape`, `src_path` — the source file has since been fixed, so the span cannot be reconstructed; the row join is chunk-neighborhood approximate by design, per the field's own doc).
- The eval corpus has **zero authored acceptable sets** (`OwnedEvalSpan.acceptable` — "Empty everywhere today" by its own doc).

With the sampled population itself at 12 pairs / 1 span, criterion (2) is **UNPOWERED** — recorded, never folded into a verdict it cannot support. Any future P1 needs BOTH: span-text persistence (or a re-draft-on-demand pass) AND authored acceptable sets on the miss spans.

## Verdict

**DECLINE-for-now on criterion (1)** — the raw volume is ~0 (12 pairs, one span, three-day-old field): the channel has nothing to consume, so building the consumer today is vacuous. This is a data-state decline, NOT a mechanism negative. The census in fact shows the flip TARGET exists:

- 4 of 15 miss spans have gold at rank 1 — one demotion of the wrong top-1 (all of which are `inst=false`, as is gold) would promote gold under the shipped canon;
- 2–5 spans are near-tie at the pre-registered margins;
- miss-set n = 15 would support a paired read if an effect existed.

So criterion (4) ("near-tie ~0") does NOT fire — the mechanism could flip real misses IF negatives existed for the stealing rules. The binding constraint is purely the negative volume + the certification data model.

**Standing re-open trigger (recorded in the issue):** the local store (or any live store) carrying **≥ 60 negative pairs across ≥ 10 distinct spans** re-arms the triage: run the sampled acceptable-share certification (needs the data-model fixes above), re-run this census at the then-current corpus, and only then the P1 A/B with the round-1 gates (paired LB95 top-1 delta + zero regressions vs baseline + per-family retention + pre-registered λ/threshold).

**Unchanged:** the mint consumer (the v4 `rejected` label's existing consumer) is untouched; local negatives only, fleet rows out of scope pending a separate consent decision; the census example stays as the re-open instrument.

## What this bench does NOT claim

- It does not measure the channel (P1 was never built — the round-3 AGREE made P1 conditional on P0, and P0 returned DECLINE-for-now).
- The 91.2% hits_at_1 is a POSTURE disclosure (5 domains incl. default-on docker/dist), not comparable to the 3-domain floors gate or to Bench 027's 88.1% (different corpus generations).
- The store dump used a binary some commits behind HEAD (byte-read only; disclosed above).
