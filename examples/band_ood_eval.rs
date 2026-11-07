//! Issue 217 T0 — the one-shot jaggedness eval driver: assemble the full
//! static population (corpus + rustc + kernel + the synth populations),
//! run [`band_ood::run_band_ood`], print the curve. NOT a harness row —
//! the T0 read is one frozen measurement recorded in the issue; this bin
//! is how it was produced (reproducibility), nothing more.

use riir_refine::draft::band_ood::{print_band_report, run_band_ood};
use riir_refine::draft::withheld_pair_ood::Binding;
use riir_refine::draft::{eval_synth, withheld_pair_ood};

fn main() {
    let mut population: Vec<Binding> = Vec::new();
    population.extend(withheld_pair_ood::clippy_population());
    population.extend(withheld_pair_ood::rustc_errors_population());
    population.extend(withheld_pair_ood::kernel_opt_population());
    population.extend(eval_synth::clippy_synth_population());
    // The perf synth population, built the same way `clippy_synth_population`
    // builds its bindings (gold = the owning rule; provenance `synth`).
    for s in eval_synth::perf_synth_spans() {
        population.push(Binding::clippy(s.rule, s.code, "synth"));
    }

    let started = std::time::Instant::now();
    let report = run_band_ood(population);
    let elapsed = started.elapsed();

    println!("{}", print_band_report(&report));
    eprintln!("run_band_ood wall: {elapsed:.1?}");
}
