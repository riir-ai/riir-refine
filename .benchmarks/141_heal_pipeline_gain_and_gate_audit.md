# Bench 141 — heal-pipeline gain/gate audit + corpus-technique inventory (the codegram record)

**Status:** OPEN — T1 COMPLETE (kernel_opt promotion verified at all five surfaces: nothing to do, the 2026-11-06 owner promotion is live end-to-end) · T2 COMPLETE (gains-by-default audit applied: **0 new promotions today** — every remaining opt-in is pre-gain, measured-negative, cost-gated, or harness-by-design; queue: elo_k_eff cell, seat_rerank T5.2) · T3 COMPLETE (coverage snapshot: **718 uncovered / 405 default-firing / 0 fix-set-only**; charter input, gates nothing) · T4 COMPLETE (corpus-technique inventory recorded below; `codegram` coined in the riir-ai glossary; the span-shape POC = issue 216)

## Ask (owner, 2026-11-07)

1. "do promote kernel_opt" — audited, see T1.
2. "i need report and note to md so we can improve after that" — this record + the improvement queue.
3. "anything that gain should promote by default" — the standing owner law ("any gain no perf hurt must enabled by default, dont make good goat sad") re-affirmed and APPLIED per-feature in T2.
4. "can we call it codegram … poc perf proof" — T4 + issue 216.

## T1 — kernel_opt promotion: five-surface audit (feature-gate-audit doctrine)

| surface | evidence | verdict |
|---|---|---|
| 1. source `.rs` doc | `src/kernel_pregate.rs:22` — "Bench-137 posture (SCOPE + `LINE_CAP=2` + `QUERY_TEXT_CAP=2048` baked as [in-code defaults]"; envs DEMOTE-ONLY | ✅ |
| 2. `lib.rs` mod decl | `src/lib.rs:102-104` — `pub mod kernel_pregate` behind `kernel_pregate`, comment names the arms-off posture | ✅ |
| 3. `Cargo.toml` default | L236 `default = [..., "kernel_pregate", "ruliology_search"]` + dated comment (L222-226, 2026-11-06 owner call, benches cited) | ✅ |
| 4. shipped binary / release set | installed `cargo-refine 0.4.8`, build `5e61bb594541` (2026-11-06 16:55 +0700, **ancestor of HEAD `734efc623`**), prints `kernel-opt,kernel-pregate` and `release set: complete (…,kernel_pregate)` | ✅ |
| 5. promotion records | issue 191 closed `71fbb64a1` ("kernel_opt GOAT fully passed, default via kernel_pregate"); benches 131–137 on disk; needle-fix follow-up issue 214 `8e781fce0` | ✅ |

**Verdict: ALREADY-PROMOTED — nothing to do.** The GOAT evidence chain: preregistered A/B (Bench 131) → run (Bench 132: added p50 0.191/0.036 ms, 173×/920× median cut) → line-cap pair (134/135) → query-text cap (136/137: composed posture p95 4.685 ms ≤ 5.0, 1.07× headroom, utility floor 15/15). Disclosed loss at the shipped posture: `QUERY_TEXT_CAP=2048` flips 109/664 rule rankings deterministically (paired rank-1, accepted by the owner call).

## T2 — the gains-by-default audit (owner law applied)

| class | features | verdict |
|---|---|---|
| promoted (default today) | `kernel_pregate`→`kernel_opt`, `ruliology_search`, `self_evolve`, `rustc_errors`, `docker`, `dist` | correct per law |
| shipped release set | `latent_retrieval`, `points`, `net-mainnet`, `fix_verify`, `update_check` | correct |
| **pre-gain** (law blocks promotion until a gain is MEASURED) | `elo_k_eff` (rating-quality lever, feature-off byte-identical, **no bench on record**), `seat_rerank` (armed T5.1 on the dev box; production fix-path wiring is T5.2 — **no wired gain to promote yet**) | queue, not promote |
| cost-gated (gain plausible, G5 cost leg failed → opt-in until the cost lever lands) | `sec` (Bench 140: +~2.0 ms p50/file), `shader_perf` (Bench 140: 1.10–1.28 ms p50; the file-gate fix is issue 213, G3 PASS local, G2 queued on amo) | wait on issue 213 G2 |
| measured-negative (loser stays demoted; never promote) | `set_rerank` 096 · `gw_corpus` 083 · `nonergodic` 093 · `choice_scorer_poc` 098 · `mattr_k_poc` 109 · `differential_anchor` 102 · `kernel_route_narrow` 126 · `basis_cert` (Bench 115, no transferable subspace) · `demonstrability_gate` (verdict-off) · `l4_lora` (Plan 336: 0/60) | standing refusals |
| harness / consent-by-design (not runtime gains) | `clippy_verify`, `rust_perf_verify`, `rust_perf_bench`, `kernel_verify` (STUB), `rescue_probe`, `coverage_gap_event` (report-only), `contrastive_scope`, `structural_admission`, `vessel_artifacts`, `wasm_validator` (Bench 105: native ≈285× faster), `hosted_expert`, `ternary_inference` | correct as opt-in |

**Verdict: 0 promotions today is the honest answer** — promoting a pre-gain or cost-gated feature would violate the same law that promoted kernel_pregate. Next two candidates carry NAMED unblocks: elo_k_eff (needs a paired rating-quality cell) and seat_rerank (needs T5.2 wiring, then the paired regate).

## T3 — coverage snapshot (the improvement charter; report-only, gates nothing)

`cargo-refine 0.4.8 --coverage-report .` on 2026-11-07 (clippy 0.1.99 `b940084d7e` 2026-09-28; derived 830 lints / 9 groups; corpus 112 clippy_lints rules + 1402 other-domain; compiled fix set 109):

| metric | value |
|---|---|
| covered by the clippy_lints corpus | 112 |
| fix set only (no corpus rule) | **0** |
| UNCOVERED (the coverage debt) | **718** |
| — of which fire by default | **405** ← the debt a plain `cargo clippy` shows a user |
| — pattern-mentioned in rule bodies | 28 (possibly covered under a different key) |
| uncovered with NO evidence from any source | 588 (uncited, NOT zero) |
| default-firing uncovered WITH evidence | 97 (default-eco histogram) |

By group (total/covered/uncovered/firing): restriction 135/0/135/0 · pedantic 145/20/125/0 · complexity 144/31/113/113 · style 159/50/109/109 · suspicious 85/4/81/81 · correctness 67/1/66/66 · nursery 52/4/48/0 · perf 38/2/36/36 · cargo 5/0/5/0.

Evidence heads (JOINED, never estimated): local = Bench 061 fired histogram (30 rows / 6 repos, 2026-08-30, toolchain mismatch — **ranks, does not measure today**); default-eco = Bench 094 (186 rows / 663 crates, 2026-09-17, same caveat); study = kornel 100K-crate run Sep 2024 (3 rows); **dual-evidence head = 2 rows: `missing_errors_doc` (90) + `missing_panics_doc` (78)** — the batch discipline stages these first.

Standing reading laws (from the report's own footer): name-keyed coverage is a LOWER BOUND (rules heal by pattern — `len_zero` reads uncovered while `comparison_to_empty` heals it); run the Protocol-A dedup grep BEFORE authoring anything this report calls uncovered; dedup 0 collisions today.

## T4 — what the corpus machinery does today (the codegram inventory)

1. **Rules-as-productions** — per-domain corpus (clippy 112, rust_perf 304, kernel 1073, sec 48, docker 31, dist 8, shader 8), each rule pattern + fix template + `context_notes`, fixtures standalone-compiled AND executed before entry, counts pinned by `pins_manifest.py`.
2. **Flat per-span fan-out** — every enabled domain matched per span; measured RIGHT at today's sizes: the LSA/monotone crossover law (Bench 126 + riir-infer `.research/020`) puts the retrieval knee near ~32k buckets; 112–1073 rules/domain is far below it, so dense fan-out beats routed search. Re-open trigger recorded, not a task.
3. **Signal-gated dispatch** — kernel_pregate (file signal + span scope + caps, Bench 137 posture, promoted) and issue 213's `DomainMask` (the Issue-145 path contract applied BEFORE retrieval, docker+dist shed per `.rs` span) — the FILE-granularity half, in flight.
4. **Latent retrieval (opt-in)** — riir-rag span-level KNN as a per-span ANNOTATION, never a gate; the 8-D latent term never moves clippy top-1 (Bench 027) yet is ≈60% of retrieve time (issue 213 probe) — sec's lever is ndb-side, pre-registered separately (issue 213 T5).
5. **Rating** — Elo over the batch replay (score-bench, provenance strata; `elo_k_eff` evidence-damped K opt-in), Pareto/ruliology L3 candidate search, gen2/gen3 freeze-table regeneration on every corpus-touching landing.
6. **Self-evolve** — trajectories persist to the ndb Warm-tier store; tier certification via the Lemma-4.2 likelihood-ratio test; learning half inert without `clippy_verify` (disclosed as `memory-only`).
7. **Coverage intake loop** — `--coverage-report` (this charter), `--corpus-delta`, `--corpus-gap` (katgpt-core coverage_manifest substrate, content-addressed intake rows), `--misses` queues, `--oracle-census` + default gap-arming.
8. **Corpus lease serving** — the corpus as an add-only replicated artifact (epoch + per-segment BLAKE3s, ETag == commitment) over the KAT/points planes; `update`/`sync`/`corpus-pr` verbs.

**Naming verdict:** the act has a name now — **codegram** (coined in the riir-ai glossary, Text/grammar/traces section): the healer's grammar-is-the-model act over code; family `quest_grammar` (quests) · `topologammar` (maps) · `codegram` (code fixes); the shared machinery noun is **grammaturgy** (existing row). It is NOT a new mechanism — the healer already is codegram; what does NOT exist is the span-SHAPE (syntax-keyed) half of dispatch, filed as issue 216 with pre-registered perf bars.

## Improvement queue (the "so we can improve after" list)

- [ ] A. elo_k_eff promotion cell — paired Elo-quality A/B (bar: monotone evidence-damping, no rating regression on the score-bench replay); owner gate after the numbers.
- [ ] B. seat_rerank T5.2 — wire the armed manifest row into the production fix path, then the paired regate; release-set promotion only on measured gain.
- [x] C. issue 213 G2 — DONE by the foreign lane (zed-2f171db9@m3): G2+G3 PASS, **shader_perf PROMOTED to default + release set** `9cc961530`, Bench 143; issue 213 closed; issue 214 (@compute needle) closed in the same wave.
- [ ] D. codegram POC — issue 216: T1 LANDED (Bench 144: classifier + 6-row annotation table + filter + 5 test pins; clippy-green at default/feature/release-set); T4 G2 cost leg QUEUED on a quiet rig; promotion (T5) waits on the numbers.
- [ ] E. coverage-debt authoring lane — stage the 2 dual-evidence rows first (`missing_errors_doc`, `missing_panics_doc`), then default-firing complexity/style/suspicious heads; Protocol-A dedup grep before EACH authoring; per the mining-batch discipline.
- [ ] F. standing re-open triggers (watch, never tasks-until-triggered): `kernel_route_narrow` at corpus ≥ ~32k rules (LSA knee); sec per-query overhead waits on issue 213 T5 (ndb score-only mode).

## Records

Binary `cargo-refine 0.4.8` build `5e61bb594541` · HEAD `734efc623` (origin/develop equal, tree clean at audit) · issue 191 closed `71fbb64a1` · issue 214 `8e781fce0` · benches 131–137 · issue 213 in flight (claim zed-2f171db9@m3 exp 2026-11-09) · coverage report executed 2026-11-07 · glossary coin: riir-ai `.docs/01_orientation/glossary.md` · POC: `.issues/216_codegram_poc.md`

Session: zed-native-agent (m3), epoch 1794… (see the commit body)
