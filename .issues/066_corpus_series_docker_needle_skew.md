# Issue 066 — the corpus series' docker counts are wrong since the domain landed (needle counts constructor plumbing, not entries)

**Status:** T1/T2/T3 DONE (2026-09-04; `c9863d6` needle fix + count-pin test, `d731cac` store repair) · T4 DONE (2026-09-04, this commit — landed as the log-not-fail design note the task specified; owner can still veto the wording/verbosity)

- [x] **T1** — fix the docker source-walk needle in BOTH walkers: `src/bin/heal.rs` `CORPUS_DOMAIN_TABLE` + `scripts/gen_dashboard.py` `CORPUS_DOMAINS` (the two are a documented mirror pair). `rule_id:` appears exactly **2×** in `src/domains/docker/corpus.rs` at every rev (the `rule()` fn param + the struct-field init) — the shorthand positional constructor (the file's own design note celebrates it) means there is no per-entry `rule_id:` text. Needle `    rule(` matches exactly the 8-space-indented constructor calls: 27 at landing (`3839457`), 31 since `e63f819` (08-29), 31 at HEAD. Whitespace-coupled — acceptable for a documented best-effort walk, but pin it with a count test in T1. **DONE `c9863d6`.**
- [x] **T2** — add the count-pin test: the Rust source-walk for docker at HEAD must equal `build_docker_seed_corpus().entries.len()` (31); same assertion shape on the Python side (or a fixture file if the Python walk is not unit-tested). Without this the needle can silently rot again on a re-indent. **DONE `c9863d6`** — `corpus_stats_needle_tests` in `src/bin/heal.rs`: (a) `source_walk_agrees_with_runtime_count_for_compiled_domains` pins EVERY compiled domain's walk == runtime count (generalizes beyond docker), (b) `docker_needle_is_not_the_old_plumbing_count` pins the table needle ≠ `rule_id:` AND its count > 2 with the Issue-066 message. Green under both the default triple and the 9-feature union. The Python side is covered by the same invariant transitively — the projection's store rows now equal the mined layer (verified below), and a Python needle rot would surface as a projection-vs-store divergence in T4's logging if landed.
- [x] **T3** — repair the poisoned history: true series from git is **08-28=27, 08-29=31, 08-31=31, 09-01=31** (09-02/09-03 already stamped 31 by runtime-corpus binaries and are correct). The store rows for the four bad days must be corrected (check whether they are `source-walk`-stamped or jsonl-imported — `--import-series` yields to stamped rows, so if they were imported, a corrected jsonl + re-import fixes them; if stamped, the repair needs the stamp path or a documented one-off). Then re-run the projection so `corpus_history.jsonl` is true and the dashboard series stops showing docker collapsing 2→31→2 across days that never happened. **DONE `d731cac`.** Measured along the way: the poisoned rows are FIRST-HAND (a corrected-jsonl `--import-series` run against the real store wrote **0 corpus rows** — every bad docker row yielded, proving `source-walk` provenance), so the repair went through the only sanctioned dated writer: `examples/repair_066_corpus_store.rs` (one-shot `upsert_corpus_count` per day, each value verified against `git show <rev>` BEFORE writing — a repair that trusts its own table is a second poison — provenance honestly `repair-066`, second-hand by definition so a future import MAY supersede it; every writer now converges on the truth). Post-repair: store series = projection = mined layer (27/31/31/31/31/31), the regeneration diff is gone, `git checkout` no longer needed after a dashboard run.
- [x] T4 — **DONE 2026-09-04** (same day). `load_corpus_history` now logs (never fails, never
  overrides) every store-vs-mined per-domain divergence on an overlapping day before the
  recorded-wins merge, citing this issue in the log line. Landed together with the mined-layer
  CWD anchor it exposed: `_count_in_rev`'s `git show`/`git ls-tree` inherited the caller's CWD —
  run from anywhere but the repo root, every count read 0 and the merge then "won" those zeros
  over real store rows (a mined-layer failure indistinguishable from drift; the `git log` call
  already carried `cwd=REPO_ROOT`, the two count helpers did not). Both anchored to `REPO_ROOT`.
  Validated three ways: synthetic rows reproduce the exact 066 poison shape (docker store=2 vs
  mined=31 fires; missing-key and non-overlapping rows skip); a foreign-CWD run now produces the
  correct series; and the real store run surfaces exactly 3 residual lines, each read to root
  cause and all benign: **rust_perf 08-28 (store 91 vs mined 89)** + **kernel_opt 09-03 (store 463
  vs mined 453)** — the store counts the WORKING TREE at stamp time (batch-102/103 rules were
  staged pre-commit), the mined layer counts committed history at the day's last corpus commit —
  and **rust_perf 09-02 (store 98 vs mined 99)** — the stamp preceded batch 101's same-day push.
  Commit-time vs stamp-time is an inherent definitional boundary, not skew to repair; the lines
  make it visible so nobody re-reads it as a second 066. The projection's 09-03 row follows the
  store to 463 (recorded wins, unchanged semantics).
  **Evidence:** this commit; [`.benchmarks/corpus_history.jsonl`](../.benchmarks/corpus_history.jsonl)
  projection updated in the same commit.

## Evidence

- The 065 T6 de-risk pre-run (2026-09-04 ~00:52 +0700) regenerated `corpus_history.jsonl` and flipped the 09-01 row `docker:31 → 2` — the same diff commit `a7c473c` declined to commit ("a feature-skewed store row — the store owner's machinery to reconcile"). This issue IS that reconciliation.
- Measured: `git show 3839457:src/domains/docker/corpus.rs | grep -c 'rule_id:'` = 2; `grep -c '^        rule('` = 27. At HEAD (`e63f819`+): 2 and 31. Runtime count via `build_docker_seed_corpus().entries.len()` = 31 (docker default-on since 08-28, so every current production binary stamps 31 correctly — only historical rows and the feature-off source-walk path are wrong).
- Store series (2026-09-04 read): 08-28/29/31 + 09-01 all carry `docker:2`; 09-02/09-03 carry 31. All other domains walk correctly because their corpus entries carry per-entry `rule_id:` / `lint_name:` fields — docker is the only corpus using a positional shorthand constructor.
- Layering that preserves the poison: `load_corpus_history` (gen_dashboard.py) merges git-mined (layer 1), store rows (layer 2), today (layer 3) with **store winning over mined** — so the git-mined layer, which counts text correctly-per-needle and would say 31 for every day since 08-29, loses to the wrong store rows every regeneration.

## Non-goals

- Not a 065 T6 problem: T6's budget/timeout/merge fix is orthogonal and unaffected; the poisoned series deploys in the dashboard either way.
- Not restructuring the docker corpus to satisfy the needle — the shorthand constructor is good design; the needle is the wrong end to preserve.
