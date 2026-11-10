## HISTORY.md — riir-refine

Compacted 2026-10-30: every `##` heading kept verbatim, one compact entry per record (≤2 short lines); the full pre-compaction text is in git history (pre-compaction HEAD 403d72bbb703; the 2026-10-16 pass: cf0f52dbb180). Operational rules live in `AGENTS.md`; removed issue files: git history.

## 2026-11-10 — Issue 192 CLOSED + removed: the public-goods pivot is LANDED; the tracker's own header said it dies on completion

All directives executed or housed: MIT on disk (`license = "MIT"` + LICENSE verified 2026-11-10); CLI surface SHIPS (join/leave/rank/info/upgrade verified live; `corpus`/`update` PARTIAL → issue 203, landed per AGENTS.md — every points-bearing build); GAP 10 vocabulary law landed 2026-11-06 (`--mine`/`--unmine` REMOVED, join/leave is the surface); GAP 11 crates.io = issue 221 (live lane); freeze residue + seal moat posture landed (T16.7/T16.8). Residue has named live homes: websites Phase 2/3 = Plan 324; page-prose class = riir-dapps 157; adoption-gate intake = issue 220. The durable model lives in Proposal 025 §"The points plane in full" — the file was the TASK TRACKER, removed per its own noise-reduction note; `mozilla_grant_readiness.md`'s inbound link repointed to HISTORY in the same commit. Claim (zed-riir-docs) expired 2026-11-03, folded. Session: amo-idle-i1191mid.

## 2026-11-08 — Issue 198 CLOSED + removed (noise rule): the outcome poller fired — #2977 MERGED, ledger updated, post-mortem lives in issue 218

PR #2977 (the CACHEDIR.TAG goodwill PR) merged 2026-11-07 12:31:11Z by sylvestre (`907230be`, API-verified) — already recorded in the [bench 127](.benchmarks/127_mozilla_wetrun/README.md) ledger's MERGED poll row + the accept-rate table, and the failure-class post-mortem moved to issue 218 when it was filed the same day, so the file carried zero remaining obligations. The six-round arc stays in git history (the file's full life: `git log --follow -- .issues/198_mozilla_outbound_pr_wetrun.md`). Same commit: issue 190's stale T14 checkbox ticked (regular bar LANDED 2026-11-01 bench 123; strict bar owner-gated by the 2026-11-04 D3 verdict, status quo kept — no lane work remains).

## 2026-11-07 — Issue 214 CLOSED + removed: the kernel pregate's `@compute` needle fires (fold fix), KEPT after G5 PASS (Bench 143)

T1 `8e781fce0` (ASCII-letter-only fold; `@` rides in `NEEDLE_FIRST_BYTES` as itself; pins `at_compute_needle_signals_alone` red-before/green-after + `lone_backtick_never_signals`), T2 census `examples/kernel_pregate_census.rs` (registered walk 0 file flips / 72 span-seat self-matches; riir-shader/seal-render 0/0; riir-gpu UNSEEN on m3/amo/4090), T3 G5 on bmo via the amo jump — OLD vs NEW fold, 3 interleaved rounds, added p50 ≤ 0.025 / p95 ≤ 0.403 ms/file, all PASS ⇒ `@compute` stays in `SIGNAL_NEEDLES`. Instrument: `g5_cost_math.py --code-axis` (the feature-delta guard INVERTS for a source A/B — stamps must be identical). Bench 131 annotated (appended) with the corrected effective set. Session: katgpt-rs-f5.

## 2026-11-07 — Issue 213 CLOSED + removed: `shader_perf` PROMOTED to default + the shipped release set (DomainMask path gate; Bench 143 G2 + G3 PASS)

T1/T2 the `DomainMask` path gate (the Issue-145 `applies_to_path` contract applied BEFORE retrieval — `.rs` spans no longer pay the shader/docker/dist indexes; the needle-set design superseded pre-measurement), T3 G3 PASS `c56c11e6` (4359 external spans, applying domains rank-1 byte-identical) + G2 PASS on bmo (added p50 0.086–0.170 / p95 0.798–0.973 ms/file vs Bench 140's 1.10–1.28 / 5.30–5.92, all 3 rounds; the amo jobs were cancelled unrun — Bench 143 is the only G2 record). T4: `default` + RELEASE_FEATURES + release.yml (build + licenses) + manual_release.sh in lockstep. T5: ndb Issue 643 closed NO-promotion — `sec` stays opt-in on its own Bench 140 cost axis (no latent-leg shortcut). Session: katgpt-rs-f5.

## 2026-11-07 — Issue 215 CLOSED NEGATIVE + removed: budget-gated try-next-on-compile-fail refuted at G0 on BOTH pools (Bench 142)

G0 census, full populations (405 v2edit + 29 score-bench): `heal_with_pool` pools differ densely (0.844 / 0.966) but STRUCTURALLY — corpus snippets don't merge; the proposer member (appended last, idx 4–5) is the one that compiles → try-next K≤4 recovers 2/405 (`74c1c86e`); the production seat-lane rule-book pool (`bin/heal.rs`, builder factored to `LatentMatcher::seat_rule_book_pool` unchanged, `01fb2c1f`) differs 0.235 / 0.172 and try-next recovers 2/405 at ANY K (a failed first pick nearly always means an all-fail pool). G1–G4 never run; the outcome harvest would teach only the known "proposer compiles, snippets don't". Not shipped: a proposer-first pool REORDER (341/342 oracle-reachable at 1 compile) — no production consumer picks by pool order (seat presentation cap 8 ≥ width 6) and it would move frozen-pool BLAKE fixtures for zero measured gain. **Re-open trigger:** a consumer that picks from `heal_with_pool` BY ORDER (or a measured seat position-bias) → pre-register the proposer-first arm. Session: katgpt-rs-f5.

## 2026-11-07 — Issue 211 CLOSED + removed: the huacnlee/cargo fork DECLINED, blaze FAIL (Bench 138/139)

Pilot measured: fork worktree amortization real only at dep-heavy scale (sge 6.95× on the 2nd worktree) with a 2.0×/3.3× first-build tax → crossover at the 3rd worktree; sccache no wall gain; the CARGO_TARGET_DIR hazard reproduced and NOT prevented by the fork (Bench 139 Part A `c26f9690`); blaze 0/14 valid arms (Part B); sha-law verified-negative on both cargos (`3f37c4a5`); hygiene residue `b43a3c32` + ndb `04fc3c8`. Delegated-owner verdict 4785a7a0: DECLINE, `cargo-hc` uninstalled. Moot rows (pre-registered adoption bars, 4090 install) dropped with the decline; the first-build-tax repro (amo `200b5b44`, curiosity-grade) moved to Bench 138 §Anomalies. Session: katgpt-rs-f5.

## 2026-11-07 — Issue 209 CLOSED + removed: the c′ referee is a STANDING read, not a task (`.docs/10_self_evolve/corpus_referee.md`)

T1–T5 landed `a6eb6be0` + `a7307895` (certificate + telemetry/DiD state machine + `corpus_diagnose --referee/--enroll`, case #1 `hashmap-with-capacity` @ ac280021 enrolled). T6 was the only open row and it is passive accrual (last read 2026-11-07: PENDING BelowSupport 2/30, batches 3/8, controls 8) — moved to the doc's enrolled-cases table with the operating runbook; a verdict (PASS/FAIL at n ≥ 30 or UNDECIDED-by-starvation at 8/8) is recorded there + here. Session: katgpt-rs-f5.

## 2026-11-07 — Issue 200 CLOSED + removed: the root-package workspace + fixer/trainer tier crates landed; owner acts moved to Plan 335 P2.0

C1 `4f2de6e80` (workspace + tier stub crates + Layer 5t + BOUNDARY tier law) + C2 `18ba1e644` (`fixer_regate` → fixer crate, `kat_drain` → trainer crate, surface repairs); tier KAT surfaces default-OFF. The three remaining rows were owner acts only (fixer bin name → Dist artifact under it; TUNA point-weight table, owner-armed per Proposal 025) — carried verbatim into Plan 335 as P2.0 with the design notes (onboarding UX, self-host runbook, native-only tier bins). Session: katgpt-rs-f5.

## 2026-11-06 — Issue 212 CLOSED + removed: `ruliology_search` promoted to default; `sec` + `shader_perf` FAIL the G5 cost leg and stay opt-in (Bench 140)

T1 `5f985649` (generic `scripts/g5_domain_cost.sh` + shared `g5_cost_math.py`, `kernel_goat_g5` stamps `compiled_features`, G3 arm takes a feature), T5 `76b6af81` (elo_k_eff comment re-pointed to `280f15766`; shipped-binary inert EMA learning half disclosed), T2–T4 `615744da`: sec added p50 1.95–2.16 / p95 9.18–10.31 ms per file, shader_perf p50 1.10–1.28 / p95 5.30–5.92 (ceilings 1.0/5.0, all 3 interleaved rounds FAIL; G3 byte-stable both) — axis: the domain corpus joins every `.rs` span's matcher fan-out (lever: a pregate/scope, `.wgsl`-only for shader). Measured on amo under load (bmo offline, preflight REFUSED — disclosed in Bench 140); a quiet-rig re-run is the open confirmation if a scoping lever is attempted.

## 2026-11-06 — Issue 210 CLOSED + removed: the `fix`-verb docs sweep is workspace-complete; `--fix` stays a quiet alias (no hard removal)

The last rows landed: seal-remake `8fec50de` (was blocked on sibling WIP — landed from a clean develop worktree, the sibling branch untouched) + three repos the original list missed, found by the closing workspace grep: riir-ledger `f4f5bfa`, riir-llm `b1ccdb3`, riir-mmorpg-examples `d0631a2`. Re-grep: zero AGENTS.md rows teach `cargo refine --fix` (only `*-i172` scratch worktrees + dated records). The deferred `--fix` hard-removal (owner-gated #OwnerGate 1) is DECLINED, not deferred: the removal buys nothing (the flag is already untaught, the parser alias costs one arm) and breaks every pinned script/muscle memory — `--fix` stays a permanent quiet alias. Session: katgpt-rs-18.

## 2026-11-06 — Issue 191 CLOSED + removed: the kernel_opt GOAT gate is fully passed and the domain is default-on (via kernel_pregate)

Every pre-registered bar now holds: G1 248 green / 0 FAIL (Issue 193 repairs), G2 GPU-host median 1.3774 ≥ 1.06 (Bench 135; the CPU-portable axis closed with disclosure per the 11-04 D1 verdict), G3 byte-stable (`2ff3c0a61`), G4 structural, G5 utility 15/15 + cost cleared under the pregate posture (Bench 137, p95 3.99/3.44 ≤ 5.0 confirmed alone). Promotion shipped as `kernel_pregate` (25d90f2b — implies `kernel_opt`; scan-first scoping, so non-kernel files pay no latency); the stale "not promoted" `kernel_opt` Cargo.toml comment rewritten in the closing commit. Standing harnesses stay: `kernel_goat_g1`/`g2` examples + `tests/kernel_goat_g1_smoke.rs`. Session: katgpt-rs-18.

## 2026-11-06 — the kernel pregate + kernel_opt PROMOTED to default + the shipped set (Issue 208 CLOSED + removed; owner call)

Owner call, verbatim: "any gain no perf hurt must enabled by default, dont make good goat sad" — the Bench-137 composed posture (GATE+SCOPE+LINE_CAP=2+QUERY_TEXT_CAP=2048; fence cleared 1.07× headroom confirmed alone, floor 15/15) ships as the in-code defaults: `kernel_pregate` joined the crate's default features (implies `kernel_opt`) and the release set in lockstep (RELEASE_FEATURES + release.yml build line AND licenses line — dep:regex enters the shipped license graph — + manual_release.sh, pinned by the build_stamp tests, all green); the three envs became DEMOTE-ONLY (exact literal `0` restores each axis's pre-promotion behavior byte-identically; 4/4096/1024 remain accepted tunings). The 109/664 rank-1 flip class is disclosed and accepted by the same call (census 462→538/1061 cited; the non-lossy complement stays riir-neuron-db Issue 642); the real-corpus kernel-GOAT validator stays open as hygiene, not a blocker. Gates at the landing: default lib 3434/0 · flag-OFF 2352/0 · clippy −D clean (2 doc-backtick findings fixed by the rulewright, compile-gated) · build_stamp 14/14. Issue 208 removed per the noise-reduction rule (git history is the durable record). Consuming session: zed-m3-kernel-promo.

## 2026-11-05 — Issue 191 T6 EXECUTED + CLOSED: the GPU-host G2 re-gate PASSES (Bench 130 prereg → 133 sources pin → 135 run; lease closed in-window)

The 4090 exclusive run (`67fbe2e7`; the first attempt `a419844b` FAILED AT BUILD — zero timed chunks, no prereg violation, the toolchain alignment fixed + retried same-session): correctness 23/23 PASS on CUDA, 23/23 measured (MIN_MEASURED_PAIRS=12 satisfied), **leg median 1.3774 ≥ 1.06 → the G2(GPU-host) leg PASSES**; thermal clean (49 °C idle/31.4 W → 51 °C load/122.7 W); per-rule regressions recorded, never pooled (8 pairs < 1.0: P121-A 0.0127×, P117-C 0.1219×, P125-A 0.2441×, P136-A 0.4949×; wins P135-A 459×, P134-A 44×, P112-A 39×, P138-A 37×). Both G2 legs now closed (CPU tranche-median 1.0000 disclosure stands, never re-run; the GPU-host leg the 11-04 D1 verdict re-opened is measured). `kernel_opt` STAYS opt-in: the re-campaign remains blocked on G5-cost only (Bench 129/132). Lease `zed-m3-i191t6` closed `b5ecb3456`; scratch cleaned both boxes. The 131→133 dual-allocation collision (the pregate prereg took 131 first) resolved by FIRST ALLOCATION `3d5aff179`, sources renumbered 131→133, bytes + digests untouched.

## 2026-11-05 — Issue 208 T1–T3 LANDED: the kernel pregate (Bench 131 prereg → Bench 132 run) — median leg GREEN, p95 leg FAIL, the fence stays UP

The Bench-129 named lever (b), owner-directed: `kernel_pregate = ["kernel_opt"]` (default-off) — file-level recall-biased kernel-signal detector (`.wgsl` always; `.rs` by the pinned needle set) gates the kernel domain's fan-out, ±24-line span scope inside signaled files, `--domain kernel` bypasses, detector cost counted in every ON arm. Bench 132 (Bench 131's preregistration executed, `8582b176a`): added p50 **0.191 ms (GATE) / 0.036 ms (GATE+SCOPE)** vs the flat 33.127 — the median leg flips GREEN for the first time (173×/920×) — but added p95 **FAIL 122.5/92.4 ms vs the ≤ 5.0 ceiling**, utility floor HOLDS 15/15 both arms. Fence verdict: FAIL — `kernel_opt` stays opt-in; issue 208 T4/T5 ticked, the per-span cap lever (a) promoted to T6 as the active path (lane OPEN, claim zed-m3-i208pregate exp 11-07).

## 2026-11-05 — Issue 207 RESOLVED same-session (option b): the narrow posture gets its own blended floors — the silent-drift hole closed

Found by the Issue-191 T7 validation, reproduced on CLEAN develop with T7's files reverted (byte-identical ratio both states — the adoption is retrieval-neutral). The narrow posture (kernel_route_narrow, the Bench-126 replication-only lane) labels its OWN population (172 spans vs the flat 142) and no pinned suite runs its gate, so the corpus growth since the Issue-196 pin (B323→B333) spent the flat constants' margin UNNOTICED: blended top-3 measured 156/172 (0.906977) vs the flat pin 129/142 (0.908451). Fix (option b, the structural one): `mod floors` cfg-split — narrow `BLENDED_TOP1/3/5` = 149/155/157 of 172 (measured − ONE span, the Issue-063 convention, full attribution in-code), else the flat 123/129/131 of 142; the per-domain asserts (clippy/perf/kernel) cfg'd to the flat posture only — at narrow they are disclosure rows (flat-population constants over a different labeled set; the routed pool's structural rank-1 loss is Bench 126's own R2 finding — pinning them narrow would encode the refused lane's deficit as a floor). Gates: narrow 1/0 (was 0/1) · flat 1/0 · retrieval battery 27/0 · clippy −D ×2 postures. Residual disclosed: no pinned suite runs the narrow gate (the drift hole itself) — re-arming a suite row is the owner's call. Issue file removed per the noise-reduction rule (git history is the durable record).

## 2026-11-05 — Issue 191 T7 EXECUTED: the scratch-reuse twin landed, the G5-cost fence re-measured and still FAILING (Bench 129; promotion stays refused)

The riir-neuron-db Issue-642 twins (`retrieve_for_scoring_into` / `RetrievalScratch` / `DiverseScanScratch`) adopted into the per-line suggest loop (`77f7e46a9`: `SuggestScratch.retrieval`, `retrieve_scored_into` routes the scratch twin; the Issue-196 narrowed index + its diagnostic test updated; byte-identical by contract test and by measurement). Floors held at every standing posture (latent_retrieval 3036/0 · kernel_opt 3390/0 · narrow 6/0 · clippy −D ×4). Bench 129: added p50 33.127 ms flat / 37.306 narrow vs the ≤ 1.0 ms fence — the lever is −9%/−19% on a LARGER corpus+walk (Bench 126: 36.346/45.849), the remaining cost is the per-line query mass (structural: query_for + O(N) scan per line query at 1053 rules; heal.rs-class files drive p95). Per the 2026-11-04 delegated verdict D2 the ceiling is never re-registered: the fence STAYS failing, kernel_opt stays opt-in, and the re-campaign stays blocked on both G2-GPU-host (T6, parked) and G5-cost. Next lever named (per-line query cap/batch, cheaper scan key, narrowed-at-reduced-K) — a retrieval-contract design change, owner-visible. Side finding: Issue 207 (the narrow-posture blended top-3 floor decayed pre-existing on clean develop — byte-identical with/without T7). Validated in worktree `riir-refine-i191t7` (the shared tree's test posture was blocked by the sibling's in-flight Plan-342 WIP, untouched; the worktree was removed after the maintained binary was rebuilt at `77f7e46a9e6e` with the complete release set through the `~/.cargo/bin` symlink target).

## 2026-11-04 — Issue 202 CLOSED LANDED + REMOVED: the traj-store score-run toolchain axis (option a: stamp fields, key unchanged)

The 197-followup landed `c4f7ccd31` (zed-m3-issues-fixall, owner reassignment "do 10", the issue-197-takeover precedent — the foreign amo claim reassigned + discharged inside its window): `ScoreRunRow` carries `rustc_version` + `clippy_driver_version` (serde-default `None` = unknown-toolchain for pre-202 rows), `From<&HistoryRecord>` mirrors the pair verbatim (new rows stamp through the dual-write tail; the jsonl side stamped since plan-329 P4-a), and `score_runs_share_toolchain` is the store-side bucket rule (exact-pair match; `None` pools only with `None`; a clippy-only difference is its own bucket). Gates: schema-identity extended (the pair rides, pinned both directions incl. the legacy no-key decode) + the pooling G1 (truth table + store round-trip + detect_saturation anchor rule, both anchor directions); default lib 2817/0 · traj battery 49/0 · retrieval 27/0 · schema+G1 3/3 at score_bench · 47/0 all-features · clippy −D ×3 postures clean. Renumber note: this issue held 198 first (`7702625e` 2026-11-03) and lost the first-allocation COLLISION to the wetrun (`531b83af` 2026-11-02; citation_weight agreed) — the wetrun keeps 198, this file renumbered 202. Issue file removed per the noise-reduction rule (git history is the durable record).

## 2026-11-04 — Issue 205 FOLDED NEGATIVE + REMOVED at its pinned base-rate gate: campaign failure decomposition (LEAP × Research 649 fusion; katgpt-rs Research 650)

The demotion law fired pre-PoC exactly as pre-registered: measured over the ENTIRE blueprint era on this box — 38 campaigns across 12 repos (2026-10-27..11-03, the lane's full history; the pinned ≥20 floor met by the whole era) — **0 witnessed refusal/loop-exhausted events**; 19 at-risk campaigns (verify posture on + edits written) kept 100% of their edits with real compile checks (≥2 each); the era's entire failure incidence is **1 benign revert** (riir-refab 10-28: the loop caught a breaking edit, reverted 1, kept 3 — the machinery SUCCEEDING, nothing left for a recovery vocabulary to do). 0% << 5% fold bar under every denominator (0/20 window, 0/38, 0/19 at-risk). Witness mapping: refusal-spawn class = `posture.verify && !verify_ran && edits>0` (exact); unverified-kept subset = `kept>0 && checks==0` (strict subset); blind spots stated — the blueprint row counts checks not check-outcomes (kept-under-failed-final-check unwitnessable) and the traj store over-counts (`verified=false` = all failures incl. benign reverts — no separative power), no `.refine/misses/` queues ever harvested (weak absence). Filed from katgpt-rs Research 650 (sweep record `e7a746274`, filing commit `52126109`, verdict round-1 REVISE addressed + round-2 AGREE session `6a18619e`); the fold outcome recorded in 650 §8 (the pre-registered kill home). Sibling issue 206 UNAFFECTED (separate referee, separate failure class); its corpus-binding module-half landed same day (see the corpus_binding entry). File-removal attribution: the `git rm` deletion was swept into the sibling batch-330 claim commit `0182f9d7e` (shared-index race — this fold commit `e2188729` carries the record; the removal diff-tree lives in the sibling's commit).

## 2026-11-04 — Issue 198 close-out + the 190/191 delegated verdicts + the 198-collision renumber (issue 198 stays OPEN on the PR poll)

Mozilla outbound wetrun Phase 1 closed: mozilla/sccache#2977 polled OPEN at round-5 head `903f0be` (five rounds: the owner REVISE→KEEP slop verdict, maintainer rounds 3/4/5 fixed + replied, Claude AGREE ×2; goodwill-only per the owner verdict — outside the campaign ledger; the four send gates bind all future sends); T7 intake landed (P113 upstream-state check + P114 walk×strict-parse sec candidate); T8 executed — Proposal 022 Status + Proposal 025 self-grade evidence lines, `.raw/mozilla-heal` removed (16 GB, the sccache clone verified at the pushed fork head first). Delegated-owner verdict (session fe55a9d0, claude_code reviewer, AGREE): D1 kernel_opt re-campaign G2 = GPU-host 4090 microbench (the CPU-G2 axis closed with the tranche-median-1.0000 disclosure; the ≥ 6% bar unchanged; three pre-registration conditions now issue 191 T6); D2 G5-cost fence kept (the ceiling never re-registered; the clearing lane = the refine-side scratch-twin adoption of riir-neuron-db Issue 642 + the registered-corpus re-measure, issue 191 T7); D3 190 T14 strict bar = status quo + the wording law ("no payment or token surface; the riir-kat secret-scanner tier is present", never "zero kat crates") + the riir-kat-internal leaf-crate strict-path option; D4 190 T2 recorded CONFIRMED-BY-ACT (`aea8e270`, issue 192 v5) with delegated ratification, owner veto noted — 190 T1 ticked as bookkeeping via issue 192's owner decision. Issue 191 T3 verified LANDED while recording (`context_notes` G1 annotation + `verify.rs` verdict-store consumption). Numbering: `198_traj_store_toolchain_key.md` renumbered → 202 (COLLISION with the wetrun issue; first-allocation decides — wetrun `531b83af` 11-02 vs traj-store `7702625e` 11-03; citation_weight agrees on direction, 5 UNRESOLVED sites disclosed; highwater → 202; the plan-329 cite moved in the same commit). 192 T16.7/T16.8 ticked (both were already done on disk).
Commits: this commit; issue 198 stays OPEN on the outcome poller.
## 2026-11-04 — Issue 201 CLOSED NEGATIVE + REMOVED: the ρ-embedder alignability lane — verdict half MATCH, savings arm FAIL (riir-reflex 106/107 + katgpt-rs 978 T5)

Verdict-reproduction half MATCH (ρ_diff(struct4)=0.4657 < 0.5 ⇒ NOT-ARMED = Bench 115's recorded refusal; ranking rows measure cleanly — struct4 ρ=0.3810, 21× above the hash4 noise baseline 0.0178; 14 single-row families excluded BY NAME; deterministic ×3). Savings arm FAIL — both ledger halves at/below the 5 ms resolution bound: the d=8 transport is too cheap to gate. Commit `25832570`, revised `e02dfa35`; session zed-b860ef72@amo; lint-clean lease folded. Consumer records: riir-reflex 106/107 (Bench 172 pair + Bench 173); katgpt-rs 978 T5 carries the cross-repo verdict (T4 promotion REFUSED — the primitive stays opt-in). Issue file removed per the noise-reduction rule (git history is the durable record).

## 2026-11-04 — Issue 199 REMOVED (CLOSED 2026-11-03): the heal→rulewright coin landing + vocabulary sweep — all tiers landed; one gate residue fixed next day

The owner-deprecated "heal/healer" vocabulary swept to the three coins — **rulewright** (the product), **rulegram** (the artifact), **the refinery** (the plane) — across every doc surface: Tier A refine AGENTS/README/BOUNDARY/orientation (`771e7ad15`), the heal-seat→rerank-seat rename in instinct (`1862743`), katgpt-rs skills doc-sync/build-hygiene/research (`47e71ce78`), Tier B the 41-file deep-book + five skills sweep (`f9727c207`), the fleet-wide "## Lint fixing" header act across 16 sibling AGENTS.md files (`c6d258115`, the sibling idle lane), the coin landing in riir-ai's glossary/twinarm docs (`4d185a863`), and the KAT node-tier rows ("Rulewright client"/"rulewright-proof") on the owner call. KEEP-list (deliberately un-swept): code identifiers (`HealDomain`/`HealPipeline`/`heal()`/`heal_with_pool` — API stability, a code rename is a separate owner-gated act), metrics/history prose (`heal_rate`, recorded bench rows), MAPE-K self-healing (riir-neuron-db's autonomic-loop term), the retired record file names. One residue: the sweep reworded this repo's AGENTS.md clippy_lints row ("heals" → "drafts fixes for") without updating `scripts/pins_manifest.py`'s PROSE_SITES regex — ci_feature_guard Layer 7 red 2026-11-03→11-04, fixed `2509e797` (regex rides the new wording; counts never drifted).
Commits: `771e7ad1`, `1862743`, `47e71ce78`, `f9727c207`, `c6d258115`, `4d185a863`, residue fix `2509e797`; issue file removed per the noise-reduction rule (git history is the durable record).

## 2026-11-03 — Issue 193 REMOVED (CLOSED 2026-11-01): the 65 kernel_opt fixture repairs — G1 gate clean, re-campaign path step 1 done

**Status: Resolved.** All 65 defective fixtures repaired (zero demotions, zero asserts weakened); full G1 248 green / 0 FAIL / 803 parse-only in 100.8 s, rustc 1.99.0; the G1 verdict store regenerated over the green population (`d84578aa` R6) and the close recorded with the full commit roster (`320ec193` — R3/4/5 waves E/F and the rest in the fix series 1a966e6f/a66f40a7 and siblings, all pushed 2026-11-01; lease zed-4090-193 closed). Feeds Issue 191's re-campaign path (next: the owner-gated G2-convention decision, then re-run G1+G2+G5). The file's malformed lease block (a `Claimed:` line with the close prose jammed after it — the shape `claim_lease_check --lint` reds) is removed with it.

## 2026-11-03 — Plan 329 LANDED: the Mozilla-quartet methodology port, T0b (issue 197; P4-b → issue 198)

The uniffi-checksum / sccache-mock / vet-aggregate ENGINEERING doctrines ported onto the healer (issue 197's T0b; execution reassigned to zed-m3-24c8b43a by owner directive, the amo lease superseded, agent-mail 01a19ed3). **P1** the corpus pins manifest: `scripts/pins_manifest.py` + `.benchmarks/pins_manifest.json` derived from the compiled builders (`src/pins_manifest.rs` dump test) — every count assert reads the manifest (`crate::pins_manifest::count`), the prose half gates AGENTS/README/.docs live rows, ci_feature_guard Layer 7, `--self-test` proves both red directions; first run caught README's stale "100 corpus lints" (live 112) and the kernel total's 35 duplicated re-pin sites collapsed to one manifest diff per batch (the B308 class, closed structurally). **P2** `VerifyReport.refusals: RefusalTally` (7 classes) + the `verify refusals:` roll-up + `tests/fix_verify_failure_matrix.rs` (9 arms + the two-sided completion-sentinel G1); ⛔ labeled fail-closed delta: `run_check` refuses zero-JSON output regardless of exit (the empty/garbage-success shape used to read CLEAN and keep the edit). **P3** `DOMAIN_REGISTRY` + dist/docker exemptions (deletions-only ratchet) + 5 wiring tests in `src/domains/mod.rs`. **P4-a** `HistoryRecord.{rustc_version,clippy_driver_version}` + `ToolchainVersions` capture + **generation 0 = the LEGACY UNPINNED sentinel** + the `detect_saturation` pooling split (never pool on toolchain ∨ generation); row #220 carries both versions live; the pinned row RESETS the gen-1 saturation series (1/6, honest) and an all-legacy history refuses v1 admission. **P5** audit verdict ALREADY-SATISFIED (per-entry fire+re-parse, per-rule shape goldens, BLAKE3-frozen `fixed_code`, label-drift detection — stronger than byte-snapshots; the byte lane deliberately not built). **P4-b** filed as issue 198. Gates: default lib 2807/0 · pins posture 3668/0 · matrix 11/11 · clippy −D ×4 postures 0 (three committed pre-existing lints fixed en route). Concurrent-session note: a sibling's uncommitted rustfmt-style sweep (~60 files) ran during the landing — out-of-scope files stay uncommitted; two sibling-dirty files needing lint fixes staged as HEAD+fix blobs per the shared-worktree doctrine.

## 2026-11-02 — Batch 325 LANDED: the uniffi-rs FFI-hygiene conversion (Plan 327 / Research 361; sec corpus 31 → 35)

FOUR suggest-only sec rules: `arc-refcount-across-raw-boundary` (DropLeak, the A− lead — the borrowed/consume two-regime custody law, `into_raw` round-trip suppresses, `ManuallyDrop` custody suppresses; detector `arc_raw_boundary`) · `ffi-callback-invoke-outside-lock` (Ffi — the invoke-under-live-guard cross-FFI ABBA deadlock, adjacent-distinct from `host-lock-across-reentrant-blocking-call` per the mandatory in-rule adjudication note; detector `callback_lock_invoke`) · `out-param-manuallydrop-not-maybeuninit` (Ffi — the caller-init repr(C) contract; full-write fires, partial field-init keeps MaybeUninit; detector `outparam_init_contract`) · `panic-handler-nested-catch-unwind` (PanicSafety — the handler path self-guards; detector `panic_handler_guard`). Two honest non-conversions: #5 `null-sentinel-companion-conjunction` (dropped at its own conjunction-or-demote bar — a named FFI-batch lead) + #6 `vec-promote-manuallydrop-not-forget` (thin, perf-neutral). Gates: default 2800/0 · sec 3080/0 · all-features 4466/0 (the b324-missed kernel-side rust_perf witness repaired en route 289 → 292, the B266/B271/B305 class) · clippy −D × postures · score-bench #220 FLAT (freeze NOT owed) · binary rebuild NOT owed (sec not in the release set) · retrieval floors re-pinned in-code (self-variants 35/35 top-1 — the out-param fixture's status-code return keeps it out of the catch-unwind proposer's unit-return territory; router annotation 84/195 → 85/210 at measured 90/210, the B131 sec-centroid mechanism recorded). Fixtures ORIGINAL (never uniffi source verbatim — license hygiene + binary-only rule). Claim `22ae33d8` folded to the landing commit.

## 2026-11-02 — Proposal 022 outbound fix-PR wetrun #1 SENT: mozilla/sccache PR #2977 (issue 198 open, Plan 331 — filed as Plan 329, renumbered 2026-11-03 first-allocation collision loss — bench 127 the record)

Owner-directed wetrun over the Mozilla quartet (025-grant evidence). Scan: ALL targets clippy-clean at their own posture (uniffi-rs 0/598 @ pinned 1.94.0; sccache 0 @ their CI flags @ MSRV 1.91.0; mp4parse 0; rca dormant-skip; supply-chain data-only) — the 022 mechanical gate yields zero clippy PRs, honestly. Pivot to the issue-driven lane: sccache#2975 (CACHEDIR.TAG) healed on branch `refine/cachedir-tag` @ `b97bcc0` (80 insertions, 1 file, +2 tests; tag bypasses the LRU so eviction can't delete it, and the size limit so tiny caches init); G1 at their toolchain: 598 lib tests green, their exact CI clippy flags 0 findings, fmt clean. Fork `riir-ai/sccache` (org repo-create confirmed) → **PR https://github.com/mozilla/sccache/pull/2977 open 2026-11-02T10:05Z** (mergeable: true, 0 comments at open). Distill coupling: intake P113 (upstream-STATE check for mining candidates — Research 362's A-lead was sccache's OWN landed fix); the uniffi waker lead WITHDRAWN pre-PR (RawWaker contract, not a defect). `.raw` clones stay until the PR resolves (rework posture), then T8 removes them.

## 2026-11-03 — Issues 196 + 197 REMOVED (both CLOSED with durable records): the route-narrow measured-negative + the methodology port landed

- **Issue 196 (the kernel_opt route-narrowed matcher index): CLOSED MEASURED-NEGATIVE** — Bench 126 (2026-11-02): the narrowed index FAILS the pre-registered G5 ceilings (added p50 45.8 ms vs flat's 36.3 ms vs the 1.0 ms bar) and 14/15 on the utility floor; the flag stays opt-in for replication, never default; the G5 cost fence's next lever named (substrate-side score-only retrieval, riir-neuron-db's lane). The full verdict + the two standing by-products (the embedding-dot-routing refutation 16.7% ≈ random; the K≥6 quality datum) live in AGENTS.md's `kernel_route_narrow` bullet (lease zed-riir-g5 closed 2026-11-02 in-file).
- **Issue 197 (the Mozilla-quartet methodology port): CLOSED 2026-11-03** — T0b landed (P1 pins manifest + P2 VerifyReport.refusals + P3 DOMAIN_REGISTRY + P4-a toolchain capture + P5 audit already-satisfied; P4-b filed as issue 198; verdict round 2 AGREE; execution zed-m3-24c8b43a by owner directive, the amo lease superseded via agent-mail). The full landing is the Plan-329 HISTORY row above (2026-11-03); the issue file added no fact beyond it.

Folded from `.issues/196_kernel_route_narrowed_retrieval.md` +
`.issues/197_engine_mozilla_methodology_port.md` (removed per the noise-reduction rule — the AGENTS bullet and the Plan-329 row are the durable records).
Commits: the landings (Bench 126's; `7d0752b07` for T0b), this removal.

## 2026-11-02 — Issue 197 CLOSED: the pre-existing develop reds healed (file removed per the noise-reduction rule — this row + commit `3c5004fb9` are the durable record)

T5 pin moved `rust:1.98.1-alpine` → `rust:1.99.0-alpine` (riir-chain `de1b3d9f` bumped the tag deliberately; the file's own dated-comment precedent). All-features clippy: the Issue-153 doc comment re-attached to `run_update_surface_hooks` (orphaned by the Plan 325 insert `7b8aebe51` — root cause, not lint-silence), `doc_markdown` backticked, `cast_precision_loss` house-allow. Reproducers re-run green: docker_t5_control 10/10 · all-features clippy clean · points wired posture 3018 · regime 7/7. Claim `1c0a2c671` folded to the Lease-closed row.

T5 pin moved `rust:1.98.1-alpine` → `rust:1.99.0-alpine` (riir-chain `de1b3d9f` bumped the tag deliberately; the file's own dated-comment precedent). All-features clippy: the Issue-153 doc comment re-attached to `run_update_surface_hooks` (orphaned by the Plan 325 insert `7b8aebe51` — root cause, not lint-silence), `doc_markdown` backticked, `cast_precision_loss` house-allow. Reproducers re-run green: docker_t5_control 10/10 · all-features clippy clean · points wired posture 3018 · regime 7/7. Claim `1c0a2c671` folded to the Lease-closed row.

## 2026-10-31 — Issue 175 REMOVED (mainnet frozen for refine): owner directive "no more mainnet" — the migration defers to this record + plan 302's re-genesis posture; proposal 023 status now DEFERRED-FROZEN. STANDING DIRECTIVE carried out of the file: any key ACTUALLY EXPOSED rotates NOW regardless of the freeze; routine rotation belongs to the re-genesis ceremony. Full file life: `git log --follow -- .issues/175_refine_mainnet_account_migration.md`. Part of the public-goods pivot (issue 192).

## 2026-10-30 — AGENTS.md compaction: the Role-bullet per-batch rule arcs (Batches 282–319) moved out of AGENTS.md

rust_perf: B282 Plan 286 · B283 Plan 287 · B284 Plan 288 · B288 Plan 293 · B298+B305 Plan 289 (entries_52) · B308 Plan 313 (entries_54) · B316 Plan 316 (entries_55) · B318 Plan 318 (entries_56) → 287 rules. kernel_opt: B283 Plan 287 · B284 Plan 288 · B286 Plan 291 · B288 Plan 293 · B295 Plan 300 · B296 Plan 301 (gpu_132) · B297 Plan 303 (gpu_133) · B299 Plan 304 (gpu_134) · B302 Plan 307 (gpu_135) · B303 Plan 308 (gpu_136) · B304 Plan 309 (gpu_137+138) · B307 Plan 312 (gpu_139) · B309 Plan 314 (gpu_140) · B310 Plan 315 (gpu_141) · B317 Plan 317 (gpu_142) · B319 Plan 319 (gpu_143) → 1033 rules.
Rule-by-rule arcs: the batch plans + the pre-compaction AGENTS.md at `403d72bbb703`.

## 2026-10-30 — Issue 189 tails DONE: the wire-KAT census render + the E-5 live refusal + plan-302 T1 ticked (file removed per the noise-reduction rule — plan 302 + this row are the durable record; T3b + promote stay owner-parked in plan 302)

Three tails of the Issue-175/plan-302 T-E2E battery, landed `1efc918d` (session zed-m3-189-tails): (1) the two census render sites (`kat_migrate_client.rs` …)

## 2026-10-28 — Issue 186 CLOSED: structure-grounded synthetic retrieval-eval fixtures land as `draft::eval_synth` (file removed per the noise-reduction rule — Research 339 is the durable record)

The STAIR recipe (arXiv:2609.03874 via ndb Research 311) landed tier-1: gold derives from the corpus's own structure (every span from rule R's `buggy_code` carries gold = R, zero teacher opinion), questions are three deterministic mutations (`ident-rename` / `comment-strip` / `reindent`, identity-skipped).

## 2026-10-27 — Issue 185 CLOSED: the `softmax-collapse` rule family ships with a zero-unexplained-hit workspace sweep (file removed per the noise-reduction rule — this row is the durable record)

T1–T4 (`17965377`): `sigmoid-minus-one` (MECHANICAL, `FIXABLE_SEC`: `σ(z) − 1` → `−σ(−z)`, split one-hot form, name-allowlisted σ), `softmax-onehot-collapse` + `neg-ln-softmax-prob` (suggest-only, row evidence in walk order per innermost fn), the new APPEND-ONLY `SecFamily::Numerics`, before/after fixtures from six …
Commits: `ad394fc6`, `38b62684e`, `11a00d6`, `b07f40af`.

## 2026-10-27 — Issue 181 CLOSED: the gitsync `:selfheal` proved itself on the first real wedge — close condition met in the scheduled 12:15:33+07 run (file removed per the noise-reduction rule — this row is the durable record)

The wedge (sync-repos.log, the 2026-10-27 12:15:33+07 run): riir-infer's sync died with "Your local changes to the following files would be overwritten by merge: .issues/1134_norm_pair_epilogue_fold_rung.md" …

## 2026-10-24 — Issue 169 CLOSED: FLINT multi-vector T0 kill-test measured KILL — full-token MaxSim ≤ the shipped matcher at EVERY width (file removed per the noise-reduction rule — this row is the durable record)

The pre-registered kill-test ran FIRST (T0 before any pooling code): hashed `op_tokenizer` op-token embeddings (BLAKE3-XOF → D u32 → f32 [-1,1] → L2, scheme pin `3466d6ed36f96a0c`, document-side dedup EXACT under MaxSim) scored by the shipped `maxsim_score`, vs the shipped `LatentMatcher`, ONE binary at default …
Commits: `794771c3`, `9a7399c6`.

## 2026-10-19 — Issue 168 CLOSED: the "5 stale lint keys" were NOT renamed/removed — all five are Batch-230 AHEAD-OF-PIN; the coverage report partitions them by the registry (file removed per the noise-reduction rule — this row is the durable record)

The issue's premise refuted by probe at both drivers (2026-10-19, `clippy-driver -W help`, checks sections only — the `-W help` lint names are kebab-case, the underscore greps silently read zero): at the pinned 1.98.1 none of the five names exist …
Commits: `dfd4dd11`.

## 2026-10-18 — Issue 166 record CORRECTED: the T1 inventory's `evolve.rs` row was false — `score_candidate` has a production weighted-sum consumer; the capacity-law ABSENT verdict survives on corrected grounds

Found by the closure-verification pass (a read-only re-audit of `cdb53f81`/`5012782d` against the code at `origin/develop`). The T1 row "`evolve.rs` `score_candidate` has zero production consumers (test-only API)" and the verdict sentence "no serve-path weighted-sum/absolute-threshold consumer of any scored sigmoid …

## 2026-10-18 — Issue 166 CLOSED: the capacity-law lever class is ABSENT in the healer (file removed per the noise-reduction rule — this row is the durable record)

T1 inventory OVERTURNED two of the issue's own table rows: (1) `traj_store` `sigmoid_dot_8` is NOT pure ranking — its one consumer fuses it into a weighted sum (`sort_by(bm25/max + latent)` before the top-k truncate), and the serving consumer (`cf_workers_ai::growing_exemplars`) re-sorts the fused pool by (tier …)
Commits: `cdb53f81`, `78d20871`, `272995d4`.

## 2026-10-17 — Issue 166 CLOSED: the `tuna` verb names the account state, not the deployment, on the absent pair (file removed per the noise-reduction rule — this row is the durable record)

The `/history` wire omits the zipped `tuna_balance_micro` pair when the account holds no SPENDABLE trial credit — a consumed free tier (the KAT-grandfathered fleet account `c79c…5345` on mainnet) and a genuinely pre-TUNA worker read identically, and the None-arm blamed the deployment ("pre-TUNA deployment") for the …
Commits: `9582bf8`.

## 2026-10-16 — Issue 164 CLOSED: the install-probe plain-copy hazard — installers now symlink-aware (file removed per the noise-reduction rule — this row is the durable record)

Hazard: `install.sh`/`install.ps1` (gist-rs/cargo-refine) replaced a developer's `~/.cargo/bin/cargo-refine` symlink with a plain release-cut copy; later local rebuilds propagated nothing (measured both boxes 2026-10-13).
Commits: `26e935e5`, `59bbcd0f`, `d9a75b3`.

## 2026-10-12 — Issue 160 CLOSED: CRLF line-ending contamination — all 7 legs normalized, workspace doc-surface census reads 0/7 (file removed per the noise-reduction rule — this row is the durable record)

Six legs landed 2026-10-11 (refine `7fbca3a7b` 82 · reflex `9e517d2` 2 · reflexer `68cdaae` 1 · ai `ff56c5e113` 26 · katgpt-rs `f2a4fd5a7` 23 · infer `e7b0e49` 6 …)
Commits: `afc0a249`.

## 2026-10-12 — Issue 162 CLOSED: formatting-invariant dense retrieval — T2 measured, T3 decided NOT ARMED (no transferable formatting subspace) (file removed per the noise-reduction rule — this row is the durable record)

T1 `cea4d679` (cycle-22): frozen pair dataset `assets/format_pairs_v1.jsonl` — 4,213 pairs / 37 families / 1.48 MB / sha256 `85dea25b…46e79e` (mined via the existing `--dump-trajectories` export; uninlined_format_args 72% dominance) …
Commits: `e6c02dae`.

## 2026-10-12 — Issue 161 CLOSED: module-applicable pool candidates — pool birth point 3 (the proposer member) + the `rewrite` merge strategy + the behav driver repairs; T1.6 re-computed PASS (file removed per the noise-reduction rule — this row is the durable record)

Landed `1afd5d15`: pool widened with proposer members (POOL-ONLY, `heal_with_pool.result` bit-identical to `heal` — the R6 seam contract; plain `heal` pays one branch, G4 untouched) + the `rewrite` whole-module merge (a file-preserving response merges WHOLE) + the behavioural driver's float/int-slice extension …

## 2026-10-11 — the 089–158 close-out audit: one missing record BACKFILLED (Issue 112), two index gaps named (137 → bench 103; 142 a legal burned number) — file:record agreement now total over every removed 089–158 file

Issue 112 BACKFILLED (false-claim repair, the Issue-798 class): removal `fcb4a15b` (2026-09-16) claimed a HISTORY row that was never written; 112 = the probe-identity WITNESS — primary records are its removal message + the 235-line `.issues/112_probe_identity_witness.md` in git history (this row is the index) …
Commits: `8e67ee16`, `2839c361`, `d25bb830`, `941e049e`.

## 2026-10-09 — Issue 156 CLOSED: the FlyBy state-level rescue mining filter (b0=0 ∧ bd≥3) — T1–T6 all landed, yield honest-EMPTY locally, live-fire owner-gated (file removed per the noise-reduction rule — this row is the durable record)

Code half 2026-10-08 `cf34f6c8` (T1/T2/T3/T5): ungated `src/rescue_mine.rs` (`rescue_keep`, `RescueTally`, BLAKE3 `keep_set_hash`) + arm-H per-state `b0`/`bd` tallies (`FLYBY_CONTINUATIONS_PER_ARM=4` documents the paper denominator) + five serde-default `TrajRowDoc` rescue fields (wire-safe) + the `--misses` …
Commits: `86fe38a`.

## 2026-10-08 — Issue 150 RESOLVED: the Homebrew cargo-refine shadow — option 3 (build-commit stamp) landed, option 2 (brew unlink, verdict-REVISEd to reversible) applied, the standing check fired twice in one day (file removed per the noise-reduction rule — this row is the durable record)

Found 2026-10-06 (dapps plan-045 verdict round 2, M3): the tap Cellar 0.2.1 binary shadowed `~/.cargo/bin` on PATH — bare `cargo refine` silently ran the RELEASED build.
Commits: `0a4c010c`, `3a9d1e96`, `6ee548ca`, `32d0ac34`.

## 2026-10-08 — Issue 133 CLOSED-NEGATIVE: Matryoshka-k supervision for `rule_embed` — the shipped fixed objective wins at every measurable depth (file removed per the noise-reduction rule — this row is the durable record)

Reopen fired on the count bar 2026-10-08 (1095 in-vocab span↔rule pairs = 4.1× the Bench 099 bar, tier 800 = 3.0×; doc_markdown 47% of pairs); Claude arbitration `9537e7f3` took B1 with three binding conditions. T1 randomized-k matrix (five arms × k ∈ {1,3,5,10,20} × both BLAKE3 splits): NEGATIVE (bench 109) …
Commits: `007435e5`, `3ca8ac4d`.

## 2026-10-08 — Issue 148 CLOSED: the traj-mining lane named + labeled + deployed — T-a/T-b/T-c all landed, no healer-side work remains (file removed per the noise-reduction rule — this row is the durable record)

T-a the training-source designation paragraph in `src/kat_sync_payload.rs` (landed 2026-10-05; katgpt-rs joined the lane same day via the T-c lever-1 verdict: 385 rows / 353 new).
Commits: `34d32063`.

## 2026-10-08 — Issue 153 CLOSED: CLI version alert + binary self-update + the stale/new corpus surface — T1–T5 all LANDED, zero open code items (file removed per the noise-reduction rule — this row is the durable record)

Owner ask 2026-10-08, verdict GO/GO/GO/GO + one deferred. T1 the alert (`src/update_check.rs`, `RIIR_REFINE_UPDATE_CHECK=0` opts out, `CI` skips); T2 `cargo refine upgrade` (verified download → SHA256SUMS → backup-swap, refuses dev builds + unknown triples; `RIIR_REFINE_AUTO_UPGRADE=1` opt-in) …
Commits: `bdc2bad0`, `fcd3f2db`.

## 2026-10-08 — Issue 155 FIXED: the 154 fix commit never backticked the allowlist cell — prose-green residue caught by the 172nd scoped boundary run (file removed per the noise-reduction rule — this row is the durable record)

`6135b0f3`'s message claimed "backtick the cell; C3 green" but its diff touched ONLY the drift-ledger prose — the `May depend on` row kept its bare column-1 cell, so `allowlist()` (family-regex or backticked tokens only) still read `reflexer-vessel` as UNDECLARED …

## 2026-10-08 — Issue 154 FIXED: the reflexer-vessel BOUNDARY.md allowlist cell backticked — the 170th boundary run's C3 shape finding (file removed per the noise-reduction rule — this row is the durable record)

The Plan-202-R4 row spelled column 1 bare (`| reflexer-vessel |`), and `ci_boundary_contract.sh`'s `allowlist()` recognises only family-regex names or BACKTICKED cells — the committed edge read as an undeclared C3 violation, workspace gate exit 1.
Commits: `7fdd7b08`.

## 2026-10-08 — Issue 152 FIXED: sync_export_cli green again — the seeded rows carry their src_path now, and the fail-closed law is pinned at the process boundary (file removed per the noise-reduction rule — this row is the durable record)

Root cause (as filed): `21f76593` (2026-10-03, riir-dapps 119 T2 — empty-path fail-closed) landed after the test file; the suite seeded legacy rows via `TrajStore::record` (no `src_path`), so the DEFAULT policy's non-empty `allow_paths` blocked them at `gate_row` → 2/4 red since then (pre-existing …)
Commits: `1759fd41`.

## 2026-10-08 — the RIIR_HEAL_* compat shim RETIRED + the live .heal path residue closed (owner call: "heal is retire for refine branding")

The owner adjudicated plan 192 P3.1's legacy-name honoring lazy: `install_env_rename_compat()` deleted from lib.rs + all three bin mains (`5aa91f82`) — legacy `RIIR_HEAL_*` no longer honored anywhere in this repo; AGENTS.md kill-switch rows → the `RIIR_REFINE_*` spellings the code reads …
Commits: `f71bf4a`, `21f76593`.

## 2026-10-08 — Issue 102 CLOSED: corpus lifecycle management (Library Drift adoption) — T1–T5 all LANDED, zero open items (file removed per the noise-reduction rule — this row is the durable record)

Landed: Research 167 `1cb35cce` (2026-09-16) · T1 the `--frontier-report` fade section `56f0b0fd` (report-first, live REFUSAL under the population floor …)
Commits: `d0457204`, `8e0d52f6`, `d9d69cfc`, `4233f7b9`.

## 2026-10-08 — Issue 148 T-b follow-up LANDED: the rustc_errors driver-lane strategy-menu capture (the second lane on the v4 label path)

Capture (`34d32063`): the `--fix-compile` driver computes the strategy menu at each apply decision — chosen-site candidate strategy ids MINUS the chosen one, sorted + deduped, capped at `REJECTED_CAP` …

## 2026-10-07 — Issue 148 T-b: payload v4 — the rejected-candidate label (the suite-mint training source rides the traj lane)

Capture: ONE `matcher.suggest_file` per edited file at `--fix --write`; each edit joins its OWN-PASS line (chunk-neighborhood, exact at pass 0) and carries span rules minus the applied rule (cap 16) into `record_applied` + the post-verify `record_traj`/`record_traj_fail` via `EditAtLine.rejected` …

## 2026-10-07 — Issue 151 CLOSED: the secret-blind surface shipped end to end — scan/warn/blind live, marketing card deployed, the --blind red-window closed (file removed per the noise-reduction rule — this row is the durable record)

Surface (this repo): `56d73162` (`src/secret_blind.rs` — `--secret-scan` report with blinded contexts + the quiet `[sec]` in-run warn + `--secret-scan --blind` shareable text; fingerprint never on stdout) · `675b2acf` (record) · `3d2c6208` (reef fixture) · `fd414fc9` (docs).
Commits: `eed88c6`, `e8ab39f`, `47057d3`, `4b4125a`.

## 2026-10-07 — the 9cfd0c33 corpus expansion's red window closed: the ncswap fixture E0422 + the gen4 re-freeze + the all-features lane unblocked

Three compounding pre-existing reds behind the recorded riir-infer-gpu E0308 (which blocked every all-features lane): (1) the sibling E0308 (riir-infer `d09d1ec`, the metal upload arm's `attn_wo` missed the `q4_refuse` wrapper); (2) ncswap ground-truth `Tkn` unqualified from the child `mod ground_truth` …
Commits: `fac3dc36`.

## 2026-10-06 — dapps plan 045 (issue 117): the refine-side lanes recorded (E1 sweep half) — A1/A2/A3, A8, the verdict follow-ups, B7

- A1/A2/A3 first-join env choice `8c4e67b0` (ships v0.2.1) via the existing config-set writer; A8 stranger drill `8b08ed43` PASS live on devnet; verdict follow-ups `e5a24918`. B7 `cargo refine topup [--pack N]` `c80a5794` — USDC→KAT checkout, composer canonical in riir-kat `26ea7f4` (the Issue-082 one-definition law); LIVE SMOKE PASS (dapps `.benchmarks/018` B7 addendum); global binary rebuilt v0.2.1.

## 2026-10-06 — Issue 149 RESOLVED: the v0.2.1 capture QA findings — pre-scan disclosure, cross-pass resplice dedup, and the measured 0.1.98 firing-surface correction

- T1 `scan root:` disclosure line before the first read; T2 `dedup_respliced_edits` with authority-follows-replacement + one loud `accounting:` line (file text + revert machinery unchanged); T3 probe on clippy 0.1.98: both disputed lints still EXIST — the coverage report's declared-lint axis cannot see firing-surface narrowing; both rules carry context_notes.

## 2026-10-05 — Issue 146 CLOSED (T0.6 executed): the profile-guided-healing REAL-workload arm — rethink ESC loop profiled end-to-end, alloc gate flat, the G4 pin held, the heal landed at rethink `4cd8dcf`

- First REAL-repo `[hot]` feed (rethink ESC decide loop, sst5, 1205 decides, pinned SHA `d01a8e5`): measured-path allocs are ownership-driven exact clones — no corpus surface; structural surface filed as leads to riir-instinct/riir-infer. 3 verified edits landed at rethink `4cd8dcf`; alloc gate +0.0% everywhere, `esc_g4_alloc` pin 190 held. En-route: `~/.cargo/bin/cargo-refine` was a plain FILE COPY, now the documented symlink → rebuilds propagate.

## 2026-10-05 — Issue 146 T0.2+T0.3+T0.4: the profile-guided-healing oracle lane LANDED end-to-end — the T0 POC gate PASSED on a real hotpath run (alloc −51.1%)

- hotpath JSON = second oracle beside `--clippy-json` (`src/hotpath_report.rs`); `[hot]` spans hoist first by time share (ranking-only), marker precedence `[oracle] > [hot] > [heuristic]`; fix lane declines edits overlapping hotpath's own instrumentation lines. E2e gate `scripts/hotpath_e2e_poc.sh` PASSED: alloc bytes −51.1% (4.7 GB → 2.3 GB, N=3), timing 2.1×, instrumentation survived byte-identical.

## 2026-10-05 — Issue 147 CLOSED (T0′ executed): the Decision-CPI escalation tier killed on the corpus axis — headroom GO, corpus 19× off + converged

- Headroom GO: 141 spans, θ=0.05 subset top-1 0.750, max recoverable +2.8pp banked. Corpus KILL: fleet contributed 7,513 / credited 533 = 19× off the 10⁴ target — the tier CLOSES, never-per-span law stands; re-open at the 10³–10⁴ clean-context class (riir-dapps Issue 148 T-c levers).

## 2026-10-05 — Issue 138 CLOSED (owner-gate pickup): E7 ruled free-tier-only — operative DENY until a cap lands; E8 C8/C9 draft ADOPTED (master riir-ai `.issues/1016`; plan 181 / issue 138)

- E7: the standing CF sign-off loop is free-tier-only; metered use DENIED until an enforced daily cap with auto-stop lands. E8 ADOPTED (delegated verdict): C8 audits each git-rev pin individually; a multi-rev split on one owner is C9; same-branch lag-pin splits read as C9 NOTES, branch-mixed splits stay loud C9.

## 2026-10-03 — Issue 145 CLOSED: first-run output reads like hand-written Rust (`d40f7b5e`)

- `needless_range_loop` binding = collection's singular else `item`; `DomainId::applies_to_path` display filter (a span lists only domains that can touch its file kind; `--fix` unchanged). T2 deferred as optional polish.

## 2026-10-02 — Issue 143 LANDED: the retrieval GOAT's G2 bar dual-pinned per host (the Issue-filed platform gap closed at root cause)

- `tests/latent_suggest_goat.rs` carries `G2_BAR_US`: `1000.0` under `#[cfg(target_os = "macos")]` (historical bar unchanged), `3000.0` elsewhere; reading always prints beside its bar + OS. G2 1505 µs/chunk PASS on this box; 5/5.

## 2026-10-02 — Issue 141 CLOSED: the four pre-existing integration-red classes (A fixture-era floor, B the two-consumer fixture, C a REAL v2 client regression in riir-kat, D the level-gate fixture)

- Full integration repro green: 69 targets / 3086 passed. A: wetrun fixture attests at the floor (3). B: eval-span spelling `b.count_ones()` — a fixture feeding BOTH the clippy oracle AND labeled eval spans must be verified against both in one landing. C: REAL riir-kat client regression — `seen_live` flag pins fully-down→queued vs live-then-quiet→bail. D: the e2e fixture now DECLARES the lint it teaches (`[lints.clippy] match_bool = "warn"`).

## 2026-10-01 — Issue 141: the `cargo refine` cargo-convention argv leak (the rename's one real regression, found from the owner's brew question)

- cargo passes the binary's own name as argv[1]; the strip now removes BOTH `refine` and `heal` spellings; `tests/cargo_convention_cli.rs` 4 arms at the process boundary. Brew answer (owner-gated T1.2f): the tap still serves `cargo-heal` until the owner-side dist rename flips.

## 2026-10-01 — `repo_posture_cli` red CLOSED + the mid-posture clippy red + three owed `[[test]]` rows (the rename's flagged remainder)

Root cause: the 3 applying arms ran `--fix` without `--no-verify` — red only under `--all-features` where `fix_verify` compiles in; fixed with `--no-verify`. Plus: `heal.rs` unreachable-`return` at the mid posture moved inside the `kat_transport` block …

## 2026-10-01 — The refine rename, Phase 1 landed (`ac4b3d16` + docs; Proposal 018 / Plan 192): binary `cargo heal` → `cargo refine`

- `[[bin]] cargo-refine`, all display strings + test pins in lockstep; riir-kat leaf swept too (18 files / 71 replacements). Internal vocabulary unchanged (`heal.rs`, `HealDomain`, `RIIR_HEAL_*`); dist URLs stay `cargo-heal` until the owner-side flip.

## 2026-09-28 — Owner-gate pickup E3/E4/E5/C7/E8 (master riir-ai `.issues/1016`; plan 181 / issue 138)

- E3 dead `gist-rs/riir-unity` checkout gone from `ops_dashboard.yml`; E4 stale-no-op (the 4 never-defined-feature targets never existed — verified); E5 one-line disambiguator in `.research/083_omlx_external_distill_batch71.md`; C7 `structural_admission` stays INERT until a priced first consumer; E8 draft recorded (ADOPTED 2026-10-05, see above).

## 2026-09-26 — Issue 135 CLOSED (`c1c6cf21`): `cargo heal --fix` gates allow-by-default lints on the target crate's effective level

- `LINT_GROUPS` from the pinned `clippy-driver -W help` (20 allow-by-default lints, not 13); `LevelResolver` over `[lints.clippy]`/workspace lints/crate+file attrs; `--groups pedantic,nursery,restriction,cargo|all` (requires `--fix`); `doc_markdown` skips shell-command doc lines. Measured 0 live instances of the undocumented skip-direction forms workspace-wide.

## 2026-09-25 — Batch 179 LANDED (Plan 172 / Research 210): the rsconstmap distill — the std-map-probe family's layout + MLP + map-selection cells

- lemire/rsconstmap @ `bf7b3341`: `paired-probe-layout-present-vs-absent` (1.68×/1.45×/1.80×), `block-hash-before-gather-mlp` (1.68×/1.30×/1.02×), `immutable-static-map-filter-selection` (2.09×/4.27×, `[dep: constmap]`) — all ParetoDominant; fixture lesson: derive index arithmetic from the source (`h * m` multiply-high), not habit. rust_perf 153→156.

## 2026-09-24 — Issue 134 CLOSED (T3 re-arm executed): seven-domain sweep, the ρ=0.536 artifact, and the degenerate-axis guard

- The sweep refused on a degenerate gain axis (Spearman against a constant vector renders a pseudo-value); final verdict REFUSED — `demonstrability_gate` stays default-off BY VERDICT; T1 standing instrumentation survives. Latent `HeadroomReport.per_domain` `[_; 3]` OOB fixed.

## 2026-09-24 — Issue 134 T4: the fanout controls — best-single-domain + homogeneous-at-budget

- `EvalReport.homogeneous` (equalized-budget pass) + best-single-domain ceiling row. First live read: homogeneous@budget = blended EXACTLY — blended top-1 is entirely per-domain selection, ZERO fan-out contribution. Rider: `print_report`'s `names` had 6 entries vs 7 — fixed.

## 2026-09-24 — Issue 134 T3: the ρ-prediction depth sweep — instrument landed, verdict REFUSED at the power floor

- `examples/demonstrability_t3_sweep.rs` (pool ladder 28→56→112, hand-rolled Spearman): every depth curve FLAT; n=3 qualifying domains < the ≥5 power floor — honest refusal, re-run after the T4 fixture re-arm. Kernel fixtures are corpus-derived (de-bias note).

## 2026-09-24 — Issue 134 T2: the pool-depth allocation gate (`demonstrability_gate`)

- `src/demonstrability_gate.rs` (ungated policy) + `cfg(demonstrability_gate)` matcher wiring scaling pool AND truncation K; kill-switch `RIIR_HEAL_DEMONSTRABILITY=0` guards the auto-arm loader only. Standing constants round INERT at pool 28/K 3 — correct at two nearly-equal constants. Landing's own `bool::then_some` inversion caught by its tests pre-commit.

## 2026-09-24 — Issue 134 T1: the standing demonstrability column (SAT / katgpt-rs Research 585)

- Per-domain demonstrability constant (formula corrected to `resolved / (resolved + persist)`); `src/rule_domains.rs` moved UNGATED from `kat_sync_payload` (re-exported at its historical path — goldens unchanged). First readings: clippy 0.935, rust_perf 0.962.

## 2026-09-24 — Batch 174: the cubecl/bevy merged-fix distill (host-lock + flush-hoist + the B56 rider)

- `host-lock-across-reentrant-blocking-call` (NotBenchable) + `deferred-command-flush-hoist-out-of-batch-loop` (ParetoDominant 21.3×/13.3×/16.5×, alloc flat; the bench-honesty arc took 3 iterations to an honest toy). rust_perf 151→153 + kernel_opt B56 rider (cubecl #1686 double-checked-lookup).

## 2026-09-23 — Issue 132: the role-perspective e2e flows — doc + four per-tab SVGs + the front-page embed (file removed per the noise-reduction rule — this row is the durable record)

- Four role e2e SVGs (`e884f0f5` here) + the riir-dapps front-page embed (`eefd8cd`, mermaid id-suffixing for 5 inline SVGs sharing one document, `svg_contract.rs` extracted). Deploy live-verified devnet `323d4ff6` + apex `375f95f2`.

## 2026-09-23 — Issue 131: the e2e self-evolve flywheel as a doc diagram + the KAT-pages static SVG hero (file removed per the noise-reduction rule — this row is the durable record)

- `.docs/10_self_evolve/self_evolve_flow.md` + static SVG (`ed7bf3aa` here) + riir-dapps mirror `82933de`; hero LIVE on devnet verified same day.

## 2026-09-23 — Batch 173 / Plan 167: the serde zero-copy family distill, rust_perf 149 → 151 — the corpus's first dedicated serde rules

- `serde-borrow-deserialize` (ParetoDominant 4/0/0; alloc axis EXACT 300016 → 16 at n=100k; escaped-payload regime witness printed in-bench) + `rawvalue-passthrough-defer-parse` (3/0/0; alloc 950000 → 100000 at n=50k). Both suggest-only → freeze tables structurally unmoved.

## 2026-09-23 — Issues 089+102+130 batch: active-set lifecycle (T2/T3/T4), the admission-gate REPORT half (T1+T3), the owed gen-3 sweep #161, and the switch_cost cross-fire adjudication

- `src/active_set.rs` landed gate-CLOSED (`--active-set-demote/readmit/clear/cap`; nothing demotes automatically); `src/gate_calibration.rs` report-only (δ=0 prints UNCALIBRATED, both stores STALLED); gen-3 sweep row #161 in a detached worktree; switch_cost cross-fire adjudicated TRUE_POSITIVE (clippy `map_clone` on the perf fixture idiom).

## 2026-09-23 — CI sibling provisioning: riir-infer joins every checkout (the carve's T2.3 riir-clippy half) + release.yml's pre-existing kat-plane gap fixed

- cargo resolves path deps even behind `optional = true` — `riir-infer` joined rust.yml's sibling loop; release.yml was BROADER and PRE-EXISTING broken since 2026-09-10 (riir-kat spin-out): both loops now clone `riir-ai riir-neuron-db riir-kat riir-auth riir-llm riir-infer`.

## 2026-09-23 — P36: the `[C2·documented-write-only]` downgrade (config-audit; the seal-remake `FeatureFlags`/`SessionMeta`/`WasmLimits` idle-pass intake)

- `classify_self_referential` takes `field_docs` (one marker vocabulary home); the EVERY-field rule is load-bearing (one unmarked field keeps the strong verdict — negative arm pinned). seal-remake re-audit live-verifies all three C2s render weak.

## 2026-09-23 — Batch 172 / Plan 166: the mezura rust_perf distill, rust_perf 145 → 149 — the dual-mask GOAT lead at 43×/18×

- `.raw/mezura` @ `2dbacb7`: `dual-mask-fused-block-scan` (ParetoDominant 4/0/0: 43.11×/43.95×/23.10×/18.29×; AVX2+NEON arms; exhaustive G1 caught the `\n`-in-needle-set contract subtlety), `mmap-settled-size-gated-read`, `steal-batch-not-per-item`, `nonblocking-open-for-head-probes` + 2 riders.

## 2026-09-21 — Issue 128 EXECUTED (T1-T7): the fixseq cascade-law + derailment-index report shipped, two mint corpora measured — the law is GENERATOR-CONDITIONAL, G4 fired live, HORIZON(prev) does not raise

- `src/fixseq_stats.rs` + `--fixseq-stats` + T6 transition arm in `horizon_report`; two mint configs committed (`.benchmarks/data/128_t5/`). Cascade law GENERATOR-CONDITIONAL; G4 fired live (first-wrong is a point mass at position 0); silent failure 100% on minted corpora; HORIZON(prev)=1 honest null. Reopen: organic multi-revert rings.

## 2026-09-21 — P35: the `format_collect` import goes to the INNERMOST enclosing mod — the cfg(test) unused-import class closed (queue item 14, Protocol C)

- Probe reproduced the incident live (import at file scope + usage under `#[cfg(test)]` → unused_imports warning; `--verify` blind by construction). Repair: mod-stack tracking lands the import at each innermost mod; any top-level site keeps one file-level import. 5 fixtures, module 19/19.

## 2026-09-21 — Plan 165: `--repo-posture` — the repo-lint-posture fix mode (the intake's OPEN product suggestion, closed)

- Clippy-lane edits apply only where the target repo's own clippy feed carries a same-lint diagnostic whose primary span overlaps (215 proposed vs 16 real in the motivating intake); `map_span_through_splices` + `posture_spans`/`posture_allows`. No feed → REFUSE exit 2; `tests/repo_posture_cli.rs` hermetic.

## 2026-09-21 — Plan 164 (queue item 14, Protocol C): the three 09-21 post-mining intake classes repaired at root cause (`eb4827ea`)

- Class 1: const-generic-arg-for-runtime-size refused via a file-wide binding pre-pass. Class 2: `unreadable_literal` consumes `excessive_precision::shortest_round_trip` (one decision, two consumers); the real shape was unsuffixed literals in `[f32; N]` fixtures — refusal CORRECT. Class 2b: `--all-targets` is the auto-oracle DEFAULT (`RIIR_HEAL_AUTO_ORACLE_NO_ALL_TARGETS=1` restores). Class 3: `[wire-const]` runs on non-test sites only (`in_test` bit).

## 2026-09-21 — Issue 089/102 verdict session: the frontier snapshot store discriminator (the x-axis was non-monotone on two unnamed stores) + Issue 102 T1's fade report (report-first, live state REFUSES)

- Every snapshot row now carries `store` = blake3(store root)[..8]; the 09-20 per-store readings were WITHDRAWN as unrecoverable. Issue 102 T1: fade keyed on DISTINCT FULL-CORPUS SWEEPS (≥25 rules attributed); FADED ≠ NEVER-CONTRIBUTED split; live state is a loud REFUSAL until 3 sweeps exist; the ≥3-future-generations clock started.

## 2026-09-20 — Batch 165 LANDED (Plan 156 / Research 193 → RECORD): `unchecked-wire-arithmetic` — the overflow sink beside the alloc sink, on the SHARED wire-taint walker; sec 15 → 16, InputValidation 3 → 4; the Kani property axis closes the loop (g2c/g2d run on the WSL2 lane, the pre-validation block returns to EMPTY)

- `proposers/wire_taint.rs` extracts the Batch-131 walk as a `WireSink`-parameterized substrate; arithmetic sink `7ab7c628`; sink-DISTINCT adjudication (no composition gate). Kani g2c FAILED as desired / g2d SUCCESSFUL on the 4090 WSL2 lane; probe found `arithmetic_side_effects` SILENT on literal/const operands — strengthens the rule's premise.

## 2026-09-20 — the auto-oracle's empty-project-root spawn failure FIXED (idle-sweep find): `detect_project_root` cwd-anchored — the documented `cargo heal --suggest src/` form works on every platform again

- Relative 1-component targets walked to the EMPTY path (os error 123 on Windows / ENOENT on Unix); cwd-anchored before the parent climb (`0f053d0a`), 3 regression tests. The never-generalised-repair shape: `compile_oracle::owning_package_root` fixed this exact class and the repair was never ported. The 9-repo sweep then ran clean.

## 2026-09-20 — Issue 126 COMPLETE (both lanes): the Lemma-4.2 likelihood-ratio tier guard gates trajectory certification — wrong-goal credit and lucky-keep overcertification are now excluded with a priced bound (`src/tier_guard.rs`)

- `success && verified` certifies only when the goal-side Elo model and the row's Laplace history agree within ε=0.5 (γ=0.9 prices the 2γε/(1−γ)=9 slack); counters always record, only the promotion gates; cold start behavior-identical; non-finite ratings fail closed at the input. Store arms in `tests/tier_guard_store_gates.rs`. T0 hindsight-label corpus committed at `.benchmarks/data/126_t0/` (172 runs / 242 rows, BLAKE3 manifest).

## 2026-09-20 — Issue 127 POC EXECUTED, verdict POSITIVE: Kani machine-checks the sec rules' property claims — witnesses exhibited both directions, cost floor sub-second per harness (WSL2 lane)

- Kani 0.68.0 (pinned nightly, WSL2-only — Windows-native unsupported): G1 NaN-unsafe comparator witness exhibited (`-inf, +NaN, +inf`); G2 wire-arithmetic two-sided verified. Harnesses at `.proofs/kani_sec_poc/` — never built by the workspace's stable rustc, never in CI. Owner pulled the staged lead → Batch 165.

## 2026-09-20 — Issue 123 RESOLVED: the TUNA client surfaces are complete + live on devnet — ack v4, the soft gate's second plane, the fuel-gauge tank, the plane-split tail, the depleted-run survey; only the owner's apex/mainnet light-up remains

- ack v4 (`tuna_balance_micro`), soft gate lifts on a positive TUNA balance (`None` never an invented zero), footer `TunaLine`, `plane_split_note()`, depleted-run survey (count-only), `--info` TUNA line. Sole remainder is the owner's apex/mainnet deploy + `/tuna/genesis`.

## 2026-09-20 — P34 CLOSED: the `needless_range_loop` LENGTH-MISMATCH class guarded at all three lanes (oracle door + proposer fixture-pins + the manual-conversion divergence row) — the riir-ai Issue 981 healer lesson

- The 981 rewrite was NOT healer output (hand-applied); all three lanes guarded: oracle door requires a LOCAL SYNTACTIC length proof (receiver-identity correction: the `.len()` must name the collection it indexes); proposer fixture-pins; SKILL divergence row. Residue: all 18 conversions in the known emitter commit audited — 17 sound, 1 already fixed.

## 2026-09-20 — Issue 125 RESOLVED (REFUTED): the CUA-S1 specialist option-scorer PoC loses to the modelless baseline at our corpus scale — harness + 719k tinyx land as the re-run lane ([Bench 098](.benchmarks/098_choice_scorer_poc.md))

- G1 27.7% vs baseline 88.0% in-dist, G1-OOD 5.6% vs 100%, G2 8868 µs/span; shuffled-context control PASSED, frozen control 7.2% — genuinely learned, genuinely starves on 101 gold contexts. `RerankMode::Structural` default stands; nothing wired into any heal path. Reopen: ~10⁴–10⁵ labeled contexts.

## 2026-09-20 — Issue 125's metric-taxonomy half: the CUA-S1 decision-outcome rows land in `retrieval_eval` (wrong-rule/wrong-span, acceptable-alternative gold, the three-way decline axis)

- `ChoiceTaxonomy` on `EvalReport` + `OwnedEvalSpan.acceptable` (zero sets authored by design — an acceptable set is a corpus claim) + covered/declined/proposer-blind axis. First live: coverage 50.5%, selective-accuracy 100% — every error lives in the declined half.

## 2026-09-19 — the two cross-repo hardening fixes (katgpt-rs Issues 832 T5 + 841): scratch-root read-before-write + verify's incomplete-check refusal

- `651728c0`: a killed/truncated recheck is now a REFUSAL, never a VERIFIED keep. `83f4cd89`: scratch-crate work roots get the read-before-write mtime guard (the riir-clippy 9 re-offers = deliberate scratch roots).

## 2026-09-19 — Batch 164 / Plan 155: the coverage-report re-arm — the frontier-lead mining pack X (the EIGHTH consecutive all-FIXABLE pack; corpus 84 → 86, clippy 65/16 → 67/16; TWO freeze bad-atom joins — a first for one batch)

- `f84037c2`: `match_like_matches_macro` + `excessive_precision` (record: `.research/191_batch164_coverage_report_rearm.md`). Bad sets 6→8 (v2) / 7→9 (v3); ladder 86 labeled. Score-bench #140/#141 96.36% FLAT; global binary rebuilt (was 6 batches stale).

## 2026-09-19 — Batch 163 / Plan 154: the frontier-lead mining pack IX (the SEVENTH consecutive all-FIXABLE pack; corpus 82 → 84, clippy 63/16 → 65/16; the recorded-spare queue EMPTIES)

- `6408ea44`: `manual_string_new` + `needless_late_init`; the syn finding: statement attributes fold into the statement span and syn RESPANS inner exprs over it. Freeze regen v2 90/11 + v3 89/16; score-bench #139 96.36% FLAT.

## 2026-09-19 — Batch 162 / Plan 153: the frontier-lead mining pack VIII (the SIXTH consecutive all-FIXABLE pack; corpus 80 → 82, clippy 61/16 → 63/16; THE FRONTIER IS EMPTY — all 17 leads graduated)

- `bytes_count_to_len` + `bool_comparison`; the e2e now records the `all_graduated` state explicitly. `pool_depth_rescue_guard` re-fired third time — RERANK_POOL 24 → 28 + the 32/40 rungs. Freeze regen refusal guard: non-id rows under an EXCLUDED header refused.

## 2026-09-18 — Batch 161 / Plan 152: the frontier-lead mining pack VII (the FIFTH consecutive all-FIXABLE pack; corpus 78 → 80, clippy 59/16 → 61/16)

- `unit_arg` (hoist the side-effect block, re-spell the arg as `()`; the heldout marker must be the in-parens BROKEN-only shape) + `string_lit_as_bytes` (`b"abc"`; refuses the `\u{` escape — clippy's own suggestion there is INVALID RUST). Freeze v2 84/13 + v3 85/16, both e2es PASS.

## 2026-09-18 — Batch 160 / Plan 151: the frontier-lead mining pack VI (the FOURTH consecutive all-FIXABLE pack; corpus 76 → 78, clippy 57/16 → 59/16)

- `from_str_radix_10` (construction identity: std's `FromStr` IS radix-10; no shadow guard) + `assign_op_pattern` (exactly-once binding derivation; swapped-operand arm commutative-only). RERANK_POOL 20 → 24 (the rescue guard re-fired); Batch 158's missing freeze record paragraph + stale `.research/.highwater` repaired.

## 2026-09-18 — Issue 103: sync payload v3 — the per-row `domain` (the corpus-view Phase-2 axis)

- `PayloadRow.domain` (v2→3) from the compiled `RuleDomains` table; receiver `verify_sync_payload` accepts {2,3}, monotone refresh merge; cap hardening — `other` can never be evicted; golden dual-pinned to v3 bytes (v2 preserved as `GOLDEN_PAYLOAD_V2`). En-route: the all-features lane had been red since `8fc52d22` (import at groq.rs top level, module lives in `mod router`); the Issue-088 redact pin was keying on a coincidence (now asserts floor == `REDACT_VERSION`).

## 2026-09-18 — Batch 159 / Plan 150: the frontier-lead mining pack V (the THIRD consecutive all-FIXABLE pack; corpus 74 → 76, clippy 55/16 → 57/16)

- `single_char_add_str` (mut_string_param_heads_over twin; `escape_default` emit) + `manual_clamp` (lattice-absorption; FLOATS refuse on NaN max/min-vs-clamp divergence, inverted bounds refuse on the release-mode assert — both in-rule). Archive step is LOAD-BEARING (archiving consumed rows breaks the family-done deadlock). Freeze v2 78/15 + v3 80/17.

## 2026-09-18 — Batch 158 / Plan 149: the frontier-lead mining pack IV (the second all-FIXABLE pack; corpus 72 → 74, clippy 53/16 → 55/16)

- `to_string_in_format_args` (perf/default-on, scalar-derivation provenance; bare-`{}` placeholder guard; composes with `uninlined_format_args` by design) + `manual_range_contains` (contains IS the two leg comparisons; literal-only bounds). Freeze v2 77/17 + v3 78/18; score-bench #133 96.36% byte-identical.

## 2026-09-18 — Batch 157 / Plan 148: the frontier-lead mining pack III (the first all-FIXABLE pack; corpus 70 → 72, clippy 51/16 → 53/16)

- `redundant_closure_for_method_calls` (construction-identity whitelist: String/Vec/HashMap/HashSet × len/is_empty/capacity; `helpers::slice_param_heads_over` substrate) + `iter_skip_next` (slice containers heal to `v.get(n)` — one step better than clippy's `.nth()`). `unnecessary_sort_by` joined the bad-atom set; 12 orphaned EXCLUDED rows pruned.

## 2026-09-18 — Batch 156 / Plan 147: the frontier-lead mining pack II (the re-armed 2 leads consumed; corpus 68 → 70, clippy 50/15 → 51/16)

- `io_other_error` (FIXABLE; identical payload bound) + `wrong_self_convention` (VERIFY; `into_*(&self)` → `into_*(self)`, NO-vis inherent only). Re-arm probe: `redundant_closure_for_method_calls` + `iter_skip_next`. Score-bench #131 96.36% FLAT.

## 2026-09-18 — Batch 155 / Plan 146: the frontier-lead mining pack (the miner's first 5 leads consumed; corpus 64 → 68, clippy 47/14 → 50/15)

- `let_and_return`, `redundant_slicing`, `map_flatten` (FIXABLE) + `ptr_arg` (VERIFY); `needless_borrow` receiver whitelist extended (str-inherent). `let_and_return` measured BROAD — 3 pre-existing atoms' fixtures fire it too (bad set 2→5). The e2e runs one loop per covered family.

## 2026-09-18 — Issue 122 / Plan 145: the PowerPlay frontier miner PoC (G1–G4 PASS, opt-in)

- `src/frontier_miner.rs` + `examples/frontier_miner.rs`: modelless, deterministic, zero-LLM generator; full invariant loop (unsolved-before / solved-after / regression-green / exactly-once BLAKE3) + per-family K-minimality; 16/16 seeds clean, 6 rows committed. Promotion NOT claimed — rides the Issue-089 band-predictivity clock.

## 2026-09-18 — Batch 154 / Plan 144: the tokio upstream-surface refresh (CHANGELOG 1.47–1.52.3 + "Actors with Tokio"), rust_perf 142 → 145

- `write-vectored-capability-flag`, `oneshot-response-send-unwrap-panics`, `actor-bounded-channel-cycle-deadlock` (all NotBenchable, domain reasons) + 4 annotation folds (spawn_blocking sharded-queue REVERT, LIFO-steal REVERT, coop public API, socket-option family). Process note: build corpus entries with the Python path for multi-line fixtures.

## 2026-09-18 — Plan 141: the devnet burn-process e2e, both machines live (Bench 097)

- `scripts/burn_e2e.sh` 9/9 hard assertions green twice (refuses off-devnet configs + fleet account ids). Headline server-read: 100 KAT grant burned to exactly 0 by 90 metered scans; depleted+never-contributed refused at run start; clamp held; contributed-lift ran clean; conservation held. Findings recorded in Bench 097 §4.

## 2026-09-18 — Plan 035 wave 2 (T3/T10/T6-client/T12): the client becomes a typed µ consumer + the ×1_000_000 grep gate

- `MicroKat` adoption (kat_footer is the unit-vocabulary module; `MICRO_PER_KAT as f64`, never bare 1e6); L4 value-level display pin; `tests/micro_literal_gate.rs` bans the ANONYMOUS million (TOKENS_PER_MTOK beside MICRO_PER_KAT — same value, different dimension). Latent: dead `wallet_lines`/`receipts_line` at kat sets without `kat_transport`.

## 2026-09-17 — Bench 094 wave 4: the third default-eco extension (501 → 663 clean crates, flip 110/442 → 129/442) + the dead cross-wave base-dedup found by audit

- The cross-wave base-dedup in `cmd_sample` was DEAD CODE (bare name vs versioned keys — never matched); fixed with the constructed-key law + a merge-time hard-refusal. Fresh multi-crate head: `io_other_error` 57/8, `unit_arg` 46/2, `wrong_self_convention` 19/6.

## 2026-09-17 — Bench 094 wave 3: the second default-eco extension (342 → 501 clean crates, flip 94/442 → 110/442) + the resume prefix-collision fix

- Resume prefix-match over-matched (`gsk4-sys-0.11.4` startswith `gsk4-`); caught by the reconstruction diff before any number was used; fixed to the exact constructed key. Head: `needless_borrow` 92/30, `redundant_field_names` 58/9.

## 2026-09-17 — the fresh-pass pedantic residue closes (34 + 6 findings, the stale-cache bite confirmed on the Batch-150 close-out itself)

- A fresh-target `cargo clippy --all-targets` at HEAD reported 34 findings the stale-cache close-out had read as 0 — a fresh worktree + empty target dir is the only clippy reading that means "0". The landed doc_markdown rule healed 38 edits (dogfood closing the loop). Fixed the one fresh lib-test failure (worktree cwd artifact in `downstream_hint::relative_target_fires`).

## 2026-09-17 — Issue 121 RESOLVED: the set_rerank consumer landed, promotion REFUTED, the all_rules canonical-order fix is the GOAT (`2aa53e1d5`, Bench 096)

- Bench 096: the canonical-merge control beat both admission arms (85.2% vs 70.4/74.1% top-1); every α-ablation lost to shipped on the OOD rig. The real yield: `all_rules` discarded the T3.5.4 instantiation re-rank across domains — canonical key restored on the DEFAULT path (+55.6pp top-1), regression test landed. Re-open trigger: `pool_depth_rescue_guard` firing again.

## 2026-09-17 — Plan 134 T2.1 COMPLETE: the frame-time budget tier ships (Bench 095)

- `src/frame_time_report.rs` (ungated) + `--frame-time-report` (`--frame-budget`/`--vs`/`--frame-delta-pct`); warmup-parity + pairing-uniqueness evidence guards; refuses exit 1 on empty/unparsable/zero-pairs. Walker matched `Path::extension()` (= `json`) — every real sweep dir read EMPTY; fixed to full suffix match.

## 2026-09-17 — Bench 094 wave 2: the default-eco histogram extension (184 → 342 clean crates, flip 74/448 → 94/442) + the T7.5 fold

- Sampler is tracked instrumentation (`scripts/sample_default_fired_histogram.py`). Two incidents caught before any number was used: cargo's JSON carries diagnostics under `message` not `msg` (the known-dirty control caught it); resume skip compared bare names against versioned keys (merge dedups keep-first). T7.5 fold: corpus-touching landings require the freeze-table regeneration in the SAME batch.

## 2026-09-18 — Batch 153: the async-blocking distill lands (rust_perf 139 → 142)

- `blocking-sleep-in-async`, `rayon-bridge-oneshot`, `tcp-nodelay-request-response-accepted-socket` (NotBenchable, domain reasons) + 5 annotation folds. All source quotes re-verified live at mine time. The stale "Not benchable 36/127" ledger line repaired to 50/142.

## 2026-09-18 — Batch 152: the dial9 tokio-principles distill lands (rust_perf 136 → 139)

- `batch-blocking-pool-segment`, `adaptive-yield-consecutive-ready`, `bounded-fanout-semaphore`. Honest corrections: the mini-redis p50/p99 figures are NOT on the live page — the rule carries only the verifiable ≈10× and the K=4.

## 2026-09-17 — Batch 151: the doc-line state-machine port lands (corpus 62 → 64, FIXABLE 45 → 47)

- `doc_lazy_continuation` + `doc_overindented_list_items` in ONE proposer module; measured: clippy's zero-width insert span sits at the CONTENT start (after the conventional space); +4-over still fires (pulldown reads it as paragraph text); `helpers::doc_lines` substrate extracted.

## 2026-09-17 — Batch 150: the staged default-level + pedantic probe pack lands (corpus 56 → 62)

- FIXABLE +4 (`print_with_newline`/`write_with_newline` one module two names; `unreadable_literal` digit-count ≥6; `legacy_numeric_constants` with a shadow guard); VERIFY +2 (`partialeq_to_none`, `cast_lossless` safe-let gate). DECLINED with evidence: `redundant_closure_for_method_calls` (typeck-resolved path). Rider: the Bench 094 dead test (missing `#[test]` attr — an uninvoked assertion is unknown, not passing).

## 2026-09-17 — the default-level fired histogram lands (Bench 094, Issue 119's T4 successor)

- Sibling-git-history REFUTED as the population (CI `-D warnings` since before the healer); Li et al. transcription REFUTED (Figure 2 carries no printed value labels); measured first-party: 184 clean crates / 761 warnings / 105 distinct default-firing lints. The join flips the cross-tab 0 → 74 of 448; the CONCENTRATION LAW: the nominal top-1 came from ONE crate — read the spread column.

## 2026-09-17 — the manual_range_patterns corpus rule lands (the intake MISS folded, P31)

- VERBATIM port of clippy's own coverage-walk algorithm (`53269d3a`) after probes refuted two naive models; order-free, duplicates absorb, range alternatives join the walk, boundary texts splice verbatim. `manual_is_multiple_of` adjudicated by-design-silence-plus-oracle-lane (E0689 class is check-INVISIBLE).

## 2026-09-17 — the doc_markdown bare-URL arm lands (the staged follow-up lead)

- Real clippy's URL branch is `Url::parse` + `!cannot_be_a_base` — any RFC scheme angle-wraps; `is_absolute_url` re-derived bounded (no new dep); autolink guard on the RAW token. Follow-up: the link-target guard moved to the raw token (`](` stripping made the trimmed word miss it) — `broken_intra_doc_links` is a rustdoc not rustc diagnostic.

## 2026-09-16 — Issue 120, the parallel session's side: the yield + the rust_perf debt closes (the allow lifted, 157 edits healed by the landed rule)

- The rust_perf module-level `#![allow(clippy::doc_markdown)]` LIFTED — 157 edits healed by the landed rule itself; 3 feature-gated residues fixed by hand. Score-bench rows #123/#124 independent derivations agreed.

## 2026-09-16 — Issue 120: `doc_markdown` lands — the every-landing manual backtick pass is the healer's now

- Clippy's word predicates ported VERBATIM; located as REAL `#[doc]` attributes (string literals unreachable by construction — the load-bearing FIXABLE property). Freeze tables regenerated + re-verified; router floor re-pinned 78→72 (−6 from the rule's PRESENCE, proven by a reword control); the rust_perf wholesale allow deliberately NOT lifted that day (paid 09-17).

## 2026-09-16 — Issue 114 resolved: the refusal-code client half (landed in riir-kat `a117163`)

- `kat_refusal.rs` (13-code table + `RetryVerdict`) + the code-aware burn stack; absent/unknown code = the prose-era default, never a guessed retry; monotone watermark makes replay-retention always safe.

## 2026-09-16 — Issue 116: the corpus-truth delta surface — `[no corpus rule]` carries TWO opposite messages

- `src/corpus_delta.rs` + `--corpus-delta`: "not leased YET" (the fix lane writes it today) vs "unsupported" (mining intake); the partition is asserted; FIXABLE/VERIFY tiers stay split (a VERIFY row only applies under `--fix --verify`). Freshness bucket is 0 BY CONSTRUCTION on a lease-less build — the exercise arm drives the REAL producer.

## 2026-09-16 — Issue 119: the corpus coverage-debt sweep — and the blind spot its first run found

- `cargo heal --coverage-report` (`300411ed`): population DERIVED from the installed driver; first measurement 822 derived / 54 covered / 768 UNCOVERED / 449 default-firing. The finding: 0 of the 449 carried ANY frequency evidence (the local histogram was captured pedantic-enabled — structurally blind to default-level debt). Fix-set-only = 0 (corpus-lockstep holds from the converse side); 0 cross-domain collisions.

## 2026-09-16 — Issue 119, the parallel session's side: the yield + the process-boundary delta (e2e arms + the never-bare row)

- Parallel implementation measured the identical first numbers (822/54/768/449 — a two-way cross-check); yielded; deltas re-landed: `tests/coverage_report_cli.rs` (exit contracts at the process boundary) + the `standalone_modes_are_never_bare` row.

## 2026-09-16 — Issue 118: the bench-target guard extended to the compile-repair surfaces (`--fix-compile` writes + the oracle lane)

- A compile repair can hoist a loop or delete the timed "dead" result — P30's "compile-shaped is safe" superseded. `bench_guard` promoted to crate top level (ungated); driver writes filtered before the pick; `user_path_forces` explicit override; P21 oracle lane rides the same door; `tests/fix_compile_bench_guard.rs` (real cargo).

## 2026-09-16 — Issue 117 completed (the delta half): the six remaining stubs + the three silent tails — and the dual-implementation yield behind it

- The staleness guard landed TWICE in one window; this session's duplicate YIELDED, additive delta re-landed: six stubs (`stake`/`unstake`/`vote`/`pay`/`faucet`/`wetrun`) route through `build_stamp::refuse`; the three auto-push tails print loud SKIPPED lines naming the rebuild.

## 2026-09-16 — Issue 115 resolved: the auto-oracle feature knob (`RIIR_HEAL_AUTO_ORACLE_FEATURES`) — gated diagnostics leave the `[heuristic]` blind spot

- `features_from` → `--features <csv>` injection, loud knob-active line, both failure arms name the knob; `RIIR_HEAL_AUTO_ORACLE=0` still wins; manual `--clippy-json` wins over both. E2E proven on a gated scratch crate (with the knob: `[oracle] needless_return` anchored).

## 2026-09-16 — Issue 113: the auto-oracle — `cargo heal` now runs the clippy oracle for you (the "agave 6951 rows → +1 credit" session)

- With no manual feed and a rust lane, a scan over a cargo project spawns `cargo clippy --workspace --no-deps --message-format=json --quiet` at DEFAULT features in the detected root; loud heuristic-only fallback; kill-switch `RIIR_HEAL_AUTO_ORACLE=0`. A gain path behind a discoverability wall is not a gain path.

## 2026-09-16 — Issues 110 + 111: two silent-wrong-verdict defects, both found by running this crate somewhere it had never been run

110: all three scratch-crate oracles BLAKE3-key the directory but wrote a FIXED package name — under a shared `CARGO_TARGET_DIR` two crates are one cargo unit and the second gets the first's cached answer (the more carefully somebody isolates, the more reliably the oracles lied) …

## 2026-09-16 — P30: the bench-target guard (rust_perf edits declined-and-flagged on measurement targets)

- `is_bench_target(path, src)` (path axis + harness-signature axis; `black_box` NOT a marker); `--fix` declines perf edits on bench targets with loud lines; `--suggest` stamps `⚠ bench-guard`. Residue counted: 0 measurement-altering heal edits landed in consumer bench files. Collision note: the wiring rode a sibling's commit; the module + docs commit completed the landing.

## 2026-09-16 — P33: use-row lexical scoping (platform-audit; the riir-chain `BrowserNodeState` intake)

- A module-PRIVATE use row reads only same-file mentions at its scope or descendants; pub re-exports keep name-global evidence. Measured: riir-chain CLEAN; the two `use anyhow::Context` rows mechanized away; full workspace re-sweep zero new rows.

## 2026-09-16 — P32: the `[C2·test-rig]` downgrade (config-audit; the riir-ai `Outcome`/`ProbeConfig` intake)

- A [C2] row renders `[C2·test-rig]` when the decl is tests-tree AND every read is under tests-path files (path axis kept separate from cfg(test) confinement; name-in-both-trees not eligible). riir-ai converts exactly the two predicted rows.

## 2026-09-16 — P29: the `[A·documented-write-only]` class-A extension (closes the P28 handoff)

- Class-A rows take the same marker arm class B uses (`is_deliberate_write_only_marker`); gpu-uniform keeps precedence. riir-ai `HlaKarcConfig.delay_k` renders weak; riir-engine field docs carry the marker wording (their repo).

## 2026-09-16 — P28: the `[gpu-uniform]` downgrade (config-audit × shader uniform blocks)

- `collect_gpu_uniform_structs` closes `#[uniform(…)]` blocks transitively; rows render `[gpu-uniform]` (downgrade-not-suppress — WGSL consumes the bytes, the detector cannot see it). seal-game-editor 2 [A] → 0 [A] + 2 gpu-uniform; canaries hold.

## 2026-09-16 — Issue 102 T5: the corpus authoring meta-rule guide (Library Drift's meta-skill, formalized)

- `.docs/02_corpus/authoring_discipline.md`: route-before-propose, vocabulary-translation dedup, probe-first, decision-content, bounded fix-space, honest tiering, fixpoint safety, GOAT + count-pin walks, provenance/`.raw` discipline, distinct-from rows, meta-recursion. Nothing new is policy — every rule cites the practice it formalizes.

## Issue 107 CLOSED 2026-09-15 — NEGATIVE on evidence; T3–T6 closed UNBUILT (record; file removed per the noise-reduction rule)

- T1: horizon 1 < 2 on every corpus; where a readout AUC is 1.000 the history floor MATCHES it (sufficient-statistic verdict). T2: the span-embed column did NOT revive the lane on synthetic corpora either. No corpus on which outcomes are NOT strategy-determined → T3–T6 closed UNBUILT. Reopen: organic fixseq rings with ≥2-revert runs + the embed column; store densification; any corpus where the resolved rate moves between orderings.

## 2026-09-15 — Intake class P23: the `useless_format` escape guard on the oracle lane (clippy's own suggestion, refused where it is wrong)

- The oracle lane drops any `useless_format` diagnostic whose covered call text contains ANY brace (a brace in a compiling no-args `format!` literal is an escape; clippy's `.to_string()` suggestion reuses the ESCAPED literal and would freeze the doubled form).

## Issue 107 T2 — the span-shape embedding column lands (write-time capture) + the embed arm (`src/span_embed.rs`)

- Write-time capture at the one `push_attempt` chokepoint (span bytes DRIFT as fixes land); hashed unigram+bigram bag, signed feature-hashing, L2-normalized, ×127 i8. Measured: auc(embed) = 1.000 WITH floor 1.000 at k=1 — BELOW-FLOOR, the same sufficient-statistic verdict.

## Issue 107 T1 — the fixseq horizon report + the lane-viability verdict (`src/horizon_report.rs`)

- Strict-lead features at i-k, LORO-held-out ridge readout, base + history-aggregate floors, three refusal classes. Measured: horizon = 1 on every corpus; the unblock was riir-kat `4bc8672` (ungated transport-tier imports E0432 at default features).

## Plan 127 — P015 Phase 2: the `cargo heal replay` client landed (`4e1983ee`, byte movers riir-kat `a80ec7a`)

- Lease → local clippy-oracle replay → signed verdict; `blake3(rustc -vV)` fingerprint, step-0 canary hash check, the pinned single-item rate rule (determinism law making K=2 agreement possible); the measured rule change: REMOVED-set evidence (ambient lints cancel in the diff BY CONSTRUCTION). Kill-switch `RIIR_HEAL_REPLAY=0`; lane inert until Phase 3.

## Issue 106 — the infron free leg (groq → infron → CF ladder) landed (`859dc6de`)

- `GroqClient` → generic `OaiClient` over static `EndpointConf`; 3-leg `LlmRouter` with windowed disable; live probes: the free pool is account-gated (429 "requires Team balance > $4.99") — activation = top up; degradation is exactly right (first 429 disables infron for the window).

## Issue 105 — the journey-credit third axis (PC-ALM dual accumulator) landed (`e58ff9b9`)

- The strand-keep poison (a KEEP ending at a typeck-masked E0614 scored as full success): `JourneyCredit` over the fixseq ring + `journey_factor` discounts success evidence in tainted journeys; EXACTLY 1.0 with no journey data (pre-105 stores bit-identical). Gate `tests/journey_credit_gate` 3/3.

## The 09-14 evening queue-4 hygiene sweep — eleven more seal-game-editor issues closed on code evidence, the numbering-collision defect measured and filed as issue 151 (2026-09-14)

- 11 issue files removed on landed-at-HEAD evidence (seal-game-editor `2415427b`); issue 125 deliberately left OPEN (verified not fixed); issue 151 filed (TWELVE dual-allocated plan numbers measured).

## The 09-14 evening queue-1 unit — seal-game-editor issues 088/099/100 closed on verified HEAD evidence, the 088 sqlite_store half fixed (2026-09-14)

- 099/100 verified fixed at HEAD (carrier FilterDropdown + relation-driven options); 088's remaining half (two orphaned doc blocks) deleted. seal-game-editor `bbbeac56`.

## P21 post-mining intake — the oracle-suggestion-children consumption: the `--clippy-json` fix lane applies clippy's own MachineApplicable suggestions verbatim (the P15 Unknown-receiver residue closed) (2026-09-14)

- `FeedSuggestion` capture (MachineApplicable-only, single-child, primary file) + `oracle_edits` merge (gap text preserved; whole-file parse gate, all-or-nothing) + pass-0 wiring (proposer edits win on overlap). E2E: three Unknown receivers healed verbatim; stale-feed re-run drops loud, file byte-identical.

## P15 post-mining intake — `unnecessary_map_or` lands: the born-lint closure, the `receiver_kind` substrate extraction + the SAFE-LET extension, and the gen2/gen3 re-freeze (2026-09-14)

- Probe-first: the full firing envelope (Option+false→`is_some_and`, Option+true→`is_none_or`, Result+false→`is_ok_and`; Result+true SILENT — no `is_err_or` in std). `receiver_kind` substrate + SAFE-LET extension (exactly-once binding, never assigned, not a param) + the proposer (Unknown REFUSES). P-numbering note: the P-series is per-session and has dual-allocated before.

## The 09-14 production re-heal of seal-game-editor — the P13 "owning lane's move" discharged same-day (2026-09-14)

- Oracle-fed `--fix --write --verify` at `d310c030`: 25 verified edits / 9 files, findings 49 → 19 (the remainder is exactly the documented divergence classes). The fresh feed caught 3 `unused_must_use` in the same session's own new tests (cfg(test) code needs `--all-targets`).

## The 09-14 morning re-arm pass — sweep blockers narrowed to ONE (riir-ai's stale checkout), healer HEAD binary staged clean-of-sibling-WIP, seal-game-editor #147 closed as the queue-1 unit (2026-09-14)

- riir-viewbridge CLONED (16/20); every sweep re-arm gated on riir-ai advancing. Issue 104 RETRACTED (false alarm; lesson: `git ls-remote` lists ref TIPS not ancestry; `git merge-base --is-ancestor <sha> origin/<branch>` settles any "history loss" allegation). Junction farm at `C:\git2` (E: is exFAT — no junctions). seal-game-editor #147 closed at `d310c030`.

## The 2026-09-14 Protocol B sweep — 10 repos screened, seal-game-editor healed oracle-fed (the queue-3 unit; first large multi-crate `--clippy-json --fix --write --verify` run) (2026-09-14)

Dev-sweep network discipline: `RIIR_HEAL_KAT_METER=0 RIIR_HEAL_KAT_GATE=0 RIIR_HEAL_KAT_SERVICE_URL=http://127.0.0.1:9` neutralizes the lease context. Screen: 4 CLEAN, seal-game-editor healed (`e2112bce`, 61→49), 4 blocked with named re-arm conditions, riir-shader recorded-not-touched.

## P13 post-mining intake — the `chunks_exact_to_as_chunks` CHAIN-ROOT extension lands (the sweep's own product consumed same-day) (2026-09-14)

- Real clippy fires on iterator-CHAIN roots; the ROOT is rewritten, downstream adapters compose unchanged; the `_mut` twin inserts `.0.iter_mut()` — a MEASURED safer-than-clippy divergence. LIVE-FIRE: 37 proposed / 24 kept, all oracle-Certified; new divergence class: MACRO-ARGUMENT chains (syn's visitor does not descend into macro token streams).

## P12 post-mining intake — `needless_pass_by_ref_mut` lands: the 2026-09-09 nursery-gap row closes, probe-corrected twice (2026-09-14)

- Probe corrections: `pub` methods SILENT, `pub(crate)` FIRES, trait-impls SILENT; the "must emit both halves" premise wrong (the gate is errors-only; clippy's own suggestion is definition-site-ONLY — the cascade emits as a narrow provable shape). VERIFY tier; the corpus LEASE gap signal was honest for that config (the rule reaches leased users at the next sync).

## P14 post-mining intake — the cross-repo consumer-surface hint landed: the FP-class #3 open candidate closes (2026-09-14)

- `src/downstream_hint.rs`: after `--config-audit`/`--orphan-report`, names every sibling repo ON DISK whose manifests path-dep into the scanned repo (measured-graph discovery from manifests, NOT BOUNDARY.md tables — prose drifts, manifests don't); root + member manifests only, ~50 ms.

## P13 post-mining intake — the FP-class #3 refinement (a) landed: trait-impl reads are external, and the probe surfaced the payload-Result class (2026-09-14)

- katgpt-rs 772 trio reproduced exactly; fix 1 trait-impl externality (3→0); the probe then corrected the intake's own attribution — `SchurSolver` is INHERENT payload-`Result` (fix 2: `Result<Payload, E>` = flow-out, `Result<(), E>` = confining). riir-train 1→0.

## P12 post-mining intake — `useless_conversion` shape 2 (the `.into_iter()` call-arg drop): the probe re-keyed the lint, the extension rode the existing type-blind tier (2026-09-14, `115a9185`)

- The probe inverted the intake's key: the fn-arg shape fires as `useless_conversion` (`useless_into_iter` is the for-loop shape, pedantic); direct call argument `<expr>.into_iter()` replaced by its receiver. Windows red fixed portable in `bbea191e`.

## The physician-heal-thyself self-audit — config-audit + orphan-report over riir-clippy itself at FULL scope (2026-09-14)

- config-audit clean both scopes; orphan-report 14 stillborn, ALL adjudicated benign (0 actionable TPs). The scope lesson: the src-only invocation's 15th row was a SCAN-SCOPE FP — shipped as `src/scope_hint.rs` (`d4e87f54`).

## `cargo heal --version` exit-2 regression — found by the global-binary rebuild, closed by merging the two hand-mirrored flag lists (2026-09-13, `ad31a04a`)

- The 09-12 fix completed `is_bare_posture` but the SECOND hand-mirrored paths-guard list kept its missing row; both now consume ONE shared `standalone_mode()` predicate — the mirror-drift class structurally closed.

## The `--corpus-json` lane retired (2026-09-13, riir-dapps Plan 026)

- `gen_dashboard.py`'s `--corpus-json` + `write_corpus_growth_json` deleted (the front-page corpus card went chain-native); only the consumer-facing export lane is gone. Phase-2 follow-up = Issue 103.

## P11 post-mining intake — `format_collect` (the first two-span clippy rule) + the `needless_borrow` receiver-shape extension (2026-09-13)

- `format_collect` two-span capability resolved BY MEASUREMENT (anonymous `as _` cannot collide; io::Write ambiguity impossible). `needless_borrow` receiver extension `(&x).m()` → `x.m()` for the whitelist. RERANK_POOL 16→20 (the rescue guard's first designed fire).

## P10 post-mining intake — the 089-T3 corpus-gap pair landed, and the probe inverted the intake's own description (2026-09-13)

- `filter_map_bool_then` fires on `bool::then` INSIDE `filter_map` (the split is the FIX), `vec_init_then_push` help-only in real clippy; both VERIFY-tier, corpus 49→51; attributed pushes refuse (a `vec![]` literal cannot carry cfg'd elements).

## Issue 101 resolved — the E0502 read-relocate: F3 cleared, interacting corpus 100%, the first ordering IMPROVEMENT (2026-09-13, `32b19a1b`+ / Bench 091; hashes post-rebase on `5042af31`)

- Design 1 (relocate-after-last-use) MEASURED DEAD before landing; the shipped hoist-to-first-use template cleared F3: interacting corpus 100%, driver 66.7% (unfixed 12→0), G1 172/172. T4 A/B: every arm 46/46, learned arms at 10 reverts vs static's 24 — the gain is WASTED ATTEMPTS, not resolved rate; Issue 093's negative verdict STANDS.

## Issue 089 T4 hardening — the bench-tail frontier snapshot: generation boundaries record themselves (2026-09-13, `11c1799d`; the 7 lane runs + snapshot row #2 recorded the pre-rebase build sha `b17c2dd8` — identical diff, the rebase only moved the base)

- `finish_score_bench` (the ONE tail every generation lane runs) appends the snapshot — a boundary nobody reports at is one the correlation never samples. T8 accumulation rode along (8 rows carry `rule_outcomes`).

## Issue 093 resolved — per-run strategy posterior: PoC win real, wired policy NO GAIN anywhere, ordering closed NEGATIVE on evidence (2026-09-13, `0fec5257`+`8993ec51`+`27e8ffcf`+`b08ef85d` / Benches 084+085+088+089+090)

- PoC WON on the toy (41.7% vs 35.2/36.7); wired green with honest gain-NULL (no real surface with revert dynamics); on the interacting corpus every learned ordering REGRESSED (10/34) — the keep signal was the defect. Final: static min-delta stays shipped; opt-in, out of the release set; never ship a long-lived `NonergodicOrdering` handle. Reopen triggers: store densification, organic rings, any corpus where the resolved rate moves.

## Issue 100 resolved — the unmasking trade: E0614 playbook + lexicographic keep rule, the T4 regression REPAIRED (2026-09-13, `65b9ff11`+ / Bench 090; hashes post-rebase on `ae9da3d6`)

- The playbook alone was inert; the load-bearing half is the lexicographic keep `(masking, count)` with `MASKING_STAGE_CODES = ["E0614"]`; F4 = the first legal equal-count keep; regression repaired to 34/46 parity for every arm. Floor: N 246→258, G1 160/160.

## Issue 099 resolved — the interacting-error corpus: gate 34/34, first non-zero reverts, learned orderings REGRESS clean 22→10 (2026-09-13, `a384acc1`+ / Bench 089)

- 34 shared-binding E0502×E0499 pairs in 3 families; first non-zero revert rate on the stock driver; the per-attempt keep signal is NOT a run-outcome signal on interacting corpora (clone-out strand-keeps score well in counters but strand the run at a masked E0614).

## Issue 91 resolved — fix-loop trap detector: T2 measured NEGATIVE (structural), T3 recorded, detector NOT built (2026-09-12, `e44e885f` / Bench 087)

- 0 flips on all three keys over 230 attempts — the shipped `--fix-compile` loop carries a well-founded measure making a within-run state revisit IMPOSSIBLE; a detector with no phenomenon is not built. Reopen triggers in Bench 087 §6.

## Issue 097 — linking↔confusion PoC: NO-GO, the fusion idea retired as measured-refuted; RESOLVED (2026-09-12)

- Both config classes refute (k=8 degenerates to 99.9% linked; k=3 shows ANTI-enrichment); the Spearman trap named (ρ moved with tie structure, not the confused-pair set). Mechanism: rename ORBITS, 1 exemplar/rule — the manifold assumption fails on this corpus. Reopen: a real-repo mined span set.

## Orphan-report serde boundary — the riir-ai Issue 934 follow-on repair (2026-09-12, `d2e39d1f`)

- The boundary is the ATTR, not the derive pair: `skip_serializing_if` fields excluded (conditionally-emitted OUTPUT schema), plain `#[serde(default)]` fields stay candidates (the canonical `BehaviorConfig` Ser+De case); `#[serde(skip)]` stays a candidate.

## Plan 126 — orphan-report: intake class C1, the config knob orphaned by optimization (2026-09-12)

- `--orphan-report`: two-leg report (zero-external-read + git-history pickaxe evidence of vanished readers); serde-literal candidate extension forced by the canonical case (no `impl Default`); the intake's "reader that vanished" narrative was syntactically FALSE — the knob was BORN unwired, fired honestly as `[stillborn]`. En-route: canonicalize scan paths before `strip_prefix` on macOS.

## Plan 125 — config-audit class C2: the self-referential config struct detector (2026-09-12)

- Whole struct whose every Default-impl field has zero EXTERNAL reads; the landing's own bug: syn 2 impl methods are `ImplItem::Fn` — `visit_item_fn` never fires inside impls (need `visit_impl_item_fn`). First-contact yield: 1 verified TP / 0 FP (riir-games `PetAiConfig`).

## Issue 91 T1.5 — the per-attempt ring: `--fix-compile` attempt sequences to `.heal/fixseq/` (2026-09-12)

- `AttemptRecord { seq, code, span_hash, strategy_id, outcome, errors }` at ALL five loop outcome sites; `span_hash` = blake3(`file:line`) 16-hex; one `run-<ts>.jsonl` per write-mode run. The instrumentation prerequisite for T2 (oscillation) + Issue 093 (per-run posterior).

## Issue 089 T4+T8 — the evidence-emission halves: band-mass snapshots + per-rule bench rows (2026-09-12)

- T4: `--frontier-report` appends band-mass snapshots to the committed `.benchmarks/frontier_snapshots.jsonl` (state-change dedup; `generation` = the FRONTIER, lanes interleave). T8: every bench row carries per-rule `rule_outcomes` (single-target attribution only). Both stay open on ACCUMULATION, emission now live.

## Issue 089 T7 — the shaped 0.5 verdict: generation 8 + the Elo-tie / Beta-half-count feeds (2026-09-12)

- "Target cleared ∧ new diags" = first-class: priced 0.5 in the bench, TIE in Elo, HALF-COUNT (+0.5/+0.5) in Beta; gen-8 admission is SHAPE non-vacuity (n_shaped ≥ 1), deliberately not the M5 heal_rate comparison. Freeze fences: INNOCENCE + SHAPE gates.

## Issue 089 T6 — the stratified score-bench read + the D-formula decline (2026-09-12)

- Every clippy-lane bench run splits heal rate by fixture provenance (repo/authored/construction/other) via the `ood` mining join; v1's 96.2% aggregate is authored-dominated (authored 98.0% vs repo 50.0%). D declined on the record. En-route: the score_bench BIN lane had been E0631-broken (a lane nobody compiles).

## Issue 089 T5 — provenance weighting: the priced pair namespace + the enforced rating fence (2026-09-12)

- `PROVENANCE_ORGANIC` / `SYNTHETIC_PAIR_WEIGHT = 0.05` / unknown fail-closed 0.0; `rate_pairs` ENFORCES the non-circularity fence (only construction rows feed the BT fit — observed wins are never rating evidence); the outcome-feed split measures band-mass PER FEED.

## Issue 088 resolved — the kat service-plane spin-out: every task + fleet follow-up landed, file removed at close (2026-09-10 → 2026-09-12)

- The KAT service plane (25 modules) → the private sibling `gist-rs/riir-kat`; riir-clippy is a consumer via non-optional path dep, root re-exports at historical paths; three healer-coupled keepers stayed root. Full guard GREEN after 8 latent rot classes fixed; fleet follow-ups closed 09-11 (katgpt-rs repo count 17→18; dapps/dao protocol-tier adoption).

## Issue 090 resolved — latent-only test lane: 17 domain-panic tests gated on the calibration state (2026-09-11, 23e051bc)

- The three draft test modules gate on `all(test, latent_retrieval, clippy_lints, rust_perf)` — the state the fixture set + floors are calibrated on; the issue's sketched `any(single-domain)` gate would have been WRONG (the tests assert Clippy rows).

## Issue 085 — the `dist` domain: the promotion-evidence narrative (2026-09-09 → 2026-09-10)

- Phase 1 `79dcf9b` (6th retrieval domain, no-syn Cargo.toml scanner, suggest-only); Bench 082: score-bench FLAT by construction, consumer scans true-positive on game-sdk/deployer; B1 (autodiscovered bin) + B2 (root profile knobs) closed `759e209d`; PROMOTED DEFAULT-ON 2026-09-13 (owner pull; all four GOAT conditions PASS; `--no-default-features` is the tested flag-OFF path).

## The domain swap — the client half + the config surface, post-flip state (2026-09-10, riir-dapps Plans 023+024)

- The apex IS mainnet now; `devnet.ai.gist.rs` hosts the MOVED fleet ledger; the fallback pair (default→devnet ONLY) is load-bearing in the other direction; `cargo heal config set --url <alias|url>` / `config get` (`cf236f1a`).

## Issue 081 resolved — burn wire unit mismatch: end-to-end closed, rescale STOOD DOWN (2026-09-10)

- Client sent micro book value in the whole-KAT wire field (clamp-wiped the grant, poisoned the watermark); fix `e9eca71d` + the cross-repo unit golden; the rescale STOOD DOWN — the watermark row was already WHOLE-SCALE (the CAS guard refused without touching it). Lesson: a money unit has ONE name per side.

## Issue 087 resolved — kat_burn_stack e2e: stale harness units, not the account plane (2026-09-10)

- The harness bills 11 µKAT/run; `wire_kat_total(11) = 0 ≤ ack 0` is a legitimate Skipped — the harness was the stale side; fixture now bills exactly 1 whole KAT with a self-assert. Lesson generalizes Issue 081's: a contract change must re-pin its e2e the same day.

## Issue 080 resolved — the sticky-grace marker retired, server truth only (2026-09-10)

- `kat_contribution.rs` deleted (the forgeable `contributed_once.json` is gone entirely); gate predicate `ack.contributed == Some(true)` (server truth ONLY); tests re-pinned both directions; dated removal landed once the fleet was v3 (`143c5a8`).

## Issues 077 + 082 resolved — margin-weighted mining + the OOD axis (2026-09-09)

- Issue 077: the e^{−2M} margin law, OOD axis always-on; T3 verdict DEMOTE (retrieval-similarity margins do not select lint-bearing spans). Issue 082: R1 CONVERGENT VALIDITY DEMONSTRATED (gap −0.7608 on BOTH fresh families — the composition attribution held); `regime_probe` stays opt-in.

## Issues 083 + 084 resolved — the green-zero class and the telemetry-key consumer (2026-09-09)

- Issue 083: all 16 `#![cfg]`-gated test targets got their `[[test]]` rows, verified in BOTH directions (13 exact feature-set builds; SILENT-NOW 16 → 0). Issue 084: filed as 082, RENUMBERED 084 before removal (the OOD issue had consumed 082 — the standing fix is renumber + citation rewrite); `parse_supply` reads `*_micro` first, falls back to `*_kat`.

## Plan 117 — `cargo heal --refill`, the dev/test faucet refill (2026-09-09)

- Composes shipped wire surfaces (`HttpPayreqTransport` + `HttpWetrunTransport`); the client adds no host gate deliberately (prod never distributes a crank token); server truth wins on the gate input; exit 0 iff the final synced balance is positive; crank leg fail-soft.

## Both shell gates could have reported a PASS on an abort (fixed 2026-09-07)

- macOS `/bin/bash` 3.2.57 enters the EXIT trap with `$?` already 0 on an unbound-expansion/eval abort — a trap whose last command succeeds makes the abort exit 0; only a completion sentinel catches it. Both `scripts/ci_feature_guard.sh` + `scripts/ops_nightly.sh` repaired and verified against their real preambles. shellcheck does NOT find this.

## Batch-mining batch records + Role/domain/pipeline full record (was AGENTS.md lines 23–3043)

- Compacted out of AGENTS.md 2026-09-06; the Role/domain records + corpus-build batch narratives live below and in git history. NOTHING was deleted.

## Role

Multi-domain modelless code healer behind the `HealDomain` trait (Plan 001) — developer tooling, NOT game/training/chain. L0 rule distillation → L1 ternary SIMD drafter → L2 syn validator (+ opt-in domain oracles) → L3 Pareto search (opt-in) → L4 LoRA seam (inert unless installed; NOT promoted …)
Commits: `e7b1f89`.

## CF ops surface + the `.env` contract — full record (was AGENTS.md lines 3044–3177; Issues 044/045/051/062/065 resolutions + dashboard-auth state + deploy hazard)

## CF ops surface + the `.env` contract (Issues 044/045 — read before touching ops)

LLM ops plane: `riir-kat-housekeeping` worker, `riir-ops-dashboard` worker, nightly generator (`scripts/ops_nightly.sh`); credentials in `.env` (gitignored; pipe into `wrangler secret put` via stdin, never echo). Issue 051 T8 defer preserved: `fixer_regate` arms stay CF-pinned BY DESIGN (model comparisons).

## The `riir-clippy` auto-continue skill — full description (was AGENTS.md lines 3178–3212)

## The `riir-clippy` skill (auto-continue idle loop)

`.agents/skills/riir-clippy/SKILL.md` (canonical home THIS repo, global is a symlink) encodes the idle-time workflow — 13 queue items (fix issues → finish plans → clippy sweeps → resolved-issue hygiene → batch mining + dry-run → boundary check → substrate-first → benchmark sweep → online unblock → defer-marking → …)

## Production adoption — full record (was AGENTS.md lines 3213–3525; verdict evidence + resolved issues 037/038/050/059/060/061/066/067/068/069/070/071)

## Production adoption — `cargo heal` as the dev-time default (2026-08-24)

Verdict GO (live trial): 12/12 planted fixes landed, 2/2 divergence classes correctly refused, compile gate 2 checks / 0 reverts, idempotent. Global binary + post-mining intake (mining improvement candidates from the healer's OWN usage artifacts; seed inventory 8 ranked classes …)

## Guard-landing protocol — full record incl. the measured story (was AGENTS.md lines 3526–3583)

## Guard-landing protocol — a guard is not landed until the residue is counted

Measured, not hypothetical: the `227637e` guards were correct yet 32 pre-guard residue sites landed the next day. The screen is the claim: `cargo clippy --workspace --all-features --all-targets --keep-going` (not `check` — the guarded classes are accepted by check's typeck and rejected by clippy's; not default features …)

## Plan 001 multi-domain architecture — full record (was AGENTS.md lines 3639–3665)

Plan 001 generalized the clippy-only healer into the multi-domain `HealDomain` framework; Proposal 012's GPU kernel optimizer absorbed as `kernel_opt` (user correction overrode the "new repo" recommendation); cross-domain `--domain auto` fans out with the router verdict as a per-span ANNOTATION, never a gate (the 8-D centroids mis-routed fixtures).

## `kernel_opt`-specific layers — full record (was AGENTS.md lines 3666–3685)

1. `KernelExpertRouter` wraps katgpt-core `pick_domain<15, 8>` (8-dim activity vector, 15 per-category centroids, 15/15 reachable; `build_activity()` the consumer constructor; modelless argmax, zero alloc).
2. `KernelThermalRouter` — Plasma→Hot→Warm→Cold→Freeze (`ThermalTier` `#[repr(u8)]`); the scaffold returns Plasma (modelless-first); Warm/Cold/Freeze require 4090 CUDA + LoRA (deferred to riir-train).

## Feature Flag Discipline — full record (was AGENTS.md lines 3760–3809; per-feature batch narratives + corpus-growth history)

## Feature Flag Discipline

`default` = `["clippy_lints"]`. Per-feature records: rust_perf promotion (Bench 013/015/016/017/020 GOAT; 37/40 benchable rules Pareto-dominant incl. `regex-hoist-lazylock` 5443–5675× …)
Commits: `4bfe3b30`.

## Full text of remaining sections compacted in AGENTS.md (operative boilerplate — archived so nothing removed is unpreserved)

## Boundary contract — read `BOUNDARY.md` first

Full text preserved in AGENTS.md (canonical). Domain test: developer tool, zero game-domain coupling in the default build; enforcement via `../riir-ai/scripts/ci_boundary_contract.sh`; violations file the issue FIRST, add the drift row, then fix.

## Why this is a separate repo (not in riir-ai)

Full text in AGENTS.md. Borrows the drafter+pruner+corpus pattern from `riir-games-quest/quest_grammar` with zero game-domain coupling; default build's only public dep is katgpt-core; opt-in arms consume `../riir-infer` (ternary) + riir-rag (latent_retrieval).

## Documentation Shape

Full text in AGENTS.md: 11-folder numbered `.docs/` book; files inside numbered folders have no number prefix; `README.md` is the build surface.

## Numbering Discipline

Full text in AGENTS.md: monotonic never-reused counters per directory, one counter per directory ENFORCED (Issue 069 T3); katgpt-rs `scripts/numbering_drift_sweep.py` covers this repo. Research-note shape: born-complete records with a `**Status:**` line (RECORD / DISTILLED / NEGATIVE).

## 2026-09-11 — Plan 124: `--config-audit` (post-mining intake classes A+B consumed)

- Class A (default-is-a-named-const-nobody-field-reads) + Class B (assert-only-against-own-Default); probe-verified clippy fires ZERO on the inert shape; riir-chain 1 verified TP / 0 FP. Class C stays an intake row (needs git history).

## 2026-09-12 — Issue 089 T3: `--misses` generalized to every domain + the 09-08 bare-default regression fixed

- Per-domain frontier-ordered authoring queues; en-route: `--misses` had been exit-2 broken since 09-08 (bare default under every standalone mode); the first loader was an undercounting `collect()`; two "clean" clippy `--bins` lanes were INERT (required-features silently skip the target).

## 2026-09-12 — Issue 092 T1: the GW corpus-alignment PoC, measured and closed (defend-wrong NEGATIVE)

- GW precision@60 = base rate 0.017; name/keyword 0.083 = 5× base. Mechanism: surface-trigram distance is not fix-shape structure. Reopen: a semantic fix-shape embedding or a disjoint-vocabulary corpus pair.

## 2026-09-12 — Issue 093 T1: the per-run strategy-posterior PoC, measured WIN (defend-wrong)

- `NonergodicFilter` 41.7% vs 35.2/36.7 (McNemar p<1e-4), misspec-robust, store-independent. Harness lesson: revive-on-posterior-flip fired 356 times, 0 helped (frozen-context retry is the same bytes); revive requires the suppression CONTEXT to have moved.

## 2026-09-12 — Issue 094: trajectory covariability-class mining PoC, measured and closed (defend-wrong NEGATIVE)

- AUC 0.497 = chance; the failure is the STORE SCHEMA's (cumulative counters, last-verdict timestamp — no session axis). Reopen: per-event trajectory rows or balanced per-rule labels.

## 2026-09-12 — Issue 093 T2: the nonergodic driver wiring LANDED (gain-null on no-surface) + the orphan-report Windows blind-leg fix

- Driver `with_observer`/`with_revive` seams + the policy; both arms first-try-perfect on every real scaffold; the family-error test moves to the reopen trigger. Windows fix: canonicalize the git root before pathspec comparison (`\\?\` verbatim form).

## 2026-09-12 — Issue 098: the rule-corpus cross-OOO axis, measured PASS + wired (absorption candidates found)

- TF-IDF cosine (plain TF-cosine REJECTED — generic scaffold swamped rare op signatures); 3 absorption candidates found incl. clippy's own split of one lint detected unprompted (the calibration anchor); `score_history.jsonl` rows gain `cross_ooo`.

## 2026-09-14 — Plan 195 executed (seal-game-editor issue 150): the UNIQUE carrier index, its reconcile twin, and the clash report that opens the file

- Landed on seal-game-editor develop `e25f98a4`: UNIQUE index + `working_schema::reconcile_indexes` + the clash row; five plan amendments recorded (EDITOR_SCHEMA mirror + version bump; `drift_split`; duplicates counted before drop). editor-data 876/876 + 10/10.

## 2026-09-15 — Issue 108 T1+T2: the component-necessity flip-rate harness (CVRR at the dispatch seam)

`ComponentOverride` + `heal_fixpoint_cfg` (empty = byte-identical, PINNED) + `--necessity-bench`; smoke e2e's first real finding: `needless_return` solo_resolved=1 but Δdisabled=−17 — solo rate massively understates composed importance.

## 2026-09-15 — Intake class P22: `--platform-audit`, the platform-dead_code decl-vs-use cfg lattice

Items declared ungated whose every use sits inside a platform-gated context — invisible to any single-platform CI matrix. Validated against katgpt-pruners @ `ea4c2873^` (exactly NEON_U8; 1 TP / 0 FP over 5950 decls). First workspace sweep 14 rows → 3 TP / 11 FP …

## 2026-09-15 — Issue 109: the orphan-report FP specimens repaired (foreign-literal attribution + generic serde bridges)

- (a) per-file foreign-literal attribution with the workspace-sibling rescue (`scan_local_crate_roots`); (b) generic serde bridges harvested at turbofish call sites (stillborn → `bridge-cleared`, orphaned → `wire-dto` weak). Lesson: raw-string terminators `}"#` swallow following tests silently — when tests vanish from `--list`, suspect the terminators first.

## 2026-09-16 — the Issue-109 re-sight closed: scan-global struct field sets (the foreign-glob + local-glob shape)

- The bevy migration re-entered 4 rows through the documented undecidable shape; closed by the compile argument (`literal_is_field_mismatch` — a literal naming a field no local same-named decl has provably cannot be any of them); canaries byte-identical.

## 2026-09-16 — P31: the fn-A/fn-B inter-procedural wire-clearance (orphan-report; the riir-kat queue-writer intake)

- `serialized_concrete_param_types` (a fn serde-serializing its own concrete param contributes that type) + macro token-stream scanning (serde calls inside `writeln!`); riir-kat 16 stillborn → 4; remaining conservative gaps on record (turbofish `to_string::<T>`, method-shaped serializers, Deserialize-side classes).

## 2026-09-18 — Issue 124: the ops-plane ladder re-pointed at the riir-llm leaf

- The leaf owns WIRE MECHANICS (URLs, body builders, parsers, `OaiError` = alias of `riir_llm::LlmFailure`, timeouts, bearer hygiene); riir-clippy keeps POLICY (windowed-disable router, stats, pricing, cap). Behavior changes: free-leg 401/403 is STOP-class (was silently shifting the window onto the PAID leg); empty-content falls through; `groq_first` no longer names `dep:ureq`.

## 2026-09-22 — Issue 129: the cargo-heal scoop-bucket 0.1.4 manifest 404 (URL pointed at the v0.1.3 tag)

- One-line URL tag fix, verified end-to-end (sha256 byte-identical). Prevention: prefer the release.yml bucket-PR lane; when hand-editing, copy BOTH `url` and `hash` from the fresh release and curl once before pushing.

## 2026-09-26 — Issue 130: RRSI admission-generalization gate — closed report-only, both unblock paths measured shut

- T1: repo stratum δ = 0.000 (the oracle axis is deterministic per pinned toolchain — a zero-δ floor admits everything). T2 repeats: refuted BY CONSTRUCTION (no timeout/parallelism in the bench path). T2-OOD: the δ=0.247 window spread is corpus-state drift, NOT repeat noise (within-state repeats bit-identical). Live verdict wired into `--gate-calibration`.

## 2026-09-27 — Issue 136: the structural self-mutation lane landed (reef P1+P3+P5, Plan 175)

- `src/structural_admission.rs` (ungated pure: per-kind shape gates, kind-keyed review classes — 4 of 5 `AlwaysReviewed`, the reef P2 secret-shape scan, the `ChainState` CAS machine) + the gated content-addressed store (`.heal/structural/`; every touch re-verifies the hash; `CasRace` on moved state). NOTHING auto-mutates — activation is a separate owner call.

## 2026-10-06 — Issue 144: the dist README + CLI-help audit — T1–T5 landed; T6 dispositioned on the owner's re-arm posture

- T1 dist README rewrite + CHANGELOG (incl. three corrections beyond the audit's evidence table); T2 repo rename; T3 `--help`/`--help-all` split with internal plan/issue ids banned from the user page; T4 `--domain` choices derived from the compiled feature set; T5 EULA/LICENSE + the private-crate leak fixed at the `about.toml` level. T6 dispositioned NOT done: the testnet alias stays pinned (the recorded re-arm posture wins over a cosmetic help-text win).

## 2026-10-10 — Issue 159 CLOSED: the net-pin conflict posture refuses every run at startup — fixed `2e54d205`, verified

Found during the kat_ua (riir-kat 003 T15) landing's guard run; pre-existing and reproducible since `492381b2`. Root cause: under `--all-features` both net pins are on, so the `command_service_url()` conflict refusal fired at STARTUP (via the Issue-153 update check's URL resolution) …

## 2026-10-11 — Issue 089 CLOSED: the frontier-band curriculum delivered report-first; the admission gate stays declined on the record

Landed 2026-09-11→09-12 (`11c1799d` the self-winding tail): T1 `--frontier-report` (per-rule `r/f/n/p̂/LCB₅` + band-mass; snapshots append to `.benchmarks/frontier_snapshots.jsonl` with the `store` discriminator), T2 saturation candidates (report-only), T3 `--misses` generalized to all domains, T5 provenance weighting …

## 2026-10-11 — Issue 139 CLOSED: noise-weighted evidence consumed report-only (T3); T1/T2/T4 terminal-deferred behind the qualified tripwire

T3 LANDED 2026-10-01 behind `noise_weighted_evidence` (report-only `--frontier-report` censoring decomposition; live store 28 rules / 0 unseen — armed but vacuous on this data). T1/T2/T4 DEFERRED 2026-10-01 as measured VACUOUS (10 of 657 n≥2 groups mixed = 1.5%; prior-epoch σ̂ is 0 for 99.75% of groups) …
Commits: `9537e7f3`, `9ad5e21ce`.

## 2026-10-11 — Issue 140 CLOSED: insight-conditioned retry landed with its measured verdict; T4/T5 have an empty input population

T1 (distiller `src/insight.rs`), T2 (admission predicate), T3 (`InsightConditioning` at the `with_ordering`+`with_observer` seams; eval lane `--only insight`) LANDED 2026-10-01 …
Commits: `977ff1e4651c`, `cd756252`, `9cfd0c33`, `b72bb62f`.

## 2026-10-11 — Issue 157 CLOSED: span-conditional contrastive negatives DECLINED-for-now on volume; the tripwire lives in bench 111

Owner triage RESOLVED 2026-10-10 (verdict session `2b8a1316`, rounds 1–3 AGREE): the retrieval-negative consumer is NOT wanted today — ~12 negative pairs over ONE span (the v4 `rejected` label landed 2026-10-08), kill criterion 1 (volume-vacuous); NOT a mechanism negative …
Commits: `b41e1403`.

## 2026-10-11 — Issue 158 CLOSED: the Matryoshka-k re-test tripwire absorbed into its record (bench 109 + AGENTS.md)

The file was a tripwire by its own text; none of T-A/T-B/T-C has fired (corpus still at the Bench-109 generation; no k-subvector consumer shipped). The 133 negative's boundary stands: the shipped FIXED objective wins shallow-k everywhere (rig 134 vs ≤ 130); deep-k corpus-limited at 1.16 labels/span …
Commits: `007435e5`.

## 2026-10-14 — Issue 163 CLOSED: the conflict posture keeps its loopback lane (the --all-features hermetic tier green again)

Verdict AGREE (reviewer session `6bb9b19a`, 2026-10-14): option 1 — reorder the seam in riir-kat — landed `riir-kat 9a36759`: `ensure_net_allowed` checks the loopback/`allow-local` branch BEFORE the empty-overlap refusal (EMPTY check stays ahead of the allowed-set match …)
Commits: `2682d7a0`, `492381b2`.

## 2026-10-24 — Issue 170 CLOSED: the retrieval gate was RED at every latent_retrieval posture (router 60.3% < 72.3%) and no standing lane could see it

Found by Issue 169 T0's paired-baseline cross-check (Bench 120), verified pre-existing at plain `latent_retrieval`. **T1 attributed, NOT a defect:** growth competition at the SHARED 128-slot GlobalFreq vocab …
Commits: `4c02b8f8`, `80fb8920`, `013d5c04`, `746421f5`.

## 2026-10-26 — Issue 171 CLOSED: T0b package-scoped re-checks promoted (the RA pre-filter deferred on measurement, verdict AGREE)

Landed `1f77c585`: `fix_verify` implies `scoped_recheck` (GOAT PASS) — every verify round re-checks only packages the round's writes/reverts (own or dependency) actually touched; clean untouched packages skip the subprocess spawn + JSON parse. Kill-switch `RIIR_REFINE_SCOPED_RECHECK=0` …

## 2026-10-26 — Issue 172 CLOSED: all 16 pre-existing all-features failures accounted (14 fixed, one filed as 173, one closed via riir-dapps 138)

T0 verdict: `ci_feature_guard.sh` Layer 5c runs every test target at `--all-features`, so the 16 reds were a live red CI lane, not a dormant configuration — all 16 re-verified pre-existing at `8d731fd5^` (comm of FAILED sets empty both ways).
Commits: `fd4be603`, `e067e1b`, `6f52f25f`, `4085aa24`.

## 2026-10-26 — Issue 173 CLOSED: `latent_suggest_goat` G2 latency regression fixed at the source (1587 → ~680 µs/chunk), bar made posture-aware

T1 profile (macOS `sample`, line-tables release): the per-query hot path was riir-neuron-db's `Bm25Index::search` score accumulator, not riir-refine — std SipHash on a `HashMap<&[u8], f32>` outweighed the search body.
Commits: `ef8599a`, `12b1af4`.

## 2026-10-26 — Issue 174 CLOSED: CLI tests no longer adopt the box's real `~/.ssh/id_ed25519`; Issue 172's out-of-lane rows green

riir-auth `account_key` adopts `$HOME/.ssh/id_ed25519` IN PLACE as the account key when one exists, so every CLI test that ran `login` with only the account dirs isolated signed with the developer's real SSH identity on a box that had one and a minted key elsewhere — two code paths chosen by the box.
Commits: `7f076057`, `b543c256`, `3b89cba1`.

## 2026-10-27 — Issue 179 CLOSED: kat_account adopts the provider-identity substrate — all six items landed the same day

Landed `95bc8c18` (amo-auth-followup): `kat account` + `--info` render the
Commits: `eae0f05`, `1258eef`, `c130df9c`, `f22a5ec5`.

## 2026-10-27 — INCIDENT: a bare `git commit` in the shared worktree swept a sibling's staged work under the wrong message (`7f965bc3`)

Folded from `.issues/180_incident_commit_sweep.md` (removed per the noise

## 2026-10-27 — INCIDENT: a Protocol-H worktree sweep removed a staging worktree holding an unlanded commit; the misfired follow-up command then wiped a sibling's WIP in the shared tree

Three-fault chain, all measured in the reflogs (session: zed-amo fmt-census
Commits: `a2389e6`, `3a5acfc`, `d58afb51`, `c9923d2f`.

## 2026-10-28 — Issue 187 CLOSED: the migration proposal issue is FILER-authored; no GitHub write credential exists anywhere in the worker

Folded from `.issues/187_migration_proposal_filer_authored_no_worker_write_token.md`
Commits: `c12d9cb`, `0b143d5`, `7dec777c`, `9d35593`.

## 2026-10-28 — Issues 183 + 184 CLOSED: the migrate release law landed (owner-👍 authorization + rule-C weighted voting + the autonomous cron closer), rehearsed live on devnet; the live half superseded by the filer-authored flow

Folded from `.issues/183_migration_release_requires_owner_endorsement.md` +
Commits: `a7a1c13`, `86a1622`, `e968b3e`, `921d2739`.

## 2026-11-03 — Issue 195 LANDED + REMOVED: the escalation-chain placebo arm (the JevAct Table-3 falsification shape) — plus the FROZEN E0432 under-gate fix found while landing it

The three-arm measurement shape for the escalation chain: baseline (no
escalation) / real (seat rerank) / placebo (SAME trigger rate, BLAKE3-seeded
random pick over the pool the seat saw, no judgment). "Seat gain over
placebo" (paired LB95 > 0) is the only gain figure attributable to the seat's
judgment; "gain over baseline" is the harness's. Landing: `--arm PLACEBO`
(seatless, no verifier, frozen-gold scoring, manifest-alone posture — the
arm that LEGITIMATELY occupies the posture where FROZEN previously failed to
compile) + `--arm FROZEN --placebo` (the three arms in ONE pass, trigger-
matched by construction, the ATTRIBUTION verdict with the issue's
refutation language; the pre-registered T4.2 A1-vs-A0 read byte-unchanged
when `--placebo` is absent). The pick formula is auditable:
`blake3(case_id)` first 8 bytes LE, modulo the PRESENTATION_CAP-truncated
pool length. Policy home: `src/escalation_placebo.rs` (+ PRESENTATION_CAP
moved to `escalation_manifest` with a dispatch re-export). The standing
shape + claim-language law: `.docs/08_benchmarks/placebo_arm.md`.

**The defect found en route** (`70d5e072`, the cfg_row_implication class):
`--arm FROZEN` was gated `escalation_manifest` ALONE but imports
`escalation_dispatch` (manifest + a transport) — E0432/E0433 at
`--features escalation_manifest,cf_workers_ai --bin fixer_regate` (verified
before the fix). Now gated `all(manifest, any(seat_rerank, hosted_expert))`
with a loud runtime refusal naming the missing features (exit 2, verified).
Also en route: the INVALID-ledger flush before `exit(1)` in both frozen
arms — `std::process::exit` skips destructors and the BufWriter ledger was
silently lost on an invalidated read (the Bench-119 INVALID_WIRING ledger
precedent says an adjudicated invalid read keeps evidence; verified by
fixture: the wall fires at 20% drops and the ledger now survives).

Fixture e2e evidence (synthetic frozen set, /tmp, instrument proof not a
measurement): determinism bit-identical across runs; every pick verified
against blake3 by hand (c1=1 c2=1 c3=3 c4=1 c5=0); SKIP exit 3 with no row;
lib tests 5/5 at escalation_manifest; clippy 0-warnings at all touched
postures + default --all-targets.

**Owner-gated residue:** adopting the three-arm shape as the STANDING
convention for every future gate-shaped feature (the doc's Generalization
section records the template; the mandate is the owner's call).

Folded from `.issues/195_escalation_chain_placebo_arm.md` (removed per the
noise-reduction rule; claim lease zed-riir-refine-195 honored — claimed and
pushed `5c2851d4` before authoring, closed by this removal).
Commits: `5c2851d4` (claim), `70d5e072` (the arm + the fix), `41613d2c`
(this record).

## 2026-11-04 — Issue 206 corpus half: live-telemetry diagnose + the arm-A writer + the first templated note + the referee's UNDECIDED (support 4 < 30)

The re-open trigger fired (Batch 329 landed `f53555e04`; the sibling's
Batch-330 WIP `entries_57.rs` untouched — disjoint write sets). Harvested
the REAL telemetry both sources: blueprint strata (11 repos, 37 campaigns,
23 rules — 0 candidates) + the four trajectory stores (riir-refine 376 /
katgpt-rs 397 / riir-infer 8 / seal-game-editor 315 rows; riir-chain's
store decodes EMPTY — 0 rows), aggregated per rule with the W/C component
split. The pinned predicate (attempts ≥ 8 ∧ share ≥ 0.5) flags exactly
ONE rule all-time: `hashmap-with-capacity` — 9/18, ALL W-class, last seen
15 d ago (~5 batches back; vintage disclosed in the note). Recent-14 d:
ZERO candidates. Watch (below the bar, not flagged): `redundant_closure`
0.48, consec 3, 6 d fresh. **Arm B population: zero C-class candidates —
there is nothing to demote** (deferred with that concrete reason, not
contention).

Arm A landed: `FailureClass` (W/C tags shared with
`component::ComponentClass`), `degenerate_context_note` (probe-derivable
values only — class, stage word, counts, share, epoch label; NO fix
content per the scope restriction), `CorpusBinding::compose_context_note`
(the composition gate: stale-epoch refusal + rule-present + diagnosis-
identity → the rendered note; the note never carries the binding digest —
the digest binds the pre-edit epoch, a landed note would falsify it by
construction). 16/16 module tests, clippy-clean, default lib 2835/0. The
corpus edit: `entries_03.rs` hashmap-with-capacity `context_notes: None →
Some(note)` (epoch `ac280021`), counts unchanged; the three annotation
count pins re-pinned 243→244 / 53→52 ×2 with the issue-206 lead clause.

The referee leg MEASURED, verdict **UNDECIDED by the pinned support
floor** (gen-5 admits exactly 4 fixtures containing the rule — 4 < 30,
structural for single-rule arms; no generation can exceed it). The A/B
itself (baseline ac280021 vs note-spliced corpus, one-off dev probe):
atom tables BYTE-IDENTICAL (14 atoms, 0 diffs — atoms are a pure function
of `buggy_code`), healed output of `gen5-k1-hashmap-with-capacity`
BYTE-IDENTICAL — gen-5 heal dispatch is compile-time, corpus entry text
is not an input: the bench axis is structurally identity. The note's real
effect surface (consumer-facing advisory + the latent index's corpus
text) is not bench-measurable. Kill criteria NOT fired (UNDECIDED ≠
miss); the NOTE rides as an annotated rule (normal corpus practice), the
WRITER LANE stays inert until a recent-window candidate exists — no
zombie machinery: 3 small functions over the landed module. Follow-up
recorded in the issue: the n≥30 bar may need re-pricing for single-rule
edits (a per-rule stratum cannot reach 30 on a finite fixture set) — a
referee-design decision, never a silent bend.

Commits: this one (writer + corpus edit + pins + issue + record).
Full diagnose table + honest-scope notes: `.issues/206` (kept open — the
harvest→bind→note CLI verb + arm B re-open on their recorded triggers).

## 2026-11-04 — Issue 203 CLOSED LANDED: the corpus lease migrates to the points read (the shipped `update`/`corpus` dead-end ends)

Filed from the first live-mainnet e2e of the shipped v0.4.0: `corpus`
printed an unsatisfiable rebuild hint and `update` refused naming
`kat_transport` — the plane the pivot deliberately compiles out. Root
cause: the corpus-lease refresh wire stayed on the KAT transport while
the release set moved to the points plane.

**riir-kat `2bc6020`** (the substrate half): the lease READ half moved to
the client tier — `LeaseReadTransport` (get_lease/get_segment) +
`HttpResponse` + `LeaseReadStatus`/`LeaseReadOutcome` +
`run_lease_read_phase` (the contract chain verbatim: decode+verify →
compat → diff fetch → stage → atomic install; no charge, no stamps); the
new `lease_http` feature carries the GET-only `HttpLeaseTransport` half +
the failover machinery (NO `/sync`, NO `/claim`, NO signing, NO QR — the
money surface stays kat_transport-only by construction); `LeaseTransport`
keeps the write half + money stamps as the kat_transport supertrait;
`run_update_phase` delegates the read chain and keeps the out-of-KAT
pause + the charge judge + the charge book; `kat_ua::apply` rides
`any(kat_transport, lease_http)` with the `X-Kat-Totp` factor arm
extracted kat_transport-keyed (the header literal must never ride the
charitable binary — measured bar-3 hard-zero hit, fixed by extraction).
4 new read-phase tests; 538-544 passed at the kat_transport postures,
444 at default+lease_http; clippy -D clean at five postures. COMPLETES
`fad9e1d`'s net-mainnet lib-test compile fix: the devnet const's test
sites now ride the const's own posture gate (the devnet rows are COMPILED
OUT of the pair table + the refill classifier under the pin — gating the
arms, not re-literalizing).

**riir-refine (this commit)** (the consumer half): `points` gains
`riir-kat/lease_http`; `kat_lease_client` re-exported under
`all(points, not(kat_service))`; `points_lease.rs` (`run_points_update_at`
— the offline law, the points dial law, the net pin, the gain renderer;
`resolved_lease_url` shared with the run-context partition);
`run_points_corpus`'s dead-end fork → the working offer; the Update
dispatch points arm (kat_transport keeps the kat lane + its charge book);
the run-context gates `any(kat_service, points)` with generation wording
(scan-surface clean of the bar-1 classes) and the points partition
resolved through the SAME points dial law (an install and the runs that
read it name one partition); `maybe_auto_corpus`'s points arm (the
hands-free refresh works in shipped builds, NotModified silent).
charitable_gate: bar-2 names `lease_http` (GET-only read transport,
explicitly not the money plane); R2 records the standing rule that
`RIIR_REFINE_KAT_SERVICE_URL` can NEVER ride (its name contains the
KAT_SERVICE hard-zero substring — measured: pulling the KAT dial seam in
tripped bar 3, so the points posture resolves through the points plane's
own env).

**Gates + e2e**: CHARITABLE_GATE PASS (hard-zero zero · mainnet 2/12 ·
R2 4/4 · bar-1 zero · --mine typed refusal · leak-scan PASS); refine lib
2852 (points) / 2836 (default) / 2876 (kat_transport) · g10 battery 13/13;
clippy -D clean at default / points / kat_transport / the release set.
LIVE (release-set binary, net mainnet): `update` fetched + installed
epoch 2971 (74 rules) — `never billed`; the second call 304'd; offline
typed refusal; the next scan printed `corpus: running the refreshed rule
set (generation 2971, domains: clippy)` — the installed epoch feeds the
run overlay end to end. Residue (owner acts, recorded in the issue's
sister findings): the tuna.riir.ai custom-domain DNS row; the join
Device-Flow dashboard act.

## 2026-11-04 — Issue 204 CLOSED LANDED: the contribution feed — join arms the auto-send (the owner's UX law), the points plane grows /points/contribute

The fix run-end's "unfixed" evidence now flows to the network when the
user is joined: consent `Answer::Joined` (join writes it, leave
unwrites — the state machine already carried the law) auto-sends
per-rule written/declined counts over the new `POST /points/contribute`
(the joined-identity lane; anonymous stays the `--stats` plane).
Three halves: riir-ledger `a1ee0fb` (the wire DTOs — counts only, the
privacy floor by shape, goldens + the privacy census; the machine label
deliberately dropped — T16.1), riir-dapps `21b461e` (the plane + route:
the run tombstone FIRST-WRITE-WINS, per-rule counters on the points:
family inside the settle anchor's chunked_root fold, the join signature
family + the canonical-order law, the ACTIVE-binding gate with the
typed not_joined refusal; 3 contract tests, 16/16), riir-refine (the
client fold+sign method, the run-end tail — one 📡 line, ~5 s, never an
exit-code change; the join CTA suppressed while the feed is armed;
`tests/points_contribute_feed_law.rs` both directions + the shape
floor). CHARITABLE_GATE PASS. Live-probed: Joined → the tail dials and
surfaces the deployed worker's typed not_found until the route deploys
(the deploy + the bench-018 ledger row are the owner lane); Never/Unset
→ zero data sent. Found + fixed while landing: the points_client env
race (the service-url env test vs parallel client-construction tests —
a static lock serializes; the contribute tests exposed it). The v1
payload carries the outcome strata the run already computes; the
shape-hash miss axis is the v2 wire field when a client-side aggregator
exists — deliberately not invented sloppy.

## 2026-11-05 — Issue 206 CLOSED per delegated-owner verdict: the c′ referee filed as issue 209 (probe-authored corpus repair, full arc)

The corpus half LANDED in three steps: the module (`src/corpus_binding.rs`,
2026-11-04 — diagnose → BLAKE3 epoch-keyed binding record → stale-refusing
compose gate), arm A (the templated `context_notes` writer + the first live
edit on `hashmap-with-capacity` @ epoch ac280021, pins-manifest same-commit),
and the harvest→bind→note CLI verb (`src/bin/corpus_diagnose.rs`,
`289cab470` — 15 stores / 1,539 rows folded, the one candidate mechanically
reproduced, `--bind` proof through the live-epoch compose gate, REPORT-ONLY,
blind-walk exit 2). Arm B deferred with a concrete reason (zero C-class
candidates at the pinned predicate — reopen trigger moved to issue 209 T4).
The referee leg measured UNDECIDED-BY-STRUCTURE: gen-5 admits exactly 4
fixtures carrying the rule's atom (4 < the pre-registered n ≥ 30 support
floor — a per-rule stratum cannot reach 30 on a finite fixture set), and the
bench axis is compile-time dispatch (atom tables + healed output
byte-identical baseline vs note-spliced), so the fixture A/B cannot decide
single-rule note edits in either direction. Delegated-owner verdict (Claude,
round 1, REVISE → **c′**): (a) keep-n≥30 leaves the referee permanently
unable to judge its own input; (b) the zero-flip floor cannot fail when the
edit provably misses the bench axis; (c′) replaces the EVIDENCE SOURCE —
structural-identity certificate (BLAKE3 atom+healed digests in the binding
record) where the axis is provably untouched + a telemetry before/after
referee on the rule's own live failure rate (one-sided Beta-LCB95 on the
failure-rate drop, split at the edit epoch, n ≥ 30 post-edit attempts,
co-epoch difference-in-differences control vs un-noted W-class rules,
8-batch starvation kill window, PENDING never folded into a pass). The
11-04 note re-enrolls as case #1 (post-edit window starts at ac280021).
Batch/multi-rule arms unchanged (fixture A/B + paired LCB95 > 0 + n ≥ 30).
The pre-registration is `.issues/209_cprime_referee_telemetry_before_after.md`
(highwater bumped 208 → 209) — pinned BEFORE any data was read, so no silent
bar-bend. The stale-epoch observation (the 11-04 note's epoch label vs the
live corpus epoch) is EXPECTED: the compose gate's stale-epoch refusal is
the policing working as designed. The note itself stays landed as normal
advisory corpus annotation (annotations carry no GOAT gate). Issue file
removed per the noise-reduction rule — full arc in git history
(`git log --follow -- .issues/206_probe_authored_corpus_repair.md`).

## 2026-11-06 — the scan perf lane LANDED: parallel precompute (3.4×) + byte-weighted ranges + the ⏳/⏱ run-cost lines (owner UX ask, no issue file)

The serial per-file scan was the run's floor. `374394d0f`: read + secret + suggest
hoisted into a scoped-thread PARALLEL phase, default-on
(`RIIR_REFINE_SCAN_PARALLEL=0` demotes byte-identically) — `suggest_file(&mut)`
stays single-threaded by the G4-scratch discipline, so each thread rebuilds its
OWN matcher at the identical posture (lease overlay + active-set + standing
depth policy); the armed escalation seat keeps serial (the hosted
deadline/rate-latch contract); matcher-build failure exits loud pre-thread;
thread panics propagate via `resume_unwind`. Measured A/B same binary on
seal-game-editor (770 files, 63 MB, ~7 ms/span): 53.3 s → 15.7 s wall (3.4×,
8 threads), stdout byte-identical vs the serial arm, the roll-up (770 files ·
7048 spans · 159 findings) matches. `2986c4a92`: file-count chunks let the
1.5 MB monster file pin one thread — greedy BYTE-PREFIX ranges keep slices
contiguous (order preserved, results byte-identical) while balancing
bytes/thread; re-measured 17.8 s parallel vs 55.5 s serial (3.1× same build).
The same commit adds the two UX lines the owner asked for: the
`⏳ this is CARGO CLIPPY compiling the workspace — cargo refine is waiting, not
working` notice at oracle spawn (an oracle-on 4-file run is 97% cargo clippy:
31.8 s of 32.8 s), and the end-of-run `⏱ clippy | scan | refine` phase-blame
split (only the legs that ran; the oracle-off run's scan leg is the line that
moves). `86324551` (follow-up): dropped a dead `scan_started` shadow in the
parallel arm — an unused-variable red that broke clippy -D warnings (Layer 5a)
for every lane on develop; the outer binding already covers both arms. Both
lines are in the AGENTS.md CLI UX contract (same commit). Direct owner-ask perf
work — no issue file; the full measurements live in the two commit messages.

## 2026-11-06 — Issue 209 T1–T5 LANDED: the c′ referee — telemetry before/after + the structural-identity certificate (verdicts PENDING, case #1 enrolled)

`a6eb6be0` + `a7307895` (zed-ceeedd52@m3, the issue 206 delegated-owner
follow-up): new ungated module `src/corpus_referee.rs` — a store-free state
machine over the dependency-inverted `AttemptRow` view (the state machine + LCB
math + verdict table test at DEFAULT features; the `TrajRowDoc` mapping is the
one `latent_retrieval`-gated seam). T1: `BenchIdentity` — domain-separated
BLAKE3 over the gen-N atom tables + healed output —+
`CorpusBinding::with/verify_bench_identity`; the binding digest domain bumps
v1 → v2 (safe: nothing persisted in the wild; the certificate folds the record
identity; an ABSENT cert field is not an axis move — the pre-209 binding shape
passes through). T2: `split_windows` partitions per-row `last_seen_at` vs the
edit epoch (untouched rows = pure pre-edit evidence); controls = rules with
evidence on BOTH sides; the control drift is a deterministic sorted median
(`f64::midpoint` at even n). T3: the pre-registered bar =
`best_belief_score(f_pre, s_pre, 0.025) − best_belief_score(f_post, s_post,
0.975)` — the Bonferroni split of LCB95 across the two windows (joint ≥95% by
the union bound; the katgpt-core `best_belief` LUT/closed form, zero new
deps); PASS = lcb_drop > 0 ∧ drop > control drift (the DiD gate — the
false-attribution pin); verdict states PASS / FAIL / PENDING /
UNDECIDED-by-starvation (8-batch kill window), PENDING never folded into a
pass; 16 tests incl. the four-state table, the DiD bind, vintage, starvation,
and the enrollment-file round-trip + unknown-version refusal. T4:
`corpus_diagnose --referee [<rule>]` (REPORT-ONLY; the read BUMPS the batch
counter and persists it — a report that forgets its own observations would
re-count the same epoch forever) + `--enroll` (atomic machine-local ledger
`.refine/corpus_referee_enrollments.json`, unknown-version refusal). T5: case
#1 `hashmap-with-capacity` @ ac280021 enrolled (pre snapshot 9 fails / 18
attempts, compile bucket, edit_ts 1793793600); the live read (15 stores /
1,557 rows): PENDING (BelowSupport) — 2/30 post-edit attempts, 6 controls; the
snapshot-over-3-stores population bias disclosed in the enrollment record
(conservative against PASS). Gates: 16/16 referee tests, default lib 2873/0,
clippy −D clean at both postures (default + latent_retrieval --all-targets),
rulewright-first mechanical fixes (6 edits via `cargo refine --verify`) +
manual semantic fixes (`manual_midpoint`, `assert_is_empty`). The referee is
now the STANDING read — verdicts accrue passively as the stores grow; the next
meaningful event is a read at n ≥ 30. The issue stays OPEN until then.

## 2026-11-10 — Issue 219 REMOVED (done): the gist-rs → riir-ai org migration closes

The issue file is removed per the noise-reduction rule; every residue it
tracked is landed or owner-confirmed. A2 (prod redeploys: kat-service gen-2
net-leg at `49a59d0`, reflex-site `84730a68`, rethink `2108dbd0`) CLOSED
2026-11-08 with all five live surfaces probing 0 gist-rs org refs; A3 (the
4090 disk-full incident) RESOLVED 2026-11-08 with the gitsync lane verified
end-to-end over the flipped remotes; and A1 (the Actions PAT/secret rescoping)
is recorded DONE by the later 221 session (issue 221 P4's deprecation
sequencing: "issue 219's A1 PAT rescope already done"). The drift-ledger
observations (amo's 2026-11-10 remote flip, the ENOSPC misdiagnosis lesson,
the 308 redirect confirmation, the seal `.git` lock sweep) live on in git
history (`git log -- .issues/219_gist_rs_org_migration_owner_residue.md`);
the cross-repo landing records stay in dapps bench 018 (`c0fa14f`). Removed
at HEAD `22666bfa8` lineage; the `.highwater` (223) is untouched — numbers
are never reused.

## Lessons

A fresh worktree + empty target dir is the only clippy reading that means "0" — a stale-cache close-out claimed 0 and 34 findings were live.

## 2026-10-24 — AGENTS.md compaction: per-batch corpus histories moved verbatim

The batch-by-batch narratives below (rust_perf + kernel_opt Role bullets, their Feature-Flag
Commits: `fa63d97`, `88d0e57`, `81ae003`, `4d17995`.
