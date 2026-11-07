//! Cost probe — population size + one `RuleIndex` build time (T0 feasibility).

use riir_refine::draft::corpus_index::{RetrievalConfig, RuleIndex};
use riir_refine::draft::eval_synth;
use riir_refine::draft::withheld_pair_ood::Binding;
use riir_refine::draft::{band_ood, withheld_pair_ood};

fn main() {
    let population: Vec<Binding> = {
        let mut p: Vec<Binding> = Vec::new();
        p.extend(withheld_pair_ood::clippy_population());
        p.extend(withheld_pair_ood::rustc_errors_population());
        p.extend(withheld_pair_ood::kernel_opt_population());
        p.extend(eval_synth::clippy_synth_population());
        for s in eval_synth::perf_synth_spans() {
            p.push(Binding::clippy(s.rule, s.code, "synth"));
        }
        p
    };
    // Distinct-text dedup — the protocol's own law.
    let raw = population.len();
    let mut seen = std::collections::BTreeSet::new();
    let pop: Vec<Binding> = population
        .into_iter()
        .filter(|b| seen.insert(*blake3::hash(b.text.as_bytes()).as_bytes()))
        .collect();
    eprintln!("population raw={raw} distinct={}", pop.len());

    for band in band_ood::Band::ALL {
        let n = pop
            .iter()
            .filter(|b| band_ood::Band::of(&b.text) == band)
            .count();
        eprintln!("{:?} members={n}", band);
    }

    for size in [600usize, 800, 1000, 1200] {
        let started = std::time::Instant::now();
        let patterns: Vec<(String, String)> = pop
            .iter()
            .take(size)
            .map(|b| (b.rule.clone(), b.text.clone()))
            .collect();
        let _idx = RuleIndex::build_with(patterns, RetrievalConfig::default());
        eprintln!("build({size}) wall: {:.1?}", started.elapsed());
    }
    let started = std::time::Instant::now();
    let patterns: Vec<(String, String)> = pop
        .iter()
        .map(|b| (b.rule.clone(), b.text.clone()))
        .collect();
    let _idx = RuleIndex::build_with(patterns, RetrievalConfig::default());
    eprintln!("build(all) wall: {:.1?}", started.elapsed());
}
