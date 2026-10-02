# Plan 163 — Batch 171: the coverage-report pack XIV (tabs_in_doc_comments + map_clone)

**Status:** COMPLETE (2026-09-21 — T13/T14 landing at commit time; all
GOAT gates green)

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
- [x] T2 `tabs_in_doc_comments` proposer (comment-text `\t`→4-space edit in
      doc comments: `///`, `//!`, `/** */` — including code blocks; regular
      `//` untouched) + module tests
- [x] T3 `map_clone` proposer (by-ref-adapter-chained `.map(|x| x.clone())`
      → `.cloned()` / `.map(|x| *x)` → `.copied()`; single-param identity
      closures only; into_iter/owned chains refused; nested-expr bodies
      refused — the mc_nested silence) + module tests
- [x] T4 registration: doc-table rows + pub mods + FIXABLE_LINTS 73→75
      (count pin bumped) + dispatch arms + `all_lints_fire_and_parse` rows
- [x] T5 corpus.rs entries ×2 + count test 92→94
- [x] T6 G1: clippy_lints module + corpus tests green — 1183/0
- [x] T7 freeze regen gen2/gen3 — 2-run convergence (the regen's run 1
      caught the map_clone clone-shape co-fire: clone_on_copy + rcmfc fire
      BY NATURE on `|x| x.clone()`; the corpus atom switched to the DEREF
      shape; run 2 clean). Tables: 93/18 v2 + 93/22 v3; e2es v2 93/0/18
      M4 PASS (20.3s) + v3 93/0/22 stratification PASS (19.2s).
- [x] T8 G2: lib default 2137/0 + kernel_opt 2493/0
- [x] T9 retrieval gate: 18/18 floors held
- [x] T10 G3: clippy -D clean BOTH states (healer took 2 doc_markdown;
      4 structural one-liners manual — if_same_then_else/manual_strip ×2/
      while_let_loop, all outside the compiled fix set today)
- [x] T11 score-bench #153 96.4% FLAT vocab 75 + frontier snapshot + the
      Issue-102 cadence row — gen-3 sweep #154 at 66 rules, the THIRD
      full-corpus sweep; the fade section is LIVE (0 candidates over
      36→47→66)
- [x] T12 Research 199 + AGENTS.md counts (92→94, 75→75 fixable) +
      `.distill/001` update
- [x] T13 commit `feat: batch 171: ...` + push — **COMPLETE `f9002260`**
- [x] T14 global binary rebuild + `--corpus-stats` verify — DONE: built
      from `f9002260` in an isolated worktree BESIDE the repo (a Temp-path
      worktree breaks the relative path deps — the beside-the-repo
      placement is load-bearing), release set complete, installed over
      the copy at `~/.cargo/bin/cargo-heal.exe`; verified: clippy_lints
      **94** / kernel_opt 627 / rust_perf 145. Worktree + temp target
      removed.
