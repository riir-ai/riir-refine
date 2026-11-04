cargo-refine 0.4.2 — help: `cargo refine help`
scan root: /Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs — 609 source file(s)
Refining…
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi/uniffi-bindgen-swift.rs:5-5 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.41  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi/uniffi-bindgen-swift.rs:7-9 (routed: clippy)
  [clippy] semicolon_if_nothing_returned    score    22.03  →  fn greet() {}
  [clippy] collapsible_else_if              score    10.25  →  fn work() {}
  [clippy] collapsible_if                   score     8.37  →  fn work() {}
  [perf  ] seqlock-copy-read-unlatched      score    34.33  →  #![allow(dead_code)]
  [perf  ] revalidation-skipped-by-certifying-identity-token score    26.64  →  #![allow(dead_code)]
  [perf  ] sort-unstable                    score    16.63  →  fn sort_scores(scores: &mut Vec<u32>) {
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi/uniffi-bindgen.rs:5-5 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.41  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi/uniffi-bindgen.rs:7-9 (routed: clippy)
  [clippy] semicolon_if_nothing_returned    score    22.03  →  fn greet() {}
  [clippy] collapsible_else_if              score    10.25  →  fn work() {}
  [clippy] collapsible_if                   score     8.37  →  fn work() {}
  [perf  ] seqlock-copy-read-unlatched      score    34.33  →  #![allow(dead_code)]
  [perf  ] revalidation-skipped-by-certifying-identity-token score    26.64  →  #![allow(dead_code)]
  [perf  ] sort-unstable                    score    16.63  →  fn sort_scores(scores: &mut Vec<u32>) {
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:8-8 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] redundant_pattern_matching       score     5.41  →  fn f(r: Result<u8, u8>) -> bool { if r.is_ok() { true } else { false } }
  [clippy] from_str_radix_10                score     5.05  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [perf  ] write-vectored-capability-flag   score    43.35  →  use std::io;
  [perf  ] varint-compose-word-single-store score    30.18  →  use std::io::{Write, Result};
  [perf  ] steal-batch-not-per-item         score    14.96  →  use crossbeam_deque::{Injector, Steal};
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:9-9 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] match_ref_pats                   score     2.59  →  pub enum Fruit { Apple, Pear }
  [clippy] redundant_field_names            score     2.55  →  struct Point { x: i32, y: i32 }
  [perf  ] steal-batch-not-per-item         score    14.96  →  use crossbeam_deque::{Injector, Steal};
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    14.14  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
  [perf  ] mtime-encoded-lru-recency-persistence score    14.14  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:10-10 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] match_ref_pats                   score     2.59  →  pub enum Fruit { Apple, Pear }
  [clippy] redundant_field_names            score     2.55  →  struct Point { x: i32, y: i32 }
  [perf  ] steal-batch-not-per-item         score    14.96  →  use crossbeam_deque::{Injector, Steal};
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    14.14  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
  [perf  ] mtime-encoded-lru-recency-persistence score    14.14  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:11-11 (routed: perf)
  [clippy] unnecessary_get_then_check       score    21.38  →  fn ugtc(m: std::collections::HashMap<u8, u8>, k: u8) -> bool { !m.contains_key(&k) }
  [clippy] write_with_newline               score    17.61  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    13.44  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] tier-hit-refreshes-lru-recency-at-owning-tier score    45.14  →  #![allow(dead_code)]
  [perf  ] sorted-input-previous-row-fast-path score    42.09  →  use std::collections::HashMap;
  [perf  ] papaya-lockfree                  score    39.47  →  use papaya::HashMap;
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:13-13 (routed: perf)
  [clippy] write_with_newline               score     9.76  →  use std::fmt::Write as _;
  [clippy] from_str_radix_10                score     6.83  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [clippy] io_other_error                   score     6.61  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    18.79  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
  [perf  ] mtime-encoded-lru-recency-persistence score    18.79  →  #![allow(dead_code)]
  [perf  ] papaya-lockfree                  score    18.79  →  use papaya::HashMap;
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:15-22 (routed: perf)
  [clippy] unnecessary_get_then_check       score    31.69  →  fn ugtc(m: std::collections::HashMap<u8, u8>, k: u8) -> bool { !m.contains_key(&k) }
  [clippy] mem_replace_with_default         score    20.92  →  fn mrw(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::take(dst)) }
  [clippy] multiple_bound_locations         score    19.89  →  pub fn mbl_a<T>(t: T) -> T
  [perf  ] immutable-static-map-filter-selection score   111.35  →  use constmap::ConstMap;
  [perf  ] mtime-encoded-lru-recency-persistence score   111.35  →  #![allow(dead_code)]
  [perf  ] refusal-quarantines-entry-transient-passes score    98.10  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:24-81 (routed: perf)
  [clippy] needless_pass_by_ref_mut         score   163.04  →  struct Probe { hits: u32 }
  [clippy] len_without_is_empty             score   162.23  →  pub struct LwA { items: Vec<u8> }
  [clippy] wrong_self_convention            score   145.24  →  struct Sig { code: u32 }
  [perf  ] per-scope-map-scan-hash-crossover score   296.18  →  // optimized: the HYBRID — each scope is a SmallMap that scans its first
  [perf  ] immutable-static-map-filter-selection score   283.36  →  use constmap::ConstMap;
  [perf  ] encoded-layout-borrowed-view     score   273.65  →  struct DictColumn {
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:84-196 (routed: perf)
  [clippy] redundant_closure_for_method_calls score   391.02  →  fn lens(v: &[String]) -> Vec<usize> { v.iter().map(String::len).collect() }
  [clippy] map_flatten                      score   340.94  →  fn word_lines(xs: &[String]) -> Vec<&str> { xs.iter().flat_map(|x| x.split(' ')).collect() }
  [clippy] map_unwrap_or                    score   305.97  →  fn f(x: Option<u8>) -> u8 { x.map_or(0, |v| v + 1) }
  [perf  ] encoded-layout-borrowed-view     score   617.75  →  struct DictColumn {
  [perf  ] page-miss-blocks-one-ranged-read-scatter score   611.56  →  #![allow(dead_code)]
  [perf  ] restore-point-chain-copies-keyed-on-disk-state-count score   581.85  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:198-214 (routed: clippy)
  [clippy] redundant_closure_for_method_calls score    47.54  →  fn lens(v: &[String]) -> Vec<usize> { v.iter().map(String::len).collect() }
  [clippy] map_unwrap_or                    score    41.58  →  fn f(x: Option<u8>) -> u8 { x.map_or(0, |v| v + 1) }
  [clippy] format_collect                   score    41.43  →  use std::fmt::Write as _;
  [perf  ] powi-vs-powf                     score    86.52  →  fn sum_squares(vals: &[f32]) -> f32 {
  [perf  ] constrained-unconstrained-lanes-one-decode-command score    82.62  →  #![allow(dead_code)]
  [perf  ] fallthrough-toggle-oracle        score    81.15  →  // OPT: ONE entry point, ONE admission bool. The "oracle" is the same
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:216-239 (routed: perf)
  [clippy] vec_init_then_push               score    68.84  →  fn build() -> Vec<u8> {
  [clippy] redundant_closure_for_method_calls score    58.45  →  fn lens(v: &[String]) -> Vec<usize> { v.iter().map(String::len).collect() }
  [clippy] chunks_exact_to_as_chunks        score    56.97  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [perf  ] mtime-encoded-lru-recency-persistence score   125.20  →  #![allow(dead_code)]
  [perf  ] smallvec-stack                   score   124.66  →  fn sum_small(n: usize) -> u64 {
  [perf  ] reuse-allocation-clear           score   119.69  →  fn hot(n: usize, buf: &mut Vec<u32>) -> u64 {
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/cargo_metadata.rs:241-247 (routed: clippy)
  [clippy] manual_slice_fill                score    73.42  →  struct Terrain { data: Vec<f32> }
  [clippy] multiple_bound_locations         score    45.44  →  pub fn mbl_a<T>(t: T) -> T
  [clippy] redundant_closure_for_method_calls score    39.50  →  fn lens(v: &[String]) -> Vec<usize> { v.iter().map(String::len).collect() }
  [perf  ] restore-point-chain-copies-keyed-on-disk-state-count score   111.54  →  #![allow(dead_code)]
  [perf  ] prefilter-choice-shape-ladder    score   108.39  →  // OPT: the matcher is keyed ONCE at build on the target-set shape:
  [perf  ] prefix-cache-entry-one-token-early score   103.11  →  // OPT: the kept point sits at max(1, len - 1) — the longest prefix any
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/test_util.rs:5-5 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.41  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    12.67  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    12.67  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/test_util.rs:8-13 (routed: perf)
  [clippy] write_with_newline               score    20.98  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    20.84  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] from_str_radix_10                score    18.15  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [perf  ] exclusive-mkdir-temp-resource-reservation score    53.90  →  #![allow(dead_code)]
  [perf  ] generation-counter-stale-timer-invalidation score    53.90  →  #![allow(dead_code)]
  [perf  ] per-read-timeout-vs-request-deadline score    53.90  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/test_util.rs:31-34 (routed: clippy)
  [clippy] let_and_return                   score    29.94  →  fn shout(s: &str) -> String { s.to_uppercase() }
  [clippy] uninlined_format_args            score    29.08  →  fn f(name: &str) -> String { format!("{name}") }
  [clippy] map_unwrap_or                    score    22.51  →  fn f(x: Option<u8>) -> u8 { x.map_or(0, |v| v + 1) }
  [perf  ] format-concat-loop               score    64.54  →  fn build(chunks: &[&str]) -> String {
  [perf  ] cow-conditional-alloc            score    63.35  →  use std::borrow::Cow;
  [perf  ] prompt-state-fingerprint-probe-bits-keyed-persistence score    62.12  →  // OPT: the identity is a FINGERPRINT over every term the state's
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/test_util.rs:83-90 (routed: perf)
  [clippy] collapsible_if                   score    34.05  →  fn work() {}
  [clippy] len_without_is_empty             score    32.49  →  pub struct LwA { items: Vec<u8> }
  [clippy] collapsible_else_if              score    31.61  →  fn work() {}
  [perf  ] batch-load-admission-sized-from-files-refuses-past-the-hard-budget score    98.47  →  #![allow(dead_code)]
  [perf  ] refusal-quarantines-entry-transient-passes score    84.32  →  #![allow(dead_code)]
  [perf  ] immutable-static-map-filter-selection score    83.50  →  use constmap::ConstMap;
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/test_util.rs:114-120 (routed: perf)
  [clippy] chunks_exact_to_as_chunks        score    49.32  →  fn ids(bytes: &[u8]) -> Vec<u32> {
  [clippy] unnecessary_unwrap               score    43.83  →  fn process(opt: Option<i32>) -> i32 {
  [clippy] map_unwrap_or                    score    37.31  →  fn f(x: Option<u8>) -> u8 { x.map_or(0, |v| v + 1) }
  [perf  ] exclusive-mkdir-temp-resource-reservation score   145.96  →  #![allow(dead_code)]
  [perf  ] arrival-order-turnstile-lane     score   120.04  →  #![allow(dead_code)]
  [perf  ] resume-revalidates-provenance-before-reuse score   118.86  →  // The manifest records the build IDENTITY (source + calibration +
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/global_config.rs:5-5 (routed: perf)
  [clippy] unnecessary_get_then_check       score    23.14  →  fn ugtc(m: std::collections::HashMap<u8, u8>, k: u8) -> bool { !m.contains_key(&k) }
  [clippy] write_with_newline               score    13.68  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score     9.43  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] sorted-input-previous-row-fast-path score    44.47  →  use std::collections::HashMap;
  [perf  ] papaya-lockfree                  score    39.47  →  use papaya::HashMap;
  [perf  ] tune-cache-keyed-by-spelled-variant-names score    34.90  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/global_config.rs:7-7 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] redundant_pattern_matching       score     5.41  →  fn f(r: Result<u8, u8>) -> bool { if r.is_ok() { true } else { false } }
  [clippy] from_str_radix_10                score     5.05  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [perf  ] write-vectored-capability-flag   score    43.35  →  use std::io;
  [perf  ] varint-compose-word-single-store score    30.18  →  use std::io::{Write, Result};
  [perf  ] steal-batch-not-per-item         score    14.96  →  use crossbeam_deque::{Injector, Steal};
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/global_config.rs:8-8 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] match_ref_pats                   score     2.59  →  pub enum Fruit { Apple, Pear }
  [clippy] redundant_field_names            score     2.55  →  struct Point { x: i32, y: i32 }
  [perf  ] steal-batch-not-per-item         score    14.96  →  use crossbeam_deque::{Injector, Steal};
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    14.14  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
  [perf  ] mtime-encoded-lru-recency-persistence score    14.14  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/global_config.rs:9-9 (routed: perf)
  [clippy] write_with_newline               score    12.51  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     5.41  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] iter_next_loop                   score     4.81  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [perf  ] serde-borrow-deserialize         score    71.05  →  use serde::Deserialize;
  [perf  ] rawvalue-passthrough-defer-parse score    21.08  →  use serde::Deserialize;
  [perf  ] bevy-reactive-loop               score    12.67  →  use bevy::winit::{UpdateMode, WinitSettings};
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi_bindgen/src/global_config.rs:11-11 (routed: perf)
  [clippy] write_with_newline               score     6.99  →  use std::fmt::Write as _;
  [clippy] match_ref_pats                   score     4.64  →  pub enum Fruit { Apple, Pear }
  [clippy] redundant_field_names            score     4.57  →  struct Point { x: i32, y: i32 }
  [perf  ] steal-batch-not-per-item         score    16.46  →  use crossbeam_deque::{Injector, Steal};
  [perf  ] bufread-scan-in-buffer-scratch-fragment score    15.55  →  // OPT: lend out complete record slices from fill_buf() (zero-copy),
  [perf  ] mtime-encoded-lru-recency-persistence score    15.55  →  #![allow(dead_code)]
  … and 3947 more span(s) — the summary below is the full count
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi-bindgen-kotlin-jni/Cargo.toml:1 [dist  ] shipped-binary-unstripped                — autodiscovered bin (src/main.rs or src/bin/)
           ↳ add `strip = true` — the default keeps the symbol table, and a shipped binary's symtab IS the internal architecture (14,485 symbols measured on this tool's own v0.1.3)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi-bindgen-kotlin-jni/Cargo.toml:1 [dist  ] shipped-binary-missing-size-profile      — autodiscovered bin (src/main.rs or src/bin/)
           ↳ add a size profile ([profile.dist]: lto = "fat" + codegen-units = 1 + opt-level = "s" + panic = "abort" + strip = true) and build releases with --profile dist — measured -61% on this tool itself; NEVER put opt-level/codegen-units into [profile.release] (the bench profile inherits release)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi-bindgen-kotlin-jni/Cargo.toml:1 [dist  ] cli-panic-unwind-tables                  — autodiscovered bin (src/main.rs or src/bin/)
           ↳ consider `panic = "abort"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi/Cargo.toml:71 [dist  ] shipped-binary-unstripped                — [[bin]]
           ↳ add `strip = true` — the default keeps the symbol table, and a shipped binary's symtab IS the internal architecture (14,485 symbols measured on this tool's own v0.1.3)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi/Cargo.toml:71 [dist  ] shipped-binary-missing-size-profile      — [[bin]]
           ↳ add a size profile ([profile.dist]: lto = "fat" + codegen-units = 1 + opt-level = "s" + panic = "abort" + strip = true) and build releases with --profile dist — measured -61% on this tool itself; NEVER put opt-level/codegen-units into [profile.release] (the bench profile inherits release)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/uniffi/Cargo.toml:71 [dist  ] cli-panic-unwind-tables                  — [[bin]]
           ↳ consider `panic = "abort"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/fixtures/futures/Cargo.toml:12 [dist  ] shipped-binary-unstripped                — [[bin]]
           ↳ add `strip = true` — the default keeps the symbol table, and a shipped binary's symtab IS the internal architecture (14,485 symbols measured on this tool's own v0.1.3)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/fixtures/futures/Cargo.toml:12 [dist  ] shipped-binary-missing-size-profile      — [[bin]]
           ↳ add a size profile ([profile.dist]: lto = "fat" + codegen-units = 1 + opt-level = "s" + panic = "abort" + strip = true) and build releases with --profile dist — measured -61% on this tool itself; NEVER put opt-level/codegen-units into [profile.release] (the bench profile inherits release)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/fixtures/futures/Cargo.toml:12 [dist  ] cli-panic-unwind-tables                  — [[bin]]
           ↳ consider `panic = "abort"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/fixtures/version-mismatch/Cargo.toml:12 [dist  ] shipped-binary-unstripped                — [[bin]]
           ↳ add `strip = true` — the default keeps the symbol table, and a shipped binary's symtab IS the internal architecture (14,485 symbols measured on this tool's own v0.1.3)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/fixtures/version-mismatch/Cargo.toml:12 [dist  ] shipped-binary-missing-size-profile      — [[bin]]
           ↳ add a size profile ([profile.dist]: lto = "fat" + codegen-units = 1 + opt-level = "s" + panic = "abort" + strip = true) and build releases with --profile dist — measured -61% on this tool itself; NEVER put opt-level/codegen-units into [profile.release] (the bench profile inherits release)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/fixtures/version-mismatch/Cargo.toml:12 [dist  ] cli-panic-unwind-tables                  — [[bin]]
           ↳ consider `panic = "abort"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/examples/app/uniffi-bindgen-cli/Cargo.toml:7 [dist  ] shipped-binary-unstripped                — [[bin]]
           ↳ add `strip = true` — the default keeps the symbol table, and a shipped binary's symtab IS the internal architecture (14,485 symbols measured on this tool's own v0.1.3)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/examples/app/uniffi-bindgen-cli/Cargo.toml:7 [dist  ] shipped-binary-missing-size-profile      — [[bin]]
           ↳ add a size profile ([profile.dist]: lto = "fat" + codegen-units = 1 + opt-level = "s" + panic = "abort" + strip = true) and build releases with --profile dist — measured -61% on this tool itself; NEVER put opt-level/codegen-units into [profile.release] (the bench profile inherits release)
/Users/kat/git/riir-refine/.raw/mozilla-heal/uniffi-rs/examples/app/uniffi-bindgen-cli/Cargo.toml:7 [dist  ] cli-panic-unwind-tables                  — [[bin]]
           ↳ consider `panic = "abort"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection
🔒 3 secret-shaped literal(s) in 2 files (+1 digest-pin(s) adjudicated benign — suppressed from this count; the sync gate stays fail-closed) — suggest-only, never auto-fixed; details: `cargo refine scan`
🔍 Found clippy 75 | perf 75 | dist 15 | sec 3 — 168 total
🛠️ ~44% fixable via `cargo refine fix` (compile-gated; the suggest-only rows stay manual)

scanned 520 files · 3972 spans · 3972 with suggestions (top-3 per domain)
