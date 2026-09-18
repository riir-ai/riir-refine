# Research 185 — Batch 158: the frontier-lead mining pack IV (distillation record)

**Status:** RECORD (batch landed 2026-09-18, Plan 149)

Source: the Batch 157 re-arm's committed leads (Issue 122 / Plan 145 /
Research 182's designed flywheel, fourth turn). Not an external-source
distillation — a consumption record for the two leads, plus the re-arm
probe matrix that selected their successors.

## The two rules (both FIXABLE — the second all-FIXABLE pack)

| lint | rewrite | tier | decision content |
|---|---|---|---|
| `to_string_in_format_args` | `format!("n={}", x.to_string())` → `format!("n={}", x)` | FIXABLE | The rewrite needs the receiver's `to_string` to be the std Display BLANKET — undecidable parse-only in general. Solved by SIGNATURE DERIVATION (the `slice_param_heads_over` pattern, generalized as `helpers::scalar_param_heads_over`): the receiver is a single-segment ident that is a fn/impl-method param with an EXPLICIT scalar annotation (primitives + `&str`/`String`/`&String`), never rebound in the body. For every such type the ONLY in-scope `to_string` IS the blanket — primitives/`str`/`String` have no inherent `to_string`, and a second in-scope trait method named `to_string` for the same receiver would make the ORIGINAL not compile (E0031 ambiguity) — so every compiling input is safe by construction. The string-LITERAL receiver arm is construction-identical (the `str` blanket). PLACEHOLDER GUARD: fires only where the arg fills a BARE `{}` — a `{:spec}` placeholder changes the trait (Debug String ≠ Debug u32) and refuses; dynamic-spec refs (`{:1$}`/`{:.*}`) abort the whole macro (the `has_dynamic_spec_ref` envelope reused from `uninlined_format_args`, whose family table + placeholder classification + template walk are now shared substrate via `template_and_post_template_args`). Named/assign args abort (the positional mapping must be exact); direct post-template family-macro children are skipped (the `format_in_format_args` boundary, the uninlined precedent). COMPOSITION BY DESIGN: uninlined skips method-call args (real clippy's own envelope), this rule fires exactly there, and uninlined inlines the plain-ident result on the NEXT fixpoint iteration — `format!("n={}", x.to_string())` → `format!("n={}", x)` → `format!("n={x}")`. GROUP CORRECTION: the lint is **perf/default-on** (the plan149 default-groups arm fired it with no `-W` flags) — the old probe comment's "pedantic" was stale. |
| `manual_range_contains` | `x >= 3 && x <= 5` → `(3..=5).contains(&x)` (+ exclusive `x > lo && x < hi` → `(lo..hi)`, + both mirrored orders `lo <= x && hi >= x` / `lo < x && hi > x`) | FIXABLE | `lo <= x && x <= hi` ≡ `(lo..=hi).contains(&x)` for EVERY PartialOrd by std's own `contains` definition (the two leg comparisons exactly — NaN false both sides, ±∞ equal both sides); the exclusive twin identically (`Range::contains` = strictly-less both legs). The COMPILE-DIRECTION hole (custom `E: PartialOrd<L>` mixed types — original compiles, fixed needs `L: PartialOrd<E>`) is closed by the SAME signature derivation: shared operand = a param-proven numeric/char/bool scalar (bare-ident text), bounds = LITERALS only (int/float/char, optional sign) — then both range ends unify with the operand's type and every impl in play is same-type. Shared-operand text identity byte-compared (the `identity_op` precedent). Mixed >=/< pairs, variable bounds, leading-`&`/paren-wrapped operands, `||` chains, and comments in the deleted glue refuse. Documented narrower-than-clippy divergences: variable bounds + non-scalar operands (clippy type-resolves; we derive from the signature). |

## The re-arm probe matrix (1 round, `scripts/plan149_frontier_probe.py`)

Accepted: **`single_char_add_str`** (the Batch 156/157 verified spare,
third turn — `s.push('!')` clean, `s.push_str("!")` fires EXACTLY the
lint; default-level confirmed by the group arm; the `String::push`
interior-mutability dodge keeps the nursery const-ability lint silent) +
**`manual_clamp`** (`x.clamp(3, 5)` clean, `x.max(3).min(5)` fires
EXACTLY; `Vec::push` dodge; the natural sibling of the just-landed
`manual_range_contains`).

Rejects (measured): `unit_arg` — the explicit-`()` shape's seed is DIRTY
(`missing_const_for_fn` — `()` params no longer dodge const-ability) and
the block-expr broken form crossfires `no_effect` + `unit_arg` + the
const rider (3 lints, not exact); the family needs a shape that dodges
both. `from_str_radix_10` — broken carries `missing_const_for_fn` beside
the target; recorded as the SPARE with the `Vec::push` dodge noted for
the next re-arm.

Group pins (the default-groups arms): `to_string_in_format_args` fires
under DEFAULT clippy → **perf/default-on**; `manual_range_contains` →
style/default-on (consistent with Bench 094's default-eco histogram
evidence, 121/5).

## Measurements

- Corpus 72 → 74; FIXABLE 53 → 55; VERIFY 16 (unchanged); atoms 53 → 55
  (both leads FIXABLE — every lead joins the atom set).
- Bad-atom sets UNCHANGED (v2 6, v3 7) — neither new atom is broad as an
  oracle lint (both fire exactly their own families' fixtures; no
  crossfire on other families' fixtures — the rcmfc lesson did not
  repeat).
- Freeze regen (real oracle): v2 e2e — 55 atoms (bad 6), 77 verified /
  17 pre-excluded (0 newly; the splice pruned the old-layout windows
  Batch 157 left), solve 100.0% at every k (49/49, 26/26, 30/30, 20/20),
  M4 PASS; v3 e2e — 59 atoms (bad 7), 78 verified / 18 pre-excluded,
  solve 94.2/92.3/95.8/100.0% (the pre-registered reported-not-asserted
  posture), stratification PASS.
- Frontier e2e: VERDICT ✓ ×2 (graduation + verify-pass). Graduation run:
  24 seeds clean, 9 family loops, 2 fresh heldout admissions (the
  graduated leads), 4 rows appended (2 heldout + 2 frontier — the
  re-arm's leads, 1 per family per pass, K-minimality). Verify-pass: 11
  committed rows verified, 0 drift, no new rows. The 2 consumed frontier
  rows were ARCHIVED (the KnownSet contract: "a mining batch that lands
  the rule re-opens the family by archiving the row") — heldout 7→9
  families / 16→20 cases; frontier 4→4 (2 new families × 2).
- score-bench row #133 (gen 1): heal_rate **96.36% byte-identical FLAT**
  vs baseline (vocab 53→55, edits 69, parse_safety 1.0, strata repo
  75.0% / authored 98.0%); frontier snapshot appended (generation 7);
  the 2 known pre-existing v1-frozen quantile drift warnings stand
  (unrelated lane).
- Gates: lib default **1865/0** (+36 tests; 1868 post-rebase with the
  sibling's TUNA +3) · lib `latent_retrieval` **2018/0** · lib
  `frontier_miner` **2182/0** (retrieval floors HELD) · freeze-status
  7/7 · clippy `-D` clean in 4 states (default --all-targets /
  latent_retrieval --lib / frontier example / score_bench lib). En-route
  lint repairs in the new modules (5 findings: similar-names, doc
  backticks ×2, let-else, lifetime elision).

## Evidence context (Bench 094 + probes)

Both consumed leads were probe-verified (round 1, Batch 157) rather than
histogram rows; `manual_range_contains` carried the strongest default-eco
evidence of any frontier lead so far (121 warnings / 5 crates, wave 2).
`to_string_in_format_args` rides the perf group (default-on) — the group
pin corrected a stale probe comment this batch.
