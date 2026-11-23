# Plan 377 — the `ane` domain + the two-pass ANE pregate (owner call 2026-11-14)

**Status:** IN FLIGHT — Phase 1 (domain + pregate substrate) landing; Phase 2 (cost leg) queued.
**Lease closed:** zed-84603cf7@amo, epoch 1794902600, was 2026-11-19T07:23:20+00:00 — the lane's work LANDED: ane_pregate PROMOTED to default + the shipped release set 2026-11-17 (the G2 cost leg PASSED two-run-stable on bmo; AGENTS.md §ane_pregate carries the record)
(P2.1 G2 cost cell — the work order from zed-m3-i242-t0@m3 via zed-0b82cbef@m3: harness `examples/ane_pregate_g2.rs` mirroring the calibrated_dispatch G2 lane, three arms baseline/ane-ungated/ane+pregate through the real suggest path, 3 interleaved rounds, quiet-box PROVENANCE; then P2.2 per the owner's standing law. The measurement queues via `agent-mail rig run amo --exclusive` — never a BUSY-box number.)
**Owner ask (verbatim, 2026-11-14):** "why no ane domain btw? we should add and better do code scan for ane first pass before apply second pass like kernel".

## Why there was no ANE domain (the honest answer, now superseded)

Research 409 routed the liquidcodec-ane distill to `kernel_opt` under the
Batch-337/357/358 ANE/Metal-graph precedent — "NOT a new domain (the class has
owners)" was MY routing call, not an owner decision. The class has owners in
the sense that nothing was dropped, but the routing had a real cost: the ANE
rules' retrieval surface (CoreML/ANE host vocabulary) is drowned inside a
1081-rule GPU-kernel corpus, the demonstrability column cannot separate them,
and future ANE distills keep widening an unrelated expert pool. The owner's
call converts the routing: ANE gets its OWN domain, and — the substantive half
of the ask — its own **first-pass code scan** before the second-pass rule
application, exactly the `kernel_pregate` two-pass shape (Issue 208 / Bench
131: file-level signal gate → span scope → domain joins the fan-out on
signaling files only).

## Shape (mirrors kernel_opt + kernel_pregate verbatim)

- **`ane` feature (opt-in, NOT default, NOT release set):** the 8th retrieval
  domain (`DomainId::Ane`, `DomainMask` u8 bit 7 — the mask had exactly one
  free bit; `ALL` len 7→8). Seed corpus = Research 409's six candidates
  (R1–R6), suggestion-only, `NotBenchable-by-domain` annotated in-rule (the
  win metric lives in the CoreML/ANE runtime; no validator here hosts it —
  the domain-unbenchable annotation class, Batch 136/337 precedent).
- **`ane_pregate` feature (opt-in, implies `ane`):** the FIRST PASS. A
  recall-biased needle scan over the raw source detects ANE-bearing files
  BEFORE the ane matcher fans out (second pass); non-signaling files pay one
  byte-scan and never see the ane domain; signaling files scope the fan-out
  to signal-line ± window. Path signal: any path component word `== "ane"`
  (`crates/riir-infer-ane/…`, `laya_ane.rs`, `ane/mod.rs`) → the whole file
  is an ANE surface. `RIIR_ANE_PREGATE=0` (exact literal) is the DEMOTE-only
  env from birth (env-switch law), so the promotion posture is ready and the
  A/B (armed vs `=0` within one build) is honest.
- **Where the pregate lives:** `span_allow` widens to two gates (kernel +
  ane, same fallback semantics: gate each denied domain out; an emptied set
  falls back to ALL-minus-denied — byte-identical when ane is off/Off).

## Seed corpus (Research 409 R1–R6, two-source convergences kept)

1. `ane-fixed-window-mask-stateless-streaming` (B+ LEAD; liquidcodec
   `lc_ane.py` L5–6/L14 + beshkenadze README caches-in/caches-out — the
   receptive-field ≤ window admission gate + validity mask is the decision
   content).
2. `ane-fp16-norm-pow2-prescale-overflow` (B; the overflow pole of the pow2
   prescale family — B242 owns the subnormal-floor pole).
3. `ane-compute-plan-pin-cpu-and-neural-engine` (B−; placement islands
   acceptable, `.all` demotes to GPU).
4. `ane-graph-restructure-for-ane-lowering` (B−; graph shape decides
   lowering; int4-post-training negative as the regime evidence).
5. `ane-precision-dsp-stays-cpu-fp32` (B−; STFT/iFFT tail stays CPU fp32).
6. `ane-weight-palette-depth-retention-gate` (B−; measure retention per
   posture — the FluidAudio-6-bit vs beshkenadze-6-bit discrepancy recorded
   IN-rule).

Fixtures: in-corpus test arms (per-rule shape assertion + parse gates +
freeze/thaw round-trip + pins count) — the batch-fixture discipline at seed
scale; measured-cell-backed regime rows land with Plan 633's cells (same
P4.2 dependency Plan 376 records).

## Tasks

- [x] P1.1 Plan 377 filed; research 409 status updated (owner decision recorded); plan 376 redirected (domain home, not kernel_opt batch).
- [x] P1.2 Cargo.toml `ane = []` + `ane_pregate = ["ane"]` (opt-in, zero new deps).
- [x] P1.3 `DomainId::Ane` (ALL[8], label "ane", applies_to_path `.rs`, is_compiled) + `DomainMask` (u8 bit 7) — domain_router.rs.
- [x] P1.4 `src/domains/ane/{mod.rs,corpus.rs}` — AneRule/AneDomain (parse-only verdict) + the 6 seed rules + in-corpus fixture tests.
- [x] P1.5 `src/ane_pregate.rs` — needles (pinned), boundary needle `ane`, path signal, `AnePlan::{Off,PathSignal,Scope}`, `RIIR_ANE_PREGATE=0` demote, `--domain ane` bypass arm, unit tests.
- [x] P1.6 latent_matcher wiring (field, resolve/book/index, has_domain, span_allow 3-gate, suggest_file_inner, suggest_all_domains, pool_and_ranked, with_rule_embed, error string) + gating tests mirroring kernel_pregate_tests.
- [x] P1.7 index_cache domain_tag/domain_from_tag 7 (additive wire); retrieval_eval domain_index + router_eval_domains + eval_fixtures_ane; demonstrability T3_DOMAINS[9]; domains/mod.rs registry row.
- [x] P1.8 pins manifest (`ane_rules` key + dump insert + script posture + required list) + health_views `--info` row; regenerate `.benchmarks/pins_manifest.json` in the same landing.
- [x] P1.9 fmt + clippy (default, all-features, no-default) + lib tests at the ane postures.
- [x] P2.1 G2 cost leg (pre-registered): added p50/p95 per .rs file, three rounds, quiet-box PROVENANCE (amo), arms = baseline / ane (ungated) / ane+pregate. Bar (kernel_pregate precedent): p50 ≤ 1.0 / p95 ≤ 5.0 ms added per file on the mixed walk; utility floor = every eval fixture span still cited under the gate. — **MEASURED TWO-RUN-STABLE on bmo 2026-11-17** (the honest road first: the `[x]` at landing was a FALSE PASS — commit `486978a29` checked this box while its own prose said "unrun", flipped back same-day by zed-m3-i242-t0; then THREE amo exclusive attempts poisoned + cancelled — ① `01a1e879` i216-merge storm ×35; ② `01a1e893` merge-aibrain `cargo test --workspace` ×42; ③ `01a1e8af` the same gate re-fired a fresh wave within 1 min of the quiet gap, ×44: the seal merge review loop spawns compiles continuously on amo, quiet-gap timing cannot win while it's live); **ESCALATED to bmo per the pre-committed fallback** (checkout pulled to develop `383082c9`, bins built locally — shared jobs; bmo's riir-refine gitignores Cargo.lock so `--locked` refused, build without it; first measurement failed on the rig's missing cwd — `src` walked zero files, fixed with `--cwd`; the adjudicator itself then crashed on a REAL bug — `per_file_us[f]` is the per-round list of 3 timed-pass samples and the old spelling fed `median()` a list of lists — fixed `8f4a32e8`, flatten to 9 samples/file before the median, bars unchanged). **THE PAIR: run 1 `01a1e8c1` (PROVENANCE load=1.35) pregate added p50 0.015 / p95 0.423 ms; run 2 `01a1e972` (load=1.28) 0.045 / 0.657 ms — both PASS, 1.5–22× under the ceilings, utility floor 8/8 both runs, 728-file walk, verdict class identical (two-run stable).** The ungated arm PASSes MARGINAL both runs (p50 0.805/0.828) — the Bench-131 re-run-alone law belongs to IT, never the promotion posture; data `/tmp/i377g2.{5DteR2,WNSJ3D}/`.
- [x] P2.2 Promotion verdict per the owner's standing law (gain no perf hurt → default): only on a two-run-stable PASS; a FAIL stays opt-in with the record. Owner-notified either way. — **PROMOTED 2026-11-17 (`5052f5bf`): ane_pregate → cargo default + the shipped release set** (implies ane — the domain ships with the gate that keeps its cost off non-ANE files); the typed RELEASE_FEATURES + release.yml's about/build lines + manual_release.sh FEATURES moved in the same commit (the `release_set_matches` trio pins the lockstep — 18/18 build_stamp tests green; clippy −D warnings at ane_pregate/ane+ane_pregate/no-default postures). Kill-switch `RIIR_ANE_PREGATE=0` stays DEMOTE-ONLY. OWNER-NOTIFIED via the boss lane with the measured pair + the road (three amo poisons → bmo escalation → the pair).
- [x] P3.1 AGENTS.md rows (domain list + both features) — same commit.

## Non-goals

- NO `.py` scan lane: the walk collects `.rs`/`.wgsl`/Dockerfile/Cargo.toml
  only; coremltools conversion scripts are out of scope until a walk lane
  exists (recorded, not smuggled).
- NO proposers (suggest-only seed; the fix lane declines an ane citation the
  same way it declines a sec suggest-only one).
- NO RuleDomains row (the KAT receiver's 8-slot registry has one free slot —
  the shader_perf precedent: ane rows cross as their own spelling until the
  registry widens; never mislabel).
- Existing kernel_opt ANE-family rules DO NOT move (corpus/Elo/lease
  churn for zero retrieval gain; future ANE distills route here).
