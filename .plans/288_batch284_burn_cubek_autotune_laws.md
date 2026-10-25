# Batch 284 — burn/cubek autotune + fusion laws (the distill-verdict conversion)

**Status:** PLAN OPEN — verdict LANDED (Research 321, 2026-10-25); authoring DEFERRED until the box frees (the sibling QRR gauge-attention cell holds the GPU; the GOAT gates need cargo).
**Source claim:** `.distill/001_mining_queue_snapshot.md` claim row 2026-10-25 ~16:1x +07 (shikuwa/4090) — verdict + plan only; the B284 batch number was deliberately left unclaimed for authoring so whichever lane frees first converts. **This plan is that conversion vehicle.**

## Sources (pins — re-verify EVERY quote at these revs at authoring time)

- `tracel-ai/burn @ v0.22.1 = 907ab111a02c2dea47bc41633754f095c6b2220e` (MIT OR Apache-2.0) — the burn-pin surface: `crates/burn-cubecl/src/kernel/{matmul,attention,reduce,interpolate}/tune*` + `bounds.rs` ×4 + `memory_order.rs` + `crates/burn-fusion/src/search/block.rs` + `crates/burn-fusion/src/backend.rs` (FuserStatus) + `GUIDE.md` lives in cubek.
- `tracel-ai/cubek @ tip 08659297eec3414311df573aada3f22fb8142054` (MIT OR Apache-2.0) — **PROVENANCE DRIFT, disclosed:** burn v0.22.1 pins cubek 0.3.1 = rev `51c43a1af7eb07a904b8a13568e953d377d20bb4`, which does NOT contain `cubek-std/src/autotune/` (roofline.rs/priority.rs are newer than the pin; verified by `git ls-tree` at the pin — the module is ABSENT there) — burn 0.22.1 consumes the roofline through `cubecl::std::throughput::roofline_bounds` + `burn_backend::cubecl::autotune::with_roofline_bounds` (the cubecl rev burn's cubecl dep carries), NOT through the tip's cubek-std module. Candidate A's law therefore cites BOTH: the tip's cubek-std autotune docs (the mechanism prose, richest form) AND the burn-pin's `roofline_bounds` consumers (what 0.22.1 actually ships). The anchor-bucket law (candidate G) IS at the burn pin (`cubek-matmul/src/tune_key.rs` @ 51c43a1a lines 195-215 verified).

## Tasks

- [ ] T0 (authoring-time): re-clone both repos at the pins above into `.raw/b284-*`, re-verify every Research-321 quote verbatim at the pin (the B96-98 precedent), re-run the dedup greps under 3+ vocabulary variants (corpus may have moved).
- [-] T0-defer note: authoring waits for a quiet box — the gates below are cargo-bound (G2/G3), Plan-337.
- [ ] T1 Dedup re-verification (gate): for each candidate below, grep `src/domains/kernel_opt/corpus/` + `src/domains/rust_perf/corpus/` under both vocabularies; record one evidence line per candidate in the batch research note. The 2026-10-25 census (Research 321 §5) is the baseline.
- [ ] T2 Entry authoring — target 4-6 rules from the candidate table (Research 321 §4). Priority order: A (roofline-bounded round + Ceiling/Either), B (read-cold-not-fill), C (timed-zero guard), D (cost-model-skipped-work), E (unfusable-head settle), G (anchor bucketing). F/H/I/J convert only if a slot remains and dedup holds after re-grep. Fixtures ORIGINAL minimal reproductions (licence hygiene); never paste burn/cubek source verbatim.
- [ ] T3 Standing gates: `batch284` needle + parse gates (both sides of every rule); kernel_opt lib + default lib + latent lib runs (never two full suites in one command — the watchdog class); retrieval floors HELD or re-pinned with the delta recorded.
- [ ] T4 Count pins: kernel_opt tests.rs multi-line + single-line + router expert-total; rust_perf sites pin; repair any stale cross-domain pins the batch exposes (the B283 batch249_slice3 class).
- [ ] T5 GOAT: G1 parse/contracts; G2 n/a for NotBenchable rules (annotate `NotBenchable`, `bench_spec: None` — the domain-reason class per Research 321 §6); G3 clippy `-D` ×3 postures; G4 alloc-free where the fixture path touches it.
- [ ] T6 Freeze/score-bench: freeze NOT owed (kernel_opt + rust_perf are not the clippy corpus, P30); score-bench run after the corpus delta (append-only history); rust_perf IS release-set surface → **binary rebuild + install REQUIRED at landing** (the 0-law; build == landing HEAD).
- [ ] T7 Research note (batch record, `.research/` next number) + docs/AGENTS/queue-snapshot updates + `.plans/.highwater` + `.research/.highwater` bumps; `rm -rf .raw/b284-*` at close (the cleanup law).
- [ ] T8 Commit `feat: batch 284: ...` + push; verify `cargo refine --version` build hash == HEAD post-rebuild (the bin-age assertion).
