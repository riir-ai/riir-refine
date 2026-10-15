# Plan 239 — the P58–P61 conversion batch (Batch 237)

**Status:** LANDED 2026-10-15 — Batch 237 (3 rust_perf rules + 1 count-neutral kernel_opt rider; landing commit: this commit; slice B's entry pre-landed `0c71bea2`, slice C's rider pre-landed `6b00330c`)

Convert the four graded post-mining intake items into corpus rules/riders. Every quote
re-verified at the pin BEFORE landing; homes adjudicated at claim per each row's own note;
`.raw` clones removed at close. Batch 236 stands at corpus 813 — counts move only for new rules.

- [x] T0 — sources at pins: clone `ggml-org/llama.cpp` (`685f7089` for P58, `5cdadfc3` for P59), `incoai/splash` (`ffc2d4be` for P60), `tracel-ai/cubecl` (`5a4f5882c` for P61) into `.raw/`; verify each pin exists + read the diffs; verify licences (P58/P59 MIT, P60 Apache-2.0, P61 dual Apache-2.0/MIT — verified at the clone root)
- [x] T1 — P58 home adjudication + quote verification: OWN rust_perf rule `f16-to-f32-accumulator-lut-breaks-conversion-serialization` (simd; the register-dependency axis of the LUT-boundary family — distinct-from the GPU constant-cache/density axes; quotes verbatim, zero drift)
- [x] T2 — P59 home adjudication + quote verification: OWN rust_perf rule `work-buffer-sized-by-execution-width-not-plan-tasks` (concurrency; adjacent-not-covering B223's largest-consumer workspace law — plan-vs-execution WIDTH is a different mismatch; quotes verbatim, zero drift)
- [x] T3 — P60 quote verification: OWN rust_perf rule `discretionary-cache-growth-keys-on-spare-not-available` (io; all 8 verbatim quotes + ~40 file:line refs verified at the clone; the RUNTIME growth face — the Batch-232 HEADROOM-ADMISSION rider owns the LOAD-time face)
- [x] T4 — P61 rider authoring: the autotune-cache hardware-path axis as a count-neutral rider on B223's `autotune-fidelity-synthetic-carries-producer-params` (landed in `6b00330c` with 8 verbatim needles)
- [x] T5 — author the rules/riders (three authoring subagents over disjoint slices: A = entries_25 P58+P59, B = entries_26 P60, C = the kernel_opt rider; coordinator wired mod.rs + pins + batch needle tests)
- [x] T6 — gates: batch237 rust_perf parse+needles 2/2 first shot; kernel_opt batch237 rider needles 1/1 (in `6b00330c`); default lib 2668/0; kernel_opt lib 3119/0 (1 ignored documented); kernel_opt+latent_retrieval lib 3305/0 (3 ignored documented, 2550 s); clippy -D ×3 postures clean (6 doc_markdown mechanicals in the new slice headers healed by cargo refine, compile-gated, 12,037 tokens); freeze/score-bench NOT owed (zero clippy-corpus delta, P30 — the write set is rust_perf + kernel_opt only, verified)
- [x] T7 — land: rust_perf 198→201 in AGENTS.md ×2 + README + .docs/11_domains/rust_perf.md; kernel_opt count-pin chains carry the Batch-237 count-neutral note (tests.rs ×4 sites); `.distill/001` landing row rides the batch commit
- [x] T8 — close: `.raw/llamacpp-b237` + `.raw/splash-b237` + `.raw/cubecl-b237` removed; plan status → LANDED
