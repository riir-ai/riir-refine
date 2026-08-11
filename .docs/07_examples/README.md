# 07 — Examples

> **Purpose:** the 4 in-tree examples. Each demonstrates one layer of the
> pipeline or one GOAT gate.

## Files

| File | Scope |
|---|---|
| [`heal_demo.md`](heal_demo.md) | `examples/heal_demo.rs` — end-to-end L0→L1→L2 demo |
| [`clippy_verify_demo.md`](clippy_verify_demo.md) | `examples/clippy_verify_demo.rs` — Phase 3 GOAT gate |
| [`ruliology_search_demo.md`](ruliology_search_demo.md) | `examples/ruliology_search_demo.rs` — L3 GOAT gate |
| [`self_evolve_real_world_demo.md`](self_evolve_real_world_demo.md) | `examples/self_evolve_real_world_demo.rs` — self_evolve G6/G7 GOAT gate |

## Source

- [`examples/heal_demo.rs`](../../examples/heal_demo.rs)
- [`examples/clippy_verify_demo.rs`](../../examples/clippy_verify_demo.rs)
- [`examples/ruliology_search_demo.rs`](../../examples/ruliology_search_demo.rs)
- [`examples/self_evolve_real_world_demo.rs`](../../examples/self_evolve_real_world_demo.rs)

## Why GOAT gates live in `examples/` (not `benches/`)

The Phase 3 + L3 GOAT gates live in `examples/` to avoid the `benches/`
auto-target conflict with feature gates. `benches/` targets are
auto-discovered and require all their dependencies to be in the default
feature set; `examples/` with `required-features` are only compiled when
their feature is enabled, which is exactly the opt-in semantics we want
for the GOAT gates.

The hot-path benches (L1 + L2) DO live in `benches/` because they have
no feature gates — they're always-on.

## See also

- [`../08_benchmarks/`](../08_benchmarks/README.md) — GOAT gate methodology
