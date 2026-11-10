# Plan 359: Batch 348 — the P111 approx_constant + P112 useless_vec clippy_lints corpus batch

**Status:** LANDED 2026-11-10 (zed-amo-b348) — corpus 118→120, all gates green (Research 392)
**Origin:** `.distill/001_mining_queue_snapshot.md` intake rows P111 (2026-10-31, the riir-shader idle sweep's oracle-anchored lane) + P112 (2026-11-01, the seal-render clippy sweep) + P94's probe record (2026-10-28).
**Lesson applied:** P120 — the intake premise is probed BEFORE authoring; P112's row itself orders the triage ("probe whether P21's verbatim-oracle lane already covers it").

- [x] T0 — claim row in `.distill/001_mining_queue_snapshot.md` + this plan + highwater bump, committed + pushed BEFORE authoring (the push is the arbiter).
- [x] T1 — probe-first (temp fixture crates under /tmp, removed at close): `approx_constant` (group, message format, suggestion children — re-verify P94's matrix at the current clippy: deny-by-default, NO suggestions, message names the target const) and `useless_vec` (group, message, suggestion applicability + the exact replacement text for `&vec![a, b]` arg sites and let-bound read-only vecs; the `vec![x; n]` repeat-syntax shape; the non-`&`-borrowed shape).
- [x] T2 — author: `approx_constant` proposer (literal → fully-qualified `std::f64::consts::*` / `std::f32::consts::*` path; bit-identical value precondition, else decline; the ~13-const clippy table; divergence disclosure: `std::` vs `core::` on no_std crates) + `useless_vec` proposer (the bounded mechanical half per the probe verdict; scope law: only shapes the probe proves; refuse non-const repeat lengths) + corpus entries (next free entries file, re-derived at landing) + lint_level LINT_GROUPS registration + dispatch + FIXABLE tier + batch348 pseudocode-parse + parity/needle tests (both buggy forms fire under real clippy, healed forms silent).
- [x] T3 — freeze-table regeneration (clippy_lints delta — the gen2/gen3 `#[ignore]`d e2es re-run; labels/exclusions land as data per plan 137 T7.5; the P119 gen6/gen7 rosters stay as landed).
- [x] T4 — retrieval gates: BOTH lanes (kernel self-variants + blended floors at `latent_retrieval`) + the default-posture regression gate (Issue 170 T3). A decay fails the batch → re-measure + re-pin with the delta recorded, never silent.
- [x] T5 — score-bench append (`--score-bench`; floors never weakened silently) + frontier snapshot.
- [x] T6 — count pins: `pins_manifest.py --write` + AGENTS.md + README.md + `.docs/11_domains/` (corpus 118→120, fixable count) + health_views if it names the counts.
- [x] T7 — release binary rebuild at the pinned release set (corpus delta compiles into the shipped binary; `cargo refine --version` build hash == HEAD) + §0 bin-age.
- [x] T8 — Research 392 (probe record + authoring + gate results), plan ticks, snapshot landing row, dual_allocation gate, `feat: batch 348: ...` commit + push (named-file staging only; the 18-file fmt-churn class never staged).
