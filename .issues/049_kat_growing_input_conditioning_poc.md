# Issue 049: KAT/CF-arm growing-input conditioning POC (Meta^n-guided)

**Status:** OPEN — POC filed 2026-08-27 from katgpt-rs Research 512 (arXiv:2608.24735 distillation)

## Background

[katgpt-rs Research 512](../../katgpt-rs/.research/512_Meta_n_Fixed_Omega_Growing_Input_PASS.md) distilled **Meta^n: Recursive Self-Improvement through Emergent Depth** (Kim et al., arXiv:2608.24735). The paper's core reframe — *"the gain comes from giving the improver more to read, not from rewriting the improver"* — plus its decisive ablation (**inter-layer conditioning carries ~72% of recursion gain; callable code transfer ~15%**) lands on a measured gap in our one live LLM-improver lane:

- **Today** `DaemonL4::generate_fix` sends exactly `TRAINED_INSTRUCTION + buggy_code` (`src/l4_daemon.rs` L199-202). No trajectory context, no proposer source, no prior outcomes. The growing archive (TrajStore / LatentFixMemory) feeds only the modelless selection path — **it never reaches the LLM**. The improver's input is fixed-shape; Meta^n's is strictly-growing, and that is where it locates the gain.
- **The Bonsai adapter arm is NOT the target.** Its minimal prompt is a distribution constraint (all 1092 v2edit rows share one lint-agnostic instruction line, `l4_daemon.rs` L23-27) and stays untouched. Enriching it would go off-distribution.
- **The CF/general-LLM arm has no such constraint.** Issue 040's `request_prompt` already builds caller-rich prompts (instruction + module + modelless candidate + few-shots), and its **arm-B pre-pass beat arm A on every partial-credit axis** (v2edit slice 18 vs 12 keeps, parse 20/20 vs 15/20, merge 19/20 vs 13/20). That is our own single data point in Meta^n's conditioning-dominance direction.

## The POC

Extend the CF-arm prompt with **growing input** (three Meta^n-derived channels, all readable from existing stores — zero new infrastructure):

1. **Prior trajectory outcomes** for the same rule/span — top-k from TrajStore, `Certified` first, `Withdrawn` excluded (trust-weighted retrieval already exists).
2. **The modelless candidate(s) already proposed** for the span — the "code that produced them" channel (what the proposer emitted + why it declined/reverted, where recorded).
3. **A cross-span failure-pattern summary** — the depth≥3 structured-summary analog (per-rule failure-pattern distribution + 2-3 representative traces), the paper's highest-leverage conditioning form.

**Measure:** strict-keep rate + partial-credit axes on Issue 040's exact 9-span set + baselines (arm A raw, arm B pre-pass). Never-wrong contract unchanged (0 divergent / 0 regressions or the arm fails).

**Guards (all existing, unchanged):** `contrastive_scope` input gate declines garbage before the LLM; the Issue-030 rule-coverage relevance gate declines out-of-corpus rules; KAT leak-posture sign-off is the owner's separate gate and is NOT affected by this issue.

## Tasks

- [ ] T1: Build the growing-input prompt composer for the CF arm (channels 1-3 above) behind a flag/env so arm A / arm B remain reproducible baselines
- [ ] T2: Run the POC on Issue 040's span set; record strict-keep + partial-credit vs arm A/B in a bench note (`.benchmarks/`)
- [ ] T3: Verdict: material gain → wire as the KAT fixer's default input contract (Proposal 005 lane); no gain → record the negative result + close (the conditioning finding is then regime-bound to Meta^n's cross-task scale, honestly noted)
- [ ] T4: If T3 lands positive, propagate one line to katgpt-rs Research 512's re-evaluation guard (upgrading the finding's weight) + this issue's commit hash back into the note
- [-] T5 (deferred): depth/iteration — feeding the CF arm its OWN prior outputs across calls (true Ω-recursion) is out of scope until single-call growing-input shows gain; our loop is per-span one-shot today

## Non-goals

- No change to the Bonsai adapter arm or its distribution-frozen prompt.
- No new selection machinery (BetaPosterior stands; the 039 T5 report-only verdict for ELO stands).
- No per-task-best Pareto archive over fix chains (no consumer; re-file against 039 if one appears).
