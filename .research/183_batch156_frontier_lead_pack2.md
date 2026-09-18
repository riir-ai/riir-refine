# Research 183 — Batch 156: the frontier-lead mining pack II (distillation record)

**Status:** RECORD (batch landed 2026-09-18, Plan 147 — closed same day)

Source: the frontier miner's re-armed leads (Issue 122 / Plan 145 /
Research 182's designed flywheel, second turn). Not an external-source
distillation — a consumption record for the two committed mining leads,
plus the re-arm probe matrix that selected their successors.

## The two rules

| lint | rewrite | tier | decision content |
|---|---|---|---|
| `io_other_error` | `io::Error::new(io::ErrorKind::Other, e)` → `io::Error::other(e)` | FIXABLE | `Error::new` and `Error::other` share the IDENTICAL `E: Into<Box<dyn Error + Send + Sync>>` bound — the splice is inference-identical whenever the kind is the literal `ErrorKind::Other` path. The shadow hazard (path resolution prefers in-scope items, and the rewrite COMPILES under shadowing) is closed with the `legacy_numeric_constants` file-level guard: items or non-std use-aliases named `Error`/`ErrorKind`/`io` refuse the whole file; `use std::…` re-openings exempt. MSRV floor 1.74, documented not gated (the workspace floats 1.98.1). |
| `wrong_self_convention` | `fn into_*(&self)` → `fn into_*(self)` | VERIFY | The receiver flip's failure classes (E0515 body-borrow, E0308/E0382 caller-borrow) are all check-visible → the compile gate reverts (the `ptr_arg` pattern). Conservative v1 scope: inherent impls, `into_` prefix, no-vis methods (clippy's `avoid_breaking_exported_api` default skips exported items — and the API break compiles, so the gate is blind to it: refused up front, documented divergence). `&'a self` rides the lifetime in `reference`, NOT in `colon_token` — a syn detail the own test suite caught. |

The `to_*`/`is_*`/`from_*` convention rows are deliberately out of scope:
`to_*(self)` fires only on non-Copy types (typeck-driven). A parse-only
proposer would fire on Copy types where clippy is silent AND the rewrite
compiles — a silent false positive, the worst class (the compile gate
exists to catch wrong rewrites, not wrong fires).

## The re-arm probe matrix (3 rounds, `scripts/plan147_frontier_probe.py`)

Measured rejects — each a probe verdict, not a guess:

- `double_ended_iterator_last`: `v.iter().last()` on a slice iter fires
  NOTHING under all+pedantic+nursery (clippy 1.98) — refuted.
- `match_ref_pats`: both the wildcard form and the all-ref-binding form
  silent (the binding form's seed fires `match_single_binding` instead) —
  refuted as a frontier target on this toolchain.
- `manual_strip`: `if s.starts_with(p) { &s[p.len()..] } else { s }` silent
  — refuted on this shape.
- `redundant_closure_for_method_calls` v1 (`.map(|s| s.to_string())`):
  co-fires `implicit_clone` — dirty fixture; v2 with `String::len` is clean.
- `obfuscated_if_else`: seed dirty (`needless_pass_by_value` on `Vec<bool>`).
- `len_zero` / `iter_count`: clean forms are CONST-ABLE → nursery
  `missing_const_for_fn` dirties the seed. Notable: `&mut` params no longer
  block const-ability (const_mut_refs stabilized) — the old "add a &mut
  param" trick is dead as a seed-cleanliness dodge.
- `iter_skip_next` seed subtlety: clippy's OWN suggested fix (`.nth(n)`)
  fires `iter_nth` on slice iters — the oracle-clean inverse is `v.get(n)`,
  one step further than clippy's fix. Recorded for the future proposer
  author: the emit target should be `.get(n)` on slices, not `.nth(n)`.

Accepted: `redundant_closure_for_method_calls` + `iter_skip_next` (both
seed-clean + exact-fire). Verified spare for the next re-arm:
`to_string_in_format_args` (`format!("n={}", x.to_string())` fires exactly
it; seed `format!("n={x}")` clean).

## Measurements

- Corpus 68 → 70; FIXABLE 50 → 51; VERIFY 15 → 16; atoms 50 → 51
  (vocabulary is FIXABLE-only).
- Freeze regen: bad-atom set UNCHANGED at 5 (`io_other_error` fires exactly
  its own lint — the frontier row's oracle verification held under the
  freeze oracle too). v2 73 labeled/14 excluded (e2e: 100% solve at every
  k, M4 PASS, 0 drift); v3 77/14 (92.0/84.6/85.2/80.0% reported,
  stratification PASS).
- Frontier e2e: VERDICT ✓ ×2 (graduation + verify-pass); 7 committed rows
  0 drift; the 5th heldout family loop (`io_other_error`) admitted fresh.
- score-bench row #131: heal_rate 96.36% FLAT vs baseline (vocab 50→51);
  strata repo 75.0% / authored 98.0%.

## Default-eco evidence context (Bench 094 wave 4)

`io_other_error` 57 warnings / 8 crates (the wave's biggest new row);
`wrong_self_convention` 19 / 6. The two consumed leads were the two
highest-evidence staged rows.
