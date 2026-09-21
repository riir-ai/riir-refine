# Research 197 — Batch 169: the coverage-report pack XIII (the staged alternates)

**Status:** RECORD (Batch 169 landed 2026-09-21 as Plan 160; corpus 90 → 92, FIXABLE 71 → 73)

## Source

The Batch-168 staging (Plan 159) designated its two probe-verified alternates
as pack XIII's leads; the default-eco wave-5 derivation was the fallback and
was NOT owed (wave-4's remainder is consumed-or-single-crate-anecdote). This
batch's own probe (`scripts/plan160_pack13_probe.py`, 2026-09-21,
`rust-clippy 1.93.0` ambient / repo pin 1.98.1 — the shapes are old stable
lints, behavior identical) extended the envelope before any proposer was
authored (the probe-first law).

## The measured envelope (one clippy run per fixture — exact attribution)

`match_ref_pats` (style, joined count 17):

| shape | fires | note |
|---|---|---|
| all arms `&`-led, binding-free | `match_ref_pats` ONLY | the clean-atom spelling |
| `&Some(ref x)` bindings | + co-fires `needless_borrowed_reference` | dirty — and the rewrite changes binding modes |
| `&_ => ..` / guards | fires | `&_` IS a `&`-pattern; suggestion `&_` → `_`; guard rides |
| mixed arm / bare `_` arm / or-pattern arm | **SILENT** | mirrored as proposer refusals |
| suggestion | `match *f { A, B }` | deref scrutinee + strip each `&`; MachineApplicable |

`into_iter_on_ref` (style, joined count 14):

| shape | fires | note |
|---|---|---|
| `(&v).into_iter().next()` | `into_iter_on_ref` ONLY | the clean-atom spelling |
| `(&mut v).into_iter()` | fires → `iter_mut` | |
| `.count()` tails | + co-fires `iter_count` | the gen-8 shaped-verdict class (born lint, priced) |
| `for x in (&v).into_iter()` | fires | but the rename births `explicit_iter_loop` → we REFUSE |
| `r.into_iter()` (r: &Vec) | fires | typeck shape — out of parse reach, refused (staged scope) |
| suggestion | method rename only: `iter`/`iter_mut` | receiver parens KEPT — "call directly: `iter`" |

## The rules

- **`match_ref_pats`** — FIXABLE by construction identity: every arm
  top-level `&`-pattern, BINDING-FREE sub-patterns (the staged binding-mode
  hazard), no nested `&&` (our own fixpoint would re-fire), plain-ident
  scrutinee. The rewrite: insert `*` before the scrutinee + delete each
  arm's `&`-prefix.
- **`into_iter_on_ref`** — FIXABLE by the std DELEGATION LAW (`impl
  IntoIterator for &T` IS `self.iter()` for Vec/[T;N]/HashMap/HashSet/
  BTreeMap/BTreeSet/VecDeque/BinaryHeap) + the `param_heads_over`
  signature-provenance family (params-only v1, `body_rebinds` now
  `pub(crate)`, never-rebound guard, closures refuse — shadowing).
  Method-ident rename only. For-heads refuse. Distinct-from rows vs
  `iter_nth_zero`/`iter_next_loop`/`needless_borrowed_reference` in-rule.

## GOAT

- G1: clippy_lints module 1161/0 (was 1154 + the new 25 tests).
- G2: lib default **2115/0** (B168: 2083) + kernel_heal **2468/0** (2436).
- G3: clippy `-D` clean BOTH states (default --all-targets + all-features
  --all-targets).
- Retrieval: latent_retrieval suite 17/17 (floors held) + full
  latent_retrieval,score_bench lib **2428/0**.
- Freeze: regen 3-run convergence; tables 91/18 v2 + 91/22 v3; BOTH new
  atoms CLEAN; bad atoms UNCHANGED (9 v2 = the 8 known + nwd; 10 v3 +
  persist_array_default); e2es v2 **91 verified / 0 newly / 18 pre-excluded,
  M4 PASS** (31.3 s) + v3 **91/0/22, stratification PASS** (23.3 s);
  generations module invariants 35/0.
- Score-bench #149/#150 **96.4% FLAT** (0.96363634, byte-identical to
  #148), created 1.8% unchanged, vocab_size **73**; the 2 drift warnings
  are the pre-existing toolchain pair (quantile_balance_router
  doc_markdown, fresh-wins, B167-recorded). Frontier snapshot appended
  (store `865ae1da`, generation 7, population 17, band 1 — this box's
  store; the per-store clock, post the 09-21 discriminator correction).

## The batch's findings (the knowledge half)

1. **The E0515 fixture**: the first `into_iter_on_ref` corpus spelling
   returned `Option<&u64>` borrowing a fn param — a NON-COMPILING atom.
   The regen's run-1 bad-atom list caught it (the atom test demands
   compiles); repaired to `.next().copied()`. The corpus fixture authoring
   rule: a fixture that borrows a param must not RETURN the borrow.
2. **The CASCADE mirror**: the v3 freeze e2e silently excludes composites
   of unverified atoms (no printed reason — the branch has no println). A
   regen example that derives tables from oracle truth alone produces a
   table the e2e reads as "1 newly excluded" with nothing to grep. The
   regen must mirror the cascade. Recorded in the generations.rs regen
   paragraph.
3. **Probe-harness traps (twice)**: `write_text` without
   `encoding="utf-8"` wrote the probe crate's lib.rs under the cp874
   locale — one em dash became an invalid byte and rustc reported "stream
   did not contain valid UTF-8" (the katgpt-rs `locale_io_gate` class,
   reproduced here); span-text attribution fails (fn names sit OUTSIDE
   diagnostic spans) — one clippy run per fixture is the exact-by-
   construction shape.
4. **Pre-existing-at-HEAD gating one-liners taken in passing** (the B168
   doc_markdown precedent): the sibling 56f0b0fd fade-self-test
   `bool_to_int_with_if` + `match_same_arms` + `heal.rs:5780` bare-fn
   E0631 on the all-features bin lane — the last one had a comment
   describing the exact closure fix that was never applied.

## Next pool

Unchanged from Batch 168's pointer: the walk #10 close-out delta check
AFTER the Mon 09-21 20:00 ET announce (≈ Tue 07:00 +07) folds into Research
192; walk #11 opens Mon 09-28; the nnethercote trigger stays checked per
idle pass; the next clippy pack rides the NEXT coverage-report derivation
(the head's remainder is the Research 191 skip list) or a default-eco
wave-5 if pack XIV ever needs it.


## Follow-through (2026-09-21, later the same morning): the born-lint close

The landing's `into_iter_on_ref` edit was the METHOD RENAME ONLY — the
literal upstream suggestion, parens kept (`(&v).iter()`). A second,
independent probe of the same pack (the duplicate batch's, see below)
MEASURED that form co-firing `needless_borrow` on every fixed site: the
parenthesized receiver borrow is redundant (autoref supplies it), and
`needless_borrow` is warn-by-default style — every plain `--fix` left a
NEW warning behind.

**Why every landing gate missed it, by construction:** the score-bench
created axis and the freeze e2e are FIXABLE-vocabulary-filtered
(`needless_borrow` is VERIFY-tier — invisible to both), and the probe
measures clippy on BUGGY fixtures, never the fixer's output. The probe's
tracked-suggestion list named `needless_borrowed_reference` — a DIFFERENT
lint (allow-by-default restriction) one character away — the near-miss-
name class.

**The fix** (landed as the follow-through commit): the edit SPLICES —
`[receiver.start, method.end)` → `v.iter` — dropping the now-redundant
borrow with the parens (the `string_lit_as_bytes` /
`redundant_closure_for_method_calls` diverge-from-the-literal-suggestion
precedent). The provenance guard makes the splice trivially postfix-safe:
the receiver inner is always a proven plain ident. Corpus `fixed_code` and
the dispatch fixture marker moved to the spliced form; module tests assert
the parens are GONE.

## The TWIN record

Two sessions landed the SAME Batch 169 (same lint pair, same plan/research
numbers) the same morning: this one (pushed `6d32c3b1`, 10:39 +07) and a
duplicate (local commit, discarded — origin is canonical, never
force-pushed). The duplicate's independent probe is what measured the
born lint above; its G2-latency A/B datum is recorded here as the
surviving content: under the sibling shader agent's sustained bevy
`--workspace --doc` compile (loadavg 22-35),
`latent_suggest_goat::g2_latency_under_1ms_per_chunk` (release-only)
measures 1217-3512 µs/chunk at this batch AND **11526 µs/chunk at the
Batch-168 parent `e49e621d`** (where it passed at 07:27 on a quiet box) —
a box-state artifact, not a batch regression; re-adjudicate on a quiet
box.
