# LatentSkillEvolution — EMA direction-vector update

Maintains per-lint evolved "preference directions" that represent what
kind of fix tends to work for each lint. Updates via EMA (exponential
moving average) on fix outcomes.

## The evolution rule

```text
On success: direction += α · (fix_direction - direction)   // move toward
On fail:    direction -= β · (fix_direction - direction)   // move away
```

Where:
- `α` (success learning rate) = 0.1 — default, same as
  `CommittedFieldBlend`'s α (katgpt-rs Plan 321)
- `β` (fail unlearning rate) = 0.05 — smaller than α because failures
  are less informative (a success confirms the fix works; a failure
  only says "this fix didn't work this time", which could be due to
  many causes)

## Why EMA, not gradient descent?

EMA is **modelless**: pure vector arithmetic, no loss function, no
gradient, no backprop. This satisfies the modelless-first mandate.

The update is:
- **Deterministic**: same outcomes → same evolved direction
- **Bounded**: the direction stays in the convex hull of observed fix
  directions (EMA is a weighted average)
- **Alloc-free on the read path**: lookup + arithmetic only

## Freeze/thaw invariant

The base drafter weights are **NEVER mutated**. This evolution layer is a
**reader-side overlay**: the evolved direction is used to *score*
candidates, not to modify the frozen ternary weights.

This preserves:
- The modelless mandate (no weight mutation at runtime)
- The freeze/thaw invariant (base snapshot is immutable; evolution is a
  versioned overlay)

## Init-on-success-only rule

A lint's evolved direction is initialized **only on the first success**
—not on insert, not on failure. This prevents the evolution from
biasing toward a direction that has never been confirmed to work.

Before the first success, `score_candidate_into` returns 0.5 (neutral)
for that lint — candidates are scored by success rate only.

## EvolveConfig

```rust
pub struct EvolveConfig {
    pub alpha: f32,        // success learning rate (default 0.1)
    pub beta: f32,         // fail unlearning rate (default 0.05)
    pub temperature: f32,  // sigmoid temperature (default 1.0)
}
```

## API

```rust
pub struct LatentSkillEvolution {
    directions: HashMap<String, Direction>,
    config: EvolveConfig,
}

impl LatentSkillEvolution {
    pub fn new(config: EvolveConfig) -> Self;
    pub fn record_success(&mut self, lint_name: &str, fix_direction: &Direction);
    pub fn record_fail(&mut self, lint_name: &str, fix_direction: &Direction);
    pub fn score_candidate_into(&self, lint_name: &str,
                                candidate_direction: &Direction,
                                out: &mut f32);
}
```

## Scoring

```text
score = sigmoid(dot(evolved_direction, candidate_direction) / temperature)
```

- `evolved_direction` = the per-lint EMA-updated direction (or absent →
  neutral 0.5)
- `candidate_direction` = the BLAKE3 embedding of the candidate's
  `(lint_name, fixed_code)`
- `temperature` controls sharpness: lower = sharper preference

Higher score = the evolved direction favors this candidate.

## Pattern source

Borrows the modelless EMA direction-update pattern from:
- `CommittedFieldBlend` (katgpt-rs Plan 321) — the α/β asymmetric
  learning rate
- `PersonalityWeightedComposition` (Plan 297) — the per-key direction
  overlay shape

Reimplemented locally (~310 LOC) because riir-refine cannot depend on
katgpt-rs beyond `katgpt-core` (the evolution pattern isn't in
`katgpt-core`).

## See also

- [`heal_pipeline.md`](heal_pipeline.md) — how the evolution is composed
- [`../09_lessons/modelless_first.md`](../09_lessons/modelless_first.md) — why EMA, not gradient descent
