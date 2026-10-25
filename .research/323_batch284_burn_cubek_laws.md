# Research 323 — Batch 284 landing record: the burn/cubek autotune + fusion laws (Plan 288 / Research 321 conversion)

**Status:** RECORD — LANDED (this note is the batch's landing record; the verdict + candidate table + dedup census live in Research 321, unchanged).

## 1. What landed

SIX rules from the burn/cubek distill verdict (Research 321 §4), corpus 970 → **974** kernel_opt + 271 → **273** rust_perf:

| # | rule_id | domain/category | grade | source pin |
|---|---|---|---|---|
| 1 | `autotune-round-bounded-by-roofline-ceiling` | kernel_opt K | A− | cubek tip `08659297` roofline.rs (Ceiling enum + Either trap) + burn `907ab111` attention/bounds.rs consumer |
| 2 | `autotune-sample-read-cold-not-cache-fill` | kernel_opt K | B+ | cubek tip roofline.rs (`read_cold`/`touch`/`COLD_READ = 256 << 20`/read-not-fill) |
| 3 | `cost-model-counts-work-the-kernel-skips` | kernel_opt K | B | cubek tip cubek-attention cost.rs (triangle/window/decode/GQA laws + numeric tests) + burn bounds.rs |
| 4 | `fusion-unfusable-head-settles-prefix-to-next-search` | kernel_opt C | B | burn v0.22.1 burn-fusion/search/block.rs (the two settle fns) + backend.rs FuserStatus::Closed |
| 5 | `timed-zero-refuses-round-stop` | rust_perf **measurement (NEW 14th category)** | B | cubek tip autotune/base.rs:325-330 |
| 6 | `tune-cache-keyed-by-spelled-variant-names` | rust_perf caching | B− | burn-pin interpolate/tune.rs:35-36 (spelled-out names) + cubek base.rs:43-44 (checksum/index + sentinel-name law) |

Plus the count-neutral RIDER (candidate G faces 2+3 on the Batch-8 popcorn host `anchor-shape-bucketing`): the derived-from-anchored discipline (verbatim at cubek pin `51c43a1a` tune_key.rs — "computed from the *anchored* dims…never from the raw values") + the should_tune_double_buffering candidate-set gate. T1 (plan 288) found G's face 1 already owned by the host; the census §5 "anchor|bucket → zero hits" line is corrected by that finding.

## 2. The batch's own findings (en-route)

1. **T0 quote re-verification caught ONE Research-321 sentence not-verbatim-at-pin** — D's "Both contractions, and nothing between them. A roofline that counted one would be half the ceiling…" is absent from cost.rs + bounds.rs at the pinned revs (likely cubecl's `tune::Work` docs, an external dep, or a paraphrase). The rule cites the VERIFIED forms: the doc laws (cost.rs:35-38, 109-112) + the numeric test contracts (`counts_both_matmuls`, the causal-triangle 100-of-128 test, `a_decode_step_costs_a_full_pass_over_the_cache`). The B96-98 precedent applied: re-verify every quote, flag what fails.
2. **Two citation-path corrections**: candidate I's burn-side "First entry should always work" lives in `crates/burn-cubecl/src/kernel/attention/tune.rs` (Research 321 said "tune/base.rs" — no such file); cubek repo paths carry a `crates/` prefix throughout.
3. **A new 14th rust_perf category** — `measurement` (the instrument-validity family; the timed-zero/loud-zero class recurs — katgpt-rs Issue 855's timed_region_guard is the same class one layer up). Deliberate, the thermal-precedent form.
4. **T1's census correction** (recorded in plan 288 §T1): candidate G face 1 OWNED by popcorn `anchor-shape-bucketing` — G converted as a rider, not an own rule; every other candidate face confirmed unowned (E by a corpus-wide ZERO, the strongest form); I's census grep-line gap closed (run this pass, unowned).

## 3. Gates (all run this session, the /tmp/b284_target isolation)

- batch284 needles + parse: 3/3 GREEN first content-run (one rider-needle miss repaired at the gate — the test asserted a checksum-face mark that belongs to the rust_perf F rule, not the G rider; the mark was replaced with the rider's actual FACE 3 text).
- kernel_opt lib (`--features kernel_opt`): **3235 passed / 0 failed** (after the stale-pin cascade repair — 23 multi-line count witnesses + Plan-003 Fusion 62→63 + K 22→25 + router expert-total + the batch7 concept exact-pin converted to the rider-tolerant prefix form, its own precedent from Batch 225).
- default lib: **2732 passed / 0 failed**.
- rust_perf_bench lib: **2763 passed / 0 failed** (without_spec 150 → **152**; the `bench_spec: None` lines carry the `#[cfg(feature = "rust_perf_bench")]` gate per the entries_49 pattern — without it the non-bench postures fail E0560).
- clippy −D ×3 postures: default / kernel_opt / no-default ALL CLEAN (5 doc-lint findings healed in-batch: 2 `NotBenchable` backticks + the entries_gpu_129 header's `+`-continuation doc-list parse — reworded, no `+`-led line in the module doc).
- retrieval floors (`--features latent_retrieval`): **21 passed / 0 failed — HELD, no re-pin** (2898 filtered, 81.9s).
- score-bench run 201: heal **0.9636** / created 0.0182 / parse **1.0** — FLAT (the P74 expectation for a non-clippy-domain corpus delta; frontier snapshot gen-7 appended).
- freeze gen2/gen3: NOT owed (P30 — kernel_opt + rust_perf are not the clippy corpus).
- Release binary: rebuilt at the landing commit with the full release set (`latent_retrieval,kat_service,kat_transport,fix_verify,update_check`), installed to `~/.cargo/bin/cargo-refine.exe` (Windows plain-copy rule), `--version` build hash verified == landing HEAD, `release set: complete`.

## 4. NotBenchable annotations (the domain-reason class, Research 321 §6)

All six carry the NotBenchable/metric-mismatch verdict class in-rule (the drafter-propagation gate enforces the vocabulary): the win metric lives in GPU autotune rounds / fusion-search deferrals / persisted-tune integrity — none microbench-able; the decision content (ceiling arithmetic, stop predicates, work models, settle boundaries, checksum/index laws) is CPU-testable and IS the fixtures. The two unsafe-adjacent rewrite classes (PerfR's to_int_unchecked / slice-view recovery, Batch 282) do not recur here — no rule requires unsafe to express.

## 5. Queue state at landing

B284's claim (2026-10-25 ~16:1x +07, expiry 2026-10-27 ~04:3x +07) is DISCHARGED by this landing. Remaining pool: F/H/I/J (Research 321 §4, unowned per the T1 re-grep, B−/C+ grades — convert only if a future slot wants them; J is docs-borne, the weakest provenance); a SECOND burn/cubec batch could take the conv/pool/quant kernel bodies (unread at the pins — the Research 321 drops list). NOTHING INVENTED — every quote a same-session read at the pins or a corpus grep at HEAD; every gate a run.
