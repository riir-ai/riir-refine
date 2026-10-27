# Issue 186: structure-grounded synthetic retrieval-eval fixtures (gold from artifact structure, questions from the teacher)

**Status:** Open — poc/eval-harness task, unclaimed
**Deferred (not claimed) 2026-10-27:** the integration surface this issue needs (`src/draft/retrieval_eval.rs`, `src/draft/eval_fixtures_sec.rs`) is dirty with the live issue-185 softmax lane (`zed-m3-i185`); implementing now would edit a sibling's in-flight files. Re-check `git status` before claiming.
**Source:** [riir-neuron-db Research 311](../../riir-neuron-db/.research/311_STAIR_ToC_Structure_Aware_Retrieval.md) (arXiv:2609.03874 STAIR — its SearchTome recipe: teacher writes only questions, gold labels derive from the artifact's own structure, immune to teacher-hallucinated gold)
**Date:** 2026-10-27

## The idea

The healer's retrieval eval (`src/draft/retrieval_eval.rs` + `eval_fixtures_*.rs`) needs (span, rule) pairs where gold = the rule that owns the span. Today fixtures are hand-authored. STAIR's recipe transfers directly: **derive gold from corpus structure, generate only the questions.**

- Gold construction: each corpus rule's own fixtures/doc-comment examples ARE the structural truth — a span extracted from rule R's own material has gold = R, with no teacher opinion in the label.
- Question generation tier 1 (zero GPU): deterministic template mutations of the span (identifier renames, formatting variance, comment stripping) — measures embedding robustness, not teacher style.
- Question generation tier 2 (optional, LLM): teacher paraphrases the query; the **echo gate + leak-scan discipline** (reflex `--corpus-ab` gates; grammar-not-answers doctrine) applies — no test-row ingestion, echo-gated acceptance, one frozen read.
- Feeds the withheld-pair OOD axis (`withheld_pair_ood.rs`): withhold whole RULES (not spans) so the fixture family gets the same OOD axis the retrieval index is judged on.

## Why it matters

Oracle-labeled eval data is the healer's scarcest artifact (Issue 030's relevance gate, `clippy_oracle.rs` live-gap signals). Structure-grounded gold scales fixture count with corpus size at ~zero cost and zero label-hallucination risk — the STAIR recipe's actual transferable content (their LoRA checkpoint is not; see Research 311 §4 for the training-track verdict).

## Gate

New fixtures enter the score-bench population only through the existing eval harness; a tier-2 batch must pass the echo gate or it dies (recorded, like every corpus-ab negative). No serving-path change.
