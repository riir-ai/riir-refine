# Plan 274 — Batch 270: the splash-wave rust_perf serving pool (incoai/splash, the B268-staged leads)

**Status:** LANDED 2026-10-21 (shikuwa/4090, riir-refine idle loop) — corpus 242→254 rust_perf (12 rules, entries_39-42; the f52d1ad grade LAND verdict made it 12, not 11; 4 riders; kernel pins are DOMAIN-LOCAL 937 — the conflation catch in Research 306).
**Claim:** shikuwa/4090 idle lane, expires 2026-10-23 (48 h); push is the arbiter; other lanes
re-fetch before staging and yield to this row. The pool is THIS lane's by construction — the
B268 landing row staged it ("Batch 270 = the wave's rust_perf serving pool ... claim plan-first
at the next slot — freeze regen + score-bench + release-set binary rebuild owed at landing").

## Source

`incoai/splash` @ the staged window `be3b083c..origin/main` (205 non-merge commits, Apache-2.0;
the graded pool from Research 305 §STAGED — the B-slice read already grep-zero verified every
candidate). Clone tip moved past the window to `1482f6b` (tag 1.4.0, README + accumulated
post-staging commits) — beyond the staged window, disclosed, NOT mined this batch (next watch's
concern). Clone: `.raw/b270/splash`, removed at close.

## The pool (Research 305 §STAGED, verbatim inventory)

Own rules (11, category from the staged grade):

1. `reclaim-progress-keyed-on-refused-resource-returned` (allocation; `9ca9768`+`f6957e8`+`32cf878`)
2. `refusal-wait-keyed-on-shortfall-returning-transfer` (concurrency; `fdc8f01`+`c27aaf7`+`e90b43c`+`a7406bb`+`41e8c82`)
3. `restore-point-chain-copies-keyed-on-disk-state-count` (caching; `a20f344`+`65f9713`)
4. `persistent-cache-teardown-needs-second-witness` (caching; `6e21cc9`)
5. `hot-slot-recycler-flat-tables-zero-alloc-claims` (caching; `a7e5347`+`f3eb1a0` — 13 ns/entry vs 22 source-reported)
6. `cache-share-capped-at-measured-distinct-working-set` (caching; `18f9c69` — hit-flatness sizing)
7. `cache-reuse-assertion-gated-on-eviction-counter` (caching; `2712e67` — the third serving-cache oracle law)
8. `publication-takes-over-condemned-cache-entry` (caching; `716760d`)
9. `watchdog-deadline-moves-on-command-progress` (dispatch; `8df9e95`)
10. `page-miss-blocks-one-ranged-read-scatter` (io; `c7a42cc`+`c14ba73`)
11. `tier-write-defers-to-urgent-reads-except-a-read-behind-it` (io; `fc6fa54`+`be30994`+`47325f9`)

Riders (4, count-neutral — host by family grep at placement):

- runway-refilled-by-consuming-op (`945635d`)
- release-unit-matches-demand-unit (`d26ff04`)
- retry-hint-from-refill-ledger (`ea1683d`)
- form-keyed reusable counts (`41e8c82`)

Second tier (grade B/B-, graded at T0, honest drops allowed): `253506f`, `88d0e57`, `91d4ddb`,
`4d17995`, `fa63d97`+`593f2a6`, `68b9e70`, `81ae003`, `ed1092f`+`2f5fe8d`,
`dab8841`+`6fcd9b1`+`25502fd`+`7e51fcb`, `5ea4043`+`82b93c5`, `b7844dc`, `4fc7465`, `e2c0af2`.
Intel row with a staged conversion grade: `f52d1ad` (constrained+unconstrained one command,
measured -23.7%/-34.0% time/token on 35B/27B mixed-lane traffic — rust_perf:dispatch candidate;
lands only if it survives fresh dedup).

## Tasks

- [x] T0 — pool reconciliation: clone to `.raw/b270/splash`; all 34 staged commit ids verified
      present; tip-move disclosure (`1482f6b`, outside the window); dedup greps (3+ vocabulary
      variants per concept) over BOTH corpora — the named watch families: B265's
      cache-budget/prompt-state rules, the B257 TensorFold serving pack, the B236/B247 splash
      owners, the eviction/refusal families.
- [x] T1 — plan-first landed + claim pushed (this commit).
- [x] T2 — authoring slices: subagents over disjoint commit sets, <=3 rules each (the B268
      stream-idle lesson); every fixture compiles STANDALONE (inline shared items with a
      provenance comment); every source quote machine-verified at its pin; annotations
      (NotBenchable vs benchable) adjudicated per rule at authoring.
- [x] T3 — riders placed by family grep (the host rule named per rider; wrong-host staging is
      the B257 RD2 class — verify the anchor appears in the intended host only).
- [x] T4 — coordinator wiring: `entries_39.rs` NEW; mod.rs (mod decl + extend + category
      comments + the count header lines).
- [x] T5 — count pins via ONE shape-asserted python pass (the B265 multi-site lesson): rust_perf
      tests.rs total 242->253 + category tuples + annotated/unannotated + witness pins; the
      cross-domain total pins (kernel_opt tests.rs 937->948, all sites); router.rs expert total;
      mod.rs doc comments; AGENTS.md; README; `.docs/11_domains/rust_perf.md`.
- [x] T6 — needle gate `batch270` at `--features rust_perf` (the feature-gated green-zero class).
- [x] T7 — gates: default lib, rust_perf lib, kernel_opt+latent_retrieval lib (retrieval floors
      re-measure), clippy -D x postures.
- [x] T8 — freeze-table regeneration gen2+gen3 (rust_perf corpus delta, plan 137 T7.5) +
      score-bench appended via the rebuilt binary.
- [ ] T9 — release-set binary rebuild + `cp` to `~/.cargo/bin` (the Windows plain-copy hazard:
      explicit cp, never a silent symlink assumption) + `--version` build hash == HEAD verified.
- [x] T10 — Research 306 + AGENTS.md rust_perf bullet + `.docs/11_domains/rust_perf.md` entry +
      snapshot landing row (same commit as the landing).
- [x] T11 — staged_set_audit + commit `feat: batch 270: ...` + push; `.raw/b270` removed at close.
