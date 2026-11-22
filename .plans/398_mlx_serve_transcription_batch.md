# 398 — mlx-serve transcription batch (kernel_opt + rust_perf + sec)

**Status:** CLAIMED (zed-f50f3819@m3) — T0 in flight 2026-11-23. Research 432 is the controlling verdict. **RETURN VISIT: Research 274 / Batch 242 landed 31 rules @ `1015f78` (2026-10-14) — this batch is the DELTA batch; every candidate below survived the B242 dedup re-run (coordinator-verified 2026-11-22).**
**Claimed:** zed-f50f3819@m3, 1796463200, expires 2026-11-25T18:00:00+00:00

## Source

ddalcu/mlx-serve @ `2fe9be834a601c70ef1a597563026340f0e722fe` (tip 2026-11-21 "Inkling 2 Mini", MIT + Apache-2.0 third-party Metal kernels per NOTICE). Zig LLM server, Apple Silicon, wraps MLX + embedded llama.cpp. 621k LOC Zig + 184 `.metal` (11.3k lines) + 22 `.cu` + `docs/gotchas/engine-mlx.md` (243-entry incident book). Two-lane inspection + coordinator dedup done at verdict (2026-11-22, Research 432); the repo moved to "Inkling 2 Mini" the day before the verdict — T0 re-pin is load-bearing.

## Tasks

- [ ] T0 — re-clone `.raw/mlx-serve` at the tip, PIN it; **the delta input is `git log/diff 1015f7854fd8ee5c22e34993aaddee2443b9cfbf..TIP --stat`** (fetch with enough depth to cover since 2026-10-14 — ~589 commits / ~5,027 files by the verdict's date-approximated count; the shallow clone could not resolve the pin, disclosed in Research 432); re-verify EVERY anchor quoted in Research 432 (file:line + byte-exact quotes; anchors re-derived by `grep -n` per the atlas round-1 lesson); skim the NEW-since-pin surfaces (`docs/mlx-stream/`, new plugins) for candidates this verdict missed
- [ ] T0b — unread-residue reads owed at authoring: the candidate-bearing files only (moe/gqmv + gather_qmm_nax bodies for candidate 8's face decision; qsa_decode_* + row_attn_merge for the attention faces; `docs/reference.md` ANE rules — ane-domain seed note, NOT this batch; `docs/gotchas/server-http.md` for F10; the `−3.9%` GDN prefill-fold passage re-read (negative 12 direction check))
- [ ] T1 — dedup re-verification at the batch slot, BOTH against the landed-set drift since 2026-11-22 AND re-anchored on B242's set (`entries_gpu_97/_98/_99` + the B189/B276 riders); the owned-set adjudications IN-RULE both directions: gpu_20/26 ordinal-map (F1), entries_88 reference-round + Issue-940 (F4), gpu_34 kv-dequant (F5), B337 reuseaddr (F6), gpu_29 break-even-flip (F7 + candidate 3), sybil entries_84 specdec-composite (candidate 4), B242's `resource-limit-first-dispatch-probe-decline-latch` (candidate 4's sticky cross-ref), B242's `admission-clones-upstream-dispatch-for-bit-order` (candidate 8's threshold), B242's `bit-equal-fusion-mirrors-reference-reduction-regime` + the _99 merge family (candidate 10's contrast arm), entries_80 latency-histogram-split (F8), argv/browser-reachable/cors (F9/F10), cgroup-honest + reserve-then-commit (F11/F12)
- [ ] T2 — tranche A authoring (kernel_opt): candidate 1 `io-gpu-preemption-dispatch-budget` (the LEAD) · candidate 2 `constant-materializes-in-activation-dtype` · candidate 3 `measured-round-cost-spec-controller` · candidate 4 gate-normalization cluster (authoring decides 1-3 rules: normalization-by-drafts-per-round / arch-normalized MoE floor / sticky-vs-re-enable + the echo two-metric split)
- [ ] T3 — tranche B authoring (kernel_opt + rust_perf): candidate 6 `mxu-op-exactness-scoping` · candidate 7 `verify-width-named-scalars-supported-width-list` (adjudicate vs gpu_120 + gpu_44) · candidate 8 `expert-stream-once-pad-to-threshold` (FACE-OR-DROP: the threshold is OWNED by B242's admission-clones rule — only the pad-to-qualify remedy direction + 64-byte load datum adjudicate) · candidate 10 `exact-chunked-attention-dispatch-carry` (FACE-OR-THIN: the merge space is B242-owned; only the host-forced-split carry protocol + cache-drift rationale adjudicate, contrast-arm vs `bit-equal-fusion`)
- [ ] T4 — tranche C authoring (sec): candidate 9 `mdns-discovery-fed-egress-unauthenticated` (NEW FAMILY seed; regime must name the Rust discovery surfaces: mdns-sd/libmdns/Docker labels/K8s endpoint watches) · F10's unparsable-bind-host fail-open shape (face-or-thin-rule on the browser-reachable family)
- [ ] T5 — faces landing: F1, F4-F12, F14-F15 as count-neutral riders on their hosts (F2/F3 KILLED at revision — positive instances recorded, never re-landed); F9/F14/F15 positive instances RECORDED in-rule; F13 commit-floor/typed-commit-outcome (face-or-thin-rule decision front-loaded); the token-~55 sampling detail folded INTO the existing B242 rider's notes on `exact-bf16-domain-activation-lut` (never a standalone row); the qmv2 RE-SCOPED atoms (MMA-fragment-direct decode + group-boundary scale) folded into the named B242 rider (the half2 magic ladder → lop3)
- [ ] T6 — the measured-negative package folded as in-rule anti-regime/notes only (the 15 rows in Research 432; none is a rule)
- [ ] T7 — fixtures: standalone-compiled AND executed pre-entry, four-arm discipline per law (buggy/fixed/all-safe/tampered); original Rust reproductions — never paste Zig/Metal source (license hygiene); the candidate-5 fixture's G1 IS the 65536-pattern exhaustive equivalence; candidate 1's fixture models the work-unit budget split deterministically
- [ ] T8 — gates: clippy −D at the affected postures + lib tests + retrieval floor gate + corpus needles batch398 + fmt gate (never `cargo fmt -p`)
- [ ] T9 — count pins one-landing: `pins_manifest --write` + AGENTS + README + `.docs/11_domains` heads + health_views + the rust_perf mod.rs narrative head + the kernel-side witness + annotated pins (only the counts this batch actually moves)
- [ ] T10 — score-bench/freeze adjudication: zero clippy_lints delta expected — assert, never run-to-pass
- [ ] T11 — lane-intel filing (NOT corpus): the league-doc watch rows recommendation (Bonsai MLX posture via prism_hadamard_qwen35 + the qwen3.8 family +1) as a riir-ai league-doc edit proposal — a scored-row ADD stays the perf-rematch charter's owner call, recommendation "watch + reference rows first"; the int8-qk lever pointer to the prefill lane owner (issue 1146 family addendum or lane mail); the Qwen3.8-MTP-onto-Bonsai graft noted beside 1179's paused record — no lane re-open from this batch
- [ ] T12 — Research 432 delta section + queue-snapshot landing row + AGENTS/queue counts
- [ ] T13 — commit+push on develop (feat: prefix), teardown `.raw/mlx-serve` in the same session; the claim row lands when the lane starts

## Pre-registered expectations

- Firm rules: candidates 1, 2, 3, 4-cluster, 9. Contestable face-or-rule: 6, 7. Face-or-drop: 8, 10, F13. R5 KILLED at revision (B189 + the B242/B276 riders own it). Drops expected at T0/T1 — the verdict's grades carry the falsification history (~50% kill rate is normal).
- All entries suggestion-only `NotBenchable-by-domain` (`bench_spec: None`) — the Batch-136 annotation class; NO benchable GOAT exists in this source (criterion-1 domain miss recorded in Research 432).
- Outbound-PR lane: EMPTY by construction (every mechanism is their own landed decision).
- Yield restated: **4-7 rules + 8-10 faces** after falsification (the first draft's 6-10 + 8-12 was pre-B242-dedup).

## Notes

- The `ane` domain gets a follow-up seed only (T0b's `docs/reference.md` read) — no ane rule authoring in this batch.
- Candidate 9's rule text must carry the honest-disclosure contrast ("the host sees tunneled prompts in plaintext" is disclosed; the DESTINATION control is the missing piece) so the rule targets the class, not this repo's UX.
- The vendor attributions ride the rules' CITE lines (oMLX/MTPLX/MLX-steel Apache-2.0; mlxfast-bonsai2/mlxfast-challenge MIT) per NOTICE.
