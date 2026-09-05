# Plan 087 — Batch 114: oMLX ANE-prefill + GDN blocked-sequential distill (Research 138)

**Status:** OPEN — plan-first per the distill-skill discipline; verdict YES B+ in `.research/138_omlx_ane_gdn_prefill_distill.md`. No corpus edit before T0–T1.

**Source pin:** jundot/omlx @ `e467261edc786efd33b1e9023d5c4a827f8aa1c1` (main, 2026-09-03, Apache-2.0). Surfaces: `omlx/patches/qwen35_ane_prefill.py` (3,604 L), `omlx/custom_kernels/qwen35_prefill/gdn.py` (599 L). `.raw/omlx` removed at batch commit — re-clone at the pin for any quote re-verification.

## Tasks

- [ ] **T0 — dedup re-verification (mandatory pre-edit).** Re-run the two-vocabulary greps from Research 138 against the CURRENT corpus (it moved since the verdict): `reformulat|blocked.seq|half the FLOPs`, `jetsam|address window|resident program|bank split`, `recurrent.safe|token.local|state horizon`, `shuffle_down|leader broadcast|cooperation radius`, `delivered chunk|compile.*never.*execute`. One evidence line per candidate; kills recorded in the batch record.
- [ ] **T1 — entry authoring (2–4 rules, order by grade).** Lead `recurrence-prefill-exact-restructure-before-reformulation` (G, Kernel S + the ×3 reformulation-loser evidence, ⚠ source-reported M3-Ultra numbers); `ane-bank-split-ladder` (D, near-halves ladder + async-release gc/settle + jetsam fraction gate + bank-count determinism); then per post-dedup survival: `recurrent-safe-approximate-slice-cap` (B84 extension rider OR thin rule), `sub-simdgroup-cooperation-radius` (B22/B95 rider), `program-shape-vs-delivered-chunk-coupling` (thin rule or B107 rider), tail-padding-profitability + recurrence-boundary tiling (B84 riders), B79 vectorized-vs-scalar staging-pad boundary rider. ORIGINAL fixtures only (no oMLX source verbatim); Metal-flavored pseudocode for the G rule, host-orchestration pseudocode for the D rules.
- [ ] **T2 — standing gates.** `batch114_rules_present` + `batch114_concepts_covered` + `batch114_pseudocode_parses` (cfg(not(kernel_verify)) per the B70 convention); any ported numeric construction gets an exhaustive/boundary equivalence gate (none expected — no numeric construction is being ported; record if that changes).
- [ ] **T3 — count pins walked.** corpus total (516 → N), category expected-array (D at minimum; G if rule 1 lands in G), sums x2, grouped, router expert total, mod.rs doc line + category comments + member-list tails; `.research/.highwater` already bumped to 138 by this note; `.plans/.highwater` bump to 87 rides this plan's commit.
- [ ] **T4 — GOAT.** batch gates 3/3 + kernel_heal lib + default lib + kernel_heal+latent floors HELD (zero re-pins expected) + clippy 0 in the affected states + count-pin suite green. GPU-exclusive not required (CPU-only unit).
- [ ] **T5 — lane-intel coupling (the axis the qwen38-recipe miss added).** The batch record names the consumer rows: riir-ai Issue 876 (the prefill-first M3 queue) carries T-probes pointing at rules #1/#2; Issue 769 T2/R1/R4a/R5/R6/R7 get the located-implementation references (issue-update is the M3-queue lane's job, NOT this batch's edit surface).
- [ ] **T6 — docs + queue + commit.** AGENTS.md Batch 114 entry (corpus counts, pins walked, gate results, the lane-intel paragraph); queue snapshot line (Last landed); `.docs/11_domains/kernel_opt.md` batch row if the file carries batch history; `rm -rf .raw/omlx` at the batch commit (pin recorded in Research 138); commit `feat: batch 114 — omlx ane/gdn prefill distill (research 138 / plan 087)` on develop, push.

## Non-goals

- No corpus batch for the qwen38-recipe repo itself (MARGINAL corpus verdict stands; its lane line lives in the queue snapshot).
- No riir-ai edits from THIS repo beyond the queue-snapshot lane line — Issue 876/769 updates belong to the M3-queue lane (riir-ai-side session).
- No adoption work (bank ladder, recurrent-safe caps) inside riir-clippy — the corpus records the patterns; adoption is riir-gpu/769 territory.
