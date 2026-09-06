# Research 145 — Batch 122: the ds4 PR vision-delta distill (Qwen3.8 vision turns)

**Status:** RECORD — Batch 122 landed 2026-09-07 (M3 box, CPU-only, box idle: zero compute consumers).

## Source + provenance

ivanfioravanti/ds4-metal (MIT fork of antirez/ds4), branch `qwen3.8-flash-next` @ `69c38d23aace88240ad5e57d8822b1979c4826c6` — the Batch-121 pin `c5ae2eb5` beaten by 4 commits (`9f4313d` vision download target → `3bf8552` merge → `c64bdff` vision quality results → `69c38d2` model-aware chat formatting fix). Clone staged in `.raw/ds4-metal` (gitignored), HEAD re-verified == ls-remote at stage time, every quoted line re-verified at the pin before entry authoring (the Batch-31 discipline); `rm -rf` at landing.

Trigger chain: Plan 102 T0 (2026-09-07 05:52 +07) caught BOTH ds4 triggers fired by `git ls-remote` — PR head past the B121 pin AND upstream `antirez/ds4` main `9ab7053` → `c0a6119f` (41 files, +6118/−1100). Part A (the 361-line PR vision delta) = this batch; **Part B (the upstream surge: GLM attention/rope + NEW Metal SSD-experts + session-state + SSD-cache tests) is QUEUED as Batch 123** — different domain (kernel_opt), different size class; dedup grep `ssd|state.space|expert` REQUIRED before proposing (the SSD dual-cache lineage has corpus coverage).

## The delta (361 lines, 8 files) — build tooling + measurement methodology, no kernel code

`download_model.sh` (+19: the `qwen38-vision` target) · `ds4_cli.c` (±13: the image-turn fix) · `docs/QWEN38_FLASH_NEXT.md` (+25) · `docs/MODELS.md` (+19) · `tests/test_qwen4_cli_vision.py` (NEW +62) · `speed-bench/qwen38-vision-quality.{md,json}` (NEW +228: IQ2/Q4K vision smoke results).

## Rules (rust_perf io +3, corpus 105 → 108)

1. **`model-aware-format-over-hand-assembled-tokens`** (the lead, `69c38d2`) — the CLI hand-assembled an image turn (user token + vision span + `"\n"` + text pushes); image turns broke because the assembly drifted from the model's chat template. Fix: ONE model-aware `ds4_chat_append_multimodal_message` owns the turn layout, with atomic rollback (`rollback_len`/`rollback_images`) so a failed span reservation leaves nothing half-entered. Failure class: template drift is SILENT — the transcript tokenizes, the model answers, nothing can tell the layout stopped matching training. Pinned phrases: `ds4: image %ux%u, %u image tokens` (the success marker, ds4_cli.c:1846), `append_multimodal` (ds4_cli.c:1542).
2. **`implemented-surface-requires-provisioning-target`** (`9f4313d`) — vision fully implemented (Qwen3-VL tower + `metal/qwen4_vision.metal` + `make test-qwen4-vision`) but the downloader never fetched the encoder: `--vision` dead out of the box. The commit message names its own class: *"Follow the GLM 5.3 pattern: a standalone qwen38-vision target."* Three parts: named per-artifact provision target; load-time failure NAMES the missing target; on-disk cost documented ("about 0.6 GB on disk. Load it with --vision; this target does not update ./ds4flash.gguf", download_model.sh:167-168).
3. **`diagnostics-not-just-exit-code`** (`69c38d2`'s regression, the methodology rule) — *"The REPL can return zero after an image turn fails if a later text turn succeeds. Check diagnostics as well as the process exit code"* (tests/test_qwen4_cli_vision.py:46-48). Exit code aggregates the SESSION, not the turns; the regression scans captured stderr for per-turn failure patterns (`failed|outside the vocabulary`) + the encoder success marker, saves full transcripts per decode mode (ordinary + MTP), and **deliberately does not grade image content** ("it saves the responses and diagnostics for inspection; it does not grade image content") — grading content makes the test flaky; grading diagnostics keeps it deterministic.

## Deliberately NOT taken

- The line-break path guard (`image paths must not contain line breaks`, test:36-37) — real but a one-line input-validation note, already subsumed as an anti-shape mention inside rule 3's family; a fourth rule would pad the batch.
- The `ds4_chat_append_multimodal_message` implementation internals — the rule is the CONTRACT (one model-aware formatter owns the turn), not the C function's body.
- LEAGUE INTEL: none new — the vision smoke runs on the 512 GiB Ultra box, IQ2/Q4K both 6/6 fixtures, no league-relevant quant economics beyond B121's record.

## Dedup (grep evidence, 2026-09-07)

`chat_template|format-over|hand-assembl|multimodal|provisioning|exit.code` across `src/domains/rust_perf/` + `src/domains/kernel_opt/`: zero concept coverage. Nearest-neighbor recorded in-rule: B121's `write-verify-readback-before-publish` (the transcript IS an assembled artifact — composition, not duplicate), B121's `resume-revalidates-provenance` (template version joins the identity), `coalesce-nearby-reads` B94 (io regime, unrelated mechanism).

## Gates (all FIRST-SHOT except the parse gate, repaired once)

- `batch122_rules_present` + `batch122_pseudocode_parses` + `batch122_concepts_covered`: 3/3 (the parse gate caught the diagnostics snippets as bare statement lists — bare `let` at file top level is not a Rust file; wrapped in `fn run_session`. The gate doing its job).
- Count pins walked: total 105 → **108** (corpus.rs:8092), annotated 48 → **51** (×2: serde test :9754 + the forty-three test :9785), header ledger 108/io-12/21-NotBenchable + the three new ledger lines + `(87/108)`, benchable **87 UNCHANGED** (:9606 — all three Suggestion-only, no bench_spec).
- Lib default **1162/0**; clippy 0 (default, +rust_perf); retrieval re-run **kernel 100/100/100, blended 94.7/96.9/97.7** (byte-identical to B118/119/121 — the three io rules did not disturb the blended floors); score-bench **FLAT 96.2/1.9** (gen-7, 72 runs).
- Parse-gate repair lesson recorded: the syn gate reads WHOLE FILES — a snippet of bare statements is not a file. Second occurrence of this class (the B-x predecessor was the pseudocode rewrite forced on B117's 3 entries); proposer-side fixtures should always wrap in a fn.
