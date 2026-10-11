# Research 184 — Batch 157: the frontier-lead mining pack III (distillation record)

**Status:** RECORD (batch landed 2026-09-18, Plan 148)

Source: the Batch 156 re-arm's committed leads (Issue 122 / Plan 145 /
Research 182's designed flywheel, third turn). Not an external-source
distillation — a consumption record for the two leads, plus the re-arm
probe matrix that selected their successors.

## The two rules (both FIXABLE — the first all-FIXABLE pack)

| lint | rewrite | tier | decision content |
|---|---|---|---|
| `redundant_closure_for_method_calls` | `v.iter().map(\|s\| s.len())` → `v.iter().map(String::len)` | FIXABLE | The rewrite needs the closure param's type — undecidable parse-only in general. Solved by SIGNATURE DERIVATION: the binding is a fn/impl-method param typed `&[T]`/`&[T; N]`/`&Vec<T>` (shared ref), proven never-rebound in the body (`helpers::slice_param_heads_over`: any same-name PatIdent/assignment anywhere in the fn refuses — position-blind, the conservative direction), `.iter()`-receiver chain, single-param closure whose body is exactly `p.m()` (0 args). The emit `T::m` is restricted to a CONSTRUCTION-IDENTITY WHITELIST — String/Vec/HashMap/HashSet × len/is_empty/capacity, all inherent shared-`&self` 0-arg std methods — because FIXABLE demands identical-for-every-compiling-input, not merely error-visible on wrong ones: a user type's `m` could live on a trait or on `&T` itself (`impl Trait for &T`), where `LocalType::m` is a DIFFERENT call that still compiles (the silent-FP class). Shadow guard on the four head names (the io_other_error pattern, factored into `helpers::file_shadows` at this batch); generic type params named like the head refuse; pedantic-group (the bench oracle runs pedantic — no blind spot). |
| `iter_skip_next` | `v.iter().skip(n).next()` → `v.get(n)` (proven slice/Vec/array binding) · `X.skip(A).next()` → `X.nth(A)` (everything else) | FIXABLE | TWO arms, ladder-ordered. Arm 1 is ONE STEP BETTER than clippy's own fix (measured, the Batch 156 round-3 probe: `.nth(n)` fires `iter_nth` on slice iters — the oracle-clean inverse is `.get(n)`; `get(i)` ≡ `iter().skip(i).next()` exactly). Arm 2 is clippy's own suggestion, identical for every iterator (`Iterator::nth(n)` IS `skip(n).next()`); the parse-only method-name match is the `iter_nth_zero`/`filter_next` corpus precedent. Unproven-but-actually-slice receivers heal to `.nth(n)` — the documented divergence (arm 1 narrows, never widens). Skip-arg text rides verbatim; comments in the two deleted ranges refuse. |

## The re-arm probe matrix (1 round, `scripts/plan148_frontier_probe.py`)

Accepted: **`to_string_in_format_args`** (the Batch 156 verified spare,
re-verified: seed MUST be the inlined `format!("n={x}")` form — the
`format!("n={}", x)` inverse is dirty via `uninlined_format_args`, which
skips method-call args on both clippy's side and ours) +
**`manual_range_contains`** (Bench 094 wave-2 evidence, 121 warnings / 5
crates; seed dodges const-ability via `Vec::push` — `&mut` params no longer
dodge `missing_const_for_fn` since const_mut_refs stabilized). Verified
spare: `single_char_add_str` (seed clean, exact-fire).

Rejects (measured): `manual_range_contains`'s iterator form
(`.iter().filter(|x| **x >= 3 && **x <= 5)`) SILENT — the double-deref
shape does not fire; `iter_next_slice` SILENT on the
`format!("{:?}", v.iter().next())` shape (the format context or the lint's
receiver rules suppress it).

## Measurements

- Corpus 70 → 72; FIXABLE 51 → 53; VERIFY 16 (unchanged); atoms 51 → 53
  (both leads FIXABLE — every lead joins the atom set for the first time).
- **The new rcmfc atom is BROAD as an ORACLE LINT**: clippy's own surface
  fires on `unnecessary_sort_by`'s fixture (`v.sort_by(|a, b| a.cmp(b))` —
  a 2-param method-forwarding closure OUR proposer rightly refuses) —
  `unnecessary_sort_by` joined the bad-atom set: v2 bad 5→6, v3 6→7.
  Recorded cost: the ladder narrows (v2 75 labeled / 20 excluded vs
  73/14; v3 76/23 vs 77/14). Truthful, not fought.
- Freeze regen (real oracle): v2 e2e 30.1 s — 75 verified / 6 newly
  excluded / 0 drift, solve 100.0% at every k (47/47, 26/26, 30/30, 20/20),
  M4 PASS; v3 e2e 27.3 s — 76 verified / 9 newly excluded, solve
  94.0/92.3/95.8/100.0% (reported, the pre-registered v3 posture),
  stratification PASS. **12 orphaned EXCLUDED rows pruned** (old-layout
  windows the 53-atom builder no longer produces — the e2e's printed
  record carries the pre-excluded set blindly; the gen-2/gen-3 score-bench
  lanes' orphan check caught them, `freeze_status_for` refused until
  pruned). Process note for the next re-freeze: paste the record, then
  RUN the generation lane before claiming green.
- Frontier e2e: VERDICT ✓ ×2 (graduation + verify-pass); 9 committed rows
  (7 heldout + 2 frontier), 0 drift; 7 family loops — the first packs'
  5+1+2 accumulation; 4 fresh rows appended, 2 consumed frontier rows
  archived.
- score-bench row #132 (gen 1): heal_rate **96.36% FLAT** vs baseline
  (vocab 51→53, edits 69, parse_safety 1.0, strata repo 75.0% / authored
  98.0%); a gen-3 lane row appended at 95.0% (the orphan-check run — an
  honest measurement, history 133 runs). Drift warnings: the 2 known
  pre-existing v1-frozen quantile fixtures (doc_markdown under the current
  toolchain, unrelated lane).
- Gates: lib default **1829/0** (+32 tests) · lib `latent_retrieval`
  **1982/0** · lib `frontier_miner` **2146/0** (retrieval floors held) ·
  clippy `-D` clean in 3 states. Pre-existing, not fixed (unrelated lane):
  `src/bin/kat_info/mod.rs` carries 2 dead-fn warnings under the
  `score_bench,kat_service` bin combo (the `--info` lane's files).

## Default-eco evidence context (Bench 094)

`manual_range_contains` 121 warnings / 5 crates (wave 2 — the highest-body
re-arm lead yet); `to_string_in_format_args` + `single_char_add_str` ride
the verified-spare lane. The consumed leads were probe-verified (round 3,
Batch 156) rather than histogram rows.
