# Bench 144 — codegram span-shape dispatch landing (issue 216 T1): classifier + annotation table + filter

**Status:** OPEN — T1 LANDED 2026-11-07 (this record) · local gates GREEN (clippy `-D warnings` all-targets at THREE postures: default / `--features span_shape_dispatch` / release-set; tests 5/5) · T4 (G2 cost) QUEUED on a quiet rig, pre-registered protocol verbatim · promotion (T5) waits on the numbers

## What landed

The categorical dispatch mask of the codegram POC (issue 216; the machinery record is Bench 141 T4):

| piece | file | what |
|---|---|---|
| vocabulary | `src/domain.rs` | `SpanShape` (10 shapes, `#[repr(u8)]`) + `SpanShapeSet` (hand-rolled u16 bitflags — no dep, zero-alloc, `Copy`); `ALL` = the fail-open verdict; all `#[cfg(feature = "span_shape_dispatch")]`-gated so no orphan posture |
| annotation table | `src/domains/rust_perf/corpus/mod.rs` | `span_shape_annotations()` — 6 V1 rows, SINGLE-SHAPE rules only; module-level consts + a `static` (the `&'static` slice cannot promote fn-body const-fn calls — E0515, fixed by construction) |
| classifier | `src/draft/latent_matcher.rs` (`pub mod span_shape`) | `classify(code, parsed)` — root-item kinds folded from the Issue-063 shared per-span parse (zero additional parse work under the default Structural rerank posture) + 2 pinned body needles (HotLoop: `for `/`while `/`loop {`; MatchBlock: `match `); fail-open on `None` / parse-fail / unrecognized |
| filter | `src/draft/latent_matcher.rs` (`suggest_all_domains`) | after the Issue-102 active-set retain, BEFORE instantiation marking + rerank + truncate (the active-set semantics: the surviving pool fills top_k); Perf domain only; early-outs on `ALL` (zero work) |
| feature | `Cargo.toml` | `span_shape_dispatch = ["latent_retrieval", "rust_perf"]` — opt-in until G2 passes; the implication set makes it self-contained (a retrieval-lane filter over the perf corpus's table) |

V1 annotations (each verified against its `buggy_code`): `layout-reorder`/`repr-u8-enum`/`box-large-enum-variant` → TypeDecl · `async-future-size` → FnDecl+AsyncFn · `simd-chunk`/`chunks-exact` → FnDecl+HotLoop. Mixed-shape patterns (`usize-index-to-u32`: struct+fn+loop) deliberately NOT annotated — the mask must be provably safe per row.

## The pre-measurement amendments (both BEFORE any bench — the issue 213 pattern)

1. **Classifier design:** the pre-registration's "syn node kind of the enclosing item, derived inside the existing L2 pruner pass" was verified against production code and corrected: the L2 parse lives in the FIX lane, but the DISPATCH site already parses each chunk (the Issue-063 shared instantiation parse) and an `AstChunker` chunk IS an item span — root-item kinds ARE the shape. No enclosing-item line mapping, no span-locations, no new parse. The syn-derived v2 classifier is recorded in issue 216, never silently substituted.
2. **Placement:** the draft→domains import precedent (`instantiation_rerank`/`headroom_probe`) settles the table home beside the corpus it annotates; the vocabulary sits in the neutral `domain.rs`; the classifier nests in `latent_matcher.rs` (no mod-decl churn).

## Local gates (m3, CARGO_TARGET_DIR=/tmp/plan216-gates, dev profile)

| gate | posture | verdict |
|---|---|---|
| `cargo clippy --all-targets -- -D warnings` | default | PASS |
| `cargo clippy --all-targets --features span_shape_dispatch -- -D warnings` | feature | PASS |
| `cargo check --features latent_retrieval,points,net-mainnet,fix_verify,update_check` | release set | PASS |
| `cargo test --features span_shape_dispatch --lib span_shape` | feature | 9 passed / 0 failed (5 mine: classifier pin · fail-open · rule_admits · crosswalk · end-to-end keep-side; 4 pre-existing `eval_span_shape` proposer tests, name overlap only) |
| `cargo test --features span_shape_dispatch --lib path_gate` | feature | 3 passed / 0 failed (the Issue-213 DomainMask machinery unaffected) |
| `rustfmt --edition 2024` | all 3 files | clean |

Clippy lessons en route (all mechanical, fixed in the same commit): `return_self_not_must_use` on the const builder, `doc_markdown` ×5, `doc_lazy_continuation` (a doc line starting `+ loop)` parsed as a list item), `assert_ne!` form, and the green-zero trap's mirror — an impl block NOT cfg-gated while its type was (E0425 at default posture, caught by running the DEFAULT posture, not just the feature posture).

## Byte-identity / safety argument

- **Feature off (default + release set):** every gate point is `#[cfg]`-gated — the compiled surface is byte-identical to pre-landing.
- **Feature on, non-Perf domain:** the filter arm is `domain == DomainId::Perf` only — G3 by construction (path_gate tests re-pin the Issue-213 machinery green).
- **Feature on, Perf, unannotated rule:** `rule_admits` → true — no behavior change.
- **Feature on, Perf, annotated, matching shape:** admits — G1 keep-side proven END-TO-END in-crate (`annotated_rule_survives_on_a_matching_shape_span` through the real `suggest_file` walk on the exact `layout-reorder` pattern).
- **Feature on, Perf, annotated, non-matching shape:** the rule leaves the fan-out — the DESIGNED behavior (drop-side pinned at `rule_admits` level; end-to-end byte-parity = the g5 lane below).
- **No parse available (`RerankMode::Off`):** verdict `ALL` → the retain early-outs — byte-identical.

## T4 — the G2 cost leg (pre-registered protocol, verbatim from issue 216)

`sh scripts/g5_domain_cost.sh --feature span_shape_dispatch --base latent_retrieval --rounds 3` on a QUIET RIG (m3 is BUSY this cycle — spindump — and takes no benchmark-class work). Bar: added ON-OFF scan latency per file, per-file medians, nearest-rank **p50 ≤ 1.0 ms AND p95 ≤ 5.0 ms**, all 3 interleaved rounds (the Issue-191 bar-5 form, shared with Benches 132/137/140/143).

Expected magnitude: the ON arm adds ONE u16 fold over the chunk's root items + one retain scan over the Perf pool per span — the parse both arms already pay; the OFF-vs-ON delta should be noise-level. The counter-press: masked rules LEAVE the pool, so the ON arm's marking/rerank work can only shrink for annotated-rule hits.

**Utility-floor disposition (disclosed, not gated):** `g5_utility_cost.sh`'s three postures are kernel-expert-flavored (flat/narrowed/off) and cannot parameterize this feature — running it would measure a posture without the mask. The floor carries BY CONSTRUCTION: the mask touches only the 6 annotated rust_perf rules; the kernel-expert census (Bench 125/137's 15/15) and every other domain are structurally untouched; the g5 ON run's harness census is the disclosure record. The annotation-priority feedback loop (annotate where fix accuracy is shape-concentrated) remains issue 217 T9(b)'s curve — currently unopened (217 closed as a measurement note; no consumer waiting).

## Coordination (T8, final)

The T8 ordering resolved THIS cycle: 217's T0 band axis landed first but its floor did not fire — 217 closed as a measurement note (`band_ood` removed), so **this classifier is the sole landed shape axis** and no consumer is waiting. The sibling's additive coordination line on issue 216 (2026-11-07) is honored verbatim; nothing binds.

## Records

issue 216 (filed Bench 141 T4; claim zed-native-agent@m3 exp 2026-11-09T12:00Z) · claim commit `8eaf16d42` · issue 213 closed `9cc961530`/Bench 143 (the file-mask generalization, shader_perf promoted) · issue 217 closed-null (band axis) · issue 208/Benches 131–137 (the pregate precedent) · Bench 140 (the G5 protocol) · issue 102 (the active-set retain semantics) · Issue-063 (the shared per-span parse the classifier reuses)

Session: zed-native-agent@m3, epoch 1794… (commit body)
