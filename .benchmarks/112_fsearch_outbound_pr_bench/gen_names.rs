//! Name-side corpus generator for the riir-refine outbound-PR bench (UNTRACKED).
//! Replicates examples/rig.rs `gen_corpus`'s name + typing sections VERBATIM
//! (same xorshift seed, same draw order — names/typing are drawn before the
//! content section, so the sampled queries are identical to the stock rig's),
//! minus the content/grep half: the scoped bench snapshot carries no content index.
//!   gen_names <snap dir> <corpus.json>

use fsearch::index::Index;
use serde_json::json;
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    gen_corpus(Path::new(&a[0]), &a[1]);
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn gen_corpus(snap: &Path, out: &str) {
    let idx = Index::load(&snap.join("index.bin")).expect("index");
    let ne = idx.name_ents_off();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let text = |k: usize| String::from_utf8_lossy(idx.uname(k as u32)).into_owned();
    let mut names: Vec<(String, String)> = Vec::new();
    let mut push = |cat: &str, q: String| names.push((cat.into(), q));

    let mut targets = Vec::new();
    while targets.len() < 60 {
        let k = rng.below(idx.u);
        let n = text(k);
        let letters = n.chars().filter(char::is_ascii_alphabetic).count();
        let hexy = n.as_bytes().windows(8).any(|w| w.iter().all(u8::is_ascii_hexdigit));
        if ne[k + 1] - ne[k] == 1 && (6..=32).contains(&n.len()) && n.is_ascii() && !n.contains(' ') && letters * 10 >= n.len() * 7 && !hexy {
            targets.push((k, n));
        }
    }
    for (_, t) in &targets {
        push("exact", t.clone());
    }
    for (i, (_, t)) in targets.iter().enumerate() {
        let mut s: Vec<char> = t.chars().collect();
        let letters: Vec<usize> = (1..s.len()).filter(|&i| s[i].is_ascii_alphabetic()).collect();
        let p = letters[rng.below(letters.len())];
        match i % 4 {
            0 if p + 1 < s.len() && s[p + 1].is_ascii_alphabetic() && s[p] != s[p + 1] => s.swap(p, p + 1),
            1 => {
                s.remove(p);
            }
            2 => s.insert(p, (b'a' + rng.below(26) as u8) as char),
            _ => s[p] = if s[p].eq_ignore_ascii_case(&'x') { 'q' } else { 'x' },
        }
        push("typo", s.into_iter().collect());
    }
    let mut words = std::collections::BTreeSet::new();
    while words.len() < 30 {
        let k = rng.below(idx.u);
        let stem = text(k).split('.').next().unwrap_or("").to_ascii_lowercase();
        if ne[k + 1] - ne[k] >= 20 && (3..=10).contains(&stem.len()) && stem.chars().all(|c| c.is_ascii_lowercase()) {
            words.insert(stem);
        }
    }
    for w in words {
        push("word", w);
    }
    for (k, t) in targets.iter().take(30) {
        let e = idx.name_ents()[ne[*k] as usize] as usize;
        let dir = idx.dir_entry()[idx.parent()[e] as usize] as usize;
        let folder = String::from_utf8_lossy(idx.name(dir)).into_owned();
        let stem = t.split('.').next().unwrap_or(t);
        push("multi", format!("{} {stem}", folder.split(['.', ' ']).next().unwrap_or("")));
    }
    #[rustfmt::skip]
    let filters = [
        "ext:rs engine", "type:image screenshot", "kind:dir node_modules", "type:app safari", "in:~/Developer main",
        "size:>100mb", "mtime:<1d", "re:^IMG_[0-9]+", "path:Developer.*swift$ view", "main !test", "'main.rs", "^Cargo",
        "toml$", "ext:pdf", "a", "e s", "in:~/Developer ext:md readme", "type:code mtime:<7d", "kind:file size:>1gb", "Developer/Tools fsearch",
    ];
    for f in filters {
        push("filter", f.into());
    }
    let typing: Vec<Vec<String>> = targets.iter().take(10).map(|(_, t)| (1..=t.len()).map(|n| t[..n].to_string()).collect()).collect();
    let corpus = json!({
        "names": names.iter().map(|(c, q)| json!({"cat": c, "q": q})).collect::<Vec<_>>(),
        "typing": typing,
        "grep": [],
    });
    std::fs::write(out, serde_json::to_string_pretty(&corpus).unwrap()).unwrap();
    eprintln!("{} name queries, {} typing runs (grep: none — scoped snapshot)", names.len(), typing.len());
}
