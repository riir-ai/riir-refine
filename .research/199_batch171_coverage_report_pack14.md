# Research 199 — Batch 171: the coverage-report pack XIV (tabs_in_doc_comments + map_clone)

**Status:** RECORD (batch landed 2026-09-21 — commit recorded in Plan 163)

## What this batch was

The second post-spare-queue re-arm after the corpus's own coverage debt
(Batches 164/167/168/169 the precedents): a fresh `--coverage-report`
derivation (clippy 0.1.98, 822 lints / 92 covered / 730 uncovered / 417
default-firing) adjudicated through the Research-191 skip list PLUS this
batch's own negative results, then probe-verified leads
(`scripts/plan163_pack14_probe.py`), then the full gate walk.

## The adjudication's own findings (the new records)

1. **`len_zero` is a DUPLICATE, not a gap** — `comparison_to_empty`'s
   proposer has carried a Form 2 (`.len() == 0` → `.is_empty()`, either
   operand order, self-recursion-guarded) since its landing; the corpus
   comment documents it. The coverage report diffs LINT NAMES, so the
   healed span reads uncovered — the seam is served.
2. **`useless_borrows_in_formatting` is MEASURED INERT** in this
   toolchain — ~20 shapes probed INCLUDING the lint's own doc example
   (`let s: &str = "hello"; println!("{}", &s);`): ZERO firings. The
   `needless_borrowed_reference` class (documented-warn, empirically
   silent — plan158's REJECT precedent). Its 12 default-eco histogram
   firings are not reproducible on any discoverable shape.
3. **`int_plus_one` caught at ADJUDICATION this time** — the
   manual_div_ceil overflow class (y+1 wraps/pansics at MAX while `x > y`
   does not), rejected before authoring rather than by a probe round.

## The two leads that landed

- **`tabs_in_doc_comments`** (12, style/warn) — comment bytes carry zero
  semantics; each `\t` → four spaces, clippy's own per-occurrence
  suggestion. The probe pinned the territory WIDER than intuition: `//!`
  inner docs, `/** */` block docs, and tabs inside FENCED CODE SAMPLES all
  fire (clippy rewrites code samples verbatim — mirrored); regular `//`
  and `////` stay silent. Doc-attrs only (item/impl/trait level + the
  File's own attrs) — string literals invisible by construction; the
  `#[doc = "..."]` escape-encoded attribute form refused (v1 divergence).
- **`map_clone`** (6/5, perf/warn) — the shape's own compilation PROVES
  the adapter's bound: `x.clone()` on `&T` compiles only for T: Clone
  (= `cloned()`), `*x` through a shared ref only for T: Copy (=
  `copied()`) — the adapter choice is TYPE-INDEPENDENT given the shape.
  Guards: by-ref adapter chains only (iter/values/keys, probe-verified;
  into_iter refused — clippy's EMPTY suggestion there is a Copy-only
  map-deletion that would erase a real clone on owned-String chains, the
  documented narrower-than-clippy divergence); nested bodies refused
  (probe SILENT, mirrored); clippy prefers `copied()` for Copy
  clone-shapes — we emit `cloned()`, always-compilable and
  semantics-identical (the diverge-from-the-literal-suggestion precedent).

## The freeze regen's catch (the Batch-169 lesson replayed)

Run 1 flagged `map_clone` as a BAD ATOM: the clone-shape corpus fixture
co-fires `clone_on_copy` (Copy elements) AND
`redundant_closure_for_method_calls` (a `|x| x.clone()` closure IS a
method-forwarding closure — rcmfc fires BY NATURE, not by fixture
accident). The corpus atom switched to the DEREF shape (`|x| *x` →
`.copied()`, probe-clean), which fires exactly map_clone; the clone arm
lives in the module tests. Run 2 converged: **93 v2 labels / 18
exclusions, 93 v3 labels / 22 exclusions** (bad sets back at the known
9/10); e2es v2 93/0/18 M4 PASS (20.3s) + v3 93/0/22 stratification PASS
(19.2s).

## The Issue-102 cadence row (the deliberate sweep)

The gen-3 full-corpus sweep ran as part of the batch: row #154 with **66
attributed rules** (36 → 47 → 66 across the three sweeps) — the THIRD
full-corpus sweep row, which takes the contribution-fade section LIVE
(population floor ≥3 met). First live verdict: **no FADED candidates**
over the three sweeps — every ≥3-sweep rule either still resolves or
never contributed. The section self-winds at each corpus-generation
landing (the Q3 cadence, adopted).

## Gates

corpus 92→94, FIXABLE 73→75, atoms 91→93; clippy_lints module 1183/0;
lib default **2137/0** + kernel_heal **2493/0**; retrieval 18/18 floors
HELD; clippy -D clean ×2 states (the healer took the 2 doc_markdown
mechanicals; 4 structural one-liners manual — if_same_then_else ×1,
manual_strip ×2, while_let_loop ×1, all outside the compiled fix set
today); score-bench #153 **96.4% FLAT** vocab 75 + gen-3 sweep #154 +
frontier snapshot appended (gen 7, unchanged state — not re-appended);
fade section LIVE with 0 candidates.
