cargo-refine 0.4.2 — help: `cargo refine help`
scan root: /Users/kat/git/riir-refine/.raw/mozilla-heal/sccache — 91 source file(s)
Refining…
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/build.rs:1-1 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.83  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/build.rs:3-33 (routed: clippy)
  [clippy] redundant_static_lifetimes       score   162.78  →  static GREETING: &str = "hi";
  [clippy] match_same_arms                  score    89.79  →  fn msa(x: &str) -> u8 { match x { "a" | "b" => 1, _ => 2 } }
  [clippy] used_underscore_binding          score    76.17  →  fn uub(segs: &[String]) -> bool { let [_root, mod_name, c, m] = segs else { return false }; mod_name == "clone" && c == m }
  [perf  ] linker-dce-custom-section-volatile-marker score   193.06  →  #[link_section = "appmeta"]
  [perf  ] connect-timeout-walk-over-resolved-addrs score   190.24  →  #![allow(dead_code)]
  [perf  ] wasm-unwrap-panic-machinery-elimination score   190.07  →  // The same two sites behind a trait whose wasm arm compiles to a
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:6-6 (routed: perf)
  [clippy] needless_borrowed_reference      score     4.38  →  pub fn nbr_b(a: u64) -> String {
  [clippy] new_without_default              score     4.28  →  pub struct W { pub x: u8 }
  [clippy] byte_char_slices                 score     4.10  →  pub const SHARD_MAGIC: [u8; 4] = *b"SHRD";
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    14.58  →  #![allow(dead_code)]
  [perf  ] reserve-then-commit-capacity-accounting score     8.63  →  #![allow(dead_code)]
  [perf  ] mtime-encoded-lru-recency-persistence score     8.32  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:8-8 (routed: perf)
  [clippy] write_with_newline               score     6.99  →  use std::fmt::Write as _;
  [clippy] from_str_radix_10                score     6.94  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [clippy] redundant_pattern_matching       score     6.68  →  fn f(r: Result<u8, u8>) -> bool { if r.is_ok() { true } else { false } }
  [perf  ] write-vectored-capability-flag   score    44.76  →  use std::io;
  [perf  ] varint-compose-word-single-store score    31.59  →  use std::io::{Write, Result};
  [perf  ] publication-takes-over-condemned-cache-entry score    18.38  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:9-9 (routed: perf)
  [clippy] write_with_newline               score     8.31  →  use std::fmt::Write as _;
  [clippy] manual_map                       score     6.72  →  fn opt_double(x: Option<i32>) -> Option<i32> {
  [clippy] multiple_bound_locations         score     6.72  →  pub fn mbl_a<T>(t: T) -> T
  [perf  ] steal-batch-not-per-item         score    17.95  →  use crossbeam_deque::{Injector, Steal};
  [perf  ] hot-slot-recycler-flat-tables-zero-alloc-claims score    17.18  →  #![allow(dead_code)]
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    16.96  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:11-11 (routed: perf)
  [clippy] write_with_newline               score     9.76  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score     3.92  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] legacy_numeric_constants         score     3.83  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] rayon-bridge-oneshot             score    54.79  →  use rayon::prelude::*;
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    19.72  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    17.57  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:12-12 (routed: perf)
  [clippy] write_with_newline               score    13.03  →  use std::fmt::Write as _;
  [clippy] cast_abs_to_unsigned             score     4.41  →  pub fn cau_a(x: i64) -> u64 {
  [clippy] ptr_offset_with_cast             score     3.79  →  pub fn powc_a(b: &[u8], i: usize) -> u8 {
  [perf  ] mtime-encoded-lru-recency-persistence score    27.34  →  #![allow(dead_code)]
  [perf  ] nonblocking-open-for-head-probes score    15.49  →  #[cfg(unix)]
  [perf  ] mmap-settled-size-gated-read     score     9.67  →  #[cfg(unix)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:13-13 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] redundant_field_names            score     2.93  →  struct Point { x: i32, y: i32 }
  [clippy] match_ref_pats                   score     2.59  →  pub enum Fruit { Apple, Pear }
  [perf  ] steal-batch-not-per-item         score    14.96  →  use crossbeam_deque::{Injector, Steal};
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    14.14  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
  [perf  ] mtime-encoded-lru-recency-persistence score    14.14  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:14-14 (routed: perf)
  [clippy] write_with_newline               score     9.76  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score     3.92  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] nonnull_unchecked_on_box_ptr     score     3.61  →  fn nub(one: Box<u32>) -> std::ptr::NonNull<u32> { NonNull::from_mut(Box::leak(one)) }
  [perf  ] rayon-bridge-oneshot             score    54.79  →  use rayon::prelude::*;
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    19.72  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    17.57  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:15-15 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.85  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:16-16 (routed: perf)
  [clippy] write_with_newline               score    17.61  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    13.44  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] nonnull_unchecked_on_box_ptr     score    10.51  →  fn nub(one: Box<u32>) -> std::ptr::NonNull<u32> { NonNull::from_mut(Box::leak(one)) }
  [perf  ] mtime-encoded-lru-recency-persistence score    99.73  →  #![allow(dead_code)]
  [perf  ] exclusive-mkdir-temp-resource-reservation score    32.64  →  #![allow(dead_code)]
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    27.71  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:17-17 (routed: perf)
  [clippy] write_with_newline               score    24.01  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    15.85  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] legacy_numeric_constants         score    14.93  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] exclusive-mkdir-temp-resource-reservation score    36.64  →  #![allow(dead_code)]
  [perf  ] batch-load-admission-sized-from-files-refuses-past-the-hard-budget score    27.90  →  #![allow(dead_code)]
  [perf  ] batch-reserve-monotonic-counters-offload-bulk score    27.90  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:22-26 (routed: clippy)
  [clippy] io_other_error                   score    14.80  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] vec_init_then_push               score    12.42  →  fn build() -> Vec<u8> {
  [clippy] chunks_exact_to_as_chunks        score    12.09  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] mutex-to-atomic                  score    37.09  →  use std::sync::atomic::{AtomicU32, Ordering};
  [perf  ] varint-compose-word-single-store score    36.03  →  use std::io::{Write, Result};
  [perf  ] diagnostics-not-just-exit-code   score    35.89  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:28-32 (routed: clippy)
  [clippy] io_other_error                   score    14.80  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] vec_init_then_push               score    12.42  →  fn build() -> Vec<u8> {
  [clippy] chunks_exact_to_as_chunks        score    12.09  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] mutex-to-atomic                  score    37.09  →  use std::sync::atomic::{AtomicU32, Ordering};
  [perf  ] varint-compose-word-single-store score    36.03  →  use std::io::{Write, Result};
  [perf  ] diagnostics-not-just-exit-code   score    35.89  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:34-38 (routed: clippy)
  [clippy] vec_init_then_push               score    16.20  →  fn build() -> Vec<u8> {
  [clippy] io_other_error                   score    14.80  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] chunks_exact_to_as_chunks        score    12.09  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] mutex-to-atomic                  score    37.09  →  use std::sync::atomic::{AtomicU32, Ordering};
  [perf  ] varint-compose-word-single-store score    36.03  →  use std::io::{Write, Result};
  [perf  ] diagnostics-not-just-exit-code   score    35.89  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:40-44 (routed: clippy)
  [clippy] vec_init_then_push               score    16.20  →  fn build() -> Vec<u8> {
  [clippy] io_other_error                   score    14.80  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] chunks_exact_to_as_chunks        score    12.09  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] mutex-to-atomic                  score    37.09  →  use std::sync::atomic::{AtomicU32, Ordering};
  [perf  ] varint-compose-word-single-store score    36.03  →  use std::io::{Write, Result};
  [perf  ] diagnostics-not-just-exit-code   score    35.89  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:46-61 (routed: clippy)
  [clippy] single_char_pattern              score    42.54  →  fn f(s: &str) -> bool { s.contains('x') }
  [clippy] write_with_newline               score    30.78  →  use std::fmt::Write as _;
  [clippy] vec_init_then_push               score    28.49  →  fn build() -> Vec<u8> {
  [perf  ] one-event-per-writer-consumer-waits-on-the-side-it-needs score    96.58  →  #![allow(dead_code)]
  [perf  ] refusal-quarantines-entry-transient-passes score    82.35  →  #![allow(dead_code)]
  [perf  ] diagnostics-not-just-exit-code   score    81.73  →  // The exit code is the LAST gate, not the only one: scan the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:63-83 (routed: perf)
  [clippy] chunks_exact_to_as_chunks        score    59.76  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [clippy] vec_init_then_push               score    59.76  →  fn build() -> Vec<u8> {
  [clippy] single_char_pattern              score    48.58  →  fn f(s: &str) -> bool { s.contains('x') }
  [perf  ] one-event-per-writer-consumer-waits-on-the-side-it-needs score   123.40  →  #![allow(dead_code)]
  [perf  ] diagnostics-not-just-exit-code   score   110.04  →  // The exit code is the LAST gate, not the only one: scan the
  [perf  ] keyword-map-to-match             score    89.05  →  #[derive(Clone, Copy)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:93-106 (routed: docker)
  [clippy] io_other_error                   score    38.03  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    33.88  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    29.09  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] software-prefetch-storage-bound  score    86.40  →  fn checksum_stream(data: &[u32]) -> u64 {
  [perf  ] write-verify-readback-before-publish score    57.45  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    56.83  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:108-121 (routed: docker)
  [clippy] io_other_error                   score    38.03  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    33.88  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    29.09  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] software-prefetch-storage-bound  score    86.40  →  fn checksum_stream(data: &[u32]) -> u64 {
  [perf  ] write-verify-readback-before-publish score    57.45  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    56.83  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:123-136 (routed: docker)
  [clippy] io_other_error                   score    38.03  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    33.88  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    29.22  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] software-prefetch-storage-bound  score    86.65  →  fn checksum_stream(data: &[u32]) -> u64 {
  [perf  ] write-verify-readback-before-publish score    57.45  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    56.83  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:138-151 (routed: docker)
  [clippy] io_other_error                   score    38.03  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    33.88  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    29.09  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] software-prefetch-storage-bound  score    86.40  →  fn checksum_stream(data: &[u32]) -> u64 {
  [perf  ] write-verify-readback-before-publish score    57.45  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    56.83  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/sccache_cargo.rs:312-678 (routed: perf)
  [clippy] single_char_pattern              score   790.10  →  fn f(s: &str) -> bool { s.contains('x') }
  [clippy] vec_init_then_push               score   788.31  →  fn build() -> Vec<u8> {
  [clippy] from_str_radix_10                score   753.42  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [perf  ] exclusive-mkdir-temp-resource-reservation score  1778.68  →  #![allow(dead_code)]
  [perf  ] diagnostics-not-just-exit-code   score  1572.23  →  // The exit code is the LAST gate, not the only one: scan the
  [perf  ] resume-revalidates-provenance-before-reuse score  1492.36  →  // The manifest records the build IDENTITY (source + calibration +
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/logging.rs:19-19 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.61  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] regex-hoist-lazylock             score    76.93  →  use regex::Regex;
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/logging.rs:20-20 (routed: perf)
  [clippy] write_with_newline               score    20.36  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    12.35  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] io_other_error                   score    11.93  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] exclusive-mkdir-temp-resource-reservation score    27.25  →  #![allow(dead_code)]
  [perf  ] batch-load-admission-sized-from-files-refuses-past-the-hard-budget score    24.25  →  #![allow(dead_code)]
  [perf  ] batch-reserve-monotonic-counters-offload-bulk score    24.25  →  #![allow(dead_code)]
  … and 1910 more span(s) — the summary below is the full count
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/integration/cmake-modules.Dockerfile:1 [docker suggest] dockerignore-missing                     — build context has no .dockerignore
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/integration/cmake-modules.Dockerfile:2 [docker fix    ] apt-no-install-recommends                — RUN apt-get update -qq && apt-get install -y -qq cmake ninja-build python3 && rm -rf /var/lib/apt/lists/*
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/integration/cmake-modules-v4.Dockerfile:1 [docker suggest] dockerignore-missing                     — build context has no .dockerignore
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/integration/cmake-modules-v4.Dockerfile:2 [docker fix    ] apt-no-install-recommends                — RUN apt-get update -qq && apt-get install -y -qq ninja-build python3 python3-pip \
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/harness/Dockerfile.sccache-dist:1 [docker suggest] dockerignore-missing                     — build context has no .dockerignore
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/harness/Dockerfile.sccache-dist:2 [docker fix    ] apt-lists-not-removed                    — RUN apt-get update && \
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/harness/Dockerfile.sccache-dist:2 [docker fix    ] apt-no-install-recommends                — RUN apt-get update && \
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/Cargo.toml:33 [dist  ] cli-panic-unwind-tables                  — [profile.test]
           ↳ consider `panic = "abort"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection
/Users/kat/git/riir-refine/.raw/mozilla-heal/sccache/tests/test-crate/Cargo.toml:14 [dist  ] cli-panic-unwind-tables                  — [[bin]]
           ↳ consider `panic = "abort"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection
🔒 27 secret-shaped literal(s) in 9 files (+12 digest-pin(s) adjudicated benign — suppressed from this count; the sync gate stays fail-closed) — suggest-only, never auto-fixed; details: `cargo refine scan`
🔍 Found clippy 75 | perf 75 | sec 27 | docker 7 | dist 2 — 186 total
🛠️ ~40% fixable via `cargo refine fix` (compile-gated; the suggest-only rows stay manual)

scanned 85 files · 1935 spans · 1935 with suggestions (top-3 per domain)
