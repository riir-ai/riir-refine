cargo-refine 0.4.2 — help: `cargo refine help`
scan root: /Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust — 26 source file(s)
Refining…
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_colour_info.rs:1-1 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] manual_map                       score     0.71  →  fn opt_double(x: Option<i32>) -> Option<i32> {
  [clippy] manual_is_multiple_of            score     0.59  →  fn f(n: u64) -> bool { n.is_multiple_of(4) }
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    17.57  →  #![allow(dead_code)]
  [perf  ] rayon-bridge-oneshot             score    16.48  →  use rayon::prelude::*;
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    15.52  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_colour_info.rs:2-2 (routed: perf)
  [clippy] io_other_error                   score    36.72  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    24.01  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    14.93  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] per-read-timeout-vs-request-deadline score    76.74  →  #![allow(dead_code)]
  [perf  ] bufwriter-batched                score    51.88  →  use std::io::{BufWriter, Write};
  [perf  ] write-vectored-capability-flag   score    44.53  →  use std::io;
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_colour_info.rs:4-11 (routed: clippy)
  [clippy] write_with_newline               score    86.79  →  use std::fmt::Write as _;
  [clippy] manual_let_else                  score    85.62  →  fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }
  [clippy] io_other_error                   score    72.09  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] hot-slot-recycler-flat-tables-zero-alloc-claims score   141.13  →  #![allow(dead_code)]
  [perf  ] bufwriter-batched                score   134.99  →  use std::io::{BufWriter, Write};
  [perf  ] raw-pointer-traversal-slice-view-recovery-load-elimination score   134.45  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_colour_info.rs:13-24 (routed: perf)
  [clippy] io_other_error                   score    82.07  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    81.88  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    78.54  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] buffered-io                      score   226.86  →  use std::io::Read;
  [perf  ] write-verify-readback-before-publish score   174.02  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] bufwriter-batched                score   168.84  →  use std::io::{BufWriter, Write};
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_sample_table.rs:1-1 (routed: perf)
  [clippy] write_with_newline               score    16.16  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score     8.73  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [clippy] io_other_error                   score     7.83  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] bevy-reactive-loop               score    16.87  →  use bevy::winit::{UpdateMode, WinitSettings};
  [perf  ] bufwriter-batched                score    16.87  →  use std::io::{BufWriter, Write};
  [perf  ] dataflow-fixpoint-convergence-traversal-order score    16.87  →  // OPT: a MIN-RANK worklist. Give every block a rank = its position
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_sample_table.rs:2-2 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] manual_map                       score     0.71  →  fn opt_double(x: Option<i32>) -> Option<i32> {
  [clippy] manual_is_multiple_of            score     0.59  →  fn f(n: u64) -> bool { n.is_multiple_of(4) }
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    17.57  →  #![allow(dead_code)]
  [perf  ] rayon-bridge-oneshot             score    16.48  →  use rayon::prelude::*;
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    15.52  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_sample_table.rs:3-3 (routed: perf)
  [clippy] io_other_error                   score    36.72  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    24.01  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    14.93  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] per-read-timeout-vs-request-deadline score    76.74  →  #![allow(dead_code)]
  [perf  ] bufwriter-batched                score    51.88  →  use std::io::{BufWriter, Write};
  [perf  ] write-vectored-capability-flag   score    44.53  →  use std::io;
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_sample_table.rs:5-12 (routed: clippy)
  [clippy] write_with_newline               score    86.79  →  use std::fmt::Write as _;
  [clippy] manual_let_else                  score    85.62  →  fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }
  [clippy] io_other_error                   score    72.09  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] hot-slot-recycler-flat-tables-zero-alloc-claims score   141.13  →  #![allow(dead_code)]
  [perf  ] bufwriter-batched                score   134.99  →  use std::io::{BufWriter, Write};
  [perf  ] raw-pointer-traversal-slice-view-recovery-load-elimination score   134.45  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_sample_table.rs:14-132 (routed: perf)
  [clippy] useless_conversion               score   557.26  →  fn add_one(x: i32) -> i32 {
  [clippy] unit_arg                         score   370.44  →  fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }
  [clippy] from_str_radix_10                score   334.04  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [perf  ] hot-slot-recycler-flat-tables-zero-alloc-claims score   588.24  →  #![allow(dead_code)]
  [perf  ] restore-point-chain-copies-keyed-on-disk-state-count score   588.24  →  #![allow(dead_code)]
  [perf  ] stack-read-fast-path-length-prefixed-record score   584.98  →  const HEADER: usize = 4;
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_sample_table.rs:134-183 (routed: perf)
  [clippy] write_with_newline               score   164.58  →  use std::fmt::Write as _;
  [clippy] manual_let_else                  score   159.30  →  fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }
  [clippy] mem_replace_with_default         score   159.30  →  fn mrw(dst: &mut Vec<u32>, sink: &mut Vec<Vec<u32>>) { sink.push(Vec::new()); sink.push(std::mem::take(dst)) }
  [perf  ] buffered-io                      score   281.47  →  use std::io::Read;
  [perf  ] stack-read-fast-path-length-prefixed-record score   273.34  →  const HEADER: usize = 4;
  [perf  ] checkpoint-reads-skip-page-cache-with-refusal-fallback score   272.55  →  // OPT: checkpoint reads skip the page cache -- the open asks for O_DIRECT
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_sample_table.rs:185-253 (routed: perf)
  [clippy] useless_conversion               score   269.05  →  fn add_one(x: i32) -> i32 {
  [clippy] unit_arg                         score   201.89  →  fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }
  [clippy] vec_init_then_push               score   197.62  →  fn build() -> Vec<u8> {
  [perf  ] checkpoint-reads-skip-page-cache-with-refusal-fallback score   338.04  →  // OPT: checkpoint reads skip the page cache -- the open asks for O_DIRECT
  [perf  ] stack-read-fast-path-length-prefixed-record score   336.41  →  const HEADER: usize = 4;
  [perf  ] batch-load-admission-sized-from-files-refuses-past-the-hard-budget score   315.20  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_sample_table.rs:255-329 (routed: perf)
  [clippy] useless_conversion               score   363.47  →  fn add_one(x: i32) -> i32 {
  [clippy] unit_arg                         score   239.72  →  fn touch(x: (), hits: &mut Vec<u8>) { hits.push(0); }
  [clippy] from_str_radix_10                score   209.86  →  fn num(s: &str, hits: &mut Vec<u32>) -> Result<u32, std::num::ParseIntError> { hits.push(0); s.parse::<u32>() }
  [perf  ] stack-read-fast-path-length-prefixed-record score   363.87  →  const HEADER: usize = 4;
  [perf  ] hot-slot-recycler-flat-tables-zero-alloc-claims score   359.66  →  #![allow(dead_code)]
  [perf  ] restore-point-chain-copies-keyed-on-disk-state-count score   359.66  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_afconvert.rs:4-4 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] manual_map                       score     0.71  →  fn opt_double(x: Option<i32>) -> Option<i32> {
  [clippy] manual_is_multiple_of            score     0.59  →  fn f(n: u64) -> bool { n.is_multiple_of(4) }
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    17.57  →  #![allow(dead_code)]
  [perf  ] rayon-bridge-oneshot             score    16.48  →  use rayon::prelude::*;
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    15.52  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_afconvert.rs:5-5 (routed: perf)
  [clippy] write_with_newline               score    24.01  →  use std::fmt::Write as _;
  [clippy] io_other_error                   score    15.85  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] legacy_numeric_constants         score    14.93  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] nonblocking-open-for-head-probes score    79.68  →  #[cfg(unix)]
  [perf  ] buffered-io                      score    50.78  →  use std::io::Read;
  [perf  ] write-verify-readback-before-publish score    46.61  →  // Three gates before the artifact exists: (1) pre-flight template
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_afconvert.rs:6-6 (routed: perf)
  [clippy] io_other_error                   score    36.72  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    24.01  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    14.93  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] per-read-timeout-vs-request-deadline score    76.74  →  #![allow(dead_code)]
  [perf  ] bufwriter-batched                score    51.88  →  use std::io::{BufWriter, Write};
  [perf  ] write-vectored-capability-flag   score    44.53  →  use std::io;
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_afconvert.rs:8-8 (routed: clippy)
  [clippy] redundant_static_lifetimes       score    37.55  →  static GREETING: &str = "hi";
  [clippy] doc_overindented_list_items      score    13.70  →  /// - the item text.
  [clippy] map_unwrap_or                    score    12.92  →  fn f(x: Option<u8>) -> u8 { x.map_or(0, |v| v + 1) }
  [perf  ] fastest-of-n-dip-drift-retime-cost-table score    55.11  →  // OPT: the capture is a bounded REPAIR protocol and the persistence is
  [perf  ] batch-load-admission-sized-from-files-refuses-past-the-hard-budget score    48.96  →  #![allow(dead_code)]
  [perf  ] finished-command-copy-leaves-at-once-slot-returns-at-last-reader score    35.99  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_afconvert.rs:10-17 (routed: clippy)
  [clippy] manual_let_else                  score    85.62  →  fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }
  [clippy] write_with_newline               score    76.17  →  use std::fmt::Write as _;
  [clippy] manual_ok_err                    score    64.72  →  fn moe(r: Result<u32, String>) -> Option<u32> { r.ok() }
  [perf  ] hot-slot-recycler-flat-tables-zero-alloc-claims score   141.13  →  #![allow(dead_code)]
  [perf  ] raw-pointer-traversal-slice-view-recovery-load-elimination score   134.45  →  #![allow(dead_code)]
  [perf  ] page-miss-blocks-one-ranged-read-scatter score   124.23  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_afconvert.rs:19-63 (routed: perf)
  [clippy] write_with_newline               score   164.16  →  use std::fmt::Write as _;
  [clippy] manual_slice_fill                score   154.15  →  struct Terrain { data: Vec<f32> }
  [clippy] manual_let_else                  score   147.55  →  fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }
  [perf  ] saturating-f2i-cast-proven-range-to-int-unchecked score   278.21  →  #![allow(dead_code)]
  [perf  ] stack-read-fast-path-length-prefixed-record score   235.09  →  const HEADER: usize = 4;
  [perf  ] buffered-io                      score   234.14  →  use std::io::Read;
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_chunk_out_of_range.rs:1-1 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] manual_map                       score     0.71  →  fn opt_double(x: Option<i32>) -> Option<i32> {
  [clippy] manual_is_multiple_of            score     0.59  →  fn f(n: u64) -> bool { n.is_multiple_of(4) }
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    17.57  →  #![allow(dead_code)]
  [perf  ] rayon-bridge-oneshot             score    16.48  →  use rayon::prelude::*;
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    15.52  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_chunk_out_of_range.rs:2-2 (routed: perf)
  [clippy] io_other_error                   score    36.72  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    24.01  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    14.93  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] per-read-timeout-vs-request-deadline score    76.74  →  #![allow(dead_code)]
  [perf  ] bufwriter-batched                score    51.88  →  use std::io::{BufWriter, Write};
  [perf  ] write-vectored-capability-flag   score    44.53  →  use std::io;
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_chunk_out_of_range.rs:4-11 (routed: clippy)
  [clippy] write_with_newline               score    86.79  →  use std::fmt::Write as _;
  [clippy] manual_let_else                  score    85.62  →  fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }
  [clippy] io_other_error                   score    72.09  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] hot-slot-recycler-flat-tables-zero-alloc-claims score   141.13  →  #![allow(dead_code)]
  [perf  ] bufwriter-batched                score   134.99  →  use std::io::{BufWriter, Write};
  [perf  ] raw-pointer-traversal-slice-view-recovery-load-elimination score   134.45  →  #![allow(dead_code)]
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_chunk_out_of_range.rs:13-44 (routed: perf)
  [clippy] write_with_newline               score   132.34  →  use std::fmt::Write as _;
  [clippy] while_let_on_iterator            score   116.54  →  fn f(mut it: std::vec::IntoIter<u8>) -> u8 { let mut s = 0; for x in it { s += x; } s }
  [clippy] manual_let_else                  score   112.80  →  fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }
  [perf  ] buffered-io                      score   234.45  →  use std::io::Read;
  [perf  ] write-verify-readback-before-publish score   213.27  →  // Three gates before the artifact exists: (1) pre-flight template
  [perf  ] bufwriter-batched                score   209.68  →  use std::io::{BufWriter, Write};
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_encryption.rs:1-1 (routed: perf)
  [clippy] write_with_newline               score     6.11  →  use std::fmt::Write as _;
  [clippy] manual_map                       score     0.71  →  fn opt_double(x: Option<i32>) -> Option<i32> {
  [clippy] manual_is_multiple_of            score     0.59  →  fn f(n: u64) -> bool { n.is_multiple_of(4) }
  [perf  ] cache-share-capped-at-measured-distinct-working-set score    17.57  →  #![allow(dead_code)]
  [perf  ] rayon-bridge-oneshot             score    16.48  →  use rayon::prelude::*;
  [perf  ] avx2-tiled-accumulator-butterfly-epilogue score    15.52  →  // OPT: the bank is ONE reduction problem, not eight. Reduce the
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_encryption.rs:2-2 (routed: perf)
  [clippy] io_other_error                   score    36.72  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [clippy] write_with_newline               score    24.01  →  use std::fmt::Write as _;
  [clippy] legacy_numeric_constants         score    14.93  →  fn cap() -> u32 { let cap = u32::MAX; cap }
  [perf  ] per-read-timeout-vs-request-deadline score    76.74  →  #![allow(dead_code)]
  [perf  ] bufwriter-batched                score    51.88  →  use std::io::{BufWriter, Write};
  [perf  ] write-vectored-capability-flag   score    44.53  →  use std::io;
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/tests/test_encryption.rs:4-11 (routed: clippy)
  [clippy] write_with_newline               score    86.79  →  use std::fmt::Write as _;
  [clippy] manual_let_else                  score    85.62  →  fn f(x: Option<u32>) -> u32 { let Some(v) = x else { return 0 }; v + 1 }
  [clippy] io_other_error                   score    72.09  →  fn mk(e: String) -> std::io::Error { std::io::Error::other(e) }
  [perf  ] hot-slot-recycler-flat-tables-zero-alloc-claims score   141.13  →  #![allow(dead_code)]
  [perf  ] bufwriter-batched                score   134.99  →  use std::io::{BufWriter, Write};
  [perf  ] raw-pointer-traversal-slice-view-recovery-load-elimination score   134.45  →  #![allow(dead_code)]
  … and 589 more span(s) — the summary below is the full count
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/fuzz/Cargo.toml:21 [dist  ] shipped-binary-unstripped                — [[bin]]
           ↳ add `strip = true` — the default keeps the symbol table, and a shipped binary's symtab IS the internal architecture (14,485 symbols measured on this tool's own v0.1.3)
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/fuzz/Cargo.toml:21 [dist  ] shipped-binary-missing-size-profile      — [[bin]]
           ↳ add a size profile ([profile.dist]: lto = "fat" + codegen-units = 1 + opt-level = "s" + panic = "abort" + strip = true) and build releases with --profile dist — measured -61% on this tool itself; NEVER put opt-level/codegen-units into [profile.release] (the bench profile inherits release)
/Users/kat/git/riir-refine/.raw/mozilla-heal/mp4parse-rust/mp4parse_capi/fuzz/Cargo.toml:21 [dist  ] cli-panic-unwind-tables                  — [[bin]]
           ↳ consider `panic = "abort"` in the shipped profile — landing pads and unwind tables are real bytes in a CLI; NOT for servers that isolate panics per task/connection
🔒 2 secret-shaped literal(s) in 2 files (+8 digest-pin(s) adjudicated benign — suppressed from this count; the sync gate stays fail-closed) — suggest-only, never auto-fixed; details: `cargo refine scan`
🔍 Found clippy 75 | perf 75 | dist 3 | sec 2 — 155 total
🛠️ ~46% fixable via `cargo refine fix` (compile-gated; the suggest-only rows stay manual)

scanned 22 files · 614 spans · 614 with suggestions (top-3 per domain)
