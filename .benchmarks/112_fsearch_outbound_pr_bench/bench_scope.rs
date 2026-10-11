//! Scoped index build for the riir-refine outbound-PR bench (UNTRACKED harness).
//!   bench_scope <root> <out-index.bin> [probe]
use fsearch::index::Index;
use fsearch::walk;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a[0] == "probe" {
        let idx = Index::load(std::path::Path::new(&a[1])).expect("load");
        eprintln!("probe: n={} u={} bytes={}", idx.n, idx.u, idx.bytes());
        return;
    }
    let t = std::time::Instant::now();
    let ls = walk::scan(a[0].as_bytes(), 8);
    let entries: usize = ls.iter().map(|l| l.ents.len()).sum();
    eprintln!("scan: {} listings, {} entries in {:?}", ls.len(), entries, t.elapsed());
    let idx = Index::build(ls, 1, 0, b"/Users/katopz");
    eprintln!("build: n={} u={} bytes={} in {:?}", idx.n, idx.u, idx.bytes(), t.elapsed());
    idx.save(std::path::Path::new(&a[1])).expect("save index");
    eprintln!("saved {} (total {:?})", a[1], t.elapsed());
}
