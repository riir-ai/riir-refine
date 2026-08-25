# Bench 049 — fixer re-gate POC: harness + offline validation (arms pending)

**Issue:** [040](../.issues/040_fixer_regate_poc_cf_workers_ai.md) · **Proposal:** [005](../.proposals/005_kat_heal_service_network.md) caveat #2 / Phase 3
**Date:** 2026-08-25
**Machine:** M3 Max (macOS, arm64) — **UNDER SIBLING COMPUTE LOAD** (`bonsai_clippy_l4_sft_train` ~822% CPU + `maglev_drafter_train` ~85% CPU for the whole session). No latency number in this note is gate-grade; every number here is a COUNT or a VERDICT (load-insensitive).
**Status:** T1 (harness) + T2 (pre-pass) SHIPPED + instrument-validated offline. **Arms A/B pending `CF_API_TOKEN`; arm C pending checkpoint + idle GPU — the verdict is PENDING ARMS, by design of this run.**

---

## Setup

- **Harness:** `src/cf_workers_ai.rs` (feature `cf_workers_ai`: `dep:ureq` + `latent_retrieval` + `clippy_verify` + `l4_lora`) + `src/bin/fixer_regate.rs` (`--arm A|B|C|DRY`) + `tests/t40_fixer_regate_harness.rs` (5 tests, 1 ignored full-corpus).
- **Default build untouched:** `cargo check` green; `cargo clippy --all-targets --features cf_workers_ai -- -D warnings` clean (healer first for mechanical findings — 5 edits — then manual for pedantic/doc/must_use).
- **Substrate reuse (the issue's "~80% shipped" made concrete):** reachable-miss set + `control_fires` + `contract_idents` from `tests/t51_l4_vs_decline.rs` (copied per that file's own precedent, premise re-asserted at construction); miss classification + modelless candidate via `HealPipeline::heal` with the bench_003 per-lint candidate construction; few-shots via `LatentMatcher::suggest_span` (clippy domain arm) + `rule_book`; fragment-parse via `RustAnalyzerPruner` (the seam's invariant-3 gate); oracle via `ClippyVerifier`; behavioural veto via `behavioural_equivalence::check` (the Bench 039 "route 3" module); arm C transport via `DaemonL4` (gained `request_prompt` for the pre-pass shape). NEW code: the CF REST client (~80 LOC), the v2edit loader, and the edit-application ladder — everything else is composition.
- **Never-wrong keep rule (two filters, both reported):** `keep_oracle` = control fired ∧ oracle ran ∧ findings strictly fewer ∧ not a no-op (the proposal's filter); `keep_strict` = `keep_oracle` ∧ contract-preserved ∧ not-behaviourally-divergent (the Bench 039 hardening — both false-WIN routes the n=9 run actually produced).

## Arms status

| arm | model | status | blocker |
|---|---|---|---|
| A (CF raw) | `@cf/meta/llama-3.3-70b-instruct-fp8-fast` REST | **PENDING** | no `CF_API_TOKEN`/`CF_ACCOUNT_ID` in env, no `~/.wrangler/config`, no `~/.cloudflare` (presence-checked, never echoed) |
| B (CF + pre-pass) | same | **PENDING** | same |
| C (Bonsai daemon) | local 27B base via `bonsai_clippy_l4_eval --serve` | **PENDING** | (a) M3 GPU busy with sibling Plan-349 training; 4090 has `bonsai_clippy_l4_sft_train.exe` resident — GPU-exclusivity rule bars gate runs; (b) **no `.bcl4` checkpoint exists on disk** (t51's `/tmp/t51_reach/` is gone) AND the riir-train driver hard-requires `--lora` (no base-model mode), so "arm C base" needs either a checkpoint or a driver change in riir-train (sibling WIP — not touched) |
| DRY (offline) | planted fixes | **RUN** | — |

## Arm DRY results (instrument validation — NOT model quality)

```
decline baseline        9/9  off-corpus reachable cases serve no modelless fix
                            (t51 arm0 re-measured through this harness — the Issue 030 gate holds)
planted POSITIVE        4/4  keep_strict (ptr_arg f1→0 behav Equivalent{4};
                            or_fun_call f1→0 Equivalent{5}; redundant_clone f1→0 Equivalent{1};
                            box_collection f1→0 behav Unsupported[no Debug] — reported, not vetoed)
planted NEGATIVE        3/3  correctly rejected:
                            noop echo        keep_oracle=false (findings 1→1) — never kept
                            destructive drop keep_oracle=TRUE (!) keep_strict=false — the contract
                                             veto caught what the raw oracle cannot see (Bench 039 route 2)
                            garbage          merge=None — refused to guess
v2edit EM               8/8  (first 8) and 408/408 (full corpus, release) — the EM machinery
                            reproduces the riir-train driver's semantics exactly
behavioural veto        REFUTES the real Bench 039 route-3 generation
                            (`o.unwrap_or(0) + 1`: names preserved, clippy clean, None-branch wrong)
                            → pinned as a standing test
```

**Instrument verdict: ALL CONTROLS PASS — the harness is validated in both directions; arms pending token/checkpoint.**

## Corpus findings (measured, load-insensitive — these stand regardless of arms)

1. **The v2edit reference responses are 96.6% unmergeable by deterministic means: 14/408 rows merge (all `lcs-bounded`), 394 fail.** The "output ONLY the corrected lines that differ" contract carries position information ONLY when a changed line happens to repeat an unchanged context line (row 0 does — its `fn baz` header anchors it; rows 1/3/5 emit pure new-content runs with no anchors, and the runs concatenate edits from multiple fns with no separators). The in-tree riir-train `rlvr_reward::substitute` (whole-fn blocks) cannot apply them either — these references were never mechanically applicable in general; the G1 EM gate never needed to apply them (EM is text compare). **Consequence: any oracle-verified metric on v2edit requires either semantic merging or a corpus rebuild with anchors/line numbers.** The harness reports merge failure honestly rather than guessing (a mis-merge would fabricate oracle verdicts).
2. **`manual_map_result` is not a clippy lint** — `clippy::manual_map_result` is `unknown lint` (E0602) on clippy 1.93/1.97; the corpus labels **64 rows** with it as the first lint. This accounts for 64 of the 67 control failures (control = 341/408 corpus-wide, 55/60 on chunk 00 where the label is rarer). The remaining ~3 are scaffold-shape issues. A label defect on 16% of rows is a further confound for the historical G1 record.
3. **keep_o 9/408 on the mergeable reference subset (9 of 14):** the reference responses, where they CAN be applied, do resolve findings (5 of the 14 merged without reducing the target count — multi-lint rows where the reference fixes only some sites).
4. **The reachable-miss premise holds:** all 9 t51 reachable lints remain absent from the seed corpus (asserted at harness construction — a corpus growth fails loudly).

## Metrics columns (per arm, when arms run)

EM (v2edit only) · fragment-parses (L2 gate, raw generation) · merge strategy + rate · control rate · oracle-scoreable · findings strictly-fewer · **keep_oracle / keep_strict (the bar)** · vetoed / divergent / regressions · tokens · $/fix (price assumptions `--price-in 0.29 --price-out 2.25` per-M, overridable, NOT a quote) · latency/fix (disclose load).

**Pre-registered verdict rule (from the issue):** an arm beats `decline` iff `reachable keep_strict ≥ 1` AND no kept case is later shown wrong (the vetoes make kept-wrong structurally unlikely, not impossible — `Unsupported` behavioural results remain admissible-but-disclosed).

## T5 — CF data-usage policy (leak-posture half)

The historical per-model docs page (`developers.cloudflare.com/workers-ai/reference/data-management/`) is **404** now. The current primary source is Cloudflare's Trust Hub page, fetched 2026-08-25:

> "We recognize that the privacy and security of our customers' content is paramount, and **we do not use our customers' content to train any LLMs**."
> — cloudflare.com/trust-hub/responsible-ai/ (FAQ: "Does Cloudflare train AI on customer data?")

> "Importantly, Cloudflare **does not train AI models** or provide GPU access for training." (Workers AI section)

**Honest nuance for the sign-off:** Workers AI models are third-party-provided ("The AI models, including GPAI models, made available on Workers AI, are provided by third-party providers") — Cloudflare's commitment covers Cloudflare's use of the content; the model side additionally rides the provider's license (Meta Llama for arm A/B). T3.2's sign-off should quote the Trust Hub page + name the model provider's license, not treat "Cloudflare says no-training" as the whole posture.

## Reproduce / unblock

```sh
cd riir-clippy
# offline validation (what this note records)
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm DRY
cargo test --features cf_workers_ai --test t40_fixer_regate_harness -- --nocapture

# arms A/B — one command once the owner exports the token (~$0.02-0.05 at default
# 25-sample scale; ~$0.30-0.80 for the full 417-sample set):
export CF_API_TOKEN=... CF_ACCOUNT_ID=...
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm A --out /tmp/armA.jsonl
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm B --out /tmp/armB.jsonl
# full eval set: add --source both --limit 0 --chunks 0,1,2,3,4,5,6

# arm C — needs a .bcl4 checkpoint (none on disk today; regenerate via riir-train
# Plan 336 T2.9 or convert the 4090's E:\tmp\p336_t29 LORC best) AND an idle GPU:
cargo run --release --features cf_workers_ai --bin fixer_regate -- --arm C \
  --l4-bin /Users/katopz/git/riir-train/target/release/examples/bonsai_clippy_l4_eval \
  --l4-gguf /Users/katopz/git/riir-train/data/Ternary-Bonsai-27B-Q2_0.gguf \
  --l4-lora /path/to/ckpt.bcl4 --limit 8
```

## What this run settles / what it does not

- **Settles:** the instrument is trustworthy in both directions (positive controls kept, both false-WIN routes vetoed on real data, EM exact, decline baseline re-measured). Two v2edit corpus defects are now measured facts (unmergeable-by-contract responses at 96.6%; an invented lint label on 64 rows).
- **Does not settle:** the falsifiable question itself — whether verify-filtered acceptance beats decline on any REAL arm. No model ran. The verdict is **PENDING ARMS** and Proposal 005's blocker stands until an arm runs.
