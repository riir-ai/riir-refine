# Plan 239 — the P58–P61 conversion batch (Batch 237)

**Status:** IN FLIGHT — claimed 2026-10-15 (M3, idle-loop Decision-ordered unit; 4090 unreachable → M3 owns the mining lane; claim row in `.distill/001` this commit)

Convert the four graded post-mining intake items into corpus rules/riders. Every quote
re-verified at the pin BEFORE landing; homes adjudicated at claim per each row's own note;
`.raw` clones removed at close. Batch 236 stands at corpus 813 — counts move only for new rules.

- [ ] T0 — sources at pins: clone `ggml-org/llama.cpp` (`685f7089` for P58, `5cdadfc3` for P59), `incoai/splash` (`ffc2d4be3` for P60), `tracel-ai/cubecl` (`5a4f5882c` for P61) into `.raw/`; verify each pin exists + read the diffs; verify licences (P58/P59 MIT, P60 Apache-2.0, P61 Apache-2.0/MIT per repo)
- [ ] T1 — P58 home adjudication + quote verification: the LUT-vs-intrinsic accumulator-serialization law (ggml-cpu f16→f32 lookup kept on x86 with F16C); decide own rust_perf rule vs rider on the lut-boundary family; verify quotes verbatim at the pin
- [ ] T2 — P59 home adjudication + quote verification: the plan-width-vs-execution-width buffer-sizing law (set_rows conversion buffer sized by the planner's task count while the op parallelizes across all threads); decide small own rule vs rider on the B223 workspace-sizing family
- [ ] T3 — P60 quote verification: the spare-not-available cache-growth law (splash memory governor: discretionary cache keys on spare memory — free + file-backed + purgeable + half compression credit, ≤ ¼ anonymous — not the OS available estimate); own rust_perf rule (io/memory)
- [ ] T4 — P61 rider authoring: the autotune-cache hardware-path axis (tune cached under one adapter/API pair misprices on another) as a count-neutral rider on B223's `autotune-fidelity-synthetic-carries-producer-params`
- [ ] T5 — author the rules/riders (disjoint slices, subagents where useful); entry files + LINT_GROUPS + dispatch + FIXABLE/VERIFY tiers per landing convention; probes per Batch-33 discipline where a divergence class is claimed
- [ ] T6 — gates: parse+needle gates for new entries; domain lib tests; clippy `-D` at the landing postures; retrieval floors check (kernel_opt class); freeze/score-bench NOT owed if zero clippy-corpus delta (P30) — verify that premise
- [ ] T7 — land: counts/AGENTS.md rows for whatever moved; `.distill/001` landing row (claim discharged); `.plans/.highwater` bump consumed at creation
- [ ] T8 — close: `.raw` clones removed; plan status → DONE with the landing commit hash
