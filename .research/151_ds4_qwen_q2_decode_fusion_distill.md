# Research 151 — the ds4 Qwen Q2 decode-fusion + prefill-structural distill (Batch 129)

**Status:** RECORD — Batch 129 landed 2026-09-08 (Plan 110); corpus kernel_opt 570 → 575, rust_perf 112 → 114; gates green; pins current: PR `b85a6174`, upstream `f62ca29a`.

## Trigger (dual, verified by API — the B120 zero-build protocol)

1. **PR #991 head moved** `ffd85d42` → `b85a6174` (updated 2026-09-07T22:07:20Z, AFTER the B125 mine read). **LINEAR** — compare status `ahead`, `behind_by 0`, no force-push: the compare IS the true delta (12 commits, 42 files). The B125 rebase-surge trap did NOT re-fire.
2. **Upstream main moved** `c0a6119f` → `f62ca29a` (2 commits, 43 ln) — the standing Plan-106-T7 trigger. Its content commit `b6af0ad` rode into the PR branch via the PR's upstream merge (`f9d76f9d`), so ONE mine covers both triggers.
3. prismml master re-checked the same pass: identical to the B128 pin `5ea87dda` — the P14 watch's "15 commits past the league pin" are tag-ancestry already inside B128's mine. No trigger.
4. 4090 checked pre-mine (LAN): 20% / 565 MiB idle. M3 box: zero cargo processes.

## Sources

- `ivanfioravanti/ds4-metal` @ `b85a6174`, branch `qwen3.8-flash-next` (MIT verified at stage time; fresh clone, HEAD == ls-remote; removed at landing).
- `antirez/ds4` @ `f62ca29a` (MIT; fresh clone; removed at landing) — read for the `b6af0ad` SSD commit + the carve-out-arm context.

## Lanes (2 parallel read lanes + coordinator)

- **Lane 1 (kernel surfaces):** 8 commits — `metal/qwen4.metal`, `metal/dense.metal`, `ds4_gpu.h`, `ds4_metal.m`. Returned 5 candidates + 1 rider + 7 negatives + 4 skips, quotes verified character-exact at the pin.
- **Lane 2 (methodology + negatives + io):** rejected-experiments commit `01a9489`, both speed-bench decision writeups, host/io surfaces, the upstream SSD commit. Returned 2 candidates + 2 riders + 4 negatives + 4 skips.
- **Coordinator:** spot-re-verified the load-bearing quotes at the pins (cache-key block, padding guard, tail ladder, halo write, alias reject, argmax gate, the b6af0ad diff itself — read in full at the upstream pin). All verified.
- **Lane correction adopted:** Lane 2 found `static-seed-priority-bounded-to-observed-scale` already landed (entries_gpu_31.rs, Batch 123, anchored on `91dda0b`) — the coordinator's draft "new rule" for `b6af0ad` was reclassified a RIDER (the dynamic-producer twin). Honest-correction path worked as designed.

## Landed (5 kernel_opt + 2 rust_perf io + 5 riders)

New kernel_opt (entries_gpu_35.rs):
1. `pipeline-cache-key-covers-every-function-constant` (M) — a keyed pipeline cache serving driver-function-constant-specialized variants must name EVERY bound constant; subset keys serve one variant's compiled binary for another's launch args (silent wrong math). Sentinel keeps its own key entry; out-of-range refuses fail-closed.
2. `block-quant-weight-pad-never-extends-activations` (N) — Q2_K 256-block padding is weight-stride-only; dequant loops gate on the LOGICAL bound; partial activation groups reject fail-closed; tests poison the padding nonzero.
3. `expert-remainder-routed-to-smaller-tile-twins` (H) — per-expert tails run the smallest covering tile twin under a compile-time `tail_base`; the runtime-branch alternative measured −10.98% in-source and was removed.
4. `window-recurrence-halo-snapshot-block-parallel` (L) — K≤4 causal conv parallelizes over token blocks behind a HALO-only snapshot; full-input snapshot measured slower and was superseded in-source.
5. `fused-residual-update-writes-separate-next-buffer` (C) — fused residual+norm reads old buffers read-only, writes distinct next buffers, alias-rejects fail-closed, gates on the measured session shape.

New rust_perf io:
6. `instrumented-attribution-not-production-times` — instrumentation slowed the pipeline ~38% in-source; instrumented absolute times are ranking-only evidence; ranked-#1 family measured NEGATIVE while the ranking's LAST item produced the retained win.
7. `unset-control-arm-silent-default-parity` — unsetting the candidate env on a DEFAULT-ON candidate runs the same enabled code in both arms; the harness needs an explicit control value; validation passes control 0 / candidate 1 (the `--control-value` fix shipped in-source).

Riders (5): `device-side-argmax-minimal-download` (CPU-parity semantics + consumer-shaped gate + request-shape dual path; both lanes flagged it independently — convergent), `fusion-payback-gated-on-per-dispatch-overhead` (second-stage fan-in: 124,160 partials vs 61; fail-fast ordering; negative-recording contract), `static-seed-priority-bounded-to-observed-scale` (dynamic-producer twin; seam feeds TWO fields; carve-out premise recorded UNVERIFIED), `kv-dequant-materialization-batch-threshold-gate` (expansion-ratio axis: Q8 ~1.9× won, IQ2 ~6-8× lost at the same batch; round-identity facet), `full-vector-bit-identity-over-argmax` (two weak gates passed while full-logit caught 2.63 max-abs drift; MoE routing amplifies local rounding).

Negatives banked as rule context: unpack/materialization rejections, expert-reuse two-token kernels, expanded-IQ2 storage variants, the runtime-branch skip, the MTP-shape fusion regression.

Skip ledger: `0bb323a` (pure tuning constant), Q2-down 64-token crossover (twin mechanism covers), Q8 concat pair projection (grid-range-split-verbatim-impl-composition covers), gguf-tools provenance work (resume-revalidates-provenance family covers), `b4501c0` default-download flip (banked in ab-isolate notes), Makefile test targets (workspace-universal practice).

## Gates (all green, isolated `CARGO_TARGET_DIR=/tmp/batch129_target`, removed at landing)

- batch129 gates 4/4 FIRST-SHOT (rules_present + concepts_covered + pseudocode_parses under `kernel_opt`; rust_perf present_and_parse).
- lib default **1168/0**; kernel_opt **1493/0/1i**; kernel_opt+latent **1610/0/2i** (B128 floors 1167/1489/1606 + the 4 new tests).
- Retrieval re-run **byte-identical to B118–B128**: kernel 100/100/100, blended 94.7/96.9/97.7.
- **Router annotation re-pinned per the Issue 020 contract**: re-measured 88/131 = 67.2% (was 94/131 = 71.8% at the B128-era pins); floor 89/131 → **83/131** per the measured−5 Plan-030 convention; delta + cause recorded in `retrieval_eval.rs` (the 7 corpus adds shifted the TF-cosine centroids; exactly one annotation flipped).
- clippy 0 ×3 states (default / kernel_opt / kernel_opt+latent) — 6 doc_markdown findings in the new mod.rs doc block healed same-commit.
- score-bench **#78 FLAT** 96.2/1.9/100/100 (gen-1 SATURATED advisory unchanged).

## Count pins walked

Totals ×5 (tests.rs ×4 + router.rs) 570→575; per-category table C 45→46, H 27→28, L 14→15, M 18→19, N 41→42; KERNEL_CATEGORIES membership ×5; mod.rs doc history (Batch 129 line + the "570-rule corpus" comment); rust_perf header (:12 total, :13 io 16→18), :8325 total, annotated 55→57 ×4 sites; AGENTS.md ×3; README.md ×2 (B128 had missed README again — the unit-13 doc-sync gap class re-fixed here); .docs/11_domains both files; batch25 argmax FULL-STRING pin regenerated in prefix+rider form (the B125 lesson).

## Process notes (for the next batch)

- **Tool-mixing clobber (new lesson):** a shell `python3` write to mod.rs was silently reverted by a later `edit_file` on the same file (stale cached read) — the category inserts vanished while both `edit_file` changes survived; only the batch-gate run caught it. Rule: after ANY shell-side write to a file, do not `edit_file` that file in the same session without re-reading via shell first (or keep the file on one tool).
- The count-pin python insert needed a no-trailing-comma anchor case (the Fusion list's last id); the first run's regex silently mis-anchored and the printed success was unverified — the second run verified post-write (`VERIFIED all 5`).
- The batch gates run under `--features kernel_opt` (the domain is feature-gated; a default-state `cargo test -- batch129` reports a green zero for kernel-side gates — the docs-gate's green-zero rule, met again in the wild).
