# Plan 154 — Batch 163: the frontier-lead mining pack IX (`manual_string_new` + `needless_late_init`)

**Status:** COMPLETE — landed 2026-09-19 (the Batch 162 re-arm; the seventh
frontier-lead pack; research note: `.research/190_batch163_frontier_lead_pack9.md`).
Landing hashes recorded in the batch commit.

**Context:** Batch 162 (Plan 153) graduated all 17 frontier leads — the frontier
population is EMPTY, the e2e yield floor now records `all_graduated`. The two
recorded spares from the plan152 probe (`scripts/plan152_frontier_probe.py`,
2026-09-18): `manual_string_new` (`String::from("")` → `String::new()`) and
`needless_late_init` (`let x; x = v;` → `let x = v;` — the twice-used-binding
`x + x` tail dodge clears the let_and_return crossfire on the naive seed).
This batch ARMS them: rules + proposers + atoms + frontier families, the full
standing-gate walk.

- [x] T1 — Probe re-verification at the live toolchain: re-run the probe arms
      (msn/nli broken-fires / seed-clean / healed-compiles + tier/group reads)
      against clippy 1.98; record verdicts in the research note. If a lead
      FAILS the probe (upstream drift), record and fall to the alternates.
      **DONE**: plan152 re-run confirmed both; the new plan154 probe added the
      full guard matrix + group pins (msn = PEDANTIC, nli = style/default-on).
- [x] T2 — Dedup grep BEFORE proposing (vocabulary translation:
      `from\(""\)|string.new|late.init|deferred.init` across
      `src/domains/*/corpus.rs` + proposers) — DONE pre-plan: 0 hits
      (the one `String::new` hit is an unrelated fixed_code in the
      write-inside-fmt rule).
- [x] T3 — `manual_string_new`: corpus entry + proposer
      (`src/domains/clippy_lints/proposers/manual_string_new.rs`) + mod
      registration. Guards: exact `String::from("")` zero-arg-empty-literal
      call only (typed vars/`format!`/`to_string()`/`""`-with-content/
      turbofish `From::<&str>::from` refuse), per the probe + clippy's own
      predicate. **Landed with two probe-driven scope extensions: the
      explicit turbofish FIRES upstream (handled, resolution-identical) and
      `"".to_string()` fires upstream (refused — different seam, documented
      divergence). Pedantic group pinned.**
- [x] T4 — `needless_late_init`: corpus entry + proposer
      (`src/domains/clippy_lints/proposers/needless_late_init.rs`) + mod
      registration. Guards landed: adjacent non-mut pair, plain-ident pat
      (Pat::Type carried), branch/attrs/mut refuse, annotation + rhs
      verbatim. **The attrs guard is the batch's syn finding: statement
      attributes respan the inner exprs — the guard is the textual
      leading-check (3 structural attempts were blind; Research 190).**
- [x] T5 — Batch parse gate: the clippy domain's standing corpus gates
      (`all_fixed_code_parses` + `all_buggy_codes_fire_their_lint`) + the
      mod.rs `all_lints_fire_and_parse` fixture table (+2 rows, 80 total)
      + the two proposer modules' own test suites (25 tests).
- [x] T6 — Frontier miner families: both added (`covered: true`, 2 fixture
      pairs each, `x + x` tail dodge on nli) + the BROKEN-only probe
      markers in `family_probe` + the count pins 36→40 / 17→19 families.
- [x] T7 — GOAT: G1 (syn parse all new rules) · G2 (lib default 1991/0 ·
      latent 2144/0 · miner,score,latent 2308/0) · G3 (clippy `-D` clean
      default + all-features + latent states).
- [x] T8 — Freeze-table regeneration: v2 **90/11** + v3 **89/16** (v3
      purely additive; v2 exclusions 12→11 the benign shuffle; bad atoms
      UNCHANGED 6/7 — neither new atom interferes); both e2es PASS 0 drift
      (40.6s + 43.1s); splice via plan150 script; the temp regen example
      deleted with its Cargo.toml row (carried required-features while it
      lived — the Issue-513 class).
- [x] T9 — Frontier e2e re-arm: VERDICT ✓ (40 seeds clean, 19 family
      loops, 2 fresh admissions, 17 rows 0 drift, band fold 19/19; store
      holds 19 solved rows).
- [x] T10 — Score-bench #139: 96.36% FLAT bit-identical (vocab 65; created
      unchanged; pool_depth_rescue_guard did NOT re-fire — no re-aim; 2
      pre-existing toolchain drift warnings recorded).
- [x] T11 — Retrieval floors HELD (15/15, blended 94.7/96.2/96.9).
- [x] T12 — Research note `.research/190_batch163_frontier_lead_pack9.md` +
      AGENTS.md corpus counts (84/65) + the clippy_lints + frontier_miner
      bullets + `.distill/001` snapshot turn.
- [x] T13 — Commit `feat: batch 163: ...` + push.

**GOAT gate (batch-level):** G1/G2/G3 above + the freeze/e2e/score-bench/retrieval
walk. No promotion claims — clippy_lints is already default-on; the pack only
widens the corpus.

**Defers:** none planned. Arxiv walk #10 opens Mon 09-21 (per snapshot — not
this batch).
