# Issue 216 — codegram POC: syntax-shape-keyed span dispatch (the grammar lever beyond issue 213's file mask)

**Status:** OPEN — filed 2026-11-07 (Bench 141 T4); pre-registered BEFORE implementation; builds ON issue 213's `DomainMask` (consume, never fork). **T1 LANDED 2026-11-07** (classifier + annotation table + filter + pins; postures clippy-green at default / feature / release-set; tests: classifier pin + fail-open + rule_admits + crosswalk + end-to-end keep-side — 5/5). **T2/T3 (G1/G3) disposition:** keep-side proven end-to-end in-crate; drop-side pinned at `rule_admits` level; end-to-end byte-parity = the g5 rig lane (T4). **T4 (G2 cost) QUEUED** — the pre-registered protocol: `sh scripts/g5_domain_cost.sh --feature span_shape_dispatch --base latent_retrieval` on a quiet rig; **the utility-floor half is satisfied BY CONSTRUCTION + disclosure**: `g5_utility_cost.sh`'s three postures are kernel-expert-flavored and cannot parameterize this feature, while the mask touches ONLY annotated rust_perf rules — the kernel-expert census (Bench 125/137's 15/15) and every non-Perf domain are structurally untouched; the ON run's harness census is the disclosure record. **T8 ORDERING RESOLVED:** this classifier (the categorical AST-kind + needle axis) is the SOLE landed shape axis — 217's T0 band axis closed as a measurement note (floor did not fire; `band_ood` removed), so no consumer is waiting and T9's annotation-priority loop never opened; the categorical axis stands alone until a consumer materializes. **T5 (promotion) WAITS on T4's numbers** — owner law applies only after the cost leg passes.
**Coordination (zed-m3-mechinterv, 2026-11-07, additive line per the 217 T8/T9 record):** the ordered band axis of refine issue 217 LANDED (`src/draft/band_ood.rs` — `token_count`/`Band`, pinned by `band_axis_is_pinned`; the T0 floor did not fire and 217 closed as a measurement note, file removed per the noise rule — record in `Research 386` §5). Per T8's consume-direction: your shape classifier MAY consume `band_ood`'s band axis if useful; 217's twin/sham + tier-gate rows never opened (their consumer gate failed), so no constraint binds on you. Your span_shapes dispatch work is unaffected.
**Claimed:** zed-native-agent@m3, 1794052000, expires 2026-11-09T12:00:00Z
**Claimed:** amo-idle-216, 1794238557, expires 2026-11-11T15:35:57+00:00
(the zed-native-agent@m3 claim lapsed — zero corpus commits from that lane; re-claimed per the documented normal path. amo idle lane, 2026-11-09T15:35Z — executing the queued T4: the pre-registered G2 cost protocol `scripts/g5_domain_cost.sh --feature span_shape_dispatch --base latent_retrieval` on this quiet rig; T5 promotion verdict rides the numbers)

## Design amendment (2026-11-07, BEFORE any measurement — the verified-premise correction)

The pre-registration below says "the syn node kind of the span's enclosing item, derived INSIDE the existing L2 pruner pass". Verified against production code (feature-gate-audit Defense 1): the L2 syn parse lives in the FIX lane (`heal.rs` main loop, `reheat_fix_pass`), NOT at the dispatch site — but the DISPATCH site already parses each chunk (the Issue-063 shared instantiation parse, `suggest_all_domains`: one `syn::parse_file(code)` per span, shared across domains, under the default Structural rerank posture). And an `AstChunker` chunk IS an item span (fn/impl/struct), so the chunk's ROOT items are its shape — no enclosing-item line mapping and no span-locations needed. The amended v1 classifier: root-item kinds folded from the SAME shared parse (zero additional parse work under the default posture) + two pinned body needles (`for `/`while `/`loop {` → HotLoop; `match ` → MatchBlock) for signals root items do not surface. Fail-open: `RerankMode::Off` (no parse), a parse failure, or a parse with no recognized root item all classify ALL — byte-identical to the ungated fan-out. The syn-derived enclosing-item classifier (span-locations mapping) is the recorded v2 upgrade path, never silently substituted.

## Placement amendment (same commit)

Annotation table lives in `rust_perf/corpus/mod.rs` (`span_shape_annotations()` — beside the corpus it annotates; the draft→domains import direction is established: `instantiation_rerank`/`headroom_probe` import domain proposers). The vocabulary type (`SpanShape`/`SpanShapeSet`, hand-rolled u16 bitflags, no dep) lives in the neutral `domain.rs`. The classifier + filter live in `draft/latent_matcher.rs` (`pub mod span_shape` nested — no mod-decl churn beside the sibling lanes' dirty `mod.rs`). The `span_shape_dispatch` feature implies `latent_retrieval` + `rust_perf` (a retrieval-lane filter over the perf corpus's table — self-contained, no orphan posture).

## Why

`codegram` is the coined name (riir-ai glossary) for the act the healer already performs: grammar-is-the-model over code — the per-domain rule corpus is the production set, the drafter generates candidates over matched spans, the verify ladder adjudicates. The machinery exists (L0–L4). What does NOT exist is **grammar-SHAPE awareness in dispatch**: today's cost gates are file-path/needle-signal granularity (kernel_pregate; issue 213's per-span `DomainMask` over the Issue-145 path contract). A rule whose signal is syntactic — fires only inside hot loops, trait impls, match arms, test fns — pays the full fan-out on every span of every enabled file.

The measured precedent that says span shape CAN pay: kernel_pregate GATE+SCOPE (Benches 132/137) cut added p50 by 173×/920× at an equal utility floor using a ±24-line span scope keyed on a needle signal. Issue 213 generalizes the FILE half to every domain. This issue is the SPAN half: key rules by the span's syntactic shape.

## The POC (pre-registered bars — pinned before any code)

- **Shape classifier:** cheap, deterministic, zero-alloc — the syn node kind of the span's enclosing item (`fn` / `impl` / `match` arm / `mod` / `#[test]` fn / trait decl), derived INSIDE the existing L2 pruner pass. `syn` is already parsed there — **no new parse, no new dep**.
- **Rule annotation:** each rust_perf (later: clippy_lints, sec) rule gains an optional `span_shapes` key set in its corpus row; rules WITHOUT keys stay unmasked (safe default — the annotation roll-out is gradual and never a cliff).
- **G1 correctness:** for every masked rule, output on matching-shape spans is byte-identical gated vs ungated (unit over the corpus fixtures + the 15-rule utility floor). A wrongly masked rule is a G1 FAIL, not a disclosed loss.
- **G2 cost:** the Bench-140 protocol verbatim (`scripts/g5_domain_cost.sh`, same walk, 3 interleaved rounds), `rust_perf` ON: added **p50 ≤ 1.0 ms/file AND p95 ≤ 5.0 ms/file**, quiet rig via `agent-mail rig run`, PROVENANCE beside every number.
- **G3 no-regression:** every other domain's output byte-identical.
- **G4 determinism:** the classifier is a pure function of the span bytes — **no lossy cap analog** (contrast `QUERY_TEXT_CAP`'s disclosed 109/664 flips). Any lossy shortcut proposed here needs its own ΩMSR-style per-family gate first.
- **Promotion on pass:** the owner law ("any gain no perf hurt must enabled by default") applies directly — mask defaults ON, env DEMOTE-ONLY, the five surfaces (source doc, lib.rs, Cargo default, RELEASE_FEATURES/release.yml, this issue + a bench record) in one commit.

## Non-goals / watch rows (do not fold these in)

- NOT retrieval — `kernel_route_narrow` stays closed (Bench 126); the re-open trigger is corpus ≥ ~32k rules (LSA knee, riir-infer `.research/020`).
- NOT sec's per-query retrieval overhead — that is issue 213 T5 (cross-repo, ndb-side score-only mode), pre-registered separately.
- NOT a new parser or AST grammar engine — productions stay pattern+template; "codegram" names the existing act, this issue adds only shape-keyed DISPATCH.

## Records

Bench 141 (audit + inventory) · issue 213 (file-granularity mask, in flight) · kernel_pregate (Benches 131–137, promoted 2026-11-06) · Bench 140 (the G5 cost finding that framed both gates) · issue 208 (the pregate origin).
