cargo-refine 0.4.8
cargo refine: dry run — reviewing 1 path(s) (no edits). Real fixes: cargo refine fix
scan root: /Users/kat/git/riir-refine/.raw/mozilla-heal/sccache — 92 source file(s)
Refining…
clippy oracle (auto): linting /Users/kat/git/riir-refine/.raw/mozilla-heal/sccache at default features, all targets (tests/benches/examples included) — this compiles the project, first run can take a while (RIIR_REFINE_AUTO_ORACLE=0 disables)
⏳ clippy compiling...
clippy oracle (auto): cache HIT 2684f3c56150 — 90 file(s)/clippy 0.1.99 (b940084d7e 2026-09-28) hashed, 1 gap pass(es) reused (RIIR_REFINE_ORACLE_CACHE=0 disables)
clippy oracle (auto): 1 required-features-gated target(s) invisible at this posture — 1 gap pass(es) armed [sccache:sccache-dist←dist-server]
clippy oracle (auto): gap pass features [dist-server] exited 101 (partial compile — its diagnostics are still real)
clippy oracle (auto): 12 real diagnostic(s) anchored ahead of ranking
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/build.rs:1-1 (routed: clippy) [heuristic]
  [clippy] write_with_newline               score    12.63  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.90  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.88  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/build.rs:3-33 (routed: clippy) [heuristic]
  [clippy] redundant_static_lifetimes       score   164.14  →  static GREETING: &str = "hi";
  [clippy] match_same_arms                  score    90.25  →  fn msa(x: &str) -> u8 { match x { "a" | "b" => 1, _ => 2 } }
  [clippy] used_underscore_binding          score    76.90  →  fn uub(segs: &[String]) -> bool { let [_root, mod_name, c, m] = segs else { return false }; mod_name == "clone" && c == m }
  [perf  ] linker-dce-custom-section-volatile-marker score   194.01  →  #[link_section = "appmeta"]
  [perf  ] connect-timeout-walk-over-resolved-addrs score   191.17  →  #![allow(dead_code)]
  [perf  ] wasm-unwrap-panic-machinery-elimination score   190.95  →  // The same two sites behind a trait whose wasm arm compiles to a
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:6-6 (routed: perf) [heuristic]
  [clippy] needless_borrowed_reference      score     4.44  →  pub fn nbr_b(a: u64) -> String {
  [clippy] new_without_default              score     4.32  →  pub struct W { pub x: u8 }
  [clippy] byte_char_slices                 score     4.16  →  pub const SHARD_MAGIC: [u8; 4] = *b"SHRD";
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    14.58  →  #![allow(dead_code)]
  [perf  ] reserve-then-commit-capacity-accounting score     8.66  →  #![allow(dead_code)]
  [perf  ] mtime-encoded-lru-recency-persistence score     8.35  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:8-8 (routed: clippy) [heuristic]
  [clippy] map_or_identity                  score     7.66  →  fn f(r: Result<u8, String>) -> u8 { r.unwrap_or(0) }
  [clippy] write_with_newline               score     6.97  →  use std::fmt::Write as _;
  [clippy] from_str_radix_10                score     6.72  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [perf  ] write-vectored-capability-flag   score    44.82  →  use std::io;
  [perf  ] varint-compose-word-single-store score    31.52  →  use std::io::{Write, Result};
  [perf  ] publication-takes-over-condemned-cache-entry score    18.36  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:9-9 (routed: clippy) [heuristic]
  [clippy] write_with_newline               score     8.26  →  use std::fmt::Write as _;
  [clippy] manual_map                       score     6.57  →  fn opt_double(x: Option<i32>) -> Option<i32> {
  [clippy] multiple_bound_locations         score     6.57  →  pub fn mbl_a<T>(t: T) -> T
  [perf  ] steal-batch-not-per-item         score    17.88  →  use crossbeam_deque::{Injector, Steal};
  [perf  ] hot-slot-recycler-flat-tables-zero-alloc-claims score    17.12  →  #![allow(dead_code)]
  [perf  ] batch-retry-ladder-classifies-failure-before-permanent-state score    16.90  →  // GOVERNED: classify FIRST — the three-way ladder serves only the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:11-11 (routed: clippy) [heuristic]
  [clippy] write_with_newline               score     9.84  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score     3.96  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] nonnull_unchecked_on_box_ptr     score     3.66  →  fn nub(one: Box<u32>) -> std::ptr::NonNull<u32> { NonNull::from_mut(Box::leak(one)) }
  [perf  ] rayon-bridge-oneshot             score    54.98  →  use rayon::prelude::*;
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    19.77  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    17.63  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:12-12 (routed: perf) [heuristic]
  [clippy] write_with_newline               score    13.14  →  use std::fmt::Write as _;
  [clippy] cast_abs_to_unsigned             score     4.44  →  pub fn cau_a(x: i64) -> u64 {
  [clippy] ptr_offset_with_cast             score     3.83  →  pub fn powc_a(b: &[u8], i: usize) -> u8 {
  [perf  ] mtime-encoded-lru-recency-persistence score    27.43  →  #![allow(dead_code)]
  [perf  ] nonblocking-open-for-head-probes score    15.53  →  #[cfg(unix)]
  [perf  ] mmap-settled-size-gated-read     score     9.69  →  #[cfg(unix)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:13-13 (routed: clippy) [heuristic]
  [clippy] write_with_newline               score     6.15  →  use std::fmt::Write as _;
  [clippy] redundant_field_names            score     2.89  →  struct Point { x: i32, y: i32 }
  [clippy] match_ref_pats                   score     2.54  →  pub enum Fruit { Apple, Pear }
  [perf  ] steal-batch-not-per-item         score    14.90  →  use crossbeam_deque::{Injector, Steal};
  [perf  ] batch-retry-ladder-classifies-failure-before-permanent-state score    14.07  →  // GOVERNED: classify FIRST — the three-way ladder serves only the
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    14.07  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:14-14 (routed: clippy) [heuristic]
  [clippy] write_with_newline               score     9.84  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score     3.96  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] nonnull_unchecked_on_box_ptr     score     3.66  →  fn nub(one: Box<u32>) -> std::ptr::NonNull<u32> { NonNull::from_mut(Box::leak(one)) }
  [perf  ] rayon-bridge-oneshot             score    54.98  →  use rayon::prelude::*;
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    19.77  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    17.63  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:15-15 (routed: clippy) [heuristic]
  [clippy] write_with_newline               score    12.63  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.92  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.88  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:16-16 (routed: dist) [heuristic]
  [clippy] write_with_newline               score    17.76  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    13.54  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] nonnull_unchecked_on_box_ptr     score    10.63  →  fn nub(one: Box<u32>) -> std::ptr::NonNull<u32> { NonNull::from_mut(Box::leak(one)) }
  [perf  ] mtime-encoded-lru-recency-persistence score    99.94  →  #![allow(dead_code)]
  [perf  ] exclusive-mkdir-temp-resource-reservation score    32.70  →  #![allow(dead_code)]
  [perf  ] batch-retry-ladder-classifies-failure-before-permanent-state score    27.58  →  // GOVERNED: classify FIRST — the three-way ladder serves only the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:17-17 (routed: perf) [heuristic]
  [clippy] write_with_newline               score    24.24  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    16.02  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] legacy_numeric_constants         score    15.10  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] exclusive-mkdir-temp-resource-reservation score    36.65  →  #![allow(dead_code)]
  [perf  ] batch-load-admission-sized-from-files-refuses-past-the-hard-budget score    27.83  →  #![allow(dead_code)]
  [perf  ] fanout-first-error-leaves-siblings-running score    27.83  →  // GOVERNED: the error path cancels the outstanding work and AWAITS
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:22-26 (routed: clippy) [heuristic]
  [clippy] io_other_error                   score    14.92  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] vec_init_then_push               score    12.46  →  fn build() -> Vec<u8> {
  [clippy] chunks_exact_to_as_chunks        score    12.21  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] mutex-to-atomic                  score    37.14  →  use std::sync::atomic::{AtomicU32, Ordering};
  [perf  ] varint-compose-word-single-store score    35.98  →  use std::io::{Write, Result};
  [perf  ] diagnostics-not-just-exit-code   score    35.93  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:28-32 (routed: clippy) [heuristic]
  [clippy] io_other_error                   score    14.92  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] vec_init_then_push               score    12.46  →  fn build() -> Vec<u8> {
  [clippy] chunks_exact_to_as_chunks        score    12.21  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] mutex-to-atomic                  score    37.14  →  use std::sync::atomic::{AtomicU32, Ordering};
  [perf  ] varint-compose-word-single-store score    35.98  →  use std::io::{Write, Result};
  [perf  ] diagnostics-not-just-exit-code   score    35.93  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:34-38 (routed: clippy) [heuristic]
  [clippy] vec_init_then_push               score    16.28  →  fn build() -> Vec<u8> {
  [clippy] io_other_error                   score    14.92  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] chunks_exact_to_as_chunks        score    12.21  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] mutex-to-atomic                  score    37.14  →  use std::sync::atomic::{AtomicU32, Ordering};
  [perf  ] varint-compose-word-single-store score    35.98  →  use std::io::{Write, Result};
  [perf  ] diagnostics-not-just-exit-code   score    35.93  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:40-44 (routed: clippy) [heuristic]
  [clippy] vec_init_then_push               score    16.28  →  fn build() -> Vec<u8> {
  [clippy] io_other_error                   score    14.92  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] chunks_exact_to_as_chunks        score    12.21  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] mutex-to-atomic                  score    37.14  →  use std::sync::atomic::{AtomicU32, Ordering};
  [perf  ] varint-compose-word-single-store score    35.98  →  use std::io::{Write, Result};
  [perf  ] diagnostics-not-just-exit-code   score    35.93  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:46-61 (routed: clippy) [heuristic]
  [clippy] single_char_pattern              score    42.79  →  fn f(s: &str) -> bool { s.contains('x') }
  [clippy] write_with_newline               score    31.05  →  use std::fmt::Write as _;
  [clippy] vec_init_then_push               score    28.67  →  fn build() -> Vec<u8> {
  [perf  ] one-event-per-writer-consumer-waits-on-the-side-it-needs score    96.87  →  #![allow(dead_code)]
  [perf  ] refusal-quarantines-entry-transient-passes score    81.75  →  #![allow(dead_code)]
  [perf  ] diagnostics-not-just-exit-code   score    81.74  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:63-83 (routed: perf) [heuristic]
  [clippy] chunks_exact_to_as_chunks        score    60.41  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [clippy] vec_init_then_push               score    60.41  →  fn build() -> Vec<u8> {
  [clippy] single_char_pattern              score    48.90  →  fn f(s: &str) -> bool { s.contains('x') }
  [perf  ] one-event-per-writer-consumer-waits-on-the-side-it-needs score   123.70  →  #![allow(dead_code)]
  [perf  ] diagnostics-not-just-exit-code   score   110.09  →  // The exit code is the LAST gate, not the only one: scan the
  [perf  ] keyword-map-to-match             score    88.95  →  #[derive(Clone, Copy)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:93-106 (routed: clippy) [heuristic]
  [clippy] io_other_error                   score    38.24  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    34.20  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    29.33  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] software-prefetch-storage-bound  score    86.64  →  fn checksum_stream(data: &[u32]) -> u64 {
  [perf  ] write-verify-readback-before-publish score    57.43  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    56.95  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:108-121 (routed: clippy) [heuristic]
  [clippy] io_other_error                   score    38.24  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    34.20  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    29.33  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] software-prefetch-storage-bound  score    86.64  →  fn checksum_stream(data: &[u32]) -> u64 {
  [perf  ] write-verify-readback-before-publish score    57.43  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    56.95  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:123-136 (routed: clippy) [heuristic]
  [clippy] io_other_error                   score    38.24  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    34.20  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    29.46  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] software-prefetch-storage-bound  score    86.90  →  fn checksum_stream(data: &[u32]) -> u64 {
  [perf  ] write-verify-readback-before-publish score    57.43  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    56.95  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:138-151 (routed: clippy) [heuristic]
  [clippy] io_other_error                   score    38.24  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    34.20  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    29.33  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] software-prefetch-storage-bound  score    86.64  →  fn checksum_stream(data: &[u32]) -> u64 {
  [perf  ] write-verify-readback-before-publish score    57.43  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    56.95  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:312-760 (routed: perf) [heuristic]
  [clippy] vec_init_then_push               score   940.38  →  fn build() -> Vec<u8> {
  [clippy] single_char_pattern              score   902.98  →  fn f(s: &str) -> bool { s.contains('x') }
  [clippy] from_str_radix_10                score   854.52  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [perf  ] exclusive-mkdir-temp-resource-reservation score  2284.98  →  #![allow(dead_code)]
  [perf  ] resume-revalidates-provenance-before-reuse score  1855.70  →  // The manifest records the build IDENTITY (source + calibration +
  [perf  ] diagnostics-not-just-exit-code   score  1817.02  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/logging.rs:19-19 (routed: clippy) [heuristic]
  [clippy] write_with_newline               score    12.63  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.68  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.88  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] regex-hoist-lazylock             score    77.16  →  use regex::Regex;
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/logging.rs:20-20 (routed: perf) [heuristic]
  [clippy] write_with_newline               score    20.55  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    12.49  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] io_other_error                   score    12.06  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] exclusive-mkdir-temp-resource-reservation score    27.23  →  #![allow(dead_code)]
  [perf  ] batch-load-admission-sized-from-files-refuses-past-the-hard-budget score    24.19  →  #![allow(dead_code)]
  [perf  ] fanout-first-error-leaves-siblings-running score    24.19  →  // GOVERNED: the error path cancels the outstanding work and AWAITS
  … and 1936 more span(s) — the summary below is the full count
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/integration/cmake-modules.Dockerfile:1 [docker suggest] dockerignore-missing                     — build context has no .dockerignore
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/integration/cmake-modules.Dockerfile:2 [docker fix    ] apt-no-install-recommends                — RUN apt-get update -qq && apt-get install -y -qq cmake ninja-build python3 && rm -rf /var/lib/apt/lists/*
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/integration/cmake-modules-v4.Dockerfile:1 [docker suggest] dockerignore-missing                     — build context has no .dockerignore
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/integration/cmake-modules-v4.Dockerfile:2 [docker fix    ] apt-no-install-recommends                — RUN apt-get update -qq && apt-get install -y -qq ninja-build python3 python3-pip \
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/harness/Dockerfile.sccache-dist:1 [docker suggest] dockerignore-missing                     — build context has no .dockerignore
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/harness/Dockerfile.sccache-dist:2 [docker fix    ] apt-lists-not-removed                    — RUN apt-get update && \
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/harness/Dockerfile.sccache-dist:2 [docker fix    ] apt-no-install-recommends                — RUN apt-get update && \
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/Cargo.toml:33 [dist  ] pgo-training-set-covers-real-workload-hint — [profile.test]
           ↳ every manifest-level size/tuning knob this domain advises is set here — the next build-time lever is out-of-manifest: IF you use (or add) PGO, the training set must cover the SHIPPED workload mix (rustc #159091: adding rustdoc benches to the PGO training set = 2.85% mean wall-time across rustdoc benchmarks, >6% best; 'PGO is fiddly to set up but it can really make a difference, and the training set matters'); train on the real workload corpus, not a narrow benchmark — measure before/after
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/test-crate/Cargo.toml:14 [dist  ] cli-panic-unwind-tables                  — [[bin]]
           ↳ consider `panic = "abort"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection
🔒 29 secret-shaped literal(s) in 10 files (+12 digest-pin(s) adjudicated benign — suppressed from this count; the sync gate stays fail-closed) — suggest-only, never auto-fixed; details: `cargo refine scan`
🔍 Found clippy 75 | perf 75 | sec 29 | docker 7 | dist 2 — 188 total
🛠️ ~39% fixable via `cargo refine fix` (compile-gated; the suggest-only rows stay manual)

scanned 86 files · 1961 spans · 1961 with suggestions (top-3 per domain) · 12 oracle-anchored · 12 no-corpus-rule
⏱️ clippy 0.1s | scan 19.8s | refine 22.4s
