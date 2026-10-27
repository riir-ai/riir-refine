# Research 339 — Issue 186: structure-grounded synthetic retrieval-eval fixtures (the STAIR recipe landed as `draft::eval_synth`)

**Status:** RECORD — landed 2026-10-28 (zed-m3-blaze-run); module `src/draft/eval_synth.rs`, issue closed same day per the noise rule.

## Source

- [riir-neuron-db Research 311](../../riir-neuron-db/.research/311_STAIR_ToC_Structure_Aware_Retrieval.md) — arXiv:2609.03874 (STAIR). The transferable content is the **SearchTome recipe**: teacher writes only questions; gold labels derive from the artifact's own structure — immune to teacher-hallucinated gold. Their LoRA checkpoint is NOT transferable (Research 311 §4's training-track verdict stands).
- Feeds Issue 030's relevance-gate axis (the healer's scarcest artifact is oracle-labeled eval data) + the withheld-pair OOD axis (`withheld_pair_ood.rs`).

## What landed

**Tier 1 (zero GPU, this module):** the corpus IS the artifact — each rule's `buggy_code` is the structural truth, so every span derived from rule R's own material carries gold = R with zero teacher opinion in any label. Three deterministic mutations (the issue's named set, nothing invented): `ident-rename` (every non-keyword identifier gains an `x` prefix — word-boundary-exact, the `mutate_shape` kinship made general), `comment-strip` (`//`-family removed, string-aware), `reindent` (+4 spaces per non-empty line). Identity results are SKIPPED — the corpus text is already an index chunk, an identity query would measure lookup, not retrieval.

**The two consumers:**

1. **`clippy_synth_population()` → `run_withheld_pair`** — the corpus alone puts ONE binding per rule, so the withheld-pair eligibility floor (`|cell| >= 2`) is mostly unmeetable on fixture populations. The variants put rules 2–4 deep under their own shape class. Landing read: population 341 bindings · 112 roles · 8 fillers · 127 cells · **15 ELIGIBLE cells (30 paired queries)** — vs 0 eligible on the corpus-only population (measured by the protocol's own eligibility arithmetic). Arms: ID 100/100/100 → OOD 6.7/33.3/33.3 (paired drop +93.3pp top-1), NOROLE canary 0.0 ✓, atomic null ID 100% → OOD 0% by construction ✓, chance 0.9/2.7/4.5. The OOD floor above chance (6.7 vs 0.9 top-1) with a +93pp paired drop is the composition-gap shape Bench 062 measured on the bigger population — reproduced here on a purely structure-grounded one. Honest caveat: 97 of 112 roles are still shape-collinear (the clippy corpus is one-spans-per-rule), so the 15 eligible cells are the axis's current ceiling for this family; growth comes from multi-shape rules, not from more mutations.
2. **`run_synth_floor()`** — the mutated queries against the standard matcher, the robustness floor the hand-authored fixture sets bound from the other side. Landing read (1005 spans, both default-on domains, shipped posture): **blended 97.7 / 99.9 / 100.0** — ident-rename 96.9 · comment-strip 97.9 · reindent 98.4 top-1. The modelless embedder is naming/layout robust at corpus scale, which is exactly the STAIR premise; the pinned floor is 0.50 top-1 (a collapse floor, not a tuned bar — re-pin only with a recorded re-measure).

## Gates

- 6 always-on tests (`retrieval_synth_*` — named to ride Layer 5b2's `retrieval_` filter): determinism per mutation, rename-invisible-to-the-atomic-key (`normalized_tokens` invariant), string-aware comment strip, reindent totality, gold structural membership over the live corpus, population dedup-safety (all texts distinct — the substrate collapses identical chunks). Lane stays at its 63.7s baseline.
- 2 `#[ignore]`d priced arms (the freeze-table convention): the full synth floor (one two-domain matcher build, ≈+90s wall) and the withheld-pair protocol read — run at corpus landings + re-pins, `--ignored --nocapture`.
- Clippy `-D`-clean at default / `latent_retrieval` / `--all-features` / `--no-default-features` / `--tests`. One shared-const change outside the new module: `withheld_pair_ood::RUST_KEYWORDS` → `pub(crate)` (the rename transform reuses the keyword table — a rename that touched a keyword would change the atomic key).

## Tier 2 — deliberately NOT here

Teacher paraphrase under the echo-gate + leak-scan discipline (grammar-not-answers) is a separate owner-gated lane. The structure-grounded half needs no teacher at all — that is its point.
