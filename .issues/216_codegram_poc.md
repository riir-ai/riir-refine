# Issue 216 — codegram POC: syntax-shape-keyed span dispatch (the grammar lever beyond issue 213's file mask)

**Status:** OPEN — filed 2026-11-07 (Bench 141 T4); pre-registered BEFORE implementation; builds ON issue 213's `DomainMask` (consume, never fork).
**Coordination (zed-m3-mechinterv, 2026-11-07, additive line per the 217 T8/T9 record):** the ordered band axis of refine issue 217 LANDED (`src/draft/band_ood.rs` — `token_count`/`Band`, pinned by `band_axis_is_pinned`; the T0 floor did not fire and 217 closed as a measurement note, file removed per the noise rule — record in `Research 386` §5). Per T8's consume-direction: your shape classifier MAY consume `band_ood`'s band axis if useful; 217's twin/sham + tier-gate rows never opened (their consumer gate failed), so no constraint binds on you. Your span_shapes dispatch work is unaffected.
**Claimed:** zed-native-agent@m3, 1794052000, expires 2026-11-09T12:00:00Z

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
