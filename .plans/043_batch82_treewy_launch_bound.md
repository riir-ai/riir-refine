# Plan 043 — Batch 82: TreeWY + Launch-Bound kernel distill

**Status:** IN PROGRESS — executing 2026-08-28 (verdict: Research 096)

## Source

Research 096 (`.research/096_treewy_launch_bound_distill.md`): arXiv:2608.20961
(TreeWY) + arXiv:2608.26612 (Launch-Bound). Both fully read; dedup grep clean;
3 rule candidates GOAT-lead ranked.

## Tasks

- [ ] T1 Author 3 corpus entries in `entries_gpu_08.rs` (rules 394–396) via
      `target/gen_b82.py` (rustlit()-escape path, needle + anchor assertions):
      `wy-transform-tree-verify-state-free` (G), `work-scaling-probe-launch-bound`
      (D), `capture-break-count-not-perf-target` (E)
- [ ] T2 Wire `KERNEL_CATEGORIES` slices (D, E, G) + walk the category-count
      comments (D 82→83, E 23→24, G 22→23)
- [ ] T3 Standing gates in `tests.rs`: `batch82_rules_present` +
      `batch82_concepts_covered` + `batch82_pseudocode_parses`
      (`#[cfg(not(feature = "kernel_verify"))]`)
- [ ] T4 Walk all count pins: tests.rs 393→396 (5 sites + the Plan-003
      expected-counts table), mod.rs header + doc, router.rs (3 sites),
      entries_gpu_08.rs header
- [ ] T5 GOAT: G1 kernel_heal lib (new gates + corpus counts) · G2 default
      lib · G3 all-features + clippy both states + CI guard layers ·
      retrieval floors (kernel_heal,latent_retrieval) HELD
- [ ] T6 Docs: Research 096 §6 batch record (measured numbers), AGENTS.md
      Batch 82 record + corpus counts, `.docs/11_domains/kernel_opt.md`,
      `.distill/001_mining_queue_snapshot.md` queue entry
- [ ] T7 Commit + push (re-scan snapshot folder at write time; sibling is
      active on dashboard/Issue-050 territory — disjoint files)

## Non-goals

- No proposers (kernel_opt rules are retrieval/suggestion-grade).
- No rust_perf bench (rules are kernel_opt domain).
- The substitutability/router-exemption findings stay research-record-only
  (no bounded fix-space — Research 096 §3.3/§7).
