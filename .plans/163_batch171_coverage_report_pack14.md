# Plan 163 — Batch 171: the coverage-report pack XIV (tabs_in_doc_comments + map_clone)

**Status:** IN PROGRESS (2026-09-21 — T0/T1 done; the probe-first law rejected
TWO staged leads en route and this file records why)

The standing pipeline (Batch 164+): fresh `--coverage-report` derivation →
adjudicate the evidenced head with reasons → probe-verify the leads → land.
The derivation (clippy 0.1.98, 822 lints / 92 covered / 730 uncovered / 417
default-firing) adjudicated down through the Research-191 skip list PLUS this
batch's own negative results:

| candidate | count/spread | verdict |
|---|---|---|
| missing_errors_doc / missing_panics_doc / missing_safety_doc | 90+78+186 | doc-authoring (the 191 class) — REFUSE |
| expect_used / absolute_paths | 34+30 | restriction semantics — REFUSE |
| result_unit_err / too_many_arguments | 28+28 | type invention / architectural (191) — REFUSE |
| needless_borrowed_reference | 27 | VERIFY-tier, probe-silent (plan158) — RETIRE |
| **len_zero** | 20 | **DUPLICATE** — `comparison_to_empty`'s proposer Form 2 already heals `.len() == 0` either operand order (corpus.rs:388 comment + code verified) |
| needless_borrows_for_generic_args | 20 | typeck (the borrow's necessity depends on the callee's expected generic type) — REFUSE |
| double_ended_iterator_last / len_without_is_empty | 17+17 | type-blind / fn-body invention (191) — REFUSE |
| derivable_impls | 16 | semantic-eq proof + multi-part attr edit — DEFER (not this pack) |
| ptr_offset_with_cast | 16 | unsafe pointer arithmetic — REFUSE |
| type_complexity / missing_transmute_annotations / cast_abs_to_unsigned | 15+13+12 | threshold / typeck / typeck — REFUSE |
| manual_div_ceil | 13 | overflow divergence (Batch-168 REJECT, standing) |
| **useless_borrows_in_formatting** | 12 | **REJECTED — MEASURED INERT** in clippy 0.1.98: ~20 shapes probed INCLUDING the lint's own doc example (`let s: &str = "hello"; println!("{}", &s)`) — ZERO firings; the needless_borrowed_reference class (documented-warn, empirically silent) |
| **int_plus_one** | 10/3 | **REJECTED — overflow divergence at the extremes** (y+1 wraps/pansics at MAX while `x > y` does not — the manual_div_ceil class, caught at adjudication this time) |
| get_first | 6/5 | type-blind (`.get(0)` on a keyed map is a KEY lookup, `first()` does not exist) — REFUSE |

The two leads that survived:

| lint | group | evidence | probe (`scripts/plan163_pack14_probe.py`) |
|---|---|---|---|
| `tabs_in_doc_comments` | style/warn | 12 default-eco | FIRES on `///` lines (each `\t` → 4 spaces), `//!` inner, `/** */` block, AND tabs inside fenced code blocks (clippy rewrites code samples too — mirrored verbatim); SILENT on regular `//` comments |
| `map_clone` | perf/warn | 6/5 default-eco | FIRES on `.iter().map(|x| x.clone())` and `.map(|x| *x)` and `.values()` chains; the clone-on-Copy co-fire measured (`clone_on_copy` fires on u8 elements — clippy suggests `copied()` there, we suggest `cloned()` — always-compilable for the shape, the diverge-from-the-literal-suggestion precedent); `into_iter` chains REFUSED in our scope (clippy's empty-replacement suggestion there is Copy-type-dependent — deleting `.map(|x| x.clone())` on an owned-String iterator would change semantics; the documented narrower-than-clippy divergence, v1 = by-ref adapters only) |

## Tasks

- [x] T0 fresh `--coverage-report` derivation + adjudication (the table above)
- [x] T1 probe `scripts/plan163_pack14_probe.py` — PASS for both leads (with
      the two en-route REJECTS recorded in its header)
- [ ] T2 `tabs_in_doc_comments` proposer (comment-text `\t`→4-space edit in
      doc comments: `///`, `//!`, `/** */` — including code blocks; regular
      `//` untouched) + module tests
- [ ] T3 `map_clone` proposer (by-ref-adapter-chained `.map(|x| x.clone())`
      → `.cloned()` / `.map(|x| *x)` → `.copied()`; single-param identity
      closures only; into_iter/owned chains refused; nested-expr bodies
      refused — the mc_nested silence) + module tests
- [ ] T4 registration: doc-table rows + pub mods + FIXABLE_LINTS 73→75
      (count pin bumped) + dispatch arms + `all_lints_fire_and_parse` rows
- [ ] T5 corpus.rs entries ×2 + count test 92→94
- [ ] T6 G1: clippy_lints module + corpus tests green
- [ ] T7 freeze regen gen2/gen3 (the plan-137 T7.5 law — the corpus delta
      moves the stride windows; 3-run convergence)
- [ ] T8 G2: lib default + kernel_heal counts (floors move up, no re-pins
      down)
- [ ] T9 retrieval gate: latent_retrieval suite floors held
- [ ] T10 G3: clippy -D clean BOTH states
- [ ] T11 score-bench + frontier snapshot + the Issue-102 cadence row (the
      deliberate full-corpus sweep — row #3 takes the fade section live)
- [ ] T12 Research 199 + AGENTS.md counts (92→94) + `.distill/001` update
- [ ] T13 commit `feat: batch 171: ...` + push
- [ ] T14 global binary rebuild + `--corpus-stats` verify (clippy_lints 94)
