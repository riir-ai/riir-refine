# Plan 150 — Batch 159: the frontier-lead mining pack V (2 lints)

**Status:** COMPLETE (2026-09-18) — all tasks `[x]`; closed with the batch
landing. T9's archive step was load-bearing (measured: without it the e2e
reads heldout_ok=false — the KnownSet family-done guard). En-route fallout
repairs: RERANK_POOL 20→24 + the HEADROOM_DEPTHS 28 rung (the P10/P11
ladder, second re-fire — the +2 corpus entries crowded the Issue-024
casualty to depth 21); Batch 158's missing freeze record paragraph
repaired; `.research/.highwater` was stale at 184 (Batch 158 used 185
without bumping).

Source: the Batch 158 re-arm's committed leads (frontier rows in
`.benchmarks/frontier_miner_rows.jsonl`, probe-verified 2026-09-18 by
`scripts/plan149_frontier_probe.py`) — consumed as rules, families
graduated, frontier re-armed with fresh probe-verified leads.

## The leads → rules

| lead | shape | tier | rationale |
|---|---|---|---|
| `single_char_add_str` | `s.push_str("!")` → `s.push('!')` (the single-char string-literal arg becomes the char literal; both append the SAME UTF-8 bytes) | FIXABLE | The compile-direction hole — a CUSTOM type with `push_str(&str)` but a `push` of another signature (or none) — is closed by SIGNATURE DERIVATION (a new third twin: `mut_string_param_heads_over`): the receiver must be a fn/impl-method param annotated `String` or `&mut String`, never rebound (the `body_rebinds` soundness leg). For `String` the only in-scope `push_str` is the inherent `&str` method and `push` the inherent `char` method — `push_str(c)` ≡ `push(c)` for any single-`char` string VALUE by construction (same bytes appended). Arg must be a plain string literal whose VALUE is exactly one `char` (escapes ride via `LitStr::value()`; the EMIT re-escapes `'`/`\`/control chars); multi-char, empty, byte (`b".."`), and C-string literals refuse. Escaping is value-preserving both directions. Documented narrower-than-clippy divergence: local `let mut s = String::new()` receivers and UFCS `String::push_str(&mut s, "..")` refuse pending a local-binding derivation. |
| `manual_clamp` | `x.max(3).min(5)` → `x.clamp(3, 5)` (+ the mirrored `x.min(hi).max(lo)` order twin) | FIXABLE | `min(max(x,lo),hi)` and `max(min(x,hi),lo)` ≡ `clamp(x,lo,hi)` for every TOTAL order when `lo <= hi` — the lattice absorption law; ints + `char` are total orders where `Ord::max/min/clamp` are all defined by `partial_cmp` on the same impl, so value-identity is construction. TWO divergences REFUSE by design: (1) `f32`/`f64` — `NaN.max(3).min(5) == 3.0` but `NaN.clamp(3,5) == NaN` (max/min ignore NaN, clamp propagates) — the anti-regime witness carried in-rule; (2) `lo > hi` — clamp PANICS (`assert!(min <= max)`, release too) while the manual form silently returns `hi` — closed by LITERAL-bounds-only + a decidable `lo <= hi` comparison (int digits as i128 / char as u32). Receiver must be a signature-proven param of an INTEGER or `char` scalar (`scalar_param_heads_over` + a `CLAMP_SCALARS` filter excluding floats/bool). Variable bounds refuse (the `manual_range_contains` divergence precedent). |

Counts: corpus 74 → 76 (2 new entries), FIXABLE 55 → 57, atoms 55 → 57
(both leads FIXABLE — every lead joins the atom set; both were verified
oracle-exact by the plan149 probe).

## The graduation (frontier miner)

- BOTH families flip `covered: true` and join the HeldOut per-family loop
  (10th + 11th family loops — both FIXABLE, so both loop-verifiable).
- The 2 consumed frontier rows are ARCHIVED out of the active store.
- Re-arm the frontier with 2 NEW oracle-verified families (probe-first:
  `scripts/plan150_frontier_probe.py`; recorded spare: `from_str_radix_10`
  with the `Vec::push` const-dodge; candidates `assign_op_pattern`,
  `string_extend`, `double_neg`, the `unit_arg` side-effect-block retry).

## Tasks

- [x] T0 Plan file (this) + `.plans/.highwater` 149→150
- [x] T1 Re-arm probe (`scripts/plan150_frontier_probe.py`): pinned BOTH
       leads' lint groups via default-groups arms (both style/default-on;
       `manual_clamp` had no grp arm in plan149 — the pin was owed) +
       picked the 2 fresh re-arm leads — ACCEPTS `from_str_radix_10` (the
       recorded spare, Vec::push dodge) + `assign_op_pattern`; REJECTS
       `string_extend` (fires nothing on the shape) + `double_neg`
       (nonminimal_bool crossfire); `unit_arg` VERIFIED (side-effect-block
       dodge) — kept as the recorded spare
- [x] T2 Helpers: `mut_string_param_heads_over` + `mut_string_head_of` in
       `helpers.rs` (the third derivation twin — `String` + `&mut String`,
       `body_rebinds` guard)
- [x] T3 `single_char_add_str` proposer + module tests (13 arms: escape
       arms `\n`/`'`/`\\`/`\u{41}`→`'A'`, multi-byte `\u{..}` emit,
       multi-char/empty refusal, byte-string refusal via Lit::Str-only,
       non-proven receiver refusal (custom type / local / field), owned-
       `String` param arm, rebound refusal, shared-ref refusal, paren
       receiver + two-arg refusal, comment-in-glue refusal)
- [x] T4 `manual_clamp` proposer + module tests (12 arms: both chain
       orders, int + char + unsigned receivers, negative + suffixed
       bounds, equal bounds, float refusal — the NaN witness, lo>hi
       panic-guard refusal, variable/unproven bounds refusal, non-ident
       receiver, same-method pair refusal + the three-deep sub-chain
       rewrite, comment-in-glue refusal, impl-method derivation)
- [x] T5 Dispatch rows + FIXABLE 55→57 + tier-count test (len 57)
- [x] T6 Corpus entries ×2 + size pin 74→76 (`corpus_size_is_76`) + header
       doc + `all_lints_fire_and_parse` rows 74→76 + mod.rs doc counts
- [x] T7 Miner graduation: families flip ×2 + 2 new frontier families
       (`from_str_radix_10` + `assign_op_pattern`, probe fixtures verbatim)
       + family_cases + hermetic probe markers (family_probe += 2 arms:
       `.push_str("` / `.max(`; frontier_probe = the new leads'
       markers) + test counts (heldout 20→24 cases / 9→11 families;
       frontier 4→4)
- [x] T8 Freeze regen v2+v3 (real oracle) + orphan prune + record
       paragraph (incl. the Batch-158 missing-paragraph repair). Regen via
       a temporary example (deterministic from specs() + the bad sets;
       spliced by `scripts/plan150_splice_freeze_tables.py` — kept, the
       future-batch path). v2 e2e 78/0-newly/15-pre M4 PASS; v3 80/0/17
       stratification PASS; bad sets unchanged 6/7
- [x] T9 frontier-miner e2e ×2 — VERDICT ✓ (28 seeds clean; 11 family
       loops, 2 fresh heldout admissions; the re-arm's 2 leads admitted;
       4 rows appended); the 2 consumed frontier rows ARCHIVED (required —
       without the archive the verdict reads heldout_ok=false, measured)
- [x] T10 score-bench row #135 — heal_rate 96.4% FLAT (vocab 57) +
       frontier snapshot (generation 7)
- [x] T11 Gates: lib default 1892/0 · lib `latent_retrieval` 2045/0
       (floors held) · lib `frontier_miner` 2209/0 · lib `score_bench`
       2196/0 · clippy `-D` clean in 4 states. En-route: 5 lint findings
       healed/fixed in the new modules; **RERANK_POOL 20→24 +
       HEADROOM_DEPTHS +28 rung + PROD_DEPTH_IDX 3→4** (the
       `pool_depth_rescue_guard` canary fired at casualty depth 21 — the
       P10/P11 ladder re-run, corpus-growth crowding by design)
- [x] T12 Docs: Research note (186), HISTORY batch entry, `.distill/001`
       queue line, AGENTS counts (76/57) + frontier_miner bullet, README
       row; commit + push
- [x] T13 Global fleet binary rebuild (release set, isolated worktree
       beside the repo + `/tmp` target) + `--corpus-stats` verify —
       clippy_lints 76 through the installed copy

## GOAT gates

- G1: module tests + `all_buggy_codes_fire_their_lint` lockstep +
  `all_lints_fire_and_parse` + corpus `all_fixed_code_parses`.
- G2: score-bench row (heal_rate flat vs the 96.36% baseline ± the 2 new
  atoms).
- G3: freeze e2e v2/v3 re-verified green (0 drift); frontier miner e2e
  VERDICT ✓ ×2 (graduation incl. the 2 new family loops + fresh leads).
- G4: no new deps; all edits span-preserving (the toolkit contract).
