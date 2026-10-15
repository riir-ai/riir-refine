# Plan 244 — Batch 242: the mlx-serve GDN/QSA pack (kernel_opt)

**Status:** IN_FLIGHT — verdict landed (Research 274, claim `4ace9a2ea`); conversion tasks below. Numbering note: plan 244 / research 274 / Batch 242 per the committed claim — the tokenmaxxing lane renumbered to 245/275/243 around it (Bench-881 first-committed-on-origin rule; their note discloses the yield); the `.highwater` files stand at 245/275 (already above — monotone, no future reuse).

Source: `ddalcu/mlx-serve` @ pin `1015f7854fd8ee5c22e34993aaddee2443b9cfbf` (MIT, 2026-10-14). Verdict: YES B+ kernel_opt / LANE INTEL HIGH (Research 274 — the full ranked table, dedup evidence, and anti-leads live THERE; this plan is the execution charter).

## Tasks

- [ ] **T0 — re-clone at the pin + freshness check.** `git clone --depth 1` → `.raw/mlx-serve`; verify HEAD == `1015f785`; if the tip moved, re-grade ONLY the delta files (the B193 freshness law) and record drift. Read the UNREAD surfaces from the verdict: `gather_qmm_nax`'s embedded `.metal` bodies + a targeted `transformer.zig` grep for prefill-GDN/prework markers outside the module files.
- [ ] **T1 — per-candidate dedup re-run** (the standing law — the verdict's greps adjudicated the A-set; every second-tier candidate gets a fresh both-vocabulary grep at authoring time).
- [ ] **T2 — author slice A (GDN/QSA/RHT core)**: candidates #1 (split-prework), #2 (decline latch, absorbing `declined-shape-negative-latch`), #15 (read-amplify ceiling), #16 (gate-dtype lock), #18 (tree protocol — adjudicate 1 rule + sub-faces vs 3), + second-tier: op-chain-collapse, partial-softmax-l-zero, gqa-write-once, joined-in-place-strides, device-keyed-tg-size, fold-depth rider.
- [ ] **T3 — author slice B (quant matmul + packs)**: #8 (masked-in-place prescale — the GOAT; try the portable CPU-exact Rust fixture), #5 (clone-upstream-dispatch), #6 (row-count-invariant splits + probe arm), #7 (threadgroup-cap halving), #17 (prompt-form anti-lead row), #4 (canary admission), + second-tier: per-generation table, per-expert rungs, quad-lane ring, tp-rank solve; riders: half2-magic→lop3 host, e4m3→minifloat host, in-buffer-repack→derived-pack host, sigmoid-lut→exact-bf16 host, activation-permutation→B217 host, resident-bytes→stored-bytes host.
- [ ] **T4 — author slice C (attention + oracles)**: #9 (tree attention partial/tail/merge), #12 (noise-floor error bar), #14 (finiteness-before-diff), #19 (oracle-keyed-on-reference-dispatch), #13 (gqa-reshape-down), #3 (bit-equal decomposition mirroring + rounding-point placement), #10 (identity memo, merged), #11 (row-bucket budget), + second-tier: strided-interleave, gqa-head-position, config-cache-key, where-not-multiply, dequant-dtype.
- [ ] **T5 — wiring**: entries files (new `entries_gpu_96.rs`+ as needed), `mod.rs` registration + category counts + doc counts, `router.rs` expert-total if named, `tests.rs` count pins + `batch242_pseudocode_parses` + per-slice needle tests (verbatim quotes from the pin).
- [ ] **T6 — GOAT/G1**: batch242 parse + needles green; kernel_opt `--lib` green (count pins move only by OWN rules landed — corpus 816 → 816+N); default lib green; clippy `-D` ×3 postures (default / no-default / kernel_opt+latent_retrieval) — doc_markdown heals via `cargo refine --fix --write --verify` (the B192 class); **retrieval floors re-measure** (`--features kernel_opt,latent_retrieval --lib retrieval` — kernel self-variants 100%, blended vs floors; re-pin ONLY with pre-batch A/B attribution, the B223 precedent); freeze/score-bench NOT owed (zero clippy-corpus delta, P30 — verify the write set is kernel_opt-only).
- [ ] **T7 — docs**: AGENTS.md kernel_opt bullet + count; `.docs/11_domains/kernel_opt.md` head count + Batch 242 row; README count if stale; Research 274 status → LANDED (batch hash); this plan ticked; `.distill/001` landing row; `rm -rf .raw/mlx-serve` (the ephemeral-clone law — cleanup IS part of the batch).
- [ ] **T8 — commit + push** (`feat: batch 242 — the mlx-serve GDN/QSA pack`); push race → re-fetch, rebase non-interactive, yield-on-rejection per protocol.

## Yield ledger (vs Research 274)

Target: 12-16 own rules + 4-7 riders. If conversion falsification kills below 8 own rules, record the kills honestly in the landing row (the lithos disclosure precedent — landed ABOVE estimate was disclosed; landing BELOW is equally disclosed).

## Anti-goals

- No edits to `.raw/` (read-only; copy out for experiments).
- No sibling-repo edits (lane-intel addenda are recommendations recorded in Research 274, not edits).
- No batch242 corpus edits before this plan is committed (plan-first, Protocol A step 0 — satisfied by this file's commit).
