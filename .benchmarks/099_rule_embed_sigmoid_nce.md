# Bench 099 — Issue 126 lane (a): sigmoid-NCE two-tower span↔rule embeddings (GATED PASS)

**Status:** COMPLETE 2026-09-20 — verdict **GATED PASS at the PoC gates** (G2 full-rig +4 top-1 with G1 zero regressions, G2-OOD no harm, G3 clean FP unchanged). Shipped as the opt-in `rule_embed_poc` feature + the BLAKE3-sealed frozen artifact (`assets/rule_embed_frozen_v1.bin`); the shipped suggest lane is UNCHANGED — the posture attaches only when a caller builds a matcher with `with_rule_embed` (the standing gate test drives it). Promotion to a production path is a separate owner decision (the choice_scorer/Bench-098 posture: measurement lands first, wiring follows gates + need).

**Source:** [Issue 126](../.issues/126_sigmoid_nce_rule_embeddings_hindsight.md) · [riir-ai Research 386](../../riir-ai/.research/386_Contrastive_RL_Goal_Conditioned_Value.md) (CRL critic reframe; arXiv:2206.07568) · Bench 048 (sigmoid-vs-InfoNCE dimension law)

## What ran

1. **Model** (`src/draft/rule_embed.rs`): two-tower bilinear — φ(span) = W_s·x, ψ(rule) = W_r·y over the SHARED hashed-bag law (`span_embed::fill_hashbag`, 4096 buckets, signed unigram+bigram, L2-normalized; the Issue-126 refactor of `span_embed` is byte-identity-pinned by test), d=64 (the paper's Table 2 pin), loss = sigmoid-NCE with in-batch negatives (same-rule pairs skipped as negatives), τ = 0.1 SHARED between training and runtime scoring. Hand-rolled f32 forward/backward + local flat AdamW (schedule via `choice_scorer::AdamW::lr_at`), 3000 full-batch steps, seeded SplitMix64 — deterministic per machine; the ARTIFACT is committed so nothing downstream retrains. 7/7 module tests: the finite-difference gradient gate (bucket-derived coordinate sample + inactive-bucket exact-zero pin), artifact roundtrip + tamper/truncation/bad-magic refusal, withheld-pair refusal, σ(0)=0.5 identity, toy separation WITH a magnitude floor.
2. **Data** (the machine-checked supervision; NO organic fixseq rings exist workspace-wide — measured): 270 pairs = 70 TRAIN-split labeled fixtures + 200 corpus self-pairs (non-withheld rules' buggy text); ψ rows for the WHOLE 268-rule book from buggy+fixed text. **Rule-level split** (blake3(id)[0] ≡ 0 mod 4, exactly 25%): withheld rules' fixtures AND self-pairs stay out of training — rule-level, not fixture-level, because perf/kernel fixtures are authored variants of corpus shapes and a withheld rule's self-pair would leak the association the OOD gate measures.
3. **Artifact**: per-matrix absmax → ×127 i8, versioned wire format, BLAKE3-sealed (hash over the full payload), 287,883 bytes; sha256 `15a727ea03309fa72c1394ba5a3edc3f578863c3f49962c1a2f1e3b3c5aca5a5`; loaded via `embedded_tables()` (`include_bytes!` + `decode_verified` on first use; a corrupt artifact panics at startup, never scores).
4. **Measurement** (`run_split_eval` in retrieval_eval.rs): baseline (hashed BM25+latent fusion + `RerankMode::Structural`) vs baseline ⊕ w·σ(φᵀψ/τ) — additive on retrieved candidates, standalone for rules retrieval missed (the rescue path), per-rule fallback to baseline for uncovered rules. Both postures over the SAME 101 labeled spans (70 train-split / 31 withheld) + 12 clean spans.

## Numbers (w=16, the train-split sweep's argmax, ties → smaller)

| arm | train (n=70) | withheld (n=31) | full rig | clean FP top-1 |
|---|---|---|---|---|
| baseline (shipped posture) | 63 | 31 | 94/101 = 93.1% | 26 |
| trained @ w=16 | **67** | 31 | **98/101 = 97.0%** | 26 |

Sweep (train-split only; the withheld split is never swept on): w ∈ {0.25…8} all 63/70 — the trained term rides below the BM25 margins until w=8 (64/70); w=16 → 67/70.

- **G2 PASS: +4 top-1 (94 → 98), zero losses.** The rescue anatomy is the two failure classes the fusion cannot reach: `useless_conversion` (b=1→0), `single_char_pattern` (b=5→0), `comparison_to_empty` (b=7→0), `box-str-immutable` (b=1→0) — outranked-baseline misses the trained term overcame — and `needless_borrow` (**b=None → 6**: a rule BM25 retrieval missed ENTIRELY, recovered by the standalone tower alone). 3 remain missed (`redundant_closure` still unretrieved — its gold rule carries no ψ-row signal for that variant, `needless_return` gold σ=0.0015, `dyn-to-enum` rank 3 unchanged).
- **G1 PASS: 0 regressions** — all 94 baseline wins held.
- **G2-OOD PASS: withheld 31 = 31.** ⚠ The withheld split has ZERO HEADROOM (baseline already perfect) — it is the anti-memorization read, not a win denominator; a win gate placed on it can never pass (that gate-design error was made and corrected in-flight, recorded below).
- **G3 PASS: clean FP top-1 26 = 26**, top-1 identity changes: none.

Gate test: `tests/rule_embed_gate.rs` — count-pinned floors (baseline 63/31/26 AND trained 67/31/26, the G2 win-floor as the sum, G1 wall at 0), fires downward on trainer/corpus drift; a baseline shift re-pins beside it.

## The in-flight defect the gates caught (recorded — it is the issue-126 method working)

1. **Dequant convention bug (caught by the self-pair probe, not by the first eval):** `quantize` uses `scale = absmax/127` but the first `dequant` divided by 127 again — every decoded weight shipped **127× small**, both towers' dots collapsed to ≈6×10⁻⁵, and ALL scores read the σ(0)=0.5 null while final training loss was 0.0288. The first eval read trained ≡ baseline at every weight — an honest-looking all-tie that was actually a broken artifact. The `tiny_training_separates_positives` test had PASSED through the bug (`hit > miss` is satisfied by ±1e-4 noise) — it now carries a **magnitude floor** (σ > 0.75 on a trained positive), the lesson: an ordering assertion does not pin a scale. The self-pair probe (exact training text through the artifact) is the discriminator between "artifact broke the weights" and "no generalization" and stays in the example (`--scores`).
2. **G2 denominator error (caught by arithmetic, fixed before landing):** the first gate placed the win on the withheld split — where the baseline is already 31/31. A win gate on a zero-headroom split is unsatisfiable by construction (guaranteed-fail = the cries-wolf shape). Corrected: G2 reads the full rig (the committed axis, the Issue-121 +55.6pp precedent); the withheld split is the no-harm read.

## Prior-art honesty (per the issue)

ContraCode / DPR / CodeContrast / ContrastRepair publish contrastive code embeddings — this is an APPLICATION on machine-checked repair outcomes (gold labels no external corpus carries), not a new class. The trained towers also ride the house sigmoid rule (no softmax anywhere) and Bench 048's dimension law.

## Verdict + what would move it further

**The PoC gates hold**; nothing in the heal path changed. Constraints on the result, stated:
- The 4 wins came from the TRAIN split (the withheld split had no misses to win). The wins are not memorization-flavored — `needless_borrow`'s rescue is a retrieval gap, and the fixture spans are variants, not training texts — but the honest generalization population (withheld misses) is EMPTY at the current corpus state. The split's discriminating power grows as the corpus adds misses off-distribution.
- w=16 is a large additive weight against fused BM25 scores; it was pinned by the train-split sweep, and G3 confirms no FP cost — but a production wiring (if ever) should re-sweep at its own pool shapes.
- Data scale remains the binding constraint (270 pairs vs the paper's regime). Reopen-to-promote trigger: organic fixseq rings at fleet scale (the issue's original data source — none exist today, measured) or oracle-anchored real-repo spans in the `headroom_probe` class; re-run `--train --eval` unchanged and re-pin the gate.

Reproduce: `cargo run --release --features rule_embed_poc --example rule_embed_poc -- --train` (~8 s) then `--eval /tmp/rule_embed_frozen.bin` (~35 s, 8 weights × 2 postures); M3 Max, release, isolated CARGO_TARGET_DIR, no GPU contention (CPU-only run).
