# Plan 275 — Batch 271: the splash-wave second-tier serving pool

**Status:** IN PROGRESS — claimed 2026-10-21 22:59 +0700 (shikuwa/4090, riir-refine idle
loop, Batch 271; claim expiry 2026-10-23 22:59 +0700; push is the arbiter).
Source: incoai/splash, the B268/B270 staged second tier — 13 B/B− candidates
(`253506f`, `88d0e57`, `91d4ddb`, `4d17995`, `fa63d97`+`593f2a6`, `68b9e70`, `81ae003`,
`ed1092f`+`2f5fe8d`, `dab8841`+`6fcd9b1`+`25502fd`+`7e51fcb`, `5ea4043`+`82b93c5`,
`b7844dc`, `4fc7465`, `e2c0af2`) enumerated in Research 305 §STAGED + Research 306
§staged. All rust_perf-lane. Honest drops allowed at the T0 re-read (the B268 drops
already consumed `51ae046`/`cfa91f5` from the wider staging).

## Tasks

- [ ] T0 — clone splash to `.raw/b271/splash`, verify all 13 staged commit ids present;
      read each candidate's subject+diff, grade own-rule / rider / drop (dedup grep
      against rust_perf 254 + kernel_opt first, 3+ vocabulary variants)
- [ ] T1 — claim row committed + pushed BEFORE authoring (this plan + snapshot row)
- [ ] T2 — author rules via disjoint-slice subagents (≤3 rules per slice, standalone
      compile + execute fixtures before landing; ALL NotBenchable annotated unless
      measured; every quote machine-verified at the pin)
- [ ] T3 — wire entries files + batch271 needle gate; count pins via ONE shape-asserted
      script pass (the B265 lesson: never fuzzy-edit multi-site pins)
- [ ] T4 — gates: batch271 needle FIRST-RUN; default lib; rust_perf_bench lib;
      clippy −D ×3 postures; retrieval floors re-measure (isolated CARGO_TARGET_DIR)
- [ ] T5 — freeze gen2+gen3 e2es (rust_perf corpus delta ⇒ regen owed if tables move;
      B270 precedent: e2es run, no movement expected for NotBenchable-only adds)
- [ ] T6 — score-bench appended; release-set binary rebuilt + installed (build hash ==
      HEAD) BEFORE push (the §0 bin-age law)
- [ ] T7 — docs: Research 307 + this plan all [x] + AGENTS.md rust_perf bullet + README
      row + `.docs/11_domains/rust_perf.md` + snapshot landing row; `.raw/b271` removed
- [ ] T8 — commit `feat: batch 271: ...` + push
