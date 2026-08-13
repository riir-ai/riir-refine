# `perf_bench_demo` — the `rust_perf` GOAT gate

> Plan 001 Phase 2 promotion gate. Runs generated microbenchmarks for all
> benchable rules in the `rust_perf` corpus + reports Pareto verdicts.
> See [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) for
> the measured results.

## What it does

For each corpus rule:
- If it has a `BenchSpec`: generates a temp bin crate, runs a timing loop at
  multiple input sizes, reports per-size outcome (Faster/Slower/Tie) + an
  overall Pareto verdict (ParetoDominant / Mixed / ParetoDominated).
- If it has no `BenchSpec`: reports `NotBenchable` (honest — the rule needs
  an external runtime like Bevy or WASM).

Then aggregates the results into GOAT gates:
- **G1** (correctness): all benchable rules produce real measurements
- **G5** (Pareto verdict): majority of benchable rules are Pareto-dominant

## Run

```sh
# Clean temp dirs (forces fresh cargo builds)
rm -rf /tmp/riir_perf_bench/

# Run the GOAT gate (slow — compiles + runs 5 temp crates, ~5-15s each)
cargo run --example perf_bench_demo --features rust_perf_bench --release
```

## Feature requirements

- `rust_perf_bench` (implies `rust_perf`)
- `cargo` on PATH
- Release mode (`--release`) for accurate measurements

## See also

- [Bench 013](../../.benchmarks/013_rust_perf_bench_pareto_goat.md) — the GOAT results
- [`rust_perf.md`](../11_domains/rust_perf.md) — the domain doc
- [`perf_heal_demo.md`](perf_heal_demo.md) — the scaffold end-to-end demo
