# Bench 144 — codegram span-shape dispatch landing (issue 216 T1): classifier + annotation table + filter

**Status:** CLOSED — PROMOTED 2026-11-11 (T4 cost leg PASS on amo, both runs; release set carries `span_shape_dispatch`; demote `RIIR_REFINE_SPAN_SHAPE=0`). T1 LANDED 2026-11-07 (classifier + annotation table + filter + pins; local gates GREEN at default / feature / release-set postures; tests 9/9). T4 (G2 cost) RAN 2026-11-11 on a quiet amo per the pre-registered protocol — see the T4 section. T5 promotion landed the same day: RELEASE_FEATURES (build_stamp.rs) + release.yml + manual_release.sh in lockstep (the two `release_set_matches_*` tests pin the trio), lib.rs doc line, demote env, this record + issue 216 closed.

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

## T4 — the G2 cost leg — RAN 2026-11-11, PASS ×2 (amo)

`sh scripts/g5_domain_cost.sh --feature span_shape_dispatch --base latent_retrieval --rounds 3 --out-dir /tmp/g5_216` on amo (Apple M5 Ultra, 30 cores, 96 GB, on AC; load 3.7→10.6 rising across run 1, 4.4→6.7 across run 2 — quiet-builder class, the orchestrator watchdog beside; the `agent-mail rig status` verdict at start: QUIET). Bar (Issue-191 bar-5 form, shared with Benches 132/137/140/143): added ON−OFF scan latency per file, per-file medians, nearest-rank **p50 ≤ 1.0 ms AND p95 ≤ 5.0 ms**, ALL 3 interleaved rounds. 651 files (this crate's `src/`), corpus = the compiled rule corpus; the runner's ON-only-features stamp guard confirms the ON arm differs by exactly `span_shape_dispatch` (engagement proof).

**Run 1 — the T1 code as landed (11-07), executed 2026-11-11 ~03:11Z:**

| round | added p50 ms/file (support 326) | added p95 ms/file (support 34) | verdict |
|---|---|---|---|
| 1 | −0.231 | 0.023 (217×) | PASS |
| 2 | 0.006 | 0.432 (11.6×) | PASS |
| 3 | −0.261 | 2.017 (2.5×, load spiked to 10.6 mid-round) | PASS |

**Run 2 — the PROMOTED code (env latch in, the shipping posture), executed 2026-11-11 ~03:41Z:**

| round | added p50 ms/file (support 326) | added p95 ms/file (support 34) | verdict |
|---|---|---|---|
| 1 | −0.153 | 0.390 (12.8×) | PASS |
| 2 | −0.042 | 0.658 (7.6×) | PASS |
| 3 | −0.134 | 0.144 (34.7×) | PASS |

Verdict: the dispatcher is free at the serving posture — the ON arm reuses the Issue-063 shared per-span parse (zero added parse work) and `rule_admits` is a 6-row table lookup per Perf candidate; the ON-only work is one u16 fold + one 6-row retain scan per span. Negative p50s across run 2 = ON never slower than interleaving noise; masked rules LEAVING the pool can only shrink the ON arm's marking/rerank work (the counter-press, observed). Run 1 rode the pre-latch code (the lease holder's claimed lane produced nothing for 34h — closed by owner call, executed by the amo zed session; the claim line on issue 216 is folded below in the same commit). Run 2 re-ran the identical protocol on the promoted binary so the shipped posture is the measured one.

**Utility-floor disposition (unchanged, disclosed):** `g5_utility_cost.sh`'s three postures are kernel-expert-flavored and cannot parameterize this feature — running it would measure a posture without the mask. The floor carries BY CONSTRUCTION: the mask touches only the 6 annotated rust_perf rules; the kernel-expert census (Bench 125/137's 15/15) and every other domain are structurally untouched; the g5 ON run's harness census is the disclosure record. The annotation-priority feedback loop remains issue 217 T9(b)'s curve — unopened (no consumer waiting).

## T5 — promotion (landed 2026-11-11, one commit)

- **The release set carries `span_shape_dispatch`** — `RELEASE_FEATURES` (src/build_stamp.rs, with the dated comment), `.github/workflows/release.yml`'s cargo-refine build line, and `scripts/manual_release.sh` FEATURES moved in lockstep (the `release_set_matches_workflow` + `release_set_matches_the_manual_script` tests pin the trio; 4/4 green post-landing). The cargo-about line is deliberately NOT extended: the feature implies `latent_retrieval` + `rust_perf` — BOTH already in the shipped graph — so the dep + license graphs are unchanged (the kernel_pregate precedent named it there only because dep:regex was NEW).
- **Bare cargo default stays light BY DECISION** — the pre-registration's "Cargo default" surface is amended: `span_shape_dispatch` implies the `latent_retrieval` tree (dep:riir-rag), and the default build's lightness is a standing law. The owner-facing surface that ships the feature is the release set — the honest reading of "defaults ON" for a released binary.
- **Demote env (the env-switch law):** `RIIR_REFINE_SPAN_SHAPE=0` (the exact literal, DEMOTE-ONLY — unset or any other value arms; promote-shaped literals are deliberate no-ops). Demoted = classify skipped + verdict `ALL` + the retain early-outs = the pre-promotion fan-out byte-identically. Read once per process (`OnceLock`, zero-alloc); pinned by `demote_literal_is_exact_zero` (the parse is a pure fn, no process-global env mutation in tests).
- **Post-promotion gates (amo, 2026-11-11):** span_shape tests 10/10 (9 T1 + the new demote pin); release_set lockstep 4/4; `cargo clippy --all-targets -- -D warnings` GREEN at default AND at the full release set (all-targets — T1's release-set row was `cargo check`, which never compiled the test targets; see the en-route repairs below); katplane_parity 4/4 at BOTH net postures.
- **En-route repairs (all in this commit, the T1 "fixed in the same commit" precedent):** the release-set all-targets clippy posture reds on 4 pre-existing mechanicals + 1 of mine — lib.rs doc backtick (`rust_perf`), corpus/tests.rs doc backticks (`WordPiece`, `entries_29`), approx_constant's intentional f64→f32 derivation casts (fn-scoped `#[allow(clippy::cast_possible_truncation)]` with the reason — the truncation IS the bit-exactness assertion; a statement-level allow does not govern a macro's expansion), and `tests/katplane_parity.rs` importing `DEVNET_SERVICE_URL` unconditionally — that const compiles to NOTHING under the shipped net-mainnet pin (the foreign-hosts compile-to-nothing discipline), so the target could not build at the release-set posture at all since the pivot; its devnet row now rides the same cfg (4/4 green at both net postures). The installed cargo-refine binary was STALE (pre-points-pivot build), so these went manual per the documented fallback; the release-set rebuild rides this landing.
- **Lease:** the amo-idle-216 claim (epoch 1794238557, expiry 2026-11-11T15:35:57Z) never produced numbers in 34h — closed by owner call with the rig quiet; folded on issue 216 in the same commit.

## Coordination (T8, final)

The T8 ordering resolved THIS cycle: 217's T0 band axis landed first but its floor did not fire — 217 closed as a measurement note (`band_ood` removed), so **this classifier is the sole landed shape axis** and no consumer is waiting. The sibling's additive coordination line on issue 216 (2026-11-07) is honored verbatim; nothing binds.

## Records

issue 216 (filed Bench 141 T4; claim zed-native-agent@m3 exp 2026-11-09T12:00Z) · claim commit `8eaf16d42` · issue 213 closed `9cc961530`/Bench 143 (the file-mask generalization, shader_perf promoted) · issue 217 closed-null (band axis) · issue 208/Benches 131–137 (the pregate precedent) · Bench 140 (the G5 protocol) · issue 102 (the active-set retain semantics) · Issue-063 (the shared per-span parse the classifier reuses)

Session: zed-native-agent@m3, epoch 1794… (commit body)
