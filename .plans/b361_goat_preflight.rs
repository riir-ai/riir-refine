//! Plan 372 T4 GOAT preflight — R3 `map-inplace-same-size-transmute` (the
//! lead) + R8 `hot-key-skew-split-table` (the swing). Standalone rustc
//! --release; two-run stability per the corpus law; interleaved arms per
//! the sequential-timing law (the ratio of two SEQUENTIAL arms measures the
//! box). Bars are pre-registered in the plan: R3 = ParetoDominant ≥2 of 3
//! sizes on the owned-source posture + the alloc axis disclosed (never
//! invented); R8 = no regression at uniform + a win at skew, else the rule
//! demotes to suggestion-only in the landing commit.
//!
//! Compile: rustc --edition 2021 -O b361_goat_preflight.rs -o /tmp/b361_preflight
//! Run:     /tmp/b361_preflight   (run TWICE — two-run stability)

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.realloc(p, l, n)
    }
}
#[global_allocator]
static A: Counting = Counting;

fn rng(state: &mut u64) -> u64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *state >> 33
}

fn median(v: &mut [u128]) -> u128 {
    v.sort_unstable();
    v[v.len() / 2]
}

// ── R3 ────────────────────────────────────────────────────────────────
const R3_SAME_SIZE: () = {
    assert!(core::mem::size_of::<f32>() == core::mem::size_of::<i32>());
    assert!(core::mem::align_of::<f32>() == core::mem::align_of::<i32>());
};

fn r3_buggy(v: &[f32]) -> Vec<i32> {
    v.iter().map(|&x| x as i32).collect()
}

fn r3_fixed(v: Vec<f32>) -> Vec<i32> {
    let _: () = R3_SAME_SIZE;
    let mut v = v;
    let len = v.len();
    let cap = v.capacity();
    let ptr = v.as_mut_ptr() as *mut i32;
    for i in 0..len {
        unsafe {
            let x = *ptr.cast::<f32>().add(i);
            *ptr.add(i) = x as i32;
        }
    }
    core::mem::forget(v);
    unsafe { Vec::from_raw_parts(ptr, len, cap) }
}

fn r3_bench() {
    println!("== R3 map-inplace-same-size-transmute (owned-source posture) ==");
    let mut summary = String::new();
    let mut wins = 0usize;
    for &n in &[1_024usize, 16_384, 262_144] {
        let mut s = 0x5EED_u64 ^ n as u64;
        let src: Vec<f32> =
            (0..n).map(|_| ((rng(&mut s) % 4001) as i64 - 2000) as i32 as f32).collect();
        // G1
        let want = r3_buggy(&src);
        let got = r3_fixed(src.clone());
        assert_eq!(want, got, "G1 failed at n={n}");

        // alloc census (the clone is SETUP, disclosed separately)
        ALLOCS.store(0, Ordering::Relaxed);
        let out = black_box(r3_buggy(&src));
        let a_buggy = ALLOCS.load(Ordering::Relaxed);
        drop(out);
        ALLOCS.store(0, Ordering::Relaxed);
        let own = src.clone();
        let a_clone = ALLOCS.load(Ordering::Relaxed);
        ALLOCS.store(0, Ordering::Relaxed);
        let out = black_box(r3_fixed(own));
        let a_fixed = ALLOCS.load(Ordering::Relaxed);
        drop(out);

        // interleaved pairs, median across pairs
        let (mut tb, mut tf) = (Vec::with_capacity(31), Vec::with_capacity(31));
        for _ in 0..31 {
            let t0 = Instant::now();
            let out = black_box(r3_buggy(&src));
            tb.push(t0.elapsed().as_nanos());
            drop(out);
            let own = src.clone();
            let t1 = Instant::now();
            let out = black_box(r3_fixed(own));
            tf.push(t1.elapsed().as_nanos());
            drop(out);
        }
        let (mb, mf) = (median(&mut tb), median(&mut tf));
        let ratio = mb as f64 / mf as f64;
        if ratio > 1.0 {
            wins += 1;
        }
        summary.push_str(&format!(
            "n={n}: buggy {mb}ns ({a_buggy} allocs) vs fixed {mf}ns ({a_fixed} allocs + {a_clone} setup clone) = {ratio:.3}x\n"
        ));
    }
    print!("{summary}");
    println!("R3 verdict: wins {wins}/3 sizes (bar: ParetoDominant ≥2/3)");
}

// ── R8 ────────────────────────────────────────────────────────────────
use std::collections::HashMap;

const HOT_MAX: usize = 8;
const PROMOTE_CHECK_EVERY: u64 = 16;

fn count_single(stream: &[u64]) -> HashMap<u64, u64> {
    let mut m = HashMap::new();
    for &k in stream {
        *m.entry(k).or_insert(0) += 1;
    }
    m
}

struct TwoTier {
    hot: HashMap<u64, u64>,
    main: HashMap<u64, u64>,
    since_check: u64,
}

impl TwoTier {
    fn new() -> Self {
        TwoTier { hot: HashMap::new(), main: HashMap::new(), since_check: 0 }
    }
    fn add(&mut self, k: u64) {
        if let Some(c) = self.hot.get_mut(&k) {
            *c += 1;
            return;
        }
        *self.main.entry(k).or_insert(0) += 1;
        self.since_check += 1;
        if self.since_check >= PROMOTE_CHECK_EVERY {
            self.since_check = 0;
            self.promote_top();
        }
    }
    fn promote_top(&mut self) {
        let mut rows: Vec<(u64, u64)> =
            self.main.iter().map(|(&k, &c)| (k, c)).collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (k, c) in rows.into_iter().take(HOT_MAX) {
            if self.hot.len() >= HOT_MAX {
                break;
            }
            if !self.hot.contains_key(&k) {
                self.hot.insert(k, c);
                self.main.remove(&k);
            }
        }
        while self.hot.len() > HOT_MAX {
            let cold = self.hot.iter().min_by_key(|(_, &c)| c).map(|(&k, _)| k);
            match cold {
                Some(k) => {
                    let c = self.hot.remove(&k).unwrap();
                    *self.main.entry(k).or_insert(0) = c;
                }
                None => break,
            }
        }
    }
    fn merged(&self) -> HashMap<u64, u64> {
        let mut m = self.main.clone();
        for (k, c) in &self.hot {
            *m.entry(*k).or_insert(0) += *c;
        }
        m
    }
}

fn skew_stream(n: usize, seed: u64) -> Vec<u64> {
    // 99% draws from an 8-key head, 1% from a 4096-key tail — the regime
    // the rule names (statistical skew over arbitrary access order)
    let mut s = seed;
    (0..n)
        .map(|_| {
            let r = rng(&mut s);
            if r % 100 < 99 {
                r % 8
            } else {
                8 + r % 4096
            }
        })
        .collect()
}

fn uniform_stream(n: usize, seed: u64) -> Vec<u64> {
    let mut s = seed;
    (0..n).map(|_| rng(&mut s) % 4096).collect()
}

fn r8_bench() {
    println!("== R8 hot-key-skew-split-table ==");
    for (name, stream) in [
        ("skew", skew_stream(262_144, 0xC0FFEE)),
        ("uniform", uniform_stream(262_144, 0x42)),
    ] {
        let truth = count_single(&stream);
        // G1
        let mut tier = TwoTier::new();
        for &k in &stream {
            tier.add(k);
        }
        assert_eq!(truth, tier.merged(), "G1 {name} failed");

        let (mut ts, mut tt) = (Vec::with_capacity(15), Vec::with_capacity(15));
        for _ in 0..15 {
            let t0 = Instant::now();
            let out = black_box(count_single(&stream));
            ts.push(t0.elapsed().as_nanos());
            drop(out);
            let mut tier = TwoTier::new();
            let t1 = Instant::now();
            for &k in &stream {
                tier.add(k);
            }
            black_box(&tier);
            tt.push(t1.elapsed().as_nanos());
        }
        let (ms, mt) = (median(&mut ts), median(&mut tt));
        let ratio = ms as f64 / mt as f64;
        println!("{name}: single {ms}ns vs two-tier {mt}ns = {ratio:.3}x");
    }
    println!("R8 bars: uniform ≥0.95x (no regression) AND skew >1.05x — else demote to suggestion-only");
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() > 1 && argv[1] == "r8" {
        r8_bench();
    } else {
        r3_bench();
        r8_bench();
    }
}
