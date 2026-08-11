# HealPipeline — self-evolving heal composition

The top-level composition that wraps the frozen L0–L2 pipeline (drafter +
pruner) with the latent memory + evolution layers.

## Flow

```text
heal_with_evolution(lint, code):
  1. Query memory for past trajectories (top-K).
  2. Draft a fix using the existing L1 drafter.
  3. Collect candidates: [drafter output] + [memory's past fixes].
  4. Score each candidate via LatentSkillEvolution (dot + sigmoid).
  5. Validate the top-scoring candidate with the L2 pruner.
  6. Return the result. Caller reports outcome → feeds back to memory + evolution.
```

## Additive design

The frozen layers (L1 drafter, L2 pruner) are **NEVER mutated**. The
evolution layer is a **reader-side overlay**: the evolved direction is
used to *score* candidates, not to modify the frozen ternary weights.

This preserves:
- The modelless mandate (no weight mutation at runtime)
- The freeze/thaw invariant (base snapshot is immutable; evolution is a
  versioned overlay)
- The L0–L2 pipeline's GOAT gates (they still pass because they're
  unchanged)

## Candidate scoring

Each candidate gets a composite score:

```text
score = W_EVO · evolution_score + W_RATE · success_rate
```

Where:
- `W_EVO = 0.6` — weight on the evolution layer's dot+sigmoid score
- `W_RATE = 0.4` — weight on the candidate's historical success rate
- `evolution_score = sigmoid(dot(evolved_direction, candidate_direction) / temperature)`
- `success_rate` = trajectory's net success rate. For memory candidates this
  is the trajectory's own stats. For drafter candidates, it is looked up via
  `LatentFixMemory::success_rate_for(lint_name, fixed_code)` (Issue 008) —
  returns `0.5` neutral if the pair has no trajectory, or the actual rate
  if it does. This lets fail feedback bias selection away from repeatedly-
  failed drafter picks.

The highest-scoring candidate that passes the L2 pruner is returned.

## API

```rust
pub struct HealPipeline {
    drafter: ClippyTernaryDrafter,      // frozen L1
    pruner: RustAnalyzerPruner,         // frozen L2
    memory: LatentFixMemory,            // grows with use
    evolution: LatentSkillEvolution,    // reader-side overlay
    retrieve_scratch: Vec<usize>,       // reused (alloc-free after warm-up)
    retrieve_out: Vec<ScoredTrajectory>, // reused (alloc-free after warm-up)
}

impl HealPipeline {
    pub fn new(drafter: ClippyTernaryDrafter, pruner: RustAnalyzerPruner) -> Self;
    pub fn heal(&mut self, lint_name: &str, buggy_code: &str,
                candidates: &[ClippyFixEntry]) -> Option<HealResult>;
    pub fn record_outcome(&mut self, lint_name: &str, fixed_code: &str, success: bool);
}
```

## HealResult

```rust
pub struct HealResult {
    pub fixed_code: String,       // the proposed fix
    pub parses: bool,             // passed L2 syntax pruner?
    pub evolution_score: f32,     // the evolved direction's score (0.0-1.0)
    pub source: HealSource,       // Drafter or Memory
}

pub enum HealSource {
    Drafter,  // the L1 ternary drafter proposed this fix
    Memory,   // a past fix trajectory was retrieved + reused
}
```

## Degeneration

When `memory` is empty (no past trajectories):
- Step 1 returns no trajectories.
- Step 3 collects only the drafter's output.
- Step 4 scores it with the neutral `success_rate = 0.5`.
- The pipeline degenerates to the existing L0–L2 path.

This means the `self_evolve` feature is safe to enable even with no
training data — it just acts as the baseline pipeline until outcomes are
recorded.

## GOAT gate results

| Gate | Target | Measured | Status |
|---|---|---|---|
| G1 (retrieval relevance) | ≥ 80% | 100% | ✅ PASS |
| G2 (retrieval latency) | < 50µs p50 @ N=1000 | 6µs p50 | ✅ PASS |
| G3 (no-regression) | tests pass | 35 off / 55 on | ✅ PASS |
| G4 (alloc-free) | capacity stable | stable across 100 heals | ✅ PASS |
| G5 (self-evolution gain, oracle) | ≥ 5pp improvement | **70.0pp** (30% → 100%) | ✅ PASS |
| G6 (corpus truthfulness, real clippy) | 10/10 entries verified | 10/10 | ✅ PASS |
| G7 (self-evolution gain, real clippy) | ≥ 5pp improvement | **50.0pp** (50% → 100%) | ✅ PASS |

G5 uses a controlled fixture with oracle feedback; G7 confirms the
mechanism generalizes to real `cargo clippy` feedback. The gap (70pp vs
50pp) is explained by G7 using 2 candidates per lint (higher baseline)
vs G5's 4 candidates. Both validated in [Bench 008](../../.benchmarks/008_real_world_validation.md).

`self_evolve` stays opt-in — not a correctness concern, but a deployment
constraint (`clippy_verify` is itself opt-in: slow, requires cargo on PATH).

## See also

- [`latent_fix_memory.md`](latent_fix_memory.md) — the memory layer
- [`latent_skill_evolution.md`](latent_skill_evolution.md) — the evolution layer
